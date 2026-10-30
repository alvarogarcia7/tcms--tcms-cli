//! Lenient loaders used by `trace-matrix` and `regression-select`, moved unchanged from
//! `test-executor` (D-11, D-14). They skip non-run YAML silently on purpose; `report` must not
//! use them, it uses the strict loaders in `load`.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use testcase_common::load_and_validate_yaml;
use testcase_models::TestCase;

pub fn load_test_case(yaml_file: &PathBuf) -> Result<TestCase> {
    let test_case =
        load_and_validate_yaml::<TestCase, _, _>(yaml_file, "schemas/").context(format!(
            "Failed to load and validate YAML file: {}",
            yaml_file.display()
        ))?;
    test_case
        .validate_classification()
        .map_err(|e| anyhow::anyhow!("{}: {}", yaml_file.display(), e))?;
    Ok(test_case)
}

/// Read test run records from files and directories; other YAML files are skipped
pub fn read_test_runs(paths: &[PathBuf]) -> Result<Vec<testcase_verification::TestRunRecord>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            for entry in fs::read_dir(path)
                .context(format!("Failed to read directory: {}", path.display()))?
            {
                files.push(entry?.path());
            }
        } else {
            files.push(path.clone());
        }
    }
    files.sort();

    let mut runs = Vec::new();
    for file in files {
        let is_yaml = matches!(
            file.extension().and_then(|e| e.to_str()),
            Some("yml") | Some("yaml")
        );
        if !file.is_file() || !is_yaml {
            continue;
        }
        let content = fs::read_to_string(&file)
            .context(format!("Failed to read test run: {}", file.display()))?;
        match serde_yaml::from_str::<testcase_verification::TestRunRecord>(&content) {
            Ok(run) if run.doc_type == "test_run" => runs.push(run),
            Ok(_) => {}
            Err(e) if paths.contains(&file) => {
                anyhow::bail!("{} is not a test run record: {}", file.display(), e)
            }
            Err(_) => {}
        }
    }
    Ok(runs)
}

/// Load every test case below `dir`. A coverage claim must not rest on test cases that could
/// not be read, so any file that fails to load fails the whole load, naming each one.
pub fn load_test_cases_strict(dir: &Path) -> Result<Vec<TestCase>> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in
            fs::read_dir(dir).context(format!("Failed to read directory: {}", dir.display()))?
        {
            let path = entry?.path();
            if path.is_dir() {
                walk(&path, files)?;
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yml") | Some("yaml")
            ) {
                files.push(path);
            }
        }
        Ok(())
    }

    if !dir.is_dir() {
        anyhow::bail!("Not a directory: {}", dir.display());
    }
    let mut files = Vec::new();
    walk(dir, &mut files)?;
    files.sort();

    let mut test_cases = Vec::new();
    let mut failures = Vec::new();
    for file in &files {
        match load_test_case(file) {
            Ok(test_case) => test_cases.push(test_case),
            Err(e) => failures.push(format!("  {}: {:#}", file.display(), e)),
        }
    }
    if !failures.is_empty() {
        anyhow::bail!(
            "{} of {} YAML files below {} are not valid test cases (the directory must hold \
             test cases only):\n{}",
            failures.len(),
            files.len(),
            dir.display(),
            failures.join("\n")
        );
    }
    Ok(test_cases)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: &str =
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: RUN-7\nbuild_id: b\n\
         environment: e\ntester: t\nstarted_at: '2026-10-28T10:00:00Z'\n\
         results:\n  - {test_case_id: N1, verdict: fail}\n";

    #[test]
    fn directory_scan_skips_non_run_yaml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("run.yaml"), RUN).unwrap();
        fs::write(dir.path().join("notes.yaml"), "just: some other yaml\n").unwrap();
        let runs = read_test_runs(&[dir.path().to_path_buf()]).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_id, "RUN-7");
    }

    #[test]
    fn named_non_run_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("notes.yaml");
        fs::write(&f, "just: some other yaml\n").unwrap();
        let e = read_test_runs(&[f]).unwrap_err().to_string();
        assert!(e.contains("is not a test run record"), "{e}");
    }
}
