//! Canonical Context full-state invariant boundary.
//!
//! Transition preconditions remain with the command that interprets an intent.
//! This module owns validation of the complete resulting Project state. Portable
//! callers submit the complete portable Project payload; direct commands submit
//! the transaction's complete Project view before commit. Local operation rows
//! and clone bindings remain outside portable Canonical Context.

use crate::portable::{
    export_table, optional_text, required_table, validate_portable_canonical_invariants,
    value_bytes, value_integer, value_key, value_text, Lineage, Payload, PortableValue, TABLES,
};
use crate::store::{decode_source_ids, decode_strings, meaning_preserving_correction};
use crate::{
    ApplicabilityScope, Availability, CheckpointDraft, CheckpointId, CheckpointKind,
    CommandOutcome, CommandTermination, ContextItemDraft, ContextItemId, ContextItemRole,
    DecisionId, Error, ErrorKind, Principal, PrincipalKind, ProjectId, QuestionId,
    QuestionReference, Source, SourceDraft, SourceId, SourcePayload, SourceRelationKind,
    StatementProvenanceRole, TimestampMicros, UserAcceptanceFact, UserAcceptanceState,
    UserReviewFact, UserReviewState, VerificationFact, VerificationState, WorkState,
};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Validate one complete portable Project state through the canonical boundary.
pub(crate) fn validate_payload(payload: &Payload, project_id: ProjectId) -> Result<(), Error> {
    validate_portable_canonical_invariants(payload, project_id)?;
    validate_complete_semantics(payload, project_id)
}

/// Validate the complete canonical view produced by a direct transaction.
///
/// `Connection` is also the deref target of `rusqlite::Transaction`, so callers
/// can run this after all canonical rows are written and before commit.
pub(crate) fn validate_project_state(
    connection: &Connection,
    project_id: ProjectId,
) -> Result<(), Error> {
    let mut tables = Vec::with_capacity(TABLES.len());
    for spec in TABLES {
        tables.push(export_table(connection, spec, project_id)?);
    }
    validate_payload(
        &Payload {
            project_id: project_id.to_string(),
            lineage: Lineage {
                common_base_basis: String::new(),
                history_basis: String::new(),
            },
            tables,
        },
        project_id,
    )
}

pub(crate) fn validate_source_draft(draft: &SourceDraft) -> Result<(), Error> {
    validate_nonempty("Source actor identity", &draft.actor.identity)?;
    if let Some(observer) = &draft.observer {
        validate_nonempty("Source observer identity", &observer.identity)?;
    }
    match &draft.payload {
        SourcePayload::RepositorySnapshot { revision } => {
            validate_nonempty("Source snapshot basis", revision)?;
        }
        SourcePayload::RepositoryCommit { commit } => {
            validate_nonempty("Source snapshot basis", commit)?;
        }
        SourcePayload::File { locator, snapshot } | SourcePayload::Symbol { locator, snapshot } => {
            validate_portable_locator(locator)?;
            validate_nonempty("Source snapshot basis", snapshot)?;
        }
        SourcePayload::CommandExecution { command_label, .. } => {
            validate_nonempty("Source locator", command_label)?;
        }
        SourcePayload::CurrentHostUserTurn {
            host,
            session,
            turn,
        } => {
            validate_nonempty("Source host", host)?;
            validate_nonempty("Source session", session)?;
            validate_nonempty("Source locator", turn)?;
        }
        SourcePayload::Url { url } => validate_nonempty("Source locator", url)?,
        SourcePayload::AdoptedArtifact { locator, revision } => {
            validate_portable_locator(locator)?;
            validate_nonempty("Source snapshot basis", revision)?;
        }
    }
    Ok(())
}

