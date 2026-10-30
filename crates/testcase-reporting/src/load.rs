//! Strict, fail-closed loaders for `test-executor report` and the reconciliation block
//! (D-06, D-07, D-08, D-15, D-17, D-18). Implemented in plan 01-02.
//! Unlike the legacy loaders in `legacy`, nothing is skipped silently: every discovered file is
//! either loaded or listed in the [`Reconciliation`] with a reason.

use crate::run::LoadedRun;
use std::path::{Path, PathBuf};
use testcase_verification::{TestRunRecord, TraceMatrix};

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

impl std::fmt::Display for Reconciliation {
    /// The reconciliation block printed in the report header (fixed layout)
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Reconciliation")?;
        writeln!(
            f,
            "  Inputs discovered: {} (loaded: {}, skipped: {})",
            self.discovered(),
            self.loaded(),
            self.skipped()
        )?;
        for entry in &self.inputs {
            let (status, reason) = match &entry.status {
                InputStatus::Loaded => ("loaded", None),
                InputStatus::Skipped(reason) => ("skipped", Some(reason)),
            };
            let kind = match entry.kind {
                InputKind::Matrix => "matrix",
                InputKind::Run => "run",
            };
            write!(f, "  {:<8} {:<7} {}", status, kind, entry.path.display())?;
            match (&entry.run_id, reason) {
                (Some(run_id), _) => write!(f, " ({run_id})")?,
                (None, Some(reason)) => write!(f, ": {reason}")?,
                (None, None) => {}
            }
            writeln!(f)?;
        }
        if self.notes.is_empty() {
            writeln!(f, "  Notes: none")?;
        } else {
            writeln!(f, "  Notes:")?;
            for note in &self.notes {
                writeln!(f, "    - {note}")?;
            }
        }
        Ok(())
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
///
/// Every bad input is collected and reported in one error, each failure on its own line starting
/// with the offending path. A directory is scanned without recursion; YAML files in it that are
/// not test run records are skipped with a reason, explicitly named files are not.
pub fn load_inputs(
    matrix: &Path,
    run_paths: &[PathBuf],
    schemas_root: &str,
) -> anyhow::Result<LoadedInputs> {
    let mut failures: Vec<String> = Vec::new();
    let mut inputs = Vec::new();

    let loaded_matrix = match load_matrix(matrix, schemas_root) {
        Ok(m) => {
            inputs.push(InputEntry {
                path: matrix.to_path_buf(),
                kind: InputKind::Matrix,
                status: InputStatus::Loaded,
                run_id: None,
            });
            Some(m)
        }
        Err(e) => {
            failures.push(format!("{e:#}"));
            None
        }
    };

    let mut runs = Vec::new();
    for candidate in discover_runs(run_paths, &mut failures) {
        match load_run(&candidate.path, candidate.from_dir, schemas_root) {
            Ok(RunOutcome::Loaded(run)) => {
                inputs.push(InputEntry {
                    path: candidate.path.clone(),
                    kind: InputKind::Run,
                    status: InputStatus::Loaded,
                    run_id: Some(run.record.run_id.clone()),
                });
                runs.push(*run);
            }
            Ok(RunOutcome::Skipped(reason)) => inputs.push(InputEntry {
                path: candidate.path.clone(),
                kind: InputKind::Run,
                status: InputStatus::Skipped(reason),
                run_id: None,
            }),
            Err(e) => failures.push(format!("{e:#}")),
        }
    }

    if !failures.is_empty() {
        let lines: Vec<String> = failures.iter().map(|f| single_line(f)).collect();
        anyhow::bail!(
            "{} report input(s) failed to load:\n{}",
            failures.len(),
            lines.join("\n")
        );
    }
    let matrix_path = matrix.to_path_buf();
    let matrix = loaded_matrix.expect("a matrix failure is recorded above");

    check_unique_run_ids(&runs)?;
    let notes = matrix_notes(&matrix, &runs);

    Ok(LoadedInputs {
        matrix_path,
        matrix,
        runs,
        reconciliation: Reconciliation { inputs, notes },
    })
}

/// D-18: mismatches between the matrix and the loaded runs are notes, not failures.
/// First the run IDs the matrix names but which were not loaded, then per run (by `run_id`) the
/// test cases it reports that the matrix does not trace.
pub fn matrix_notes(matrix: &TraceMatrix, runs: &[LoadedRun]) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut notes = Vec::new();

