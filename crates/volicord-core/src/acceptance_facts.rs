use crate::pipeline::{CorePipelineError, CoreResult};
use volicord_store::core_pipeline::{AcceptanceCriterionRecord, CoreProjectStore};
use volicord_types::ids::{AcceptanceCriterionId, TaskId};
use volicord_types::schema::AcceptanceCriterion;

pub(crate) fn acceptance_criterion_from_record(
    record: &AcceptanceCriterionRecord,
) -> AcceptanceCriterion {
    AcceptanceCriterion {
        acceptance_criterion_id: AcceptanceCriterionId::new(record.acceptance_criterion_id.clone()),
        statement: record.statement.clone(),
        evidence_requirement: record.evidence_requirement,
    }
}

pub(crate) fn active_acceptance_criteria(
    store: &CoreProjectStore,
    task_id: &TaskId,
) -> CoreResult<Vec<AcceptanceCriterion>> {
    Ok(store
        .active_acceptance_criteria(task_id)
        .map_err(CorePipelineError::from)?
        .iter()
        .map(acceptance_criterion_from_record)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_store::core_pipeline::AcceptanceCriterionStatus;
    use volicord_types::values::EvidenceRequirement;

    #[test]
    fn acceptance_fact_owner_projects_store_records_to_typed_facts() {
        let record = AcceptanceCriterionRecord {
            project_id: "project_acceptance_facts".to_owned(),
            acceptance_criterion_id: "criterion_acceptance_facts".to_owned(),
            task_id: "task_acceptance_facts".to_owned(),
            statement: "The focused fact is available.".to_owned(),
            evidence_requirement: EvidenceRequirement::Required,
            position: 0,
            status: AcceptanceCriterionStatus::Active,
        };

        let fact = acceptance_criterion_from_record(&record);

        assert_eq!(
            fact.acceptance_criterion_id.as_str(),
            record.acceptance_criterion_id
        );
        assert_eq!(fact.statement, record.statement);
        assert_eq!(fact.evidence_requirement, EvidenceRequirement::Required);
    }
}
