//! TCMS-41: edge cases of the verifier binary. Each test writes a handwritten test case and
//! execution log into a tempdir and runs the real binary.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One step of a handwritten test case: (sequence, step, expected result, expected output)
type StepSpec = (i64, i64, &'static str, &'static str);

fn test_case_yaml(id: &str, steps: &[StepSpec]) -> String {
    let mut yaml = format!(
        "type: test_case\nschema: tcms/test-case.schema.v1.json\nrequirement: R\nitem: 1\ntc: 1\n\
         id: {id}\ndescription: edge case\ngeneral_initial_conditions: {{}}\n\
         initial_conditions: {{}}\n"
    );
    if steps.is_empty() {
        yaml.push_str("test_sequences: []\n");
        return yaml;
    }
    yaml.push_str("test_sequences:\n");
    let mut seqs: Vec<i64> = steps.iter().map(|s| s.0).collect();
    seqs.dedup();
    for seq in seqs {
        yaml.push_str(&format!(
            "- id: {seq}\n  name: seq{seq}\n  description: d\n  initial_conditions: {{}}\n  steps:\n"
        ));
        for (_, step, result, output) in steps.iter().filter(|s| s.0 == seq) {
            // JSON strings are valid YAML scalars, which keeps odd values quoted
            yaml.push_str(&format!(
                "  - step: {step}\n    description: step {step}\n    command: echo\n    \
                 expected:\n      success: true\n      result: {}\n      output: {}\n    \
                 verification:\n      result: '[[ $? -eq 0 ]]'\n      output: 'true'\n",
                json!(result),
                json!(output)
            ));
        }
    }
    yaml
}

fn entry(seq: i64, step: i64, exit_code: i64, output: &str) -> Value {
    json!({
        "test_sequence": seq, "step": step, "command": "echo", "exit_code": exit_code,
        "output": output, "timestamp": "2026-10-28T10:00:00",
        "result_verification_pass": true, "output_verification_pass": true
    })
}

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("cases")).unwrap();
        Fixture { dir }
    }
    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }
    fn case(&self, id: &str, steps: &[StepSpec]) -> &Self {
        std::fs::write(
            self.path(&format!("cases/{id}.yaml")),
            test_case_yaml(id, steps),
        )
        .unwrap();
        self
    }
    fn log(&self, rel: &str, content: &str) -> &Self {
        let path = self.path(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
        self
    }
    fn log_entries(&self, rel: &str, entries: &[Value]) -> &Self {
        self.log(rel, &serde_json::to_string(entries).unwrap())
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_verifier"))
            .current_dir(root())
            .env_remove("TCMS_RUN_ID")
            .env_remove("TCMS_BUILD_ID")
            .env_remove("TCMS_TESTER")
            .env_remove("TCMS_REVIEWER")
            .env_remove("RUST_LOG")
            .args(args)
            .arg("--test-case-dir")
            .arg(self.path("cases"))
            .arg("--schema")
            .arg(root().join("data/testcase_results_container/schema.json"))
            .output()
            .unwrap()
    }
    /// Single-file mode on `<id>_execution_log.json`
    fn single(&self, id: &str, extra: &[&str]) -> Output {
        let log = self.path(&format!("{id}_execution_log.json"));
        let mut args = vec!["--test-case", id, "--log", log.to_str().unwrap()];
        args.extend_from_slice(extra);
        self.run(&args)
    }
    fn folder(&self, extra: &[&str]) -> Output {
        let folder = self.dir.path().to_str().unwrap().to_string();
        let mut args = vec!["--folder", folder.as_str()];
        args.extend_from_slice(extra);
        self.run(&args)
    }
}

fn stdout_json(output: &Output) -> Value {
    serde_yaml::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not YAML ({e}): {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn code(output: &Output) -> i32 {
    output
        .status
        .code()
        .expect("verifier was killed by a signal")
}

/// The single test case result of a container
fn first_result(container: &Value) -> &Value {
    &container["test_results"][0]
}

/// The outcome of each step of a test case result, in order: "Pass", "Fail" or "NotExecuted".
/// Each step result is an externally tagged enum, so its only key is the outcome.
fn step_statuses(result: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for seq in result["sequences"].as_array().unwrap() {
        for step in seq["step_results"].as_array().unwrap() {
            out.push(step.as_object().unwrap().keys().next().unwrap().clone());
        }
    }
    out
}

fn out_arg(f: &Fixture) -> (PathBuf, String) {
    let p = f.path("out.yaml");
    let a = p.to_str().unwrap().to_string();
    (p, a)
}

fn read_result(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap();
    serde_yaml::from_str(&text).unwrap()
}

fn success(output: &Output) {
    assert_eq!(code(output), 0, "{}", stderr(output));
}

// ---------------------------------------------------------------------------------------------
// logs
// ---------------------------------------------------------------------------------------------

#[test]
fn an_empty_log_array_leaves_every_step_not_executed_and_exits_1() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "hi"), (1, 2, "0", "yo")]);
    f.log("TC_A_execution_log.json", "[]");
    let (path, arg) = out_arg(&f);
    let out = f.single("TC_A", &["--output", &arg]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let result = read_result(&path);
    let r = first_result(&result);
    assert_eq!(r["overall_pass"], false);
    assert_eq!(r["not_executed_steps"], 2);
    assert_eq!(step_statuses(r), ["NotExecuted", "NotExecuted"]);
}

#[test]
fn a_truncated_log_is_never_a_pass() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "hi")]);
    let full = serde_json::to_string(&[entry(1, 1, 0, "hi")]).unwrap();
    f.log("TC_A_execution_log.json", &full[..full.len() - 8]);
    let out = f.single("TC_A", &[]);
    assert_ne!(
        code(&out),
        0,
        "a truncated log must not verify: {}",
        stderr(&out)
    );
}

