package ae.tii.java.base.report;

import java.time.LocalDateTime;
import java.util.List;

/**
 * Since which run a failing case has been failing (DECISIONS.md D-98). Pure.
 *
 * <p>Starting from the selected run, in which the case failed, walk back through the plan's earlier
 * runs: FAILED extends the streak, PASSED ends it, anything else (BLOCKED, SKIPPED, UNTESTED, or
 * the case being absent) is skipped. The result is the oldest run of the streak.
 */
public final class FailingSince {

  private FailingSince() {}

  /** A run of the plan as the walk sees it. */
  public record RunPoint(Long runId, int number, LocalDateTime createdAt) {}

  /** One earlier run and the case's status in it (null when the case is absent). */
  public record Earlier(RunPoint run, String status) {}

  /**
   * @param selected the selected run
   * @param earlierNewestFirst the plan's earlier runs, the one just before the selected run first
   */
  public static RunPoint compute(RunPoint selected, List<Earlier> earlierNewestFirst) {
    RunPoint since = selected;
    for (Earlier earlier : earlierNewestFirst) {
      if ("FAILED".equals(earlier.status())) {
        since = earlier.run();
      } else if ("PASSED".equals(earlier.status())) {
        break;
      }
    }
    return since;
  }
}
