//! Request-local catalogue metadata. No article or Source body is an input.
use crate::{ExplanationProvenance, UnderstandingWorkState};
use volicord_context::{CanonicalReadBasis, CanonicalRecordKind, ContextItemId};

pub const WORK_CATALOGUE_PAGE_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WorkCatalogueOrder {
    #[default]
    Recent,
    Oldest,
    Name,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkCatalogueQuery {
    pub term: String,
    pub state: Option<UnderstandingWorkState>,
    pub order: WorkCatalogueOrder,
    /// Zero based; changing the query starts again at zero.
    pub page: usize,
    pub expected_basis: Option<String>,
}
impl WorkCatalogueQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.term.len() > 128
            || self.term.chars().any(char::is_control)
            || self.page >= 1_000_000
        {
            return Err("invalid Work query");
        }
        if self.expected_basis.as_ref().is_some_and(|s| {
            s.len() != 71
                || !s.starts_with("sha256:")
                || !s[7..].bytes().all(|b| b.is_ascii_hexdigit())
        }) {
            return Err("invalid catalogue basis");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogueArticle {
    Missing,
    /// Metadata only: admission and exact ranges are checked on opening.
    Recorded,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkCatalogueEntry {
    pub id: ContextItemId,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub publications: Vec<ExplanationProvenance>,
    pub state: UnderstandingWorkState,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub checkpoint_count: usize,
    pub article: CatalogueArticle,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkCatalogueCost {
    pub classified_works: usize,
    pub card_publications_validated: usize,
    /// Bytes delivered by storage, excluding all article and Source payloads.
    pub storage_metadata_and_card_bytes: usize,
    pub article_preparations: usize,
    pub source_body_reads: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkCataloguePage {
    pub query: WorkCatalogueQuery,
    pub basis: String,
    pub refreshed: bool,
    pub matched_count: usize,
    pub entries: Vec<WorkCatalogueEntry>,
    /// Most recently recorded InProgress Work, never a completed fallback.
    pub current: Option<WorkCatalogueEntry>,
    pub cost: WorkCatalogueCost,
}

/// Complete identity/state/date classification without materializing WorkReading
/// or its quadratic later-change observations. Missing original dates stay absent.
pub fn work_catalogue_metadata(canonical: &CanonicalReadBasis) -> Vec<WorkCatalogueEntry> {
    crate::reading::WorkHistoryIndex::new(canonical)
        .0
        .values()
        .map(|g| {
            let creation = canonical
                .revisions
                .iter()
                .find(|r| {
                    r.record_kind == CanonicalRecordKind::ContextItem
                        && r.record_identity == g.goal.id.to_string()
                        && r.revisions.contains(&1)
                })
                .and_then(|r| r.first_recorded_at)
                .map(|t| t.as_unix_micros());
            let latest = g.checkpoints.last();
            WorkCatalogueEntry {
                id: g.goal.id,
                name: None,
                summary: None,
                publications: Vec::new(),
                state: latest.map_or(UnderstandingWorkState::Open, |c| c.work_state.into()),
                created_at: creation,
                updated_at: latest.map(|c| c.recorded_at.as_unix_micros()).or(creation),
                checkpoint_count: g.checkpoints.len(),
                article: CatalogueArticle::Missing,
            }
        })
        .collect()
}
