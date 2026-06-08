use crate::config::Config;
use serde_json::json;
use std::time::Instant;

pub struct ExecutionResult {
    pub output: String,
    pub processing_time_ms: u64,
}

pub async fn execute_agent(
    domain: &str,
    prompt: &str,
    endpoint_url: Option<&str>,
    api_key: Option<&str>,
    system_prompt: Option<&str>,
    config: &Config,
) -> Result<ExecutionResult, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();

    // Check if hosted or external
    let is_hosted = match endpoint_url {
        None => true,
        Some(url) => url.is_empty() || url == "hosted",
    };

    let output = if is_hosted {
        // Hosted agent: platform executes the LLM directly using system prompt
        let sys_prompt = system_prompt.unwrap_or("You are a helpful AI assistant.");
        
        if let Some(ref key) = config.openai_api_key {
            let client = reqwest::Client::new();
            let payload = json!({
                "model": "gpt-4o-mini",
                "messages": [
                    { "role": "system", "content": sys_prompt },
                    { "role": "user", "content": prompt }
                ]
            });

            let res = client.post("https://api.openai.com/v1/chat/completions")
                .bearer_auth(key)
                .json(&payload)
                .send()
                .await?;

            let res_json: serde_json::Value = res.json().await?;
            res_json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("Error generating response")
                .to_string()
        } else if let Some(ref key) = config.claude_api_key {
            let client = reqwest::Client::new();
            let payload = json!({
                "model": "claude-3-5-sonnet-20240620",
                "max_tokens": 1000,
                "system": sys_prompt,
                "messages": [
                    { "role": "user", "content": prompt }
                ]
            });

            let res = client.post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01")
                .json(&payload)
                .send()
                .await?;

            let res_json: serde_json::Value = res.json().await?;
            res_json["content"][0]["text"]
                .as_str()
                .unwrap_or("Error generating response")
                .to_string()
        } else if let Some(ref ollama_url) = config.ollama_url {
            let client = reqwest::Client::new();
            let model_name = config.ollama_model.as_deref().unwrap_or("qwen3.5:4b-gpu");
            let payload = json!({
                "model": model_name,
                "system": sys_prompt,
                "prompt": prompt,
                "stream": false,
                "options": {
                    "num_ctx": 8192
                }
            });

            let res = client.post(format!("{}/api/generate", ollama_url))
                .json(&payload)
                .send()
                .await?;

            let res_json: serde_json::Value = res.json().await?;
            println!("Ollama hosted agent response received. Model: {}", res_json["model"]);
            
            let output_text = if let Some(thinking) = res_json["thinking"].as_str() {
                if !thinking.is_empty() {
                    thinking.to_string()
                } else {
                    res_json["response"]
                        .as_str()
                        .unwrap_or("Error generating response")
                        .to_string()
                }
            } else {
                res_json["response"]
                    .as_str()
                    .unwrap_or("Error generating response")
                    .to_string()
            };
            output_text
        } else {
            // Simulated Response if no keys are available
            format!(
                "Simulated Hosted Agent Response for domain '{}': Analyzed prompt '{}'. Evaluated details are structured and safe.",
                domain, prompt
            )
        }
    } else {
        // External agent: POST call to user-hosted API endpoint
        let client = reqwest::Client::new();
        let mut request = client.post(endpoint_url.unwrap())
            .json(&json!({
                "task_id": format!("task-{}", start.elapsed().as_nanos()),
                "domain": domain,
                "prompt": prompt
            }));

        if let Some(key) = api_key {
            if !key.is_empty() {
                request = request.bearer_auth(key);
            }
        }

        let res = request.send().await?;
        let res_json: serde_json::Value = res.json().await?;
        
        res_json["result"]
            .as_str()
            .or_else(|| res_json["output"].as_str())
            .ok_or("Invalid agent response format: must return 'result' or 'output' field")?
            .to_string()
    };

    let processing_time_ms = start.elapsed().as_millis() as u64;

    Ok(ExecutionResult {
        output,
        processing_time_ms,
    })
}
