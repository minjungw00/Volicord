use super::*;

impl CoreService {
    /// Executes `harness.stage_artifact` as storage-owned transient staging.
    pub fn stage_artifact(
        &self,
        request: StageArtifactRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        if let Some(envelope_task_id) = request.envelope.task_id.as_ref() {
            if envelope_task_id != &request.task_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "task_id",
                    "envelope.task_id must match StageArtifactRequest.task_id",
                );
            }
        }

        let policy = MethodPolicy::exact(
            request.requested_access_class(),
            TaskRequirement::Exact(request.task_id.clone()),
            ReplayPolicy::None,
            FreshnessPolicy::IfPresent,
            if request.envelope.dry_run {
                MethodEffectPolicy::DryRunPreview
            } else {
                MethodEffectPolicy::Staging
            },
        );
        let mut prepared = match prepare_or_response(
            self,
            MethodName::StageArtifact,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let project_state = prepared.context.project_state.clone();
        let verified_surface = prepared.context.verified_surface.clone();

        let stage_input = match validate_stage_artifact_input(&request) {
            Ok(input) => input,
            Err(errors) => {
                return rejected_pipeline_response(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    errors,
                );
            }
        };
        if !surface_supports_artifact_staging(&verified_surface.capability_profile) {
            return rejected_pipeline_response(
                request.envelope.dry_run,
                Some(project_state.state_version),
                vec![capability_error(
                    "surface lacks manual artifact attachment support",
                    Some(json!({
                        "required_capability": "manual_artifact_attachment_supported"
                    })),
                )],
            );
        }

        if request.envelope.dry_run {
            let response = dry_run_response(
                Some(project_state.state_version),
                dry_run_summary(
                    "artifact_staging",
                    "would_stage",
                    "Stage artifact would create one transient staged handle.",
                    Vec::new(),
                ),
            );
            let response_value = serde_json::to_value(response)?;
            let response_json = serde_json::to_string(&response_value)?;
            return Ok(PipelineResponse {
                response_json,
                response_value,
                verified_surface: Some(verified_surface),
                resolved_task_id: Some(request.task_id),
                replayed: false,
            });
        }

        let handle_id = match allocate_staged_artifact_handle_id(self, &prepared.store) {
            Ok(handle_id) => handle_id,
            Err(error) => {
                return core_error_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    error,
                )
            }
        };
        let created_at = utc_timestamp(self.now());
        let expires_at = utc_timestamp(*created_at.as_datetime() + Duration::hours(24));
        let staging_record = match prepared
            .store
            .create_artifact_staging(ArtifactStagingInsert {
                handle_id: handle_id.into_inner(),
                task_id: request.task_id.as_str().to_owned(),
                created_by_surface_id: verified_surface.surface_id.as_str().to_owned(),
                created_by_surface_instance_id: verified_surface
                    .surface_instance_id
                    .as_str()
                    .to_owned(),
                display_name: request.display_name,
                content_type: request.content_type,
                sha256: stage_input.sha256.clone(),
                size_bytes: stage_input.size_bytes,
                redaction_state: redaction_state_value(request.redaction_state).to_owned(),
                relation_hint: request.relation_hint.into_option(),
                payload_kind: stage_input.payload_kind,
                safe_bytes_or_notice: stage_input.safe_bytes,
                created_at: created_at.to_string(),
                expires_at: expires_at.to_string(),
            }) {
            Ok(record) => record,
            Err(error) => {
                return rejected_pipeline_response(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    vec![store_failure_error(error)],
                )
            }
        };
        let staged_expires_at: UtcTimestamp = match parse_owner_storage_value(
            "artifact_staging",
            staging_record.handle_id.clone(),
            "expires_at",
            &staging_record.expires_at,
        ) {
            Ok(expires_at) => expires_at,
            Err(error) => {
                return core_error_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    error,
                )
            }
        };

        let resolved_task_id = TaskId::new(staging_record.task_id.clone());
        let handle = StagedArtifactHandle {
            handle_id: StagedArtifactHandleId::new(staging_record.handle_id),
            project_id: request.envelope.project_id.clone(),
            task_id: resolved_task_id.clone(),
            created_by_surface_id: SurfaceId::new(staging_record.created_by_surface_id),
            created_by_surface_instance_id: SurfaceInstanceId::new(
                staging_record.created_by_surface_instance_id,
            ),
            content_type: staging_record.content_type,
            sha256: staging_record.sha256,
            size_bytes: staging_record.size_bytes,
            redaction_state: request.redaction_state,
            expires_at: staged_expires_at.clone(),
            consumed: false,
        };
        let result = StageArtifactResult {
            base: method_result_base(
                EffectKind::StagingCreated,
                false,
                Some(project_state.state_version),
                Vec::new(),
            ),
            staged_artifact_handle: handle,
            expires_at: staged_expires_at,
        };
        let response_value = serde_json::to_value(result)?;
        let response_json = serde_json::to_string(&response_value)?;
        Ok(PipelineResponse {
            response_json,
            response_value,
            verified_surface: Some(verified_surface),
            resolved_task_id: Some(resolved_task_id),
            replayed: false,
        })
    }
}

