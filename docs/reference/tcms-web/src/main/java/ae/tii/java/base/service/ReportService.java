package ae.tii.java.base.service;

import ae.tii.java.base.audit.AuditEvent;
import ae.tii.java.base.audit.AuditRecorder;
import ae.tii.java.base.common.NotFoundException;
import ae.tii.java.base.dto.DefectResponse;
import ae.tii.java.base.dto.HistoryResponse;
import ae.tii.java.base.dto.PlanScopeDto;
import ae.tii.java.base.dto.RunDocumentResponse;
import ae.tii.java.base.dto.RunReportResponse;
import ae.tii.java.base.dto.TraceabilityResponse;
import ae.tii.java.base.model.Defect;
import ae.tii.java.base.model.PlanRun;
import ae.tii.java.base.model.PlanRunCase;
import ae.tii.java.base.model.TestPlan;
import ae.tii.java.base.model.TestPlanVersion;
import ae.tii.java.base.model.User;
import ae.tii.java.base.model.WorkspaceSettings;
import ae.tii.java.base.report.CsvWriter;
import ae.tii.java.base.report.FailingSince;
import ae.tii.java.base.report.PassRate;
import ae.tii.java.base.report.StabilityClassifier;
import ae.tii.java.base.report.SuiteBreakdown;
import ae.tii.java.base.repository.ConnectedRepositoryRepository;
import ae.tii.java.base.repository.DefectRepository;
import ae.tii.java.base.repository.PlanRunCaseRepository;
import ae.tii.java.base.repository.PlanRunRepository;
import ae.tii.java.base.repository.TestPlanRepository;
import ae.tii.java.base.repository.TestPlanVersionRepository;
import ae.tii.java.base.repository.UserRepository;
import ae.tii.java.base.repository.WorkspaceSettingsRepository;
import ae.tii.java.base.security.ProductAccess;
import ae.tii.java.base.security.WorkspaceAccess;
import ae.tii.java.base.security.WorkspaceContext;
import ae.tii.java.base.security.WorkspaceContextHolder;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import tools.jackson.databind.ObjectMapper;

/**
 * Reports over plan runs (ORCH-016, DECISIONS.md D-86 to D-100): one run's summary, the history
 * matrix of a plan and the CSV exports. Everything is computed on read from the run tables, so a
 * change of the quality gate recolours history. Any member of the workspace may read.
 */
@Service
@RequiredArgsConstructor
public class ReportService {

  static final int DEFAULT_WINDOW = 12;
  static final int MAX_WINDOW = 50;

  /** How many earlier runs "failing since" may look back over. */
  private static final int WALK_BACK = 50;

  private final PlanRunRepository runs;
  private final PlanRunCaseRepository runCases;
  private final TestPlanRepository plans;
  private final WorkspaceSettingsRepository settings;
  private final UserRepository users;
  private final WorkspaceAccess workspaceAccess;
  private final AuditRecorder audit;
  private final TestPlanVersionRepository planVersions;
  private final ConnectedRepositoryRepository repositories;
  private final DefectRepository defects;
  private final TraceabilityService traceability;
  private final ObjectMapper json;
  private final ProductAccess productAccess;

  /** A generated file: its name and bytes. */
  public record CsvFile(String fileName, byte[] content) {}

  /** The selected run, the plan's earlier runs and the cases of each, loaded once. */
  private record Loaded(
      PlanRun selected,
      TestPlan plan,
      int gate,
      List<PlanRun> window,
      Map<Long, List<PlanRunCase>> casesByRun,
      List<PlanRun> earlierNewestFirst) {}

  @Transactional(readOnly = true)
  public RunReportResponse summary(Long workspaceId, Long runId) {
    workspaceAccess.require(workspaceId);
    return buildSummary(workspaceId, runId);
  }

