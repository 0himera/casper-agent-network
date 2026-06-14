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

    for case in cases {
        let input = golden_case_to_input(case).map_err(ValidatorError::Llm)?;
        let output = evaluate_with_options(input, config, options).await?;
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
    let comparisons_per_case = repeats.saturating_sub(1);
    let total_comparisons = cases.len() as u32 * comparisons_per_case;

    for case in cases {
        let input = golden_case_to_input(case).map_err(ValidatorError::Llm)?;
        let baseline = evaluate_with_options(input.clone(), config, options).await?;
        let baseline_snapshot = snapshot_from_output(&baseline);

        let mut case_flips = 0u32;
        for _ in 1..repeats {
            let output = evaluate_with_options(input.clone(), config, options).await?;
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
        case_results,
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
        assert!(metrics.case_results.iter().all(|r| r.matched));
    }
}
