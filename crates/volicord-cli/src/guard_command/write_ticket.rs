use std::collections::BTreeSet;
use volicord_types::product_path::path_is_within;

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

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::values::PromptCaptureStatus;

    #[test]
    fn active_write_ticket_match_reports_ambiguity_instead_of_selecting_by_order() {
        let summary = summary_with_active_write_tickets(vec![
            active_write_ticket("write_ticket_z"),
            active_write_ticket("write_ticket_a"),
        ]);

        assert_eq!(
            active_write_ticket_match(&summary, &["src/export.rs".to_owned()]),
            ActiveWriteTicketMatchOutcome::Ambiguous(vec![
                "write_ticket_z".to_owned(),
                "write_ticket_a".to_owned(),
            ])
        );
    }

    fn active_write_ticket(write_ticket_id: &str) -> ActiveWriteTicketSummary {
        ActiveWriteTicketSummary {
            write_ticket_id: write_ticket_id.to_owned(),
            change_unit_id: "change_unit_current".to_owned(),
            intended_paths: vec!["src".to_owned()],
            denied_paths: Vec::new(),
            idle_expires_at: None,
            workspace_validity_uncertain: false,
        }
    }

    fn summary_with_active_write_tickets(
        active_write_tickets: Vec<ActiveWriteTicketSummary>,
    ) -> GuardStateSummary {
        GuardStateSummary {
            project_id: "project_test".to_owned(),
            project_name: "Project Test".to_owned(),
            repo_root: "/repo".to_owned(),
            state_version: 1,
            active_task_id: Some("task_current".to_owned()),
            active_task_effective_control_level: None,
            policy_control_reevaluation: None,
            active_change_unit_id: Some("change_unit_current".to_owned()),
            prompt_capture_status: PromptCaptureStatus::Unavailable,
            prompt_capture_operational: false,
            current_write_ticket_ids: Vec::new(),
            stale_write_ticket_ids: Vec::new(),
            uncertain_write_ticket_ids: Vec::new(),
            active_write_tickets,
            policy_stale_write_tickets: Vec::new(),
            pending_user_action_count: 0,
            pending_user_actions: Vec::new(),
            active_blocker_count: 0,
            unresolved_unrecorded_change_count: 0,
            suspected_unrecorded_change_count: 0,
        }
    }
}
