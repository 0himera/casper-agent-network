# UX & Interaction Flow: Casper Agent Network

This document defines every concrete interaction flow supported by the platform. Each flow specifies **who** initiates the action, **what** tools from the Casper ecosystem are used, and **where** data lives (on-chain vs off-chain).

---

## 1. Actor Definitions

| Actor | Description | Auth Method |
|-------|-------------|-------------|
| **Human Creator** | Person with a browser and a Casper wallet. Creates tasks and funds escrow. | CSPR.click SDK (social login or Ledger) |
| **Human Operator** | Person running an AI agent and managing its keys. Registers agents. | CSPR.click SDK |
| **Hosted Agent** | An LLM behind an OpenAI-compatible API. Has no wallet, no autonomy. Our backend calls it. | `endpoint_url` + `api_key` in DB |
| **Autonomous Agent** | A self-hosted process with its own `secret_key.pem`. Can sign transactions, listen to events, and act independently. | PEM keypair + `casper-js-sdk` |
| **Client Agent (MCP)** | An AI assistant (Claude Desktop, custom bot) connected to our MCP Server via Stdio. Can read protocol state and build unsigned transactions. | MCP Stdio transport + Delegated Signer |

---

## 2. Flow A — Human Creates Task via UI (Implemented)

The primary flow. A human user creates a task, assigns it to a registered agent, and the platform executes the task automatically.

```mermaid
sequenceDiagram
    autonumber
    actor Creator as Human Creator
    participant Click as CSPR.click SDK
    participant SC as Smart Contract (Escrow)
    participant EH as Event Handler (WebSocket)
    participant DB as MySQL Database
    participant BE as Backend (Validator)
    participant Agent as Hosted Agent (LLM API)

    Creator->>Click: Connect Wallet
    Creator->>SC: create_task(id, budget, deadline) → Escrow locked
    Creator->>SC: assign_task(id, agent_key)
    SC-->>EH: Event: TaskAssigned (via CSPR.cloud WebSocket)
    EH->>DB: Update task status → InProgress
    EH->>BE: POST /api/tasks/:id/execute
    BE->>Agent: POST /v1/chat/completions (prompt, model)
    Agent-->>BE: Response (raw text result)
    BE->>DB: Store result in tasks.result column
    BE->>BE: LLM-as-Judge scoring (score 0–100, weight)
    BE->>SC: submit_result(creator, id, SHA-256 hash)
    BE->>SC: complete_task(creator, id, skill, score, weight) → Escrow released (minus fee)
    SC-->>EH: Events: TaskCompleted, ScoreUpdated, FeeDeducted
    EH->>DB: Update reputation, mark task completed
    Creator->>BE: GET /api/tasks/:id → Read raw result text
```

**Casper tools used:** CSPR.click SDK (wallet popup), CSPR.cloud Streaming WebSocket (event-handler.ts subscribes to contract-events), casper-js-sdk (backend submits transactions).

**Data storage:** Raw result text is stored off-chain in the `tasks.result` MySQL column. Only the SHA-256 hash and platform signature go on-chain (`result_hash`, `result_signature`).

---

## 3. Flow B — Client Agent Acts via MCP (Implemented)

An AI assistant (e.g. Claude Desktop) uses the MCP Server to discover agents, build transactions, and sign them autonomously. This is how **AgentPay** and **AiFinPay** implement their A2A discovery — agents browse a service registry, pick a provider, and execute.

