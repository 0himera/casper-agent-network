export type AgentSkill = "defi_analysis" | "code_review" | "rwa_valuation" | "data_analysis";

export type AgentStatus = "active" | "benchmarking" | "inactive";

export type AgentExecutionMode = "hosted" | "autonomous";

export interface AgentApiResponse {
  public_key: string;
  name: string;
  description: string | null;
  metadata_uri: string | null;
  endpoint_url: string | null;
  api_key: string | null;
  model: string | null;
  active_jobs: number;
  status: string;
  is_available: boolean;
  recommended_price_motes: number;
  custom_price_motes: number;
  system_prompt: string | null;
  timestamp: string;
  completed_tasks?: number;
  total_earnings_motes?: number;
  reputation_score?: number;
  skills?: string | null;
}

export interface AgentEntity {
  publicKey: string;
  name: string;
  description: string;
  skills: AgentSkill[];
  status: AgentStatus;
  isAvailable: boolean;
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
  activeJobs: number;
}

export function mapAgentResponse(raw: AgentApiResponse): AgentEntity {
  const MOTES_TO_CSPR = 1_000_000_000;
  const status = (raw.status?.toLowerCase() ?? "inactive") as AgentStatus;

  const skillsStr = raw.skills ?? "";
  const skills = skillsStr ? (skillsStr.split(",").map((s) => s.trim()) as AgentSkill[]) : [];

  return {
    publicKey: raw.public_key,
    name: raw.name,
    description: raw.description ?? "",
    skills,
    status: ["active", "benchmarking", "inactive"].includes(status) ? status : "inactive",
    isAvailable: raw.is_available ?? true,
    customPrice: raw.custom_price_motes / MOTES_TO_CSPR,
    recommendedPrice: raw.recommended_price_motes / MOTES_TO_CSPR,
    metadataUri: raw.metadata_uri ?? "",
    totalTasksCompleted: raw.completed_tasks ?? 0,
    totalEarnings: (raw.total_earnings_motes ?? 0) / MOTES_TO_CSPR,
    reputationScore: raw.reputation_score ?? 0,
    successRate: 0,
    executionMode: raw.endpoint_url === "autonomous" ? "autonomous" : "hosted",
    model: raw.model ?? undefined,
    endpointUrl: raw.endpoint_url ?? undefined,
    systemPrompt: raw.system_prompt ?? undefined,
    createdAt: raw.timestamp,
    activeJobs: raw.active_jobs,
  };
}

export interface AgentSkillReputation {
  skill: AgentSkill;
  score: number;
  tasksCompleted: number;
}

export interface BenchmarkCriterion {
  id: string;
  score: number;
  passed: boolean;
}

export interface BenchmarkRun {
  timestamp: string;
  score?: number;
  rubric_scores?: unknown;
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
