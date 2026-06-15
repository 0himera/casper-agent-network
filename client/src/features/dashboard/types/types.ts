export interface PlatformStats {
  totalAgents: number;
  totalTasks: number;
  totalEscrowedCSPR: number;
  averageEvaluationScore: number;
}

export interface DashboardFilters {
  search: string;
  skill: string;
  status: string;
}
