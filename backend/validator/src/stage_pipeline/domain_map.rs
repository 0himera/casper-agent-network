/// Maps platform-declared domain strings to expected-domain labels for stage 3 prompts.
pub fn expected_domain_label(domain: &str) -> &'static str {
    match domain {
        "code_review" => "software code review and security audit",
        "defi_yield_routing" | "defi_protocol_risk" => {
            "DeFi yield routing and protocol risk analysis"
        }
        "rwa_appraisal" | "rwa_compliance" => "real-world asset (RWA) appraisal and compliance",
        "defi_analysis" | "DeFi/RWA" => "DeFi and real-world asset financial analysis",
        _ => "general financial and technical analysis",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_review_has_distinct_label() {
        assert_eq!(
            expected_domain_label("code_review"),
            "software code review and security audit"
        );
    }

    #[test]
    fn unknown_domain_uses_fallback() {
        assert_eq!(
            expected_domain_label("unknown_domain_xyz"),
            "general financial and technical analysis"
        );
    }
}