#[test]
fn garbage_and_empty_log_files_are_never_a_pass() {
    for content in ["", "not json at all", "{\"a\": 1}", "[1, 2, 3]", "[null]"] {
        let f = Fixture::new();
        f.case("TC_A", &[(1, 1, "0", "hi")]);
        f.log("TC_A_execution_log.json", content);
        let out = f.single("TC_A", &[]);
        assert_ne!(code(&out), 0, "log {content:?} verified: {}", stderr(&out));
        assert_ne!(
            code(&out),
            101,
            "log {content:?} panicked: {}",
            stderr(&out)
        );
    }
}

#[test]
fn a_log_with_only_a_failed_hook_does_not_verify() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "hi")]);
    f.log_entries(
        "TC_A_execution_log.json",
        &[
            json!({"test_sequence": 0, "step": 0, "hook_type": "script_start", "command": "false",
                 "exit_code": 1, "output": "boom", "timestamp": "2026-10-28T10:00:00",
                 "result_verification_pass": false, "output_verification_pass": false}),
        ],
    );
    let out = f.single("TC_A", &[]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
}

#[test]
#[ignore = "gap D-41b: a duplicated step keeps only the last entry (HashMap insert), so a failed attempt followed by a passing retry verifies as a pass; the fix needs a product decision on retries"]
fn a_failed_attempt_followed_by_a_passing_retry_must_not_pass() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "hi")]);
    f.log_entries(
        "TC_A_execution_log.json",
        &[entry(1, 1, 0, "wrong"), entry(1, 1, 0, "hi")],
    );
    assert_eq!(code(&f.single("TC_A", &[])), 1);
}

#[test]
fn a_duplicated_step_with_identical_entries_is_one_step() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "hi")]);
    let (path, arg) = out_arg(&f);
    f.log_entries(
        "TC_A_execution_log.json",
        &[entry(1, 1, 0, "hi"), entry(1, 1, 0, "hi")],
    );
    success(&f.single("TC_A", &["--output", &arg]));
    let r = first_result(&read_result(&path)).clone();
    assert_eq!(
        (r["total_steps"].as_i64(), r["passed_steps"].as_i64()),
        (Some(1), Some(1))
    );
}

#[test]
fn steps_logged_out_of_order_are_matched_by_number() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a"), (1, 2, "0", "b")]);
    f.log_entries(
        "TC_A_execution_log.json",
        &[entry(1, 2, 0, "b"), entry(1, 1, 0, "a")],
    );
    success(&f.single("TC_A", &[]));
}

#[test]
fn a_missing_step_fails_the_case_but_the_others_still_verify() {
    let f = Fixture::new();
    f.case(
        "TC_A",
        &[(1, 1, "0", "a"), (1, 2, "0", "b"), (1, 3, "0", "c")],
    );
    f.log_entries(
        "TC_A_execution_log.json",
        &[entry(1, 1, 0, "a"), entry(1, 3, 0, "c")],
    );
    let (path, arg) = out_arg(&f);
    assert_eq!(code(&f.single("TC_A", &["--output", &arg])), 1);
    assert_eq!(
        step_statuses(first_result(&read_result(&path))),
        ["Pass", "NotExecuted", "Pass"]
    );
}

#[test]
fn log_entries_for_steps_not_in_the_test_case_are_ignored() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.log_entries(
        "TC_A_execution_log.json",
        &[
            entry(1, 1, 0, "a"),
            entry(1, 9, 1, "extra"),
            entry(7, 1, 1, "extra"),
        ],
    );
    let (path, arg) = out_arg(&f);
    success(&f.single("TC_A", &["--output", &arg]));
    assert_eq!(first_result(&read_result(&path))["total_steps"], 1);
}

#[test]
fn a_passing_step_with_a_wrong_exit_code_fails() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 2, "a")]);
    assert_eq!(code(&f.single("TC_A", &[])), 1);
}

#[test]
fn output_that_looks_like_other_yaml_types_stays_a_string() {
    let values = [
        "2026-10-28",
        "2026-10-28T10:00:00Z",
        "yes",
        "on",
        "no",
        "off",
        "true",
        "null",
        "~",
        "0755",
        "0x1f",
        "1e3",
        "007",
        "-",
        "",
        "  padded  ",
        "a: b",
        "- item",
        "# comment",
        "&anchor",
        "*alias",
        "!tag x",
        "{a: 1}",
        "[1, 2]",
    ];
    let container = compile(&root().join("data/testcase_results_container/schema.json"));
    let result_schema =
        compile(&root().join("schemas/tcms-schemas/tcms/verification-result.schema.json"));
    for format in ["yaml", "json"] {
        for value in values {
            let f = Fixture::new();
            f.case("TC_A", &[(1, 1, "0", "x")]);
            f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, value)]);
            let path = f.path(&format!("out.{format}"));
            // exact matching fails (expected "x"), so the actual output lands in the result
            let out = f.single(
                "TC_A",
                &["--format", format, "--output", path.to_str().unwrap()],
            );
            assert_eq!(code(&out), 1, "{format} {value:?}: {}", stderr(&out));
            let text = std::fs::read_to_string(&path).unwrap();
            let result: Value = if format == "json" {
                serde_json::from_str(&text).unwrap()
            } else {
                serde_yaml::from_str(&text).unwrap()
            };
            let fail = &first_result(&result)["sequences"][0]["step_results"][0]["Fail"];
            assert_eq!(
                fail["actual_output"],
                json!(value),
                "{format}: output {value:?} did not round-trip as a string"
            );
            assert_eq!(
                violations(&container, &result),
                Vec::<String>::new(),
                "{format} {value:?}"
            );
            assert_eq!(
                violations(&result_schema, first_result(&result)),
                Vec::<String>::new(),
                "{format} {value:?}"
            );
        }
    }
}

/// The schema violations of an instance, as text (empty when it conforms)
fn violations(schema: &jsonschema::JSONSchema, instance: &Value) -> Vec<String> {
    match schema.validate(instance) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .map(|e| format!("{e} at {}", e.instance_path))
            .collect(),
    }
}

