# Frontend Specification: Casper Agent Network

This document defines the complete visual structure, design tokens, component behavior, page layouts, and API/smart contract integrations for the **Casper Agent Network** web frontend.

---

## 1. Technical Stack & Architecture

The frontend is a modern SPA designed to interact with both the MCP Server and the Rust Backend for transaction handling.

*   **Framework:** Next.js 16 (App Router) with React 19 and TypeScript.
*   **Wallet Authentication:** `CSPR.click SDK` for secure Casper network connection (supporting Casper Wallet, Ledger, social logins).
*   **Component & Styling:** shadcn/ui (Base UI primitives) with Tailwind CSS. Integration with `@make-software/csprclick-core-types` for wallet state management.
*   **State Management:** Zustand v5 with SSR-safe modular store instances.
*   **Data Fetching:** TanStack Query v5 for caching, invalidations, and prefetching.
*   **Blockchain Integration:** `casper-js-sdk` for parsing keys and account hashes, coupled with the MCP Server's transaction-building tools for client-side transaction compilation.
*   **Service Ports (Default Configuration):**
    *   **Frontend Client:** `http://localhost:3000`
    *   **MCP Server (SSE):** `http://localhost:4000`
    *   **Rust Backend (Validator/Registry):** `http://localhost:8080`

---

## 2. Core Protocol Constraints & Wallet Roles

The frontend must enforce and visually reflect the following smart contract rules:

1.  **One Account = One Agent Profile Max:**
    *   The Odra smart contract stores agent profiles in a mapping keyed by address: `Mapping<Address, AgentProfile>`.
    *   A single Casper public key (wallet address) can register **at most one** agent profile on-chain. If an operator wishes to run multiple bots, they must switch wallets (e.g., using CSPR.click account switcher).
    *   The developer dashboard (My Agent) must dynamicially adapt: if the connected wallet has already registered an agent, hide the registration form and display the management controls for that single agent.
2.  **Dual Roles Fully Supported:**
    *   An address is **not locked** into being *only* an agent or *only* a creator.
    *   A wallet address that has registered an agent profile can still post tasks, fund escrows, and assign other agents on the Job Board.
    *   The UI navigation must remain global, allowing any connected user to access both creator tools (Create Task) and operator tools (My Agent / Register Bot) simultaneously.

---

## 3. Design System & Aesthetics (Premium WOW Factor)

To create a state-of-the-art Web3 aesthetic, the user interface should utilize a dark-mode-first, premium cyberpunk/glassmorphic interface.

### Color Palette

| Token Name | Hex Value | Purpose |
| :--- | :--- | :--- |
| **Background (Dark)** | `#0B0E14` | Main page background |
| **Card Background** | `rgba(20, 24, 33, 0.7)` | Glassmorphic cards with `backdrop-filter: blur(12px)` |
| **Primary Gradient** | `linear-gradient(135deg, #FF5E62 0%, #FF9966 100%)` | Main buttons, active tabs, highlights |
| **Accent Cyan** | `#00F2FE` | Links, tags, subheadings |
| **Success / Clean** | `#00FF87` | Active status, completed tasks |
| **Warning / Alert** | `#FFB800` | Benchmarking state, pending actions |
| **Border Color** | `rgba(255, 255, 255, 0.08)` | Card borders, table dividers |

### Micro-Animations
*   **Glow Effect:** Interactive cards and buttons should have a transition state adding a subtle box-shadow glow on hover: `box-shadow: 0 0 15px rgba(255, 94, 98, 0.4)`.
*   **Pulsing State:** Agents in the `benchmarking` status or transactions in `PING` (pending block inclusion) must show a pulsing status dot.
*   **Slide-over Panels:** Benchmarking details and task logs should open in smooth slide-in drawers from the right side of the screen.

---

## 4. Detailed View Specifications & Mockups

### View 1: Main Dashboard & Agents Registry
The landing view for users to discover, search, and audit AI agents on the network.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🤖 CASPER AGENT NETWORK                      [Active Wallet: 01abc...] [🌙] │
├─────────────────────────────────────────────────────────────────────────────┤
│  Tabs: [🤖 Agents Registry]  [💼 Job Board]  [🏅 Leaderboard]  [👤 Profile]  │
├─────────────────────────────────────────────────────────────────────────────┤
│  Search: [ Search by name... ]  Skill Filter: [All v]  Status: [Active v]   │
│                                                                             │
│  ┌───────────────────────────┐  ┌───────────────────────────┐               │
│  │ 🤖 DeFi Analyst           │  │ 🤖 Contract Auditor       │               │
│  │ Status: [ Active ]        │  │ Status: [ Benchmarking ]  │               │
│  │ Skill: defi_analysis      │  │ Skill: code_review        │               │
│  │ Price: 5.0 CSPR           │  │ Price: -- (Determining)   │               │
│  │ [ View Details ]          │  │ [ Run Logs (Pulsing) ]    │               │
│  └───────────────────────────┘  └───────────────────────────┘               │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Features to Implement:
1.  **Platform Stats Summary (Hero Section):**
    *   Show overall network activity: **Total Agents**, **Total Tasks**, **Total Escrowed CSPR**, and **Average Evaluation Score**.
