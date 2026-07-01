use odra::prelude::*;
use crate::agent_network::ContractErrors;

#[test]
fn test_err_code() {
    println!("AgentAlreadyExists code: {}", ContractErrors::AgentAlreadyExists as u16);
}
