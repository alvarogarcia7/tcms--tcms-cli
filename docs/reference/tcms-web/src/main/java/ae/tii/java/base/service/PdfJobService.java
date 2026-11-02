package ae.tii.java.base.service;

import ae.tii.java.base.common.ConflictException;
import ae.tii.java.base.common.NotFoundException;
import ae.tii.java.base.dto.RunDocumentResponse;
import ae.tii.java.base.dto.RunReportResponse;
import ae.tii.java.base.security.WorkspaceAccess;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.springframework.stereotype.Service;
import org.yaml.snakeyaml.DumperOptions;
import org.yaml.snakeyaml.Yaml;

/**
 * "Generate a PDF" (ORCH-017.13, D-885): a run takes minutes, so it is a job the page polls. The
 * job turns the run document into YAML, renders AsciiDoc with the installed tpdg and a PDF with
 * YODA. Jobs live in memory (a restart forgets them; the user generates again), at most 50 are
 * kept, and a job is only visible to the workspace that started it.
 */
@Service
public class PdfJobService {

  public enum State {
    RUNNING,
    DONE,
    ERROR
  }

  /** A snapshot of one job for the API. */
  public record JobView(
      String id, State state, String step, String message, String fileName, long startedAtMillis) {}

  private static final int KEEP = 50;
  private static final String SCHEMA = "/tpdg/run-document.schema.json";
  private static final String TEMPLATE = "/tpdg/run-document.adoc.j2";

  private static final class Job {
    final String id = UUID.randomUUID().toString();
    final Long workspaceId;
    final long startedAt = System.currentTimeMillis();
    volatile State state = State.RUNNING;
    volatile String step = "Preparing the document";
    volatile String message;
    volatile String fileName;
    volatile Path dir;

    Job(Long workspaceId) {
      this.workspaceId = workspaceId;
    }

    JobView view() {
      return new JobView(id, state, step, message, fileName, startedAt);
    }
  }

  private final WorkspaceAccess workspaceAccess;
  private final TpdgRenderer tpdg;
  private final YodaPdfRenderer yoda;
  private final ExecutorService executor = Executors.newFixedThreadPool(2);
  private final Map<String, Job> jobs = new LinkedHashMap<>();

  public PdfJobService(WorkspaceAccess workspaceAccess, TpdgRenderer tpdg, YodaPdfRenderer yoda) {
    this.workspaceAccess = workspaceAccess;
    this.tpdg = tpdg;
    this.yoda = yoda;
  }

  public JobView start(Long workspaceId, RunDocumentResponse doc) {
    workspaceAccess.require(workspaceId);
    Job job = new Job(workspaceId);
    synchronized (jobs) {
      jobs.put(job.id, job);
      Iterator<Job> it = jobs.values().iterator();
      while (jobs.size() > KEEP && it.hasNext()) {
        Job old = it.next();
        if (old.state != State.RUNNING) {
          it.remove();
          deleteQuietly(old.dir);
        }
      }
    }
    executor.execute(() -> execute(job, doc));
    return job.view();
  }

  public JobView get(Long workspaceId, String id) {
    workspaceAccess.require(workspaceId);
    return find(workspaceId, id).view();
  }

  /** The finished PDF, or empty when the job is unknown, from another workspace, or not done. */
  public Path pdf(Long workspaceId, String id) {
    workspaceAccess.require(workspaceId);
    Job job = find(workspaceId, id);
    if (job.state != State.DONE) {
      throw new ConflictException("The PDF is not ready");
    }
    return job.dir.resolve(job.fileName);
  }

  private Job find(Long workspaceId, String id) {
    Job job;
    synchronized (jobs) {
      job = jobs.get(id);
    }
    if (job == null || !job.workspaceId.equals(workspaceId)) {
      throw new NotFoundException("PDF job not found");
    }
    return job;
  }

