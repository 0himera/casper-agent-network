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
    model: Option<&str>,
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

        if let (Some(account_id), Some(api_token)) =
            (&config.cloudflare_account_id, &config.cloudflare_api_token)
        {
            let client = reqwest::Client::new();
            let payload = json!({
                "messages": [
                    { "role": "system", "content": sys_prompt },
                    { "role": "user", "content": prompt }
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
            res_json["result"]["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("Error generating response")
                .to_string()
        } else if let Some(ref key) = config.openai_api_key {
            let client = reqwest::Client::new();
            let payload = json!({
                "model": "gpt-4o-mini",
                "messages": [
                    { "role": "system", "content": sys_prompt },
                    { "role": "user", "content": prompt }
                ]
            });

            let res = client
                .post("https://api.openai.com/v1/chat/completions")
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

            let res = client
                .post("https://api.anthropic.com/v1/messages")
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

            let res = client
                .post(format!("{}/api/generate", ollama_url))
                .json(&payload)
                .send()
                .await?;

            let res_json: serde_json::Value = res.json().await?;
            tracing::info!(
                "Ollama hosted agent response received. Model: {}",
                res_json["model"]
            );

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
        // External agent: POST call to user-provided API endpoint
        tracing::info!(
            "Executing external agent call to URL: {}",
            endpoint_url.unwrap()
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()?;
        let mut request = client.post(endpoint_url.unwrap());

        if let Some(key) = api_key {
            if !key.is_empty() {
                request = request.bearer_auth(key);
            }
        }

        let has_model = model.is_some() && !model.unwrap().is_empty();
        let res = if has_model {
            let sys_prompt = system_prompt.unwrap_or("You are a helpful AI assistant.");
            let payload = json!({
                "model": model.unwrap(),
                "messages": [
                    { "role": "system", "content": sys_prompt },
                    { "role": "user", "content": prompt }
                ]
            });
            request.json(&payload).send().await?
        } else {
            let payload = json!({
                "task_id": format!("task-{}", start.elapsed().as_nanos()),
                "domain": domain,
                "prompt": prompt
            });
            request.json(&payload).send().await?
        };

        tracing::info!("External agent call returned status: {}", res.status());
        let res_json: serde_json::Value = res.json().await?;
        tracing::info!("External agent JSON parsed successfully.");

        if let Some(content) = res_json["choices"][0]["message"]["content"].as_str() {
            content.to_string()
        } else if let Some(result) = res_json["result"].as_str() {
            result.to_string()
        } else if let Some(output) = res_json["output"].as_str() {
            output.to_string()
        } else {
            return Err("Invalid external agent response format: must return OpenAI structure or 'result' / 'output'".into());
        }
    };

    let processing_time_ms = start.elapsed().as_millis() as u64;

    Ok(ExecutionResult {
        output,
        processing_time_ms,
    })
}
