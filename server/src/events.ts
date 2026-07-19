export interface AgentRegisteredPayload {
  agent: string;
  name: string;
}

export interface TaskCreatedPayload {
  task_id: string;
  creator: string;
  budget: string;
  deadline: string;
  parent_task_id: string | null;
}

export interface TaskCancelledPayload {
  task_id: string;
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

export interface PriceUpdatedPayload {
  agent: string;
  custom_price: string;
}

export interface RecommendedPriceUpdatedPayload {
  agent: string;
  recommended_price: string;
}

export interface AgentUpdatedPayload {
  agent: string;
  name: string;
}

export interface TaskDisputedPayload {
  task_id: string;
  creator: string;
  disputer: string;
}

export interface PaymentClaimedPayload {
  task_id: string;
  creator: string;
  agent: string;
  amount: string;
}

export interface MetadataUpdatedPayload {
  name: string | null;
  description: string | null;
  icon_uri: string | null;
  project_uri: string | null;
}

export interface OwnershipTransferStartedPayload {
  previous_owner: string | null;
  new_owner: string | null;
}

export interface OwnershipTransferredPayload {
  previous_owner: string | null;
  new_owner: string | null;
}

export interface FeeDeductedPayload {
  task_id: string;
  agent: string;
  fee: string;
  payout: string;
}

export interface FeeRateUpdatedPayload {
  fee_bps: number;
}

export interface AgentAvailabilityChangedPayload {
  agent: string;
  available: boolean;
}

export interface TaskBudgetIncreasedPayload {
  task_id: string;
  creator: string;
  new_budget: string;
}


export interface ValidatorRegisteredPayload {
  validator: string;
}

export interface ValidatorStakedPayload {
  validator: string;
  amount: string;
}

export interface ValidatorUnstakedPayload {
  validator: string;
  amount: string;
}

export interface TreasuryDistributedPayload {
  total_yield: string;
  validators_paid: number;
}

export interface TreasuryBurnedPayload {
  burned_amount: string;
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

export interface DelegatedSignerUpdatedPayload {
  agent: string;
  delegated_signer: string | null;
}