#[test]
fn json_report_on_stdout_is_parseable() {
    let f = one_step("a", "a");
    let out = f.single("TC_A", &["--format", "json"]);
    success(&out);
    let parsed: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    assert_eq!(first_result(&parsed)["test_case_id"], "TC_A");
}

#[test]
fn unicode_quotes_backslashes_newlines_and_tabs_round_trip_in_both_formats() {
    let nasty = "caf\u{e9} \u{1F600} \"quoted\" back\\slash\nline2\n\ttabbed\r\n end";
    for format in ["yaml", "json"] {
        let f = Fixture::new();
        f.case("TC_A", &[(1, 1, "0", nasty)]);
        f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, nasty)]);
        let out_path = f.path(&format!("out.{format}"));
        let o = out_path.to_str().unwrap();
        success(&f.single("TC_A", &["--format", format, "--output", o]));
        let text = std::fs::read_to_string(&out_path).unwrap();
        let parsed: Value = serde_yaml::from_str(&text).unwrap();
        assert_eq!(first_result(&parsed)["overall_pass"], true, "{format}");
        // the same output as the expected side must also be stored without corruption
        let fail_case = Fixture::new();
        fail_case.case("TC_A", &[(1, 1, "0", "other")]);
        fail_case.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, nasty)]);
        let fo = fail_case.path(&format!("out.{format}"));
        assert_eq!(
            code(&fail_case.single(
                "TC_A",
                &["--format", format, "--output", fo.to_str().unwrap()]
            )),
            1
        );
        let parsed: Value = serde_yaml::from_str(&std::fs::read_to_string(&fo).unwrap()).unwrap();
        assert_eq!(
            first_result(&parsed)["sequences"][0]["step_results"][0]["Fail"]["actual_output"],
            json!(nasty),
            "{format}"
        );
    }
}

#[test]
fn a_very_long_output_is_verified_and_reported() {
    let long = "x".repeat(1_000_000);
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "x")]);
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, &long)]);
    let (path, arg) = out_arg(&f);
    // contains-match passes, exact would not
    success(&f.single("TC_A", &["--match-strategy", "contains", "--output", &arg]));
    assert_eq!(first_result(&read_result(&path))["overall_pass"], true);
    assert_eq!(code(&f.single("TC_A", &[])), 1);
}

// ---------------------------------------------------------------------------------------------
// test case problems
// ---------------------------------------------------------------------------------------------

#[test]
fn an_unknown_test_case_id_is_an_error() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.log_entries("TC_NOPE_execution_log.json", &[entry(1, 1, 0, "a")]);
    let out = f.single("TC_NOPE", &[]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(stderr(&out).contains("TC_NOPE"), "{}", stderr(&out));
}

#[test]
fn a_missing_log_file_is_an_error() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    let out = f.single("TC_A", &[]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("Log file does not exist"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn an_invalid_test_case_file_is_an_error() {
    let f = Fixture::new();
    std::fs::write(
        f.path("cases/TC_A.yaml"),
        "id: TC_A\ntest_sequences: [unclosed",
    )
    .unwrap();
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, "a")]);
    let out = f.single("TC_A", &[]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
}

#[test]
fn a_missing_test_case_directory_is_an_error() {
    let f = Fixture::new();
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, "a")]);
    let log = f.path("TC_A_execution_log.json");
    let out = Command::new(env!("CARGO_BIN_EXE_verifier"))
        .current_dir(root())
        .args([
            "--test-case",
            "TC_A",
            "--log",
            log.to_str().unwrap(),
            "--test-case-dir",
        ])
        .arg(f.path("no-such-dir"))
        .output()
        .unwrap();
    assert_eq!(code(&out), 1);
}

#[test]
fn a_test_case_without_sequences_or_steps_is_not_a_silent_pass() {
    // A case with nothing to verify proves nothing. It must not report a passing verdict.
    let f = Fixture::new();
    f.case("TC_A", &[]);
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, "a")]);
    let (path, arg) = out_arg(&f);
    let out = f.single("TC_A", &["--output", &arg]);
    let result = read_result(&path);
    assert_eq!(first_result(&result)["total_steps"], 0);
    assert_eq!(
        first_result(&result)["overall_pass"],
        false,
        "a test case with zero steps verified as passed (exit {})",
        code(&out)
    );
}

/// Two steps, the second one manual. The verifier does not special-case manual steps (decision
/// D-41d): each step needs its own log entry like any other.
fn case_with_manual_step(f: &Fixture) {
    let yaml = "type: test_case\nschema: tcms/test-case.schema.v1.json\nrequirement: R\nitem: 1\n\
        tc: 1\nid: TC_A\ndescription: manual\ngeneral_initial_conditions: {}\ninitial_conditions: {}\n\
        test_sequences:\n- id: 1\n  name: seq1\n  description: d\n  initial_conditions: {}\n  steps:\n\
        \x20 - step: 1\n    description: automated\n    command: echo\n    expected: {success: true, \
        result: '0', output: a}\n    verification: {result: '[[ $? -eq 0 ]]', output: 'true'}\n\
        \x20 - step: 2\n    description: by hand\n    manual: true\n    command: echo\n    \
        expected: {success: true, result: '0', output: b}\n    verification: {result: '[[ $? -eq 0 ]]', \
        output: 'true'}\n";
    std::fs::write(f.path("cases/TC_A.yaml"), yaml).unwrap();
}

#[test]
fn a_manual_step_without_a_log_entry_fails_the_case() {
    let f = Fixture::new();
    case_with_manual_step(&f);
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, "a")]);
    let (path, arg) = out_arg(&f);
    assert_eq!(code(&f.single("TC_A", &["--output", &arg])), 1);
    assert_eq!(
        step_statuses(first_result(&read_result(&path))),
        ["Pass", "NotExecuted"]
    );
}

#[test]
fn a_manual_step_with_a_log_entry_verifies_like_any_other_step() {
    let f = Fixture::new();
    case_with_manual_step(&f);
    f.log_entries(
        "TC_A_execution_log.json",
        &[entry(1, 1, 0, "a"), entry(1, 2, 0, "b")],
    );
    success(&f.single("TC_A", &[]));
}

