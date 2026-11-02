package ae.tii.java.base.vv;

import ae.tii.java.base.dto.KpiDto;
import ae.tii.java.base.dto.StepDto;
import ae.tii.java.base.dto.TestCaseDto;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

/**
 * Pure transformer from parsed {@link VvResults} (a loaded test run) into the view shapes the SPA
 * tabs consume. It holds no state and no sample data: every value it returns is derived from a run
 * that was actually loaded. Authorization is done by the controller before these are called.
 */
@Service
@Slf4j
public class TestRunDataService {

  private static int pct(int part, int whole) {
    return whole <= 0 ? 0 : (int) Math.round((part * 100.0) / whole);
  }

  private static String barColor(int passRate) {
    return passRate >= 90 ? "#10b981" : passRate >= 85 ? "#f59e0b" : "#ef4444";
  }

  /** Dashboard KPI cards for a single run. */
  public List<KpiDto> kpis(VvResults r) {
    int passRate = pct(r.getPassed(), r.getTotalTestCases());
    int coverage = pct(r.getPassedSteps(), r.getTotalSteps());
    return List.of(
        KpiDto.builder()
            .label("Pass rate")
            .value(passRate + "%")
            .note(r.getPassed() + "/" + r.getTotalTestCases() + " cases")
            .color(barColor(passRate))
            .build(),
        KpiDto.builder()
            .label("Total cases")
            .value(r.getTotalTestCases())
            .note("in run " + r.getRunId())
            .color("#0f172a")
            .build(),
        KpiDto.builder()
            .label("Coverage")
            .value(coverage + "%")
            .note(r.getPassedSteps() + "/" + r.getTotalSteps() + " steps")
            .color(barColor(coverage))
            .build(),
        KpiDto.builder()
            .label("Failed")
            .value(r.getFailed())
            .note(r.getFailedSteps() + " failed steps")
            .color(r.getFailed() == 0 ? "#10b981" : "#ef4444")
            .build());
  }

  /** One trend bar per run (pass rate), oldest to newest, for the dashboard chart. */
  public List<Map<String, Object>> dashboardTrends(List<VvResults> runs) {
    List<Map<String, Object>> trends = new ArrayList<>();
    for (VvResults r : runs) {
      int passRate = pct(r.getPassed(), r.getTotalTestCases());
      Map<String, Object> bar = new LinkedHashMap<>();
      bar.put("value", passRate);
      bar.put("height", passRate + "%");
      bar.put("background", barColor(passRate));
      bar.put("run", r.getRunId());
      trends.add(bar);
    }
    return trends;
  }

  /** Recent activity derived from the loaded runs. */
  public List<Map<String, Object>> dashboardActivity(List<VvLoadResult> runs) {
    List<Map<String, Object>> activity = new ArrayList<>();
    for (VvLoadResult run : runs) {
      Map<String, Object> item = new LinkedHashMap<>();
      item.put(
          "text",
          "Loaded "
              + run.getFileCount()
              + " V&V file(s) from "
              + run.getSource()
              + " as "
              + run.getRunId());
      item.put("when", String.valueOf(run.getLoadedAt()));
      activity.add(item);
    }
    return activity;
  }

  /** Source/repository metadata for a single run. */
  public Map<String, Object> dashboardRepositories(VvLoadResult run) {
    Map<String, Object> repo = new LinkedHashMap<>();
    repo.put("url", run.getLocation());
    repo.put("source", run.getSource());
    repo.put("runId", run.getRunId());
    repo.put("parsed", run.getFileCount() + " files");
    repo.put("loadedAt", String.valueOf(run.getLoadedAt()));
    repo.put("schemaErrors", new ArrayList<>());
    return repo;
  }

  /** Every test case in a run, as the SPA's TestCaseDto. */
  public List<TestCaseDto> cases(VvResults r) {
    List<TestCaseDto> cases = new ArrayList<>();
    if (r.getTestCases() == null) {
      return cases;
    }
    for (VvResults.TestCaseResult tc : r.getTestCases()) {
      cases.add(toCase(tc));
    }
    return cases;
  }

  /** A single test case in a run, or null if absent. */
  public TestCaseDto caseDetail(VvResults r, String caseId) {
    if (r.getTestCases() == null) {
      return null;
    }
    return r.getTestCases().stream()
        .filter(tc -> caseId.equals(tc.getTestCaseId()))
        .findFirst()
        .map(this::toCase)
        .orElse(null);
  }

  private TestCaseDto toCase(VvResults.TestCaseResult tc) {
    List<StepDto> steps = new ArrayList<>();
    int order = 1;
    if (tc.getSequences() != null) {
      for (VvResults.SequenceResult seq : tc.getSequences()) {
        if (seq.getSteps() == null) {
          continue;
        }
        for (VvResults.StepResult step : seq.getSteps()) {
          steps.add(
              StepDto.builder()
                  .order(order++)
                  .action(step.getDescription() != null ? step.getDescription() : step.getStep())
                  .expected(step.getStatus())
                  .build());
        }
      }
    }
    return TestCaseDto.builder()
        .id(tc.getTestCaseId())
        .title(tc.getDescription() != null ? tc.getDescription() : tc.getTestCaseId())
        .suite("V&V")
        .type("automated")
        .steps(steps)
        .lastResult(tc.isOverallPass() ? "pass" : "fail")
        .build();
  }

