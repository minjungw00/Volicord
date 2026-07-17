use serde_json::Error as JsonError;

use crate::{ArtifactRef, EvidenceTarget, UserActionInboxChoice, UserActionInboxForm};

/// Channel-neutral, non-authoritative presentation plan derived from one
/// Core-owned closed user-action form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionPresentationPlan {
    pub form: UserActionPresentationForm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserActionPresentationForm {
    Choice {
        choices: Vec<UserActionPresentationChoice>,
        note_allowed: bool,
        note_max_chars: u64,
    },
    EvidenceObservation {
        targets: Vec<UserActionPresentationTarget>,
        artifacts: Vec<UserActionPresentationArtifact>,
        relevance_options: Vec<String>,
        summary_max_chars: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionPresentationChoice {
    pub choice_id: String,
    pub label: String,
    pub description: String,
    pub consequence: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionPresentationTarget {
    pub selector: String,
    pub display_name: String,
    pub metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserActionPresentationArtifact {
    pub artifact_id: String,
    pub display_name: String,
    pub metadata_json: String,
}

/// Channel-neutral safety classification for opening a new agent-facing user
/// input surface from a closed user-action presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionPresentationSafety {
    AgentFacingInputAllowed,
    UserOnlyInputRequired,
}

impl UserActionPresentationSafety {
    pub const fn allows_agent_facing_input(self) -> bool {
        matches!(self, Self::AgentFacingInputAllowed)
    }
}

impl UserActionPresentationPlan {
    pub fn from_form(form: &UserActionInboxForm) -> Result<Self, JsonError> {
        let form = match form {
            UserActionInboxForm::Choice {
                choices,
                note_allowed,
                note_max_chars,
            } => UserActionPresentationForm::Choice {
                choices: choices.iter().map(choice_plan).collect(),
                note_allowed: *note_allowed,
                note_max_chars: *note_max_chars,
            },
            UserActionInboxForm::EvidenceObservation {
                target_candidates,
                artifact_candidates,
                relevance_options,
                summary_max_chars,
            } => UserActionPresentationForm::EvidenceObservation {
                targets: target_candidates
                    .iter()
                    .map(target_plan)
                    .collect::<Result<Vec<_>, _>>()?,
                artifacts: artifact_candidates
                    .iter()
                    .map(artifact_plan)
                    .collect::<Result<Vec<_>, _>>()?,
                relevance_options: relevance_options
                    .iter()
                    .map(serde_json::to_value)
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect(),
                summary_max_chars: *summary_max_chars,
            },
        };
        Ok(Self { form })
    }

    /// Renders every decision-relevant field without truncating it. Adapters
    /// remain responsible for escaping the surrounding transport envelope and
    /// enforcing that envelope's byte budget.
    pub fn render_plain_text(&self) -> Result<String, JsonError> {
        let mut text = String::new();
        match &self.form {
            UserActionPresentationForm::Choice {
                choices,
                note_allowed,
                note_max_chars,
            } => {
                text.push_str("Form type: choice\nChoices:\n");
                for choice in choices {
                    text.push_str("- choice_id: ");
                    text.push_str(&choice.choice_id);
                    text.push_str("\n  label: ");
                    text.push_str(&serde_json::to_string(&choice.label)?);
                    text.push_str("\n  description: ");
                    text.push_str(&serde_json::to_string(&choice.description)?);
                    text.push_str("\n  consequence: ");
                    text.push_str(&serde_json::to_string(&choice.consequence)?);
                    text.push_str("\n  is_default: ");
                    text.push_str(if choice.is_default { "true" } else { "false" });
                    text.push('\n');
                }
                text.push_str("Note allowed: ");
                text.push_str(if *note_allowed { "true" } else { "false" });
                text.push_str("\nNote max characters: ");
                text.push_str(&note_max_chars.to_string());
            }
            UserActionPresentationForm::EvidenceObservation {
                targets,
                artifacts,
                relevance_options,
                summary_max_chars,
            } => {
                text.push_str("Form type: evidence_observation\nTarget candidates:\n");
                for target in targets {
                    text.push_str("- selector: ");
                    text.push_str(&target.selector);
                    text.push_str("\n  display_name: ");
                    text.push_str(&serde_json::to_string(&target.display_name)?);
                    text.push_str("\n  metadata: ");
                    text.push_str(&target.metadata_json);
                    text.push('\n');
                }
                text.push_str("Artifact candidates:\n");
                for artifact in artifacts {
                    text.push_str("- artifact_id: ");
                    text.push_str(&artifact.artifact_id);
                    text.push_str("\n  display_name: ");
                    text.push_str(&serde_json::to_string(&artifact.display_name)?);
                    text.push_str("\n  metadata: ");
                    text.push_str(&artifact.metadata_json);
                    text.push('\n');
                }
                text.push_str("Relevance options: ");
                text.push_str(&relevance_options.join(", "));
                text.push_str("\nSummary max characters: ");
                text.push_str(&summary_max_chars.to_string());
            }
        }
        Ok(text)
    }

    /// Evaluates the exact question, context, and complete rendered form that
    /// an adapter would otherwise place in a new agent-facing user-input
    /// surface. The user-only CLI inbox may still render the full canonical
    /// form when this classification requires it.
    pub fn agent_facing_input_safety(
        &self,
        question: &str,
        context_summary: &str,
    ) -> Result<UserActionPresentationSafety, JsonError> {
        let form_text = self.render_plain_text()?;
        let normalized = format!("{question}\n{context_summary}\n{form_text}").to_ascii_lowercase();
        let user_only_required = [
            "password",
            "passphrase",
            "private key",
            "api key",
            "secret",
            "credential",
            "token",
        ]
        .into_iter()
        .any(|marker| normalized.contains(marker));
        Ok(if user_only_required {
            UserActionPresentationSafety::UserOnlyInputRequired
        } else {
            UserActionPresentationSafety::AgentFacingInputAllowed
        })
    }

    pub fn prompt_capture_instruction(
        &self,
        chat_id: &str,
        user_action_request_id: &str,
        code: &str,
    ) -> String {
        match &self.form {
            UserActionPresentationForm::Choice { note_allowed, .. } => format!(
                "Volicord: resolve {chat_id} --request {user_action_request_id} --choice <choice_id>{} {code}",
                if *note_allowed {
                    " [--note \"text\"]"
                } else {
                    ""
                }
            ),
            UserActionPresentationForm::EvidenceObservation { .. } => format!(
                "Volicord: resolve {chat_id} --request {user_action_request_id} (--criterion <id> | --claim <id>) --artifact <artifact_id> [--artifact <artifact_id> ...] --summary \"text\" [--contradicted] {code}"
            ),
        }
    }
}

fn choice_plan(choice: &UserActionInboxChoice) -> UserActionPresentationChoice {
    UserActionPresentationChoice {
        choice_id: choice.choice_id.as_str().to_owned(),
        label: choice.label.clone(),
        description: choice.description.clone(),
        consequence: choice.consequence.clone(),
        is_default: choice.is_default,
    }
}

fn target_plan(target: &EvidenceTarget) -> Result<UserActionPresentationTarget, JsonError> {
    let (selector, display_name) = match target {
        EvidenceTarget::AcceptanceCriterion {
            acceptance_criterion_id,
        } => (
            format!("--criterion {acceptance_criterion_id}"),
            format!("Acceptance criterion {acceptance_criterion_id}"),
        ),
        EvidenceTarget::SupplementalClaim {
            evidence_claim_id,
            statement,
        } => (
            format!("--claim {evidence_claim_id}"),
            format!("Supplemental claim {evidence_claim_id}: {statement}"),
        ),
    };
    Ok(UserActionPresentationTarget {
        selector,
        display_name,
        metadata_json: serde_json::to_string(target)?,
    })
}

fn artifact_plan(artifact: &ArtifactRef) -> Result<UserActionPresentationArtifact, JsonError> {
    Ok(UserActionPresentationArtifact {
        artifact_id: artifact.artifact_id.as_str().to_owned(),
        display_name: artifact.display_name.clone(),
        metadata_json: serde_json::to_string(artifact)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AcceptanceCriterionId, ActorSource, ArtifactAvailability, ArtifactId,
        ArtifactIntegrityStatus, EvidenceClaimId, EvidenceRelevanceStatus, ProjectId, RecordId,
        RedactionState, StateRecordKind, StateRecordRef, StorageRef, TaskId, UserActionOptionId,
    };

    #[test]
    fn choice_plan_preserves_every_field_limit_and_order() {
        let form = UserActionInboxForm::Choice {
            choices: vec![
                UserActionInboxChoice {
                    choice_id: UserActionOptionId::new("first"),
                    label: "First label".to_owned(),
                    description: "First description".to_owned(),
                    consequence: "First consequence".to_owned(),
                    is_default: false,
                },
                UserActionInboxChoice {
                    choice_id: UserActionOptionId::new("second"),
                    label: "Second label".to_owned(),
                    description: "Second description".to_owned(),
                    consequence: "Second consequence".to_owned(),
                    is_default: true,
                },
            ],
            note_allowed: true,
            note_max_chars: 321,
        };

        let plan = UserActionPresentationPlan::from_form(&form).expect("choice plan");
        let UserActionPresentationForm::Choice {
            choices,
            note_allowed,
            note_max_chars,
        } = &plan.form
        else {
            panic!("choice form must remain a choice plan");
        };
        assert_eq!(
            choices,
            &vec![
                UserActionPresentationChoice {
                    choice_id: "first".to_owned(),
                    label: "First label".to_owned(),
                    description: "First description".to_owned(),
                    consequence: "First consequence".to_owned(),
                    is_default: false,
                },
                UserActionPresentationChoice {
                    choice_id: "second".to_owned(),
                    label: "Second label".to_owned(),
                    description: "Second description".to_owned(),
                    consequence: "Second consequence".to_owned(),
                    is_default: true,
                },
            ]
        );
        assert!(*note_allowed);
        assert_eq!(*note_max_chars, 321);

        let rendered = plan.render_plain_text().expect("plain text");
        for expected in [
            "choice_id: first",
            "label: \"First label\"",
            "description: \"First description\"",
            "consequence: \"First consequence\"",
            "is_default: false",
            "choice_id: second",
            "is_default: true",
            "Note max characters: 321",
        ] {
            assert!(rendered.contains(expected), "missing {expected}");
        }
        assert!(rendered.find("choice_id: first") < rendered.find("choice_id: second"));
    }

    #[test]
    fn evidence_plan_preserves_exact_candidates_metadata_relevance_limits_and_order() {
        let targets = vec![
            EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id: AcceptanceCriterionId::new("criterion_1"),
            },
            EvidenceTarget::SupplementalClaim {
                evidence_claim_id: EvidenceClaimId::new("claim_2"),
                statement: "Exact supplemental claim statement".to_owned(),
            },
        ];
        let artifacts = vec![
            artifact("artifact_1", "First artifact"),
            artifact("artifact_2", "Second artifact"),
        ];
        let relevance_options = vec![
            EvidenceRelevanceStatus::Contradicted,
            EvidenceRelevanceStatus::Supported,
        ];
        let form = UserActionInboxForm::EvidenceObservation {
            target_candidates: targets.clone(),
            artifact_candidates: artifacts.clone(),
            relevance_options,
            summary_max_chars: 654,
        };

        let plan = UserActionPresentationPlan::from_form(&form).expect("evidence plan");
        let UserActionPresentationForm::EvidenceObservation {
            targets: planned_targets,
            artifacts: planned_artifacts,
            relevance_options: planned_relevance,
            summary_max_chars,
        } = &plan.form
        else {
            panic!("evidence form must remain an evidence plan");
        };
        assert_eq!(
            planned_targets
                .iter()
                .map(|target| serde_json::from_str::<EvidenceTarget>(&target.metadata_json))
                .collect::<Result<Vec<_>, _>>()
                .expect("target metadata"),
            targets
        );
        assert_eq!(
            planned_artifacts
                .iter()
                .map(|artifact| serde_json::from_str::<ArtifactRef>(&artifact.metadata_json))
                .collect::<Result<Vec<_>, _>>()
                .expect("artifact metadata"),
            artifacts
        );
        assert_eq!(
            planned_targets
                .iter()
                .map(|target| target.selector.as_str())
                .collect::<Vec<_>>(),
            vec!["--criterion criterion_1", "--claim claim_2"]
        );
        assert_eq!(
            planned_artifacts
                .iter()
                .map(|artifact| artifact.artifact_id.as_str())
                .collect::<Vec<_>>(),
            vec!["artifact_1", "artifact_2"]
        );
        assert_eq!(planned_relevance, &vec!["contradicted", "supported"]);
        assert_eq!(*summary_max_chars, 654);

        let rendered = plan.render_plain_text().expect("plain text");
        for expected in [
            "Exact supplemental claim statement",
            "artifact_1",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "volicord://artifacts/artifact_1",
            "agent_connection:connection_test",
            "Relevance options: contradicted, supported",
            "Summary max characters: 654",
        ] {
            assert!(rendered.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn agent_facing_safety_scans_question_context_and_complete_rendered_form() {
        let safe_form = UserActionInboxForm::Choice {
            choices: vec![UserActionInboxChoice {
                choice_id: UserActionOptionId::new("safe"),
                label: "Review the plan".to_owned(),
                description: "Choose whether to proceed.".to_owned(),
                consequence: "The stored choice is recorded.".to_owned(),
                is_default: true,
            }],
            note_allowed: true,
            note_max_chars: 50,
        };
        let safe_plan = UserActionPresentationPlan::from_form(&safe_form).expect("safe plan");
        assert_eq!(
            safe_plan
                .agent_facing_input_safety("Choose an option.", "Ordinary context.")
                .expect("safe evaluation"),
            UserActionPresentationSafety::AgentFacingInputAllowed
        );
        assert_eq!(
            safe_plan
                .agent_facing_input_safety("Enter an API key.", "Ordinary context.")
                .expect("question evaluation"),
            UserActionPresentationSafety::UserOnlyInputRequired
        );
        assert_eq!(
            safe_plan
                .agent_facing_input_safety("Choose an option.", "Contains a private key.")
                .expect("context evaluation"),
            UserActionPresentationSafety::UserOnlyInputRequired
        );

        let target_only_form = UserActionInboxForm::EvidenceObservation {
            target_candidates: vec![EvidenceTarget::SupplementalClaim {
                evidence_claim_id: EvidenceClaimId::new("claim_sensitive"),
                statement: "Do not paste SECRET_TARGET_MARKER credentials here.".to_owned(),
            }],
            artifact_candidates: vec![artifact("artifact_safe", "Ordinary evidence")],
            relevance_options: vec![EvidenceRelevanceStatus::Supported],
            summary_max_chars: 100,
        };
        let target_only_plan =
            UserActionPresentationPlan::from_form(&target_only_form).expect("target-only plan");
        assert!(target_only_plan
            .render_plain_text()
            .expect("complete target render")
            .contains("SECRET_TARGET_MARKER"));
        assert_eq!(
            target_only_plan
                .agent_facing_input_safety("Review evidence.", "Ordinary context.")
                .expect("target metadata evaluation"),
            UserActionPresentationSafety::UserOnlyInputRequired
        );

        let artifact_only_form = UserActionInboxForm::EvidenceObservation {
            target_candidates: vec![EvidenceTarget::AcceptanceCriterion {
                acceptance_criterion_id: AcceptanceCriterionId::new("criterion_safe"),
            }],
            artifact_candidates: vec![artifact(
                "artifact_sensitive",
                "API key material SECRET_ARTIFACT_MARKER",
            )],
            relevance_options: vec![EvidenceRelevanceStatus::Supported],
            summary_max_chars: 100,
        };
        let artifact_only_plan =
            UserActionPresentationPlan::from_form(&artifact_only_form).expect("artifact-only plan");
        assert!(artifact_only_plan
            .render_plain_text()
            .expect("complete artifact render")
            .contains("SECRET_ARTIFACT_MARKER"));
        assert_eq!(
            artifact_only_plan
                .agent_facing_input_safety("Review evidence.", "Ordinary context.")
                .expect("artifact metadata evaluation"),
            UserActionPresentationSafety::UserOnlyInputRequired
        );
    }

    fn artifact(artifact_id: &str, display_name: &str) -> ArtifactRef {
        ArtifactRef {
            artifact_id: ArtifactId::new(artifact_id),
            project_id: ProjectId::new("project_test"),
            task_id: TaskId::new("task_test"),
            display_name: display_name.to_owned(),
            content_type: Some("application/json".to_owned()).into(),
            sha256: Some(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            )
            .into(),
            size_bytes: Some(42).into(),
            integrity_status: ArtifactIntegrityStatus::Verified,
            redaction_state: RedactionState::Redacted,
            availability: ArtifactAvailability::Available,
            created_by_run_ref: Some(StateRecordRef {
                record_kind: StateRecordKind::Run,
                record_id: RecordId::new("run_test"),
                project_id: ProjectId::new("project_test"),
                task_id: Some(TaskId::new("task_test")).into(),
                produced_at_state_version: Some(7).into(),
            })
            .into(),
            created_by_actor_source: Some(ActorSource::agent_connection("connection_test")).into(),
            storage_ref: Some(StorageRef::new(format!(
                "volicord://artifacts/{artifact_id}"
            )))
            .into(),
        }
    }
}
