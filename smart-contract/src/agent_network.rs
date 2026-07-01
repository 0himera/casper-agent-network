use odra::casper_types::U512;
use odra::prelude::*;

const MINIMUM_BUDGET: u64 = 1_000_000_000u64; // 1 CSPR
const MAX_TASK_ID_LEN: usize = 128;
const CLAIM_GRACE_PERIOD: u64 = 86_400_000; // 24h in ms
const FEE_TIER_HIGH: u32 = 90;
const FEE_TIER_LOW: u32 = 50;
const DEFAULT_FEE_BPS: u32 = 500; // 5%
const MAX_FEE_BPS: u32 = 3000; // 30%

// Staking & Slashing
const MINIMUM_STAKE: u64 = 50_000_000_000u64; // 50 CSPR
const UNBONDING_PERIOD: u64 = 1_800_000u64; // 30 min in ms (testing)
const SLASH_DEADLINE_BPS: u32 = 1000; // 10% slash for missed deadline
const SLASH_DISPUTE_BPS: u32 = 2000; // 20% slash for dispute → cancel
const SLASH_LOW_SCORE_BPS: u32 = 500; // 5% slash for score < threshold
const LOW_SCORE_THRESHOLD: u32 = 30;

// Validator Settings
const MINIMUM_VALIDATOR_STAKE: u64 = 100_000_000_000u64; // 100 CSPR
const DEVIATION_TOLERANCE: u32 = 10;
const SLASH_DEVIATION_BPS_PER_10: u32 = 500; // 5% per 10 points over tolerance

#[odra::odra_type]
pub struct AgentProfile {
    pub name: String,
    pub description: String,
    pub metadata_uri: String,
    pub active_jobs: u32,
    pub custom_price: U512,
    pub recommended_price: U512,
    pub is_available: bool,
}

#[odra::odra_type]
pub enum TaskStatus {
    Open,
    InProgress,
    Completed,
    Disputed,
    Cancelled,
}

#[odra::odra_type]
pub struct Task {
    pub creator: Address,
    pub assigned_agent: Option<Address>,
    pub budget: U512,
    pub status: TaskStatus,
    pub result_hash: String,
    pub metadata_uri: String,
    pub deadline: u64,
    pub parent_task_id: Option<String>,
}

#[odra::odra_type]
#[derive(Default)]
pub struct ReputationState {
    pub weighted_sum: u64,
    pub total_weight: u64,
    pub tasks_completed: u32,
    pub last_update: u64,
}

#[odra::odra_type]
#[derive(Default)]
pub struct StakeInfo {
    pub amount: U512,
    pub unbonding_amount: U512,
    pub unbonding_start: u64,
}

#[odra::odra_type]
pub struct ValidatorProfile {
    pub stake: U512,
    pub is_active: bool,
    pub total_validations: u32,
}

#[odra::odra_type]
pub struct Validation {
    pub validator: Address,
    pub score: u32,
}

#[odra::event]
pub struct AgentRegistered {
    pub agent: Address,
    pub name: String,
}

#[odra::event]
pub struct AgentUpdated {
    pub agent: Address,
    pub name: String,
}

#[odra::event]
pub struct TaskCreated {
    pub task_id: String,
    pub creator: Address,
    pub budget: U512,
    pub deadline: u64,
    pub parent_task_id: Option<String>,
}

#[odra::event]
pub struct TaskAssigned {
    pub task_id: String,
    pub agent: Address,
}

#[odra::event]
pub struct TaskSubmitted {
    pub task_id: String,
    pub agent: Address,
    pub result_hash: String,
}

#[odra::event]
pub struct TaskCompleted {
    pub task_id: String,
    pub score: u32,
}

#[odra::event]
pub struct ValidationSubmitted {
    pub task_id: String,
    pub validator: Address,
    pub score: u32,
}

#[odra::event]
pub struct ScoreUpdated {
    pub agent: Address,
    pub skill: String,
    pub new_score: u32,
}

#[odra::event]
pub struct ReputationDecayed {
    pub agent: Address,
    pub skill: String,
    pub new_weighted_sum: u64,
    pub new_total_weight: u64,
}


#[odra::event]
pub struct TreasuryDistributed {
    pub agent: Address,
    pub amount: U512,
}

#[odra::event]
pub struct TreasuryBurned {
    pub amount: U512,
}

#[odra::event]
pub struct PriceUpdated {
    pub agent: Address,
    pub custom_price: U512,
}

#[odra::event]
pub struct RecommendedPriceUpdated {
    pub agent: Address,
    pub recommended_price: U512,
}

#[odra::event]
pub struct TaskCancelled {
    pub task_id: String,
}

#[odra::event]
pub struct TaskDisputed {
    pub task_id: String,
    pub creator: Address,
    pub disputer: Address,
}

#[odra::event]
pub struct PaymentClaimed {
    pub task_id: String,
    pub creator: Address,
    pub agent: Address,
    pub amount: U512,
}

#[odra::event]
pub struct MetadataUpdated {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon_uri: Option<String>,
    pub project_uri: Option<String>,
}

#[odra::event]
pub struct OwnershipTransferStarted {
    pub previous_owner: Option<Address>,
    pub new_owner: Option<Address>,
}

#[odra::event]
pub struct OwnershipTransferred {
    pub previous_owner: Option<Address>,
    pub new_owner: Option<Address>,
}

#[odra::event]
pub struct FeeDeducted {
    pub task_id: String,
    pub agent: Address,
    pub fee: U512,
    pub payout: U512,
}

#[odra::event]
pub struct FeeRateUpdated {
    pub fee_bps: u32,
}

#[odra::event]
pub struct AgentAvailabilityChanged {
    pub agent: Address,
    pub available: bool,
}

#[odra::event]
pub struct TaskBudgetIncreased {
    pub task_id: String,
    pub creator: Address,
    pub new_budget: U512,
}

#[odra::event]
pub struct Staked {
    pub agent: Address,
    pub amount: U512,
    pub total_stake: U512,
}

#[odra::event]
pub struct UnstakeRequested {
    pub agent: Address,
    pub amount: U512,
    pub available_at: u64,
}

#[odra::event]
pub struct StakeWithdrawn {
    pub agent: Address,
    pub amount: U512,
}

#[odra::event]
pub struct UnstakeCancelled {
    pub agent: Address,
    pub amount: U512,
}

#[odra::event]
pub struct SlashApplied {
    pub agent: Address,
    pub amount: U512,
    pub remaining_stake: U512,
}

#[odra::event]
pub struct ValidatorRegistered {
    pub validator: Address,
}

#[odra::event]
pub struct ValidatorStaked {
    pub validator: Address,
    pub amount: U512,
    pub total_stake: U512,
}

#[odra::event]
pub struct ValidatorSlashed {
    pub validator: Address,
    pub amount: U512,
    pub remaining_stake: U512,
    pub reason: String,
}

#[odra::odra_error]
pub enum ContractErrors {
    AgentAlreadyExists = 3001,
    AgentNotFound = 3002,
    TaskNotFound = 3003,
    TaskNotOpen = 3004,
    TaskNotAssigned = 3005,
    NotTaskCreator = 3006,
    NotAssignedAgent = 3007,
    BelowMinimumBudget = 3008,
    TaskNotSubmitted = 3009,
    TaskAlreadyAssigned = 3010,
    NotContractAdmin = 3011,
    TaskAlreadyExists = 3012,
    DeadlinePassed = 3013,
    DeadlineNotPassed = 3014,
    InvalidScore = 3015,
    InvalidWeight = 3016,
    TaskIdTooLong = 3017,
    DeadlineInPast = 3018,
    ResultAlreadySubmitted = 3019,
    ClaimTooEarly = 3020,
    TaskNotDisputed = 3021,
    ArithmeticOverflow = 3022,
    InvalidFeeRate = 3023,
    AgentNotAvailable = 3024,
    InsufficientStake = 3025,
    StakeNotFound = 3026,
    UnbondingInProgress = 3027,
    UnbondingNotReady = 3028,
    NoUnbondingInProgress = 3029,
    AgentUnbonding = 3030,
    InvalidSlashRate = 3031,
    ActiveJobsExist = 3032,
    InvalidUnstakeAmount = 3033,
    ValidatorAlreadyExists = 3034,
    ValidatorNotFound = 3035,
    ValidatorNotActive = 3036,
    InsufficientValidatorStake = 3037,
    TaskAlreadyValidated = 3038,
    NoValidations = 3039,
}

