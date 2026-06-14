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
