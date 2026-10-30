//! Strict, fail-closed loaders for `test-executor report` and the reconciliation block
//! (D-06, D-07, D-08, D-15, D-17, D-18). Implemented in plan 01-02.
//! Unlike the legacy loaders in `legacy`, nothing is skipped silently: every discovered file is
//! either loaded or listed in the [`Reconciliation`] with a reason.

use crate::run::LoadedRun;
use std::path::{Path, PathBuf};
use testcase_verification::TraceMatrix;

/// Schema every run record must declare (D-06)
pub const RUN_SCHEMA: &str = "tcms/test-run.schema.json";
/// Schema every trace matrix must declare (D-06)
pub const MATRIX_SCHEMA: &str = "tcms/trace-matrix.schema.json";

/// What kind of input a file was
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Matrix,
    Run,
}

/// Whether a discovered file was loaded, or skipped with a reason
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputStatus {
    Loaded,
    Skipped(String),
}

/// One discovered file in the reconciliation
#[derive(Debug, Clone)]
pub struct InputEntry {
    pub path: PathBuf,
    pub kind: InputKind,
    pub status: InputStatus,
    /// The `run_id` of a loaded run
    pub run_id: Option<String>,
}

/// Every discovered input, and the notes about how the matrix and the runs fit together
#[derive(Debug, Clone, Default)]
pub struct Reconciliation {
    /// Matrix first, then runs in path order
    pub inputs: Vec<InputEntry>,
    /// Informational notes; they never fail a report
    pub notes: Vec<String>,
}

impl Reconciliation {
    /// Number of discovered inputs
    pub fn discovered(&self) -> usize {
        self.inputs.len()
    }

    /// Number of inputs that were loaded
    pub fn loaded(&self) -> usize {
        self.inputs
            .iter()
            .filter(|e| e.status == InputStatus::Loaded)
            .count()
    }

    /// Number of inputs that were skipped
    pub fn skipped(&self) -> usize {
        self.inputs
            .iter()
            .filter(|e| matches!(e.status, InputStatus::Skipped(_)))
            .count()
    }
}

/// Everything `report` loads, all of it validated
#[derive(Debug, Clone)]
pub struct LoadedInputs {
    pub matrix_path: PathBuf,
    pub matrix: TraceMatrix,
    /// Sorted by path
    pub runs: Vec<LoadedRun>,
    pub reconciliation: Reconciliation,
}

