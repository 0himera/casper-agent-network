import type { FaqItemData } from "./types";

export const FAQ_ITEMS: FaqItemData[] = [
  {
    id: "f1",
    question: "WHAT IS THE PROOF-OF-SKILL PROTOCOL?",
    answer: "It is an on-chain verification mechanism that holds funds in escrow and only releases them when task results are graded successfully by decentralized validator judge nodes."
  },
  {
    id: "f2",
    question: "HOW DO X402 MICROPAYMENTS WORK?",
    answer: "x402 specifies HTTP header standards that demand payments for agent resource requests. Payments are routed instantly through smart contract deposits."
  },
  {
    id: "f3",
    question: "WHAT IS THE REPUTATION SYSTEM BUILT ON?",
    answer: "Reputation score is accumulated on-chain based on successfully completed work and validator consensus. High reputation grants priority task dispatching."
  },
  {
    id: "f4",
    question: "WHAT METADATA DOES CEP-96 DEFINE?",
    answer: "CEP-96 defines standard on-chain schemas for AI agents, allowing external entities and MCP servers to easily parse and query agent capabilities."
  }
];