pub(crate) fn decode_source_payload(
    kind: &str,
    locator: Option<String>,
    snapshot_basis: Option<String>,
    detail_one: Option<String>,
    detail_two: Option<String>,
    exit_code: Option<i32>,
    termination: Option<String>,
) -> Result<SourcePayload, Error> {
    let missing = || {
        Error::new(
            ErrorKind::CorruptState,
            format!("stored {kind} Source payload is incomplete or inconsistent"),
        )
    };
    let no_locator = locator.is_none();
    let no_snapshot = snapshot_basis.is_none();
    let no_details = detail_one.is_none() && detail_two.is_none();
    let no_outcome = exit_code.is_none() && termination.is_none();
    match kind {
        "repository_snapshot" if no_locator && no_details && no_outcome => {
            Ok(SourcePayload::RepositorySnapshot {
                revision: snapshot_basis.ok_or_else(missing)?,
            })
        }
        "repository_commit" if no_locator && no_details && no_outcome => {
            Ok(SourcePayload::RepositoryCommit {
                commit: snapshot_basis.ok_or_else(missing)?,
            })
        }
        "file" if no_details && no_outcome => Ok(SourcePayload::File {
            locator: locator.ok_or_else(missing)?,
            snapshot: snapshot_basis.ok_or_else(missing)?,
        }),
        "symbol" if no_details && no_outcome => Ok(SourcePayload::Symbol {
            locator: locator.ok_or_else(missing)?,
            snapshot: snapshot_basis.ok_or_else(missing)?,
        }),
        "command_execution" if no_snapshot && no_details => Ok(SourcePayload::CommandExecution {
            command_label: locator.ok_or_else(missing)?,
            outcome: CommandOutcome {
                exit_code,
                termination: CommandTermination::parse(&termination.ok_or_else(missing)?)
                    .ok_or_else(missing)?,
            },
        }),
        "current_host_user_turn" if no_snapshot && no_outcome => {
            Ok(SourcePayload::CurrentHostUserTurn {
                host: detail_one.ok_or_else(missing)?,
                session: detail_two.ok_or_else(missing)?,
                turn: locator.ok_or_else(missing)?,
            })
        }
        "url" if no_snapshot && no_details && no_outcome => Ok(SourcePayload::Url {
            url: locator.ok_or_else(missing)?,
        }),
        "adopted_artifact" if no_details && no_outcome => Ok(SourcePayload::AdoptedArtifact {
            locator: locator.ok_or_else(missing)?,
            revision: snapshot_basis.ok_or_else(missing)?,
        }),
        _ if matches!(
            kind,
            "repository_snapshot"
                | "repository_commit"
                | "file"
                | "symbol"
                | "command_execution"
                | "current_host_user_turn"
                | "url"
                | "adopted_artifact"
        ) =>
        {
            Err(missing())
        }
        _ => Err(Error::new(
            ErrorKind::CorruptState,
            format!("stored Source kind {kind:?} is invalid"),
        )),
    }
}

pub(crate) fn validate_context_item_draft(draft: &ContextItemDraft) -> Result<(), Error> {
    validate_nonempty("Context Item statement", &draft.statement)?;
    validate_nonempty("Context Item author identity", &draft.author.identity)?;
    if draft.source_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Context Item requires an explicit Source basis",
        ));
    }
    ensure_unique("Context Item Source basis", &draft.source_basis)?;
    validate_portable_string_list(
        "Context Item applicability path",
        &draft.applicability.paths,
    )?;
    validate_string_list(
        "Context Item applicability component",
        &draft.applicability.components,
    )?;
    validate_string_list(
        "Context Item applicability work context",
        &draft.applicability.work_contexts,
    )
}

pub(crate) fn validate_context_provenance(
    draft: &ContextItemDraft,
    sources: &[Source],
) -> Result<(), Error> {
    validate_context_provenance_with_missing(draft, sources, false)
}

fn validate_context_provenance_with_missing(
    draft: &ContextItemDraft,
    sources: &[Source],
    missing_source_witness: bool,
) -> Result<(), Error> {
    let has_user_turn = sources.iter().any(|source| {
        source.actor.kind == PrincipalKind::User
            && matches!(source.payload, SourcePayload::CurrentHostUserTurn { .. })
    });
    let has_observation = sources.iter().any(|source| {
        matches!(
            source.actor.kind,
            PrincipalKind::Repository | PrincipalKind::Command
        ) || matches!(
            source.payload,
            SourcePayload::RepositorySnapshot { .. }
                | SourcePayload::RepositoryCommit { .. }
                | SourcePayload::File { .. }
                | SourcePayload::Symbol { .. }
                | SourcePayload::CommandExecution { .. }
        )
    });
    let has_generated = sources.iter().any(|source| {
        matches!(
            source.actor.kind,
            PrincipalKind::Agent | PrincipalKind::Provider | PrincipalKind::Generator
        )
    });
    match draft.provenance_role {
        StatementProvenanceRole::UserStatement => {
            if draft.author.kind != PrincipalKind::User
                || (!has_user_turn && !missing_source_witness)
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "user-authored Context Item requires user provenance and a current-host user-turn Source",
                ));
            }
        }
        StatementProvenanceRole::Observed => {
            if (!has_observation && !missing_source_witness)
                || matches!(
                    draft.author.kind,
                    PrincipalKind::Provider | PrincipalKind::Generator
                )
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "observed Context Item requires repository or command observation provenance",
                ));
            }
        }
        StatementProvenanceRole::AgentStatement => {
            if draft.author.kind != PrincipalKind::Agent {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "agent-authored Context Item requires an agent author",
                ));
            }
        }
        StatementProvenanceRole::GeneratedInterpretation => {
            if (!has_generated && !missing_source_witness)
                || !matches!(
                    draft.author.kind,
                    PrincipalKind::Agent | PrincipalKind::Provider | PrincipalKind::Generator
                )
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "generated interpretation requires agent, provider, or generator provenance",
                ));
            }
        }
    }
    if draft.role == ContextItemRole::Fact
        && draft.provenance_role != StatementProvenanceRole::Observed
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "only observed provenance may be recorded with the fact role",
        ));
    }
    if draft.role == ContextItemRole::Preference
        && (draft.provenance_role != StatementProvenanceRole::UserStatement
            || (!has_user_turn && !missing_source_witness))
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "explicit preference requires a current-host user-turn Source",
        ));
    }
    Ok(())
}

pub(crate) fn validate_checkpoint_draft(draft: &CheckpointDraft) -> Result<(), Error> {
    validate_checkpoint_draft_with_missing(draft, &CheckpointForgottenSources::default())
}

