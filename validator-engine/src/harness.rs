use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::stage_pipeline::{PipelineVerdict, StagePipelineOutput, evaluate_stage_pipeline};
use crate::types::{LlmConfig, ValidatorError};

#[derive(Debug, Clone, Deserialize)]
pub struct StageCalibrationExpectation {
    pub verdict: String,
    #[serde(default)]
    pub total_min: Option<u32>,
    #[serde(default)]
    pub total_max: Option<u32>,
    #[serde(default)]
    pub total_center: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StageCalibrationCase {
    pub id: String,
    pub domain: String,
    pub task_prompt: String,
    pub agent_output: String,
    #[serde(default)]
    pub factuality_enabled: bool,
    #[serde(default)]
    pub search_mode: Option<String>,
    pub expect: StageCalibrationExpectation,
}

#[derive(Debug, Clone)]
pub struct StageCalibrationCaseResult {
    pub case_id: String,
    pub matched: bool,
    pub actual_total: u32,
    pub mismatch_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StageCalibrationMetrics {
    pub band_hit_rate: f64,
    pub matched_cases: u32,
    pub case_results: Vec<StageCalibrationCaseResult>,
}

pub const STAGE_CALIBRATION_BAND: u32 = 10;

pub fn calibration_band(center: u32) -> (u32, u32) {
    let min = center.saturating_sub(STAGE_CALIBRATION_BAND);
    let max = (center + STAGE_CALIBRATION_BAND).min(100);
    (min, max)
}

pub fn load_stage_calibration_from_path(path: &Path) -> Result<Vec<StageCalibrationCase>, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read stage calibration manifest {}: {e}",
            path.display()
        )
    })?;
    serde_json::from_str(&content)
        .map_err(|e| format!("invalid stage calibration manifest JSON: {e}"))
}

pub fn load_stage_calibration() -> Result<Vec<StageCalibrationCase>, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("stage_calibration.json");
    load_stage_calibration_from_path(&path)
}

fn parse_pipeline_verdict(verdict: &str) -> Result<PipelineVerdict, String> {
    match verdict {
        "factual" => Ok(PipelineVerdict::Factual),
        "refusal" => Ok(PipelineVerdict::Refusal),
        "gibberish" => Ok(PipelineVerdict::Gibberish),
        "irrelevant" => Ok(PipelineVerdict::Irrelevant),
        "out_of_domain" => Ok(PipelineVerdict::OutOfDomain),
        "hallucinated" => Ok(PipelineVerdict::Hallucinated),
        "unverifiable" => Ok(PipelineVerdict::Unverifiable),
        other => Err(format!("unknown pipeline verdict: {other}")),
    }
}

fn check_calibration_expectation(
    case: &StageCalibrationCase,
    output: &StagePipelineOutput,
) -> Result<(), String> {
    let expected_verdict = parse_pipeline_verdict(&case.expect.verdict)?;
    if output.verdict != expected_verdict {
        return Err(format!(
            "verdict mismatch: expected {:?}, got {:?}",
            expected_verdict, output.verdict
        ));
    }

    let (min, max) = if let (Some(min), Some(max)) = (case.expect.total_min, case.expect.total_max)
    {
        (min, max)
    } else if let Some(center) = case.expect.total_center {
        calibration_band(center)
    } else {
        return Err(format!(
            "case {} must define total_min/total_max or total_center",
            case.id
        ));
    };

    if output.total < min || output.total > max {
        return Err(format!(
            "total {} outside band [{min}, {max}]",
            output.total
        ));
    }

    if output.total > 100 {
        return Err(format!("total out of range: {}", output.total));
    }

    Ok(())
}

