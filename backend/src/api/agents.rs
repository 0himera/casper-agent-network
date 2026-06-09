use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use crate::api::AppState;
use crate::db::models::Agent;
use crate::orchestrator::benchmark::start_benchmark_background;

#[derive(Deserialize)]
pub struct RegisterAgentPayload {
    pub public_key: String,
    pub name: String,
    pub description: Option<String>,
    pub metadata_uri: Option<String>,
    pub endpoint_url: Option<String>,
    pub api_key: Option<String>,
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
    let agents = sqlx::query_as::<_, Agent>(
        "SELECT * FROM agents ORDER BY timestamp DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(agents))
}

pub async fn get_agent(
    State(state): State<AppState>,
    Path(public_key): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent = sqlx::query_as::<_, Agent>(
        "SELECT * FROM agents WHERE public_key = ?"
    )
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
    Json(payload): Json<RegisterAgentPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. Check if agent already exists
    let agent_opt: Option<(String,)> = sqlx::query_as(
        "SELECT status FROM agents WHERE public_key = ?"
    )
    .bind(&payload.public_key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if agent_opt.is_some() {
        // Update existing agent with benchmarking configuration
        sqlx::query(
            "UPDATE agents 
             SET name = ?, description = ?, metadata_uri = ?, endpoint_url = ?, api_key = ?, system_prompt = ?, status = 'benchmarking' 
             WHERE public_key = ?"
        )
        .bind(&payload.name)
        .bind(&payload.description)
        .bind(&payload.metadata_uri)
        .bind(&payload.endpoint_url)
        .bind(&payload.api_key)
        .bind(&payload.system_prompt)
        .bind(&payload.public_key)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        // Insert agent with 'benchmarking' status
        sqlx::query(
            "INSERT INTO agents (public_key, name, description, metadata_uri, endpoint_url, api_key, system_prompt, status) 
             VALUES (?, ?, ?, ?, ?, ?, ?, 'benchmarking')"
        )
        .bind(&payload.public_key)
        .bind(&payload.name)
        .bind(&payload.description)
        .bind(&payload.metadata_uri)
        .bind(&payload.endpoint_url)
        .bind(&payload.api_key)
        .bind(&payload.system_prompt)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // 3. Start benchmarking in background
    let skills = if payload.skills.is_empty() {
        vec!["defi_analysis".to_string()]
    } else {
        payload.skills
    };

    start_benchmark_background(
        state.pool.clone(),
        payload.public_key.clone(),
        skills,
        payload.endpoint_url,
        payload.api_key,
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
    let result = sqlx::query(
        "UPDATE agents SET custom_price_motes = ? WHERE public_key = ?"
    )
    .bind(payload.custom_price_motes)
    .bind(&public_key)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Agent not found".to_string()));
    }

    // 2. Fetch updated agent
    let agent = sqlx::query_as::<_, Agent>(
        "SELECT * FROM agents WHERE public_key = ?"
    )
    .bind(&public_key)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(agent))
}
