use super::*;

const LOCAL_WEB_CONSENT_FALLBACK_KIND: &str = "local_web_consent";
const LOCAL_WEB_CONSENT_DELIVERY_SURFACE: &str = "model_invisible_user_surface";
const LOCAL_WEB_CONSENT_ENDPOINT: &str = "/consent";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalWebConsentCreatedMetadata {
    fallback_kind: String,
    delivery_surface: String,
    endpoint: String,
    form_digest: String,
}

impl CoreService {
    /// Executes `volicord.request_user_action` through the shared Core mutation pipeline.
    pub fn request_user_action(
        &self,
        request: RequestUserActionRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        execute_request_user_action(self, request, invocation)
    }

    /// Resolves one pending action from a verified User Channel invocation.
    pub fn resolve_user_action(
        &self,
        request: ResolveUserActionRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        execute_resolve_user_action(self, request, invocation, None)
    }

    /// Resolves one local-web action and consumes its bearer token atomically.
    pub fn resolve_local_web_consent_user_action(
        &self,
        request: LocalWebConsentUserActionRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        if request.request.envelope.dry_run {
            return validation_rejected(
                true,
                None,
                "envelope.dry_run",
                "local-web user-action resolution does not support dry_run",
            );
        }
        let LocalWebConsentUserActionRequest {
            request,
            token,
            expected_connection_internal_id,
            completion_metadata_json,
        } = request;
        execute_resolve_user_action(
            self,
            request,
            invocation,
            Some(LocalWebTokenContext {
                token,
                expected_connection_internal_id,
                completion_metadata_json,
            }),
        )
    }

    /// Projects the complete canonical local-web form for one bearer token.
    ///
    /// The adapter supplies the raw presented credential and the exact token
    /// record returned by its non-recording validation. Core then rereads that
    /// record and all project-local authority in one read snapshot before
    /// returning this nonserialized User Channel value.
    pub fn local_web_consent_user_action_projection(
        &self,
        request: LocalWebConsentUserActionProjectionRequest,
    ) -> CoreResult<LocalWebConsentUserActionProjectionOutcome> {
        let LocalWebConsentUserActionProjectionRequest {
            token,
            validated_token,
            allow_resolved_replay,
        } = request;
        let Ok(token_hash) = user_action_channel_token_hash(&token) else {
            return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
        };
        if token_hash != validated_token.token_hash
            || validated_token.channel_kind != UserActionChannelKind::LocalWebConsent
            || validated_token.capture_basis
                != UserActionChannelKind::LocalWebConsent.verification_basis()
        {
            return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
        }

        let current_access = agent_connection_project_access_read_only(
            self.runtime_home(),
            &validated_token.connection_internal_id,
            &validated_token.project_id,
        )?;
        if !current_access.is_some_and(|access| {
            access.connection_internal_id == validated_token.connection_internal_id
                && access.project_id == validated_token.project_id
                && access.connection_enabled
                && access.project_allowed
                && access.project.is_some()
        }) {
            return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
        }

        let project_id = ProjectId::new(validated_token.project_id.clone());
        let store = CoreProjectStore::open_read_only(self.runtime_home(), &project_id)?;
        store.with_read_snapshot(|store| {
            Ok(
                (|| -> CoreResult<LocalWebConsentUserActionProjectionOutcome> {
                    let Some(snapshot_token) =
                        store.user_action_channel_token_record(&validated_token.token_hash)?
                    else {
                        return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
                    };
                    if snapshot_token != validated_token || snapshot_token.token_hash != token_hash
                    {
                        return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
                    }
                    if !store.has_active_agent_session_for_connection(
                        &snapshot_token.connection_internal_id,
                    )? {
                        return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
                    }

                    let project_state = store.project_state()?;
                    let observed_at = self.project_store_now(store)?;
                    let (created_at, expires_at) =
                        store.validate_user_action_channel_token_window(&snapshot_token)?;
                    let Some(effective) = store
                        .user_action_record(&snapshot_token.user_action_request_id, &observed_at)?
                    else {
                        return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
                    };
                    let expected_creator =
                        format!("agent_connection:{}", snapshot_token.connection_internal_id);
                    if effective.request.requested_by_actor_source != expected_creator {
                        return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
                    }

                    match snapshot_token.status.as_str() {
                        "pending"
                            if observed_at >= created_at
                                && observed_at < expires_at
                                && effective.status == UserActionStatus::Pending
                                && effective.resolution.is_none() => {}
                        "consumed"
                            if allow_resolved_replay
                                && observed_at >= created_at
                                && observed_at < expires_at
                                && effective.resolution.is_some() =>
                        {
                            let Some(consumed_at) = snapshot_token
                                .consumed_at
                                .as_deref()
                                .and_then(|value| UtcTimestamp::parse(value).ok())
                            else {
                                return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
                            };
                            let Some(completed_at) = snapshot_token
                                .completed_at
                                .as_deref()
                                .and_then(|value| UtcTimestamp::parse(value).ok())
                            else {
                                return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
                            };
                            if consumed_at != completed_at
                                || consumed_at < created_at
                                || consumed_at >= expires_at
                            {
                                return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid);
                            }
                        }
                        _ => return Ok(LocalWebConsentUserActionProjectionOutcome::Invalid),
                    }

                    let Ok(metadata) = serde_json::from_str::<LocalWebConsentCreatedMetadata>(
                        &snapshot_token.created_metadata_json,
                    ) else {
                        return Ok(LocalWebConsentUserActionProjectionOutcome::FormMismatch);
                    };
                    if metadata.fallback_kind != LOCAL_WEB_CONSENT_FALLBACK_KIND
                        || metadata.delivery_surface != LOCAL_WEB_CONSENT_DELIVERY_SURFACE
                        || metadata.endpoint != LOCAL_WEB_CONSENT_ENDPOINT
                    {
                        return Ok(LocalWebConsentUserActionProjectionOutcome::FormMismatch);
                    }

                    let public_request =
                        user_action_from_record(&effective, project_state.state_version)?;
                    let form = public_request.body.capture_form().map_err(|_| {
                        CorePipelineError::Store(StoreError::corrupt_owner_state_json(
                            "user_action_requests",
                            snapshot_token.user_action_request_id.clone(),
                            "request_json",
                        ))
                    })?;
                    let current_form_digest = canonical_json_bare_sha256(&form)?;
                    if metadata.form_digest != current_form_digest {
                        return Ok(LocalWebConsentUserActionProjectionOutcome::FormMismatch);
                    }
                    Ok(LocalWebConsentUserActionProjectionOutcome::Projected(
                        Box::new(LocalWebConsentUserActionProjection {
                            request: public_request,
                            form,
                        }),
                    ))
                })(),
            )
        })?
    }

    /// Projects pending user-action forms only for an authenticated User
    /// Channel consumer.
    ///
    /// This is an internal, nonserialized boundary. Public methods and MCP
    /// structured output use the separate agent-safe summary projection.
    pub fn user_channel_inbox_projection(
        &self,
        request: UserChannelInboxProjectionRequest,
        invocation: InvocationContext,
    ) -> CoreResult<Option<UserChannelInboxProjection>> {
        if request.project_id != invocation.project_id
            || invocation.operation_category != OperationCategory::Read
        {
            return Ok(None);
        }

        let (same_connection_actor, user_channel, required_active_session) =
            match &invocation.actor_source {
                ActorSource::LocalUser
                    if invocation.invocation_binding_basis
                        == VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL =>
                {
                    (
                        None,
                        UserChannelContext {
                            prompt_capture_available: false,
                            host_elicitation_available: false,
                            local_web_consent_available: false,
                        },
                        None,
                    )
                }
                ActorSource::AgentConnection(connection_id) => {
                    let prompt_capture = invocation.invocation_binding_basis
                        == VERIFICATION_BASIS_USER_PROMPT_SUBMIT_HOOK;
                    let mcp_connection_binding = matches!(
                        invocation.invocation_binding_basis.as_str(),
                        VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING
                            | VERIFICATION_BASIS_MCP_LOCAL_HTTP_CONNECTION_BINDING
                    );
                    if !prompt_capture && !mcp_connection_binding {
                        return Ok(None);
                    }
                    let Some(session_id) = invocation
                        .session_id
                        .as_deref()
                        .filter(|session_id| !session_id.trim().is_empty())
                    else {
                        return Ok(None);
                    };
                    (
                        Some(invocation.actor_source.to_canonical_string()),
                        UserChannelContext {
                            prompt_capture_available: prompt_capture,
                            host_elicitation_available: mcp_connection_binding
                                && invocation.host_elicitation_available,
                            local_web_consent_available: mcp_connection_binding
                                && invocation.local_web_consent_available,
                        },
                        Some((session_id.to_owned(), connection_id.as_str().to_owned())),
                    )
                }
                ActorSource::LocalUser | ActorSource::System => return Ok(None),
            };

        let store = CoreProjectStore::open_read_only(self.runtime_home(), &request.project_id)?;
        let Some((project_state, observed_at, records)) = store.with_read_snapshot(|store| {
            if let Some((session_id, connection_id)) = required_active_session.as_ref() {
                let Some(session) = store.agent_session(session_id)? else {
                    return Ok(None);
                };
                if session.project_id != request.project_id.as_str()
                    || session.connection_internal_id.as_str() != connection_id
                    || session.ended_at.is_some()
                {
                    return Ok(None);
                }
            }
            if !store.task_exists(&request.task_id)? {
                return Ok(None);
            }
            let project_state = store.project_state()?;
            let observed_at = self.project_store_now(store)?;
            let records = store.pending_user_action_records(&request.task_id, &observed_at)?;
            Ok(Some((project_state, observed_at, records)))
        })?
        else {
            return Ok(None);
        };
        let items = records
            .iter()
            .filter(|record| {
                same_connection_actor
                    .as_ref()
                    .is_none_or(|actor| record.request.requested_by_actor_source == actor.as_str())
            })
            .map(|record| {
                let request = user_action_from_record(record, project_state.state_version)?;
                let inbox_item = user_action_inbox_item_from_request(
                    record,
                    request.clone(),
                    project_state.state_version,
                    user_channel,
                )?;
                Ok(UserChannelInboxProjectionItem {
                    request,
                    inbox_item,
                })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let user_channel_availability = user_channel_availability(user_channel);
        Ok(Some(UserChannelInboxProjection {
            project_id: request.project_id,
            task_id: request.task_id,
            observed_state_version: project_state.state_version,
            observed_at,
            user_channel_availability,
            items,
        }))
    }

    /// Reads the current effective status and agent-safe resolution projection
    /// for one user-action request without replaying either mutation.
    pub fn current_user_action_projection(
        &self,
        project_id: &ProjectId,
        user_action_request_id: &UserActionRequestId,
    ) -> CoreResult<Option<CurrentUserActionProjection>> {
        let store = CoreProjectStore::open_read_only(self.runtime_home(), project_id)?;
        let (project_state, observed_at, record, resolution_replay, continuity_records) = store
            .with_read_snapshot(|store| {
                let project_state = store.project_state()?;
                let observed_at = self.project_store_now(store)?;
                let record =
                    store.user_action_record(user_action_request_id.as_str(), &observed_at)?;
                let resolution_replay = record
                    .as_ref()
                    .and_then(|record| record.resolution.as_ref())
                    .map(|resolution| {
                        store.tool_invocation(
                            MethodName::ResolveUserAction,
                            &IdempotencyKey::new(resolution.channel_submission_id.clone()),
                        )
                    })
                    .transpose()?
                    .flatten();
                let continuity_records = match record.as_ref() {
                    Some(record) if record.resolution.is_some() => {
                        store.project_continuity_records_for_task(&record.request.task_id)?
                    }
                    _ => Vec::new(),
                };
                Ok((
                    project_state,
                    observed_at.clone(),
                    record,
                    resolution_replay,
                    continuity_records,
                ))
            })?;
        let Some(record) = record else {
            return Ok(None);
        };
        let request = user_action_from_record(&record, project_state.state_version)?;
        let (resolution_ref, resolution, derived_refs) = match record.resolution.as_ref() {
            Some(stored_resolution) => {
                let public_resolution =
                    user_action_resolution_from_record(stored_resolution, &request.task_id)?;
                let resolution_ref = request
                    .user_action_resolution_ref
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| {
                        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                            "user_action_requests",
                            user_action_request_id.as_str(),
                            "user_action_resolution_ref",
                        ))
                    })?;
                let safe = agent_safe_user_action_resolution(&request, &public_resolution)?;
                let (resolution_ref, derived_refs) = user_action_resolution_replay_projection(
                    resolution_replay.as_ref(),
                    &continuity_records,
                    stored_resolution,
                    &public_resolution,
                    &resolution_ref,
                )?;
                (Some(resolution_ref), Some(safe), derived_refs)
            }
            None => (None, None, Vec::new()),
        };
        Ok(Some(CurrentUserActionProjection {
            project_id: project_id.clone(),
            user_action_request_id: user_action_request_id.clone(),
            observed_state_version: project_state.state_version,
            observed_at,
            status: record.status,
            user_action_resolution_ref: resolution_ref,
            user_action_resolution: resolution,
            derived_refs,
        }))
    }

    /// Replays the exact originating `request_user_action` result for an
    /// access-matching Agent Connection without creating a second request.
    pub fn resume_user_action_request(
        &self,
        project_id: ProjectId,
        user_action_request_id: UserActionRequestId,
        invocation: InvocationContext,
    ) -> CoreResult<Option<PipelineResponse>> {
        let store = CoreProjectStore::open_read_only(self.runtime_home(), &project_id)?;
        let (project_state, record, origin_replay) = store.with_read_snapshot(|store| {
            let project_state = store.project_state()?;
            let now = self.project_store_now(store)?;
            let record = store.user_action_record(user_action_request_id.as_str(), &now)?;
            let origin_replay = record
                .as_ref()
                .filter(|record| {
                    record.request.source_method == MethodName::RequestUserAction.as_str()
                })
                .map(|record| {
                    store.tool_invocation(
                        MethodName::RequestUserAction,
                        &IdempotencyKey::new(record.request.source_idempotency_key.clone()),
                    )
                })
                .transpose()?
                .flatten();
            Ok((project_state, record, origin_replay))
        })?;
        let Some(record) = record else {
            return Ok(None);
        };
        if record.request.source_method != MethodName::RequestUserAction.as_str() {
            return Ok(None);
        }
        let task_id = TaskId::new(record.request.task_id.clone());
        let envelope = ToolEnvelope {
            project_id: project_id.clone(),
            task_id: Some(task_id.clone()).into(),
            request_id: RequestId::new("req_internal_user_action_resume"),
            idempotency_key: RequiredNullable::null(),
            expected_state_version: RequiredNullable::null(),
            dry_run: false,
            locale: RequiredNullable::null(),
        };
        let policy = MethodPolicy::exact(
            OperationCategory::AgentWorkflow,
            TaskRequirement::Exact(task_id.clone()),
            ReplayPolicy::None,
            FreshnessPolicy::None,
            MethodEffectPolicy::ReadOnly,
        );
        let Ok(verified_invocation) = crate::policy::access::derive_verified_invocation(
            &project_state,
            &envelope,
            &invocation,
            &policy,
        ) else {
            return Ok(None);
        };
        let source_idempotency_key =
            IdempotencyKey::new(record.request.source_idempotency_key.clone());
        let replay = origin_replay.ok_or_else(|| {
            CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                "user_action_requests",
                user_action_request_id.as_str(),
                "source_idempotency_key",
            ))
        })?;
        if replay.operation_category != OperationCategory::AgentWorkflow.as_str()
            || replay.actor_source != verified_invocation.actor_source.to_canonical_string()
        {
            return Ok(None);
        }
        if replay.actor_source != record.request.requested_by_actor_source {
            return Err(CorePipelineError::Store(
                StoreError::corrupt_owner_state_value(
                    "user_action_requests",
                    user_action_request_id.as_str(),
                    "requested_by_actor_source",
                ),
            ));
        }
        if !crate::pipeline::stored_public_response_is_current(
            MethodName::RequestUserAction,
            &replay.response_json,
            replay.committed_state_version,
        ) {
            return crate::pipeline::stored_response_unavailable_response(
                project_state.state_version,
                Some(verified_invocation),
                Some(task_id),
            )
            .map(Some);
        }
        let result: RequestUserActionResult = decode_required_json(
            "tool_invocations",
            format!(
                "{}/{}/{}",
                replay.project_id, replay.tool_name, replay.idempotency_key
            ),
            "response_json",
            Some(&replay.response_json),
        )?;
        let exact_origin = replay.project_id == project_id.as_str()
            && replay.tool_name == MethodName::RequestUserAction.as_str()
            && replay.idempotency_key == source_idempotency_key.as_str()
            && replay.operation_category == OperationCategory::AgentWorkflow.as_str()
            && replay.actor_source == record.request.requested_by_actor_source
            && replay.committed_state_version > replay.basis_state_version
            && result.base.response_kind == ResponseKind::Result
            && result.base.effect_kind == EffectKind::CoreCommitted
            && !result.base.dry_run
            && result.base.state_version == Some(replay.committed_state_version)
            && result.user_action_request_summary
                == AgentSafeUserActionRequestSummary::pending(user_action_request_id.clone());
        if !exact_origin {
            return Err(CorePipelineError::Store(
                StoreError::corrupt_owner_state_json(
                    "tool_invocations",
                    format!(
                        "{}/{}/{}",
                        replay.project_id, replay.tool_name, replay.idempotency_key
                    ),
                    "response_json",
                ),
            ));
        }
        let response_value = serde_json::from_str(&replay.response_json)?;
        let result_ref = operation_result_ref(
            &replay.response_json,
            &project_id,
            MethodName::RequestUserAction,
            Some(&source_idempotency_key),
            replay.committed_state_version,
            &verified_invocation,
        );
        Ok(Some(PipelineResponse {
            response_json: replay.response_json,
            response_value,
            operation_result_ref: result_ref,
            verified_invocation: Some(verified_invocation),
            resolved_task_id: Some(task_id),
            replayed: true,
        }))
    }
}

