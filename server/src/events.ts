export interface AgentRegisteredPayload {
  agent: string;
  name: string;
}

export interface TaskCreatedPayload {
  task_id: string;
  creator: string;
  budget: string;
}

export interface TaskAssignedPayload {
  task_id: string;
  agent: string;
}

export interface TaskSubmittedPayload {
  task_id: string;
  agent: string;
  result_hash: string;
}

export interface TaskCompletedPayload {
  task_id: string;
  score: number;
}

export interface ScoreUpdatedPayload {
  agent: string;
  skill: string;
  new_score: number;
}

export interface ContractEvent<T> {
  action: string;
  data: {
    contract_package_hash: string;
    contract_hash: string;
    name: string;
    data: T;
  };
  extra: {
    deploy_hash: string;
    event_id: number;
    transform_id: number;
  };
  timestamp: string;
}