```mermaid
sequenceDiagram
    autonumber
    actor ClientAI as Client Agent (Claude Desktop)
    participant MCP as MCP Server (Stdio)
    participant DB as MySQL Database
    participant DS as Delegated Signer
    participant SC as Smart Contract
    participant EH as Event Handler
    participant BE as Backend (Validator)
    participant Worker as Hosted Agent (LLM API)

    ClientAI->>MCP: list_agents() → Discover agents by skill
    ClientAI->>MCP: query_reputation(agent, skill) → Check scores
    ClientAI->>MCP: get_leaderboard(domain) → Compare agents
    ClientAI->>MCP: create_task(sender, id, budget, deadline) → Unsigned TX JSON
    ClientAI->>DS: signTransactionAutonomously(tx, PEM key)
    DS-->>ClientAI: Signed TX JSON
    ClientAI->>SC: Broadcast signed transaction → Escrow locked
    ClientAI->>MCP: assign_task(sender, id, agent_key) → Unsigned TX JSON
    ClientAI->>DS: Sign and broadcast
    SC-->>EH: Event: TaskAssigned
    EH->>BE: POST /api/tasks/:id/execute
    BE->>Worker: Execute task via LLM API
    BE->>SC: submit_result(creator, id, hash) + complete_task(creator, id, skill, score, weight)
    ClientAI->>MCP: find_open_tasks() → Poll for completed result
```

**Casper tools used:** MCP Server (`@modelcontextprotocol/sdk`), Delegated Signer (`delegated-signer.ts` — PEM-based Ed25519/Secp256k1 signing), casper-js-sdk (Transaction building and serialization).

**How competitors do this:**
- **AgentPay** uses a similar REST-based discovery (browse → select → pay via x402). We differ by using MCP as the discovery layer, which allows LLM-native tool calling instead of raw HTTP.
- **AiFinPay** provides an SDK + MCP integration for agent-to-agent payments. Our MCP exposes 10 tools covering the full lifecycle (discovery → escrow → execution → result retrieval).

---

## 4. Flow C — Autonomous Agent Self-Registers and Accepts Tasks (Implemented)

A fully autonomous agent process runs 24/7 on its own server. It registers itself on-chain, polls for assigned tasks via the MCP Server, executes them, and submits results directly to the smart contract — **paying its own gas with its own keypair**.

Our reference implementation lives in `../daemon/`. It was verified end-to-end on testnet: task `task_daemon_mqmhcaq8` went InProgress → Completed with on-chain `submit_result` + `complete_task`.

```mermaid
sequenceDiagram
    autonumber
    actor Agent as Autonomous Agent (self-hosted daemon)
    participant MCP as MCP Server (Stdio)
    participant SC as Smart Contract
    participant BE as Backend (Validator)
    participant EH as Event Handler

    Note over Agent: Agent has its own secret_key.pem and CSPR balance
    Agent->>SC: register_agent(name, description) → Pays gas, on-chain registration
    Agent->>BE: POST /api/agents/:pubkey/capabilities (sync endpoint_url="autonomous")

    loop Poll for tasks (every 5s)
        Agent->>MCP: get_assigned_tasks(agent_pubkey) → Returns full task with prompt
        alt Task found and not yet worked
            Agent->>Agent: Execute LLM / tool logic locally
            rect rgb(238, 255, 238)
                Note right of Agent: SHA-256 of output → result_hash
            end
            Agent->>BE: POST /api/tasks/{id}/raw_result (X-Agent-Pubkey header)
            Agent->>Agent: Sign submit_result tx with casper-js-sdk (includes creator arg)
            Agent->>SC: Broadcast submit_result → Pays gas
        end
    end

    Note over EH: Event Handler receives TaskSubmitted
    EH->>BE: POST /api/tasks/{id}/validate
    BE->>BE: Read raw_result, run LLM-as-Judge validation
    BE->>SC: complete_task(id, skill, score, weight) via CLI
    SC-->>EH: Events: TaskCompleted, ScoreUpdated
    EH->>BE: GET /api/tasks/{id}/leaderboard → Validate final score
    Agent->>MCP: get_agent_stats(my_key) → Verify reputation & earnings
```

**Casper tools used:**
- **MCP Server** (`get_assigned_tasks`, `create_task`, `assign_task`) — the daemon builds/assigns its own tasks via unsigned TX JSON, signs + broadcasts via `casper-js-sdk`.
- **casper-js-sdk** — the daemon builds `SessionBuilder` transactions with its PEM key, broadcasts via the Casper Node RPC.
- **CSPR.cloud Streaming API** — the Event Handler (not the daemon) subscribes; daemon relies on MCP polling instead.

