package ae.tii.java.base.vv;

import static org.assertj.core.api.Assertions.assertThat;

import ae.tii.java.base.dto.KpiDto;
import ae.tii.java.base.dto.TestCaseDto;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * Exercises the whole real data path: load the bundled {@code vv-sample-results} folder, parse it
 * with the real {@link VvResultsParser}, and transform it with {@link TestRunDataService}. Proves
 * the derived views are computed from the loaded run, with no sample/default data anywhere.
 */
@DisplayName("Test run views (from real parsed V&V data)")
class TestRunDataServiceTest {

  private static final Long WS = 1L;

  private VvProperties props(Path cloneDir) {
    VvProperties p = new VvProperties();
    p.getGit().setCloneDir(cloneDir.toString());
    return p;
  }

  private VvResults loadSample(Path cloneDir) {
    VvDataLoaderService loader = new VvDataLoaderService(props(cloneDir), new VvResultsParser());
    VvLoadResult run = loader.loadFromFolder(WS, "vv-sample-results");
    return loader.parseResults(WS, run.getRunId());
  }

  @Test
  @DisplayName("parses the sample results container into two cases, one failing")
  void parsesSample(@TempDir Path cloneDir) {
    VvResults r = loadSample(cloneDir);

    assertThat(r.getTotalTestCases()).isEqualTo(2);
    assertThat(r.getPassed()).isEqualTo(1);
    assertThat(r.getFailed()).isEqualTo(1);
    assertThat(r.getTotalSteps()).isEqualTo(4);
    assertThat(r.getPassedSteps()).isEqualTo(3);
    assertThat(r.getWorkspaceId()).isEqualTo(WS);
  }

  @Test
  @DisplayName("KPIs are derived from the run (pass rate 50%, coverage 75%)")
  void kpis(@TempDir Path cloneDir) {
    List<KpiDto> kpis = new TestRunDataService().kpis(loadSample(cloneDir));

    assertThat(kpis).extracting(KpiDto::getLabel).contains("Pass rate", "Total cases", "Coverage");
    assertThat(kpis.get(0).getValue()).isEqualTo("50%");
    assertThat(kpis.get(2).getValue()).isEqualTo("75%");
  }

  @Test
  @DisplayName("cases, failing tests and requirements come from the run")
  void views(@TempDir Path cloneDir) {
    VvResults r = loadSample(cloneDir);
    TestRunDataService view = new TestRunDataService();

    List<TestCaseDto> cases = view.cases(r);
    assertThat(cases)
        .extracting(TestCaseDto::getId)
        .containsExactly("TC_SMOKE_001", "TC_SMOKE_002");

    List<Map<String, Object>> failing = view.failingTests(r);
    assertThat(failing).hasSize(1);
    assertThat(failing.get(0).get("id")).isEqualTo("TC_SMOKE_002");

    List<Map<String, Object>> requirements = view.requirements(r);
    assertThat(requirements).hasSize(2);

    List<Map<String, Object>> heatmap = view.heatmap(List.of(r));
    assertThat(heatmap).hasSize(2);
  }
}
