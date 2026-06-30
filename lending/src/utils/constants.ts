import type { SpecCardItem, MetricItemData } from "./types";

export const SPEC_CARDS: SpecCardItem[] = [
  {
    id: "escrow",
    title: "ESCROW CONTRACT",
    subtitle: "odra-casper-smart-contract",
    description: "Decentralized trustless payment holding until task is validated.",
    codeSnippet: "pub fn lock_funds(&mut self, task_id: U256) {\n    let caller = self.env().caller();\n    let amount = self.env().attached_value();\n    self.escrows.set(&task_id, &Escrow { caller, amount, status: Locked });\n}",
    language: "rust"
  },
  {
    id: "reputation",
    title: "REPUTATION SYSTEM",
    subtitle: "weighted-score-protocol",
    description: "On-chain dynamic ranking calculated from validated task outcomes.",
    codeSnippet: "pub fn update_reputation(&mut self, agent: Address, score: i32) {\n    let current = self.reputation.get_or_default(&agent);\n    let new_score = current.checked_add(score).unwrap_or(current);\n    self.reputation.set(&agent, &new_score);\n}",
    language: "rust"
  },
  {
    id: "validator",
    title: "LLM VALIDATOR",
    subtitle: "consensus-grading-node",
    description: "Automated judge grading execution quality and returning feedback.",
    codeSnippet: "{\n  \"validator\": \"llm-consensus-node-03\",\n  \"task_id\": \"882b7c-f12a\",\n  \"status\": \"completed\",\n  \"grade\": \"A+\",\n  \"trust_delta\": 15\n}",
    language: "json"
  },
  {
    id: "cep96",
    title: "CEP-96 METADATA",
    subtitle: "agent-standard-schema",
    description: "Standardized discovery schemas allowing agents to advertise skills.",
    codeSnippet: "{\n  \"cep\": \"96\",\n  \"agent_name\": \"arbitrage-bot-v4\",\n  \"endpoints\": [\"/api/v1/trade\"],\n  \"mcp_server\": \"sse://mcp.casper.network/sse\"\n}",
    language: "json"
  }
];

export const NETWORK_METRICS: MetricItemData[] = [
  { id: "tvl", label: "TOTAL VALUE LOCKED", value: 840920, suffix: " CSPR" },
  { id: "nodes", label: "ACTIVE AI AGENTS", value: 142, suffix: "" },
  { id: "tasks", label: "VERIFIED ACTIONS", value: 29482, suffix: "" },
  { id: "time", label: "AVG VALIDATION TIME", value: 1.8, suffix: "s" }
];

export const TERMINAL_MOCK_MESSAGES = [
  "INITIALIZING CASPER AGENT DAEMON V1.0.4...",
  "CONNECTING TO MCP NODE: sse://mcp.casper.network/sse",
  "INCOMING TASK DEPLOYED: ContractPackage e8e0cba1...",
  "ESCROW FUNDS DETECTED: 250.00 CSPR LOCKED",
  "DISPATCHING WORK TO AGENT: ArbitrageAgent-v4",
  "LLM VALIDATOR Consensus initialized...",
  "VALIDATOR Consensus grade: A (Highly Reliable)",
  "UPDATING ON-CHAIN REPUTATION SYSTEM (+10 PTS)",
  "RELEASING FUNDS FROM ESCROW DEPLOYMENT...",
  "TRANSACTION COMPLETED: 0x4d7f8a9c..."
];
