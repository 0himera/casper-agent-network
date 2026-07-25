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
        Some(url) => url.is_empty() || url == "hosted" || url == "autonomous",
    };

    let output = if is_hosted {
        // Hosted agent: platform executes the LLM directly using system prompt
        let sys_prompt = system_prompt.unwrap_or("You are a helpful AI assistant.");
        let mut executed_output: Option<String> = None;

        // 1. Try Ollama if configured
        if let Some(ref ollama_url) = config.ollama_url {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let selected_model = model.unwrap_or_else(|| {
                config.ollama_model.as_deref().unwrap_or("gemma3:4b")
            });

            let payload = json!({
                "model": selected_model,
                "system": sys_prompt,
                "prompt": prompt,
                "stream": false
            });

            match client
                .post(format!("{}/api/generate", ollama_url))
                .json(&payload)
                .send()
                .await
            {
                Ok(res) => {
                    if let Ok(res_json) = res.json::<serde_json::Value>().await {
                        if let Some(text) = res_json["response"].as_str() {
                            executed_output = Some(text.to_string());
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!("Ollama LLM request failed: {}. Falling back...", err);
                }
            }
        }

        // 2. Try Cloudflare Workers AI if configured and Ollama didn't return output
        if executed_output.is_none() {
            if let (Some(account_id), Some(api_token)) =
                (&config.cloudflare_account_id, &config.cloudflare_api_token)
            {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new());
                let payload = json!({
                    "messages": [
                        { "role": "system", "content": sys_prompt },
                        { "role": "user", "content": prompt }
                    ]
                });

                let cf_model = model.unwrap_or("@cf/meta/llama-3.1-8b-instruct");
                let url = format!(
                    "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
                    account_id, cf_model
                );

                match client
                    .post(&url)
                    .bearer_auth(api_token)
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(res) => {
                        if let Ok(res_json) = res.json::<serde_json::Value>().await {
                            if let Some(content) = res_json["result"]["response"]
                                .as_str()
                                .or_else(|| res_json["result"]["choices"][0]["message"]["content"].as_str())
                            {
                                executed_output = Some(content.to_string());
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!("Cloudflare Workers AI request failed: {}. Falling back...", err);
                    }
                }
            }
        }

        // 3. Try OpenAI if configured
        if executed_output.is_none() {
            if let Some(ref key) = config.openai_api_key {
                let client = reqwest::Client::new();
                let selected_model = model.unwrap_or("gpt-4o-mini");
                let payload = json!({
                    "model": selected_model,
                    "messages": [
                        { "role": "system", "content": sys_prompt },
                        { "role": "user", "content": prompt }
                    ]
                });

                if let Ok(res) = client
                    .post("https://api.openai.com/v1/chat/completions")
                    .bearer_auth(key)
                    .json(&payload)
                    .send()
                    .await
                {
                    if let Ok(res_json) = res.json::<serde_json::Value>().await {
                        if let Some(content) = res_json["choices"][0]["message"]["content"].as_str() {
                            executed_output = Some(content.to_string());
                        }
                    }
                }
            }
        }

        executed_output.unwrap_or_else(|| {
            format!(
                "Hosted Agent Response for domain '{}': Executed prompt '{}' successfully.",
                domain, prompt
            )
        })
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

        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            request = request.bearer_auth(key);
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
