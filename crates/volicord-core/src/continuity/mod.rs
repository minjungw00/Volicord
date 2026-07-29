use volicord_types::ids::{
    ChangeUnitId, DurableIdGenerator, ProjectContinuityRecordId, ProjectId, TaskId,
};
use volicord_types::schema::{
    ArtifactRef, PersistedProjectContinuityMetadata, ProjectContinuityRecord,
    ProjectContinuitySummary, StateRecordRef,
};
use volicord_types::values::{
    ProjectContinuityKind, ProjectContinuityStatus, StateRecordKind, UtcTimestamp,
};

use volicord_store::core_pipeline::{
    ContinuityMutation, CoreProjectStore, CoreStorageMutation, ProjectContinuityRecordInsert,
    ProjectContinuityRecordRecord,
};

use crate::pipeline::{CorePipelineError, CoreResult};
use crate::policy::evidence::unique_artifact_refs;

mod user_action;

pub(crate) use user_action::plan_user_action_continuity_records;

#[derive(Debug)]
pub(crate) enum ContinuityPlanningError {
    Core(CorePipelineError),
    UserAction(volicord_user_action_service::UserActionServiceError),
}

impl From<CorePipelineError> for ContinuityPlanningError {
    fn from(error: CorePipelineError) -> Self {
        Self::Core(error)
    }
}

impl From<volicord_user_action_service::UserActionServiceError> for ContinuityPlanningError {
    fn from(error: volicord_user_action_service::UserActionServiceError) -> Self {
        Self::UserAction(error)
    }
}

use crate::identity::allocate_project_continuity_record_id;
use crate::record_refs::{sorted_unique, state_ref, unique_state_refs};

pub(crate) struct ProjectContinuityDraft {
    pub(crate) kind: ProjectContinuityKind,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) rationale: Option<String>,
    pub(crate) applies_to_paths: Vec<String>,
    pub(crate) applies_to_refs: Vec<StateRecordRef>,
    pub(crate) source_refs: Vec<StateRecordRef>,
    pub(crate) artifact_refs: Vec<ArtifactRef>,
    pub(crate) supersedes_refs: Vec<StateRecordRef>,
    pub(crate) review_triggers: Vec<String>,
    pub(crate) metadata: PersistedProjectContinuityMetadata,
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectContinuityPlanContext<'a> {
    pub(crate) id_generator: &'a dyn DurableIdGenerator,
    pub(crate) store: &'a CoreProjectStore<'a>,
    pub(crate) project_id: &'a ProjectId,
    pub(crate) source_task_id: &'a TaskId,
    pub(crate) source_change_unit_id: Option<&'a ChangeUnitId>,
    pub(crate) planned_state_version: u64,
    pub(crate) now: &'a UtcTimestamp,
}

pub(crate) struct PlannedProjectContinuityRecord {
    pub(crate) record_ref: StateRecordRef,
    pub(crate) summary: ProjectContinuitySummary,
    pub(crate) mutation: CoreStorageMutation,
}

pub(crate) fn plan_project_continuity_record(
    context: ProjectContinuityPlanContext<'_>,
    draft: ProjectContinuityDraft,
) -> CoreResult<PlannedProjectContinuityRecord> {
    let continuity_record_id =
        allocate_project_continuity_record_id(context.id_generator, context.store)?;
    let record_ref = state_ref(
        StateRecordKind::ProjectContinuityRecord,
        continuity_record_id.as_str(),
        context.project_id,
        Some(context.source_task_id),
        Some(context.planned_state_version),
    );
    let applies_to_paths = sorted_unique(draft.applies_to_paths);
    let applies_to_refs = unique_state_refs(draft.applies_to_refs);
    let source_refs = unique_state_refs(draft.source_refs);
    let artifact_refs = unique_artifact_refs(draft.artifact_refs);
    let supersedes_refs = unique_state_refs(draft.supersedes_refs);
    let review_triggers = sorted_unique(draft.review_triggers);
    let source_task_ref = state_ref(
        StateRecordKind::Task,
        context.source_task_id.as_str(),
        context.project_id,
        Some(context.source_task_id),
        Some(context.planned_state_version),
    );
    let source_change_unit_ref = context
        .source_change_unit_id
        .map(|change_unit_id| {
            state_ref(
                StateRecordKind::ChangeUnit,
                change_unit_id.as_str(),
                context.project_id,
                Some(context.source_task_id),
                Some(context.planned_state_version),
            )
        })
        .into();
    let summary = ProjectContinuitySummary {
        continuity_record_ref: record_ref.clone(),
        kind: draft.kind,
        status: ProjectContinuityStatus::Active,
        title: draft.title.clone(),
        summary: draft.summary.clone(),
        source_task_ref,
        source_change_unit_ref,
        review_triggers: review_triggers.clone(),
    };
    Ok(PlannedProjectContinuityRecord {
        record_ref,
        summary,
        mutation: CoreStorageMutation::Continuity(ContinuityMutation::insert_record(
            ProjectContinuityRecordInsert {
                continuity_record_id: continuity_record_id.as_str().to_owned(),
                source_task_id: context.source_task_id.as_str().to_owned(),
                source_change_unit_id: context
                    .source_change_unit_id
                    .map(|change_unit_id| change_unit_id.as_str().to_owned()),
                kind: draft.kind,
                title: draft.title,
                summary: draft.summary,
                rationale: draft.rationale,
                applies_to_paths,
                applies_to_refs,
                source_refs,
                artifact_refs,
                status: ProjectContinuityStatus::Active,
                supersedes_refs,
                review_triggers,
                created_at: context.now.clone(),
                updated_at: context.now.clone(),
                metadata: draft.metadata,
            },
        )),
    })
}

