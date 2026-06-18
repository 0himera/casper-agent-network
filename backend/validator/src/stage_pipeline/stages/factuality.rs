use crate::llm::call_judge_raw;
use crate::prompts::{FactualityConfig, build_stage_claim_verification_prompts};
use crate::search::{CachedSearchProvider, SearchProvider};
use crate::stage_pipeline::factuality_types::{
    Claim, ClaimVerdict, ClaimVerification, FactualitySummary, SearchSnippet,
};
use crate::stage_pipeline::stage_scoring::quality_factuality;
use crate::stage_pipeline::types::PipelineVerdict;
use crate::types::{LlmConfig, ValidatorError};

use super::claims::{ClaimsExtraction, extract_claims};

#[derive(Debug, Clone, PartialEq)]
pub struct FactualityStageEval {
    pub raw_output: String,
    pub normalized_quality: f32,
    pub passed: bool,
    pub verdict: PipelineVerdict,
    pub summary: FactualitySummary,
    pub verifications: Vec<ClaimVerification>,
    pub extraction: ClaimsExtraction,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

pub fn should_skip_factuality(
    domain: &str,
    agent_output: &str,
    factuality_config: &FactualityConfig,
    factuality_enabled: bool,
) -> Option<String> {
    if !factuality_enabled {
        return Some("factuality disabled".to_string());
    }

    if domain == "code_review" {
        return Some("factuality disabled for code_review".to_string());
    }

    let char_count = agent_output.chars().count() as u32;
    if char_count < factuality_config.min_chars_for_factcheck {
        return Some(format!(
            "answer shorter than {} chars",
            factuality_config.min_chars_for_factcheck
        ));
    }

    None
}

pub fn format_snippets(snippets: &[SearchSnippet]) -> String {
    if snippets.is_empty() {
        return "- (no snippets)".to_string();
    }

    snippets
        .iter()
        .enumerate()
        .map(|(index, snippet)| {
            let title = snippet.title.as_deref().unwrap_or("Untitled");
            format!(
                "- [{index}] {title}: {} ({})",
                snippet.snippet,
                snippet.url.as_deref().unwrap_or("no-url")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_claim_verdict(text: &str) -> ClaimVerdict {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.contains("contradicted") {
        ClaimVerdict::Contradicted
    } else if normalized.contains("supported") {
        ClaimVerdict::Supported
    } else {
        ClaimVerdict::Unverifiable
    }
}

pub fn aggregate_factuality(summary: &FactualitySummary) -> (PipelineVerdict, f32, bool) {
    if summary.total == 0 {
        return (PipelineVerdict::Unverifiable, 0.0, false);
    }

    if summary.contradicted > 0 {
        return (
            PipelineVerdict::Hallucinated,
            quality_factuality(summary),
            false,
        );
    }

    if summary.supported == summary.total {
        return (PipelineVerdict::Factual, quality_factuality(summary), true);
    }

    (
        PipelineVerdict::Unverifiable,
        quality_factuality(summary),
        summary.supported > 0,
    )
}

pub fn factuality_details(
    extraction: &ClaimsExtraction,
    verifications: &[ClaimVerification],
    summary: &FactualitySummary,
) -> serde_json::Value {
    serde_json::json!({
        "parse_fallback": extraction.parse_fallback,
        "claims": verifications.iter().map(|verification| {
            serde_json::json!({
                "claim": verification.claim.text,
                "verdict": verification.verdict.as_str(),
                "evidence": verification.evidence,
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "supported": summary.supported,
            "contradicted": summary.contradicted,
            "unverifiable": summary.unverifiable,
            "total": summary.total,
        }
    })
}

pub async fn verify_claim(
    config: &LlmConfig,
    claim: &Claim,
    snippets: &[SearchSnippet],
) -> Result<ClaimVerdict, ValidatorError> {
    if config.mock {
        return Ok(parse_claim_verdict_from_snippets(snippets, &claim.text));
    }

    if snippets.is_empty() {
        return Ok(ClaimVerdict::Unverifiable);
    }

    let snippets_text = format_snippets(snippets);
    let (system, user) = build_stage_claim_verification_prompts(&claim.text, &snippets_text)?;
    let response = call_judge_raw(config, "stage_claim_verification", &system, &user).await?;
    Ok(parse_claim_verdict(&response))
}

fn parse_claim_verdict_from_snippets(
    snippets: &[SearchSnippet],
    _claim_text: &str,
) -> ClaimVerdict {
    if snippets.is_empty() {
        return ClaimVerdict::Unverifiable;
    }

    let combined = snippets
        .iter()
        .map(|snippet| snippet.snippet.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    if combined.contains("contradict") {
        ClaimVerdict::Contradicted
    } else if combined.contains("support") {
        ClaimVerdict::Supported
    } else {
        ClaimVerdict::Unverifiable
    }
}

#[cfg(test)]
fn parse_claim_verdict_mock_fallback(claim_text: &str) -> ClaimVerdict {
    let lower = claim_text.to_ascii_lowercase();
    if lower.contains("risk-free") || lower.contains("50%") {
        ClaimVerdict::Contradicted
    } else if lower.contains("obscure") {
        ClaimVerdict::Unverifiable
    } else {
        ClaimVerdict::Supported
    }
}

pub async fn evaluate_factuality_stage<P: SearchProvider + Sync + ?Sized>(
    config: &LlmConfig,
    domain: &str,
    agent_output: &str,
    factuality_config: &FactualityConfig,
    factuality_enabled: bool,
    search_provider: &P,
) -> Result<FactualityStageEval, ValidatorError> {
    if let Some(reason) =
        should_skip_factuality(domain, agent_output, factuality_config, factuality_enabled)
    {
        return Ok(FactualityStageEval {
            raw_output: String::new(),
            normalized_quality: 0.0,
            passed: false,
            verdict: PipelineVerdict::Factual,
            summary: FactualitySummary {
                supported: 0,
                contradicted: 0,
                unverifiable: 0,
                total: 0,
            },
            verifications: Vec::new(),
            extraction: ClaimsExtraction {
                claims: Vec::new(),
                parse_fallback: false,
            },
            skipped: true,
            skip_reason: Some(reason),
        });
    }

    let extraction = extract_claims(config, agent_output, factuality_config).await?;
    let mut verifications = Vec::new();

    for claim in &extraction.claims {
        let snippets = match search_provider
            .search(&claim.text, factuality_config.snippets_per_claim as usize)
            .await
        {
            Ok(snippets) => snippets,
            Err(_) => {
                verifications.push(ClaimVerification {
                    claim: claim.clone(),
                    verdict: ClaimVerdict::Unverifiable,
                    evidence: Vec::new(),
                });
                continue;
            }
        };

        let verdict = verify_claim(config, claim, &snippets).await?;
        verifications.push(ClaimVerification {
            claim: claim.clone(),
            verdict,
            evidence: snippets,
        });
    }

    let summary = FactualitySummary::from_verifications(&verifications);
    let (verdict, normalized_quality, passed) = aggregate_factuality(&summary);
    let raw_output = format!(
        "supported={} contradicted={} unverifiable={}",
        summary.supported, summary.contradicted, summary.unverifiable
    );

    Ok(FactualityStageEval {
        raw_output,
        normalized_quality,
        passed,
        verdict,
        summary,
        verifications,
        extraction,
        skipped: false,
        skip_reason: None,
    })
}

pub fn build_search_provider(
    config: &LlmConfig,
) -> Result<CachedSearchProvider<Box<dyn SearchProvider>>, ValidatorError> {
    if config.mock {
        return Ok(CachedSearchProvider::new(
            crate::search::mock::provider_for_mock_mode(None),
        ));
    }

    let provider = Box::new(crate::search::SerpApiProvider::from_optional_key(
        config.serpapi_api_key.clone(),
    )?) as Box<dyn SearchProvider>;
    Ok(CachedSearchProvider::new(provider))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::FactualityConfig;
    use crate::search::MockSearchProvider;
    use crate::stage_pipeline::factuality_types::SearchSnippet;

    fn factuality_config() -> FactualityConfig {
        FactualityConfig {
            enabled: true,
            max_claims: 5,
            snippets_per_claim: 3,
            min_chars_for_factcheck: 200,
        }
    }

    fn mock_config() -> LlmConfig {
        LlmConfig {
            mock: true,
            ..Default::default()
        }
    }

    #[test]
    fn parse_claim_verdict_variants() {
        assert_eq!(parse_claim_verdict("supported"), ClaimVerdict::Supported);
        assert_eq!(
            parse_claim_verdict("Contradicted"),
            ClaimVerdict::Contradicted
        );
        assert_eq!(parse_claim_verdict("unknown"), ClaimVerdict::Unverifiable);
    }

    #[test]
    fn aggregate_all_supported_is_factual() {
        let summary = FactualitySummary {
            supported: 2,
            contradicted: 0,
            unverifiable: 0,
            total: 2,
        };
        let (verdict, quality, passed) = aggregate_factuality(&summary);
        assert_eq!(verdict, PipelineVerdict::Factual);
        assert_eq!(quality, 1.0);
        assert!(passed);
    }

    #[test]
    fn aggregate_one_contradicted_is_hallucinated() {
        let summary = FactualitySummary {
            supported: 1,
            contradicted: 1,
            unverifiable: 0,
            total: 2,
        };
        let (verdict, _, passed) = aggregate_factuality(&summary);
        assert_eq!(verdict, PipelineVerdict::Hallucinated);
        assert!(!passed);
    }

    #[test]
    fn aggregate_mixed_is_unverifiable() {
        let summary = FactualitySummary {
            supported: 1,
            contradicted: 0,
            unverifiable: 1,
            total: 2,
        };
        let (verdict, _, passed) = aggregate_factuality(&summary);
        assert_eq!(verdict, PipelineVerdict::Unverifiable);
        assert!(passed);
    }

    #[test]
    fn aggregate_zero_claims_is_unverifiable() {
        let summary = FactualitySummary {
            supported: 0,
            contradicted: 0,
            unverifiable: 0,
            total: 0,
        };
        let (verdict, quality, passed) = aggregate_factuality(&summary);
        assert_eq!(verdict, PipelineVerdict::Unverifiable);
        assert_eq!(quality, 0.0);
        assert!(!passed);
    }

    #[test]
    fn skip_rules_cover_disabled_short_and_code_review() {
        let config = factuality_config();
        assert!(
            should_skip_factuality("defi_analysis", "x".repeat(250).as_str(), &config, false)
                .is_some()
        );
        assert!(should_skip_factuality("defi_analysis", "short", &config, true).is_some());
        assert!(
            should_skip_factuality("code_review", "x".repeat(250).as_str(), &config, true)
                .is_some()
        );
        assert!(
            should_skip_factuality("defi_analysis", "x".repeat(250).as_str(), &config, true)
                .is_none()
        );
    }

    #[tokio::test]
    async fn evaluate_factuality_supported_mock() {
        let config = mock_config();
        let provider = MockSearchProvider::with_snippets(vec![SearchSnippet {
            title: Some("Support".to_string()),
            snippet: "Evidence supports the claim.".to_string(),
            url: None,
        }]);
        let output = "MOCK_FACT_SUPPORTED: CSPR can be staked on the network. DeFi pools expose users to smart contract risk and should be evaluated carefully before allocation because capital can be lost due to exploits or market volatility in live markets.";

        let eval = evaluate_factuality_stage(
            &config,
            "defi_analysis",
            output,
            &factuality_config(),
            true,
            &provider,
        )
        .await
        .expect("factuality eval");

        assert!(!eval.skipped);
        assert_eq!(eval.verdict, PipelineVerdict::Factual);
        assert_eq!(eval.summary.supported, 2);
    }

    #[tokio::test]
    async fn evaluate_factuality_contradicted_mock() {
        let config = mock_config();
        let provider = MockSearchProvider::with_snippets(vec![SearchSnippet {
            title: Some("Contradiction".to_string()),
            snippet: "Evidence contradicts the claim.".to_string(),
            url: None,
        }]);
        let output = "MOCK_FACT_CONTRADICTED: CSPR staking APY is 50%. All DeFi pools are risk-free regardless of contract audits or market conditions and users should treat every pool as guaranteed principal protection without further due diligence.";

        let eval = evaluate_factuality_stage(
            &config,
            "defi_analysis",
            output,
            &factuality_config(),
            true,
            &provider,
        )
        .await
        .expect("factuality eval");

        assert_eq!(eval.verdict, PipelineVerdict::Hallucinated);
        assert_eq!(eval.summary.contradicted, 2);
    }

    #[test]
    fn mock_verdict_uses_snippet_evidence() {
        let supported = parse_claim_verdict_from_snippets(
            &[SearchSnippet {
                title: None,
                snippet: "Evidence supports the claim.".to_string(),
                url: None,
            }],
            "Generic factual claim.",
        );
        assert_eq!(supported, ClaimVerdict::Supported);

        let contradicted = parse_claim_verdict_from_snippets(
            &[SearchSnippet {
                title: None,
                snippet: "Evidence contradicts the claim.".to_string(),
                url: None,
            }],
            "Generic factual claim.",
        );
        assert_eq!(contradicted, ClaimVerdict::Contradicted);

        let unverifiable = parse_claim_verdict_from_snippets(&[], "Generic factual claim.");
        assert_eq!(unverifiable, ClaimVerdict::Unverifiable);
    }

    #[tokio::test]
    async fn evaluate_factuality_skips_short_answer() {
        let config = mock_config();
        let provider = MockSearchProvider::default();
        let eval = evaluate_factuality_stage(
            &config,
            "defi_analysis",
            "short answer",
            &factuality_config(),
            true,
            &provider,
        )
        .await
        .expect("factuality eval");

        assert!(eval.skipped);
        assert!(eval.skip_reason.unwrap().contains("shorter than"));
    }
}
