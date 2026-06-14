use crate::types::{CriterionDef, CriterionKind, SkillId};

const DEFI_YIELD_ROUTING: &[CriterionDef] = &[
    CriterionDef {
        id: "allocation_sum",
        description: "Evaluates whether the agent allocates exactly 10,000 CSPR across selected pools without rounding or sum errors.",
        tools: &["check_allocation_sum"],
        weight: 20,
        kind: CriterionKind::Hard,
        critical: true,
    },
    CriterionDef {
        id: "apy_math",
        description: "Evaluates mathematical accuracy of APY calculations, including compounding where applicable.",
        tools: &["validate_apy"],
        weight: 25,
        kind: CriterionKind::Hard,
        critical: true,
    },
    CriterionDef {
        id: "fee_inclusion",
        description: "Evaluates whether network fees and pool fees are included in yield and routing decisions.",
        tools: &["check_fees"],
        weight: 15,
        kind: CriterionKind::Hard,
        critical: false,
    },
    CriterionDef {
        id: "il_reasoning",
        description: "Evaluates impermanent loss reasoning and how pool volatility affects the recommended allocation.",
        tools: &["validate_il"],
        weight: 20,
        kind: CriterionKind::Hard,
        critical: false,
    },
    CriterionDef {
        id: "pool_selection",
        description: "Evaluates the logic for choosing pools given APY, liquidity depth, volume, and risk trade-offs.",
        tools: &[],
        weight: 20,
        kind: CriterionKind::Soft,
        critical: false,
    },
];

const DEFI_PROTOCOL_RISK: &[CriterionDef] = &[
    CriterionDef {
        id: "revert_rate",
        description: "Evaluates whether the agent correctly analyzes revert rates and transaction failure patterns from on-chain logs.",
        tools: &["validate_revert_rate"],
        weight: 35,
        kind: CriterionKind::Hard,
        critical: true,
    },
    CriterionDef {
        id: "risk_class",
        description: "Evaluates the accuracy and justification of the protocol safety classification (Safe / High Risk).",
        tools: &["check_risk_thresholds"],
        weight: 35,
        kind: CriterionKind::Hard,
        critical: true,
    },
    CriterionDef {
        id: "mitigation_steps",
        description: "Evaluates clarity and actionability of recommended steps to prevent loss of funds during network or protocol anomalies.",
        tools: &[],
        weight: 30,
        kind: CriterionKind::Soft,
        critical: false,
    },
];

const RWA_APPRAISAL: &[CriterionDef] = &[
    CriterionDef {
        id: "outlier_filtering",
        description: "Evaluates quality of filtering unreliable or manipulated price data from external Web2 sources.",
        tools: &["validate_outliers"],
        weight: 35,
        kind: CriterionKind::Hard,
        critical: true,
    },
    CriterionDef {
        id: "source_quality",
        description: "Evaluates whether the agent verifies source credibility and cross-checks data across providers.",
        tools: &["check_sources"],
        weight: 30,
        kind: CriterionKind::Hard,
        critical: true,
    },
    CriterionDef {
        id: "price_algorithm",
        description: "Evaluates the algorithm used to derive the final fair price for on-chain oracle updates.",
        tools: &["validate_price_derivation"],
        weight: 35,
        kind: CriterionKind::Hard,
        critical: true,
    },
];

const RWA_COMPLIANCE: &[CriterionDef] = &[
    CriterionDef {
        id: "threat_vs_fud",
        description: "Evaluates depth of contextual news analysis and ability to distinguish real threats from FUD.",
        tools: &["classify_news"],
        weight: 40,
        kind: CriterionKind::Hard,
        critical: true,
    },
    CriterionDef {
        id: "collateral_decision",
        description: "Evaluates whether the collateral factor adjustment decision is well-reasoned based on issuer risk signals.",
        tools: &["validate_collateral_logic"],
        weight: 35,
        kind: CriterionKind::Hard,
        critical: true,
    },
    CriterionDef {
        id: "remediation_plan",
        description: "Evaluates quality and completeness of the remediation plan for identified compliance or collateral risks.",
        tools: &[],
        weight: 25,
        kind: CriterionKind::Soft,
        critical: false,
    },
];

pub fn criteria(skill: SkillId) -> &'static [CriterionDef] {
    match skill {
        SkillId::DefiYieldRouting => DEFI_YIELD_ROUTING,
        SkillId::DefiProtocolRisk => DEFI_PROTOCOL_RISK,
        SkillId::RwaAppraisal => RWA_APPRAISAL,
        SkillId::RwaCompliance => RWA_COMPLIANCE,
    }
}

