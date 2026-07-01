//! Register a single agent on the already-deployed AgentNetwork contract.
//!
//! ## Usage
//! ```bash
//! CONTRACT_HASH=hash-... cargo run --bin agent_network_register --features livenet -- \
//!   <name> <description> <metadata_uri>
//! ```

use agent_network::agent_network::AgentNetwork;
use odra::host::HostRefLoader;
use odra::prelude::Address;
use std::str::FromStr;
use std::env as std_env;

fn main() {
    env_logger::init();
    let env = odra_casper_livenet_env::env();

    let contract_hash = std_env::var("CONTRACT_HASH").unwrap_or_else(|_| {
        eprintln!("⚠️  CONTRACT_HASH env var not set. Set it to the deployed contract hash.");
        eprintln!("   Example: CONTRACT_HASH=hash-42cdff... cargo run --bin agent_network_register --features livenet");
        std::process::exit(1);
    });
    let address = Address::from_str(&contract_hash).expect("Invalid contract hash");

    let args: Vec<String> = std_env::args().collect();
    let (name, description, metadata_uri) = if args.len() >= 4 {
        (args[1].clone(), args[2].clone(), args[3].clone())
    } else {
        (
            "DeFi Arbitrage Agent".to_string(),
            "Autonomous agent that monitors DeFi yield opportunities across DEXs".to_string(),
            "https://agent-network.casper.dev/agents/defi-arb".to_string(),
        )
    };

    println!("=== Register Agent on Existing Contract ===");
    println!("Contract: {}", contract_hash);
    println!("Name:      {}", name);

    env.set_gas(10_000_000_000u64);

    let mut contract = AgentNetwork::load(&env, address);

    println!("Registering agent...");
    contract.register_agent(name, description, metadata_uri);

    println!("✅ Agent registered successfully!");
    println!("The event-handler will pick up the AgentRegistered event and add it to the DB.");
}
