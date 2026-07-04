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

/// Agent selection policy for exam dispatch (middle scenario Phase 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExamSelectionMode {
    /// As-built bucket + probability gate (rollback path).
    Bucket,
    /// Per-agent `exam_urgency` ranking with frequency cap (default path).
    Urgency,
}

impl ExamSelectionMode {
    /// Reads `EXAM_SELECTION_MODE`; defaults to `urgency`.
    pub fn from_env() -> Self {
        match env::var("EXAM_SELECTION_MODE")
            .ok()
            .as_deref()
            .map(str::trim)
        {
            Some("bucket") => Self::Bucket,
            _ => Self::Urgency,
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
    /// E7: enable in-process background exam dispatch loop (default off).
    pub exam_dispatch_loop_enabled: bool,
    /// E7: interval between background dispatch attempts (seconds).
    pub exam_dispatch_loop_interval_secs: u64,
    /// Phase 3: `bucket` (default) or `urgency` selection mode.
    pub exam_selection_mode: ExamSelectionMode,
    /// Phase 3: base per-agent dispatch probability in urgency mode.
    pub exam_urgency_base_prob: f32,
    /// Phase 3: weight for `tasks_since_last_exam` in urgency formula.
    pub exam_urgency_task_weight: f64,
    /// Phase 3: weight for verdict instability in urgency formula.
    pub exam_urgency_variance_weight: f64,
    /// Phase 3: number of recent validated exam verdicts for instability window.
    pub exam_urgency_recent_verdicts: u32,
    /// Phase 4: EMA decay factor for off-chain `smoothed_score` (does not affect on-chain submit).
    pub exam_smoothed_ema_alpha: f64,
    /// Phase 5: use off-chain `smoothed_score` in global leaderboard (default off).
    pub exam_leaderboard_use_smoothed: bool,
}

/// Clamp EMA alpha to `(0.0, 1.0]`; invalid values fall back to `0.3`.
pub fn clamp_ema_alpha(alpha: f64) -> f64 {
    const DEFAULT: f64 = 0.3;
    if !alpha.is_finite() || alpha <= 0.0 || alpha > 1.0 {
        DEFAULT
    } else {
        alpha
    }
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

        let exam_dispatch_loop_enabled = env::var("EXAM_DISPATCH_LOOP_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);

        let exam_dispatch_loop_interval_secs = env::var("EXAM_DISPATCH_LOOP_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300)
            .max(1);

        let exam_selection_mode = ExamSelectionMode::from_env();

        let exam_urgency_base_prob = clamp_dispatch_probability(
            env::var("EXAM_URGENCY_BASE_PROB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.1),
        );

        let exam_urgency_task_weight = env::var("EXAM_URGENCY_TASK_WEIGHT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.05);

        let exam_urgency_variance_weight = env::var("EXAM_URGENCY_VARIANCE_WEIGHT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.2);

        let exam_urgency_recent_verdicts = env::var("EXAM_URGENCY_RECENT_VERDICTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5)
            .max(1);

        let exam_smoothed_ema_alpha = clamp_ema_alpha(
            env::var("EXAM_SMOOTHED_EMA_ALPHA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.3),
        );

        let exam_leaderboard_use_smoothed = env::var("EXAM_LEADERBOARD_USE_SMOOTHED")
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
            exam_dispatch_loop_enabled,
            exam_dispatch_loop_interval_secs,
            exam_selection_mode,
            exam_urgency_base_prob,
            exam_urgency_task_weight,
            exam_urgency_variance_weight,
            exam_urgency_recent_verdicts,
            exam_smoothed_ema_alpha,
            exam_leaderboard_use_smoothed,
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
                exam_dispatch_loop_enabled: false,
                exam_dispatch_loop_interval_secs: 300,
                exam_selection_mode: crate::config::ExamSelectionMode::Bucket,
                exam_urgency_base_prob: 0.1,
                exam_urgency_task_weight: 0.05,
                exam_urgency_variance_weight: 0.2,
                exam_urgency_recent_verdicts: 5,
                exam_smoothed_ema_alpha: 0.3,
                exam_leaderboard_use_smoothed: false,
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
    fn exam_dispatch_loop_defaults_to_enabled() {
        temp_env::with_vars(
            [
                ("EXAM_DISPATCH_LOOP_ENABLED", None::<&str>),
                ("EXAM_DISPATCH_LOOP_INTERVAL_SECS", None::<&str>),
            ],
            || {
                let config = Config::from_env();
                assert!(config.exam_dispatch_loop_enabled);
                assert_eq!(config.exam_dispatch_loop_interval_secs, 300);
            },
        );
    }

    #[test]
    fn exam_dispatch_loop_reads_from_env() {
        temp_env::with_vars(
            [
                ("EXAM_DISPATCH_LOOP_ENABLED", Some("0")),
                ("EXAM_DISPATCH_LOOP_INTERVAL_SECS", Some("60")),
            ],
            || {
                let config = Config::from_env();
                assert!(!config.exam_dispatch_loop_enabled);
                assert_eq!(config.exam_dispatch_loop_interval_secs, 60);
            },
        );
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

    #[test]
    fn exam_selection_mode_defaults_to_urgency() {
        temp_env::with_var("EXAM_SELECTION_MODE", None::<&str>, || {
            let config = Config::from_env();
            assert_eq!(config.exam_selection_mode, ExamSelectionMode::Urgency);
        });
    }

    #[test]
    fn exam_selection_mode_reads_bucket_from_env() {
        temp_env::with_var("EXAM_SELECTION_MODE", Some("bucket"), || {
            let config = Config::from_env();
            assert_eq!(config.exam_selection_mode, ExamSelectionMode::Bucket);
        });
    }

    #[test]
    fn exam_smoothed_ema_alpha_defaults_to_03() {
        temp_env::with_var("EXAM_SMOOTHED_EMA_ALPHA", None::<&str>, || {
            let config = Config::from_env();
            assert!((config.exam_smoothed_ema_alpha - 0.3).abs() < f64::EPSILON);
        });
    }

    #[test]
    fn exam_smoothed_ema_alpha_reads_from_env() {
        temp_env::with_var("EXAM_SMOOTHED_EMA_ALPHA", Some("0.5"), || {
            let config = Config::from_env();
            assert!((config.exam_smoothed_ema_alpha - 0.5).abs() < f64::EPSILON);
        });
    }

    #[test]
    fn clamp_ema_alpha_rejects_invalid_values() {
        assert!((clamp_ema_alpha(-0.1) - 0.3).abs() < f64::EPSILON);
        assert!((clamp_ema_alpha(0.0) - 0.3).abs() < f64::EPSILON);
        assert!((clamp_ema_alpha(1.5) - 0.3).abs() < f64::EPSILON);
        assert!((clamp_ema_alpha(f64::NAN) - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn exam_leaderboard_use_smoothed_defaults_to_false() {
        temp_env::with_var("EXAM_LEADERBOARD_USE_SMOOTHED", None::<&str>, || {
            let config = Config::from_env();
            assert!(!config.exam_leaderboard_use_smoothed);
        });
    }

    #[test]
    fn exam_leaderboard_use_smoothed_reads_from_env() {
        temp_env::with_var("EXAM_LEADERBOARD_USE_SMOOTHED", Some("1"), || {
            let config = Config::from_env();
            assert!(config.exam_leaderboard_use_smoothed);
        });
    }
}
