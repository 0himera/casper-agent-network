//! Submit task result and complete the task on-chain.
//!
//! ## Usage
//! ```bash
//! cargo run --bin agent_network_submit_complete --features livenet -- \
//!   <creator_address> <task_id> <result_hash> <skill> <score>
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
    if args.len() < 6 {
        eprintln!(
            "Usage: {} <creator_address> <task_id> <result_hash> <skill> <score>",
            args[0]
        );
        std::process::exit(1);
    }

    let creator = parse_address(&args[1]);
    let task_id = args[2].clone();
    let result_hash = args[3].clone();
    let skill = args[4].clone();
    let score: u32 = args[5].parse().expect("Invalid score: must be u32");

    let env = odra_casper_livenet_env::env();

    let contract_hash = std_env::var("CONTRACT_HASH").unwrap_or_else(|_| {
        eprintln!("⚠️  CONTRACT_HASH env var not set. Set it to the deployed contract hash.");
        eprintln!("   Example: CONTRACT_HASH=hash-42cdff... cargo run --bin agent_network_submit_complete --features livenet -- ...");
        std::process::exit(1);
    });

    let address = parse_contract_address(&contract_hash);

    println!("=== On-Chain Submit & Complete Task ===");
    println!("Contract Address: {}", contract_hash);
    println!("Creator:          {}", args[1]);
    println!("Task ID:          {}", task_id);
    println!("Result Hash:      {}", result_hash);
    println!("Skill:            {}", skill);
    println!("Score:            {}", score);

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
            }
        }

        println!("Submitting result hash...");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut contract_mut = AgentNetwork::load(&env, address);
            if let Some(Some(t)) = get_task_safe(&contract_mut, &creator, &task_id) {
                if let Some(agent_addr) = t.assigned_agent {
                    if let Some(agent_profile) = contract_mut.get_agent(agent_addr) {
                        let caller_addr = env.caller();
                        if agent_addr != caller_addr && agent_profile.delegated_signer != Some(caller_addr) {
                            println!("Ensuring delegated signer is configured on-chain for hosted agent...");
                            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                contract_mut.admin_set_delegated_signer(agent_addr, Some(caller_addr));
                            }));
                        }
                    }
                }
            }
            contract_mut.submit_result(creator, task_id.clone(), result_hash.clone());
        }));

        if result.is_err() {
            println!("⚠️ Transaction call returned error or already submitted. Proceeding to validation...");
            step1_done = true;
            break;
        } else {
            println!("✅ Result hash submitted successfully! Waiting 3s...");
            std::thread::sleep(std::time::Duration::from_secs(3));
            step1_done = true;
            break;
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
            }
        }

        println!("Submitting validation score...");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut contract_mut = AgentNetwork::load(&env, address);
            contract_mut.submit_validation(creator, task_id.clone(), score);
        }));

        if result.is_err() {
            println!("⚠️ Validation submission returned error/already submitted. Proceeding...");
        } else {
            println!("✅ Validation score submitted successfully! Waiting 3s...");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }

        println!("Finalizing task...");
        let finalize_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut contract_mut = AgentNetwork::load(&env, address);
            contract_mut.finalize_task(creator, task_id.clone(), skill.clone());
        }));

        if finalize_res.is_err() {
            println!("⚠️ Finalize call returned error or failed.");
        } else {
            println!("🎉 Task finalization call succeeded! Waiting 3s...");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }

        if let Some(task_opt) = get_task_safe(&contract, &creator, &task_id) {
            if let Some(t) = task_opt {
                if matches!(t.status, TaskStatus::Completed) {
                    println!("✅ Task is confirmed Completed on-chain!");
                    step2_done = true;
                    break;
                } else {
                    eprintln!("⚠️ Task status on-chain is not Completed: {:?}.", t.status);
                }
            }
        } else if finalize_res.is_ok() {
            println!("🎉 Finalize call completed successfully!");
            step2_done = true;
            break;
        }
    }

    if !step2_done {
        eprintln!("❌ Failed to complete task on-chain after retries.");
        std::process::exit(1);
    }
}
