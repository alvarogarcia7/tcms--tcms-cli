package ae.tii.java.base.service;

import ae.tii.java.base.audit.AuditEvent;
import ae.tii.java.base.audit.AuditRecorder;
import ae.tii.java.base.common.NotFoundException;
import ae.tii.java.base.dto.JiraStatusResponse;
import ae.tii.java.base.dto.TraceabilityResponse;
import ae.tii.java.base.dto.TraceabilityResponse.LinkedCase;
import ae.tii.java.base.dto.TraceabilityResponse.LinkedDefect;
import ae.tii.java.base.jira.ProductJiraLookup;
import ae.tii.java.base.jira.ProductJiraSettings;
import ae.tii.java.base.model.ConnectedRepository;
import ae.tii.java.base.model.Defect;
import ae.tii.java.base.model.PlanRun;
import ae.tii.java.base.model.PlanRunCase;
import ae.tii.java.base.model.RepositoryCase;
import ae.tii.java.base.model.Requirement;
import ae.tii.java.base.ref.RefResolver;
import ae.tii.java.base.report.CsvWriter;
import ae.tii.java.base.repository.ConnectedRepositoryRepository;
import ae.tii.java.base.repository.DefectRepository;
import ae.tii.java.base.repository.JiraConfigRepository;
import ae.tii.java.base.repository.PlanRunCaseRepository;
import ae.tii.java.base.repository.PlanRunRepository;
import ae.tii.java.base.repository.RepositoryCaseRepository;
import ae.tii.java.base.repository.RequirementRepository;
import ae.tii.java.base.security.ProductAccess;
import ae.tii.java.base.security.WorkspaceAccess;
import ae.tii.java.base.security.WorkspaceContext;
import ae.tii.java.base.security.WorkspaceContextHolder;
import ae.tii.java.base.trace.CoverageClassifier;
import ae.tii.java.base.trace.RequirementKeys;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import lombok.RequiredArgsConstructor;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import tools.jackson.core.JacksonException;
import tools.jackson.core.type.TypeReference;
import tools.jackson.databind.ObjectMapper;

/**
 * Requirement traceability (ORCH-018, DECISIONS.md D-116): which requirements the cases of a test
 * repository cite and how a run covers them. Requirements come from the cases' own YAML; titles and
 * statuses from Jira when it is connected. Any member may read.
 */
@Service
@RequiredArgsConstructor
public class TraceabilityService {

  private static final int WALK_BACK = 50;
  private static final Pattern NUMBERED = Pattern.compile("^(.*?)-(\\d{1,17})$");
  private static final Comparator<String> KEY_ORDER =
      Comparator.comparing(TraceabilityService::keyPrefix)
          .thenComparingLong(TraceabilityService::keyNumber)
          .thenComparing(Comparator.naturalOrder());

  private static String keyPrefix(String key) {
    Matcher m = NUMBERED.matcher(key);
    return m.matches() ? m.group(1) : key;
  }

  private static long keyNumber(String key) {
    Matcher m = NUMBERED.matcher(key);
    return m.matches() ? Long.parseLong(m.group(2)) : -1L;
  }

  private final ConnectedRepositoryRepository repositories;
  private final RepositoryCaseRepository repositoryCases;
  private final PlanRunRepository runs;
  private final PlanRunCaseRepository runCases;
  private final RequirementRepository requirements;
  private final DefectRepository defects;
  private final JiraConfigRepository jira;
  private final WorkspaceAccess workspaceAccess;
  private final AuditRecorder audit;
  private final ObjectMapper json;
  private final ProductAccess productAccess;
  private final ProductJiraLookup productConfigs;
  private final JiraWriteBackService writeBack;

  /** A generated CSV: its name and bytes. */
  public record CsvFile(String fileName, byte[] content) {}

  @Transactional(readOnly = true)
  public TraceabilityResponse trace(Long workspaceId, Long repositoryId, Long runId) {
    workspaceAccess.require(workspaceId);
    return build(workspaceId, repositoryId, runId);
  }

