//! Request-local inspection over existing authorized reads, never database queries.
use crate::{Error, LocalOperations};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use volicord_context::{ContextItemRole, DecisionWorkScope, ProjectId, SourcePayload};
use volicord_projections::{inspect_candidate, CandidateContentAccess, CanonicalInspectionKind};

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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectionFailure {
    Invalid,
    NotFound,
    Changed,
    Unavailable,
}
impl std::fmt::Display for InspectionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self { Self::Invalid => "Invalid bounded inspection query.",
            Self::NotFound => "The requested record is unavailable in this Project.",
            Self::Changed => "The permitted read basis changed. Restart inspection.",
            Self::Unavailable => "The requested inspection dependency is unavailable. Use Analysis for recovery guidance." })
    }
}
impl std::error::Error for InspectionFailure {}
impl From<Error> for InspectionFailure {
    fn from(_: Error) -> Self {
        Self::Unavailable
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
        query.validate()?;
        let _read = self.layout().acquire_health_lock()?;
        self.inspect_project_records_unlocked(project, query)
    }
    fn inspect_project_records_unlocked(
        &self,
        project: ProjectId,
        query: &InspectionQuery,
    ) -> Result<InspectionPage, InspectionFailure> {
        let (name, mut rows) = self.inspection_rows(project, query.class)?;
        let term = query.term.to_lowercase();
        rows.retain(|r| {
            r.class == query.class
                && query.state.as_ref().is_none_or(|s| *s == r.state)
                && query.role.as_ref().is_none_or(|s| *s == r.role)
                && (term.is_empty()
                    || r.summary.to_lowercase().contains(&term)
                    || r.identity.contains(&term))
        });
        rows.sort_by(|a, b| (a.class, &a.identity).cmp(&(b.class, &b.identity)));
        // Bind query independently of offset/selection, never hash excluded body bytes.
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(
                    project.to_string(),
                    query.class,
                    &query.term,
                    &query.state,
                    &query.role,
                    query.limit,
                    &rows
                ))
                .map_err(|_| InspectionFailure::Unavailable)?
            )
        );
        if query.offset > 0 && query.expected_fingerprint.is_none()
            || query
                .expected_fingerprint
                .as_ref()
                .is_some_and(|f| *f != fingerprint)
        {
            return Err(InspectionFailure::Changed);
        }
        if let Some(id) = &query.identity {
            rows.retain(|r| r.identity == *id);
        }
        if query.identity.is_some() && rows.is_empty() {
            return Err(InspectionFailure::NotFound);
        }
        if query.offset > rows.len() {
            return Err(InspectionFailure::Invalid);
        }
        let count = rows.len();
        let end = query.offset.saturating_add(query.limit).min(count);
        let mut selected = rows[query.offset..end].to_vec();
        for row in &mut selected {
            row.next_relationship_offset = (row.relationships.len() > 8).then_some(8);
            row.relationships.truncate(8);
        }
        Ok(InspectionPage {
            project_id: project.to_string(),
            project_name: name,
            rows: selected,
            fingerprint,
            offset: query.offset,
            next_offset: (end < count).then_some(end),
            permitted_match_count: count,
            omitted_match_count: count - end,
            read_only: true,
        })
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
        // Hold coordination through both basis reads. No mutation can interleave.
        let _read = self.layout().acquire_health_lock()?;
        let mut page = self.inspect_project_records_unlocked(project, query)?;
        let (_, rows) = self.inspection_rows(project, query.class)?;
        let row = rows
            .into_iter()
            .find(|r| Some(&r.identity) == query.identity.as_ref())
            .ok_or(InspectionFailure::NotFound)?;
        if offset > row.relationships.len() {
            return Err(InspectionFailure::Invalid);
        }
        let end = offset.saturating_add(8).min(row.relationships.len());
        page.rows[0].relationships = row.relationships[offset..end].to_vec();
        page.rows[0].next_relationship_offset = (end < row.relationships.len()).then_some(end);
        Ok(page)
    }

    fn inspection_rows(
        &self,
        project: ProjectId,
        class: InspectionClass,
    ) -> Result<(String, Vec<InspectionRow>), InspectionFailure> {
        let basis = self.canonical_basis(project)?;
        let mut rows = Vec::new();
        for item in
            volicord_projections::build_canonical_inspection(&basis, usize::MAX, &mut Vec::new())
        {
            let c = match item.kind {
                CanonicalInspectionKind::Project => continue,
                CanonicalInspectionKind::ContextItem
                    if item.statement_role.as_deref() == Some("goal") =>
                {
                    InspectionClass::Work
                }
                CanonicalInspectionKind::ContextItem => InspectionClass::Context,
                CanonicalInspectionKind::Checkpoint => InspectionClass::Checkpoint,
                CanonicalInspectionKind::Decision => InspectionClass::Decision,
                CanonicalInspectionKind::Question => InspectionClass::Question,
                CanonicalInspectionKind::Source => InspectionClass::Source,
            };
            let mut summary = item.summary;
            if c == InspectionClass::Checkpoint {
                if let Some(cp) = basis
                    .checkpoint_history
                    .iter()
                    .chain(basis.latest_checkpoint.iter())
                    .find(|cp| cp.id.to_string() == item.identity)
                {
                    if let Some(change) = &cp.state_change {
                        summary = change.clone();
                    }
                }
            }
            if c == InspectionClass::Decision {
                if let Some(d) = basis
                    .active_decisions
                    .iter()
                    .chain(&basis.superseded_decisions)
                    .find(|d| d.decision.id.to_string() == item.identity)
                {
                    if let Some(q) = basis
                        .active_questions
                        .iter()
                        .chain(&basis.terminal_question_history)
                        .find(|q| q.id == d.decision.question_id)
                    {
                        summary = format!("{} · {}", q.prompt_basis, summary);
                    }
                }
            }
            let mut disclosure = "permitted_canonical_quotation".to_owned();
            if c == InspectionClass::Source {
                let s = basis
                    .sources
                    .iter()
                    .find(|s| s.source.id.to_string() == item.identity)
                    .ok_or(InspectionFailure::Unavailable)?;
                summary = match &s.source.payload {
                    SourcePayload::CurrentHostUserTurn { .. } => {
                        "User input · body withheld".into()
                    }
                    SourcePayload::CommandExecution {
                        command_label,
                        outcome,
                        ..
                    } => format!("Command {command_label} · exit {:?}", outcome.exit_code),
                    SourcePayload::File { locator, .. } => format!("File observation · {locator}"),
                    SourcePayload::Symbol { locator, .. } => {
                        format!("Symbol observation · {locator}")
                    }
                    SourcePayload::RepositorySnapshot { .. } => "Repository snapshot".into(),
                    SourcePayload::RepositoryCommit { .. } => "Repository commit".into(),
                    SourcePayload::Url { .. } => "URL reference · body not retained".into(),
                    SourcePayload::AdoptedArtifact { .. } => "Adopted artifact reference".into(),
                };
                disclosure = "metadata_only_body_requires_exact_scoped_reader".into();
            }
            let mut row = new_row(
                c,
                item.identity,
                Some(item.revision),
                item.statement_role.unwrap_or_default(),
                item.lifecycle_state,
                summary,
                disclosure,
            );
            if c == InspectionClass::Work {
                if let Some(cp) = basis
                    .checkpoint_history
                    .iter()
                    .chain(basis.latest_checkpoint.iter())
                    .filter(|cp| {
                        cp.work_item_id
                            .is_some_and(|id| id.to_string() == row.identity)
                    })
                    .max_by_key(|cp| (cp.recorded_at, cp.id))
                {
                    row.state = match cp.work_state {
                        volicord_context::WorkState::InProgress => "in_progress",
                        volicord_context::WorkState::Paused => "paused",
                        volicord_context::WorkState::Completed => "completed",
                        volicord_context::WorkState::Abandoned => "abandoned",
                        volicord_context::WorkState::Superseded => "superseded",
                    }
                    .into();
                }
            }
            row.fields = crate::evidence::canonical_fields(c.evidence_kind().unwrap_or_default())
                .iter()
                .map(|s| (*s).into())
                .collect();
            for source in item.source_basis {
                if c != InspectionClass::Source {
                    link(
                        &mut row,
                        "supported_by",
                        InspectionClass::Source,
                        source.to_string(),
                    );
                }
            }
            rows.push(row);
        }
        // Only explicit canonical relations; endpoint admission below rejects foreign IDs.
        for cp in basis
            .checkpoint_history
            .iter()
            .chain(basis.latest_checkpoint.iter())
        {
            if let Some(row) = rows
                .iter_mut()
                .find(|r| r.class == InspectionClass::Checkpoint && r.identity == cp.id.to_string())
            {
                if let Some(work) = cp.work_item_id {
                    link(
                        row,
                        "records_state_of",
                        InspectionClass::Work,
                        work.to_string(),
                    );
                }
                for d in &cp.applied_decisions {
                    link(
                        row,
                        "applied_decision",
                        InspectionClass::Decision,
                        d.to_string(),
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
                    link(
                        row,
                        "observed_source",
                        InspectionClass::Source,
                        s.to_string(),
                    );
                }
                for q in &cp.open_questions {
                    link(
                        row,
                        "open_question",
                        InspectionClass::Question,
                        q.question_id.to_string(),
                    );
                }
            }
        }
        for d in basis
            .active_decisions
            .iter()
            .chain(&basis.superseded_decisions)
        {
            if let Some(row) = rows.iter_mut().find(|r| {
                r.class == InspectionClass::Decision && r.identity == d.decision.id.to_string()
            }) {
                link(
                    row,
                    "answers",
                    InspectionClass::Question,
                    d.decision.question_id.to_string(),
                );
                if let DecisionWorkScope::WorkItem(work) = d.decision.work_scope {
                    link(row, "applies_to", InspectionClass::Work, work.to_string());
                }
                if let Some(id) = d.superseded_by {
                    link(
                        row,
                        "superseded_by",
                        InspectionClass::Decision,
                        id.to_string(),
                    );
                }
            }
        }
        let now = volicord_context::Clock::now(&mut volicord_context::SystemClock)
            .map_err(|_| InspectionFailure::Unavailable)?;
        if class == InspectionClass::Candidate {
            let candidates = self.candidate_basis(project)?;
            for c in &candidates.candidates {
                let i = inspect_candidate(
                    &candidates,
                    c.id,
                    CandidateContentAccess::AllowBoundedSummary,
                    now,
                );
                let state = match i.content_omission {
                    Some(volicord_projections::CandidateContentOmission::RetentionExpired) => {
                        "expired"
                    }
                    Some(volicord_projections::CandidateContentOmission::RetentionCleaned) => {
                        "deleted"
                    }
                    Some(_) => "withheld",
                    None => match c.disposition {
                        volicord_inquiry::CandidateDisposition::PendingOrRetained => "pending",
                        volicord_inquiry::CandidateDisposition::Promoted { .. } => "promoted",
                        volicord_inquiry::CandidateDisposition::Dismissed { .. } => "dismissed",
                        volicord_inquiry::CandidateDisposition::ExpiredOrRetentionCleaned => {
                            "expired"
                        }
                    },
                }
                .to_owned();
                let mut row = new_row(
                    class,
                    c.id.to_string(),
                    i.revision,
                    "candidate_metadata".into(),
                    state,
                    i.bounded_summary
                        .unwrap_or_else(|| format!("{:?} · content withheld", c.kind)),
                    i.content_omission.as_ref().map_or_else(
                        || "permitted_bounded_candidate_summary".into(),
                        |o| format!("{o:?}"),
                    ),
                );
                if i.content_omission.is_none() {
                    row.fields = [
                        "summary",
                        "engineering_choice_discovery",
                        "materiality_review",
                        "learning_deliberation",
                        "repository_research_basis",
                        "source_status",
                    ]
                    .iter()
                    .map(|s| (*s).into())
                    .collect();
                    for source in &c.observation_basis.source_basis {
                        link(
                            &mut row,
                            "derived_from",
                            InspectionClass::Source,
                            source.to_string(),
                        );
                    }
                    let work = i
                        .learning_deliberation
                        .map(|d| d.goal_context_id)
                        .or_else(|| i.materiality_review.map(|r| r.goal_context_id))
                        .or_else(|| i.engineering_choice_discovery.map(|d| d.goal_context_id));
                    if let Some(work) = work {
                        link(
                            &mut row,
                            "candidate_for",
                            InspectionClass::Work,
                            work.to_string(),
                        );
                    }
                }
                rows.push(row);
            }
        }
        if class == InspectionClass::Derived {
            let privacy = self.privacy_status(project)?;
            for d in privacy.managed_derived {
                let state = if privacy.withheld_for_canonical_forgetting.contains(&d.id) {
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
                let mut row = new_row(
                    class,
                    d.id.to_string(),
                    None,
                    "derived_metadata".into(),
                    state.clone(),
                    format!("{:?} · {state}", d.kind),
                    "metadata_only_no_general_body_reader".into(),
                );
                if !["withheld", "deleted", "expired"].contains(&state.as_str()) {
                    for l in d.canonical_links {
                        use volicord_privacy::ManagedCanonicalLink::*;
                        let (c, id) = match l {
                            Project(_) => continue,
                            Source(id) => (InspectionClass::Source, id.to_string()),
                            Question(id) => (InspectionClass::Question, id.to_string()),
                            Decision(id) => (InspectionClass::Decision, id.to_string()),
                            ContextItem(id) => (
                                if basis
                                    .context_items
                                    .iter()
                                    .any(|i| i.id == id && i.role == ContextItemRole::Goal)
                                {
                                    InspectionClass::Work
                                } else {
                                    InspectionClass::Context
                                },
                                id.to_string(),
                            ),
                            Checkpoint(id) => (InspectionClass::Checkpoint, id.to_string()),
                        };
                        link(&mut row, "derived_from", c, id);
                    }
                }
                rows.push(row);
            }
        }
        let endpoints = rows
            .iter()
            .map(|r| ((r.class, r.identity.clone()), r.revision))
            .collect::<std::collections::BTreeMap<_, _>>();
        for row in &mut rows {
            row.relationships.retain_mut(|r| {
                if let Some(revision) = endpoints.get(&(r.class, r.identity.clone())) {
                    r.revision = *revision;
                    true
                } else {
                    row.relationships_omitted = true;
                    false
                }
            });
            row.relationships.sort();
            row.relationships.dedup();
        }
        // Reverse only admitted explicit edges, useful for Work/Source drill-down.
        let edges = rows
            .iter()
            .flat_map(|r| {
                r.relationships
                    .iter()
                    .map(|l| (r.class, r.identity.clone(), r.revision, l.clone()))
            })
            .collect::<Vec<_>>();
        for (class, id, revision, l) in edges {
            if let Some(target) = rows
                .iter_mut()
                .find(|r| r.class == l.class && r.identity == l.identity)
            {
                target.relationships.push(InspectionRelation {
                    relation: format!("referenced_by_{}", l.relation),
                    class,
                    identity: id,
                    revision,
                });
            }
        }
        for row in &mut rows {
            row.relationships.sort();
            row.relationships.dedup();
        }
        Ok((basis.project.display_name, rows))
    }
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
        relationships: Vec::new(),
        next_relationship_offset: None,
        relationships_omitted: false,
    }
}
fn link(row: &mut InspectionRow, relation: &str, class: InspectionClass, identity: String) {
    row.relationships.push(InspectionRelation {
        relation: relation.into(),
        class,
        identity,
        revision: None,
    });
}
