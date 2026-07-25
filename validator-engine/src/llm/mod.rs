mod providers;
mod routing;

pub use routing::call_judge_raw;

use std::cell::Cell;

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

pub(crate) fn record_provider_call(provider: crate::types::JudgeProvider) {
    LLM_CALL_COUNT.with(|c| c.set(c.get() + 1));
    LAST_PROVIDER_USED.with(|p| p.set(Some(provider)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts;
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
    fn ollama_payload_uses_temperature_zero_and_json_format() {
        let generation = judge_generation();
        let payload = build_ollama_payload("test-model", "system", "user", true);
        assert_eq!(payload["format"], "json");
        assert_eq!(payload["options"]["temperature"], generation.temperature);
    }
}
