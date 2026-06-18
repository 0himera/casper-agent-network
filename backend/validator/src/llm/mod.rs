mod providers;
mod routing;
mod self_consistency;

pub use routing::call_judge_raw;

use std::cell::Cell;

use crate::types::{CriterionDef, LlmConfig, SkillId, SoftLabel, ValidatorError};

use self_consistency::{aggregate_soft_responses, should_run_self_consistency};

thread_local! {
    static LLM_CALL_COUNT: Cell<u32> = const { Cell::new(0) };
    static LAST_PROVIDER_USED: Cell<Option<crate::types::JudgeProvider>> = const { Cell::new(None) };
}

pub fn reset_judge_call_stats() {
    LLM_CALL_COUNT.with(|c| c.set(0));
    LAST_PROVIDER_USED.with(|p| p.set(None));
}

pub fn judge_call_count() -> u32 {
    LLM_CALL_COUNT.with(|c| c.get())
}

pub fn last_judge_provider_used() -> Option<crate::types::JudgeProvider> {
    LAST_PROVIDER_USED.with(|p| p.get())
}

fn record_judge_call(provider: crate::types::JudgeProvider) {
    LLM_CALL_COUNT.with(|c| c.set(c.get() + 1));
    LAST_PROVIDER_USED.with(|p| p.set(Some(provider)));
}

#[derive(Debug, Clone)]
pub struct LlmSoftCriterionResponse {
    pub id: String,
    pub label: SoftLabel,
    pub gap: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SoftGraderLlmResponse {
    pub criteria: Vec<LlmSoftCriterionResponse>,
    pub explanation: String,
}

pub async fn grade_soft_labels(
    config: &LlmConfig,
    skill: SkillId,
    soft_defs: &[&CriterionDef],
    system_prompt: &str,
    user_prompt: &str,
    agent_output: &str,
) -> Result<SoftGraderLlmResponse, ValidatorError> {
    if config.mock {
        return Ok(mock_response_f3(skill, soft_defs, agent_output));
    }

    let text =
        match routing::call_judge_with_fallback(config, skill, system_prompt, user_prompt).await {
            Ok(text) => text,
            Err(ValidatorError::Llm(ref msg)) if msg.contains("no judge LLM provider") => {
                return Ok(mock_response_f3(skill, soft_defs, agent_output));
            }
            Err(err) => return Err(err),
        };
    parse_soft_grader_response(text)
}

pub async fn grade_soft_labels_with_self_consistency(
    config: &LlmConfig,
    skill: SkillId,
    soft_defs: &[&CriterionDef],
    system_prompt: &str,
    user_prompt: &str,
    agent_output: &str,
    self_consistency_enabled: bool,
) -> Result<SoftGraderLlmResponse, ValidatorError> {
    if config.mock || !self_consistency_enabled {
        return grade_soft_labels(
            config,
            skill,
            soft_defs,
            system_prompt,
            user_prompt,
            agent_output,
        )
        .await;
    }

    let first = grade_soft_labels(
        config,
        skill,
        soft_defs,
        system_prompt,
        user_prompt,
        agent_output,
    )
    .await?;

    let sc_config = crate::prompts::skill_judge_config(skill)?
        .and_then(|c| c.self_consistency)
        .filter(|c| c.enabled);

    let Some(sc_config) = sc_config else {
        return Ok(first);
    };

    if !should_run_self_consistency(&first, sc_config.trigger) {
        return Ok(first);
    }

    let mut samples = vec![first];
    let extra = sc_config.samples.saturating_sub(1);
    for _ in 0..extra {
        let response = grade_soft_labels(
            config,
            skill,
            soft_defs,
            system_prompt,
            user_prompt,
            agent_output,
        )
        .await?;
        samples.push(response);
    }

    Ok(aggregate_soft_responses(&samples))
}

pub(crate) fn mock_response_f3(
    skill: SkillId,
    soft_defs: &[&CriterionDef],
    _agent_output: &str,
) -> SoftGraderLlmResponse {
    let criteria = soft_defs
        .iter()
        .map(|def| LlmSoftCriterionResponse {
            id: def.id.to_string(),
            label: SoftLabel::Strong,
            gap: None,
        })
        .collect();

    SoftGraderLlmResponse {
        criteria,
        explanation: format!("F3 mock evaluation for skill {skill}"),
    }
}

pub(crate) fn parse_soft_label(value: &str) -> Result<SoftLabel, ValidatorError> {
    match value {
        "strong" => Ok(SoftLabel::Strong),
        "partial" => Ok(SoftLabel::Partial),
        "missing" => Ok(SoftLabel::Missing),
        other => Err(ValidatorError::Parse(format!(
            "unknown soft label: {other}"
        ))),
    }
}

pub(crate) fn parse_soft_grader_response(
    text: String,
) -> Result<SoftGraderLlmResponse, ValidatorError> {
    let json_str = extract_json(&text)?;
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| ValidatorError::Parse(e.to_string()))?;

