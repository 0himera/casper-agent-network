use serde::{Serialize, Deserialize};
use crate::config::Config;

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
}

// Base prices in motes (1 CSPR = 1,000,000,000 motes)
const BASE_DEFI_PRICE: u64 = 5_000_000_000;  // 5 CSPR
const BASE_CODE_PRICE: u64 = 10_000_000_000; // 10 CSPR

pub async fn evaluate_task(
    domain: &str,
    task_prompt: &str,
    agent_result: &str,
    processing_time_ms: u64,
    config: &Config,
) -> Result<EvaluationResult, Box<dyn std::error::Error + Send + Sync>> {
    
    // Choose rubric system prompt
    let rubric_prompt = match domain {
        "code_review" => r#"
You are an expert code auditor. Evaluate the agent's code review response.
Rate the following dimensions:
1. safety_or_security (0-30): Vulnerability analysis, reentrancy, access control.
2. depth_or_quality (0-25): Best practices, design patterns, gas efficiency.
3. sources_or_testing (0-20): Test scenario suggestions, fuzzing recommendations.
4. actionability_or_explanation (0-15): Actionable refactoring examples and code blocks.
5. presentation (0-10): Clear structure, severity labeling.

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
"#,
        _ => r#"
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
"#,
    };

    let user_content = format!(
        "Task Prompt: {}\n\nAgent Response: {}",
        task_prompt, agent_result
    );

    let (total, scores, reasoning) = if let Some(ref api_key) = config.openai_api_key {
        // 1. OpenAI Integration
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                { "role": "system", "content": rubric_prompt },
                { "role": "user", "content": user_content }
            ],
            "response_format": { "type": "json_object" }
        });

        let res = client.post("https://api.openai.com/v1/chat/completions")
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
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "model": "claude-3-5-sonnet-20240620",
            "max_tokens": 1000,
            "system": rubric_prompt,
            "messages": [
                { "role": "user", "content": user_content }
            ]
        });

        let res = client.post("https://api.anthropic.com/v1/messages")
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
        let json_start = text_content.find('{').ok_or("No JSON object found in Claude response")?;
        let json_end = text_content.rfind('}').ok_or("No JSON object found in Claude response")? + 1;
        let json_str = &text_content[json_start..json_end];

        let parsed: serde_json::Value = serde_json::from_str(json_str)?;
        let scores: RubricScores = serde_json::from_value(parsed["scores"].clone())?;
        let total = parsed["total"].as_u64().unwrap_or(0) as u32;
        let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

        (total, scores, reasoning)

    } else if let Some(ref ollama_url) = config.ollama_url {
        // 3. Ollama Integration
        let client = reqwest::Client::new();
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

        let res = client.post(format!("{}/api/generate", ollama_url))
            .json(&payload)
            .send()
            .await?;

        let res_json: serde_json::Value = res.json().await?;
        println!("Ollama validator response received. Model: {}", res_json["model"]);
        
        let text_content = if let Some(thinking) = res_json["thinking"].as_str() {
            if !thinking.is_empty() {
                thinking
            } else {
                res_json["response"].as_str().ok_or("Invalid Ollama response format: response field missing or not a string")?
            }
        } else {
            res_json["response"].as_str().ok_or("Invalid Ollama response format: response field missing or not a string")?
        };

        let parsed: serde_json::Value = serde_json::from_str(text_content)?;
        let scores: RubricScores = serde_json::from_value(parsed["scores"].clone())?;
        let total = parsed["total"].as_u64().unwrap_or(0) as u32;
        let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

        (total, scores, reasoning)

    } else {
        // 4. Fallback Mock Evaluator (if no API keys configured)
        println!("WARNING: No LLM API key set. Running in Mock Evaluator mode.");
        
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
            domain, agent_result.len()
        );

        (total, scores, reasoning)
    };

    // Calculate pricing based on score and processing speed
    let base_price = match domain {
        "code_review" => BASE_CODE_PRICE,
        _ => BASE_DEFI_PRICE,
    };

    let speed_multiplier = if processing_time_ms < 5000 {
        1.2
    } else if processing_time_ms < 15000 {
        1.0
    } else if processing_time_ms < 30000 {
        0.8
    } else {
        0.6
    };

    let recommended_price_motes = (base_price as f64 * (total as f64 / 100.0) * speed_multiplier) as u64;

    Ok(EvaluationResult {
        scores,
        total,
        reasoning,
        recommended_price_motes,
    })
}
