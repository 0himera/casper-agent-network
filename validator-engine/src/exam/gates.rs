#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExamGateFailure {
    EmptyOutput,
    ErrorMarker,
}

impl ExamGateFailure {
    pub fn reason(self) -> &'static str {
        match self {
            ExamGateFailure::EmptyOutput => "empty output",
            ExamGateFailure::ErrorMarker => "output contains error marker",
        }
    }
}

/// Exam-specific input gate: empty/error checks only (no minimum length).
pub fn check_exam_input_gate(agent_output: &str) -> Result<(), ExamGateFailure> {
    if agent_output.trim().is_empty() {
        return Err(ExamGateFailure::EmptyOutput);
    }

    if agent_output.to_ascii_lowercase().contains("error") {
        return Err(ExamGateFailure::ErrorMarker);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exam_gate_short_answer_passes() {
        assert!(check_exam_input_gate("ANSWER: 1 usd").is_ok());
    }

    #[test]
    fn exam_gate_empty_output_fails() {
        assert_eq!(
            check_exam_input_gate("   "),
            Err(ExamGateFailure::EmptyOutput)
        );
    }

    #[test]
    fn exam_gate_error_marker_fails() {
        assert_eq!(
            check_exam_input_gate("execution error"),
            Err(ExamGateFailure::ErrorMarker)
        );
    }

    #[test]
    fn exam_gate_allows_short_output_that_generic_gate_rejects() {
        use crate::gates::{GateFailure, check_input_fixture_free};

        let short_answer = "ANSWER: 1 usd";
        assert!(check_exam_input_gate(short_answer).is_ok());
        assert_eq!(
            check_input_fixture_free(short_answer),
            Err(GateFailure::MinLength)
        );
    }
}
