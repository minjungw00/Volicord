#![forbid(unsafe_code)]

//! Typed CLI presentation for adapter-neutral UserAction facts.

use std::{error::Error, fmt};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use volicord_command_model::{
    CommandIntrospectionError, InboxEvidenceTarget, InboxResolutionArguments,
    InboxResolveInvocation,
};
use volicord_types::ids::{ChangeUnitId, ProjectId, TaskId, UserActionRequestId};
use volicord_types::schema::{
    RequiredNullable, StateRecordRef, SummaryCard, UserActionRequest, UserActionResolutionForm,
};
use volicord_types::values::{
    UserActionChannelKind, UserActionKind, UserActionRequiredFor, UserActionStatus,
    UserActionVerificationBasis, UtcTimestamp,
};

const CLI_INBOX_LABEL: &str = "CLI inbox";

/// Whether one inbox action participates in a required workflow condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CliUserActionRequirement {
    Required,
    Optional,
}

impl CliUserActionRequirement {
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Required)
    }
}

/// Current credential-free availability of the supported CLI User Channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CliUserChannelAvailability {
    pub paths: Vec<CliUserChannelPath>,
    pub recommended_path_kind: RequiredNullable<UserActionChannelKind>,
    pub recommended_path_label: RequiredNullable<String>,
    pub recommendation: RequiredNullable<String>,
}

/// One CLI User Channel path whose state and facts cannot contradict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CliUserChannelPath {
    Available {
        kind: UserActionChannelKind,
        label: String,
        capture_basis: UserActionVerificationBasis,
    },
    Unavailable {
        kind: UserActionChannelKind,
        label: String,
        detail: String,
    },
}

impl CliUserChannelPath {
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    pub const fn kind(&self) -> UserActionChannelKind {
        match self {
            Self::Available { kind, .. } | Self::Unavailable { kind, .. } => *kind,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Available { label, .. } | Self::Unavailable { label, .. } => label,
        }
    }
}

/// Request-specific CLI path whose available branch carries a canonical command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CliUserActionCapturePath {
    Available {
        kind: UserActionChannelKind,
        label: String,
        command: String,
        capture_basis: UserActionVerificationBasis,
    },
    Unavailable {
        kind: UserActionChannelKind,
        label: String,
        detail: String,
    },
}

impl CliUserActionCapturePath {
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Available { command, .. } => Some(command),
            Self::Unavailable { .. } => None,
        }
    }
}

/// CLI inbox presentation for one actionable semantic request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CliUserActionInboxItem {
    pub user_action_request_id: UserActionRequestId,
    pub request_ref: StateRecordRef,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub change_unit_id: RequiredNullable<ChangeUnitId>,
    pub action_kind: UserActionKind,
    pub question: String,
    pub context_summary: String,
    pub resolution_form: UserActionResolutionForm,
    pub requirement: CliUserActionRequirement,
    pub required_for: Vec<UserActionRequiredFor>,
    pub status: UserActionStatus,
    pub capture_path: CliUserActionCapturePath,
    pub expires_at: RequiredNullable<UtcTimestamp>,
}

impl CliUserActionInboxItem {
    pub const fn is_required(&self) -> bool {
        self.requirement.is_required()
    }
}

/// Complete typed JSON document produced by `volicord inbox --json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CliUserActionInboxResponse {
    pub summary_card: SummaryCard,
    pub user_channel_availability: RequiredNullable<CliUserChannelAvailability>,
    pub pending_user_action_inbox_items: Vec<CliUserActionInboxItem>,
}

/// Failure to build adapter presentation from typed current facts.
#[derive(Debug)]
pub enum UserActionPresentationError {
    InvalidSemanticFacts(String),
    CommandModel(CommandIntrospectionError),
}