pub fn soft_criteria(skill: SkillId) -> Vec<&'static CriterionDef> {
    criteria(skill)
        .iter()
        .filter(|c| c.kind == CriterionKind::Soft)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_weights_sum_to_100(skill: SkillId) {
        let criteria = criteria(skill);
        let total: u32 = criteria.iter().map(|c| c.weight).sum();
        assert_eq!(total, 100, "weights for {:?} must sum to 100", skill);
    }

    #[test]
    fn defi_yield_routing_weights_sum_to_100() {
        assert_weights_sum_to_100(SkillId::DefiYieldRouting);
        let criteria = criteria(SkillId::DefiYieldRouting);
        assert_eq!(criteria.len(), 5);
    }

    #[test]
    fn defi_yield_routing_ids_and_tools_match_contract() {
        let criteria = criteria(SkillId::DefiYieldRouting);
        let expected = [
            ("allocation_sum", &["check_allocation_sum"][..]),
            ("apy_math", &["validate_apy"][..]),
            ("fee_inclusion", &["check_fees"][..]),
            ("il_reasoning", &["validate_il"][..]),
            ("pool_selection", &[][..]),
        ];

        for (criterion, (id, tools)) in criteria.iter().zip(expected.iter()) {
            assert_eq!(criterion.id, *id);
            assert_eq!(criterion.tools, *tools);
        }
    }

    #[test]
    fn defi_protocol_risk_weights_and_contract() {
        assert_weights_sum_to_100(SkillId::DefiProtocolRisk);
        let criteria = criteria(SkillId::DefiProtocolRisk);
        assert_eq!(criteria.len(), 3);
        let expected = [
            ("revert_rate", &["validate_revert_rate"][..]),
            ("risk_class", &["check_risk_thresholds"][..]),
            ("mitigation_steps", &[][..]),
        ];
        for (criterion, (id, tools)) in criteria.iter().zip(expected.iter()) {
            assert_eq!(criterion.id, *id);
            assert_eq!(criterion.tools, *tools);
        }
    }

    #[test]
    fn rwa_appraisal_weights_and_contract() {
        assert_weights_sum_to_100(SkillId::RwaAppraisal);
        let criteria = criteria(SkillId::RwaAppraisal);
        assert_eq!(criteria.len(), 3);
        let expected = [
            ("outlier_filtering", &["validate_outliers"][..]),
            ("source_quality", &["check_sources"][..]),
            ("price_algorithm", &["validate_price_derivation"][..]),
        ];
        for (criterion, (id, tools)) in criteria.iter().zip(expected.iter()) {
            assert_eq!(criterion.id, *id);
            assert_eq!(criterion.tools, *tools);
        }
    }

    #[test]
    fn rwa_compliance_weights_and_contract() {
        assert_weights_sum_to_100(SkillId::RwaCompliance);
        let criteria = criteria(SkillId::RwaCompliance);
        assert_eq!(criteria.len(), 3);
        let expected = [
            ("threat_vs_fud", &["classify_news"][..]),
            ("collateral_decision", &["validate_collateral_logic"][..]),
            ("remediation_plan", &[][..]),
        ];
        for (criterion, (id, tools)) in criteria.iter().zip(expected.iter()) {
            assert_eq!(criterion.id, *id);
            assert_eq!(criterion.tools, *tools);
        }
    }

    #[test]
    fn defi_yield_routing_critical_flags() {
        let criteria = criteria(SkillId::DefiYieldRouting);
        assert!(criteria.iter().find(|c| c.id == "allocation_sum").unwrap().critical);
        assert!(criteria.iter().find(|c| c.id == "apy_math").unwrap().critical);
        assert!(!criteria.iter().find(|c| c.id == "fee_inclusion").unwrap().critical);
        assert!(!criteria.iter().find(|c| c.id == "pool_selection").unwrap().critical);
    }

    #[test]
    fn rwa_appraisal_all_hard_and_critical() {
        let criteria = criteria(SkillId::RwaAppraisal);
        assert!(criteria.iter().all(|c| c.kind == CriterionKind::Hard && c.critical));
        assert!(soft_criteria(SkillId::RwaAppraisal).is_empty());
    }

    #[test]
    fn soft_criteria_match_llm_only_ids() {
        let soft = soft_criteria(SkillId::DefiYieldRouting);
        assert_eq!(soft.len(), 1);
        assert_eq!(soft[0].id, "pool_selection");
    }
}
