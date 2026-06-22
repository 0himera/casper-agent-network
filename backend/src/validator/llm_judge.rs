use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricScores {
    pub accuracy_or_safety: u32,
    pub depth_or_quality: u32,
    pub sources_or_testing: u32,
    pub actionability_or_explanation: u32,
    pub presentation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub scores: RubricScores,
    pub total: u32,
    pub reasoning: String,
    pub recommended_price_motes: u64,
    /// Stage pipeline audit JSON; `None` for legacy judge path.
    pub validator_audit: Option<serde_json::Value>,
}

// Base prices in motes (1 CSPR = 1,000,000,000 motes)
const BASE_DEFI_PRICE: u64 = 5_000_000_000; // 5 CSPR

pub(crate) fn recommended_price_motes(_domain: &str, total: u32, processing_time_ms: u64) -> u64 {
    let base_price = BASE_DEFI_PRICE;

    let speed_multiplier = if processing_time_ms < 5000 {
        1.2
    } else if processing_time_ms < 15000 {
        1.0
    } else if processing_time_ms < 30000 {
        0.8
    } else {
        0.6
    };

    (base_price as f64 * (total as f64 / 100.0) * speed_multiplier) as u64
}

pub async fn evaluate_task(
    domain: &str,
    task_prompt: &str,
    agent_result: &str,
    processing_time_ms: u64,
    config: &Config,
) -> Result<EvaluationResult, Box<dyn std::error::Error + Send + Sync>> {
    use crate::config::ValidatorPipeline;

    if config.validator_pipeline == ValidatorPipeline::Stage {
        return super::stage_adapter::evaluate_task_stage(
            domain,
            task_prompt,
            agent_result,
            processing_time_ms,
            config,
        )
        .await;
    }

    evaluate_task_legacy(
        domain,
        task_prompt,
        agent_result,
        processing_time_ms,
        config,
    )
    .await
}

async fn evaluate_task_legacy(
    domain: &str,
    task_prompt: &str,
    agent_result: &str,
    processing_time_ms: u64,
    config: &Config,
) -> Result<EvaluationResult, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Validator pipeline: legacy");

    // Choose rubric system prompt
    let rubric_prompt = r#"
You are an expert financial and data validator. Evaluate the agent's DeFi analysis response.
Rate the following dimensions:
1. accuracy_or_safety (0-30): Correctness of yield calculations, impermanent loss calculations.
2. depth_or_quality (0-25): Multi-protocol comparisons, risk profiles.
3. sources_or_testing (0-20): Freshness of sources, links, or protocol names referenced.
4. actionability_or_explanation (0-15): Clear steps/actions for users based on analysis.
5. presentation (0-10): Structure, tables, list layout.

