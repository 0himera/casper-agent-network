use crate::prompts::{self, FactualityConfig};
use crate::search::SearchProvider;
use crate::types::{LlmConfig, ValidatorError};
use std::time::Instant;

use super::criterion_mapping::map_stages_to_criteria;
use super::domain_map;
use super::stage_scoring::{
    self, aggregate, aggregate_with_factuality, resolved_thresholds, weight, weighted_score,
};
use super::stages::domain_match::DomainMatchStageEval;
use super::stages::factuality::{FactualityStageEval, build_search_provider, factuality_details};
use super::stages::gibberish::GibberishStageEval;
use super::stages::refusal::RefusalStageEval;
use super::stages::relevance::RelevanceStageEval;
use super::stages::{self, domain_match, factuality, gibberish, refusal, relevance};
use super::types::{
    PipelineRunStats, PipelineVerdict, StageId, StagePipelineOutput, StageResult, StageTiming,
};

struct RawStageInput {
    id: StageId,
    raw_output: String,
    normalized_quality: f32,
    passed: bool,
    details: Option<serde_json::Value>,
}

fn resolve_factuality_enabled(config: &LlmConfig) -> bool {
    if let Some(enabled) = config.factuality_enabled {
        return enabled;
    }
    prompts::factuality_config()
        .map(|cfg| cfg.enabled)
        .unwrap_or(false)
}

fn factuality_config() -> FactualityConfig {
    prompts::factuality_config().unwrap_or_default()
}

fn stage_details(parse_fallback: bool) -> Option<serde_json::Value> {
    if parse_fallback {
        Some(serde_json::json!({ "parse_fallback": true }))
    } else {
        None
    }
}

fn assemble(
    raw_stages: [RawStageInput; 4],
    factuality_stage: Option<FactualityStageEval>,
) -> StagePipelineOutput {
    let mut stages = Vec::with_capacity(5);
    let mut verdict = PipelineVerdict::Factual;
    let mut early_exit = false;
    let mut exit_reason = String::new();

    for raw in raw_stages {
        if early_exit {
            stages.push(StageResult {
                id: raw.id,
                passed: false,
                raw_output: raw.raw_output,
                normalized_quality: 0.0,
                weight: weight(raw.id),
                weighted_score: 0,
                skipped_due_to_gate: true,
                reason: Some("skipped due to earlier stage failure".to_string()),
                details: raw.details,
            });
            continue;
        }

        let stage_weight = weight(raw.id);
        let stage = StageResult {
            id: raw.id,
            passed: raw.passed,
            raw_output: raw.raw_output,
            normalized_quality: raw.normalized_quality,
            weight: stage_weight,
            weighted_score: weighted_score(raw.normalized_quality, stage_weight),
            skipped_due_to_gate: false,
            reason: None,
            details: raw.details,
        };

        if !raw.passed {
            early_exit = true;
            exit_reason = format!("{} failed", raw.id.as_str());
            verdict = match raw.id {
                StageId::Refusal => PipelineVerdict::Refusal,
                StageId::Gibberish => PipelineVerdict::Gibberish,
                StageId::Relevance => PipelineVerdict::Irrelevant,
                StageId::DomainMatch => PipelineVerdict::OutOfDomain,
                StageId::Factuality => PipelineVerdict::Unverifiable,
            };
        }

        stages.push(stage);
    }

    let factuality_ran = if early_exit {
        false
    } else if let Some(eval) = factuality_stage {
        let details = if eval.skipped {
            eval.skip_reason
                .as_ref()
                .map(|reason| serde_json::json!({ "skipped": true, "reason": reason }))
        } else {
            Some(factuality_details(
                &eval.extraction,
                &eval.verifications,
                &eval.summary,
            ))
        };

        stages.push(StageResult {
            id: StageId::Factuality,
            passed: eval.passed,
            raw_output: eval.raw_output,
            normalized_quality: if eval.skipped {
                0.0
            } else {
                eval.normalized_quality
            },
            weight: weight(StageId::Factuality),
            weighted_score: if eval.skipped {
                0
            } else {
                weighted_score(eval.normalized_quality, weight(StageId::Factuality))
            },
            skipped_due_to_gate: eval.skipped,
            reason: eval.skip_reason.clone(),
            details,
        });

        if !eval.skipped {
            verdict = eval.verdict;
        }
        !eval.skipped
    } else {
        false
    };

    let total = if factuality_ran {
        aggregate_with_factuality(&stages)
    } else {
        aggregate(&stages)
    };

    let explanation = if early_exit {
        format!("Stage pipeline early exit: {exit_reason}. Total score: {total}.")
    } else if factuality_ran {
        format!("Stage pipeline completed with factuality check. Total score: {total}.")
    } else {
        format!("Stage pipeline passed all checks. Total score: {total}.")
    };

    StagePipelineOutput {
        verdict,
        stages: stages.clone(),
        criteria: map_stages_to_criteria(&stages),
        total,
        explanation,
    }
}

