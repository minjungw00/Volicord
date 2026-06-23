use serde_json::{Map, Value};
use volicord_store::{
    bootstrap::SurfaceRecord, core_pipeline::CoreProjectStore, core_pipeline::ProjectStateHeader,
    StoreError,
};
use volicord_types::{
    AccessClass, ErrorCode, ProjectId, SurfaceId, SurfaceInstanceId, SurfaceInteractionRole,
    ToolEnvelope, ToolError, VERIFICATION_BASIS_CLI_DIRECT_SURFACE_BINDING,
    VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION, VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING,
    VERIFICATION_BASIS_TEST_FIXTURE_BINDING,
};

use crate::pipeline::{
    store_failure_error, tool_error, InvocationContext, MethodAccessPolicy, VerifiedSurfaceContext,
};

pub(crate) fn method_access_error(
    policy: MethodAccessPolicy,
    verified_surface: &VerifiedSurfaceContext,
) -> Option<ToolError> {
    match policy {
        MethodAccessPolicy::Exact(required_access_class)
            if verified_surface.access_class != required_access_class =>
        {
            Some(access_class_mismatch_error(
                required_access_class,
                verified_surface.access_class,
            ))
        }
        MethodAccessPolicy::Exact(_) => None,
    }
}

pub(crate) fn derive_verified_surface(
    store: &CoreProjectStore,
    _project_state: &ProjectStateHeader,
    _envelope: &ToolEnvelope,
    invocation: &InvocationContext,
) -> Result<VerifiedSurfaceContext, ToolError> {
    let surface = store
        .surface(
            &invocation.binding.surface_id,
            invocation.binding.surface_instance_id.as_str(),
        )
        .map_err(store_failure_error)?
        .ok_or_else(|| local_access_mismatch_error("adapter_session_binding"))?;

    let capability_profile = serde_json::from_str::<Value>(&surface.capability_profile_json)
        .map_err(|_| {
            store_failure_error(StoreError::corrupt_stored_json(
                "project_state",
                "surfaces.capability_profile_json",
            ))
        })?;

    verified_surface_from_registered_surface(surface, invocation, capability_profile)
}