// ---------------------------------------------------------------------------------------------
// match strategies and exit codes
// ---------------------------------------------------------------------------------------------

fn verdict(f: &Fixture, strategy: &str) -> i32 {
    code(&f.single("TC_A", &["--match-strategy", strategy]))
}

fn one_step(expected_output: &'static str, actual_output: &str) -> Fixture {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", expected_output)]);
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, actual_output)]);
    f
}

#[test]
fn exact_matching_needs_the_whole_output() {
    assert_eq!(verdict(&one_step("hello", "hello"), "exact"), 0);
    assert_eq!(verdict(&one_step("hello", "hello world"), "exact"), 1);
    assert_eq!(verdict(&one_step("hello", "Hello"), "exact"), 1);
}

#[test]
fn regex_matching_accepts_a_pattern_and_rejects_a_non_match() {
    assert_eq!(verdict(&one_step("^h.*o$", "hello"), "regex"), 0);
    assert_eq!(verdict(&one_step("^h.*x$", "hello"), "regex"), 1);
}

#[test]
fn an_invalid_regex_fails_the_step_without_panicking() {
    let f = one_step("([unclosed", "([unclosed");
    let (path, arg) = out_arg(&f);
    let out = f.single("TC_A", &["--match-strategy", "regex", "--output", &arg]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert_eq!(step_statuses(first_result(&read_result(&path))), ["Fail"]);
}

#[test]
fn contains_matching_accepts_a_substring_only() {
    assert_eq!(verdict(&one_step("ell", "hello"), "contains"), 0);
    assert_eq!(verdict(&one_step("xyz", "hello"), "contains"), 1);
}

#[test]
fn precomputed_matching_trusts_the_verification_flags_of_the_log() {
    let f = one_step("never compared", "whatever");
    assert_eq!(verdict(&f, "precomputed"), 0);

    let mut failing = entry(1, 1, 0, "whatever");
    failing["output_verification_pass"] = json!(false);
    f.log_entries("TC_A_execution_log.json", &[failing]);
    assert_eq!(verdict(&f, "precomputed"), 1);

    // a log without the flags cannot pass on trust
    let mut bare = entry(1, 1, 0, "whatever");
    bare.as_object_mut()
        .unwrap()
        .remove("result_verification_pass");
    bare.as_object_mut()
        .unwrap()
        .remove("output_verification_pass");
    f.log_entries("TC_A_execution_log.json", &[bare]);
    assert_eq!(verdict(&f, "precomputed"), 1);
}

#[test]
fn an_unknown_match_strategy_is_refused() {
    let f = one_step("a", "a");
    let out = f.single("TC_A", &["--match-strategy", "fuzzy"]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("Invalid match strategy"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn match_strategy_names_are_case_insensitive() {
    assert_eq!(verdict(&one_step("a", "a"), "EXACT"), 0);
}

#[test]
fn success_on_completion_turns_a_failed_verdict_into_exit_0_but_not_an_error() {
    let f = one_step("a", "b");
    assert_eq!(code(&f.single("TC_A", &[])), 1);
    let (path, arg) = out_arg(&f);
    success(&f.single("TC_A", &["--success-on-completion", "--output", &arg]));
    assert_eq!(first_result(&read_result(&path))["overall_pass"], false);

    // an error is still an error
    let out = f.single("TC_NOPE", &["--success-on-completion"]);
    assert_ne!(code(&out), 0);
}

#[test]
fn argument_errors_have_exit_codes_and_messages() {
    let f = one_step("a", "a");
    let log = f.path("TC_A_execution_log.json");
    let log = log.to_str().unwrap();
    let folder = f.dir.path().to_str().unwrap();

    let out = f.run(&[]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("Must specify either"),
        "{}",
        stderr(&out)
    );

    let out = f.run(&["--log", log]);
    assert_eq!(
        code(&out),
        1,
        "--log without --test-case must not pick a mode"
    );

    let out = f.run(&["--test-case", "TC_A"]);
    assert_eq!(code(&out), 1);

    let out = f.run(&["--test-case", "TC_A", "--log", log, "--folder", folder]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("Cannot use both"), "{}", stderr(&out));

    let out = f.run(&["--folder", folder, "--format", "xml"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("Format must be"), "{}", stderr(&out));

    let out = f.run(&["--folder", f.path("nope").to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("Folder does not exist"),
        "{}",
        stderr(&out)
    );

    let out = f.run(&["--folder", log]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("not a directory"), "{}", stderr(&out));

    let out = f.run(&["--no-such-flag"]);
    assert_eq!(code(&out), 2, "clap usage errors exit 2");
}

#[test]
fn a_test_case_id_that_does_not_match_the_log_content_is_not_a_pass() {
    // the log belongs to TC_B (its step 1 expects "b"), verified as TC_A (expects "a")
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.case("TC_B", &[(1, 1, "0", "b")]);
    f.log_entries("TC_B_execution_log.json", &[entry(1, 1, 0, "b")]);
    let log = f.path("TC_B_execution_log.json");
    let out = f.run(&["--test-case", "TC_A", "--log", log.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
}

// ---------------------------------------------------------------------------------------------
// source hash
// ---------------------------------------------------------------------------------------------

fn source_hash(f: &Fixture) -> String {
    let (path, arg) = out_arg(f);
    f.single("TC_A", &["--output", &arg]);
    first_result(&read_result(&path))["source_yaml_sha256"]
        .as_str()
        .expect("every result carries the hash of its test case file")
        .to_string()
}

#[test]
fn verify_source_hash_accepts_a_log_made_from_the_same_test_case() {
    let f = one_step("a", "a");
    let hash = source_hash(&f);
    let mut e = entry(1, 1, 0, "a");
    e["source_yaml_sha256"] = json!(hash);
    f.log_entries("TC_A_execution_log.json", &[e]);
    success(&f.single("TC_A", &["--verify-source-hash"]));
}

#[test]
fn verify_source_hash_rejects_a_log_made_from_an_edited_test_case() {
    let f = one_step("a", "a");
    let mut e = entry(1, 1, 0, "a");
    e["source_yaml_sha256"] = json!("0".repeat(64));
    f.log_entries("TC_A_execution_log.json", &[e]);
    let out = f.single("TC_A", &["--verify-source-hash"]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("Source hash verification failed"),
        "{}",
        stderr(&out)
    );
    // without the flag the same log verifies
    success(&f.single("TC_A", &[]));
}

#[test]
fn verify_source_hash_rejects_a_log_without_hashes() {
    let f = one_step("a", "a");
    let out = f.single("TC_A", &["--verify-source-hash"]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("missing source_yaml_sha256"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn verify_source_hash_rejects_a_log_mixing_good_and_bad_hashes() {
    let f = one_step("a", "a");
    let hash = source_hash(&f);
    let mut good = entry(1, 1, 0, "a");
    good["source_yaml_sha256"] = json!(hash);
    let mut bad = entry(1, 2, 0, "a");
    bad["source_yaml_sha256"] = json!("f".repeat(64));
    f.log_entries("TC_A_execution_log.json", &[good, bad]);
    assert_eq!(code(&f.single("TC_A", &["--verify-source-hash"])), 1);
}

#[test]
fn a_hash_mismatch_in_folder_mode_is_never_a_silent_success() {
    let f = one_step("a", "a");
    let mut e = entry(1, 1, 0, "a");
    e["source_yaml_sha256"] = json!("0".repeat(64));
    f.log_entries("TC_A_execution_log.json", &[e]);
    let out = f.folder(&["--verify-source-hash"]);
    assert_ne!(
        code(&out),
        0,
        "folder mode skipped a log whose hash mismatches and still exited 0"
    );
}

#[test]
fn a_matching_hash_in_folder_mode_verifies() {
    let f = one_step("a", "a");
    let hash = source_hash(&f);
    let mut e = entry(1, 1, 0, "a");
    e["source_yaml_sha256"] = json!(hash);
    f.log_entries("TC_A_execution_log.json", &[e]);
    success(&f.folder(&["--verify-source-hash"]));
}

#[test]
fn success_on_completion_in_folder_mode_forgives_a_failed_verdict_only() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, "WRONG")]);
    let (path, arg) = out_arg(&f);
    success(&f.folder(&["--success-on-completion", "--output", &arg]));
    assert_eq!(first_result(&read_result(&path))["overall_pass"], false);
    // without the flag the same folder exits 1
    assert_eq!(code(&f.folder(&[])), 1);
}

// ---------------------------------------------------------------------------------------------
// folder mode
// ---------------------------------------------------------------------------------------------

#[test]
fn an_empty_folder_is_an_error_not_a_pass() {
    // Nothing was verified, so exit 0 would read as a pass in CI (decision D-41c)
    let f = Fixture::new();
    let (path, arg) = out_arg(&f);
    let empty = f.path("empty");
    std::fs::create_dir(&empty).unwrap();
    let out = f.run(&["--folder", empty.to_str().unwrap(), "--output", &arg]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("No execution log files"),
        "{}",
        stderr(&out)
    );
    assert!(!path.exists(), "no report is written for an empty folder");
    // --success-on-completion only forgives a failed verdict, not a run with nothing verified
    let out = f.run(&[
        "--folder",
        empty.to_str().unwrap(),
        "--success-on-completion",
    ]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
}

#[test]
fn folder_mode_finds_nested_logs_and_ignores_other_files() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.case("TC_B", &[(1, 1, "0", "b")]);
    f.log_entries("logs/TC_A_execution_log.json", &[entry(1, 1, 0, "a")]);
    f.log_entries(
        "logs/deep/er/TC_B_execution_log.json",
        &[entry(1, 1, 0, "b")],
    );
    f.log("logs/notes.txt", "not a log");
    f.log("logs/TC_C_execution_log.txt", "wrong extension");
    f.log("logs/other.json", "[]");
    let (path, arg) = out_arg(&f);
    let logs = f.path("logs");
    let out = f.run(&["--folder", logs.to_str().unwrap(), "--output", &arg]);
    success(&out);
    let result = read_result(&path);
    assert_eq!(result["metadata"]["total_test_cases"], 2);
    assert_eq!(result["metadata"]["passed_test_cases"], 2);
}

#[test]
fn folder_mode_reports_every_log_even_when_one_fails() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.case("TC_B", &[(1, 1, "0", "b")]);
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, "a")]);
    f.log_entries("TC_B_execution_log.json", &[entry(1, 1, 0, "WRONG")]);
    let (path, arg) = out_arg(&f);
    assert_eq!(code(&f.folder(&["--output", &arg])), 1);
    let result = read_result(&path);
    assert_eq!(result["metadata"]["passed_test_cases"], 1);
    assert_eq!(result["metadata"]["failed_test_cases"], 1);
}

#[test]
fn a_log_whose_test_case_is_unknown_is_never_a_silent_success_in_folder_mode() {
    let f = Fixture::new();
    f.log_entries("TC_GHOST_execution_log.json", &[entry(1, 1, 0, "a")]);
    let out = f.folder(&[]);
    assert_ne!(
        code(&out),
        0,
        "a log without a test case was skipped and the run exited 0: {}",
        stderr(&out)
    );
}

#[test]
fn an_unparsable_log_is_never_a_silent_success_in_folder_mode() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.log("TC_A_execution_log.json", "{{{{ not json");
    let out = f.folder(&[]);
    assert_ne!(code(&out), 0, "{}", stderr(&out));
}

#[test]
fn two_logs_for_the_same_test_case_are_both_reported() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.log_entries("run1/TC_A_execution_log.json", &[entry(1, 1, 0, "a")]);
    f.log_entries("run2/TC_A_execution_log.json", &[entry(1, 1, 0, "WRONG")]);
    let (path, arg) = out_arg(&f);
    assert_eq!(code(&f.folder(&["--output", &arg])), 1);
    let result = read_result(&path);
    assert_eq!(
        result["test_results"].as_array().unwrap().len(),
        2,
        "the second log for a test case must not be dropped"
    );
}

// ---------------------------------------------------------------------------------------------
// output
// ---------------------------------------------------------------------------------------------

fn compile(path: &Path) -> jsonschema::JSONSchema {
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    jsonschema::JSONSchema::options().compile(&schema).unwrap()
}

#[test]
fn the_report_goes_to_stdout_and_stays_there_whatever_the_log_level() {
    let f = one_step("a", "a");
    let out = f.single("TC_A", &["--log-level", "error"]);
    success(&out);
    let parsed = stdout_json(&out);
    assert_eq!(first_result(&parsed)["test_case_id"], "TC_A");
}

#[test]
fn json_and_yaml_reports_say_the_same_and_validate_against_both_schemas() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a"), (1, 2, "0", "b")]);
    f.case("TC_B", &[(1, 1, "0", "a")]);
    f.log_entries(
        "TC_A_execution_log.json",
        &[entry(1, 1, 0, "a"), entry(1, 2, 0, "WRONG")],
    );
    f.log_entries("TC_B_execution_log.json", &[]);
    let container = compile(&root().join("data/testcase_results_container/schema.json"));
    let result_schema =
        compile(&root().join("schemas/tcms-schemas/tcms/verification-result.schema.json"));

    let mut reports = Vec::new();
    for format in ["yaml", "json"] {
        let path = f.path(&format!("out.{format}"));
        let out = f.folder(&[
            "--format",
            format,
            "--output",
            path.to_str().unwrap(),
            "--run-id",
            "RUN-1",
            "--build-id",
            "b-1",
            "--tester",
            "alice",
            "--environment",
            "Staging",
        ]);
        assert_eq!(code(&out), 1, "{}", stderr(&out));
        let text = std::fs::read_to_string(&path).unwrap();
        let report: Value = if format == "json" {
            serde_json::from_str(&text).unwrap()
        } else {
            serde_yaml::from_str(&text).unwrap()
        };
        if let Err(errors) = container.validate(&report) {
            panic!(
                "{format} report violates the container schema: {:?}",
                errors.map(|e| e.to_string()).collect::<Vec<_>>()
            );
        }
        for result in report["test_results"].as_array().unwrap() {
            if let Err(errors) = result_schema.validate(result) {
                panic!(
                    "{format} result {} violates verification-result schema: {:?}",
                    result["test_case_id"],
                    errors
                        .map(|e| format!("{e} at {}", e.instance_path))
                        .collect::<Vec<_>>()
                );
            }
            assert_eq!(result["run_id"], "RUN-1");
            assert_eq!(result["build_id"], "b-1");
            assert_eq!(result["tester"], "alice");
            assert_eq!(result["environment"], "Staging");
        }
        reports.push(report);
    }
    let strip = |mut v: Value| {
        v.as_object_mut().unwrap().remove("test_date");
        for r in v["test_results"].as_array_mut().unwrap() {
            r.as_object_mut().unwrap().remove("date");
        }
        v.as_object_mut().unwrap().remove("metadata");
        v
    };
    let json = strip(reports.pop().unwrap());
    let yaml = strip(reports.pop().unwrap());
    assert_eq!(yaml, json);
}

