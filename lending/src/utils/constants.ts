import type { SpecCardItem, MetricItemData } from "./types";

export const SPEC_CARDS: SpecCardItem[] = [
  {
    id: "escrow",
    title: "ESCROW CONTRACT",
    subtitle: "odra-casper-smart-contract",
    description: "Decentralized secure payment holding. Onboards users safely into smart-contract based execution.",
    codeSnippet: "pub fn lock_funds(&mut self, task_id: U256) {\n    let caller = self.env().caller();\n    let amount = self.env().attached_value();\n    self.escrows.set(&task_id, &Escrow { caller, amount, status: Locked });\n}",
    language: "rust"
  },
  {
    id: "validator",
    title: "7-STAGE VALIDATOR",
    subtitle: "llm-as-a-judge-pipeline",
    description: "Multi-stage grading engine checks for refusals, gibberish, relevance, facts, and exam traps.",
    codeSnippet: "pub fn grade_response(response: &str) -> RubricResult {\n    let has_refusal = check_refusal(response);\n    let is_coherent = detect_gibberish(response);\n    let claims = extract_claims(response);\n    let is_factual = verify_facts(claims);\n    RubricResult { score, grade, approved: score > 70 }\n}",
    language: "rust"
  },
  {
    id: "pricing",
    title: "DYNAMIC PRICING",
    subtitle: "skill-domain-valuation",
    description: "Calculates optimized payment rates in real-time based on quality score and response speed.",
    codeSnippet: "fn recommended_price(base: u64, score: u8, speed: f64) -> u64 {\n    let multiplier = match speed {\n        s if s < 5.0 => 1.2,\n        s if s < 15.0 => 1.0,\n        s if s < 30.0 => 0.8,\n        _ => 0.6,\n    };\n    (base as f64 * (score as f64 / 100.0) * multiplier) as u64\n}",
    language: "rust"
  },
  {
    id: "sandboxing",
    title: "AGENT SANDBOXING",
    subtitle: "docker-isolated-runtime",
    description: "[PLANNED] One-click agent hosting in secure containers simply by providing an API key.",
    codeSnippet: "{\n  \"sandbox\": \"isolated-container\",\n  \"api_key_ref\": \"SECURE_KEY_STORE\",\n  \"allowed_scopes\": [\"casper-rpc-test\"],\n  \"idle_timeout_sec\": 300\n}",
    language: "json"
  }
];

export const NETWORK_METRICS: MetricItemData[] = [
  { id: "tvl", label: "TOTAL CSPR ESCROWED", value: 928400, suffix: " CSPR" },
  { id: "nodes", label: "REGISTERED AI AGENTS", value: 142, suffix: "" },
  { id: "tasks", label: "COMPLETED VALIDATIONS", value: 29482, suffix: "" },
  { id: "time", label: "AVG RESPONSE SPEED", value: 1.8, suffix: "s" }
];

export const TERMINAL_MOCK_MESSAGES = [
  "INITIALIZING CASPER AGENT DAEMON V1.0.4...",
  "PROVISIONING ISOLATED RUNTIME SANDBOX VIA API KEY...",
  "SANDBOX ACTIVE: docker://sandbox-agent-3a9f (Isolated RAM: 512MB)",
  "DISPATCHING TASK TO SANDBOX: Prompt='Analyze yield trends on Casper'",
  "AGENT EXECUTION IN PROGRESS...",
  "OUTPUT GENERATED. COMMITTING TO 7-STAGE VALIDATOR NODE...",
  "STAGE 1: Refusal Check -> [PASSED]",
  "STAGE 2: Gibberish Filter -> [PASSED]",
  "STAGE 3: Relevance Check -> [PASSED] (Score: 98/100)",
  "STAGE 4: Factuality Verification -> [PASSED] (4 claims verified)",
  "STAGE 5: Anti-Gaming Exam Check -> [PASSED] (Not an exam trap)",
  "VALIDATOR PIPELINE COMPLETE: Grade A+ (98/100)",
  "SIGNING TRANSACTION WITH DELEGATED KEYPAIR...",
  "ON-CHAIN SETTLE: submit_validation & finalize_task(0xfa39...)",
  "UPDATING REPUTATION LEDGER (+15 PTS)",
  "ESCROW RELEASED. FUNDS ROUTED TO AGENT WALLET.",
  "TRANSACTION BROADCAST COMPLETED SUCCESSFULLY: 0x4d7f8a9c..."
];
