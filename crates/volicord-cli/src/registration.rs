use std::{error::Error, fmt};

use serde_json::{json, Map, Value};
use volicord_types::{
    AccessClass, SurfaceInteractionRole, BASELINE_WORKFLOW_ACCESS_CLASSES,
    VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
};

/// Default local surface registration kind for the low-level CLI command.
pub const DEFAULT_SURFACE_KIND: &str = "cli";

/// Default local surface access class for the low-level CLI command.
pub const DEFAULT_ACCESS_CLASS: AccessClass = AccessClass::ReadStatus;

/// Supported administrative registration profile.
pub const BASELINE_WORKFLOW_PROFILE: &str = "baseline-workflow";

/// Metadata used by existing low-level administrative registration commands.
pub const ADMIN_METADATA_JSON: &str = r#"{"created_by":"harness_cli_admin"}"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationMetadataError {
    Usage(String),
    Runtime(String),
}

impl RegistrationMetadataError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for RegistrationMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl Error for RegistrationMetadataError {}

/// Builds deterministic capability-profile JSON for a surface registration.
pub fn capability_profile_json(
    access_classes: &[AccessClass],
    provided: Option<&str>,
) -> Result<String, RegistrationMetadataError> {
    let mut value = match provided {
        Some(text) => serde_json::from_str::<Value>(text).map_err(|error| {
            RegistrationMetadataError::usage(format!("invalid --capability-profile JSON: {error}"))
        })?,
        None => Value::Object(Map::new()),
    };

    let Some(object) = value.as_object_mut() else {
        return Err(RegistrationMetadataError::usage(
            "--capability-profile must be a JSON object",
        ));
    };
    let primary = primary_access_class(access_classes)?;
    object.insert("access_class".to_owned(), json!(primary.as_str()));
    object
        .entry("supported_access_classes".to_owned())
        .or_insert_with(|| json!(access_class_strings(access_classes)));

    serde_json::to_string(&value).map_err(|error| {
        RegistrationMetadataError::runtime(format!("failed to encode capability profile: {error}"))
    })
}

/// Builds deterministic local-access metadata JSON for a surface registration.
pub fn local_access_json(
    access_classes: &[AccessClass],
) -> Result<String, RegistrationMetadataError> {
    primary_access_class(access_classes)?;
    serde_json::to_string(&json!({
        "authorized_access_classes": access_class_strings(access_classes),
        "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
    }))
    .map_err(|error| {
        RegistrationMetadataError::runtime(format!(
            "failed to encode local access metadata: {error}"
        ))
    })
}

/// Parses registered local-access metadata into a de-duplicated access set.
pub fn normalized_access_classes_from_local_access(
    text: &str,
) -> Result<Vec<AccessClass>, RegistrationMetadataError> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| {
        RegistrationMetadataError::usage(format!("invalid local access JSON: {error}"))
    })?;
    let Some(object) = value.as_object() else {
        return Err(RegistrationMetadataError::usage(
            "local access metadata must be a JSON object",
        ));
    };
    if object.contains_key("access_class") {
        return Err(RegistrationMetadataError::usage(
            "local access metadata must not include access_class",
        ));
    }

    let values = object
        .get("authorized_access_classes")
        .ok_or_else(|| {
            RegistrationMetadataError::usage(
                "local access metadata must include authorized_access_classes",
            )
        })?
        .as_array()
        .ok_or_else(|| {
            RegistrationMetadataError::usage("authorized_access_classes must be an array")
        })?;
    let mut access_classes = Vec::new();
    for value in values {
        let Some(raw) = value.as_str() else {
            return Err(RegistrationMetadataError::usage(
                "authorized_access_classes entries must be strings",
            ));
        };
        push_access_class(&mut access_classes, parse_access_class(raw)?);
    }
    if access_classes.is_empty() {
        return Err(RegistrationMetadataError::usage(
            "authorized_access_classes must not be empty",
        ));
    }
    match object.get("verification_basis") {
        Some(Value::String(value)) if !value.trim().is_empty() => (),
        Some(_) => {
            return Err(RegistrationMetadataError::usage(
                "verification_basis must be a non-empty string",
            ))
        }
        None => {
            return Err(RegistrationMetadataError::usage(
                "local access metadata must include verification_basis",
            ))
        }
    }

    Ok(access_classes)
}

