use std::collections::HashMap;

use crate::prompts::MAX_PROMPT_BLOCK_CHARS;
use crate::types::{SelfConsistencyTrigger, SoftLabel};

use super::SoftGraderLlmResponse;

pub fn should_run_self_consistency(
    response: &SoftGraderLlmResponse,
    trigger: SelfConsistencyTrigger,
) -> bool {
    match trigger {
        SelfConsistencyTrigger::PartialOnly => response
            .criteria
            .iter()
            .any(|c| c.label == SoftLabel::Partial),
    }
}

fn label_rank(label: SoftLabel) -> u8 {
    match label {
        SoftLabel::Missing => 0,
        SoftLabel::Partial => 1,
        SoftLabel::Strong => 2,
    }
}

fn pick_majority_label(votes: &[SoftLabel]) -> SoftLabel {
    let mut counts = HashMap::new();
    for label in votes {
        *counts.entry(*label).or_insert(0u32) += 1;
    }

    let max_count = counts.values().copied().max().unwrap_or(0);
    let tied: Vec<SoftLabel> = counts
        .into_iter()
        .filter(|(_, count)| *count == max_count)
        .map(|(label, _)| label)
        .collect();

    tied.into_iter()
        .min_by_key(|label| label_rank(*label))
        .unwrap_or(SoftLabel::Missing)
}

fn merge_gaps(gaps: &[Option<String>]) -> Option<String> {
    let unique: Vec<String> = gaps
        .iter()
        .filter_map(|g| g.as_ref())
        .filter(|g| !g.is_empty())
        .map(|g| g.trim().to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if unique.is_empty() {
        return None;
    }

    let merged = unique.join("; ");
    Some(if merged.len() <= MAX_PROMPT_BLOCK_CHARS {
        merged
    } else {
        format!("{}...", &merged[..MAX_PROMPT_BLOCK_CHARS])
    })
}

pub fn aggregate_soft_responses(samples: &[SoftGraderLlmResponse]) -> SoftGraderLlmResponse {
    let first = samples
        .first()
        .expect("self-consistency requires at least one sample");

    let criterion_ids: Vec<String> = first
        .criteria
        .iter()
        .map(|c| c.id.clone())
        .collect();

    let mut aggregated_criteria = Vec::with_capacity(criterion_ids.len());

    for id in criterion_ids {
        let labels: Vec<SoftLabel> = samples
            .iter()
            .filter_map(|sample| {
                sample
                    .criteria
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.label)
            })
            .collect();

        let majority = pick_majority_label(&labels);
        let gaps: Vec<Option<String>> = samples
            .iter()
            .filter_map(|sample| {
                sample
                    .criteria
                    .iter()
                    .find(|c| c.id == id && c.label == majority)
                    .map(|c| c.gap.clone())
            })
            .collect();

        let gap = if majority == SoftLabel::Strong {
            None
        } else {
            merge_gaps(&gaps)
        };

        aggregated_criteria.push(super::LlmSoftCriterionResponse {
            id,
            label: majority,
            gap,
        });
    }

    let explanation = samples
        .iter()
        .find_map(|sample| {
            sample
                .criteria
                .first()
                .map(|_| sample.explanation.clone())
        })
        .unwrap_or_else(|| first.explanation.clone());

    SoftGraderLlmResponse {
        criteria: aggregated_criteria,
        explanation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, label: SoftLabel, gap: Option<&str>) -> SoftGraderLlmResponse {
        SoftGraderLlmResponse {
            criteria: vec![super::super::LlmSoftCriterionResponse {
                id: id.to_string(),
                label,
                gap: gap.map(str::to_string),
            }],
            explanation: format!("sample for {id}"),
        }
    }

    #[test]
    fn partial_only_trigger_detects_partial_label() {
        let response = sample("remediation_plan", SoftLabel::Partial, Some("thin plan"));
        assert!(should_run_self_consistency(
            &response,
            SelfConsistencyTrigger::PartialOnly
        ));

        let strong = sample("remediation_plan", SoftLabel::Strong, None);
        assert!(!should_run_self_consistency(
            &strong,
            SelfConsistencyTrigger::PartialOnly
        ));
    }

    #[test]
    fn majority_vote_picks_conservative_label_on_tie() {
        let samples = vec![
            sample("remediation_plan", SoftLabel::Strong, None),
            sample("remediation_plan", SoftLabel::Partial, Some("gap")),
            sample("remediation_plan", SoftLabel::Partial, Some("other")),
        ];

        let aggregated = aggregate_soft_responses(&samples);
        assert_eq!(aggregated.criteria[0].label, SoftLabel::Partial);
    }

    #[test]
    fn strong_majority_sets_gap_to_none() {
        let samples = vec![
            sample("remediation_plan", SoftLabel::Strong, None),
            sample("remediation_plan", SoftLabel::Strong, None),
            sample("remediation_plan", SoftLabel::Partial, Some("gap")),
        ];

        let aggregated = aggregate_soft_responses(&samples);
        assert_eq!(aggregated.criteria[0].label, SoftLabel::Strong);
        assert!(aggregated.criteria[0].gap.is_none());
    }
}
