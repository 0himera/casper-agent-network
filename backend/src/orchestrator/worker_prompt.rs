const MAX_PROMPT_BLOCK_CHARS: usize = 4000;

fn truncate(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        format!("{}...", &text[..max_chars])
    }
}

/// Build the worker user prompt: task text plus an optional `<fixture>` block.
///
/// When `fixture` is null or not an object, returns `task_prompt` unchanged.
pub fn build_worker_prompt(task_prompt: &str, fixture: &serde_json::Value) -> String {
    if fixture.is_null() || !fixture.is_object() {
        return task_prompt.to_string();
    }

    let fixture_block = truncate(&fixture.to_string(), MAX_PROMPT_BLOCK_CHARS);
    format!(
        "{task_prompt}\n\n<fixture>\n{fixture_block}\n</fixture>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_worker_prompt_includes_fixture_block() {
        let fixture = serde_json::json!({ "amount_cspr": 10000 });
        let prompt = build_worker_prompt("Allocate CSPR", &fixture);
        assert!(prompt.contains("Allocate CSPR"));
        assert!(prompt.contains("<fixture>"));
        assert!(prompt.contains("amount_cspr"));
        assert!(prompt.contains("</fixture>"));
    }

    #[test]
    fn build_worker_prompt_skips_null_fixture() {
        let prompt = build_worker_prompt("Allocate CSPR", &serde_json::Value::Null);
        assert_eq!(prompt, "Allocate CSPR");
    }
}
