//! Casper blockchain integration module.
//!
//! Provides an HTTP client to interact with the Casper Testnet via
//! CSPR.cloud REST API. This module handles:
//! - Reading agent profiles on-chain
//! - Querying reputation scores
//! - Contract state lookups
//!
//! Actual write transactions (register_agent, complete_task, set_price, etc.)
//! are performed by the smart contract CLI (`cargo run --bin agent_network_cli`)
//! or directly from the frontend via CSPR.click wallet integration.

use serde::{Deserialize, Serialize};
use std::env;

#[allow(dead_code)]
/// Client for interacting with the Casper network via CSPR.cloud API.
#[derive(Clone, Debug)]
pub struct CasperClient {
    /// CSPR.cloud REST API base URL
    api_url: String,
    /// CSPR.cloud access key for authentication
    access_key: String,
    /// Contract package hash (hex, no prefix)
    contract_package_hash: String,
    /// HTTP client
    client: reqwest::Client,
}

/// Represents an on-chain deploy result from CSPR.cloud
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DeployResult {
    pub deploy_hash: String,
    pub status: String,
}

/// Account info from CSPR.cloud
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AccountInfo {
    pub public_key: String,
    pub account_hash: String,
    #[serde(default)]
    pub balance: String,
}

/// CSPR.cloud API response wrapper
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiResponse<T> {
    data: T,
}

/// Contract event from CSPR.cloud
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ContractEvent {
    pub name: String,
    pub data: serde_json::Value,
    pub deploy_hash: String,
    pub timestamp: String,
}

#[allow(dead_code)]
impl CasperClient {
    /// Create a new CasperClient from environment variables.
    ///
    /// Required env vars:
    /// - `CSPR_CLOUD_URL` — CSPR.cloud REST API URL
    /// - `CSPR_CLOUD_ACCESS_KEY` — API access key
    /// - `CONTRACT_PACKAGE_HASH` — deployed contract package hash
    pub fn from_env() -> Result<Self, String> {
        let api_url = env::var("CSPR_CLOUD_URL")
            .unwrap_or_else(|_| "https://api.testnet.cspr.cloud".to_string());
        let access_key = env::var("CSPR_CLOUD_ACCESS_KEY")
            .map_err(|_| "CSPR_CLOUD_ACCESS_KEY not set".to_string())?;
        let contract_package_hash = env::var("CONTRACT_PACKAGE_HASH").unwrap_or_default();

        Ok(Self {
            api_url,
            access_key,
            contract_package_hash,
            client: reqwest::Client::new(),
        })
    }

    /// Create a CasperClient with explicit configuration.
    pub fn new(api_url: String, access_key: String, contract_package_hash: String) -> Self {
        Self {
            api_url,
            access_key,
            contract_package_hash,
            client: reqwest::Client::new(),
        }
    }

    /// Look up an account's balance and info by public key.
    pub async fn get_account(&self, public_key: &str) -> Result<AccountInfo, String> {
        let url = format!("{}/accounts/{}", self.api_url, public_key);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.access_key)
            .send()
            .await
            .map_err(|e| format!("CSPR.cloud request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "CSPR.cloud API error: {} - {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }

        let body: ApiResponse<AccountInfo> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse account info: {}", e))?;

        Ok(body.data)
    }

    /// Get recent contract events for our contract package.
    pub async fn get_contract_events(
        &self,
        limit: u32,
        page: u32,
    ) -> Result<Vec<ContractEvent>, String> {
        if self.contract_package_hash.is_empty() {
            return Err("CONTRACT_PACKAGE_HASH not configured".to_string());
        }

        let url = format!(
            "{}/contract-events?contract_package_hash={}&page={}&limit={}",
            self.api_url, self.contract_package_hash, page, limit
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.access_key)
            .send()
            .await
            .map_err(|e| format!("CSPR.cloud request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "CSPR.cloud API error: {} - {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }

        let body: ApiResponse<Vec<ContractEvent>> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse events: {}", e))?;

        Ok(body.data)
    }

    /// Check if the contract is deployed (returns true if contract_package_hash is set).
    pub fn is_configured(&self) -> bool {
        !self.contract_package_hash.is_empty()
    }

    /// Get the contract package hash.
    pub fn contract_hash(&self) -> &str {
        &self.contract_package_hash
    }

    /// Verifies that a deploy hash represents a valid payment of at least `expected_amount_motes`
    /// to the specified target public key or account hash.
    pub async fn verify_payment_proof(
        &self,
        deploy_hash: &str,
        expected_amount_motes: u64,
        merchant_pubkey: &str,
    ) -> Result<bool, String> {
        let url = format!("{}/deploys/{}", self.api_url, deploy_hash);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.access_key)
            .send()
            .await
            .map_err(|e| format!("CSPR.cloud request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "CSPR.cloud API error: {} - {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            ));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse deploy details: {}", e))?;

        let status = body
            .get("data")
            .and_then(|d| d.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or_default();

        if status != "executed" && status != "success" {
            return Ok(false);
        }

        let transfers = body
            .get("data")
            .and_then(|d| d.get("transfers"))
            .and_then(|t| t.as_array());

        if let Some(transfers_list) = transfers {
            for transfer in transfers_list {
                let amount_str = transfer
                    .get("amount")
                    .and_then(|a| a.as_str())
                    .unwrap_or("0");
                let to = transfer
                    .get("to")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();

                let amount: u64 = amount_str.parse().unwrap_or(0);

                if amount >= expected_amount_motes
                    && (to.to_lowercase() == merchant_pubkey.to_lowercase()
                        || merchant_pubkey.to_lowercase().contains(&to.to_lowercase()))
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = CasperClient::new(
            "https://api.testnet.cspr.cloud".to_string(),
            "test-key".to_string(),
            "abc123".to_string(),
        );
        assert!(client.is_configured());
        assert_eq!(client.contract_hash(), "abc123");
    }

    #[test]
    fn test_client_not_configured() {
        let client = CasperClient::new(
            "https://api.testnet.cspr.cloud".to_string(),
            "test-key".to_string(),
            "".to_string(),
        );
        assert!(!client.is_configured());
    }
}