Return JSON format exactly matching:
{
  "scores": {
    "accuracy_or_safety": N,
    "depth_or_quality": N,
    "sources_or_testing": N,
    "actionability_or_explanation": N,
    "presentation": N
  },
  "total": SumOfAbove,
  "reasoning": "Brief explanation of scores..."
}
"#;

    let user_content = format!(
        "Task Prompt: {}\n\nAgent Response: {}",
        task_prompt, agent_result
    );

    let (total, scores, reasoning) = if let (Some(api_key), Some(model)) = (
        config.fireworks_api_key.as_deref(),
        config.fireworks_model.as_deref(),
    ) {
        // 0. Fireworks AI Integration
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let payload = serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": rubric_prompt },
                { "role": "user", "content": user_content }
            ]
        });

        let res = client
            .post("https://api.fireworks.ai/inference/v1/chat/completions")
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await?;

        let res_json: serde_json::Value = res.json().await?;
        let text_content = res_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Invalid Fireworks response format")?;

        // Extract JSON block if wrapped in markdown code fence or text
        let json_start = text_content
            .find('{')
            .ok_or("No JSON object found in Fireworks response")?;
        let json_end = text_content
            .rfind('}')
            .ok_or("No JSON object found in Fireworks response")?
            + 1;
        let json_str = &text_content[json_start..json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)?;
        let scores: RubricScores = serde_json::from_value(parsed["scores"].clone())?;
        let total = parsed["total"].as_u64().unwrap_or(0) as u32;
        let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

        (total, scores, reasoning)
    } else if let (Some(account_id), Some(api_token)) =
        (&config.cloudflare_account_id, &config.cloudflare_api_token)
    {
        // 1. Cloudflare Workers AI Integration (Moonshot Kimi k2.6)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let payload = serde_json::json!({
            "messages": [
                { "role": "system", "content": rubric_prompt },
                { "role": "user", "content": user_content }
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
            .await?;

        let res_json: serde_json::Value = res.json().await?;
        let text_content = res_json["result"]["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Invalid Cloudflare AI response format")?;

        // Extract JSON block if wrapped in markdown code fence or text
        let json_start = text_content
            .find('{')
            .ok_or("No JSON object found in Cloudflare AI response")?;
        let json_end = text_content
            .rfind('}')
            .ok_or("No JSON object found in Cloudflare AI response")?
            + 1;
        let json_str = &text_content[json_start..json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)?;
        let scores: RubricScores = serde_json::from_value(parsed["scores"].clone())?;
        let total = parsed["total"].as_u64().unwrap_or(0) as u32;
        let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

        (total, scores, reasoning)
    } else if let Some(ref api_key) = config.openai_api_key {
        // 1. OpenAI Integration
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let payload = serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                { "role": "system", "content": rubric_prompt },
                { "role": "user", "content": user_content }
            ],
            "response_format": { "type": "json_object" }
        });

        let res = client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await?;

        let res_json: serde_json::Value = res.json().await?;
        let text_content = res_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Invalid OpenAI response format")?;

        let parsed: serde_json::Value = serde_json::from_str(text_content)?;

        let scores: RubricScores = serde_json::from_value(parsed["scores"].clone())?;
        let total = parsed["total"].as_u64().unwrap_or(0) as u32;
        let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

        (total, scores, reasoning)
    } else if let Some(ref api_key) = config.claude_api_key {
        // 2. Claude Integration
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let payload = serde_json::json!({
            "model": "claude-3-5-sonnet-20240620",
            "max_tokens": 1000,
            "system": rubric_prompt,
            "messages": [
                { "role": "user", "content": user_content }
            ]
        });

        let res = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await?;

        let res_json: serde_json::Value = res.json().await?;
        let text_content = res_json["content"][0]["text"]
            .as_str()
            .ok_or("Invalid Claude response format")?;

        // Extract JSON block if Claude wrapped it in markdown or text
        let json_start = text_content
            .find('{')
            .ok_or("No JSON object found in Claude response")?;
        let json_end = text_content
            .rfind('}')
            .ok_or("No JSON object found in Claude response")?
            + 1;
        let json_str = &text_content[json_start..json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)?;
        let scores: RubricScores = serde_json::from_value(parsed["scores"].clone())?;
        let total = parsed["total"].as_u64().unwrap_or(0) as u32;
        let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

        (total, scores, reasoning)
    } else if let Some(ref ollama_url) = config.ollama_url {
        // 3. Ollama Integration
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let model_name = config.ollama_model.as_deref().unwrap_or("qwen3.5:4b-gpu");
        let payload = serde_json::json!({
            "model": model_name,
            "system": rubric_prompt,
            "prompt": user_content,
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
            .await?;

        let res_json: serde_json::Value = res.json().await?;
        tracing::info!(
            "Ollama validator response received. Model: {}",
            res_json["model"]
        );

        let text_content = if let Some(thinking) = res_json["thinking"].as_str() {
            if !thinking.is_empty() {
                thinking
            } else {
                res_json["response"].as_str().ok_or(
                    "Invalid Ollama response format: response field missing or not a string",
                )?
            }
        } else {
            res_json["response"]
                .as_str()
                .ok_or("Invalid Ollama response format: response field missing or not a string")?
        };

        let parsed: serde_json::Value = serde_json::from_str(text_content)?;
        let scores: RubricScores = serde_json::from_value(parsed["scores"].clone())?;
        let total = parsed["total"].as_u64().unwrap_or(0) as u32;
        let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

        (total, scores, reasoning)
    } else {
        // 4. Fallback Mock Evaluator (if no API keys configured)
        tracing::info!("WARNING: No LLM API key set. Running in Mock Evaluator mode.");

        let total = if agent_result.contains("error") || agent_result.len() < 20 {
            55
        } else {
            // Generate a deterministic score based on input length
            let hash = agent_result.len() % 20;
            80 + hash as u32 // score between 80 and 99
        };

        let val = total / 5;
        let scores = RubricScores {
            accuracy_or_safety: val + 2,
            depth_or_quality: val + 1,
            sources_or_testing: val - 1,
            actionability_or_explanation: val,
            presentation: total - (val * 4 + 2),
        };

        let reasoning = format!(
            "Mock Evaluator (No API Key): Result analyzed for domain {}. Content length was {} chars. Detail scores generated.",
            domain,
            agent_result.len()
        );

        (total, scores, reasoning)
    };

    // Calculate pricing based on score and processing speed
    let recommended_price_motes = recommended_price_motes(domain, total, processing_time_ms);

    let audit_json = serde_json::json!({
        "pipeline": "legacy",
        "scores": {
            "accuracy": scores.accuracy_or_safety,
            "depth": scores.depth_or_quality,
            "sources": scores.sources_or_testing,
            "actionability": scores.actionability_or_explanation,
            "presentation": scores.presentation,
        },
        "total": total,
        "reasoning": reasoning
    });

    Ok(EvaluationResult {
        scores,
        total,
        reasoning,
        recommended_price_motes,
        validator_audit: Some(audit_json),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ValidatorPipeline};

    fn sample_config(pipeline: ValidatorPipeline) -> Config {
        Config {
            database_url: "mysql://localhost".to_string(),
            port: 3000,
            openai_api_key: None,
            claude_api_key: None,
            ollama_url: None,
            ollama_model: None,
            internal_service_key: None,
            cloudflare_account_id: None,
            cloudflare_api_token: None,
            fireworks_api_key: None,
            fireworks_model: None,
            validator_url: None,
            validator_api_key: None,
            validator_model: None,
            validator_provider: None,
            validator_pipeline: pipeline,
            admin_account: String::new(),
        }
    }

    #[tokio::test]
    async fn evaluate_task_stage_pipeline_mock_smoke() {
        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
            let config = sample_config(ValidatorPipeline::Stage);
            let result = evaluate_task(
                "defi_analysis",
                "Analyze yield",
                "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY.",
                4000,
                &config,
            )
            .await
            .expect("stage path smoke");

            assert!(result.total <= 100);
            assert!(!result.reasoning.is_empty());
            assert!(result.validator_audit.is_some());
        })
        .await;
    }

    #[tokio::test]
    async fn evaluate_task_legacy_pipeline_mock_smoke() {
        let config = sample_config(ValidatorPipeline::Legacy);
        let result = evaluate_task(
            "defi_analysis",
            "Analyze yield",
            "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY.",
            4000,
            &config,
        )
        .await
        .expect("legacy path smoke");

        assert!(result.total <= 100);
        assert!(!result.reasoning.is_empty());
        assert!(result.validator_audit.is_some());
    }
}
