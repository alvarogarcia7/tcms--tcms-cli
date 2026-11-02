package ae.tii.java.base.vv;

import ae.tii.java.base.dto.SyncReportResponse.SchemaError;
import ae.tii.java.base.git.CaseFileParser;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Component;

/**
 * What one V&amp;V load read (ORCH-061, D-673), for the result summary and the source card: the
 * distinct cases (case files in a {@code cases/} or {@code testcases/} folder that pass the case
 * schema, both formats of D-380, plus the test cases of result containers, counted once by id), the
 * case files with schema errors (first error each, at most {@link #MAX_ERRORS}), and the recorded
 * V&amp;V runs ({@code vv-tcms}, D-430). Reuses the parsers of the Library and the Runs tab; it
 * never fails a load (an unreadable file is a schema error, a parser crash is logged and counted as
 * nothing read).
 */
@Component
@Slf4j
public class VvLoadSummarizer {

  static final int MAX_ERRORS = 20;
  private static final Set<String> CASE_FOLDERS = Set.of("cases", "testcases");

  private final CaseFileParser cases;
  private final VvResultsParser results;
  private final VvRecordedRunParser runs;

  public VvLoadSummarizer(CaseFileParser cases, VvResultsParser results, VvRecordedRunParser runs) {
    this.cases = cases;
    this.results = results;
    this.runs = runs;
  }

  /** For tests and the no-Spring constructor of the loader. */
  public VvLoadSummarizer() {
    this(new CaseFileParser(), new VvResultsParser(), new VvRecordedRunParser());
  }

  /** Counts for one load. */
  public record Summary(
      int casesRead, int schemaErrorCount, List<SchemaError> schemaErrors, int runsRead) {

    public Summary {
      schemaErrors = List.copyOf(schemaErrors);
    }

    static final Summary EMPTY = new Summary(0, 0, List.of(), 0);
  }

  /**
   * Summarise the files of a load under {@code root}; {@code items} are the V&amp;V files found.
   */
  public Summary summarize(Path root, List<VvItem> items) {
    if (root == null || !Files.isDirectory(root)) {
      return Summary.EMPTY;
    }
    Set<String> ids = new LinkedHashSet<>();
    List<SchemaError> errors = new ArrayList<>();
    int errorCount = 0;
    for (VvItem item : items) {
      if (!isCaseFile(item.getPath())) {
        continue;
      }
      try {
        CaseFileParser.ParseResult parsed =
            cases.parse(item.getPath(), Files.readAllBytes(root.resolve(item.getPath())));
        if (parsed.valid() && parsed.parsedCase() != null) {
          ids.add(parsed.parsedCase().id());
        } else {
          errorCount++;
          if (errors.size() < MAX_ERRORS && !parsed.errors().isEmpty()) {
            errors.add(parsed.errors().get(0));
          }
        }
      } catch (IOException | RuntimeException e) {
        errorCount++;
        if (errors.size() < MAX_ERRORS) {
          errors.add(new SchemaError(item.getPath(), 0, "", "File could not be read"));
        }
      }
    }
    try {
      for (VvResults.TestCaseResult tc : results.parseFromDirectory(root).getTestCases()) {
        if (tc.getTestCaseId() != null) {
          ids.add(tc.getTestCaseId());
        }
      }
    } catch (RuntimeException e) {
      log.info("Could not read the V&V results under {}: {}", root, e.getMessage());
    }
    int runCount = 0;
    try {
      runCount = runs.parse(root).size();
    } catch (RuntimeException e) {
      log.info("Could not read the recorded V&V runs under {}: {}", root, e.getMessage());
    }
    return new Summary(ids.size(), errorCount, errors, runCount);
  }

  /** A YAML file inside a {@code cases/} or {@code testcases/} folder, at any depth. */
  static boolean isCaseFile(String relativePath) {
    String path = relativePath.replace('\\', '/');
    String lower = path.toLowerCase(Locale.ROOT);
    if (!(lower.endsWith(".yml") || lower.endsWith(".yaml"))) {
      return false;
    }
    String[] parts = path.split("/");
    for (int i = 0; i < parts.length - 1; i++) {
      if (CASE_FOLDERS.contains(parts[i].toLowerCase(Locale.ROOT))) {
        return true;
      }
    }
    return false;
  }
}