#[odra::module(
    errors = ContractErrors,
    events = [
        AgentRegistered,
        AgentUpdated,
        TaskCreated,
        TaskAssigned,
        TaskSubmitted,
        TaskCompleted,
        ScoreUpdated,
        PriceUpdated,
        RecommendedPriceUpdated,
        TaskCancelled,
        TaskDisputed,
        PaymentClaimed,
        MetadataUpdated,
        OwnershipTransferStarted,
        OwnershipTransferred,
        FeeDeducted,
        FeeRateUpdated,
        AgentAvailabilityChanged,
        TaskBudgetIncreased,
        Staked,
        UnstakeRequested,
        StakeWithdrawn,
        UnstakeCancelled,
        SlashApplied,
        ValidationSubmitted,
        ValidatorRegistered,
        ValidatorStaked,
        ValidatorSlashed
    ]
)]
pub struct AgentNetwork {
    admin: Var<Option<Address>>,
    pending_admin: Var<Option<Address>>,
    agents: Mapping<Address, AgentProfile>,
    tasks: Mapping<(Address, String), Task>,
    reputations: Mapping<(Address, String), ReputationState>,
    contract_name: Var<String>,
    contract_description: Var<String>,
    contract_icon_uri: Var<String>,
    contract_project_uri: Var<String>,
    fee_bps: Var<u32>,
    stakes: Mapping<Address, StakeInfo>,
    total_slashed: Var<U512>,
    treasury_balance: Var<U512>,
    validators: Mapping<Address, ValidatorProfile>,
    task_validations: Mapping<(Address, String), Vec<Validation>>,
}

#[odra::module]
impl AgentNetwork {
    pub fn init(&mut self, admin: Address) {
        self.admin.set(Some(admin));
        self.env().emit_event(OwnershipTransferred {
            previous_owner: None,
            new_owner: Some(admin),
        });
        self.contract_name.set("Casper Agent Network".to_string());
        self.contract_description.set(
            "A decentralized reputation protocol and task marketplace for AI agents on the Casper Network.".to_string()
        );
        self.contract_icon_uri.set("https://agent-network.casper.dev/icon.png".to_string());
        self.contract_project_uri.set("https://agent-network.casper.dev".to_string());
        self.fee_bps.set(DEFAULT_FEE_BPS);
    }

    pub fn transfer_ownership(&mut self, new_owner: &Address) {
        self.assert_admin();
        let previous_owner = self.admin.get().flatten();
        let new_owner_opt = Some(*new_owner);
        self.pending_admin.set(Some(*new_owner));
        self.env().emit_event(OwnershipTransferStarted {
            previous_owner,
            new_owner: new_owner_opt,
        });
    }

    pub fn accept_ownership(&mut self) {
        let caller = self.env().caller();
        let pending = self.pending_admin.get().flatten();
        if pending != Some(caller) {
            self.env().revert(ContractErrors::NotContractAdmin);
        }
        let previous_owner = self.admin.get().flatten();
        self.admin.set(Some(caller));
        self.pending_admin.set(None);
        self.env().emit_event(OwnershipTransferred {
            previous_owner,
            new_owner: Some(caller),
        });
    }

    pub fn renounce_ownership(&mut self) {
        self.assert_admin();
        let previous_owner = self.admin.get().flatten();
        self.admin.set(None);
        self.pending_admin.set(None);
        self.env().emit_event(OwnershipTransferred {
            previous_owner,
            new_owner: None,
        });
    }

    pub fn get_pending_owner(&self) -> Option<Address> {
        self.pending_admin.get().flatten()
    }

    pub fn contract_name(&self) -> Option<String> {
        self.contract_name.get()
    }

    pub fn contract_description(&self) -> Option<String> {
        self.contract_description.get()
    }

    pub fn contract_icon_uri(&self) -> Option<String> {
        self.contract_icon_uri.get()
    }

    pub fn contract_project_uri(&self) -> Option<String> {
        self.contract_project_uri.get()
    }

    pub fn get_admin(&self) -> Option<Address> {
        self.admin.get().flatten()
    }

    fn assert_admin(&self) {
        if self.admin.get().flatten() != Some(self.env().caller()) {
            self.env().revert(ContractErrors::NotContractAdmin);
        }
    }

    pub fn update_metadata(
        &mut self,
        name: Option<String>,
        description: Option<String>,
        icon_uri: Option<String>,
        project_uri: Option<String>,
    ) {
        self.assert_admin();
        if let Some(ref v) = name {
            self.contract_name.set(v.clone());
        }
        if let Some(ref v) = description {
            self.contract_description.set(v.clone());
        }
        if let Some(ref v) = icon_uri {
            self.contract_icon_uri.set(v.clone());
        }
        if let Some(ref v) = project_uri {
            self.contract_project_uri.set(v.clone());
        }
        self.env().emit_event(MetadataUpdated {
            name,
            description,
            icon_uri,
            project_uri,
        });
    }

    pub fn set_fee_rate(&mut self, fee_bps: u32) {
        self.assert_admin();
        if fee_bps > MAX_FEE_BPS {
            self.env().revert(ContractErrors::InvalidFeeRate);
        }
        self.fee_bps.set(fee_bps);
        self.env().emit_event(FeeRateUpdated { fee_bps });
    }

    pub fn get_fee_rate(&self) -> u32 {
        self.fee_bps.get().unwrap_or(DEFAULT_FEE_BPS)
    }

    pub fn get_effective_fee_rate(&self, agent: Address, skill: String) -> u32 {
        self.compute_fee_rate(agent, &skill)
    }

    fn compute_fee_rate(&self, agent: Address, skill: &str) -> u32 {
        let base = self.fee_bps.get().unwrap_or(DEFAULT_FEE_BPS);
        let rep = self.reputations.get_or_default(&(agent, skill.to_string()));
        if rep.total_weight == 0 {
            return base;
        }
        let avg = (rep.weighted_sum / rep.total_weight) as u32;
        if avg >= FEE_TIER_HIGH {
            base / 5
        } else if avg < FEE_TIER_LOW {
            (base * 2).min(MAX_FEE_BPS)
        } else {
            base
        }
    }