fn gate_failure_output(reason: &str) -> StagePipelineOutput {
    let skipped = |id: StageId| StageResult {
        id,
        passed: false,
        raw_output: String::new(),
        normalized_quality: 0.0,
        weight: weight(id),
        weighted_score: 0,
        skipped_due_to_gate: true,
        reason: Some(reason.to_string()),
        details: None,
    };

    let stages = vec![
        skipped(StageId::Refusal),
        skipped(StageId::Gibberish),
        skipped(StageId::Relevance),
        skipped(StageId::DomainMatch),
    ];
    StagePipelineOutput {
        verdict: PipelineVerdict::Refusal,
        total: 0,
        explanation: format!("Input gate failed: {reason}"),
        stages: stages.clone(),
        criteria: map_stages_to_criteria(&stages),
    }
}

fn build_raw_stages(
    is_refusal: bool,
    gibberish_raw: u32,
    relevance_raw: u32,
    domain_matches: bool,
) -> [RawStageInput; 4] {
    let thresholds = resolved_thresholds();
    [
        RawStageInput {
            id: StageId::Refusal,
            raw_output: if is_refusal {
                "yes".to_string()
            } else {
                "no".to_string()
            },
            normalized_quality: stage_scoring::quality_refusal(is_refusal),
            passed: !is_refusal,
            details: None,
        },
        RawStageInput {
            id: StageId::Gibberish,
            raw_output: gibberish_raw.to_string(),
            normalized_quality: stage_scoring::quality_gibberish(gibberish_raw),
            passed: gibberish_raw >= thresholds.gibberish_min,
            details: None,
        },
        RawStageInput {
            id: StageId::Relevance,
            raw_output: relevance_raw.to_string(),
            normalized_quality: stage_scoring::quality_relevance(relevance_raw),
            passed: relevance_raw >= thresholds.relevance_min,
            details: None,
        },
        RawStageInput {
            id: StageId::DomainMatch,
            raw_output: if domain_matches {
                "yes".to_string()
            } else {
                "no".to_string()
            },
            normalized_quality: stage_scoring::quality_domain(domain_matches),
            passed: domain_matches,
            details: None,
        },
    ]
}

fn refusal_to_raw(eval: RefusalStageEval) -> RawStageInput {
    RawStageInput {
        id: StageId::Refusal,
        raw_output: eval.raw_output,
        normalized_quality: eval.normalized_quality,
        passed: eval.passed,
        details: stage_details(eval.parse_fallback),
    }
}

fn gibberish_to_raw(eval: GibberishStageEval) -> RawStageInput {
    RawStageInput {
        id: StageId::Gibberish,
        raw_output: eval.raw_output,
        normalized_quality: eval.normalized_quality,
        passed: eval.passed,
        details: stage_details(eval.parse_fallback),
    }
}

fn relevance_to_raw(eval: RelevanceStageEval) -> RawStageInput {
    RawStageInput {
        id: StageId::Relevance,
        raw_output: eval.raw_output,
        normalized_quality: eval.normalized_quality,
        passed: eval.passed,
        details: stage_details(eval.parse_fallback),
    }
}

fn domain_match_to_raw(eval: DomainMatchStageEval) -> RawStageInput {
    RawStageInput {
        id: StageId::DomainMatch,
        raw_output: eval.raw_output,
        normalized_quality: eval.normalized_quality,
        passed: eval.passed,
        details: stage_details(eval.parse_fallback),
    }
}