#[derive(Default)]
struct CheckpointForgottenSources {
    supporting_basis: BTreeMap<i64, SourceId>,
    changed_basis: BTreeMap<i64, SourceId>,
    verification: BTreeMap<i64, SourceId>,
    user_review: Option<SourceId>,
    user_acceptance: Option<SourceId>,
}

fn validate_checkpoint_draft_with_missing(
    draft: &CheckpointDraft,
    missing: &CheckpointForgottenSources,
) -> Result<(), Error> {
    validate_nonempty("Checkpoint goal", &draft.goal)?;
    validate_nonempty("Checkpoint next step", &draft.next_step)?;
    validate_optional_nonempty("Checkpoint state change", draft.state_change.as_deref())?;
    validate_optional_nonempty("Checkpoint handoff target", draft.handoff_to.as_deref())?;
    if draft.source_basis.is_empty() && missing.supporting_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Checkpoint requires an explicit supporting Source basis",
        ));
    }
    ensure_unique("Checkpoint Source basis", &draft.source_basis)?;
    ensure_unique(
        "Checkpoint changed Source basis",
        &draft.changed_source_basis,
    )?;
    ensure_unique("Checkpoint applied Decisions", &draft.applied_decisions)?;
    validate_portable_string_list("Checkpoint changed path", &draft.changed_paths)?;
    validate_string_list("Checkpoint known limit", &draft.known_limits)?;
    validate_string_list("Checkpoint non-goal", &draft.non_goals)?;
    for (position, verification) in draft.verification.iter().enumerate() {
        let missing_verification = i64::try_from(position)
            .ok()
            .is_some_and(|position| missing.verification.contains_key(&position));
        match verification.state {
            VerificationState::NotRun => {
                if verification.source_id.is_some()
                    || verification.outcome.is_some()
                    || missing_verification
                {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "not-run verification cannot claim a Source or outcome",
                    ));
                }
            }
            VerificationState::Partial | VerificationState::Passed | VerificationState::Failed => {
                if verification.source_id.is_none() && !missing_verification {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "executed verification requires an explicit Source",
                    ));
                }
                validate_nonempty(
                    "verification outcome",
                    verification.outcome.as_deref().unwrap_or_default(),
                )?;
            }
        }
    }
    match draft.user_review.state {
        UserReviewState::NotRequested | UserReviewState::Pending => {
            if draft.user_review.source_id.is_some() || missing.user_review.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "unobserved user review state cannot claim a user Source",
                ));
            }
        }
        UserReviewState::Reviewed => {
            if draft.user_review.source_id.is_none() && missing.user_review.is_none() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "reviewed state requires an explicit current-host user-turn Source",
                ));
            }
        }
    }
    match draft.user_acceptance.state {
        UserAcceptanceState::NotRequested | UserAcceptanceState::Pending => {
            if draft.user_acceptance.source_id.is_some() || missing.user_acceptance.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "unobserved user acceptance state cannot claim a user Source",
                ));
            }
        }
        UserAcceptanceState::Accepted | UserAcceptanceState::Rejected => {
            if draft.user_acceptance.source_id.is_none() && missing.user_acceptance.is_none() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "accepted or rejected state requires an explicit current-host user-turn Source",
                ));
            }
        }
    }
    let completion_basis = draft.state_change.is_some()
        || !draft.changed_source_basis.is_empty()
        || !draft.changed_paths.is_empty()
        || !draft.applied_decisions.is_empty()
        || draft
            .verification
            .iter()
            .any(|fact| fact.state != VerificationState::NotRun)
        || !draft.known_limits.is_empty()
        || !missing.changed_basis.is_empty();
    match draft.kind {
        CheckpointKind::Completion => {
            if draft.work_state != WorkState::Completed || !completion_basis {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "completion Checkpoint requires completed work and an explicit meaningful basis",
                ));
            }
            if draft.handoff_to.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "completion Checkpoint cannot claim a handoff target",
                ));
            }
        }
        CheckpointKind::Pause => {
            if draft.work_state != WorkState::Paused || draft.handoff_to.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "pause Checkpoint requires paused work and no handoff target",
                ));
            }
        }
        CheckpointKind::Handoff => {
            if draft.handoff_to.is_none() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "handoff Checkpoint requires an explicit handoff target",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_executed_verification_source(source: &Source) -> Result<(), Error> {
    if !matches!(source.payload, SourcePayload::CommandExecution { .. }) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "executed verification requires a command-execution Source",
        ));
    }
    Ok(())
}

pub(crate) fn validate_current_host_user_source(source: &Source) -> Result<(), Error> {
    if source.actor.kind != PrincipalKind::User
        || !matches!(source.payload, SourcePayload::CurrentHostUserTurn { .. })
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "observed user state requires a current-host user-turn Source authored by the user",
        ));
    }
    Ok(())
}

