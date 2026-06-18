use std::path::PathBuf;
use std::sync::OnceLock;

use jsonschema::{Draft, Validator};
use serde_json::Value;

use crate::types::{SkillId, ValidatorError};

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas")
}

fn compile_schema(filename: &str) -> Result<Validator, ValidatorError> {
    let path = schemas_dir().join(filename);
    let content = std::fs::read_to_string(&path).map_err(|e| {
        ValidatorError::Fixture(format!("failed to read schema {}: {e}", path.display()))
    })?;
    let schema: Value = serde_json::from_str(&content).map_err(|e| {
        ValidatorError::Fixture(format!("invalid schema JSON {}: {e}", path.display()))
    })?;
    jsonschema::options()
        .with_draft(Draft::Draft7)
        .build(&schema)
        .map_err(|e| {
            ValidatorError::Fixture(format!("schema compile failed for {}: {e}", path.display()))
        })
}

fn meta_schema() -> Result<&'static Validator, ValidatorError> {
    static SCHEMA: OnceLock<Result<Validator, String>> = OnceLock::new();
    let result =
        SCHEMA.get_or_init(|| compile_schema("_meta.schema.json").map_err(|e| e.to_string()));
    match result {
        Ok(schema) => Ok(schema),
        Err(msg) => Err(ValidatorError::Fixture(msg.clone())),
    }
}

fn skill_schema(skill: SkillId) -> Result<&'static Validator, ValidatorError> {
    static DEFI_YIELD: OnceLock<Result<Validator, String>> = OnceLock::new();
    static DEFI_PROTOCOL: OnceLock<Result<Validator, String>> = OnceLock::new();
    static RWA_APPRAISAL: OnceLock<Result<Validator, String>> = OnceLock::new();
    static RWA_COMPLIANCE: OnceLock<Result<Validator, String>> = OnceLock::new();

    let (cell, filename) = match skill {
        SkillId::DefiYieldRouting => (&DEFI_YIELD, "defi_yield_routing.schema.json"),
        SkillId::DefiProtocolRisk => (&DEFI_PROTOCOL, "defi_protocol_risk.schema.json"),
        SkillId::RwaAppraisal => (&RWA_APPRAISAL, "rwa_appraisal.schema.json"),
        SkillId::RwaCompliance => (&RWA_COMPLIANCE, "rwa_compliance.schema.json"),
    };

    let result = cell.get_or_init(|| compile_schema(filename).map_err(|e| e.to_string()));
    match result {
        Ok(schema) => Ok(schema),
        Err(msg) => Err(ValidatorError::Fixture(msg.clone())),
    }
}

#[allow(dead_code)]
fn skill_schema_file(skill: SkillId) -> &'static str {
    match skill {
        SkillId::DefiYieldRouting => "defi_yield_routing.schema.json",
        SkillId::DefiProtocolRisk => "defi_protocol_risk.schema.json",
        SkillId::RwaAppraisal => "rwa_appraisal.schema.json",
        SkillId::RwaCompliance => "rwa_compliance.schema.json",
    }
}

/// Returns true when the value looks like a meta envelope (`data` + `source` or `captured_at`).
pub fn is_fixture_envelope(value: &Value) -> bool {
    value.get("data").map(Value::is_object).unwrap_or(false)
        && (value.get("source").is_some() || value.get("captured_at").is_some())
}

fn validation_errors(schema: &Validator, value: &Value) -> Result<(), ValidatorError> {
    let errors: Vec<String> = schema.iter_errors(value).map(|e| e.to_string()).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidatorError::Fixture(errors.join("; ")))
    }
}

/// Validate and normalize a fixture for the given skill.
///
/// Accepts either a raw skill payload (backward compatible with static fixtures)
/// or a meta envelope `{ captured_at?, source?, data }`. Returns the unwrapped
/// skill payload on success.
pub fn validate_fixture(skill: SkillId, value: &Value) -> Result<Value, ValidatorError> {
    if !value.is_object() {
        return Err(ValidatorError::Fixture(
            "fixture must be a JSON object".to_string(),
        ));
    }

    let payload = if is_fixture_envelope(value) {
        validation_errors(meta_schema()?, value)?;
        value
            .get("data")
            .cloned()
            .ok_or_else(|| ValidatorError::Fixture("envelope missing data object".to_string()))?
    } else {
        value.clone()
    };

    validation_errors(skill_schema(skill)?, &payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::load_fixture;

    #[test]
    fn static_fixtures_pass_validation() {
        for skill in [
            SkillId::DefiYieldRouting,
            SkillId::DefiProtocolRisk,
            SkillId::RwaAppraisal,
            SkillId::RwaCompliance,
        ] {
            let fixture_file = format!("{}.json", skill.as_str());
            let fixture = load_fixture(&fixture_file).expect("fixture load");
            validate_fixture(skill, &fixture).expect("valid static fixture");
        }
    }

    #[test]
    fn envelope_unwraps_data() {
        let raw = load_fixture("defi_yield_routing.json").expect("fixture");
        let envelope = serde_json::json!({
            "captured_at": "2026-06-15T12:00:00Z",
            "source": "seed",
            "data": raw
        });
        let normalized =
            validate_fixture(SkillId::DefiYieldRouting, &envelope).expect("envelope ok");
        assert_eq!(normalized["amount_cspr"], 10000);
    }

    #[test]
    fn missing_required_field_fails() {
        let invalid = serde_json::json!({ "amount_cspr": 10000 });
        let err = validate_fixture(SkillId::DefiYieldRouting, &invalid).unwrap_err();
        assert!(matches!(err, ValidatorError::Fixture(_)));
    }

    #[test]
    fn is_fixture_envelope_detects_meta_wrapper() {
        let envelope = serde_json::json!({
            "source": "creator",
            "data": { "amount_cspr": 1 }
        });
        assert!(is_fixture_envelope(&envelope));
        assert!(!is_fixture_envelope(
            &serde_json::json!({ "amount_cspr": 1 })
        ));
    }

    #[test]
    fn skill_schema_files_exist() {
        for skill in [
            SkillId::DefiYieldRouting,
            SkillId::DefiProtocolRisk,
            SkillId::RwaAppraisal,
            SkillId::RwaCompliance,
        ] {
            let path = schemas_dir().join(skill_schema_file(skill));
            assert!(path.exists(), "missing schema {}", path.display());
        }
    }
}