pub fn run_stage_calibration(cases: &[StageCalibrationCase]) -> StageCalibrationMetrics {
    use crate::stage_pipeline::evaluate_stage_pipeline_mock_with_factuality_and_search;

    let mut case_results = Vec::with_capacity(cases.len());
    let mut matched_cases = 0u32;

    for case in cases {
        let output = evaluate_stage_pipeline_mock_with_factuality_and_search(
            &case.domain,
            &case.task_prompt,
            &case.agent_output,
            case.factuality_enabled,
            case.search_mode.as_deref(),
        );

        let check = check_calibration_expectation(case, &output);
        let matched = check.is_ok();
        if matched {
            matched_cases += 1;
        }

        case_results.push(StageCalibrationCaseResult {
            case_id: case.id.clone(),
            matched,
            actual_total: output.total,
            mismatch_reason: check.err(),
        });
    }

    let band_hit_rate = if cases.is_empty() {
        1.0
    } else {
        matched_cases as f64 / cases.len() as f64
    };

    StageCalibrationMetrics {
        band_hit_rate,
        matched_cases,
        case_results,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StageGoldenExpectation {
    pub verdict: String,
    #[serde(default)]
    pub total: Option<u32>,
    #[serde(default)]
    pub total_gte: Option<u32>,
    #[serde(default)]
    pub total_lt: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StageFactualityGoldenCase {
    pub id: String,
    pub domain: String,
    pub task_prompt: String,
    pub agent_output: String,
    #[serde(default)]
    pub search_mode: Option<String>,
    #[serde(default = "default_factuality_enabled")]
    pub factuality_enabled: bool,
    pub expect: StageGoldenExpectation,
}

fn default_factuality_enabled() -> bool {
    true
}

pub fn load_stage_factuality_golden_cases_from_path(
    path: &Path,
) -> Result<Vec<StageFactualityGoldenCase>, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read stage factuality golden manifest {}: {e}",
            path.display()
        )
    })?;
    serde_json::from_str(&content)
        .map_err(|e| format!("invalid stage factuality golden manifest JSON: {e}"))
}

pub fn load_stage_factuality_golden_cases() -> Result<Vec<StageFactualityGoldenCase>, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("stage_factuality_golden_cases.json");
    load_stage_factuality_golden_cases_from_path(&path)
}

pub fn run_stage_factuality_regression(
    cases: &[StageFactualityGoldenCase],
) -> StageRegressionMetrics {
    use crate::stage_pipeline::evaluate_stage_pipeline_mock_with_factuality_and_search;

    let mut case_results = Vec::with_capacity(cases.len());
    let mut matched_count = 0u32;

    for case in cases {
        let output = evaluate_stage_pipeline_mock_with_factuality_and_search(
            &case.domain,
            &case.task_prompt,
            &case.agent_output,
            case.factuality_enabled,
            case.search_mode.as_deref(),
        );

        let adapted = StageGoldenCase {
            id: case.id.clone(),
            domain: case.domain.clone(),
            task_prompt: case.task_prompt.clone(),
            agent_output: case.agent_output.clone(),
            expect: case.expect.clone(),
        };
        let check = check_stage_expectation(&adapted, &output);
        let matched = check.is_ok();
        if matched {
            matched_count += 1;
        }

        case_results.push(StageCaseResult {
            case_id: case.id.clone(),
            matched,
            output,
            mismatch_reason: check.err(),
        });
    }

    let accuracy = if cases.is_empty() {
        1.0
    } else {
        matched_count as f64 / cases.len() as f64
    };

    StageRegressionMetrics {
        accuracy,
        case_results,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StageGoldenCase {
    pub id: String,
    pub domain: String,
    pub task_prompt: String,
    pub agent_output: String,
    pub expect: StageGoldenExpectation,
}

#[derive(Debug, Clone)]
pub struct StageCaseResult {
    pub case_id: String,
    pub matched: bool,
    pub output: StagePipelineOutput,
    pub mismatch_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StageRegressionMetrics {
    pub accuracy: f64,
    pub case_results: Vec<StageCaseResult>,
}

pub fn load_stage_golden_cases_from_path(path: &Path) -> Result<Vec<StageGoldenCase>, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read stage golden manifest {}: {e}",
            path.display()
        )
    })?;
    serde_json::from_str(&content).map_err(|e| format!("invalid stage golden manifest JSON: {e}"))
}

pub fn load_stage_golden_cases() -> Result<Vec<StageGoldenCase>, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("stage_golden_cases.json");
    load_stage_golden_cases_from_path(&path)
}

