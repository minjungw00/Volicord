//! Request-local inspection over existing authorized reads, never database queries.
use crate::{Error, LocalOperations};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use volicord_context::*;
use volicord_context::{ContextItemRole, DecisionWorkScope, ProjectId, SourcePayload};
use volicord_projections::{CandidateContentAccess, CandidateContentAdmission};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum InspectionClass {
    Work,
    Context,
    Checkpoint,
    Decision,
    Question,
    Source,
    Candidate,
    Derived,
}
impl InspectionClass {
    pub const ALL: [Self; 8] = [
        Self::Work,
        Self::Checkpoint,
        Self::Decision,
        Self::Question,
        Self::Source,
        Self::Context,
        Self::Candidate,
        Self::Derived,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Context => "context",
            Self::Checkpoint => "checkpoint",
            Self::Decision => "decision",
            Self::Question => "question",
            Self::Source => "source",
            Self::Candidate => "candidate",
            Self::Derived => "derived",
        }
    }
    pub fn parse(value: &str) -> Result<Self, InspectionFailure> {
        Self::ALL
            .into_iter()
            .find(|c| c.key() == value)
            .ok_or(InspectionFailure::Invalid)
    }
    pub fn evidence_kind(self) -> Option<&'static str> {
        match self {
            Self::Work | Self::Context => Some("context_item"),
            Self::Derived => None,
            c => Some(c.key()),
        }
    }
}

