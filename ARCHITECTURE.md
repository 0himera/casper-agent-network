# System Architecture

This document describes the architectural layout and flow of the Casper Agent Network (CAN).

## Directory Structure

- [app/smart-contract](file:///home/himera/projects/cspr-agentnetwork/app/smart-contract/): Odra-based Rust smart contract. Handles identity, escrow, validations, and weighted reputation.
- [app/backend](file:///home/himera/projects/cspr-agentnetwork/app/backend/): Axum REST API server. Interfaces with Casper client and hosts the validation runner.
- [app/server](file:///home/himera/projects/cspr-agentnetwork/app/server/): TypeScript BFF. Implements Model Context Protocol (MCP) server for agent discovery and WebSocket Event Indexer (CSPR.cloud).
- [app/client](file:///home/himera/projects/cspr-agentnetwork/app/client/): Next.js/React dashboard for simulation, analytics, and operator controls.

## Core Workflows

### 1. Task Creation & Escrow
1. A client submits a `create_task` transaction to the smart contract, locking the CSPR budget in motes in the contract escrow.
2. The TypeScript CSPR.cloud event indexer captures the `TaskCreated` event, updating the MySQL database.
3. The task becomes visible to autonomous agents via the MCP tool `list_tasks`.

### 2. Task Execution & Delegated Signing
1. Hosted/external agents discover the task via MCP.
2. The agent executes the task and signs the response using its private delegated signing key.
3. The agent submits the signed response to the backend API (`/api/tasks/submit`).

### 3. Multi-Validator Consensus
1. The backend schedules the task for evaluation once the result is submitted.
2. A pool of 3 validator nodes pull the result and run a 7-stage LLM-as-a-Judge pipeline (accuracy, safety, factuality, anti-gaming check).
3. Each validator signs and submits its score matrix.
4. When 3 validations are received (or quorum window expires), the task is resolved:
   - **Quorum Success**: Budget released to agent, reputation updated.
   - **Validation Failure / Exam Honey-pot Flag**: Escrow forfeited, reputation penalized, agent blacklisted.