  @Transactional(readOnly = true)
  public CsvFile exportCsv(Long workspaceId, Long repositoryId, Long runId) {
    WorkspaceContext context = workspaceAccess.require(workspaceId);
    TraceabilityResponse report = build(workspaceId, repositoryId, runId);
    CsvWriter csv = new CsvWriter();
    csv.row(
        List.of(
            "requirement",
            "title",
            "status",
            "coverage",
            "executed",
            "linked",
            "cases",
            "failing_cases",
            "defects"));
    for (TraceabilityResponse.Requirement r : report.requirements()) {
      List<String> all = new ArrayList<>();
      List<String> failing = new ArrayList<>();
      for (LinkedCase c : r.cases()) {
        String name = c.caseId() != null ? c.caseId() : c.file();
        all.add(name);
        if ("FAILED".equals(c.status())) {
          failing.add(name);
        }
      }
      csv.row(
          List.of(
              r.key(),
              nullToEmpty(r.title()),
              nullToEmpty(r.status()),
              r.coverage(),
              String.valueOf(r.executed()),
              String.valueOf(r.linked()),
              String.join("; ", all),
              String.join("; ", failing),
              String.join(
                  "; ", r.defects().stream().map(LinkedDefect::issueKey).distinct().toList())));
    }
    audit.record(
        new AuditEvent(
            "report.export",
            workspaceId,
            context.userId(),
            "repository:" + report.repository().id(),
            Map.of(
                "layout",
                "traceability",
                "format",
                "csv",
                "runId",
                report.run() == null ? "" : String.valueOf(report.run().id()))));
    String name =
        "traceability_"
            + report.repository().name().replaceAll("[^A-Za-z0-9._-]+", "-")
            + (report.run() == null ? "" : "_run-" + report.run().number())
            + ".csv";
    return new CsvFile(name, csv.bytes());
  }

