use serde::{Deserialize, Serialize};

/// Configuration for the standalone validator node service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorNodeConfig {
    pub enabled: bool,
    pub database_url: String,
    pub poll_interval_secs: u64,
    pub validator_secret_key_path: Option<String>,
    pub validator_public_key: Option<String>,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub min_validations: u32,
    pub validation_window_secs: u64,
    pub fireworks_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
}

impl Default for ValidatorNodeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            database_url: "mysql://root:rootpassword@127.0.0.1:3306/cspr_agent_network".to_string(),
            poll_interval_secs: 15,
            validator_secret_key_path: None,
            validator_public_key: None,
            llm_provider: None,
            llm_model: None,
            min_validations: 3,
            validation_window_secs: 300,
            fireworks_api_key: None,
            gemini_api_key: None,
            openrouter_api_key: None,
        }
    }
}

impl ValidatorNodeConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let enabled = std::env::var("VALIDATOR_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "mysql://root:rootpassword@127.0.0.1:3306/cspr_agent_network".to_string()
        });

        let poll_interval_secs = std::env::var("POLL_INTERVAL_SECS")
            .or_else(|_| std::env::var("VALIDATOR_POLL_INTERVAL_SECS"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15);

        let validator_secret_key_path = std::env::var("VALIDATOR_SECRET_KEY_PATH")
            .ok()
            .filter(|v| !v.is_empty());

        let validator_public_key = std::env::var("VALIDATOR_PUBLIC_KEY")
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| {
                std::env::var("VALIDATOR_NODE_ID")
                    .ok()
                    .filter(|v| !v.is_empty())
            });

        let llm_provider = std::env::var("VALIDATOR_LLM_PROVIDER")
            .ok()
            .filter(|v| !v.is_empty());

        let llm_model = std::env::var("VALIDATOR_LLM_MODEL")
            .ok()
            .filter(|v| !v.is_empty());

        let min_validations = std::env::var("VALIDATOR_MIN_VALIDATIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        let validation_window_secs = std::env::var("VALIDATOR_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);

        let fireworks_api_key = std::env::var("FIREWORKS_API_KEY")
            .ok()
            .filter(|v| !v.is_empty());

        let gemini_api_key = std::env::var("GEMINI_API_KEY")
            .ok()
            .filter(|v| !v.is_empty());

        let openrouter_api_key = std::env::var("OPENROUTER_API_KEY")
            .ok()
            .filter(|v| !v.is_empty());

        Self {
            enabled,
            database_url,
            poll_interval_secs,
            validator_secret_key_path,
            validator_public_key,
            llm_provider,
            llm_model,
            min_validations,
            validation_window_secs,
            fireworks_api_key,
            gemini_api_key,
            openrouter_api_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_defaults() {
        let cfg = ValidatorNodeConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.poll_interval_secs, 15);
        assert_eq!(cfg.min_validations, 3);
        assert_eq!(cfg.validation_window_secs, 300);
    }
}