fn validate_stage_artifact_input(
    request: &StageArtifactRequest,
) -> Result<ValidatedStageArtifactInput, Vec<volicord_types::ToolError>> {
    let mut errors = Vec::new();
    validate_stage_envelope(&request.envelope, &mut errors);
    validate_stage_text_field("task_id", request.task_id.as_str(), &mut errors);
    validate_stage_text_field("display_name", &request.display_name, &mut errors);
    validate_stage_text_field("content_type", &request.content_type, &mut errors);

    let safe_bytes = request.safe_bytes_or_notice.as_bytes().to_vec();
    if safe_bytes.len() > MAX_STAGED_BODY_BYTES {
        errors.push(stage_validation_error(
            "safe_bytes_or_notice",
            "safe_bytes_or_notice exceeds the 10 MiB staging limit",
        ));
    }
    if contains_obvious_raw_secret(&request.safe_bytes_or_notice) {
        errors.push(stage_validation_error(
            "safe_bytes_or_notice",
            "raw secret-like content must be omitted or replaced with a safe notice",
        ));
    }

    let media_type = normalized_media_type(&request.content_type);
    let textual_media_type = media_type
        .as_deref()
        .is_some_and(is_safe_textual_media_type);
    let payload_kind = if matches!(
        request.redaction_state,
        RedactionState::SecretOmitted | RedactionState::Blocked
    ) {
        StagedPayloadKind::SafeNotice
    } else if textual_media_type {
        StagedPayloadKind::SafeTextBody
    } else {
        StagedPayloadKind::SafeNotice
    };
    if media_type.is_none() {
        errors.push(stage_validation_error(
            "content_type",
            "content_type must be a valid media type",
        ));
    }
    if !textual_media_type
        && !matches!(
            request.redaction_state,
            RedactionState::SecretOmitted | RedactionState::Blocked
        )
    {
        errors.push(stage_validation_error(
            "content_type",
            "binary or unsupported content types must be represented by a safe notice",
        ));
    }

    let size_bytes = safe_bytes.len() as u64;
    if let Some(expected_size_bytes) = request.expected_size_bytes.as_ref().copied() {
        if expected_size_bytes != size_bytes {
            errors.push(stage_validation_error(
                "expected_size_bytes",
                "expected_size_bytes does not match safe_bytes_or_notice byte length",
            ));
        }
    }
    let sha256 = sha256_string(&safe_bytes);
    if let Some(expected_sha256) = request.expected_sha256.as_ref() {
        if expected_sha256.trim().is_empty() {
            errors.push(stage_validation_error(
                "expected_sha256",
                "expected_sha256 must not be empty when present",
            ));
        } else if !is_lowercase_sha256_hex(expected_sha256) {
            errors.push(stage_validation_error(
                "expected_sha256",
                "expected_sha256 must be a lowercase 64-character SHA-256 hex string",
            ));
        } else if expected_sha256 != &sha256 {
            errors.push(stage_validation_error(
                "expected_sha256",
                "expected_sha256 does not match safe_bytes_or_notice",
            ));
        }
    }

    if errors.is_empty() {
        Ok(ValidatedStageArtifactInput {
            safe_bytes,
            sha256,
            size_bytes,
            payload_kind,
        })
    } else {
        Err(errors)
    }
}

