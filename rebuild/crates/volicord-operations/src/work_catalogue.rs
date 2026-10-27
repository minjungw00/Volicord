//! Authorized catalogue selection, before search, counts, sorting and paging.
use crate::{Error, LocalOperations};
use std::cmp::Reverse;
use volicord_context::{CanonicalReadBasis, ProjectId};
use volicord_privacy::{ManagedDerivedKind, ManagedDerivedState, PrivacyStore};
use volicord_projections::*;

impl LocalOperations {
    pub fn query_work_catalogue(
        &self,
        project: ProjectId,
        query: &WorkCatalogueQuery,
        language: &str,
    ) -> Result<WorkCataloguePage, Error> {
        query.validate().map_err(Error::new)?;
        if language.is_empty() || language.len() > 128 {
            return Err(Error::new("invalid content language"));
        }
        let _inspection = self.layout().acquire_health_lock()?;
        self.work_catalogue_from(&self.canonical_basis(project)?, query, language)
    }

    pub(crate) fn work_catalogue_from(
        &self,
        canonical: &CanonicalReadBasis,
        query: &WorkCatalogueQuery,
        language: &str,
    ) -> Result<WorkCataloguePage, Error> {
        let project = canonical.project.id;
        let invalidations = self.incomplete_committed_invalidations(project)?;
        let records = PrivacyStore::open(self.layout().privacy_store())
            .and_then(|s| s.inspect_work_catalogue_with_invalidations(project, &invalidations))
            .map_err(|e| Error::with_source("catalogue authority unavailable", e))?;
        let mut entries = work_catalogue_metadata(canonical);
        let mut cost = WorkCatalogueCost {
            classified_works: entries.len(),
            ..Default::default()
        };
        cost.storage_metadata_and_card_bytes = serde_json::to_vec(&records.managed_derived)
            .map_err(|e| Error::with_source("catalogue metadata encoding failed", e))?
            .len();
        for entry in &mut entries {
            for (subject, question) in [
                (
                    ExplanationSubject::WorkName(entry.id),
                    ExplanationQuestion::Heading,
                ),
                (
                    ExplanationSubject::WorkSummary(entry.id),
                    ExplanationQuestion::Synopsis,
                ),
            ] {
                cost.card_publications_validated += 1;
                if let Ok((retained, _)) =
                    self.permitted_publication_from(canonical, &records, subject, language)
                {
                    let value = retained
                        .realization
                        .generated_units()
                        .find(|u| u.question == question)
                        .map(|u| u.text.clone());
                    if question == ExplanationQuestion::Heading {
                        entry.name = value;
                    } else {
                        entry.summary = value;
                    }
                    entry
                        .publications
                        .push(ExplanationProvenance::from(&retained));
                }
            }
            let purpose = format!("explanation:work:{}:{language}", entry.id);
            if let Some(record) = records
                .managed_derived
                .iter()
                .rfind(|r| r.kind == ManagedDerivedKind::CachedSummary && r.purpose == purpose)
            {
                entry.article = if matches!(
                    record.state,
                    ManagedDerivedState::Current | ManagedDerivedState::Stale
                ) && !records
                    .withheld_for_canonical_forgetting
                    .contains(&record.id)
                    && (record.source_computed_explanation_deadline(&records.managed_derived)
                        || record.retained_until.is_none_or(|t| {
                            crate::operations::now_micros().is_ok_and(|now| t > now)
                        })) {
                    CatalogueArticle::Recorded
                } else {
                    CatalogueArticle::Unavailable
                };
            }
        }
        // Hash only authorized visible fields, exact publication identities and
        // query scope. Forbidden prose and article bytes never affect this token.
        let visible = entries.iter().map(|e| serde_json::json!({"id":e.id.to_string(),"name":e.name,"summary":e.summary,"publications":e.publications.iter().map(|p| (&p.publication_id,&p.realization_fingerprint)).collect::<Vec<_>>(),"state":format!("{:?}",e.state),"created":e.created_at,"updated":e.updated_at,"checkpoints":e.checkpoint_count,"article":format!("{:?}",e.article)})).collect::<Vec<_>>();
        let basis = explanation_text_hash(&serde_json::json!({"project":project.to_string(),"language":language,"term":query.term.trim().to_lowercase(),"state":format!("{:?}",query.state),"order":format!("{:?}",query.order),"entries":visible}).to_string());
        entries.sort_by_key(|e| (e.updated_at.is_none(), Reverse(e.updated_at), e.id));
        let current = entries
            .iter()
            .find(|e| e.state == UnderstandingWorkState::InProgress)
            .cloned();
        let term = query.term.trim().to_lowercase();
        entries.retain(|e| {
            query.state.is_none_or(|s| s == e.state)
                && (term.is_empty()
                    || e.name
                        .iter()
                        .chain(&e.summary)
                        .any(|s| s.to_lowercase().contains(&term)))
        });
        match query.order {
            WorkCatalogueOrder::Recent => {}
            WorkCatalogueOrder::Oldest => {
                entries.sort_by_key(|e| (e.updated_at.is_none(), e.updated_at, e.id))
            }
            WorkCatalogueOrder::Name => entries.sort_by_cached_key(|e| {
                (
                    e.name.is_none(),
                    e.name.as_deref().unwrap_or_default().to_lowercase(),
                    e.id,
                )
            }),
        }
        let matched_count = entries.len();
        let mut effective = query.clone();
        let refreshed = query.expected_basis.as_ref().is_some_and(|b| b != &basis)
            || query.page.saturating_mul(WORK_CATALOGUE_PAGE_SIZE) >= matched_count
                && query.page > 0;
        if refreshed {
            effective.page = 0;
        }
        effective.expected_basis = Some(basis.clone());
        let entries = entries
            .into_iter()
            .skip(effective.page.saturating_mul(WORK_CATALOGUE_PAGE_SIZE))
            .take(WORK_CATALOGUE_PAGE_SIZE)
            .collect();
        Ok(WorkCataloguePage {
            query: effective,
            basis,
            refreshed,
            matched_count,
            entries,
            current,
            cost,
        })
    }
}