fn agent_safe_user_action_resolution(
    request: &UserActionRequest,
    resolution: &UserActionResolution,
) -> CoreResult<AgentSafeUserActionResolution> {
    let resolution_summary = match &resolution.body {
        UserActionResolutionBody::Choice {
            selected_option_id,
            machine_action,
            resolution_outcome,
            ..
        } => {
            let UserActionRequestBody::Choice(choice) = &request.body else {
                return Err(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.as_str(),
                        "resolution_json",
                    ),
                ));
            };
            let selected_option_label = choice
                .options
                .iter()
                .find(|option| option.option_id == *selected_option_id)
                .map(|option| option.label.clone())
                .ok_or_else(|| {
                    CorePipelineError::Store(StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.as_str(),
                        "resolution_json",
                    ))
                })?;
            McpUserActionResolutionSummary::Choice {
                selected_option_id: selected_option_id.clone(),
                selected_option_label,
                machine_action: *machine_action,
                resolution_outcome: *resolution_outcome,
            }
        }
        UserActionResolutionBody::EvidenceObservation { observation } => {
            if !matches!(request.body, UserActionRequestBody::EvidenceObservation(_)) {
                return Err(CorePipelineError::Store(
                    StoreError::corrupt_owner_state_value(
                        "user_action_resolutions",
                        resolution.user_action_resolution_id.as_str(),
                        "resolution_json",
                    ),
                ));
            }
            McpUserActionResolutionSummary::EvidenceObservation {
                target: observation.target.clone(),
                artifact_refs: observation.output_artifact_refs.clone(),
                relevance_status: observation.relevance_status,
            }
        }
    };
    Ok(AgentSafeUserActionResolution {
        user_action_resolution_id: resolution.user_action_resolution_id.clone(),
        user_action_request_id: resolution.user_action_request_id.clone(),
        action_kind: resolution.action_kind,
        channel_kind: resolution.channel_kind,
        resolved_at: resolution.resolved_at.clone(),
        resolution_summary,
    })
}

fn user_action_resolution_replay_projection(
    replay: Option<&ToolInvocationRecord>,
    continuity_records: &[ProjectContinuityRecordRecord],
    resolution: &UserActionResolutionRecord,
    public_resolution: &UserActionResolution,
    resolution_ref: &StateRecordRef,
) -> CoreResult<(StateRecordRef, Vec<StateRecordRef>)> {
    let replay = replay.ok_or_else(|| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_value(
            "user_action_resolutions",
            resolution.user_action_resolution_id.clone(),
            "channel_submission_id",
        ))
    })?;
    let replay_ref = format!(
        "{}/{}/{}",
        replay.project_id, replay.tool_name, replay.idempotency_key
    );
    let result: ResolveUserActionResult = decode_required_json(
        "tool_invocations",
        replay_ref.clone(),
        "response_json",
        Some(&replay.response_json),
    )?;
    let exact_replay_context = replay.project_id == resolution.project_id
        && replay.tool_name == MethodName::ResolveUserAction.as_str()
        && replay.idempotency_key == resolution.channel_submission_id
        && replay.actor_source == resolution.resolved_by_actor_source
        && replay.actor_source == ActorSource::LocalUser.to_canonical_string()
        && replay.operation_category == OperationCategory::UserOnly.as_str()
        && replay.verification_basis.as_deref()
            == Some(resolution.resolved_verification_basis.as_str())
        && replay.git_workspace_context_json.is_none();
    let exact_resolution = exact_replay_context
        && result.user_action_resolution == *public_resolution
        && result.user_action_resolution_ref.record_kind == StateRecordKind::UserActionResolution
        && result.user_action_resolution_ref.record_id == resolution_ref.record_id
        && result.user_action_resolution_ref.project_id == resolution_ref.project_id
        && result.user_action_resolution_ref.task_id == resolution_ref.task_id
        && result.base.response_kind == ResponseKind::Result
        && result.base.effect_kind == EffectKind::CoreCommitted
        && !result.base.dry_run
        && result.base.state_version == Some(replay.committed_state_version)
        && result
            .user_action_resolution_ref
            .produced_at_state_version
            .as_ref()
            == Some(&replay.committed_state_version);
    if !exact_resolution {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json("tool_invocations", replay_ref, "response_json"),
        ));
    }
    let resolution_source_ref = state_ref(
        StateRecordKind::UserActionResolution,
        &resolution.user_action_resolution_id,
        &ProjectId::new(resolution.project_id.clone()),
        Some(&public_resolution.task_id),
        Some(replay.committed_state_version),
    );
    let mut expected_derived_refs = Vec::new();
    for record in continuity_records {
        if record.project_id != resolution.project_id
            || record.source_task_id != public_resolution.task_id.as_str()
        {
            return Err(CorePipelineError::Store(
                StoreError::corrupt_owner_state_value(
                    "project_continuity_records",
                    &record.continuity_record_id,
                    "source_task_id",
                ),
            ));
        }
        let source_refs = decode_required_json::<Vec<StateRecordRef>>(
            "project_continuity_records",
            record.continuity_record_id.clone(),
            "source_refs_json",
            Some(&record.source_refs_json),
        )?;
        if source_refs.first() == Some(&resolution_source_ref) {
            expected_derived_refs.push(state_ref(
                StateRecordKind::ProjectContinuityRecord,
                &record.continuity_record_id,
                &ProjectId::new(record.project_id.clone()),
                Some(&TaskId::new(record.source_task_id.clone())),
                Some(replay.committed_state_version),
            ));
        }
    }
    let mut actual_derived_refs = result.derived_refs.clone();
    expected_derived_refs.sort_by_key(state_record_ref_identity_key);
    actual_derived_refs.sort_by_key(state_record_ref_identity_key);
    if actual_derived_refs != expected_derived_refs {
        return Err(CorePipelineError::Store(
            StoreError::corrupt_owner_state_json("tool_invocations", replay_ref, "response_json"),
        ));
    }
    Ok((result.user_action_resolution_ref, result.derived_refs))
}

