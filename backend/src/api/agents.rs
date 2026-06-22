use crate::api::AppState;
use crate::api::x402::verify_payment;
use crate::db::models::Agent;
use crate::orchestrator::benchmark::{normalize_benchmark_domain, start_benchmark_background};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RegisterAgentPayload {
    pub public_key: String,
    pub name: String,
    pub description: Option<String>,
    pub metadata_uri: Option<String>,
    pub endpoint_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub skills: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdatePricePayload {
    pub custom_price_motes: u64,
}

pub async fn get_agents(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agents = sqlx::query_as::<_, Agent>("SELECT * FROM agents ORDER BY timestamp DESC")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(agents))
}

pub async fn get_agent(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE public_key = ?")
        .bind(public_key)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match agent {
        Some(agent) => Ok(Json(agent)),
        None => Err((StatusCode::NOT_FOUND, "Agent not found".to_string())),
    }
}

pub async fn register_agent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterAgentPayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // 0.1 CSPR = 100,000,000 motes
    if let Err(e) = verify_payment(
        &headers,
        &state.pool,
        &state.casper_client,
        100_000_000,
        &state.config.admin_account,
    )
    .await
    {
        return Err(e);
    }

    // 1. Check if agent already exists
    let agent_opt: Option<(String,)> =
        sqlx::query_as("SELECT status FROM agents WHERE public_key = ?")
            .bind(&payload.public_key)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;

    if agent_opt.is_some() {
        // Update existing agent with benchmarking configuration
        sqlx::query(
            "UPDATE agents 
             SET name = ?, description = ?, metadata_uri = ?, endpoint_url = ?, api_key = ?, model = ?, system_prompt = ?, status = 'benchmarking' 
             WHERE public_key = ?"
        )
        .bind(&payload.name)
        .bind(&payload.description)
        .bind(&payload.metadata_uri)
        .bind(&payload.endpoint_url)
        .bind(&payload.api_key)
        .bind(&payload.model)
        .bind(&payload.system_prompt)
        .bind(&payload.public_key)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;
    } else {
        // Insert agent with 'benchmarking' status
        sqlx::query(
            "INSERT INTO agents (public_key, name, description, metadata_uri, endpoint_url, api_key, model, system_prompt, status) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'benchmarking')"
        )
        .bind(&payload.public_key)
        .bind(&payload.name)
        .bind(&payload.description)
        .bind(&payload.metadata_uri)
        .bind(&payload.endpoint_url)
        .bind(&payload.api_key)
        .bind(&payload.model)
        .bind(&payload.system_prompt)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;
    }

    // 3. Start benchmarking in background
    let skills = if payload.skills.is_empty() {
        vec!["defi".to_string()]
    } else {
        payload
            .skills
            .iter()
            .filter_map(|skill| normalize_benchmark_domain(skill).map(str::to_string))
            .collect()
    };

    start_benchmark_background(
        state.pool.clone(),
        payload.public_key.clone(),
        skills,
        payload.endpoint_url,
        payload.api_key,
        payload.model,
        payload.system_prompt,
        state.config.clone(),
    );

    // 4. Return initial response
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "benchmarking",
            "message": "Agent registered successfully. Benchmark started in the background."
        })),
    ))
}

pub async fn update_agent_price(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
    Json(payload): Json<UpdatePricePayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. Update price
    let result = sqlx::query("UPDATE agents SET custom_price_motes = ? WHERE public_key = ?")
        .bind(payload.custom_price_motes)
        .bind(&public_key)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Agent not found".to_string()));
    }

    // 2. Fetch updated agent
    let agent = sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE public_key = ?")
        .bind(&public_key)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(agent))
}

#[derive(Deserialize)]
pub struct UpdateCapabilitiesPayload {
    pub name: Option<String>,
    pub endpoint_url: Option<String>,
    pub skills: Vec<String>,
    pub system_prompt: Option<String>,
}

pub async fn update_agent_capabilities(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateCapabilitiesPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Some(expected_key) = &state.config.internal_service_key {
        let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
        if auth_header != Some(expected_key.as_str()) {
            return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
        }
    }

    let name = payload.name.unwrap_or_else(|| "Autonomous Agent".to_string());
    let _ = sqlx::query(
        "INSERT INTO agents (public_key, name, endpoint_url, system_prompt, status)
         VALUES (?, ?, ?, ?, 'active')
         ON DUPLICATE KEY UPDATE endpoint_url = ?, system_prompt = ?",
    )
    .bind(&public_key)
    .bind(&name)
    .bind(&payload.endpoint_url)
    .bind(&payload.system_prompt)
    .bind(&payload.endpoint_url)
    .bind(&payload.system_prompt)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("Capabilities updated for agent {}", public_key);
    Ok(StatusCode::OK)
}

pub async fn get_agent_benchmarks(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let runs = sqlx::query_as::<_, crate::db::models::BenchmarkRun>(
        "SELECT * FROM benchmark_runs WHERE agent_public_key = ? ORDER BY timestamp DESC",
    )
    .bind(public_key)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(runs))
}
