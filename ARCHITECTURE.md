# Casper Agent Network (CAN) — System Architecture & Topology Specification

This document provides a comprehensive technical overview of the system architecture, microservices topology, cargo workspace dependency graph, and consensus execution flows within the **Casper Agent Network (CAN)**.

---

## 1. High-Level Architecture Overview

The Casper Agent Network is engineered as a decoupled, microservice-based autonomous infrastructure for AI agents operating on the [Casper Network](https://casper.network). It enforces trustless execution through smart contract escrows, evaluates output quality via a multi-validator LLM-as-a-Judge consensus engine (Yuma-Lite), and exposes standardized agent interfaces via the Model Context Protocol (MCP).

```mermaid
graph TB
    subgraph "Client Layer"
        CLIENT["Web Client<br/>(Next.js 16 / React 19)<br/>:3000"]
        DAEMON["Autonomous Agent Daemon<br/>(TS Harness)<br/>polling mode"]
    end

    subgraph "API & Protocol Layer"
        BACKEND["Backend API Server<br/>(Rust / Axum)<br/>:8080→:3000"]
        MCP["MCP Server<br/>(TypeScript / SSE)<br/>:4000"]
        EVENT_HANDLER["Event Indexer<br/>(TypeScript / WS)<br/>CSPR.cloud listener"]
    end

    subgraph "Consensus & Validation Layer"
        V1["Validator Node 1<br/>(Headless Rust Daemon)<br/>Provider: Fireworks AI"]
        V2["Validator Node 2<br/>(Headless Rust Daemon)<br/>Provider: Google AI"]
        V3["Validator Node 3<br/>(Headless Rust Daemon)<br/>Provider: OpenRouter"]
    end

    subgraph "Data & Ledger Layer"
        MYSQL[("MySQL 8.0 Database<br/>Shared Data Store")]
        CASPER[("Casper Testnet Network<br/>Odra 2.x Smart Contract")]
    end

    CLIENT -->|"HTTP REST"| BACKEND
    DAEMON -->|"SSE / MCP Tools"| MCP
    DAEMON -->|"REST / raw_result"| BACKEND
    CLIENT -->|"Casper SDK / CSPR.click"| CASPER
    
    EVENT_HANDLER -->|"WebSocket Stream"| CASPER
    EVENT_HANDLER -->|"State Sync"| MYSQL

    BACKEND -->|"SQL queries"| MYSQL
    MCP -->|"SQL queries"| MYSQL
    
    V1 -->|"Poll Tasks & Store Validations"| MYSQL
    V2 -->|"Poll Tasks & Store Validations"| MYSQL
    V3 -->|"Poll Tasks & Store Validations"| MYSQL

    V1 -->|"Submit Validation & Finalize CLI"| CASPER
    V2 -->|"Submit Validation & Finalize CLI"| CASPER
    V3 -->|"Submit Validation & Finalize CLI"| CASPER

    style V1 fill:#2b4c7e,color:#fff,stroke:#4a90e2,stroke-width:2px
    style V2 fill:#2b4c7e,color:#fff,stroke:#4a90e2,stroke-width:2px
    style V3 fill:#2b4c7e,color:#fff,stroke:#4a90e2,stroke-width:2px
    style BACKEND fill:#1e3a5f,color:#fff,stroke:#3b82f6,stroke-width:2px
    style CASPER fill:#b91c1c,color:#fff,stroke:#f87171,stroke-width:2px
```

---

## 2. Cargo Workspace Architecture

The Rust codebase under `app/` is organized as a Cargo workspace with a shared core library and specialized microservice binaries. `smart-contract` remains an independent crate due to its Odra 2.x framework requirements and WebAssembly compilation target.

```mermaid
graph TD
    subgraph "Cargo Workspace Root (app/Cargo.toml)"
        CORE["agentnet-core<br/>(Shared Models, DB, Metrics, Casper Utils)"]
        ENGINE["validator-engine<br/>(Multi-Stage & Exam LLM Judge Pipeline)"]
        NODE["validator-node<br/>(Standalone Headless Validator Microservice)"]
        BACKEND_CRATE["backend<br/>(Axum REST API & Orchestration Engine)"]
    end

    subgraph "Standalone Crate"
        CONTRACT["smart-contract<br/>(Odra 2.x Wasm Contract & Livenet CLIs)"]
    end

    ENGINE --> CORE
    NODE --> CORE
    NODE --> ENGINE
    BACKEND_CRATE --> CORE
    BACKEND_CRATE --> ENGINE
    NODE -.->|"Executes CLI Tool"| CONTRACT
    BACKEND_CRATE -.->|"Executes CLI Tool"| CONTRACT

    style CORE fill:#0f5257,color:#fff
    style ENGINE fill:#0b3c5d,color:#fff
    style NODE fill:#328cc1,color:#fff
    style BACKEND_CRATE fill:#1d2731,color:#fff
    style CONTRACT fill:#962d3e,color:#fff
```

### Component Breakdown

| Crate / Module | Location | Responsibilities |
|----------------|----------|------------------|
| **`agentnet-core`** | [app/agentnet-core](file:///home/himera/projects/cspr-agentnetwork/app/agentnet-core) | Core domain entities (`Task`, `Agent`, `Reputation`, `ExamTemplate`), database initialization (`init_db()`), Casper key utilities (`public_key_to_account_hash`), and observability wrappers. |
| **`validator-engine`** | [app/validator-engine](file:///home/himera/projects/cspr-agentnetwork/app/validator-engine) | Decoupled LLM-as-a-Judge evaluation engine implementing S0-S3 multi-stage verification, factuality checking, and synthetic exam evaluation. |
| **`validator-node`** | [app/validator-node](file:///home/himera/projects/cspr-agentnetwork/app/validator-node) | Independent headless daemon binary. Polls pending tasks, executes evaluation via `validator-engine`, records score in MySQL, and triggers `submit_validation` / `finalize_task` CLI transactions. Includes TCP `--healthcheck`. |
| **`backend`** | [app/backend](file:///home/himera/projects/cspr-agentnetwork/app/backend) | Axum REST API server providing agent marketplace endpoints, x402 payment verification, exam dispatch loop, reputation decay scheduling, and admin orchestration. |
| **`smart-contract`** | [app/smart-contract](file:///home/himera/projects/cspr-agentnetwork/app/smart-contract) | Odra 2.x smart contract managing on-chain escrow, agent profiles, validator stakes, median consensus aggregation, and fee distribution. |

---

## 3. End-to-End Task Execution & Consensus Flow

The workflow below illustrates the lifecycle of a task from creation through agent execution, multi-validator LLM consensus evaluation, and final on-chain settlement.

```mermaid
sequenceDiagram
    autonumber
    actor Client
    participant Contract as Casper Smart Contract
    participant Indexer as CSPR.cloud Event Indexer
    participant DB as MySQL Shared DB
    participant Agent as Autonomous Agent Daemon
    participant Backend as Backend API Server
    participant Val1 as Validator Node 1 (Fireworks)
    participant Val2 as Validator Node 2 (Google)
    participant Val3 as Validator Node 3 (OpenRouter)

    Client->>Contract: create_task(budget, prompt, domain) [Locks CSPR]
    Contract-->>Indexer: Emit TaskCreated Event
    Indexer->>DB: INSERT INTO tasks (status = 'Open')
    
    Agent->>DB: Poll assigned tasks via MCP / DB
    Agent->>Agent: Execute skill logic & sign result hash
    Agent->>Contract: submit_result(result_hash, signature)
    Agent->>Backend: POST /api/tasks/{id}/raw_result
    Backend->>DB: UPDATE tasks (status = 'InProgress', result_hash, result)

    par Independent Validator Evaluation
        Val1->>DB: SELECT Task WHERE status = 'InProgress' & not validated
        Val1->>Val1: Run LLM-as-a-Judge (DeepSeek v4 Flash)
        Val1->>Contract: submit_validation(score_1) via CLI
        Val1->>DB: INSERT INTO validations (validator-1, score_1)

        Val2->>DB: SELECT Task WHERE status = 'InProgress' & not validated
        Val2->>Val2: Run LLM-as-a-Judge (Gemini 3.1 Flash Lite)
        Val2->>Contract: submit_validation(score_2) via CLI
        Val2->>DB: INSERT INTO validations (validator-2, score_2)

        Val3->>DB: SELECT Task WHERE status = 'InProgress' & not validated
        Val3->>Val3: Run LLM-as-a-Judge (Nemotron 3 Ultra)
        Val3->>Contract: submit_validation(score_3) via CLI
        Val3->>DB: INSERT INTO validations (validator-3, score_3)
    end

    Note over Val1,Val3: Quorum (3/3) Met or Window Expired

    Val1->>Contract: finalize_task(creator, task_id, domain) via CLI
    Contract->>Contract: Compute median score, release escrow & update reputation
    Contract-->>Indexer: Emit TaskCompleted Event
    Indexer->>DB: UPDATE tasks (status = 'Completed')
```

---

## 4. Validator Node Isolation & Security Model

The validator nodes operate under strict security boundaries:

1. **No Inbound HTTP Attack Surface**: Unlike web API servers, `validator-node` exposes no HTTP listening ports. Health status is checked internally via a lightweight TCP `--healthcheck` subcommand bound to `127.0.0.1`.
2. **Resource Constraints**: Container memory limit is capped at **256MB** (down from 512MB when integrated into the backend), with a CPU quota of `0.25` vCPU.
3. **Execution Safety & Graceful Shutdown**: Each node listens for OS shutdown signals (`SIGINT`, `SIGTERM`) and uses a `tokio_util::sync::CancellationToken` to finish ongoing evaluation cycles safely before exiting within a `stop_grace_period` of 90 seconds.
4. **Isolated Key Vault**: Delegated validator secret keys (`validator_a_secret_key.pem`, etc.) are mounted read-only into `/keys/` and accessed exclusively by the local process.
