use super::types::{StageId, StageResult};
use crate::prompts::{self, StagePipelineThresholds, StagePipelineWeights};

pub fn resolved_weights() -> StagePipelineWeights {
    prompts::stage_pipeline_weights().unwrap_or_default()
}

pub fn resolved_thresholds() -> StagePipelineThresholds {
    prompts::stage_pipeline_thresholds().unwrap_or_default()
}

/// MVP active stages S0–S3 weight sum (factuality excluded from denominator).
pub fn mvp_weight_denominator() -> u32 {
    resolved_weights().mvp_denominator()
}

/// Full pipeline weight sum when factuality is included.
pub fn full_weight_denominator() -> u32 {
    resolved_weights().full_denominator()
}

pub fn quality_refusal(is_refusal: bool) -> f32 {
    if is_refusal { 0.0 } else { 1.0 }
}

pub fn quality_gibberish(raw: u32) -> f32 {
    let clamped = raw.clamp(1, 5);
    (clamped - 1) as f32 / 4.0
}

pub fn quality_relevance(raw: u32) -> f32 {
    let clamped = raw.clamp(0, 10);
    clamped as f32 / 10.0
}

pub fn quality_domain(matches: bool) -> f32 {
    if matches { 1.0 } else { 0.0 }
}

pub fn quality_factuality(
    summary: &crate::stage_pipeline::factuality_types::FactualitySummary,
) -> f32 {
    if summary.total == 0 {
        return 0.0;
    }
    summary.supported as f32 / summary.total as f32
}

pub fn weight(stage: StageId) -> u32 {
    let weights = resolved_weights();
    match stage {
        StageId::Refusal => weights.refusal,
        StageId::Gibberish => weights.gibberish,
        StageId::Relevance => weights.relevance,
        StageId::DomainMatch => weights.domain_match,
        StageId::Factuality => weights.factuality,
    }
}

pub fn weighted_score(quality: f32, stage_weight: u32) -> u32 {
    (quality * stage_weight as f32).round() as u32
}

pub fn aggregate(stages: &[StageResult]) -> u32 {
    aggregate_with_denominator(stages, mvp_weight_denominator())
}

pub fn aggregate_with_factuality(stages: &[StageResult]) -> u32 {
    aggregate_with_denominator(stages, full_weight_denominator())
}

fn aggregate_with_denominator(stages: &[StageResult], denominator: u32) -> u32 {
    let numerator: f32 = stages
        .iter()
        .filter(|stage| !stage.skipped_due_to_gate)
        .map(|stage| stage.normalized_quality * stage.weight as f32)
        .sum();

    let total = (numerator / denominator as f32 * 100.0).round() as u32;
    total.clamp(0, 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage_pipeline::factuality_types::FactualitySummary;
    use crate::stage_pipeline::types::StageId;

    #[test]
    fn quality_refusal_no_is_good() {
        assert_eq!(quality_refusal(false), 1.0);
        assert_eq!(quality_refusal(true), 0.0);
    }

    #[test]
    fn quality_gibberish_scale() {
        assert_eq!(quality_gibberish(1), 0.0);
        assert_eq!(quality_gibberish(5), 1.0);
        assert!((quality_gibberish(3) - 0.5).abs() < f32::EPSILON);
        assert_eq!(quality_gibberish(0), 0.0);
        assert_eq!(quality_gibberish(99), 1.0);
    }

    #[test]
    fn quality_relevance_scale() {
        assert_eq!(quality_relevance(0), 0.0);
        assert_eq!(quality_relevance(10), 1.0);
        assert!((quality_relevance(6) - 0.6).abs() < f32::EPSILON);
        assert_eq!(quality_relevance(99), 1.0);
    }

    #[test]
    fn quality_domain_binary() {
        assert_eq!(quality_domain(true), 1.0);
        assert_eq!(quality_domain(false), 0.0);
    }

    #[test]
    fn weights_variant_a_from_config() {
        assert_eq!(weight(StageId::Refusal), 10);
        assert_eq!(weight(StageId::Gibberish), 15);
        assert_eq!(weight(StageId::Relevance), 20);
        assert_eq!(weight(StageId::DomainMatch), 15);
        assert_eq!(weight(StageId::Factuality), 40);
        assert_eq!(mvp_weight_denominator(), 60);
        assert_eq!(full_weight_denominator(), 100);
    }

    fn stage(id: StageId, quality: f32) -> StageResult {
        let w = weight(id);
        StageResult {
            id,
            passed: quality >= 0.5,
            raw_output: String::new(),
            normalized_quality: quality,
            weight: w,
            weighted_score: weighted_score(quality, w),
            skipped_due_to_gate: false,
            reason: None,
            details: None,
        }
    }

    #[test]
    fn aggregate_all_perfect_is_100() {
        let stages = vec![
            stage(StageId::Refusal, 1.0),
            stage(StageId::Gibberish, 1.0),
            stage(StageId::Relevance, 1.0),
            stage(StageId::DomainMatch, 1.0),
        ];
        assert_eq!(aggregate(&stages), 100);
    }

    #[test]
    fn aggregate_refusal_fail_early_exit() {
        let stages = vec![
            stage(StageId::Refusal, 0.0),
            StageResult {
                skipped_due_to_gate: true,
                normalized_quality: 0.0,
                weighted_score: 0,
                passed: false,
                ..stage(StageId::Gibberish, 0.0)
            },
            StageResult {
                skipped_due_to_gate: true,
                normalized_quality: 0.0,
                weighted_score: 0,
                passed: false,
                ..stage(StageId::Relevance, 0.0)
            },
            StageResult {
                skipped_due_to_gate: true,
                normalized_quality: 0.0,
                weighted_score: 0,
                passed: false,
                ..stage(StageId::DomainMatch, 0.0)
            },
        ];
        assert_eq!(aggregate(&stages), 0);
    }

    #[test]
    fn aggregate_partial_quality() {
        let stages = vec![
            stage(StageId::Refusal, 1.0),
            stage(StageId::Gibberish, 0.5),
            stage(StageId::Relevance, 1.0),
            stage(StageId::DomainMatch, 1.0),
        ];
        let total = aggregate(&stages);
        assert!(
            (0..100).contains(&total),
            "partial total should be in (0,100), got {total}"
        );
    }

    #[test]
    fn aggregate_with_factuality_includes_stage_weight() {
        let stages = vec![
            stage(StageId::Refusal, 1.0),
            stage(StageId::Gibberish, 1.0),
            stage(StageId::Relevance, 1.0),
            stage(StageId::DomainMatch, 1.0),
            stage(StageId::Factuality, 1.0),
        ];
        assert_eq!(aggregate_with_factuality(&stages), 100);
    }

    #[test]
    fn quality_factuality_uses_supported_ratio() {
        let summary = FactualitySummary {
            supported: 2,
            contradicted: 0,
            unverifiable: 1,
            total: 3,
        };
        assert!((quality_factuality(&summary) - 0.6666667).abs() < 0.001);
    }
}
