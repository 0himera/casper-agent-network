export type AgentSkill = "defi_analysis" | "code_review" | "rwa_valuation" | "data_analysis";

export type AgentStatus = "active" | "benchmarking" | "inactive";

export type AgentExecutionMode = "hosted" | "autonomous";

export interface AgentEntity {
  publicKey: string;
  name: string;
  description: string;
  skills: AgentSkill[];
  status: AgentStatus;
  customPrice: number;
  recommendedPrice: number;
  metadataUri: string;
  totalTasksCompleted: number;
  totalEarnings: number;
  reputationScore: number;
  successRate: number;
  executionMode: AgentExecutionMode;
  model?: string;
  endpointUrl?: string;
  systemPrompt?: string;
  createdAt: string;
}

export interface AgentSkillReputation {
  skill: AgentSkill;
  score: number;
  tasksCompleted: number;
}

export const SKILL_LABELS: Record<AgentSkill, string> = {
  defi_analysis: "DeFi Analysis",
  code_review: "Code Review",
  rwa_valuation: "RWA Valuation",
  data_analysis: "Data Analysis",
};

export const SKILL_BASE_PRICES: Record<AgentSkill, number> = {
  defi_analysis: 5,
  code_review: 10,
  rwa_valuation: 15,
  data_analysis: 2,
};

export const STATUS_LABELS: Record<AgentStatus, string> = {
  active: "Active",
  benchmarking: "Benchmarking",
  inactive: "Inactive",
};
