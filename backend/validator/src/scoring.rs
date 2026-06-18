use crate::llm::LlmSoftCriterionResponse;
use crate::types::{CriterionDef, CriterionEval, SoftLabel, ToolResult, Verdict};

pub fn hard_from_tool(def: &CriterionDef, evidence: &[ToolResult]) -> CriterionEval {
    let tool_failed = evidence.iter().any(|e| !e.ok);
    let (passed, score, gap) = if tool_failed {
        (false, 0, Some("tool check failed".to_string()))
    } else {
        (true, def.weight, None)
    };

    CriterionEval {
        id: def.id.to_string(),
        passed,
        score,
        gap,
        evidence: evidence.to_vec(),
    }
}

pub fn soft_from_label(def: &CriterionDef, label: SoftLabel, gap: Option<String>) -> CriterionEval {
    let (passed, score, gap) = match label {
        SoftLabel::Strong => (true, def.weight, None),
        SoftLabel::Partial => {
            let gap = gap.or(Some("partial coverage".to_string()));
            (false, def.weight / 2, gap)
        }
        SoftLabel::Missing => {
            let gap = gap.or(Some("criterion missing".to_string()));
            (false, 0, gap)
        }
    };

    CriterionEval {
        id: def.id.to_string(),
        passed,
        score,
        gap,
        evidence: Vec::new(),
    }
}

pub fn soft_from_llm_response(
    def: &CriterionDef,
    llm_criterion: &LlmSoftCriterionResponse,
) -> CriterionEval {
    let mut gap = llm_criterion.gap.clone();
    if matches!(llm_criterion.label, SoftLabel::Partial | SoftLabel::Missing)
        && gap.as_ref().is_none_or(|g| g.is_empty())
    {
        gap = Some("no feedback provided".to_string());
    }
    soft_from_label(def, llm_criterion.label, gap)
}

