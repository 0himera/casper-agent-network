use std::fs;
use std::path::PathBuf;

use validator_engine::{LlmConfig, SkillId, ValidationInput, Verdict, evaluate};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn load_fixture(name: &str) -> serde_json::Value {
    let path = fixture_path(name);
    let content = fs::read_to_string(&path).expect("fixture file must exist");
    serde_json::from_str(&content).expect("fixture must be valid JSON")
}

fn mock_config() -> LlmConfig {
    LlmConfig {
        mock: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn golden_defi_yield_routing_good_output() {
    let input = ValidationInput {
        skill: SkillId::DefiYieldRouting,
        task_prompt: "Allocate 10,000 CSPR across Casper liquidity pools minimizing impermanent loss risk."
            .to_string(),
        agent_output: "Allocate 4,000 CSPR to cspr-usdt (8.2% APY, high TVL), 3,500 CSPR to cspr-eth (6.1% APY, moderate IL), and 2,500 CSPR to cspr-wbtc (11.4% APY, higher IL risk). Total: 10,000 CSPR. Network gas fees (~2.5 CSPR per swap) included. IL analysis shows cspr-usdt lowest volatility exposure."
            .to_string(),
        fixture: load_fixture("defi_yield_routing.json"),
        processing_time_ms: 4_000,
    };

    let output = evaluate(input, &mock_config()).await.expect("evaluate ok");

    assert_eq!(output.verdict, Verdict::Satisfied);
    assert_eq!(output.total, 100);
    assert_eq!(output.criteria.len(), 5);
    assert!(output.criteria.iter().all(|c| c.passed));
    assert_eq!(output.recommended_price_motes, 6_000_000_000);
}

#[tokio::test]
async fn golden_defi_yield_routing_bad_output() {
    let input = ValidationInput {
        skill: SkillId::DefiYieldRouting,
        task_prompt: "Allocate 10,000 CSPR across Casper liquidity pools.".to_string(),
        agent_output: "error: allocation failed".to_string(),
        fixture: load_fixture("defi_yield_routing.json"),
        processing_time_ms: 10_000,
    };

    let output = evaluate(input, &mock_config()).await.expect("evaluate ok");

    assert_eq!(output.verdict, Verdict::Failed);
    assert!(output.total < 100);
    assert!(output.criteria.iter().all(|c| !c.passed));
    assert!(
        output
            .criteria
            .iter()
            .all(|c| c.gap.as_deref() == Some("mock: output too short or contains error"))
    );
}

#[tokio::test]
async fn golden_defi_protocol_risk_good_output() {
    let input = ValidationInput {
        skill: SkillId::DefiProtocolRisk,
        task_prompt: "Analyze protocol risk based on recent transaction revert patterns."
            .to_string(),
        agent_output: "Revert rate is 15% over the last 6 hours, exceeding the 10% anomaly threshold. Classification: High Risk. Recommended mitigation: pause new deposits, increase monitoring frequency, alert users to withdraw idle funds, and prepare emergency circuit breaker activation."
            .to_string(),
        fixture: load_fixture("defi_protocol_risk.json"),
        processing_time_ms: 12_000,
    };

    let output = evaluate(input, &mock_config()).await.expect("evaluate ok");

    assert_eq!(output.verdict, Verdict::Satisfied);
    assert_eq!(output.total, 100);
    assert_eq!(output.criteria.len(), 3);
    assert!(output.criteria.iter().all(|c| c.passed));
}

#[tokio::test]
async fn golden_defi_protocol_risk_bad_output() {
    let input = ValidationInput {
        skill: SkillId::DefiProtocolRisk,
        task_prompt: "Analyze protocol risk.".to_string(),
        agent_output: "error".to_string(),
        fixture: load_fixture("defi_protocol_risk.json"),
        processing_time_ms: 10_000,
    };

    let output = evaluate(input, &mock_config()).await.expect("evaluate ok");

    assert_eq!(output.verdict, Verdict::Failed);
    assert!(output.total < 100);
}

#[tokio::test]
async fn golden_rwa_appraisal_good_output() {
    let input = ValidationInput {
        skill: SkillId::RwaAppraisal,
        task_prompt: "Determine fair gold price for on-chain oracle update from external sources."
            .to_string(),
        agent_output: "Filtered retail_feed ($2410) as outlier (>3% deviation). Cross-checked LBMA, COMEX, ECB sources. Weighted median fair price: $2,346.50 USD/oz based on reliability scores. Algorithm: exclude outliers, weight by source reliability, compute median."
            .to_string(),
        fixture: load_fixture("rwa_appraisal.json"),
        processing_time_ms: 15_000,
    };

    let output = evaluate(input, &mock_config()).await.expect("evaluate ok");

    assert_eq!(output.verdict, Verdict::Satisfied);
    assert_eq!(output.total, 100);
    assert_eq!(output.criteria.len(), 3);
}

#[tokio::test]
async fn golden_rwa_appraisal_bad_output() {
    let input = ValidationInput {
        skill: SkillId::RwaAppraisal,
        task_prompt: "Determine fair gold price.".to_string(),
        agent_output: "too short".to_string(),
        fixture: load_fixture("rwa_appraisal.json"),
        processing_time_ms: 10_000,
    };

    let output = evaluate(input, &mock_config()).await.expect("evaluate ok");

    assert_eq!(output.verdict, Verdict::Failed);
    assert!(output.total < 100);
}

#[tokio::test]
async fn golden_rwa_compliance_good_output() {
    let input = ValidationInput {
        skill: SkillId::RwaCompliance,
        task_prompt: "Assess issuer compliance risk and recommend collateral factor adjustment."
            .to_string(),
        agent_output: "SEC inquiry (verified, high severity) is a real threat requiring collateral reduction. Social media default claims are unverified FUD. Recommendation: reduce collateral factor from 0.85 to 0.70. Remediation: monitor SEC proceedings, require additional disclosures, set 30-day review checkpoint."
            .to_string(),
        fixture: load_fixture("rwa_compliance.json"),
        processing_time_ms: 20_000,
    };

    let output = evaluate(input, &mock_config()).await.expect("evaluate ok");

    assert_eq!(output.verdict, Verdict::Satisfied);
    assert_eq!(output.total, 100);
    assert_eq!(output.criteria.len(), 3);
}

#[tokio::test]
async fn golden_rwa_compliance_bad_output() {
    let input = ValidationInput {
        skill: SkillId::RwaCompliance,
        task_prompt: "Assess compliance risk.".to_string(),
        agent_output: "error in analysis".to_string(),
        fixture: load_fixture("rwa_compliance.json"),
        processing_time_ms: 10_000,
    };

    let output = evaluate(input, &mock_config()).await.expect("evaluate ok");

    assert_eq!(output.verdict, Verdict::Failed);
    assert!(output.total < 100);
}
