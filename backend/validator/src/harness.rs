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
    fn stage_calibration_mock_band_hit_rate_at_least_eighty_percent() {
        let cases = load_stage_calibration().expect("stage calibration manifest must load");
        let metrics = run_stage_calibration(&cases);
        assert!(
            metrics.band_hit_rate >= 0.8,
            "expected >= 80% band hits, got {:.2}%: {:?}",
            metrics.band_hit_rate * 100.0,
            metrics
                .case_results
                .iter()
                .filter(|result| !result.matched)
                .collect::<Vec<_>>()
        );
    }
}