#[test]
fn an_unwritable_output_path_is_an_error() {
    let f = one_step("a", "a");
    let bad = f.path("no/such/dir/out.yaml");
    let out = f.single("TC_A", &["--output", bad.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("Failed to write output"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_missing_schema_file_is_an_error() {
    let f = one_step("a", "a");
    let log = f.path("TC_A_execution_log.json");
    let out = Command::new(env!("CARGO_BIN_EXE_verifier"))
        .current_dir(root())
        .args([
            "--test-case",
            "TC_A",
            "--log",
            log.to_str().unwrap(),
            "--schema",
            "/no/such/schema.json",
            "--test-case-dir",
        ])
        .arg(f.path("cases"))
        .output()
        .unwrap();
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("Schema file not found"),
        "{}",
        stderr(&out)
    );
}

// ---------------------------------------------------------------------------------------------
// configuration file and run metadata
// ---------------------------------------------------------------------------------------------

fn config(f: &Fixture, body: &str) -> String {
    let path = f.path("config.yaml");
    // title and project are required by the schema; tests that set them themselves win
    let mut defaults = String::new();
    for key in ["title", "project"] {
        if !body.contains(&format!("{key}:")) {
            defaults.push_str(&format!("{key}: default {key}\n"));
        }
    }
    std::fs::write(
        &path,
        format!(
            "type: container_config\nschema: tcms/container-config.schema.v1.json\n{defaults}{body}"
        ),
    )
    .unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn a_config_file_sets_the_report_metadata() {
    let f = one_step("a", "a");
    let cfg = config(&f, "title: From config\nplatform: Plan9\nrun_id: RUN-C\n");
    let (path, arg) = out_arg(&f);
    success(&f.single("TC_A", &["--config", &cfg, "--output", &arg]));
    let result = read_result(&path);
    assert_eq!(result["title"], "From config");
    assert_eq!(result["metadata"]["platform"], "Plan9");
    assert_eq!(first_result(&result)["run_id"], "RUN-C");
}

#[test]
fn a_config_file_with_an_unknown_key_or_wrong_type_is_refused() {
    let f = one_step("a", "a");
    for body in [
        "unknown_key: 1\n",
        "title: 42\n",
        "title: [a, b]\n",
        "run_id: {a: 1}\n",
    ] {
        let cfg = config(&f, body);
        let out = f.single("TC_A", &["--config", &cfg]);
        assert_eq!(code(&out), 1, "config {body:?} accepted: {}", stderr(&out));
        assert!(
            stderr(&out).contains("configuration file"),
            "{}",
            stderr(&out)
        );
    }
}

#[test]
fn a_missing_or_malformed_config_file_is_refused() {
    let f = one_step("a", "a");
    let missing = f.path("nope.yaml");
    assert_eq!(
        code(&f.single("TC_A", &["--config", missing.to_str().unwrap()])),
        1
    );
    f.log("config.yaml", "{{{ not yaml");
    let cfg = f.path("config.yaml");
    assert_eq!(
        code(&f.single("TC_A", &["--config", cfg.to_str().unwrap()])),
        1
    );
}

#[test]
fn run_metadata_precedence_and_blank_values() {
    let f = one_step("a", "a");
    let cfg = config(&f, "run_id: RUN-CFG\nbuild_id: B-CFG\ntester: cfg-tester\n");
    let (path, arg) = out_arg(&f);
    let log = f.path("TC_A_execution_log.json");
    let run = |env: &[(&str, &str)], flags: &[&str]| -> Value {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_verifier"));
        cmd.current_dir(root())
            .env_remove("TCMS_RUN_ID")
            .env_remove("TCMS_BUILD_ID")
            .env_remove("TCMS_TESTER")
            .args([
                "--test-case",
                "TC_A",
                "--log",
                log.to_str().unwrap(),
                "--config",
                &cfg,
            ])
            .args([
                "--schema",
                root()
                    .join("data/testcase_results_container/schema.json")
                    .to_str()
                    .unwrap(),
            ])
            .arg("--test-case-dir")
            .arg(f.path("cases"))
            .args(["--output", &arg])
            .args(flags);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let out = cmd.output().unwrap();
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        first_result(&read_result(&path)).clone()
    };

    let r = run(&[], &[]);
    assert_eq!(
        (
            r["run_id"].as_str(),
            r["build_id"].as_str(),
            r["tester"].as_str()
        ),
        (Some("RUN-CFG"), Some("B-CFG"), Some("cfg-tester"))
    );

    let r = run(
        &[("TCMS_RUN_ID", "RUN-ENV"), ("TCMS_TESTER", "env-tester")],
        &[],
    );
    assert_eq!(
        (r["run_id"].as_str(), r["tester"].as_str()),
        (Some("RUN-ENV"), Some("env-tester"))
    );

    let r = run(&[("TCMS_RUN_ID", "RUN-ENV")], &["--run-id", "RUN-FLAG"]);
    assert_eq!(r["run_id"], "RUN-FLAG");

    // blank flag or variable counts as absent and falls through
    let r = run(
        &[("TCMS_TESTER", "   ")],
        &["--run-id", "", "--build-id", "  "],
    );
    assert_eq!(
        (
            r["run_id"].as_str(),
            r["build_id"].as_str(),
            r["tester"].as_str()
        ),
        (Some("RUN-CFG"), Some("B-CFG"), Some("cfg-tester"))
    );

    // values are trimmed
    let r = run(&[], &["--run-id", "  RUN-PAD  "]);
    assert_eq!(r["run_id"], "RUN-PAD");
}

#[test]
fn blank_run_metadata_is_never_written_as_an_empty_value() {
    let f = one_step("a", "a");
    let (path, arg) = out_arg(&f);
    success(&f.single(
        "TC_A",
        &[
            "--run-id",
            "",
            "--build-id",
            " ",
            "--tester",
            "",
            "--output",
            &arg,
        ],
    ));
    let r = first_result(&read_result(&path)).clone();
    for key in ["run_id", "build_id", "tester"] {
        assert!(
            r.get(key).is_none() || r[key].is_null(),
            "{key} = {:?}",
            r[key]
        );
    }
}

#[test]
fn the_run_id_of_the_log_is_used_when_nothing_else_gives_one() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    let mut e = entry(1, 1, 0, "a");
    e["run_id"] = json!("RUN-LOG");
    f.log_entries("TC_A_execution_log.json", &[e]);
    let (path, arg) = out_arg(&f);
    success(&f.single("TC_A", &["--output", &arg]));
    assert_eq!(first_result(&read_result(&path))["run_id"], "RUN-LOG");
}

// ---------------------------------------------------------------------------------------------
// --test-run and --problem-report
// ---------------------------------------------------------------------------------------------

fn test_run_args<'a>(run: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "--run-id",
        "RUN-1",
        "--build-id",
        "b1",
        "--environment",
        "Staging",
        "--tester",
        "alice",
        "--test-run",
        run,
    ];
    args.extend_from_slice(extra);
    args
}

#[test]
fn test_run_needs_run_id_build_id_environment_and_tester() {
    let f = one_step("a", "a");
    let run = f.path("run.yaml");
    let run = run.to_str().unwrap();
    for missing in ["--run-id", "--build-id", "--environment", "--tester"] {
        let mut args = test_run_args(run, &[]);
        let at = args.iter().position(|a| *a == missing).unwrap();
        args.drain(at..at + 2);
        let out = f.single("TC_A", &args);
        assert_eq!(code(&out), 1, "without {missing}: {}", stderr(&out));
        assert!(
            !f.path("run.yaml").exists(),
            "a record was written without {missing}"
        );
    }
    success(&f.single("TC_A", &test_run_args(run, &[])));
    assert!(f.path("run.yaml").exists());
}

#[test]
fn problem_report_error_paths() {
    let f = one_step("a", "WRONG");
    let run = f.path("run.yaml");
    let run = run.to_str().unwrap();

    // malformed assignment
    let out = f.single("TC_A", &test_run_args(run, &["--problem-report", "TC_A"]));
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("TEST_CASE_ID=PROBLEM_REPORT_ID"),
        "{}",
        stderr(&out)
    );

    // a test case that is not in the run
    let out = f.single(
        "TC_A",
        &test_run_args(run, &["--problem-report", "TC_OTHER=PR-1"]),
    );
    assert_eq!(code(&out), 1, "{}", stderr(&out));

    // a failed case takes one; the exit code is still the verdict (1)
    let out = f.single(
        "TC_A",
        &test_run_args(run, &["--problem-report", "TC_A=PR-7"]),
    );
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let record = std::fs::read_to_string(f.path("run.yaml")).unwrap();
    assert!(
        record.contains("PR-7") && record.contains("verdict: fail"),
        "{record}"
    );
}

