/// Exact match after canonicalization (Type H MVP).
pub fn compare_type_h(canonical_actual: &str, canonical_expected: &str) -> bool {
    canonical_actual == canonical_expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_type_h_match() {
        assert!(compare_type_h("12345.67 usd", "12345.67 usd"));
    }

    #[test]
    fn compare_type_h_mismatch() {
        assert!(!compare_type_h("999 usd", "12345.67 usd"));
    }
}