fn execute_request_user_action(
    service: &CoreService,
    request: RequestUserActionRequest,
    invocation: InvocationContext,
) -> CoreResult<PipelineResponse> {
    let request_json = serde_json::to_value(&request)?;
    if request.envelope.task_id.as_ref() != Some(&request.task_id) {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            "envelope.task_id",
            "envelope.task_id must match RequestUserActionRequest.task_id",
        );
    }
    if let Err(error) = request.action.validate_bounds() {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            error.field(),
            error.message(),
        );
    }
    let prepared = match prepare_or_response(
        service,
        MethodName::RequestUserAction,
        request.envelope.clone(),
        request_json,
        invocation,
        mutation_method_policy(
            request.operation_category(),
            TaskRequirement::Exact(request.task_id.clone()),
            request.envelope.dry_run,
        ),
    )? {
        Ok(prepared) => prepared,
        Err(response) => return Ok(response),
    };
    let plan = match plan_request_user_action(
        service,
        &prepared.store,
        &prepared.context.project_state,
        request.clone(),
        &prepared.context.verified_invocation,
        &prepared.operation_now,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            return plan_error_response(&request.envelope, &prepared.context.project_state, error)
        }
    };
    if request.envelope.dry_run {
        return service.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::DryRunPreview {
                dry_run_summary: dry_run_summary(
                    "user_action_request",
                    "create_pending",
                    "Request would create one bounded pending user action.",
                    plan.next_actions,
                ),
            },
        );
    }
    service.execute_prepared_request(
        prepared,
        OwnerPipelineBranch::CommitMutation {
            result_fields: plan.result_fields,
            event_kind: "user_action_requested".to_owned(),
            event_payload: plan.event_payload,
            task_id: Some(plan.task_id),
            change_unit_id: plan.change_unit_id,
            storage_mutations: plan.storage_mutations,
        },
    )
}

fn plan_request_user_action(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: RequestUserActionRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<MethodPlan, PlanError> {
    let now = operation_now.clone();
    if request.required_for.is_empty() {
        return user_action_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "required_for",
            "required_for must contain at least one bounded operation",
        );
    }
    if request
        .required_for
        .iter()
        .enumerate()
        .any(|(index, target)| request.required_for[..index].contains(target))
    {
        return user_action_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "required_for",
            "required_for must not contain duplicate operation targets",
        );
    }
    validate_choice_affected_refs(
        &request.action,
        &request.envelope.project_id,
        &request.task_id,
        request.envelope.dry_run,
        project_state.state_version,
    )?;
    let effective_expires_at = if matches!(&request.action, UserActionDraft::EvidenceObservation(_))
    {
        if request.expires_at.is_some() {
            return user_action_validation_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "expires_at",
                "evidence-observation actions require caller expires_at to be null",
            );
        }
        RequiredNullable::some(checked_derived_expiration(
            &now,
            Duration::minutes(USER_ACTION_EVIDENCE_OBSERVATION_TTL_MINUTES),
            request.envelope.dry_run,
            Some(project_state.state_version),
            "expires_at",
        )?)
    } else {
        request.expires_at.clone()
    };
    if effective_expires_at
        .as_ref()
        .is_some_and(|expires_at| expires_at.ensure_canonical_rfc3339_representable().is_err())
    {
        return user_action_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "expires_at",
            "expires_at must be representable as a canonical four-digit RFC 3339 timestamp",
        );
    }
    if effective_expires_at
        .as_ref()
        .is_some_and(|expires_at| expires_at <= &now)
    {
        return user_action_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "expires_at",
            "expires_at must be later than the request timestamp",
        );
    }
    let task = store
        .task_record(&request.task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&request.task_id)
        .map_err(CorePipelineError::from)?;
    validate_required_for_compatibility(
        request.action.action_kind(),
        &request.required_for,
        request.envelope.dry_run,
        project_state.state_version,
    )?;
    if matches!(&request.action, UserActionDraft::EvidenceObservation(_))
        && (current_change_unit.is_none() || scope_baseline_is_missing(&task)?)
    {
        return user_action_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "action",
            "evidence-observation actions require a current Change Unit and baseline",
        );
    }
    if let Some(change_unit_id) = request.change_unit_id.as_ref() {
        if store
            .change_unit_record(&request.task_id, change_unit_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_none()
        {
            return user_action_validation_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "change_unit_id",
                "change_unit_id must identify a Change Unit owned by the Task",
            );
        }
    }
    let scope = StoredScope::from_task(&task)?;
    let action_needs_current_change_unit = matches!(
        request.action.action_kind(),
        UserActionKind::SensitiveApproval
            | UserActionKind::FinalAcceptance
            | UserActionKind::ResidualRiskAcceptance
            | UserActionKind::EvidenceObservation
    );
    if action_needs_current_change_unit {
        let Some(current) = current_change_unit.as_ref() else {
            return user_action_validation_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "change_unit_id",
                "this action kind requires the current active Change Unit",
            );
        };
        if request
            .change_unit_id
            .as_ref()
            .is_some_and(|requested| requested.as_str() != current.change_unit_id)
        {
            return user_action_validation_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "change_unit_id",
                "change_unit_id must match the current active Change Unit",
            );
        }
    }
    let coordinate_change_unit_id = request.change_unit_id.clone().or_else(|| {
        current_change_unit
            .as_ref()
            .map(|record| ChangeUnitId::new(record.change_unit_id.clone()))
    });
    let coordinates = UserActionBasisCoordinates {
        task_id: request.task_id.clone(),
        change_unit_id: coordinate_change_unit_id.clone().into(),
        scope_revision: task.scope_revision,
        baseline_ref: scope.baseline_ref.map(BaselineRef::new).into(),
        created_at_state_version: project_state.state_version,
        compatibility_status: UserActionBasisStatus::Current,
    };
    let (body, basis) =
        canonical_request_body_and_basis(store, project_state, &request, coordinates)?;
    body.capture_form().map_err(|error| {
        PlanError::Response(Box::new(
            validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                error.field(),
                error.message(),
            )
            .expect("user-action validation response should serialize"),
        ))
    })?;
    let planned_state_version = project_state.state_version + 1;
    let materialized = materialize_user_action_request(MaterializeUserActionRequestInput {
        service,
        store,
        project_state,
        verified_invocation,
        envelope: &request.envelope,
        source_method: MethodName::RequestUserAction,
        task_id: &request.task_id,
        coordinate_change_unit_id: coordinate_change_unit_id.clone(),
        body,
        basis,
        required_for: request.required_for.clone(),
        expires_at: effective_expires_at,
        created_at: now.clone(),
        metadata_json: "{}".to_owned(),
    })?;
    let action_kind = materialized.public_request.action_kind;
    let request_id = materialized.public_request.user_action_request_id.clone();
    let request_ref = materialized.request_ref.clone();
    let effective = materialized.effective;
    let mut pending_authorities = pending_user_action_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &request.task_id,
        &now,
    )?;
    pending_authorities.push(user_action_authority_from_record(&effective)?);
    let lifecycle_phase = projected_user_action_lifecycle_phase(
        project_state,
        &task,
        current_change_unit.as_ref(),
        &pending_authorities,
    );
    let mut projected_task = task.clone();
    if let Some(lifecycle_phase) = lifecycle_phase {
        projected_task.lifecycle_phase = lifecycle_phase.to_owned();
    }
    let (state, blocker_refs, next_actions) = projected_user_action_state(
        store,
        project_state,
        verified_invocation,
        &request.envelope,
        &projected_task,
        current_change_unit.as_ref(),
        &now,
        planned_state_version,
        Some(user_action_authority_from_record(&effective)?),
        Some(request_ref.clone()),
        None,
    )?;
    let result = RequestUserActionResult {
        base: placeholder_base(),
        user_action_request_summary: AgentSafeUserActionRequestSummary::pending(request_id.clone()),
        blocker_refs,
        state,
    };
    let mut storage_mutations = vec![materialized.mutation];
    if let Some(lifecycle_phase) = lifecycle_phase {
        storage_mutations.push(task_lifecycle_mutation(&request.task_id, lifecycle_phase));
    }
    Ok(MethodPlan {
        task_id: request.task_id,
        change_unit_id: coordinate_change_unit_id,
        storage_mutations,
        event_payload: object_from_value(json!({
            "user_action_request_id": request_id,
            "action_kind": action_kind,
            "required_for": request.required_for,
        }))?,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

pub(super) fn canonical_request_body_and_basis(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RequestUserActionRequest,
    coordinates: UserActionBasisCoordinates,
) -> Result<(UserActionRequestBody, UserActionBasis), PlanError> {
    match &request.action {
        UserActionDraft::Choice(choice) => {
            let UserActionChoiceDraft {
                judgment_kind,
                presentation,
                question,
                options,
                context,
                affected_refs,
                sensitive_action_scope,
            } = choice.as_ref();
            let options = canonical_choice_options(
                *judgment_kind,
                options.as_ref().map(Vec::as_slice).unwrap_or_default(),
                request.envelope.locale.as_ref().map(String::as_str),
                request.envelope.dry_run,
                project_state.state_version,
            )?;
            if normalize_display_text(question).is_empty()
                || normalize_display_text(&context.summary).is_empty()
            {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "action.question",
                    "choice question and context summary must be non-empty",
                );
            }
            if *judgment_kind != JudgmentKind::SensitiveApproval && sensitive_action_scope.is_some()
            {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "action.sensitive_action_scope",
                    "sensitive_action_scope is only valid for sensitive approval",
                );
            }
            let sensitive_action_scope = sensitive_action_scope
                .as_ref()
                .map(|scope| {
                    normalize_sensitive_action_scope(&store.project_record().repo_root, scope)
                        .map_err(|_| {
                            PlanError::Response(Box::new(
                                validation_rejected(
                                    request.envelope.dry_run,
                                    Some(project_state.state_version),
                                    "action.sensitive_action_scope.intended_paths",
                                    "sensitive action paths must stay within the Product Repository",
                                )
                                .expect("validation response should serialize"),
                            ))
                        })
                })
                .transpose()?;
            if *judgment_kind == JudgmentKind::SensitiveApproval && sensitive_action_scope.is_none()
            {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "action.sensitive_action_scope",
                    "sensitive approval requires a bounded sensitive action scope",
                );
            }
            let close_coordinates =
                choice_close_coordinates(store, project_state, request, *judgment_kind)?;
            Ok((
                UserActionRequestBody::Choice(Box::new(UserActionChoiceRequestBody {
                    judgment_kind: *judgment_kind,
                    presentation: *presentation,
                    question: normalize_display_text(question),
                    options,
                    context: context.clone(),
                    affected_refs: affected_refs.clone(),
                    sensitive_action_scope: sensitive_action_scope.clone().into(),
                })),
                UserActionBasis::Choice(Box::new(UserActionChoiceBasis {
                    coordinates,
                    close_basis_revision: close_coordinates.close_basis_revision.into(),
                    result_refs: close_coordinates.result_refs,
                    residual_risk_ids: close_coordinates.residual_risk_ids,
                    sensitive_action_scope: sensitive_action_scope.into(),
                })),
            ))
        }
        UserActionDraft::EvidenceObservation(observation) => {
            let UserActionEvidenceObservationDraft {
                question,
                context_summary,
                target_candidates,
                artifact_candidate_ids,
            } = observation;
            if normalize_display_text(question).is_empty()
                || normalize_display_text(context_summary).is_empty()
            {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "action.question",
                    "observation question and context summary must be non-empty",
                );
            }
            if target_candidates.iter().collect::<BTreeSet<_>>().len() != target_candidates.len() {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "action.target_candidates",
                    "target candidates must not contain duplicates",
                );
            }
            if artifact_candidate_ids.iter().collect::<BTreeSet<_>>().len()
                != artifact_candidate_ids.len()
            {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "action.artifact_candidate_ids",
                    "artifact candidates must not contain duplicates",
                );
            }
            for target in target_candidates {
                validate_user_action_target(
                    store,
                    project_state,
                    &request.envelope,
                    &request.task_id,
                    target,
                    "action.target_candidates",
                )?;
            }
            let artifact_candidates = canonical_user_action_artifacts(
                store,
                project_state,
                &request.envelope,
                &request.task_id,
                artifact_candidate_ids,
                "action.artifact_candidate_ids",
            )?;
            Ok((
                UserActionRequestBody::EvidenceObservation(
                    UserActionEvidenceObservationRequestBody {
                        question: normalize_display_text(question),
                        context_summary: normalize_display_text(context_summary),
                        target_candidates: target_candidates.clone(),
                        artifact_candidates: artifact_candidates.clone(),
                    },
                ),
                UserActionBasis::EvidenceObservation(UserActionEvidenceObservationBasis {
                    coordinates,
                    target_candidates: target_candidates.clone(),
                    artifact_candidates,
                }),
            ))
        }
    }
}