fn check_stage_expectation(
    case: &StageGoldenCase,
    output: &StagePipelineOutput,
) -> Result<(), String> {
    let expected_verdict = parse_pipeline_verdict(&case.expect.verdict)?;
    if output.verdict != expected_verdict {
        return Err(format!(
            "verdict mismatch: expected {:?}, got {:?}",
            expected_verdict, output.verdict
        ));
    }

    if let Some(total) = case.expect.total
        && output.total != total
    {
        return Err(format!(
            "total mismatch: expected {total}, got {}",
            output.total
        ));
    }

    if let Some(total_gte) = case.expect.total_gte
        && output.total < total_gte
    {
        return Err(format!(
            "expected total >= {total_gte}, got {}",
            output.total
        ));
    }

    if let Some(total_lt) = case.expect.total_lt
        && output.total >= total_lt
    {
        return Err(format!("expected total < {total_lt}, got {}", output.total));
    }

    if output.total > 100 {
        return Err(format!("total out of range: {}", output.total));
    }

    Ok(())
}

pub async fn run_stage_regression(
    cases: &[StageGoldenCase],
    config: &LlmConfig,
) -> Result<StageRegressionMetrics, ValidatorError> {
    let mut case_results = Vec::with_capacity(cases.len());
    let mut matched_count = 0u32;

    for case in cases {
        let output =
            evaluate_stage_pipeline(&case.domain, &case.task_prompt, &case.agent_output, config)
                .await?;

        let check = check_stage_expectation(case, &output);
        let matched = check.is_ok();
        if matched {
            matched_count += 1;
        }

        case_results.push(StageCaseResult {
            case_id: case.id.clone(),
            matched,
            output,
            mismatch_reason: check.err(),
        });
    }

    let accuracy = if cases.is_empty() {
        1.0
    } else {
        matched_count as f64 / cases.len() as f64
    };

    Ok(StageRegressionMetrics {
        accuracy,
        case_results,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExamEqualityGoldenCase {
    pub id: String,
    pub expected_answer: String,
    pub candidate_answer: String,
    /// Human label: `pass` or `fail`.
    pub label: String,
    pub expected_exact: bool,
    pub expected_llm: bool,
    #[serde(default)]
    pub answer_verification_mode: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExamEqualityBenchmarkCaseResult {
    pub case_id: String,
    pub label_pass: bool,
    pub mode_a_passed: bool,
    pub mode_ab_passed: bool,
}

#[derive(Debug, Clone)]
pub struct ExamEqualityBenchmarkMetrics {
    pub total_cases: usize,
    pub mode_a_false_fails: u32,
    pub mode_ab_false_fails: u32,
    pub mode_a_false_fail_rate: f64,
    pub mode_ab_false_fail_rate: f64,
    pub mode_a_precision: f64,
    pub mode_a_recall: f64,
    pub mode_ab_precision: f64,
    pub mode_ab_recall: f64,
    pub case_results: Vec<ExamEqualityBenchmarkCaseResult>,
}

fn verification_policy_for_case(
    case: &ExamEqualityGoldenCase,
) -> crate::exam::ExamVerificationPolicy {
    let mode = case.answer_verification_mode.as_deref();
    let metadata = mode.map(|value| {
        serde_json::json!({
            "answer_verification_mode": value,
            "verification_reason": "golden benchmark case"
        })
    });
    crate::exam::resolve_exam_verification_policy(metadata.as_ref())
}

fn precision_recall(true_positive: u32, false_positive: u32, false_negative: u32) -> (f64, f64) {
    let precision = if true_positive + false_positive == 0 {
        1.0
    } else {
        true_positive as f64 / (true_positive + false_positive) as f64
    };
    let recall = if true_positive + false_negative == 0 {
        1.0
    } else {
        true_positive as f64 / (true_positive + false_negative) as f64
    };
    (precision, recall)
}

pub fn load_exam_equality_golden_cases_from_path(
    path: &Path,
) -> Result<Vec<ExamEqualityGoldenCase>, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read exam equality golden manifest {}: {e}",
            path.display()
        )
    })?;
    serde_json::from_str(&content)
        .map_err(|e| format!("invalid exam equality golden manifest JSON: {e}"))
}