  /**
   * The summary without the caller check, for paths that proved access another way: a share link
   * resolved by its token (DECISIONS.md D-119.1).
   */
  RunReportResponse buildSummary(Long workspaceId, Long runId) {
    PlanRun selected = findRun(workspaceId, runId);
    Loaded data = load(workspaceId, selected, DEFAULT_WINDOW);
    List<PlanRunCase> cases = data.casesByRun().get(selected.getId());
    Counts counts = Counts.of(cases);
    PassRate.Result rate = PassRate.of(counts.passed, counts.failed, counts.blocked, data.gate());

    PlanRun previousRun =
        data.earlierNewestFirst().isEmpty() ? null : data.earlierNewestFirst().get(0);
    RunReportResponse.Previous previous = null;
    if (previousRun != null) {
      Counts before = Counts.of(data.casesByRun().get(previousRun.getId()));
      PassRate.Result beforeRate =
          PassRate.of(before.passed, before.failed, before.blocked, data.gate());
      previous =
          new RunReportResponse.Previous(
              previousRun.getId(),
              previousRun.getNumber(),
              beforeRate.displayPercent(),
              PassRate.delta(rate, beforeRate));
    }

    Map<String, Map<Long, String>> statusByKey = statusByKey(data.casesByRun());
    List<PlanRun> stabilityWindow = data.window();
    List<RunReportResponse.Problem> problems = new ArrayList<>();
    Map<Long, List<DefectResponse>> defectsByCase = defectsOf(selected.getId());
    int flaky = 0;
    for (PlanRunCase c : cases) {
      String key = key(c);
      StabilityClassifier.Result stability =
          StabilityClassifier.classify(cells(statusByKey.get(key), stabilityWindow));
      if (StabilityClassifier.FLAKY.equals(stability.stability())) {
        flaky++;
      }
      if (c.getStatus() == PlanRunCase.Status.FAILED
          || c.getStatus() == PlanRunCase.Status.BLOCKED) {
        RunReportResponse.Since since = null;
        if (c.getStatus() == PlanRunCase.Status.FAILED) {
          List<FailingSince.Earlier> earlier = new ArrayList<>();
          for (PlanRun run : data.earlierNewestFirst()) {
            earlier.add(
                new FailingSince.Earlier(point(run), statusByKey.get(key).get(run.getId())));
          }
          FailingSince.RunPoint start = FailingSince.compute(point(selected), earlier);
          since = new RunReportResponse.Since(start.runId(), start.number(), start.createdAt());
        }
        problems.add(
            new RunReportResponse.Problem(
                c.getId(),
                c.getCaseId(),
                c.getTitle(),
                suite(c),
                c.getStatus().name(),
                c.getSource() == null ? null : c.getSource().name(),
                c.getNotes(),
                defectsByCase.getOrDefault(c.getId(), List.of()).stream()
                    .findFirst()
                    .map(DefectResponse::issueKey)
                    .orElse(null),
                since,
                stability.stability(),
                defectsByCase.getOrDefault(c.getId(), List.of())));
      }
    }
    problems.sort(
        Comparator.comparing((RunReportResponse.Problem p) -> !"FAILED".equals(p.status()))
            .thenComparing(RunReportResponse.Problem::suite)
            .thenComparing(p -> p.caseId() == null ? "" : p.caseId()));

    List<SuiteBreakdown.Entry> entries = new ArrayList<>();
    cases.forEach(c -> entries.add(new SuiteBreakdown.Entry(suite(c), c.getStatus().name())));
    List<RunReportResponse.Suite> suites =
        SuiteBreakdown.of(entries).stream()
            .map(
                r ->
                    new RunReportResponse.Suite(
                        r.path(),
                        r.name(),
                        r.depth(),
                        r.total(),
                        r.passed(),
                        r.failed(),
                        r.blocked(),
                        r.skipped(),
                        r.untested(),
                        r.passRate()))
            .toList();

    return new RunReportResponse(
        new RunReportResponse.Run(
            selected.getId(),
            selected.getNumber(),
            selected.getPlanId(),
            selected.getPlanName(),
            data.plan() == null ? null : data.plan().getSlug(),
            selected.getStatus().name(),
            selected.getOrigin().name(),
            userName(selected.getCreatedBy()),
            selected.getCreatedAt(),
            selected.getClosedAt(),
            selected.getTestsBranch(),
            selected.getTestsCommit(),
            selected.getCodeRepositoryName(),
            selected.getCodeBranch(),
            selected.getCodeCommit(),
            selected.getCiPipelineId(),
            selected.getCiPipelineUrl()),
        new RunReportResponse.Gate(
            data.gate(),
            rate.displayPercent(),
            rate.verdict(),
            selected.getStatus() == PlanRun.Status.OPEN),
        new RunReportResponse.Counts(
            counts.total,
            counts.total - counts.untested,
            counts.passed,
            counts.failed,
            counts.blocked,
            counts.skipped,
            counts.untested),
        previous,
        flaky,
        suites,
        problems);
  }

