package ae.tii.java.base.report;

import static org.assertj.core.api.Assertions.assertThat;

import ae.tii.java.base.report.FailingSince.Earlier;
import ae.tii.java.base.report.FailingSince.RunPoint;
import ae.tii.java.base.report.SuiteBreakdown.Entry;
import ae.tii.java.base.report.SuiteBreakdown.Row;
import java.time.LocalDateTime;
import java.util.Arrays;
import java.util.List;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

@DisplayName("Report calculations (D-95 to D-100)")
class ReportClassesTest {

  @Nested
  @DisplayName("PassRate")
  class PassRateTests {

    @Test
    @DisplayName("Passed / (Passed + Failed + Blocked); the gate compares exactly")
    void verdict() {
      PassRate.Result r = PassRate.of(9, 1, 0, 90);
      assertThat(r.displayPercent()).isEqualTo(90);
      assertThat(r.verdict()).isEqualTo(PassRate.PASSED);
      assertThat(PassRate.of(89, 11, 0, 90).verdict()).isEqualTo(PassRate.FAILED);
      assertThat(PassRate.of(0, 1, 0, 50).verdict()).isEqualTo(PassRate.FAILED);
      assertThat(PassRate.of(1, 0, 0, 100).verdict()).isEqualTo(PassRate.PASSED);
    }

    @Test
    @DisplayName("the percentage is rounded down so 89.9% never reads 90% FAILED")
    void roundsDown() {
      PassRate.Result r = PassRate.of(899, 101, 0, 90);
      assertThat(r.displayPercent()).isEqualTo(89);
      assertThat(r.verdict()).isEqualTo(PassRate.FAILED);
      assertThat(PassRate.of(2, 1, 0, 50).displayPercent()).isEqualTo(66);
    }

    @Test
    @DisplayName("blocked counts against the rate; no denominator is NO_DATA")
    void denominators() {
      assertThat(PassRate.of(1, 0, 1, 50).displayPercent()).isEqualTo(50);
      PassRate.Result none = PassRate.of(0, 0, 0, 90);
      assertThat(none.verdict()).isEqualTo(PassRate.NO_DATA);
      assertThat(none.displayPercent()).isNull();
    }

    @Test
    @DisplayName("the delta is the rounded difference of the exact rates, or null without data")
    void delta() {
      PassRate.Result now = PassRate.of(9, 1, 0, 90);
      PassRate.Result before = PassRate.of(8, 2, 0, 90);
      assertThat(PassRate.delta(now, before)).isEqualTo(10);
      assertThat(PassRate.delta(before, now)).isEqualTo(-10);
      assertThat(PassRate.delta(now, now)).isZero();
      assertThat(PassRate.delta(now, PassRate.of(0, 0, 0, 90))).isNull();
      assertThat(PassRate.delta(now, null)).isNull();
      assertThat(PassRate.delta(PassRate.of(2, 1, 0, 90), PassRate.of(1, 0, 0, 90))).isEqualTo(-33);
    }
  }

  @Nested
  @DisplayName("StabilityClassifier")
  class StabilityTests {

    private List<String> s(String... statuses) {
      return Arrays.asList(statuses);
    }

    @Test
    @DisplayName("four or more flips is flaky, and outranks failing")
    void flaky() {
      StabilityClassifier.Result r =
          StabilityClassifier.classify(s("PASSED", "FAILED", "PASSED", "FAILED", "PASSED"));
      assertThat(r.stability()).isEqualTo("FLAKY");
      assertThat(r.flips()).isEqualTo(4);
      assertThat(
              StabilityClassifier.classify(
                      s("PASSED", "FAILED", "PASSED", "FAILED", "PASSED", "FAILED"))
                  .stability())
          .isEqualTo("FLAKY");
    }

    @Test
    @DisplayName("fewer flips: a last Failed is failing, otherwise stable")
    void failingAndStable() {
      assertThat(
              StabilityClassifier.classify(s("PASSED", "FAILED", "PASSED", "FAILED")).stability())
          .as("three flips")
          .isEqualTo("FAILING");
      assertThat(StabilityClassifier.classify(s("FAILED", "FAILED")).stability())
          .isEqualTo("FAILING");
      assertThat(StabilityClassifier.classify(s("FAILED", "PASSED")).stability())
          .isEqualTo("STABLE");
      assertThat(StabilityClassifier.classify(s("PASSED", "PASSED")).stability())
          .isEqualTo("STABLE");
    }