fn validate_complete_semantics(payload: &Payload, project_id: ProjectId) -> Result<(), Error> {
    let source_tombstones = required_table(payload, "tombstones")?
        .rows
        .iter()
        .filter(|row| value_text(&row[1]).ok() == Some("source"))
        .map(|row| SourceId::from_slice(&value_bytes(&row[2])?))
        .collect::<Result<BTreeSet<_>, Error>>()?;
    let sources = validate_sources(payload, project_id)?;
    validate_context_items(payload, &sources, &source_tombstones)?;
    validate_checkpoints(payload, &sources, &source_tombstones)
}

fn validate_sources(
    payload: &Payload,
    project_id: ProjectId,
) -> Result<BTreeMap<SourceId, Source>, Error> {
    let mut sources = BTreeMap::new();
    for row in &required_table(payload, "sources")?.rows {
        if value_integer(&row[2])? != 1 {
            return corrupt("Source revision must be exactly one");
        }
        let id = SourceId::from_slice(&value_bytes(&row[0])?)?;
        let payload = decode_source_payload(
            value_text(&row[3])?,
            owned_optional_text(&row[4])?,
            owned_optional_text(&row[5])?,
            owned_optional_text(&row[6])?,
            owned_optional_text(&row[7])?,
            optional_i32(&row[8])?,
            owned_optional_text(&row[9])?,
        )?;
        let observer = match (optional_text(&row[12])?, optional_text(&row[13])?) {
            (None, None) => None,
            (Some(kind), Some(identity)) => Some(Principal {
                kind: parse_principal(kind, "Source observer")?,
                identity: identity.to_owned(),
            }),
            _ => return corrupt("Source observer provenance is incomplete"),
        };
        let source = Source {
            id,
            project_id,
            payload,
            actor: Principal {
                kind: parse_principal(value_text(&row[10])?, "Source actor")?,
                identity: value_text(&row[11])?.to_owned(),
            },
            observer,
            availability: Availability::parse(value_text(&row[14])?).ok_or_else(|| {
                Error::new(ErrorKind::CorruptState, "Source availability is invalid")
            })?,
            recorded_at: TimestampMicros::from_unix_micros(value_integer(&row[15])?),
        };
        validate_source_draft(&SourceDraft {
            expected_project_revision: 0,
            payload: source.payload.clone(),
            actor: source.actor.clone(),
            observer: source.observer.clone(),
            availability: source.availability,
        })
        .map_err(|error| semantic_corruption("Source", error))?;
        sources.insert(id, source);
    }
    for row in &required_table(payload, "source_relations")?.rows {
        if SourceRelationKind::parse(value_text(&row[2])?).is_none() {
            return corrupt("Source relation kind is invalid");
        }
        let _ = value_integer(&row[4])?;
    }
    Ok(sources)
}

fn validate_context_items(
    payload: &Payload,
    sources: &BTreeMap<SourceId, Source>,
    source_tombstones: &BTreeSet<SourceId>,
) -> Result<(), Error> {
    let links = ordered_link_ids(payload, "context_item_sources", 1, 2, 3, None)?;
    let mut revisions = BTreeMap::<String, BTreeMap<i64, &Vec<PortableValue>>>::new();
    for row in &required_table(payload, "context_item_revisions")?.rows {
        let _ = ContextItemId::from_slice(&value_bytes(&row[0])?)?;
        let _ = value_integer(&row[14])?;
        revisions
            .entry(value_key(&row[0]))
            .or_default()
            .insert(value_integer(&row[1])?, row);
    }
    for active in &required_table(payload, "context_items")?.rows {
        let _ = ContextItemId::from_slice(&value_bytes(&active[0])?)?;
        let _ = value_integer(&active[11])?;
        let identity = value_key(&active[0]);
        let current = value_integer(&active[2])?;
        let history = revisions.get(&identity).ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "active Context Item has no revision history",
            )
        })?;
        validate_revision_sequence("Context Item", current, history)?;
        let current_row = history.get(&current).ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Context Item current revision snapshot is missing",
            )
        })?;
        if active[3] != current_row[3]
            || active[4] != current_row[4]
            || active[5] != current_row[5]
            || active[6] != current_row[6]
            || active[7] != current_row[7]
            || active[8] != current_row[9]
            || active[9] != current_row[10]
            || active[10] != current_row[11]
        {
            return corrupt("Context Item current row differs from its current revision snapshot");
        }
        let current_basis = decode_source_ids(&value_bytes(&current_row[8])?)?;
        let active_basis = links.get(&identity).cloned().unwrap_or_default();
        let expected_active = current_basis
            .iter()
            .copied()
            .filter(|id| sources.contains_key(id))
            .collect::<Vec<_>>();
        if active_basis != expected_active {
            return corrupt("Context Item Source links differ from its current revision basis");
        }

        let mut previous: Option<ContextItemDraft> = None;
        for revision_number in 1..=current {
            let row = history.get(&revision_number).ok_or_else(|| {
                Error::new(ErrorKind::CorruptState, "Context Item revision is missing")
            })?;
            let draft = context_revision_draft(row)?;
            let mut available_sources = Vec::new();
            let mut missing_source_witness = false;
            for source_id in &draft.source_basis {
                if let Some(source) = sources.get(source_id) {
                    available_sources.push(source.clone());
                } else if source_tombstones.contains(source_id) {
                    missing_source_witness = true;
                } else {
                    return corrupt(
                        "Context Item revision Source basis has no active Source or tombstone",
                    );
                }
            }
            validate_context_item_draft(&draft)
                .and_then(|()| {
                    validate_context_provenance_with_missing(
                        &draft,
                        &available_sources,
                        missing_source_witness,
                    )
                })
                .map_err(|error| semantic_corruption("Context Item", error))?;
            if revision_number == 1 {
                if !matches!(row[12], PortableValue::Null)
                    || !matches!(row[13], PortableValue::Null)
                {
                    return corrupt("initial Context Item revision carries correction authority");
                }
            } else {
                let kind = value_text(&row[12])?;
                if !matches!(kind, "typography" | "formatting" | "expression") {
                    return corrupt("Context Item correction kind is invalid");
                }
                validate_authority_source(&row[13], sources, source_tombstones)?;
                let prior = previous.as_ref().ok_or_else(|| {
                    Error::new(
                        ErrorKind::CorruptState,
                        "Context Item correction basis is missing",
                    )
                })?;
                if prior.role != draft.role
                    || prior.provenance_role != draft.provenance_role
                    || prior.author != draft.author
                    || prior.source_basis != draft.source_basis
                    || prior.applicability != draft.applicability
                    || !meaning_preserving_correction(&prior.statement, &draft.statement, kind)
                {
                    return corrupt(
                        "Context Item correction changes semantic meaning or provenance",
                    );
                }
            }
            previous = Some(draft);
        }
    }
    Ok(())
}

