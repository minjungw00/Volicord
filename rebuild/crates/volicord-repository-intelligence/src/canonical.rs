use crate::{
    AnalysisSnapshot, CanonicalCheckpointRef, CanonicalContextItemRef, CanonicalDecisionRef,
    CanonicalProjectRef, CanonicalReference, CanonicalSourceBasis, CanonicalSourceRef,
    RepositorySnapshot,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::fmt;
use volicord_context::{
    CanonicalReadBasis, CanonicalRecordKind, CheckpointId, ContextItemId, DecisionId, ProjectId,
    SourceId, SourcePayload,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalGroundingIssueKind {
    WrongProject,
    DanglingTarget,
    RevisionMismatch,
    SourceBasisMismatch,
    InvalidRepositorySource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalGroundingIssue {
    pub kind: CanonicalGroundingIssueKind,
    pub target_kind: &'static str,
    pub target_identity: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalGroundingError {
    issues: Vec<CanonicalGroundingIssue>,
}

impl CanonicalGroundingError {
    fn one(issue: CanonicalGroundingIssue) -> Self {
        Self {
            issues: vec![issue],
        }
    }

    pub fn issues(&self) -> &[CanonicalGroundingIssue] {
        &self.issues
    }
}

impl fmt::Display for CanonicalGroundingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.issues.len() == 1 {
            return formatter.write_str(&self.issues[0].message);
        }
        write!(
            formatter,
            "{} canonical grounding issues were detected",
            self.issues.len()
        )
    }
}

impl StdError for CanonicalGroundingError {}

#[derive(Clone, Debug)]
struct SourceGrounding {
    basis: CanonicalSourceBasis,
    is_repository_snapshot: bool,
}

/// An immutable identity-and-basis index derived from Canonical Context's
/// authoritative read model. It contains no canonical content or write handle.
#[derive(Clone, Debug)]
pub struct CanonicalGrounding {
    project: ProjectId,
    sources: BTreeMap<SourceId, SourceGrounding>,
    revisions: BTreeMap<(CanonicalRecordKind, String), BTreeSet<u64>>,
}

impl CanonicalGrounding {
    pub fn from_read_basis(basis: &CanonicalReadBasis) -> Result<Self, CanonicalGroundingError> {
        let project = basis.project.id;
        let mut sources = BTreeMap::new();
        for source in &basis.sources {
            if source.source.project_id != project {
                return Err(CanonicalGroundingError::one(CanonicalGroundingIssue {
                    kind: CanonicalGroundingIssueKind::WrongProject,
                    target_kind: "source",
                    target_identity: source.source.id.to_string(),
                    message: format!(
                        "canonical Source {} does not belong to Project {}",
                        source.source.id, project
                    ),
                }));
            }
            let source_grounding = SourceGrounding {
                basis: source.snapshot_basis.clone().map_or(
                    CanonicalSourceBasis::NotApplicable,
                    CanonicalSourceBasis::Snapshot,
                ),
                is_repository_snapshot: matches!(
                    source.source.payload,
                    SourcePayload::RepositorySnapshot { .. }
                ),
            };
            if let Some(previous) = sources.insert(source.source.id, source_grounding.clone()) {
                if previous.basis != source_grounding.basis
                    || previous.is_repository_snapshot != source_grounding.is_repository_snapshot
                {
                    return Err(CanonicalGroundingError::one(CanonicalGroundingIssue {
                        kind: CanonicalGroundingIssueKind::SourceBasisMismatch,
                        target_kind: "source",
                        target_identity: source.source.id.to_string(),
                        message: format!(
                            "canonical Source {} has conflicting read bases",
                            source.source.id
                        ),
                    }));
                }
            }
        }
        let revisions = basis
            .revisions
            .iter()
            .map(|record| {
                (
                    (record.record_kind, record.record_identity.clone()),
                    record.revisions.iter().copied().collect(),
                )
            })
            .collect();
        Ok(Self {
            project,
            sources,
            revisions,
        })
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project
    }

    pub fn project_reference(&self) -> CanonicalProjectRef {
        CanonicalProjectRef::new(self.project)
    }

    pub fn source_reference(
        &self,
        identity: SourceId,
    ) -> Result<CanonicalSourceRef, CanonicalGroundingError> {
        let source = self.source(identity)?;
        Ok(CanonicalSourceRef::new(
            self.project,
            identity,
            source.basis.clone(),
        ))
    }

    pub fn source_reference_at(
        &self,
        identity: SourceId,
        basis: CanonicalSourceBasis,
    ) -> Result<CanonicalSourceRef, CanonicalGroundingError> {
        let reference = CanonicalSourceRef::new(self.project, identity, basis);
        self.validate_source(&reference, false)?;
        Ok(reference)
    }

    pub fn decision_reference(
        &self,
        identity: DecisionId,
        revision: u64,
    ) -> Result<CanonicalDecisionRef, CanonicalGroundingError> {
        let reference = CanonicalDecisionRef::new(self.project, identity, revision);
        self.validate_revisioned(
            "decision",
            CanonicalRecordKind::Decision,
            identity.to_string(),
            self.project,
            revision,
        )?;
        Ok(reference)
    }

    pub fn context_item_reference(
        &self,
        identity: ContextItemId,
        revision: u64,
    ) -> Result<CanonicalContextItemRef, CanonicalGroundingError> {
        let reference = CanonicalContextItemRef::new(self.project, identity, revision);
        self.validate_revisioned(
            "context_item",
            CanonicalRecordKind::ContextItem,
            identity.to_string(),
            self.project,
            revision,
        )?;
        Ok(reference)
    }

    pub fn checkpoint_reference(
        &self,
        identity: CheckpointId,
        revision: u64,
    ) -> Result<CanonicalCheckpointRef, CanonicalGroundingError> {
        let reference = CanonicalCheckpointRef::new(self.project, identity, revision);
        self.validate_revisioned(
            "checkpoint",
            CanonicalRecordKind::Checkpoint,
            identity.to_string(),
            self.project,
            revision,
        )?;
        Ok(reference)
    }

    pub fn validate_reference(
        &self,
        reference: &CanonicalReference,
    ) -> Result<(), CanonicalGroundingError> {
        match reference {
            CanonicalReference::Source(reference) => self.validate_source(reference, false),
            CanonicalReference::Decision(reference) => self.validate_revisioned(
                "decision",
                CanonicalRecordKind::Decision,
                reference.identity().to_string(),
                reference.project(),
                reference.revision(),
            ),
            CanonicalReference::ContextItem(reference) => self.validate_revisioned(
                "context_item",
                CanonicalRecordKind::ContextItem,
                reference.identity().to_string(),
                reference.project(),
                reference.revision(),
            ),
            CanonicalReference::Checkpoint(reference) => self.validate_revisioned(
                "checkpoint",
                CanonicalRecordKind::Checkpoint,
                reference.identity().to_string(),
                reference.project(),
                reference.revision(),
            ),
        }
    }

    pub fn validate_repository_snapshot(
        &self,
        snapshot: &RepositorySnapshot,
    ) -> Result<(), CanonicalGroundingError> {
        let mut issues = Vec::new();
        self.collect_project(snapshot.project, &mut issues);
        self.collect_source(&snapshot.repository_source, true, &mut issues);
        finish(issues)
    }

    pub fn validate_analysis_snapshot(
        &self,
        analysis: &AnalysisSnapshot,
    ) -> Result<(), CanonicalGroundingError> {
        let mut issues = Vec::new();
        self.collect_project(analysis.project, &mut issues);
        self.collect_source(&analysis.repository_source, true, &mut issues);
        for fact in &analysis.structural_facts {
            self.collect_source(&fact.entity.source, false, &mut issues);
            if let Some(range) = &fact.entity.source_range {
                self.collect_source(&range.source, false, &mut issues);
            }
            for extension in &fact.entity.extensions {
                if let Some(range) = &extension.source_range {
                    self.collect_source(&range.source, false, &mut issues);
                }
            }
            for reference in &fact.entity.canonical_links {
                self.collect_reference(reference, &mut issues);
            }
            for relation in &fact.relations {
                if let Some(range) = &relation.supporting_range {
                    self.collect_source(&range.source, false, &mut issues);
                }
                for extension in &relation.extensions {
                    if let Some(range) = &extension.source_range {
                        self.collect_source(&range.source, false, &mut issues);
                    }
                }
            }
            for source in &fact.provenance.analysis.source_basis {
                self.collect_source(source, false, &mut issues);
            }
        }
        for result in &analysis.semantic_results {
            if let Some(range) = &result.relation.supporting_range {
                self.collect_source(&range.source, false, &mut issues);
            }
            for extension in &result.relation.extensions {
                if let Some(range) = &extension.source_range {
                    self.collect_source(&range.source, false, &mut issues);
                }
            }
            for source in &result.provenance.analysis.source_basis {
                self.collect_source(source, false, &mut issues);
            }
        }
        for annotation in &analysis.semantic_annotations {
            for source in &annotation.included_sources {
                self.collect_source(source, false, &mut issues);
            }
        }
        for interpretation in &analysis.agent_interpretations {
            for source in &interpretation.source_basis {
                self.collect_source(source, false, &mut issues);
            }
        }
        finish(issues)
    }

    pub(crate) fn repository_source_reference(
        &self,
        identity: SourceId,
    ) -> Result<CanonicalSourceRef, CanonicalGroundingError> {
        let reference = self.source_reference(identity)?;
        self.validate_source(&reference, true)?;
        Ok(reference)
    }

    fn source(&self, identity: SourceId) -> Result<&SourceGrounding, CanonicalGroundingError> {
        self.sources.get(&identity).ok_or_else(|| {
            CanonicalGroundingError::one(CanonicalGroundingIssue {
                kind: CanonicalGroundingIssueKind::DanglingTarget,
                target_kind: "source",
                target_identity: identity.to_string(),
                message: format!(
                    "canonical Source {identity} does not exist in Project {}",
                    self.project
                ),
            })
        })
    }

    fn validate_source(
        &self,
        reference: &CanonicalSourceRef,
        require_repository_snapshot: bool,
    ) -> Result<(), CanonicalGroundingError> {
        let mut issues = Vec::new();
        self.collect_source(reference, require_repository_snapshot, &mut issues);
        finish(issues)
    }

    fn collect_project(
        &self,
        reference: CanonicalProjectRef,
        issues: &mut Vec<CanonicalGroundingIssue>,
    ) {
        if reference.identity() != self.project {
            issues.push(CanonicalGroundingIssue {
                kind: CanonicalGroundingIssueKind::WrongProject,
                target_kind: "project",
                target_identity: reference.identity().to_string(),
                message: format!(
                    "canonical Project {} does not match analysis Project {}",
                    reference.identity(),
                    self.project
                ),
            });
        }
    }

    fn collect_source(
        &self,
        reference: &CanonicalSourceRef,
        require_repository_snapshot: bool,
        issues: &mut Vec<CanonicalGroundingIssue>,
    ) {
        if reference.project() != self.project {
            issues.push(CanonicalGroundingIssue {
                kind: CanonicalGroundingIssueKind::WrongProject,
                target_kind: "source",
                target_identity: reference.identity().to_string(),
                message: format!(
                    "canonical Source {} belongs to Project {}, not analysis Project {}",
                    reference.identity(),
                    reference.project(),
                    self.project
                ),
            });
            return;
        }
        let Some(source) = self.sources.get(&reference.identity()) else {
            issues.push(CanonicalGroundingIssue {
                kind: CanonicalGroundingIssueKind::DanglingTarget,
                target_kind: "source",
                target_identity: reference.identity().to_string(),
                message: format!(
                    "canonical Source {} does not exist in Project {}",
                    reference.identity(),
                    self.project
                ),
            });
            return;
        };
        if source.basis != *reference.basis() {
            issues.push(CanonicalGroundingIssue {
                kind: CanonicalGroundingIssueKind::SourceBasisMismatch,
                target_kind: "source",
                target_identity: reference.identity().to_string(),
                message: format!(
                    "canonical Source {} does not exist at the claimed snapshot basis",
                    reference.identity()
                ),
            });
        }
        if require_repository_snapshot && !source.is_repository_snapshot {
            issues.push(CanonicalGroundingIssue {
                kind: CanonicalGroundingIssueKind::InvalidRepositorySource,
                target_kind: "source",
                target_identity: reference.identity().to_string(),
                message: format!(
                    "canonical Source {} is not a repository snapshot Source",
                    reference.identity()
                ),
            });
        }
    }

    fn validate_revisioned(
        &self,
        target_kind: &'static str,
        record_kind: CanonicalRecordKind,
        identity: String,
        project: ProjectId,
        revision: u64,
    ) -> Result<(), CanonicalGroundingError> {
        let mut issues = Vec::new();
        if project != self.project {
            issues.push(CanonicalGroundingIssue {
                kind: CanonicalGroundingIssueKind::WrongProject,
                target_kind,
                target_identity: identity.clone(),
                message: format!(
                    "canonical {target_kind} {identity} belongs to Project {project}, not analysis Project {}",
                    self.project
                ),
            });
        } else {
            match self.revisions.get(&(record_kind, identity.clone())) {
                None => issues.push(CanonicalGroundingIssue {
                    kind: CanonicalGroundingIssueKind::DanglingTarget,
                    target_kind,
                    target_identity: identity.clone(),
                    message: format!(
                        "canonical {target_kind} {identity} does not exist in Project {}",
                        self.project
                    ),
                }),
                Some(revisions) if !revisions.contains(&revision) => {
                    issues.push(CanonicalGroundingIssue {
                        kind: CanonicalGroundingIssueKind::RevisionMismatch,
                        target_kind,
                        target_identity: identity.clone(),
                        message: format!(
                            "canonical {target_kind} {identity} does not have revision {revision}"
                        ),
                    });
                }
                Some(_) => {}
            }
        }
        finish(issues)
    }

    fn collect_reference(
        &self,
        reference: &CanonicalReference,
        issues: &mut Vec<CanonicalGroundingIssue>,
    ) {
        if let Err(error) = self.validate_reference(reference) {
            issues.extend(error.issues);
        }
    }
}

fn finish(issues: Vec<CanonicalGroundingIssue>) -> Result<(), CanonicalGroundingError> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(CanonicalGroundingError { issues })
    }
}

