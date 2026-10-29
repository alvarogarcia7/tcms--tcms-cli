mod coverage;
mod independence;
mod log_cleaner;
mod regression;
mod test_run;
mod trace_matrix;
mod verification;
mod verification_templates;

pub use coverage::{
    build_coverage_report, classification_counts, read_annotations, CoverageFindings,
    CoverageInput, CoverageReport, CoverageType, DetectedGap, GapAnnotation, Lcov, McdcInput,
};
pub use independence::{check_run_independence, CaseIndependence, IndependenceAssignment};
pub use log_cleaner::LogCleaner;
pub use regression::{
    read_baseline, read_baselines, read_regression_plan, resolve_baseline, select_regression,
    Baseline, ChangeImpact, RegressionPlan, RegressionSelection, SelectedTestCase,
};
pub use test_run::{TestRunInput, TestRunRecord, TestRunResult, Verdict};
pub use trace_matrix::{
    build_matrix, read_requirements, CoverageGap, MatrixFindings, Requirement, RequirementLevel,
    TraceLink, TraceMatrix,
};
pub use verification::{
    BatchVerificationReport, ContainerReport, ContainerReportConfig, ContainerReportMetadata,
    DiffDetail, ExecutionVerificationResult, MatchStrategy, RunInfo, SequenceVerificationResult,
    StepVerificationResult, StepVerificationResultEnum, TestCaseVerificationResult,
    TestExecutionLog, TestVerifier, VerificationDiff,
};
pub use verification_templates::{
    TemplateCategory, VerificationTemplate, VerificationTemplateLibrary,
};