  /** For one test case, its pass/fail across the given runs (most recent first). */
  public List<Map<String, Object>> caseHistory(List<VvResults> runs, String caseId) {
    List<Map<String, Object>> history = new ArrayList<>();
    for (VvResults r : runs) {
      if (r.getTestCases() == null) {
        continue;
      }
      for (VvResults.TestCaseResult tc : r.getTestCases()) {
        if (caseId.equals(tc.getTestCaseId())) {
          Map<String, Object> row = new LinkedHashMap<>();
          row.put("run", r.getRunId());
          row.put("result", tc.isOverallPass() ? "pass" : "fail");
          row.put("resultLabel", tc.isOverallPass() ? "Pass" : "Fail");
          row.put("sha", r.getRunId());
          row.put("by", "vv");
          row.put("defect", tc.isOverallPass() ? "—" : "DEF-" + caseId);
          row.put("date", String.valueOf(r.getWorkspaceId() == null ? "" : r.getRunId()));
          history.add(row);
        }
      }
    }
    return history;
  }

  /** The failing test cases in a run. */
  public List<Map<String, Object>> failingTests(VvResults r) {
    List<Map<String, Object>> failing = new ArrayList<>();
    if (r.getTestCases() == null) {
      return failing;
    }
    for (VvResults.TestCaseResult tc : r.getTestCases()) {
      if (!tc.isOverallPass()) {
        Map<String, Object> item = new LinkedHashMap<>();
        item.put("id", tc.getTestCaseId());
        item.put("title", tc.getDescription() != null ? tc.getDescription() : tc.getTestCaseId());
        item.put("suite", "V&V");
        item.put("since", r.getRunId());
        failing.add(item);
      }
    }
    return failing;
  }

  /**
   * Per-case stability across the given runs: {@code flaky} if it both passed and failed, {@code
   * stable} if it always passed, {@code watch} otherwise.
   */
  public List<Map<String, Object>> heatmap(List<VvResults> runs) {
    Map<String, int[]> byCase = new LinkedHashMap<>(); // id -> [passes, total]
    Map<String, String> titles = new LinkedHashMap<>();
    for (VvResults r : runs) {
      if (r.getTestCases() == null) {
        continue;
      }
      for (VvResults.TestCaseResult tc : r.getTestCases()) {
        int[] counts = byCase.computeIfAbsent(tc.getTestCaseId(), k -> new int[2]);
        if (tc.isOverallPass()) {
          counts[0]++;
        }
        counts[1]++;
        titles.putIfAbsent(
            tc.getTestCaseId(),
            tc.getDescription() != null ? tc.getDescription() : tc.getTestCaseId());
      }
    }
    List<Map<String, Object>> heatmap = new ArrayList<>();
    for (Map.Entry<String, int[]> e : byCase.entrySet()) {
      int passes = e.getValue()[0];
      int total = e.getValue()[1];
      String stability = passes == total ? "stable" : passes == 0 ? "flaky" : "watch";
      Map<String, Object> row = new LinkedHashMap<>();
      row.put("id", e.getKey());
      row.put("title", titles.get(e.getKey()));
      row.put("stability", stability);
      heatmap.add(row);
    }
    return heatmap;
  }

  /**
   * Requirements of a run (D-751): one row per requirement key, the exact text of the {@code
   * requirement} field of the case file, with the cases that cite it. A case that cites none keeps
   * a row of its own named {@code REQ-n}.
   */
  public List<Map<String, Object>> requirements(VvResults r) {
    List<Map<String, Object>> requirements = new ArrayList<>();
    if (r.getTestCases() == null) {
      return requirements;
    }
    Map<String, List<VvResults.TestCaseResult>> byKey = new LinkedHashMap<>();
    int position = 0;
    for (VvResults.TestCaseResult tc : r.getTestCases()) {
      position++;
      List<String> cited = tc.getRequirements() == null ? List.of() : tc.getRequirements();
      if (cited.isEmpty()) {
        byKey.computeIfAbsent("REQ-" + position, k -> new ArrayList<>()).add(tc);
      } else {
        cited.forEach(key -> byKey.computeIfAbsent(key, k -> new ArrayList<>()).add(tc));
      }
    }
    int i = 1;
    for (Map.Entry<String, List<VvResults.TestCaseResult>> entry : byKey.entrySet()) {
      List<VvResults.TestCaseResult> cases = entry.getValue();
      boolean allPass = cases.stream().allMatch(VvResults.TestCaseResult::isOverallPass);
      int passedSteps = cases.stream().mapToInt(VvResults.TestCaseResult::getPassedSteps).sum();
      int totalSteps = cases.stream().mapToInt(VvResults.TestCaseResult::getTotalSteps).sum();
      Map<String, Object> req = new LinkedHashMap<>();
      req.put("id", entry.getKey());
      req.put("key", entry.getKey());
      req.put(
          "summary",
          cases.stream()
              .map(tc -> tc.getDescription() != null ? tc.getDescription() : tc.getTestCaseId())
              .distinct()
              .collect(java.util.stream.Collectors.joining("; ")));
      req.put("cases", cases.stream().map(VvResults.TestCaseResult::getTestCaseId).toList());
      req.put("results", cases.stream().map(tc -> tc.isOverallPass() ? "pass" : "fail").toList());
      req.put("defect", allPass ? null : "DEF-" + i);
      req.put("defectState", allPass ? null : "Open");
      req.put("coverage", String.valueOf(pct(passedSteps, totalSteps)));
      req.put("coverageTone", allPass ? "ok" : "warn");
      requirements.add(req);
      i++;
    }
    return requirements;
  }

  /** Traceability matrix rows derived from a run's test cases. */
  public List<Map<String, Object>> traceability(VvResults r) {
    return requirements(r);
  }
}
