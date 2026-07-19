//! CLI tool for deploying and interacting with the AgentNetwork contract.
use agent_network::agent_network::AgentNetwork;
use odra::host::HostEnv;
use odra_cli::{
    deploy::DeployScript,
    scenario::{Args, Error, Scenario, ScenarioMetadata},
    CommandArg, DeployedContractsContainer, DeployerExt, OdraCli,
};

/// Deploys the `AgentNetwork` contract.
pub struct AgentNetworkDeployScript;

impl DeployScript for AgentNetworkDeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer,
    ) -> Result<(), odra_cli::deploy::Error> {
        let admin = env.get_account(0);
        AgentNetwork::load_or_deploy(
            env,
            agent_network::agent_network::AgentNetworkInitArgs { admin },
            container,
            350_000_000_000, // Gas limit
        )?;

        Ok(())
    }
}

/// A simple demo scenario to ping/check the contract.
pub struct PingScenario;

impl Scenario for PingScenario {
    fn args(&self) -> Vec<CommandArg> {
        vec![]
    }

    fn run(
        &self,
        _env: &HostEnv,
        _container: &DeployedContractsContainer,
        _args: Args,
    ) -> Result<(), Error> {
        println!("AgentNetwork contract CLI is configured and ready.");
        Ok(())
    }
}

impl ScenarioMetadata for PingScenario {
    const NAME: &'static str = "ping";
    const DESCRIPTION: &'static str = "Check contract status.";
}

/// Main function to run the CLI tool.
pub fn main() {
    OdraCli::new()
        .about("CLI tool for AgentNetwork smart contract")
        .deploy(AgentNetworkDeployScript)
        .contract::<AgentNetwork>()
        .scenario(PingScenario)
        .build()
        .run();
}