pub(crate) fn verified_surface_from_registered_surface(
    surface: SurfaceRecord,
    invocation: &InvocationContext,
    capability_profile: Value,
) -> Result<VerifiedSurfaceContext, ToolError> {
    let grant = parse_registered_local_access_grant(&surface.local_access_json).map_err(
        |error| match error {
            RegisteredLocalAccessGrantError::InvalidJson => store_failure_error(
                StoreError::corrupt_stored_json("project_state", "surfaces.local_access_json"),
            ),
            RegisteredLocalAccessGrantError::InvalidShape => store_failure_error(
                StoreError::corrupt_stored_value("project_state", "surfaces.local_access_json"),
            ),
        },
    )?;
    if !grant
        .authorized_access_classes
        .contains(&invocation.requested_access_class)
    {
        return Err(local_access_mismatch_error("surfaces.local_access_json"));
    }
    let interaction_role =
        parse_surface_interaction_role(&surface.interaction_role).map_err(|_| {
            store_failure_error(StoreError::corrupt_stored_value(
                "project_state",
                "surfaces.interaction_role",
            ))
        })?;

    Ok(VerifiedSurfaceContext {
        project_id: ProjectId::new(surface.project_id),
        surface_id: SurfaceId::new(surface.surface_id),
        surface_instance_id: SurfaceInstanceId::new(surface.surface_instance_id),
        access_class: invocation.requested_access_class,
        capability_profile,
        verification_basis: verified_surface_basis(
            &grant.verification_basis,
            &invocation.binding.invocation_binding_basis,
        ),
        interaction_role,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisteredLocalAccessGrant {
    pub authorized_access_classes: Vec<AccessClass>,
    pub verification_basis: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegisteredLocalAccessGrantError {
    InvalidJson,
    InvalidShape,
}

pub(crate) fn parse_registered_local_access_grant(
    text: &str,
) -> Result<RegisteredLocalAccessGrant, RegisteredLocalAccessGrantError> {
    let value = serde_json::from_str::<Value>(text)
        .map_err(|_| RegisteredLocalAccessGrantError::InvalidJson)?;
    let object = value
        .as_object()
        .ok_or(RegisteredLocalAccessGrantError::InvalidShape)?;
    if object.contains_key("access_class") {
        return Err(RegisteredLocalAccessGrantError::InvalidShape);
    }
    let authorized_access_classes = parse_authorized_access_classes(
        object
            .get("authorized_access_classes")
            .ok_or(RegisteredLocalAccessGrantError::InvalidShape)?,
    )?;

    let verification_basis = match object.get("verification_basis") {
        Some(Value::String(value)) if !value.trim().is_empty() => value.clone(),
        Some(_) | None => return Err(RegisteredLocalAccessGrantError::InvalidShape),
    };

    Ok(RegisteredLocalAccessGrant {
        authorized_access_classes,
        verification_basis,
    })
}

fn parse_authorized_access_classes(
    value: &Value,
) -> Result<Vec<AccessClass>, RegisteredLocalAccessGrantError> {
    let values = value
        .as_array()
        .ok_or(RegisteredLocalAccessGrantError::InvalidShape)?;

    let mut access_classes = Vec::new();
    for value in values {
        let access_class = parse_access_class_field(value)?;
        if !access_classes.contains(&access_class) {
            access_classes.push(access_class);
        }
    }
    if access_classes.is_empty() {
        return Err(RegisteredLocalAccessGrantError::InvalidShape);
    }
    Ok(access_classes)
}

fn parse_access_class_field(value: &Value) -> Result<AccessClass, RegisteredLocalAccessGrantError> {
    let value = value
        .as_str()
        .ok_or(RegisteredLocalAccessGrantError::InvalidShape)?;
    if value.trim().is_empty() {
        return Err(RegisteredLocalAccessGrantError::InvalidShape);
    }
    match value {
        "read_status" => Ok(AccessClass::ReadStatus),
        "core_mutation" => Ok(AccessClass::CoreMutation),
        "write_authorization" => Ok(AccessClass::WriteAuthorization),
        "run_recording" => Ok(AccessClass::RunRecording),
        "artifact_registration" => Ok(AccessClass::ArtifactRegistration),
        "artifact_read" => Ok(AccessClass::ArtifactRead),
        _ => Err(RegisteredLocalAccessGrantError::InvalidShape),
    }
}

fn parse_surface_interaction_role(value: &str) -> Result<SurfaceInteractionRole, ()> {
    match value {
        "agent" => Ok(SurfaceInteractionRole::Agent),
        "user_interaction" => Ok(SurfaceInteractionRole::UserInteraction),
        _ => Err(()),
    }
}

fn verified_surface_basis(registered_basis: &str, invocation_binding_basis: &str) -> String {
    let registered_basis = controlled_registration_basis(registered_basis);
    match controlled_binding_basis(invocation_binding_basis) {
        Some(invocation_binding_basis) => format!("{registered_basis}:{invocation_binding_basis}"),
        None => registered_basis.to_owned(),
    }
}

fn controlled_registration_basis(value: &str) -> &'static str {
    match value.trim() {
        VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION => VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
        _ => VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION,
    }
}

fn controlled_binding_basis(value: &str) -> Option<&'static str> {
    match value.trim() {
        VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING => {
            Some(VERIFICATION_BASIS_MCP_STDIO_SURFACE_BINDING)
        }
        VERIFICATION_BASIS_CLI_DIRECT_SURFACE_BINDING => {
            Some(VERIFICATION_BASIS_CLI_DIRECT_SURFACE_BINDING)
        }
        VERIFICATION_BASIS_TEST_FIXTURE_BINDING => Some(VERIFICATION_BASIS_TEST_FIXTURE_BINDING),
        _ => None,
    }
}

pub(crate) fn access_class_value(access_class: AccessClass) -> &'static str {
    access_class.as_str()
}

pub(crate) fn local_access_mismatch_error(field: &'static str) -> ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    tool_error(
        ErrorCode::LocalAccessMismatch,
        "local surface context does not match the registered surface",
        false,
        Some(details),
    )
}

fn access_class_mismatch_error(
    required_access_class: AccessClass,
    actual_access_class: AccessClass,
) -> ToolError {
    let mut details = Map::new();
    details.insert(
        "field".to_owned(),
        Value::String("invocation.access_class".to_owned()),
    );
    details.insert(
        "required_access_class".to_owned(),
        serde_json::to_value(required_access_class).unwrap_or(Value::Null),
    );
    details.insert(
        "actual_access_class".to_owned(),
        serde_json::to_value(actual_access_class).unwrap_or(Value::Null),
    );
    tool_error(
        ErrorCode::LocalAccessMismatch,
        "local surface context does not match the method-derived access class",
        false,
        Some(details),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn registered_local_access_grant_accepts_current_array_shape() {
        let grant = parse_registered_local_access_grant(
            &json!({
                "authorized_access_classes": [
                    "read_status",
                    "core_mutation",
                    "read_status"
                ],
                "verification_basis": VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            grant.authorized_access_classes,
            vec![AccessClass::ReadStatus, AccessClass::CoreMutation]
        );
        assert_eq!(
            grant.verification_basis,
            VERIFICATION_BASIS_LOCAL_ADMIN_REGISTRATION
        );
    }

    #[test]
    fn registered_local_access_grant_rejects_obsolete_or_incomplete_shapes() {
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
            assert_eq!(
                parse_registered_local_access_grant(&grant.to_string()),
                Err(RegisteredLocalAccessGrantError::InvalidShape),
                "grant should be rejected: {grant}"
            );
        }
    }
}