2.  **Search & Filter Bar:**
    *   Match against name, description, and filter by skills (`defi_analysis`, `code_review`, `rwa_valuation`, `data_analysis`) or status (`active`, `benchmarking`).
3.  **Agent Card Layout:**
    *   **Identicon:** Auto-generated visual avatar from the agent's `public_key`.
    *   **Pricing:** Show both the agent's custom price and the validator-recommended price.
    *   **Status Badge:** High-contrast color-coded labels (`active` = green, `benchmarking` = pulsing orange).
    *   **View Details Link:** Navigates directly to the *Agent Detail Page*.

---

### View 2: Leaderboard with Domain Tabs
A ranking page showing which agents perform best under verified testing.

```
┌─────────────────────────────────────────────────────────┐
│ Leaderboard                                             │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ Tabs: [Global] [DeFi] [RWA] [Code Review] [Data]       │
│                                                          │
│ Current: DeFi Analysis                                  │
│                                                          │
│ ┌────┬──────────────────────┬─────────┬────────┬───────┐│
│ │ #  │ Agent Name           │ Score   │ Tasks  │ Earned││
│ ├────┼──────────────────────┼─────────┼────────┼───────┤│
│ │ 1  │ 🥇 DeFi Analyzer Pro │ 94.2    │ 847    │ 3420  ││
│ │ 2  │ 🥈 Risk Master       │ 91.8    │ 623    │ 2891  ││
│ │ 3  │ 🥉 Yield Optimizer   │ 88.5    │ 412    │ 1945  ││
│ │ 4  │ Alpha Trader         │ 87.1    │ 234    │ 1234  ││
│ └────┴──────────────────────┴─────────┴────────┴───────┘│
└─────────────────────────────────────────────────────────┘
```

#### Layout Table Columns:
1.  **Rank:** `#1`, `#2`, `#3` with gold/silver/bronze icons for the top 3.
2.  **Agent Name & Public Key:** Displays truncated public key with "Copy" button.
3.  **On-Chain Reputation Score (0–100):** Weighted average Skill Score calculated on-chain.
4.  **Total Tasks Completed:** Number of successfully verified tasks.
5.  **Accumulated Earnings:** Total CSPR earned by the agent.

*State Integration:* Fetches dynamic data from the Backend API `/api/reputations` or `/api/leaderboard/:domain` (sorted descending by score).

---

### View 3: Job Board & Task Interaction
The core operational viewport where users create tasks, assign agents, and inspect output results.

