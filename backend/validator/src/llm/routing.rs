use std::time::Duration;

use crate::prompts;
use crate::types::{JudgeCascadeMode, JudgeProvider, LlmConfig, SkillId, ValidatorError};

use super::extract_json;
use super::record_provider_call;
use super::providers::{call_provider, provider_available};

pub fn resolve_effective_cascade(config: &LlmConfig) -> JudgeCascadeMode {
    if let Some(cascade) = config.judge_cascade {
        return cascade;
    }
    prompts::judge_routing()
        .map(|r| r.cascade)
        .unwrap_or(JudgeCascadeMode::ApiFirst)
}

pub fn provider_chain(
    cascade: JudgeCascadeMode,
    skill_override: Option<JudgeProvider>,
) -> Vec<JudgeProvider> {
    let base = match cascade {
        JudgeCascadeMode::ApiFirst => vec![
            JudgeProvider::Cloudflare,
            JudgeProvider::Openai,
            JudgeProvider::Claude,
            JudgeProvider::Ollama,
        ],
        JudgeCascadeMode::LocalFirst => vec![
            JudgeProvider::Ollama,
            JudgeProvider::Cloudflare,
            JudgeProvider::Openai,
            JudgeProvider::Claude,
        ],
    };

    if let Some(first) = skill_override {
        let mut chain = vec![first];
        for provider in base {
            if provider != first {
                chain.push(provider);
            }
        }
        chain
    } else {
        base
    }
}

fn resolve_timeout_ms(config: &LlmConfig) -> u64 {
    config
        .judge_timeout_ms
        .or_else(|| prompts::judge_routing().ok().map(|r| r.default_timeout_ms))
        .unwrap_or(15_000)
}

fn skill_provider_override(skill: SkillId) -> Option<JudgeProvider> {
    prompts::skill_judge_config(skill)
        .ok()
        .flatten()
        .and_then(|c| c.provider)
}

fn skill_model_override(skill: SkillId) -> Option<String> {
    prompts::skill_judge_config(skill)
        .ok()
        .flatten()
        .and_then(|c| c.model)
}

pub async fn call_judge_with_fallback(
    config: &LlmConfig,
    skill: SkillId,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let cascade = resolve_effective_cascade(config);
    let skill_override = skill_provider_override(skill);
    let chain = provider_chain(cascade, skill_override);
    let timeout_ms = resolve_timeout_ms(config);
    let model_override = skill_model_override(skill);
    let model_ref = model_override.as_deref();

    let mut last_error: Option<ValidatorError> = None;
    let mut fallback_from: Option<JudgeProvider> = None;

    for provider in chain {
        if !provider_available(provider, config) {
            continue;
        }

        let call = call_provider(
            provider,
            config,
            model_ref,
            system_prompt,
            user_prompt,
        );

        let result = tokio::time::timeout(Duration::from_millis(timeout_ms), call).await;

        match result {
            Ok(Ok(text)) => match validate_judge_json(&text) {
                Ok(()) => {
                    record_provider_call(provider);
                    if fallback_from.is_some() {
                        eprintln!(
                            "judge LLM: used {:?} after fallback from {:?}",
                            provider, fallback_from
                        );
                    }
                    return Ok(text);
                }
                Err(parse_err) => {
                    fallback_from.get_or_insert(provider);
                    last_error = Some(parse_err);
                }
            },
            Ok(Err(err)) => {
                fallback_from.get_or_insert(provider);
                last_error = Some(err);
            }
            Err(_) => {
                fallback_from.get_or_insert(provider);
                last_error = Some(ValidatorError::Llm(format!(
                    "judge LLM timeout after {timeout_ms}ms via {provider:?}"
                )));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        ValidatorError::Llm("no judge LLM provider available in cascade chain".into())
    }))
}

fn validate_judge_json(text: &str) -> Result<(), ValidatorError> {
    let json_str = extract_json(text)?;
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| ValidatorError::Parse(e.to_string()))?;

    if !parsed["criteria"].is_array() {
        return Err(ValidatorError::Parse(
            "Missing criteria array in LLM response".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_first_chain_order() {
        let chain = provider_chain(JudgeCascadeMode::ApiFirst, None);
        assert_eq!(
            chain,
            vec![
                JudgeProvider::Cloudflare,
                JudgeProvider::Openai,
                JudgeProvider::Claude,
                JudgeProvider::Ollama,
            ]
        );
    }

    #[test]
    fn local_first_chain_order() {
        let chain = provider_chain(JudgeCascadeMode::LocalFirst, None);
        assert_eq!(
            chain,
            vec![
                JudgeProvider::Ollama,
                JudgeProvider::Cloudflare,
                JudgeProvider::Openai,
                JudgeProvider::Claude,
            ]
        );
    }

    #[test]
    fn skill_override_puts_provider_first_without_duplicate() {
        let chain = provider_chain(JudgeCascadeMode::LocalFirst, Some(JudgeProvider::Openai));
        assert_eq!(chain[0], JudgeProvider::Openai);
        assert_eq!(chain.iter().filter(|p| **p == JudgeProvider::Openai).count(), 1);
        assert!(!chain.contains(&JudgeProvider::Ollama) || chain[0] != JudgeProvider::Ollama);
    }
}
