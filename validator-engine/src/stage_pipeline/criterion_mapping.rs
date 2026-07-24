use crate::types::{CriterionEval, ToolResult};

use super::types::{StageId, StageResult};

fn default_gap(stage: &StageResult) -> Option<String> {
    if stage.skipped_due_to_gate {
        return Some(
            stage
                .reason
                .clone()
                .unwrap_or_else(|| "skipped due to earlier stage failure".to_string()),
        );
    }
    if !stage.passed {
        return Some(
            stage
                .reason
                .clone()
                .unwrap_or_else(|| format!("{} failed", stage.id.as_str())),
        );
    }
    None
}

fn parse_fallback_from_details(details: &Option<serde_json::Value>) -> bool {
    details
        .as_ref()
        .and_then(|value| value.get("parse_fallback"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn base_stage_evidence(stage: &StageResult) -> ToolResult {
    ToolResult {
        tool: stage.id.as_str().to_string(),
        ok: stage.passed && !stage.skipped_due_to_gate,
        details: serde_json::json!({
            "raw_output": stage.raw_output,
            "normalized_quality": stage.normalized_quality,
            "weight": stage.weight,
            "weighted_score": stage.weighted_score,
            "parse_fallback": parse_fallback_from_details(&stage.details),
            "skipped_due_to_gate": stage.skipped_due_to_gate,
        }),
    }
}

fn factuality_evidence(stage: &StageResult) -> Vec<ToolResult> {
    let Some(details) = stage.details.as_ref() else {
        return vec![base_stage_evidence(stage)];
    };

    let mut evidence = vec![ToolResult {
        tool: "factuality_check".to_string(),
        ok: stage.passed && !stage.skipped_due_to_gate,
        details: serde_json::json!({
            "parse_fallback": details.get("parse_fallback").cloned().unwrap_or(serde_json::Value::Null),
            "summary": details.get("summary").cloned().unwrap_or(serde_json::Value::Null),
            "raw_output": stage.raw_output,
            "normalized_quality": stage.normalized_quality,
            "weight": stage.weight,
            "weighted_score": stage.weighted_score,
            "skipped_due_to_gate": stage.skipped_due_to_gate,
        }),
    }];

    if let Some(claims) = details.get("claims").and_then(|value| value.as_array()) {
        for (index, claim) in claims.iter().enumerate() {
            evidence.push(ToolResult {
                tool: format!("factuality_claim_{index}"),
                ok: claim
                    .get("verdict")
                    .and_then(|value| value.as_str())
                    .map(|verdict| verdict == "supported")
                    .unwrap_or(false),
                details: serde_json::json!({
                    "claim": claim.get("claim").cloned().unwrap_or(serde_json::Value::Null),
                    "verdict": claim.get("verdict").cloned().unwrap_or(serde_json::Value::Null),
                    "snippets": claim.get("evidence").cloned().unwrap_or(serde_json::Value::Null),
                }),
            });
        }
    }

    evidence
}

fn stage_evidence(stage: &StageResult) -> Vec<ToolResult> {
    if stage.id == StageId::Factuality {
        return factuality_evidence(stage);
    }

    vec![base_stage_evidence(stage)]
}

/// Map a single stage result to the shared `CriterionEval` contract.
pub fn stage_to_criterion(stage: &StageResult) -> CriterionEval {
    CriterionEval {
        id: stage.id.as_str().to_string(),
        passed: stage.passed && !stage.skipped_due_to_gate,
        score: stage.weighted_score,
        gap: stage.reason.clone().or_else(|| default_gap(stage)),
        evidence: stage_evidence(stage),
    }
}

/// Map all stage results to criteria, preserving order.
pub fn map_stages_to_criteria(stages: &[StageResult]) -> Vec<CriterionEval> {
    stages.iter().map(stage_to_criterion).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage_pipeline::stage_scoring::{weight, weighted_score};

    fn sample_stage(id: StageId, passed: bool, skipped: bool) -> StageResult {
        let quality = if passed { 1.0 } else { 0.0 };
        let stage_weight = weight(id);
        StageResult {
            id,
            passed,
            raw_output: "ok".to_string(),
            normalized_quality: quality,
            weight: stage_weight,
            weighted_score: weighted_score(quality, stage_weight),
            skipped_due_to_gate: skipped,
            reason: if skipped {
                Some("skipped due to earlier stage failure".to_string())
            } else {
                None
            },
            details: None,
        }
    }

    #[test]
    fn stage_to_criterion_maps_core_fields() {
        let stage = sample_stage(StageId::Relevance, true, false);
        let criterion = stage_to_criterion(&stage);

        assert_eq!(criterion.id, "relevance_check");
        assert!(criterion.passed);
        assert_eq!(criterion.score, stage.weighted_score);
        assert!(criterion.gap.is_none());
        assert_eq!(criterion.evidence.len(), 1);
        assert_eq!(criterion.evidence[0].tool, "relevance_check");
        assert!(criterion.evidence[0].ok);
    }

    #[test]
    fn skipped_stage_is_not_passed_and_has_gap() {
        let stage = sample_stage(StageId::Gibberish, false, true);
        let criterion = stage_to_criterion(&stage);

        assert!(!criterion.passed);
        assert!(criterion.gap.is_some());
        assert!(!criterion.evidence[0].ok);
    }

    #[test]
    fn map_stages_preserves_count_and_order() {
        let stages = vec![
            sample_stage(StageId::Refusal, true, false),
            sample_stage(StageId::Gibberish, true, false),
            sample_stage(StageId::Relevance, true, false),
            sample_stage(StageId::DomainMatch, true, false),
        ];
        let criteria = map_stages_to_criteria(&stages);

        assert_eq!(criteria.len(), stages.len());
        assert_eq!(criteria[0].id, "refusal_check");
        assert_eq!(criteria[3].id, "domain_check");
    }

    #[test]
    fn factuality_stage_maps_claim_evidence() {
        let stage_weight = weight(StageId::Factuality);
        let stage = StageResult {
            id: StageId::Factuality,
            passed: true,
            raw_output: "supported".to_string(),
            normalized_quality: 1.0,
            weight: stage_weight,
            weighted_score: weighted_score(1.0, stage_weight),
            skipped_due_to_gate: false,
            reason: None,
            details: Some(serde_json::json!({
                "parse_fallback": false,
                "claims": [{
                    "claim": "CSPR can be staked",
                    "verdict": "supported",
                    "evidence": [{"title": "Casper docs", "snippet": "staking guide"}]
                }],
                "summary": {
                    "supported": 1,
                    "contradicted": 0,
                    "unverifiable": 0,
                    "total": 1
                }
            })),
        };

        let criterion = stage_to_criterion(&stage);
        assert_eq!(criterion.id, "factuality_check");
        assert!(!criterion.evidence.is_empty());
        assert!(
            criterion
                .evidence
                .iter()
                .any(|entry| entry.tool == "factuality_check")
        );
        assert!(
            criterion
                .evidence
                .iter()
                .any(|entry| entry.tool == "factuality_claim_0")
        );
    }
}
