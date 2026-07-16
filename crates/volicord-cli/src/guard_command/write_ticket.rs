use std::collections::BTreeSet;

use super::{
    context::{ActiveWriteTicketSummary, GuardStateSummary},
    mutation::PathAssessment,
    tool_observation::ToolObservation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ActiveWriteTicketMatchOutcome {
    Matched(ActiveWriteTicketSummary),
    NoActiveTickets,
    OutOfScope(Vec<String>),
    Ambiguous(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WriteTicketCoverage {
    NotWriteLike,
    TicketBacked {
        ticket: ActiveWriteTicketSummary,
        observed_paths: Vec<String>,
    },
    NoObservedPaths,
    NoActiveTickets {
        observed_paths: Vec<String>,
    },
    PolicyAuthorityStale {
        observed_paths: Vec<String>,
        stale_ticket_ids: Vec<String>,
    },
    OutOfScope {
        observed_paths: Vec<String>,
        active_ticket_ids: Vec<String>,
    },
    Ambiguous {
        observed_paths: Vec<String>,
        matching_ticket_ids: Vec<String>,
    },
}

pub(super) fn write_ticket_coverage(
    summary: &GuardStateSummary,
    observation: &ToolObservation,
) -> WriteTicketCoverage {
    let observed_paths = if observation.changed_paths.is_empty() {
        normalized_observed_paths(observation.structured_paths.iter())
    } else {
        normalized_observed_paths(observation.changed_paths.iter())
    };
    if observed_paths.is_empty() {
        return WriteTicketCoverage::NoObservedPaths;
    }
    let matching = summary
        .active_write_tickets
        .iter()
        .filter(|ticket| {
            paths_are_authorized(
                &observed_paths,
                &ticket.intended_paths,
                &ticket.denied_paths,
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if matching.len() == 1 {
        return WriteTicketCoverage::TicketBacked {
            ticket: matching.into_iter().next().expect("length checked"),
            observed_paths,
        };
    }
    if matching.len() > 1 {
        return WriteTicketCoverage::Ambiguous {
            observed_paths,
            matching_ticket_ids: matching
                .into_iter()
                .map(|ticket| ticket.write_ticket_id)
                .collect(),
        };
    }
    let policy_stale_ticket_ids = summary
        .policy_stale_write_tickets
        .iter()
        .filter(|ticket| {
            paths_are_authorized(
                &observed_paths,
                &ticket.intended_paths,
                &ticket.denied_paths,
            )
        })
        .map(|ticket| ticket.write_ticket_id.clone())
        .collect::<Vec<_>>();
    if !policy_stale_ticket_ids.is_empty() {
        return WriteTicketCoverage::PolicyAuthorityStale {
            observed_paths,
            stale_ticket_ids: policy_stale_ticket_ids,
        };
    }
    if summary.active_write_tickets.is_empty() {
        return WriteTicketCoverage::NoActiveTickets { observed_paths };
    }
    WriteTicketCoverage::OutOfScope {
        observed_paths,
        active_ticket_ids: summary
            .active_write_tickets
            .iter()
            .map(|ticket| ticket.write_ticket_id.clone())
            .collect(),
    }
}

pub(super) fn active_write_ticket_match(
    summary: &GuardStateSummary,
    changed: &[String],
) -> ActiveWriteTicketMatchOutcome {
    if summary.active_write_tickets.is_empty() {
        return ActiveWriteTicketMatchOutcome::NoActiveTickets;
    }
    let matching = summary
        .active_write_tickets
        .iter()
        .filter(|ticket| {
            paths_are_authorized(changed, &ticket.intended_paths, &ticket.denied_paths)
        })
        .cloned()
        .collect::<Vec<_>>();
    if matching.len() == 1 {
        return ActiveWriteTicketMatchOutcome::Matched(
            matching.into_iter().next().expect("length checked"),
        );
    }
    if matching.len() > 1 {
        return ActiveWriteTicketMatchOutcome::Ambiguous(
            matching
                .into_iter()
                .map(|ticket| ticket.write_ticket_id)
                .collect(),
        );
    }
    ActiveWriteTicketMatchOutcome::OutOfScope(
        summary
            .active_write_tickets
            .iter()
            .map(|ticket| ticket.write_ticket_id.clone())
            .collect(),
    )
}

pub(super) fn normalized_observed_paths<'a>(
    paths: impl Iterator<Item = &'a PathAssessment>,
) -> Vec<String> {
    paths
        .filter(|path| path.inside_repo)
        .filter_map(|path| path.normalized.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn paths_are_authorized(
    observed_paths: &[String],
    authorized_paths: &[String],
    denied_paths: &[String],
) -> bool {
    !observed_paths.is_empty()
        && !authorized_paths.is_empty()
        && observed_paths.iter().all(|path| {
            authorized_paths
                .iter()
                .any(|authorized| path_is_within(path, authorized))
                && !denied_paths
                    .iter()
                    .any(|denied| path_is_within(path, denied))
        })
}

pub(super) fn path_is_within(path: &str, scope: &str) -> bool {
    path == scope
        || path
            .strip_prefix(scope)
            .is_some_and(|rest| rest.starts_with('/'))
}
