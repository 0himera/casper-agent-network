//! Submit task result and complete the task on-chain.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_submit_complete --features livenet -- \
//!   <creator_address> <task_id> <result_hash> <skill> <score> <weight>
//! ```

use agent_network::agent_network::{AgentNetwork, AgentNetworkHostRef, Task, TaskStatus};
use odra::host::HostRefLoader;
use odra::prelude::Address;
use std::str::FromStr;
use std::env as std_env;

fn get_task_safe(contract: &AgentNetworkHostRef, creator: &Address, task_id: &str) -> Option<Option<Task>> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.get_task(*creator, task_id.to_string())
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

    let args: Vec<String> = std_env::args().collect();
    if args.len() < 7 {
        eprintln!(
            "Usage: {} <creator_address> <task_id> <result_hash> <skill> <score> <weight>",
            args[0]
        );
        std::process::exit(1);
    }

    let creator = Address::from_str(&args[1]).expect("Invalid creator address");
    let task_id = args[2].clone();
    let result_hash = args[3].clone();
    let skill = args[4].clone();
    let score: u32 = args[5].parse().expect("Invalid score: must be u32");
    let weight: u32 = args[6].parse().expect("Invalid weight: must be u32");

    let env = odra_casper_livenet_env::env();

    let contract_hash = std_env::var("CONTRACT_HASH").unwrap_or_else(|_| {
        eprintln!("⚠️  CONTRACT_HASH env var not set. Set it to the deployed contract hash.");
        eprintln!("   Example: CONTRACT_HASH=hash-42cdff... cargo run --bin agent_network_submit_complete --features livenet -- ...");
        std::process::exit(1);
    });

    let address = Address::from_str(&contract_hash).expect("Invalid contract hash");

    println!("=== On-Chain Submit & Complete Task ===");
    println!("Contract Address: {}", contract_hash);
    println!("Creator:          {}", args[1]);
    println!("Task ID:          {}", task_id);
    println!("Result Hash:      {}", result_hash);
    println!("Skill:            {}", skill);
    println!("Score:            {}", score);
    println!("Weight:           {}", weight);

    env.set_gas(15_000_000_000u64);

    let contract = AgentNetwork::load(&env, address);

    let mut step1_done = false;
    for attempt in 1..=5 {
        println!("Checking task status (Step 1, attempt {})...", attempt);
        if let Some(task_opt) = get_task_safe(&contract, &creator, &task_id) {
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
            contract_mut.submit_result(creator, task_id.clone(), result_hash.clone());
        }));

        if result.is_err() {
            println!("⚠️ Transaction call panicked. Waiting 10s...");
            std::thread::sleep(std::time::Duration::from_secs(10));
        } else {
            println!("✅ Transaction call succeeded. Waiting 3s...");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }

    if !step1_done {
        eprintln!("❌ Failed to submit result hash after retries.");
        std::process::exit(1);
    }

    let mut step2_done = false;
    for attempt in 1..=5 {
        println!("Checking task status (Step 2, attempt {})...", attempt);
        if let Some(task_opt) = get_task_safe(&contract, &creator, &task_id) {
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
            contract_mut.complete_task(creator, task_id.clone(), skill.clone(), score, weight);
        }));

        if result.is_err() {
            println!("⚠️ Transaction call panicked. Waiting 10s...");
            std::thread::sleep(std::time::Duration::from_secs(10));
        } else {
            println!("✅ Transaction call succeeded. Waiting 3s...");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }

    if !step2_done {
        eprintln!("❌ Failed to complete task after retries.");
        std::process::exit(1);
    }
}
