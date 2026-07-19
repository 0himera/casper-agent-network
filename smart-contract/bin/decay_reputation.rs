//! Decay reputation for an agent and skill.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_decay_reputation --features livenet -- \
//!   <agent_address> <skill>
//! ```

use agent_network::agent_network::AgentNetwork;
use odra::host::HostRefLoader;
use odra::prelude::Address;
use std::env as std_env;
use std::str::FromStr;

const HALF_LIFE_MS: u64 = 30 * 86_400 * 1000; // 30 days in milliseconds

fn main() {
    env_logger::init();

    let args: Vec<String> = std_env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <agent_address> <skill>", args[0]);
        std::process::exit(1);
    }

    let agent = Address::from_str(&args[1]).expect("Invalid agent address");
    let skill = args[2].clone();

    let env = odra_casper_livenet_env::env();

    let contract_hash = std_env::var("CONTRACT_HASH").unwrap_or_else(|_| {
        eprintln!("⚠️  CONTRACT_HASH env var not set. Set it to the deployed contract hash.");
        std::process::exit(1);
    });

    let address = Address::from_str(&contract_hash).expect("Invalid contract hash");

    println!("=== On-Chain Reputation Decay ===");
    println!("Contract Address: {}", contract_hash);
    println!("Agent Address:    {}", args[1]);
    println!("Skill:            {}", skill);

    env.set_gas(15_000_000_000u64);

    let contract = AgentNetwork::load(&env, address);

    // 1. Query current reputation state on-chain
    let rep_state = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.get_reputation(agent, skill.clone())
    }));

    let rep = match rep_state {
        Ok(r) => r,
        Err(_) => {
            eprintln!("❌ Failed to fetch reputation from contract.");
            std::process::exit(1);
        }
    };

    println!(
        "Current on-chain reputation: weighted_sum={}, total_weight={}, last_update={}",
        rep.weighted_sum, rep.total_weight, rep.last_update
    );

    if rep.total_weight == 0 {
        println!("Reputation has zero weight, no decay needed.");
        return;
    }

    // 2. Calculate decay ratio based on elapsed block time (using block time/current time)
    // For Casper, env.block_time() returns current block time in milliseconds
    let now_ms = env.block_time();
    println!("Current block time: {}", now_ms);

    if now_ms <= rep.last_update {
        println!("Current block time is not past last update. Skipping decay.");
        return;
    }

    let elapsed_ms = now_ms - rep.last_update;
    let elapsed_periods = elapsed_ms as f64 / HALF_LIFE_MS as f64;
    let decay_ratio = 0.5_f64.powf(elapsed_periods);

    let decayed_weighted_sum = (rep.weighted_sum as f64 * decay_ratio).round() as u64;
    let decayed_total_weight = (rep.total_weight as f64 * decay_ratio).round() as u64;

    println!(
        "Calculated decayed reputation: weighted_sum={}, total_weight={} (ratio={})",
        decayed_weighted_sum, decayed_total_weight, decay_ratio
    );

    if decayed_weighted_sum == rep.weighted_sum && decayed_total_weight == rep.total_weight {
        println!("Decayed values are identical to current values. Skipping sync.");
        return;
    }

    // Monotonic decay guard (ensure weights do not increase)
    if decayed_weighted_sum > rep.weighted_sum || decayed_total_weight > rep.total_weight {
        println!("⚠️ Guard warning: Decayed weights calculated are higher than current. Skipping to prevent contract revert.");
        return;
    }

    // 3. Sync decayed reputation on-chain
    println!("Submitting synced decayed reputation...");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut contract_mut = AgentNetwork::load(&env, address);
        contract_mut.sync_decayed_reputation(
            agent,
            skill.clone(),
            decayed_weighted_sum,
            decayed_total_weight,
        );
    }));

    match result {
        Ok(_) => {
            println!("✅ Synced decayed reputation successfully on-chain!");
        }
        Err(_) => {
            eprintln!("❌ Failed to sync decayed reputation on-chain.");
            std::process::exit(1);
        }
    }
}
