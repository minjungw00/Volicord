#![forbid(unsafe_code)]

//! Shared adapter presentation for the current CLI UserAction path.

use std::{error::Error, fmt};
use volicord_command_model::{
    CommandIntrospectionError, InboxEvidenceTarget, InboxResolutionArguments,
    InboxResolveInvocation,
};
use volicord_types::ids::UserActionRequestId;
use volicord_types::schema::{
    RequiredNullable, StateRecordRef, UserActionCapturePath, UserActionInboxForm,
    UserActionInboxItem, UserActionRequest, UserChannelAvailability, UserChannelPathAvailability,
};
use volicord_types::values::{UserActionRequiredFor, VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL};

const CLI_INBOX_KIND: &str = "cli";
const CLI_INBOX_LABEL: &str = "CLI inbox";

/// Failure to render adapter presentation from typed current facts.
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

/// Builds the current CLI inbox item from adapter-neutral Core facts.
pub fn cli_inbox_item(
    request_ref: StateRecordRef,
    request: UserActionRequest,
) -> Result<UserActionInboxItem, UserActionPresentationError> {
    let form = request
        .body
        .capture_form()
        .map_err(|error| UserActionPresentationError::InvalidSemanticFacts(error.to_string()))?;
    let command = cli_resolution_command_for_form(&request.user_action_request_id, &form)?;
    let required = request
        .required_for
        .iter()
        .any(|target| *target != UserActionRequiredFor::Informational);
    Ok(UserActionInboxItem {
        user_action_request_id: request.user_action_request_id,
        request_ref,
        project_id: request.project_id,
        task_id: request.task_id,
        change_unit_id: request.change_unit_id,
        action_kind: request.action_kind,
        question: request.body.question().to_owned(),
        context_summary: request.body.context_summary().to_owned(),
        form,
        required,
        requirement_status: if required { "required" } else { "optional" }.to_owned(),
        required_for: request.required_for,
        status: request.status,
        answer_path_availability: cli_user_channel_availability(),
        preferred_capture_path: Some(UserActionCapturePath {
            kind: CLI_INBOX_KIND.to_owned(),
            label: CLI_INBOX_LABEL.to_owned(),
            available: true,
            command: Some(command).into(),
            url: RequiredNullable::null(),
            capture_basis: Some(VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL.to_owned()).into(),
            expires_at: RequiredNullable::null(),
            detail: RequiredNullable::null(),
        })
        .into(),
        fallbacks: Vec::new(),
        expires_at: request.expires_at,
    })
}

/// Returns the current credential-free CLI User Channel availability.
pub fn cli_user_channel_availability() -> UserChannelAvailability {
    let path = UserChannelPathAvailability {
        kind: CLI_INBOX_KIND.to_owned(),
        label: CLI_INBOX_LABEL.to_owned(),
        available: true,
        status: "available".to_owned(),
        capture_basis: Some(VERIFICATION_BASIS_CLI_DIRECT_USER_CHANNEL.to_owned()).into(),
        detail: RequiredNullable::null(),
    };
    UserChannelAvailability {
        paths: vec![path.clone()],
        recommended_path_kind: Some(path.kind).into(),
        recommended_path_label: Some(path.label.clone()).into(),
        recommendation: Some(format!(
            "Use {} to resolve pending user actions.",
            path.label
        ))
        .into(),
    }
}

/// Builds the current CLI resolution-path command for an adapter that has only safe facts.
pub fn cli_resolution_path_command(
    request_id: &UserActionRequestId,
) -> Result<String, UserActionPresentationError> {
    display_invocation(InboxResolveInvocation::new(
        request_id.as_str(),
        InboxResolutionArguments::Pending,
    ))
}

/// Builds the one shared CLI instruction used by adapters for a pending action.
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
    form: &UserActionInboxForm,
) -> Result<String, UserActionPresentationError> {
    let resolution = match form {
        UserActionInboxForm::Choice { .. } => InboxResolutionArguments::Choice {
            choice: "<choice>".to_owned(),
            note: None,
        },
        UserActionInboxForm::EvidenceObservation { .. } => {
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
    use super::*;

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
    }

    #[test]
    fn evidence_command_display_uses_both_typed_target_invocations() {
        let request_id = UserActionRequestId::new("user_action_observation");
        let form = UserActionInboxForm::EvidenceObservation {
            target_candidates: Vec::new(),
            artifact_candidates: Vec::new(),
            relevance_options: Vec::new(),
            summary_max_chars: 1,
        };

        assert_eq!(
            cli_resolution_command_for_form(&request_id, &form)
                .expect("both canonical target invocations should share a display shape"),
            "volicord inbox resolve user_action_observation (--criterion ID | --claim ID) --artifact ID --summary TEXT"
        );
    }
}