pub const INSPECTION_STATES: &[&str] = &[
    "current",
    "active",
    "open",
    "terminal",
    "review_due",
    "superseded",
    "in_progress",
    "paused",
    "completed",
    "abandoned",
    "stale",
    "unavailable",
    "unknown",
    "withheld",
    "expired",
    "deleted",
    "invalidated",
    "pending",
    "promoted",
    "dismissed",
];
pub const INSPECTION_ROLES: &[&str] = &[
    "goal",
    "project_purpose",
    "fact",
    "assumption",
    "constraint",
    "preference",
    "risk",
    "learning",
    "known_limit",
    "source_basis",
    "material_question",
    "user_judgment",
    "source_grounded_checkpoint",
    "candidate_metadata",
    "derived_metadata",
];

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InspectionQuery {
    pub class: InspectionClass,
    pub term: String,
    pub state: Option<String>,
    pub role: Option<String>,
    pub offset: usize,
    pub limit: usize,
    pub expected_fingerprint: Option<String>,
    /// Exact row selection still admits and filters all fields before returning it.
    pub identity: Option<String>,
}
impl Default for InspectionQuery {
    fn default() -> Self {
        Self {
            class: InspectionClass::Work,
            term: String::new(),
            state: None,
            role: None,
            offset: 0,
            limit: 10,
            expected_fingerprint: None,
            identity: None,
        }
    }
}
impl InspectionQuery {
    pub fn validate(&self) -> Result<(), InspectionFailure> {
        if self.term.len() > 128
            || self.term.chars().any(char::is_control)
            || !(1..=20).contains(&self.limit)
            || self.offset > 100_000
            || self
                .state
                .as_ref()
                .is_some_and(|s| !INSPECTION_STATES.contains(&s.as_str()))
            || self
                .role
                .as_ref()
                .is_some_and(|s| !INSPECTION_ROLES.contains(&s.as_str()))
            || self.identity.as_ref().is_some_and(|s| !hex_identity(s, 32))
            || self
                .expected_fingerprint
                .as_ref()
                .is_some_and(|s| !hex_identity(s, 64))
            || self.identity.is_some() && self.offset != 0
        {
            return Err(InspectionFailure::Invalid);
        }
        Ok(())
    }
}
fn hex_identity(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct InspectionRelation {
    pub recorded_revision: Option<u64>,
    pub relation: String,
    pub class: InspectionClass,
    pub identity: String,
    pub revision: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct InspectionRow {
    pub class: InspectionClass,
    pub identity: String,
    pub revision: Option<u64>,
    pub role: String,
    pub state: String,
    pub summary: String,
    pub summary_omitted: bool,
    pub disclosure: String,
    pub fields: Vec<String>,
    pub metadata: serde_json::Value,
    pub relationships: Vec<InspectionRelation>,
    pub next_relationship_offset: Option<usize>,
    /// No hidden-target count or identity is exposed.
    pub relationships_omitted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct InspectionPage {
    pub project_id: String,
    pub project_name: String,
    pub rows: Vec<InspectionRow>,
    pub fingerprint: String,
    pub offset: usize,
    pub next_offset: Option<usize>,
    pub permitted_match_count: usize,
    pub omitted_match_count: usize,
    pub read_only: bool,
}
/// Request-local observations, separate from the public row/page representation.
/// Byte/row counters are work performed, not allocation or RSS measurements.
#[derive(Clone, Debug, Default)]
pub struct InspectionReadCost {
    pub indexed_records: usize,
    pub indexed_relationships: usize,
    pub visited_rows: usize,
    pub matched_rows: usize,
    pub retained_rows: usize,
    pub fingerprint_bytes: usize,
    pub maximum_row_relationships: usize,
    pub canonical_read: std::time::Duration,
    pub auxiliary_read: std::time::Duration,
    pub index_build: std::time::Duration,
    pub query_and_fingerprint: std::time::Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectionFailure {
    Invalid,
    NotFound,
    Changed,
    Unavailable,
    Unsupported,
    Corrupt,
    Withheld,
}
impl std::fmt::Display for InspectionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self { Self::Invalid => "Invalid bounded inspection query.",
            Self::NotFound => "The requested record is unavailable in this Project.",
            Self::Changed => "The permitted read basis changed. Restart inspection.",
            Self::Unsupported => "The requested inspection dependency has an unsupported format. Use Analysis for recovery guidance.",
            Self::Withheld => "The requested Candidate field is withheld; only permitted metadata is readable.",
            Self::Corrupt => "The requested inspection dependency is corrupt. Use Analysis for recovery guidance.",
            Self::Unavailable => "The requested inspection dependency is unavailable. Use Analysis for recovery guidance." })
    }
}
impl std::error::Error for InspectionFailure {}
impl From<Error> for InspectionFailure {
    fn from(error: Error) -> Self {
        match error.inquiry_cause().map(|e| e.kind()) {
            Some(volicord_inquiry::ErrorKind::UnsupportedVersion) => Self::Unsupported,
            Some(volicord_inquiry::ErrorKind::CorruptState) => Self::Corrupt,
            _ => Self::Unavailable,
        }
    }
}

impl LocalOperations {
    /// Filters, counts, ordering, fingerprints and relationships use only admitted fields.
    /// The existing lock coordinates invalidation; no persistence or retention mutation.
    pub fn inspect_project_records(
        &self,
        project: ProjectId,
        query: &InspectionQuery,
    ) -> Result<InspectionPage, InspectionFailure> {
        self.inspect_project_records_profiled(project, query)
            .map(|(page, _)| page)
    }
    pub fn inspect_project_records_profiled(
        &self,
        project: ProjectId,
        query: &InspectionQuery,
    ) -> Result<(InspectionPage, InspectionReadCost), InspectionFailure> {
        query.validate()?;
        let _read = self.layout().acquire_health_lock()?;
        self.inspection_page(project, query, None)
    }
    fn inspect_project_records_unlocked(
        &self,
        project: ProjectId,
        query: &InspectionQuery,
    ) -> Result<InspectionPage, InspectionFailure> {
        self.inspection_page(project, query, None)
            .map(|(page, _)| page)
    }

    fn inspection_page(
        &self,
        project: ProjectId,
        query: &InspectionQuery,
        relationship_offset: Option<usize>,
    ) -> Result<(InspectionPage, InspectionReadCost), InspectionFailure> {
        let start = std::time::Instant::now();
        let basis = self.canonical_basis(project)?;
        let mut cost = InspectionReadCost {
            canonical_read: start.elapsed(),
            ..Default::default()
        };
        let auxiliary = std::time::Instant::now();
        let candidates = (query.class == InspectionClass::Candidate)
            .then(|| self.candidate_basis(project))
            .transpose()?;
        let privacy = (query.class == InspectionClass::Derived)
            .then(|| self.privacy_status(project))
            .transpose()?;
        let now = volicord_context::Clock::now(&mut volicord_context::SystemClock)
            .map_err(|_| InspectionFailure::Unavailable)?;
        cost.auxiliary_read = auxiliary.elapsed();
        let indexing = std::time::Instant::now();
        let index = InspectionIndex::new(&basis, candidates.as_ref(), privacy.as_ref(), now);
        cost.index_build = indexing.elapsed();
        cost.indexed_records = index.nodes.len();
        cost.indexed_relationships = index.nodes.values().map(|n| n.links.len()).sum();
        let querying = std::time::Instant::now();
        let term = query.term.to_lowercase();
        // Stream the exact previous JSON fingerprint encoding. No full matched
        // row vector or serialized copy, and offset/identity do not affect it.
        let mut hash = HashWriter(Sha256::new(), 0);
        hash.write(b"[")?;
        let header = serde_json::to_vec(&(
            project.to_string(),
            query.class,
            &query.term,
            &query.state,
            &query.role,
            query.limit,
        ))
        .map_err(|_| InspectionFailure::Unavailable)?;
        hash.write(&header[1..header.len() - 1])?;
        hash.write(b",[")?;
        let mut count = 0usize;
        let mut selected = Vec::new();
        let mut invalid_relationship_offset = false;
        for ((class, identity), node) in &index.nodes {
            if *class != query.class {
                continue;
            }
            cost.visited_rows += 1;
            let mut row = index.row(*class, identity, node);
            cost.maximum_row_relationships =
                cost.maximum_row_relationships.max(row.relationships.len());
            if query.state.as_ref().is_some_and(|s| *s != row.state)
                || query.role.as_ref().is_some_and(|s| *s != row.role)
                || !(term.is_empty()
                    || row.summary.to_lowercase().contains(&term)
                    || row.identity.contains(&term))
            {
                continue;
            }
            if count > 0 {
                hash.write(b",")?;
            }
            serde_json::to_writer(&mut hash, &row).map_err(|_| InspectionFailure::Unavailable)?;
            let keep = query.identity.as_ref().map_or_else(
                || count >= query.offset && count - query.offset < query.limit,
                |id| *id == row.identity,
            );
            count += 1;
            if keep {
                let offset = relationship_offset.unwrap_or(0);
                if offset > row.relationships.len() {
                    invalid_relationship_offset = true;
                    continue;
                }
                let end = offset.saturating_add(8).min(row.relationships.len());
                row.next_relationship_offset = (end < row.relationships.len()).then_some(end);
                row.relationships = row.relationships.drain(offset..end).collect();
                selected.push(row);
            }
        }
        hash.write(b"]]")?;
        cost.fingerprint_bytes = hash.1;
        cost.matched_rows = count;
        cost.retained_rows = selected.len();
        cost.query_and_fingerprint = querying.elapsed();
        let fingerprint = format!("{:x}", hash.0.finalize());
        if query.offset > 0 && query.expected_fingerprint.is_none()
            || query
                .expected_fingerprint
                .as_ref()
                .is_some_and(|f| *f != fingerprint)
        {
            return Err(InspectionFailure::Changed);
        }
        if invalid_relationship_offset {
            return Err(InspectionFailure::Invalid);
        }
        if query.identity.is_some() {
            if selected.is_empty() {
                return Err(InspectionFailure::NotFound);
            }
            count = 1;
        }
        if query.offset > count {
            return Err(InspectionFailure::Invalid);
        }
        let end = query.offset.saturating_add(query.limit).min(count);
        Ok((
            InspectionPage {
                project_id: project.to_string(),
                project_name: basis.project.display_name,
                rows: selected,
                fingerprint,
                offset: query.offset,
                next_offset: (end < count).then_some(end),
                permitted_match_count: count,
                omitted_match_count: count - end,
                read_only: true,
            },
            cost,
        ))
    }

    /// The selected row and field share coordination with correction/forgetting.
    pub fn inspect_project_record_field(
        &self,
        project: ProjectId,
        query: &InspectionQuery,
        request: &crate::EvidenceDetailRequest,
    ) -> Result<(InspectionPage, serde_json::Value), InspectionFailure> {
        query.validate()?;
        if query.identity.as_ref() != Some(&request.record_id)
            || query.class.evidence_kind() != Some(request.record_kind.as_str())
        {
            return Err(InspectionFailure::Invalid);
        }
        let _read = self.layout().acquire_health_lock()?;
        let page = self.inspect_project_records_unlocked(project, query)?;
        let row = page.rows.first().ok_or(InspectionFailure::NotFound)?;
        if row.revision != Some(request.revision) {
            return Err(InspectionFailure::Changed);
        }
        if !row.fields.contains(&request.field) {
            if query.class == InspectionClass::Candidate
                && [
                    "summary",
                    "engineering_choice_discovery",
                    "materiality_review",
                    "learning_deliberation",
                    "repository_research_basis",
                    "source_status",
                ]
                .contains(&request.field.as_str())
            {
                return Err(InspectionFailure::Withheld);
            }
            return Err(InspectionFailure::Invalid);
        }
        let detail = if query.class == InspectionClass::Candidate {
            let basis = self.candidate_basis(project)?;
            let id = basis
                .candidates
                .iter()
                .find(|c| c.id.to_string() == row.identity)
                .ok_or(InspectionFailure::NotFound)?
                .id;
            self.candidate_evidence_detail_unlocked(project, id, request)
        } else {
            self.canonical_evidence_detail_unlocked(project, request)
        }
        .map_err(|_| InspectionFailure::Changed)?;
        Ok((page, detail))
    }

    /// Exact relationship pages recheck the same admitted view and continuation basis.
    pub fn inspect_record_relationships(
        &self,
        project: ProjectId,
        query: &InspectionQuery,
        offset: usize,
    ) -> Result<InspectionPage, InspectionFailure> {
        query.validate()?;
        if query.identity.is_none()
            || offset > 100_000
            || offset > 0 && query.expected_fingerprint.is_none()
        {
            return Err(InspectionFailure::Invalid);
        }
        // Hold coordination through selection, fingerprinting and relationship paging.
        let _read = self.layout().acquire_health_lock()?;
        self.inspection_page(project, query, Some(offset))
            .map(|(page, _)| page)
    }
}

struct HashWriter(Sha256, usize);
impl HashWriter {
    fn write(&mut self, bytes: &[u8]) -> Result<(), InspectionFailure> {
        self.0.update(bytes);
        self.1 += bytes.len();
        Ok(())
    }
}
impl std::io::Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        self.1 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

type Key = (InspectionClass, String);
/// Only borrowed record payloads and admitted identity/edge metadata live here.
/// O(N + E) index storage; rich rows are transient plus at most the requested page.
struct InspectionIndex<'a> {
    nodes: BTreeMap<Key, InspectionNode<'a>>,
    questions: BTreeMap<QuestionId, &'a Question>,
    latest_work: BTreeMap<ContextItemId, &'a Checkpoint>,
}
struct InspectionNode<'a> {
    record: InspectionRecord<'a>,
    revision: Option<u64>,
    links: Vec<InspectionRelation>,
    omitted: bool,
}
enum InspectionRecord<'a> {
    Context(&'a ContextItem),
    Checkpoint(&'a Checkpoint),
    Decision(&'a DecisionLifecycle),
    Question(&'a Question),
    Source(&'a SourceReadBasis),
    Candidate(
        &'a volicord_inquiry::CandidateRecord,
        Option<volicord_projections::CandidateContentOmission>,
    ),
    Derived(&'a volicord_privacy::ManagedDerivedRecord, String),
}

fn new_row(
    class: InspectionClass,
    identity: String,
    revision: Option<u64>,
    role: String,
    state: String,
    summary: String,
    disclosure: String,
) -> InspectionRow {
    let clipped = summary.chars().take(240).collect::<String>();
    InspectionRow {
        class,
        identity,
        revision,
        role,
        state,
        summary_omitted: clipped.len() < summary.len(),
        summary: clipped,
        disclosure,
        fields: Vec::new(),
        metadata: serde_json::json!({}),
        relationships: Vec::new(),
        next_relationship_offset: None,
        relationships_omitted: false,
    }
}
impl<'a> InspectionIndex<'a> {
    fn new(
        basis: &'a CanonicalReadBasis,
        candidates: Option<&'a volicord_inquiry::CandidateReadBasis>,
        privacy: Option<&'a volicord_privacy::ProjectPrivacyInspection>,
        now: TimestampMicros,
    ) -> Self {
        let mut index = Self {
            nodes: BTreeMap::new(),
            questions: BTreeMap::new(),
            latest_work: BTreeMap::new(),
        };
        for item in &basis.context_items {
            index.insert(
                context_class(item),
                item.id.to_string(),
                Some(item.revision),
                InspectionRecord::Context(item),
            );
        }
        for q in basis
            .active_questions
            .iter()
            .chain(&basis.terminal_question_history)
        {
            index.questions.insert(q.id, q);
            index.insert(
                InspectionClass::Question,
                q.id.to_string(),
                Some(q.revision),
                InspectionRecord::Question(q),
            );
        }
        for d in basis
            .active_decisions
            .iter()
            .chain(&basis.superseded_decisions)
        {
            index.insert(
                InspectionClass::Decision,
                d.decision.id.to_string(),
                Some(d.decision.revision),
                InspectionRecord::Decision(d),
            );
        }
        // History already includes latest; do not index its duplicate or synthesize duplicate edges.
        let checkpoints: Box<dyn Iterator<Item = &Checkpoint>> =
            if basis.checkpoint_history.is_empty() {
                Box::new(basis.latest_checkpoint.iter())
            } else {
                Box::new(basis.checkpoint_history.iter())
            };
        for cp in checkpoints {
            if let Some(work) = cp.work_item_id {
                let latest = index.latest_work.entry(work).or_insert(cp);
                if (cp.recorded_at, cp.id) > (latest.recorded_at, latest.id) {
                    *latest = cp;
                }
            }
            index.insert(
                InspectionClass::Checkpoint,
                cp.id.to_string(),
                Some(cp.revision),
                InspectionRecord::Checkpoint(cp),
            );
        }
        let source_revisions: BTreeMap<_, _> = basis
            .revisions
            .iter()
            .filter(|r| r.record_kind == CanonicalRecordKind::Source)
            .map(|r| {
                (
                    r.record_identity.as_str(),
                    r.revisions.last().copied().unwrap_or(1),
                )
            })
            .collect();
        for source in &basis.sources {
            let id = source.source.id.to_string();
            let revision = source_revisions.get(id.as_str()).copied().unwrap_or(1);
            index.insert(
                InspectionClass::Source,
                id,
                Some(revision),
                InspectionRecord::Source(source),
            );
        }
        if let Some(candidates) = candidates {
            let admission = CandidateContentAdmission::new(candidates);
            for c in &candidates.candidates {
                if c.project_id != basis.project.id {
                    continue;
                }
                let omission =
                    admission.omission(c, CandidateContentAccess::AllowBoundedSummary, now);
                index.insert(
                    InspectionClass::Candidate,
                    c.id.to_string(),
                    Some(c.revision),
                    InspectionRecord::Candidate(c, omission),
                );
            }
        }
        if let Some(privacy) = privacy {
            let withheld: std::collections::BTreeSet<_> = privacy
                .withheld_for_canonical_forgetting
                .iter()
                .copied()
                .collect();
            for d in &privacy.managed_derived {
                if d.project_id != basis.project.id {
                    continue;
                }
                let state = if withheld.contains(&d.id) {
                    "withheld".into()
                } else if d.state == volicord_privacy::ManagedDerivedState::Deleted {
                    "deleted".into()
                } else if d.retained_until.is_some_and(|t| t <= now) {
                    "expired".into()
                } else {
                    serde_json::to_value(d.state)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_else(|| "unavailable".into())
                };
                index.insert(
                    InspectionClass::Derived,
                    d.id.to_string(),
                    None,
                    InspectionRecord::Derived(d, state),
                );
            }
        }
        // Resolve each endpoint by class AND identity. Hidden endpoints never enter
        // reverse edges, hashes, counts or public relationship pagination.
        let mut edges = Vec::new();
        let mut omitted = Vec::new();
        for (key, node) in &index.nodes {
            let mut missing = false;
            for mut relation in index.forward(node) {
                if let Some(target) = index
                    .nodes
                    .get(&(relation.class, relation.identity.clone()))
                {
                    relation.revision = target.revision;
                    edges.push((key.clone(), relation));
                } else {
                    missing = true;
                }
            }
            if missing {
                omitted.push(key.clone());
            }
        }
        for key in omitted {
            if let Some(n) = index.nodes.get_mut(&key) {
                n.omitted = true;
            }
        }
        for (key, relation) in edges {
            let revision = index.nodes.get(&key).and_then(|n| n.revision);
            if let Some(target) = index
                .nodes
                .get_mut(&(relation.class, relation.identity.clone()))
            {
                target.links.push(InspectionRelation {
                    recorded_revision: None,
                    relation: format!("referenced_by_{}", relation.relation),
                    class: key.0,
                    identity: key.1.clone(),
                    revision,
                });
            }
            if let Some(source) = index.nodes.get_mut(&key) {
                source.links.push(relation);
            }
        }
        for node in index.nodes.values_mut() {
            node.links.sort();
            node.links.dedup();
        }
        index
    }
    fn insert(
        &mut self,
        class: InspectionClass,
        id: String,
        revision: Option<u64>,
        record: InspectionRecord<'a>,
    ) {
        self.nodes.insert(
            (class, id),
            InspectionNode {
                record,
                revision,
                links: Vec::new(),
                omitted: false,
            },
        );
    }
    fn forward(&self, node: &InspectionNode<'_>) -> Vec<InspectionRelation> {
        let mut links = Vec::new();
        let mut add = |name: &str, class, identity: String, recorded_revision| {
            links.push(InspectionRelation {
                recorded_revision,
                relation: name.into(),
                class,
                identity,
                revision: None,
            })
        };
        match &node.record {
            InspectionRecord::Context(c) => {
                for s in &c.source_basis {
                    add("supported_by", InspectionClass::Source, s.to_string(), None);
                }
            }
            InspectionRecord::Question(q) => {
                for s in &q.source_basis {
                    add("supported_by", InspectionClass::Source, s.to_string(), None);
                }
            }
            InspectionRecord::Decision(d) => {
                for s in d
                    .decision
                    .displayed_recommendation
                    .source_basis
                    .iter()
                    .copied()
                    .chain([d.decision.user_turn_source_id])
                {
                    add("supported_by", InspectionClass::Source, s.to_string(), None);
                }
                add(
                    "answers",
                    InspectionClass::Question,
                    d.decision.question_id.to_string(),
                    Some(d.decision.question_revision),
                );
                if let DecisionWorkScope::WorkItem(work) = d.decision.work_scope {
                    add("applies_to", InspectionClass::Work, work.to_string(), None);
                }
                if let Some(id) = d.superseded_by {
                    add(
                        "superseded_by",
                        InspectionClass::Decision,
                        id.to_string(),
                        None,
                    );
                }
            }
            InspectionRecord::Checkpoint(cp) => {
                for s in &cp.source_basis {
                    add("supported_by", InspectionClass::Source, s.to_string(), None);
                }
                if let Some(work) = cp.work_item_id {
                    add(
                        "records_state_of",
                        InspectionClass::Work,
                        work.to_string(),
                        None,
                    );
                }
                for d in &cp.applied_decisions {
                    add(
                        "applied_decision",
                        InspectionClass::Decision,
                        d.to_string(),
                        None,
                    );
                }
                for s in cp
                    .changed_source_basis
                    .iter()
                    .copied()
                    .chain(cp.verification.iter().filter_map(|v| v.source_id))
                    .chain(cp.user_review.source_id)
                    .chain(cp.user_acceptance.source_id)
                {
                    add(
                        "observed_source",
                        InspectionClass::Source,
                        s.to_string(),
                        None,
                    );
                }
                for q in &cp.open_questions {
                    add(
                        "open_question",
                        InspectionClass::Question,
                        q.question_id.to_string(),
                        Some(q.revision),
                    );
                }
            }
            InspectionRecord::Candidate(c, omission) if omission.is_none() => {
                for s in &c.observation_basis.source_basis {
                    add("derived_from", InspectionClass::Source, s.to_string(), None);
                }
                if let Some(content) = &c.content {
                    let work = content
                        .learning_deliberation
                        .as_ref()
                        .map(|d| d.goal_context_id)
                        .or_else(|| {
                            content
                                .materiality_review
                                .as_ref()
                                .map(|d| d.goal_context_id)
                        })
                        .or_else(|| {
                            content
                                .engineering_choice_discovery
                                .as_ref()
                                .map(|d| d.goal_context_id)
                        });
                    if let Some(work) = work {
                        add(
                            "candidate_for",
                            InspectionClass::Work,
                            work.to_string(),
                            None,
                        );
                    }
                }
            }
            InspectionRecord::Derived(d, state)
                if !["withheld", "deleted", "expired"].contains(&state.as_str()) =>
            {
                for l in &d.canonical_links {
                    use volicord_privacy::ManagedCanonicalLink::*;
                    let (class, id) = match l {
                        Project(_) => continue,
                        Source(id) => (InspectionClass::Source, id.to_string()),
                        Question(id) => (InspectionClass::Question, id.to_string()),
                        Decision(id) => (InspectionClass::Decision, id.to_string()),
                        Checkpoint(id) => (InspectionClass::Checkpoint, id.to_string()),
                        ContextItem(id) => {
                            let id = id.to_string();
                            (
                                if self
                                    .nodes
                                    .contains_key(&(InspectionClass::Work, id.clone()))
                                {
                                    InspectionClass::Work
                                } else {
                                    InspectionClass::Context
                                },
                                id,
                            )
                        }
                    };
                    add("derived_from", class, id, None);
                }
            }
            _ => {}
        }
        links
    }
    fn row(&self, class: InspectionClass, id: &str, node: &InspectionNode<'_>) -> InspectionRow {
        let (role, state, summary, disclosure) = match &node.record {
            InspectionRecord::Context(c) => (
                context_role(c.role).into(),
                self.latest_work
                    .get(&c.id)
                    .filter(|_| class == InspectionClass::Work)
                    .map_or_else(|| "current".into(), |cp| work_state(cp.work_state).into()),
                clip(&c.statement),
                "permitted_canonical_quotation".into(),
            ),
            InspectionRecord::Question(q) => (
                "material_question".into(),
                if q.state == QuestionState::Open {
                    "open"
                } else {
                    "terminal"
                }
                .into(),
                clip(&q.prompt_basis),
                "permitted_canonical_quotation".into(),
            ),
            InspectionRecord::Decision(d) => {
                let choice = match &d.decision.choice {
                    DecisionChoice::Alternative { alternative_key } => {
                        format!("Alternative: {}", clip(alternative_key))
                    }
                    DecisionChoice::Delegation { delegate_to } => {
                        format!("Delegated to: {}", clip(delegate_to))
                    }
                };
                let summary = self.questions.get(&d.decision.question_id).map_or_else(
                    || choice.clone(),
                    |q| format!("{} · {}", clip(&q.prompt_basis), choice),
                );
                (
                    "user_judgment".into(),
                    if d.superseded_by.is_some() {
                        "superseded"
                    } else if d.review_due.is_some() {
                        "review_due"
                    } else {
                        "active"
                    }
                    .into(),
                    summary,
                    "permitted_canonical_quotation".into(),
                )
            }
            InspectionRecord::Checkpoint(cp) => (
                "source_grounded_checkpoint".into(),
                work_state(cp.work_state).into(),
                clip(cp.state_change.as_ref().unwrap_or(&cp.goal)),
                "permitted_canonical_quotation".into(),
            ),
            InspectionRecord::Source(s) => {
                let summary = match &s.source.payload {
                    SourcePayload::CurrentHostUserTurn { .. } => {
                        "User input · body withheld".into()
                    }
                    SourcePayload::CommandExecution {
                        command_label,
                        outcome,
                        ..
                    } => format!(
                        "Command {} · exit {:?}",
                        clip(command_label),
                        outcome.exit_code
                    ),
                    SourcePayload::File { locator, .. } => {
                        format!("File observation · {}", clip(locator))
                    }
                    SourcePayload::Symbol { locator, .. } => {
                        format!("Symbol observation · {}", clip(locator))
                    }
                    SourcePayload::RepositorySnapshot { .. } => "Repository snapshot".into(),
                    SourcePayload::RepositoryCommit { .. } => "Repository commit".into(),
                    SourcePayload::Url { .. } => "URL reference · body not retained".into(),
                    SourcePayload::AdoptedArtifact { .. } => "Adopted artifact reference".into(),
                };
                (
                    "source_basis".into(),
                    format!("{:?}", s.freshness).to_lowercase(),
                    summary,
                    "metadata_only_body_requires_exact_scoped_reader".into(),
                )
            }
            InspectionRecord::Candidate(c, omission) => {
                use volicord_projections::CandidateContentOmission::*;
                let state = match omission {
                    Some(RetentionExpired) => "expired",
                    Some(RetentionCleaned) => "deleted",
                    Some(_) => "withheld",
                    None => match c.disposition {
                        volicord_inquiry::CandidateDisposition::PendingOrRetained => "pending",
                        volicord_inquiry::CandidateDisposition::Promoted { .. } => "promoted",
                        volicord_inquiry::CandidateDisposition::Dismissed { .. } => "dismissed",
                        volicord_inquiry::CandidateDisposition::ExpiredOrRetentionCleaned => {
                            "expired"
                        }
                    },
                };
                let summary = if omission.is_none() {
                    c.content.as_ref().map(|v| clip(&v.bounded_summary))
                } else {
                    None
                }
                .unwrap_or_else(|| format!("{:?} · content withheld", c.kind));
                (
                    "candidate_metadata".into(),
                    state.into(),
                    summary,
                    omission.as_ref().map_or_else(
                        || "permitted_bounded_candidate_summary".into(),
                        |o| format!("{o:?}"),
                    ),
                )
            }
            InspectionRecord::Derived(d, state) => (
                "derived_metadata".into(),
                state.clone(),
                format!("{:?} · {state}", d.kind),
                "metadata_only_no_general_body_reader".into(),
            ),
        };
        let mut row = new_row(
            class,
            id.into(),
            node.revision,
            role,
            state,
            summary,
            disclosure,
        );
        // clip keeps one extra character so new_row computes the exact omission bit.
        match &node.record {
            InspectionRecord::Candidate(c, omission) => {
                row.metadata = serde_json::json!({"candidate_kind":format!("{:?}",c.kind),"created_at_unix_micros":c.created_at.as_unix_micros(),"observed_at_unix_micros":c.observed_at.as_unix_micros(),"retained_until_unix_micros":c.retention.retained_until.map(|t|t.as_unix_micros())});
                if omission.is_none() {
                    row.fields = [
                        "summary",
                        "engineering_choice_discovery",
                        "materiality_review",
                        "learning_deliberation",
                        "repository_research_basis",
                        "source_status",
                    ]
                    .map(str::to_owned)
                    .to_vec();
                }
            }
            InspectionRecord::Derived(d, _) => {
                row.metadata = serde_json::json!({"derived_kind":d.kind,"created_at_unix_micros":d.created_at.as_unix_micros(),"retained_until_unix_micros":d.retained_until.map(|t|t.as_unix_micros()),"analysis_snapshot":d.analysis_snapshot.map(|a|a.to_string())})
            }
            _ => {
                row.fields =
                    crate::evidence::canonical_fields(class.evidence_kind().unwrap_or_default())
                        .iter()
                        .map(|s| (*s).into())
                        .collect();
                if let InspectionRecord::Source(s) = &node.record {
                    row.metadata = serde_json::json!({"recorded_at_unix_micros":s.source.recorded_at.as_unix_micros(),"availability":format!("{:?}",s.availability).to_lowercase(),"freshness":row.state});
                }
            }
        }
        row.relationships = node.links.clone();
        row.relationships_omitted = node.omitted;
        row
    }
}
fn clip(text: &str) -> String {
    text.chars().take(241).collect()
}
fn context_class(item: &ContextItem) -> InspectionClass {
    if item.role == ContextItemRole::Goal {
        InspectionClass::Work
    } else {
        InspectionClass::Context
    }
}
fn work_state(state: WorkState) -> &'static str {
    match state {
        WorkState::InProgress => "in_progress",
        WorkState::Paused => "paused",
        WorkState::Completed => "completed",
        WorkState::Abandoned => "abandoned",
        WorkState::Superseded => "superseded",
    }
}

fn context_role(role: ContextItemRole) -> &'static str {
    match role {
        ContextItemRole::ProjectPurpose => "project_purpose",
        ContextItemRole::Goal => "goal",
        ContextItemRole::Fact => "fact",
        ContextItemRole::Assumption => "assumption",
        ContextItemRole::Constraint => "constraint",
        ContextItemRole::Preference => "preference",
        ContextItemRole::Risk => "risk",
        ContextItemRole::Learning => "learning",
        ContextItemRole::KnownLimit => "known_limit",
    }
}
