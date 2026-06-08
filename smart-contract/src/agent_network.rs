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
        RecommendedPriceUpdated
    ]
)]
pub struct AgentNetwork {
    admin: Var<Address>,
    agents: Mapping<Address, AgentProfile>,
    tasks: Mapping<String, Task>,
    reputations: Mapping<(Address, String), u32>,
}

#[odra::module]
impl AgentNetwork {
    /// Initialize the contract.
    pub fn init(&mut self) {
        self.admin.set(self.env().caller());
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
    pub fn create_task(&mut self, task_id: String, metadata_uri: String) {
        let caller = self.env().caller();
        let attached_value = self.env().attached_value();

        if attached_value < U512::from(MINIMUM_BUDGET) {
            self.env().revert(ContractErrors::BelowMinimumBudget);
        }

        if self.tasks.get(&task_id).is_some() {
            // Task ID must be unique
            self.env().revert(ContractErrors::BelowMinimumBudget); // Re-use or define new error
        }

        let task = Task {
            creator: caller,
            assigned_agent: None,
            budget: attached_value,
            status: TaskStatus::Open,
            result_hash: String::new(),
            metadata_uri,
        };

        self.tasks.set(&task_id, task);
        self.env().emit_event(TaskCreated {
            task_id,
            creator: caller,
            budget: attached_value,
        });
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

    /// Submit execution results. Only the assigned agent can call this.
    pub fn submit_result(&mut self, task_id: String, result_hash: String) {
        let caller = self.env().caller();
        
        let mut task = self.tasks.get(&task_id).unwrap_or_revert(&self.env());
        if task.assigned_agent != Some(caller) {
            self.env().revert(ContractErrors::NotAssignedAgent);
        }

        if task.status != TaskStatus::InProgress {
            self.env().revert(ContractErrors::TaskNotAssigned);
        }

        task.result_hash = result_hash.clone();
        self.tasks.set(&task_id, task);

        self.env().emit_event(TaskSubmitted {
            task_id,
            agent: caller,
            result_hash,
        });
    }

    /// Complete task execution, release escrow, and update agent's reputation for a given skill.
    pub fn complete_task(&mut self, task_id: String, skill: String, score: u32) {
        let caller = self.env().caller();

        let mut task = self.tasks.get(&task_id).unwrap_or_revert(&self.env());
        if task.creator != caller {
            self.env().revert(ContractErrors::NotTaskCreator);
        }

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
        let current_score = self.reputations.get_or_default(&(agent, skill.clone()));
        let new_score = current_score + score;
        self.reputations.set(&(agent, skill.clone()), new_score);

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

    /// Get details of a task.
    pub fn get_task(&self, task_id: String) -> Option<Task> {
        self.tasks.get(&task_id)
    }

    /// Get reputation score of an agent for a specific skill.
    pub fn get_reputation(&self, agent: Address, skill: String) -> u32 {
        self.reputations.get_or_default(&(agent, skill))
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
    use odra::host::{Deployer, HostRef, NoArgs};

    #[test]
    fn it_registers_agents() {
        let env = odra_test::env();
        let admin = env.get_account(0);
        let agent_user = env.get_account(1);

        env.set_caller(admin);
        let mut contract = AgentNetwork::deploy(&env, NoArgs);

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
        let client = env.get_account(0);
        let agent = env.get_account(1);

        env.set_caller(client);
        let mut contract = AgentNetwork::deploy(&env, NoArgs);

        // Register agent
        env.set_caller(agent);
        contract.register_agent(
            "Agent_1".to_string(),
            "Generic Agent".to_string(),
            "https://meta".to_string(),
        );

        // Client creates a task with 5 CSPR budget
        env.set_caller(client);
        let budget = U512::from(5_000_000_000u64);
        contract.with_tokens(budget).create_task("task_01".to_string(), "https://task_meta".to_string());

        let task = contract.get_task("task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(task.budget, budget);

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

        // Client completes task, rewarding agent with reputation
        let agent_balance_before = env.balance_of(&agent);
        env.set_caller(client);
        contract.complete_task("task_01".to_string(), "DeFi".to_string(), 10);

        let task = contract.get_task("task_01".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);

        // Check reputation and payment
        assert_eq!(contract.get_reputation(agent, "DeFi".to_string()), 10);
        
        let agent_profile = contract.get_agent(agent).unwrap();
        assert_eq!(agent_profile.active_jobs, 0);

        let agent_balance_after = env.balance_of(&agent);
        assert_eq!(agent_balance_after, agent_balance_before + budget);
    }
}
