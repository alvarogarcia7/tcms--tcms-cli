//! Layer 3: load, select (later: model, render) for `test-executor report`.
//! Only `test-executor` depends on this crate.

pub mod legacy;
pub mod load;
pub mod run;
pub mod select;

pub use run::LoadedRun;

/// The text block `test-executor report` prints before the report kinds: which matrix was
/// loaded, the reconciliation of every discovered input, and the run selection.
pub fn input_summary(inputs: &load::LoadedInputs, selection: &select::RunSelection) -> String {
    let baseline = inputs
        .matrix
        .baseline_id
        .as_deref()
        .unwrap_or("not recorded");
    format!(
        "Trace matrix: {} (baseline: {})\n\n{}\n{}",
        inputs.matrix.id, baseline, inputs.reconciliation, selection
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::test_support::record;
    use testcase_verification::{TraceMatrix, Verdict};

    fn matrix(baseline: Option<&str>) -> TraceMatrix {
        TraceMatrix {
            doc_type: "trace_matrix".to_string(),
            schema: "tcms/trace-matrix.schema.json".to_string(),
            id: "TM-1".to_string(),
            baseline_id: baseline.map(str::to_string),
            links: Vec::new(),
        }
    }

    #[test]
    fn summary_starts_with_matrix_line_and_baseline_placeholder() {
        let inputs = load::LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: matrix(None),
            runs: Vec::new(),
            reconciliation: load::Reconciliation {
                inputs: Vec::new(),
                notes: Vec::new(),
            },
        };
        let selection = select::select_results(&inputs.runs);
        let text = input_summary(&inputs, &selection);
        assert_eq!(
            text.lines().next(),
            Some("Trace matrix: TM-1 (baseline: not recorded)")
        );
        assert!(text.contains("Reconciliation"));
        assert!(text.contains("Run selection"));
    }

    #[test]
    fn summary_names_recorded_baseline() {
        let inputs = load::LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: matrix(Some("BL-7")),
            runs: vec![run::LoadedRun::new(
                "run.yaml".into(),
                record(
                    "RUN-1",
                    "b1",
                    "2026-01-01T10:00:00Z",
                    &[("T1", Verdict::Pass)],
                ),
            )
            .expect("valid run")],
            reconciliation: load::Reconciliation {
                inputs: Vec::new(),
                notes: Vec::new(),
            },
        };
        let selection = select::select_results(&inputs.runs);
        let text = input_summary(&inputs, &selection);
        assert_eq!(
            text.lines().next(),
            Some("Trace matrix: TM-1 (baseline: BL-7)")
        );
    }
}