fn context_revision_draft(row: &[PortableValue]) -> Result<ContextItemDraft, Error> {
    Ok(ContextItemDraft {
        expected_project_revision: 0,
        role: ContextItemRole::parse(value_text(&row[3])?)
            .ok_or_else(|| Error::new(ErrorKind::CorruptState, "Context Item role is invalid"))?,
        statement: value_text(&row[4])?.to_owned(),
        provenance_role: StatementProvenanceRole::parse(value_text(&row[5])?).ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Context Item provenance role is invalid",
            )
        })?,
        author: Principal {
            kind: parse_principal(value_text(&row[6])?, "Context Item author")?,
            identity: value_text(&row[7])?.to_owned(),
        },
        source_basis: decode_source_ids(&value_bytes(&row[8])?)?,
        applicability: ApplicabilityScope {
            paths: decode_strings(&value_bytes(&row[9])?)?,
            components: decode_strings(&value_bytes(&row[10])?)?,
            work_contexts: decode_strings(&value_bytes(&row[11])?)?,
        },
    })
}

fn validate_checkpoints(
    payload: &Payload,
    sources: &BTreeMap<SourceId, Source>,
    source_tombstones: &BTreeSet<SourceId>,
) -> Result<(), Error> {
    for row in &required_table(payload, "checkpoint_source_relations")?.rows {
        if !matches!(value_text(&row[2])?, "supported_by" | "changed_basis") {
            return corrupt("Checkpoint Source relation kind is invalid");
        }
    }
    let supported = positioned_checkpoint_source_ids(payload, "supported_by")?;
    let changed = positioned_checkpoint_source_ids(payload, "changed_basis")?;
    let forgotten = checkpoint_forgotten_sources(payload, source_tombstones)?;
    let decisions = ordered_decision_ids(payload)?;
    let questions = ordered_question_refs(payload)?;
    let verification = ordered_verification(payload)?;
    let active_questions = required_table(payload, "questions")?
        .rows
        .iter()
        .map(|row| Ok((value_key(&row[0]), value_integer(&row[2])?)))
        .collect::<Result<BTreeMap<_, _>, Error>>()?;
    let question_revisions = required_table(payload, "question_revisions")?
        .rows
        .iter()
        .map(|row| Ok((value_key(&row[0]), value_integer(&row[1])?)))
        .collect::<Result<BTreeSet<_>, Error>>()?;
    let no_forgotten_sources = CheckpointForgottenSources::default();

    for row in &required_table(payload, "checkpoints")?.rows {
        let _ = CheckpointId::from_slice(&value_bytes(&row[0])?)?;
        let _ = value_integer(&row[16])?;
        if value_integer(&row[2])? != 1 {
            return corrupt("Checkpoint revision must be exactly one");
        }
        let identity = value_key(&row[0]);
        let missing = forgotten.get(&identity).unwrap_or(&no_forgotten_sources);
        let review_source = optional_source_id(&row[9])?;
        let acceptance_source = optional_source_id(&row[11])?;
        let draft = CheckpointDraft {
            expected_project_revision: 0,
            kind: CheckpointKind::parse(value_text(&row[3])?)
                .ok_or_else(|| Error::new(ErrorKind::CorruptState, "Checkpoint kind is invalid"))?,
            goal: value_text(&row[4])?.to_owned(),
            work_state: WorkState::parse(value_text(&row[5])?).ok_or_else(|| {
                Error::new(ErrorKind::CorruptState, "Checkpoint work state is invalid")
            })?,
            state_change: owned_optional_text(&row[6])?,
            source_basis: checkpoint_source_values(
                supported.get(&identity),
                &missing.supporting_basis,
                "supporting basis",
            )?,
            changed_source_basis: checkpoint_source_values(
                changed.get(&identity),
                &missing.changed_basis,
                "changed basis",
            )?,
            changed_paths: decode_strings(&value_bytes(&row[7])?)?,
            applied_decisions: decisions.get(&identity).cloned().unwrap_or_default(),
            verification: verification.get(&identity).cloned().unwrap_or_default(),
            user_review: UserReviewFact {
                state: UserReviewState::parse(value_text(&row[8])?).ok_or_else(|| {
                    Error::new(
                        ErrorKind::CorruptState,
                        "Checkpoint user review state is invalid",
                    )
                })?,
                source_id: review_source,
            },
            user_acceptance: UserAcceptanceFact {
                state: UserAcceptanceState::parse(value_text(&row[10])?).ok_or_else(|| {
                    Error::new(
                        ErrorKind::CorruptState,
                        "Checkpoint user acceptance state is invalid",
                    )
                })?,
                source_id: acceptance_source,
            },
            known_limits: decode_strings(&value_bytes(&row[12])?)?,
            non_goals: decode_strings(&value_bytes(&row[13])?)?,
            open_questions: questions.get(&identity).cloned().unwrap_or_default(),
            next_step: value_text(&row[14])?.to_owned(),
            handoff_to: owned_optional_text(&row[15])?,
        };
        validate_checkpoint_observation_witnesses(&draft, missing)?;
        validate_checkpoint_draft_with_missing(&draft, missing)
            .map_err(|error| semantic_corruption("Checkpoint", error))?;
        for fact in &draft.verification {
            if let Some(source_id) = fact.source_id {
                let source = sources.get(&source_id).ok_or_else(|| {
                    Error::new(
                        ErrorKind::CorruptState,
                        "Checkpoint verification Source is missing",
                    )
                })?;
                validate_executed_verification_source(source)
                    .map_err(|error| semantic_corruption("Checkpoint", error))?;
            }
        }
        for source_id in [review_source, acceptance_source].into_iter().flatten() {
            let source = sources.get(&source_id).ok_or_else(|| {
                Error::new(ErrorKind::CorruptState, "Checkpoint user Source is missing")
            })?;
            validate_current_host_user_source(source)
                .map_err(|error| semantic_corruption("Checkpoint", error))?;
        }
        for reference in &draft.open_questions {
            let key = value_key_from_bytes(reference.question_id.as_bytes());
            if !active_questions.contains_key(&key)
                || !question_revisions
                    .contains(&(key, i64::try_from(reference.revision).unwrap_or(-1)))
            {
                return corrupt("Checkpoint Question link names no exact active revision");
            }
        }
    }
    Ok(())
}