  /**
   * The history matrix. Selects the run by {@code runId}, else the latest run of {@code planId},
   * else the most recent run of the workspace.
   */
  @Transactional(readOnly = true)
  public HistoryResponse history(Long workspaceId, Long planId, Long runId, Integer limit) {
    workspaceAccess.require(workspaceId);
    PlanRun selected = selectRun(workspaceId, planId, runId);
    return buildHistory(workspaceId, selected, windowSize(limit));
  }

  /**
   * The summary, heatmap or tpdg layout of a run as CSV (DECISIONS.md D-100, D-115.4). {@code
   * fromRun} and {@code toRun} apply to tpdg only.
   */
  @Transactional(readOnly = true)
  public CsvFile exportCsv(
      Long workspaceId, Long runId, String layout, Integer fromRun, Integer toRun) {
    WorkspaceContext context = workspaceAccess.require(workspaceId);
    String kind = layout == null ? "summary" : layout.trim().toLowerCase(Locale.ROOT);
    if (!kind.equals("summary") && !kind.equals("heatmap") && !kind.equals("tpdg")) {
      throw new IllegalArgumentException("layout must be summary, heatmap or tpdg");
    }
    PlanRun selected = findRun(workspaceId, runId);
    CsvWriter csv = new CsvWriter();
    Map<String, String> details = new LinkedHashMap<>();
    details.put("runId", String.valueOf(selected.getId()));
    details.put("layout", kind);
    details.put("format", "csv");
    if (kind.equals("summary")) {
      writeSummary(csv, load(workspaceId, selected, DEFAULT_WINDOW));
    } else if (kind.equals("heatmap")) {
      writeHeatmap(csv, buildHistory(workspaceId, selected, DEFAULT_WINDOW));
    } else {
      Range range = range(workspaceId, selected, fromRun, toRun);
      writeTpdg(csv, workspaceId, selected, range);
      details.put("fromRun", String.valueOf(range.window().get(0).getNumber()));
      details.put("toRun", String.valueOf(range.end().getNumber()));
    }
    TestPlan plan = plans.findByIdAndWorkspaceId(selected.getPlanId(), workspaceId).orElse(null);
    String name =
        (plan == null ? "plan" : plan.getSlug())
            + "_run-"
            + selected.getNumber()
            + "_"
            + kind
            + ".csv";
    audit.record(
        new AuditEvent(
            "report.export", workspaceId, context.userId(), "run:" + selected.getId(), details));
    return new CsvFile(name, csv.bytes());
  }

  /**
   * The data of the printable Plan + Run document (DECISIONS.md D-115). {@code sections} is a comma
   * separated subset of {@code history,trace}; null means both.
   */
  @Transactional(readOnly = true)
  public RunDocumentResponse document(
      Long workspaceId, Long runId, Integer fromRun, Integer toRun, String sections) {
    WorkspaceContext context = workspaceAccess.require(workspaceId);
    PlanRun selected = findRun(workspaceId, runId);
    Set<String> wanted = new HashSet<>();
    if (sections == null || sections.isBlank()) {
      wanted.add("history");
      wanted.add("trace");
    } else {
      for (String section : sections.split(",")) {
        wanted.add(section.trim().toLowerCase(Locale.ROOT));
      }
    }
    Range range = range(workspaceId, selected, fromRun, toRun);

    RunReportResponse summary = summary(workspaceId, runId);
    HistoryResponse history =
        wanted.contains("history")
            ? buildHistory(workspaceId, range.end(), range.window().size())
            : null;
    TraceabilityResponse trace = null;
    if (wanted.contains("trace") && selected.getRepositoryId() != null) {
      TraceabilityResponse all = traceability.trace(workspaceId, selected.getRepositoryId(), runId);
      List<TraceabilityResponse.Requirement> inScope =
          all.requirements().stream()
              .filter(r -> r.cases().stream().anyMatch(c -> c.status() != null))
              .toList();
      trace =
          new TraceabilityResponse(
              all.run(),
              all.repository(),
              all.jiraConfigured(),
              all.summary(),
              inScope,
              all.requirementSource());
    }

    audit.record(
        new AuditEvent(
            "report.generate",
            workspaceId,
            context.userId(),
            "run:" + selected.getId(),
            Map.of(
                "runId", String.valueOf(selected.getId()),
                "from", String.valueOf(range.window().get(0).getNumber()),
                "to", String.valueOf(range.end().getNumber()),
                "sections", String.join(",", wanted.stream().sorted().toList()),
                "format", "pdf")));
    return new RunDocumentResponse(
        new RunDocumentResponse.Range(range.window().get(0).getNumber(), range.end().getNumber()),
        summary,
        planAtRun(workspaceId, selected),
        history,
        trace,
        LocalDateTime.now());
  }