fn detect_mock_signals(agent_output: &str) -> (bool, u32, u32, bool) {
    let refusal = stages::parse_refusal_mock_response(agent_output);
    let gibberish = stages::parse_gibberish_mock_response(agent_output);
    let relevance = stages::parse_relevance_mock_response(agent_output);
    let domain = stages::parse_domain_match_mock_response(agent_output);
    (
        refusal.is_refusal,
        gibberish.raw_score,
        relevance.raw_score,
        domain.domain_matches,
    )
}

async fn run_live_stages(
    domain: &str,
    task_prompt: &str,
    agent_output: &str,
    config: &LlmConfig,
    stage_ms: &mut Vec<StageTiming>,
) -> Result<[RawStageInput; 4], ValidatorError> {
    let expected_domain = domain_map::expected_domain_label(domain);

    let start = Instant::now();
    let refusal = refusal::evaluate_refusal_stage(config, task_prompt, agent_output).await?;
    stage_ms.push(StageTiming {
        id: StageId::Refusal,
        elapsed_ms: start.elapsed().as_millis() as u64,
    });
    if !refusal.passed {
        return Ok([
            refusal_to_raw(refusal),
            skipped_raw(StageId::Gibberish),
            skipped_raw(StageId::Relevance),
            skipped_raw(StageId::DomainMatch),
        ]);
    }

    let start = Instant::now();
    let gibberish = gibberish::evaluate_gibberish_stage(config, task_prompt, agent_output).await?;
    stage_ms.push(StageTiming {
        id: StageId::Gibberish,
        elapsed_ms: start.elapsed().as_millis() as u64,
    });
    if !gibberish.passed {
        return Ok([
            refusal_to_raw(refusal),
            gibberish_to_raw(gibberish),
            skipped_raw(StageId::Relevance),
            skipped_raw(StageId::DomainMatch),
        ]);
    }

    let start = Instant::now();
    let relevance = relevance::evaluate_relevance_stage(config, task_prompt, agent_output).await?;
    stage_ms.push(StageTiming {
        id: StageId::Relevance,
        elapsed_ms: start.elapsed().as_millis() as u64,
    });
    if !relevance.passed {
        return Ok([
            refusal_to_raw(refusal),
            gibberish_to_raw(gibberish),
            relevance_to_raw(relevance),
            skipped_raw(StageId::DomainMatch),
        ]);
    }

    let start = Instant::now();
    let domain_match = domain_match::evaluate_domain_match_stage(
        config,
        domain,
        expected_domain,
        task_prompt,
        agent_output,
    )
    .await?;
    stage_ms.push(StageTiming {
        id: StageId::DomainMatch,
        elapsed_ms: start.elapsed().as_millis() as u64,
    });

    Ok([
        refusal_to_raw(refusal),
        gibberish_to_raw(gibberish),
        relevance_to_raw(relevance),
        domain_match_to_raw(domain_match),
    ])
}

async fn maybe_run_factuality<P: SearchProvider + Sync + ?Sized>(
    config: &LlmConfig,
    domain: &str,
    agent_output: &str,
    search_provider: &P,
) -> Result<Option<FactualityStageEval>, ValidatorError> {
    if !resolve_factuality_enabled(config) {
        return Ok(None);
    }

    let eval = factuality::evaluate_factuality_stage(
        config,
        domain,
        agent_output,
        &factuality_config(),
        true,
        search_provider,
    )
    .await?;

    Ok(Some(eval))
}

fn skipped_raw(id: StageId) -> RawStageInput {
    RawStageInput {
        id,
        raw_output: String::new(),
        normalized_quality: 0.0,
        passed: false,
        details: None,
    }
}

fn factuality_ran_from_output(output: &StagePipelineOutput) -> bool {
    output
        .stages
        .iter()
        .any(|stage| stage.id == StageId::Factuality && !stage.skipped_due_to_gate)
}

fn build_run_stats(
    output: &StagePipelineOutput,
    factuality_enabled: bool,
    stage_ms: Vec<StageTiming>,
    search_cache_hits: u32,
    search_cache_misses: u32,
) -> PipelineRunStats {
    PipelineRunStats {
        pipeline: "stage".to_string(),
        factuality_enabled,
        factuality_ran: factuality_ran_from_output(output),
        verdict: output.verdict,
        total: output.total,
        llm_calls: crate::llm::judge_call_count(),
        search_cache_hits,
        search_cache_misses,
        stage_ms,
    }
}

