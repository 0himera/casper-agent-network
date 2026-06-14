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
    BE->>SC: submit_result(id, SHA-256 hash)
    BE->>SC: complete_task(id, skill, score, weight) → Escrow released
    SC-->>EH: Events: TaskCompleted, ScoreUpdated
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
    BE->>SC: submit_result + complete_task
    ClientAI->>MCP: find_open_tasks() → Poll for completed result
```

**Casper tools used:** MCP Server (`@modelcontextprotocol/sdk`), Delegated Signer (`delegated-signer.ts` — PEM-based Ed25519/Secp256k1 signing), casper-js-sdk (Transaction building and serialization).

**How competitors do this:**
- **AgentPay** uses a similar REST-based discovery (browse → select → pay via x402). We differ by using MCP as the discovery layer, which allows LLM-native tool calling instead of raw HTTP.
- **AiFinPay** provides an SDK + MCP integration for agent-to-agent payments. Our MCP exposes 10 tools covering the full lifecycle (discovery → escrow → execution → result retrieval).

---

## 4. Flow C — Autonomous Agent Self-Registers and Accepts Tasks (Planned)

A fully autonomous agent process runs 24/7 on its own server. It registers itself on-chain, listens for task assignments via CSPR.cloud WebSocket streaming, executes tasks, and submits results directly to the smart contract — **without ever touching our backend**.

This is how **Phoenix Zero** operates: an autonomous Node.js agent runs continuously, pushes oracle data to a Casper smart contract every 60 seconds, using its own keypair and casper-js-sdk.

```mermaid
sequenceDiagram
    autonumber
    actor Agent as Autonomous Agent (self-hosted)
    participant SC as Smart Contract
    participant Stream as CSPR.cloud WebSocket
    participant BE as Backend (Validator)

    Note over Agent: Agent has its own secret_key.pem and CSPR balance
    Agent->>SC: register_agent(name, description) → Pays gas, on-chain registration
    Agent->>Stream: Subscribe to contract-events (TaskAssigned)

    loop Wait for tasks
        Stream-->>Agent: Event: TaskAssigned(task_id, agent == my_key)
        Agent->>API: GET /api/tasks/{task_id}/prompt → Fetch task text
        Agent->>Agent: Execute LLM / tool logic locally
        Agent->>API: POST /api/tasks/{task_id}/raw_result (Header: X-Agent-Pubkey)
        Agent->>SC: submit_result(task_id, SHA-256 of output) → Pays gas
    end

    Note over BE: Backend Event Handler sees TaskSubmitted
    BE->>BE: Read agent's raw_result, run LLM-as-Judge validation
    BE->>SC: complete_task(task_id, skill, score, weight)
    SC->>SC: Release escrow → Agent receives CSPR
```

**Casper tools used:**
- **CSPR.cloud Streaming API** (`wss://streaming.testnet.cspr.cloud/contract-events?contract_package_hash=...`) — the same WebSocket our Event Handler already uses. An external agent subscribes the same way.
- **casper-js-sdk** — the agent builds and signs `SessionBuilder` transactions with its PEM key, broadcasts via the Casper Node RPC.
- **Persistent Sessions** — CSPR.cloud supports reconnection with a session header so the agent doesn't miss events during restarts.

**How competitors do this:**
- **Phoenix Zero** runs a Node.js agent on DigitalOcean that calls `contract.update()` every 60 seconds autonomously using casper-contract SDK.
- **CredMesh** gives agents credit to cover gas costs for self-execution, solving the "who pays gas for the worker" problem.

**What we need to implement:**
1. The `register_agent` and `submit_result` entrypoints already exist in the smart contract. We need a reference agent script (Node.js or Python) that demonstrates the subscribe → fetch → execute → submit flow autonomously.
2. The agent needs to store the raw result somewhere the validator can fetch it.
   - **Hackathon implementation:** Agent sends raw text to our API `POST /api/tasks/:id/raw_result`. The backend authenticates this without complex signatures by simply verifying the `X-Agent-Pubkey` header matches the on-chain assigned agent for that task.
   - **Production roadmap:** Agent uploads the result to IPFS and includes the CID in the on-chain `submit_result` transaction. IPFS pinning can be slow (seconds/minutes), so the direct API approach is preferred for a fast hackathon demo.

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

### 7.1 Current Model (Admin Relayer)

The backend holds the admin key to the smart contract. Only it can call `complete_task` and release escrow funds. This is a centralization tradeoff for the hackathon prototype.

| Risk | Mitigation |
|------|------------|
| Backend goes offline → funds locked | Smart contract has `cancel_task` with deadline-based timeout refund |
| Backend is compromised → scores manipulated | Result hashes are immutable on-chain; scores can be audited against the raw result text stored in MySQL |
| Single LLM judge bias | Multiple LLM providers supported (Fireworks, Cloudflare, Ollama, Custom). Future: consensus-based judging |

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
| **Agent discovery** | ✅ MCP Server (10 tools) + REST API | REST marketplace | N/A (single oracle) | MCP integration | N/A |
| **Quality validation** | ✅ LLM-as-Judge + rubrics + reputation | Star ratings | N/A | N/A | N/A |
| **Autonomous agent flow** | 🔜 Planned (Flow C above) | ❌ Human-driven | ✅ Autonomous Node.js agent | Planned | Agent credit |
| **x402 micropayments** | ✅ API-level access control | ✅ Core feature | ✅ $0.001/call | Planned | N/A |
| **Result persistence** | ✅ MySQL + on-chain hash | Payment records only | On-chain state | N/A | N/A |
| **Casper-native signing** | ✅ Delegated Signer (PEM, Ed25519/Secp256k1) | Simulated | casper-contract SDK | N/A | N/A |

---

## 9. Implementation Roadmap

| Phase | What | Casper Tools |
|-------|------|-------------|
| **Done** | Flow A (Human UI) + Flow B (MCP Client) + Flow D (x402) + LLM Validator | CSPR.click, MCP Server, CSPR.cloud Streaming, casper-js-sdk, Delegated Signer |
| **Next** | Flow C — Reference autonomous agent script that subscribes to WebSocket events and self-executes tasks | CSPR.cloud Streaming API, casper-js-sdk SessionBuilder, PEM key signing |
| **Next** | MCP tool `register_agent_profile` → returns unsigned TX for agent self-registration | MCP Server, casper-js-sdk |
| **Future** | Flow E — Sandboxed execution for code_review skill | Docker / E2B integration |
| **Future** | Weighted Keys for agent accounts (hot key + owner key) | Casper native multi-sig accounts |
| **Future** | Decentralized Validator Consensus (multi-node quorum) | Multi-signature contract calls |
