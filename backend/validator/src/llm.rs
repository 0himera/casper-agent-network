use crate::types::{CriterionDef, LlmConfig, SkillId, SoftLabel, ValidatorError};

const JUDGE_TEMPERATURE: f32 = 0.0;
const JUDGE_MAX_TOKENS: u32 = 1000;
const CLAUDE_JSON_PREFILL: &str = "{\"criteria\":";

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
        return Ok(mock_response_f3(skill, soft_defs, agent_output));
    };

    parse_soft_grader_response(text)
}

fn mock_response_f3(
    skill: SkillId,
    soft_defs: &[&CriterionDef],
    _agent_output: &str,
) -> SoftGraderLlmResponse {
    let criteria = soft_defs
        .iter()
        .map(|def| {
            LlmSoftCriterionResponse {
                id: def.id.to_string(),
                label: SoftLabel::Strong,
                gap: None,
            }
        })
        .collect();

    SoftGraderLlmResponse {
        criteria,
        explanation: format!("F3 mock evaluation for skill {skill}"),
    }
}

fn parse_soft_label(value: &str) -> Result<SoftLabel, ValidatorError> {
    match value {
        "strong" => Ok(SoftLabel::Strong),
        "partial" => Ok(SoftLabel::Partial),
        "missing" => Ok(SoftLabel::Missing),
        other => Err(ValidatorError::Parse(format!("unknown soft label: {other}"))),
    }
}

fn parse_soft_grader_response(text: String) -> Result<SoftGraderLlmResponse, ValidatorError> {
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

fn build_cloudflare_payload(system_prompt: &str, user_prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "temperature": JUDGE_TEMPERATURE,
        "messages": [
            { "role": "system", "content": format!("{system_prompt}\n\nRespond with JSON only.") },
            { "role": "user", "content": user_prompt }
        ]
    })
}

async fn call_cloudflare(
    account_id: &str,
    api_token: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = build_cloudflare_payload(system_prompt, user_prompt);

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

fn build_openai_payload(system_prompt: &str, user_prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "gpt-4o-mini",
        "temperature": JUDGE_TEMPERATURE,
        "max_tokens": JUDGE_MAX_TOKENS,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "response_format": { "type": "json_object" }
    })
}

async fn call_openai(
    api_key: &str,
    base_url: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = build_openai_payload(system_prompt, user_prompt);

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

fn build_claude_payload(system_prompt: &str, user_prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "claude-3-5-sonnet-20240620",
        "temperature": JUDGE_TEMPERATURE,
        "max_tokens": JUDGE_MAX_TOKENS,
        "system": format!("{system_prompt}\n\nRespond with JSON only."),
        "messages": [
            { "role": "user", "content": user_prompt },
            { "role": "assistant", "content": CLAUDE_JSON_PREFILL }
        ]
    })
}

async fn call_claude(
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let payload = build_claude_payload(system_prompt, user_prompt);

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

    Ok(format!("{CLAUDE_JSON_PREFILL}{text}"))
}

fn build_ollama_payload(
    model_name: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> serde_json::Value {
    serde_json::json!({
        "model": model_name,
        "system": system_prompt,
        "prompt": user_prompt,
        "stream": false,
        "format": "json",
        "options": {
            "temperature": JUDGE_TEMPERATURE,
            "num_ctx": 8192
        }
    })
}

async fn call_ollama(
    ollama_url: &str,
    ollama_model: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, ValidatorError> {
    let client = reqwest::Client::new();
    let model_name = ollama_model.unwrap_or("qwen3.5:4b-gpu");
    let payload = build_ollama_payload(model_name, system_prompt, user_prompt);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CriterionKind;

    #[test]
    fn openai_payload_uses_temperature_zero_and_json_format() {
        let payload = build_openai_payload("system", "user");
        assert_eq!(payload["temperature"], JUDGE_TEMPERATURE);
        assert_eq!(payload["max_tokens"], JUDGE_MAX_TOKENS);
        assert_eq!(payload["response_format"]["type"], "json_object");
    }

    #[test]
    fn claude_payload_uses_temperature_zero_and_json_prefill() {
        let payload = build_claude_payload("system", "user");
        assert_eq!(payload["temperature"], JUDGE_TEMPERATURE);
        assert_eq!(payload["max_tokens"], JUDGE_MAX_TOKENS);
        let messages = payload["messages"].as_array().expect("messages array");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"], CLAUDE_JSON_PREFILL);
    }

    #[test]
    fn cloudflare_payload_uses_temperature_zero() {
        let payload = build_cloudflare_payload("system", "user");
        assert_eq!(payload["temperature"], JUDGE_TEMPERATURE);
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
        let payload = build_ollama_payload("test-model", "system", "user");
        assert_eq!(payload["format"], "json");
        assert_eq!(payload["options"]["temperature"], JUDGE_TEMPERATURE);
    }
}
