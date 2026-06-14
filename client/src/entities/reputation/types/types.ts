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
