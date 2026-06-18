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
    use serde_json::{json, Value};

    use super::*;

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
                sha256: "sha256:example-trace".to_owned(),
                size_bytes: 42,
                redaction_state: RedactionState::None,
                expires_at: "<future-expiration-timestamp>".to_owned(),
                consumed: false,
            },
            expires_at: "<future-expiration-timestamp>".to_owned(),
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
            "evidence_updates": []
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
            "required_for": "close",
            "expires_at": null
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
