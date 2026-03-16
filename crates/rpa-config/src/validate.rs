// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>

//! Configuration validation framework
//!
//! Provides the [`Validator`] trait and built-in validators for common checks
//! such as required-field presence and type constraints. Multiple validators
//! can be composed via [`validate_config`].

use serde_json::Value;

/// Outcome of running one or more validators against a configuration value.
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// Hard errors — the configuration is unusable if any are present.
    pub errors: Vec<String>,
    /// Soft warnings — the configuration will work but may behave unexpectedly.
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create an empty (passing) result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when there are no errors.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// Trait for configuration validators.
///
/// Implement this to create custom validation logic that can be composed
/// with the built-in validators via [`validate_config`].
pub trait Validator {
    /// Validate the given configuration value and return any errors/warnings.
    fn validate(&self, config: &Value) -> ValidationResult;
}

/// Checks that a set of top-level fields are present in the config object.
///
/// Any missing field is reported as an error.
#[derive(Debug)]
pub struct RequiredFieldValidator {
    /// Field names that must exist at the top level of the config object.
    pub fields: Vec<String>,
}

impl RequiredFieldValidator {
    /// Create a validator for the given required field names.
    pub fn new(fields: Vec<String>) -> Self {
        Self { fields }
    }
}

impl Validator for RequiredFieldValidator {
    fn validate(&self, config: &Value) -> ValidationResult {
        let mut result = ValidationResult::new();
        if let Value::Object(map) = config {
            for field in &self.fields {
                if !map.contains_key(field) {
                    result
                        .errors
                        .push(format!("Required field '{}' is missing", field));
                }
            }
        } else {
            result
                .errors
                .push("Configuration must be a JSON object".into());
        }
        result
    }
}

/// Checks that specific fields have the expected JSON type.
///
/// Each entry maps a field name to the expected type name (one of
/// `"string"`, `"number"`, `"boolean"`, `"array"`, `"object"`, `"null"`).
/// Missing fields are silently skipped — use [`RequiredFieldValidator`]
/// to enforce presence.
#[derive(Debug)]
pub struct TypeValidator {
    /// (field_name, expected_type) pairs.
    pub expectations: Vec<(String, String)>,
}

impl TypeValidator {
    /// Create a validator with the given field/type expectations.
    pub fn new(expectations: Vec<(String, String)>) -> Self {
        Self { expectations }
    }
}

impl Validator for TypeValidator {
    fn validate(&self, config: &Value) -> ValidationResult {
        let mut result = ValidationResult::new();
        if let Value::Object(map) = config {
            for (field, expected_type) in &self.expectations {
                if let Some(value) = map.get(field) {
                    let actual = json_type_name(value);
                    if actual != expected_type.as_str() {
                        result.errors.push(format!(
                            "Field '{}' should be {} but is {}",
                            field, expected_type, actual
                        ));
                    }
                }
                // Missing fields are not checked here — use RequiredFieldValidator.
            }
        }
        result
    }
}

/// Run a list of validators against a configuration value, merging results.
///
/// Returns a single [`ValidationResult`] that aggregates all errors and
/// warnings from every validator.
pub fn validate_config(config: &Value, validators: &[&dyn Validator]) -> ValidationResult {
    let mut combined = ValidationResult::new();
    for validator in validators {
        combined.merge(validator.validate(config));
    }
    combined
}

/// Return the JSON type name for a value (matches Nickel/JSON terminology).
fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validation_required_fields() {
        let config = json!({"name": "test", "version": 1});
        let validator =
            RequiredFieldValidator::new(vec!["name".into(), "version".into(), "rules".into()]);
        let result = validator.validate(&config);
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("rules"));
    }

    #[test]
    fn test_validation_all_present() {
        let config = json!({"name": "test", "version": 1});
        let validator = RequiredFieldValidator::new(vec!["name".into(), "version".into()]);
        let result = validator.validate(&config);
        assert!(result.is_valid());
    }

    #[test]
    fn test_type_validator() {
        let config = json!({"name": "hello", "count": 42, "enabled": true});
        let validator = TypeValidator::new(vec![
            ("name".into(), "string".into()),
            ("count".into(), "number".into()),
            ("enabled".into(), "string".into()), // deliberate mismatch
        ]);
        let result = validator.validate(&config);
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("enabled"));
    }

    #[test]
    fn test_validate_config_composable() {
        let config = json!({"name": "test"});
        let req = RequiredFieldValidator::new(vec!["name".into(), "rules".into()]);
        let typ = TypeValidator::new(vec![("name".into(), "number".into())]);
        let result = validate_config(&config, &[&req, &typ]);
        // "rules" missing + "name" wrong type = 2 errors
        assert_eq!(result.errors.len(), 2);
    }
}