    pub fn set_availability(&mut self, available: bool) {
        let caller = self.env().caller();
        let mut profile = self
            .agents
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);
        profile.is_available = available;
        self.agents.set(&caller, profile);
        self.env().emit_event(AgentAvailabilityChanged {
            agent: caller,
            available,
        });
    }

    #[odra(payable)]
    pub fn increase_budget(&mut self, task_id: String) {
        let caller = self.env().caller();
        let attached = self.env().attached_value();

        if attached == U512::zero() {
            self.env().revert(ContractErrors::BelowMinimumBudget);
        }

        let key = (caller, task_id.clone());
        let mut task = self
            .tasks
            .get(&key)
            .unwrap_or_revert_with(&self.env(), ContractErrors::TaskNotFound);

        match task.status {
            TaskStatus::Open | TaskStatus::InProgress => {}
            _ => self.env().revert(ContractErrors::TaskNotOpen),
        }

        task.budget = task
            .budget
            .checked_add(attached)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        self.tasks.set(&key, task.clone());

        self.env().emit_event(TaskBudgetIncreased {
            task_id,
            creator: caller,
            new_budget: task.budget,
        });
    }

    pub fn register_agent(&mut self, name: String, description: String, metadata_uri: String) {
        let caller = self.env().caller();
        if self.agents.get(&caller).is_some() {
            self.env().revert(ContractErrors::AgentAlreadyExists);
        }

        let profile = AgentProfile {
            name: name.clone(),
            description,
            metadata_uri,
            active_jobs: 0,
            custom_price: U512::zero(),
            recommended_price: U512::zero(),
            is_available: true,
        };

        self.agents.set(&caller, profile);
        self.env().emit_event(AgentRegistered { agent: caller, name });
    }

    pub fn update_agent(&mut self, name: String, description: String, metadata_uri: String) {
        let caller = self.env().caller();
        let mut profile = self
            .agents
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);

        profile.name = name.clone();
        profile.description = description;
        profile.metadata_uri = metadata_uri;
        self.agents.set(&caller, profile);

        self.env().emit_event(AgentUpdated { agent: caller, name });
    }

    #[odra(payable)]
    pub fn create_task(&mut self, task_id: String, metadata_uri: String, deadline: u64, parent_task_id: Option<String>) {
        let caller = self.env().caller();
        let attached_value = self.env().attached_value();
        let current_time = self.env().get_block_time();

        if attached_value < U512::from(MINIMUM_BUDGET) {
            self.env().revert(ContractErrors::BelowMinimumBudget);
        }

        if task_id.len() > MAX_TASK_ID_LEN {
            self.env().revert(ContractErrors::TaskIdTooLong);
        }

        if deadline <= current_time {
            self.env().revert(ContractErrors::DeadlineInPast);
        }

        let key = (caller, task_id.clone());
        if self.tasks.get(&key).is_some() {
            self.env().revert(ContractErrors::TaskAlreadyExists);
        }

        let task = Task {
            creator: caller,
            assigned_agent: None,
            budget: attached_value,
            status: TaskStatus::Open,
            result_hash: String::new(),
            metadata_uri,
            deadline,
            parent_task_id: parent_task_id.clone(),
        };

        self.tasks.set(&key, task);
        self.env().emit_event(TaskCreated {
            task_id,
            creator: caller,
            budget: attached_value,
            deadline,
            parent_task_id,
        });
    }

    pub fn assign_task(&mut self, task_id: String, agent: Address) {
        let caller = self.env().caller();
        let current_time = self.env().get_block_time();

        let key = (caller, task_id.clone());
        let mut task = self
            .tasks
            .get(&key)
            .unwrap_or_revert_with(&self.env(), ContractErrors::TaskNotFound);

        if task.creator != caller {
            self.env().revert(ContractErrors::NotTaskCreator);
        }

        if task.status != TaskStatus::Open {
            self.env().revert(ContractErrors::TaskNotOpen);
        }

        if task.deadline <= current_time {
            self.env().revert(ContractErrors::DeadlinePassed);
        }

        let mut agent_profile = self
            .agents
            .get(&agent)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);

        if !agent_profile.is_available {
            self.env().revert(ContractErrors::AgentNotAvailable);
        }

        // Staking: agent must have minimum stake and not be unbonding
        let stake_info = self.stakes.get_or_default(&agent);
        if stake_info.amount < U512::from(MINIMUM_STAKE) {
            self.env().revert(ContractErrors::InsufficientStake);
        }
        if stake_info.unbonding_amount > U512::zero() {
            self.env().revert(ContractErrors::AgentUnbonding);
        }

        task.assigned_agent = Some(agent);
        task.status = TaskStatus::InProgress;
        self.tasks.set(&key, task);

        agent_profile.active_jobs = agent_profile
            .active_jobs
            .checked_add(1)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        self.agents.set(&agent, agent_profile);

        self.env().emit_event(TaskAssigned { task_id, agent });
    }

    pub fn submit_result(&mut self, creator: Address, task_id: String, result_hash: String) {
        let caller = self.env().caller();
        let current_time = self.env().get_block_time();

        let key = (creator, task_id.clone());
        let mut task = self
            .tasks
            .get(&key)
            .unwrap_or_revert_with(&self.env(), ContractErrors::TaskNotFound);

        if task.assigned_agent != Some(caller) && self.admin.get().flatten() != Some(caller) {
            self.env().revert(ContractErrors::NotAssignedAgent);
        }

        if task.status != TaskStatus::InProgress {
            self.env().revert(ContractErrors::TaskNotAssigned);
        }

        if current_time > task.deadline {
            self.env().revert(ContractErrors::DeadlinePassed);
        }

        if !task.result_hash.is_empty() {
            self.env().revert(ContractErrors::ResultAlreadySubmitted);
        }

        let assigned_agent = task.assigned_agent.unwrap();
        task.result_hash = result_hash.clone();
        self.tasks.set(&key, task);

        self.env().emit_event(TaskSubmitted {
            task_id,
            agent: assigned_agent,
            result_hash,
        });
    }

    // ── Validators ──────────────────────────────────────────────

    #[odra(payable)]
    pub fn register_validator(&mut self) {
        let caller = self.env().caller();
        let attached = self.env().attached_value();

        if self.validators.get(&caller).is_some() {
            self.env().revert(ContractErrors::ValidatorAlreadyExists);
        }

        if attached < U512::from(MINIMUM_VALIDATOR_STAKE) {
            self.env().revert(ContractErrors::InsufficientValidatorStake);
        }

        let profile = ValidatorProfile {
            stake: attached,
            is_active: true,
            total_validations: 0,
        };
        self.validators.set(&caller, profile);

        self.env().emit_event(ValidatorRegistered { validator: caller });
        self.env().emit_event(ValidatorStaked {
            validator: caller,
            amount: attached,
            total_stake: attached,
        });
    }

    #[odra(payable)]
    pub fn stake_validator(&mut self) {
        let caller = self.env().caller();
        let attached = self.env().attached_value();

        let mut profile = self
            .validators
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ValidatorNotFound);

        profile.stake = profile
            .stake
            .checked_add(attached)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);

        if profile.stake >= U512::from(MINIMUM_VALIDATOR_STAKE) {
            profile.is_active = true;
        }

        self.validators.set(&caller, profile.clone());

        self.env().emit_event(ValidatorStaked {
            validator: caller,
            amount: attached,
            total_stake: profile.stake,
        });
    }

    pub fn unstake_validator(&mut self, amount: U512) {
        let caller = self.env().caller();
        let mut profile = self
            .validators
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ValidatorNotFound);

        if amount.is_zero() || amount > profile.stake {
            self.env().revert(ContractErrors::InvalidUnstakeAmount);
        }

        profile.stake = profile.stake - amount;
        if profile.stake < U512::from(MINIMUM_VALIDATOR_STAKE) {
            profile.is_active = false;
        }
        self.validators.set(&caller, profile.clone());

        self.env().transfer_tokens(&caller, &amount);

        self.env().emit_event(ValidatorSlashed { // using Slashed event shape for unstake log or creating new one. Better to just transfer.
            validator: caller,
            amount: amount,
            remaining_stake: profile.stake,
            reason: "unstake".to_string(),
        });
    }

    pub fn submit_validation(&mut self, creator: Address, task_id: String, score: u32) {
        let caller = self.env().caller();

        if score > 100 {
            self.env().revert(ContractErrors::InvalidScore);
        }

        let mut profile = self
            .validators
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ValidatorNotFound);

        if !profile.is_active || profile.stake < U512::from(MINIMUM_VALIDATOR_STAKE) {
            self.env().revert(ContractErrors::ValidatorNotActive);
        }

        let key = (creator, task_id.clone());
        let task = self
            .tasks
            .get(&key)
            .unwrap_or_revert_with(&self.env(), ContractErrors::TaskNotFound);

        if task.status != TaskStatus::InProgress && task.status != TaskStatus::Disputed {
            self.env().revert(ContractErrors::TaskNotAssigned);
        }

        if task.result_hash.is_empty() {
            self.env().revert(ContractErrors::TaskNotSubmitted);
        }

        let mut validations = self.task_validations.get_or_default(&key);
        if validations.iter().any(|v| v.validator == caller) {
            self.env().revert(ContractErrors::TaskAlreadyValidated);
        }

        validations.push(Validation { validator: caller, score });
        self.task_validations.set(&key, validations);

        profile.total_validations = profile
            .total_validations
            .checked_add(1)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        self.validators.set(&caller, profile);

        self.env().emit_event(ValidationSubmitted {
            task_id,
            validator: caller,
            score,
        });
    }

    pub fn finalize_task(&mut self, creator: Address, task_id: String, skill: String, weight: u32) {
        if weight == 0 {
            self.env().revert(ContractErrors::InvalidWeight);
        }

        let key = (creator, task_id.clone());
        let mut task = self
            .tasks
            .get(&key)
            .unwrap_or_revert_with(&self.env(), ContractErrors::TaskNotFound);

        if task.status != TaskStatus::InProgress && task.status != TaskStatus::Disputed {
            self.env().revert(ContractErrors::TaskNotAssigned);
        }

        let validations = self.task_validations.get_or_default(&key);
        if validations.is_empty() {
            self.env().revert(ContractErrors::NoValidations);
        }

        // Calculate median score
        let mut scores: Vec<u32> = validations.iter().map(|v| v.score).collect();
        scores.sort_unstable();
        let mid = scores.len() / 2;
        let median_score = if scores.len() % 2 == 0 {
            (scores[mid - 1] + scores[mid]) / 2
        } else {
            scores[mid]
        };

        let agent = task.assigned_agent.unwrap();
        let mut agent_profile = self
            .agents
            .get(&agent)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);

        let budget = task.budget;
        let fee_rate = self.compute_fee_rate(agent, &skill);
        let total_fee = budget * U512::from(fee_rate) / U512::from(10_000u32);
        let payout = budget - total_fee;

        task.status = TaskStatus::Completed;
        self.tasks.set(&key, task);

        agent_profile.active_jobs = agent_profile
            .active_jobs
            .checked_sub(1)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        self.agents.set(&agent, agent_profile);

        let mut reward_pool = total_fee * U512::from(5000u32) / U512::from(10_000u32);
        let treasury_share = total_fee - reward_pool;
        
        let mut treasury = self.treasury_balance.get().unwrap_or(U512::zero());
        treasury += treasury_share;

        let mut eligible_stake = U512::zero();
        let mut eligible_validators = Vec::new();

        // Slashing logic based on deviation
        for val in &validations {
            let deviation = if val.score > median_score {
                val.score - median_score
            } else {
                median_score - val.score
            };

            let mut profile = self.validators.get(&val.validator).unwrap();

            if deviation <= DEVIATION_TOLERANCE {
                eligible_stake += profile.stake;
                eligible_validators.push((val.validator, profile.stake));
            } else {
                let diff = deviation - DEVIATION_TOLERANCE;
                let penalty_factor = (diff / 10) * SLASH_DEVIATION_BPS_PER_10;
                let penalty_bps = if penalty_factor > 10_000 { 10_000 } else { penalty_factor };
                
                if penalty_bps > 0 {
                    let slash_amount = profile.stake * U512::from(penalty_bps) / U512::from(10_000u32);
                    profile.stake = profile.stake - slash_amount;
                    if profile.stake < U512::from(MINIMUM_VALIDATOR_STAKE) {
                        profile.is_active = false;
                    }
                    self.validators.set(&val.validator, profile.clone());
                    
                    treasury += slash_amount;
                    let mut total_slashed = self.total_slashed.get().unwrap_or(U512::zero());
                    total_slashed += slash_amount;
                    self.total_slashed.set(total_slashed);

                    self.env().emit_event(ValidatorSlashed {
                        validator: val.validator,
                        amount: slash_amount,
                        remaining_stake: profile.stake,
                        reason: "deviation".to_string(),
                    });
                }
            }
        }

        if eligible_stake > U512::zero() {
            for (val_addr, stake) in eligible_validators {
                let reward = reward_pool * stake / eligible_stake;
                self.env().transfer_tokens(&val_addr, &reward);
            }
        } else {
            treasury += reward_pool;
        }

        self.treasury_balance.set(treasury);

        self.env().transfer_tokens(&agent, &payout);
        
        self.env().emit_event(FeeDeducted {
            task_id: task_id.clone(),
            agent,
            fee: total_fee,
            payout,
        });

        // Reputation update
        let mut rep_state = self.reputations.get_or_default(&(agent, skill.clone()));
        rep_state.weighted_sum = rep_state
            .weighted_sum
            .checked_add((median_score as u64) * (weight as u64))
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        rep_state.total_weight = rep_state
            .total_weight
            .checked_add(weight as u64)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        rep_state.tasks_completed = rep_state
            .tasks_completed
            .checked_add(1)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        rep_state.last_update = self.env().get_block_time();

        let new_score = if rep_state.total_weight == 0 {
            0
        } else {
            (rep_state.weighted_sum / rep_state.total_weight) as u32
        };
        self.reputations.set(&(agent, skill.clone()), rep_state);

        self.env().emit_event(TaskCompleted { task_id: task_id.clone(), score: median_score });
        self.env().emit_event(ScoreUpdated {
            agent,
            skill,
            new_score,
        });

        if median_score < LOW_SCORE_THRESHOLD {
            self.apply_slash(agent, SLASH_LOW_SCORE_BPS);
        }
    }


    pub fn distribute_treasury(&mut self, agent: Address, amount: U512) {
        self.assert_admin();
        let mut treasury = self.treasury_balance.get().unwrap_or(U512::zero());
        if amount > treasury {
            self.env().revert(ContractErrors::ArithmeticOverflow);
        }
        treasury -= amount;
        self.treasury_balance.set(treasury);
        self.env().transfer_tokens(&agent, &amount);
        self.env().emit_event(TreasuryDistributed { agent, amount });
    }

    pub fn burn_treasury(&mut self, amount: U512) {
        self.assert_admin();
        let mut treasury = self.treasury_balance.get().unwrap_or(U512::zero());
        if amount > treasury {
            self.env().revert(ContractErrors::ArithmeticOverflow);
        }
        treasury -= amount;
        self.treasury_balance.set(treasury);
        // By decrementing the internal balance without transferring, 
        // the tokens are permanently locked in the contract (burned).
        self.env().emit_event(TreasuryBurned { amount });
    }

    pub fn sync_decayed_reputation(
        &mut self,
        agent: Address,
        skill: String,
        decayed_weighted_sum: u64,
        decayed_total_weight: u64,
    ) {
        self.assert_admin();

        let mut rep_state = self.reputations.get_or_default(&(agent, skill.clone()));
        
        rep_state.weighted_sum = decayed_weighted_sum;
        rep_state.total_weight = decayed_total_weight;
        rep_state.last_update = self.env().get_block_time();
        
        self.reputations.set(&(agent, skill.clone()), rep_state);

        self.env().emit_event(ReputationDecayed {
            agent,
            skill,
            new_weighted_sum: decayed_weighted_sum,
            new_total_weight: decayed_total_weight,
        });
    }

    pub fn cancel_task(&mut self, task_id: String) {
        let caller = self.env().caller();
        let current_time = self.env().get_block_time();

        let key = (caller, task_id.clone());
        let mut task = self
            .tasks
            .get(&key)
            .unwrap_or_revert_with(&self.env(), ContractErrors::TaskNotFound);

        if task.creator != caller {
            self.env().revert(ContractErrors::NotTaskCreator);
        }

        let can_cancel = match task.status {
            TaskStatus::Open => true,
            TaskStatus::InProgress => current_time >= task.deadline && task.result_hash.is_empty(),
            TaskStatus::Disputed => true,
            _ => false,
        };

        if !can_cancel {
            if task.status == TaskStatus::InProgress && current_time < task.deadline {
                self.env().revert(ContractErrors::DeadlineNotPassed);
            } else {
                self.env().revert(ContractErrors::TaskNotOpen);
            }
        }

        let budget = task.budget;
        let assigned_agent = task.assigned_agent;

        // Slash agent if applicable
        if let Some(agent_addr) = assigned_agent {
            match task.status {
                TaskStatus::InProgress => {
                    // Missed deadline — slash 10%
                    self.apply_slash(agent_addr, SLASH_DEADLINE_BPS);
                }
                TaskStatus::Disputed => {
                    // Dispute resolved against agent — slash 20%
                    self.apply_slash(agent_addr, SLASH_DISPUTE_BPS);
                }
                _ => {}
            }
        }

        task.status = TaskStatus::Cancelled;
        self.tasks.set(&key, task);

        if let Some(agent) = assigned_agent {
            if let Some(mut agent_profile) = self.agents.get(&agent) {
                agent_profile.active_jobs = agent_profile
                    .active_jobs
                    .checked_sub(1)
                    .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
                self.agents.set(&agent, agent_profile);
            }
        }

        self.env().transfer_tokens(&caller, &budget);
        self.env().emit_event(TaskCancelled { task_id });
    }

    pub fn dispute_task(&mut self, creator: Address, task_id: String) {
        let caller = self.env().caller();
        let is_admin = self.admin.get().flatten() == Some(caller);

        let key = (creator, task_id.clone());
        let mut task = self
            .tasks
            .get(&key)
            .unwrap_or_revert_with(&self.env(), ContractErrors::TaskNotFound);

        if task.creator != caller && !is_admin {
            self.env().revert(ContractErrors::NotTaskCreator);
        }

        if task.status != TaskStatus::InProgress {
            self.env().revert(ContractErrors::TaskNotAssigned);
        }

        if task.result_hash.is_empty() {
            self.env().revert(ContractErrors::TaskNotSubmitted);
        }

        task.status = TaskStatus::Disputed;
        self.tasks.set(&key, task);

        self.env().emit_event(TaskDisputed {
            task_id,
            creator,
            disputer: caller,
        });
    }

    pub fn claim_payment(&mut self, creator: Address, task_id: String) {
        let caller = self.env().caller();
        let current_time = self.env().get_block_time();

        let key = (creator, task_id.clone());
        let mut task = self
            .tasks
            .get(&key)
            .unwrap_or_revert_with(&self.env(), ContractErrors::TaskNotFound);

        if task.assigned_agent != Some(caller) {
            self.env().revert(ContractErrors::NotAssignedAgent);
        }

        if task.status != TaskStatus::InProgress {
            self.env().revert(ContractErrors::TaskNotAssigned);
        }

        if task.result_hash.is_empty() {
            self.env().revert(ContractErrors::TaskNotSubmitted);
        }

        if current_time < task.deadline + CLAIM_GRACE_PERIOD {
            self.env().revert(ContractErrors::ClaimTooEarly);
        }

        let agent = task.assigned_agent.unwrap();
        let budget = task.budget;
        let fee_rate = self.fee_bps.get().unwrap_or(DEFAULT_FEE_BPS);
        let fee = budget * U512::from(fee_rate) / U512::from(10_000u32);
        let payout = budget - fee;

        task.status = TaskStatus::Completed;
        self.tasks.set(&key, task);

        let mut agent_profile = self
            .agents
            .get(&agent)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);
        agent_profile.active_jobs = agent_profile
            .active_jobs
            .checked_sub(1)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        self.agents.set(&agent, agent_profile);

        self.env().transfer_tokens(&agent, &payout);
        if fee > U512::zero() {
            if let Some(admin) = self.admin.get().flatten() {
                self.env().transfer_tokens(&admin, &fee);
            }
        }

        self.env().emit_event(PaymentClaimed {
            task_id: task_id.clone(),
            creator,
            agent,
            amount: payout,
        });
        self.env().emit_event(FeeDeducted {
            task_id,
            agent,
            fee,
            payout,
        });
    }

    pub fn get_agent(&self, agent: Address) -> Option<AgentProfile> {
        self.agents.get(&agent)
    }

    pub fn get_task(&self, creator: Address, task_id: String) -> Option<Task> {
        self.tasks.get(&(creator, task_id))
    }

    pub fn get_reputation(&self, agent: Address, skill: String) -> ReputationState {
        self.reputations.get_or_default(&(agent, skill))
    }

    pub fn get_validator(&self, validator: Address) -> Option<ValidatorProfile> {
        self.validators.get(&validator)
    }

    pub fn get_stake(&self, agent: Address) -> StakeInfo {
        self.stakes.get_or_default(&agent)
    }

    pub fn set_price(&mut self, price: U512) {
        let caller = self.env().caller();
        let mut profile = self
            .agents
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);
        profile.custom_price = price;
        self.agents.set(&caller, profile);
        self.env().emit_event(PriceUpdated {
            agent: caller,
            custom_price: price,
        });
    }

    pub fn update_recommended_price(&mut self, agent: Address, price: U512) {
        self.assert_admin();
        let mut profile = self
            .agents
            .get(&agent)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);
        profile.recommended_price = price;
        self.agents.set(&agent, profile);
        self.env().emit_event(RecommendedPriceUpdated {
            agent,
            recommended_price: price,
        });
    }

    // ── Staking ────────────────────────────────────────────────

    #[odra(payable)]
    pub fn stake(&mut self) {
        let caller = self.env().caller();
        let attached = self.env().attached_value();

        // Agent must be registered
        self.agents
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);

        let mut info = self.stakes.get_or_default(&caller);
        info.amount = info
            .amount
            .checked_add(attached)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        self.stakes.set(&caller, info.clone());

        self.env().emit_event(Staked {
            agent: caller,
            amount: attached,
            total_stake: info.amount,
        });
    }

    pub fn request_unstake(&mut self, amount: U512) {
        let caller = self.env().caller();
        let current_time = self.env().get_block_time();

        let mut info = self
            .stakes
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::StakeNotFound);

        // Cannot unstake while already unbonding
        if !info.unbonding_amount.is_zero() {
            self.env().revert(ContractErrors::UnbondingInProgress);
        }

        // Cannot unstake with active jobs
        if let Some(profile) = self.agents.get(&caller) {
            if profile.active_jobs > 0 {
                self.env().revert(ContractErrors::ActiveJobsExist);
            }
        }

        // Amount must be valid
        if amount.is_zero() || amount > info.amount {
            self.env().revert(ContractErrors::InvalidUnstakeAmount);
        }

        // Remaining stake must be >= MINIMUM_STAKE or amount == full stake (exit)
        let remaining = info.amount - amount;
        if !remaining.is_zero() && remaining < U512::from(MINIMUM_STAKE) {
            self.env().revert(ContractErrors::InvalidUnstakeAmount);
        }

        info.unbonding_amount = amount;
        info.unbonding_start = current_time;
        self.stakes.set(&caller, info);

        // If full unstake, mark agent unavailable
        if remaining.is_zero() {
            if let Some(mut profile) = self.agents.get(&caller) {
                profile.is_available = false;
                self.agents.set(&caller, profile);
                self.env().emit_event(AgentAvailabilityChanged {
                    agent: caller,
                    available: false,
                });
            }
        }

        let available_at = current_time + UNBONDING_PERIOD;
        self.env().emit_event(UnstakeRequested {
            agent: caller,
            amount,
            available_at,
        });
    }

    pub fn withdraw_stake(&mut self) {
        let caller = self.env().caller();
        let current_time = self.env().get_block_time();

        let mut info = self
            .stakes
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::StakeNotFound);

        if info.unbonding_amount.is_zero() {
            self.env().revert(ContractErrors::NoUnbondingInProgress);
        }

        if current_time < info.unbonding_start + UNBONDING_PERIOD {
            self.env().revert(ContractErrors::UnbondingNotReady);
        }

        let withdraw_amount = info.unbonding_amount;
        info.amount = info.amount - withdraw_amount;
        info.unbonding_amount = U512::zero();
        info.unbonding_start = 0;
        self.stakes.set(&caller, info);

        self.env().transfer_tokens(&caller, &withdraw_amount);

        self.env().emit_event(StakeWithdrawn {
            agent: caller,
            amount: withdraw_amount,
        });
    }

    pub fn cancel_unstake(&mut self) {
        let caller = self.env().caller();

        let mut info = self
            .stakes
            .get(&caller)
            .unwrap_or_revert_with(&self.env(), ContractErrors::StakeNotFound);

        if info.unbonding_amount.is_zero() {
            self.env().revert(ContractErrors::NoUnbondingInProgress);
        }

        let cancelled_amount = info.unbonding_amount;
        info.unbonding_amount = U512::zero();
        info.unbonding_start = 0;
        self.stakes.set(&caller, info);

        self.env().emit_event(UnstakeCancelled {
            agent: caller,
            amount: cancelled_amount,
        });
    }

    pub fn slash_agent(&mut self, agent: Address, bps: u32) {
        self.assert_admin();

        if bps == 0 || bps > MAX_FEE_BPS {
            self.env().revert(ContractErrors::InvalidSlashRate);
        }

        self.apply_slash(agent, bps);
    }

    pub fn get_total_slashed(&self) -> U512 {
        self.total_slashed.get().unwrap_or(U512::zero())
    }

    // ── Internal ──────────────────────────────────────────────

    fn apply_slash(&mut self, agent: Address, bps: u32) {
        let mut info = self.stakes.get_or_default(&agent);
        if info.amount.is_zero() {
            return;
        }

        let slash = info.amount * U512::from(bps) / U512::from(10_000u32);
        if slash.is_zero() {
            return;
        }

        info.amount = info.amount - slash;

        // Also reduce unbonding_amount if needed (can't withdraw more than remaining)
        if info.unbonding_amount > info.amount {
            info.unbonding_amount = info.amount;
        }

        self.stakes.set(&agent, info.clone());

        // Add slashed amount to treasury
        let mut treasury = self.treasury_balance.get().unwrap_or(U512::zero());
        treasury += slash;
        self.treasury_balance.set(treasury);

        let mut total = self.total_slashed.get().unwrap_or(U512::zero());
        total = total
            .checked_add(slash)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        self.total_slashed.set(total);

        self.env().emit_event(SlashApplied {
            agent,
            amount: slash,
            remaining_stake: info.amount,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostRef, HostRefLoader};

    const STAKE_AMOUNT: u64 = 60_000_000_000u64; // 60 CSPR (above 50 CSPR minimum)

    fn setup() -> (odra::host::HostEnv, AgentNetworkHostRef, Address, Address) {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let agent = env.get_account(1);

        env.set_caller(admin);
        let contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin: admin });

        (env, contract, admin, agent)
    }

    /// Register agent and stake the minimum. Call with agent as caller.
    fn register_and_stake(
        env: &odra::host::HostEnv,
        contract: &mut AgentNetworkHostRef,
        agent: Address,
    ) {
        env.set_caller(agent);
        contract.register_agent(
            "Agent_1".to_string(),
            "Generic Agent".to_string(),
            "https://meta".to_string(),
        );
        contract
            .with_tokens(U512::from(STAKE_AMOUNT))
            .stake();
    }

    fn complete_task_as_validator(
        env: &odra::host::HostEnv,
        contract: &mut AgentNetworkHostRef,
        creator: Address,
        task_id: String,
        skill: String,
        score: u32,
        weight: u32,
    ) {
        let validator = env.get_account(9);
        env.set_caller(validator);
        if contract.get_validator(validator).is_none() {
            contract.with_tokens(U512::from(100_000_000_000u64)).register_validator(); // MINIMUM_VALIDATOR_STAKE
        }
        contract.submit_validation(creator, task_id.clone(), score);
        contract.finalize_task(creator, task_id, skill, weight);
        // Reset caller to admin to not break subsequent test flow
        env.set_caller(env.get_account(0));
    }

    #[test]
    fn it_registers_agents() {
        let (env, mut contract, _admin, agent_user) = setup();

        env.set_caller(agent_user);
        contract.register_agent(
            "Agent_DeFi_1".to_string(),
            "DeFi Analyzer Agent".to_string(),
            "https://metadata.uri".to_string(),
        );

        let profile = contract.get_agent(agent_user).unwrap();
        assert_eq!(profile.name, "Agent_DeFi_1");
        assert_eq!(profile.active_jobs, 0);
    }

    #[test]
    fn it_updates_agent_profile() {
        let (env, mut contract, _admin, agent_user) = setup();

        env.set_caller(agent_user);
        contract.register_agent(
            "Agent_1".to_string(),
            "Original".to_string(),
            "https://meta".to_string(),
        );

        contract.update_agent(
            "Agent_2".to_string(),
            "Updated description".to_string(),
            "https://new-meta".to_string(),
        );

        let profile = contract.get_agent(agent_user).unwrap();
        assert_eq!(profile.name, "Agent_2");
        assert_eq!(profile.description, "Updated description");
        assert_eq!(profile.metadata_uri, "https://new-meta");
    }

    #[test]
    fn it_rejects_deadline_in_past() {
        let (env, contract, admin, _agent) = setup();
        let budget = U512::from(5_000_000_000u64);

        env.set_caller(admin);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.with_tokens(budget).create_task(
                "past_task".to_string(), "https://meta".to_string(), 0, None);
        }));
        assert!(result.is_err(), "Should reject deadline=0");
    }

    #[test]
    fn it_handles_task_lifecycle() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task(
            "task_01".to_string(), "https://task_meta".to_string(), deadline, None);

        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(task.budget, budget);

        contract.assign_task("task_01".to_string(), agent);
        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);

        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 1);

        env.set_caller(agent);
        contract.submit_result(admin, "task_01".to_string(), "ipfs_hash".to_string());
        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert_eq!(task.result_hash, "ipfs_hash");

        let agent_balance_before = env.balance_of(&agent);
        env.set_caller(admin);
        complete_task_as_validator(&env, &mut contract, admin, "task_01".to_string(), "DeFi".to_string(), 90, 10);

        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);

        let rep = contract.get_reputation(agent, "DeFi".to_string());
        assert_eq!(rep.weighted_sum / rep.total_weight, 90);

        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 0);

        let agent_balance_after = env.balance_of(&agent);
        let expected_fee = budget * U512::from(500u32) / U512::from(10_000u32);
        assert_eq!(agent_balance_after, agent_balance_before + budget - expected_fee);
    }

    #[test]
    fn it_cancels_open_tasks() {
        let (env, mut contract, admin, _agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(admin);
        contract.with_tokens(budget).create_task(
            "task_01".to_string(), "https://meta".to_string(), deadline, None);

        let balance_before = env.balance_of(&admin);
        contract.cancel_task("task_01".to_string());

        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Cancelled));

        let balance_after = env.balance_of(&admin);
        assert_eq!(balance_after, balance_before + budget);
    }

    #[test]
    fn it_cancels_expired_in_progress_tasks() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("task_01".to_string(), agent);

        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.cancel_task("task_01".to_string());
        }));
        assert!(result.is_err(), "Should not cancel in-progress before deadline");

        env.advance_block_time(3_600_001);

        let balance_before = env.balance_of(&admin);
        contract.cancel_task("task_01".to_string());

        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Cancelled));

        let balance_after = env.balance_of(&admin);
        let slash = U512::from(STAKE_AMOUNT) * U512::from(SLASH_DEADLINE_BPS) / U512::from(10_000u32);
        assert_eq!(balance_after, balance_before + budget); // admin only gets budget back
        assert_eq!(contract.get_total_slashed(), slash); // slash goes to treasury

        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 0);
    }

    #[test]
    fn it_prevents_unauthorized_submit_validation() {
        let (env, mut contract, admin, agent) = setup();
        let non_admin = env.get_account(2);
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("task_01".to_string(), agent);

        env.set_caller(agent);
        contract.submit_result(admin, "task_01".to_string(), "hash".to_string());

        env.set_caller(non_admin);
        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.submit_validation(admin, "task_01".to_string(), 90);
        }));
        assert!(result.is_err(), "Non-validator must not submit validation");
    }

    #[test]
    fn it_calculates_weighted_reputation() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("t1".to_string(), agent);
        env.set_caller(agent);
        contract.submit_result(admin, "t1".to_string(), "hash".to_string());
        env.set_caller(admin);
        complete_task_as_validator(&env, &mut contract, admin, "t1".to_string(), "DeFi".to_string(), 90, 2);

        let rep = contract.get_reputation(agent, "DeFi".to_string());
        assert_eq!(rep.weighted_sum / rep.total_weight, 90);

        contract.with_tokens(budget).create_task("t2".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("t2".to_string(), agent);
        env.set_caller(agent);
        contract.submit_result(admin, "t2".to_string(), "hash".to_string());
        env.set_caller(admin);
        complete_task_as_validator(&env, &mut contract, admin, "t2".to_string(), "DeFi".to_string(), 85, 5);

        let rep = contract.get_reputation(agent, "DeFi".to_string());
        assert_eq!(rep.weighted_sum, 605);
        assert_eq!(rep.total_weight, 7);
        assert_eq!(rep.weighted_sum / rep.total_weight, 86);
        assert_eq!(rep.tasks_completed, 2);
    }

    #[test]
    fn it_exposes_cep96_metadata() {
        let (_env, contract, _admin, _agent) = setup();

        assert_eq!(contract.contract_name(), Some("Casper Agent Network".to_string()));
        assert_eq!(
            contract.contract_description(),
            Some("A decentralized reputation protocol and task marketplace for AI agents on the Casper Network.".to_string())
        );
        assert_eq!(contract.contract_icon_uri(), Some("https://agent-network.casper.dev/icon.png".to_string()));
        assert_eq!(contract.contract_project_uri(), Some("https://agent-network.casper.dev".to_string()));
    }

    #[test]
    fn it_updates_metadata() {
        let (env, mut contract, _admin, _agent) = setup();

        env.set_caller(env.get_account(0));
        contract.update_metadata(
            Some("New Name".to_string()),
            None,
            Some("https://new-icon.png".to_string()),
            None,
        );

        assert_eq!(contract.contract_name(), Some("New Name".to_string()));
        assert_eq!(contract.contract_icon_uri(), Some("https://new-icon.png".to_string()));
        assert_eq!(
            contract.contract_description(),
            Some("A decentralized reputation protocol and task marketplace for AI agents on the Casper Network.".to_string())
        );
    }

    #[test]
    fn it_transfers_ownership_two_step() {
        let (env, mut contract, admin, _agent) = setup();
        let new_admin = env.get_account(3);

        env.set_caller(admin);
        contract.transfer_ownership(&new_admin);
        assert_eq!(contract.get_pending_owner(), Some(new_admin));
        assert_eq!(contract.get_admin(), Some(admin));

        env.set_caller(new_admin);
        contract.accept_ownership();
        assert_eq!(contract.get_admin(), Some(new_admin));
        assert_eq!(contract.get_pending_owner(), None);
    }

    #[test]
    fn it_namespaces_tasks_by_creator() {
        let (env, mut contract, _admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());

        env.set_caller(_admin);
        contract.with_tokens(budget).create_task("shared_id".to_string(), "https://meta".to_string(), deadline, None);

        env.set_caller(agent);
        contract.with_tokens(budget).create_task("shared_id".to_string(), "https://meta".to_string(), deadline, None);

        let task_a = contract.get_task(_admin, "shared_id".to_string()).unwrap();
        let task_b = contract.get_task(agent, "shared_id".to_string()).unwrap();
        assert_eq!(task_a.creator, _admin);
        assert_eq!(task_b.creator, agent);
    }

    #[test]
    fn it_disputes_and_resolves_tasks() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("task_01".to_string(), agent);

        env.set_caller(agent);
        contract.submit_result(admin, "task_01".to_string(), "hash".to_string());

        env.set_caller(admin);
        contract.dispute_task(admin, "task_01".to_string());
        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Disputed));

        complete_task_as_validator(&env, &mut contract, admin, "task_01".to_string(), "DeFi".to_string(), 75, 5);
        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Completed));
    }

    #[test]
    fn it_claims_payment_after_grace() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("task_01".to_string(), agent);

        env.set_caller(agent);
        contract.submit_result(admin, "task_01".to_string(), "hash".to_string());

        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.claim_payment(admin, "task_01".to_string());
        }));
        assert!(result.is_err(), "Should not claim before grace period");

        env.advance_block_time(3_600_000 + CLAIM_GRACE_PERIOD + 1);

        let balance_before = env.balance_of(&agent);
        contract.claim_payment(admin, "task_01".to_string());

        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Completed));

        let balance_after = env.balance_of(&agent);
        let expected_fee = budget * U512::from(500u32) / U512::from(10_000u32);
        assert_eq!(balance_after, balance_before + budget - expected_fee);
    }

    #[test]
    fn it_rejects_duplicate_result_submission() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("task_01".to_string(), agent);

        env.set_caller(agent);
        contract.submit_result(admin, "task_01".to_string(), "hash1".to_string());

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.submit_result(admin, "task_01".to_string(), "hash2".to_string());
        }));
        assert!(result.is_err(), "Should not allow duplicate submission");
    }

    #[test]
    fn it_handles_pricing() {
        let (env, mut contract, admin, agent) = setup();
        let non_admin = env.get_account(2);

        env.set_caller(agent);
        contract.register_agent("PricedAgent".to_string(), "Agent with pricing".to_string(), "https://meta".to_string());

        let custom_price = U512::from(3_000_000_000u64);
        contract.set_price(custom_price);
        let profile = contract.get_agent(agent).unwrap();
        assert_eq!(profile.custom_price, custom_price);

        env.set_caller(admin);
        let rec_price = U512::from(5_000_000_000u64);
        contract.update_recommended_price(agent, rec_price);
        let profile = contract.get_agent(agent).unwrap();
        assert_eq!(profile.recommended_price, rec_price);

        env.set_caller(non_admin);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract.address());
            c.update_recommended_price(agent, U512::from(1u64));
        }));
        assert!(result.is_err(), "Non-admin should not set recommended price");
    }

    #[test]
    fn it_applies_reputation_based_fee() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(10_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        assert_eq!(contract.get_effective_fee_rate(agent, "DeFi".to_string()), 500);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("t1".to_string(), agent);
        env.set_caller(agent);
        contract.submit_result(admin, "t1".to_string(), "hash".to_string());

        let agent_before = env.balance_of(&agent);
        env.set_caller(admin);
        complete_task_as_validator(&env, &mut contract, admin, "t1".to_string(), "DeFi".to_string(), 95, 10);

        let fee = budget * U512::from(500u32) / U512::from(10_000u32);
        let payout = budget - fee;
        assert_eq!(env.balance_of(&agent), agent_before + payout);

        assert_eq!(contract.get_effective_fee_rate(agent, "DeFi".to_string()), 100);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t2".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("t2".to_string(), agent);
        env.set_caller(agent);
        contract.submit_result(admin, "t2".to_string(), "hash".to_string());

        let agent_before = env.balance_of(&agent);
        env.set_caller(admin);
        complete_task_as_validator(&env, &mut contract, admin, "t2".to_string(), "DeFi".to_string(), 95, 10);

        let fee = budget * U512::from(100u32) / U512::from(10_000u32);
        let payout = budget - fee;
        assert_eq!(env.balance_of(&agent), agent_before + payout);
    }

    #[test]
    fn it_toggles_agent_availability() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);
        assert!(contract.get_agent(agent).unwrap().is_available);

        env.set_caller(agent);
        contract.set_availability(false);
        assert!(!contract.get_agent(agent).unwrap().is_available);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.assign_task("t1".to_string(), agent);
        }));
        assert!(result.is_err(), "Should not assign to unavailable agent");

        env.set_caller(agent);
        contract.set_availability(true);

        env.set_caller(admin);
        contract.assign_task("t1".to_string(), agent);
        let task = contract.get_task(admin, "t1".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
    }

    #[test]
    fn it_increases_task_budget() {
        let (env, contract, admin, _agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let extra = U512::from(3_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);

        let task = contract.get_task(admin, "t1".to_string()).unwrap();
        assert_eq!(task.budget, budget);

        contract.with_tokens(extra).increase_budget("t1".to_string());

        let task = contract.get_task(admin, "t1".to_string()).unwrap();
        assert_eq!(task.budget, budget + extra);
    }

    // ── Staking & Slashing Tests ────────────────────────────────

    #[test]
    fn it_stakes_and_reads_stake() {
        let (env, mut contract, _admin, agent) = setup();
        let stake = U512::from(STAKE_AMOUNT);

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());

        let info = contract.get_stake(agent);
        assert_eq!(info.amount, U512::zero());

        contract.with_tokens(stake).stake();

        let info = contract.get_stake(agent);
        assert_eq!(info.amount, stake);
        assert_eq!(info.unbonding_amount, U512::zero());
        assert_eq!(info.unbonding_start, 0);

        // Stake more
        let extra = U512::from(10_000_000_000u64);
        contract.with_tokens(extra).stake();
        let info = contract.get_stake(agent);
        assert_eq!(info.amount, stake + extra);
    }

    #[test]
    fn it_requires_minimum_stake_for_assignment() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());
        // No stake — should fail

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.assign_task("t1".to_string(), agent);
        }));
        assert!(result.is_err(), "Should not assign without minimum stake");

        // Stake below minimum
        env.set_caller(agent);
        contract.with_tokens(U512::from(10_000_000_000u64)).stake(); // 10 CSPR < 50 CSPR

        env.set_caller(admin);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.assign_task("t1".to_string(), agent);
        }));
        assert!(result.is_err(), "Should not assign with insufficient stake");

        // Stake above minimum
        env.set_caller(agent);
        contract.with_tokens(U512::from(50_000_000_000u64)).stake(); // total 60 CSPR

        env.set_caller(admin);
        contract.assign_task("t1".to_string(), agent);
        let task = contract.get_task(admin, "t1".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
    }

    #[test]
    fn it_slashes_on_low_score() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        let stake_before = contract.get_stake(agent).amount;

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("t1".to_string(), agent);

        env.set_caller(agent);
        contract.submit_result(admin, "t1".to_string(), "hash".to_string());

        env.set_caller(admin);
        complete_task_as_validator(&env, &mut contract, admin, "t1".to_string(), "DeFi".to_string(), 20, 10); // score < 30

        let stake_after = contract.get_stake(agent).amount;
        let expected_slash = stake_before * U512::from(SLASH_LOW_SCORE_BPS) / U512::from(10_000u32);
        assert_eq!(stake_after, stake_before - expected_slash);
        assert!(contract.get_total_slashed() > U512::zero());
    }

    #[test]
    fn it_slashes_on_deadline_miss() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        let stake_before = contract.get_stake(agent).amount;

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("t1".to_string(), agent);

        // Agent doesn't submit, deadline passes
        env.advance_block_time(3_600_001);

        env.set_caller(admin);
        contract.cancel_task("t1".to_string());

        let stake_after = contract.get_stake(agent).amount;
        let expected_slash = stake_before * U512::from(SLASH_DEADLINE_BPS) / U512::from(10_000u32);
        assert_eq!(stake_after, stake_before - expected_slash);
    }

    #[test]
    fn it_slashes_on_dispute_cancel() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        let stake_before = contract.get_stake(agent).amount;

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("t1".to_string(), agent);

        env.set_caller(agent);
        contract.submit_result(admin, "t1".to_string(), "bad_hash".to_string());

        // Admin disputes
        env.set_caller(admin);
        contract.dispute_task(admin, "t1".to_string());

        // Admin cancels disputed task → 20% slash
        contract.cancel_task("t1".to_string());

        let stake_after = contract.get_stake(agent).amount;
        let expected_slash = stake_before * U512::from(SLASH_DISPUTE_BPS) / U512::from(10_000u32);
        assert_eq!(stake_after, stake_before - expected_slash);
    }

    #[test]
    fn it_handles_unstake_lifecycle() {
        let (env, mut contract, _admin, agent) = setup();

        register_and_stake(&env, &mut contract, agent);
        let stake = contract.get_stake(agent).amount;

        // Request full unstake
        env.set_caller(agent);
        contract.request_unstake(stake);

        let info = contract.get_stake(agent);
        assert_eq!(info.unbonding_amount, stake);
        assert!(info.unbonding_start >= 0); // block_time may be 0 in tests

        // Agent should be marked unavailable on full unstake
        assert!(!contract.get_agent(agent).unwrap().is_available);

        // Cannot withdraw before unbonding period
        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.withdraw_stake();
        }));
        assert!(result.is_err(), "Should not withdraw before unbonding period");

        // Advance past unbonding period (30 min)
        env.advance_block_time(UNBONDING_PERIOD + 1);

        let balance_before = env.balance_of(&agent);
        env.set_caller(agent);
        contract.withdraw_stake();

        let balance_after = env.balance_of(&agent);
        assert_eq!(balance_after, balance_before + stake);

        let info = contract.get_stake(agent);
        assert_eq!(info.amount, U512::zero());
        assert_eq!(info.unbonding_amount, U512::zero());
        assert_eq!(info.unbonding_start, 0);
    }

    #[test]
    fn it_prevents_unstake_with_active_jobs() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);
        contract.assign_task("t1".to_string(), agent);

        // Agent has active job — cannot unstake
        env.set_caller(agent);
        let stake = contract.get_stake(agent).amount;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.request_unstake(stake);
        }));
        assert!(result.is_err(), "Should not unstake with active jobs");
    }

    #[test]
    fn it_prevents_assignment_during_unbonding() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        // Request partial unstake (keep above minimum)
        env.set_caller(agent);
        let stake = contract.get_stake(agent).amount;
        let partial = stake - U512::from(MINIMUM_STAKE); // unstake just enough to stay above min
        if partial > U512::zero() {
            contract.request_unstake(partial);
        } else {
            // If stake == minimum, request full unstake
            contract.request_unstake(stake);
        }

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline, None);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.assign_task("t1".to_string(), agent);
        }));
        assert!(result.is_err(), "Should not assign to unbonding agent");
    }

    #[test]
    fn it_cancels_unstake() {
        let (env, mut contract, _admin, agent) = setup();

        register_and_stake(&env, &mut contract, agent);
        let stake = contract.get_stake(agent).amount;

        env.set_caller(agent);
        contract.request_unstake(stake);

        let info = contract.get_stake(agent);
        assert!(!info.unbonding_amount.is_zero());

        contract.cancel_unstake();

        let info = contract.get_stake(agent);
        assert_eq!(info.unbonding_start, 0);
        assert_eq!(info.unbonding_amount, U512::zero());
        assert_eq!(info.amount, stake); // Stake unchanged
    }

    #[test]
    fn it_admin_slashes_explicitly() {
        let (env, mut contract, admin, agent) = setup();

        register_and_stake(&env, &mut contract, agent);
        let stake_before = contract.get_stake(agent).amount;

        env.set_caller(admin);
        contract.slash_agent(agent, 1500); // 15%

        let stake_after = contract.get_stake(agent).amount;
        let expected_slash = stake_before * U512::from(1500u32) / U512::from(10_000u32);
        assert_eq!(stake_after, stake_before - expected_slash);
        assert_eq!(contract.get_total_slashed(), expected_slash);

        // Non-admin cannot slash
        let non_admin = env.get_account(2);
        env.set_caller(non_admin);
        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.slash_agent(agent, 500);
        }));
        assert!(result.is_err(), "Non-admin should not slash");
    }

    #[test]
    fn it_calculates_median_and_slashes_validators() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(10_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        register_and_stake(&env, &mut contract, agent);

        // Create task
        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t_yuma".to_string(), "meta".to_string(), deadline, None);
        contract.assign_task("t_yuma".to_string(), agent);

        // Submit result
        env.set_caller(agent);
        contract.submit_result(admin, "t_yuma".to_string(), "hash".to_string());

        // Setup 3 validators
        let v1 = env.get_account(3); // Score 90
        let v2 = env.get_account(4); // Score 95
        let v3 = env.get_account(5); // Score 60 (outlier)

        let val_stake = U512::from(100_000_000_000u64);

        env.set_caller(v1);
        contract.with_tokens(val_stake).register_validator();
        contract.submit_validation(admin, "t_yuma".to_string(), 90);

        env.set_caller(v2);
        contract.with_tokens(val_stake).register_validator();
        contract.submit_validation(admin, "t_yuma".to_string(), 95);

        env.set_caller(v3);
        contract.with_tokens(val_stake).register_validator();
        contract.submit_validation(admin, "t_yuma".to_string(), 60);

        let v1_balance_before = env.balance_of(&v1);

        // Finalize task (median of 60, 90, 95 is 90)
        env.set_caller(admin);
        contract.finalize_task(admin, "t_yuma".to_string(), "General".to_string(), 10);

        // Check scores and slashes
        let rep = contract.get_reputation(agent, "General".to_string());
        assert_eq!(rep.weighted_sum / rep.total_weight, 90);

        // V3 deviation is 30 points (90 - 60). Tolerance is 10. Diff is 20.
        // Penalty factor = (20 / 10) * 500 bps = 1000 bps (10%)
        let v3_profile = contract.get_validator(v3).unwrap();
        assert_eq!(v3_profile.stake, val_stake * U512::from(90u32) / U512::from(100u32)); // 90% left

        // V1 should have received rewards in their account balance
        let v1_balance_after = env.balance_of(&v1);
        assert!(v1_balance_after > v1_balance_before);
    }

    #[test]
    fn it_syncs_decayed_reputation() {
        let (env, mut contract, admin, agent) = setup();
        
        env.set_caller(admin);
        contract.sync_decayed_reputation(agent, "General".to_string(), 450, 5);
        
        let rep = contract.get_reputation(agent, "General".to_string());
        assert_eq!(rep.weighted_sum, 450);
        assert_eq!(rep.total_weight, 5);
        assert_eq!(rep.tasks_completed, 0); // shouldn't change
        
        // Ensure non-admin can't sync
        let non_admin = env.get_account(2);
        env.set_caller(non_admin);
        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.sync_decayed_reputation(agent, "General".to_string(), 400, 4);
        }));
        assert!(result.is_err());
    }
}