  private TraceabilityResponse build(Long workspaceId, Long repositoryId, Long runId) {
    PlanRun run = null;
    if (runId != null) {
      run =
          runs.findByIdAndWorkspaceId(runId, workspaceId)
              .orElseThrow(() -> new NotFoundException("Run not found"));
      productAccess.require(
          WorkspaceContextHolder.require(), run.getRepositoryId(), "Run not found");
      if (repositoryId != null
          && run.getRepositoryId() != null
          && !repositoryId.equals(run.getRepositoryId())) {
        throw new IllegalArgumentException("The run belongs to another test repository");
      }
      if (repositoryId == null) {
        repositoryId = run.getRepositoryId();
      }
    }
    ConnectedRepository repository = pickRepository(workspaceId, repositoryId);
    if (run == null) {
      Long repoId = repository.getId();
      run =
          runs.findByWorkspaceIdOrderByNumberDesc(workspaceId).stream()
              .filter(r -> repoId.equals(r.getRepositoryId()))
              .findFirst()
              .orElse(null);
    }

    // The cases and the requirements they cite, over every test repository of the product
    // (ORCH-067, D-818): the selected one first, then the others that joined its product.
    List<ConnectedRepository> scope = productScope(workspaceId, repository);
    List<RepositoryCase> scopeCases = new ArrayList<>();
    for (ConnectedRepository member : scope) {
      scopeCases.addAll(repositoryCases.findByRepositoryIdOrderByFilePath(member.getId()));
    }
    Map<String, List<RepositoryCase>> casesByKey = new TreeMap<>(KEY_ORDER);
    for (RepositoryCase row : scopeCases) {
      if (!row.isValid()) {
        continue;
      }
      Set<String> keys = new LinkedHashSet<>();
      for (String raw : readList(row.getRequirements())) {
        String key = RequirementKeys.normalize(raw);
        if (key != null) {
          keys.add(key);
        }
      }
      keys.forEach(key -> casesByKey.computeIfAbsent(key, k -> new ArrayList<>()).add(row));
    }
    Map<String, Requirement> known = new HashMap<>();
    for (ConnectedRepository member : scope) {
      for (Requirement r : requirements.findByRepositoryIdOrderByIssueKey(member.getId())) {
        known.putIfAbsent(r.getIssueKey(), r);
        casesByKey.computeIfAbsent(r.getIssueKey(), k -> new ArrayList<>());
      }
    }
    boolean jiraConfigured =
        jira.findById(workspaceId).filter(c -> "VERIFIED".equals(c.getStatus())).isPresent();
    // The product's one requirement source and the shared id resolver (ORCH-023.25, D-1112/D-1113).
    Long productId = repository.productKey();
    ProductJiraSettings productSettings =
        productConfigs.settings(workspaceId, productId).orElse(null);
    boolean strictdoc = productSettings != null && productSettings.strictdocSource();
    RefResolver resolver = productConfigs.resolver(workspaceId, productId);
    Map<String, JiraStatusResponse.Ref> written = new HashMap<>();
    if (productSettings != null) {
      for (JiraStatusResponse.Ref f : writeBack.statusOf(workspaceId, productId).features()) {
        written.put(f.ref(), f);
      }
    }

    // The run's results, by case id else file path.
    List<PlanRunCase> runCaseList =
        run == null ? List.of() : runCases.findByRunIdOrderById(run.getId());
    Map<String, PlanRunCase> byCaseId = new HashMap<>();
    Map<String, PlanRunCase> byFile = new HashMap<>();
    for (PlanRunCase c : runCaseList) {
      // A case is named in its repository (D-817); runs from before ORCH-067 have none.
      String in = c.getRepositoryId() == null ? "" : c.getRepositoryId() + "\n";
      if (c.getCaseId() != null && !c.getCaseId().isBlank()) {
        byCaseId.put(in + c.getCaseId(), c);
      }
      byFile.put(in + c.getFilePath(), c);
    }

    // Defects: on the linked cases in this run, and earlier in a failing streak.
    Map<Long, List<Defect>> defectsByRunCase = new HashMap<>();
    List<PlanRun> earlier = new ArrayList<>();
    Map<Long, List<PlanRunCase>> earlierCases = new HashMap<>();
    if (run != null) {
      for (Defect d : defects.findByWorkspaceId(workspaceId)) {
        defectsByRunCase.computeIfAbsent(d.getRunCaseId(), k -> new ArrayList<>()).add(d);
      }
      PlanRun selected = run;
      earlier =
          runs
              .findByWorkspaceIdAndPlanIdOrderByNumberDesc(workspaceId, selected.getPlanId())
              .stream()
              .filter(r -> r.getNumber() < selected.getNumber())
              .limit(WALK_BACK)
              .toList();
      if (!earlier.isEmpty() && !defectsByRunCase.isEmpty()) {
        for (PlanRunCase c :
            runCases.findByRunIdIn(earlier.stream().map(PlanRun::getId).toList())) {
          earlierCases.computeIfAbsent(c.getRunId(), k -> new ArrayList<>()).add(c);
        }
      }
    }

    List<TraceabilityResponse.Requirement> result = new ArrayList<>();
    Set<String> openKeys = new LinkedHashSet<>();
    int covered = 0;
    int notRun = 0;
    int failing = 0;
    int noCases = 0;
    int withDefect = 0;
    for (Map.Entry<String, List<RepositoryCase>> entry : casesByKey.entrySet()) {
      List<LinkedCase> linked = new ArrayList<>();
      List<String> statuses = new ArrayList<>();
      Map<String, LinkedDefect> defectList = new LinkedHashMap<>();
      for (RepositoryCase row : entry.getValue()) {
        PlanRunCase inRun = null;
        String in = row.getRepositoryId() + "\n";
        if (row.getCaseId() != null) {
          inRun = byCaseId.getOrDefault(in + row.getCaseId(), byCaseId.get(row.getCaseId()));
        }
        if (inRun == null) {
          inRun = byFile.getOrDefault(in + row.getFilePath(), byFile.get(row.getFilePath()));
        }
        String status = inRun == null ? null : inRun.getStatus().name();
        statuses.add(status);
        linked.add(
            new LinkedCase(
                row.getCaseId(), row.getRepositoryId(), row.getFilePath(), row.getTitle(), status));
        if (inRun != null) {
          for (Defect d : defectsByRunCase.getOrDefault(inRun.getId(), List.of())) {
            defectList.putIfAbsent(d.getIssueKey(), toDefect(d, row.getCaseId(), null));
          }
          if (inRun.getStatus() == PlanRunCase.Status.FAILED) {
            streakDefects(
                inRun, earlier, earlierCases, defectsByRunCase, row.getCaseId(), defectList);
          }
        }
      }
      CoverageClassifier.Result classified = CoverageClassifier.classify(statuses);
      switch (classified.coverage()) {
        case CoverageClassifier.COVERED -> covered++;
        case CoverageClassifier.FAILING -> failing++;
        case CoverageClassifier.NO_CASES -> noCases++;
        default -> notRun++;
      }
      if (!defectList.isEmpty()) {
        withDefect++;
      }
      defectList.values().stream()
          .filter(LinkedDefect::open)
          .forEach(d -> openKeys.add(d.issueKey()));
      Requirement info =
          strictdoc ? null : known.get(entry.getKey().toUpperCase(java.util.Locale.ROOT));
      RefResolver.Resolved resolved = resolver.resolve(entry.getKey()).orElse(null);
      JiraStatusResponse.Ref jiraRef = written.get(entry.getKey());
      result.add(
          new TraceabilityResponse.Requirement(
              entry.getKey(),
              info == null ? null : info.getTitle(),
              info == null ? null : info.getStatus(),
              info != null && info.getUrl() != null
                  ? info.getUrl()
                  : resolved == null ? null : resolved.url(),
              jiraConfigured && !strictdoc && info == null,
              classified.coverage(),
              classified.executed(),
              classified.total(),
              linked,
              new ArrayList<>(defectList.values()),
              resolved == null ? null : resolved.system(),
              jiraRef == null
                  ? null
                  : new TraceabilityResponse.JiraWrite(
                      jiraRef.values().stream()
                          .map(v -> new TraceabilityResponse.Value(v.label(), v.value()))
                          .toList(),
                      jiraRef.outcome(),
                      jiraRef.reason())));
    }

    return new TraceabilityResponse(
        run == null
            ? null
            : new TraceabilityResponse.Run(
                run.getId(),
                run.getNumber(),
                run.getPlanId(),
                run.getPlanName(),
                run.getStatus().name()),
        new TraceabilityResponse.Repo(repository.getId(), repository.getName()),
        jiraConfigured,
        new TraceabilityResponse.Summary(
            result.size(), covered, notRun, failing, noCases, withDefect, openKeys.size()),
        result,
        productSettings == null ? null : productSettings.requirementSource());
  }

