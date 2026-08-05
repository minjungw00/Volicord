#![forbid(unsafe_code)]
#![deny(clippy::wildcard_imports)]

//! Shared Rust type boundary for Volicord public API and domain-shaped values.
//!
//! This crate contains serde models, controlled API value sets, opaque string
//! identifier wrappers, and deterministic canonical JSON hashing helpers. It
//! does not implement Core behavior, storage effects, CLI behavior, or MCP
//! adapter behavior.

pub mod canonical;
pub mod connection_verification;
pub mod contracts;
pub mod diagnostics;
pub mod guard_manifest;
pub mod guard_outcome;
pub mod host_configuration;
pub mod ids;
pub mod integration_revision;
pub mod integration_verification;
pub mod managed_guidance;
pub mod managed_mcp_client_info;
pub mod mcp_verification_evidence;
pub mod methods;
pub mod platform;
pub mod product_path;
pub mod release_target;
pub mod schema;
pub mod storage_contract;
pub mod tool_names;
pub mod values;
pub mod workflow_policy;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use schemars::{schema_for, JsonSchema};
    use serde_json::{json, Value};

    use crate::{canonical::*, ids::*, methods::*, schema::*, tool_names::*, values::*};

    fn timestamp(value: &str) -> UtcTimestamp {
        UtcTimestamp::parse(value).expect("test timestamp should be RFC 3339")
    }

    fn assert_non_guarantees(disclosure: &Value, expected: &[&str]) {
        let values = disclosure["non_guarantees"]
            .as_array()
            .expect("non_guarantees should be an array")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("non_guarantees should contain strings")
            })
            .collect::<BTreeSet<_>>();
        for expected_value in expected {
            assert!(
                values.contains(expected_value),
                "missing non-guarantee {expected_value}: {disclosure}"
            );
        }
    }

    #[test]
    fn prepare_evidence_capture_method_and_agent_tool_identity_are_stable() {
        assert_eq!(
            MethodName::PrepareEvidenceCapture.as_str(),
            "volicord.prepare_evidence_capture"
        );
        assert_eq!(
            AgentToolId::PREPARE_EVIDENCE_CAPTURE.wire_name(),
            "volicord.prepare_evidence_capture"
        );
        assert!(AgentToolId::PREPARE_EVIDENCE_CAPTURE.available_in(AgentConnectionMode::Workflow));
        assert!(!AgentToolId::PREPARE_EVIDENCE_CAPTURE.available_in(AgentConnectionMode::ReadOnly));
    }

    #[test]
    fn workflow_action_catalog_accepts_variant_groups_and_rejects_incoherent_keys() {
        let action = |variant: &str, operation: &str| {
            json!({
                "method": "volicord.update_scope",
                "semantic_variant": variant,
                "role": "required",
                "expected_state_version": 7,
                "fixed_authority_coordinates": {
                    "coordinate_kind": "update_scope",
                    "task_id": "task_catalog",
                    "scope_revision": 2,
                    "baseline_ref": "baseline_catalog",
                    "current_change_unit_id": "change_unit_catalog",
                    "related_scope_decision_refs": [],
                    "selected_change_unit_operation": operation
                },
                "required_refs": []
            })
        };
        let valid = json!({
            "required_method": "volicord.update_scope",
            "actions": [
                action("keep_current_change_unit", "keep_current"),
                action("replace_current_change_unit", "replace_current")
            ]
        });
        assert!(serde_json::from_value::<WorkflowActionCatalog>(valid).is_ok());

        let duplicate = json!({
            "required_method": "volicord.update_scope",
            "actions": [
                action("keep_current_change_unit", "keep_current"),
                action("keep_current_change_unit", "keep_current")
            ]
        });
        assert!(serde_json::from_value::<WorkflowActionCatalog>(duplicate).is_err());

        let variants_out_of_order = json!({
            "required_method": "volicord.update_scope",
            "actions": [
                action("replace_current_change_unit", "replace_current"),
                action("keep_current_change_unit", "keep_current")
            ]
        });
        assert!(serde_json::from_value::<WorkflowActionCatalog>(variants_out_of_order).is_err());

        let methods_out_of_order = json!({
            "required_method": "volicord.update_scope",
            "actions": [
                action("keep_current_change_unit", "keep_current"),
                {
                    "method": "volicord.check_close",
                    "semantic_variant": "check_close",
                    "role": "allowed",
                    "expected_state_version": 7,
                    "fixed_authority_coordinates": {
                        "coordinate_kind": "check_close",
                        "task_id": "task_catalog"
                    },
                    "required_refs": []
                }
            ]
        });
        assert!(serde_json::from_value::<WorkflowActionCatalog>(methods_out_of_order).is_err());

        let missing_required = json!({
            "required_method": "volicord.update_scope",
            "actions": []
        });
        assert!(serde_json::from_value::<WorkflowActionCatalog>(missing_required).is_err());

        let wrong_variant_operation = json!({
            "required_method": "volicord.update_scope",
            "actions": [action("keep_current_change_unit", "replace_current")]
        });
        assert!(serde_json::from_value::<WorkflowActionCatalog>(wrong_variant_operation).is_err());
    }

    #[test]
    fn source_ref_tagged_variants_round_trip_strictly() {
        let values = [
            json!({
                "source_kind": "repository_file",
                "source": {
                    "repository_path": "README.md",
                    "baseline_commit_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "content_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "line_range": {"start_line": 1, "end_line": 3}
                }
            }),
            json!({
                "source_kind": "git_commit",
                "source": {"commit_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
            }),
            json!({
                "source_kind": "git_diff",
                "source": {
                    "base_commit_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "head_commit_sha": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "diff_artifact_ref": null
                }
            }),
            json!({
                "source_kind": "command",
                "source": {
                    "invocation_id": "invocation_001",
                    "command_summary": "cargo test",
                    "exit_code": 0,
                    "output_artifact_ref": null
                }
            }),
            json!({
                "source_kind": "external_uri",
                "source": {
                    "uri": "https://example.invalid/spec",
                    "retrieved_at": "2026-07-12T00:00:00Z",
                    "content_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                }
            }),
            json!({
                "source_kind": "user_context",
                "source": {"context_id": "message_001"}
            }),
        ];
        for value in values {
            let parsed: SourceRef = serde_json::from_value(value.clone()).expect("source parses");
            assert_eq!(
                serde_json::to_value(parsed).expect("source serializes"),
                value
            );
        }
        assert!(serde_json::from_value::<SourceRef>(json!({
            "source_kind": "user_context",
            "source": {"context_id": "message_001", "uri": "https://example.invalid"}
        }))
        .is_err());
        assert!(serde_json::from_value::<SourceRef>(json!({
            "source_kind": "user_context",
            "source": {"context_id": "message_001"},
            "unexpected": true
        }))
        .is_err());
    }

    #[test]
    fn integration_profile_values_serialize_stable_names() {
        assert_eq!(
            serde_json::to_value(IntegrationProfile::Record).expect("profile serializes"),
            json!("record")
        );
        assert_eq!(
            serde_json::from_value::<IntegrationProfile>(json!("record"))
                .expect("record profile deserializes"),
            IntegrationProfile::Record
        );
        assert!(serde_json::from_value::<IntegrationProfile>(json!("unsupported")).is_err());
        assert!(
            serde_json::from_value::<IntegrationProfile>(json!("observe")).is_err(),
            "observe is intentionally rejected as an integration profile"
        );
        assert!(
            serde_json::from_value::<IntegrationProfile>(json!("managed")).is_err(),
            "only record is a public integration profile"
        );
        assert_eq!(
            serde_json::to_value(GuardDecision::InjectContext).expect("guard decision serializes"),
            json!("inject_context")
        );
        assert_eq!(
            serde_json::to_value(GuardHookPhase::PromptCapture).expect("Guard phase serializes"),
            json!("prompt_capture")
        );
        assert_eq!(
            serde_json::to_value(UnrecordedChangeStatus::Unresolved)
                .expect("change status serializes"),
            json!("unresolved")
        );

        assert!(serde_json::from_value::<HostKind>(json!("custom_host")).is_err());
        assert_eq!(
            serde_json::to_value(HostKind::Codex).expect("Codex host serializes"),
            json!("codex")
        );
    }

    #[test]
    fn integration_profile_schema_exposes_only_public_profile_values() {
        let schema =
            serde_json::to_value(schema_for!(IntegrationProfile)).expect("schema should serialize");

        assert_eq!(
            schema_enum_strings(schema),
            BTreeSet::from(["record".to_owned()])
        );
    }

    #[test]
    fn guarantee_disclosure_serializes_stable_machine_values() {
        let disclosure = serde_json::to_value(GuaranteeDisclosure::authority_record())
            .expect("disclosure should serialize");

        assert_eq!(disclosure["guarantee_class"], "authority_record");
        assert_non_guarantees(
            &disclosure,
            &[
                "NotOsSandbox",
                "NotActorAttributionProof",
                "NotCorrectnessProof",
                "NotTestSufficiencyProof",
                "NotHumanReviewReplacement",
                "NotFullFilesystemMonitoring",
            ],
        );
    }

    #[test]
    fn exact_result_metadata_schemas_require_guarantee_disclosure() {
        for schema in [
            serde_json::to_value(schema_for!(RequestedIntentReadOnlyResultBase)),
            serde_json::to_value(schema_for!(NotRequestedReadOnlyResultBase)),
            serde_json::to_value(schema_for!(CoreCommittedResultBase)),
            serde_json::to_value(schema_for!(StagingCreatedResultBase)),
            serde_json::to_value(schema_for!(NoEffectResultBase)),
        ]
        .map(|schema| schema.expect("schema should serialize"))
        {
            let required = schema["required"]
                .as_array()
                .expect("result metadata should have required fields");
            assert!(
                required.iter().any(|field| field == "disclosure"),
                "result metadata should require disclosure: {schema}"
            );
            assert!(
                schema["properties"]["disclosure"].is_object(),
                "result metadata should expose disclosure: {schema}"
            );
            assert_eq!(schema["additionalProperties"], false, "{schema}");
        }
    }

    #[test]
    fn response_base_schemas_expose_exact_constants_and_closed_objects() {
        let result_metadata = [
            (
                serde_json::to_value(schema_for!(RequestedIntentReadOnlyResultBase))
                    .expect("requested-intent read-only schema"),
                "read_only",
                true,
                None,
            ),
            (
                serde_json::to_value(schema_for!(NotRequestedReadOnlyResultBase))
                    .expect("not-requested read-only schema"),
                "read_only",
                false,
                None,
            ),
            (
                serde_json::to_value(schema_for!(CoreCommittedResultBase))
                    .expect("Core-committed schema"),
                "core_committed",
                false,
                Some(1),
            ),
            (
                serde_json::to_value(schema_for!(StagingCreatedResultBase))
                    .expect("staging-created schema"),
                "staging_created",
                false,
                Some(0),
            ),
            (
                serde_json::to_value(schema_for!(NoEffectResultBase)).expect("no-effect schema"),
                "no_effect",
                false,
                Some(0),
            ),
        ];
        for (schema, effect, variable_dry_run, event_cardinality) in result_metadata {
            assert_eq!(schema["additionalProperties"], false, "{schema}");
            assert_eq!(
                schema["properties"]["response_kind"]["enum"],
                json!(["result"]),
                "{schema}"
            );
            assert_eq!(
                schema["properties"]["effect_kind"]["enum"],
                json!([effect]),
                "{schema}"
            );
            if variable_dry_run {
                assert_eq!(schema["properties"]["dry_run"]["type"], "boolean");
            } else {
                assert_eq!(
                    schema["properties"]["dry_run"]["enum"],
                    json!([false]),
                    "{schema}"
                );
            }
            match event_cardinality {
                Some(1) => {
                    assert_eq!(
                        definition(&schema, "NonEmptyEventRefs")["minItems"],
                        1,
                        "{schema}"
                    )
                }
                Some(0) | None => {
                    assert_eq!(
                        definition(&schema, "EmptyEventRefs")["maxItems"],
                        0,
                        "{schema}"
                    )
                }
                _ => unreachable!(),
            }
        }

        let rejected =
            serde_json::to_value(schema_for!(ToolRejectedBase)).expect("rejected base schema");
        assert_eq!(rejected["additionalProperties"], false);
        assert_eq!(
            rejected["properties"]["response_kind"]["enum"],
            json!(["rejected"])
        );
        assert_eq!(
            rejected["properties"]["effect_kind"]["enum"],
            json!(["no_effect"])
        );
        assert_eq!(rejected["properties"]["dry_run"]["type"], "boolean");

        let preview =
            serde_json::to_value(schema_for!(ToolDryRunBase)).expect("preview base schema");
        assert_eq!(preview["additionalProperties"], false);
        assert_eq!(
            preview["properties"]["response_kind"]["enum"],
            json!(["dry_run"])
        );
        assert_eq!(
            preview["properties"]["effect_kind"]["enum"],
            json!(["no_effect"])
        );
        assert_eq!(preview["properties"]["dry_run"]["enum"], json!([true]));

        for schema in [
            serde_json::to_value(schema_for!(ToolRejectedResponse))
                .expect("rejected response schema"),
            serde_json::to_value(schema_for!(ToolDryRunResponse)).expect("preview response schema"),
        ] {
            assert_eq!(schema["additionalProperties"], false, "{schema}");
        }
    }

    #[test]
    fn guard_records_serialize_documented_field_names() {
        let capture = PromptCapture {
            prompt_capture_id: PromptCaptureId::new("prompt_capture_001"),
            project_id: ProjectId::new("project_guard_001"),
            session_id: AgentSessionId::new("session_guard_001"),
            connection_id: AgentConnectionId::new("conn_guard_001"),
            capture_kind: "user_prompt".to_owned(),
            prompt_sha256: "sha256:abc123".to_owned(),
            prompt_text: RequiredNullable::null(),
            captured_at: timestamp("2026-06-30T00:00:00Z"),
            metadata: JsonObject::new(),
        };

        let encoded = serde_json::to_value(&capture).expect("prompt capture serializes");
        assert_eq!(encoded["prompt_capture_id"], "prompt_capture_001");
        assert_eq!(encoded["connection_id"], "conn_guard_001");
        assert_eq!(encoded["prompt_text"], Value::Null);
        assert_unknown::<PromptCapture>(
            json!({
                "prompt_capture_id": "prompt_capture_001",
                "project_id": "project_guard_001",
                "session_id": "session_guard_001",
                "connection_id": "conn_guard_001",
                "capture_kind": "user_prompt",
                "prompt_sha256": "sha256:abc123",
                "prompt_text": null,
                "captured_at": "2026-06-30T00:00:00Z",
                "metadata": {},
                "extra": true
            }),
            "extra",
        );
    }

    #[test]
    fn tool_envelope_round_trips_documented_field_names() {
        let envelope: ToolEnvelope = serde_json::from_value(json!({
            "project_id": "proj_onboard_001",
            "task_id": null,
            "request_id": "req_intake_onboard_001",
            "idempotency_key": "idem_intake_onboard_001",
            "expected_state_version": 17,
            "dry_run": false,
            "locale": "en-US"
        }))
        .expect("documented envelope example should deserialize");

        assert_eq!(envelope.project_id.as_str(), "proj_onboard_001");
        assert_eq!(
            envelope
                .idempotency_key
                .as_ref()
                .map(IdempotencyKey::as_str),
            Some("idem_intake_onboard_001")
        );

        let encoded = serde_json::to_value(&envelope).expect("envelope should serialize");
        assert_eq!(encoded["project_id"], "proj_onboard_001");
        assert_eq!(encoded["task_id"], Value::Null);
    }

    #[test]
    fn authority_looking_request_fields_are_rejected() {
        let mut envelope_value = envelope_json();
        envelope_value["verified"] = json!(true);
        assert_unknown::<ToolEnvelope>(envelope_value, "verified");

        let mut envelope_value = envelope_json();
        envelope_value["actor_source"] = json!("agent_connection:conn_forged");
        assert_unknown::<ToolEnvelope>(envelope_value, "actor_source");

        for field in ["operation_category", "connection_id", "verification_basis"] {
            let mut request = status_request_json();
            request[field] = json!({ "forged": true });
            assert_unknown::<StatusRequest>(request, field);
        }
    }

    #[test]
    fn unknown_top_level_fields_are_rejected_on_public_requests() {
        for (method_name, mut request) in public_request_json_samples() {
            request["ordinary_unknown_field"] = json!("not documented");
            let error = deserialize_public_request(method_name, request).unwrap_err();
            assert!(
                error.to_string().contains("ordinary_unknown_field"),
                "unexpected error for {method_name}: {error}"
            );
        }
    }

    #[test]
    fn typed_requests_derive_documented_operation_categories() {
        assert_eq!(
            serde_json::from_value::<StatusRequest>(status_request_json())
                .expect("status request")
                .operation_category(),
            OperationCategory::Read
        );
        assert_eq!(
            serde_json::from_value::<GetOperationResultRequest>(
                get_operation_result_request_json(),
            )
            .expect("operation-result request")
            .operation_category(),
            OperationCategory::Read
        );
        assert_eq!(
            serde_json::from_value::<IntakeRequest>(intake_request_json())
                .expect("intake request")
                .operation_category(),
            OperationCategory::AgentWorkflow
        );
        assert_eq!(
            serde_json::from_value::<PrepareWriteRequest>(prepare_write_request_json())
                .expect("prepare request")
                .operation_category(),
            OperationCategory::AgentWorkflow
        );
        assert_eq!(
            serde_json::from_value::<PrepareEvidenceCaptureRequest>(
                prepare_evidence_capture_request_json(),
            )
            .expect("prepare evidence capture request")
            .operation_category(),
            OperationCategory::AgentWorkflow
        );
        assert_eq!(
            serde_json::from_value::<StageArtifactRequest>(stage_artifact_request_json())
                .expect("stage request")
                .operation_category(),
            OperationCategory::AgentWorkflow
        );
        assert_eq!(
            serde_json::from_value::<RecordRunRequest>(record_run_request_json())
                .expect("record run request")
                .operation_category(),
            OperationCategory::AgentWorkflow
        );
        assert_eq!(
            serde_json::from_value::<RequestUserActionRequest>(request_user_action_request_json())
                .expect("request user action")
                .operation_category(),
            OperationCategory::AgentWorkflow
        );
        assert_eq!(
            serde_json::from_value::<ResolveUserActionRequest>(resolve_user_action_request_json())
                .expect("resolve user action")
                .operation_category(),
            OperationCategory::UserOnly
        );

        let check = serde_json::from_value::<CheckCloseRequest>(check_close_request_json())
            .expect("check_close request");
        assert_eq!(check.operation_category(), OperationCategory::Read);

        for intent in ["complete", "cancel", "supersede"] {
            let mut request = close_task_request_json();
            request["intent"] = json!(intent);
            request["close_reason"] = json!(match intent {
                "complete" => "completed_self_checked",
                "cancel" => "cancelled",
                "supersede" => "superseded",
                _ => unreachable!(),
            });
            if intent == "supersede" {
                request["superseding_task_id"] = json!("task_replacement_001");
            }
            let request = serde_json::from_value::<CloseTaskRequest>(request)
                .expect("mutating close request should decode");
            assert_eq!(
                request.operation_category(),
                OperationCategory::AgentWorkflow
            );
        }
    }

    #[test]
    fn status_continuity_page_is_optional_nullable_and_strictly_typed() {
        let omitted: StatusRequest = serde_json::from_value(status_request_json())
            .expect("omitted continuity_page should decode");
        assert!(omitted.continuity_page.is_none());

        let mut null_page = status_request_json();
        null_page["continuity_page"] = Value::Null;
        let null_page: StatusRequest =
            serde_json::from_value(null_page).expect("null continuity_page should decode");
        assert!(null_page.continuity_page.is_none());

        let mut explicit = status_request_json();
        explicit["include"]["continuity"] = json!(true);
        explicit["continuity_page"] = json!({
            "page_size": 64,
            "cursor": {
                "updated_at": "2026-07-17T00:00:00Z",
                "continuity_record_id": "continuity_064"
            }
        });
        let explicit: StatusRequest =
            serde_json::from_value(explicit).expect("explicit continuity page should decode");
        let page = explicit
            .continuity_page
            .as_ref()
            .and_then(|page| page.as_ref())
            .expect("explicit page");
        assert_eq!(page.page_size, 64);
        assert_eq!(
            page.cursor
                .as_ref()
                .expect("explicit cursor")
                .continuity_record_id
                .as_str(),
            "continuity_064"
        );

        for malformed in [
            json!({"page_size": 8}),
            json!({"page_size": 8, "cursor": {"updated_at": "not-a-time", "continuity_record_id": "continuity_008"}}),
            json!({"page_size": 8, "cursor": {"updated_at": "2026-07-17T00:00:00Z"}}),
            json!({"page_size": 8, "cursor": null, "extra": true}),
            json!({"page_size": 8, "cursor": {"updated_at": "2026-07-17T00:00:00Z", "continuity_record_id": "continuity_008", "extra": true}}),
        ] {
            let mut request = status_request_json();
            request["include"]["continuity"] = json!(true);
            request["continuity_page"] = malformed;
            assert!(
                serde_json::from_value::<StatusRequest>(request).is_err(),
                "malformed continuity page must be rejected"
            );
        }
    }

    #[test]
    fn documented_extension_objects_remain_usable() {
        let mut update_scope = update_scope_request_json();
        update_scope["change_unit"]["owner_defined_note"] = json!({
            "kept": true,
            "reason": "change_unit carries method-owned object data"
        });
        let request: UpdateScopeRequest =
            serde_json::from_value(update_scope).expect("change_unit object fields remain open");
        assert!(request
            .change_unit
            .fields
            .contains_key("owner_defined_note"));
    }

    #[test]
    fn stage_artifact_result_serializes_documented_shape() {
        let result = StageArtifactResult {
            base: StageArtifactRequest::staging_created_result_base(42),
            evidence_state: EvidenceDisplayState::Prepared,
            staged_artifact_handle: StagedArtifactHandle {
                handle_id: StagedArtifactHandleId::new("staged_trace_log_001"),
                project_id: ProjectId::new("proj_trace_001"),
                task_id: TaskId::new("task_trace_001"),
                created_by_actor_source: ActorSource::agent_connection("conn_artifact"),
                content_type: "text/plain".to_owned(),
                sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_owned(),
                size_bytes: 42,
                redaction_state: RedactionState::None,
                expires_at: timestamp("2026-06-19T00:00:00Z"),
                consumed: false,
            },
            expires_at: timestamp("2026-06-19T00:00:00Z"),
        };

        let encoded = serde_json::to_value(&result).expect("result should serialize");

        assert_eq!(encoded["base"]["response_kind"], "result");
        assert_eq!(encoded["base"]["effect_kind"], "staging_created");
        assert_eq!(encoded["evidence_state"], "prepared");
        assert_eq!(encoded["staged_artifact_handle"]["redaction_state"], "none");
        assert_eq!(
            encoded["staged_artifact_handle"]["created_by_actor_source"],
            "agent_connection:conn_artifact"
        );

        let decoded: StageArtifactResult =
            serde_json::from_value(encoded).expect("result should deserialize");
        assert!(!decoded.staged_artifact_handle.consumed);
        assert_eq!(decoded.staged_artifact_handle.size_bytes, 42);
    }

    #[test]
    fn close_basis_and_user_action_basis_round_trip_json() {
        let close_basis: CurrentCloseBasis = serde_json::from_value(json!({
            "close_basis_revision": 4,
            "scope_revision": 2,
            "task_id": "task_close_basis_001",
            "change_unit_id": "cu_close_basis_001",
            "baseline_ref": "baseline_close_basis",
            "result_summary": "The requested export is implemented.",
            "result_refs": [
                state_ref_json("run", "run_close_basis_001", "task_close_basis_001")
            ],
            "evidence_refs": [],
            "evidence_summary_ref": null,
            "residual_risks": [
                {
                    "risk_id": "risk_close_basis_001",
                    "summary": "The downstream importer may reject older files.",
                    "consequence": "A manual retry may be needed.",
                    "acceptance_required": true,
                    "source_refs": [
                        state_ref_json("run", "run_close_basis_001", "task_close_basis_001")
                    ]
                }
            ],
            "sensitive_categories": ["network"],
            "sensitive_action_requirements": [
                {
                    "action_kind": "export customer data",
                    "normalized_paths": ["src/exporter.ts"],
                    "sensitive_categories": ["network"],
                    "baseline_ref": "baseline_close_basis",
                    "change_unit_id": "cu_close_basis_001",
                    "source_run_ref": state_ref_json("run", "run_close_basis_001", "task_close_basis_001"),
                    "source_write_ticket_ref": state_ref_json(
                        "write_ticket",
                        "wt_close_basis_001",
                        "task_close_basis_001"
                    )
                }
            ],
            "recovery_constraints": ["Rollback requires restoring the previous exporter."],
            "source_run_ref": state_ref_json("run", "run_close_basis_001", "task_close_basis_001"),
            "shaping_checkpoint_ref": null,
            "shaping_decision_application_refs": [],
            "updated_at": "2026-06-18T00:00:00.000Z"
        }))
        .expect("CurrentCloseBasis should deserialize");

        assert_eq!(
            close_basis.residual_risks[0].risk_id.as_str(),
            "risk_close_basis_001"
        );
        let encoded = serde_json::to_value(&close_basis).expect("CurrentCloseBasis serializes");
        assert_eq!(
            encoded["residual_risks"][0]["risk_id"],
            "risk_close_basis_001"
        );
        let decoded: CurrentCloseBasis =
            serde_json::from_value(encoded).expect("CurrentCloseBasis round-trips");
        assert_eq!(decoded, close_basis);

        let action_basis: UserActionBasis = serde_json::from_value(json!({
            "action_type": "choice",
            "coordinates": {
                "task_id": "task_close_basis_001",
                "change_unit_id": "cu_close_basis_001",
                "scope_revision": 2,
                "baseline_ref": "baseline_close_basis",
                "created_at_state_version": 11,
                "compatibility_status": "current"
            },
            "close_basis_revision": 4,
            "result_refs": [
                state_ref_json("run", "run_close_basis_001", "task_close_basis_001")
            ],
            "residual_risk_ids": ["risk_close_basis_001"],
            "sensitive_action_scope": null
        }))
        .expect("UserActionBasis should deserialize");

        assert_eq!(
            action_basis.compatibility_status(),
            UserActionBasisStatus::Current
        );
        let encoded = serde_json::to_value(&action_basis).expect("UserActionBasis serializes");
        assert_eq!(encoded["coordinates"]["compatibility_status"], "current");
        let decoded: UserActionBasis =
            serde_json::from_value(encoded).expect("UserActionBasis round-trips");
        assert_eq!(decoded, action_basis);
    }

    #[test]
    fn method_local_reason_codes_remain_strings() {
        let reason: WriteDecisionReason = serde_json::from_value(json!({
            "category": "sensitive_approval",
            "code": "sensitive_approval_missing",
            "message": "Approval is required.",
            "related_refs": []
        }))
        .expect("write decision reason should deserialize");

        assert_eq!(reason.category, WriteDecisionCategory::SensitiveApproval);
        assert_eq!(reason.code, "sensitive_approval_missing");

        let blocker: CloseReadinessBlocker = serde_json::from_value(json!({
            "category": "final_acceptance",
            "code": "missing_final_acceptance",
            "message": "Final acceptance is required.",
            "related_refs": [],
            "next_actions": []
        }))
        .expect("close blocker should deserialize");

        assert_eq!(
            blocker.category,
            CloseReadinessBlockerCategory::FinalAcceptance
        );
        assert_eq!(blocker.code, "missing_final_acceptance");
    }

    #[test]
    fn mutation_assessment_exposes_bounded_classification_and_reason_codes() {
        let assessment: MutationAssessment = serde_json::from_value(json!({
            "effect": "product_file_write",
            "confidence": "structured",
            "paths": [{
                "raw": "src/lib.rs",
                "normalized": "src/lib.rs",
                "inside_repo": true
            }],
            "reason_codes": ["structured_product_write"]
        }))
        .expect("mutation assessment should deserialize");

        assert_eq!(assessment.effect, ObservedEffectKind::ProductFileWrite);
        assert_eq!(assessment.confidence, ObservationConfidence::Structured);
        assert_eq!(assessment.paths[0].raw, "src/lib.rs");
        assert_eq!(assessment.reason_codes, ["structured_product_write"]);

        let schema = serde_json::to_value(schema_for!(MutationAssessment))
            .expect("MutationAssessment schema should serialize");
        let required = schema["required"]
            .as_array()
            .expect("MutationAssessment schema should have required fields");
        for field in ["effect", "confidence", "paths", "reason_codes"] {
            assert!(
                required.iter().any(|required| required == field),
                "MutationAssessment schema should require {field}: {schema}"
            );
        }
        assert!(
            schema["properties"]["reason_codes"].is_object(),
            "MutationAssessment schema should expose reason_codes: {schema}"
        );
    }

    #[test]
    fn canonical_json_hash_is_order_stable() {
        let first = json!({
            "z": 3,
            "a": {
                "b": true,
                "a": [2, 1]
            }
        });
        let second = json!({
            "a": {
                "a": [2, 1],
                "b": true
            },
            "z": 3
        });

        let canonical = canonical_json_string(&first).expect("canonical JSON should serialize");
        assert_eq!(canonical, r#"{"a":{"a":[2,1],"b":true},"z":3}"#);

        let first_hash = canonical_request_hash(&first).expect("hash should compute");
        let second_hash = canonical_request_hash(&second).expect("hash should compute");

        assert_eq!(first_hash, second_hash);
        assert_eq!(
            first_hash.as_str(),
            "sha256:22b1cca5763ebd5996581c6551cea0c733f4267c2fb26da60176f1bcac3ca5de"
        );
    }

    #[test]
    fn generated_schema_and_serde_agree_for_public_requests() {
        for (method_name, valid) in public_request_json_samples() {
            assert_schema_and_serde(method_name, valid.clone(), true);

            let mut missing_required = valid.clone();
            missing_required
                .as_object_mut()
                .expect("sample request should be an object")
                .remove(first_required_field(method_name));
            assert_schema_and_serde(method_name, missing_required, false);

            let mut unknown = valid.clone();
            unknown["unknown_public_field"] = json!(true);
            assert_schema_and_serde(method_name, unknown, false);
        }
    }

    #[test]
    fn authority_looking_fields_are_rejected_for_every_public_request() {
        for (method_name, valid) in public_request_json_samples() {
            for (field, value) in [
                ("operation_category", json!("agent_workflow")),
                ("actor_source", json!("agent_connection:conn_forged")),
                ("connection_id", json!("connection_forged")),
                ("verification_basis", json!("caller_supplied_basis")),
            ] {
                let mut forged = valid.clone();
                forged[field] = value;
                assert_schema_and_serde(method_name, forged, false);
            }

            for (field, value) in [
                ("verified", json!(true)),
                ("actor_source", json!("agent_connection:conn_forged")),
            ] {
                let mut forged = valid.clone();
                forged["envelope"][field] = value;
                assert_schema_and_serde(method_name, forged, false);
            }
        }
    }

    #[test]
    fn required_nullable_presence_parity_covers_public_requests() {
        for (method_name, valid) in public_request_json_samples() {
            let mut explicit_null = valid.clone();
            set_path(
                &mut explicit_null,
                &["envelope", "idempotency_key"],
                Value::Null,
            );
            assert_schema_and_serde(method_name, explicit_null, true);

            let mut missing = valid;
            remove_path(&mut missing, &["envelope", "idempotency_key"]);
            assert_schema_and_serde(method_name, missing, false);
        }

        for (method_name, path) in required_nullable_request_paths() {
            let mut explicit_null = sample_for_method(method_name);
            set_path(&mut explicit_null, path, Value::Null);
            assert_schema_and_serde(method_name, explicit_null, true);

            let mut missing = sample_for_method(method_name);
            remove_path(&mut missing, path);
            assert_schema_and_serde(method_name, missing, false);
        }
    }

    #[test]
    fn record_run_performed_operation_is_required_nullable() {
        let schema = public_request_schema("volicord.record_run").expect("record_run schema");
        assert!(schema["required"]
            .as_array()
            .expect("record_run required fields")
            .iter()
            .any(|field| field == "performed_operation"));
        assert_schema_allows_null_property(&schema, "performed_operation");

        let mut omitted = record_run_request_json();
        omitted
            .as_object_mut()
            .expect("record_run request object")
            .remove("performed_operation");
        assert_schema_and_serde("volicord.record_run", omitted, false);

        let mut explicit_null = record_run_request_json();
        explicit_null["performed_operation"] = Value::Null;
        assert_schema_and_serde("volicord.record_run", explicit_null.clone(), true);
        let decoded: RecordRunRequest =
            serde_json::from_value(explicit_null).expect("explicit null operation should decode");
        assert!(decoded.performed_operation.is_none());
        assert!(
            serde_json::to_value(decoded).expect("record_run request should serialize")
                ["performed_operation"]
                .is_null()
        );
    }

    #[test]
    fn owner_extension_field_omission_remains_accepted_where_documented_open() {
        let mut update = update_scope_request_json();
        remove_path(&mut update, &["change_unit", "scope_summary"]);
        remove_path(&mut update, &["change_unit", "affected_paths"]);
        assert_schema_and_serde("volicord.update_scope", update, true);
    }

    #[test]
    fn required_nullable_fields_must_be_present_but_accept_null() {
        let mut stage = stage_artifact_request_json();
        stage["expected_sha256"] = Value::Null;
        assert_schema_and_serde("volicord.stage_artifact", stage.clone(), true);
        stage
            .as_object_mut()
            .expect("stage request should be an object")
            .remove("expected_sha256");
        assert_schema_and_serde("volicord.stage_artifact", stage, false);

        let mut envelope_missing_nullable = status_request_json();
        envelope_missing_nullable["envelope"]
            .as_object_mut()
            .expect("envelope should be an object")
            .remove("idempotency_key");
        assert_schema_and_serde("volicord.status", envelope_missing_nullable, false);

        let mut selected_option_missing = resolve_user_action_request_json();
        selected_option_missing["resolution"]
            .as_object_mut()
            .expect("resolution should be an object")
            .remove("selected_option_id");
        assert_schema_and_serde(
            "volicord.resolve_user_action",
            selected_option_missing,
            false,
        );
    }

    #[test]
    fn public_timestamp_inputs_reject_invalid_strings() {
        for invalid in ["zzzz", "tomorrow", "9999"] {
            let mut request = request_user_action_request_json();
            request["expires_at"] = json!(invalid);
            assert!(
                deserialize_public_request("volicord.request_user_action", request).is_err(),
                "request_user_action expires_at should reject {invalid}"
            );
        }

        let mut request = request_user_action_request_json();
        request["action"]["sensitive_action_scope"] = sensitive_action_scope_json(json!("zzzz"));
        assert!(
            deserialize_public_request("volicord.request_user_action", request).is_err(),
            "request_user_action action.sensitive_action_scope.expires_at should reject invalid text"
        );

        let mut run = record_run_request_json();
        run["artifact_inputs"] = json!([staged_artifact_input_json("9999")]);
        assert!(
            deserialize_public_request("volicord.record_run", run).is_err(),
            "record_run staged_artifact_handle.expires_at should reject invalid text"
        );
    }

    #[test]
    fn timestamp_serialization_normalizes_to_canonical_utc() {
        let without_fraction: UtcTimestamp =
            serde_json::from_value(json!("2026-06-18T09:00:00+09:00"))
                .expect("offset timestamp should decode");
        assert_eq!(
            serde_json::to_value(&without_fraction).expect("timestamp should serialize"),
            json!("2026-06-18T00:00:00Z")
        );

        let with_fraction: UtcTimestamp =
            serde_json::from_value(json!("2026-06-18T09:00:00.123400+09:00"))
                .expect("fractional offset timestamp should decode");
        assert_eq!(
            serde_json::to_value(&with_fraction).expect("timestamp should serialize"),
            json!("2026-06-18T00:00:00.123400Z")
        );
    }

    #[test]
    fn equivalent_timestamp_offsets_have_equal_canonical_request_hashes() {
        let mut zulu = request_user_action_request_json();
        zulu["expires_at"] = json!("2026-06-18T00:00:00Z");
        let mut offset = request_user_action_request_json();
        offset["expires_at"] = json!("2026-06-18T09:00:00+09:00");

        assert_eq!(
            typed_request_hash("volicord.request_user_action", zulu),
            typed_request_hash("volicord.request_user_action", offset.clone())
        );

        let decoded: RequestUserActionRequest =
            serde_json::from_value(offset).expect("offset request should decode");
        assert_eq!(
            serde_json::to_value(decoded.expires_at).expect("expires_at should serialize"),
            json!("2026-06-18T00:00:00Z")
        );
    }

    #[test]
    fn generated_request_schemas_mark_only_documented_fields_required() {
        for (method_name, _) in public_request_json_samples() {
            let schema = public_request_schema(method_name).expect("schema should exist");
            assert_required(
                &schema,
                expected_required_fields(method_name),
                &format!("{method_name} root"),
            );
            assert_eq!(
                schema["additionalProperties"], false,
                "{method_name} should be an exact request object"
            );
        }

        let stage = public_request_schema("volicord.stage_artifact").expect("stage schema");
        assert_required(
            definition(&stage, "ToolEnvelope"),
            &[
                "project_id",
                "task_id",
                "request_id",
                "idempotency_key",
                "expected_state_version",
                "locale",
            ],
            "ToolEnvelope",
        );
        assert_required(
            &stage,
            expected_required_fields("volicord.stage_artifact"),
            "StageArtifactRequest",
        );
        assert_schema_allows_null_property(&stage, "expected_sha256");

        let status = public_request_schema("volicord.status").expect("status schema");
        assert_required(&status, &["envelope", "include"], "StatusRequest");
        assert_schema_allows_null_property(&status, "continuity_page");
        let continuity_page = definition(&status, "ContinuityPageRequest");
        assert_required(
            continuity_page,
            &["page_size", "cursor"],
            "ContinuityPageRequest",
        );
        assert_eq!(continuity_page["additionalProperties"], false);
        assert_eq!(continuity_page["properties"]["page_size"]["minimum"], 1.0);
        assert_eq!(continuity_page["properties"]["page_size"]["maximum"], 64.0);
        let continuity_cursor = definition(&status, "ContinuityCursor");
        assert_required(
            continuity_cursor,
            &["updated_at", "continuity_record_id"],
            "ContinuityCursor",
        );
        assert_eq!(continuity_cursor["additionalProperties"], false);

        let record = public_request_schema("volicord.record_run").expect("record_run schema");
        assert_schema_allows_null_property(&record, "close_assessment");
        assert_required(
            definition(&record, "CloseAssessmentInput"),
            &[
                "result_summary",
                "result_refs",
                "residual_risks",
                "sensitive_categories",
                "recovery_constraints",
            ],
            "CloseAssessmentInput",
        );
        assert_required(
            definition(&record, "ResidualRiskInput"),
            &[
                "summary",
                "consequence",
                "acceptance_required",
                "source_refs",
            ],
            "ResidualRiskInput",
        );
        assert_required(
            definition(&record, "ObservedChanges"),
            &[
                "changed_paths",
                "product_file_write_observed",
                "sensitive_categories",
                "baseline_ref",
            ],
            "ObservedChanges",
        );
        assert_required(
            definition(&record, "ArtifactInput"),
            &[
                "artifact_input_id",
                "source_kind",
                "staged_artifact_handle",
                "existing_artifact_ref",
                "relation_hint",
                "evidence_target",
                "expected_sha256",
                "expected_size_bytes",
                "redaction_state",
            ],
            "ArtifactInput",
        );
        assert_required(
            definition(&record, "EvidenceCoverageUpdate"),
            &[
                "target",
                "coverage_state",
                "supporting_run_refs",
                "observation_refs",
                "supporting_artifact_refs",
                "gap_refs",
            ],
            "EvidenceCoverageUpdate",
        );
        assert_required(
            definition(&record, "EvidenceObservationInput"),
            &[
                "target",
                "source_kind",
                "assurance_level",
                "observed_by_actor_source",
                "tool_name",
                "tool_invocation_id",
                "tool_metadata",
                "input_refs",
                "source_refs",
                "output_artifact_refs",
                "limitations",
                "observed_at",
            ],
            "EvidenceObservationInput",
        );
    }

    #[test]
    fn every_canonical_method_normalizes_false_and_omitted_dry_run_identically() {
        for contract in PUBLIC_METHOD_CONTRACTS {
            let method_name = contract.method().as_str();
            let explicit_false = sample_for_method(method_name);
            assert_eq!(
                explicit_false.pointer("/envelope/dry_run"),
                Some(&Value::Bool(false)),
                "{method_name} sample must exercise explicit dry_run=false"
            );

            let mut omitted = explicit_false.clone();
            omitted["envelope"]
                .as_object_mut()
                .expect("public request envelope")
                .remove("dry_run");
            assert_schema_and_serde(method_name, omitted.clone(), true);
            assert_eq!(
                typed_request_hash(method_name, explicit_false),
                typed_request_hash(method_name, omitted),
                "{method_name} must normalize omitted dry_run to the explicit false intent"
            );
            assert_eq!(
                contract.dry_run_policy().route(DryRunIntent::NotRequested),
                DryRunRequestRoute::Result
            );
        }
    }

    #[test]
    fn generated_public_response_schemas_are_root_objects() {
        for method_name in [
            "volicord.intake",
            "volicord.update_scope",
            "volicord.status",
            "volicord.get_operation_result",
            "volicord.check_close",
            "volicord.prepare_evidence_capture",
            "volicord.prepare_write",
            "volicord.stage_artifact",
            "volicord.record_run",
            "volicord.request_user_action",
            "volicord.resolve_user_action",
            "volicord.reconcile_changes",
            "volicord.close_task",
        ] {
            let schema = public_response_schema(method_name)
                .unwrap_or_else(|| panic!("missing public response schema for {method_name}"));
            assert_eq!(
                schema["type"], "object",
                "{method_name} response schema must have an object root"
            );
            assert!(
                schema["anyOf"].is_array(),
                "{method_name} response schema should expose public response branches"
            );
            let rendered =
                serde_json::to_string(&schema).expect("public response schema should serialize");
            assert!(
                !rendered.contains("MCP_UNAVAILABLE"),
                "{method_name} public response schema must not expose MCP wire identities"
            );
        }

        assert!(public_response_schema("volicord.unknown").is_none());
    }

    #[test]
    fn tool_error_decodes_only_the_closed_required_nullable_shape() {
        for details in [Value::Null, json!({"field": "summary"})] {
            let value = tool_error_json(details);
            let decoded: ToolError =
                serde_json::from_value(value.clone()).expect("complete ToolError should decode");
            assert_eq!(
                serde_json::to_value(decoded).expect("ToolError should serialize"),
                value
            );
        }

        for field in ["category", "code", "message", "retryable", "details"] {
            let mut missing = tool_error_json(Value::Null);
            missing
                .as_object_mut()
                .expect("ToolError fixture should be an object")
                .remove(field);
            assert!(
                serde_json::from_value::<ToolError>(missing).is_err(),
                "ToolError should require {field}"
            );
        }

        for details in [json!("text"), json!(7), json!([{"field": "summary"}])] {
            assert!(
                serde_json::from_value::<ToolError>(tool_error_json(details)).is_err(),
                "ToolError should reject a non-object non-null details value"
            );
        }

        let mut unknown = tool_error_json(Value::Null);
        unknown["unexpected"] = json!(true);
        assert!(serde_json::from_value::<ToolError>(unknown).is_err());

        for (field, value) in [
            ("category", json!("unknown")),
            ("code", json!("UNKNOWN_ERROR")),
        ] {
            let mut invalid = tool_error_json(Value::Null);
            invalid[field] = value;
            assert!(
                serde_json::from_value::<ToolError>(invalid).is_err(),
                "ToolError should reject invalid {field}"
            );
        }
    }

    #[test]
    fn tool_error_rejects_duplicate_wire_properties() {
        for duplicate in [
            r#"{"category":"rejected","category":"rejected","code":"VALIDATION_FAILED","message":"invalid request","retryable":false,"details":null}"#,
            r#"{"category":"rejected","code":"VALIDATION_FAILED","message":"invalid request","retryable":false,"details":null,"details":{}}"#,
            r#"{"category":"rejected","code":"VALIDATION_FAILED","code":"VALIDATION_FAILED","message":"invalid request","retryable":false,"details":null}"#,
            r#"{"category":"rejected","code":"VALIDATION_FAILED","message":"invalid request","message":"invalid request","retryable":false,"details":null}"#,
            r#"{"category":"rejected","code":"VALIDATION_FAILED","message":"invalid request","retryable":false,"retryable":false,"details":null}"#,
        ] {
            assert!(
                serde_json::from_str::<ToolError>(duplicate).is_err(),
                "ToolError should reject duplicate properties: {duplicate}"
            );
        }
    }

    #[test]
    fn tool_error_constructor_serializes_explicit_null_or_object_details() {
        let without_details =
            ToolError::new(ErrorCode::ValidationFailed, "invalid request", false, None);
        let serialized =
            serde_json::to_value(without_details).expect("ToolError without details serializes");
        assert_eq!(serialized["details"], Value::Null);
        assert!(serialized
            .as_object()
            .expect("ToolError should serialize as an object")
            .contains_key("details"));

        let details = JsonObject::from_iter([("field".to_owned(), json!("summary"))]);
        let with_details = ToolError::new(
            ErrorCode::ValidationFailed,
            "invalid request",
            false,
            Some(details.clone()),
        );
        assert_eq!(
            serde_json::to_value(with_details).expect("ToolError with details serializes")
                ["details"],
            Value::Object(details)
        );
    }

    #[test]
    fn tool_error_schema_requires_nullable_details_and_closes_the_object() {
        let schema = serde_json::to_value(schema_for!(ToolError))
            .expect("ToolError schema should serialize");
        assert_required(
            &schema,
            &["category", "code", "message", "retryable", "details"],
            "ToolError",
        );
        assert_eq!(schema["additionalProperties"], false);
        assert_schema_allows_null_property(&schema, "details");
        assert!(validate_json_schema(&schema, &tool_error_json(Value::Null)).is_ok());
        assert!(
            validate_json_schema(&schema, &tool_error_json(json!({"field": "summary"}))).is_ok()
        );

        for field in ["category", "code", "message", "retryable", "details"] {
            let mut missing = tool_error_json(Value::Null);
            missing
                .as_object_mut()
                .expect("ToolError fixture should be an object")
                .remove(field);
            assert!(
                validate_json_schema(&schema, &missing).is_err(),
                "ToolError schema should require {field}"
            );
        }
        for details in [json!("text"), json!(7), json!([{"field": "summary"}])] {
            assert!(validate_json_schema(&schema, &tool_error_json(details)).is_err());
        }
        let mut unknown = tool_error_json(Value::Null);
        unknown["unexpected"] = json!(true);
        assert!(validate_json_schema(&schema, &unknown).is_err());
        for (field, value) in [
            ("category", json!("unknown")),
            ("code", json!("UNKNOWN_ERROR")),
        ] {
            let mut invalid = tool_error_json(Value::Null);
            invalid[field] = value;
            assert!(
                validate_json_schema(&schema, &invalid).is_err(),
                "ToolError schema should reject invalid {field}"
            );
        }
    }

    #[test]
    fn every_public_error_pair_round_trips_and_every_wrong_pair_is_rejected() {
        let schema = serde_json::to_value(schema_for!(ToolError))
            .expect("ToolError schema should serialize");
        assert_eq!(
            schema["oneOf"]
                .as_array()
                .expect("ToolError schema should expose relational branches")
                .len(),
            PUBLIC_ERROR_CODE_CONTRACTS.len()
        );

        for contract in PUBLIC_ERROR_CODE_CONTRACTS {
            let details = if WorkflowRejectionDetails::is_required_for(contract.code()) {
                workflow_rejection_details_json()
            } else {
                Value::Null
            };
            let canonical = json!({
                "category": contract.category().as_str(),
                "code": contract.wire_name(),
                "message": "fixture",
                "retryable": false,
                "details": details,
            });
            let decoded: ToolError = serde_json::from_value(canonical.clone())
                .unwrap_or_else(|error| panic!("{} should decode: {error}", contract.wire_name()));
            assert_eq!(decoded.code(), contract.code());
            assert_eq!(decoded.category(), contract.category());
            assert_eq!(decoded.message(), "fixture");
            assert!(!decoded.retryable());
            assert_eq!(
                decoded.details().is_some(),
                WorkflowRejectionDetails::is_required_for(contract.code())
            );
            assert_eq!(
                serde_json::to_value(decoded).expect("ToolError should serialize"),
                canonical
            );
            assert!(
                validate_json_schema(&schema, &canonical).is_ok(),
                "{} canonical pair should satisfy the schema",
                contract.wire_name()
            );

            for wrong_category in FailureCategory::ALL
                .iter()
                .copied()
                .filter(|category| *category != contract.category())
            {
                let mut mismatch = canonical.clone();
                mismatch["category"] = json!(wrong_category.as_str());
                assert!(
                    serde_json::from_value::<ToolError>(mismatch.clone()).is_err(),
                    "{} with {} should fail Serde",
                    contract.wire_name(),
                    wrong_category.as_str()
                );
                assert!(
                    validate_json_schema(&schema, &mismatch).is_err(),
                    "{} with {} should fail the schema",
                    contract.wire_name(),
                    wrong_category.as_str()
                );
            }

            if WorkflowRejectionDetails::is_required_for(contract.code()) {
                let mut missing_details = canonical.clone();
                missing_details["details"] = Value::Null;
                assert!(serde_json::from_value::<ToolError>(missing_details.clone()).is_err());
                assert!(validate_json_schema(&schema, &missing_details).is_err());
            }
        }
    }

    #[test]
    fn public_response_families_decode_only_closed_branch_shapes() {
        let rejection = valid_rejection_response_json(false);
        let preview = valid_dry_run_response_json();

        for contract in PUBLIC_METHOD_CONTRACTS {
            let result = example_from_schema(&contract.result_schema());
            assert_public_response_acceptance(contract, result.clone(), true, "valid result");
            assert_public_response_acceptance(contract, rejection.clone(), true, "valid rejection");

            let supports_preview = contract.supports_response_branch(MethodResponseBranch::DryRun);
            assert_public_response_acceptance(
                contract,
                preview.clone(),
                supports_preview,
                "declared preview",
            );

            let mut rejection_with_result_kind = rejection.clone();
            rejection_with_result_kind["base"]["response_kind"] = json!("result");
            assert_public_response_acceptance(
                contract,
                rejection_with_result_kind,
                false,
                "rejection with result discriminant",
            );

            let mut rejection_with_effect = rejection.clone();
            rejection_with_effect["base"]["effect_kind"] = json!("core_committed");
            assert_public_response_acceptance(
                contract,
                rejection_with_effect,
                false,
                "rejection with result effect",
            );

            let mut result_with_preview_kind = result.clone();
            result_with_preview_kind["base"]["response_kind"] = json!("dry_run");
            assert_public_response_acceptance(
                contract,
                result_with_preview_kind,
                false,
                "result with preview discriminant",
            );

            let mut result_with_rejection_field = result.clone();
            result_with_rejection_field["errors"] = rejection["errors"].clone();
            assert_public_response_acceptance(
                contract,
                result_with_rejection_field,
                false,
                "result with rejection-only field",
            );

            let mut rejection_with_result_field = rejection.clone();
            let (result_field, result_value) = result
                .as_object()
                .expect("generated result example should be an object")
                .iter()
                .find(|(field, _)| field.as_str() != "base")
                .expect("every public result should have a method-owned field");
            rejection_with_result_field[result_field] = result_value.clone();
            assert_public_response_acceptance(
                contract,
                rejection_with_result_field,
                false,
                "rejection with result-only field",
            );

            let mut rejection_with_preview_field = rejection.clone();
            rejection_with_preview_field["dry_run_summary"] = preview["dry_run_summary"].clone();
            assert_public_response_acceptance(
                contract,
                rejection_with_preview_field,
                false,
                "rejection with preview-only field",
            );

            for (label, mut value) in [
                ("unknown result base field", result.clone()),
                ("unknown rejection base field", rejection.clone()),
                ("unknown preview base field", preview.clone()),
            ] {
                value["base"]["unknown_base_field"] = json!(true);
                assert_public_response_acceptance(contract, value, false, label);
            }

            for (label, mut value) in [
                ("unknown result branch field", result.clone()),
                ("unknown rejection branch field", rejection.clone()),
                ("unknown preview branch field", preview.clone()),
            ] {
                value["unknown_branch_field"] = json!(true);
                assert_public_response_acceptance(contract, value, false, label);
            }
        }
    }

    #[test]
    fn response_base_constants_are_identical_in_schema_and_serde() {
        let rejection = valid_rejection_response_json(true);
        let preview = valid_dry_run_response_json();
        let previewable = public_method_contract(MethodName::Intake);

        for (label, mut value) in [
            ("preview with rejection discriminant", preview.clone()),
            ("preview with result effect", preview.clone()),
            ("preview with dry_run false", preview.clone()),
        ] {
            match label {
                "preview with rejection discriminant" => {
                    value["base"]["response_kind"] = json!("rejected")
                }
                "preview with result effect" => value["base"]["effect_kind"] = json!("read_only"),
                "preview with dry_run false" => value["base"]["dry_run"] = json!(false),
                _ => unreachable!(),
            }
            assert_public_response_acceptance(previewable, value, false, label);
        }

        let non_previewable = public_method_contract(MethodName::Status);
        assert_public_response_acceptance(
            non_previewable,
            preview,
            false,
            "fabricated preview for non-previewable method",
        );
        assert_public_response_acceptance(
            previewable,
            rejection,
            true,
            "dry-run request rejection",
        );
    }

    #[test]
    fn method_result_fields_compose_every_affected_public_result_schema() {
        assert_method_result_schema::<IntakeRequest, IntakeResultFields, IntakeResult>(
            "volicord.intake",
            "IntakeResult",
        );
        assert_method_result_schema::<UpdateScopeRequest, UpdateScopeResultFields, UpdateScopeResult>(
            "volicord.update_scope",
            "UpdateScopeResult",
        );
        assert_method_result_schema::<StatusRequest, StatusResultFields, StatusResult>(
            "volicord.status",
            "StatusResult",
        );
        assert_method_result_schema::<
            GetOperationResultRequest,
            GetOperationResultResultFields,
            GetOperationResultResult,
        >("volicord.get_operation_result", "GetOperationResultResult");
        assert_method_result_schema::<
            CheckCloseRequest,
            CloseAssessmentResultFields,
            CheckCloseResult,
        >("volicord.check_close", "CheckCloseResult");
        assert_method_result_schema::<
            PrepareEvidenceCaptureRequest,
            PrepareEvidenceCaptureResultFields,
            PrepareEvidenceCaptureResult,
        >(
            "volicord.prepare_evidence_capture",
            "PrepareEvidenceCaptureResult",
        );
        assert_method_result_schema::<
            PrepareWriteRequest,
            PrepareWriteResultFields,
            PrepareWriteResult,
        >("volicord.prepare_write", "PrepareWriteResult");
        assert_method_result_schema::<
            StageArtifactRequest,
            StageArtifactResultFields,
            StageArtifactResult,
        >("volicord.stage_artifact", "StageArtifactResult");
        assert_method_result_schema::<RecordRunRequest, RecordRunResultFields, RecordRunResult>(
            "volicord.record_run",
            "RecordRunResult",
        );
        assert_method_result_schema::<
            RequestUserActionRequest,
            RequestUserActionResultFields,
            RequestUserActionResult,
        >("volicord.request_user_action", "RequestUserActionResult");
        assert_method_result_schema::<
            ResolveUserActionRequest,
            ResolveUserActionResultFields,
            ResolveUserActionResult,
        >("volicord.resolve_user_action", "ResolveUserActionResult");
        assert_method_result_schema::<
            ReconcileChangesRequest,
            ReconcileChangesResultFields,
            ReconcileChangesResult,
        >("volicord.reconcile_changes", "ReconcileChangesResult");
        assert_method_result_schema::<CloseTaskRequest, CloseAssessmentResultFields, CloseTaskResult>(
            "volicord.close_task",
            "CloseTaskResult",
        );
    }

    #[test]
    fn operation_result_contract_is_strict_and_paged() {
        let request = public_request_schema("volicord.get_operation_result")
            .expect("operation-result request schema");
        assert_required(
            &request,
            &["envelope", "operation_result_ref", "cursor"],
            "GetOperationResultRequest",
        );
        assert_eq!(request["additionalProperties"], false);
        assert_required(
            definition(&request, "OperationResultRef"),
            &[
                "project_id",
                "source_method",
                "source_idempotency_key",
                "committed_state_version",
                "response_sha256",
                "response_size_bytes",
            ],
            "OperationResultRef",
        );

        let response = public_response_schema("volicord.get_operation_result")
            .expect("operation-result response schema");
        assert_required(
            definition(&response, "GetOperationResultResult"),
            &[
                "base",
                "operation_result_ref",
                "start_offset_bytes",
                "end_offset_bytes",
                "chunk_utf8",
                "next_cursor",
                "complete",
                "historical",
                "current_authority_refresh_required",
            ],
            "GetOperationResultResult",
        );

        assert_eq!(MAX_OPERATION_RESULT_PAGE_BYTES, 16_384);
        assert_eq!(
            serde_json::to_value(ErrorCode::OperationResultUnavailable)
                .expect("operation-result error code should serialize"),
            json!("OPERATION_RESULT_UNAVAILABLE")
        );
    }

    #[test]
    fn user_action_channels_have_one_exhaustive_verification_basis_mapping() {
        let rows = [(
            UserActionChannelKind::Cli,
            UserActionVerificationBasis::CliDirectUserChannel,
        )];
        for (channel_kind, verification_basis) in rows {
            assert_eq!(channel_kind.verification_basis(), verification_basis);
            assert_eq!(
                UserActionChannelKind::from_verification_basis(verification_basis),
                channel_kind
            );
            assert_eq!(
                UserActionVerificationBasis::parse(verification_basis.as_str()),
                Some(verification_basis)
            );
        }
        assert_eq!(
            UserActionVerificationBasis::parse("unsupported_user_channel"),
            None
        );
    }

    #[test]
    fn channel_submission_id_runtime_and_schema_share_visible_ascii_byte_bounds() {
        assert!(validate_channel_submission_id(&"x".repeat(256)).is_ok());
        for rejected in [
            String::new(),
            " ".to_owned(),
            "contains whitespace".to_owned(),
            "x".repeat(257),
            "submission\nnewline".to_owned(),
            "제출".to_owned(),
        ] {
            assert!(
                validate_channel_submission_id(&rejected).is_err(),
                "unexpectedly accepted {rejected:?}"
            );
        }

        let request = public_request_schema("volicord.resolve_user_action")
            .expect("resolve-user-action request schema");
        let submission = &request["properties"]["channel_submission_id"];
        assert_eq!(submission["minLength"], 1);
        assert_eq!(submission["maxLength"], 256);
        assert_eq!(submission["pattern"], "^[!-~]+$");

        let resolution = serde_json::to_value(schemars::schema_for!(UserActionResolution))
            .expect("user-action resolution schema serializes");
        let submission = &resolution["properties"]["channel_submission_id"];
        assert_eq!(submission["minLength"], 1);
        assert_eq!(submission["maxLength"], 256);
        assert_eq!(submission["pattern"], "^[!-~]+$");
    }

    #[test]
    fn user_action_kind_required_for_compatibility_matrix_is_exhaustive() {
        use UserActionKind::{
            Cancellation, EvidenceObservation, FinalAcceptance, ProductDecision,
            ResidualRiskAcceptance, ScopeDecision, SensitiveApproval, TechnicalDecision,
        };
        use UserActionRequiredFor::{
            AdvanceTask, CloseCancel, CloseComplete, CloseSupersede, Informational, PrepareWrite,
            RecordRun, ScopeUpdate,
        };

        let required_for_values = [
            ScopeUpdate,
            AdvanceTask,
            PrepareWrite,
            RecordRun,
            CloseComplete,
            CloseCancel,
            CloseSupersede,
            Informational,
        ];
        let rows: [(UserActionKind, &[UserActionRequiredFor]); 8] = [
            (
                ProductDecision,
                &[
                    ScopeUpdate,
                    AdvanceTask,
                    PrepareWrite,
                    RecordRun,
                    CloseComplete,
                    CloseSupersede,
                    Informational,
                ],
            ),
            (
                TechnicalDecision,
                &[
                    ScopeUpdate,
                    AdvanceTask,
                    PrepareWrite,
                    RecordRun,
                    CloseComplete,
                    CloseSupersede,
                    Informational,
                ],
            ),
            (
                ScopeDecision,
                &[
                    ScopeUpdate,
                    AdvanceTask,
                    PrepareWrite,
                    RecordRun,
                    CloseComplete,
                    CloseSupersede,
                    Informational,
                ],
            ),
            (
                SensitiveApproval,
                &[
                    AdvanceTask,
                    PrepareWrite,
                    RecordRun,
                    CloseComplete,
                    CloseSupersede,
                    Informational,
                ],
            ),
            (FinalAcceptance, &[CloseComplete, Informational]),
            (ResidualRiskAcceptance, &[CloseComplete, Informational]),
            (Cancellation, &[CloseCancel, Informational]),
            (
                EvidenceObservation,
                &[RecordRun, CloseComplete, Informational],
            ),
        ];

        for (action_kind, compatible_values) in rows {
            for required_for in required_for_values {
                assert_eq!(
                    action_kind.is_compatible_with_required_for(required_for),
                    compatible_values.contains(&required_for),
                    "unexpected compatibility for {action_kind:?} × {required_for:?}"
                );
            }
        }
    }

    #[test]
    fn close_method_family_schema_separates_read_check_from_mutation_request() {
        let check = public_request_schema("volicord.check_close").expect("check_close schema");
        assert_required(&check, &["envelope", "task_id"], "CheckCloseRequest");
        for field in ["intent", "close_reason", "superseding_task_id", "user_note"] {
            assert!(
                check["properties"].get(field).is_none(),
                "check_close must not expose close_task mutation field {field}"
            );
        }

        let mut check_with_intent = check_close_request_json();
        check_with_intent["intent"] = json!("complete");
        assert_schema_and_serde("volicord.check_close", check_with_intent, false);

        let close = public_request_schema("volicord.close_task").expect("close_task schema");
        assert_required(
            &close,
            &[
                "envelope",
                "task_id",
                "intent",
                "close_reason",
                "superseding_task_id",
                "user_note",
            ],
            "CloseTaskRequest",
        );

        let mut close_without_intent = close_task_request_json();
        remove_path(&mut close_without_intent, &["intent"]);
        assert_schema_and_serde("volicord.close_task", close_without_intent, false);

        let mut close_with_read_intent = close_task_request_json();
        close_with_read_intent["intent"] = json!("check");
        assert_schema_and_serde("volicord.close_task", close_with_read_intent, false);
    }

    #[test]
    fn request_user_action_option_input_exposes_no_authority_outcome_mapping() {
        let schema =
            public_request_schema("volicord.request_user_action").expect("user-action schema");
        let option_input = definition(&schema, "UserActionOptionInput");
        assert!(
            option_input["properties"].get("machine_action").is_none(),
            "request option input must not expose machine_action"
        );
        assert!(
            option_input["properties"]
                .get("resolution_outcome")
                .is_none(),
            "request option input must not expose resolution_outcome"
        );

        let mut request = request_user_action_request_json();
        request["action"]["judgment_kind"] = json!("cancellation");
        request["action"]["options"][0]["resolution_outcome"] = json!("accepted");
        assert_schema_and_serde("volicord.request_user_action", request, false);

        let mut request = request_user_action_request_json();
        request["action"]["judgment_kind"] = json!("cancellation");
        request["action"]["options"][0]["machine_action"] = json!("reject");
        assert_schema_and_serde("volicord.request_user_action", request, false);
    }

    #[test]
    fn current_user_action_option_requires_action_and_outcome() {
        let schema = serde_json::to_value(schemars::schema_for!(UserActionOption))
            .expect("option schema should serialize");
        assert_required(
            &schema,
            &[
                "option_id",
                "label",
                "description",
                "consequence",
                "machine_action",
                "resolution_outcome",
                "is_default",
            ],
            "UserActionOption",
        );
        assert_eq!(
            schema["additionalProperties"], false,
            "UserActionOption should be closed"
        );

        let valid = user_action_option_json();
        assert!(serde_json::from_value::<UserActionOption>(valid.clone()).is_ok());
        assert!(validate_json_schema(&schema, &valid).is_ok());

        let mut missing_action = user_action_option_json();
        remove_path(&mut missing_action, &["machine_action"]);
        assert!(serde_json::from_value::<UserActionOption>(missing_action.clone()).is_err());
        assert!(validate_json_schema(&schema, &missing_action).is_err());

        let mut missing_outcome = user_action_option_json();
        remove_path(&mut missing_outcome, &["resolution_outcome"]);
        assert!(serde_json::from_value::<UserActionOption>(missing_outcome.clone()).is_err());
        assert!(validate_json_schema(&schema, &missing_outcome).is_err());

        let mut blocked_outcome = user_action_option_json();
        blocked_outcome["resolution_outcome"] = json!("blocked");
        assert!(serde_json::from_value::<UserActionOption>(blocked_outcome.clone()).is_err());
        assert!(validate_json_schema(&schema, &blocked_outcome).is_err());

        let mut unknown = user_action_option_json();
        unknown["unsupported_note"] = json!("not current public shape");
        assert_unknown::<UserActionOption>(unknown, "unsupported_note");
    }

    #[test]
    fn docs_facing_schemas_include_authority_boundary_enum_values() {
        let record_kinds = schema_enum_strings(
            serde_json::to_value(schemars::schema_for!(StateRecordKind))
                .expect("StateRecordKind schema should serialize"),
        );
        assert_contains_all(
            "StateRecordKind",
            &record_kinds,
            &[
                "write_ticket",
                "user_action_request",
                "user_action_resolution",
                "evidence_summary",
                "evidence_observation",
                "project_continuity_record",
            ],
        );

        let blocker_categories = schema_enum_strings(
            serde_json::to_value(schemars::schema_for!(CloseReadinessBlockerCategory))
                .expect("CloseReadinessBlockerCategory schema should serialize"),
        );
        assert_contains_all(
            "CloseReadinessBlockerCategory",
            &blocker_categories,
            &[
                "evidence_claim",
                "evidence_provenance",
                "final_acceptance",
                "residual_risk_acceptance",
                "pending_user_action",
            ],
        );

        let evidence_sources = schema_enum_strings(
            serde_json::to_value(schemars::schema_for!(EvidenceSourceKind))
                .expect("EvidenceSourceKind schema should serialize"),
        );
        assert_contains_all(
            "EvidenceSourceKind",
            &evidence_sources,
            &[
                "agent_report",
                "external_tool",
                "user_observation",
                "unverified_claim",
            ],
        );

        let evidence_gate_states = schema_enum_strings(
            serde_json::to_value(schemars::schema_for!(EvidenceGateState))
                .expect("EvidenceGateState schema should serialize"),
        );
        assert_eq!(
            evidence_gate_states,
            BTreeSet::from([
                "blocked".to_owned(),
                "not_required".to_owned(),
                "optional_none".to_owned(),
                "partial".to_owned(),
                "required_missing".to_owned(),
                "stale".to_owned(),
                "sufficient".to_owned(),
            ])
        );

        let continuity_kinds = schema_enum_strings(
            serde_json::to_value(schemars::schema_for!(ProjectContinuityKind))
                .expect("ProjectContinuityKind schema should serialize"),
        );
        assert_contains_all(
            "ProjectContinuityKind",
            &continuity_kinds,
            &[
                "decision",
                "obligation",
                "known_limit",
                "accepted_risk",
                "constraint",
            ],
        );
    }

    #[test]
    fn user_action_resolution_inputs_are_closed_and_bounded() {
        let valid: UserActionResolutionInput =
            serde_json::from_value(resolve_user_action_request_json()["resolution"].clone())
                .expect("choice resolution input");
        valid.validate_bounds().expect("bounded note");

        let oversized: UserActionResolutionInput = serde_json::from_value(json!({
            "resolution_type": "choice",
            "selected_option_id": "accept",
            "note": "한".repeat(USER_ACTION_NOTE_MAX_CHARS + 1)
        }))
        .expect("shape remains syntactically valid");
        assert_eq!(
            oversized.validate_bounds().unwrap_err().field(),
            "resolution.note"
        );

        let unknown = json!({
            "resolution_type": "choice",
            "selected_option_id": "accept",
            "note": null,
            "resolution_outcome": "accepted"
        });
        assert_unknown::<UserActionResolutionInput>(unknown, "resolution_outcome");
    }

    #[test]
    fn user_action_validation_rejects_ambiguous_or_empty_closed_forms() {
        for (field, value) in [
            (
                "resolution.artifact_ids",
                json!({
                    "resolution_type": "evidence_observation",
                    "target": {"target_kind": "acceptance_criterion", "acceptance_criterion_id": "ac_1"},
                    "artifact_ids": ["artifact_1", "artifact_1"],
                    "relevance_status": "supported",
                    "summary": "Observed current output."
                }),
            ),
            (
                "resolution.relevance_status",
                json!({
                    "resolution_type": "evidence_observation",
                    "target": {"target_kind": "acceptance_criterion", "acceptance_criterion_id": "ac_1"},
                    "artifact_ids": ["artifact_1"],
                    "relevance_status": "unassessed",
                    "summary": "Observed current output."
                }),
            ),
            (
                "resolution.summary",
                json!({
                    "resolution_type": "evidence_observation",
                    "target": {"target_kind": "acceptance_criterion", "acceptance_criterion_id": "ac_1"},
                    "artifact_ids": ["artifact_1"],
                    "relevance_status": "supported",
                    "summary": "   "
                }),
            ),
        ] {
            let input: UserActionResolutionInput =
                serde_json::from_value(value).expect("observation input shape");
            assert_eq!(input.validate_bounds().unwrap_err().field(), field);
        }

        let mut choice_body = json!({
            "action_type": "choice",
            "judgment_kind": "product_decision",
            "presentation": "short",
            "question": "Choose one option.",
            "options": [user_action_option_json(), user_action_option_json()],
            "context": {
                "summary": "One bounded decision.",
                "related_refs": [],
                "artifact_refs": [],
                "visible_risks": [],
                "constraints": []
            },
            "affected_refs": [],
            "sensitive_action_scope": null
        });
        let duplicated: UserActionRequestBody =
            serde_json::from_value(choice_body.clone()).expect("choice body shape");
        assert_eq!(
            duplicated.validate_bounds().unwrap_err().field(),
            "body.options"
        );
        choice_body["options"] = json!([user_action_option_json()]);
        choice_body["question"] = json!("  ");
        let blank: UserActionRequestBody =
            serde_json::from_value(choice_body).expect("blank choice body shape");
        assert_eq!(
            blank.validate_bounds().unwrap_err().field(),
            "body.question"
        );
    }

    #[test]
    fn immutable_user_action_body_is_the_single_validated_resolution_form_owner() {
        let choice_context_private = "choice-context-must-not-enter-the-resolution-form";
        let choice_body: UserActionRequestBody = serde_json::from_value(json!({
            "action_type": "choice",
            "judgment_kind": "product_decision",
            "presentation": "short",
            "question": "Choose one exact option.",
            "options": [user_action_option_json()],
            "context": {
                "summary": choice_context_private,
                "related_refs": [],
                "artifact_refs": [],
                "visible_risks": [],
                "constraints": []
            },
            "affected_refs": [],
            "sensitive_action_scope": null
        }))
        .expect("choice request body");
        let choice_form = choice_body
            .resolution_form()
            .expect("valid choice resolution form");
        assert_eq!(
            choice_form,
            UserActionResolutionForm::Choice {
                choices: vec![UserActionResolutionChoice {
                    choice_id: UserActionOptionId::new("accept"),
                    label: "Accept".to_owned(),
                    description: "Accept the focused judgment.".to_owned(),
                    consequence: "The accepted option is recorded.".to_owned(),
                    is_default: true,
                }],
                note_allowed: true,
                note_max_chars: USER_ACTION_NOTE_MAX_CHARS as u64,
            }
        );
        assert!(!serde_json::to_string(&choice_form)
            .expect("choice form serializes")
            .contains(choice_context_private));

        let observation_context_private = "observation-context-must-not-enter-the-resolution-form";
        let target = json!({
            "target_kind": "acceptance_criterion",
            "acceptance_criterion_id": "criterion_resolution_form"
        });
        let artifact = artifact_ref_json(
            "verified",
            json!("text/plain"),
            json!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
            json!(18),
        );
        let observation_body: UserActionRequestBody = serde_json::from_value(json!({
            "action_type": "evidence_observation",
            "question": "Does the exact artifact support this target?",
            "context_summary": observation_context_private,
            "target_candidates": [target.clone()],
            "artifact_candidates": [artifact.clone()]
        }))
        .expect("observation request body");
        let observation_form = observation_body
            .resolution_form()
            .expect("valid observation resolution form");
        assert_eq!(
            serde_json::to_value(&observation_form).expect("observation form serializes"),
            json!({
                "form_type": "evidence_observation",
                "target_candidates": [target],
                "artifact_candidates": [artifact],
                "relevance_options": ["supported", "contradicted"],
                "summary_max_chars": USER_ACTION_OBSERVATION_SUMMARY_MAX_CHARS as u64
            })
        );
        assert!(!serde_json::to_string(&observation_form)
            .expect("observation form serializes")
            .contains(observation_context_private));

        let invalid_body: UserActionRequestBody = serde_json::from_value(json!({
            "action_type": "choice",
            "judgment_kind": "product_decision",
            "presentation": "short",
            "question": "  ",
            "options": [user_action_option_json()],
            "context": {
                "summary": "Invalid body must fail before projection.",
                "related_refs": [],
                "artifact_refs": [],
                "visible_risks": [],
                "constraints": []
            },
            "affected_refs": [],
            "sensitive_action_scope": null
        }))
        .expect("invalid body remains syntactically decodable");
        assert_eq!(
            invalid_body.resolution_form().unwrap_err().field(),
            "body.question"
        );
    }

    #[test]
    fn canonical_user_action_form_size_accepts_32768_and_rejects_the_next_byte() {
        fn form_with_size(target: usize) -> UserActionResolutionForm {
            let mut form = UserActionResolutionForm::Choice {
                choices: vec![UserActionResolutionChoice {
                    choice_id: UserActionOptionId::new("boundary"),
                    label: "Boundary".to_owned(),
                    description: "Canonical form byte boundary.".to_owned(),
                    consequence: String::new(),
                    is_default: true,
                }],
                note_allowed: true,
                note_max_chars: USER_ACTION_NOTE_MAX_CHARS as u64,
            };
            let base = canonical_json_size_bytes(&form).expect("form should serialize");
            let UserActionResolutionForm::Choice { choices, .. } = &mut form else {
                unreachable!("choice fixture")
            };
            choices[0].consequence = "x".repeat(target - base);
            assert_eq!(
                canonical_json_size_bytes(&form).expect("form should serialize"),
                target
            );
            form
        }

        let at_limit = form_with_size(USER_ACTION_FORM_MAX_BYTES);
        assert!(at_limit.validate_canonical_size().is_ok());
        let over_limit = form_with_size(USER_ACTION_FORM_MAX_BYTES + 1);
        let error = over_limit
            .validate_canonical_size()
            .expect_err("one byte above the canonical limit must reject");
        assert_eq!(error.field(), "form");
    }

    #[test]
    fn effective_user_action_status_has_half_open_expiry_and_basis_precedence() {
        let created_at = timestamp("2026-06-17T23:59:59Z");
        let now = timestamp("2026-06-18T00:00:00Z");
        assert_eq!(
            effective_user_action_status(
                UserActionBasisStatus::Current,
                &created_at,
                None,
                false,
                &now,
            ),
            Some(UserActionStatus::Pending)
        );
        assert_eq!(
            effective_user_action_status(
                UserActionBasisStatus::Current,
                &created_at,
                Some(&now),
                false,
                &now,
            ),
            Some(UserActionStatus::Expired)
        );
        assert_eq!(
            effective_user_action_status(
                UserActionBasisStatus::Current,
                &created_at,
                Some(&now),
                true,
                &now,
            ),
            Some(UserActionStatus::Resolved)
        );
        assert_eq!(
            effective_user_action_status(
                UserActionBasisStatus::Stale,
                &created_at,
                None,
                true,
                &now,
            ),
            Some(UserActionStatus::Stale)
        );
        assert_eq!(
            effective_user_action_status(
                UserActionBasisStatus::Superseded,
                &created_at,
                None,
                true,
                &now,
            ),
            Some(UserActionStatus::Superseded)
        );
        assert_eq!(
            effective_user_action_status(UserActionBasisStatus::Current, &now, None, false, &now,),
            Some(UserActionStatus::Pending)
        );
        assert_eq!(
            effective_user_action_status(
                UserActionBasisStatus::Stale,
                &now,
                None,
                true,
                &created_at,
            ),
            None
        );
    }

    #[test]
    fn artifact_ref_requires_integrity_status_and_rejects_unsupported_status() {
        let schema = serde_json::to_value(schemars::schema_for!(ArtifactRef))
            .expect("artifact schema should serialize");
        assert_required(
            &schema,
            &[
                "artifact_id",
                "project_id",
                "task_id",
                "display_name",
                "content_type",
                "sha256",
                "size_bytes",
                "integrity_status",
                "redaction_state",
                "availability",
                "created_by_run_ref",
                "created_by_actor_source",
                "storage_ref",
            ],
            "ArtifactRef",
        );
        assert_eq!(
            schema["additionalProperties"], false,
            "ArtifactRef should be closed"
        );

        let corrupt = artifact_ref_json("corrupt", Value::Null, Value::Null, Value::Null);
        assert!(serde_json::from_value::<ArtifactRef>(corrupt.clone()).is_ok());
        assert!(validate_json_schema(&schema, &corrupt).is_ok());

        let unsupported_status =
            artifact_ref_json("unsupported_status", Value::Null, Value::Null, Value::Null);
        assert!(serde_json::from_value::<ArtifactRef>(unsupported_status.clone()).is_err());
        assert!(validate_json_schema(&schema, &unsupported_status).is_err());

        let mut missing_integrity =
            artifact_ref_json("corrupt", Value::Null, Value::Null, Value::Null);
        remove_path(&mut missing_integrity, &["integrity_status"]);
        assert!(serde_json::from_value::<ArtifactRef>(missing_integrity.clone()).is_err());
        assert!(validate_json_schema(&schema, &missing_integrity).is_err());
    }

    #[test]
    fn artifact_input_source_kind_schema_and_serde_match_baseline_values() {
        for (kind, value) in [
            (ArtifactInputSourceKind::StagedArtifact, "staged_artifact"),
            (
                ArtifactInputSourceKind::ExistingArtifact,
                "existing_artifact",
            ),
        ] {
            assert_eq!(
                serde_json::to_value(kind).expect("artifact input source kind should serialize"),
                json!(value)
            );
            assert_eq!(
                serde_json::from_value::<ArtifactInputSourceKind>(json!(value))
                    .expect("baseline artifact input source kind should deserialize"),
                kind
            );
        }

        let schema = serde_json::to_value(schema_for!(ArtifactInputSourceKind))
            .expect("ArtifactInputSourceKind schema should serialize");
        assert_eq!(
            schema_enum_strings(schema.clone()),
            BTreeSet::from(["existing_artifact".to_owned(), "staged_artifact".to_owned(),])
        );

        for unsupported in ["captured_artifact", "native_artifact"] {
            let value = json!(unsupported);
            assert!(
                serde_json::from_value::<ArtifactInputSourceKind>(value.clone()).is_err(),
                "unsupported artifact input source kind should fail serde: {unsupported}"
            );
            assert!(
                validate_json_schema(&schema, &value).is_err(),
                "unsupported artifact input source kind should fail schema validation: {unsupported}"
            );
        }
    }

    #[test]
    fn evidence_observation_round_trips_and_rejects_unknown_fields() {
        let observation = evidence_observation_json();
        let decoded: EvidenceObservation =
            serde_json::from_value(observation.clone()).expect("observation should decode");
        assert_eq!(decoded.observation_id.as_str(), "evidence_observation_001");
        assert_eq!(decoded.source_kind, EvidenceSourceKind::ExternalTool);
        assert_eq!(
            decoded.assurance_level,
            EvidenceAssuranceLevel::ExternalToolResult
        );

        let encoded = serde_json::to_value(&decoded).expect("observation should encode");
        assert_eq!(
            encoded["target"]["evidence_claim_id"],
            "claim_search_count_001"
        );
        assert_eq!(encoded["source_kind"], "external_tool");
        assert_eq!(encoded["assurance_level"], "external_tool_result");

        let mut with_unknown = observation.clone();
        with_unknown["verified"] = json!(true);
        assert_unknown::<EvidenceObservation>(with_unknown, "verified");

        let input = evidence_observation_input_json();
        let decoded_input: EvidenceObservationInput =
            serde_json::from_value(input.clone()).expect("observation input should decode");
        assert_eq!(decoded_input.source_kind, EvidenceSourceKind::ExternalTool);
        let mut input_with_unknown = input;
        input_with_unknown["final_acceptance"] = json!(true);
        assert_unknown::<EvidenceObservationInput>(input_with_unknown, "final_acceptance");
    }

    #[test]
    fn record_run_schema_and_serde_reject_existing_artifact_ref_missing_integrity_status() {
        let mut valid = record_run_request_json();
        valid["artifact_inputs"] = json!([existing_artifact_input_json(artifact_ref_json(
            "verified",
            json!("text/plain"),
            json!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
            json!(18)
        ))]);
        assert_schema_and_serde("volicord.record_run", valid.clone(), true);

        let mut missing = valid;
        remove_path(
            &mut missing,
            &[
                "artifact_inputs",
                "0",
                "existing_artifact_ref",
                "integrity_status",
            ],
        );
        assert_schema_and_serde("volicord.record_run", missing, false);
    }

    #[test]
    fn timestamp_json_schemas_are_date_time_strings() {
        let action =
            public_request_schema("volicord.request_user_action").expect("user-action schema");
        assert_date_time_schema(
            &action,
            &action["properties"]["expires_at"],
            "RequestUserActionRequest.expires_at",
        );
        assert_date_time_schema(
            &action,
            &definition(&action, "SensitiveActionScope")["properties"]["expires_at"],
            "SensitiveActionScope.expires_at",
        );

        let run = public_request_schema("volicord.record_run").expect("record_run schema");
        assert_date_time_schema(
            &run,
            &definition(&run, "StagedArtifactHandle")["properties"]["expires_at"],
            "StagedArtifactHandle.expires_at",
        );

        let stage_result = serde_json::to_value(schemars::schema_for!(StageArtifactResult))
            .expect("stage result schema should serialize");
        assert_date_time_schema(
            &stage_result,
            &stage_result["properties"]["expires_at"],
            "StageArtifactResult.expires_at",
        );
    }

    #[test]
    fn exact_request_and_user_action_payload_objects_are_closed() {
        let record = public_request_schema("volicord.record_run").expect("record_run schema");
        for definition_name in [
            "ToolEnvelope",
            "ObservedChanges",
            "ArtifactInput",
            "StateRecordRef",
            "ArtifactRef",
            "StagedArtifactHandle",
            "EvidenceCoverageUpdate",
            "EvidenceObservationInput",
            "CloseAssessmentInput",
            "ResidualRiskInput",
        ] {
            assert_eq!(
                definition(&record, definition_name)["additionalProperties"],
                false,
                "{definition_name} should be closed"
            );
        }

        let update = public_request_schema("volicord.update_scope").expect("update schema");
        assert_ne!(
            definition(&update, "ChangeUnitUpdate")["additionalProperties"],
            false,
            "ChangeUnitUpdate intentionally carries open owner-defined fields"
        );

        let mut unknown = resolve_user_action_request_json();
        unknown["resolution"]["rationale"] = json!({ "owner_defined": true });
        assert_schema_and_serde("volicord.resolve_user_action", unknown, false);
    }

    #[test]
    fn user_action_resolution_ref_has_strict_typed_owner_identity_and_version() {
        let valid = json!({
            "record_kind": "user_action_resolution",
            "record_id": "resolution-shared",
            "project_id": "project-a",
            "task_id": "task-a",
            "produced_at_state_version": 7
        });
        let schema = serde_json::to_value(schema_for!(UserActionResolutionRef))
            .expect("resolution reference schema should serialize");
        assert!(schema["required"]
            .as_array()
            .expect("resolution reference schema should list required fields")
            .contains(&json!("produced_at_state_version")));
        assert_eq!(
            schema["properties"]["produced_at_state_version"]["type"],
            "integer"
        );
        assert!(validate_json_schema(&schema, &valid).is_ok());

        let decoded: UserActionResolutionRef =
            serde_json::from_value(valid).expect("canonical resolution reference should decode");
        assert_eq!(decoded.project_id().as_str(), "project-a");
        assert_eq!(decoded.task_id().as_str(), "task-a");
        assert_eq!(decoded.resolution_id().as_str(), "resolution-shared");
        assert_eq!(decoded.produced_at_state_version(), 7);
        assert_eq!(
            serde_json::to_value(&decoded).expect("resolution reference should serialize"),
            json!({
                "record_kind": "user_action_resolution",
                "record_id": "resolution-shared",
                "project_id": "project-a",
                "task_id": "task-a",
                "produced_at_state_version": 7
            })
        );

        let other_project = UserActionResolutionRef::new(
            ProjectId::new("project-b"),
            TaskId::new("task-a"),
            UserActionResolutionId::new("resolution-shared"),
            8,
        );
        let other_task = UserActionResolutionRef::new(
            ProjectId::new("project-a"),
            TaskId::new("task-b"),
            UserActionResolutionId::new("resolution-shared"),
            9,
        );
        assert_ne!(decoded.identity(), other_project.identity());
        assert_ne!(decoded.identity(), other_task.identity());

        for invalid in [
            json!({
                "record_kind": "task",
                "record_id": "resolution-shared",
                "project_id": "project-a",
                "task_id": "task-a",
                "produced_at_state_version": 7
            }),
            json!({
                "record_kind": "user_action_resolution",
                "record_id": "resolution-shared",
                "project_id": "project-a",
                "produced_at_state_version": 7
            }),
            json!({
                "record_kind": "user_action_resolution",
                "record_id": "resolution-shared",
                "task_id": "task-a",
                "produced_at_state_version": 7
            }),
            json!({
                "record_kind": "user_action_resolution",
                "project_id": "project-a",
                "task_id": "task-a",
                "produced_at_state_version": 7
            }),
            json!({
                "record_kind": "user_action_resolution",
                "record_id": "resolution-shared",
                "project_id": "project-a",
                "task_id": "task-a"
            }),
            json!({
                "record_kind": "user_action_resolution",
                "record_id": "resolution-shared",
                "project_id": "project-a",
                "task_id": "task-a",
                "produced_at_state_version": null
            }),
            json!({
                "record_kind": "user_action_resolution",
                "record_id": "resolution-shared",
                "project_id": "project-a",
                "task_id": "task-a",
                "produced_at_state_version": "7"
            }),
            json!({
                "record_kind": "user_action_resolution",
                "record_id": "resolution-shared",
                "project_id": "project-a",
                "task_id": "task-a",
                "produced_at_state_version": 7,
                "unexpected": true
            }),
        ] {
            assert!(validate_json_schema(&schema, &invalid).is_err());
            assert!(serde_json::from_value::<UserActionResolutionRef>(invalid).is_err());
        }
    }

    #[test]
    fn typed_request_hash_ignores_raw_order_and_preserves_semantic_differences() {
        let first_json = r#"{
            "safe_bytes_or_notice": "Local trace sample.",
            "relation_hint": "diagnostic_log",
            "expected_size_bytes": null,
            "expected_sha256": null,
            "redaction_state": "none",
            "content_type": "text/plain",
            "display_name": "diagnostic_trace.log",
            "task_id": "task_empty_001",
            "envelope": {
                "locale": "en-US",
                "dry_run": false,
                "expected_state_version": 62,
                "idempotency_key": "idem_empty_answer_001",
                "request_id": "req_empty_answer_001",
                "task_id": "task_empty_001",
                "project_id": "proj_empty_001"
            }
        }"#;
        let second_json = serde_json::to_string_pretty(&stage_artifact_request_json())
            .expect("sample should serialize");
        let first: StageArtifactRequest =
            serde_json::from_str(first_json).expect("first request should decode");
        let second: StageArtifactRequest =
            serde_json::from_str(&second_json).expect("second request should decode");

        let first_hash = canonical_request_hash(&first).expect("first hash");
        let second_hash = canonical_request_hash(&second).expect("second hash");
        assert_eq!(first_hash, second_hash);

        let mut changed = stage_artifact_request_json();
        changed["relation_hint"] = json!("other_relation");
        let changed: StageArtifactRequest =
            serde_json::from_value(changed).expect("changed request should decode");
        let changed_hash = canonical_request_hash(&changed).expect("changed hash");
        assert_ne!(first_hash, changed_hash);
    }

    #[test]
    fn typed_request_hashes_are_stable_across_public_request_serialization() {
        for (method_name, sample) in public_request_json_samples() {
            let compact_json = serde_json::to_string(&sample).expect("sample should serialize");
            let pretty_json =
                serde_json::to_string_pretty(&sample).expect("sample should serialize");
            let reordered_json = serde_json::to_string(&reversed_object_value(&sample))
                .expect("sample should serialize");

            let compact = serde_json::from_str(&compact_json).expect("compact should parse");
            let pretty = serde_json::from_str(&pretty_json).expect("pretty should parse");
            let reordered = serde_json::from_str(&reordered_json).expect("reordered should parse");

            let compact_hash = typed_request_hash(method_name, compact);
            assert_eq!(compact_hash, typed_request_hash(method_name, pretty));
            assert_eq!(compact_hash, typed_request_hash(method_name, reordered));
        }

        let null_hash = typed_request_hash("volicord.record_run", record_run_request_json());
        let mut changed = record_run_request_json();
        changed["write_ticket_id"] = json!("wt_hash_change");
        assert_ne!(
            null_hash,
            typed_request_hash("volicord.record_run", changed)
        );
    }

    fn envelope_json() -> Value {
        json!({
            "project_id": "proj_empty_001",
            "task_id": "task_empty_001",
            "request_id": "req_empty_answer_001",
            "idempotency_key": "idem_empty_answer_001",
            "expected_state_version": 62,
            "dry_run": false,
            "locale": "en-US"
        })
    }

    fn state_ref_json(record_kind: &str, record_id: &str, task_id: &str) -> Value {
        json!({
            "record_kind": record_kind,
            "record_id": record_id,
            "project_id": "proj_empty_001",
            "task_id": task_id,
            "produced_at_state_version": 11
        })
    }

    fn public_request_json_samples() -> Vec<(&'static str, Value)> {
        vec![
            ("volicord.intake", intake_request_json()),
            ("volicord.update_scope", update_scope_request_json()),
            (
                "volicord.record_shaping_checkpoint",
                record_shaping_checkpoint_request_json(),
            ),
            ("volicord.finalize_advice", finalize_advice_request_json()),
            ("volicord.advance_task", advance_task_request_json()),
            ("volicord.status", status_request_json()),
            (
                "volicord.get_operation_result",
                get_operation_result_request_json(),
            ),
            ("volicord.check_close", check_close_request_json()),
            (
                "volicord.prepare_evidence_capture",
                prepare_evidence_capture_request_json(),
            ),
            ("volicord.prepare_write", prepare_write_request_json()),
            ("volicord.stage_artifact", stage_artifact_request_json()),
            ("volicord.record_run", record_run_request_json()),
            (
                "volicord.request_user_action",
                request_user_action_request_json(),
            ),
            (
                "volicord.resolve_user_action",
                resolve_user_action_request_json(),
            ),
            (
                "volicord.reconcile_changes",
                reconcile_changes_request_json(),
            ),
            ("volicord.close_task", close_task_request_json()),
        ]
    }

    fn valid_rejection_response_json(dry_run: bool) -> Value {
        serde_json::to_value(ToolRejectedResponse::new(
            DryRunIntent::from_wire_bool(dry_run),
            Some(7),
            GuaranteeDisclosure::authority_record(),
            vec![ToolError::new(
                ErrorCode::ValidationFailed,
                "request validation failed",
                false,
                None,
            )],
        ))
        .expect("valid rejection should serialize")
    }

    fn tool_error_json(details: Value) -> Value {
        json!({
            "category": "rejected",
            "code": "VALIDATION_FAILED",
            "message": "invalid request",
            "retryable": false,
            "details": details
        })
    }

    fn workflow_rejection_details_json() -> Value {
        json!({
            "state_change_applied": false,
            "current_task_mode": "work",
            "current_work_phase": "shaping",
            "received_action": "volicord.advance_task",
            "received_run_kind": null,
            "allowed_run_kinds": [],
            "allowed_actions": ["volicord.record_shaping_checkpoint"],
            "blockers": [{
                "code": "SHAPING_CHECKPOINT_REQUIRED",
                "owner_method": "volicord.record_shaping_checkpoint",
                "required_refs": [],
                "user_actions": []
            }],
            "workflow": {
                "kind": "shaping_required",
                "next_actor": "agent",
                "required_action": "volicord.record_shaping_checkpoint",
                "allowed_actions": ["volicord.record_shaping_checkpoint"],
                "required_refs": [],
                "expected_state_version": 4,
                "blocking_reason": "no_current_checkpoint",
                "checkpoint": null,
                "action_catalog": {
                    "required_method": "volicord.record_shaping_checkpoint",
                    "actions": [{
                        "method": "volicord.record_shaping_checkpoint",
                        "semantic_variant": "create_initial",
                        "role": "required",
                        "expected_state_version": 4,
                        "fixed_authority_coordinates": {
                            "coordinate_kind": "record_shaping_checkpoint",
                            "task_id": "task_fixture",
                            "checkpoint_operation": {"operation": "create_initial"},
                            "scope_revision": 1,
                            "baseline_ref": null
                        },
                        "required_refs": []
                    }]
                }
            },
            "corrected_retry_allowed": true,
            "recovery": {"owner_method": "volicord.record_shaping_checkpoint"}
        })
    }

    fn valid_dry_run_response_json() -> Value {
        serde_json::to_value(ToolDryRunResponse::new(
            Some(7),
            GuaranteeDisclosure::authority_record(),
            DryRunSummary {
                planned_effects: Vec::new(),
                would_blockers: Vec::new(),
                would_errors: Vec::new(),
                next_actions: Vec::new(),
                diagnostics: Vec::new(),
            },
        ))
        .expect("valid preview should serialize")
    }

    fn assert_public_response_acceptance(
        contract: &PublicMethodContract,
        value: Value,
        should_accept: bool,
        label: &str,
    ) {
        let schema = contract.response_schema();
        let schema_result = validate_json_schema(&schema, &value);
        let serde_result = contract.accepts_response(&value);
        assert_eq!(
            serde_result,
            should_accept,
            "{} {label} Serde result disagrees: {value}",
            contract.method().as_str()
        );
        assert_eq!(
            schema_result.is_ok(),
            should_accept,
            "{} {label} schema result disagrees: {schema_result:?}; {value}",
            contract.method().as_str()
        );
    }

    fn example_from_schema(schema: &Value) -> Value {
        example_from_schema_node(schema, schema)
    }

    fn example_from_schema_node(root: &Value, schema: &Value) -> Value {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            return example_from_schema_node(
                root,
                resolve_ref(root, reference).expect("example schema reference should resolve"),
            );
        }
        if let Some(value) = schema.get("const") {
            return value.clone();
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            return values
                .first()
                .expect("closed schema enum should not be empty")
                .clone();
        }
        for key in ["anyOf", "oneOf"] {
            if let Some(branches) = schema.get(key).and_then(Value::as_array) {
                if branches
                    .iter()
                    .any(|branch| validate_against(root, branch, &Value::Null, "$").is_ok())
                {
                    return Value::Null;
                }
                return example_from_schema_node(
                    root,
                    branches.first().expect("schema union should not be empty"),
                );
            }
        }

        let schema_type = schema.get("type");
        if schema_type.is_some_and(|kind| {
            kind == "object"
                || kind
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "object"))
        }) || schema.get("properties").is_some()
        {
            let mut object = serde_json::Map::new();
            let empty = serde_json::Map::new();
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .unwrap_or(&empty);
            for field in schema
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                let property = properties
                    .get(field)
                    .unwrap_or_else(|| panic!("required example field {field} has no schema"));
                let value = if field.ends_with("actor_source") {
                    json!("local_user")
                } else {
                    example_from_schema_node(root, property)
                };
                object.insert(field.to_owned(), value);
            }
            if let Some(all_of) = schema.get("allOf").and_then(Value::as_array) {
                for branch in all_of {
                    let Value::Object(fields) = example_from_schema_node(root, branch) else {
                        continue;
                    };
                    object.extend(fields);
                }
            }
            return Value::Object(object);
        }
        if schema_type.is_some_and(|kind| {
            kind == "array"
                || kind
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "array"))
        }) || schema.get("items").is_some()
        {
            let count = schema.get("minItems").and_then(Value::as_u64).unwrap_or(0) as usize;
            let item = schema
                .get("items")
                .map(|item| example_from_schema_node(root, item))
                .unwrap_or(Value::Null);
            return Value::Array(vec![item; count]);
        }
        if schema_type.is_some_and(|kind| {
            kind == "boolean"
                || kind
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "boolean"))
        }) {
            return Value::Bool(false);
        }
        if schema_type.is_some_and(|kind| {
            kind == "integer"
                || kind
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "integer"))
        }) {
            return json!(schema.get("minimum").and_then(Value::as_i64).unwrap_or(0));
        }
        if schema_type.is_some_and(|kind| {
            kind == "number"
                || kind
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "number"))
        }) {
            return json!(schema.get("minimum").and_then(Value::as_f64).unwrap_or(0.0));
        }
        if schema_type.is_some_and(|kind| {
            kind == "string"
                || kind
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "string"))
        }) {
            if schema.get("format").and_then(Value::as_str) == Some("date-time") {
                return json!("2026-07-28T00:00:00Z");
            }
            if schema.get("title").and_then(Value::as_str) == Some("ActorSource") {
                return json!("local_user");
            }
            let length = schema.get("minLength").and_then(Value::as_u64).unwrap_or(1) as usize;
            return Value::String("x".repeat(length.max(1)));
        }
        if schema_type.is_some_and(|kind| {
            kind == "null"
                || kind
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "null"))
        }) {
            return Value::Null;
        }
        panic!("cannot build a valid example for schema node {schema}");
    }

    fn schema_enum_strings(schema: Value) -> BTreeSet<String> {
        schema
            .get("enum")
            .and_then(Value::as_array)
            .expect("schema should expose a direct enum")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("enum value should be a string: {value:?}"))
                    .to_owned()
            })
            .collect()
    }

    fn assert_contains_all(label: &str, actual: &BTreeSet<String>, expected: &[&str]) {
        for value in expected {
            assert!(
                actual.contains(*value),
                "{label} should include {value}, got {actual:?}"
            );
        }
    }

    fn assert_schema_and_serde(method_name: &str, value: Value, should_accept: bool) {
        let schema = public_request_schema(method_name).expect("schema should exist");
        let schema_result = validate_json_schema(&schema, &value);
        let serde_result = deserialize_public_request(method_name, value);
        assert_eq!(
            schema_result.is_ok(),
            should_accept,
            "{method_name} schema result: {schema_result:?}"
        );
        assert_eq!(
            serde_result.is_ok(),
            should_accept,
            "{method_name} serde result: {serde_result:?}"
        );
        assert_eq!(
            schema_result.is_ok(),
            serde_result.is_ok(),
            "{method_name} schema and serde should agree"
        );
    }

    fn validate_json_schema(schema: &Value, instance: &Value) -> Result<(), String> {
        validate_against(schema, schema, instance, "$")
    }

    fn validate_against(
        root: &Value,
        schema: &Value,
        instance: &Value,
        path: &str,
    ) -> Result<(), String> {
        match schema {
            Value::Bool(true) => return Ok(()),
            Value::Bool(false) => return Err(format!("{path}: schema is false")),
            Value::Object(_) => {}
            _ => return Err(format!("{path}: schema must be object or bool")),
        }

        if schema.get("nullable").and_then(Value::as_bool) == Some(true) && instance.is_null() {
            return Ok(());
        }
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            return validate_against(root, resolve_ref(root, reference)?, instance, path);
        }
        if let Some(any_of) = schema.get("anyOf").and_then(Value::as_array) {
            if any_of
                .iter()
                .any(|candidate| validate_against(root, candidate, instance, path).is_ok())
            {
                return Ok(());
            }
            return Err(format!("{path}: did not match anyOf"));
        }
        if let Some(one_of) = schema.get("oneOf").and_then(Value::as_array) {
            let matches = one_of
                .iter()
                .filter(|candidate| validate_against(root, candidate, instance, path).is_ok())
                .count();
            if matches != 1 {
                return Err(format!("{path}: matched {matches} oneOf branches"));
            }
        }
        if let Some(all_of) = schema.get("allOf").and_then(Value::as_array) {
            for candidate in all_of {
                validate_against(root, candidate, instance, path)?;
            }
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            if !values.iter().any(|value| value == instance) {
                return Err(format!("{path}: enum mismatch"));
            }
        }
        if let Some(schema_type) = schema.get("type") {
            validate_type(schema_type, instance, path)?;
        }

        if instance.is_object()
            && (schema.get("properties").is_some()
                || schema.get("required").is_some()
                || schema.get("additionalProperties").is_some())
        {
            validate_object(root, schema, instance, path)?;
        }
        if let (Some(items), Some(array)) = (schema.get("items"), instance.as_array()) {
            for (index, item) in array.iter().enumerate() {
                validate_against(root, items, item, &format!("{path}[{index}]"))?;
            }
        }
        Ok(())
    }

    fn validate_type(schema_type: &Value, instance: &Value, path: &str) -> Result<(), String> {
        let accepts = match schema_type {
            Value::String(kind) => json_type_matches(kind, instance),
            Value::Array(kinds) => kinds
                .iter()
                .filter_map(Value::as_str)
                .any(|kind| json_type_matches(kind, instance)),
            _ => return Err(format!("{path}: invalid type schema")),
        };
        if accepts {
            Ok(())
        } else {
            Err(format!("{path}: type mismatch for {schema_type}"))
        }
    }

    fn json_type_matches(kind: &str, value: &Value) -> bool {
        match kind {
            "null" => value.is_null(),
            "boolean" => value.is_boolean(),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "number" => value.is_number(),
            "string" => value.is_string(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            _ => false,
        }
    }

    fn validate_object(
        root: &Value,
        schema: &Value,
        instance: &Value,
        path: &str,
    ) -> Result<(), String> {
        let object = instance
            .as_object()
            .ok_or_else(|| format!("{path}: expected object"))?;
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for field in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(field) {
                    return Err(format!("{path}: missing required {field}"));
                }
            }
        }

        let empty = serde_json::Map::new();
        let properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        for (field, value) in object {
            if let Some(field_schema) = properties.get(field) {
                validate_against(root, field_schema, value, &format!("{path}.{field}"))?;
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                return Err(format!("{path}: unknown property {field}"));
            } else if let Some(additional) = schema.get("additionalProperties") {
                validate_against(root, additional, value, &format!("{path}.{field}"))?;
            }
        }
        Ok(())
    }

    fn resolve_ref<'a>(root: &'a Value, reference: &str) -> Result<&'a Value, String> {
        let Some(name) = reference.strip_prefix("#/definitions/") else {
            return Err(format!("unsupported ref {reference}"));
        };
        root.pointer(&format!("/definitions/{name}"))
            .ok_or_else(|| format!("missing definition {name}"))
    }

    fn definition<'a>(schema: &'a Value, name: &str) -> &'a Value {
        schema
            .pointer(&format!("/definitions/{name}"))
            .unwrap_or_else(|| {
                schema["definitions"]
                    .as_object()
                    .and_then(|definitions| {
                        definitions
                            .iter()
                            .find_map(|(definition_name, definition)| {
                                definition_name
                                    .starts_with(&format!("{name}_for_"))
                                    .then_some(definition)
                            })
                    })
                    .unwrap_or_else(|| panic!("missing schema definition {name}"))
            })
    }

    fn assert_method_result_schema<M, F, R>(method_name: &str, result_name: &str)
    where
        M: MethodResponseContract<ResultFields = F, Result = R>,
        F: MethodResultFields<M, Result = R> + JsonSchema,
        R: JsonSchema,
    {
        let fields = serde_json::to_value(schema_for!(F)).expect("fields schema should serialize");
        let result = serde_json::to_value(schema_for!(R)).expect("result schema should serialize");
        let public = public_response_schema(method_name)
            .unwrap_or_else(|| panic!("missing public response schema for {method_name}"));
        let public_result = definition(&public, result_name);

        assert_eq!(
            fields["additionalProperties"], false,
            "{result_name} fields"
        );
        assert_eq!(result["additionalProperties"], false, "{result_name}");
        assert_eq!(
            public_result["additionalProperties"], false,
            "{result_name} public response"
        );

        let mut expected_properties = fields["properties"]
            .as_object()
            .expect("fields schema should expose properties")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert!(
            expected_properties.insert("base".to_owned()),
            "method fields must not declare base"
        );
        let result_properties = result["properties"]
            .as_object()
            .expect("result schema should expose properties")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let public_properties = public_result["properties"]
            .as_object()
            .expect("public result schema should expose properties")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        assert_eq!(result_properties, expected_properties, "{result_name}");
        assert_eq!(
            public_properties, expected_properties,
            "{result_name} public response"
        );

        let mut expected_required = fields["required"]
            .as_array()
            .expect("fields schema should expose required properties")
            .iter()
            .map(|value| value.as_str().expect("required field").to_owned())
            .collect::<BTreeSet<_>>();
        assert!(expected_required.insert("base".to_owned()));
        let result_required = result["required"]
            .as_array()
            .expect("result schema should expose required properties")
            .iter()
            .map(|value| value.as_str().expect("required field").to_owned())
            .collect::<BTreeSet<_>>();
        let public_required = public_result["required"]
            .as_array()
            .expect("public result schema should expose required properties")
            .iter()
            .map(|value| value.as_str().expect("required field").to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(result_required, expected_required, "{result_name}");
        assert_eq!(
            public_required, expected_required,
            "{result_name} public response"
        );
    }

    fn assert_required(schema: &Value, expected: &[&str], label: &str) {
        let actual = schema["required"]
            .as_array()
            .expect("schema should have required array")
            .iter()
            .map(|value| value.as_str().expect("required field"))
            .collect::<BTreeSet<_>>();
        let expected = expected.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{label}");
    }

    fn assert_schema_allows_null_property(schema: &Value, field: &str) {
        let property = &schema["properties"][field];
        assert!(
            validate_against(schema, property, &Value::Null, field).is_ok(),
            "{field} should allow null"
        );
    }

    fn schema_contains_date_time(root: &Value, schema: &Value) -> bool {
        if schema.get("format").and_then(Value::as_str) == Some("date-time") {
            return true;
        }
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            return resolve_ref(root, reference)
                .map(|schema| schema_contains_date_time(root, schema))
                .unwrap_or(false);
        }
        ["anyOf", "oneOf", "allOf"].iter().any(|keyword| {
            schema
                .get(*keyword)
                .and_then(Value::as_array)
                .is_some_and(|schemas| {
                    schemas
                        .iter()
                        .any(|schema| schema_contains_date_time(root, schema))
                })
        })
    }

    fn assert_date_time_schema(root: &Value, schema: &Value, label: &str) {
        assert!(
            schema_contains_date_time(root, schema),
            "{label} should include JSON Schema format=date-time, got {schema:?}"
        );
    }

    fn required_nullable_request_paths() -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            ("volicord.update_scope", &["goal_summary"]),
            ("volicord.get_operation_result", &["cursor"]),
            (
                "volicord.prepare_evidence_capture",
                &["capture", "expected_exit_code"],
            ),
            ("volicord.prepare_write", &["task_id"]),
            ("volicord.prepare_write", &["change_unit_id"]),
            ("volicord.stage_artifact", &["expected_sha256"]),
            ("volicord.stage_artifact", &["relation_hint"]),
            ("volicord.record_run", &["run_id"]),
            ("volicord.record_run", &["write_ticket_id"]),
            ("volicord.record_run", &["performed_operation"]),
            ("volicord.record_run", &["observed_changes", "baseline_ref"]),
            ("volicord.record_run", &["close_assessment"]),
            ("volicord.request_user_action", &["change_unit_id"]),
            ("volicord.request_user_action", &["expires_at"]),
            (
                "volicord.request_user_action",
                &["action", "sensitive_action_scope"],
            ),
            ("volicord.close_task", &["close_reason"]),
            ("volicord.close_task", &["superseding_task_id"]),
            ("volicord.close_task", &["user_note"]),
        ]
    }

    fn sample_for_method(method_name: &str) -> Value {
        public_request_json_samples()
            .into_iter()
            .find(|(candidate, _)| *candidate == method_name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("missing sample for {method_name}"))
    }

    fn set_path(value: &mut Value, path: &[&str], replacement: Value) {
        let pointer = format!("/{}", path.join("/"));
        *value
            .pointer_mut(&pointer)
            .unwrap_or_else(|| panic!("missing path {pointer}")) = replacement;
    }

    fn remove_path(value: &mut Value, path: &[&str]) {
        let (field, parent_path) = path
            .split_last()
            .expect("path should contain at least one segment");
        let pointer = if parent_path.is_empty() {
            String::new()
        } else {
            format!("/{}", parent_path.join("/"))
        };
        value
            .pointer_mut(&pointer)
            .unwrap_or_else(|| panic!("missing parent path {pointer}"))
            .as_object_mut()
            .expect("parent should be an object")
            .remove(*field);
    }

    fn reversed_object_value(value: &Value) -> Value {
        match value {
            Value::Array(items) => Value::Array(items.iter().map(reversed_object_value).collect()),
            Value::Object(map) => Value::Object(
                map.iter()
                    .rev()
                    .map(|(key, value)| (key.clone(), reversed_object_value(value)))
                    .collect(),
            ),
            scalar => scalar.clone(),
        }
    }

    fn typed_request_hash(method_name: &str, value: Value) -> RequestHash {
        match method_name {
            "volicord.intake" => canonical_request_hash(
                &serde_json::from_value::<IntakeRequest>(value).expect("intake request"),
            ),
            "volicord.update_scope" => canonical_request_hash(
                &serde_json::from_value::<UpdateScopeRequest>(value).expect("update request"),
            ),
            "volicord.record_shaping_checkpoint" => canonical_request_hash(
                &serde_json::from_value::<RecordShapingCheckpointRequest>(value)
                    .expect("record shaping checkpoint request"),
            ),
            "volicord.finalize_advice" => canonical_request_hash(
                &serde_json::from_value::<FinalizeAdviceRequest>(value)
                    .expect("finalize advice request"),
            ),
            "volicord.advance_task" => canonical_request_hash(
                &serde_json::from_value::<AdvanceTaskRequest>(value).expect("advance task request"),
            ),
            "volicord.status" => canonical_request_hash(
                &serde_json::from_value::<StatusRequest>(value).expect("status request"),
            ),
            "volicord.get_operation_result" => canonical_request_hash(
                &serde_json::from_value::<GetOperationResultRequest>(value)
                    .expect("operation-result request"),
            ),
            "volicord.check_close" => canonical_request_hash(
                &serde_json::from_value::<CheckCloseRequest>(value).expect("check request"),
            ),
            "volicord.prepare_evidence_capture" => canonical_request_hash(
                &serde_json::from_value::<PrepareEvidenceCaptureRequest>(value)
                    .expect("prepare evidence capture request"),
            ),
            "volicord.prepare_write" => canonical_request_hash(
                &serde_json::from_value::<PrepareWriteRequest>(value).expect("prepare request"),
            ),
            "volicord.stage_artifact" => canonical_request_hash(
                &serde_json::from_value::<StageArtifactRequest>(value).expect("stage request"),
            ),
            "volicord.record_run" => canonical_request_hash(
                &serde_json::from_value::<RecordRunRequest>(value).expect("record run request"),
            ),
            "volicord.request_user_action" => canonical_request_hash(
                &serde_json::from_value::<RequestUserActionRequest>(value)
                    .expect("request user action request"),
            ),
            "volicord.resolve_user_action" => canonical_request_hash(
                &serde_json::from_value::<ResolveUserActionRequest>(value)
                    .expect("resolve user action request"),
            ),
            "volicord.reconcile_changes" => canonical_request_hash(
                &serde_json::from_value::<ReconcileChangesRequest>(value)
                    .expect("reconcile changes request"),
            ),
            "volicord.close_task" => canonical_request_hash(
                &serde_json::from_value::<CloseTaskRequest>(value).expect("close request"),
            ),
            other => panic!("unsupported method: {other}"),
        }
        .expect("typed request hash should compute")
    }

    fn first_required_field(method_name: &str) -> &'static str {
        expected_required_fields(method_name)[0]
    }

    fn expected_required_fields(method_name: &str) -> &'static [&'static str] {
        match method_name {
            "volicord.intake" => &[
                "envelope",
                "plain_language_request",
                "requested_mode",
                "resume_policy",
                "acceptance_policy",
                "lineage",
                "initial_scope",
                "initial_context_refs",
                "initial_source_refs",
            ],
            "volicord.update_scope" => &[
                "envelope",
                "task_id",
                "goal_summary",
                "scope_update",
                "scope_boundary",
                "non_goals",
                "acceptance_criteria",
                "autonomy_boundary",
                "baseline_ref",
                "change_unit",
                "related_scope_decision_refs",
            ],
            "volicord.record_shaping_checkpoint" => &[
                "envelope",
                "task_id",
                "checkpoint_operation",
                "scope_revision",
                "baseline_ref",
                "summary",
                "implementation_boundary",
                "gaps",
                "source_refs",
                "evidence_refs",
            ],
            "volicord.finalize_advice" => &[
                "envelope",
                "task_id",
                "shaping_checkpoint_id",
                "change_unit_id",
                "scope_revision",
                "baseline_ref",
                "user_action_resolution_ids",
                "result_summary",
                "result_refs",
                "evidence_refs",
                "residual_risks",
                "recovery_constraints",
            ],
            "volicord.advance_task" => &[
                "envelope",
                "task_id",
                "shaping_checkpoint_id",
                "change_unit_id",
                "scope_revision",
                "baseline_ref",
                "user_action_resolution_ids",
            ],
            "volicord.status" => &["envelope", "include"],
            "volicord.get_operation_result" => &["envelope", "operation_result_ref", "cursor"],
            "volicord.check_close" => &["envelope", "task_id"],
            "volicord.prepare_evidence_capture" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "baseline_ref",
                "target",
                "capture",
            ],
            "volicord.prepare_write" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "intended_operation",
                "intended_paths",
                "product_file_write_intended",
                "sensitive_categories",
                "baseline_ref",
            ],
            "volicord.stage_artifact" => &[
                "envelope",
                "task_id",
                "display_name",
                "content_type",
                "redaction_state",
                "safe_bytes_or_notice",
                "expected_sha256",
                "expected_size_bytes",
                "relation_hint",
            ],
            "volicord.record_run" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "kind",
                "run_id",
                "baseline_ref",
                "write_ticket_id",
                "performed_operation",
                "summary",
                "observed_changes",
                "artifact_inputs",
                "evidence_updates",
                "evidence_observations",
                "close_assessment",
            ],
            "volicord.request_user_action" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "action",
                "required_for",
                "expires_at",
            ],
            "volicord.resolve_user_action" => &[
                "envelope",
                "user_action_request_id",
                "channel_submission_id",
                "resolution",
            ],
            "volicord.reconcile_changes" => &["envelope", "task_id"],
            "volicord.close_task" => &[
                "envelope",
                "task_id",
                "intent",
                "close_reason",
                "superseding_task_id",
                "user_note",
            ],
            other => panic!("unsupported method: {other}"),
        }
    }

    fn deserialize_public_request(
        method_name: &str,
        value: Value,
    ) -> Result<(), serde_json::Error> {
        match method_name {
            "volicord.intake" => serde_json::from_value::<IntakeRequest>(value).map(drop),
            "volicord.update_scope" => {
                serde_json::from_value::<UpdateScopeRequest>(value).map(drop)
            }
            "volicord.record_shaping_checkpoint" => {
                serde_json::from_value::<RecordShapingCheckpointRequest>(value).map(drop)
            }
            "volicord.finalize_advice" => {
                serde_json::from_value::<FinalizeAdviceRequest>(value).map(drop)
            }
            "volicord.advance_task" => {
                serde_json::from_value::<AdvanceTaskRequest>(value).map(drop)
            }
            "volicord.status" => serde_json::from_value::<StatusRequest>(value).map(drop),
            "volicord.get_operation_result" => {
                serde_json::from_value::<GetOperationResultRequest>(value).map(drop)
            }
            "volicord.check_close" => serde_json::from_value::<CheckCloseRequest>(value).map(drop),
            "volicord.prepare_evidence_capture" => {
                serde_json::from_value::<PrepareEvidenceCaptureRequest>(value).map(drop)
            }
            "volicord.prepare_write" => {
                serde_json::from_value::<PrepareWriteRequest>(value).map(drop)
            }
            "volicord.stage_artifact" => {
                serde_json::from_value::<StageArtifactRequest>(value).map(drop)
            }
            "volicord.record_run" => serde_json::from_value::<RecordRunRequest>(value).map(drop),
            "volicord.request_user_action" => {
                serde_json::from_value::<RequestUserActionRequest>(value).map(drop)
            }
            "volicord.resolve_user_action" => {
                serde_json::from_value::<ResolveUserActionRequest>(value).map(drop)
            }
            "volicord.reconcile_changes" => {
                serde_json::from_value::<ReconcileChangesRequest>(value).map(drop)
            }
            "volicord.close_task" => serde_json::from_value::<CloseTaskRequest>(value).map(drop),
            other => panic!("unsupported method sample: {other}"),
        }
    }

    fn assert_unknown<T>(value: Value, field: &str)
    where
        T: serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let error = serde_json::from_value::<T>(value).expect_err("unknown field should fail");
        assert!(
            error.to_string().contains(field),
            "expected error to mention {field}, got {error}"
        );
    }

    fn intake_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "plain_language_request": "Create a first-run checklist.",
            "requested_mode": "work",
            "resume_policy": "create_new",
            "acceptance_policy": null,
            "lineage": null,
            "initial_scope": {
                "boundary": "First-run checklist.",
                "non_goals": ["Changing account creation."],
                "acceptance_criteria": [{
                    "statement": "Checklist appears for new workspaces.",
                    "evidence_requirement": "required"
                }]
            },
            "initial_context_refs": [],
            "initial_source_refs": []
        })
    }

    fn update_scope_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "goal_summary": "Limit saved search filters.",
            "scope_update": {
                "include": ["Saved-filter owner and label edits."],
                "exclude": ["Search indexing behavior."]
            },
            "scope_boundary": "Saved-filter owner and label edits.",
            "non_goals": ["Search indexing behavior."],
            "acceptance_criteria": [{
                "acceptance_criterion_id": null,
                "statement": "Saved filters reject out-of-scope edits.",
                "evidence_requirement": "required"
            }],
            "autonomy_boundary": "Stay within saved-filter validation.",
            "baseline_ref": "baseline_empty_001",
            "change_unit": {
                "operation": "create_current",
                "scope_summary": "Saved-filter validation.",
                "affected_paths": ["src/search/saved-filter.ts"]
            },
            "related_scope_decision_refs": []
        })
    }

    fn record_shaping_checkpoint_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "checkpoint_operation": {"operation": "create_initial"},
            "scope_revision": 2,
            "baseline_ref": "baseline_empty_001",
            "summary": "The saved-filter implementation boundary is ready.",
            "implementation_boundary": "Limit edits to saved-filter validation.",
            "gaps": [],
            "source_refs": [],
            "evidence_refs": []
        })
    }

    fn finalize_advice_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "shaping_checkpoint_id": "shaping_checkpoint_empty_001",
            "change_unit_id": "cu_empty_001",
            "scope_revision": 2,
            "baseline_ref": "baseline_empty_001",
            "user_action_resolution_ids": [],
            "result_summary": "The saved-filter advice is final.",
            "result_refs": [],
            "evidence_refs": [],
            "residual_risks": [],
            "recovery_constraints": []
        })
    }

    fn advance_task_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "shaping_checkpoint_id": "shaping_checkpoint_empty_001",
            "change_unit_id": "cu_empty_001",
            "scope_revision": 2,
            "baseline_ref": "baseline_empty_001",
            "user_action_resolution_ids": []
        })
    }

    fn status_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "include": {
                "task": true,
                "pending_user_actions": true,
                "write_ticket": false,
                "evidence": false,
                "close": true,
                "guarantees": true,
                "continuity": false
            }
        })
    }

    fn operation_result_ref_json() -> Value {
        json!({
            "project_id": "proj_empty_001",
            "source_method": "volicord.record_run",
            "source_idempotency_key": "idem_run_history_001",
            "committed_state_version": 61,
            "response_sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "response_size_bytes": 32768
        })
    }

    fn get_operation_result_request_json() -> Value {
        let mut envelope = envelope_json();
        envelope["task_id"] = Value::Null;
        envelope["idempotency_key"] = Value::Null;
        envelope["expected_state_version"] = Value::Null;
        json!({
            "envelope": envelope,
            "operation_result_ref": operation_result_ref_json(),
            "cursor": null
        })
    }

    fn prepare_evidence_capture_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "change_unit_id": "cu_empty_001",
            "baseline_ref": "baseline_empty_001",
            "target": {
                "target_kind": "supplemental_claim",
                "evidence_claim_id": "claim_capture_001",
                "statement": "The focused validation command succeeds."
            },
            "capture": {
                "capture_kind": "verified_command_execution",
                "command_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "command_label": "focused validation",
                "expected_exit_code": null
            }
        })
    }

    fn prepare_write_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "change_unit_id": "cu_empty_001",
            "intended_operation": "update profile preference save flow",
            "intended_paths": ["src/preferences/profile-save.ts"],
            "product_file_write_intended": true,
            "sensitive_categories": [],
            "baseline_ref": "baseline_empty_001"
        })
    }

    fn stage_artifact_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "display_name": "diagnostic_trace.log",
            "content_type": "text/plain",
            "redaction_state": "none",
            "safe_bytes_or_notice": "Local trace sample.",
            "expected_sha256": null,
            "expected_size_bytes": null,
            "relation_hint": "diagnostic_log"
        })
    }

    fn record_run_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "change_unit_id": "cu_empty_001",
            "kind": "implementation",
            "run_id": null,
            "baseline_ref": "baseline_empty_001",
            "write_ticket_id": null,
            "performed_operation": null,
            "summary": "Search-result count validation passed.",
            "observed_changes": {
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": "baseline_empty_001"
            },
            "artifact_inputs": [],
            "evidence_updates": [],
            "evidence_observations": [],
            "close_assessment": null
        })
    }

    fn evidence_observation_input_json() -> Value {
        json!({
            "target": {
                "target_kind": "supplemental_claim",
                "evidence_claim_id": "claim_search_count_001",
                "statement": "Search result count was verified."
            },
            "source_kind": "external_tool",
            "assurance_level": "external_tool_result",
            "observed_by_actor_source": "agent_connection:conn_empty",
            "tool_name": "local-test-runner",
            "tool_invocation_id": "tool_invocation_001",
            "tool_metadata": {
                "exit_code": 0
            },
            "input_refs": [state_ref_json("run", "run_input_001", "task_empty_001")],
            "source_refs": [],
            "output_artifact_refs": [artifact_ref_json(
                "verified",
                json!("text/plain"),
                json!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
                json!(18)
            )],
            "limitations": ["External tool output is not product correctness proof."],
            "observed_at": "2026-06-18T00:00:00Z"
        })
    }

    fn evidence_observation_json() -> Value {
        let mut observation = evidence_observation_input_json();
        let output_artifact_refs = observation["output_artifact_refs"].clone();
        let object = observation
            .as_object_mut()
            .expect("observation input fixture should be an object");
        object.insert(
            "observation_id".to_owned(),
            json!("evidence_observation_001"),
        );
        object.insert("project_id".to_owned(), json!("proj_empty_001"));
        object.insert("task_id".to_owned(), json!("task_empty_001"));
        object.insert("change_unit_id".to_owned(), json!("cu_empty_001"));
        object.insert(
            "run_ref".to_owned(),
            state_ref_json("run", "run_observation_001", "task_empty_001"),
        );
        object.insert(
            "producer_anchor".to_owned(),
            json!({
                "producer_kind": "unverified_caller",
                "producer_ref": null,
                "output_artifact_refs": output_artifact_refs,
                "verification_basis": null
            }),
        );
        object.insert(
            "relevance_assessment".to_owned(),
            json!({
                "status": "unassessed",
                "assessment_ref": null,
                "assessed_by_actor_source": null
            }),
        );
        object.insert("recorded_at".to_owned(), json!("2026-06-18T00:00:01Z"));
        observation
    }

    fn staged_artifact_input_json(expires_at: &str) -> Value {
        json!({
            "artifact_input_id": "artifact_input_trace_001",
            "source_kind": "staged_artifact",
            "staged_artifact_handle": {
                "handle_id": "staged_trace_001",
                "project_id": "proj_empty_001",
                "task_id": "task_empty_001",
                "created_by_actor_source": "agent_connection:conn_empty",
                "content_type": "text/plain",
                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "size_bytes": 18,
                "redaction_state": "none",
                "expires_at": expires_at,
                "consumed": false
            },
            "existing_artifact_ref": null,
            "relation_hint": "diagnostic_log",
            "evidence_target": null,
            "expected_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "expected_size_bytes": 18,
            "redaction_state": "none"
        })
    }

    fn existing_artifact_input_json(artifact_ref: Value) -> Value {
        json!({
            "artifact_input_id": "artifact_input_existing_001",
            "source_kind": "existing_artifact",
            "staged_artifact_handle": null,
            "existing_artifact_ref": artifact_ref,
            "relation_hint": "diagnostic_log",
            "evidence_target": null,
            "expected_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "expected_size_bytes": 18,
            "redaction_state": "none"
        })
    }

    fn artifact_ref_json(
        integrity_status: &str,
        content_type: Value,
        sha256: Value,
        size_bytes: Value,
    ) -> Value {
        json!({
            "artifact_id": "artifact_trace_001",
            "project_id": "proj_empty_001",
            "task_id": "task_empty_001",
            "display_name": "diagnostic_trace.log",
            "content_type": content_type,
            "sha256": sha256,
            "size_bytes": size_bytes,
            "integrity_status": integrity_status,
            "redaction_state": "none",
            "availability": "available",
            "created_by_run_ref": state_ref_json("run", "run_trace_001", "task_empty_001"),
            "created_by_actor_source": "agent_connection:conn_empty",
            "storage_ref": "volicord-artifact://proj_empty_001/artifact_trace_001"
        })
    }

    fn user_action_option_json() -> Value {
        json!({
            "option_id": "accept",
            "label": "Accept",
            "description": "Accept the focused judgment.",
            "consequence": "The accepted option is recorded.",
            "machine_action": "accept",
            "resolution_outcome": "accepted",
            "is_default": true
        })
    }

    fn request_user_action_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "change_unit_id": "cu_empty_001",
            "action": {
                "action_type": "choice",
                "judgment_kind": "product_decision",
                "presentation": "short",
                "question": "Should the dashboard banner use concise copy?",
                "options": [
                    {
                        "option_id": "concise",
                        "label": "Use concise copy",
                        "description": "Record the focused product decision.",
                        "consequence": "The pending decision can be resolved.",
                        "is_default": true
                    }
                ],
                "context": {
                    "summary": "The banner has two candidate copy lengths.",
                    "related_refs": [],
                    "artifact_refs": [],
                    "visible_risks": [],
                    "constraints": ["Only banner copy length is in scope."]
                },
                "affected_refs": [],
                "sensitive_action_scope": null
            },
            "required_for": ["close_complete"],
            "expires_at": null
        })
    }

    fn sensitive_action_scope_json(expires_at: Value) -> Value {
        json!({
            "action_kind": "write_files",
            "description": "Apply the approved product-file edit.",
            "intended_paths": ["src/preferences/profile-save.ts"],
            "sensitive_categories": ["product_file_write"],
            "command_or_tool_summary": null,
            "network_or_host_summary": null,
            "secret_or_credential_summary": null,
            "capability_claim": "Local file update only.",
            "expires_at": expires_at
        })
    }

    fn resolve_user_action_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "user_action_request_id": "uar_empty_001",
            "channel_submission_id": "cli_submission_001",
            "resolution": {
                "resolution_type": "choice",
                "selected_option_id": "keep",
                "note": null
            }
        })
    }

    fn reconcile_changes_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001"
        })
    }

    fn close_task_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001",
            "intent": "complete",
            "close_reason": "completed_self_checked",
            "superseding_task_id": null,
            "user_note": null
        })
    }

    fn check_close_request_json() -> Value {
        json!({
            "envelope": envelope_json(),
            "task_id": "task_empty_001"
        })
    }
}