  /** The runs a document or tpdg export covers: the plan's runs from..to, to ending the window. */
  private record Range(PlanRun end, List<PlanRun> window) {}

  private Range range(Long workspaceId, PlanRun selected, Integer fromRun, Integer toRun) {
    List<PlanRun> planRuns =
        runs.findByWorkspaceIdAndPlanIdOrderByNumberDesc(workspaceId, selected.getPlanId());
    int toNumber = toRun == null ? selected.getNumber() : toRun;
    PlanRun end =
        planRuns.stream()
            .filter(r -> r.getNumber() == toNumber)
            .findFirst()
            .orElseThrow(
                () -> new NotFoundException("Run #" + toNumber + " is not a run of this plan"));
    if (fromRun != null && fromRun > toNumber) {
      throw new IllegalArgumentException("fromRun must not be after toRun");
    }
    List<PlanRun> inRange =
        planRuns.stream()
            .filter(r -> r.getNumber() <= toNumber && (fromRun == null || r.getNumber() >= fromRun))
            .limit(fromRun == null ? DEFAULT_WINDOW : MAX_WINDOW)
            .toList();
    List<PlanRun> oldestFirst = new ArrayList<>(inRange);
    Collections.reverse(oldestFirst);
    return new Range(end, oldestFirst);
  }

  private RunDocumentResponse.Plan planAtRun(Long workspaceId, PlanRun run) {
    TestPlan current = plans.findByIdAndWorkspaceId(run.getPlanId(), workspaceId).orElse(null);
    TestPlanVersion frozen =
        planVersions
            .findByPlanIdAndWorkspaceIdAndVersion(
                run.getPlanId(), workspaceId, run.getPlanVersion())
            .orElse(null);
    String name = frozen != null ? frozen.getName() : run.getPlanName();
    String scopeJson =
        frozen != null ? frozen.getScopeJson() : current == null ? null : current.getScopeJson();
    PlanScopeDto scope =
        scopeJson == null ? PlanScopeDto.empty() : json.readValue(scopeJson, PlanScopeDto.class);
    Long repositoryId = frozen != null ? frozen.getRepositoryId() : run.getRepositoryId();
    String repositoryName =
        repositoryId == null
            ? run.getRepositoryName()
            : repositories
                .findByIdAndWorkspaceId(repositoryId, workspaceId)
                .map(r -> r.getName())
                .orElse(run.getRepositoryName());
    return new RunDocumentResponse.Plan(
        run.getPlanId(),
        name,
        frozen != null ? frozen.getSlug() : current == null ? null : current.getSlug(),
        run.getPlanVersion(),
        frozen != null
            ? frozen.getDescription()
            : current == null ? null : current.getDescription(),
        frozen != null ? frozen.getJiraEpic() : current == null ? null : current.getJiraEpic(),
        repositoryName,
        scope,
        describeScope(scope));
  }

  /** The scope in one sentence, the same wording the Plans screen uses. */
  static String describeScope(PlanScopeDto scope) {
    List<String> parts = new ArrayList<>();
    if (scope.suites() != null && !scope.suites().isEmpty()) {
      parts.add("Suites " + String.join(", ", scope.suites()));
    }
    if (scope.tags() != null && !scope.tags().isEmpty()) {
      parts.add("Tags " + String.join(", ", scope.tags()));
    }
    if (scope.type() != null && !scope.type().equals("all")) {
      parts.add(scope.type().equals("manual") ? "Manual only" : "Automated only");
    }
    if (scope.includedCaseIds() != null && !scope.includedCaseIds().isEmpty()) {
      int n = scope.includedCaseIds().size();
      parts.add("+" + n + " added case" + (n == 1 ? "" : "s"));
    }
    return parts.isEmpty() ? "All cases of the repository" : String.join(" · ", parts);
  }

