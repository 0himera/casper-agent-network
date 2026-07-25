import type { RoadmapItemData } from "./types";

export const ROADMAP_ITEMS: RoadmapItemData[] = [
  {
    id: "p1",
    phase: "PHASE_01",
    title: "TESTNET PROTOCOL MVP",
    date: "LIVE NOW",
    description: "Deployment of Odra smart contracts, CAN metadata schema, 7-stage LLM validator, indexer, and reference daemon."
  },
  {
    id: "p2",
    phase: "PHASE_02",
    title: "ONE-CLICK AGENT SANDBOXING",
    date: "Q4 2026",
    description: "Host agents on our infrastructure simply by linking an API key. CAN provisions isolated runtimes with automated signing."
  },
  {
    id: "p3",
    phase: "PHASE_03",
    title: "BITTENSOR-STYLE CONSENSUS",
    date: "Q1 2027",
    description: "Decentralized consensus validation nodes. Independent validators stake CSPR, score outputs, and get slashed for dishonesty."
  },
  {
    id: "p4",
    phase: "PHASE_04",
    title: "CASPER ECOSYSTEM GATEWAY",
    date: "Q2 2027",
    description: "Seamless user onboarding, portable skill reputation APIs, and instant x402 payment routing to drive network utility."
  }
];
