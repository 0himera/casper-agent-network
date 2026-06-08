//! Livenet deployment script for the AgentNetwork contract.
//!
//! This binary deploys the AgentNetwork contract to the Casper Testnet
//! and registers a demo agent for testing.
//!
//! ## Prerequisites
//! 1. Copy `.env.example` to `.env` and configure:
//!    - `ODRA_CASPER_LIVENET_NODE_ADDRESS` — Testnet node RPC URL
//!    - `ODRA_CASPER_LIVENET_CHAIN_NAME` — `casper-test`
//!    - `ODRA_CASPER_LIVENET_SECRET_KEY_PATH` — Path to your secret key
//! 2. Ensure the account has at least 500 CSPR for deployment gas.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_livenet --features livenet
//! ```

use tips_demo::agent_network::{AgentNetwork, AgentNetworkHostRef};
use odra::host::{Deployer, HostRef, NoArgs};

fn main() {
    // Initialize the livenet environment — reads config from .env
    let env = odra_casper_livenet_env::env();

    println!("=== AgentNetwork Testnet Deployment ===\n");

    // Deploy the contract (or load if already deployed)
    println!("Step 1: Deploying AgentNetwork contract...");
    let mut contract = AgentNetwork::deploy(&env, NoArgs);
    println!("✅ Contract deployed successfully!");
    println!("   Contract address: {:?}", contract.address());

    // Register a demo agent
    println!("\nStep 2: Registering demo agent...");
    contract.register_agent(
        "DeFi Arbitrage Agent".to_string(),
        "Autonomous agent that monitors DeFi yield opportunities across DEXs".to_string(),
        "https://agent-network.casper.dev/agents/defi-arb".to_string(),
    );
    println!("✅ Demo agent registered!");

    // Verify
    let caller = env.get_account(0);
    if let Some(profile) = contract.get_agent(caller) {
        println!("\n=== Verification ===");
        println!("  Agent name: {}", profile.name);
        println!("  Description: {}", profile.description);
        println!("  Active jobs: {}", profile.active_jobs);
        println!("  Custom price: {}", profile.custom_price);
        println!("  Recommended price: {}", profile.recommended_price);
    }

    println!("\n🎉 Deployment complete!");
    println!("Copy the contract package hash above and set it as CONTRACT_PACKAGE_HASH in:");
    println!("  • app/server/.env  (for Event Handler)");
    println!("  • app/backend/.env (for Rust Backend)");
}