/// Live stage pipeline with runtime stats for observability (N4.5).
pub async fn evaluate_stage_pipeline_with_stats(
    domain: &str,
    task_prompt: &str,
    agent_output: &str,
    config: &LlmConfig,
) -> Result<(StagePipelineOutput, PipelineRunStats), ValidatorError> {
    let factuality_enabled = resolve_factuality_enabled(config);
    let mut stage_ms = Vec::with_capacity(5);
    let mut search_cache_hits = 0;
    let mut search_cache_misses = 0;

    if let Err(failure) = crate::gates::check_input_fixture_free(agent_output) {
        let output = gate_failure_output(failure.reason());
        return Ok((
            output.clone(),
            build_run_stats(&output, factuality_enabled, stage_ms, 0, 0),
        ));
    }

    let raw_stages =
        run_live_stages(domain, task_prompt, agent_output, config, &mut stage_ms).await?;
    let factuality_stage = if factuality_enabled && raw_stages.iter().all(|stage| stage.passed) {
        let search_provider = build_search_provider(config)?;
        let start = Instant::now();
        let stage = maybe_run_factuality(config, domain, agent_output, &search_provider).await?;
        stage_ms.push(StageTiming {
            id: StageId::Factuality,
            elapsed_ms: start.elapsed().as_millis() as u64,
        });
        let (hits, misses) = search_provider.cache_stats();
        search_cache_hits = hits;
        search_cache_misses = misses;
        stage
    } else {
        None
    };

    let output = assemble(raw_stages, factuality_stage);
    let stats = build_run_stats(
        &output,
        factuality_enabled,
        stage_ms,
        search_cache_hits,
        search_cache_misses,
    );
    Ok((output, stats))
}

/// Live stage pipeline with real LLM calls for S0–S3 and optional factuality.
pub async fn evaluate_stage_pipeline(
    domain: &str,
    task_prompt: &str,
    agent_output: &str,
    config: &LlmConfig,
) -> Result<StagePipelineOutput, ValidatorError> {
    evaluate_stage_pipeline_with_stats(domain, task_prompt, agent_output, config)
        .await
        .map(|(output, _stats)| output)
}

/// Mock stage pipeline for N0 scaffolding — no LLM or search calls.
pub fn evaluate_stage_pipeline_mock(
    domain: &str,
    _task_prompt: &str,
    agent_output: &str,
) -> StagePipelineOutput {
    evaluate_stage_pipeline_mock_with_factuality(domain, _task_prompt, agent_output, false)
}

pub fn evaluate_stage_pipeline_mock_with_factuality(
    domain: &str,
    task_prompt: &str,
    agent_output: &str,
    factuality_enabled: bool,
) -> StagePipelineOutput {
    evaluate_stage_pipeline_mock_with_factuality_and_search(
        domain,
        task_prompt,
        agent_output,
        factuality_enabled,
        None,
    )
}