impl fmt::Display for UserActionPresentationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSemanticFacts(message) => formatter.write_str(message),
            Self::CommandModel(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for UserActionPresentationError {}

impl From<CommandIntrospectionError> for UserActionPresentationError {
    fn from(error: CommandIntrospectionError) -> Self {
        Self::CommandModel(error)
    }
}

/// Builds one CLI inbox item from adapter-neutral Core facts.
pub fn cli_inbox_item(
    request_ref: StateRecordRef,
    request: UserActionRequest,
) -> Result<CliUserActionInboxItem, UserActionPresentationError> {
    let resolution_form = request
        .body
        .resolution_form()
        .map_err(|error| UserActionPresentationError::InvalidSemanticFacts(error.to_string()))?;
    let command =
        cli_resolution_command_for_form(&request.user_action_request_id, &resolution_form)?;
    let requirement = if request
        .required_for
        .iter()
        .any(|target| *target != UserActionRequiredFor::Informational)
    {
        CliUserActionRequirement::Required
    } else {
        CliUserActionRequirement::Optional
    };
    Ok(CliUserActionInboxItem {
        user_action_request_id: request.user_action_request_id,
        request_ref,
        project_id: request.project_id,
        task_id: request.task_id,
        change_unit_id: request.change_unit_id,
        action_kind: request.action_kind,
        question: request.body.question().to_owned(),
        context_summary: request.body.context_summary().to_owned(),
        resolution_form,
        requirement,
        required_for: request.required_for,
        status: request.status,
        capture_path: CliUserActionCapturePath::Available {
            kind: UserActionChannelKind::Cli,
            label: CLI_INBOX_LABEL.to_owned(),
            command,
            capture_basis: UserActionVerificationBasis::CliDirectUserChannel,
        },
        expires_at: request.expires_at,
    })
}

/// Returns the current credential-free CLI User Channel availability.
pub fn cli_user_channel_availability() -> CliUserChannelAvailability {
    let path = CliUserChannelPath::Available {
        kind: UserActionChannelKind::Cli,
        label: CLI_INBOX_LABEL.to_owned(),
        capture_basis: UserActionVerificationBasis::CliDirectUserChannel,
    };
    CliUserChannelAvailability {
        paths: vec![path.clone()],
        recommended_path_kind: Some(path.kind()).into(),
        recommended_path_label: Some(path.label().to_owned()).into(),
        recommendation: Some(format!(
            "Use {} to resolve pending user actions.",
            path.label()
        ))
        .into(),
    }
}

/// Builds the current CLI resolution-path command for an adapter with only safe facts.
pub fn cli_resolution_path_command(
    request_id: &UserActionRequestId,
) -> Result<String, UserActionPresentationError> {
    display_invocation(InboxResolveInvocation::new(
        request_id.as_str(),
        InboxResolutionArguments::Pending,
    ))
}

/// Builds the shared CLI recovery instruction used by non-CLI adapters.
pub fn cli_pending_user_action_instruction(
    request_id: &UserActionRequestId,
) -> Result<String, UserActionPresentationError> {
    let command = cli_resolution_path_command(request_id)?;
    Ok(format!(
        "A pending UserAction requires the user. Inspect the request in the CLI inbox, then use `{command}` with the answer arguments shown there."
    ))
}

fn cli_resolution_command_for_form(
    request_id: &UserActionRequestId,
    form: &UserActionResolutionForm,
) -> Result<String, UserActionPresentationError> {
    let resolution = match form {
        UserActionResolutionForm::Choice { .. } => InboxResolutionArguments::Choice {
            choice: "<choice>".to_owned(),
            note: None,
        },
        UserActionResolutionForm::EvidenceObservation { .. } => {
            let criterion = InboxResolveInvocation::new(
                request_id.as_str(),
                InboxResolutionArguments::EvidenceObservation {
                    target: InboxEvidenceTarget::AcceptanceCriterion("ID".to_owned()),
                    artifact_ids: vec!["ID".to_owned()],
                    summary: "TEXT".to_owned(),
                    contradicted: false,
                },
            );
            let claim = InboxResolveInvocation::new(
                request_id.as_str(),
                InboxResolutionArguments::EvidenceObservation {
                    target: InboxEvidenceTarget::EvidenceClaim("ID".to_owned()),
                    artifact_ids: vec!["ID".to_owned()],
                    summary: "TEXT".to_owned(),
                    contradicted: false,
                },
            );
            return display_alternative_invocations(criterion, claim);
        }
    };
    display_invocation(InboxResolveInvocation::new(request_id.as_str(), resolution))
}

fn display_alternative_invocations(
    first: InboxResolveInvocation,
    second: InboxResolveInvocation,
) -> Result<String, UserActionPresentationError> {
    let first = first.canonical_arguments()?;
    let second = second.canonical_arguments()?;
    let prefix_len = first
        .iter()
        .zip(&second)
        .take_while(|(left, right)| left == right)
        .count();
    let max_suffix_len = first
        .len()
        .min(second.len())
        .saturating_sub(prefix_len)
        .saturating_sub(2);
    let suffix_len = first[prefix_len..]
        .iter()
        .rev()
        .zip(second[prefix_len..].iter().rev())
        .take(max_suffix_len)
        .take_while(|(left, right)| left == right)
        .count();
    let first_branch_end = first.len().saturating_sub(suffix_len);
    let second_branch_end = second.len().saturating_sub(suffix_len);
    if prefix_len == 0
        || prefix_len >= first_branch_end
        || prefix_len >= second_branch_end
        || first[first_branch_end..] != second[second_branch_end..]
    {
        return Err(UserActionPresentationError::InvalidSemanticFacts(
            "canonical inbox resolution invocations do not share one display shape".to_owned(),
        ));
    }

    let mut display = display_tokens(&first[..prefix_len]);
    display.push(format!(
        "({} | {})",
        display_tokens(&first[prefix_len..first_branch_end]).join(" "),
        display_tokens(&second[prefix_len..second_branch_end]).join(" ")
    ));
    display.extend(display_tokens(&first[first_branch_end..]));
    Ok(display.join(" "))
}

fn display_invocation(
    invocation: InboxResolveInvocation,
) -> Result<String, UserActionPresentationError> {
    Ok(display_tokens(&invocation.canonical_arguments()?).join(" "))
}

fn display_tokens(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .map(|token| quote_display_token(token))
        .collect()
}

fn quote_display_token(token: &str) -> String {
    if !token.is_empty()
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "_-./:@<>=|".contains(ch))
    {
        token.to_owned()
    } else {
        format!("'{}'", token.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use schemars::schema_for;
    use serde::de::DeserializeOwned;
    use serde_json::json;

    use super::*;

    #[test]
    fn presentation_closed_values_round_trip_through_their_enums() {
        assert_round_trip(CliUserActionRequirement::Required);
        assert_round_trip(CliUserActionRequirement::Optional);
        assert_round_trip(CliUserChannelPath::Available {
            kind: UserActionChannelKind::Cli,
            label: "CLI inbox".to_owned(),
            capture_basis: UserActionVerificationBasis::CliDirectUserChannel,
        });
        assert_round_trip(CliUserChannelPath::Unavailable {
            kind: UserActionChannelKind::Cli,
            label: "CLI inbox".to_owned(),
            detail: "No local CLI channel is available.".to_owned(),
        });
        assert_round_trip(CliUserActionCapturePath::Available {
            kind: UserActionChannelKind::Cli,
            label: "CLI inbox".to_owned(),
            command: "canonical invocation".to_owned(),
            capture_basis: UserActionVerificationBasis::CliDirectUserChannel,
        });
        assert_round_trip(CliUserActionCapturePath::Unavailable {
            kind: UserActionChannelKind::Cli,
            label: "CLI inbox".to_owned(),
            detail: "The request cannot currently be resolved.".to_owned(),
        });
    }

    #[test]
    fn path_state_serializes_one_noncontradictory_fact_set() {
        let available = serde_json::to_value(CliUserActionCapturePath::Available {
            kind: UserActionChannelKind::Cli,
            label: "CLI inbox".to_owned(),
            command: "canonical invocation".to_owned(),
            capture_basis: UserActionVerificationBasis::CliDirectUserChannel,
        })
        .expect("available state serializes");
        assert_eq!(
            available,
            json!({
                "status": "available",
                "kind": "cli",
                "label": "CLI inbox",
                "command": "canonical invocation",
                "capture_basis": "cli_direct_user_channel"
            })
        );

        let unavailable = serde_json::to_value(CliUserActionCapturePath::Unavailable {
            kind: UserActionChannelKind::Cli,
            label: "CLI inbox".to_owned(),
            detail: "Unavailable now.".to_owned(),
        })
        .expect("unavailable state serializes");
        assert_eq!(
            unavailable,
            json!({
                "status": "unavailable",
                "kind": "cli",
                "label": "CLI inbox",
                "detail": "Unavailable now."
            })
        );
    }

    #[test]
    fn cli_resolution_path_preserves_the_command_model_request_coordinate() {
        let request_id = UserActionRequestId::new("user_action_shared_coordinate");
        let expected =
            InboxResolveInvocation::new(request_id.as_str(), InboxResolutionArguments::Pending)
                .canonical_arguments()
                .expect("command-model invocation should materialize")
                .join(" ");

        assert_eq!(
            cli_resolution_path_command(&request_id)
                .expect("shared presentation should render the invocation"),
            expected
        );
        assert!(cli_pending_user_action_instruction(&request_id)
            .expect("MCP recovery instruction should render")
            .contains(&expected));
    }

    #[test]
    fn evidence_command_display_uses_both_typed_target_invocations() {
        let request_id = UserActionRequestId::new("user_action_observation");
        let form = UserActionResolutionForm::EvidenceObservation {
            target_candidates: Vec::new(),
            artifact_candidates: Vec::new(),
            relevance_options: Vec::new(),
            summary_max_chars: 1,
        };

        let rendered = cli_resolution_command_for_form(&request_id, &form)
            .expect("both canonical target invocations should share a display shape");
        let invocations = [
            InboxResolveInvocation::new(
                request_id.as_str(),
                InboxResolutionArguments::EvidenceObservation {
                    target: InboxEvidenceTarget::AcceptanceCriterion("ID".to_owned()),
                    artifact_ids: vec!["ID".to_owned()],
                    summary: "TEXT".to_owned(),
                    contradicted: false,
                },
            ),
            InboxResolveInvocation::new(
                request_id.as_str(),
                InboxResolutionArguments::EvidenceObservation {
                    target: InboxEvidenceTarget::EvidenceClaim("ID".to_owned()),
                    artifact_ids: vec!["ID".to_owned()],
                    summary: "TEXT".to_owned(),
                    contradicted: false,
                },
            ),
        ];
        for invocation in invocations {
            for token in invocation
                .canonical_arguments()
                .expect("typed invocation materializes")
            {
                assert!(
                    rendered.contains(&quote_display_token(&token)),
                    "display omitted canonical token {token:?}: {rendered}"
                );
            }
        }
    }

    #[test]
    fn cli_inbox_json_schema_is_closed_and_enum_backed() {
        let schema = serde_json::to_value(schema_for!(CliUserActionInboxResponse))
            .expect("CLI inbox schema serializes");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["definitions"]["CliUserActionRequirement"]["enum"],
            json!(["required", "optional"])
        );
        assert!(schema["definitions"]["CliUserActionCapturePath"]["oneOf"].is_array());
    }

    fn assert_round_trip<T>(value: T)
    where
        T: Clone + PartialEq + fmt::Debug + Serialize + DeserializeOwned,
    {
        let encoded = serde_json::to_value(&value).expect("closed value serializes");
        let decoded: T = serde_json::from_value(encoded).expect("closed value deserializes");
        assert_eq!(decoded, value);
    }
}
