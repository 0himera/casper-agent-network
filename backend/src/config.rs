use std::env;

/// Live validator execution path for `/execute`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidatorPipeline {
    /// Legacy single-LLM rubric judge (`llm_judge.rs`).
    Legacy,
    /// Stage pipeline S0–S3 via `validator-engine`.
    Stage,
}

impl ValidatorPipeline {
    /// Reads `VALIDATOR_PIPELINE`; defaults to `legacy` for safe rollback.
    pub fn from_env() -> Self {
        match env::var("VALIDATOR_PIPELINE")
            .ok()
            .as_deref()
            .map(str::trim)
        {
            Some("stage") => Self::Stage,
            _ => Self::Legacy,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub openai_api_key: Option<String>,
    pub claude_api_key: Option<String>,
    pub ollama_url: Option<String>,
    pub ollama_model: Option<String>,
    pub cloudflare_account_id: Option<String>,
    pub cloudflare_api_token: Option<String>,
    pub fireworks_api_key: Option<String>,
    pub fireworks_model: Option<String>,
    pub validator_url: Option<String>,
    pub validator_api_key: Option<String>,
    pub validator_model: Option<String>,
    pub validator_provider: Option<String>,
    /// `VALIDATOR_PIPELINE=stage` enables stage pipeline; default is legacy.
    pub validator_pipeline: ValidatorPipeline,
    pub admin_account: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3306/deagentnet".to_string());

        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .unwrap_or(3000);

        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let claude_api_key = env::var("CLAUDE_API_KEY").ok();
        let ollama_url = env::var("OLLAMA_URL").ok();
        let ollama_model = env::var("OLLAMA_MODEL").ok();
        let cloudflare_account_id = env::var("CLOUDFLARE_ACCOUNT_ID").ok();
        let cloudflare_api_token = env::var("CLOUDFLARE_API_TOKEN").ok();
        let fireworks_api_key = env::var("FIREWORKS_API_KEY").ok();
        let fireworks_model = env::var("FIREWORKS_MODEL").ok();
        let validator_url = env::var("VALIDATOR_LLM_URL").ok();
        let validator_api_key = env::var("VALIDATOR_LLM_API_KEY").ok();
        let validator_model = env::var("VALIDATOR_LLM_MODEL").ok();
        let validator_provider = env::var("VALIDATOR_PROVIDER").ok();
        let validator_pipeline = ValidatorPipeline::from_env();

        let admin_account = env::var("ADMIN_ACCOUNT").unwrap_or_else(|_| {
            "ac7a93e16ccf32fa9d91d387c9fb84521e23fdae8ce57263d173beafab5fc1b8".to_string()
        });

        Config {
            database_url,
            port,
            openai_api_key,
            claude_api_key,
            ollama_url,
            ollama_model,
            cloudflare_account_id,
            cloudflare_api_token,
            fireworks_api_key,
            fireworks_model,
            validator_url,
            validator_api_key,
            validator_model,
            validator_provider,
            validator_pipeline,
            admin_account,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_pipeline_defaults_to_legacy() {
        unsafe {
            std::env::remove_var("VALIDATOR_PIPELINE");
        }
        assert_eq!(ValidatorPipeline::from_env(), ValidatorPipeline::Legacy);
    }

    #[test]
    fn validator_pipeline_stage_from_env() {
        unsafe {
            std::env::set_var("VALIDATOR_PIPELINE", "stage");
        }
        assert_eq!(ValidatorPipeline::from_env(), ValidatorPipeline::Stage);
        unsafe {
            std::env::remove_var("VALIDATOR_PIPELINE");
        }
    }
}
