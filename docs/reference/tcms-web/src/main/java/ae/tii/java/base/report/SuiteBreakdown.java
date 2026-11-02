package ae.tii.java.base.report;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Results of a run per suite (DECISIONS.md D-99). Pure. A suite is the frozen folder of a case;
 * there is one row per folder prefix, so {@code payments} aggregates {@code payments/refunds}.
 * Cases at the root of the cases folder form a "(root)" row of their own. Rows are sorted by path,
 * the root first, and carry the depth for indentation.
 */
public final class SuiteBreakdown {

  public static final String ROOT_NAME = "(root)";

  private SuiteBreakdown() {}

  /** A case's folder and status. */
  public record Entry(String folder, String status) {}

  /**
   * One suite row; {@code passRate} is the rounded-down pass rate or null without a denominator.
   */
  public record Row(
      String path,
      String name,
      int depth,
      int total,
      int passed,
      int failed,
      int blocked,
      int skipped,
      int untested,
      Integer passRate) {}

  private static final class Counts {
    int total;
    int passed;
    int failed;
    int blocked;
    int skipped;
    int untested;
  }

  public static List<Row> of(List<Entry> entries) {
    Map<String, Counts> byPath = new LinkedHashMap<>();
    for (Entry entry : entries) {
      for (String path : prefixes(entry.folder())) {
        add(byPath.computeIfAbsent(path, k -> new Counts()), entry.status());
      }
    }
    List<Row> rows = new ArrayList<>();
    for (Map.Entry<String, Counts> e : byPath.entrySet()) {
      String path = e.getKey();
      Counts c = e.getValue();
      int slash = path.lastIndexOf('/');
      rows.add(
          new Row(
              path,
              path.isEmpty() ? ROOT_NAME : path.substring(slash + 1),
              path.isEmpty() ? 0 : (int) path.chars().filter(ch -> ch == '/').count(),
              c.total,
              c.passed,
              c.failed,
              c.blocked,
              c.skipped,
              c.untested,
              PassRate.of(c.passed, c.failed, c.blocked, 0).displayPercent()));
    }
    rows.sort(Comparator.comparing(Row::path));
    return rows;
  }

  /** "a/b/c" gives a, a/b, a/b/c; the root gives the empty path only. */
  private static List<String> prefixes(String folder) {
    String clean = folder == null ? "" : folder.replaceAll("^/+|/+$", "");
    if (clean.isEmpty()) {
      return List.of("");
    }
    List<String> paths = new ArrayList<>();
    StringBuilder current = new StringBuilder();
    for (String segment : clean.split("/")) {
      if (segment.isEmpty()) {
        continue;
      }
      if (current.length() > 0) {
        current.append('/');
      }
      current.append(segment);
      paths.add(current.toString());
    }
    return paths;
  }

  private static void add(Counts counts, String status) {
    counts.total++;
    switch (status == null ? "" : status) {
      case "PASSED" -> counts.passed++;
      case "FAILED" -> counts.failed++;
      case "BLOCKED" -> counts.blocked++;
      case "SKIPPED" -> counts.skipped++;
      default -> counts.untested++;
    }
  }
}
