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
    pub validator_pipeline: ValidatorPipeline,
    pub admin_account: String,
    pub internal_service_key: Option<String>,
    /// Reputation weight for exam tasks (used in E3 on-chain completion).
    pub exam_weight: u32,
    /// Probability (0.0–1.0) of dispatching to an audit-bucket agent per attempt.
    pub exam_dispatch_prob_audit: f32,
    /// Probability (0.0–1.0) of dispatching to a rehab-bucket agent per attempt.
    pub exam_dispatch_prob_rehab: f32,
    /// Max exam assignments per agent within the dispatch period window.
    pub exam_max_per_agent_per_period: u32,
    /// Rolling window (hours) for frequency cap on exam dispatch.
    pub exam_dispatch_period_hours: u64,
    /// Agents at or below this global reputation sum land in the rehab bucket.
    pub exam_rehab_score_threshold: i32,
    /// Agents with at least this many active jobs qualify for the audit bucket.
    pub exam_audit_active_jobs_threshold: i32,
    /// Escrow budget (motes) for platform-dispatched exam tasks.
    pub exam_dispatch_budget_motes: u64,
    /// Creator public key stored on dispatched exam tasks (platform wallet).
    pub exam_dispatch_creator_public_key: String,
    /// Post-MVP (E6): enable LLM semantic equality fallback after exact mismatch.
    pub exam_llm_equality: bool,
}

/// Clamp dispatch probability to anti-gaming range `(0.0, 1.0)`; `>= 1.0` becomes `0.99`.
pub fn clamp_dispatch_probability(prob: f32) -> f32 {
    if !prob.is_finite() || prob <= 0.0 {
        return 0.0;
    }
    if prob >= 1.0 {
        return 0.99;
    }
    prob
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

        let internal_service_key = env::var("INTERNAL_SERVICE_KEY").ok();

        let exam_weight = env::var("EXAM_WEIGHT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);

        let exam_dispatch_prob_audit = clamp_dispatch_probability(
            env::var("EXAM_DISPATCH_PROB_AUDIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.2),
        );

        let exam_dispatch_prob_rehab = clamp_dispatch_probability(
            env::var("EXAM_DISPATCH_PROB_REHAB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5),
        );

        let exam_max_per_agent_per_period = env::var("EXAM_MAX_PER_AGENT_PER_PERIOD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let exam_dispatch_period_hours = env::var("EXAM_DISPATCH_PERIOD_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24);

        let exam_rehab_score_threshold = env::var("EXAM_REHAB_SCORE_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let exam_audit_active_jobs_threshold = env::var("EXAM_AUDIT_ACTIVE_JOBS_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);

        let exam_dispatch_budget_motes = env::var("EXAM_DISPATCH_BUDGET_MOTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5_000_000_000);

        let exam_dispatch_creator_public_key =
            env::var("EXAM_DISPATCH_CREATOR_PUBLIC_KEY").unwrap_or_else(|_| admin_account.clone());

        let exam_llm_equality = env::var("EXAM_LLM_EQUALITY")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

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
            internal_service_key,
            exam_weight,
            exam_dispatch_prob_audit,
            exam_dispatch_prob_rehab,
            exam_max_per_agent_per_period,
            exam_dispatch_period_hours,
            exam_rehab_score_threshold,
            exam_audit_active_jobs_threshold,
            exam_dispatch_budget_motes,
            exam_dispatch_creator_public_key,
            exam_llm_equality,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_pipeline_defaults_to_legacy() {
        temp_env::with_var("VALIDATOR_PIPELINE", None::<&str>, || {
            assert_eq!(ValidatorPipeline::from_env(), ValidatorPipeline::Legacy);
        });
    }

    #[test]
    fn exam_weight_defaults_to_300() {
        temp_env::with_var("EXAM_WEIGHT", None::<&str>, || {
            let config = Config {
                database_url: String::new(),
                port: 3000,
                openai_api_key: None,
                claude_api_key: None,
                ollama_url: None,
                ollama_model: None,
                cloudflare_account_id: None,
                cloudflare_api_token: None,
                fireworks_api_key: None,
                fireworks_model: None,
                validator_url: None,
                validator_api_key: None,
                validator_model: None,
                validator_provider: None,
                validator_pipeline: ValidatorPipeline::Legacy,
                admin_account: String::new(),
                internal_service_key: None,
                exam_weight: env::var("EXAM_WEIGHT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(300),
                exam_dispatch_prob_audit: 0.2,
                exam_dispatch_prob_rehab: 0.5,
                exam_max_per_agent_per_period: 1,
                exam_dispatch_period_hours: 24,
                exam_rehab_score_threshold: 0,
                exam_audit_active_jobs_threshold: 2,
                exam_dispatch_budget_motes: 5_000_000_000,
                exam_dispatch_creator_public_key: String::new(),
                exam_llm_equality: false,
            };
            assert_eq!(config.exam_weight, 300);
        });
    }

    #[test]
    fn exam_weight_reads_from_env() {
        temp_env::with_var("EXAM_WEIGHT", Some("450"), || {
            let weight = env::var("EXAM_WEIGHT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300);
            assert_eq!(weight, 450);
        });
    }

    #[test]
    fn exam_llm_equality_defaults_to_false() {
        temp_env::with_var("EXAM_LLM_EQUALITY", None::<&str>, || {
            let config = Config::from_env();
            assert!(!config.exam_llm_equality);
        });
    }

    #[test]
    fn exam_llm_equality_reads_from_env() {
        temp_env::with_var("EXAM_LLM_EQUALITY", Some("1"), || {
            let config = Config::from_env();
            assert!(config.exam_llm_equality);
        });
    }

    #[test]
    fn clamp_dispatch_probability_caps_at_099() {
        assert_eq!(clamp_dispatch_probability(1.0), 0.99);
        assert_eq!(clamp_dispatch_probability(1.5), 0.99);
    }

    #[test]
    fn clamp_dispatch_probability_zero_for_non_positive() {
        assert_eq!(clamp_dispatch_probability(0.0), 0.0);
        assert_eq!(clamp_dispatch_probability(-0.1), 0.0);
    }

    #[test]
    fn exam_dispatch_prob_clamped_from_env() {
        temp_env::with_vars(
            [
                ("EXAM_DISPATCH_PROB_AUDIT", Some("1.0")),
                ("EXAM_DISPATCH_PROB_REHAB", Some("0.5")),
            ],
            || {
                let config = Config::from_env();
                assert_eq!(config.exam_dispatch_prob_audit, 0.99);
                assert_eq!(config.exam_dispatch_prob_rehab, 0.5);
            },
        );
    }
}
