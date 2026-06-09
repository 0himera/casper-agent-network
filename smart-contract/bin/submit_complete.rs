//! Submit task result and complete the task on-chain.
//!
//! Uses the contract hash to load and interact with the existing contract.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_submit_complete --features livenet <task_id> <result_hash> <skill> <score> <weight>
//! ```

use agent_network::agent_network::AgentNetwork;
use odra::host::HostRefLoader;
use odra::prelude::Address;
use std::str::FromStr;
use std::env as std_env;

fn main() {
    env_logger::init();
    
    // Read CLI arguments
    let args: Vec<String> = std_env::args().collect();
    if args.len() < 6 {
        eprintln!("Usage: {} <task_id> <result_hash> <skill> <score> <weight>", args[0]);
        std::process::exit(1);
    }
    
    let task_id = args[1].clone();
    let result_hash = args[2].clone();
    let skill = args[3].clone();
    let score: u32 = args[4].parse().expect("Invalid score: must be u32");
    let weight: u32 = args[5].parse().expect("Invalid weight: must be u32");

    let env = odra_casper_livenet_env::env();

    // Read contract hash from environment, or use default
    let contract_hash = std_env::var("CONTRACT_HASH")
        .unwrap_or_else(|_| "hash-42cdff13d532e4683911bfa634752f7da6db643b380a674248ab3cf6adf6c1b0".to_string());
    
    let address = Address::from_str(&contract_hash).expect("Invalid contract hash");

    println!("=== On-Chain Submit & Complete Task ===");
    println!("Contract Address: {}", contract_hash);
    println!("Task ID:          {}", task_id);
    println!("Result Hash:      {}", result_hash);
    println!("Skill:            {}", skill);
    println!("Score:            {}", score);
    println!("Weight:           {}", weight);

    env.set_gas(15_000_000_000u64); // 15 CSPR for the execution calls

    let mut contract = AgentNetwork::load(&env, address);

    println!("Step 1: Submitting result hash...");
    contract.submit_result(task_id.clone(), result_hash);
    println!("✅ Result submitted successfully!");

    println!("Step 2: Completing task with score & weight...");
    contract.complete_task(task_id, skill, score, weight);
    println!("✅ Task completed successfully on-chain!");
}