#[cfg(test)]
pub(crate) fn test_repository_grounding(
    project: ProjectId,
    source: SourceId,
) -> Result<CanonicalGrounding, Box<dyn StdError>> {
    use tempfile::tempdir;
    use volicord_context::{
        Availability, CanonicalReadOptions, DeterministicIdGenerator, FixedClock, OperationId,
        Principal, PrincipalKind, SourceDraft, Store, TimestampMicros,
    };

    let runtime = tempdir()?;
    let mut store = Store::open_with(
        runtime.path().join("context.sqlite3"),
        DeterministicIdGenerator::new([*project.as_bytes(), *source.as_bytes()]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_725_000_000_000_000)),
    )?;
    let project = store
        .create_project(OperationId::from_bytes([0xe1; 16]), "Unit fixture")?
        .value;
    store.record_source(
        OperationId::from_bytes([0xe2; 16]),
        project.id,
        SourceDraft {
            expected_project_revision: project.revision,
            payload: SourcePayload::RepositorySnapshot {
                revision: "unit-repository-snapshot".to_owned(),
            },
            actor: Principal {
                kind: PrincipalKind::Repository,
                identity: "unit-fixture".to_owned(),
            },
            observer: None,
            availability: Availability::Available,
        },
    )?;
    let basis = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;
    Ok(CanonicalGrounding::from_read_basis(&basis)?)
}
