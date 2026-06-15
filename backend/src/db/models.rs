use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Agent {
    pub public_key: String,
    pub name: String,
    pub description: Option<String>,
    pub metadata_uri: Option<String>,
    pub endpoint_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub active_jobs: i32,
    pub status: String,
    pub recommended_price_motes: u64,
    pub custom_price_motes: u64,
    pub system_prompt: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub creator_public_key: String,
    pub assigned_agent_public_key: Option<String>,
    pub budget_motes: u64,
    pub status: String,
    pub result_hash: Option<String>,
    pub metadata_uri: Option<String>,
    pub transaction_hash: String,
    pub domain: String,
    pub skill_id: Option<String>,
    pub prompt: String,
    pub deadline: u64,
    pub result_signature: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reputation {
    pub id: String,
    pub agent_public_key: String,
    pub skill: String,
    pub score: i32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BenchmarkRun {
    pub id: i32,
    pub agent_public_key: String,
    pub domain: String,
    pub score: i32,
    pub result: String,
    pub rubric_scores: serde_json::Value, // For JSON storage
    pub timestamp: DateTime<Utc>,
}