pub(super) struct MaterializedUserActionRequest {
    pub(super) request_ref: StateRecordRef,
    pub(super) public_request: UserActionRequest,
    pub(super) effective: EffectiveUserActionRecord,
    pub(super) mutation: CoreStorageMutation,
}

pub(super) struct MaterializeUserActionRequestInput<'a> {
    pub(super) service: &'a CoreService,
    pub(super) store: &'a CoreProjectStore,
    pub(super) project_state: &'a ProjectStateHeader,
    pub(super) verified_invocation: &'a VerifiedInvocationContext,
    pub(super) envelope: &'a ToolEnvelope,
    pub(super) source_method: MethodName,
    pub(super) task_id: &'a TaskId,
    pub(super) coordinate_change_unit_id: Option<ChangeUnitId>,
    pub(super) body: UserActionRequestBody,
    pub(super) basis: UserActionBasis,
    pub(super) required_for: Vec<UserActionRequiredFor>,
    pub(super) expires_at: RequiredNullable<UtcTimestamp>,
    pub(super) created_at: UtcTimestamp,
    pub(super) metadata_json: String,
}

/// Materializes the single canonical public/store representation of one Core-planned action.
pub(super) fn materialize_user_action_request(
    input: MaterializeUserActionRequestInput<'_>,
) -> Result<MaterializedUserActionRequest, PlanError> {
    let MaterializeUserActionRequestInput {
        service,
        store,
        project_state,
        verified_invocation,
        envelope,
        source_method,
        task_id,
        coordinate_change_unit_id,
        body,
        basis,
        required_for,
        expires_at,
        created_at,
        metadata_json,
    } = input;
    let action_kind = body.action_kind();
    let Some(source_idempotency_key) = envelope.idempotency_key.as_ref() else {
        return user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            "envelope.idempotency_key",
            "a committed user-action request requires an idempotency key",
        );
    };
    let request_id = allocate_user_action_request_id(service, store).map_err(PlanError::Core)?;
    let request_ref = state_ref(
        StateRecordKind::UserActionRequest,
        request_id.as_str(),
        &envelope.project_id,
        Some(task_id),
        Some(project_state.state_version + 1),
    );
    let persisted = PersistedUserActionRequest {
        body: body.clone(),
        required_for: required_for.clone(),
        expires_at: expires_at.clone(),
    };
    let request_json = serde_json::to_string(&persisted)?;
    let basis_json = serde_json::to_string(&basis)?;
    let required_for_json = serde_json::to_string(&required_for)?;
    let requested_by_actor_source = verified_invocation.actor_source.to_canonical_string();
    let requested_at = created_at.to_string();
    let stored_expires_at = expires_at.as_ref().map(ToString::to_string);
    let public_request = UserActionRequest {
        user_action_request_id: request_id.clone(),
        project_id: envelope.project_id.clone(),
        task_id: task_id.clone(),
        change_unit_id: coordinate_change_unit_id.clone().into(),
        action_kind,
        status: UserActionStatus::Pending,
        body,
        basis,
        required_for,
        user_action_resolution_ref: RequiredNullable::null(),
        expires_at,
        created_at,
    };
    let effective = EffectiveUserActionRecord {
        request: UserActionRequestRecord {
            project_id: envelope.project_id.as_str().to_owned(),
            user_action_request_id: request_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            change_unit_id: coordinate_change_unit_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            action_kind,
            request_json: request_json.clone(),
            basis_json: basis_json.clone(),
            basis_status: UserActionBasisStatus::Current,
            required_for_json: required_for_json.clone(),
            requested_by_actor_source: requested_by_actor_source.clone(),
            source_method: source_method.as_str().to_owned(),
            source_idempotency_key: source_idempotency_key.as_str().to_owned(),
            requested_at: requested_at.clone(),
            expires_at: stored_expires_at.clone(),
            metadata_json: metadata_json.clone(),
        },
        resolution: None,
        status: UserActionStatus::Pending,
    };
    let mutation = CoreStorageMutation::InsertUserActionRequest(UserActionRequestInsert {
        user_action_request_id: request_id.as_str().to_owned(),
        task_id: task_id.as_str().to_owned(),
        change_unit_id: coordinate_change_unit_id.map(|id| id.into_inner()),
        action_kind,
        request_json,
        basis_json,
        basis_status: UserActionBasisStatus::Current,
        required_for_json,
        requested_by_actor_source,
        source_method: source_method.as_str().to_owned(),
        source_idempotency_key: source_idempotency_key.as_str().to_owned(),
        requested_at,
        expires_at: stored_expires_at,
        metadata_json,
    });
    Ok(MaterializedUserActionRequest {
        request_ref,
        public_request,
        effective,
        mutation,
    })
}

fn canonical_choice_options(
    judgment_kind: JudgmentKind,
    caller_options: &[UserActionOptionInput],
    locale: Option<&str>,
    dry_run: bool,
    state_version: u64,
) -> Result<Vec<UserActionOption>, PlanError> {
    let authority_bearing = matches!(
        judgment_kind,
        JudgmentKind::ScopeDecision
            | JudgmentKind::SensitiveApproval
            | JudgmentKind::FinalAcceptance
            | JudgmentKind::ResidualRiskAcceptance
            | JudgmentKind::Cancellation
    );
    if authority_bearing {
        if !caller_options.is_empty() {
            return user_action_validation_error(
                dry_run,
                Some(state_version),
                "action.options",
                "authority-bearing actions use only Core-owned options",
            );
        }
        return Ok([
            UserActionOptionAction::Accept,
            UserActionOptionAction::Reject,
            UserActionOptionAction::Defer,
        ]
        .into_iter()
        .map(|machine_action| {
            let (label, description, consequence) =
                authority_option_copy(judgment_kind, machine_action, locale);
            UserActionOption {
                option_id: UserActionOptionId::new(match machine_action {
                    UserActionOptionAction::Accept => "accept",
                    UserActionOptionAction::Reject => "reject",
                    UserActionOptionAction::Defer => "defer",
                }),
                label,
                description,
                consequence,
                machine_action,
                resolution_outcome: machine_action.resolution_outcome(),
                is_default: machine_action == UserActionOptionAction::Accept,
            }
        })
        .collect());
    }
    if caller_options.is_empty() {
        return user_action_validation_error(
            dry_run,
            Some(state_version),
            "action.options",
            "product and technical choices require at least one caller-authored option",
        );
    }
    let mut ids = BTreeSet::new();
    if caller_options
        .iter()
        .any(|option| !ids.insert(option.option_id.as_str().to_owned()))
    {
        return user_action_validation_error(
            dry_run,
            Some(state_version),
            "action.options",
            "choice option IDs must be unique",
        );
    }
    if caller_options
        .iter()
        .filter(|option| option.is_default)
        .count()
        > 1
    {
        return user_action_validation_error(
            dry_run,
            Some(state_version),
            "action.options",
            "choice options may contain at most one default",
        );
    }
    Ok(caller_options
        .iter()
        .map(|option| UserActionOption {
            option_id: option.option_id.clone(),
            label: option.label.clone(),
            description: option.description.clone(),
            consequence: option.consequence.clone(),
            machine_action: UserActionOptionAction::Accept,
            resolution_outcome: JudgmentResolutionOutcome::Accepted,
            is_default: option.is_default,
        })
        .collect())
}

