//! Update metadata on the already-deployed AgentNetwork contract.
//!
//! ## Usage
//! ```bash
//! CONTRACT_HASH=hash-f989247b6781ea47fdbdc83c831a793726b024ffe40cdcd9e473d4a2176be600 cargo run --bin agent_network_update_metadata --features livenet
//! ```

use agent_network::agent_network::AgentNetwork;
use odra::host::HostRefLoader;
use odra::prelude::Address;
use std::env as std_env;
use std::str::FromStr;

fn main() {
    env_logger::init();
    let env = odra_casper_livenet_env::env();

    let contract_hash = std_env::var("CONTRACT_HASH").unwrap_or_else(|_| {
        "hash-f989247b6781ea47fdbdc83c831a793726b024ffe40cdcd9e473d4a2176be600".to_string()
    });
    let address = Address::from_str(&contract_hash).expect("Invalid contract hash");

    let name = std_env::var("CONTRACT_NAME")
        .ok()
        .or_else(|| Some("Casper Agent Network".to_string()));
    let description = std_env::var("CONTRACT_DESCRIPTION").ok().or_else(|| {
        Some(
            "A decentralized reputation protocol and task marketplace for AI agents on the Casper Network."
                .to_string(),
        )
    });
    let icon_uri = std_env::var("CONTRACT_ICON_URI")
        .ok()
        .or_else(|| Some("https://casper-agent-network.vercel.app/can-logo.png".to_string()));
    let project_uri = std_env::var("CONTRACT_PROJECT_URI")
        .ok()
        .or_else(|| Some("https://casper-agent-network.vercel.app/".to_string()));

    println!("=== Updating Contract Metadata ===");
    println!("Contract:    {}", contract_hash);
    println!("Name:        {:?}", name);
    println!("Description: {:?}", description);
    println!("Icon URI:    {:?}", icon_uri);
    println!("Project URI: {:?}", project_uri);

    env.set_gas(10_000_000_000u64);

    let mut contract = AgentNetwork::load(&env, address);

    println!("Sending update_metadata transaction...");
    contract.update_metadata(name, description, icon_uri, project_uri);

    println!("✅ Contract metadata updated successfully on-chain!");
}