  /** One row per case and run of the range, after a block of "# key,value" metadata lines. */
  private void writeTpdg(CsvWriter csv, Long workspaceId, PlanRun selected, Range range) {
    Loaded data = load(workspaceId, range.end(), range.window().size());
    PassRate.Result rate = null;
    Map<Long, Integer> percentByRun = new HashMap<>();
    for (PlanRun run : range.window()) {
      Counts counts = Counts.of(data.casesByRun().get(run.getId()));
      PassRate.Result r = PassRate.of(counts.passed, counts.failed, counts.blocked, data.gate());
      percentByRun.put(run.getId(), r.displayPercent());
      if (run.getId().equals(selected.getId())) {
        rate = r;
      }
    }
    csv.row(List.of("# plan", selected.getPlanName()));
    csv.row(List.of("# plan_version", String.valueOf(selected.getPlanVersion())));
    csv.row(List.of("# run", "#" + selected.getNumber()));
    csv.row(
        List.of(
            "# range",
            "#" + range.window().get(0).getNumber() + " to #" + range.end().getNumber()));
    csv.row(List.of("# quality_gate_percent", String.valueOf(data.gate())));
    csv.row(
        List.of(
            "# run_pass_rate",
            rate == null || rate.displayPercent() == null ? "" : rate.displayPercent() + "%"));
    csv.row(List.of("# generated", LocalDateTime.now().toLocalDate().toString()));
    csv.row(
        List.of(
            "run_number",
            "run_date",
            "case_id",
            "title",
            "suite",
            "status",
            "source",
            "defect_keys",
            "run_pass_rate"));

    Map<Long, List<String>> defectKeys = new HashMap<>();
    for (Defect d : defects.findByWorkspaceId(workspaceId)) {
      defectKeys.computeIfAbsent(d.getRunCaseId(), k -> new ArrayList<>()).add(d.getIssueKey());
    }
    List<PlanRunCase> endCases = new ArrayList<>(data.casesByRun().get(range.end().getId()));
    endCases.sort(Comparator.comparing(ReportService::suite).thenComparing(ReportService::key));
    Map<String, Map<Long, PlanRunCase>> byKey = new HashMap<>();
    for (PlanRun run : range.window()) {
      for (PlanRunCase c : data.casesByRun().get(run.getId())) {
        byKey.computeIfAbsent(key(c), k -> new HashMap<>()).put(run.getId(), c);
      }
    }
    for (PlanRunCase base : endCases) {
      for (PlanRun run : range.window()) {
        PlanRunCase c = byKey.get(key(base)).get(run.getId());
        if (c == null) {
          continue;
        }
        Integer percent = percentByRun.get(run.getId());
        csv.row(
            List.of(
                String.valueOf(run.getNumber()),
                run.getCreatedAt().toLocalDate().toString(),
                nullToEmpty(c.getCaseId()),
                nullToEmpty(c.getTitle()),
                suite(c),
                c.getStatus().name(),
                c.getSource() == null ? "" : c.getSource().name(),
                String.join("; ", defectKeys.getOrDefault(c.getId(), List.of())),
                percent == null ? "" : String.valueOf(percent)));
      }
    }
  }

  // --- building blocks --------------------------------------------------------------------------

  HistoryResponse buildHistory(Long workspaceId, PlanRun selected, int limit) {
    Loaded data = load(workspaceId, selected, limit);
    List<PlanRun> window = data.window();
    Map<String, Map<Long, String>> statusByKey = statusByKey(data.casesByRun());

    List<HistoryResponse.RunColumn> columns = new ArrayList<>();
    for (PlanRun run : window) {
      Counts counts = Counts.of(data.casesByRun().get(run.getId()));
      PassRate.Result rate = PassRate.of(counts.passed, counts.failed, counts.blocked, data.gate());
      columns.add(
          new HistoryResponse.RunColumn(
              run.getId(),
              run.getNumber(),
              run.getCreatedAt(),
              run.getStatus().name(),
              rate.displayPercent(),
              rate.verdict(),
              run.getStatus() == PlanRun.Status.OPEN));
    }

    List<PlanRunCase> selectedCases = new ArrayList<>(data.casesByRun().get(selected.getId()));
    selectedCases.sort(
        Comparator.comparing(ReportService::suite).thenComparing(ReportService::key));
    List<HistoryResponse.Row> rows = new ArrayList<>();
    for (PlanRunCase c : selectedCases) {
      List<String> cells = cells(statusByKey.get(key(c)), window);
      StabilityClassifier.Result stability = StabilityClassifier.classify(cells);
      rows.add(
          new HistoryResponse.Row(
              key(c),
              c.getCaseId(),
              c.getTitle(),
              suite(c),
              cells,
              stability.flips(),
              stability.stability()));
    }
    return new HistoryResponse(new HistoryResponse.Gate(data.gate()), columns, rows);
  }