fn authority_option_copy(
    judgment_kind: JudgmentKind,
    action: UserActionOptionAction,
    locale: Option<&str>,
) -> (String, String, String) {
    let korean = locale
        .map(|locale| locale.to_ascii_lowercase().replace('_', "-"))
        .is_some_and(|locale| locale == "ko" || locale.starts_with("ko-"));
    let subject_en = match judgment_kind {
        JudgmentKind::ScopeDecision => "scope decision",
        JudgmentKind::SensitiveApproval => "sensitive action",
        JudgmentKind::FinalAcceptance => "final acceptance",
        JudgmentKind::ResidualRiskAcceptance => "residual risk",
        JudgmentKind::Cancellation => "task cancellation",
        JudgmentKind::ProductDecision => "product decision",
        JudgmentKind::TechnicalDecision => "technical decision",
    };
    let subject_ko = match judgment_kind {
        JudgmentKind::ScopeDecision => "범위 결정",
        JudgmentKind::SensitiveApproval => "민감 작업",
        JudgmentKind::FinalAcceptance => "최종 수락",
        JudgmentKind::ResidualRiskAcceptance => "잔여 위험",
        JudgmentKind::Cancellation => "작업 취소",
        JudgmentKind::ProductDecision => "제품 결정",
        JudgmentKind::TechnicalDecision => "기술 결정",
    };
    if korean {
        let (label, verb, outcome) = match action {
            UserActionOptionAction::Accept => ("수락", "수락합니다", "수락됨"),
            UserActionOptionAction::Reject => ("거부", "거부합니다", "거부됨"),
            UserActionOptionAction::Defer => ("보류", "나중으로 보류합니다", "보류됨"),
        };
        (
            label.to_owned(),
            format!("현재 근거에 따라 {subject_ko}을(를) {verb}."),
            format!("이 사용자 작업은 {outcome} 상태로 해결됩니다."),
        )
    } else {
        let (label, verb, outcome) = match action {
            UserActionOptionAction::Accept => ("Accept", "Accept", "accepted"),
            UserActionOptionAction::Reject => ("Reject", "Reject", "rejected"),
            UserActionOptionAction::Defer => ("Defer", "Defer", "deferred"),
        };
        (
            label.to_owned(),
            format!("{verb} the {subject_en} on the current basis."),
            format!("This user action resolves as {outcome}."),
        )
    }
}

struct ChoiceCloseCoordinates {
    close_basis_revision: Option<u64>,
    result_refs: Vec<StateRecordRef>,
    residual_risk_ids: Vec<RiskId>,
}

fn choice_close_coordinates(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &RequestUserActionRequest,
    judgment_kind: JudgmentKind,
) -> Result<ChoiceCloseCoordinates, PlanError> {
    if !matches!(
        judgment_kind,
        JudgmentKind::FinalAcceptance | JudgmentKind::ResidualRiskAcceptance
    ) {
        return Ok(ChoiceCloseCoordinates {
            close_basis_revision: None,
            result_refs: Vec::new(),
            residual_risk_ids: Vec::new(),
        });
    }
    let close_basis = store
        .task_revision_record(&request.task_id)
        .map_err(CorePipelineError::from)?
        .and_then(|record| record.current_close_basis)
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "a current close basis is required for this user action",
            )))
        })?;
    Ok(ChoiceCloseCoordinates {
        close_basis_revision: Some(close_basis.close_basis_revision),
        result_refs: close_basis.result_refs.clone(),
        residual_risk_ids: current_acceptance_required_risk_ids(&close_basis)
            .into_iter()
            .collect(),
    })
}

fn validate_user_action_target(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    target: &EvidenceTarget,
    field: &'static str,
) -> Result<(), PlanError> {
    let current = match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => store
            .acceptance_criterion_record(acceptance_criterion_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.task_id == task_id.as_str() && record.status == "active"),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id,
            statement,
        } => store
            .evidence_claim_record(task_id, evidence_claim_id.as_str())
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.statement == normalize_display_text(statement)),
    };
    if current {
        Ok(())
    } else {
        user_action_validation_error(
            envelope.dry_run,
            Some(project_state.state_version),
            field,
            "target must identify a current acceptance criterion or supplemental claim",
        )
    }
}

fn canonical_user_action_artifacts(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    artifact_ids: &[ArtifactId],
    field: &'static str,
) -> Result<Vec<ArtifactRef>, PlanError> {
    let mut canonical = BTreeMap::new();
    for artifact_id in artifact_ids {
        let record = store
            .artifact_record(artifact_id.as_str())
            .map_err(CorePipelineError::from)?;
        let owner_link = store
            .artifact_has_task_owner_link(artifact_id.as_str(), task_id.as_str())
            .map_err(CorePipelineError::from)?;
        let Some(record) = record else {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                field,
                "artifact candidates must identify current persistent Task artifacts",
            );
        };
        if record.project_id != envelope.project_id.as_str()
            || record.task_id != task_id.as_str()
            || !owner_link
            || !persistent_artifact_is_verified_current(store, &record)?
        {
            return user_action_validation_error(
                envelope.dry_run,
                Some(project_state.state_version),
                field,
                "artifact candidates must be verified current artifacts owned by this Task",
            );
        }
        let artifact_ref = artifact_ref_from_verified_record(
            store,
            &record,
            None,
            Some(project_state.state_version),
        )?;
        canonical.insert(artifact_id.as_str().to_owned(), artifact_ref);
    }
    Ok(canonical.into_values().collect())
}

fn user_action_validation_error<T>(
    dry_run: bool,
    state_version: Option<u64>,
    field: &'static str,
    message: &'static str,
) -> Result<T, PlanError> {
    match validation_plan_error(dry_run, state_version, field, message) {
        Err(error) => Err(error),
        Ok(()) => unreachable!("validation_plan_error always returns Err"),
    }
}

fn scope_baseline_is_missing(task: &TaskRecord) -> Result<bool, PlanError> {
    Ok(StoredScope::from_task(task)?.baseline_ref.is_none())
}

fn validate_choice_affected_refs(
    action: &UserActionDraft,
    project_id: &ProjectId,
    task_id: &TaskId,
    dry_run: bool,
    state_version: u64,
) -> Result<(), PlanError> {
    let UserActionDraft::Choice(choice) = action else {
        return Ok(());
    };
    for affected_ref in &choice.affected_refs {
        if affected_ref.project_id != *project_id {
            return user_action_validation_error(
                dry_run,
                Some(state_version),
                "action.affected_refs.project_id",
                "affected_refs must belong to the request project",
            );
        }
        let task_record_mismatch = affected_ref.record_kind == StateRecordKind::Task
            && affected_ref.record_id.as_str() != task_id.as_str();
        let task_context_mismatch = affected_ref
            .task_id
            .as_ref()
            .is_some_and(|affected_task_id| affected_task_id != task_id);
        if task_record_mismatch || task_context_mismatch {
            return user_action_validation_error(
                dry_run,
                Some(state_version),
                "action.affected_refs.task_id",
                "task-scoped affected_refs must belong to the request Task",
            );
        }
    }
    Ok(())
}

