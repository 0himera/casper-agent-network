/// Extracts the value after a case-insensitive `ANSWER:` prefix (first line only).
pub fn extract_answer_contract(agent_output: &str) -> Option<String> {
    let lower = agent_output.to_ascii_lowercase();
    let prefix = "answer:";
    let idx = lower.find(prefix)?;
    let value_start = idx + prefix.len();
    let remainder = agent_output[value_start..].trim_start();
    let first_line = remainder.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }
    Some(first_line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_answer_simple() {
        assert_eq!(
            extract_answer_contract("ANSWER: 12345.67 USD\n"),
            Some("12345.67 USD".to_string())
        );
    }

    #[test]
    fn extract_answer_case_insensitive() {
        assert_eq!(
            extract_answer_contract("answer: 1 usd"),
            Some("1 usd".to_string())
        );
    }

    #[test]
    fn extract_answer_with_markdown_noise() {
        let output = "**Result**\n\nANSWER: 42 usd\n\nDone.";
        assert_eq!(extract_answer_contract(output), Some("42 usd".to_string()));
    }

    #[test]
    fn extract_answer_missing_returns_none() {
        assert_eq!(extract_answer_contract("No structured answer here."), None);
    }
}
