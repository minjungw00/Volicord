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

impl ViewerAdapter {
    pub(crate) fn render_inspection(
        &self,
        r: &crate::ViewerRequest,
        s: &InspectionSelection,
        token: &str,
    ) -> Result<crate::ViewerPage, InspectionFailure> {
        self.render_inspection_profiled(r, s, token).map(|(p, _)| p)
    }
    pub(crate) fn render_inspection_profiled(
        &self,
        r: &crate::ViewerRequest,
        s: &InspectionSelection,
        token: &str,
    ) -> Result<(crate::ViewerPage, crate::ViewerRenderProfile), InspectionFailure> {
        let total = std::time::Instant::now();
        let (page, detail) = self.inspection_read(r.project_id, s)?;
        let mut profile = crate::ViewerRenderProfile {
            inspection_query: total.elapsed(),
            ..Default::default()
        };
        let code = matches!(
            r.view,
            crate::ViewerView::Tools {
                tool: crate::ViewerTool::Evidence
            }
        );
        let projection = self
            .operations()
            .project_projection_read_profiled(
                r.project_id,
                volicord_projections::WorkSelector::Repository,
                Default::default(),
                volicord_projections::ProjectionReadRequirements {
                    code,
                    inspection: false,
                },
            )
            .ok();
        if let Some((_, p)) = &projection {
            profile.projection = *p;
            profile.project_projection_passes = 1;
        }
        let render = std::time::Instant::now();
        let page = crate::render::inspection_page(
            self,
            r,
            s,
            page,
            detail,
            token,
            projection.as_ref().map(|(p, _)| p),
        );
        profile.html_render = render.elapsed();
        profile.total = total.elapsed();
        Ok((page, profile))
    }
}
impl From<InspectionFailure> for crate::ViewerError {
    fn from(value: InspectionFailure) -> Self {
        let mut e = crate::ViewerError::new(value.to_string());
        e.not_found = value == InspectionFailure::NotFound;
        e
    }
}
pub(crate) fn request(
    project: volicord_context::ProjectId,
    locale: crate::ViewerLocale,
    language: String,
    class: volicord_operations::InspectionClass,
) -> crate::ViewerRequest {
    crate::ViewerRequest {
        project_id: project,
        locale,
        requested_language: language,
        guarded_request: None,
        view: crate::ViewerView::Tools {
            tool: if class == volicord_operations::InspectionClass::Source {
                crate::ViewerTool::Evidence
            } else {
                crate::ViewerTool::Memory
            },
        },
    }
}