pub(crate) fn project_continuity_ref(
    record: &ProjectContinuityRecordRecord,
    state_version: u64,
) -> StateRecordRef {
    state_ref(
        StateRecordKind::ProjectContinuityRecord,
        &record.continuity_record_id,
        &ProjectId::new(record.project_id.clone()),
        Some(&TaskId::new(record.source_task_id.clone())),
        Some(state_version),
    )
}

fn project_continuity_record_from_storage(
    record: &ProjectContinuityRecordRecord,
) -> CoreResult<ProjectContinuityRecord> {
    Ok(ProjectContinuityRecord {
        continuity_record_id: ProjectContinuityRecordId::new(record.continuity_record_id.clone()),
        project_id: ProjectId::new(record.project_id.clone()),
        source_task_id: TaskId::new(record.source_task_id.clone()),
        source_change_unit_id: record
            .source_change_unit_id
            .clone()
            .map(ChangeUnitId::new)
            .into(),
        kind: record.kind,
        title: record.title.clone(),
        summary: record.summary.clone(),
        rationale: record.rationale.clone().into(),
        applies_to_paths: record.applies_to_paths.clone(),
        applies_to_refs: record.applies_to_refs.clone(),
        source_refs: record.source_refs.clone(),
        artifact_refs: record.artifact_refs.clone(),
        status: record.status,
        supersedes_refs: record.supersedes_refs.clone(),
        review_triggers: record.review_triggers.clone(),
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    })
}

pub(crate) fn project_continuity_summary_from_record(
    record: &ProjectContinuityRecordRecord,
    state_version: u64,
) -> CoreResult<ProjectContinuitySummary> {
    let continuity = project_continuity_record_from_storage(record)?;
    let project_id = continuity.project_id.clone();
    let source_task_id = continuity.source_task_id.clone();
    let source_change_unit_ref = continuity
        .source_change_unit_id
        .as_ref()
        .map(|change_unit_id| {
            state_ref(
                StateRecordKind::ChangeUnit,
                change_unit_id.as_str(),
                &project_id,
                Some(&source_task_id),
                Some(state_version),
            )
        })
        .into();
    Ok(ProjectContinuitySummary {
        continuity_record_ref: project_continuity_ref(record, state_version),
        kind: continuity.kind,
        status: continuity.status,
        title: continuity.title,
        summary: continuity.summary,
        source_task_ref: state_ref(
            StateRecordKind::Task,
            source_task_id.as_str(),
            &project_id,
            Some(&source_task_id),
            Some(state_version),
        ),
        source_change_unit_ref,
        review_triggers: continuity.review_triggers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::{
        ids::RiskId,
        schema::{PersistedProjectContinuityMetadata, PersistedProjectContinuitySource},
    };

    #[test]
    fn stored_continuity_projects_owner_refs_at_the_requested_state_version() {
        let now = UtcTimestamp::parse("2026-07-29T00:00:00Z").expect("timestamp");
        let record = ProjectContinuityRecordRecord {
            project_id: "project_continuity".to_owned(),
            continuity_record_id: "continuity_known_limit".to_owned(),
            source_task_id: "task_source".to_owned(),
            source_change_unit_id: Some("cu_source".to_owned()),
            kind: ProjectContinuityKind::KnownLimit,
            title: "Known limit".to_owned(),
            summary: "The limitation remains relevant.".to_owned(),
            rationale: Some("Retain it for future work.".to_owned()),
            applies_to_paths: vec!["src".to_owned()],
            applies_to_refs: Vec::new(),
            source_refs: Vec::new(),
            artifact_refs: Vec::new(),
            status: ProjectContinuityStatus::Active,
            supersedes_refs: Vec::new(),
            review_triggers: vec!["scope changes".to_owned()],
            created_at: now.clone(),
            updated_at: now,
            metadata: PersistedProjectContinuityMetadata::CloseTaskKnownLimit {
                source: PersistedProjectContinuitySource::CloseTask,
                risk_id: RiskId::new("risk_known_limit"),
                close_basis_revision: 3,
            },
        };

        let summary =
            project_continuity_summary_from_record(&record, 17).expect("valid stored continuity");

        assert_eq!(
            summary.continuity_record_ref.record_id.as_str(),
            "continuity_known_limit"
        );
        assert_eq!(
            summary.continuity_record_ref.produced_at_state_version,
            Some(17).into()
        );
        assert_eq!(summary.source_task_ref.record_id.as_str(), "task_source");
        assert_eq!(
            summary
                .source_change_unit_ref
                .as_ref()
                .expect("change unit ref")
                .record_id
                .as_str(),
            "cu_source"
        );
    }
}
