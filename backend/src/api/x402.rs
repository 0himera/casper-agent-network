use crate::casper::contract::CasperClient;
use crate::db::DbPool;
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, Clone)]
pub struct XPaymentHeader {
    #[serde(rename = "x402Version")]
    pub x402_version: u32,
    pub scheme: String,
    pub network: String,
    pub payload: XPaymentPayload,
}

#[derive(Debug, Deserialize, Clone)]
pub struct XPaymentPayload {
    pub txid: String,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(rename = "paymentType", default)]
    pub payment_type: Option<String>,
}

/// Decodes X-Payment header using autodetect (direct JSON, Base64, or Hex).
#[allow(clippy::collapsible_if)]
pub fn parse_x_payment_header(input: &str) -> Result<XPaymentHeader, String> {
    let trimmed = input.trim();

    // 1. Try direct JSON parsing
    if let Ok(xp) = serde_json::from_str::<XPaymentHeader>(trimmed) {
        return Ok(xp);
    }

    // 2. Try base64 decoding
    if let Ok(bytes) = base64_decode(trimmed) {
        if let Ok(xp) = serde_json::from_slice::<XPaymentHeader>(&bytes) {
            return Ok(xp);
        }
    }

    // 3. Try hex decoding
    if let Ok(bytes) = hex::decode(trimmed) {
        if let Ok(xp) = serde_json::from_slice::<XPaymentHeader>(&bytes) {
            return Ok(xp);
        }
    }

    Err("Failed to parse X-Payment header as direct JSON, Base64, or Hex".to_string())
}

/// Makes a whitepaper-compliant HTTP 402 Payment Required response with WWW-Authenticate header.
pub fn make_402_challenge_response(
    price_motes: u64,
    merchant_pubkey: &str,
    resource: &str,
    description: &str,
) -> Response {
    let body = json!({
        "x402Version": 1,
        "scheme": "exact",
        "network": "casper-testnet",
        "asset": "CSPR",
        "maxAmountRequired": price_motes.to_string(),
        "resource": resource,
        "description": description,
        "mimeType": "application/json",
        "payTo": merchant_pubkey,
        "outputSchema": null
    });

    (
        StatusCode::PAYMENT_REQUIRED,
        [
            ("WWW-Authenticate", "x402"),
            ("Content-Type", "application/json"),
        ],
        Json(body),
    )
        .into_response()
}

/// Legacy tuple helper for existing handlers.
pub fn make_402_challenge(
    price_motes: u64,
    merchant_pubkey: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::PAYMENT_REQUIRED,
        Json(json!({
            "x402Version": 1,
            "scheme": "exact",
            "network": "casper-testnet",
            "asset": "CSPR",
            "maxAmountRequired": price_motes.to_string(),
            "payTo": merchant_pubkey,
            "resource": "https://api.can.dev",
            "description": "Casper Agent Network x402 protected resource",
            "mimeType": "application/json",
            "outputSchema": null
        })),
    )
}

/// Verifies if x402 payment requirements are met.
/// Returns Ok(()) if payment is verified, or Err((StatusCode, Json)) to return 402 challenge.
pub async fn verify_payment(
    headers: &HeaderMap,
    pool: &DbPool,
    casper_client: &CasperClient,
    expected_amount_motes: u64,
    merchant_pubkey: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if std::env::var("DISABLE_X402")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return Ok(());
    }

    // 1. Get X-Payment header
    let x_payment_val = match headers.get("X-Payment") {
        Some(val) => val,
        None => return Err(make_402_challenge(expected_amount_motes, merchant_pubkey)),
    };

    let x_payment_str = match x_payment_val.to_str() {
        Ok(s) => s,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid X-Payment header format" })),
            ));
        }
    };

    // 2. Decode header (autodetect JSON, Base64, or Hex)
    let x_payment = match parse_x_payment_header(x_payment_str) {
        Ok(xp) => xp,
        Err(err) => {
            return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": err }))));
        }
    };

    let deploy_hash = &x_payment.payload.txid;

    // 3. Atomically check and reserve deploy_hash in spent_payments (INSERT IGNORE prevents race conditions)
    let insert_res = sqlx::query("INSERT IGNORE INTO spent_payments (deploy_hash) VALUES (?)")
        .bind(deploy_hash)
        .execute(pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?;

    if insert_res.rows_affected() == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Payment proof already spent" })),
        ));
    }

    // 4. Verify on-chain payment proof
    let is_verified = casper_client
        .verify_payment_proof(deploy_hash, expected_amount_motes, merchant_pubkey)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e })),
            )
        })?;

    if !is_verified {
        // Rollback reservation if verification fails
        let _ = sqlx::query("DELETE FROM spent_payments WHERE deploy_hash = ?")
            .bind(deploy_hash)
            .execute(pool)
            .await;
        return Err(make_402_challenge(expected_amount_motes, merchant_pubkey));
    }

    Ok(())
}

/// Cleans up spent payment records older than `max_age_hours` (default 24h).
pub async fn cleanup_old_spent_payments(
    pool: &DbPool,
    max_age_hours: u32,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM spent_payments WHERE timestamp < NOW() - INTERVAL ? HOUR")
        .bind(max_age_hours)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Spawns a background task that cleans up expired spent payments every hour.
pub fn spawn_spent_payments_cleanup_loop(pool: DbPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // once per hour
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            tracing::info!("Running spent payments cleanup job...");
            match cleanup_old_spent_payments(&pool, 24).await {
                Ok(rows) => {
                    if rows > 0 {
                        tracing::info!("Cleaned up {} expired spent payment records", rows);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to clean up expired spent payments: {}", e);
                }
            }
        }
    })
}

fn base64_decode(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.decode(input.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose};

    #[test]
    fn test_parse_direct_json_header() {
        let json_str = r#"{
            "x402Version": 1,
            "scheme": "exact",
            "network": "casper-testnet",
            "payload": {
                "txid": "deploy_hash_123"
            }
        }"#;

        let parsed = parse_x_payment_header(json_str).unwrap();
        assert_eq!(parsed.x402_version, 1);
        assert_eq!(parsed.scheme, "exact");
        assert_eq!(parsed.payload.txid, "deploy_hash_123");
    }

    #[test]
    fn test_parse_base64_json_header() {
        let json_str = r#"{"x402Version":1,"scheme":"exact","network":"casper-testnet","payload":{"txid":"deploy_hash_456"}}"#;
        let b64 = general_purpose::STANDARD.encode(json_str.as_bytes());

        let parsed = parse_x_payment_header(&b64).unwrap();
        assert_eq!(parsed.payload.txid, "deploy_hash_456");
    }

    #[test]
    fn test_make_402_challenge_response() {
        let resp = make_402_challenge_response(
            10_000_000,
            "01abc...",
            "https://api.can.dev/agents",
            "List agents",
        );
        assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);
        assert_eq!(resp.headers().get("WWW-Authenticate").unwrap(), "x402");
    }
}