#[test]
fn problem_report_on_a_passing_case_is_refused() {
    let f = one_step("a", "a");
    let run = f.path("run.yaml");
    let out = f.single(
        "TC_A",
        &test_run_args(run.to_str().unwrap(), &["--problem-report", "TC_A=PR-7"]),
    );
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(!run.exists());
}

#[test]
fn test_run_verdicts_pass_fail_and_not_executed() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    f.case("TC_B", &[(1, 1, "0", "b")]);
    f.case("TC_C", &[(1, 1, "0", "c"), (1, 2, "0", "c2")]);
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, "a")]);
    f.log_entries("TC_B_execution_log.json", &[entry(1, 1, 0, "WRONG")]);
    f.log_entries("TC_C_execution_log.json", &[entry(1, 1, 0, "c")]);
    let run = f.path("run.yaml");
    let out = f.folder(&test_run_args(run.to_str().unwrap(), &[]));
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let record: Value = serde_yaml::from_str(&std::fs::read_to_string(&run).unwrap()).unwrap();
    let verdicts: std::collections::BTreeMap<String, String> = record["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["test_case_id"].as_str().unwrap().to_string(),
                r["verdict"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(verdicts["TC_A"], "pass");
    assert_eq!(verdicts["TC_B"], "fail");
    assert_eq!(verdicts["TC_C"], "not_executed");
}

#[test]
fn a_failed_hook_gives_an_error_verdict_in_the_test_run_record() {
    // docs/RUN_METADATA.md: `error` when a hook failed and ended the script
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    let mut hook = json!({"test_sequence": 0, "step": 0, "hook_type": "script_start",
        "command": "false", "exit_code": 127, "output": "", "timestamp": "2026-10-28T10:00:00",
        "result_verification_pass": false, "output_verification_pass": false});
    hook["test_case_id"] = json!("TC_A");
    f.log_entries("TC_A_execution_log.json", &[entry(1, 1, 0, "a"), hook]);
    let run = f.path("run.yaml");
    let out = f.folder(&test_run_args(run.to_str().unwrap(), &[]));
    // the steps all passed, but the errored case still fails the run (decision D-41e)
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let record: Value = serde_yaml::from_str(&std::fs::read_to_string(&run).unwrap()).unwrap();
    assert_eq!(record["results"][0]["verdict"], "error");
    // --success-on-completion forgives it, as it forgives a failed verdict
    success(&f.folder(&test_run_args(
        run.to_str().unwrap(),
        &["--success-on-completion"],
    )));
}

#[test]
fn build_id_comes_from_flag_then_environment_then_nothing() {
    let f = one_step("a", "a");
    let run = f.path("run.yaml");
    let log = f.path("TC_A_execution_log.json");
    let base = |extra: &[&str]| -> Vec<String> {
        let mut args: Vec<String> = [
            "--test-case",
            "TC_A",
            "--log",
            log.to_str().unwrap(),
            "--run-id",
            "RUN-1",
            "--environment",
            "Staging",
            "--tester",
            "alice",
            "--test-run",
            run.to_str().unwrap(),
        ]
        .iter()
        .map(|a| a.to_string())
        .collect();
        args.extend(extra.iter().map(|a| a.to_string()));
        args
    };
    // the record's build_id: flag, else TCMS_BUILD_ID; without either the record is refused
    let run_with = |env: Option<&str>, extra: &[&str]| -> Output {
        let _ = std::fs::remove_file(&run);
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_verifier"));
        cmd.current_dir(root())
            .env_remove("TCMS_RUN_ID")
            .env_remove("TCMS_BUILD_ID")
            .env_remove("TCMS_TESTER")
            .args(base(extra))
            .arg("--test-case-dir")
            .arg(f.path("cases"));
        if let Some(value) = env {
            cmd.env("TCMS_BUILD_ID", value);
        }
        cmd.output().unwrap()
    };
    let build_id = || -> Option<String> {
        let record: Value = serde_yaml::from_str(&std::fs::read_to_string(&run).unwrap()).unwrap();
        record["build_id"].as_str().map(str::to_string)
    };
    success(&run_with(Some("fw-env"), &[]));
    assert_eq!(build_id().as_deref(), Some("fw-env"));
    success(&run_with(Some("fw-env"), &["--build-id", "fw-flag"]));
    assert_eq!(build_id().as_deref(), Some("fw-flag"));
    let out = run_with(None, &[]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(stderr(&out).contains("build_id"), "{}", stderr(&out));
    assert!(!run.exists(), "a record without build_id was written");
}

#[test]
fn entries_that_name_another_test_case_are_not_evidence_for_this_one() {
    let f = Fixture::new();
    f.case("TC_A", &[(1, 1, "0", "a")]);
    let mut foreign = entry(1, 1, 0, "a");
    foreign["test_case_id"] = json!("TC_OTHER");
    f.log_entries("TC_A_execution_log.json", &[foreign]);
    let (path, arg) = out_arg(&f);
    assert_eq!(code(&f.single("TC_A", &["--output", &arg])), 1);
    assert_eq!(
        step_statuses(first_result(&read_result(&path))),
        ["NotExecuted"]
    );

    // entries that name this test case count
    let mut own = entry(1, 1, 0, "a");
    own["test_case_id"] = json!("TC_A");
    f.log_entries("TC_A_execution_log.json", &[own]);
    success(&f.single("TC_A", &[]));
}

#[test]
fn skipped_logs_stay_an_error_with_success_on_completion() {
    let f = Fixture::new();
    f.log_entries("TC_GHOST_execution_log.json", &[entry(1, 1, 0, "a")]);
    let out = f.folder(&["--success-on-completion"]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
}