pub fn load_exam_equality_golden_cases() -> Result<Vec<ExamEqualityGoldenCase>, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("exam_llm_equality_golden_cases.json");
    load_exam_equality_golden_cases_from_path(&path)
}

/// Real-LLM manual smoke cases (natural phrasing, no mock markers).
#[derive(Debug, Clone, Deserialize)]
pub struct ExamEqualityRealSmokeCase {
    pub id: String,
    pub task_prompt: String,
    pub expected_answer: String,
    pub candidate_answer: String,
    #[serde(default = "default_exact_then_llm")]
    pub answer_verification_mode: String,
    #[serde(default)]
    pub verification_reason: Option<String>,
    pub label: String,
}

fn default_exact_then_llm() -> String {
    "exact_then_llm".to_string()
}

pub fn load_exam_equality_real_smoke_cases_from_path(
    path: &Path,
) -> Result<Vec<ExamEqualityRealSmokeCase>, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read exam equality real smoke manifest {}: {e}",
            path.display()
        )
    })?;
    serde_json::from_str(&content)
        .map_err(|e| format!("invalid exam equality real smoke manifest JSON: {e}"))
}

pub fn load_exam_equality_real_smoke_cases() -> Result<Vec<ExamEqualityRealSmokeCase>, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("exam_llm_equality_real_smoke_cases.json");
    load_exam_equality_real_smoke_cases_from_path(&path)
}

pub fn verification_policy_for_real_smoke_case(
    case: &ExamEqualityRealSmokeCase,
) -> crate::exam::ExamVerificationPolicy {
    let metadata = serde_json::json!({
        "answer_verification_mode": case.answer_verification_mode,
        "verification_reason": case.verification_reason,
    });
    crate::exam::resolve_exam_verification_policy(Some(&metadata))
}

fn exam_agent_output(candidate_answer: &str) -> String {
    format!("ANSWER: {candidate_answer}")
}

