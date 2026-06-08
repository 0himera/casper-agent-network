//! Register a single agent on the already-deployed AgentNetwork contract.
//!
//! Uses the contract hash (not package hash) to load and interact
//! with the existing deployed contract.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_register --features livenet
//! ```

use agent_network::agent_network::AgentNetwork;
use odra::host::HostRefLoader;
use odra::prelude::Address;
use std::str::FromStr;

fn main() {
    env_logger::init();
    let env = odra_casper_livenet_env::env();

    // contract_hash of the deployed AgentNetwork (v1 of package hash 10f38...)
    // Retrieved from: https://api.testnet.cspr.cloud/contract-packages/10f38cdf.../contracts
    let contract_hash = "hash-42cdff13d532e4683911bfa634752f7da6db643b380a674248ab3cf6adf6c1b0";
    let address = Address::from_str(contract_hash).expect("Invalid contract hash");

    println!("=== Register Agent on Existing Contract ===");
    println!("Contract: {}", contract_hash);

    env.set_gas(10_000_000_000u64); // 10 CSPR for the call

    let mut contract = AgentNetwork::load(&env, address);

    println!("Registering agent...");
    contract.register_agent(
        "DeFi Arbitrage Agent".to_string(),
        "Autonomous agent that monitors DeFi yield opportunities across DEXs".to_string(),
        "https://agent-network.casper.dev/agents/defi-arb".to_string(),
    );

    println!("✅ Agent registered successfully!");
    println!("The event-handler will pick up the AgentRegistered event and add it to the DB.");
}
