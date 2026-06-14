import type { AgentSkill } from "@/entities/agent/types/types";

export type TaskStatus = "open" | "in_progress" | "completed" | "cancelled";

export interface EvaluationScore {
  accuracy: number;
  depth: number;
  sources: number;
  actionability: number;
  presentation: number;
  total: number;
}

export interface TaskApiResponse {
  id: string;
  creator_public_key: string;
  assigned_agent_public_key: string | null;
  budget_motes: number;
  status: string;
  result_hash: string | null;
  result: string | null;
  metadata_uri: string | null;
  transaction_hash: string;
  domain: string;
  prompt: string;
  deadline: number;
  result_signature: string | null;
  timestamp: string;
}

function mapStatus(raw: string): TaskStatus {
  const s = raw.toLowerCase().replace(/\s+/g, "_");
  if (s === "open") return "open";
  if (s === "inprogress" || s === "in_progress") return "in_progress";
  if (s === "completed") return "completed";
  if (s === "cancelled") return "cancelled";
  return "open";
}

export function mapTaskResponse(raw: TaskApiResponse): TaskEntity {
  const MOTES_TO_CSPR = 1_000_000_000;
  return {
    id: raw.id,
    creator: raw.creator_public_key,
    assignedAgent: raw.assigned_agent_public_key,
    assignedAgentName: null,
    domain: (raw.domain || "defi_analysis") as AgentSkill,
    prompt: raw.prompt,
    budget: raw.budget_motes / MOTES_TO_CSPR,
    deadline: raw.deadline ? new Date(raw.deadline * 1000).toISOString() : "",
    status: mapStatus(raw.status),
    result: raw.result ?? null,
    resultHash: raw.result_hash ?? null,
    evaluation: null,
    transactionHashes: { create: raw.transaction_hash },
    createdAt: raw.timestamp,
    updatedAt: raw.timestamp,
  };
}

export interface TaskEntity {
  id: string;
  creator: string;
  assignedAgent: string | null;
  assignedAgentName: string | null;
  domain: AgentSkill;
  prompt: string;
  budget: number;
  deadline: string;
  status: TaskStatus;
  result: string | null;
  resultHash: string | null;
  evaluation: EvaluationScore | null;
  transactionHashes: {
    create?: string;
    assign?: string;
    submit?: string;
    complete?: string;
  };
  createdAt: string;
  updatedAt: string;
}

export const TASK_STATUS_LABELS: Record<TaskStatus, string> = {
  open: "Open",
  in_progress: "In Progress",
  completed: "Completed",
  cancelled: "Cancelled",
};
