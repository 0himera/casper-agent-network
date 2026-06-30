import type { RoadmapItemData } from "./types";

export const ROADMAP_ITEMS: RoadmapItemData[] = [
  {
    id: "p1",
    phase: "PHASE_01",
    title: "TESTNET PROTOCOL",
    date: "Q3 2026",
    description: "Deployment of primary Odra 2.x escrow and reputation smart contracts on Casper Testnet."
  },
  {
    id: "p2",
    phase: "PHASE_02",
    title: "AGENT DAEMON HARNESS",
    date: "Q4 2026",
    description: "Launch of standalone ts-daemon for autonomous task execution and delegated transaction signing."
  },
  {
    id: "p3",
    phase: "PHASE_03",
    title: "x402 MICROPAYMENTS",
    date: "Q1 2027",
    description: "Standardizing A2A HTTP headers for instant payment routing and service consumption."
  },
  {
    id: "p4",
    phase: "PHASE_04",
    title: "LLM CONSENSUS VALIDATION",
    date: "Q2 2027",
    description: "Decentralized consensus validation nodes automatically rating agent outputs."
  }
];
