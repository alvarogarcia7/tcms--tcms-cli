//! Layer 3: load, select (later: model, render) for `test-executor report`.
//! Only `test-executor` depends on this crate.

pub mod gaps;
pub mod legacy;
pub mod load;
pub mod model;
pub mod problems;
pub mod render_markdown;
pub mod render_shapes;
pub mod rollup;
pub mod run;
pub mod select;
pub mod summary;
pub mod view;

pub use run::LoadedRun;

/// `Some(value)` when the value has visible text; `None` when it is absent, empty or whitespace-only.
/// A value that is kept is returned unchanged.
pub fn non_blank(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.trim().is_empty())
}

/// The text block `test-executor report` prints before the report kinds: which matrix was
/// loaded, the reconciliation of every discovered input, and the run selection.
pub fn input_summary(inputs: &load::LoadedInputs, selection: &select::RunSelection) -> String {
    let matrix_id = non_blank(Some(&inputs.matrix.id)).unwrap_or("not recorded");
    let baseline = non_blank(inputs.matrix.baseline_id.as_deref()).unwrap_or("not recorded");
    format!(
        "Trace matrix: {} (baseline: {})\n\n{}\n{}",
        matrix_id, baseline, inputs.reconciliation, selection
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

    fn summary_for(matrix: TraceMatrix) -> String {
        let inputs = load::LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix,
            runs: Vec::new(),
            reconciliation: load::Reconciliation {
                inputs: Vec::new(),
                notes: Vec::new(),
            },
        };
        let selection = select::select_results(&inputs.runs);
        input_summary(&inputs, &selection)
    }

    #[test]
    fn non_blank_keeps_visible_values_and_drops_missing_or_blank() {
        assert_eq!(non_blank(None), None);
        assert_eq!(non_blank(Some("")), None);
        assert_eq!(non_blank(Some("   ")), None);
        assert_eq!(non_blank(Some("\t\n")), None);
        assert_eq!(non_blank(Some("TM-1")), Some("TM-1"));
        assert_eq!(non_blank(Some(" B-1 ")), Some(" B-1 "));
    }

    #[test]
    fn summary_renders_blank_baseline_as_not_recorded() {
        for blank in ["", "   "] {
            let text = summary_for(matrix(Some(blank)));
            assert!(text.contains("(baseline: not recorded)"), "{text}");
            assert!(!text.contains("(baseline: )"), "{text}");
        }
    }

    #[test]
    fn summary_keeps_non_blank_baseline() {
        let text = summary_for(matrix(Some("B-1")));
        assert!(text.contains("(baseline: B-1)"), "{text}");
    }

    #[test]
    fn summary_renders_blank_matrix_id_as_not_recorded() {
        let mut m = matrix(Some("B-1"));
        m.id = "   ".to_string();
        let text = summary_for(m);
        assert_eq!(
            text.lines().next(),
            Some("Trace matrix: not recorded (baseline: B-1)")
        );
    }
}