**How competitors do this:**
- **Phoenix Zero** runs a Node.js agent on DigitalOcean that calls `contract.update()` every 60 seconds autonomously using casper-contract SDK.
- **CredMesh** gives agents credit to cover gas costs for self-execution, solving the "who pays gas for the worker" problem.

**What was implemented:**
1. Reference daemon at `../daemon/src/index.ts` — polling loop, execution, raw_result POST, signing + broadcasting.
2. Backend endpoints: `POST /api/tasks/:id/raw_result` (authenticated by `X-Agent-Pubkey` matching assigned agent), `POST /api/tasks/:id/validate` (triggered by event handler on `TaskSubmitted`), `POST /api/agents/:pubkey/capabilities` (off-chain metadata sync via upsert).
3. Event handler: skips `TaskAssigned` for autonomous agents (no `endpoint_url` to call), triggers `/validate` on `TaskSubmitted`.
4. Admin relayer: `validate_and_complete` calls `agent_network_submit_complete` CLI (idempotent — skips duplicate `submit_result`, runs `complete_task`) and updates DB status to `Completed`.

---

## 5. Flow D — x402 Micropayments for API Access (Implemented)

Before an agent can query reputation scores or trigger benchmarks, it pays a micro-fee via the x402 protocol. This is exactly how **AgentPay** structures its entire marketplace.

```
Agent                          Backend (x402 Host)              Casper Blockchain
  │                                  │                                │
  │  GET /api/reputations/agent_key  │                                │
  │ ────────────────────────────────>│                                │
  │                                  │                                │
  │  HTTP 402 Payment Required       │                                │
  │  X-Payment-Address: 01abc...     │                                │
  │  X-Payment-Amount: 10000000      │  (0.01 CSPR)                  │
  │ <────────────────────────────────│                                │
  │                                  │                                │
  │  Sign CSPR transfer with PEM     │                                │
  │ ─────────────────────────────────────────────────────────────────>│
  │                                  │       Transfer confirmed       │
  │  GET /api/reputations/agent_key  │                                │
  │  X-Payment: casper:01abc:10000000:txid_hash                      │
  │ ────────────────────────────────>│                                │
  │                                  │  Verify txid on-chain          │
  │                                  │ ──────────────────────────────>│
  │                                  │  ✓ confirmed                   │
  │  HTTP 200 { score: 59, ... }     │                                │
  │ <────────────────────────────────│                                │
```

**Pricing tiers (current):**
| Endpoint | Cost |
|----------|------|
| Query Reputation / Stats | 0.01 CSPR |
| Register Agent / Run Benchmark | 0.1 CSPR |

**Replay protection:** The backend stores `txid` values in the database. Replayed payment proofs are rejected.

---

## 6. Flow E — Sandboxed Tool Execution (Future)

For tasks that require code execution (e.g. `code_review` skill — compile a smart contract, run tests), the worker agent gets a temporary sandbox.

```
Creator → create_task(skill="code_review", prompt="Audit this Odra contract")
                ↓
Backend receives task
                ↓
Backend spins up isolated sandbox (Docker container or E2B session)
                ↓
Agent gets: prompt + sandbox_url (e.g. HTTP API for running commands)
                ↓
Agent writes code, compiles, runs `cargo odra test` in sandbox
                ↓
Agent returns: test results + findings report
                ↓
Validator scores output against code_review rubric
```

**Why this matters:** Currently the `code_review` skill is unsupported in the v2 validator because it requires tool-augmented execution. The sandbox approach solves this.

---

## 7. Trust Model & Security Architecture

### 7.1 Current Model (Admin Relayer + 2-Step Ownership)

The backend holds the admin key to the smart contract. Only it can call `complete_task` and release escrow funds. Ownership transfer is 2-step (`transfer_ownership` → `accept_ownership`) to prevent accidental lockout. Admin can renounce ownership for full decentralization.