fn positioned_checkpoint_source_ids(
    payload: &Payload,
    relation_kind: &str,
) -> Result<BTreeMap<String, BTreeMap<i64, SourceId>>, Error> {
    let mut values = BTreeMap::<String, BTreeMap<i64, SourceId>>::new();
    for row in &required_table(payload, "checkpoint_source_relations")?.rows {
        if value_text(&row[2])? != relation_kind {
            continue;
        }
        let position = value_integer(&row[4])?;
        if position < 0 {
            return corrupt("Checkpoint Source relation position is invalid");
        }
        if values
            .entry(value_key(&row[1]))
            .or_default()
            .insert(position, SourceId::from_slice(&value_bytes(&row[3])?)?)
            .is_some()
        {
            return corrupt("Checkpoint Source relation position is ambiguous");
        }
    }
    Ok(values)
}

fn checkpoint_forgotten_sources(
    payload: &Payload,
    source_tombstones: &BTreeSet<SourceId>,
) -> Result<BTreeMap<String, CheckpointForgottenSources>, Error> {
    let mut values = BTreeMap::<String, CheckpointForgottenSources>::new();
    for row in &required_table(payload, "checkpoint_forgotten_source_witnesses")?.rows {
        let checkpoint = value_key(&row[1]);
        let source_id = SourceId::from_slice(&value_bytes(&row[2])?)?;
        if !source_tombstones.contains(&source_id) {
            return corrupt("Checkpoint forgotten Source witness has no matching Source tombstone");
        }
        let semantic_use = value_text(&row[3])?;
        let position = value_integer(&row[4])?;
        if position < 0 {
            return corrupt("Checkpoint forgotten Source witness position is invalid");
        }
        let missing = values.entry(checkpoint).or_default();
        let duplicate = match semantic_use {
            "supporting_basis" => missing
                .supporting_basis
                .insert(position, source_id)
                .is_some(),
            "changed_basis" => missing.changed_basis.insert(position, source_id).is_some(),
            "verification" => missing.verification.insert(position, source_id).is_some(),
            "user_review" if position == 0 => missing.user_review.replace(source_id).is_some(),
            "user_acceptance" if position == 0 => {
                missing.user_acceptance.replace(source_id).is_some()
            }
            "user_review" | "user_acceptance" => {
                return corrupt("Checkpoint singleton forgotten Source witness position is invalid")
            }
            _ => return corrupt("Checkpoint forgotten Source witness semantic use is invalid"),
        };
        if duplicate {
            return corrupt("Checkpoint forgotten Source witness slot is ambiguous");
        }
    }
    Ok(values)
}

