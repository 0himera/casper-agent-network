use axum::{
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use crate::casper::contract::CasperClient;
use crate::db::DbPool;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct XPaymentHeader {
    #[serde(rename = "x402Version")]
    pub x402_version: u32,
    pub scheme: String,
    pub network: String,
    pub payload: XPaymentPayload,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct XPaymentPayload {
    #[serde(rename = "paymentType")]
    pub payment_type: String,
    pub txid: String,
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
    // 1. Get X-Payment header
    let x_payment_val = match headers.get("X-Payment") {
        Some(val) => val,
        None => return Err(make_402_challenge(expected_amount_motes, merchant_pubkey)),
    };

    let x_payment_str = match x_payment_val.to_str() {
        Ok(s) => s,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": "Invalid X-Payment header format" })))),
    };

    // 2. Decode base64 header
    let decoded_bytes = match hex::decode(x_payment_str)
        .or_else(|_| base64_decode(x_payment_str)) {
            Ok(b) => b,
            Err(_) => return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": "Failed to decode X-Payment header" })))),
    };

    let x_payment: XPaymentHeader = match serde_json::from_slice(&decoded_bytes) {
        Ok(xp) => xp,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": "Invalid X-Payment JSON structure" })))),
    };

    let deploy_hash = &x_payment.payload.txid;

    // 3. Prevent replay attacks - check if deploy has already been spent
    let already_spent: Option<(String,)> = sqlx::query_as(
        "SELECT deploy_hash FROM spent_payments WHERE deploy_hash = ?"
    )
    .bind(deploy_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if already_spent.is_some() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({ "error": "Payment proof already spent" }))));
    }

    // 4. Verify on-chain payment proof
    let is_verified = casper_client
        .verify_payment_proof(deploy_hash, expected_amount_motes, merchant_pubkey)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))))?;

    if !is_verified {
        return Err(make_402_challenge(expected_amount_motes, merchant_pubkey));
    }

    // 5. Mark deploy hash as spent
    sqlx::query("INSERT INTO spent_payments (deploy_hash) VALUES (?)")
        .bind(deploy_hash)
        .execute(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    Ok(())
}

fn make_402_challenge(
    price_motes: u64,
    merchant_pubkey: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::PAYMENT_REQUIRED,
        Json(json!({
            "x402Version": 1,
            "scheme": "exact",
            "network": "casper",
            "paymentRequirements": {
                "price_motes": price_motes.to_string(),
                "payTo": merchant_pubkey
            }
        })),
    )
}

fn base64_decode(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.decode(input.trim())
}
