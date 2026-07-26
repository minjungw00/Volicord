use super::*;

const CURSOR_DOMAIN: &[u8] = b"volicord.operation-result.cursor\0";

impl CoreService {
    /// Reads one bounded page of an immutable historical operation result.
    pub fn get_operation_result(
        &self,
        request: GetOperationResultRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        if request.envelope.dry_run {
            return validation_rejected(
                false,
                None,
                "dry_run",
                "operation-result retrieval requires dry_run=false",
            );
        }
        if request.envelope.idempotency_key.is_some() {
            return validation_rejected(
                false,
                None,
                "idempotency_key",
                "operation-result retrieval requires idempotency_key=null",
            );
        }
        if request.envelope.expected_state_version.is_some() {
            return validation_rejected(
                false,
                None,
                "expected_state_version",
                "operation-result retrieval requires expected_state_version=null",
            );
        }
        if !valid_response_sha256(&request.operation_result_ref.response_sha256) {
            return validation_rejected(
                false,
                None,
                "operation_result_ref.response_sha256",
                "response_sha256 must use sha256: followed by 64 lowercase hexadecimal digits",
            );
        }
        if request.operation_result_ref.response_size_bytes == 0 {
            return validation_rejected(
                false,
                None,
                "operation_result_ref.response_size_bytes",
                "response_size_bytes must be greater than zero",
            );
        }

        let parsed_cursor = match request.cursor.as_ref() {
            Some(cursor) => match parse_cursor(cursor) {
                Ok(cursor) => Some(cursor),
                Err(()) => {
                    return validation_rejected(false, None, "cursor", "cursor is malformed")
                }
            },
            None => None,
        };

        let request_json = serde_json::to_value(&request)?;
        let prepared = match self.prepare_request(
            None,
            PipelinePreflightRequest {
                method_name: MethodName::GetOperationResult,
                envelope: request.envelope.clone(),
                request_json,
                invocation,
                policy: MethodPolicy::exact(
                    request.operation_category(),
                    TaskRequirement::None,
                    ReplayPolicy::None,
                    FreshnessPolicy::None,
                    MethodEffectPolicy::ReadOnly,
                ),
            },
        )? {
            PipelinePreflightOutcome::Prepared(prepared) => *prepared,
            PipelinePreflightOutcome::Response(response) => return Ok(*response),
        };
        let state_version = prepared.context.project_state.state_version;

        if request.envelope.project_id != request.operation_result_ref.project_id {
            return operation_result_rejected(
                &prepared,
                ErrorCode::InvocationContextMismatch,
                "operation-result project context does not match",
            );
        }

        let stored = match prepared.store.operation_result(
            request.operation_result_ref.source_method,
            &request.operation_result_ref.source_idempotency_key,
        ) {
            Ok(Some(stored)) => stored,
            Ok(None) => {
                return operation_result_rejected(
                    &prepared,
                    ErrorCode::OperationResultUnavailable,
                    "operation result is unavailable",
                )
            }
            Err(error) => {
                return core_error_response(
                    &request.envelope,
                    Some(state_version),
                    CorePipelineError::Store(error),
                )
            }
        };

        if stored.operation_category != OperationCategory::AgentWorkflow.as_str() {
            return operation_result_rejected(
                &prepared,
                ErrorCode::OperationResultUnavailable,
                "operation result is unavailable",
            );
        }
        if stored.actor_source
            != prepared
                .context
                .verified_invocation
                .actor_source
                .to_canonical_string()
        {
            return operation_result_rejected(
                &prepared,
                ErrorCode::InvocationContextMismatch,
                "originating actor context does not match",
            );
        }
        if !stored_matches_ref(&stored, &request.operation_result_ref) {
            return operation_result_rejected(
                &prepared,
                ErrorCode::OperationResultUnavailable,
                "operation result is unavailable",
            );
        }
        if !crate::pipeline::stored_public_response_is_current(
            request.operation_result_ref.source_method,
            &stored.response_json,
            stored.committed_state_version,
        ) {
            return operation_result_rejected(
                &prepared,
                ErrorCode::OperationResultUnavailable,
                "operation result is unavailable",
            );
        }

        let start_offset = match parsed_cursor {
            Some(cursor) if cursor_matches_ref(&cursor, &request.operation_result_ref) => {
                cursor.offset
            }
            Some(_) => {
                return operation_result_rejected(
                    &prepared,
                    ErrorCode::OperationResultUnavailable,
                    "operation result cursor does not match",
                )
            }
            None => 0,
        };
        let start = match usize::try_from(start_offset) {
            Ok(start)
                if start < stored.response_json.len()
                    && stored.response_json.is_char_boundary(start) =>
            {
                start
            }
            _ => {
                return operation_result_rejected(
                    &prepared,
                    ErrorCode::OperationResultUnavailable,
                    "operation result cursor does not match",
                )
            }
        };
        let mut end = stored
            .response_json
            .len()
            .min(start.saturating_add(MAX_OPERATION_RESULT_PAGE_BYTES));
        while end > start && !stored.response_json.is_char_boundary(end) {
            end -= 1;
        }
        let chunk_utf8 = stored.response_json[start..end].to_owned();
        let complete = end == stored.response_json.len();
        let next_cursor = if complete {
            RequiredNullable::null()
        } else {
            RequiredNullable::some(cursor_for_ref(&request.operation_result_ref, end as u64))
        };
        let result_fields = GetOperationResultResultFields {
            operation_result_ref: request.operation_result_ref,
            start_offset_bytes: start as u64,
            end_offset_bytes: end as u64,
            chunk_utf8,
            next_cursor,
            complete,
            historical: true,
            current_authority_refresh_required: true,
        };
        self.execute_prepared_request(prepared, OwnerPipelineBranch::ReadOnly { result_fields })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCursor {
    offset: u64,
    digest: String,
}

fn parse_cursor(cursor: &str) -> Result<ParsedCursor, ()> {
    let mut parts = cursor.split('.');
    let offset = parts.next().ok_or(())?;
    let digest = parts.next().ok_or(())?;
    if parts.next().is_some()
        || offset.is_empty()
        || !offset.bytes().all(|byte| byte.is_ascii_digit())
        || (offset.len() > 1 && offset.starts_with('0'))
        || !is_canonical_sha256_hex(digest)
    {
        return Err(());
    }
    Ok(ParsedCursor {
        offset: offset.parse().map_err(|_| ())?,
        digest: digest.to_owned(),
    })
}

fn cursor_for_ref(operation_result_ref: &OperationResultRef, offset: u64) -> String {
    format!("{offset}.{}", cursor_digest(operation_result_ref, offset))
}

fn cursor_matches_ref(cursor: &ParsedCursor, operation_result_ref: &OperationResultRef) -> bool {
    cursor.digest == cursor_digest(operation_result_ref, cursor.offset)
}

fn cursor_digest(operation_result_ref: &OperationResultRef, offset: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CURSOR_DOMAIN);
    hash_cursor_component(&mut hasher, operation_result_ref.project_id.as_str());
    hash_cursor_component(&mut hasher, operation_result_ref.source_method.as_str());
    hash_cursor_component(
        &mut hasher,
        operation_result_ref.source_idempotency_key.as_str(),
    );
    hasher.update(operation_result_ref.committed_state_version.to_be_bytes());
    hash_cursor_component(&mut hasher, &operation_result_ref.response_sha256);
    hasher.update(operation_result_ref.response_size_bytes.to_be_bytes());
    hasher.update(offset.to_be_bytes());
    format!("{:x}", hasher.finalize())
}

fn hash_cursor_component(hasher: &mut Sha256, component: &str) {
    hasher.update((component.len() as u64).to_be_bytes());
    hasher.update(component.as_bytes());
}

fn valid_response_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn stored_matches_ref(
    stored: &volicord_store::core_pipeline::StoredOperationResult,
    operation_result_ref: &OperationResultRef,
) -> bool {
    stored.project_id == operation_result_ref.project_id.as_str()
        && stored.source_method == operation_result_ref.source_method.as_str()
        && stored.source_idempotency_key == operation_result_ref.source_idempotency_key.as_str()
        && stored.committed_state_version == operation_result_ref.committed_state_version
        && stored.response_sha256 == operation_result_ref.response_sha256
        && stored.response_size_bytes == operation_result_ref.response_size_bytes
}

fn operation_result_rejected(
    prepared: &PreparedRequest,
    code: ErrorCode,
    message: &'static str,
) -> CoreResult<PipelineResponse> {
    let response = rejected_response(
        false,
        Some(prepared.context.project_state.state_version),
        vec![tool_error(code, message, false, None)],
    );
    let response_value = serde_json::to_value(response)?;
    let response_json = serde_json::to_string(&response_value)?;
    Ok(PipelineResponse {
        response_json,
        response_value,
        operation_result_ref: None,
        verified_invocation: Some(prepared.context.verified_invocation.clone()),
        resolved_task_id: None,
        replayed: false,
    })
}