pub fn evaluate_stage_pipeline_mock_with_factuality_and_search(
    domain: &str,
    _task_prompt: &str,
    agent_output: &str,
    factuality_enabled: bool,
    search_mode: Option<&str>,
) -> StagePipelineOutput {
    if let Err(failure) = crate::gates::check_input_fixture_free(agent_output) {
        return gate_failure_output(failure.reason());
    }

    let _expected_domain = domain_map::expected_domain_label(domain);
    let (is_refusal, gibberish_raw, relevance_raw, domain_matches) =
        detect_mock_signals(agent_output);
    let raw_stages = build_raw_stages(is_refusal, gibberish_raw, relevance_raw, domain_matches);

    let factuality_stage = if factuality_enabled && raw_stages.iter().all(|stage| stage.passed) {
        let config = LlmConfig {
            mock: true,
            factuality_enabled: Some(true),
            ..Default::default()
        };
        let provider = crate::search::mock::provider_for_mock_mode(search_mode);
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
            .block_on(factuality::evaluate_factuality_stage(
                &config,
                domain,
                agent_output,
                &factuality_config(),
                true,
                provider.as_ref(),
            ))
            .ok()
    } else {
        None
    };

    assemble(raw_stages, factuality_stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LlmConfig;

    fn mock_config() -> LlmConfig {
        LlmConfig {
            mock: true,
            ..Default::default()
        }
    }

    fn assert_total_in_range(total: u32) {
        assert!(total <= 100, "total {total} must be <= 100");
    }

    #[test]
    fn mock_good_answer_is_factual_with_high_score() {
        let output = evaluate_stage_pipeline_mock(
            "defi_analysis",
            "Analyze yield opportunities",
            "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY analysis.",
        );

        assert_eq!(output.verdict, PipelineVerdict::Factual);
        assert!(
            output.total >= 90,
            "expected high total, got {}",
            output.total
        );
        assert_total_in_range(output.total);
        assert_eq!(output.stages.len(), 4);
        assert_eq!(output.criteria.len(), output.stages.len());
        assert!(output.stages.iter().all(|s| !s.skipped_due_to_gate));
    }

    #[test]
    fn mock_refusal_exits_early_with_low_score() {
        let output = evaluate_stage_pipeline_mock(
            "defi_analysis",
            "Analyze yield",
            "MOCK_REFUSAL: I cannot fulfill this request.",
        );

        assert_eq!(output.verdict, PipelineVerdict::Refusal);
        assert!(
            output.total < 20,
            "expected low total, got {}",
            output.total
        );
        assert_total_in_range(output.total);
        assert!(!output.stages[0].passed);
        assert!(output.stages[1].skipped_due_to_gate);
        assert!(output.stages[2].skipped_due_to_gate);
        assert!(output.stages[3].skipped_due_to_gate);
    }

    #[test]
    fn mock_gibberish_exits_at_stage_1() {
        let output = evaluate_stage_pipeline_mock(
            "defi_analysis",
            "Analyze yield",
            "MOCK_GIBBERISH: asdf qwer zxcv random tokens.",
        );

        assert_eq!(output.verdict, PipelineVerdict::Gibberish);
        assert!(
            output.total < 30,
            "expected low total, got {}",
            output.total
        );
        assert_total_in_range(output.total);
        assert!(output.stages[0].passed);
        assert!(!output.stages[1].passed);
        assert!(output.stages[2].skipped_due_to_gate);
        assert!(output.stages[3].skipped_due_to_gate);
    }

    #[test]
    fn mock_parse_noisy_sets_parse_fallback_on_live_mock_path() {
        let config = mock_config();
        let agent_output = "MOCK_PARSE_NOISY: Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY analysis.";
        let output = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(evaluate_stage_pipeline(
                "defi_analysis",
                "Analyze yield",
                agent_output,
                &config,
            ))
            .expect("pipeline ok");

        assert_eq!(output.verdict, PipelineVerdict::Factual);
        let gibberish = output
            .stages
            .iter()
            .find(|stage| stage.id == StageId::Gibberish)
            .expect("gibberish stage");
        let relevance = output
            .stages
            .iter()
            .find(|stage| stage.id == StageId::Relevance)
            .expect("relevance stage");
        assert_eq!(
            gibberish
                .details
                .as_ref()
                .and_then(|d| d.get("parse_fallback")),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            relevance
                .details
                .as_ref()
                .and_then(|d| d.get("parse_fallback")),
            Some(&serde_json::Value::Bool(true))
        );
    }

    #[tokio::test]
    async fn live_pipeline_mock_mode_matches_mock_entrypoint() {
        let config = mock_config();
        let agent_output = "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY analysis.";
        let live = evaluate_stage_pipeline("defi_analysis", "Analyze yield", agent_output, &config)
            .await
            .expect("live pipeline ok");
        let mock = evaluate_stage_pipeline_mock("defi_analysis", "Analyze yield", agent_output);
        assert_eq!(live.verdict, mock.verdict);
        assert_eq!(live.total, mock.total);
    }

    #[test]
    fn mock_factuality_supported_adds_stage_and_keeps_factual_verdict() {
        let output = evaluate_stage_pipeline_mock_with_factuality_and_search(
            "defi_analysis",
            "Analyze yield",
            "MOCK_FACT_SUPPORTED: CSPR can be staked on the network. DeFi pools expose users to smart contract risk and should be evaluated carefully before allocation because capital can be lost due to exploits or market volatility in live markets.",
            true,
            None,
        );

        assert_eq!(output.verdict, PipelineVerdict::Factual);
        assert_eq!(output.stages.len(), 5);
        assert_eq!(output.criteria.len(), 5);
        assert_eq!(output.stages[4].id, StageId::Factuality);
        assert!(!output.stages[4].skipped_due_to_gate);
        let factuality_criterion = output
            .criteria
            .iter()
            .find(|c| c.id == "factuality_check")
            .expect("factuality criterion");
        assert!(factuality_criterion.evidence.len() > 1);
    }

    #[test]
    fn mock_factuality_contradicted_marks_hallucinated() {
        let output = evaluate_stage_pipeline_mock_with_factuality_and_search(
            "defi_analysis",
            "Analyze yield",
            "MOCK_FACT_CONTRADICTED: CSPR staking APY is 50%. All DeFi pools are risk-free regardless of contract audits or market conditions and users should treat every pool as guaranteed principal protection without further due diligence.",
            true,
            Some("contradicted"),
        );

        assert_eq!(output.verdict, PipelineVerdict::Hallucinated);
        assert_eq!(output.stages[4].id, StageId::Factuality);
    }

    #[test]
    fn mock_factuality_skips_short_answer() {
        let output = evaluate_stage_pipeline_mock_with_factuality_and_search(
            "defi_analysis",
            "Analyze yield",
            "MOCK_FACT_SUPPORTED: short",
            true,
            None,
        );

        assert_eq!(output.stages.len(), 5);
        assert!(output.stages[4].skipped_due_to_gate);
    }

    #[test]
    fn mock_factuality_runs_for_other_domain() {
        let output = evaluate_stage_pipeline_mock_with_factuality_and_search(
            "other",
            "Analyze the request",
            "MOCK_FACT_SUPPORTED: CSPR can be staked on the network. DeFi pools expose users to smart contract risk and should be evaluated carefully before allocation because capital can be lost due to exploits or market volatility in live markets.",
            true,
            None,
        );

        assert_eq!(output.stages.len(), 5);
        assert!(!output.stages[4].skipped_due_to_gate);
    }

    #[test]
    fn mock_factuality_search_error_becomes_unverifiable_not_pipeline_failure() {
        let output = evaluate_stage_pipeline_mock_with_factuality_and_search(
            "defi_analysis",
            "Analyze yield",
            "MOCK_FACT_SUPPORTED: CSPR can be staked on the network. DeFi pools expose users to smart contract risk and should be evaluated carefully before allocation because capital can be lost due to exploits or market volatility in live markets.",
            true,
            Some("error"),
        );

        assert_eq!(output.verdict, PipelineVerdict::Unverifiable);
    }

    #[tokio::test]
    async fn evaluate_stage_pipeline_with_stats_records_mock_timings() {
        let config = mock_config();
        let agent_output = "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY analysis.";
        let (output, stats) = evaluate_stage_pipeline_with_stats(
            "defi_analysis",
            "Analyze yield",
            agent_output,
            &config,
        )
        .await
        .expect("pipeline with stats");

        assert_eq!(output.verdict, PipelineVerdict::Factual);
        assert!(!stats.stage_ms.is_empty());
        assert_eq!(stats.pipeline, "stage");
        assert!(!stats.factuality_enabled);
        assert!(!stats.factuality_ran);
    }

    #[test]
    fn mock_stage_pipeline_is_deterministic_over_five_runs() {
        let agent_output = "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY analysis.";
        let mut totals = Vec::new();
        for _ in 0..5 {
            let output =
                evaluate_stage_pipeline_mock("defi_analysis", "Analyze yield", agent_output);
            totals.push(output.total);
            assert_eq!(output.verdict, PipelineVerdict::Factual);
        }
        assert!(totals.iter().all(|total| *total == totals[0]));
    }

    #[tokio::test]
    async fn factuality_disabled_does_not_require_serpapi_key() {
        let config = LlmConfig {
            mock: true,
            factuality_enabled: Some(false),
            serpapi_api_key: None,
            ..Default::default()
        };
        let agent_output = "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY analysis and risk notes for long-form evaluation.";
        let output =
            evaluate_stage_pipeline("defi_analysis", "Analyze yield", agent_output, &config)
                .await
                .expect("pipeline should succeed without SerpAPI key");

        assert_eq!(output.stages.len(), 4);
        assert_eq!(output.verdict, PipelineVerdict::Factual);
    }
}
