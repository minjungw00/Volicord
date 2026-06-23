#![forbid(unsafe_code)]

//! Shared Rust type boundary for Harness public API and domain-shaped values.
//!
//! This crate contains serde models, controlled API value sets, opaque string
//! identifier wrappers, and deterministic canonical JSON hashing helpers. It
//! does not implement Core behavior, storage effects, CLI behavior, or MCP
//! adapter behavior.

pub mod canonical;
pub mod ids;
pub mod methods;
pub mod schema;
pub mod values;

pub use canonical::*;
pub use ids::*;
pub use methods::*;
pub use schema::*;
pub use values::*;

/// High-level placement marker for shared type groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeBoundary {
    /// API-facing Rust types live behind this boundary.
    Api,
    /// Core/domain Rust types live behind this boundary.
    Domain,
}

impl TypeBoundary {
    /// Returns a stable implementation-facing label for the boundary marker.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Domain => "domain",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::{json, Value};

    use super::*;

    fn timestamp(value: &str) -> UtcTimestamp {
        UtcTimestamp::parse(value).expect("test timestamp should be RFC 3339")
    }

    #[test]
    fn boundary_labels_are_stable() {
        assert_eq!(TypeBoundary::Api.label(), "api");
        assert_eq!(TypeBoundary::Domain.label(), "domain");
    }

    #[test]
    fn tool_envelope_round_trips_documented_field_names() {
        let envelope: ToolEnvelope = serde_json::from_value(json!({
            "project_id": "proj_onboard_001",
            "task_id": null,
            "actor_kind": "agent",
            "surface_id": "surface_onboard",
            "request_id": "req_intake_onboard_001",
            "idempotency_key": "idem_intake_onboard_001",
            "expected_state_version": 17,
            "dry_run": false,
            "locale": "en-US"
        }))
        .expect("documented envelope example should deserialize");

        assert_eq!(envelope.project_id.as_str(), "proj_onboard_001");
        assert_eq!(envelope.actor_kind, ActorKind::Agent);
        assert_eq!(
            envelope
                .idempotency_key
                .as_ref()
                .map(IdempotencyKey::as_str),
            Some("idem_intake_onboard_001")
        );

        let encoded = serde_json::to_value(&envelope).expect("envelope should serialize");
        assert_eq!(encoded["project_id"], "proj_onboard_001");
        assert_eq!(encoded["actor_kind"], "agent");
        assert_eq!(encoded["task_id"], Value::Null);
    }