    @Test
    @DisplayName("blocked, skipped, untested and absent results are ignored")
    void ignored() {
      assertThat(
              StabilityClassifier.classify(
                      s("PASSED", "BLOCKED", null, "SKIPPED", "UNTESTED", "PASSED"))
                  .flips())
          .isZero();
      assertThat(StabilityClassifier.classify(s("FAILED", "BLOCKED", null)).stability())
          .as("last kept result is Failed")
          .isEqualTo("FAILING");
      StabilityClassifier.Result none = StabilityClassifier.classify(s(null, "UNTESTED"));
      assertThat(none.stability()).isEqualTo("STABLE");
      assertThat(none.flips()).isZero();
      assertThat(StabilityClassifier.classify(List.of()).stability()).isEqualTo("STABLE");
    }
  }

  @Nested
  @DisplayName("FailingSince")
  class FailingSinceTests {

    private RunPoint run(int number) {
      return new RunPoint((long) number, number, LocalDateTime.of(2026, 1, number, 0, 0));
    }

    @Test
    @DisplayName("the oldest run of the streak of Failed results")
    void streak() {
      List<Earlier> earlier =
          List.of(
              new Earlier(run(9), "FAILED"),
              new Earlier(run(8), "FAILED"),
              new Earlier(run(7), "PASSED"),
              new Earlier(run(6), "FAILED"));
      assertThat(FailingSince.compute(run(10), earlier).number()).isEqualTo(8);
    }

    @Test
    @DisplayName("blocked, skipped, untested and absent runs are skipped, not streak breakers")
    void skipped() {
      List<Earlier> earlier =
          List.of(
              new Earlier(run(9), "BLOCKED"),
              new Earlier(run(8), null),
              new Earlier(run(7), "FAILED"),
              new Earlier(run(6), "SKIPPED"),
              new Earlier(run(5), "UNTESTED"),
              new Earlier(run(4), "PASSED"));
      assertThat(FailingSince.compute(run(10), earlier).number()).isEqualTo(7);
    }

    @Test
    @DisplayName("with no earlier failure the selected run itself starts the streak")
    void firstFailure() {
      assertThat(FailingSince.compute(run(10), List.of(new Earlier(run(9), "PASSED"))).number())
          .isEqualTo(10);
      assertThat(FailingSince.compute(run(10), List.of()).number()).isEqualTo(10);
    }
  }

  @Nested
  @DisplayName("SuiteBreakdown")
  class SuiteTests {

    @Test
    @DisplayName("one row per folder prefix; parents aggregate children; sorted by path")
    void prefixes() {
      List<Row> rows =
          SuiteBreakdown.of(
              List.of(
                  new Entry("payments/refunds", "PASSED"),
                  new Entry("payments/refunds", "FAILED"),
                  new Entry("payments", "PASSED"),
                  new Entry("auth", "BLOCKED"),
                  new Entry("auth", "SKIPPED"),
                  new Entry("", "UNTESTED")));
      assertThat(rows)
          .extracting(Row::path)
          .containsExactly("", "auth", "payments", "payments/refunds");
      Row root = rows.get(0);
      assertThat(root.name()).isEqualTo("(root)");
      assertThat(root.depth()).isZero();
      assertThat(root.untested()).isEqualTo(1);
      assertThat(root.passRate()).as("no denominator").isNull();
      Row payments = rows.get(2);
      assertThat(payments.total()).isEqualTo(3);
      assertThat(payments.passed()).isEqualTo(2);
      assertThat(payments.failed()).isEqualTo(1);
      assertThat(payments.passRate()).isEqualTo(66);
      Row refunds = rows.get(3);
      assertThat(refunds.name()).isEqualTo("refunds");
      assertThat(refunds.depth()).isEqualTo(1);
      Row auth = rows.get(1);
      assertThat(auth.blocked()).isEqualTo(1);
      assertThat(auth.skipped()).isEqualTo(1);
      assertThat(auth.passRate()).isZero();
    }

    @Test
    @DisplayName("no cases, no rows")
    void empty() {
      assertThat(SuiteBreakdown.of(List.of())).isEmpty();
    }
  }

  @Nested
  @DisplayName("CsvWriter")
  class CsvTests {

    @Test
    @DisplayName("RFC 4180 quoting, CRLF, BOM and formula protection")
    void csv() {
      String text =
          new CsvWriter()
              .row(List.of("a", "b,c", "say \"hi\"", "line\nbreak", ""))
              .row(List.of("=SUM(A1)", "-1", "@x", "plain"))
              .text();
      assertThat(text).startsWith("﻿");
      assertThat(text)
          .contains("a,\"b,c\",\"say \"\"hi\"\"\",\"line\nbreak\",\r\n")
          .contains("'=SUM(A1),'-1,'@x,plain\r\n");
    }

    @Test
    @DisplayName("bytes are UTF-8")
    void bytes() {
      byte[] bytes = new CsvWriter().row(List.of("é")).bytes();
      assertThat(bytes[0]).isEqualTo((byte) 0xEF);
      assertThat(new String(bytes, java.nio.charset.StandardCharsets.UTF_8)).contains("é");
    }
  }
}
