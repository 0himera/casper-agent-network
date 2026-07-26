use crate::casper::contract::CasperClient;
use crate::db::DbPool;
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use std::sync::OnceLock;
use sha2::{Sha256, Digest};
use rand::RngCore;

static X402_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

fn get_x402_secret() -> &'static [u8] {
    X402_SECRET.get_or_init(|| {
        std::env::var("X402_SECRET")
            .map(|s| s.into_bytes())
            .unwrap_or_else(|_| {
                let mut bytes = vec![0u8; 32];
                rand::thread_rng().fill_bytes(&mut bytes);
                bytes
            })
    })
}

pub fn generate_challenge_token(valid_until: u64, price_motes: u64, resource: &str) -> String {
    let secret = get_x402_secret();
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.update(valid_until.to_be_bytes());
    hasher.update(price_motes.to_be_bytes());
    hasher.update(resource.as_bytes());
    let sig = hex::encode(hasher.finalize());
    format!("{}.{}", valid_until, sig)
}

pub fn verify_challenge_token(token: &str, price_motes: u64, resource: &str) -> Result<(), String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 2 {
        return Err("Invalid challenge token format".to_string());
    }
    let valid_until_str = parts[0];
    let sig = parts[1];
    
    let valid_until: u64 = valid_until_str.parse().map_err(|_| "Invalid validUntil timestamp".to_string())?;
    
    let now = chrono::Utc::now().timestamp() as u64;
    if now > valid_until {
        return Err("Challenge token expired".to_string());
    }
    
    let expected_token = generate_challenge_token(valid_until, price_motes, resource);
    let expected_sig = expected_token.split('.').nth(1).unwrap_or("");
    
    if sig != expected_sig {
        return Err("Challenge token signature mismatch".to_string());
    }
    
    Ok(())
}

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
    #[serde(rename = "challengeToken", default)]
    pub challenge_token: Option<String>,
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
    let valid_until = (chrono::Utc::now().timestamp() + 300) as u64; // 5 minutes validity
    let token = generate_challenge_token(valid_until, price_motes, resource);

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
        "outputSchema": null,
        "validUntil": valid_until,
        "challengeToken": token
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
    let valid_until = (chrono::Utc::now().timestamp() + 300) as u64; // 5 minutes validity
    let resource = "https://api.can.dev";
    let token = generate_challenge_token(valid_until, price_motes, resource);

    (
        StatusCode::PAYMENT_REQUIRED,
        Json(json!({
            "x402Version": 1,
            "scheme": "exact",
            "network": "casper-testnet",
            "asset": "CSPR",
            "maxAmountRequired": price_motes.to_string(),
            "payTo": merchant_pubkey,
            "resource": resource,
            "description": "Casper Agent Network x402 protected resource",
            "mimeType": "application/json",
            "outputSchema": null,
            "validUntil": valid_until,
            "challengeToken": token
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

    // Verify challenge token
    let challenge_token = match x_payment.payload.challenge_token {
        Some(ref token) => token,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Missing challengeToken in X-Payment payload" })),
            ));
        }
    };

    let resource = "https://api.can.dev"; // default resource
    if let Err(err_msg) = verify_challenge_token(challenge_token, expected_amount_motes, resource) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Invalid challenge token: {}", err_msg) })),
        ));
    }

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

    /// Wave 4 scenario 21: malformed / incomplete headers → strict BAD_REQUEST.
    #[test]
    fn test_w4_x402_bad_header_rejected() {
        assert!(parse_x_payment_header("not-json-or-b64").is_err());
        assert!(parse_x_payment_header("{}").is_err());
        assert!(parse_x_payment_header(r#"{"x402Version":1}"#).is_err());
        println!("[PASS] scenario 21a: bad X-Payment parse rejected");
    }

    async fn connect_test_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        if url.is_empty() {
            return None;
        }
        sqlx::MySqlPool::connect(&url).await.ok()
    }

    /// Local mock for CSPR.cloud `/deploys/{hash}` used by verify_payment_proof.
    async fn spawn_mock_cspr_cloud(
        merchant: &str,
        amount: u64,
        ok_status: &str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use axum::{Router, routing::get, extract::Path};
        let merchant = merchant.to_string();
        let status = ok_status.to_string();
        let app = Router::new().route(
            "/deploys/{hash}",
            get(move |Path(_hash): Path<String>| {
                let merchant = merchant.clone();
                let status = status.clone();
                async move {
                    Json(json!({
                        "data": {
                            "status": status,
                            "transfers": [{
                                "amount": amount.to_string(),
                                "to": merchant
                            }]
                        }
                    }))
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock");
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        (format!("http://{}", addr), handle)
    }

    /// Wave 4 scenario 20: successful verify then replay → already spent.
    #[tokio::test]
    #[ignore]
    async fn test_w4_x402_replay_rejected() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => return,
        };
        unsafe {
            std::env::remove_var("DISABLE_X402");
        }

        let merchant = "01aabbccdd00112233445566778899aabbccddeeff00112233445566778899aa";
        let amount = 5_000_000_000u64;
        let deploy = format!("w4-deploy-replay-{}", chrono::Utc::now().timestamp_millis());
        let _ = sqlx::query("DELETE FROM spent_payments WHERE deploy_hash = ?")
            .bind(&deploy)
            .execute(&pool)
            .await;

        let (base, handle) = spawn_mock_cspr_cloud(merchant, amount, "executed").await;
        let client = CasperClient::new(base, "test-key".into(), "pkg".into());

        let valid_until = (chrono::Utc::now().timestamp() + 300) as u64;
        let token = generate_challenge_token(valid_until, amount, "https://api.can.dev");

        let header_json = json!({
            "x402Version": 1,
            "scheme": "exact",
            "network": "casper-testnet",
            "payload": {
                "txid": deploy,
                "challengeToken": token
            }
        })
        .to_string();

        let mut headers = HeaderMap::new();
        headers.insert("X-Payment", header_json.parse().unwrap());

        let first = verify_payment(&headers, &pool, &client, amount, merchant).await;
        assert!(first.is_ok(), "first payment must succeed: {:?}", first);

        let second = verify_payment(&headers, &pool, &client, amount, merchant).await;
        let err = second.expect_err("replay must fail");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(
            err.1 .0["error"]
                .as_str()
                .unwrap_or("")
                .contains("already spent"),
            "got {:?}",
            err.1 .0
        );
        println!("[PASS] scenario 20: replay of same payment proof rejected");

        let _ = sqlx::query("DELETE FROM spent_payments WHERE deploy_hash = ?")
            .bind(&deploy)
            .execute(&pool)
            .await;
        handle.abort();
    }

    /// Wave 4 scenario 21: bad chain status / amount → 402, reservation rolled back.
    #[tokio::test]
    #[ignore]
    async fn test_w4_x402_bad_proof_rolls_back_reservation() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => return,
        };
        unsafe {
            std::env::remove_var("DISABLE_X402");
        }

        let merchant = "01aabbccdd00112233445566778899aabbccddeeff00112233445566778899aa";
        let amount = 5_000_000_000u64;
        let deploy = format!("w4-deploy-bad-{}", chrono::Utc::now().timestamp_millis());
        let _ = sqlx::query("DELETE FROM spent_payments WHERE deploy_hash = ?")
            .bind(&deploy)
            .execute(&pool)
            .await;

        // Mock returns non-success status → verify_payment_proof Ok(false) → 402 + DELETE
        let (base, handle) = spawn_mock_cspr_cloud(merchant, amount, "pending").await;
        let client = CasperClient::new(base, "test-key".into(), "pkg".into());

        let valid_until = (chrono::Utc::now().timestamp() + 300) as u64;
        let token = generate_challenge_token(valid_until, amount, "https://api.can.dev");

        let header_json = json!({
            "x402Version": 1,
            "scheme": "exact",
            "network": "casper-testnet",
            "payload": {
                "txid": deploy,
                "signature": "deadbeef",
                "challengeToken": token
            }
        })
        .to_string();
        let mut headers = HeaderMap::new();
        headers.insert("X-Payment", header_json.parse().unwrap());

        let res = verify_payment(&headers, &pool, &client, amount, merchant).await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().0, StatusCode::PAYMENT_REQUIRED);

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM spent_payments WHERE deploy_hash = ?")
                .bind(&deploy)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count.0, 0, "failed verify must roll back reservation");
        println!("[PASS] scenario 21: bad proof → 402 and reservation rollback");
        handle.abort();
    }

    /// Wave 4 scenario 22: mass invalid headers — error class visible, no raw secrets.
    #[tokio::test]
    #[ignore]
    async fn test_w4_x402_mass_invalid_no_secret_leak() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => return,
        };
        unsafe {
            std::env::remove_var("DISABLE_X402");
        }
        let client = CasperClient::new("http://127.0.0.1:9".into(), "secret-key-xyz".into(), "pkg".into());

        for i in 0..20 {
            let mut headers = HeaderMap::new();
            headers.insert(
                "X-Payment",
                format!("invalid-proof-{}", i).parse().unwrap(),
            );
            let err = verify_payment(&headers, &pool, &client, 1, "merchant")
                .await
                .expect_err("must fail");
            assert_eq!(err.0, StatusCode::BAD_REQUEST);
            let msg = err.1 .0.to_string();
            assert!(!msg.contains("secret-key-xyz"), "no API key leak");
            assert!(
                msg.to_lowercase().contains("parse") || msg.to_lowercase().contains("failed"),
                "reason class present: {}",
                msg
            );
        }
        println!("[PASS] scenario 22: mass invalid X-Payment — typed errors, no secrets");
    }

    #[test]
    fn test_challenge_token_valid_and_expired() {
        let price = 100_000_000;
        let resource = "https://api.can.dev";
        
        // 1. Valid token
        let valid_until = (chrono::Utc::now().timestamp() + 10) as u64;
        let token = generate_challenge_token(valid_until, price, resource);
        assert!(verify_challenge_token(&token, price, resource).is_ok());
        
        // 2. Expired token
        let expired_until = (chrono::Utc::now().timestamp() - 10) as u64;
        let expired_token = generate_challenge_token(expired_until, price, resource);
        let err = verify_challenge_token(&expired_token, price, resource).unwrap_err();
        assert!(err.contains("expired"));
        
        // 3. Tampered token
        let tampered_token = format!("{}.invalid_sig", valid_until);
        let err2 = verify_challenge_token(&tampered_token, price, resource).unwrap_err();
        assert!(err2.contains("signature mismatch"));
    }
}
