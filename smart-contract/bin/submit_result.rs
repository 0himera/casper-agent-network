//! Submit task execution result hash on-chain.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_submit_result --features livenet -- \
//!   <creator_address> <task_id> <result_hash>
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

fn parse_address(input: &str) -> Address {
    let clean = input.trim();
    if let Ok(addr) = Address::from_str(clean) {
        return addr;
    }
    let formatted = format!("account-hash-{}", clean);
    if let Ok(addr) = Address::from_str(&formatted) {
        return addr;
    }
    panic!("Invalid address format: {}", clean);
}

fn parse_contract_address(input: &str) -> Address {
    let clean = input.trim();
    if let Ok(addr) = Address::from_str(clean) {
        return addr;
    }
    let formatted = format!("hash-{}", clean);
    if let Ok(addr) = Address::from_str(&formatted) {
        return addr;
    }
    panic!("Invalid contract hash: {}", clean);
}

fn main() {
    env_logger::init();

    let args: Vec<String> = std_env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <creator_address> <task_id> <result_hash>",
            args[0]
        );
        std::process::exit(1);
    }

    let creator = parse_address(&args[1]);
    let task_id = args[2].clone();
    let result_hash = args[3].clone();

    let env = odra_casper_livenet_env::env();

    let contract_hash = std_env::var("CONTRACT_HASH").unwrap_or_else(|_| {
        eprintln!("⚠️  CONTRACT_HASH env var not set. Set it to the deployed contract hash.");
        std::process::exit(1);
    });

    let address = parse_contract_address(&contract_hash);

    println!("=== On-Chain Submit Task Result ===");
    println!("Contract Address: {}", contract_hash);
    println!("Creator:          {}", args[1]);
    println!("Task ID:          {}", task_id);
    println!("Result Hash:      {}", result_hash);

    env.set_gas(15_000_000_000u64);

    let contract = AgentNetwork::load(&env, address);

    let mut step_done = false;
    for attempt in 1..=5 {
        println!("Checking task status (attempt {})...", attempt);
        if let Some(task_opt) = get_task_safe(&contract, &creator, &task_id) {
            if let Some(t) = task_opt {
                if matches!(t.status, TaskStatus::Completed) {
                    println!("Task is already Completed on-chain. Skipping.");
                    step_done = true;
                    break;
                }
                if t.result_hash == result_hash {
                    println!("✅ Result hash is already submitted!");
                    step_done = true;
                    break;
                }
            } else {
                // Dictionary reads can lag just after create/assign on livenet.
                eprintln!(
                    "⚠️ Task not found on-chain yet (attempt {}). Waiting 10s...",
                    attempt
                );
                std::thread::sleep(std::time::Duration::from_secs(10));
                continue;
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
            step_done = true;
            break;
        }
    }

    if !step_done {
        eprintln!("❌ Failed to submit result hash after retries.");
        std::process::exit(1);
    }
}
