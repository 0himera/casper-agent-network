//! Submit task result and complete the task on-chain.
//!
//! Uses the contract hash to load and interact with the existing contract.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_submit_complete --features livenet <task_id> <result_hash> <skill> <score> <weight>
//! ```

use agent_network::agent_network::{AgentNetwork, AgentNetworkHostRef, Task, TaskStatus};
use odra::host::HostRefLoader;
use odra::prelude::Address;
use std::str::FromStr;
use std::env as std_env;

fn get_task_safe(contract: &AgentNetworkHostRef, task_id: &str) -> Option<Option<Task>> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.get_task(task_id.to_string())
    }));
    match result {
        Ok(task) => Some(task),
        Err(_) => {
            eprintln!("⚠️ Failed to query task state from contract.");
            None
        }
    }
}

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

    let contract = AgentNetwork::load(&env, address);

    // Step 1: Submit result hash if not already submitted
    let mut step1_done = false;
    for attempt in 1..=5 {
        println!("Checking task status (Step 1, attempt {})...", attempt);
        if let Some(task_opt) = get_task_safe(&contract, &task_id) {
            if let Some(t) = task_opt {
                if matches!(t.status, TaskStatus::Completed) {
                    println!("Task is already Completed on-chain. Skipping Step 1.");
                    step1_done = true;
                    break;
                }
                if t.result_hash == result_hash {
                    println!("✅ Result hash is already submitted!");
                    step1_done = true;
                    break;
                }
            } else {
                eprintln!("❌ Task not found on-chain!");
                std::process::exit(1);
            }
        }

        println!("Submitting result hash...");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut contract_mut = AgentNetwork::load(&env, address);
            contract_mut.submit_result(task_id.clone(), result_hash.clone());
        }));

        if result.is_err() {
            println!("⚠️ Transaction call panicked. Waiting 10 seconds to check if transaction was processed...");
            std::thread::sleep(std::time::Duration::from_secs(10));
        } else {
            println!("✅ Transaction call succeeded. Waiting 3 seconds to let it settle...");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }

    if !step1_done {
        eprintln!("❌ Failed to submit result hash after retries.");
        std::process::exit(1);
    }

    // Step 2: Complete task if not already completed
    let mut step2_done = false;
    for attempt in 1..=5 {
        println!("Checking task status (Step 2, attempt {})...", attempt);
        if let Some(task_opt) = get_task_safe(&contract, &task_id) {
            if let Some(t) = task_opt {
                if matches!(t.status, TaskStatus::Completed) {
                    println!("✅ Task completed successfully on-chain!");
                    step2_done = true;
                    break;
                }
            } else {
                eprintln!("❌ Task not found on-chain!");
                std::process::exit(1);
            }
        }

        println!("Completing task with score & weight...");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut contract_mut = AgentNetwork::load(&env, address);
            contract_mut.complete_task(task_id.clone(), skill.clone(), score, weight);
        }));

        if result.is_err() {
            println!("⚠️ Transaction call panicked. Waiting 10 seconds to check if transaction was processed...");
            std::thread::sleep(std::time::Duration::from_secs(10));
        } else {
            println!("✅ Transaction call succeeded. Waiting 3 seconds to let it settle...");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }

    if !step2_done {
        eprintln!("❌ Failed to complete task after retries.");
        std::process::exit(1);
    }
}