pub fn run_exam_equality_benchmark(
    cases: &[ExamEqualityGoldenCase],
) -> ExamEqualityBenchmarkMetrics {
    let mut mode_a_false_fails = 0u32;
    let mut mode_ab_false_fails = 0u32;
    let mut mode_a_tp = 0u32;
    let mut mode_a_fp = 0u32;
    let mut mode_a_fn = 0u32;
    let mut mode_ab_tp = 0u32;
    let mut mode_ab_fp = 0u32;
    let mut mode_ab_fn = 0u32;
    let mut case_results = Vec::with_capacity(cases.len());

    for case in cases {
        let label_pass = case.label.eq_ignore_ascii_case("pass");
        let agent_output = exam_agent_output(&case.candidate_answer);
        let verification_policy = verification_policy_for_case(case);

        let output_a = crate::exam::evaluate_exam_pipeline_mock_with_config(
            &case.id,
            "Exam benchmark prompt",
            &agent_output,
            &case.expected_answer,
            LlmConfig {
                mock: true,
                exam_llm_equality: false,
                ..Default::default()
            },
            verification_policy,
        );
        let output_ab = crate::exam::evaluate_exam_pipeline_mock_with_config(
            &case.id,
            "Exam benchmark prompt",
            &agent_output,
            &case.expected_answer,
            LlmConfig {
                mock: true,
                exam_llm_equality: true,
                ..Default::default()
            },
            verification_policy,
        );

        let mode_a_passed = output_a.verdict == crate::exam::ExamVerdict::Passed;
        let mode_ab_passed = output_ab.verdict == crate::exam::ExamVerdict::Passed;

        if label_pass && !mode_a_passed {
            mode_a_false_fails += 1;
        }
        if label_pass && !mode_ab_passed {
            mode_ab_false_fails += 1;
        }
        if label_pass && mode_a_passed {
            mode_a_tp += 1;
        } else if !label_pass && mode_a_passed {
            mode_a_fp += 1;
        } else if label_pass && !mode_a_passed {
            mode_a_fn += 1;
        }
        if label_pass && mode_ab_passed {
            mode_ab_tp += 1;
        } else if !label_pass && mode_ab_passed {
            mode_ab_fp += 1;
        } else if label_pass && !mode_ab_passed {
            mode_ab_fn += 1;
        }

        case_results.push(ExamEqualityBenchmarkCaseResult {
            case_id: case.id.clone(),
            label_pass,
            mode_a_passed,
            mode_ab_passed,
        });
    }

    let labeled_pass = cases
        .iter()
        .filter(|c| c.label.eq_ignore_ascii_case("pass"))
        .count();
    let labeled_pass_f = labeled_pass.max(1) as f64;

    let (mode_a_precision, mode_a_recall) = precision_recall(mode_a_tp, mode_a_fp, mode_a_fn);
    let (mode_ab_precision, mode_ab_recall) = precision_recall(mode_ab_tp, mode_ab_fp, mode_ab_fn);

    ExamEqualityBenchmarkMetrics {
        total_cases: cases.len(),
        mode_a_false_fails,
        mode_ab_false_fails,
        mode_a_false_fail_rate: mode_a_false_fails as f64 / labeled_pass_f,
        mode_ab_false_fail_rate: mode_ab_false_fails as f64 / labeled_pass_f,
        mode_a_precision,
        mode_a_recall,
        mode_ab_precision,
        mode_ab_recall,
        case_results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_stage_golden_cases_has_twelve_entries() {
        let cases = load_stage_golden_cases().expect("stage golden manifest must load");
        assert_eq!(cases.len(), 12);
    }

    #[tokio::test]
    async fn stage_regression_mock_accuracy_is_one() {
        let cases = load_stage_golden_cases().expect("stage golden manifest must load");
        let config = LlmConfig {
            mock: true,
            ..Default::default()
        };

        let metrics = run_stage_regression(&cases, &config)
            .await
            .expect("stage regression run ok");

        assert_eq!(metrics.accuracy, 1.0);
        assert!(metrics.case_results.iter().all(|r| r.matched));
    }

    #[test]
    fn load_stage_factuality_golden_cases_has_seven_entries() {
        let cases =
            load_stage_factuality_golden_cases().expect("factuality golden manifest must load");
        assert_eq!(cases.len(), 7);
    }

    #[test]
    fn stage_factuality_regression_mock_accuracy_is_one() {
        let cases =
            load_stage_factuality_golden_cases().expect("factuality golden manifest must load");
        let metrics = run_stage_factuality_regression(&cases);
        assert_eq!(metrics.accuracy, 1.0, "{:?}", metrics.case_results);
        assert!(metrics.case_results.iter().all(|result| result.matched));
    }

    #[test]
    fn load_stage_calibration_has_at_least_twenty_entries() {
        let cases = load_stage_calibration().expect("stage calibration manifest must load");
        assert!(
            cases.len() >= 20,
            "expected >= 20 calibration cases, got {}",
            cases.len()
        );
    }

    #[test]
    fn load_exam_equality_golden_cases_has_at_least_twenty_entries() {
        let cases =
            load_exam_equality_golden_cases().expect("exam equality golden manifest must load");
        assert!(
            cases.len() >= 20,
            "expected >= 20 exam equality cases, got {}",
            cases.len()
        );
    }

    #[test]
    fn load_exam_equality_real_smoke_cases_has_entries() {
        let cases =
            load_exam_equality_real_smoke_cases().expect("exam equality real smoke manifest");
        assert!(
            cases.len() >= 3,
            "expected >= 3 real smoke cases, got {}",
            cases.len()
        );
    }

    #[test]
    fn exam_equality_benchmark_mock_ab_beats_a_on_false_fails() {
        let cases =
            load_exam_equality_golden_cases().expect("exam equality golden manifest must load");
        let metrics = run_exam_equality_benchmark(&cases);
        assert!(
            metrics.mode_ab_false_fail_rate <= metrics.mode_a_false_fail_rate,
            "A+B false-fail rate should not exceed A: A={:.2}% AB={:.2}%",
            metrics.mode_a_false_fail_rate * 100.0,
            metrics.mode_ab_false_fail_rate * 100.0
        );
    }
}
