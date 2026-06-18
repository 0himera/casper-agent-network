use crate::gates;
use crate::llm;
use crate::prompts;
use crate::rubric;
use crate::scoring;
use crate::tools;
use crate::types::{
    CriterionDef, CriterionEval, CriterionKind, GraderMode, GraderOptions, LlmConfig, SkillId,
    ToolResult, ValidationInput, ValidationOutput, ValidatorError, Verdict,
};

const BASE_DEFI_PRICE_MOTES: u64 = 5_000_000_000;
const BASE_RWA_PRICE_MOTES: u64 = 15_000_000_000;
const MAX_BLOCK_CHARS: usize = 4000;

fn resolve_self_consistency_enabled(config: &LlmConfig, options: &GraderOptions) -> bool {
    if config.judge_self_consistency == Some(false) {
        return false;
    }
    match options.self_consistency_enabled {
        Some(enabled) => enabled,
        None => true,
    }
}

pub async fn evaluate_with_options(
    input: &ValidationInput,
    config: &LlmConfig,
    options: &GraderOptions,
) -> Result<ValidationOutput, ValidatorError> {
    match options.mode {
        GraderMode::F3 => evaluate_f3(input, config, options).await,
    }
}

async fn evaluate_f3(
    input: &ValidationInput,
    config: &LlmConfig,
    options: &GraderOptions,
) -> Result<ValidationOutput, ValidatorError> {
    let criteria_defs = rubric::criteria(input.skill);

    if let Err(failure) = gates::check_input(input) {
        let (criteria, explanation) = gates::gate_failure_output(criteria_defs, failure);
        return Ok(ValidationOutput {
            verdict: gates::gate_failure_verdict(),
            criteria,
            total: 0,
            explanation,
            recommended_price_motes: recommended_price_motes(
                input.skill,
                0,
                input.processing_time_ms,
            ),
        });
    }

    let hard_evidence: Vec<(CriterionDef, Vec<ToolResult>)> = criteria_defs
        .iter()
        .filter(|def| def.kind == CriterionKind::Hard)
        .map(|def| {
            let evidence: Vec<ToolResult> = def
                .tools
                .iter()
                .map(|tool| tools::run_tool(tool, &input.fixture, &input.agent_output))
                .collect();
            (*def, evidence)
        })
        .collect();

    let soft_defs = rubric::soft_criteria(input.skill);
    let soft_llm_response = if soft_defs.is_empty() {
        None
    } else {
        let version = options.prompt_version;
        let system_prompt = prompts::f3_soft_system(version)?;
        let soft_refs: Vec<&CriterionDef> = soft_defs.iter().copied().collect();
        let user_prompt =
            prompts::build_f3_user_prompt(input, &soft_refs, version, options.few_shot_enabled)?;
        let self_consistency_enabled = resolve_self_consistency_enabled(config, options);
        llm::reset_judge_call_stats();
        Some(
            llm::grade_soft_labels_with_self_consistency(
                config,
                input.skill,
                &soft_defs,
                &system_prompt,
                &user_prompt,
                &input.agent_output,
                self_consistency_enabled,
            )
            .await?,
        )
    };

    let explanation = match &soft_llm_response {
        Some(response) => response.explanation.clone(),
        None => format!(
            "F3 evaluation for skill {}: all criteria scored from tools",
            input.skill
        ),
    };

    let criteria =
        build_f3_criterion_evals(criteria_defs, &hard_evidence, &soft_defs, soft_llm_response)?;

    let total = criteria.iter().map(|c| c.score).sum();
    let verdict =
        scoring::compute_verdict_f3(&criteria, criteria_defs, total, options.pass_threshold);

    Ok(ValidationOutput {
        verdict,
        criteria,
        total,
        explanation,
        recommended_price_motes: recommended_price_motes(
            input.skill,
            total,
            input.processing_time_ms,
        ),
    })
}