    let loaded: BTreeSet<&str> = runs.iter().map(|r| r.record.run_id.as_str()).collect();
    let referenced: BTreeSet<&str> = matrix
        .links
        .iter()
        .flat_map(|link| link.run_ids.iter().map(String::as_str))
        .collect();
    for run_id in referenced.difference(&loaded) {
        notes.push(format!(
            "matrix references run {run_id}, which was not loaded"
        ));
    }

    let traced: BTreeSet<&str> = matrix
        .links
        .iter()
        .flat_map(|link| link.test_case_ids.iter().map(String::as_str))
        .collect();
    let mut by_run: Vec<&LoadedRun> = runs.iter().collect();
    by_run.sort_by(|a, b| a.record.run_id.cmp(&b.record.run_id));
    for run in by_run {
        let untraced: BTreeSet<&str> = run
            .record
            .results
            .iter()
            .map(|r| r.test_case_id.as_str())
            .filter(|id| !traced.contains(id))
            .collect();
        if !untraced.is_empty() {
            let ids: Vec<&str> = untraced.iter().copied().collect();
            notes.push(format!(
                "run {} has {} result(s) for test cases not in the matrix: {}",
                run.record.run_id,
                untraced.len(),
                ids.join(", ")
            ));
        }
    }
    notes
}

/// D-08: two records with the same `run_id` fail the load, naming every file that carries it
pub fn check_unique_run_ids(runs: &[LoadedRun]) -> anyhow::Result<()> {
    let mut by_id: std::collections::BTreeMap<&str, Vec<&Path>> = Default::default();
    for run in runs {
        by_id
            .entry(run.record.run_id.as_str())
            .or_default()
            .push(run.path.as_path());
    }
    let lines: Vec<String> = by_id
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(id, paths)| {
            let names: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
            format!("duplicate run_id {id} in {}", names.join(" and "))
        })
        .collect();
    if lines.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("{}", lines.join("\n"))
    }
}

/// Collapse a multi-line failure into one line, so that every failure starts with its path
fn single_line(failure: &str) -> String {
    failure
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strict matrix load: the type and schema are checked before the schema-validated parse
fn load_matrix(path: &Path, schemas_root: &str) -> anyhow::Result<TraceMatrix> {
    use anyhow::{bail, Context};
    if !path.is_file() {
        bail!("{}: trace matrix not found", path.display());
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("{}: cannot read trace matrix", path.display()))?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .with_context(|| format!("{}: YAML syntax error", path.display()))?;
    match value.get("type").and_then(|v| v.as_str()) {
        Some("trace_matrix") => {}
        other => bail!(
            "{} is not a trace matrix (type '{}')",
            path.display(),
            other.unwrap_or("none")
        ),
    }
    let schema = value
        .get("schema")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if schema != MATRIX_SCHEMA {
        bail!(
            "{}: declares schema '{schema}', expected {MATRIX_SCHEMA}",
            path.display()
        );
    }
    testcase_common::load_and_validate_yaml::<TraceMatrix, _, _>(path, schemas_root)
        .map_err(|e| anyhow::anyhow!("{}: {e:#}", path.display()))
}

/// A file found by discovery. `from_dir` is true when it came from scanning a directory.
struct Candidate {
    path: PathBuf,
    from_dir: bool,
}

/// Find the run candidates: named files, and the files of named directories (not recursive).
/// A file named both directly and through its directory is listed once, as named.
fn discover_runs(run_paths: &[PathBuf], failures: &mut Vec<String>) -> Vec<Candidate> {
    use std::collections::BTreeMap;
    let mut found: BTreeMap<PathBuf, Candidate> = BTreeMap::new();

    fn add(found: &mut BTreeMap<PathBuf, Candidate>, path: PathBuf, from_dir: bool) {
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        match found.get_mut(&key) {
            Some(existing) => {
                if !from_dir {
                    existing.path = path;
                    existing.from_dir = false;
                }
            }
            None => {
                found.insert(key, Candidate { path, from_dir });
            }
        }
    }

    for given in run_paths {
        if given.is_dir() {
            match std::fs::read_dir(given) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => add(&mut found, entry.path(), true),
                            Err(e) => failures.push(format!(
                                "{}: cannot read runs directory entry: {e}",
                                given.display()
                            )),
                        }
                    }
                }
                Err(e) => failures.push(format!(
                    "{}: cannot read runs directory: {e}",
                    given.display()
                )),
            }
        } else if given.exists() {
            add(&mut found, given.clone(), false);
        } else {
            failures.push(format!("{}: run input not found", given.display()));
        }
    }

    let mut candidates: Vec<Candidate> = found.into_values().collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    candidates
}

