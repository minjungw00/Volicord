//! Native, script-free query UI. Product Operations owns every readable field.
use crate::ViewerAdapter;
use volicord_context::ContextItemId;
use volicord_operations::{
    EvidenceDetailRequest, InspectionFailure, InspectionPage, InspectionQuery,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct InspectionSelection {
    pub query: InspectionQuery,
    pub revision: Option<u64>,
    pub field: Option<String>,
    pub field_offset: usize,
    pub field_basis: Option<String>,
    pub work: Option<ContextItemId>,
    pub relationship_offset: usize,
}
impl ViewerAdapter {
    pub(crate) fn inspection_read(
        &self,
        project: volicord_context::ProjectId,
        s: &InspectionSelection,
    ) -> Result<(InspectionPage, Option<serde_json::Value>), InspectionFailure> {
        if let Some(field) = &s.field {
            let request = EvidenceDetailRequest {
                record_kind: s
                    .query
                    .class
                    .evidence_kind()
                    .ok_or(InspectionFailure::Invalid)?
                    .into(),
                record_id: s.query.identity.clone().ok_or(InspectionFailure::Invalid)?,
                revision: s.revision.ok_or(InspectionFailure::Invalid)?,
                field: field.clone(),
                offset: s.field_offset,
                expected_fingerprint: s.field_basis.clone(),
                work_item_id: s.work,
            };
            self.operations()
                .inspect_project_record_field(project, &s.query, &request)
                .map(|(p, d)| (p, Some(d)))
        } else {
            let page = if s.relationship_offset > 0 {
                self.operations().inspect_record_relationships(
                    project,
                    &s.query,
                    s.relationship_offset,
                )?
            } else {
                self.operations()
                    .inspect_project_records(project, &s.query)?
            };
            Ok((page, None))
        }
    }
}