  private void writeSummary(CsvWriter csv, Loaded data) {
    PlanRun run = data.selected();
    List<PlanRunCase> cases = new ArrayList<>(data.casesByRun().get(run.getId()));
    Counts counts = Counts.of(cases);
    Integer percent =
        PassRate.of(counts.passed, counts.failed, counts.blocked, data.gate()).displayPercent();
    cases.sort(Comparator.comparing(ReportService::suite).thenComparing(ReportService::key));
    csv.row(
        List.of(
            "run_number",
            "case_id",
            "title",
            "suite",
            "status",
            "source",
            "result_date",
            "run_pass_rate"));
    for (PlanRunCase c : cases) {
      csv.row(
          List.of(
              String.valueOf(run.getNumber()),
              nullToEmpty(c.getCaseId()),
              nullToEmpty(c.getTitle()),
              suite(c),
              c.getStatus().name(),
              c.getSource() == null ? "" : c.getSource().name(),
              c.getRecordedAt() == null
                  ? ""
                  : c.getRecordedAt().toLocalDate().format(DateTimeFormatter.ISO_LOCAL_DATE),
              percent == null ? "" : String.valueOf(percent)));
    }
  }

  private void writeHeatmap(CsvWriter csv, HistoryResponse history) {
    List<String> header = new ArrayList<>(List.of("case_id", "title", "suite", "stability"));
    for (HistoryResponse.RunColumn run : history.runs()) {
      header.add(
          "#"
              + run.number()
              + " "
              + run.createdAt().toLocalDate().format(DateTimeFormatter.ISO_LOCAL_DATE));
    }
    csv.row(header);
    for (HistoryResponse.Row row : history.rows()) {
      List<String> line =
          new ArrayList<>(
              List.of(
                  nullToEmpty(row.caseId() == null ? row.key() : row.caseId()),
                  nullToEmpty(row.title()),
                  row.suite(),
                  row.stability()));
      row.cells().forEach(cell -> line.add(nullToEmpty(cell)));
      csv.row(line);
    }
    List<String> footer = new ArrayList<>(List.of("PASS RATE", "", "", ""));
    for (HistoryResponse.RunColumn run : history.runs()) {
      footer.add(run.passRate() == null ? "" : run.passRate() + "%");
    }
    csv.row(footer);
  }

  private PlanRun selectRun(Long workspaceId, Long planId, Long runId) {
    if (runId != null) {
      PlanRun run = findRun(workspaceId, runId);
      if (planId != null && !planId.equals(run.getPlanId())) {
        throw new IllegalArgumentException("The run does not belong to this plan");
      }
      return run;
    }
    List<PlanRun> candidates =
        planId == null
            ? runs.findByWorkspaceIdOrderByNumberDesc(workspaceId)
            : runs.findByWorkspaceIdAndPlanIdOrderByNumberDesc(workspaceId, planId);
    Set<Long> allowed = productAccess.allowedRepositoryIds(WorkspaceContextHolder.require());
    candidates =
        candidates.stream()
            .filter(r -> ProductAccess.contains(allowed, r.getRepositoryId()))
            .toList();
    if (candidates.isEmpty()) {
      throw new NotFoundException("There are no runs yet");
    }
    return candidates.get(0);
  }

