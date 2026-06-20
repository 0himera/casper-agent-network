/// Maps platform-declared domain strings to expected-domain labels for stage 3 prompts.
pub fn expected_domain_label(domain: &str) -> &'static str {
    match domain {
        "defi" | "defi_analysis" | "DeFi/RWA" => "DeFi financial analysis",
        "rwa" | "rwa_valuation" => "real-world asset (RWA) analysis",
        _ => "general financial and technical analysis",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defi_has_distinct_label() {
        assert_eq!(expected_domain_label("defi"), "DeFi financial analysis");
    }

    #[test]
    fn rwa_uses_rwa_label() {
        assert_eq!(expected_domain_label("rwa"), "real-world asset (RWA) analysis");
    }

    #[test]
    fn unknown_domain_uses_fallback() {
        assert_eq!(
            expected_domain_label("unknown_domain_xyz"),
            "general financial and technical analysis"
        );
    }
}