/// Returns the display value used by the existing low-level list/register output.
pub fn access_class_from_local_access(text: &str) -> Option<String> {
    normalized_access_classes_from_local_access(text)
        .ok()
        .map(|access_classes| {
            access_classes
                .iter()
                .map(|access_class| access_class.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
}

/// Validates role-specific local-access constraints.
pub fn validate_role_access_classes(
    role: SurfaceInteractionRole,
    access_classes: &[AccessClass],
) -> Result<(), RegistrationMetadataError> {
    if role != SurfaceInteractionRole::UserInteraction {
        return Ok(());
    }
    if !access_classes.contains(&AccessClass::CoreMutation) {
        return Err(RegistrationMetadataError::usage(
            "user_interaction surfaces require core_mutation access",
        ));
    }
    if access_classes.iter().any(|access_class| {
        !matches!(
            access_class,
            AccessClass::ReadStatus | AccessClass::CoreMutation
        )
    }) {
        return Err(RegistrationMetadataError::usage(
            "user_interaction surfaces may grant only read_status and core_mutation access",
        ));
    }
    Ok(())
}

pub fn parse_access_class(value: &str) -> Result<AccessClass, RegistrationMetadataError> {
    serde_json::from_value(Value::String(value.to_owned()))
        .map_err(|_| RegistrationMetadataError::usage(format!("unknown access class: {value}")))
}

pub fn push_access_classes<const N: usize>(
    target: &mut Vec<AccessClass>,
    values: [AccessClass; N],
) {
    for value in values {
        push_access_class(target, value);
    }
}

pub fn push_access_class(target: &mut Vec<AccessClass>, value: AccessClass) {
    if !target.contains(&value) {
        target.push(value);
    }
}

pub fn primary_access_class(
    access_classes: &[AccessClass],
) -> Result<AccessClass, RegistrationMetadataError> {
    access_classes.first().copied().ok_or_else(|| {
        RegistrationMetadataError::usage("surface registration requires at least one access class")
    })
}

pub fn access_class_strings(access_classes: &[AccessClass]) -> Vec<&'static str> {
    access_classes.iter().map(|value| value.as_str()).collect()
}

pub fn access_classes_match(actual: &[AccessClass], expected: &[AccessClass]) -> bool {
    actual.len() == expected.len() && expected.iter().all(|value| actual.contains(value))
}

pub fn baseline_workflow_access_classes() -> Vec<AccessClass> {
    BASELINE_WORKFLOW_ACCESS_CLASSES.to_vec()
}

pub fn user_interaction_access_classes() -> Vec<AccessClass> {
    vec![AccessClass::ReadStatus, AccessClass::CoreMutation]
}

pub fn parse_json_object(field: &str, text: &str) -> Result<Value, RegistrationMetadataError> {
    let value = serde_json::from_str::<Value>(text).map_err(|error| {
        RegistrationMetadataError::usage(format!("{field} must be JSON object text: {error}"))
    })?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(RegistrationMetadataError::usage(format!(
            "{field} must be a JSON object"
        )))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn local_access_json_uses_array_grant_only() {
        let text =
            local_access_json(&[AccessClass::ReadStatus, AccessClass::CoreMutation]).unwrap();
        let value = serde_json::from_str::<Value>(&text).unwrap();

        assert!(value.get("access_class").is_none());
        assert_eq!(
            value["authorized_access_classes"],
            json!(["read_status", "core_mutation"])
        );
        assert_eq!(
            value["verification_basis"],
            VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
        );
    }

    #[test]
    fn local_access_parser_rejects_obsolete_and_incomplete_grants() {
        for grant in [
            json!({"access_class": "read_status"}),
            json!({
                "access_class": "read_status",
                "authorized_access_classes": ["read_status"],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
            json!({"verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION}),
            json!({
                "authorized_access_classes": "read_status",
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
            json!({
                "authorized_access_classes": [],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
            json!({
                "authorized_access_classes": ["unknown"],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            }),
            json!({"authorized_access_classes": ["read_status"]}),
            json!({
                "authorized_access_classes": ["read_status"],
                "verification_basis": ""
            }),
        ] {
            assert!(
                normalized_access_classes_from_local_access(&grant.to_string()).is_err(),
                "grant should be rejected: {grant}"
            );
            assert!(access_class_from_local_access(&grant.to_string()).is_none());
        }
    }
}