pub fn compute_verdict_f3(
    criteria: &[CriterionEval],
    defs: &[CriterionDef],
    total: u32,
    threshold: u32,
) -> Verdict {
    let critical_failed = defs
        .iter()
        .zip(criteria.iter())
        .any(|(def, eval)| def.critical && !eval.passed);

    if critical_failed {
        return Verdict::Failed;
    }

    if total >= threshold {
        Verdict::Satisfied
    } else {
        Verdict::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SkillId;

    fn defi_def(id: &str) -> &'static CriterionDef {
        crate::rubric::criteria(SkillId::DefiYieldRouting)
            .iter()
            .find(|c| c.id == id)
            .expect("criterion")
    }

    fn stub_evidence(ok: bool) -> Vec<ToolResult> {
        vec![ToolResult {
            tool: "check_allocation_sum".to_string(),
            ok,
            details: serde_json::json!({ "stub": true }),
        }]
    }

    #[test]
    fn label_strong_maps_to_full_weight() {
        let def = defi_def("pool_selection");
        let eval = soft_from_label(def, SoftLabel::Strong, None);
        assert!(eval.passed);
        assert_eq!(eval.score, def.weight);
        assert!(eval.gap.is_none());
    }

    #[test]
    fn label_partial_maps_to_half() {
        let def = defi_def("pool_selection");
        let eval = soft_from_label(def, SoftLabel::Partial, Some("weak reasoning".to_string()));
        assert!(!eval.passed);
        assert_eq!(eval.score, def.weight / 2);
        assert_eq!(eval.gap.as_deref(), Some("weak reasoning"));
    }

    #[test]
    fn label_missing_maps_to_zero() {
        let def = defi_def("pool_selection");
        let eval = soft_from_label(def, SoftLabel::Missing, None);
        assert!(!eval.passed);
        assert_eq!(eval.score, 0);
    }

    #[test]
    fn hard_score_from_tool_ok() {
        let def = defi_def("allocation_sum");
        let eval = hard_from_tool(def, &stub_evidence(true));
        assert!(eval.passed);
        assert_eq!(eval.score, def.weight);
    }

    #[test]
    fn hard_score_zero_on_tool_fail() {
        let def = defi_def("allocation_sum");
        let eval = hard_from_tool(def, &stub_evidence(false));
        assert!(!eval.passed);
        assert_eq!(eval.score, 0);
    }

    #[test]
    fn critical_fail_overrides_threshold() {
        let defs = crate::rubric::criteria(SkillId::DefiYieldRouting);
        let mut criteria: Vec<CriterionEval> = defs
            .iter()
            .map(|def| CriterionEval {
                id: def.id.to_string(),
                passed: true,
                score: def.weight,
                gap: None,
                evidence: Vec::new(),
            })
            .collect();
        criteria[0].passed = false;
        criteria[0].score = 0;

        let verdict = compute_verdict_f3(&criteria, defs, 75, 70);
        assert_eq!(verdict, Verdict::Failed);
    }

    #[test]
    fn threshold_65_fails_without_critical_fail() {
        let defs = crate::rubric::criteria(SkillId::DefiYieldRouting);
        let criteria: Vec<CriterionEval> = defs
            .iter()
            .map(|def| {
                let score = if def.id == "pool_selection" {
                    0
                } else {
                    def.weight
                };
                CriterionEval {
                    id: def.id.to_string(),
                    passed: score == def.weight,
                    score,
                    gap: None,
                    evidence: Vec::new(),
                }
            })
            .collect();

        let total: u32 = criteria.iter().map(|c| c.score).sum();
        assert_eq!(total, 80);
        let verdict = compute_verdict_f3(&criteria, defs, total, 70);
        assert_eq!(verdict, Verdict::Satisfied);
    }

    #[test]
    fn threshold_72_satisfied() {
        let defs = crate::rubric::criteria(SkillId::DefiYieldRouting);
        let criteria: Vec<CriterionEval> = defs
            .iter()
            .map(|def| {
                let score = if def.id == "pool_selection" {
                    def.weight / 2
                } else {
                    def.weight
                };
                CriterionEval {
                    id: def.id.to_string(),
                    passed: score == def.weight,
                    score,
                    gap: None,
                    evidence: Vec::new(),
                }
            })
            .collect();

        let total: u32 = criteria.iter().map(|c| c.score).sum();
        assert_eq!(total, 90);
        let verdict = compute_verdict_f3(&criteria, defs, total, 70);
        assert_eq!(verdict, Verdict::Satisfied);
    }

    #[test]
    fn total_below_threshold_fails() {
        let defs = crate::rubric::criteria(SkillId::DefiProtocolRisk);
        let criteria: Vec<CriterionEval> = defs
            .iter()
            .map(|def| CriterionEval {
                id: def.id.to_string(),
                passed: def.id != "mitigation_steps",
                score: if def.id == "mitigation_steps" {
                    def.weight / 2
                } else {
                    def.weight
                },
                gap: None,
                evidence: Vec::new(),
            })
            .collect();

        let total: u32 = criteria.iter().map(|c| c.score).sum();
        assert_eq!(total, 85);
        let verdict = compute_verdict_f3(&criteria, defs, total, 70);
        assert_eq!(verdict, Verdict::Satisfied);

        let low_criteria: Vec<CriterionEval> = defs
            .iter()
            .map(|def| CriterionEval {
                id: def.id.to_string(),
                passed: false,
                score: if def.id == "mitigation_steps" {
                    0
                } else {
                    def.weight / 2
                },
                gap: None,
                evidence: Vec::new(),
            })
            .collect();
        let low_total: u32 = low_criteria.iter().map(|c| c.score).sum();
        assert_eq!(low_total, 34);
        let verdict = compute_verdict_f3(&low_criteria, defs, low_total, 70);
        assert_eq!(verdict, Verdict::Failed);
    }
}
