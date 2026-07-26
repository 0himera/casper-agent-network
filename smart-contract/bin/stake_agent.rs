//! Stake as an agent on the already-deployed AgentNetwork contract.
//!
//! ## Usage
//! ```bash
//! CONTRACT_HASH=hash-... cargo run --bin agent_network_stake_agent --features livenet
//! ```

use agent_network::agent_network::AgentNetwork;
use odra::casper_types::U512;
use odra::host::{HostRef, HostRefLoader};
use odra::prelude::Address;
use std::env as std_env;
use std::str::FromStr;

fn main() {
    env_logger::init();
    let env = odra_casper_livenet_env::env();

    let contract_hash = std_env::var("CONTRACT_HASH").unwrap_or_else(|_| {
        eprintln!("⚠️  CONTRACT_HASH env var not set. Set it to the deployed contract hash.");
        std::process::exit(1);
    });
    let address = Address::from_str(&contract_hash).expect("Invalid contract hash");

    println!("=== Stake Agent on Existing Contract ===");
    println!("Contract: {}", contract_hash);

    env.set_gas(15_000_000_000u64);

    let mut contract = AgentNetwork::load(&env, address);

    println!("Staking 100 CSPR for agent...");
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract
            .with_tokens(U512::from(100_000_000_000u64))
            .stake();
    }));

    if res.is_ok() {
        println!("✅ Agent staked successfully!");
    } else {
        println!("⚠️ Agent staking failed or already staked.");
    }
}
