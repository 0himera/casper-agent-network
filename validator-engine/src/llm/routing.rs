use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::prompts;
use crate::types::{JudgeCascadeMode, JudgeProvider, LlmConfig, ValidatorError};

use super::providers::{call_custom, call_provider, custom_provider_available, provider_available};
use super::record_provider_call;

const STAGE_RATE_LIMIT_MAX_RETRIES: u32 = 8;
const STAGE_DELAY_MAX_MS: u64 = 8_000;

static STAGE_DELAY_MS: AtomicU64 = AtomicU64::new(0);

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
    provider_override: Option<JudgeProvider>,
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

    if let Some(first) = provider_override {
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

fn base_delay_ms() -> u64 {
    std::env::var("STAGE_LLM_REQUEST_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000)
}

fn backoff_step_ms() -> u64 {
    std::env::var("STAGE_LLM_RATE_LIMIT_BACKOFF_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500)
}

fn current_stage_delay_ms() -> u64 {
    let loaded = STAGE_DELAY_MS.load(Ordering::Relaxed);
    if loaded > 0 {
        return loaded;
    }
    let base = base_delay_ms();
    STAGE_DELAY_MS
        .compare_exchange(0, base, Ordering::Relaxed, Ordering::Relaxed)
        .unwrap_or(base)
}

fn bump_stage_delay_on_rate_limit() {
    let step = backoff_step_ms();
    loop {
        let current = current_stage_delay_ms();
        let next = (current + step).min(STAGE_DELAY_MAX_MS);
        if STAGE_DELAY_MS
            .compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            break;
        }
    }
}

async fn maybe_delay_stage_request(config: &LlmConfig, routing_key: &str) {
    if config.mock || !routing_key.starts_with("stage_") {
        return;
    }
    let delay_ms = current_stage_delay_ms();
    if delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
}

async fn call_judge_impl_once(
    config: &LlmConfig,
    routing_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let try_custom_first = config.provider.as_ref().is_some_and(|provider| {
        matches!(
            provider.to_ascii_lowercase().as_str(),
            "custom" | "fireworks" | "google" | "gemini" | "openrouter"
        )
    }) || config.provider.is_none();

    let json_mode = uses_json_mode(routing_key);

    if try_custom_first && custom_provider_available(config) {
        match call_custom(config, system_prompt, user_prompt, json_mode).await {
            Ok(text) => return Ok(text),
            Err(ValidatorError::RateLimited(msg)) => {
                return Err(ValidatorError::RateLimited(msg));
            }
            Err(err) if config.provider.is_some() => return Err(err),
            Err(_) => {}
        }
    }

    let cascade = resolve_effective_cascade(config);
    let chain = provider_chain(cascade, None);
    let timeout_ms = resolve_timeout_ms(config);
    let model_ref = config.custom_model.as_deref();

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
            json_mode,
        );

        let result = tokio::time::timeout(Duration::from_millis(timeout_ms), call).await;

        match result {
            Ok(Ok(text)) => {
                record_provider_call(provider);
                if fallback_from.is_some() {
                    eprintln!(
                        "judge LLM: used {:?} after fallback from {:?}",
                        provider, fallback_from
                    );
                }
                return Ok(text);
            }
            Ok(Err(ValidatorError::RateLimited(msg))) => {
                return Err(ValidatorError::RateLimited(msg));
            }
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

fn uses_json_mode(routing_key: &str) -> bool {
    !routing_key.starts_with("stage_") && routing_key != "exam_equality"
}

async fn call_judge_impl(
    config: &LlmConfig,
    routing_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let is_stage = routing_key.starts_with("stage_") && !config.mock;
    let mut rate_limit_retries = 0u32;

    loop {
        maybe_delay_stage_request(config, routing_key).await;

        match call_judge_impl_once(config, routing_key, system_prompt, user_prompt).await {
            Ok(text) => return Ok(text),
            Err(ValidatorError::RateLimited(msg))
                if is_stage && rate_limit_retries < STAGE_RATE_LIMIT_MAX_RETRIES =>
            {
                rate_limit_retries += 1;
                bump_stage_delay_on_rate_limit();
                eprintln!(
                    "judge LLM: rate limited (retry {rate_limit_retries}/{}), delay={}ms: {msg}",
                    STAGE_RATE_LIMIT_MAX_RETRIES,
                    current_stage_delay_ms()
                );
            }
            Err(err) => return Err(err),
        }
    }
}

/// Skill-agnostic judge call: provider chain + timeout + call stats, no JSON validation.
pub async fn call_judge_raw(
    config: &LlmConfig,
    routing_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    if let Some(ref fixture_env) = config.judge_raw_fixture {
        if !fixture_env.is_empty() {
            // Try parsing as JSON map
            if let Ok(map) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(fixture_env)
            {
                if let Some(body) = map.get(routing_key) {
                    return Ok(body.clone());
                }
            } else {
                // Fallback to routing_key:body
                let prefix = format!("{}:", routing_key);
                if fixture_env.starts_with(&prefix) {
                    return Ok(fixture_env[prefix.len()..].to_string());
                }
            }
        }
    }
    call_judge_impl(config, routing_key, system_prompt, user_prompt).await
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
    fn provider_override_puts_provider_first_without_duplicate() {
        let chain = provider_chain(JudgeCascadeMode::LocalFirst, Some(JudgeProvider::Openai));
        assert_eq!(chain[0], JudgeProvider::Openai);
        assert_eq!(
            chain
                .iter()
                .filter(|p| **p == JudgeProvider::Openai)
                .count(),
            1
        );
    }

    #[test]
    fn stage_delay_defaults_to_one_second() {
        assert_eq!(base_delay_ms(), 1000);
        assert_eq!(backoff_step_ms(), 500);
    }

    #[test]
    fn exam_equality_does_not_force_json_mode() {
        assert!(!uses_json_mode("exam_equality"));
        assert!(!uses_json_mode("stage_refusal"));
        assert!(uses_json_mode("benchmark_custom"));
    }
}
