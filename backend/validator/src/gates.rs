use crate::types::{CriterionDef, CriterionEval, ValidationInput, Verdict};

const MIN_OUTPUT_LEN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateFailure {
    EmptyOutput,
    MinLength,
    ErrorMarker,
    FixturePresent,
}

impl GateFailure {
    pub fn reason(self) -> &'static str {
        match self {
            GateFailure::EmptyOutput => "empty output",
            GateFailure::MinLength => "output too short",
            GateFailure::ErrorMarker => "output contains error marker",
            GateFailure::FixturePresent => "missing fixture",
        }
    }
}

pub fn check_input(input: &ValidationInput) -> Result<(), GateFailure> {
    if input.agent_output.trim().is_empty() {
        return Err(GateFailure::EmptyOutput);
    }

    if !input.fixture.is_object() {
        return Err(GateFailure::FixturePresent);
    }

    if input.agent_output.len() < MIN_OUTPUT_LEN {
        return Err(GateFailure::MinLength);
    }

    if input.agent_output.to_ascii_lowercase().contains("error") {
        return Err(GateFailure::ErrorMarker);
    }

    Ok(())
}

/// Fixture-free input gate for the stage pipeline (no fixture required).
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

pub fn gate_failure_output(
    criteria_defs: &[CriterionDef],
    failure: GateFailure,
) -> (Vec<CriterionEval>, String) {
    let reason = failure.reason();
    let gap = reason.to_string();
    let criteria = criteria_defs
        .iter()
        .map(|def| CriterionEval {
            id: def.id.to_string(),
            passed: false,
            score: 0,
            gap: Some(gap.clone()),
            evidence: Vec::new(),
        })
        .collect();

    let explanation = format!("Input gate failed: {reason}");
    (criteria, explanation)
}

pub fn gate_failure_verdict() -> Verdict {
    Verdict::Failed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SkillId;

    fn sample_input(agent_output: &str, fixture: serde_json::Value) -> ValidationInput {
        ValidationInput {
            skill: SkillId::DefiYieldRouting,
            task_prompt: "Allocate 10k CSPR".to_string(),
            agent_output: agent_output.to_string(),
            fixture,
            processing_time_ms: 10_000,
        }
    }

    #[test]
    fn empty_output_fails_gate() {
        let input = sample_input("   ", serde_json::json!({}));
        assert_eq!(check_input(&input), Err(GateFailure::EmptyOutput));
    }

    #[test]
    fn short_output_fails_min_length_gate() {
        let input = sample_input("too short", serde_json::json!({}));
        assert_eq!(check_input(&input), Err(GateFailure::MinLength));
    }

    #[test]
    fn error_marker_fails_gate() {
        let input = sample_input(
            "Allocation failed due to error in pool math calculation",
            serde_json::json!({}),
        );
        assert_eq!(check_input(&input), Err(GateFailure::ErrorMarker));
    }

    #[test]
    fn non_object_fixture_fails_gate() {
        let input = sample_input(
            "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY.",
            serde_json::json!([]),
        );
        assert_eq!(check_input(&input), Err(GateFailure::FixturePresent));
    }

    #[test]
    fn valid_input_passes_gates() {
        let input = sample_input(
            "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY.",
            serde_json::json!({ "amount_cspr": 10000 }),
        );
        assert!(check_input(&input).is_ok());
    }

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

    #[test]
    fn gate_failure_output_sets_zero_scores() {
        let criteria_defs = crate::rubric::criteria(SkillId::DefiYieldRouting);
        let (criteria, explanation) = gate_failure_output(criteria_defs, GateFailure::MinLength);

        assert_eq!(criteria.len(), 5);
        assert!(criteria.iter().all(|c| !c.passed && c.score == 0));
        assert!(explanation.contains("output too short"));
    }
}
