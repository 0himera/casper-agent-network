use odra::casper_types::U512;
use odra::prelude::*;

const MINIMUM_BUDGET: u64 = 1_000_000_000u64; // 1 CSPR
const MAX_TASK_ID_LEN: usize = 128;
const CLAIM_GRACE_PERIOD: u64 = 86_400_000; // 24h in ms

#[odra::odra_type]
pub struct AgentProfile {
    pub name: String,
    pub description: String,
    pub metadata_uri: String,
    pub active_jobs: u32,
    pub custom_price: U512,
    pub recommended_price: U512,
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
}

#[odra::odra_type]
#[derive(Default)]
pub struct ReputationState {
    pub weighted_sum: u64,
    pub total_weight: u64,
    pub tasks_completed: u32,
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
pub struct ScoreUpdated {
    pub agent: Address,
    pub skill: String,
    pub new_score: u32,
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
        OwnershipTransferred
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
    pub fn create_task(&mut self, task_id: String, metadata_uri: String, deadline: u64) {
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
        };

        self.tasks.set(&key, task);
        self.env().emit_event(TaskCreated {
            task_id,
            creator: caller,
            budget: attached_value,
            deadline,
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

    pub fn complete_task(
        &mut self,
        creator: Address,
        task_id: String,
        skill: String,
        score: u32,
        weight: u32,
    ) {
        self.assert_admin();

        if score > 100 {
            self.env().revert(ContractErrors::InvalidScore);
        }
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

        if task.result_hash.is_empty() {
            self.env().revert(ContractErrors::TaskNotSubmitted);
        }

        let agent = task.assigned_agent.unwrap();
        let mut agent_profile = self
            .agents
            .get(&agent)
            .unwrap_or_revert_with(&self.env(), ContractErrors::AgentNotFound);

        let budget = task.budget;

        task.status = TaskStatus::Completed;
        self.tasks.set(&key, task);

        agent_profile.active_jobs = agent_profile
            .active_jobs
            .checked_sub(1)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        self.agents.set(&agent, agent_profile);

        self.env().transfer_tokens(&agent, &budget);

        let mut rep_state = self.reputations.get_or_default(&(agent, skill.clone()));
        rep_state.weighted_sum = rep_state
            .weighted_sum
            .checked_add((score as u64) * (weight as u64))
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        rep_state.total_weight = rep_state
            .total_weight
            .checked_add(weight as u64)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);
        rep_state.tasks_completed = rep_state
            .tasks_completed
            .checked_add(1)
            .unwrap_or_revert_with(&self.env(), ContractErrors::ArithmeticOverflow);

        let new_score = if rep_state.total_weight == 0 {
            0
        } else {
            (rep_state.weighted_sum / rep_state.total_weight) as u32
        };
        self.reputations.set(&(agent, skill.clone()), rep_state);

        self.env().emit_event(TaskCompleted { task_id, score });
        self.env().emit_event(ScoreUpdated {
            agent,
            skill,
            new_score,
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

        self.env().transfer_tokens(&agent, &budget);

        self.env().emit_event(PaymentClaimed {
            task_id,
            creator,
            agent,
            amount: budget,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostRef, HostRefLoader};

    fn setup() -> (odra::host::HostEnv, AgentNetworkHostRef, Address, Address) {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let agent = env.get_account(1);

        env.set_caller(admin);
        let contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin: admin });

        (env, contract, admin, agent)
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
                "past_task".to_string(),
                "https://meta".to_string(),
                0,
            );
        }));
        assert!(result.is_err(), "Should reject deadline=0");
    }

    #[test]
    fn it_handles_task_lifecycle() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(agent);
        contract.register_agent(
            "Agent_1".to_string(),
            "Generic Agent".to_string(),
            "https://meta".to_string(),
        );

        env.set_caller(admin);
        contract.with_tokens(budget).create_task(
            "task_01".to_string(),
            "https://task_meta".to_string(),
            deadline,
        );

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
        contract.complete_task(admin, "task_01".to_string(), "DeFi".to_string(), 90, 10);

        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);

        let rep = contract.get_reputation(agent, "DeFi".to_string());
        assert_eq!(rep.weighted_sum / rep.total_weight, 90);

        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 0);

        let agent_balance_after = env.balance_of(&agent);
        assert_eq!(agent_balance_after, agent_balance_before + budget);
    }

    #[test]
    fn it_cancels_open_tasks() {
        let (env, mut contract, admin, _agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(admin);
        contract.with_tokens(budget).create_task(
            "task_01".to_string(),
            "https://meta".to_string(),
            deadline,
        );

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

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline);
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
        assert_eq!(balance_after, balance_before + budget);

        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 0);
    }

    #[test]
    fn it_prevents_unauthorized_complete_task() {
        let (env, mut contract, admin, agent) = setup();
        let non_admin = env.get_account(2);
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline);
        contract.assign_task("task_01".to_string(), agent);

        env.set_caller(agent);
        contract.submit_result(admin, "task_01".to_string(), "hash".to_string());

        env.set_caller(non_admin);
        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.complete_task(admin, "task_01".to_string(), "DeFi".to_string(), 90, 10);
        }));
        assert!(result.is_err(), "Non-admin must not complete task");
    }

    #[test]
    fn it_calculates_weighted_reputation() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("t1".to_string(), "https://meta".to_string(), deadline);
        contract.assign_task("t1".to_string(), agent);
        env.set_caller(agent);
        contract.submit_result(admin, "t1".to_string(), "hash".to_string());
        env.set_caller(admin);
        contract.complete_task(admin, "t1".to_string(), "DeFi".to_string(), 90, 2);

        let rep = contract.get_reputation(agent, "DeFi".to_string());
        assert_eq!(rep.weighted_sum / rep.total_weight, 90);

        contract.with_tokens(budget).create_task("t2".to_string(), "https://meta".to_string(), deadline);
        contract.assign_task("t2".to_string(), agent);
        env.set_caller(agent);
        contract.submit_result(admin, "t2".to_string(), "hash".to_string());
        env.set_caller(admin);
        contract.complete_task(admin, "t2".to_string(), "DeFi".to_string(), 85, 5);

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
        contract.with_tokens(budget).create_task("shared_id".to_string(), "https://meta".to_string(), deadline);

        env.set_caller(agent);
        contract.with_tokens(budget).create_task("shared_id".to_string(), "https://meta".to_string(), deadline);

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

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline);
        contract.assign_task("task_01".to_string(), agent);

        env.set_caller(agent);
        contract.submit_result(admin, "task_01".to_string(), "hash".to_string());

        env.set_caller(admin);
        contract.dispute_task(admin, "task_01".to_string());
        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Disputed));

        contract.complete_task(admin, "task_01".to_string(), "DeFi".to_string(), 75, 5);
        let task = contract.get_task(admin, "task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Completed));
    }

    #[test]
    fn it_claims_payment_after_grace() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline);
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
        assert_eq!(balance_after, balance_before + budget);
    }

    #[test]
    fn it_rejects_duplicate_result_submission() {
        let (env, mut contract, admin, agent) = setup();
        let budget = U512::from(5_000_000_000u64);
        let deadline = env.block_time() + 3_600_000;

        env.set_caller(agent);
        contract.register_agent("Agent_1".to_string(), "Generic".to_string(), "https://meta".to_string());

        env.set_caller(admin);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), deadline);
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
}