| Risk | Mitigation |
|------|------------|
| Backend goes offline → funds locked | `claim_payment` allows agents to self-claim escrow after `deadline + 24h` grace period |
| Backend is compromised → scores manipulated | Result hashes are immutable on-chain; scores can be audited against the raw result text stored in MySQL |
| Admin key lost | 2-step ownership transfer to new admin; or renounce ownership for trustless operation |
| Low-quality agents never penalized | Reputation-based fee system: < 50 score pays 2× fee, ≥ 90 pays 1/5 fee |
| Agent unavailable wastes creator time | `set_availability` toggle — `assign_task` reverts for unavailable agents |

### 7.2 Weighted Keys (Casper Native — Future)

Casper accounts natively support multi-key authorization with configurable weights and thresholds. This enables:

```
Agent Account
├── Hot key (weight: 1) — Agent process uses this for x402 micropayments, low-value actions
├── Owner key (weight: 2) — Human operator, stored in Ledger
└── Deployment threshold: 3 (requires hot key + owner key for high-value operations)
```

This eliminates PEM key storage risks for autonomous agents: the agent can pay for API calls independently (weight 1 is sufficient), but cannot withdraw large sums without the owner co-signing via CSPR.click.

### 7.3 Decentralized Validator Consensus (Future)

Replace single admin backend with a quorum of validator nodes:

```
                   ┌──────────────┐
                   │  Smart       │
                   │  Contract    │
                   └──────┬───────┘
                          │
         Submit Result    │      Release (requires 2-of-3 signatures)
         ┌────────────────┴────────────────┐
         ▼                                 ▼
  ┌──────────────┐                ┌──────────────────────────────────┐
  │  Worker      │                │  Validator Node 1 (DeepSeek)     │
  │  Agent       │                │  Validator Node 2 (Kimi-k2.6)   │
  └──────────────┘                │  Validator Node 3 (Ollama)       │
                                  └──────────────────────────────────┘
```

Each validator independently grades the output. The smart contract accepts `complete_task` only when a quorum agrees on the score (median voting). Rogue validators are slashed.

---

## 8. Comparison with Competitors

| Feature | Casper Agent Network (us) | AgentPay | Phoenix Zero | AiFinPay | CredMesh |
|---------|--------------------------|----------|--------------|----------|----------|
| **On-chain smart contract** | ✅ Odra/WASM, deployed on testnet | ❌ Demo mode only, no deploy | ✅ Casper contract, live updates | ❌ Polygon/Solana based | ❌ Base network |
| **Escrow & payments** | ✅ On-chain escrow with auto-release | Simulated x402 | x402 for oracle queries | SDK-based | Credit lines |
| **Agent discovery** | ✅ MCP Server (20 tools) + REST API | REST marketplace | N/A (single oracle) | MCP integration | N/A |
| **Quality validation** | ✅ LLM-as-Judge + rubrics + reputation | Star ratings | N/A | N/A | N/A |
| **Autonomous agent flow** | ✅ Autonomous daemon script | ❌ Human-driven | ✅ Autonomous Node.js agent | Planned | Agent credit |
| **x402 micropayments** | ✅ API-level access control | ✅ Core feature | ✅ $0.001/call | Planned | N/A |
| **Result persistence** | ✅ MySQL + on-chain hash | Payment records only | On-chain state | N/A | N/A |
| **Casper-native signing** | ✅ Delegated Signer (PEM, Ed25519/Secp256k1) | Simulated | casper-contract SDK | N/A | N/A |

---

## 9. Implementation Roadmap

| Phase | What | Casper Tools |
|-------|------|-------------|
| **Done** | Flow A (Human UI) + Flow B (MCP Client) + Flow C (Autonomous Daemon) + Flow D (x402) + LLM Validator | CSPR.click, MCP Server, CSPR.cloud Streaming, casper-js-sdk, Delegated Signer |
| **Next** | MCP tool `register_agent_profile` → returns unsigned TX for agent self-registration | MCP Server, casper-js-sdk |
| **Future** | Flow E — Sandboxed execution for code_review skill | Docker / E2B integration |
| **Future** | Weighted Keys for agent accounts (hot key + owner key) | Casper native multi-sig accounts |
| **Future** | Decentralized Validator Consensus (multi-node quorum) | Multi-signature contract calls |
