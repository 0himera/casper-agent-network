const MIN_OUTPUT_LEN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateFailure {
    EmptyOutput,
    MinLength,
    ErrorMarker,
}

impl GateFailure {
    pub fn reason(self) -> &'static str {
        match self {
            GateFailure::EmptyOutput => "empty output",
            GateFailure::MinLength => "output too short",
            GateFailure::ErrorMarker => "output contains error marker",
        }
    }
}

/// Fixture-free input gate for the stage pipeline.
pub fn check_input_fixture_free(agent_output: &str) -> Result<(), GateFailure> {
    if agent_output.trim().is_empty() {
        return Err(GateFailure::EmptyOutput);
    }

    if agent_output.len() < MIN_OUTPUT_LEN {
        return Err(GateFailure::MinLength);
    }

    if agent_output.to_ascii_lowercase().contains("error") {
        return Err(GateFailure::ErrorMarker);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_free_empty_output_fails() {
        assert_eq!(
            check_input_fixture_free("   "),
            Err(GateFailure::EmptyOutput)
        );
    }

    #[test]
    fn fixture_free_short_output_fails() {
        assert_eq!(
            check_input_fixture_free("too short"),
            Err(GateFailure::MinLength)
        );
    }

    #[test]
    fn fixture_free_error_marker_fails() {
        assert_eq!(
            check_input_fixture_free("Allocation failed due to error in pool math"),
            Err(GateFailure::ErrorMarker)
        );
    }

    #[test]
    fn fixture_free_valid_output_passes() {
        assert!(
            check_input_fixture_free(
                "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY."
            )
            .is_ok()
        );
    }
}
