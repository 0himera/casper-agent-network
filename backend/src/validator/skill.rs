use validator_engine::SkillId;

/// Map a backend skill string (v2 id or legacy domain alias) to `SkillId`.
pub fn map_skill(skill: &str) -> Option<SkillId> {
    match skill {
        "defi_yield_routing" | "defi_analysis" => Some(SkillId::DefiYieldRouting),
        "defi_protocol_risk" => Some(SkillId::DefiProtocolRisk),
        "rwa_appraisal" | "rwa_valuation" => Some(SkillId::RwaAppraisal),
        "rwa_compliance" => Some(SkillId::RwaCompliance),
        _ => None,
    }
}

/// Resolve canonical v2 skill: explicit `skill_id` takes priority over legacy `domain`.
pub fn resolve_skill(skill_id: Option<&str>, domain: &str) -> Option<SkillId> {
    if let Some(id) = skill_id {
        if let Some(skill) = map_skill(id) {
            return Some(skill);
        }
    }
    map_skill(domain)
}

/// Resolved skill id as snake_case string for adapter calls.
pub fn resolve_skill_str(skill_id: Option<&str>, domain: &str) -> Option<String> {
    resolve_skill(skill_id, domain).map(|s| s.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_skill_prefers_skill_id() {
        let skill = resolve_skill(Some("rwa_appraisal"), "defi_analysis");
        assert_eq!(skill, Some(SkillId::RwaAppraisal));
    }

    #[test]
    fn resolve_skill_falls_back_to_domain() {
        let skill = resolve_skill(None, "defi_analysis");
        assert_eq!(skill, Some(SkillId::DefiYieldRouting));
    }

    #[test]
    fn resolve_skill_returns_none_for_unsupported() {
        assert_eq!(resolve_skill(None, "code_review"), None);
    }
}