/// Load one trace matrix and the run records named by `run_paths` (files or directories).
pub fn load_inputs(
    matrix: &Path,
    run_paths: &[PathBuf],
    schemas_root: &str,
) -> anyhow::Result<LoadedInputs> {
    let _ = (matrix, run_paths, schemas_root);
    unimplemented!("load_inputs: implemented in the GREEN step")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SCHEMAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/");

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn matrix_text(run_ids: &[&str]) -> String {
        let run_ids = run_ids.join(", ");
        format!(
            "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\nlinks:\n  \
             - requirement_id: HLR-1\n    level: HLR\n    test_case_ids: [T1]\n    \
             normal_range_covered: true\n    robustness_covered: false\n    \
             run_ids: [{run_ids}]\n"
        )
    }

    fn run_text(run_id: &str, started_at: &str, test_case_ids: &[&str]) -> String {
        let mut text = format!(
            "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: {run_id}\n\
             build_id: b\nenvironment: e\ntester: t\nstarted_at: '{started_at}'\nresults:\n"
        );
        for id in test_case_ids {
            text.push_str(&format!("  - {{test_case_id: {id}, verdict: pass}}\n"));
        }
        text
    }

    fn valid_run(run_id: &str) -> String {
        run_text(run_id, "2026-10-28T10:00:00Z", &["T1"])
    }

    /// A matrix in `dir` and an empty `runs` directory next to it
    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let matrix = write(dir.path(), "matrix.yaml", &matrix_text(&["RUN-1"]));
        let runs = dir.path().join("runs");
        fs::create_dir(&runs).unwrap();
        (dir, matrix, runs)
    }

    fn err_text(result: anyhow::Result<LoadedInputs>) -> String {
        match result {
            Ok(_) => panic!("expected the load to fail"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn skipped_reason(inputs: &LoadedInputs, name: &str) -> String {
        let entry = inputs
            .reconciliation
            .inputs
            .iter()
            .find(|e| e.path.file_name().unwrap() == name)
            .unwrap_or_else(|| panic!("no reconciliation entry for {name}"));
        match &entry.status {
            InputStatus::Skipped(reason) => reason.clone(),
            other => panic!("{name} should be skipped, was {other:?}"),
        }
    }

    #[test]
    fn valid_inputs_are_reconciled_with_reasons() {
        let (_dir, matrix, runs) = setup();
        write(&runs, "run.yaml", &valid_run("RUN-1"));
        write(&runs, "notes.yaml", "just: some other yaml\n");
        write(
            &runs,
            "other.yaml",
            "type: test_case\nschema: tcms/test-case.schema.v1.json\ntest_case_id: T9\n",
        );
        write(&runs, "readme.txt", "not yaml");

        let loaded = load_inputs(&matrix, &[runs.clone()], SCHEMAS).unwrap();

        assert_eq!(loaded.runs.len(), 1);
        assert_eq!(loaded.runs[0].record.run_id, "RUN-1");
        assert_eq!(loaded.reconciliation.discovered(), 5);
        assert_eq!(loaded.reconciliation.loaded(), 2);
        assert_eq!(loaded.reconciliation.skipped(), 3);
        assert_eq!(skipped_reason(&loaded, "notes.yaml"), "no `type` field");
        assert_eq!(
            skipped_reason(&loaded, "other.yaml"),
            "type is 'test_case', not test_run"
        );
        assert_eq!(skipped_reason(&loaded, "readme.txt"), "not a YAML file");
    }

    #[test]
    fn syntax_error_in_runs_directory_fails() {
        let (_dir, matrix, runs) = setup();
        write(&runs, "broken.yaml", "a: [unclosed\n");
        let e = err_text(load_inputs(&matrix, &[runs], SCHEMAS));
        assert!(
            e.contains("broken.yaml") && e.contains("YAML syntax error"),
            "{e}"
        );
    }

    #[test]
    fn run_missing_tester_fails_naming_the_run() {
        let (_dir, matrix, runs) = setup();
        let run = write(
            &runs,
            "no_tester.yaml",
            "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: RUN-1\n\
             build_id: b\nenvironment: e\nstarted_at: '2026-10-28T10:00:00Z'\nresults: []\n",
        );
        let e = err_text(load_inputs(&matrix, &[run.clone()], SCHEMAS));
        assert!(e.contains(&run.display().to_string()), "{e}");
    }

    #[test]
    fn matrix_missing_links_fails_naming_the_matrix() {
        let dir = tempfile::tempdir().unwrap();
        let matrix = write(
            dir.path(),
            "matrix.yaml",
            "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\n",
        );
        let e = err_text(load_inputs(&matrix, &[], SCHEMAS));
        assert!(e.contains(&matrix.display().to_string()), "{e}");
    }

    #[test]
    fn test_run_passed_as_matrix_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let matrix = write(dir.path(), "matrix.yaml", &valid_run("RUN-1"));
        let e = err_text(load_inputs(&matrix, &[], SCHEMAS));
        assert!(
            e.contains(&matrix.display().to_string()) && e.contains("is not a trace matrix"),
            "{e}"
        );
    }

    #[test]
    fn run_declaring_matrix_schema_is_rejected() {
        let (_dir, matrix, runs) = setup();
        let run = write(
            &runs,
            "wrong_schema.yaml",
            &valid_run("RUN-1")
                .replace("tcms/test-run.schema.json", "tcms/trace-matrix.schema.json"),
        );
        let e = err_text(load_inputs(&matrix, &[run.clone()], SCHEMAS));
        assert!(
            e.contains(&run.display().to_string())
                && e.contains("expected tcms/test-run.schema.json"),
            "{e}"
        );
    }

    #[test]
    fn all_bad_inputs_are_reported_together() {
        let dir = tempfile::tempdir().unwrap();
        let matrix = write(
            dir.path(),
            "matrix.yaml",
            "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\n",
        );
        let bad1 = write(
            dir.path(),
            "bad1.yaml",
            "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: R1\nbuild_id: b\n\
             environment: e\nstarted_at: '2026-10-28T10:00:00Z'\nresults: []\n",
        );
        let bad2 = write(
            dir.path(),
            "bad2.yaml",
            "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: R2\nenvironment: e\n\
             tester: t\nstarted_at: '2026-10-28T10:00:00Z'\nresults: []\n",
        );
        let e = err_text(load_inputs(&matrix, &[bad1.clone(), bad2.clone()], SCHEMAS));
        assert!(e.contains("3 report input(s) failed to load"), "{e}");
        for path in [&matrix, &bad1, &bad2] {
            let prefix = path.display().to_string();
            assert!(
                e.lines().any(|line| line.starts_with(&prefix)),
                "no failure line starts with {prefix}: {e}"
            );
        }
    }

    #[test]
    fn non_rfc3339_started_at_fails_naming_the_run() {
        let (_dir, matrix, runs) = setup();
        let run = write(&runs, "run.yaml", &run_text("RUN-1", "yesterday", &["T1"]));
        let e = err_text(load_inputs(&matrix, &[run.clone()], SCHEMAS));
        assert!(
            e.contains(&run.display().to_string()) && e.contains("RFC 3339"),
            "{e}"
        );
    }

    #[test]
    fn duplicate_run_id_names_both_files() {
        let (_dir, matrix, runs) = setup();
        write(&runs, "a.yaml", &valid_run("RUN-1"));
        write(&runs, "b.yaml", &valid_run("RUN-1"));
        let e = err_text(load_inputs(&matrix, &[runs], SCHEMAS));
        assert!(
            e.contains("duplicate run_id RUN-1") && e.contains("a.yaml") && e.contains("b.yaml"),
            "{e}"
        );
    }

    #[test]
    fn same_file_named_twice_is_counted_once() {
        let (_dir, matrix, runs) = setup();
        let run = write(&runs, "run.yaml", &valid_run("RUN-1"));
        let loaded = load_inputs(&matrix, &[run.clone(), runs.clone()], SCHEMAS).unwrap();
        assert_eq!(loaded.runs.len(), 1);
        assert_eq!(loaded.reconciliation.discovered(), 2);
        assert_eq!(loaded.reconciliation.loaded(), 2);
    }

    #[test]
    fn duplicate_test_case_in_one_run_fails() {
        let (_dir, matrix, runs) = setup();
        let run = write(
            &runs,
            "run.yaml",
            &run_text("RUN-1", "2026-10-28T10:00:00Z", &["T1", "T1"]),
        );
        let e = err_text(load_inputs(&matrix, &[run.clone()], SCHEMAS));
        assert!(
            e.contains(&run.display().to_string())
                && e.contains("T1")
                && e.contains("appears 2 times"),
            "{e}"
        );
    }

    #[test]
    fn explicit_non_run_file_fails() {
        let (_dir, matrix, runs) = setup();
        let notes = write(&runs, "notes.yaml", "just: some other yaml\n");
        let e = err_text(load_inputs(&matrix, &[notes.clone()], SCHEMAS));
        assert!(
            e.contains(&notes.display().to_string()) && e.contains("is not a test run record"),
            "{e}"
        );
    }

    #[test]
    fn missing_run_path_fails() {
        let (dir, matrix, _runs) = setup();
        let missing = dir.path().join("nowhere");
        let e = err_text(load_inputs(&matrix, &[missing.clone()], SCHEMAS));
        assert!(
            e.contains(&missing.display().to_string()) && e.contains("not found"),
            "{e}"
        );
    }
}
