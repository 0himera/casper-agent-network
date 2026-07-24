use crate::llm::call_judge_raw;
use crate::prompts::{FactualityConfig, build_stage_claim_decomposition_prompts};
use crate::stage_pipeline::factuality_types::Claim;
use crate::types::{LlmConfig, ValidatorError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimsExtraction {
    pub claims: Vec<Claim>,
    pub parse_fallback: bool,
}

pub fn split_sentences_fallback(answer: &str) -> Vec<Claim> {
    answer
        .split(['.', '!', '?', '\n'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .map(|sentence| Claim {
            text: sentence.to_string(),
        })
        .collect()
}

pub fn parse_claims_json(text: &str) -> Option<Vec<Claim>> {
    let trimmed = text.trim();
    let json_str = if trimmed.starts_with('[') {
        trimmed
    } else {
        trimmed
            .find('[')
            .and_then(|start| trimmed.rfind(']').map(|end| &trimmed[start..=end]))?
    };

    let parsed: Vec<String> = serde_json::from_str(json_str).ok()?;
    Some(
        parsed
            .into_iter()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
            .map(|text| Claim { text })
            .collect(),
    )
}

pub fn limit_claims(mut claims: Vec<Claim>, max_claims: u32) -> Vec<Claim> {
    claims.truncate(max_claims as usize);
    claims
}

pub fn extract_claims_from_text(text: String, max_claims: u32) -> ClaimsExtraction {
    if let Some(claims) = parse_claims_json(&text) {
        return ClaimsExtraction {
            claims: limit_claims(claims, max_claims),
            parse_fallback: false,
        };
    }

    ClaimsExtraction {
        claims: limit_claims(split_sentences_fallback(&text), max_claims),
        parse_fallback: true,
    }
}

pub async fn extract_claims(
    config: &LlmConfig,
    agent_output: &str,
    factuality_config: &FactualityConfig,
) -> Result<ClaimsExtraction, ValidatorError> {
    if agent_output.trim().is_empty() {
        return Ok(ClaimsExtraction {
            claims: Vec::new(),
            parse_fallback: false,
        });
    }

    if config.mock {
        return Ok(parse_claims_mock_response(
            agent_output,
            factuality_config.max_claims,
        ));
    }

    let (system, user) = build_stage_claim_decomposition_prompts(agent_output)?;
    let first = call_judge_raw(config, "stage_claim_decomposition", &system, &user).await?;
    let first_result = extract_claims_from_text(first, factuality_config.max_claims);
    if !first_result.parse_fallback {
        return Ok(first_result);
    }

    let retry = call_judge_raw(config, "stage_claim_decomposition", &system, &user).await?;
    let retry_result = extract_claims_from_text(retry, factuality_config.max_claims);
    if !retry_result.parse_fallback {
        return Ok(retry_result);
    }

    Ok(ClaimsExtraction {
        claims: limit_claims(
            split_sentences_fallback(agent_output),
            factuality_config.max_claims,
        ),
        parse_fallback: true,
    })
}

pub fn parse_claims_mock_response(agent_output: &str, max_claims: u32) -> ClaimsExtraction {
    let lower = agent_output.to_ascii_lowercase();

    if lower.contains("mock_fact_short") {
        return ClaimsExtraction {
            claims: Vec::new(),
            parse_fallback: false,
        };
    }

    let claims = if lower.contains("mock_fact_contradicted") {
        vec![
            Claim {
                text: "CSPR staking APY is 50%.".to_string(),
            },
            Claim {
                text: "All DeFi pools are risk-free.".to_string(),
            },
        ]
    } else if lower.contains("mock_fact_unverifiable") {
        vec![Claim {
            text: "An obscure protocol launched yesterday.".to_string(),
        }]
    } else if lower.contains("mock_fact_supported") || lower.contains("mock_factuality") {
        vec![
            Claim {
                text: "CSPR can be staked on the network.".to_string(),
            },
            Claim {
                text: "DeFi pools expose users to smart contract risk.".to_string(),
            },
        ]
    } else {
        Vec::new()
    };

    ClaimsExtraction {
        claims: limit_claims(claims, max_claims),
        parse_fallback: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json_claims() {
        let result = extract_claims_from_text(
            r#"["CSPR APY is 8%", "Pool TVL exceeds $1M"]"#.to_string(),
            5,
        );
        assert_eq!(result.claims.len(), 2);
        assert!(!result.parse_fallback);
    }

    #[test]
    fn invalid_json_uses_sentence_fallback() {
        let result =
            extract_claims_from_text("First claim. Second claim! Third claim?".to_string(), 5);
        assert_eq!(result.claims.len(), 3);
        assert!(result.parse_fallback);
    }

    #[test]
    fn max_claims_truncates_list() {
        let result = extract_claims_from_text(
            r#"["one", "two", "three", "four", "five", "six"]"#.to_string(),
            5,
        );
        assert_eq!(result.claims.len(), 5);
    }

    #[test]
    fn double_invalid_json_falls_back_to_original_agent_output() {
        let agent_output = "First claim. Second claim!";
        let first_result = extract_claims_from_text("not json".to_string(), 5);
        assert!(first_result.parse_fallback);
        let retry_result = extract_claims_from_text("also not json".to_string(), 5);
        assert!(retry_result.parse_fallback);

        let fallback = ClaimsExtraction {
            claims: limit_claims(split_sentences_fallback(agent_output), 5),
            parse_fallback: true,
        };
        assert_eq!(fallback.claims.len(), 2);
        assert_eq!(fallback.claims[0].text, "First claim");
        assert_eq!(fallback.claims[1].text, "Second claim");
    }

    #[test]
    fn retry_valid_json_uses_retry_response() {
        let first_result = extract_claims_from_text("not json".to_string(), 5);
        assert!(first_result.parse_fallback);
        let retry_result =
            extract_claims_from_text(r#"["Retry claim one", "Retry claim two"]"#.to_string(), 5);
        assert!(!retry_result.parse_fallback);
        assert_eq!(retry_result.claims.len(), 2);
        assert_eq!(retry_result.claims[0].text, "Retry claim one");
    }
}
