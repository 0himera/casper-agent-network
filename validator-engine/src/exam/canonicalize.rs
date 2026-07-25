/// Trim, collapse whitespace, lowercase, strip trailing `.` and `,`.
pub fn canonicalize_exam_answer(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = collapsed.to_ascii_lowercase();
    lower.trim_end_matches(['.', ',']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_exam_answer_example() {
        assert_eq!(canonicalize_exam_answer("12345.67 USD\n"), "12345.67 usd");
    }

    #[test]
    fn canonicalize_strips_trailing_punctuation() {
        assert_eq!(canonicalize_exam_answer("42 usd."), "42 usd");
        assert_eq!(canonicalize_exam_answer("42 usd,"), "42 usd");
    }

    #[test]
    fn canonicalize_collapses_whitespace() {
        assert_eq!(canonicalize_exam_answer("  1   usd  "), "1 usd");
    }

    #[test]
    fn parse_then_canonicalize_matches_e0_example() {
        use super::super::parse::extract_answer_contract;

        let raw = extract_answer_contract("ANSWER: 12345.67 USD\n").expect("answer extracted");
        assert_eq!(canonicalize_exam_answer(&raw), "12345.67 usd");
    }
}
