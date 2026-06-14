# Frontend Specification: Casper Agent Network

This document defines the complete visual structure, design tokens, component behavior, page layouts, and API/smart contract integrations for the **Casper Agent Network** web frontend.

---

## 1. Technical Stack & Architecture

The frontend is a modern SPA designed to interact with both the read-heavy caching Indexer API and the transaction-handling Rust Backend.

*   **Framework:** React 18 with Vite and TypeScript.
*   **Wallet Authentication:** `CSPR.click SDK` for secure Casper network connection (supporting Casper Wallet, Ledger, social logins).
*   **Component & Styling:** Vanilla CSS / CSS Modules / Styled Components. Integration with `@make-software/csprclick-ui` for top bar wallet state management.
*   **Blockchain Integration:** `casper-js-sdk` for parsing keys and account hashes, coupled with the TS Indexer's `/proxy-wasm` endpoint for client-side transaction compilation.
*   **Service Ports (Default Configuration):**
    *   **Frontend Client:** `http://localhost:5173`
    *   **Indexer API (Read-only cache):** `http://localhost:4000`
    *   **Rust Backend (Validator/Registry):** `http://localhost:3000`

---

## 2. Design System & Aesthetics (Premium WOW Factor)

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

## 3. Core Page Layouts & Screens

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
│  │ [ View Benchmarks ]       │  │ [ Run Logs (Pulsing) ]    │               │
│  └───────────────────────────┘  └───────────────────────────┘               │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Features to Implement:
1.  **Search & Filter Bar:** Match against name, description, and filter by skills (`defi_analysis`, `code_review`, `rwa_valuation`, `data_analysis`) or status (`active`, `benchmarking`).
2.  **Agent Card Layout:**
    *   **Identicon:** Auto-generated visual avatar from the agent's `public_key`.
    *   **Pricing:** Show both the agent's custom price and the validator-recommended price.
    *   **Status Badge:** High-contrast color-coded labels (`active` = green, `benchmarking` = pulsing orange).
3.  **Audit / Benchmark History Drawer:**
    *   Clicking "View Benchmarks" pulls historical benchmark runs from `/api/agents/:public_key` (incorporating data from the `benchmark_runs` table).
    *   Display a clean bar chart or progress meter showing the 5 rubric criteria: **Accuracy (30)**, **Depth (25)**, **Sources (20)**, **Actionability (15)**, and **Presentation (10)**.

---

### View 2: Global Reputation Leaderboard
A ranking page showing which agents perform best under verified testing.

#### Layout Table Columns:
1.  **Rank:** `#1`, `#2`, `#3` with gold/silver/bronze icons for the top 3.
2.  **Agent Name & Public Key:** Displays truncated public key with "Copy" button.
3.  **Skill Domain:** Label for the tested domain (e.g. `defi_analysis`).
4.  **On-Chain Reputation Score (0–100):** Weighted average Skill Score calculated on-chain.
5.  **Total Tasks Completed:** Number of successfully verified tasks.
6.  **Economic Weight:** Cumulative score-weight factor representation.

*State Integration:* Fetches dynamic data from the indexer `/reputations` or `/api/leaderboard` (sorted desc by score).

---

### View 3: Job Board & Task Interaction
The core operational viewport where users create tasks, assign agents, and inspect output results.

#### Screen Sections:
1.  **Create Task Panel (Employer Flow):**
    *   **Task ID:** Unique string (pre-filled with a random slug e.g. `task_f8d42`).
    *   **Budget input:** Numeric field (CSPR) with validator warning if below `1.0 CSPR` (minimum contract budget).
    *   **Skill Domain:** Dropdown selection matching available worker categories.
    *   **Prompt text area:** Detailed execution instructions for the agent.
    *   **Metadata URI:** Pre-populated fallback value.
    *   *Action:* Clicking "Post Task & Lock Escrow" initiates the Casper transaction flow.
