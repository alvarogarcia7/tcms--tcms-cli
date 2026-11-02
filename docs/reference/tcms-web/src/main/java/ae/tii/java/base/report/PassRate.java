package ae.tii.java.base.report;

/**
 * Pass rate and quality-gate verdict of a run (DECISIONS.md D-95, D-96). Pure.
 *
 * <p>Pass rate = Passed / (Passed + Failed + Blocked); Skipped and Untested are outside it. The
 * gate is passed when {@code passed * 100 >= gate * denominator} (exact, no rounding); the
 * displayed percentage is rounded down so a badge never reads "90% FAILED".
 */
public final class PassRate {

  public static final String PASSED = "PASSED";
  public static final String FAILED = "FAILED";
  public static final String NO_DATA = "NO_DATA";

  private PassRate() {}

  /** The figures of one run's pass rate; {@code displayPercent} is null without a denominator. */
  public record Result(
      int denominator, Integer displayPercent, Double exactPercent, String verdict) {}

  public static int denominator(int passed, int failed, int blocked) {
    return passed + failed + blocked;
  }

  public static Result of(int passed, int failed, int blocked, int gatePercent) {
    int denominator = denominator(passed, failed, blocked);
    if (denominator == 0) {
      return new Result(0, null, null, NO_DATA);
    }
    boolean gatePassed = (long) passed * 100 >= (long) gatePercent * denominator;
    return new Result(
        denominator,
        (int) ((long) passed * 100 / denominator),
        passed * 100.0 / denominator,
        gatePassed ? PASSED : FAILED);
  }

  /**
   * The change from the previous run in percentage points, rounded to an integer (D-96); null when
   * either side has no data.
   */
  public static Integer delta(Result current, Result previous) {
    if (current == null
        || previous == null
        || current.exactPercent() == null
        || previous.exactPercent() == null) {
      return null;
    }
    return (int) Math.round(current.exactPercent() - previous.exactPercent());
  }
}
