package ae.tii.java.base.trace;

import java.util.List;

/**
 * How well a requirement is covered by one run (DECISIONS.md D-116.3). Pure.
 *
 * <p>Looking at the statuses of the cases linked to the requirement in that run: no linked case is
 * NO_CASES; any Failed is FAILING; every case Passed is COVERED; anything else is NOT_RUN (some
 * case is untested, blocked, skipped or not in the run). {@code executed} counts the linked cases
 * that have a result, for the "k/n run" subtext.
 */
public final class CoverageClassifier {

  public static final String NO_CASES = "NO_CASES";
  public static final String FAILING = "FAILING";
  public static final String COVERED = "COVERED";
  public static final String NOT_RUN = "NOT_RUN";

  private CoverageClassifier() {}

  public record Result(String coverage, int executed, int total) {}

  /**
   * @param statuses one entry per linked valid case: its status in the run, or null when the case
   *     is not in the run
   */
  public static Result classify(List<String> statuses) {
    int total = statuses.size();
    if (total == 0) {
      return new Result(NO_CASES, 0, 0);
    }
    int executed = 0;
    boolean failing = false;
    boolean allPassed = true;
    for (String status : statuses) {
      if (status != null && !"UNTESTED".equals(status)) {
        executed++;
      }
      if ("FAILED".equals(status)) {
        failing = true;
      }
      if (!"PASSED".equals(status)) {
        allPassed = false;
      }
    }
    if (failing) {
      return new Result(FAILING, executed, total);
    }
    return new Result(allPassed ? COVERED : NOT_RUN, executed, total);
  }
}
