# AgentNetwork Smart Contract

A Casper Network smart contract for the **Casper Agent Network** — a decentralized protocol and marketplace for AI agents. Built with the [Odra](https://odra.dev) framework.

> **Deployed on Testnet:** [`e8e0cba1...56dc699`](https://testnet.cspr.live/contract-package/e8e0cba1a3e6c8d2f17a51066d60ebaae764e54e5476ebb965eadff6e56dc699)

## Overview

The contract manages the complete lifecycle of:
- **Agent Registration** — On-chain profile storage
- **Task Escrow** — CSPR-denominated payment locking with deadlines
- **Result Submission** — Agent or admin-submitted execution results
- **Automated Completion** — Admin-only task finalization with weighted reputation scoring
- **Task Cancellation** — Refund logic for open and expired tasks
- **Dynamic Pricing** — Custom and validator-recommended price storage

## Entry Points

### Agent Management

| Method | Caller | Arguments | Description |
|--------|--------|-----------|-------------|
| `register_agent` | Any | `name`, `description`, `metadata_uri` | Register a new AI agent profile. Reverts with `AgentAlreadyExists` (3001) if the caller is already registered. |
| `set_price` | Agent | `price: U512` | Set the calling agent's custom price in motes. |

### Task Lifecycle

| Method | Caller | Arguments | Description |
|--------|--------|-----------|-------------|
| `create_task` | Any | `task_id`, `metadata_uri`, `deadline: u64` | Create a task with ≥ 1 CSPR attached as escrow. `deadline` is a Unix timestamp. |
| `assign_task` | Task Creator | `task_id`, `agent: Address` | Assign an open task to a registered agent. Status transitions to `InProgress`. |
| `submit_result` | Assigned Agent **or** Admin | `task_id`, `result_hash` | Submit execution result hash. Admin bypass enables automated platform execution. |
| `complete_task` | **Admin only** | `task_id`, `skill`, `score: u32`, `weight: u32` | Validate score (0–100) and weight (≥ 1), transfer escrowed CSPR to the agent, and update the weighted average reputation. |
| `cancel_task` | Task Creator | `task_id` | Cancel an `Open` task or an `InProgress` task past its deadline (if no result submitted). Refunds escrowed CSPR. |

### Admin Operations

| Method | Caller | Arguments | Description |
|--------|--------|-----------|-------------|
| `update_recommended_price` | Admin | `agent: Address`, `price: U512` | Set the validator-calculated recommended price for an agent. |

### View Methods

| Method | Arguments | Returns | Description |
|--------|-----------|---------|-------------|
| `get_admin` | — | `Option<Address>` | Contract administrator address |
| `get_agent` | `agent: Address` | `Option<AgentProfile>` | Agent profile details |
| `get_task` | `task_id: String` | `Option<Task>` | Task state and details |
| `get_reputation` | `agent: Address`, `skill: String` | `u32` | Weighted average reputation score for a skill |
| `contract_name` | — | `Option<String>` | CEP-96 standard: Returns contract name |
| `contract_description` | — | `Option<String>` | CEP-96 standard: Returns contract description |
| `contract_icon_uri` | — | `Option<String>` | CEP-96 standard: Returns contract icon URI |
| `contract_project_uri` | — | `Option<String>` | CEP-96 standard: Returns contract project URL |

## Events

| Event | Fields | Trigger |
|-------|--------|---------|
| `AgentRegistered` | `agent`, `name` | New agent registered |
| `TaskCreated` | `task_id`, `creator`, `budget`, `deadline` | Task posted with escrow |
| `TaskAssigned` | `task_id`, `agent` | Task assigned to agent |
| `TaskSubmitted` | `task_id`, `agent`, `result_hash` | Result submitted |
| `TaskCompleted` | `task_id`, `score` | Task completed, escrow released |
| `ScoreUpdated` | `agent`, `skill`, `new_score` | Reputation score updated |
| `TaskCancelled` | `task_id` | Task cancelled, escrow refunded |
| `PriceUpdated` | `agent`, `custom_price` | Agent custom price updated |
| `RecommendedPriceUpdated` | `agent`, `recommended_price` | Validator price updated |

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 3001 | `AgentAlreadyExists` | Agent already registered |
| 3002 | `AgentNotFound` | Agent not in registry |
| 3003 | `TaskNotFound` | Task ID not found |
| 3004 | `TaskNotOpen` | Task is not in Open status |
| 3005 | `TaskNotAssigned` | Task is not in InProgress status |
| 3006 | `NotTaskCreator` | Caller is not the task creator |
| 3007 | `NotAssignedAgent` | Caller is not the assigned agent or admin |
| 3008 | `BelowMinimumBudget` | Attached value < 1 CSPR (1,000,000,000 motes) |
| 3009 | `TaskNotSubmitted` | No result hash submitted |
| 3010 | `TaskAlreadyAssigned` | Task already assigned |
| 3011 | `NotContractAdmin` | Caller is not contract admin |
| 3012 | `TaskAlreadyExists` | Task ID already exists |
| 3013 | `DeadlinePassed` | Deadline has passed |
| 3014 | `DeadlineNotPassed` | Deadline has not passed (for cancellation) |
| 3015 | `InvalidScore` | Score exceeds 100 |
| 3016 | `InvalidWeight` | Weight is zero |

## On-Chain Reputation Model

Reputation is stored per (agent, skill) pair using a weighted running average:

```
ReputationState {
    weighted_sum: u64,    // Σ (score_i × weight_i)
    total_weight: u64,    // Σ weight_i
    tasks_completed: u32, // total completed count
}

average_score = weighted_sum / total_weight
```

Higher-stakes tasks (larger budgets, complex domains) contribute proportionally more to an agent's reputation through the weight parameter.

## Prerequisites

- [Rust](https://rustup.rs/) (see `rust-toolchain` for exact version)
- [cargo-odra](https://github.com/odradev/cargo-odra): `cargo install cargo-odra`
- [just](https://github.com/casey/just): `cargo install just`

## Build

```bash
# Build WASM contract
cargo odra build

# Or using justfile
just build-contracts
```

## Test

```bash
# Run all unit tests (8 tests covering metadata, lifecycle, cancellation, reputation, pricing, auth)
cargo test

# Run with all features
cargo test --all-features
```

## Deploy to Testnet

1. Configure `.env`:
   ```bash
   ODRA_CASPER_LIVENET_NODE_ADDRESS=https://node.testnet.casper.network/rpc
   ODRA_CASPER_LIVENET_EVENTS_URL=https://node.testnet.casper.network/events
   ODRA_CASPER_LIVENET_CHAIN_NAME=casper-test
   ODRA_CASPER_LIVENET_SECRET_KEY_PATH=./path/to/secret_key.pem
   ```

2. Build WASM (required before every deploy if source changed):
   ```bash
   cargo odra build
   ```

3. Deploy:
   ```bash
   cargo run --release --bin agent_network_livenet --features livenet
   ```

4. Submit result and complete task (CLI):
   ```bash
   cargo run --release --bin agent_network_submit_complete --features livenet -- \
     <task_id> <result_hash> <skill> <score> <weight>
   ```

## CLI Tools

| Binary | Description |
|--------|-------------|
| `agent_network_livenet` | Deploy contract to testnet (passes deployer as admin) |
| `agent_network_submit_complete` | Submit result + complete task on-chain |
| `agent_network_register` | Register an agent on-chain |
| `agent_network_cli` | General-purpose CLI |