  private void execute(Job job, RunDocumentResponse doc) {
    try {
      Path dir = Files.createTempDirectory("verdict-pdf-");
      job.dir = dir;
      Path schema = copy(SCHEMA, dir.resolve("schema.json"));
      Path template = copy(TEMPLATE, dir.resolve("template.adoc.j2"));
      Path data = dir.resolve("data.yaml");
      Files.writeString(data, toYaml(doc), StandardCharsets.UTF_8);
      String base = fileBase(doc);

      job.step = "Rendering the AsciiDoc with TPDG";
      Path adoc = dir.resolve(base + ".adoc");
      TpdgRenderer.Result r1 = tpdg.render(schema, template, data, "adoc", adoc);
      if (!r1.ok()) {
        fail(job, "TPDG could not render the document. " + tail(r1.output()));
        return;
      }
      if (!yoda.isInstalled()) {
        fail(
            job,
            "YODA "
                + yoda.imageReference()
                + " is not installed on this server (or Docker is not available). The operator"
                + " runs `make install-yoda`; see DEPLOYMENT.md.");
        return;
      }
      job.step = "Building the PDF with YODA";
      TpdgRenderer.Result r2 = yoda.render(adoc, dir.resolve(base + ".pdf"));
      if (!r2.ok() || !Files.isRegularFile(dir.resolve(base + ".pdf"))) {
        fail(job, "YODA could not build the PDF. " + tail(r2.output()));
        return;
      }
      job.fileName = base + ".pdf";
      job.step = "Done";
      job.state = State.DONE;
    } catch (IOException | RuntimeException e) {
      fail(job, "The PDF could not be generated: " + e.getMessage());
    }
  }

  private static void fail(Job job, String message) {
    job.message = message;
    job.step = "Failed";
    job.state = State.ERROR;
  }

  private static String tail(String output) {
    String t = output == null ? "" : output.strip();
    return t.length() > 600 ? t.substring(t.length() - 600) : t;
  }

  private static Path copy(String resource, Path target) throws IOException {
    try (InputStream in = PdfJobService.class.getResourceAsStream(resource)) {
      if (in == null) {
        throw new IOException("missing resource " + resource);
      }
      Files.copy(in, target);
    }
    return target;
  }

  private static void deleteQuietly(Path dir) {
    if (dir == null) {
      return;
    }
    try (var files = Files.walk(dir)) {
      for (Path p : files.sorted(java.util.Comparator.reverseOrder()).toList()) {
        Files.deleteIfExists(p);
      }
    } catch (IOException ignored) {
      // a leftover temp folder is harmless
    }
  }

  /** plan-name_run-id_YYYY-MM-DD, the name of the printed document (D-100). */
  static String fileBase(RunDocumentResponse doc) {
    RunReportResponse.Run run = doc.summary().run();
    String plan = run.planSlug() != null ? run.planSlug() : "plan";
    return plan.replaceAll("[^A-Za-z0-9._-]", "-") + "_run-" + run.number() + "_" + LocalDate.now();
  }

  static String toYaml(RunDocumentResponse doc) {
    RunReportResponse s = doc.summary();
    RunReportResponse.Run run = s.run();
    Map<String, Object> m = new LinkedHashMap<>();
    m.put("title", "Test plan document: " + run.planName());
    m.put("plan", run.planName());
    m.put("run", "#" + run.number() + " (" + run.status() + ")");
    m.put("generated", LocalDate.now().toString());
    m.put("verdict", s.gate().verdict());
    m.put("passRate", s.gate().passRate() == null ? "no data" : s.gate().passRate() + "%");
    m.put("gate", s.gate().thresholdPercent() + "%");
    RunReportResponse.Counts c = s.counts();
    Map<String, Object> counts = new LinkedHashMap<>();
    counts.put("total", c.total());
    counts.put("executed", c.executed());
    counts.put("passed", c.passed());
    counts.put("failed", c.failed());
    counts.put("blocked", c.blocked());
    counts.put("skipped", c.skipped());
    counts.put("untested", c.untested());
    m.put("counts", counts);
    List<Object> suites = new ArrayList<>();
    for (RunReportResponse.Suite x : s.suites()) {
      Map<String, Object> e = new LinkedHashMap<>();
      e.put("name", x.name());
      e.put("total", x.total());
      e.put("passed", x.passed());
      e.put("failed", x.failed());
      suites.add(e);
    }
    m.put("suites", suites);
    List<Object> problems = new ArrayList<>();
    for (RunReportResponse.Problem x : s.problems()) {
      Map<String, Object> e = new LinkedHashMap<>();
      e.put("caseId", String.valueOf(x.caseId()));
      e.put("title", String.valueOf(x.title()));
      e.put("status", String.valueOf(x.status()));
      e.put("suite", String.valueOf(x.suite()));
      problems.add(e);
    }
    m.put("problems", problems);
    DumperOptions o = new DumperOptions();
    o.setDefaultFlowStyle(DumperOptions.FlowStyle.BLOCK);
    return new Yaml(o).dump(m);
  }
}