fn build_f3_criterion_evals(
    criteria_defs: &[CriterionDef],
    hard_evidence: &[(CriterionDef, Vec<ToolResult>)],
    _soft_defs: &[&'static CriterionDef],
    soft_llm_response: Option<llm::SoftGraderLlmResponse>,
) -> Result<Vec<CriterionEval>, ValidatorError> {
    let mut result = Vec::with_capacity(criteria_defs.len());

    for def in criteria_defs {
        let eval = if def.kind == CriterionKind::Hard {
            let evidence = hard_evidence
                .iter()
                .find(|(d, _)| d.id == def.id)
                .map(|(_, e)| e.as_slice())
                .unwrap_or(&[]);
            scoring::hard_from_tool(def, evidence)
        } else {
            let llm_response = soft_llm_response.as_ref().ok_or_else(|| {
                ValidatorError::Inconsistent(format!(
                    "missing soft LLM response for criterion {}",
                    def.id
                ))
            })?;
            let llm_criterion = llm_response.criteria.iter().find(|c| c.id == def.id);
            let llm_criterion = match llm_criterion {
                Some(c) => c,
                None => {
                    return Err(ValidatorError::Inconsistent(format!(
                        "missing soft criterion in LLM response: {}",
                        def.id
                    )));
                }
            };
            scoring::soft_from_llm_response(def, llm_criterion)
        };
        result.push(eval);
    }

    Ok(result)
}

fn truncate(text: &str, max_chars: usize) -> String {
    prompts::truncate(text, max_chars)
}

fn base_price_motes(skill: SkillId) -> u64 {
    match skill {
        SkillId::DefiYieldRouting | SkillId::DefiProtocolRisk => BASE_DEFI_PRICE_MOTES,
        SkillId::RwaAppraisal | SkillId::RwaCompliance => BASE_RWA_PRICE_MOTES,
    }
}

fn speed_multiplier(processing_time_ms: u64) -> f64 {
    if processing_time_ms < 5_000 {
        1.2
    } else if processing_time_ms < 15_000 {
        1.0
    } else if processing_time_ms < 30_000 {
        0.8
    } else {
        0.6
    }
}

fn recommended_price_motes(skill: SkillId, total: u32, processing_time_ms: u64) -> u64 {
    let base = base_price_motes(skill) as f64;
    let score_factor = total as f64 / 100.0;
    let multiplier = speed_multiplier(processing_time_ms);
    (base * score_factor * multiplier) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GraderOptions, SkillId};

    fn mock_config() -> LlmConfig {
        LlmConfig {
            mock: true,
            ..Default::default()
        }
    }

    fn sample_input(agent_output: &str, processing_time_ms: u64) -> ValidationInput {
        let fixture = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/defi_yield_routing.json"),
        )
        .expect("fixture");
        ValidationInput {
            skill: SkillId::DefiYieldRouting,
            task_prompt: "Allocate 10k CSPR".to_string(),
            agent_output: agent_output.to_string(),
            fixture: serde_json::from_str(&fixture).expect("fixture json"),
            processing_time_ms,
        }
    }

    const GOLDEN_DEFI_OUTPUT: &str = "Allocate 4,000 CSPR to cspr-usdt (8.2% APY, high TVL), 3,500 CSPR to cspr-eth (6.1% APY, moderate IL), and 2,500 CSPR to cspr-wbtc (11.4% APY, higher IL risk). Total: 10,000 CSPR. Network gas fees (~2.5 CSPR per swap) included. IL analysis shows cspr-usdt lowest volatility exposure.";

    #[tokio::test]
    async fn f3_good_output_returns_satisfied_with_full_score() {
        let output = evaluate_with_options(
            &sample_input(GOLDEN_DEFI_OUTPUT, 10_000),
            &mock_config(),
            &GraderOptions::default(),
        )
        .await
        .expect("evaluate ok");

        assert_eq!(output.verdict, Verdict::Satisfied);
        assert_eq!(output.total, 100);
        assert_eq!(output.criteria.len(), 5);
        assert!(output.criteria.iter().all(|c| c.passed));

        let tool_backed: Vec<_> = output
            .criteria
            .iter()
            .filter(|c| !c.evidence.is_empty())
            .collect();
        assert_eq!(tool_backed.len(), 4);
        assert!(tool_backed.iter().all(|c| c.evidence.iter().all(|e| e.ok)));
    }

    #[tokio::test]
    async fn f3_short_output_gate_fails_without_llm_scoring() {
        let output = evaluate_with_options(
            &sample_input("too short", 10_000),
            &mock_config(),
            &GraderOptions::default(),
        )
        .await
        .expect("evaluate ok");

        assert_eq!(output.verdict, Verdict::Failed);
        assert_eq!(output.total, 0);
        assert!(output.criteria.iter().all(|c| !c.passed && c.score == 0));
        assert!(output.explanation.contains("Input gate failed"));
        assert!(
            output
                .criteria
                .iter()
                .all(|c| c.gap.as_deref() == Some("output too short"))
        );
    }

    #[tokio::test]
    async fn f3_error_output_gate_fails_with_zero_total() {
        let output = evaluate_with_options(
            &sample_input(
                "Allocation failed due to error in pool math calculation",
                10_000,
            ),
            &mock_config(),
            &GraderOptions::default(),
        )
        .await
        .expect("evaluate ok");

        assert_eq!(output.verdict, Verdict::Failed);
        assert_eq!(output.total, 0);
        assert!(output.explanation.contains("error marker"));
    }

    #[tokio::test]
    async fn f3_hard_score_from_tool_not_llm() {
        let output = evaluate_with_options(
            &sample_input(GOLDEN_DEFI_OUTPUT, 10_000),
            &mock_config(),
            &GraderOptions::default(),
        )
        .await
        .expect("evaluate ok");

        let allocation = output
            .criteria
            .iter()
            .find(|c| c.id == "allocation_sum")
            .expect("allocation_sum");
        assert!(allocation.passed);
        assert_eq!(allocation.score, 20);
    }

    #[tokio::test]
    async fn pricing_uses_speed_multiplier() {
        let fast = evaluate_with_options(
            &sample_input(GOLDEN_DEFI_OUTPUT, 4_000),
            &mock_config(),
            &GraderOptions::default(),
        )
        .await
        .expect("evaluate ok");
        let slow = evaluate_with_options(
            &sample_input(GOLDEN_DEFI_OUTPUT, 30_000),
            &mock_config(),
            &GraderOptions::default(),
        )
        .await
        .expect("evaluate ok");

        assert_eq!(fast.recommended_price_motes, 6_000_000_000);
        assert_eq!(slow.recommended_price_motes, 3_000_000_000);
    }

    #[test]
    fn truncate_limits_long_text() {
        let long = "a".repeat(5000);
        let truncated = truncate(&long, MAX_BLOCK_CHARS);
        assert_eq!(truncated.len(), MAX_BLOCK_CHARS + 3);
        assert!(truncated.ends_with("..."));
    }
}
