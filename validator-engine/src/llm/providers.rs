use crate::prompts;
use crate::types::{JudgeProvider, LlmConfig, ValidatorError};

pub const CLAUDE_JSON_PREFILL: &str = "{\"criteria\":";

fn judge_generation() -> &'static prompts::GenerationConfig {
    prompts::generation_config().expect("model_configs.yaml generation section must parse")
}

pub fn build_cloudflare_payload(
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
) -> serde_json::Value {
    let system = if json_mode {
        format!("{system_prompt}\n\nRespond with JSON only.")
    } else {
        system_prompt.to_string()
    };
    serde_json::json!({
        "temperature": judge_generation().temperature,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user_prompt }
        ]
    })
}

pub fn build_openai_payload(
    system_prompt: &str,
    user_prompt: &str,
    model: Option<&str>,
    json_mode: bool,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "model": model.unwrap_or("gpt-4o-mini"),
        "temperature": judge_generation().temperature,
        "max_tokens": judge_generation().max_tokens,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ]
    });
    if json_mode {
        payload["response_format"] = serde_json::json!({ "type": "json_object" });
    }
    payload
}

pub fn build_claude_payload(
    system_prompt: &str,
    user_prompt: &str,
    model: Option<&str>,
    json_mode: bool,
) -> serde_json::Value {
    if json_mode {
        serde_json::json!({
            "model": model.unwrap_or("claude-3-5-sonnet-20240620"),
            "temperature": judge_generation().temperature,
            "max_tokens": judge_generation().max_tokens,
            "system": format!("{system_prompt}\n\nRespond with JSON only."),
            "messages": [
                { "role": "user", "content": user_prompt },
                { "role": "assistant", "content": CLAUDE_JSON_PREFILL }
            ]
        })
    } else {
        serde_json::json!({
            "model": model.unwrap_or("claude-3-5-sonnet-20240620"),
            "temperature": judge_generation().temperature,
            "max_tokens": judge_generation().max_tokens,
            "system": system_prompt,
            "messages": [
                { "role": "user", "content": user_prompt }
            ]
        })
    }
}

pub fn build_ollama_payload(
    model_name: &str,
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "model": model_name,
        "system": system_prompt,
        "prompt": user_prompt,
        "stream": false,
        "options": {
            "temperature": judge_generation().temperature,
            "num_ctx": 8192
        }
    });
    if json_mode {
        payload["format"] = serde_json::json!("json");
    }
    payload
}

fn openai_chat_completions_url(base_url: Option<&str>) -> String {
    const DEFAULT: &str = "https://api.openai.com/v1/chat/completions";
    let Some(base) = base_url else {
        return DEFAULT.to_string();
    };

    let base = base.trim_end_matches('/');
    if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    }
}

pub fn custom_provider_available(config: &LlmConfig) -> bool {
    config.custom_api_key.is_some() && config.custom_url.is_some() && config.custom_model.is_some()
}

fn openai_rate_limited(status: reqwest::StatusCode, body: &str) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || body.contains("Too many requests")
        || body.contains("rate limit")
        || body.contains("Rate limit")
}

fn parse_openai_chat_response(
    status: reqwest::StatusCode,
    body: &str,
) -> Result<String, ValidatorError> {
    if openai_rate_limited(status, body) {
        return Err(ValidatorError::RateLimited(body.to_string()));
    }

    if !status.is_success() {
        return Err(ValidatorError::Llm(format!("OpenAI HTTP {status}: {body}")));
    }

    let res_json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| ValidatorError::Llm(format!("OpenAI JSON parse failed: {e}")))?;

    res_json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ValidatorError::Llm("Invalid OpenAI response format".into()))
}

pub async fn call_custom(
    config: &LlmConfig,
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
) -> Result<String, ValidatorError> {
    let api_key = config
        .custom_api_key
        .as_deref()
        .ok_or_else(|| ValidatorError::Llm("Custom LLM API key missing".into()))?;
    let url = config
        .custom_url
        .as_deref()
        .ok_or_else(|| ValidatorError::Llm("Custom LLM URL missing".into()))?;
    let model = config
        .custom_model
        .as_deref()
        .ok_or_else(|| ValidatorError::Llm("Custom LLM model missing".into()))?;

    let client = reqwest::Client::new();
    let payload = build_openai_payload(system_prompt, user_prompt, Some(model), json_mode);

    let endpoint = openai_chat_completions_url(Some(url));

    let res = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    parse_openai_chat_response(status, &body)
}

