package ae.tii.java.base.report;

import java.util.List;

/**
 * Classifies a case from its results over the report window (DECISIONS.md D-97). Pure.
 *
 * <p>Only PASSED and FAILED results count, oldest to newest; BLOCKED, SKIPPED, UNTESTED and absent
 * results are ignored. A flip is a change between two neighbouring kept results. Four or more flips
 * is FLAKY (which outranks FAILING); otherwise a last kept result of FAILED is FAILING; everything
 * else, including no results at all, is STABLE.
 */
public final class StabilityClassifier {

  public static final String STABLE = "STABLE";
  public static final String FLAKY = "FLAKY";
  public static final String FAILING = "FAILING";
  public static final int FLAKY_FLIPS = 4;

  private StabilityClassifier() {}

  public record Result(String stability, int flips) {}

  /**
   * @param statuses results oldest first; null or any other word means "no usable result"
   */
  public static Result classify(List<String> statuses) {
    int flips = 0;
    String previous = null;
    for (String status : statuses) {
      if (!"PASSED".equals(status) && !"FAILED".equals(status)) {
        continue;
      }
      if (previous != null && !previous.equals(status)) {
        flips++;
      }
      previous = status;
    }
    if (flips >= FLAKY_FLIPS) {
      return new Result(FLAKY, flips);
    }
    if ("FAILED".equals(previous)) {
      return new Result(FAILING, flips);
    }
    return new Result(STABLE, flips);
  }
}
