use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::api::AppState;
use crate::exam_dispatch::{self, DispatchOutcome};

/// Verify admin dispatch auth. Fail closed when `INTERNAL_SERVICE_KEY` is unset.
fn verify_dispatch_auth(
    internal_service_key: &Option<String>,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, String)> {
    let Some(expected_key) = internal_service_key else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "INTERNAL_SERVICE_KEY is not configured".to_string(),
        ));
    };
    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
    if auth_header != Some(expected_key.as_str()) {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }
    Ok(())
}

/// Admin-only: dispatch one exam task to an eligible agent (E4).
pub async fn dispatch_exam_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    verify_dispatch_auth(&state.config.internal_service_key, &headers)?;

    let outcome: DispatchOutcome = exam_dispatch::dispatch_once(&state.pool, &state.config)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err))?;

    Ok(Json(outcome))
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as HttpStatus};
    use tower::ServiceExt;

    use crate::api::create_router;
    use crate::casper::contract::CasperClient;
    use crate::config::{Config, ValidatorPipeline};

    fn sample_config_with_key() -> Config {
        Config {
            database_url: "mysql://unused".into(),
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
            internal_service_key: Some("test-internal-key".into()),
            exam_weight: 300,
            exam_dispatch_prob_audit: 0.2,
            exam_dispatch_prob_rehab: 0.5,
            exam_max_per_agent_per_period: 1,
            exam_dispatch_period_hours: 24,
            exam_rehab_score_threshold: 0,
            exam_audit_active_jobs_threshold: 2,
            exam_dispatch_budget_motes: 5_000_000_000,
            exam_dispatch_creator_public_key: "admin".into(),
            exam_llm_equality: false,
        }
    }

    fn sample_config_without_key() -> Config {
        let mut config = sample_config_with_key();
        config.internal_service_key = None;
        config
    }

    async fn post_dispatch(app: Router, auth: Option<&str>) -> axum::http::Response<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/admin/exams/dispatch");
        if let Some(key) = auth {
            builder = builder.header("Authorization", key);
        }
        app.oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    fn make_router(config: Config) -> Router {
        let pool = sqlx::MySqlPool::connect_lazy("mysql://unused").unwrap();
        let casper = CasperClient::new(
            "https://api.testnet.cspr.cloud".into(),
            "test-key".into(),
            String::new(),
        );
        create_router(pool, config, casper)
    }

    #[tokio::test]
    async fn dispatch_endpoint_requires_auth_when_key_set() {
        let response = post_dispatch(make_router(sample_config_with_key()), None).await;
        assert_eq!(response.status(), HttpStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn dispatch_endpoint_rejects_when_internal_service_key_unset() {
        let response = post_dispatch(make_router(sample_config_without_key()), None).await;
        assert_eq!(response.status(), HttpStatus::SERVICE_UNAVAILABLE);
    }
}
