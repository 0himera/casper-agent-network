use odra::casper_types::U512;
use odra::prelude::*;

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
}

const MINIMUM_BUDGET: u64 = 1_000_000_000u64; // 1 CSPR

#[odra::module(
    errors = ContractErrors,
    events = [
        AgentRegistered, 
        TaskCreated, 
        TaskAssigned, 
        TaskSubmitted, 
        TaskCompleted, 
        ScoreUpdated,
        PriceUpdated,
        RecommendedPriceUpdated,
        TaskCancelled
    ]
)]
pub struct AgentNetwork {
    admin: Var<Address>,
    agents: Mapping<Address, AgentProfile>,
    tasks: Mapping<String, Task>,
    reputations: Mapping<(Address, String), ReputationState>,
}

#[odra::module]
impl AgentNetwork {
    /// Initialize the contract.
    pub fn init(&mut self, admin: Address) {
        self.admin.set(admin);
    }

    /// Register a new AI agent on the network.
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
        self.env().emit_event(AgentRegistered {
            agent: caller,
            name,
        });
    }

    /// Post a new task and lock CSPR as escrow.
    #[odra(payable)]
    pub fn create_task(&mut self, task_id: String, metadata_uri: String, deadline: u64) {
        let caller = self.env().caller();
        let attached_value = self.env().attached_value();

        if attached_value < U512::from(MINIMUM_BUDGET) {
            self.env().revert(ContractErrors::BelowMinimumBudget);
        }

        if self.tasks.get(&task_id).is_some() {
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

        self.tasks.set(&task_id, task);
        self.env().emit_event(TaskCreated {
            task_id,
            creator: caller,
            budget: attached_value,
            deadline,
        });
    }

    /// Cancel a task and refund escrow.
    pub fn cancel_task(&mut self, task_id: String) {
        let caller = self.env().caller();
        let mut task = self.tasks.get(&task_id).unwrap_or_revert(&self.env());
        
        if task.creator != caller {
            self.env().revert(ContractErrors::NotTaskCreator);
        }

        let current_time = self.env().get_block_time();
        
        let can_cancel = match task.status {
            TaskStatus::Open => true,
            TaskStatus::InProgress => {
                current_time >= task.deadline && task.result_hash.is_empty()
            },
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
        self.tasks.set(&task_id, task);

        if let Some(agent) = assigned_agent {
            if let Some(mut agent_profile) = self.agents.get(&agent) {
                if agent_profile.active_jobs > 0 {
                    agent_profile.active_jobs -= 1;
                    self.agents.set(&agent, agent_profile);
                }
            }
        }

        self.env().transfer_tokens(&caller, &budget);

        self.env().emit_event(TaskCancelled { task_id });
    }

    /// Assign a task to a registered agent. Only the task creator can do this.
    pub fn assign_task(&mut self, task_id: String, agent: Address) {
        let caller = self.env().caller();
        
        let mut task = self.tasks.get(&task_id).unwrap_or_revert(&self.env());
        if task.creator != caller {
            self.env().revert(ContractErrors::NotTaskCreator);
        }

        if task.status != TaskStatus::Open {
            self.env().revert(ContractErrors::TaskNotOpen);
        }

        let mut agent_profile = self.agents.get(&agent).unwrap_or_revert(&self.env());

        task.assigned_agent = Some(agent);
        task.status = TaskStatus::InProgress;
        self.tasks.set(&task_id, task);

        agent_profile.active_jobs += 1;
        self.agents.set(&agent, agent_profile);

        self.env().emit_event(TaskAssigned {
            task_id,
            agent,
        });
    }

    /// Submit execution results. Only the assigned agent or admin can call this.
    pub fn submit_result(&mut self, task_id: String, result_hash: String) {
        let caller = self.env().caller();
        
        let mut task = self.tasks.get(&task_id).unwrap_or_revert(&self.env());
        if task.assigned_agent != Some(caller) && Some(caller) != self.admin.get() {
            self.env().revert(ContractErrors::NotAssignedAgent);
        }

        if task.status != TaskStatus::InProgress {
            self.env().revert(ContractErrors::TaskNotAssigned);
        }

        let assigned_agent = task.assigned_agent.unwrap();
        task.result_hash = result_hash.clone();
        self.tasks.set(&task_id, task);

        self.env().emit_event(TaskSubmitted {
            task_id,
            agent: assigned_agent,
            result_hash,
        });
    }

    /// Complete task execution, release escrow, and update agent's reputation. Only Admin can call this.
    pub fn complete_task(&mut self, task_id: String, skill: String, score: u32, weight: u32) {
        let caller = self.env().caller();
        if Some(caller) != self.admin.get() {
            self.env().revert(ContractErrors::NotContractAdmin);
        }

        if score > 100 {
            self.env().revert(ContractErrors::InvalidScore);
        }

        if weight == 0 {
            self.env().revert(ContractErrors::InvalidWeight);
        }

        let mut task = self.tasks.get(&task_id).unwrap_or_revert(&self.env());

        if task.status != TaskStatus::InProgress {
            self.env().revert(ContractErrors::TaskNotAssigned);
        }

        if task.result_hash.is_empty() {
            self.env().revert(ContractErrors::TaskNotSubmitted);
        }

        let agent = task.assigned_agent.unwrap();
        let mut agent_profile = self.agents.get(&agent).unwrap_or_revert(&self.env());

        let budget = task.budget;

        // Update status and decrease active job count
        task.status = TaskStatus::Completed;
        self.tasks.set(&task_id, task);

        if agent_profile.active_jobs > 0 {
            agent_profile.active_jobs -= 1;
        }
        self.agents.set(&agent, agent_profile);

        // Pay the agent from escrow (transfer attached tokens)
        self.env().transfer_tokens(&agent, &budget);

        // Update reputation score for the skill
        let mut rep_state = self.reputations.get_or_default(&(agent, skill.clone()));
        rep_state.weighted_sum += (score as u64) * (weight as u64);
        rep_state.total_weight += weight as u64;
        rep_state.tasks_completed += 1;

        let new_score = (rep_state.weighted_sum / rep_state.total_weight) as u32;
        self.reputations.set(&(agent, skill.clone()), rep_state);

        // Emit events
        self.env().emit_event(TaskCompleted {
            task_id,
            score,
        });

        self.env().emit_event(ScoreUpdated {
            agent,
            skill,
            new_score,
        });
    }

    /// Get details of an agent.
    pub fn get_agent(&self, agent: Address) -> Option<AgentProfile> {
        self.agents.get(&agent)
    }

    /// Get the contract admin address.
    pub fn get_admin(&self) -> Option<Address> {
        self.admin.get()
    }

    /// Get details of a task.
    pub fn get_task(&self, task_id: String) -> Option<Task> {
        self.tasks.get(&task_id)
    }

    /// Get reputation score of an agent for a specific skill.
    pub fn get_reputation(&self, agent: Address, skill: String) -> u32 {
        let rep_state = self.reputations.get_or_default(&(agent, skill));
        if rep_state.total_weight == 0 {
            0
        } else {
            (rep_state.weighted_sum / rep_state.total_weight) as u32
        }
    }

    /// Set a custom price for the agent.
    pub fn set_price(&mut self, price: U512) {
        let caller = self.env().caller();
        let mut profile = self.agents.get(&caller).unwrap_or_revert(&self.env());
        profile.custom_price = price;
        self.agents.set(&caller, profile);
        self.env().emit_event(PriceUpdated {
            agent: caller,
            custom_price: price,
        });
    }

    /// Update recommended price for an agent. Only the admin can call this.
    pub fn update_recommended_price(&mut self, agent: Address, price: U512) {
        let caller = self.env().caller();
        if Some(caller) != self.admin.get() {
            self.env().revert(ContractErrors::NotContractAdmin);
        }
        let mut profile = self.agents.get(&agent).unwrap_or_revert(&self.env());
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
    use odra::host::{Deployer, HostRef, NoArgs, HostRefLoader};

    #[test]
    fn it_registers_agents() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let agent_user = env.get_account(1);

        env.set_caller(admin);
        let mut contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin });

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
    fn it_handles_task_lifecycle() {
        let env = odra_test::env();
        let client = env.get_account(0); // client is deployer (admin)
        let agent = env.get_account(1);

        env.set_caller(client);
        let mut contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin: client });

        // Register agent
        env.set_caller(agent);
        contract.register_agent(
            "Agent_1".to_string(),
            "Generic Agent".to_string(),
            "https://meta".to_string(),
        );

        // Client creates a task with 5 CSPR budget and 1 hour deadline (3600000 ms)
        env.set_caller(client);
        let budget = U512::from(5_000_000_000u64);
        contract.with_tokens(budget).create_task(
            "task_01".to_string(),
            "https://task_meta".to_string(),
            3600000,
        );

        let task = contract.get_task("task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(task.budget, budget);
        assert_eq!(task.deadline, 3600000);

        // Assign task to agent
        contract.assign_task("task_01".to_string(), agent);
        let task = contract.get_task("task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.assigned_agent, Some(agent));

        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 1);

        // Agent submits result
        env.set_caller(agent);
        contract.submit_result("task_01".to_string(), "ipfs_hash_result".to_string());
        let task = contract.get_task("task_01".to_string()).unwrap();
        assert_eq!(task.result_hash, "ipfs_hash_result");

        // Client (which is contract admin) completes task, rewarding agent with reputation
        let agent_balance_before = env.balance_of(&agent);
        env.set_caller(client);
        contract.complete_task("task_01".to_string(), "DeFi".to_string(), 90, 10);

        let task = contract.get_task("task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);

        // Check reputation and payment
        assert_eq!(contract.get_reputation(agent, "DeFi".to_string()), 90);
        
        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 0);

        let agent_balance_after = env.balance_of(&agent);
        assert_eq!(agent_balance_after, agent_balance_before + budget);
    }

    #[test]
    fn it_handles_pricing() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let agent_user = env.get_account(1);
        let non_admin = env.get_account(2);

        // Deploy as admin
        env.set_caller(admin);
        let mut contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin });

        // Register agent
        env.set_caller(agent_user);
        contract.register_agent(
            "PricedAgent".to_string(),
            "Agent with pricing".to_string(),
            "https://meta".to_string(),
        );

        // Agent sets custom price
        let custom_price = U512::from(3_000_000_000u64); // 3 CSPR
        contract.set_price(custom_price);
        let profile = contract.get_agent(agent_user).unwrap();
        assert_eq!(profile.custom_price, custom_price);
        assert_eq!(profile.recommended_price, U512::zero());

        // Admin sets recommended price
        env.set_caller(admin);
        let rec_price = U512::from(5_000_000_000u64); // 5 CSPR
        contract.update_recommended_price(agent_user, rec_price);
        let profile = contract.get_agent(agent_user).unwrap();
        assert_eq!(profile.recommended_price, rec_price);
        assert_eq!(profile.custom_price, custom_price); // unchanged

        // Non-admin cannot set recommended price
        env.set_caller(non_admin);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.update_recommended_price(agent_user, U512::from(1u64));
        }));
        assert!(result.is_err(), "Non-admin should not be able to set recommended price");
    }

    #[test]
    fn it_cancels_open_tasks() {
        let env = odra_test::env();
        let client = env.get_account(0);

        env.set_caller(client);
        let mut contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin: client });

        let budget = U512::from(5_000_000_000u64);
        contract.with_tokens(budget).create_task(
            "task_01".to_string(),
            "https://task_meta".to_string(),
            3600000,
        );

        let balance_before = env.balance_of(&client);
        
        // Cancel task (as creator)
        contract.cancel_task("task_01".to_string());
        
        let task = contract.get_task("task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Cancelled));

        let balance_after = env.balance_of(&client);
        assert_eq!(balance_after, balance_before + budget);
    }

    #[test]
    fn it_cancels_expired_in_progress_tasks() {
        let env = odra_test::env();
        let client = env.get_account(0);
        let agent = env.get_account(1);

        env.set_caller(client);
        let mut contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin: client });

        // Register agent
        env.set_caller(agent);
        contract.register_agent(
            "Agent_1".to_string(),
            "Generic Agent".to_string(),
            "https://meta".to_string(),
        );

        // Client creates task with budget 5 CSPR and deadline +3600000 (1 hour)
        env.set_caller(client);
        let budget = U512::from(5_000_000_000u64);
        contract.with_tokens(budget).create_task(
            "task_01".to_string(),
            "https://task_meta".to_string(),
            3600000,
        );

        // Assign task to agent
        contract.assign_task("task_01".to_string(), agent);

        // Try to cancel immediately (should fail)
        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.cancel_task("task_01".to_string());
        }));
        assert!(result.is_err(), "Should not be able to cancel in-progress task before deadline");

        // Advance time past deadline
        env.advance_block_time(3600001);

        // Now cancel task
        let balance_before = env.balance_of(&client);
        contract.cancel_task("task_01".to_string());

        let task = contract.get_task("task_01".to_string()).unwrap();
        assert!(matches!(task.status, TaskStatus::Cancelled));

        let balance_after = env.balance_of(&client);
        assert_eq!(balance_after, balance_before + budget);

        // Verify agent active_jobs decremented
        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 0);
    }

    #[test]
    fn it_prevents_unauthorized_complete_task() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let non_admin = env.get_account(1);
        let agent = env.get_account(2);

        env.set_caller(admin);
        let mut contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin });

        // Register agent
        env.set_caller(agent);
        contract.register_agent(
            "Agent_1".to_string(),
            "Generic Agent".to_string(),
            "https://meta".to_string(),
        );

        // Admin creates task
        env.set_caller(admin);
        let budget = U512::from(5_000_000_000u64);
        contract.with_tokens(budget).create_task(
            "task_01".to_string(),
            "https://task_meta".to_string(),
            3600000,
        );
        contract.assign_task("task_01".to_string(), agent);

        // Agent submits
        env.set_caller(agent);
        contract.submit_result("task_01".to_string(), "hash".to_string());

        // Try to complete as non-admin (should fail)
        env.set_caller(non_admin);
        let contract_address = contract.address();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c = AgentNetwork::load(&env, contract_address);
            c.complete_task("task_01".to_string(), "DeFi".to_string(), 90, 10);
        }));
        assert!(result.is_err(), "Non-admin must not complete task");
    }

    #[test]
    fn it_calculates_weighted_reputation() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let agent = env.get_account(1);

        env.set_caller(admin);
        let mut contract = AgentNetwork::deploy(&env, AgentNetworkInitArgs { admin });

        // Register agent
        env.set_caller(agent);
        contract.register_agent(
            "Agent_1".to_string(),
            "Generic Agent".to_string(),
            "https://meta".to_string(),
        );

        // Create, assign, submit and complete task 1: score=90, weight=2
        env.set_caller(admin);
        let budget = U512::from(5_000_000_000u64);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://meta".to_string(), 0);
        contract.assign_task("task_01".to_string(), agent);
        env.set_caller(agent);
        contract.submit_result("task_01".to_string(), "hash".to_string());
        env.set_caller(admin);
        contract.complete_task("task_01".to_string(), "DeFi".to_string(), 90, 2);

        assert_eq!(contract.get_reputation(agent, "DeFi".to_string()), 90);

        // Create, assign, submit and complete task 2: score=85, weight=5
        contract.with_tokens(budget).create_task("task_02".to_string(), "https://meta".to_string(), 0);
        contract.assign_task("task_02".to_string(), agent);
        env.set_caller(agent);
        contract.submit_result("task_02".to_string(), "hash".to_string());
        env.set_caller(admin);
        contract.complete_task("task_02".to_string(), "DeFi".to_string(), 85, 5);

        // Expected weighted sum = 90*2 + 85*5 = 180 + 425 = 605
        // Expected total weight = 2 + 5 = 7
        // Expected average = 605 / 7 = 86
        assert_eq!(contract.get_reputation(agent, "DeFi".to_string()), 86);
    }
}
