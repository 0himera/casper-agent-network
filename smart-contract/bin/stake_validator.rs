//! Top up validator stake on the already-deployed AgentNetwork contract.
//!
//! ## Usage
//! ```bash
//! CONTRACT_HASH=hash-... ODRA_CASPER_LIVENET_SECRET_KEY_PATH=/path/to/validator.pem \
//! cargo run --bin agent_network_stake_validator --features livenet
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
        eprintln!("⚠️  CONTRACT_HASH env var not set.");
        std::process::exit(1);
    });
    let address = Address::from_str(&contract_hash).expect("Invalid contract hash");

    // Default top-up: 50 CSPR (enough to restore activity after a 35 CSPR slash from 100).
    let amount_motes = std_env::var("STAKE_AMOUNT_MOTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(50_000_000_000u64);

    println!("=== Stake Validator on Existing Contract ===");
    println!("Contract: {}", contract_hash);
    println!("Amount:   {} motes", amount_motes);

    env.set_gas(20_000_000_000u64);

    let contract = AgentNetwork::load(&env, address);

    println!("Staking additional CSPR for validator...");
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract
            .with_tokens(U512::from(amount_motes))
            .stake_validator();
    }));

    if res.is_ok() {
        println!("✅ Validator stake topped up successfully!");
    } else {
        println!("⚠️ Validator stake top-up failed.");
        std::process::exit(1);
    }
}
