//! Register the caller as a validator on the already-deployed AgentNetwork contract with 100 CSPR stake.
//!
//! ## Usage
//! ```bash
//! CONTRACT_HASH=hash-... ODRA_CASPER_LIVENET_SECRET_KEY_PATH=/path/to/key.pem \
//! cargo run --bin agent_network_register_validator --features livenet
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

    println!("=== Register Validator on Existing Contract ===");
    println!("Contract: {}", contract_hash);

    env.set_gas(20_000_000_000u64);

    let contract = AgentNetwork::load(&env, address);

    println!("Registering validator (staking 100 CSPR)...");
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract
            .with_tokens(U512::from(100_000_000_000u64))
            .register_validator();
    }));

    if res.is_ok() {
        println!("✅ Validator registered successfully!");
    } else {
        println!("⚠️ Validator registration skipped or already registered.");
    }
}
