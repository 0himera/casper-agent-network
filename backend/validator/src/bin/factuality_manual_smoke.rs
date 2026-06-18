use validator_engine::{LlmConfig, StageId, evaluate_stage_pipeline};

const LONG_FACTUAL_ANSWER: &str = "CSPR can be staked on the Casper network through validator delegation, \
with rewards influenced by network participation and validator commission. DeFi liquidity pools on Casper \
expose users to smart contract risk, impermanent loss, and liquidity constraints that should be evaluated \
before allocating capital. Historical APY figures vary by epoch and should not be treated as guaranteed \
forward returns without independent verification from reputable on-chain analytics sources.";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut config = LlmConfig::from_env();
    config.mock = false;

    println!("N2 factuality manual smoke (real LLM + SerpAPI)\n");

    if !config.factuality_enabled.unwrap_or(false) {
        eprintln!("ERROR: set VALIDATOR_FACTUALITY=1 in backend/validator/.env");
        std::process::exit(1);
    }

    let serpapi_configured = config
        .serpapi_api_key
        .as_deref()
        .is_some_and(|key| !key.trim().is_empty());
    if !serpapi_configured {
        eprintln!("ERROR: set SERPAPI_API_KEY in backend/validator/.env");
        std::process::exit(1);
    }

    println!("VALIDATOR_FACTUALITY=1 OK");
    println!("SERPAPI_API_KEY set OK\n");

    let task_prompt =
        "Analyze CSPR staking APY and DeFi pool risks for long-form allocation guidance.";

    match evaluate_stage_pipeline("defi_analysis", task_prompt, LONG_FACTUAL_ANSWER, &config).await
    {
        Ok(output) => {
            println!("verdict={:?} total={}", output.verdict, output.total);
            println!("explanation: {}\n", output.explanation);

            for stage in &output.stages {
                println!(
                    "  {} passed={} raw={:?} quality={:.2} skipped={}",
                    stage.id.as_str(),
                    stage.passed,
                    stage.raw_output,
                    stage.normalized_quality,
                    stage.skipped_due_to_gate
                );
            }

            if let Some(factuality) = output
                .stages
                .iter()
                .find(|stage| stage.id == StageId::Factuality)
            {
                println!("\nfactuality_check details:");
                if let Some(details) = &factuality.details {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(details)
                            .unwrap_or_else(|_| details.to_string())
                    );
                } else if let Some(reason) = &factuality.reason {
                    println!("skipped: {reason}");
                }
            } else {
                println!("\nWARNING: no factuality_check stage in output");
            }
        }
        Err(error) => {
            eprintln!("ERROR: {error}");
            std::process::exit(1);
        }
    }
}