  private Loaded load(Long workspaceId, PlanRun selected, int window) {
    int gate =
        settings
            .findById(workspaceId)
            .map(WorkspaceSettings::getQualityGatePercent)
            .orElse(WorkspaceSettings.DEFAULT_GATE);
    List<PlanRun> upTo =
        runs.findByWorkspaceIdAndPlanIdOrderByNumberDesc(workspaceId, selected.getPlanId()).stream()
            .filter(r -> r.getNumber() <= selected.getNumber())
            .limit(Math.max(window, WALK_BACK))
            .toList();
    List<PlanRun> windowRuns = new ArrayList<>(upTo.subList(0, Math.min(window, upTo.size())));
    Collections.reverse(windowRuns);
    List<PlanRun> earlier = upTo.stream().filter(r -> !r.getId().equals(selected.getId())).toList();

    Map<Long, List<PlanRunCase>> byRun = new HashMap<>();
    upTo.forEach(r -> byRun.put(r.getId(), new ArrayList<>()));
    for (PlanRunCase c : runCases.findByRunIdIn(upTo.stream().map(PlanRun::getId).toList())) {
      byRun.get(c.getRunId()).add(c);
    }
    TestPlan plan = plans.findByIdAndWorkspaceId(selected.getPlanId(), workspaceId).orElse(null);
    return new Loaded(selected, plan, gate, windowRuns, byRun, earlier);
  }

  /** key -> (runId -> status) over every loaded run. */
  private static Map<String, Map<Long, String>> statusByKey(
      Map<Long, List<PlanRunCase>> casesByRun) {
    Map<String, Map<Long, String>> map = new HashMap<>();
    casesByRun.forEach(
        (runId, cases) ->
            cases.forEach(
                c ->
                    map.computeIfAbsent(key(c), k -> new LinkedHashMap<>())
                        .put(runId, c.getStatus().name())));
    return map;
  }

  /** The case's status in each window run, oldest first; null where the case is absent. */
  private static List<String> cells(Map<Long, String> byRun, List<PlanRun> window) {
    List<String> out = new ArrayList<>();
    for (PlanRun run : window) {
      out.add(byRun == null ? null : byRun.get(run.getId()));
    }
    return out;
  }

  private static FailingSince.RunPoint point(PlanRun run) {
    return new FailingSince.RunPoint(run.getId(), run.getNumber(), run.getCreatedAt());
  }

  private static String key(PlanRunCase c) {
    return c.getCaseId() != null && !c.getCaseId().isBlank() ? c.getCaseId() : c.getFilePath();
  }

  private static String suite(PlanRunCase c) {
    return c.getFolder() == null ? "" : c.getFolder();
  }

  private static String nullToEmpty(String value) {
    return value == null ? "" : value;
  }

  private static int windowSize(Integer limit) {
    if (limit == null) {
      return DEFAULT_WINDOW;
    }
    return Math.max(1, Math.min(MAX_WINDOW, limit));
  }

  private PlanRun findRun(Long workspaceId, Long runId) {
    PlanRun run =
        runs.findByIdAndWorkspaceId(runId, workspaceId)
            .orElseThrow(() -> new NotFoundException("Run not found"));
    productAccess.require(WorkspaceContextHolder.require(), run.getRepositoryId(), "Run not found");
    return run;
  }

  private String userName(Long userId) {
    if (userId == null) {
      return null;
    }
    return users
        .findById(userId)
        .map(
            (User u) -> {
              String full = (u.getFirstName() + " " + u.getLastName()).trim();
              return full.isEmpty() ? u.getEmail() : full;
            })
        .orElse(null);
  }

  /** Counts per status of one run's cases. */
  private static final class Counts {
    int total;
    int passed;
    int failed;
    int blocked;
    int skipped;
    int untested;

    static Counts of(List<PlanRunCase> cases) {
      Counts counts = new Counts();
      for (PlanRunCase c : cases) {
        counts.total++;
        switch (c.getStatus()) {
          case PASSED -> counts.passed++;
          case FAILED -> counts.failed++;
          case BLOCKED -> counts.blocked++;
          case SKIPPED -> counts.skipped++;
          case UNTESTED -> counts.untested++;
        }
      }
      return counts;
    }
  }

  /** The run's defects by run case, newest first (ORCH-023). */
  private Map<Long, List<DefectResponse>> defectsOf(Long runId) {
    Map<Long, List<DefectResponse>> byCase = new HashMap<>();
    defects.findByRunId(runId).stream()
        .sorted(Comparator.comparing(Defect::getId).reversed())
        .forEach(
            d ->
                byCase
                    .computeIfAbsent(d.getRunCaseId(), k -> new ArrayList<>())
                    .add(DefectResponse.of(d)));
    return byCase;
  }
}