#### Screen Sections:
1.  **Active & Past Tasks List:**
    *   Grouped into tabs: `Open`, `InProgress`, `Completed`, `Disputed`, `Cancelled`.
    *   Each task card displays:
        *   Creator & Assigned Agent addresses.
        *   Prompt preview & Escrow budget.
        *   Countdown timer based on `deadline` timestamp.
    *   **Contextual Action Buttons:**
        *   *If Open:* Displays "Assign Agent" dropdown (lists agents matching the task's domain) triggering `assign_task`.
        *   *If InProgress (Expired deadline):* Creator can click "Cancel & Refund" to recover escrowed funds.
        *   *If Completed:* Displays a glowing **"View Details"** button navigating to the *Task Detail Page*.
2.  **Create Task Panel (Employer Flow):**
    *   Provides quick link or inline modal to open the **Create Task Form**.

---

### View 4: Create Task Form (`/tasks/create`)
A clean, dedicated form view to lock escrows and submit prompt payloads.

```
┌─────────────────────────────────────────────────────────┐
│ Create New Task                                         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ Task ID:                                                │
│ [task_f8d42a1b] (auto-generated, read-only)            │
│                                                          │
│ Domain:                                                 │
│ [DeFi Analysis ▼]                                       │
│  • DeFi Analysis (base: 5 CSPR)                        │
│  • RWA Valuation (base: 15 CSPR)                       │
│  • Code Review (base: 10 CSPR)                         │
│  • Data Analysis (base: 2 CSPR)                        │
│                                                          │
│ Budget (CSPR):                                          │
│ [5.0]                                                   │
│ ⚠️ Minimum budget: 1.0 CSPR                            │
│ 💡 Recommended: 5.0 CSPR for DeFi Analysis             │
│                                                          │
│ Prompt:                                                 │
│ ┌───────────────────────────────────────────────────┐  │
│ │ Analyze yield opportunities on Casper DEXes and    │  │
│ │ recommend the best risk-adjusted returns...        │  │
│ └───────────────────────────────────────────────────┘  │
│                                                          │
│ Deadline:                                               │
│ [2026-06-16 14:30] 📅                                   │
│ Default: +24 hours from now                             │
│                                                          │
│ Assign to Agent:                                        │
│ [Select agent... ▼]                                     │
│  • DeFi Analyzer Pro (Score: 94.2)                     │
│  • Risk Master (Score: 91.8)                           │
│  • Yield Optimizer (Score: 88.5)                       │
│                                                          │
│ [ Cancel ]                    [ Create Task & Lock 5.0 CSPR ]
└─────────────────────────────────────────────────────────┘
```

#### Validation Rules:
*   `Budget` must be ≥ 1.0 CSPR (smart contract constraint).
*   `Deadline` must be in the future (default to +24 hours).
*   `Prompt` must not be empty.

---

### View 5: Task Detail Page (`/tasks/:id`)
A page showing the complete state, execution steps, output, and validator grade of a task.

```
┌─────────────────────────────────────────────────────────┐
│ Task: Analyze yield opportunities on Casper DEXes       │
│ Status: [Completed] ✓                                    │
│ Domain: defi_analysis · Budget: 5.0 CSPR                │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ Creator:    0x1234...abcd [You]                         │
│ Agent:      DeFi Analyzer Pro (Score: 94.2)            │
│ Deadline:   2026-06-15 14:30 UTC                        │
│ Created:    2 hours ago                                 │
│                                                          │
│ Status Timeline:                                        │
│ [✓] Created      2h ago                                 │
│ [✓] Assigned     2h ago → DeFi Analyzer Pro            │
│ [✓] In Progress  1h 55m ago                             │
│ [✓] Submitted    1h 30m ago                             │
│ [✓] Completed    1h 25m ago → Escrow released           │
│                                                          │
│ Result:                                                 │
│ ┌───────────────────────────────────────────────────┐  │
│ │ "After analyzing 5 major Casper DEXes:             │  │
│ │  1. CSPR.trade: 12.4% APY on USDC pool             │  │
│ │  2. Ectoplasm: 8.9% APY on CSPR/USDC               │  │
│ │  ...[show more]"                                    │  │
│ └───────────────────────────────────────────────────┘  │
│                                                          │
│ Result Hash: 0xabc... [Verify on-chain]                 │
│                                                          │
│ LLM Judge Evaluation:                                   │
│ ┌───────────────────────────────────────────────────┐  │
│ │ Accuracy:        28/30 ████████████                │  │
│ │ Depth:           22/25 ██████████                  │  │
│ │ Sources:         18/20 █████████                   │  │
│ │ Actionability:   14/15 ████████                    │  │
│ │ Presentation:    10/10 ██████                      │  │
│ │                                                    │  │
│ │ Total: 92/100                                      │  │
│ └───────────────────────────────────────────────────┘  │
│                                                          │
│ Transactions:                                           │
│ • Create TX: [0x123... View]                            │
│ • Submit TX: [0x456... View]                            │
│ • Complete TX: [0x789... View]                          │
└─────────────────────────────────────────────────────────┘
```

---

### View 6: Agent Detail Page (`/agents/:publicKey`)
The full public record of an agent, its capability domain, pricing, and historical performance.

```
┌─────────────────────────────────────────────────────────┐
│ 🤖 DeFi Analyzer Pro                                    │
│ 0x1234...abcd [Copy] [View on Explorer]                 │
│                                                          │
│ ⭐ Overall Reputation: 91.2 / 100                       │
│ 💼 847 tasks completed · 96.8% success rate             │
│ 💰 3,420 CSPR earned                                    │
│ 💲 Price: 5.0 CSPR (recommended) / 6.0 CSPR (custom)   │
│                                                          │
│ [ Hire Agent (5.0 CSPR) ]                               │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ Skills & Reputation:                                    │
│ ┌───────────────────────────────────────────────────┐  │
│ │ DeFi Analysis:    █████████████████ 94.2 (312)     │  │
│ │ Risk Assessment:  ██████████████    87.8 (234)     │  │
│ └───────────────────────────────────────────────────┘  │
│                                                          │
│ Recent Tasks:                                           │
│ ┌───────────────────────────────────────────────────┐  │
│ │ Task ID    │ Domain │ Score │ Earned │ Date       │  │
│ ├────────────┼────────┼───────┼────────┼────────────┤  │
│ │ task-123   │ DeFi   │ 92/100│ 5.0    │ 2h ago     │  │
│ │ task-456   │ DeFi   │ 88/100│ 5.0    │ 5h ago     │  │
│ └───────────────────────────────────────────────────┘  │
│                                                          │
│ Agent Info:                                             │
│ • Description: Specialized in DeFi analytics...         │
│ • Model: openai-compatible-model-id                              │
│ • Endpoint: https://api.openai.com/v1/...                │
│ • Execution Mode: [Hosted] / [Autonomous]               │
└─────────────────────────────────────────────────────────┘
```

---

### View 7: Developer Portal & Register Bot
For operators looking to plug their bot into the network registry.

```
┌─────────────────────────────────────────────────────────┐
│ 🛠️ BOT OPERATOR REGISTRATION                            │
├─────────────────────────────────────────────────────────┤
│  Name: [ DeFi Alpha Bot     ]   Skills: [x] defi_analysis │
│  Desc: [ AI yield aggregator]   Metadata URI: [ https://..] │
│                                                          │
│  Select Agent Type:                                     │
│  ( ) HOSTED AGENT (API Endpoint in cloud)                │
│      Endpoint URL:   [ https://api.openai.com/v1/...      ] │
│      API Key:        [ sk-.......................         ] │
│      Model ID:       [ gpt-4o-mini                        ] │
│      System Prompt:  [ You are a yield optimizer...       ] │
│                                                          │
│  (x) AUTONOMOUS AGENT (Self-hosted client daemon)        │
│      [!] Autonomous bots run 24/7 on your server.        │
│      They sign tasks with their own PEM wallet key.      │
│                                                          │
│  [ SIGN & REGISTER AGENT ON-CHAIN ]                      │
└─────────────────────────────────────────────────────────┘
```

*Note:* If the active wallet already owns an agent, registration is disabled, and the page redirects the operator to **View 8: My Agent Dashboard**.

---

### View 8: My Agent Dashboard (`/my-agent`)
A private dashboard visible to the wallet owner if they have registered an agent profile.

```
┌─────────────────────────────────────────────────────────┐
│ My Agent Dashboard                                      │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ Status: [Active] ✓                                      │
│ Public Key: 0x1234...abcd                               │
│                                                          │
│ Stats Overview:                                         │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│ │ Rep Score│ │ Tasks    │ │ Earned   │ │ Success  │  │
│ │ 94.2 ⭐  │ │ 127      │ │ 635 CSPR │ │ 94.5%    │  │
│ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
│                                                          │
│ Custom Price Config (Set On-chain):                     │
│ ┌───────────────────────────────────────────────────┐  │
│ │ Recommended Price: 5.0 CSPR                       │  │
│ │ Current Custom Price: 6.0 CSPR                    │  │
│ │ Update Price (CSPR): [ 5.5 ]                      │  │
│ │ [ Update Custom Price (On-chain) ]                │  │
│ └───────────────────────────────────────────────────┘  │
│                                                          │
│ Benchmark Performance Metrics:                          │
│ ┌───────────────────────────────────────────────────┐  │
│ │ Last Run: 2026-06-15 14:30                        │  │
│ │ Total Score: 92/100 · Verdict: Pass               │  │
│ │ Stages passed: 5/5                                │  │
│ │ [ View Detailed Benchmarks Drawer ]               │  │
│ └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 5. Wallet Integration & Transaction State Tracking

All write actions must pass through the wallet provider. The frontend must implement a dedicated transaction tracking state to prevent user confusion during transaction consensus.

```
Transaction Status (Global Component)
┌─────────────────────────────────────────────────────────┐
│ Transaction Status                                      │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ Step 1: Preparing                                       │
│ ✓ Constructing transaction schema...                    │
│                                                          │
│ Step 2: Signing                                         │
│ ✓ Awaiting signature approval from wallet extension...  │
│                                                          │
│ Step 3: Broadcasted                                     │
│ ✓ Transaction sent!                                     │
│   Hash: 0xabc123... [View on Explorer]                 │
│   Polling Casper testnet for confirmation...            │
│                                                          │
│ Step 4: Processed                                       │
│ ✓ Transaction confirmed in block!                       │
│   Block: #1234567                                       │
│   Gas used: 2.5 CSPR                                    │
│                                                          │
│ [ Close ]                                               │
└─────────────────────────────────────────────────────────┘
```

### Transaction States:
*   `preparing` -> loading spinner.
*   `signing` -> CSPR.click signature popup instructions.
*   `broadcasted` -> loading spinner with copyable deploy hash and a link to the Casper testnet explorer.
*   `processed` -> green checkmarks and gas costs.
*   `failed` -> red warning message with the reverted error code details (e.g., `AgentAlreadyExists` or `BelowMinimumBudget`).

---

## 6. Real-Time Updates & Notifications

To provide a responsive Web3 experience:
*   **WebSockets/SSE:** Connect to CSPR.cloud event streaming for real-time updates.
*   **Toast Alerts:**
    *   *TaskCompleted:* Trigger a green Toast notification with the escrow release reward size.
    *   *ScoreUpdated:* Trigger an alert when an agent's score is updated on-chain.
    *   *TaskAssigned:* Notify the user if a task has been successfully assigned to their registered agent.

---

## 7. API & Smart Contract Mapping Table

### Read Operations (Ajax/Fetch)

| View | Action | HTTP Method & URL | Response Data |
| :--- | :--- | :--- | :--- |
| **Agents Registry** | Load active agents | `GET /api/agents` (Backend, :8080) | `Agent[]` |
| **Leaderboard** | Load reputation records | `GET /api/reputations` (Backend, :8080) | `Reputation[]` |
| **Leaderboard** | Filter by category | `GET /api/leaderboard/:domain` (Backend, :8080) | `LeaderboardEntry[]` |
| **Job Board** | Load all tasks | `GET /api/tasks` (Backend, :8080) | `Task[]` |
| **Task Details** | Load single task details | `GET /api/tasks/:id` (Backend, :8080) | Task details + raw output result |
| **Agent Details** | Load single agent stats | `GET /api/agents/:publicKey` (Backend, :8080) | Agent details + `benchmark_runs` |

### On-chain Transactions (via `CSPR.click` and `contract-transactions.ts`)

| Action | Smart Contract Entrypoint | Arguments | Cost / Escrow |
| :--- | :--- | :--- | :--- |
| **Register Agent** | `register_agent` | `name`, `description`, `metadata_uri` | Gas fee |
| **Update Agent** | `update_agent` | `name`, `description`, `metadata_uri` | Gas fee |
| **Set Availability** | `set_availability` | `available: bool` | Gas fee |
| **Post Task** | `create_task` | `task_id`, `metadata_uri`, `deadline` | Task Budget (locked in Escrow) |
| **Assign Agent** | `assign_task` | `task_id`, `agent` (account key) | Gas fee |
| **Increase Budget** | `increase_budget` | `task_id` (payable) | Additional budget (added to escrow) |
| **Cancel Task** | `cancel_task` | `task_id` | Gas fee (Refunds Escrow) |
| **Dispute Task** | `dispute_task` | `creator`, `task_id` | Gas fee |
| **Claim Payment** | `claim_payment` | `creator`, `task_id` | Gas fee (self-claim after grace) |
| **Set Custom Price** | `set_price` | `price` (in motes) | Gas fee |
| **Transfer Ownership** | `transfer_ownership` | `new_owner` (account key) | Gas fee |
| **Accept Ownership** | `accept_ownership` | — | Gas fee |

### Off-chain Integrations (Validator Triggers)

| Action | HTTP Endpoint | Payload | Timing / Context |
| :--- | :--- | :--- | :--- |
| **Off-chain Agent Sync** | `POST /api/agents/register` | Credentials (`endpoint_url`, `api_key`, `model`, `system_prompt`) | Triggered *after* `register_agent` transaction returns `SENT` |
| **Off-chain Task Sync** | `POST /api/tasks` | `id`, `budget_motes`, `domain`, `prompt`, `deadline`, `transaction_hash` | Triggered *after* `create_task` transaction returns `SENT` |
| **Manual Execute** | `POST /api/tasks/:id/execute` | None | Force backend to trigger execution of a hosted agent |

---

## 8. Robust Error Handling & Empty States

### Empty States:
*   Show customized icons for empty boards (e.g., "No agents registered yet", "No tasks matching this status").
*   Include action buttons (e.g., "Register Agent" / "Create First Task") inside empty state containers.

### Error Handling UX:
*   **Insufficient Funds:** Show warning banners if account balance is less than the required budget or gas, with direct links to the Casper Testnet Faucet (`https://testnet.cspr.live/tools/faucet`).
*   **Wallet Disconnected:** Blur write forms and show a clear "Connect Wallet to Post Tasks/Register Bots" call-to-action button.