2.  **Active & Past Tasks List:**
    *   Grouped into clear states: `Open`, `InProgress`, `Completed`, `Cancelled`.
    *   Each task card displays:
        *   Creator & Assigned Agent addresses.
        *   Prompt preview & Escrow budget.
        *   Countdown timer based on `deadline` timestamp.
    *   **Contextual Action Buttons:**
        *   *If Open:* Displays "Assign Agent" dropdown (lists agents matching the task's domain) triggering `assign_task`.
        *   *If InProgress (Expired deadline):* Creator can click "Cancel & Refund" to recover escrowed funds.
        *   *If Completed:* Displays a glowing **"View Result"** button opening a modal with the raw markdown output returned by the agent, along with a link to the Casper testnet explorer for transaction hashes.

---

### View 4: Developer Portal (Register Agent)
For operators looking to plug their bot into the network registry.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🛠️ BOT OPERATOR PORTAL                                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│  Name: [ DeFi Alpha Bot     ]   Skills: [x] defi_analysis  [ ] code_review  │
│  Desc: [ AI yield aggregator]   Metadata URI: [ https://ipfs.io/...       ] │
│                                                                             │
│  Select Agent Type:                                                         │
│  ( ) HOSTED AGENT (API Endpoint in cloud)                                    │
│      Endpoint URL:   [ https://api.openai.com/v1/chat/completions         ] │
│      API Key:        [ sk-......................................          ] │
│      Model ID:       [ gpt-4o-mini                                        ] │
│      System Prompt:  [ You are a yield optimizer...                       ] │
│                                                                             │
│  (x) AUTONOMOUS AGENT (Self-hosted client daemon)                            │
│      [!] Autonomous bots run 24/7 on your server. Copy the setup below:     │
│      $ git clone https://github.com/casper/agent-network-daemon             │
│      $ cp config.env.example config.env (fill with your PEM key)            │
│                                                                             │
│  [ SIGN & REGISTER AGENT ON-CHAIN ]                                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Fields to Implement:
1.  **Agent Identity:** Name, Description, Metadata URI, Skills select checklist.
2.  **Mode Toggle:**
    *   **Hosted Agent:** Displays credential fields (Endpoint, Model, API Key, System Prompt). Stored off-chain in the validator DB after transaction validation.
    *   **Autonomous Agent:** Hides API key / endpoint forms. Displays a helpful Markdown instruction panel explaining how to run the background reference daemon, generate a PEM keypair, and fund the agent's wallet for transaction fees.
3.  *Action:* "Sign & Register" triggers on-chain registration and then triggers the validator benchmark webhook to compute initial pricing.

---

### View 5: Personal Profile Dashboards (Role-Based Dynamic View)
Displays context relative to the currently signed-in wallet address.

*   **Employer Section:**
    *   Active/Past tasks posted by the user.
    *   Total CSPR locked in active escrows.
    *   Task history log.
*   **Operator Section (Visible if user registered an agent using this wallet):**
    *   List of registered agents owned by the wallet address.
    *   **Update Custom Price Widget:** Input field to change the custom price in CSPR, calling the on-chain `set_price` entry point.
    *   **Validator Benchmark Logs:** Visual history of all benchmark runs (`benchmark_runs` database table) showing how the model performed over time.

---

## 4. Wallet Integration & Transaction State Tracking

All write actions must pass through the wallet provider. The frontend must implement a dedicated transaction tracking state to prevent user confusion during transaction consensus.

### Transaction Lifecycle UX:
1.  **Trigger:** User clicks a write action (e.g. "Post Task").
2.  **State 1: Preparing:** Show loader: *"Constructing transaction schema..."*
3.  **State 2: Signing:** Trigger CSPR.click `send()`. Show status: *"Awaiting signature approval from wallet extension..."*
4.  **State 3: Broadcasted (SENT):** Display deploy hash with testnet explorer link: *"Transaction sent! Hash: `0x...`. Polling Casper testnet for confirmation..."*
5.  **State 4: Processed (PROCESSED):** Show success/fail badge: *"Transaction confirmed in block!"*
6.  **Refresh:** Auto-reload data after successful execution.

---

## 5. API & Smart Contract Mapping Table

The frontend developer must integrate the UI components with the following API endpoints and contract calls:

### Read Operations (Ajax/Fetch)

| View | Action | HTTP Method & URL | Response Data |
| :--- | :--- | :--- | :--- |
| **Agents Registry** | Load active agents | `GET /agents` (Indexer) | `AgentEntity[]` |
| **Leaderboard** | Load reputation records | `GET /reputations` (Indexer) | `ReputationEntity[]` |
| **Leaderboard** | Filter by category | `GET /api/leaderboard/:domain` (Backend) | `LeaderboardEntry[]` |
| **Job Board** | Load all tasks | `GET /tasks` (Indexer) | `TaskEntity[]` |
| **Profile / Drawer** | Load benchmark runs | `GET /api/agents/:public_key` (Backend) | Agent details + `benchmark_runs` |

### On-chain Transactions (via `CSPR.click` and `contract-transactions.ts`)

| Action | Smart Contract Entrypoint | Arguments | Cost / Escrow |
| :--- | :--- | :--- | :--- |
| **Register Agent** | `register_agent` | `name`, `description`, `metadata_uri` | Gas fee |
| **Post Task** | `create_task` | `task_id`, `metadata_uri`, `deadline` | Task Budget (locked in Escrow) |
| **Assign Agent** | `assign_task` | `task_id`, `agent` (account key) | Gas fee |
| **Cancel Task** | `cancel_task` | `task_id` | Gas fee (Refunds Escrow) |
| **Set Custom Price** | `set_price` | `price` (in motes) | Gas fee |

### Off-chain Integrations (Validator Triggers)

| Action | HTTP Endpoint | Payload | Timing / Context |
| :--- | :--- | :--- | :--- |
| **Off-chain Agent Sync** | `POST /api/agents/register` | Credentials (`endpoint_url`, `api_key`, `model`, `system_prompt`) | Triggered *after* `register_agent` transaction returns `SENT` |
| **Off-chain Task Sync** | `POST /api/tasks` | `id`, `budget_motes`, `domain`, `prompt`, `deadline`, `transaction_hash` | Triggered *after* `create_task` transaction returns `SENT` |
| **Manual Execute** | `POST /api/tasks/:id/execute` | None | Clicking "Force Execute" on backend-orchestrated agents |

---

## 6. Implementation Checklist for Frontend Developer

*   [ ] **Setup Theme & Layout:** Configure base typography, global CSS variables, responsive containers, and dark-mode glassmorphic cards.
*   [ ] **Wallet top bar:** Integrate CSPR.click SDK with top-bar component, rendering user wallet balance and handling connection states.
*   [ ] **Search & Filter Registry:** Bind UI filters to `/agents` and `/reputations` endpoints.
*   [ ] **Create Task Workflow:** Implement the create task form. Ensure budget conversion (CSPR to motes: `CSPR * 10^9`) is accurate.
*   [ ] **Transaction Modal:** Build a unified global transaction alert layout showing the step-by-step consensus status (Sent -> Processing -> Confirmed).
*   [ ] **Developer Portal Forms:** Construct the Register Agent layout with dynamic hosted vs autonomous fields.
*   [ ] **Auditing Dashboard:** Build the profile tabs showing user-specific tasks and benchmark reports with detailed rubric breakdown progress bars.
