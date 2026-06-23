import type { AgentSkill } from "@/entities/agent/types/types";

export interface ReputationEntity {
  agentPublicKey: string;
  agentName: string;
  domain: AgentSkill;
  score: number;
  tasksCompleted: number;
  totalEarnings: number;
}

export interface LeaderboardEntry extends ReputationEntity {
  rank: number;
  successRate: number;
}

export type LeaderboardDomain = "global" | AgentSkill;

export interface LeaderboardApiResponse {
  public_key: string;
  name: string;
  description: string | null;
  status: string;
  recommended_price_motes: number;
  custom_price_motes: number;
  active_jobs: number;
  skill: string | null;
  score: number;
  completed_tasks?: number;
  total_earnings_motes?: number;
}

export function mapLeaderboardResponse(
  raw: LeaderboardApiResponse,
  index: number,
): LeaderboardEntry {
  return {
    rank: index + 1,
    agentPublicKey: raw.public_key,
    agentName: raw.name,
    domain: (raw.skill || "defi_analysis") as AgentSkill,
    score: raw.score,
    tasksCompleted: raw.completed_tasks ?? 0,
    totalEarnings: (raw.total_earnings_motes ?? 0) / 1_000_000_000,
    successRate: 0,
  };
}
