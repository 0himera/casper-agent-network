use crate::types::{CriterionDef, LlmConfig, SkillId, ValidatorError};

#[derive(Debug, Clone)]
pub struct GraderLlmResponse {
    pub criteria: Vec<LlmCriterionResponse>,
    pub explanation: String,
}

#[derive(Debug, Clone)]
pub struct LlmCriterionResponse {
    pub id: String,
    pub passed: bool,
    pub score: u32,
    pub gap: Option<String>,
}

pub async fn grade(
    config: &LlmConfig,
    skill: SkillId,
    criteria_defs: &[CriterionDef],
    system_prompt: &str,
    user_prompt: &str,
    agent_output: &str,
) -> Result<GraderLlmResponse, ValidatorError> {
    if config.mock {
        return Ok(mock_response(skill, criteria_defs, agent_output));
    }

    let text = if let (Some(account_id), Some(api_token)) =
        (&config.cloudflare_account_id, &config.cloudflare_api_token)
    {
        call_cloudflare(account_id, api_token, system_prompt, user_prompt).await?
    } else if let Some(api_key) = &config.openai_api_key {
        call_openai(
            api_key,
            config.openai_base_url.as_deref(),
            system_prompt,
            user_prompt,
        )
        .await?
    } else if let Some(api_key) = &config.claude_api_key {
        call_claude(api_key, system_prompt, user_prompt).await?
    } else if let Some(ollama_url) = &config.ollama_url {
        call_ollama(
            ollama_url,
            config.ollama_model.as_deref(),
            system_prompt,
            user_prompt,
        )
        .await?
    } else {
        return Ok(mock_response(skill, criteria_defs, agent_output));
    };

    parse_grader_response(text)
}

fn mock_response(
    skill: SkillId,
    criteria_defs: &[CriterionDef],
    agent_output: &str,
) -> GraderLlmResponse {
    let mock_fail = agent_output.len() < 20 || agent_output.contains("error");
    let mock_gap = "mock: output too short or contains error";

    let criteria = criteria_defs
        .iter()
        .map(|def| {
            if mock_fail {
                LlmCriterionResponse {
                    id: def.id.to_string(),
                    passed: false,
                    score: def.weight / 2,
                    gap: Some(mock_gap.to_string()),
                }
            } else {
                LlmCriterionResponse {
                    id: def.id.to_string(),
                    passed: true,
                    score: def.weight,
                    gap: None,
                }
            }
        })
        .collect();

    GraderLlmResponse {
        criteria,
        explanation: format!("Mock evaluation for skill {}", skill),
    }
}

async fn call_cloudflare(
    account_id: &str,
    api_token: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ]
    });

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

async fn call_openai(
    api_key: &str,
    base_url: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "response_format": { "type": "json_object" }
    });

    let res = client
        .post(openai_chat_completions_url(base_url))
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    let res_json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    res_json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ValidatorError::Llm("Invalid OpenAI response format".into()))
}

async fn call_claude(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "model": "claude-3-5-sonnet-20240620",
        "max_tokens": 1000,
        "system": system_prompt,
        "messages": [
            { "role": "user", "content": user_prompt }
        ]
    });

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

    res_json["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ValidatorError::Llm("Invalid Claude response format".into()))
}

async fn call_ollama(
    ollama_url: &str,
    ollama_model: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let model_name = ollama_model.unwrap_or("qwen3.5:4b-gpu");
    let payload = serde_json::json!({
        "model": model_name,
        "system": system_prompt,
        "prompt": user_prompt,
        "stream": false,
        "format": "json",
        "options": {
            "num_ctx": 8192
        }
    });

    let res = client
        .post(format!("{}/api/generate", ollama_url))
        .json(&payload)
        .send()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    let res_json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| ValidatorError::Llm(e.to_string()))?;

    if let Some(thinking) = res_json["thinking"].as_str() {
        if !thinking.is_empty() {
            return Ok(thinking.to_string());
        }
    }

    res_json["response"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ValidatorError::Llm("Invalid Ollama response format".into()))
}

fn extract_json(text: &str) -> Result<&str, ValidatorError> {
    let json_start = text
        .find('{')
        .ok_or_else(|| ValidatorError::Parse("No JSON object found in LLM response".into()))?;
    let json_end = text
        .rfind('}')
        .ok_or_else(|| ValidatorError::Parse("No JSON object found in LLM response".into()))?
        + 1;
    Ok(&text[json_start..json_end])
}

fn parse_grader_response(text: String) -> Result<GraderLlmResponse, ValidatorError> {
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
            let passed = c["passed"].as_bool().unwrap_or(false);
            let score = c["score"].as_u64().unwrap_or(0) as u32;
            let gap = c["gap"].as_str().map(|s| s.to_string());
            Ok(LlmCriterionResponse {
                id,
                passed,
                score,
                gap,
            })
        })
        .collect::<Result<Vec<_>, ValidatorError>>()?;

    Ok(GraderLlmResponse {
        criteria,
        explanation,
    })
}
