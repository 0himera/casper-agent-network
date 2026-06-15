use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::types::{GraderOptions, LlmConfig, SkillId, ValidationInput, ValidationOutput, Verdict};
use crate::{ValidatorError, evaluate_with_options};

#[derive(Debug, Clone, Deserialize)]
pub struct GoldenExpectation {
    pub verdict: String,
    #[serde(default)]
    pub total: Option<u32>,
    #[serde(default)]
    pub total_lt: Option<u32>,
    #[serde(default)]
    pub all_criteria_passed: Option<bool>,
    #[serde(default)]
    pub recommended_price_motes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoldenCase {
    pub id: String,
    pub skill: String,
    pub task_prompt: String,
    pub agent_output: String,
    pub fixture_file: String,
    pub processing_time_ms: u64,
    pub expect: GoldenExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseSnapshot {
    pub verdict: Verdict,
    pub total: u32,
    pub criteria_passed: Vec<(String, bool)>,
}

#[derive(Debug, Clone)]
pub struct CaseResult {
    pub case_id: String,
    pub matched: bool,
    pub snapshot: CaseSnapshot,
    pub mismatch_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegressionMetrics {
    pub accuracy: f64,
    pub flip_rate: f64,
    pub total_llm_calls: u32,
    pub case_results: Vec<CaseResult>,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn parse_skill(skill: &str) -> Result<SkillId, String> {
    match skill {
        "defi_yield_routing" => Ok(SkillId::DefiYieldRouting),
        "defi_protocol_risk" => Ok(SkillId::DefiProtocolRisk),
        "rwa_appraisal" => Ok(SkillId::RwaAppraisal),
        "rwa_compliance" => Ok(SkillId::RwaCompliance),
        other => Err(format!("unknown skill: {other}")),
    }
}

fn parse_verdict(verdict: &str) -> Result<Verdict, String> {
    match verdict {
        "satisfied" => Ok(Verdict::Satisfied),
        "failed" => Ok(Verdict::Failed),
        other => Err(format!("unknown verdict: {other}")),
    }
}

pub fn load_fixture(fixture_file: &str) -> Result<serde_json::Value, String> {
    let path = fixtures_dir().join(fixture_file);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read fixture {}: {e}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("invalid fixture JSON {}: {e}", path.display()))
}

/// Load the default on-disk fixture for a skill (`fixtures/{skill}.json`).
pub fn load_skill_fixture(skill: SkillId) -> Result<serde_json::Value, String> {
    load_fixture(&format!("{}.json", skill.as_str()))
}

pub fn load_golden_cases_from_path(path: &Path) -> Result<Vec<GoldenCase>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read golden manifest {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("invalid golden manifest JSON: {e}"))
}

pub fn load_golden_cases() -> Result<Vec<GoldenCase>, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden_cases.json");
    load_golden_cases_from_path(&path)
}

pub fn golden_case_to_input(case: &GoldenCase) -> Result<ValidationInput, String> {
    Ok(ValidationInput {
        skill: parse_skill(&case.skill)?,
        task_prompt: case.task_prompt.clone(),
        agent_output: case.agent_output.clone(),
        fixture: load_fixture(&case.fixture_file)?,
        processing_time_ms: case.processing_time_ms,
    })
}

pub fn snapshot_from_output(output: &ValidationOutput) -> CaseSnapshot {
    let mut criteria_passed: Vec<(String, bool)> = output
        .criteria
        .iter()
        .map(|c| (c.id.clone(), c.passed))
        .collect();
    criteria_passed.sort_by(|a, b| a.0.cmp(&b.0));

    CaseSnapshot {
        verdict: output.verdict,
        total: output.total,
        criteria_passed,
    }
}

pub fn snapshots_match(a: &CaseSnapshot, b: &CaseSnapshot) -> bool {
    a.verdict == b.verdict && a.total == b.total && a.criteria_passed == b.criteria_passed
}

pub fn compute_flip_rate(total_comparisons: u32, flipped_comparisons: u32) -> f64 {
    if total_comparisons == 0 {
        return 0.0;
    }
    flipped_comparisons as f64 / total_comparisons as f64
}

fn check_expectation(
    case: &GoldenCase,
    output: &ValidationOutput,
    snapshot: &CaseSnapshot,
) -> Result<(), String> {
    let expected_verdict = parse_verdict(&case.expect.verdict)?;

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

    if let Some(total_lt) = case.expect.total_lt
        && output.total >= total_lt
    {
        return Err(format!("expected total < {total_lt}, got {}", output.total));
    }

    if let Some(all_passed) = case.expect.all_criteria_passed {
        let actual_all_passed = output.criteria.iter().all(|c| c.passed);
        if actual_all_passed != all_passed {
            return Err(format!(
                "all_criteria_passed mismatch: expected {all_passed}, got {actual_all_passed}"
            ));
        }
    }

    if let Some(price) = case.expect.recommended_price_motes
        && output.recommended_price_motes != price
    {
        return Err(format!(
            "recommended_price_motes mismatch: expected {price}, got {}",
            output.recommended_price_motes
        ));
    }

    if snapshot.verdict != expected_verdict {
        return Err("snapshot verdict does not match expectation".to_string());
    }

    Ok(())
}

pub async fn run_regression(
    cases: &[GoldenCase],
    config: &LlmConfig,
    options: &GraderOptions,
) -> Result<RegressionMetrics, ValidatorError> {
    let mut case_results = Vec::with_capacity(cases.len());
    let mut matched_count = 0u32;
    let mut total_llm_calls = 0u32;

    for case in cases {
        let input = golden_case_to_input(case).map_err(ValidatorError::Llm)?;
        let output = evaluate_with_options(input, config, options).await?;
        total_llm_calls += crate::llm::judge_call_count();
        let snapshot = snapshot_from_output(&output);

        let check = check_expectation(case, &output, &snapshot);
        let matched = check.is_ok();
        if matched {
            matched_count += 1;
        }

        case_results.push(CaseResult {
            case_id: case.id.clone(),
            matched,
            snapshot,
            mismatch_reason: check.err(),
        });
    }

    let accuracy = if cases.is_empty() {
        1.0
    } else {
        matched_count as f64 / cases.len() as f64
    };

    Ok(RegressionMetrics {
        accuracy,
        flip_rate: 0.0,
        total_llm_calls,
        case_results,
    })
}

pub async fn run_determinism(
    cases: &[GoldenCase],
    config: &LlmConfig,
    options: &GraderOptions,
    repeats: u32,
) -> Result<RegressionMetrics, ValidatorError> {
    if repeats == 0 {
        return Err(ValidatorError::Llm(
            "determinism repeats must be at least 1".into(),
        ));
    }

    let mut case_results = Vec::with_capacity(cases.len());
    let mut flipped_comparisons = 0u32;
    let mut total_llm_calls = 0u32;
    let comparisons_per_case = repeats.saturating_sub(1);
    let total_comparisons = cases.len() as u32 * comparisons_per_case;

    for case in cases {
        let input = golden_case_to_input(case).map_err(ValidatorError::Llm)?;
        let baseline = evaluate_with_options(input.clone(), config, options).await?;
        total_llm_calls += crate::llm::judge_call_count();
        let baseline_snapshot = snapshot_from_output(&baseline);

        let mut case_flips = 0u32;
        for _ in 1..repeats {
            let output = evaluate_with_options(input.clone(), config, options).await?;
            total_llm_calls += crate::llm::judge_call_count();
            let snapshot = snapshot_from_output(&output);
            if !snapshots_match(&baseline_snapshot, &snapshot) {
                case_flips += 1;
            }
        }
        flipped_comparisons += case_flips;

        let matched = case_flips == 0;
        case_results.push(CaseResult {
            case_id: case.id.clone(),
            matched,
            snapshot: baseline_snapshot,
            mismatch_reason: if matched {
                None
            } else {
                Some(format!(
                    "{case_flips} non-deterministic repeats out of {repeats}"
                ))
            },
        });
    }

    Ok(RegressionMetrics {
        accuracy: if cases.is_empty() {
            1.0
        } else {
            case_results.iter().filter(|r| r.matched).count() as f64 / cases.len() as f64
        },
        flip_rate: compute_flip_rate(total_comparisons, flipped_comparisons),
        total_llm_calls,
        case_results,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct SoftLabelExpectation {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SoftCalibrationCase {
    pub id: String,
    pub skill: String,
    pub task_prompt: String,
    pub agent_output: String,
    pub fixture_file: String,
    pub processing_time_ms: u64,
    pub expect_soft: Vec<SoftLabelExpectation>,
}

#[derive(Debug, Clone)]
pub struct SoftLabelCaseResult {
    pub case_id: String,
    pub matched: bool,
    pub mismatch_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SoftLabelMetrics {
    pub accuracy: f64,
    pub matched_labels: u32,
    pub total_labels: u32,
    pub case_results: Vec<SoftLabelCaseResult>,
}

#[derive(Debug, Clone)]
pub struct UpliftReport {
    pub baseline: SoftLabelMetrics,
    pub treatment: SoftLabelMetrics,
    pub delta_accuracy: f64,
}

pub fn load_soft_calibration_from_path(path: &Path) -> Result<Vec<SoftCalibrationCase>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read soft calibration {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("invalid soft calibration JSON: {e}"))
}

pub fn load_soft_calibration() -> Result<Vec<SoftCalibrationCase>, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("soft_calibration.json");
    load_soft_calibration_from_path(&path)
}

pub fn soft_calibration_to_input(case: &SoftCalibrationCase) -> Result<ValidationInput, String> {
    Ok(ValidationInput {
        skill: parse_skill(&case.skill)?,
        task_prompt: case.task_prompt.clone(),
        agent_output: case.agent_output.clone(),
        fixture: load_fixture(&case.fixture_file)?,
        processing_time_ms: case.processing_time_ms,
    })
}

fn infer_soft_label(score: u32, weight: u32, passed: bool) -> &'static str {
    if passed && score == weight {
        "strong"
    } else if score == weight / 2 {
        "partial"
    } else {
        "missing"
    }
}

pub async fn run_soft_label_regression(
    cases: &[SoftCalibrationCase],
    config: &LlmConfig,
    options: &GraderOptions,
) -> Result<SoftLabelMetrics, ValidatorError> {
    let mut case_results = Vec::with_capacity(cases.len());
    let mut matched_labels = 0u32;
    let mut total_labels = 0u32;

    for case in cases {
        let input = soft_calibration_to_input(case).map_err(ValidatorError::Llm)?;
        let output = evaluate_with_options(input, config, options).await?;
        let defs = crate::rubric::criteria(parse_skill(&case.skill).map_err(ValidatorError::Llm)?);

        let mut case_matched = true;
        let mut mismatch_parts = Vec::new();

        for expected in &case.expect_soft {
            total_labels += 1;
            let def = defs
                .iter()
                .find(|d| d.id == expected.id.as_str())
                .ok_or_else(|| {
                    ValidatorError::Llm(format!("unknown criterion id: {}", expected.id))
                })?;
            let eval = output
                .criteria
                .iter()
                .find(|c| c.id == expected.id)
                .ok_or_else(|| {
                    ValidatorError::Inconsistent(format!(
                        "missing criterion in output: {}",
                        expected.id
                    ))
                })?;

            let actual = infer_soft_label(eval.score, def.weight, eval.passed);
            if actual == expected.label {
                matched_labels += 1;
            } else {
                case_matched = false;
                mismatch_parts.push(format!(
                    "{}: expected {}, got {}",
                    expected.id, expected.label, actual
                ));
            }
        }

        case_results.push(SoftLabelCaseResult {
            case_id: case.id.clone(),
            matched: case_matched,
            mismatch_reason: if case_matched {
                None
            } else {
                Some(mismatch_parts.join("; "))
            },
        });
    }

    let accuracy = if total_labels == 0 {
        1.0
    } else {
        matched_labels as f64 / total_labels as f64
    };

    Ok(SoftLabelMetrics {
        accuracy,
        matched_labels,
        total_labels,
        case_results,
    })
}

pub async fn compare_few_shot_uplift(
    cases: &[SoftCalibrationCase],
    config: &LlmConfig,
) -> Result<UpliftReport, ValidatorError> {
    let baseline = run_soft_label_regression(cases, config, &GraderOptions::f3_baseline()).await?;
    let treatment = run_soft_label_regression(cases, config, &GraderOptions::f3_few_shot()).await?;

    Ok(UpliftReport {
        delta_accuracy: treatment.accuracy - baseline.accuracy,
        baseline,
        treatment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_flip_rate_zero_when_no_flips() {
        assert_eq!(compute_flip_rate(32, 0), 0.0);
    }

    #[test]
    fn compute_flip_rate_one_quarter() {
        assert!((compute_flip_rate(4, 1) - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_flip_rate_zero_when_no_comparisons() {
        assert_eq!(compute_flip_rate(0, 0), 0.0);
    }

    #[test]
    fn load_golden_cases_has_eight_entries() {
        let cases = load_golden_cases().expect("golden manifest must load");
        assert_eq!(cases.len(), 8);
    }

    #[tokio::test]
    async fn determinism_mock_r5_flip_rate_zero() {
        let cases = load_golden_cases().expect("golden manifest must load");
        let config = LlmConfig {
            mock: true,
            ..Default::default()
        };

        let options = GraderOptions::default();

        let metrics = run_determinism(&cases, &config, &options, 5)
            .await
            .expect("determinism run ok");

        assert_eq!(metrics.flip_rate, 0.0);
        assert!(
            metrics.case_results.iter().all(|r| r.matched),
            "expected all cases deterministic: {:?}",
            metrics
                .case_results
                .iter()
                .filter(|r| !r.matched)
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn regression_mock_accuracy_is_one() {
        let cases = load_golden_cases().expect("golden manifest must load");
        let config = LlmConfig {
            mock: true,
            ..Default::default()
        };

        let options = GraderOptions::default();

        let metrics = run_regression(&cases, &config, &options)
            .await
            .expect("regression run ok");

        assert_eq!(metrics.accuracy, 1.0);
        assert_eq!(metrics.total_llm_calls, 0);
        assert!(metrics.case_results.iter().all(|r| r.matched));
    }

    #[test]
    fn load_soft_calibration_has_nine_entries() {
        let cases = load_soft_calibration().expect("soft calibration must load");
        assert_eq!(cases.len(), 9);
    }

    #[tokio::test]
    async fn soft_label_regression_mock_runs_all_cases() {
        let cases = load_soft_calibration().expect("soft calibration must load");
        let config = LlmConfig {
            mock: true,
            ..Default::default()
        };

        let metrics = run_soft_label_regression(&cases, &config, &GraderOptions::default())
            .await
            .expect("soft label regression ok");

        assert_eq!(metrics.total_labels, 9);
        assert_eq!(metrics.case_results.len(), 9);
    }

    #[tokio::test]
    async fn compare_few_shot_uplift_mock_completes() {
        let cases = load_soft_calibration().expect("soft calibration must load");
        let config = LlmConfig {
            mock: true,
            ..Default::default()
        };

        let report = compare_few_shot_uplift(&cases, &config)
            .await
            .expect("uplift compare ok");

        assert_eq!(report.baseline.total_labels, 9);
        assert_eq!(report.treatment.total_labels, 9);
    }
}