fn checkpoint_source_values(
    active: Option<&BTreeMap<i64, SourceId>>,
    forgotten: &BTreeMap<i64, SourceId>,
    label: &str,
) -> Result<Vec<SourceId>, Error> {
    let empty = BTreeMap::new();
    let active = active.unwrap_or(&empty);
    if active
        .keys()
        .any(|position| forgotten.contains_key(position))
    {
        return corrupt(format!(
            "Checkpoint {label} has both an active Source and a forgotten witness in one slot"
        ));
    }
    let mut positions = active
        .keys()
        .chain(forgotten.keys())
        .copied()
        .collect::<Vec<_>>();
    positions.sort_unstable();
    if positions
        .iter()
        .enumerate()
        .any(|(expected, actual)| *actual != i64::try_from(expected).unwrap_or(-1))
    {
        return corrupt(format!("Checkpoint {label} positions are not contiguous"));
    }
    let mut source_ids = BTreeSet::new();
    if active
        .values()
        .chain(forgotten.values())
        .any(|source_id| !source_ids.insert(*source_id))
    {
        return corrupt(format!("Checkpoint {label} contains a duplicate Source"));
    }
    Ok(active.values().copied().collect())
}

fn validate_checkpoint_observation_witnesses(
    draft: &CheckpointDraft,
    missing: &CheckpointForgottenSources,
) -> Result<(), Error> {
    for position in missing.verification.keys() {
        let position = usize::try_from(*position).map_err(|_| {
            Error::new(
                ErrorKind::CorruptState,
                "Checkpoint verification witness position is invalid",
            )
        })?;
        let fact = draft.verification.get(position).ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Checkpoint verification witness names no verification position",
            )
        })?;
        if fact.source_id.is_some() {
            return corrupt(
                "Checkpoint verification has both an active Source and a forgotten witness",
            );
        }
    }
    if missing.user_review.is_some() && draft.user_review.source_id.is_some() {
        return corrupt("Checkpoint user review has both an active Source and a forgotten witness");
    }
    if missing.user_acceptance.is_some() && draft.user_acceptance.source_id.is_some() {
        return corrupt(
            "Checkpoint user acceptance has both an active Source and a forgotten witness",
        );
    }
    Ok(())
}

fn ordered_link_ids(
    payload: &Payload,
    table_name: &str,
    owner_index: usize,
    id_index: usize,
    position_index: usize,
    filter: Option<(usize, &str)>,
) -> Result<BTreeMap<String, Vec<SourceId>>, Error> {
    let mut values = BTreeMap::<String, Vec<(i64, SourceId)>>::new();
    for row in &required_table(payload, table_name)?.rows {
        if let Some((index, expected)) = filter {
            if value_text(&row[index])? != expected {
                continue;
            }
        }
        values
            .entry(value_key(&row[owner_index]))
            .or_default()
            .push((
                value_integer(&row[position_index])?,
                SourceId::from_slice(&value_bytes(&row[id_index])?)?,
            ));
    }
    ordered_values(values, table_name)
}

fn ordered_decision_ids(payload: &Payload) -> Result<BTreeMap<String, Vec<DecisionId>>, Error> {
    let mut values = BTreeMap::<String, Vec<(i64, DecisionId)>>::new();
    for row in &required_table(payload, "checkpoint_decisions")?.rows {
        values.entry(value_key(&row[1])).or_default().push((
            value_integer(&row[3])?,
            DecisionId::from_slice(&value_bytes(&row[2])?)?,
        ));
    }
    ordered_values(values, "checkpoint_decisions")
}

fn ordered_question_refs(
    payload: &Payload,
) -> Result<BTreeMap<String, Vec<QuestionReference>>, Error> {
    let mut values = BTreeMap::<String, Vec<(i64, QuestionReference)>>::new();
    let mut unique = BTreeSet::new();
    for row in &required_table(payload, "checkpoint_questions")?.rows {
        let revision = u64::try_from(value_integer(&row[3])?)
            .map_err(|_| Error::new(ErrorKind::CorruptState, "Question revision is invalid"))?;
        let owner = value_key(&row[1]);
        let question_id = QuestionId::from_slice(&value_bytes(&row[2])?)?;
        if !unique.insert((owner.clone(), question_id, revision)) {
            return corrupt("Checkpoint Question links contain a duplicate exact revision");
        }
        values.entry(owner).or_default().push((
            value_integer(&row[4])?,
            QuestionReference {
                question_id,
                revision,
            },
        ));
    }
    ordered_values(values, "checkpoint_questions")
}

fn ordered_verification(
    payload: &Payload,
) -> Result<BTreeMap<String, Vec<VerificationFact>>, Error> {
    let mut values = BTreeMap::<String, Vec<(i64, VerificationFact)>>::new();
    for row in &required_table(payload, "checkpoint_verifications")?.rows {
        values.entry(value_key(&row[1])).or_default().push((
            value_integer(&row[2])?,
            VerificationFact {
                state: VerificationState::parse(value_text(&row[3])?).ok_or_else(|| {
                    Error::new(
                        ErrorKind::CorruptState,
                        "Checkpoint verification state is invalid",
                    )
                })?,
                source_id: optional_source_id(&row[4])?,
                outcome: owned_optional_text(&row[5])?,
            },
        ));
    }
    ordered_values(values, "checkpoint_verifications")
}

