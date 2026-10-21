//! Source-window inclusion is a reading scope, independent of prose role.
use crate::{ExplanationCitation, QuestionAnswers};
use serde::Serialize;

impl ExplanationCitation {
    /// Claim coordinates deliberately differ for repeated citations. Source
    /// authority and observed payload must agree before a window can be shared.
    pub fn same_source_range(&self, other: &Self) -> bool {
        self.source_id == other.source_id
            && self.source_revision == other.source_revision
            && self.observation_fingerprint == other.observation_fingerprint
            && self.analysis_snapshot_id == other.analysis_snapshot_id
            && self.repository_snapshot_id == other.repository_snapshot_id
            && self.repository_source_id == other.repository_source_id
            && self.locator == other.locator
            && self.field == other.field
            && self.start_byte == other.start_byte
            && self.end_byte == other.end_byte
            && self.start_line == other.start_line
            && self.end_line == other.end_line
            && self.range_sha256 == other.range_sha256
            && self.use_kind == other.use_kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWindowState {
    Complete,
    InitialOmission,
    OfflineOmission,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClaimSourceWindows {
    pub unit_index: usize,
    pub state: SourceWindowState,
    pub included_citation_indices: Vec<usize>,
    pub omitted_citation_indices: Vec<usize>,
}

impl QuestionAnswers {
    /// Complete scope requires every cited window for the claim, not merely a
    /// representative prefix. Uncited records establish no embedded code scope.
    pub fn source_reading_scope(&self, offline: bool) -> Vec<ClaimSourceWindows> {
        self.prose
            .iter()
            .enumerate()
            .filter(|(_, u)| !u.citations.is_empty())
            .map(|(unit_index, unit)| {
                let (included_citation_indices, omitted_citation_indices): (Vec<_>, Vec<_>) =
                    (0..unit.citations.len()).partition(|&i| {
                        self.source_excerpts.iter().any(|e| {
                            e.citation.same_source_range(&unit.citations[i])
                                && e.omitted_utf8_bytes == 0
                                && e.text.len()
                                    == unit.citations[i].end_byte - unit.citations[i].start_byte
                        })
                    });
                let state = if omitted_citation_indices.is_empty() {
                    SourceWindowState::Complete
                } else if offline {
                    SourceWindowState::OfflineOmission
                } else {
                    SourceWindowState::InitialOmission
                };
                ClaimSourceWindows {
                    unit_index,
                    state,
                    included_citation_indices,
                    omitted_citation_indices,
                }
            })
            .collect()
    }
}