fn validate_stage_envelope(envelope: &ToolEnvelope, errors: &mut Vec<volicord_types::ToolError>) {
    validate_stage_text_field("project_id", envelope.project_id.as_str(), errors);
    if let Some(task_id) = envelope.task_id.as_ref() {
        validate_stage_text_field("envelope.task_id", task_id.as_str(), errors);
    }
    validate_stage_text_field("surface_id", envelope.surface_id.as_str(), errors);
    validate_stage_text_field("request_id", envelope.request_id.as_str(), errors);
    if let Some(idempotency_key) = envelope.idempotency_key.as_ref() {
        validate_stage_text_field("idempotency_key", idempotency_key.as_str(), errors);
    }
}

fn validate_stage_text_field(
    field: &'static str,
    value: &str,
    errors: &mut Vec<volicord_types::ToolError>,
) {
    if value.trim().is_empty() {
        errors.push(stage_validation_error(field, "field must not be empty"));
    } else if value.chars().any(char::is_control) {
        errors.push(stage_validation_error(
            field,
            "field must not contain control characters",
        ));
    }
}

fn normalized_media_type(content_type: &str) -> Option<String> {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let (top, subtype) = media_type.split_once('/')?;
    if top.is_empty()
        || subtype.is_empty()
        || media_type.chars().any(char::is_whitespace)
        || media_type.chars().any(char::is_control)
    {
        None
    } else {
        Some(media_type)
    }
}

fn is_safe_textual_media_type(media_type: &str) -> bool {
    if media_type.starts_with("text/") {
        return true;
    }
    matches!(
        media_type,
        "application/json"
            | "application/xml"
            | "application/markdown"
            | "application/x-ndjson"
            | "application/yaml"
            | "application/x-yaml"
            | "application/toml"
            | "application/javascript"
            | "application/ecmascript"
    ) || media_type.ends_with("+json")
        || media_type.ends_with("+xml")
}

fn contains_obvious_raw_secret(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "password=",
        "passwd=",
        "secret=",
        "token=",
        "api_key=",
        "apikey=",
        "aws_secret_access_key",
        "authorization: bearer ",
        "-----begin private key-----",
        "-----begin rsa private key-----",
        "-----begin openssh private key-----",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn sha256_string(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    lowercase_hex(&digest)
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn stage_validation_error(field: &'static str, message: &'static str) -> volicord_types::ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    tool_error(ErrorCode::ValidationFailed, message, false, Some(details))
}

fn capability_error(message: &'static str, details: Option<Value>) -> volicord_types::ToolError {
    let details = details.and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    tool_error(ErrorCode::CapabilityInsufficient, message, false, details)
}

fn surface_supports_artifact_staging(capability_profile: &Value) -> bool {
    surface_declares_artifact_registration(capability_profile)
        && capability_profile
            .get("manual_artifact_attachment_supported")
            .and_then(Value::as_bool)
            .or_else(|| {
                capability_profile
                    .pointer("/capabilities/manual_artifact_attachment_supported")
                    .and_then(Value::as_bool)
            })
            == Some(true)
}

fn surface_declares_artifact_registration(capability_profile: &Value) -> bool {
    if capability_profile
        .get("supported_access_classes")
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values
                .iter()
                .any(|value| value.as_str() == Some("artifact_registration"))
        })
    {
        return true;
    }
    if capability_profile
        .get("access_class")
        .and_then(Value::as_str)
        == Some("artifact_registration")
    {
        return true;
    }
    capability_profile
        .pointer("/capabilities/artifact_registration")
        .and_then(Value::as_bool)
        == Some(true)
}