pub async fn call_provider(
    provider: JudgeProvider,
    config: &LlmConfig,
    model_override: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
) -> Result<String, ValidatorError> {
    match provider {
        JudgeProvider::Cloudflare => {
            let account_id = config
                .cloudflare_account_id
                .as_deref()
                .ok_or_else(|| ValidatorError::Llm("Cloudflare credentials missing".into()))?;
            let api_token = config
                .cloudflare_api_token
                .as_deref()
                .ok_or_else(|| ValidatorError::Llm("Cloudflare credentials missing".into()))?;
            call_cloudflare(account_id, api_token, system_prompt, user_prompt, json_mode).await
        }
        JudgeProvider::Openai => {
            let api_key = config
                .openai_api_key
                .as_deref()
                .ok_or_else(|| ValidatorError::Llm("OpenAI API key missing".into()))?;
            call_openai(
                api_key,
                config.openai_base_url.as_deref(),
                system_prompt,
                user_prompt,
                model_override,
                json_mode,
            )
            .await
        }
        JudgeProvider::Claude => {
            let api_key = config
                .claude_api_key
                .as_deref()
                .ok_or_else(|| ValidatorError::Llm("Claude API key missing".into()))?;
            call_claude(
                api_key,
                system_prompt,
                user_prompt,
                model_override,
                json_mode,
            )
            .await
        }
        JudgeProvider::Ollama => {
            let ollama_url = config
                .ollama_url
                .as_deref()
                .ok_or_else(|| ValidatorError::Llm("Ollama URL missing".into()))?;
            let model = model_override.or(config.ollama_model.as_deref());
            call_ollama(ollama_url, model, system_prompt, user_prompt, json_mode).await
        }
    }
}

pub fn provider_available(provider: JudgeProvider, config: &LlmConfig) -> bool {
    match provider {
        JudgeProvider::Cloudflare => {
            config.cloudflare_account_id.is_some() && config.cloudflare_api_token.is_some()
        }
        JudgeProvider::Openai => config.openai_api_key.is_some(),
        JudgeProvider::Claude => config.claude_api_key.is_some(),
        JudgeProvider::Ollama => config.ollama_url.is_some(),
    }
}

async fn call_cloudflare(
    account_id: &str,
    api_token: &str,
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = build_cloudflare_payload(system_prompt, user_prompt, json_mode);

    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/@cf/moonshotai/kimi-k2.6",
        account_id
    );

    let res = client
        .post(&url)
        .bearer_auth(api_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    let res_json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    res_json["result"]["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ValidatorError::Llm("Invalid Cloudflare AI response format".into()))
}

async fn call_openai(
    api_key: &str,
    base_url: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
    model: Option<&str>,
    json_mode: bool,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = build_openai_payload(system_prompt, user_prompt, model, json_mode);

    let res = client
        .post(openai_chat_completions_url(base_url))
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    parse_openai_chat_response(status, &body)
}

async fn call_claude(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    model: Option<&str>,
    json_mode: bool,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = build_claude_payload(system_prompt, user_prompt, model, json_mode);

    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    let res_json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    let text = res_json["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ValidatorError::Llm("Invalid Claude response format".into()))?;

    if json_mode {
        Ok(format!("{CLAUDE_JSON_PREFILL}{text}"))
    } else {
        Ok(text)
    }
}

async fn call_ollama(
    ollama_url: &str,
    ollama_model: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let model_name = ollama_model.unwrap_or("gemma4:e4b");
    let payload = build_ollama_payload(model_name, system_prompt, user_prompt, json_mode);

    let res = client
        .post(format!("{ollama_url}/api/generate"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    let res_json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    if let Some(thinking) = res_json["thinking"].as_str()
        && !thinking.is_empty()
    {
        return Ok(thinking.to_string());
    }

    res_json["response"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ValidatorError::Llm("Invalid Ollama response format".into()))
}