pub(super) fn validate_required_for_compatibility(
    action_kind: UserActionKind,
    required_for: &[UserActionRequiredFor],
    dry_run: bool,
    state_version: u64,
) -> Result<(), PlanError> {
    if required_for
        .iter()
        .copied()
        .all(|target| action_kind.is_compatible_with_required_for(target))
    {
        Ok(())
    } else {
        user_action_validation_error(
            dry_run,
            Some(state_version),
            "required_for",
            "required_for contains an operation incompatible with the action kind",
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn projected_user_action_state(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    envelope: &ToolEnvelope,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    now: &UtcTimestamp,
    target_state_version: u64,
    projected_authority: Option<UserActionAuthority>,
    added_pending_ref: Option<StateRecordRef>,
    resolved_request_id: Option<&UserActionRequestId>,
) -> Result<(StateSummary, Vec<StateRecordRef>, Vec<NextActionSummary>), PlanError> {
    let planned_state_version = target_state_version;
    let task_id = TaskId::new(task.task_id.clone());
    let mut pending_refs = store
        .pending_user_action_refs(&task_id, planned_state_version, now)
        .map_err(CorePipelineError::from)?
        .into_iter()
        .map(state_ref_from_stored)
        .filter(|record_ref| {
            resolved_request_id
                .is_none_or(|request_id| record_ref.record_id.as_str() != request_id.as_str())
        })
        .collect::<Vec<_>>();
    if let Some(added_pending_ref) = added_pending_ref {
        pending_refs.push(added_pending_ref);
    }
    let blocker_refs = projected_blocker_refs(store, &task_id, planned_state_version)?;
    let task_ref = state_ref(
        StateRecordKind::Task,
        task_id.as_str(),
        &envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let change_unit_ref = current_change_unit.map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &envelope.project_id,
            Some(&task_id),
            Some(record.basis_state_version.unwrap_or(planned_state_version)),
        )
    });
    let next_actions = next_actions_for_state(
        parse_task_mode(&task.mode)?,
        &task_ref,
        change_unit_ref.as_ref(),
        planned_state_version,
    );
    let guarantee_display =
        guarantee_display_for_invocation(store, verified_invocation, planned_state_version)?;
    let write_ticket_summary = projected_write_ticket_summary(
        store,
        &task_id,
        planned_state_version,
        *now.as_datetime(),
        Some(guarantee_display.clone()),
    )?;
    let current_close_basis = projected_close_basis(store, &task_id)?;
    let evidence_summary =
        projected_evidence_summary(store, &envelope.project_id, planned_state_version, task)?
            .map(|summary| evidence_summary_for_display(summary, current_close_basis.as_ref()));
    let mut pending_authorities =
        pending_user_action_authorities_for_plan(store, project_state, envelope, &task_id, now)?;
    if let Some(resolved_request_id) = resolved_request_id {
        pending_authorities
            .retain(|authority| authority.user_action_request_id != resolved_request_id.as_str());
    }
    let mut resolved_authorities = resolved_user_action_authorities_for_all_kinds(
        store,
        project_state,
        envelope,
        &task_id,
        now,
    )?;
    if let Some(authority) = projected_authority {
        match authority.status {
            UserActionStatus::Pending => {
                pending_authorities.retain(|existing| {
                    existing.user_action_request_id != authority.user_action_request_id
                });
                pending_authorities.push(authority);
            }
            UserActionStatus::Resolved => {
                resolved_authorities.retain(|existing| {
                    existing.user_action_request_id != authority.user_action_request_id
                });
                resolved_authorities.push(authority);
            }
            UserActionStatus::Stale | UserActionStatus::Superseded | UserActionStatus::Expired => {}
        }
    }
    let projected_project_state = project_state_projection(
        project_state,
        planned_state_version,
        project_state
            .active_task_id
            .clone()
            .or_else(|| Some(task_id.as_str().to_owned())),
    );
    let close_context = close_context_with_resolved_authorities(
        close_context_with_pending_authorities(
            close_context_from_projection(
                task.clone(),
                current_change_unit.cloned(),
                current_close_basis,
                pending_refs.clone(),
                blocker_refs.clone(),
                evidence_summary.clone(),
                now.clone(),
            ),
            pending_authorities,
        ),
        resolved_authorities,
    );
    let close_plan = projected_close_check(
        store,
        &projected_project_state,
        verified_invocation,
        envelope,
        &task_id,
        close_context,
        *now.as_datetime(),
    )?;
    let state = build_state_summary(SummaryBuild {
        store,
        project_id: &envelope.project_id,
        state_version: planned_state_version,
        task,
        current_change_unit,
        acceptance_criteria: active_acceptance_criteria_for_task(store, &task_id)?,
        pending_user_action_refs: pending_refs,
        blocker_refs: blocker_refs.clone(),
        write_ticket_summary,
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guard_health: close_plan.guard_health,
        guarantee_display: Some(guarantee_display),
    })?;
    Ok((state, blocker_refs, next_actions))
}

fn execute_resolve_user_action(
    service: &CoreService,
    request: ResolveUserActionRequest,
    invocation: InvocationContext,
    mut local_web: Option<LocalWebTokenContext>,
) -> CoreResult<PipelineResponse> {
    if let Err(error) = validate_channel_submission_id(&request.channel_submission_id) {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            error.field(),
            error.message(),
        );
    }
    if let Err(error) = request.resolution.validate_bounds() {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            error.field(),
            error.message(),
        );
    }
    if request.envelope.expected_state_version.is_some() {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            "envelope.expected_state_version",
            "resolve_user_action requires expected_state_version to be null",
        );
    }
    if local_web.is_none()
        && invocation.invocation_binding_basis.trim() == VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
    {
        return rejected_pipeline_response(
            request.envelope.dry_run,
            None,
            vec![tool_error(
                ErrorCode::InvocationContextMismatch,
                "local-web user authority requires the token-bearing Core entry point",
                false,
                None,
            )],
        );
    }
    if request
        .envelope
        .idempotency_key
        .as_ref()
        .map(IdempotencyKey::as_str)
        != Some(request.channel_submission_id.as_str())
    {
        return validation_rejected(
            request.envelope.dry_run,
            None,
            "envelope.idempotency_key",
            "idempotency_key must exactly match channel_submission_id",
        );
    }
    let local_web_replay_binding = match local_web.as_mut() {
        Some(context) => {
            if context.expected_connection_internal_id.is_empty()
                || context.expected_connection_internal_id.len() > 256
                || !context
                    .expected_connection_internal_id
                    .bytes()
                    .all(|byte| (0x21..=0x7e).contains(&byte))
            {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "expected_connection_internal_id",
                    "expected connection id must be 1..=256 bytes of visible ASCII",
                );
            }
            let completion_metadata = match serde_json::from_str::<LocalWebConsentCompletionMetadata>(
                &context.completion_metadata_json,
            ) {
                Ok(metadata) => metadata,
                Err(_) => {
                    return validation_rejected(
                        request.envelope.dry_run,
                        None,
                        "completion_metadata_json",
                        "local-web completion metadata must use the closed object shape",
                    )
                }
            };
            if completion_metadata
                .selection_recording
                .as_deref()
                .is_some_and(|value| value != "recorded")
                || completion_metadata
                    .endpoint
                    .as_deref()
                    .is_some_and(|value| {
                        value.is_empty()
                            || value.len() > 256
                            || !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
                    })
            {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "completion_metadata_json",
                    "local-web completion metadata contains unsupported values",
                );
            }
            let canonical_completion_metadata =
                volicord_types::canonical_json_string(&completion_metadata)?;
            let token_digest = match user_action_channel_token_hash(&context.token) {
                Ok(digest) => digest,
                Err(_) => {
                    return validation_rejected(
                        request.envelope.dry_run,
                        None,
                        "token",
                        "local-web token must use the bounded bearer-token shape",
                    )
                }
            };
            let derived_submission_id = local_web_channel_submission_id(
                &request.envelope.project_id,
                &request.user_action_request_id,
                &context.token,
                &context.expected_connection_internal_id,
                &completion_metadata,
            )?;
            if request.channel_submission_id != derived_submission_id {
                return validation_rejected(
                    request.envelope.dry_run,
                    None,
                    "channel_submission_id",
                    "local-web channel_submission_id must match the Core-derived credential binding",
                );
            }
            context.completion_metadata_json = canonical_completion_metadata;
            Some(json!({
                "token_digest": token_digest,
                "expected_connection_internal_id": context.expected_connection_internal_id,
                "completion_metadata": completion_metadata,
            }))
        }
        None => None,
    };
    let request_json = match local_web_replay_binding {
        Some(binding) => json!({
            "request": request,
            "local_web_replay_binding": binding,
        }),
        None => serde_json::to_value(&request)?,
    };
    let prepared = match prepare_or_response(
        service,
        MethodName::ResolveUserAction,
        request.envelope.clone(),
        request_json,
        invocation,
        mutation_method_policy(
            request.operation_category(),
            TaskRequirement::None,
            request.envelope.dry_run,
        )
        .with_current_state_default(),
    )? {
        Ok(prepared) => prepared,
        Err(response) => return Ok(response),
    };
    if prepared
        .context
        .verified_invocation
        .git_workspace_context
        .is_some()
    {
        return rejected_pipeline_response(
            request.envelope.dry_run,
            Some(prepared.context.project_state.state_version),
            vec![tool_error(
                ErrorCode::InvocationContextMismatch,
                "user-only resolution channels must not carry Git workspace context",
                false,
                None,
            )],
        );
    }
    let channel_kind = if local_web.is_some() {
        if channel_kind_from_verified_invocation(&prepared.context.verified_invocation)
            != Some(UserActionChannelKind::LocalWebConsent)
        {
            return rejected_pipeline_response(
                request.envelope.dry_run,
                Some(prepared.context.project_state.state_version),
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    "token-bearing local-web resolution requires verified local-web authority",
                    false,
                    None,
                )],
            );
        }
        UserActionChannelKind::LocalWebConsent
    } else {
        let Some(channel_kind) =
            channel_kind_from_verified_invocation(&prepared.context.verified_invocation)
        else {
            return rejected_pipeline_response(
                request.envelope.dry_run,
                Some(prepared.context.project_state.state_version),
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    "verified invocation is not a supported User Channel",
                    false,
                    None,
                )],
            );
        };
        if channel_kind == UserActionChannelKind::LocalWebConsent {
            return rejected_pipeline_response(
                request.envelope.dry_run,
                Some(prepared.context.project_state.state_version),
                vec![tool_error(
                    ErrorCode::InvocationContextMismatch,
                    "local-web user authority requires the token-bearing Core entry point",
                    false,
                    None,
                )],
            );
        }
        channel_kind
    };
    let now = prepared.operation_now.clone();
    let token_consumption = match local_web {
        Some(local_web) => match validated_local_web_token_consumption(
            &prepared.store,
            &prepared.context.project_state,
            &request,
            local_web,
            &now,
        ) {
            Ok(consumption) => Some(consumption),
            Err(response) => return Ok(*response),
        },
        None => None,
    };
    let plan = match plan_resolve_user_action(
        service,
        &prepared.store,
        &prepared.context.project_state,
        request.clone(),
        &prepared.context.verified_invocation,
        &prepared.context.verified_actor,
        channel_kind,
        token_consumption,
        now,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            return plan_error_response(&request.envelope, &prepared.context.project_state, error)
        }
    };
    if request.envelope.dry_run {
        return service.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::DryRunPreview {
                dry_run_summary: dry_run_summary(
                    "user_action_resolution",
                    "resolve_pending",
                    "Request would immutably resolve one pending user action.",
                    plan.method.next_actions,
                ),
            },
        );
    }
    let session_id = prepared.context.verified_invocation.session_id.clone();
    let response = service.execute_prepared_request(
        prepared,
        OwnerPipelineBranch::CommitMutation {
            result_fields: plan.method.result_fields,
            event_kind: "user_action_resolved".to_owned(),
            event_payload: plan.method.event_payload,
            task_id: Some(plan.method.task_id),
            change_unit_id: plan.method.change_unit_id,
            storage_mutations: plan.method.storage_mutations,
        },
    )?;
    if response_committed_fresh_effect(&response) {
        record_core_workflow_metric_best_effort(
            service,
            session_id.as_deref(),
            WorkflowMetricKind::UserRoundtrip,
            1,
        );
    }
    Ok(response)
}

struct LocalWebTokenContext {
    token: String,
    expected_connection_internal_id: String,
    completion_metadata_json: String,
}

struct ResolveUserActionPlan {
    method: MethodPlan,
}

fn channel_kind_from_verified_invocation(
    invocation: &VerifiedInvocationContext,
) -> Option<UserActionChannelKind> {
    UserActionChannelKind::from_verification_basis(&invocation.verification_basis)
}

fn validated_local_web_token_consumption(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &ResolveUserActionRequest,
    local_web: LocalWebTokenContext,
    now: &UtcTimestamp,
) -> Result<UserActionChannelTokenConsumption, Box<PipelineResponse>> {
    let validation = validate_user_action_channel_token(
        store.runtime_home(),
        UserActionChannelTokenCheck {
            token: local_web.token,
            expected_project_id: request.envelope.project_id.as_str().to_owned(),
            expected_connection_internal_id: local_web.expected_connection_internal_id,
            now: now.to_string(),
        },
    )
    .map_err(|error| {
        Box::new(store_error_response(
            &request.envelope,
            project_state,
            error,
        ))
    })?;
    let record = match validation {
        UserActionChannelTokenValidation::Valid(record) => record,
        UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Expired(_)) => {
            return Err(Box::new(decision_rejected_response(
                &request.envelope,
                None,
                "local web user-action token is expired",
            )))
        }
        UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Consumed(
            _,
        )) => {
            return Err(Box::new(decision_rejected_response(
                &request.envelope,
                None,
                "local web user-action token is already consumed",
            )))
        }
        UserActionChannelTokenValidation::Rejected(_) => {
            return Err(Box::new(decision_rejected_response(
                &request.envelope,
                None,
                "local web user-action token is invalid for this request",
            )))
        }
    };
    if record.channel_kind != UserActionChannelKind::LocalWebConsent
        || record.user_action_request_id != request.user_action_request_id.as_str()
        || record.capture_basis != VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB
    {
        return Err(Box::new(decision_rejected_response(
            &request.envelope,
            None,
            "local web token is not bound to this pending user action",
        )));
    }
    Ok(UserActionChannelTokenConsumption {
        token_hash: record.token_hash,
        connection_internal_id: record.connection_internal_id,
        user_action_request_id: record.user_action_request_id,
        consumed_at: now.to_string(),
        completion_metadata_json: local_web.completion_metadata_json,
    })
}

