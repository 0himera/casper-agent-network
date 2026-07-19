//! Finalize task and update agent reputations on-chain.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_finalize_task --features livenet -- \
//!   <creator_address> <task_id> <skill>
//! ```

use agent_network::agent_network::{AgentNetwork, AgentNetworkHostRef, Task, TaskStatus};
use odra::host::HostRefLoader;
use odra::prelude::Address;
use std::env as std_env;
use std::str::FromStr;

fn get_task_safe(
    contract: &AgentNetworkHostRef,
    creator: &Address,
    task_id: &str,
) -> Option<Option<Task>> {
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
    if args.len() < 4 {
        eprintln!("Usage: {} <creator_address> <task_id> <skill>", args[0]);
        std::process::exit(1);
    }

    let creator = Address::from_str(&args[1]).expect("Invalid creator address");
    let task_id = args[2].clone();
    let skill = args[3].clone();

    let env = odra_casper_livenet_env::env();

    let contract_hash = std_env::var("CONTRACT_HASH").unwrap_or_else(|_| {
        eprintln!("⚠️  CONTRACT_HASH env var not set. Set it to the deployed contract hash.");
        std::process::exit(1);
    });

    let address = Address::from_str(&contract_hash).expect("Invalid contract hash");

    println!("=== On-Chain Finalize Task ===");
    println!("Contract Address: {}", contract_hash);
    println!("Creator:          {}", args[1]);
    println!("Task ID:          {}", task_id);
    println!("Skill:            {}", skill);

    env.set_gas(15_000_000_000u64);

    let contract = AgentNetwork::load(&env, address);

    let mut step_done = false;
    for attempt in 1..=5 {
        println!("Checking task status (attempt {})...", attempt);
        if let Some(task_opt) = get_task_safe(&contract, &creator, &task_id) {
            if let Some(t) = task_opt {
                if matches!(t.status, TaskStatus::Completed) {
                    println!("✅ Task is already Completed on-chain!");
                    step_done = true;
                    break;
                }
            } else {
                eprintln!("❌ Task not found on-chain!");
                std::process::exit(1);
            }
        }

        println!("Finalizing task...");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut contract_mut = AgentNetwork::load(&env, address);
            contract_mut.finalize_task(creator, task_id.clone(), skill.clone());
        }));

        if result.is_err() {
            println!("⚠️ Transaction call panicked. Waiting 10s...");
            std::thread::sleep(std::time::Duration::from_secs(10));
        } else {
            println!("✅ Transaction call succeeded. Waiting 3s...");
            std::thread::sleep(std::time::Duration::from_secs(3));
            step_done = true;
            break;
        }
    }

    if !step_done {
        eprintln!("❌ Failed to finalize task after retries.");
        std::process::exit(1);
    }
}
