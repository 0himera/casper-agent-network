use crate::api::AppState;
use crate::api::x402::verify_payment;
use crate::db::models::Reputation;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReputationSkillEntry {
    pub skill: String,
    pub score: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReputationSnapshot {
    pub agent_public_key: String,
    pub timestamp_ms: u64,
    pub skills_reputation: Vec<ReputationSkillEntry>,
    pub signer: String,
    pub signature: String,
}

pub fn create_reputation_snapshot_payload(
    agent_public_key: &str,
    timestamp_ms: u64,
    skills: &[ReputationSkillEntry],
) -> String {
    let skills_str = serde_json::to_string(skills).unwrap_or_default();
    format!("{}:{}:{}", agent_public_key, timestamp_ms, skills_str)
}

pub fn sign_reputation_snapshot(
    agent_public_key: &str,
    timestamp_ms: u64,
    skills: &[ReputationSkillEntry],
    signer_key_hex: &str,
) -> (String, String) {
    use sha2::{Digest, Sha256};
    let payload = create_reputation_snapshot_payload(agent_public_key, timestamp_ms, skills);
    let hash = Sha256::digest(payload.as_bytes());
    let sig_hex = hex::encode(hash);
    (signer_key_hex.to_string(), sig_hex)
}

pub fn verify_reputation_snapshot(snapshot: &ReputationSnapshot) -> bool {
    use sha2::{Digest, Sha256};
    let payload = create_reputation_snapshot_payload(
        &snapshot.agent_public_key,
        snapshot.timestamp_ms,
        &snapshot.skills_reputation,
    );
    let expected_hash = hex::encode(Sha256::digest(payload.as_bytes()));
    snapshot.signature == expected_hash
}

pub async fn get_reputations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // 0.01 CSPR = 10,000,000 motes
    verify_payment(
        &headers,
        &state.pool,
        &state.casper_client,
        10_000_000,
        &state.config.admin_account,
    )
    .await?;

    let reputations =
        sqlx::query_as::<_, Reputation>("SELECT * FROM reputations ORDER BY timestamp DESC")
            .fetch_all(&state.pool)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;

    Ok(Json(serde_json::json!(reputations)))
}

pub async fn get_agent_reputations(
    State(state): State<AppState>,
    Path(agent_pubkey): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let reputations = sqlx::query_as::<_, Reputation>(
        "SELECT * FROM reputations WHERE agent_public_key = ? ORDER BY score DESC",
    )
    .bind(agent_pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(reputations))
}

pub async fn get_reputation_snapshot(
    State(state): State<AppState>,
    Path(agent_pubkey): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let reputations = sqlx::query_as::<_, Reputation>(
        "SELECT * FROM reputations WHERE agent_public_key = ? ORDER BY score DESC",
    )
    .bind(&agent_pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let skills: Vec<ReputationSkillEntry> = reputations
        .into_iter()
        .map(|r| ReputationSkillEntry {
            skill: r.skill,
            score: r.score,
        })
        .collect();

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let signer = state.config.admin_account.clone();
    let (signer_pk, signature) = sign_reputation_snapshot(&agent_pubkey, now_ms, &skills, &signer);

    let snapshot = ReputationSnapshot {
        agent_public_key: agent_pubkey,
        timestamp_ms: now_ms,
        skills_reputation: skills,
        signer: signer_pk,
        signature,
    };

    Ok(Json(serde_json::json!(snapshot)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify_reputation_snapshot() {
        let agent_pk = "01abc123def456";
        let now = 1700000000000u64;
        let skills = vec![
            ReputationSkillEntry {
                skill: "DeFi".to_string(),
                score: 95,
            },
            ReputationSkillEntry {
                skill: "Security".to_string(),
                score: 88,
            },
        ];

        let (signer, signature) = sign_reputation_snapshot(agent_pk, now, &skills, "admin_signer");

        let snapshot = ReputationSnapshot {
            agent_public_key: agent_pk.to_string(),
            timestamp_ms: now,
            skills_reputation: skills,
            signer,
            signature,
        };

        assert!(verify_reputation_snapshot(&snapshot));
    }

    #[test]
    fn test_verify_reputation_snapshot_detects_tampering() {
        let agent_pk = "01abc123def456";
        let now = 1700000000000u64;
        let skills = vec![ReputationSkillEntry {
            skill: "DeFi".to_string(),
            score: 95,
        }];

        let (signer, signature) = sign_reputation_snapshot(agent_pk, now, &skills, "admin_signer");

        let mut tampered_snapshot = ReputationSnapshot {
            agent_public_key: agent_pk.to_string(),
            timestamp_ms: now,
            skills_reputation: skills,
            signer,
            signature,
        };

        // Tamper score
        tampered_snapshot.skills_reputation[0].score = 100;
        assert!(!verify_reputation_snapshot(&tampered_snapshot));
    }
}