#[allow(clippy::too_many_arguments)]
fn plan_resolve_user_action(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: ResolveUserActionRequest,
    verified_invocation: &VerifiedInvocationContext,
    verified_actor: &VerifiedActorContext,
    channel_kind: UserActionChannelKind,
    token_consumption: Option<UserActionChannelTokenConsumption>,
    now: UtcTimestamp,
) -> Result<ResolveUserActionPlan, PlanError> {
    if verified_actor.actor_source != ActorSource::LocalUser {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "user-action resolution requires verified local-user authority",
        ))));
    }
    if let Some(existing) = store
        .user_action_resolution_for_channel_submission(channel_kind, &request.channel_submission_id)
        .map_err(CorePipelineError::from)?
    {
        let body: PersistedUserActionResolution = decode_required_json(
            "user_action_resolutions",
            existing.user_action_resolution_id.clone(),
            "resolution_json",
            Some(&existing.resolution_json),
        )?;
        let exact = existing.user_action_request_id == request.user_action_request_id.as_str()
            && existing.resolved_by_actor_source
                == verified_actor.actor_source.to_canonical_string()
            && existing.resolved_verification_basis == verified_actor.verification_basis
            && existing.resolved_assurance_level == verified_actor.assurance_level
            && resolution_input_matches_body(&request.resolution, &body);
        if !exact {
            return Err(PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "channel_submission_id conflicts with an immutable stored resolution",
            ))));
        }
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "channel submission is already committed; pipeline replay must use its original request identity",
        ))));
    }
    let effective = store
        .user_action_record(request.user_action_request_id.as_str(), &now)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "user_action_request_id does not identify a current user action",
            )))
        })?;
    if effective.status != UserActionStatus::Pending {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            match effective.status {
                UserActionStatus::Expired => "user action expired at or before this resolution",
                UserActionStatus::Stale => "user action basis is stale",
                UserActionStatus::Superseded => "user action basis is superseded",
                UserActionStatus::Resolved => "user action is already resolved",
                UserActionStatus::Pending => unreachable!(),
            },
        ))));
    }
    if request
        .envelope
        .task_id
        .as_ref()
        .is_some_and(|task_id| task_id.as_str() != effective.request.task_id)
    {
        return user_action_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "envelope.task_id",
            "envelope.task_id must match the addressed user action Task",
        );
    }
    let task_id = TaskId::new(effective.request.task_id.clone());
    let task = store
        .task_record(&task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(no_active_task_response(
                &request.envelope,
                project_state,
            )))
        })?;
    let current_change_unit = store
        .current_change_unit(&task_id)
        .map_err(CorePipelineError::from)?;
    let persisted: PersistedUserActionRequest = decode_required_json(
        "user_action_requests",
        effective.request.user_action_request_id.clone(),
        "request_json",
        Some(&effective.request.request_json),
    )?;
    let basis: UserActionBasis = decode_required_json(
        "user_action_requests",
        effective.request.user_action_request_id.clone(),
        "basis_json",
        Some(&effective.request.basis_json),
    )?;
    validate_current_resolution_basis(
        store,
        project_state,
        &request,
        &task,
        current_change_unit.as_ref(),
        &basis,
    )?;
    let resolution_id =
        allocate_user_action_resolution_id(service, store).map_err(PlanError::Core)?;
    let (resolution_body, mut derived_refs) = canonical_resolution_body(
        store,
        project_state,
        &request,
        &persisted.body,
        &basis,
        &task_id,
        current_change_unit.as_ref(),
    )?;
    resolution_body.validate().map_err(|error| {
        PlanError::Response(Box::new(
            validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                error.field(),
                error.message(),
            )
            .expect("resolution validation response should serialize"),
        ))
    })?;
    let resolution_record = UserActionResolutionRecord {
        project_id: request.envelope.project_id.as_str().to_owned(),
        user_action_resolution_id: resolution_id.as_str().to_owned(),
        user_action_request_id: request.user_action_request_id.as_str().to_owned(),
        action_kind: effective.request.action_kind,
        channel_kind,
        channel_submission_id: request.channel_submission_id.clone(),
        resolution_json: serde_json::to_string(&resolution_body)?,
        resolved_by_actor_source: verified_actor.actor_source.to_canonical_string(),
        resolved_verification_basis: verified_actor.verification_basis.clone(),
        resolved_assurance_level: verified_actor.assurance_level.clone(),
        resolved_at: now.to_string(),
    };
    let mut projected_effective = effective.clone();
    projected_effective.status = UserActionStatus::Resolved;
    projected_effective.resolution = Some(resolution_record.clone());
    let planned_state_version = project_state.state_version + 1;
    let public_request = user_action_from_record(&projected_effective, planned_state_version)?;
    let public_resolution = user_action_resolution_from_record(&resolution_record, &task_id)?;
    let request_ref = state_ref(
        StateRecordKind::UserActionRequest,
        request.user_action_request_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let resolution_ref = state_ref(
        StateRecordKind::UserActionResolution,
        resolution_id.as_str(),
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let continuity_plans = plan_user_action_continuity_records(
        service,
        store,
        project_state,
        &request.envelope,
        &task_id,
        current_change_unit.as_ref(),
        &persisted.body,
        &basis,
        &resolution_body,
        &resolution_ref,
        &now,
    )?;
    derived_refs.extend(continuity_plans.iter().map(|plan| plan.record_ref.clone()));
    let mut pending_authorities = pending_user_action_authorities_for_plan(
        store,
        project_state,
        &request.envelope,
        &task_id,
        &now,
    )?;
    pending_authorities.retain(|authority| {
        authority.user_action_request_id != request.user_action_request_id.as_str()
    });
    let lifecycle_phase = projected_user_action_lifecycle_phase(
        project_state,
        &task,
        current_change_unit.as_ref(),
        &pending_authorities,
    );
    let mut projected_task = task.clone();
    if let Some(lifecycle_phase) = lifecycle_phase {
        projected_task.lifecycle_phase = lifecycle_phase.to_owned();
    }
    let (state, _blocker_refs, next_actions) = projected_user_action_state(
        store,
        project_state,
        verified_invocation,
        &request.envelope,
        &projected_task,
        current_change_unit.as_ref(),
        &now,
        planned_state_version,
        Some(user_action_authority_from_record(&projected_effective)?),
        None,
        Some(&request.user_action_request_id),
    )?;
    let result = ResolveUserActionResult {
        base: placeholder_base(),
        user_action_request_ref: request_ref,
        user_action_resolution_ref: resolution_ref,
        user_action_request: public_request,
        user_action_resolution: public_resolution,
        derived_refs,
        state,
        next_actions: next_actions.clone(),
    };
    let mut storage_mutations = Vec::new();
    if let Some(token_consumption) = token_consumption {
        storage_mutations.push(CoreStorageMutation::ConsumeUserActionChannelToken(
            token_consumption,
        ));
    }
    storage_mutations.push(CoreStorageMutation::InsertUserActionResolution(
        UserActionResolutionInsert {
            user_action_resolution_id: resolution_record.user_action_resolution_id,
            user_action_request_id: resolution_record.user_action_request_id,
            action_kind: resolution_record.action_kind,
            channel_kind: resolution_record.channel_kind,
            channel_submission_id: resolution_record.channel_submission_id,
            resolution_json: resolution_record.resolution_json,
            resolved_by_actor_source: resolution_record.resolved_by_actor_source,
            resolved_verification_basis: resolution_record.resolved_verification_basis,
            resolved_assurance_level: resolution_record.resolved_assurance_level,
            resolved_at: resolution_record.resolved_at,
        },
    ));
    storage_mutations.extend(continuity_plans.into_iter().map(|plan| plan.mutation));
    if let Some(lifecycle_phase) = lifecycle_phase {
        storage_mutations.push(task_lifecycle_mutation(&task_id, lifecycle_phase));
    }
    Ok(ResolveUserActionPlan {
        method: MethodPlan {
            task_id,
            change_unit_id: current_change_unit
                .as_ref()
                .map(|record| ChangeUnitId::new(record.change_unit_id.clone())),
            storage_mutations,
            event_payload: object_from_value(json!({
                "user_action_request_id": request.user_action_request_id,
                "user_action_resolution_id": resolution_id,
                "action_kind": effective.request.action_kind,
                "channel_kind": channel_kind,
                "channel_submission_id": request.channel_submission_id,
            }))?,
            result_fields: strip_base(serde_json::to_value(result)?)?,
            next_actions,
        },
    })
}

fn validate_current_resolution_basis(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &ResolveUserActionRequest,
    task: &TaskRecord,
    current_change_unit: Option<&ChangeUnitRecord>,
    basis: &UserActionBasis,
) -> Result<(), PlanError> {
    let coordinates = basis.coordinates();
    let current_scope = StoredScope::from_task(task)?;
    let current_change_unit_id =
        current_change_unit.map(|record| ChangeUnitId::new(record.change_unit_id.clone()));
    if basis.compatibility_status() != UserActionBasisStatus::Current
        || coordinates.task_id.as_str() != task.task_id
        || coordinates.scope_revision != task.scope_revision
        || coordinates.created_at_state_version > project_state.state_version
        || coordinates.baseline_ref.as_ref().map(BaselineRef::as_str)
            != current_scope.baseline_ref.as_deref()
        || coordinates.change_unit_id.as_ref() != current_change_unit_id.as_ref()
    {
        return Err(PlanError::Response(Box::new(decision_rejected_response(
            &request.envelope,
            Some(project_state.state_version),
            "user-action basis is not current for this resolution",
        ))));
    }
    if let Some(close_basis_revision) = basis.close_basis_revision() {
        let current = store
            .task_revision_record(&TaskId::new(task.task_id.clone()))
            .map_err(CorePipelineError::from)?
            .is_some_and(|record| record.close_basis_revision == close_basis_revision);
        if !current {
            return Err(PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "user-action close basis is no longer current",
            ))));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn canonical_resolution_body(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &ResolveUserActionRequest,
    request_body: &UserActionRequestBody,
    basis: &UserActionBasis,
    task_id: &TaskId,
    current_change_unit: Option<&ChangeUnitRecord>,
) -> Result<(UserActionResolutionBody, Vec<StateRecordRef>), PlanError> {
    match (request_body, &request.resolution) {
        (
            UserActionRequestBody::Choice(choice),
            UserActionResolutionInput::Choice {
                selected_option_id,
                note,
            },
        ) => {
            let selected = choice
                .options
                .iter()
                .find(|option| option.option_id == *selected_option_id)
                .ok_or_else(|| {
                    PlanError::Response(Box::new(
                        validation_rejected(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            "resolution.selected_option_id",
                            "selected option must belong to the stored user-action request",
                        )
                        .expect("validation response should serialize"),
                    ))
                })?;
            let accepted_risk_ids = if choice.judgment_kind == JudgmentKind::ResidualRiskAcceptance
                && selected.machine_action == UserActionOptionAction::Accept
            {
                basis.residual_risk_ids().to_vec()
            } else {
                Vec::new()
            };
            Ok((
                UserActionResolutionBody::Choice {
                    selected_option_id: selected.option_id.clone(),
                    machine_action: selected.machine_action,
                    resolution_outcome: selected.resolution_outcome,
                    note: note.clone(),
                    accepted_risk_ids,
                },
                Vec::new(),
            ))
        }
        (
            UserActionRequestBody::EvidenceObservation(observation_request),
            UserActionResolutionInput::EvidenceObservation {
                target,
                artifact_ids,
                relevance_status,
                summary,
            },
        ) => {
            if !matches!(
                relevance_status,
                EvidenceRelevanceStatus::Supported | EvidenceRelevanceStatus::Contradicted
            ) {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.relevance_status",
                    "user observation relevance must be supported or contradicted",
                );
            }
            let normalized_summary = normalize_display_text(summary);
            if normalized_summary.is_empty() {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.summary",
                    "user observation summary must be non-empty",
                );
            }
            if !observation_request.target_candidates.contains(target) {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.target",
                    "observation target must be one of the stored candidates",
                );
            }
            if artifact_ids.iter().collect::<BTreeSet<_>>().len() != artifact_ids.len() {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.artifact_ids",
                    "observation artifact IDs must not contain duplicates",
                );
            }
            let candidate_ids = observation_request
                .artifact_candidates
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect::<BTreeSet<_>>();
            if artifact_ids
                .iter()
                .any(|artifact_id| !candidate_ids.contains(artifact_id))
            {
                return user_action_validation_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "resolution.artifact_ids",
                    "observation artifacts must be selected from the stored candidates",
                );
            }
            validate_user_action_target(
                store,
                project_state,
                &request.envelope,
                task_id,
                target,
                "resolution.target",
            )?;
            let output_artifact_refs = canonical_user_action_artifacts(
                store,
                project_state,
                &request.envelope,
                task_id,
                artifact_ids,
                "resolution.artifact_ids",
            )?;
            let selected_ids = artifact_ids.iter().collect::<BTreeSet<_>>();
            let stored_selected_refs = observation_request
                .artifact_candidates
                .iter()
                .filter(|artifact| selected_ids.contains(&artifact.artifact_id))
                .cloned()
                .collect::<Vec<_>>();
            if !current_artifact_refs_preserve_candidates(
                &stored_selected_refs,
                &output_artifact_refs,
            ) {
                return Err(PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "selected observation artifact changed after the request was created",
                ))));
            }
            let coordinates = basis.coordinates();
            let _change_unit_id = current_change_unit
                .map(|record| ChangeUnitId::new(record.change_unit_id.clone()))
                .ok_or_else(|| {
                    PlanError::Response(Box::new(no_active_change_unit_response(
                        &request.envelope,
                        Some(project_state.state_version),
                        "evidence observation resolution requires the current Change Unit",
                    )))
                })?;
            let _baseline_ref = coordinates.baseline_ref.as_ref().cloned().ok_or_else(|| {
                PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "evidence observation resolution requires a current baseline",
                )))
            })?;
            Ok((
                UserActionResolutionBody::EvidenceObservation {
                    observation: UserActionEvidenceObservation {
                        target: target.clone(),
                        relevance_status: *relevance_status,
                        output_artifact_refs: stored_selected_refs,
                        summary: normalized_summary,
                    },
                },
                Vec::new(),
            ))
        }
        _ => user_action_validation_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "resolution.resolution_type",
            "resolution type must match the stored user-action request",
        ),
    }
}