  /**
   * Defects of the same case in earlier runs of the failing streak: Failed extends it, Passed ends
   * it, anything else is skipped (DECISIONS.md D-98).
   */
  private void streakDefects(
      PlanRunCase current,
      List<PlanRun> earlierNewestFirst,
      Map<Long, List<PlanRunCase>> earlierCases,
      Map<Long, List<Defect>> defectsByRunCase,
      String caseId,
      Map<String, LinkedDefect> out) {
    for (PlanRun run : earlierNewestFirst) {
      PlanRunCase before = null;
      for (PlanRunCase c : earlierCases.getOrDefault(run.getId(), List.of())) {
        boolean same =
            current.getCaseId() != null
                ? current.getCaseId().equals(c.getCaseId())
                : current.getFilePath().equals(c.getFilePath());
        if (same) {
          before = c;
          break;
        }
      }
      if (before == null) {
        continue;
      }
      if (before.getStatus() == PlanRunCase.Status.PASSED) {
        return;
      }
      if (before.getStatus() == PlanRunCase.Status.FAILED) {
        for (Defect d : defectsByRunCase.getOrDefault(before.getId(), List.of())) {
          out.putIfAbsent(d.getIssueKey(), toDefect(d, caseId, run.getNumber()));
        }
      }
    }
  }

  private static LinkedDefect toDefect(Defect d, String caseId, Integer fromRun) {
    return new LinkedDefect(
        d.getIssueKey(),
        d.getUrl(),
        d.getSummary(),
        d.getStatus(),
        d.getStatusCategory(),
        !"done".equalsIgnoreCase(d.getStatusCategory()),
        caseId,
        fromRun);
  }

  /** The test repositories of the repository's product: itself first, then its members. */
  private List<ConnectedRepository> productScope(Long workspaceId, ConnectedRepository repository) {
    List<ConnectedRepository> scope = new ArrayList<>();
    scope.add(repository);
    Long product = repository.productKey();
    repositories.findByWorkspaceIdOrderByKindAscNameAsc(workspaceId).stream()
        .filter(r -> r.isTest() && !r.getId().equals(repository.getId()))
        .filter(r -> product.equals(r.productKey()))
        .filter(r -> productAccess.allows(WorkspaceContextHolder.require(), r.getId()))
        .forEach(scope::add);
    return scope;
  }

  private ConnectedRepository pickRepository(Long workspaceId, Long repositoryId) {
    if (repositoryId != null) {
      return repositories
          .findByIdAndWorkspaceId(repositoryId, workspaceId)
          .filter(r -> r.isTest())
          .filter(r -> productAccess.allows(WorkspaceContextHolder.require(), r.getId()))
          .orElseThrow(() -> new NotFoundException("Repository not found"));
    }
    List<ConnectedRepository> tests =
        repositories.findByWorkspaceIdOrderByKindAscNameAsc(workspaceId).stream()
            .filter(r -> r.isTest())
            .filter(r -> productAccess.allows(WorkspaceContextHolder.require(), r.getId()))
            .toList();
    if (tests.isEmpty()) {
      throw new NotFoundException("No test repository is connected");
    }
    return tests.get(0);
  }

  private List<String> readList(String text) {
    if (text == null || text.isBlank()) {
      return List.of();
    }
    try {
      return json.readValue(text, new TypeReference<List<String>>() {});
    } catch (JacksonException e) {
      return List.of();
    }
  }

  private static String nullToEmpty(String value) {
    return value == null ? "" : value;
  }
}