    let explanation = parsed["explanation"].as_str().unwrap_or("").to_string();

    let criteria = parsed["criteria"]
        .as_array()
        .ok_or_else(|| ValidatorError::Parse("Missing criteria array in LLM response".into()))?
        .iter()
        .map(|c| {
            let id = c["id"]
                .as_str()
                .ok_or_else(|| ValidatorError::Parse("Criterion missing id".into()))?
                .to_string();
            let label_str = c["label"]
                .as_str()
                .ok_or_else(|| ValidatorError::Parse(format!("Criterion {id} missing label")))?;
            let label = parse_soft_label(label_str)?;
            let gap = c["gap"].as_str().map(|s| s.to_string());
            Ok(LlmSoftCriterionResponse { id, label, gap })
        })
        .collect::<Result<Vec<_>, ValidatorError>>()?;

    Ok(SoftGraderLlmResponse {
        criteria,
        explanation,
    })
}

pub(crate) fn extract_json(text: &str) -> Result<&str, ValidatorError> {
    let json_start = text
        .find('{')
        .ok_or_else(|| ValidatorError::Parse("No JSON object found in LLM response".into()))?;
    let json_end = text
        .rfind('}')
        .ok_or_else(|| ValidatorError::Parse("No JSON object found in LLM response".into()))?
        + 1;
    Ok(&text[json_start..json_end])
}

pub(crate) fn record_provider_call(provider: crate::types::JudgeProvider) {
    record_judge_call(provider);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts;
    use crate::types::CriterionKind;
    use providers::{
        CLAUDE_JSON_PREFILL, build_claude_payload, build_cloudflare_payload, build_ollama_payload,
        build_openai_payload,
    };

    fn judge_generation() -> &'static prompts::GenerationConfig {
        prompts::generation_config().expect("model_configs.yaml generation section must parse")
    }

    #[test]
    fn openai_payload_uses_temperature_zero_and_json_format() {
        let generation = judge_generation();
        let payload = build_openai_payload("system", "user", None, true);
        assert_eq!(payload["temperature"], generation.temperature);
        assert_eq!(payload["max_tokens"], generation.max_tokens);
        assert_eq!(payload["response_format"]["type"], "json_object");
    }

    #[test]
    fn claude_payload_uses_temperature_zero_and_json_prefill() {
        let generation = judge_generation();
        let payload = build_claude_payload("system", "user", None, true);
        assert_eq!(payload["temperature"], generation.temperature);
        assert_eq!(payload["max_tokens"], generation.max_tokens);
        let messages = payload["messages"].as_array().expect("messages array");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"], CLAUDE_JSON_PREFILL);
    }

    #[test]
    fn cloudflare_payload_uses_temperature_zero() {
        let generation = judge_generation();
        let payload = build_cloudflare_payload("system", "user", true);
        assert_eq!(payload["temperature"], generation.temperature);
        let system = payload["messages"][0]["content"]
            .as_str()
            .expect("system message");
        assert!(system.contains("JSON only"));
    }

    #[test]
    fn parse_soft_grader_response_parses_labels() {
        let text = r#"{
            "criteria": [
                { "id": "pool_selection", "label": "strong", "gap": null },
                { "id": "mitigation_steps", "label": "partial", "gap": "needs detail" }
            ],
            "explanation": "Soft criteria evaluated."
        }"#;

        let parsed = parse_soft_grader_response(text.to_string()).expect("parse ok");
        assert_eq!(parsed.criteria.len(), 2);
        assert_eq!(parsed.criteria[0].label, SoftLabel::Strong);
        assert_eq!(parsed.criteria[1].label, SoftLabel::Partial);
        assert_eq!(parsed.criteria[1].gap.as_deref(), Some("needs detail"));
    }

    #[test]
    fn mock_response_f3_returns_strong_for_soft_criteria() {
        let soft_defs: Vec<&CriterionDef> = [CriterionDef {
            id: "pool_selection",
            description: "test",
            tools: &[],
            weight: 20,
            kind: CriterionKind::Soft,
            critical: false,
        }]
        .iter()
        .collect();

        let response = mock_response_f3(SkillId::DefiYieldRouting, &soft_defs, "good output");
        assert_eq!(response.criteria.len(), 1);
        assert_eq!(response.criteria[0].label, SoftLabel::Strong);
    }

    #[test]
    fn ollama_payload_uses_temperature_zero_and_json_format() {
        let generation = judge_generation();
        let payload = build_ollama_payload("test-model", "system", "user", true);
        assert_eq!(payload["format"], "json");
        assert_eq!(payload["options"]["temperature"], generation.temperature);
    }
}