enum RunOutcome {
    Loaded(Box<LoadedRun>),
    Skipped(String),
}

/// Strict run load (D-06, D-15, D-17). Directory files that are not run records are skipped
/// with a reason; the same files named explicitly are failures.
fn load_run(path: &Path, from_dir: bool, schemas_root: &str) -> anyhow::Result<RunOutcome> {
    use anyhow::{anyhow, bail, Context};
    use std::collections::BTreeMap;

    if path.is_dir() {
        return Ok(RunOutcome::Skipped(
            "directory; runs directories are not searched recursively".to_string(),
        ));
    }
    let is_yaml = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml"))
        .unwrap_or(false);
    if !is_yaml {
        if from_dir {
            return Ok(RunOutcome::Skipped("not a YAML file".to_string()));
        }
        bail!("{} is not a YAML file", path.display());
    }

    let text = std::fs::read_to_string(path)
        .with_context(|| format!("{}: cannot read run record", path.display()))?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .with_context(|| format!("{}: YAML syntax error", path.display()))?;

    match value.get("type").and_then(|v| v.as_str()) {
        Some("test_run") => {}
        Some(other) if from_dir => {
            return Ok(RunOutcome::Skipped(format!(
                "type is '{other}', not test_run"
            )))
        }
        Some(other) => bail!(
            "{} is not a test run record (type '{other}')",
            path.display()
        ),
        None if from_dir => return Ok(RunOutcome::Skipped("no `type` field".to_string())),
        None => bail!(
            "{} is not a test run record (no `type` field)",
            path.display()
        ),
    }

    let schema = value
        .get("schema")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    if schema != RUN_SCHEMA {
        bail!(
            "{}: declares schema '{schema}', expected {RUN_SCHEMA}",
            path.display()
        );
    }

    let record: TestRunRecord =
        testcase_common::load_and_validate_yaml::<TestRunRecord, _, _>(path, schemas_root)
            .map_err(|e| anyhow!("{}: {e:#}", path.display()))?;

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for result in &record.results {
        *counts.entry(result.test_case_id.as_str()).or_default() += 1;
    }
    if let Some((id, n)) = counts.iter().find(|(_, n)| **n > 1) {
        bail!(
            "{}: test case {id} appears {n} times in run {}",
            path.display(),
            record.run_id
        );
    }

    Ok(RunOutcome::Loaded(Box::new(LoadedRun::new(
        path.to_path_buf(),
        record,
    )?)))
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

        let loaded = load_inputs(&matrix, std::slice::from_ref(&runs), SCHEMAS).unwrap();

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
        let e = err_text(load_inputs(&matrix, std::slice::from_ref(&run), SCHEMAS));
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
        let e = err_text(load_inputs(&matrix, std::slice::from_ref(&run), SCHEMAS));
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
        let e = err_text(load_inputs(&matrix, std::slice::from_ref(&run), SCHEMAS));
        assert!(
            // The schema's date-time format rejects the value before LoadedRun::new sees it
            e.contains(&run.display().to_string())
                && (e.contains("RFC 3339") || e.contains("date-time")),
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
        let e = err_text(load_inputs(&matrix, std::slice::from_ref(&run), SCHEMAS));
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
        let e = err_text(load_inputs(&matrix, std::slice::from_ref(&notes), SCHEMAS));
        assert!(
            e.contains(&notes.display().to_string()) && e.contains("is not a test run record"),
            "{e}"
        );
    }

    #[test]
    fn missing_run_path_fails() {
        let (dir, matrix, _runs) = setup();
        let missing = dir.path().join("nowhere");
        let e = err_text(load_inputs(
            &matrix,
            std::slice::from_ref(&missing),
            SCHEMAS,
        ));
        assert!(
            e.contains(&missing.display().to_string()) && e.contains("not found"),
            "{e}"
        );
    }

    #[test]
    fn matrix_referencing_unloaded_run_gets_a_note() {
        let dir = tempfile::tempdir().unwrap();
        let matrix = write(dir.path(), "matrix.yaml", &matrix_text(&["RUN-1", "RUN-9"]));
        let runs = dir.path().join("runs");
        fs::create_dir(&runs).unwrap();
        write(&runs, "run.yaml", &valid_run("RUN-1"));
        let loaded = load_inputs(&matrix, &[runs], SCHEMAS).unwrap();
        assert!(
            loaded
                .reconciliation
                .notes
                .contains(&"matrix references run RUN-9, which was not loaded".to_string()),
            "{:?}",
            loaded.reconciliation.notes
        );
    }

    #[test]
    fn results_outside_the_matrix_get_a_note() {
        let (_dir, matrix, runs) = setup();
        write(
            &runs,
            "run.yaml",
            &run_text("RUN-1", "2026-10-28T10:00:00Z", &["T1", "T8", "T9"]),
        );
        let loaded = load_inputs(&matrix, &[runs], SCHEMAS).unwrap();
        assert!(
            loaded.reconciliation.notes.contains(
                &"run RUN-1 has 2 result(s) for test cases not in the matrix: T8, T9".to_string()
            ),
            "{:?}",
            loaded.reconciliation.notes
        );
    }

    #[test]
    fn notes_never_fail_the_load() {
        let (_dir, matrix, runs) = setup();
        write(
            &runs,
            "run.yaml",
            &run_text("RUN-1", "2026-10-28T10:00:00Z", &["T1", "T8"]),
        );
        assert!(load_inputs(&matrix, &[runs], SCHEMAS).is_ok());
    }

    #[test]
    fn reconciliation_block_layout_is_fixed() {
        let rec = Reconciliation {
            inputs: vec![
                InputEntry {
                    path: PathBuf::from("/tmp/x/matrix.yaml"),
                    kind: InputKind::Matrix,
                    status: InputStatus::Loaded,
                    run_id: None,
                },
                InputEntry {
                    path: PathBuf::from("/tmp/x/runs/run.yaml"),
                    kind: InputKind::Run,
                    status: InputStatus::Loaded,
                    run_id: Some("RUN-1".to_string()),
                },
                InputEntry {
                    path: PathBuf::from("/tmp/x/runs/notes.yaml"),
                    kind: InputKind::Run,
                    status: InputStatus::Skipped("no `type` field".to_string()),
                    run_id: None,
                },
                InputEntry {
                    path: PathBuf::from("/tmp/x/runs/readme.txt"),
                    kind: InputKind::Run,
                    status: InputStatus::Skipped("not a YAML file".to_string()),
                    run_id: None,
                },
            ],
            notes: vec!["matrix references run RUN-9, which was not loaded".to_string()],
        };
        let expected = "Reconciliation
  Inputs discovered: 4 (loaded: 2, skipped: 2)
  loaded   matrix  /tmp/x/matrix.yaml
  loaded   run     /tmp/x/runs/run.yaml (RUN-1)
  skipped  run     /tmp/x/runs/notes.yaml: no `type` field
  skipped  run     /tmp/x/runs/readme.txt: not a YAML file
  Notes:
    - matrix references run RUN-9, which was not loaded
";
        assert_eq!(rec.to_string(), expected);
    }

    #[test]
    fn reconciliation_without_notes_says_none() {
        let rec = Reconciliation {
            inputs: vec![InputEntry {
                path: PathBuf::from("m.yaml"),
                kind: InputKind::Matrix,
                status: InputStatus::Loaded,
                run_id: None,
            }],
            notes: Vec::new(),
        };
        assert!(
            rec.to_string().ends_with("  Notes: none\n"),
            "{}",
            rec.to_string()
        );
    }
}