fn user_action_resolution_from_record(
    record: &UserActionResolutionRecord,
    task_id: &TaskId,
) -> CoreResult<UserActionResolution> {
    let body: PersistedUserActionResolution = decode_required_json(
        "user_action_resolutions",
        record.user_action_resolution_id.clone(),
        "resolution_json",
        Some(&record.resolution_json),
    )?;
    body.validate().map_err(|_| {
        CorePipelineError::Store(StoreError::corrupt_owner_state_json(
            "user_action_resolutions",
            record.user_action_resolution_id.clone(),
            "resolution_json",
        ))
    })?;
    Ok(UserActionResolution {
        user_action_resolution_id: UserActionResolutionId::new(
            record.user_action_resolution_id.clone(),
        ),
        user_action_request_id: UserActionRequestId::new(record.user_action_request_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        task_id: task_id.clone(),
        action_kind: record.action_kind,
        body,
        resolved_by_actor_source: parse_owner_storage_value(
            "user_action_resolutions",
            record.user_action_resolution_id.clone(),
            "resolved_by_actor_source",
            &record.resolved_by_actor_source,
        )?,
        resolved_verification_basis: record.resolved_verification_basis.clone(),
        resolved_assurance_level: record.resolved_assurance_level.clone(),
        channel_kind: record.channel_kind,
        channel_submission_id: record.channel_submission_id.clone(),
        resolved_at: parse_owner_storage_value(
            "user_action_resolutions",
            record.user_action_resolution_id.clone(),
            "resolved_at",
            &record.resolved_at,
        )?,
    })
}

#[allow(clippy::too_many_arguments)]
fn plan_user_action_continuity_records(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    task_id: &TaskId,
    current_change_unit: Option<&ChangeUnitRecord>,
    request_body: &UserActionRequestBody,
    basis: &UserActionBasis,
    resolution: &UserActionResolutionBody,
    resolution_ref: &StateRecordRef,
    now: &UtcTimestamp,
) -> Result<Vec<PlannedProjectContinuityRecord>, PlanError> {
    let (
        UserActionRequestBody::Choice(choice),
        UserActionBasis::Choice(choice_basis),
        UserActionResolutionBody::Choice {
            selected_option_id,
            machine_action,
            resolution_outcome,
            note: _,
            accepted_risk_ids,
        },
    ) = (request_body, basis, resolution)
    else {
        return Ok(Vec::new());
    };
    if *machine_action != UserActionOptionAction::Accept
        || *resolution_outcome != JudgmentResolutionOutcome::Accepted
    {
        return Ok(Vec::new());
    }
    let Some(continuity_kind) = judgment_continuity_kind(choice.judgment_kind, *resolution_outcome)
    else {
        return Ok(Vec::new());
    };
    let selected = choice
        .options
        .iter()
        .find(|option| option.option_id == *selected_option_id)
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                envelope,
                Some(project_state.state_version),
                "stored user-action resolution does not select a request option",
            )))
        })?;
    let source_change_unit_id = choice_basis.coordinates.change_unit_id.as_ref();
    let applies_to_paths = current_change_unit
        .map(|record| {
            decode_required_json(
                "change_units",
                record.change_unit_id.clone(),
                "bounded_paths_json",
                Some(&record.bounded_paths_json),
            )
            .map_err(PlanError::Core)
        })
        .transpose()?
        .unwrap_or_default();
    let continuity_context = ProjectContinuityPlanContext {
        service,
        store,
        project_id: &envelope.project_id,
        source_task_id: task_id,
        source_change_unit_id,
        planned_state_version: project_state.state_version + 1,
        now,
    };
    match continuity_kind {
        ProjectContinuityKind::Decision => {
            let mut applies_to_refs = choice.affected_refs.clone();
            applies_to_refs.extend(choice.context.related_refs.clone());
            let mut source_refs = vec![resolution_ref.clone()];
            source_refs.extend(applies_to_refs.clone());
            let summary = format!(
                "Selected option: {}. {}",
                selected.label,
                choice.context.summary.trim()
            );
            let draft = ProjectContinuityDraft {
                kind: ProjectContinuityKind::Decision,
                title: format!(
                    "{}: {}",
                    decision_title_prefix(choice.judgment_kind),
                    short_user_action_continuity_title(&selected.label)
                ),
                summary,
                rationale: None,
                applies_to_paths,
                applies_to_refs,
                source_refs,
                artifact_refs: choice.context.artifact_refs.clone(),
                supersedes_refs: Vec::new(),
                review_triggers: Vec::new(),
                metadata: json!({
                    "source": "resolve_user_action",
                    "action_kind": request_body.action_kind(),
                    "resolution_outcome": resolution_outcome,
                    "selected_option_id": selected_option_id
                }),
            };
            Ok(vec![plan_project_continuity_record(
                continuity_context,
                draft,
            )
            .map_err(PlanError::Core)?])
        }
        ProjectContinuityKind::AcceptedRisk => {
            if accepted_risk_ids.is_empty() {
                return Ok(Vec::new());
            }
            let close_basis = store
                .task_revision_record(task_id)
                .map_err(CorePipelineError::from)?
                .and_then(|record| record.current_close_basis)
                .ok_or_else(|| {
                    PlanError::Response(Box::new(decision_rejected_response(
                        envelope,
                        Some(project_state.state_version),
                        "accepted residual risks require the current close basis",
                    )))
                })?;
            let accepted = accepted_risk_ids.iter().collect::<BTreeSet<_>>();
            let risks = close_basis
                .residual_risks
                .iter()
                .filter(|risk| accepted.contains(&risk.risk_id))
                .collect::<Vec<_>>();
            if risks.len() != accepted.len() {
                return Err(PlanError::Response(Box::new(decision_rejected_response(
                    envelope,
                    Some(project_state.state_version),
                    "accepted residual-risk identities do not match the current close basis",
                ))));
            }
            let mut plans = Vec::with_capacity(risks.len());
            for risk in risks {
                let mut source_refs = vec![resolution_ref.clone()];
                source_refs.extend(risk.source_refs.clone());
                let mut applies_to_refs = close_basis.result_refs.clone();
                applies_to_refs.extend(risk.source_refs.clone());
                let draft = ProjectContinuityDraft {
                    kind: ProjectContinuityKind::AcceptedRisk,
                    title: format!(
                        "Accepted residual risk: {}",
                        short_user_action_continuity_title(&risk.summary)
                    ),
                    summary: risk.summary.clone(),
                    rationale: None,
                    applies_to_paths: applies_to_paths.clone(),
                    applies_to_refs,
                    source_refs,
                    artifact_refs: choice.context.artifact_refs.clone(),
                    supersedes_refs: Vec::new(),
                    review_triggers: Vec::new(),
                    metadata: json!({
                        "source": "resolve_user_action",
                        "action_kind": request_body.action_kind(),
                        "risk_id": risk.risk_id,
                        "close_basis_revision": close_basis.close_basis_revision
                    }),
                };
                plans.push(
                    plan_project_continuity_record(continuity_context, draft)
                        .map_err(PlanError::Core)?,
                );
            }
            Ok(plans)
        }
        _ => Ok(Vec::new()),
    }
}

fn short_user_action_continuity_title(value: &str) -> String {
    const MAX_CHARS: usize = 96;
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let short = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{short}...")
    } else {
        short
    }
}

fn resolution_input_matches_body(
    input: &UserActionResolutionInput,
    body: &UserActionResolutionBody,
) -> bool {
    match (input, body) {
        (
            UserActionResolutionInput::Choice {
                selected_option_id,
                note,
            },
            UserActionResolutionBody::Choice {
                selected_option_id: stored_id,
                note: stored_note,
                ..
            },
        ) => selected_option_id == stored_id && note == stored_note,
        (
            UserActionResolutionInput::EvidenceObservation {
                target,
                artifact_ids,
                relevance_status,
                summary,
            },
            UserActionResolutionBody::EvidenceObservation { observation },
        ) => {
            let mut input_ids = artifact_ids.clone();
            let mut stored_ids = observation
                .output_artifact_refs
                .iter()
                .map(|artifact| artifact.artifact_id.clone())
                .collect::<Vec<_>>();
            input_ids.sort();
            stored_ids.sort();
            target == &observation.target
                && relevance_status == &observation.relevance_status
                && normalize_display_text(summary) == observation.summary
                && input_ids == stored_ids
        }
        _ => false,
    }
}

/// Compares immutable request candidates with their current projection. The
/// projection may rebase only the nested producer state version; every other
/// typed `ArtifactRef` field remains exact.
fn current_artifact_refs_preserve_candidates(left: &[ArtifactRef], right: &[ArtifactRef]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut candidates = left.iter().collect::<Vec<_>>();
    let mut current = right.iter().collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    current.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    candidates
        .into_iter()
        .zip(current)
        .all(|(candidate, current)| {
            let mut normalized_current = current.clone();
            match (
                candidate.created_by_run_ref.as_ref(),
                normalized_current.created_by_run_ref.as_mut(),
            ) {
                (Some(candidate_run), Some(current_run)) => {
                    current_run.produced_at_state_version = candidate_run
                        .produced_at_state_version
                        .as_ref()
                        .copied()
                        .into();
                }
                (None, None) => {}
                _ => return false,
            }
            candidate == &normalized_current
        })
}