fn ordered_values<T>(
    mut values: BTreeMap<String, Vec<(i64, T)>>,
    label: &str,
) -> Result<BTreeMap<String, Vec<T>>, Error> {
    let mut output = BTreeMap::new();
    for (owner, rows) in &mut values {
        rows.sort_by_key(|(position, _)| *position);
        if rows
            .iter()
            .enumerate()
            .any(|(expected, (actual, _))| *actual != i64::try_from(expected).unwrap_or(-1))
        {
            return corrupt(format!("{label} positions are not contiguous"));
        }
        output.insert(
            owner.clone(),
            rows.drain(..).map(|(_, value)| value).collect(),
        );
    }
    Ok(output)
}

fn validate_revision_sequence(
    label: &str,
    current: i64,
    revisions: &BTreeMap<i64, &Vec<PortableValue>>,
) -> Result<(), Error> {
    if current < 1 || revisions.len() != usize::try_from(current).unwrap_or(usize::MAX) {
        return corrupt(format!("{label} revision history contains a gap"));
    }
    for expected in 1..=current {
        if !revisions.contains_key(&expected) {
            return corrupt(format!("{label} revision history contains a gap"));
        }
    }
    Ok(())
}

fn validate_authority_source(
    value: &PortableValue,
    sources: &BTreeMap<SourceId, Source>,
    tombstones: &BTreeSet<SourceId>,
) -> Result<(), Error> {
    let id = SourceId::from_slice(&value_bytes(value)?)?;
    if let Some(source) = sources.get(&id) {
        return validate_current_host_user_source(source)
            .map_err(|error| semantic_corruption("Context Item correction", error));
    }
    if tombstones.contains(&id) {
        return Ok(());
    }
    corrupt("Context Item correction authority has no active Source or tombstone")
}

fn optional_source_id(value: &PortableValue) -> Result<Option<SourceId>, Error> {
    match value {
        PortableValue::Null => Ok(None),
        PortableValue::Bytes(_) => Ok(Some(SourceId::from_slice(&value_bytes(value)?)?)),
        _ => corrupt("optional Source identity is neither null nor bytes"),
    }
}

fn optional_i32(value: &PortableValue) -> Result<Option<i32>, Error> {
    match value {
        PortableValue::Null => Ok(None),
        PortableValue::Integer(value) => i32::try_from(*value).map(Some).map_err(|_| {
            Error::new(
                ErrorKind::CorruptState,
                "Source exit code is outside i32 range",
            )
        }),
        _ => corrupt("optional Source exit code is neither null nor an integer"),
    }
}

fn owned_optional_text(value: &PortableValue) -> Result<Option<String>, Error> {
    Ok(optional_text(value)?.map(ToOwned::to_owned))
}

fn parse_principal(value: &str, label: &str) -> Result<PrincipalKind, Error> {
    PrincipalKind::parse(value).ok_or_else(|| {
        Error::new(
            ErrorKind::CorruptState,
            format!("{label} principal kind is invalid"),
        )
    })
}

fn validate_portable_locator(locator: &str) -> Result<(), Error> {
    validate_nonempty("Source locator", locator)?;
    if Path::new(locator).is_absolute() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "portable Source locator must not be a local absolute path",
        ));
    }
    Ok(())
}

fn validate_portable_string_list(label: &str, values: &[String]) -> Result<(), Error> {
    validate_string_list(label, values)?;
    if values.iter().any(|value| Path::new(value).is_absolute()) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must not contain a local absolute path"),
        ));
    }
    Ok(())
}

fn validate_string_list(label: &str, values: &[String]) -> Result<(), Error> {
    for value in values {
        validate_nonempty(label, value)?;
    }
    let mut unique = BTreeSet::new();
    if values.iter().any(|value| !unique.insert(value)) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must not contain duplicates"),
        ));
    }
    Ok(())
}

fn validate_optional_nonempty(label: &str, value: Option<&str>) -> Result<(), Error> {
    if let Some(value) = value {
        validate_nonempty(label, value)?;
    }
    Ok(())
}

fn validate_nonempty(label: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must not be empty"),
        ));
    }
    Ok(())
}

fn ensure_unique<T: Ord>(label: &str, values: &[T]) -> Result<(), Error> {
    let mut unique = BTreeSet::new();
    if values.iter().any(|value| !unique.insert(value)) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must not contain duplicates"),
        ));
    }
    Ok(())
}

fn value_key_from_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2 + 2);
    encoded.push_str("b:");
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn semantic_corruption(owner: &str, error: Error) -> Error {
    Error::new(
        ErrorKind::CorruptState,
        format!("{owner} canonical semantics are invalid: {error}"),
    )
}

fn corrupt<T>(message: impl Into<String>) -> Result<T, Error> {
    Err(Error::new(ErrorKind::CorruptState, message))
}