    #[test]
    fn authority_looking_request_fields_are_rejected() {
        let mut envelope_value = envelope_json("agent");
        envelope_value["verified"] = json!(true);
        assert_unknown::<ToolEnvelope>(envelope_value, "verified");

        let mut envelope_value = envelope_json("agent");
        envelope_value["surface_instance_id"] = json!("surface_instance_forged");
        assert_unknown::<ToolEnvelope>(envelope_value, "surface_instance_id");

        for field in [
            "verified_surface_context",
            "access_class",
            "capability_profile",
            "verification_basis",
        ] {
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
    fn typed_requests_derive_documented_access_classes() {
        assert_eq!(
            serde_json::from_value::<StatusRequest>(status_request_json())
                .expect("status request")
                .requested_access_class(),
            AccessClass::ReadStatus
        );
        assert_eq!(
            serde_json::from_value::<IntakeRequest>(intake_request_json())
                .expect("intake request")
                .requested_access_class(),
            AccessClass::CoreMutation
        );
        assert_eq!(
            serde_json::from_value::<PrepareWriteRequest>(prepare_write_request_json())
                .expect("prepare request")
                .requested_access_class(),
            AccessClass::WriteAuthorization
        );
        assert_eq!(
            serde_json::from_value::<StageArtifactRequest>(stage_artifact_request_json())
                .expect("stage request")
                .requested_access_class(),
            AccessClass::ArtifactRegistration
        );
        assert_eq!(
            serde_json::from_value::<RecordRunRequest>(record_run_request_json())
                .expect("record run request")
                .requested_access_class(),
            AccessClass::RunRecording
        );

        let check = serde_json::from_value::<CloseTaskRequest>(close_task_request_json())
            .expect("close check request");
        assert_eq!(check.requested_access_class(), AccessClass::ReadStatus);

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
            assert_eq!(request.requested_access_class(), AccessClass::CoreMutation);
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

        let mut judgment = record_user_judgment_request_json();
        judgment["answer"]["product_decision"]["owner_defined"] = json!({
            "nested": ["payload", "data"]
        });
        let request: RecordUserJudgmentRequest = serde_json::from_value(judgment)
            .expect("decision-specific payload objects remain open");
        assert!(request
            .answer
            .product_decision
            .as_ref()
            .expect("product decision branch")
            .contains_key("owner_defined"));
    }

    #[test]
    fn stage_artifact_result_serializes_documented_shape() {
        let result = StageArtifactResult {
            base: ToolResultBase {
                response_kind: ResponseKind::Result,
                effect_kind: EffectKind::StagingCreated,
                dry_run: false,
                state_version: Some(42),
                events: vec![],
            },
            staged_artifact_handle: StagedArtifactHandle {
                handle_id: StagedArtifactHandleId::new("staged_trace_log_001"),
                project_id: ProjectId::new("proj_trace_001"),
                task_id: TaskId::new("task_trace_001"),
                created_by_surface_id: SurfaceId::new("surface_artifact"),
                created_by_surface_instance_id: SurfaceInstanceId::new("surface_instance_trace_01"),
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
        assert_eq!(encoded["staged_artifact_handle"]["redaction_state"], "none");
        assert_eq!(
            encoded["staged_artifact_handle"]["created_by_surface_instance_id"],
            "surface_instance_trace_01"
        );

        let decoded: StageArtifactResult =
            serde_json::from_value(encoded).expect("result should deserialize");
        assert!(!decoded.staged_artifact_handle.consumed);
        assert_eq!(decoded.staged_artifact_handle.size_bytes, 42);
    }

    #[test]
    fn record_user_judgment_request_keeps_payload_branches_as_objects() {
        let request: RecordUserJudgmentRequest = serde_json::from_value(json!({
            "envelope": envelope_json("user"),
            "user_judgment_id": "uj_empty_001",
            "judgment_kind": "product_decision",
            "selected_option_id": "keep",
            "answer": {
                "product_decision": {
                    "judgment": {
                        "decision": "accepted",
                        "rationale": "The illustration is suitable."
                    }
                },
                "technical_decision": null,
                "scope_decision": null,
                "sensitive_action_scope": null,
                "final_acceptance": null,
                "residual_risk_acceptance": null,
                "cancellation": null
            },
            "note": null,
            "accepted_risks": []
        }))
        .expect("judgment request should deserialize");

        assert_eq!(request.judgment_kind, JudgmentKind::ProductDecision);
        assert_eq!(request.selected_option_id.as_str(), "keep");
        assert!(request.answer.product_decision.is_some());
        assert!(request.answer.sensitive_action_scope.is_none());

        let encoded = serde_json::to_value(&request).expect("judgment request should serialize");
        assert_eq!(
            encoded["answer"]["product_decision"]["judgment"]["decision"],
            "accepted"
        );
        assert_eq!(encoded["answer"]["technical_decision"], Value::Null);
    }

    #[test]
    fn close_basis_and_judgment_basis_round_trip_json() {
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
                    "source_write_authorization_ref": state_ref_json(
                        "write_authorization",
                        "wa_close_basis_001",
                        "task_close_basis_001"
                    )
                }
            ],
            "recovery_constraints": ["Rollback requires restoring the previous exporter."],
            "source_run_ref": state_ref_json("run", "run_close_basis_001", "task_close_basis_001"),
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

        let judgment_basis: JudgmentBasis = serde_json::from_value(json!({
            "task_id": "task_close_basis_001",
            "change_unit_id": "cu_close_basis_001",
            "scope_revision": 2,
            "close_basis_revision": 4,
            "baseline_ref": "baseline_close_basis",
            "result_refs": [
                state_ref_json("run", "run_close_basis_001", "task_close_basis_001")
            ],
            "residual_risk_ids": ["risk_close_basis_001"],
            "sensitive_action_scope": null,
            "created_at_state_version": 11,
            "compatibility_status": "current"
        }))
        .expect("JudgmentBasis should deserialize");

        assert_eq!(
            judgment_basis.residual_risk_ids[0].as_str(),
            "risk_close_basis_001"
        );
        let encoded = serde_json::to_value(&judgment_basis).expect("JudgmentBasis serializes");
        assert_eq!(encoded["compatibility_status"], "current");
        let decoded: JudgmentBasis =
            serde_json::from_value(encoded).expect("JudgmentBasis round-trips");
        assert_eq!(decoded, judgment_basis);
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
                ("verified_surface_context", json!({ "verified": true })),
                ("access_class", json!("core_mutation")),
                ("capability_profile", json!({ "write_authorization": true })),
                ("verification_basis", json!("caller_supplied_basis")),
            ] {
                let mut forged = valid.clone();
                forged[field] = value;
                assert_schema_and_serde(method_name, forged, false);
            }

            for (field, value) in [
                ("verified", json!(true)),
                ("surface_instance_id", json!("surface_instance_forged")),
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
    fn owner_extension_field_omission_remains_accepted_where_documented_open() {
        let mut update = update_scope_request_json();
        remove_path(&mut update, &["change_unit", "scope_summary"]);
        remove_path(&mut update, &["change_unit", "affected_paths"]);
        assert_schema_and_serde("harness.update_scope", update, true);
    }

    #[test]
    fn required_nullable_fields_must_be_present_but_accept_null() {
        let mut stage = stage_artifact_request_json();
        stage["expected_sha256"] = Value::Null;
        assert_schema_and_serde("harness.stage_artifact", stage.clone(), true);
        stage
            .as_object_mut()
            .expect("stage request should be an object")
            .remove("expected_sha256");
        assert_schema_and_serde("harness.stage_artifact", stage, false);

        let mut envelope_missing_nullable = status_request_json();
        envelope_missing_nullable["envelope"]
            .as_object_mut()
            .expect("envelope should be an object")
            .remove("idempotency_key");
        assert_schema_and_serde("harness.status", envelope_missing_nullable, false);

        let mut answer_missing_branch = record_user_judgment_request_json();
        answer_missing_branch["answer"]
            .as_object_mut()
            .expect("answer should be an object")
            .remove("technical_decision");
        assert_schema_and_serde("harness.record_user_judgment", answer_missing_branch, false);

        let mut selected_option_missing = record_user_judgment_request_json();
        selected_option_missing
            .as_object_mut()
            .expect("record request should be an object")
            .remove("selected_option_id");
        assert_schema_and_serde(
            "harness.record_user_judgment",
            selected_option_missing,
            false,
        );
    }

    #[test]
    fn public_timestamp_inputs_reject_invalid_strings() {
        for invalid in ["zzzz", "tomorrow", "9999"] {
            let mut request = request_user_judgment_request_json();
            request["expires_at"] = json!(invalid);
            assert!(
                deserialize_public_request("harness.request_user_judgment", request).is_err(),
                "request_user_judgment expires_at should reject {invalid}"
            );
        }

        let mut request = request_user_judgment_request_json();
        request["sensitive_action_scope"] = sensitive_action_scope_json(json!("zzzz"));
        assert!(
            deserialize_public_request("harness.request_user_judgment", request).is_err(),
            "request_user_judgment sensitive_action_scope.expires_at should reject invalid text"
        );

        let mut answer = record_user_judgment_request_json();
        answer["answer"]["product_decision"] = Value::Null;
        answer["answer"]["sensitive_action_scope"] = sensitive_action_scope_json(json!("tomorrow"));
        assert!(
            deserialize_public_request("harness.record_user_judgment", answer).is_err(),
            "record_user_judgment answer.sensitive_action_scope.expires_at should reject invalid text"
        );

        let mut run = record_run_request_json();
        run["artifact_inputs"] = json!([staged_artifact_input_json("9999")]);
        assert!(
            deserialize_public_request("harness.record_run", run).is_err(),
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
        let mut zulu = request_user_judgment_request_json();
        zulu["expires_at"] = json!("2026-06-18T00:00:00Z");
        let mut offset = request_user_judgment_request_json();
        offset["expires_at"] = json!("2026-06-18T09:00:00+09:00");

        assert_eq!(
            typed_request_hash("harness.request_user_judgment", zulu),
            typed_request_hash("harness.request_user_judgment", offset.clone())
        );

        let decoded: RequestUserJudgmentRequest =
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

        let stage = public_request_schema("harness.stage_artifact").expect("stage schema");
        assert_required(
            definition(&stage, "ToolEnvelope"),
            &[
                "project_id",
                "task_id",
                "actor_kind",
                "surface_id",
                "request_id",
                "idempotency_key",
                "expected_state_version",
                "dry_run",
                "locale",
            ],
            "ToolEnvelope",
        );
        assert_required(
            &stage,
            expected_required_fields("harness.stage_artifact"),
            "StageArtifactRequest",
        );
        assert_schema_allows_null_property(&stage, "expected_sha256");

        let record = public_request_schema("harness.record_run").expect("record_run schema");
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
                "claim",
                "expected_sha256",
                "expected_size_bytes",
                "redaction_state",
            ],
            "ArtifactInput",
        );

        let judgment =
            public_request_schema("harness.record_user_judgment").expect("judgment schema");
        assert_required(
            definition(&judgment, "RecordUserJudgmentPayload"),
            &[
                "product_decision",
                "technical_decision",
                "scope_decision",
                "sensitive_action_scope",
                "final_acceptance",
                "residual_risk_acceptance",
                "cancellation",
            ],
            "RecordUserJudgmentPayload",
        );
    }

    #[test]
    fn request_user_judgment_option_input_exposes_no_authority_outcome_mapping() {
        let schema =
            public_request_schema("harness.request_user_judgment").expect("judgment schema");
        let option_input = definition(&schema, "UserJudgmentOptionInput");
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

        let mut request = request_user_judgment_request_json();
        request["judgment_kind"] = json!("cancellation");
        request["options"][0]["resolution_outcome"] = json!("accepted");
        assert_schema_and_serde("harness.request_user_judgment", request, false);

        let mut request = request_user_judgment_request_json();
        request["judgment_kind"] = json!("cancellation");
        request["options"][0]["machine_action"] = json!("reject");
        assert_schema_and_serde("harness.request_user_judgment", request, false);
    }

    #[test]
    fn current_user_judgment_option_requires_action_and_outcome() {
        let schema = serde_json::to_value(schemars::schema_for!(UserJudgmentOption))
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
            "UserJudgmentOption",
        );
        assert_eq!(
            schema["additionalProperties"], false,
            "UserJudgmentOption should be closed"
        );

        let valid = user_judgment_option_json();
        assert!(serde_json::from_value::<UserJudgmentOption>(valid.clone()).is_ok());
        assert!(validate_json_schema(&schema, &valid).is_ok());

        let mut missing_action = user_judgment_option_json();
        remove_path(&mut missing_action, &["machine_action"]);
        assert!(serde_json::from_value::<UserJudgmentOption>(missing_action.clone()).is_err());
        assert!(validate_json_schema(&schema, &missing_action).is_err());

        let mut missing_outcome = user_judgment_option_json();
        remove_path(&mut missing_outcome, &["resolution_outcome"]);
        assert!(serde_json::from_value::<UserJudgmentOption>(missing_outcome.clone()).is_err());
        assert!(validate_json_schema(&schema, &missing_outcome).is_err());

        let mut blocked_outcome = user_judgment_option_json();
        blocked_outcome["resolution_outcome"] = json!("blocked");
        assert!(serde_json::from_value::<UserJudgmentOption>(blocked_outcome.clone()).is_err());
        assert!(validate_json_schema(&schema, &blocked_outcome).is_err());

        let mut unknown = user_judgment_option_json();
        unknown["legacy_note"] = json!("not current public shape");
        assert_unknown::<UserJudgmentOption>(unknown, "legacy_note");
    }

    #[test]
    fn persisted_options_reject_bare_array_and_missing_current_fields() {
        let bare_array = serde_json::from_value::<PersistedUserJudgmentOptions>(json!([
            {
                "option_id": "legacy_accept",
                "label": "Accept",
                "description": "Legacy option with no outcome.",
                "consequence": "Audit only.",
                "machine_action": "accept",
                "is_default": true
            }
        ]));
        assert!(bare_array.is_err());

        let missing_action = serde_json::from_value::<PersistedUserJudgmentOptions>(json!({
            "schema_version": 1,
            "options": [{
                "option_id": "accept",
                "label": "Accept",
                "description": "Accept the current close basis.",
                "consequence": "The judgment can be resolved.",
                "resolution_outcome": "accepted",
                "is_default": true
            }]
        }));
        assert!(missing_action.is_err());

        let missing_outcome = serde_json::from_value::<PersistedUserJudgmentOptions>(json!({
            "schema_version": 1,
            "options": [{
                "option_id": "accept",
                "label": "Accept",
                "description": "Accept the current close basis.",
                "consequence": "The judgment can be resolved.",
                "machine_action": "accept",
                "is_default": true
            }]
        }));
        assert!(missing_outcome.is_err());

        let blocked_outcome = serde_json::from_value::<PersistedUserJudgmentOptions>(json!({
            "schema_version": 1,
            "options": [{
                "option_id": "accept",
                "label": "Accept",
                "description": "Accept the current close basis.",
                "consequence": "The judgment can be resolved.",
                "machine_action": "accept",
                "resolution_outcome": "blocked",
                "is_default": true
            }]
        }));
        assert!(blocked_outcome.is_err());
    }

    #[test]
    fn current_user_judgment_resolution_requires_non_null_outcome() {
        let schema = serde_json::to_value(schemars::schema_for!(UserJudgmentResolution))
            .expect("resolution schema should serialize");
        assert_required(
            &schema,
            &[
                "selected_option_id",
                "machine_action",
                "resolution_outcome",
                "answer",
                "note",
                "accepted_risks",
                "resolved_by_actor_kind",
            ],
            "UserJudgmentResolution",
        );
        assert_eq!(
            schema["additionalProperties"], false,
            "UserJudgmentResolution should be closed"
        );

        let valid = user_judgment_resolution_json(json!("accept"), json!("accepted"));
        assert!(serde_json::from_value::<UserJudgmentResolution>(valid.clone()).is_ok());
        assert!(validate_json_schema(&schema, &valid).is_ok());

        let blocked = user_judgment_resolution_json(json!("accept"), json!("blocked"));
        assert!(serde_json::from_value::<UserJudgmentResolution>(blocked.clone()).is_err());
        assert!(validate_json_schema(&schema, &blocked).is_err());

        let mut missing_action = user_judgment_resolution_json(json!("accept"), json!("accepted"));
        remove_path(&mut missing_action, &["machine_action"]);
        assert!(serde_json::from_value::<UserJudgmentResolution>(missing_action.clone()).is_err());
        assert!(validate_json_schema(&schema, &missing_action).is_err());

        let mut missing_outcome = user_judgment_resolution_json(json!("accept"), json!("accepted"));
        remove_path(&mut missing_outcome, &["resolution_outcome"]);
        assert!(serde_json::from_value::<UserJudgmentResolution>(missing_outcome.clone()).is_err());
        assert!(validate_json_schema(&schema, &missing_outcome).is_err());

        let null_outcome = user_judgment_resolution_json(json!("accept"), Value::Null);
        assert!(serde_json::from_value::<UserJudgmentResolution>(null_outcome.clone()).is_err());
        assert!(validate_json_schema(&schema, &null_outcome).is_err());
    }

    #[test]
    fn artifact_ref_requires_integrity_status_and_rejects_legacy_unknown() {
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
                "created_by_surface_id",
                "created_by_surface_instance_id",
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

        let legacy_unknown =
            artifact_ref_json("legacy_unknown", Value::Null, Value::Null, Value::Null);
        assert!(serde_json::from_value::<ArtifactRef>(legacy_unknown.clone()).is_err());
        assert!(validate_json_schema(&schema, &legacy_unknown).is_err());

        let mut missing_integrity =
            artifact_ref_json("corrupt", Value::Null, Value::Null, Value::Null);
        remove_path(&mut missing_integrity, &["integrity_status"]);
        assert!(serde_json::from_value::<ArtifactRef>(missing_integrity.clone()).is_err());
        assert!(validate_json_schema(&schema, &missing_integrity).is_err());
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
        assert_schema_and_serde("harness.record_run", valid.clone(), true);

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
        assert_schema_and_serde("harness.record_run", missing, false);
    }

    #[test]
    fn timestamp_json_schemas_are_date_time_strings() {
        let judgment =
            public_request_schema("harness.request_user_judgment").expect("judgment schema");
        assert_date_time_schema(
            &judgment,
            &judgment["properties"]["expires_at"],
            "RequestUserJudgmentRequest.expires_at",
        );
        assert_date_time_schema(
            &judgment,
            &definition(&judgment, "SensitiveActionScope")["properties"]["expires_at"],
            "SensitiveActionScope.expires_at",
        );

        let run = public_request_schema("harness.record_run").expect("record_run schema");
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
    fn exact_request_objects_are_closed_but_open_payload_objects_stay_open() {
        let record = public_request_schema("harness.record_run").expect("record_run schema");
        for definition_name in [
            "ToolEnvelope",
            "ObservedChanges",
            "ArtifactInput",
            "StateRecordRef",
            "ArtifactRef",
            "StagedArtifactHandle",
            "EvidenceCoverageItem",
            "CloseAssessmentInput",
            "ResidualRiskInput",
        ] {
            assert_eq!(
                definition(&record, definition_name)["additionalProperties"],
                false,
                "{definition_name} should be closed"
            );
        }

        let update = public_request_schema("harness.update_scope").expect("update schema");
        assert_ne!(
            definition(&update, "ChangeUnitUpdate")["additionalProperties"],
            false,
            "ChangeUnitUpdate intentionally carries open owner-defined fields"
        );

        let judgment =
            public_request_schema("harness.record_user_judgment").expect("judgment schema");
        let payload = definition(&judgment, "RecordUserJudgmentPayload");
        let product_decision = &payload["properties"]["product_decision"];
        assert!(
            validate_against(
                &judgment,
                product_decision,
                &json!({ "owner_defined": true }),
                "$.answer.product_decision",
            )
            .is_ok(),
            "decision-specific payload objects intentionally remain open"
        );
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
                "surface_id": "surface_empty",
                "actor_kind": "agent",
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

        let null_hash = typed_request_hash("harness.record_run", record_run_request_json());
        let mut changed = record_run_request_json();
        changed["write_authorization_id"] = json!("wa_hash_change");
        assert_ne!(null_hash, typed_request_hash("harness.record_run", changed));
    }

    fn envelope_json(actor_kind: &str) -> Value {
        json!({
            "project_id": "proj_empty_001",
            "task_id": "task_empty_001",
            "actor_kind": actor_kind,
            "surface_id": "surface_empty",
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
            "state_version": 11
        })
    }

    fn public_request_json_samples() -> Vec<(&'static str, Value)> {
        vec![
            ("harness.intake", intake_request_json()),
            ("harness.update_scope", update_scope_request_json()),
            ("harness.status", status_request_json()),
            ("harness.prepare_write", prepare_write_request_json()),
            ("harness.stage_artifact", stage_artifact_request_json()),
            ("harness.record_run", record_run_request_json()),
            (
                "harness.request_user_judgment",
                request_user_judgment_request_json(),
            ),
            (
                "harness.record_user_judgment",
                record_user_judgment_request_json(),
            ),
            ("harness.close_task", close_task_request_json()),
        ]
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
            .unwrap_or_else(|| panic!("missing schema definition {name}"))
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
            ("harness.update_scope", &["goal_summary"]),
            ("harness.prepare_write", &["task_id"]),
            ("harness.prepare_write", &["change_unit_id"]),
            ("harness.stage_artifact", &["expected_sha256"]),
            ("harness.stage_artifact", &["relation_hint"]),
            ("harness.record_run", &["run_id"]),
            ("harness.record_run", &["write_authorization_id"]),
            ("harness.record_run", &["observed_changes", "baseline_ref"]),
            ("harness.record_run", &["close_assessment"]),
            ("harness.request_user_judgment", &["change_unit_id"]),
            ("harness.request_user_judgment", &["expires_at"]),
            ("harness.record_user_judgment", &["note"]),
            (
                "harness.record_user_judgment",
                &["answer", "technical_decision"],
            ),
            ("harness.close_task", &["close_reason"]),
            ("harness.close_task", &["superseding_task_id"]),
            ("harness.close_task", &["user_note"]),
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
            "harness.intake" => canonical_request_hash(
                &serde_json::from_value::<IntakeRequest>(value).expect("intake request"),
            ),
            "harness.update_scope" => canonical_request_hash(
                &serde_json::from_value::<UpdateScopeRequest>(value).expect("update request"),
            ),
            "harness.status" => canonical_request_hash(
                &serde_json::from_value::<StatusRequest>(value).expect("status request"),
            ),
            "harness.prepare_write" => canonical_request_hash(
                &serde_json::from_value::<PrepareWriteRequest>(value).expect("prepare request"),
            ),
            "harness.stage_artifact" => canonical_request_hash(
                &serde_json::from_value::<StageArtifactRequest>(value).expect("stage request"),
            ),
            "harness.record_run" => canonical_request_hash(
                &serde_json::from_value::<RecordRunRequest>(value).expect("record run request"),
            ),
            "harness.request_user_judgment" => canonical_request_hash(
                &serde_json::from_value::<RequestUserJudgmentRequest>(value)
                    .expect("request judgment request"),
            ),
            "harness.record_user_judgment" => canonical_request_hash(
                &serde_json::from_value::<RecordUserJudgmentRequest>(value)
                    .expect("record judgment request"),
            ),
            "harness.close_task" => canonical_request_hash(
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
            "harness.intake" => &[
                "envelope",
                "plain_language_request",
                "requested_mode",
                "resume_policy",
                "initial_scope",
                "initial_context_refs",
            ],
            "harness.update_scope" => &[
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
            "harness.status" => &["envelope", "include"],
            "harness.prepare_write" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "intended_operation",
                "intended_paths",
                "product_file_write_intended",
                "sensitive_categories",
                "baseline_ref",
            ],
            "harness.stage_artifact" => &[
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
            "harness.record_run" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "kind",
                "run_id",
                "baseline_ref",
                "write_authorization_id",
                "summary",
                "observed_changes",
                "artifact_inputs",
                "evidence_updates",
                "close_assessment",
            ],
            "harness.request_user_judgment" => &[
                "envelope",
                "task_id",
                "change_unit_id",
                "judgment_kind",
                "presentation",
                "question",
                "context",
                "affected_refs",
                "required_for",
                "expires_at",
            ],
            "harness.record_user_judgment" => &[
                "envelope",
                "user_judgment_id",
                "judgment_kind",
                "selected_option_id",
                "answer",
                "note",
                "accepted_risks",
            ],
            "harness.close_task" => &[
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
            "harness.intake" => serde_json::from_value::<IntakeRequest>(value).map(drop),
            "harness.update_scope" => serde_json::from_value::<UpdateScopeRequest>(value).map(drop),
            "harness.status" => serde_json::from_value::<StatusRequest>(value).map(drop),
            "harness.prepare_write" => {
                serde_json::from_value::<PrepareWriteRequest>(value).map(drop)
            }
            "harness.stage_artifact" => {
                serde_json::from_value::<StageArtifactRequest>(value).map(drop)
            }
            "harness.record_run" => serde_json::from_value::<RecordRunRequest>(value).map(drop),
            "harness.request_user_judgment" => {
                serde_json::from_value::<RequestUserJudgmentRequest>(value).map(drop)
            }
            "harness.record_user_judgment" => {
                serde_json::from_value::<RecordUserJudgmentRequest>(value).map(drop)
            }
            "harness.close_task" => serde_json::from_value::<CloseTaskRequest>(value).map(drop),
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
            "envelope": envelope_json("agent"),
            "plain_language_request": "Create a first-run checklist.",
            "requested_mode": "work",
            "resume_policy": "create_new",
            "initial_scope": {
                "boundary": "First-run checklist.",
                "non_goals": ["Changing account creation."],
                "acceptance_criteria": ["Checklist appears for new workspaces."]
            },
            "initial_context_refs": []
        })
    }

    fn update_scope_request_json() -> Value {
        json!({
            "envelope": envelope_json("agent"),
            "task_id": "task_empty_001",
            "goal_summary": "Limit saved search filters.",
            "scope_update": {
                "include": ["Saved-filter owner and label edits."],
                "exclude": ["Search indexing behavior."]
            },
            "scope_boundary": "Saved-filter owner and label edits.",
            "non_goals": ["Search indexing behavior."],
            "acceptance_criteria": ["Saved filters reject out-of-scope edits."],
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

    fn status_request_json() -> Value {
        json!({
            "envelope": envelope_json("agent"),
            "include": {
                "task": true,
                "pending_user_judgments": true,
                "write_authority": false,
                "evidence": false,
                "close": true,
                "guarantees": true
            }
        })
    }

    fn prepare_write_request_json() -> Value {
        json!({
            "envelope": envelope_json("agent"),
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
            "envelope": envelope_json("agent"),
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
            "envelope": envelope_json("agent"),
            "task_id": "task_empty_001",
            "change_unit_id": "cu_empty_001",
            "kind": "implementation",
            "run_id": null,
            "baseline_ref": "baseline_empty_001",
            "write_authorization_id": null,
            "summary": "Search-result count validation passed.",
            "observed_changes": {
                "changed_paths": [],
                "product_file_write_observed": false,
                "sensitive_categories": [],
                "baseline_ref": "baseline_empty_001"
            },
            "artifact_inputs": [],
            "evidence_updates": [],
            "close_assessment": null
        })
    }

    fn staged_artifact_input_json(expires_at: &str) -> Value {
        json!({
            "artifact_input_id": "artifact_input_trace_001",
            "source_kind": "staged_artifact",
            "staged_artifact_handle": {
                "handle_id": "staged_trace_001",
                "project_id": "proj_empty_001",
                "task_id": "task_empty_001",
                "created_by_surface_id": "surface_empty",
                "created_by_surface_instance_id": "surface_instance_empty",
                "content_type": "text/plain",
                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "size_bytes": 18,
                "redaction_state": "none",
                "expires_at": expires_at,
                "consumed": false
            },
            "existing_artifact_ref": null,
            "relation_hint": "diagnostic_log",
            "claim": null,
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
            "claim": null,
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
            "created_by_surface_id": "surface_empty",
            "created_by_surface_instance_id": "surface_instance_empty",
            "storage_ref": "harness-artifact://proj_empty_001/artifact_trace_001"
        })
    }

    fn user_judgment_option_json() -> Value {
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

    fn user_judgment_resolution_json(machine_action: Value, resolution_outcome: Value) -> Value {
        json!({
            "selected_option_id": "accept",
            "machine_action": machine_action,
            "resolution_outcome": resolution_outcome,
            "answer": {
                "product_decision": {
                    "judgment": {
                        "decision": "accepted",
                        "rationale": "The focused judgment is accepted."
                    }
                },
                "technical_decision": null,
                "scope_decision": null,
                "sensitive_action_scope": null,
                "final_acceptance": null,
                "residual_risk_acceptance": null,
                "cancellation": null
            },
            "note": null,
            "accepted_risks": [],
            "resolved_by_actor_kind": "user"
        })
    }

    fn request_user_judgment_request_json() -> Value {
        json!({
            "envelope": envelope_json("agent"),
            "task_id": "task_empty_001",
            "change_unit_id": "cu_empty_001",
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

    fn record_user_judgment_request_json() -> Value {
        json!({
            "envelope": envelope_json("user"),
            "user_judgment_id": "uj_empty_001",
            "judgment_kind": "product_decision",
            "selected_option_id": "keep",
            "answer": {
                "product_decision": {
                    "judgment": {
                        "decision": "accepted",
                        "rationale": "The illustration is suitable."
                    }
                },
                "technical_decision": null,
                "scope_decision": null,
                "sensitive_action_scope": null,
                "final_acceptance": null,
                "residual_risk_acceptance": null,
                "cancellation": null
            },
            "note": null,
            "accepted_risks": []
        })
    }

    fn close_task_request_json() -> Value {
        json!({
            "envelope": envelope_json("agent"),
            "task_id": "task_empty_001",
            "intent": "check",
            "close_reason": null,
            "superseding_task_id": null,
            "user_note": null
        })
    }
}
