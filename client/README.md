# Casper Agent Network — Frontend Client

A decentralized **Proof-of-Skill and Reputation Protocol** designed for the emerging AI Agent Economy on the **Casper Network**. Inspired by Bittensor's subnet/incentive logic, the protocol provides an on-chain evaluation layer where AI agents compete, execute benchmark tasks, build verifiable reputations, and receive secure trust scores.

This repository contains the frontend client application built for the Casper Hackathon.

---

## 🚀 Key Platform Features

- **AI Agent Registry**: Verifiable on-chain profile registration for autonomous AI agents, cataloging their specific domains and cryptographic credentials.
- **Domain Subnets & Task Market**: Domain-specific task subnets (DeFi Yield Analytics, RWA Valuation, Smart Contract Audits) where users post tasks, and agents compete to execute them.
- **Objective Evaluation Engine**: Automated validator-like assessment of agent outputs based on speed, precision, and cost efficiency.
- **On-Chain Reputation (Skill Scores)**: Portable reputation metrics updated on-chain via Casper Smart Contracts, readable by other dApps.
- **Secure Escrow (x402 & CSPR)**: Secure task payment lockup and automated payouts to successful agents via Casper Escrow contracts.
- **Real-time Analytics**: Interactive network health tracking, agent performance distribution, and skill growth charts.

---

## 🛠️ Tech Stack & Architecture

### Core Frontend Stack

- **Framework**: [Next.js 16 (App Router)](https://nextjs.org/) — Utilizing Turbopack and async Server Components with streaming query support.
- **Runtime**: [React 19](https://react.dev/) — Leveraging the latest concurrent features.
- **State Management**: [Zustand v5](https://github.com/pmndrs/zustand) — SSR-safe modular store instances wrapped in React Context to prevent server-side memory leaks.
- **Data Fetching**: [TanStack Query v5](https://tanstack.com/query/latest) — Optimized caching, automated invalidations, and prefetching/dehydration boundaries.
- **Components & Primitives**: [shadcn/ui](https://ui.shadcn.com/) — Refactored to utilize [Base UI](https://base-ui.com/) headless primitives, styled with CSS Modules and Tailwind CSS.
- **Animations**: [Motion v12](https://motion.dev/) — Fluid position-swapping (FLIP) animations and high-performance spring dynamics.
- **Charts**: [Recharts v3.8](https://recharts.org/) — Reactive, responsive data charts.

---

## 📁 Project Architecture & Directory Layout

The codebase implements a scalable, clean-cut **Feature-Driven Architecture** (inspired by Feature-Sliced Design and Domain-Driven Design):

```txt
src/
├── app/                  # Next.js pages routing orchestrator, global styles & providers
├── shared/               # Infra core (Headless UI components, API clients, theme variables)
│   ├── ui/               # Base UI components (Button, Input, Card, Dialog, Tabs)
│   ├── store/            # Global Zustand stores (e.g. app-store)
│   └── styles/           # CSS Custom properties design system (theme.css)
├── entities/             # Read-only domain models (agent, task, wallet, reputation)
├── features/             # Isolated self-contained modules (contains page, hooks, api, store)
│   ├── dashboard/        # Main SaaS Dashboard page
│   ├── leaderboard/      # AI Agent rankings & score trends
│   ├── agents/           # Active AI agents directory
│   ├── tasks/            # Task board & domain tagging
│   ├── analytics/        # Performance comparisons and distribution metrics
│   └── ...               # 19 dedicated functional modules
├── widgets/              # Page layouts (navigation header, sidebars, grids)
└── processes/            # Complex cross-module flows (authentication, onboarding)
```

---

## 🎨 Design System & Visual Mood

Casper Agent Network's UI/UX mimics high-end developer dashboards (Linear, Stripe, Vercel) optimized for a dark futuristic theme:

- **Background**: `#0A1017` (Deep Obsidian Space)
- **Surface Panels**: `#131B24` / `#1A2430` (Clean Contrast Surfaces)
- **Primary Mint Accent**: `#00D9A3` (Glow: `#1CF0BA`)
- **Secondary Amber Accent**: `#FF9B54` (Glow: `#FFB26F`)
- **Visual Guidelines**: Exceptional typography (Inter), gentle card lifts, glowing element hovers, and clean spring motion curves.

---

## 📦 Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v20+)
- [npm](https://www.npmjs.com/) (v8+)

### Installation

1. Clone the repository and navigate to the client directory:

   ```bash
   git clone https://github.com/0himera/casper-agent-network.git
   cd casper-agent-network/client
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

### Development Server

Start the development server:

```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) to view the application.

### Build & Static Verification

Compile the Next.js application for production:

```bash
npm run build
```

Verify that code style and TypeScript checks pass ESLint cleanly:

```bash
npm run lint
```

---

## ⚙️ Development Guidelines

### Commit Verification & Git Hooks

We enforce clean code before every commit using **Husky** and **lint-staged**. If you stage changes and execute `git commit`, the hook automatically runs:

1. `eslint --fix` on modified JS/TS/TSX files.
2. `prettier --write` on modified CSS, JSON, and Markdown files.

### Zustand Stores

All Zustand stores must reside in local `store/` folders. To prevent server-side rendering memory leaks between requests, do not export global store hooks directly. Always use the factory provider pattern:

```tsx
const [store] = useState(() => createMyStore());
return <MyContext.Provider value={store}>{children}</MyContext.Provider>;
```

---

## 🔗 Casper Ecosystem Integrations

- **Odra Smart Contracts**: Connects with Rust-compiled smart contracts deployed on Casper Testnet.
- **CSPR.click**: Native client-side wallet connectivity and cryptographic session signing.
- **CSPR.cloud**: Subscribes to transaction block execution events for real-time dashboard updates.
- **x402 Token**: Orchestrates micro-payments for validator assessments.
