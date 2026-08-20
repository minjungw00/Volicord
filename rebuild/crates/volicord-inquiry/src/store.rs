use crate::{
    CandidateCleanup, CandidateCleanupKind, CandidateCollectionMode, CandidateDisposition,
    CandidateDraft, CandidateId, CandidateKind, CandidateReadBasis, CandidateRecord,
    CollectionOptOut, CollectionOptOutScope, DuplicateAssessment, Error, ErrorKind,
    MaterialityAssessment, MaterialityStatus, PromotionResult, RepositoryResearchBasis,
    SubmissionOutcome,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::collections::BTreeSet;
use std::path::Path;
use volicord_context::{
    CanonicalInvalidation, CanonicalReadBasis, CanonicalRecordId, Clock, IdGenerator, OperationId,
    OperationResult, ProjectId, Question, QuestionDispositionDraft, QuestionDraft,
    QuestionMateriality, QuestionResearchState, SourceId, Store as ContextStore, SystemClock,
    SystemIdGenerator,
};
use volicord_repository_intelligence::AnalysisSnapshot;

pub const CANDIDATE_SCHEMA_KIND: &str = "volicord-inquiry-candidates";
pub const CANDIDATE_SCHEMA_VERSION: u32 = 2;

const MAX_TEXT_BYTES: usize = 4_096;
const MAX_LIST_ITEMS: usize = 64;
const MAX_RECORD_BYTES: usize = 131_072;

pub struct CandidateStore {
    connection: Connection,
    ids: Box<dyn IdGenerator>,
    clock: Box<dyn Clock>,
}

impl CandidateStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::open_with(path, SystemIdGenerator, SystemClock)
    }

    pub fn open_with(
        path: impl AsRef<Path>,
        ids: impl IdGenerator + 'static,
        clock: impl Clock + 'static,
    ) -> Result<Self, Error> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Candidate store path must be explicit",
            ));
        }
        let mut connection = Connection::open(path).map_err(open_error)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 PRAGMA secure_delete = ON;",
            )
            .map_err(write_error)?;
        initialize_or_validate(&mut connection)?;
        Ok(Self {
            connection,
            ids: Box::new(ids),
            clock: Box::new(clock),
        })
    }

    pub fn submit(&mut self, draft: CandidateDraft) -> Result<SubmissionOutcome, Error> {
        validate_candidate_draft(&draft)?;
        let applicable_policies = self.applicable_policies(&draft.collection_scope)?;
        let matching_scopes = applicable_policies
            .iter()
            .filter(|policy| policy.opted_out)
            .cloned()
            .collect::<Vec<_>>();
        if draft.collection_mode == CandidateCollectionMode::Automatic
            && !matching_scopes.is_empty()
        {
            return Ok(SubmissionOutcome::CollectionDisabled { matching_scopes });
        }

        let id = CandidateId::from_bytes(self.ids.next_id().map_err(id_error)?);
        let now = self.clock.now().map_err(clock_error)?;
        let record = CandidateRecord {
            id,
            project_id: draft.project_id,
            revision: 1,
            kind: draft.kind,
            collection_mode: draft.collection_mode,
            origin: draft.origin,
            collection_scope: draft.collection_scope,
            observation_basis: draft.observation_basis,
            created_at: now,
            observed_at: draft.observed_at,
            retention: draft.retention,
            disposition: CandidateDisposition::PendingOrRetained,
            cleanup: None,
            promotion_target: None,
            opt_out_state_at_collection: applicable_policies,
            content: Some(draft.content),
        };
        let encoded = encode_record(&record)?;
        self.connection
            .execute(
                "INSERT INTO candidates(id, project_id, revision, record_json, created_at)
                 VALUES (?1, ?2, 1, ?3, ?4)",
                params![
                    id.as_bytes().as_slice(),
                    record.project_id.as_bytes().as_slice(),
                    encoded,
                    now.as_unix_micros(),
                ],
            )
            .map_err(write_error)?;
        Ok(SubmissionOutcome::Stored(Box::new(record)))
    }

    pub fn get(
        &self,
        project_id: ProjectId,
        candidate_id: CandidateId,
    ) -> Result<CandidateRecord, Error> {
        load_record(&self.connection, project_id, candidate_id)
    }

    pub fn read_basis(&self, project_id: ProjectId) -> Result<CandidateReadBasis, Error> {
        let mut statement = self
            .connection
            .prepare("SELECT record_json FROM candidates WHERE project_id = ?1 ORDER BY id")
            .map_err(read_error)?;
        let rows = statement
            .query_map([project_id.as_bytes().as_slice()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(read_error)?;
        let mut candidates = Vec::new();
        for row in rows {
            candidates.push(decode_record(&row.map_err(read_error)?)?);
        }
        let policies = self.read_policies(project_id)?;
        Ok(CandidateReadBasis {
            project_id,
            candidates,
            collection_policies: policies,
            withheld_for_canonical_forgetting: Vec::new(),
        })
    }

    pub fn read_basis_with_invalidations(
        &self,
        project_id: ProjectId,
        invalidations: &[CanonicalInvalidation],
    ) -> Result<CandidateReadBasis, Error> {
        let mut basis = self.read_basis(project_id)?;
        for candidate in &mut basis.candidates {
            if invalidations.iter().any(|invalidation| {
                invalidation.project_id == project_id
                    && candidate_refers_to(candidate, invalidation.record)
            }) {
                candidate.content = None;
                basis.withheld_for_canonical_forgetting.push(candidate.id);
            }
        }
        basis.withheld_for_canonical_forgetting.sort();
        basis.withheld_for_canonical_forgetting.dedup();
        Ok(basis)
    }

    pub fn set_collection_opt_out(
        &mut self,
        scope: CollectionOptOutScope,
        opted_out: bool,
        basis: impl Into<String>,
    ) -> Result<CollectionOptOut, Error> {
        validate_scope(&scope)?;
        let basis = basis.into();
        validate_text("collection policy basis", &basis)?;
        let effective_at = self.clock.now().map_err(clock_error)?;
        let policy = CollectionOptOut {
            scope,
            opted_out,
            effective_at,
            basis,
        };
        let key = encode_scope(&policy.scope)?;
        let value = serde_json::to_string(&policy).map_err(encode_error)?;
        self.connection
            .execute(
                "INSERT INTO collection_policies(scope_key, project_id, policy_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(scope_key) DO UPDATE SET policy_json = excluded.policy_json",
                params![key, policy.scope.project_id.as_bytes().as_slice(), value],
            )
            .map_err(write_error)?;
        Ok(policy)
    }

    pub fn dismiss(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        reason: impl Into<String>,
    ) -> Result<CandidateRecord, Error> {
        let reason = reason.into();
        validate_text("Candidate dismissal reason", &reason)?;
        let now = self.clock.now().map_err(clock_error)?;
        self.mutate_pending(project_id, candidate_id, |record| {
            record.disposition = CandidateDisposition::Dismissed {
                reason,
                dismissed_at: now,
            };
            Ok(())
        })
    }

    pub fn delete_candidate(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        basis: impl Into<String>,
    ) -> Result<CandidateRecord, Error> {
        let basis = basis.into();
        validate_text("Candidate deletion basis", &basis)?;
        let now = self.clock.now().map_err(clock_error)?;
        let (record, _) = self.cleanup_candidate_content(
            project_id,
            candidate_id,
            CandidateCleanup {
                kind: CandidateCleanupKind::ExplicitDeletion,
                basis,
                cleaned_at: now,
            },
        )?;
        Ok(record)
    }

    pub fn cleanup_expired(&mut self, project_id: ProjectId) -> Result<Vec<CandidateId>, Error> {
        let now = self.clock.now().map_err(clock_error)?;
        let basis = self.read_basis(project_id)?;
        let mut cleaned = Vec::new();
        for candidate in basis.candidates {
            if candidate.content.is_some()
                && candidate.cleanup.is_none()
                && candidate
                    .retention
                    .retained_until
                    .is_some_and(|expiry| expiry <= now)
            {
                let retention_basis = candidate.retention.basis.clone();
                let (_, transitioned) = self.cleanup_candidate_content(
                    project_id,
                    candidate.id,
                    CandidateCleanup {
                        kind: CandidateCleanupKind::RetentionExpiry,
                        basis: retention_basis,
                        cleaned_at: now,
                    },
                )?;
                if transitioned {
                    cleaned.push(candidate.id);
                }
            }
        }
        Ok(cleaned)
    }

    /// Removes content only from Candidates whose recorded basis refers to a
    /// forgotten canonical record. Candidate disposition and promotion target
    /// remain inspectable, and unrelated Candidates are not touched.
    pub fn cleanup_related_to_canonical(
        &mut self,
        invalidation: &CanonicalInvalidation,
        basis: impl Into<String>,
    ) -> Result<Vec<CandidateId>, Error> {
        let basis = basis.into();
        validate_text("canonical forgetting cleanup basis", &basis)?;
        let now = self.clock.now().map_err(clock_error)?;
        let records = self.read_basis(invalidation.project_id)?.candidates;
        let mut cleaned = Vec::new();
        for candidate in records {
            if candidate.content.is_some()
                && candidate.cleanup.is_none()
                && candidate_refers_to(&candidate, invalidation.record)
            {
                let (_, transitioned) = self.cleanup_candidate_content(
                    invalidation.project_id,
                    candidate.id,
                    CandidateCleanup {
                        kind: CandidateCleanupKind::ExplicitDeletion,
                        basis: basis.clone(),
                        cleaned_at: now,
                    },
                )?;
                if transitioned {
                    cleaned.push(candidate.id);
                }
            }
        }
        sanitize_deleted_content(&self.connection)?;
        Ok(cleaned)
    }

    pub fn verify_canonical_forgetting(
        &self,
        invalidation: &CanonicalInvalidation,
    ) -> Result<bool, Error> {
        Ok(self
            .read_basis(invalidation.project_id)?
            .candidates
            .iter()
            .filter(|candidate| candidate_refers_to(candidate, invalidation.record))
            .all(|candidate| candidate.content.is_none()))
    }

    pub fn assess_materiality(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        mut assessment: MaterialityAssessment,
    ) -> Result<CandidateRecord, Error> {
        if assessment.status == MaterialityStatus::Unassessed {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "materiality transition must select an assessed state",
            ));
        }
        let rationale = assessment.rationale.as_deref().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "materiality assessment requires a rationale",
            )
        })?;
        validate_text("materiality rationale", rationale)?;
        if assessment.assessed_by.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "materiality assessment requires an actor",
            ));
        }
        let now = self.clock.now().map_err(clock_error)?;
        assessment.assessed_at = Some(now);
        validate_materiality_assessment(&assessment)?;
        self.mutate_pending(project_id, candidate_id, |record| {
            let question = question_mut(record)?;
            question.materiality = assessment;
            Ok(())
        })
    }

    pub fn attach_repository_research(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        canonical: &CanonicalReadBasis,
        analysis: &AnalysisSnapshot,
        basis: RepositoryResearchBasis,
    ) -> Result<CandidateRecord, Error> {
        let repository_snapshot = analysis.repository_snapshot.to_string();
        let analysis_snapshot = analysis.identity.to_string();
        if canonical.project.id != project_id
            || analysis.project.identity() != project_id
            || basis.repository_snapshot != repository_snapshot
            || basis.analysis_snapshot.as_deref() != Some(analysis_snapshot.as_str())
        {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "research evidence does not match the Candidate Project or Analysis Snapshot",
            ));
        }
        validate_research_basis(&basis)?;
        validate_sources(canonical, &basis.source_basis)?;
        if !basis
            .source_basis
            .contains(&analysis.repository_source.identity())
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "repository research must retain its canonical Repository Source",
            ));
        }
        self.attach_research_basis(project_id, candidate_id, canonical, basis)
    }

    pub fn attach_research_basis(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        canonical: &CanonicalReadBasis,
        basis: RepositoryResearchBasis,
    ) -> Result<CandidateRecord, Error> {
        if canonical.project.id != project_id {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "research canonical basis belongs to a different Project",
            ));
        }
        validate_research_basis(&basis)?;
        validate_sources(canonical, &basis.source_basis)?;
        self.mutate_pending(project_id, candidate_id, |record| {
            let question = question_mut(record)?;
            for source in &basis.source_basis {
                if !question.source_basis.contains(source) {
                    question.source_basis.push(*source);
                }
            }
            question.repository_basis.push(basis);
            Ok(())
        })
    }

    pub fn set_research_state(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        state: QuestionResearchState,
    ) -> Result<CandidateRecord, Error> {
        self.mutate_pending(project_id, candidate_id, |record| {
            let question = question_mut(record)?;
            if state == QuestionResearchState::ReadyToAsk
                && !question
                    .repository_basis
                    .iter()
                    .any(|basis| basis.sufficient)
            {
                return Err(Error::new(
                    ErrorKind::DomainConflict,
                    "Question Candidate cannot leave research without sufficient evidence",
                ));
            }
            question.research_state = state;
            Ok(())
        })
    }

    pub fn promote_question(
        &mut self,
        context: &mut ContextStore,
        canonical: &CanonicalReadBasis,
        project_id: ProjectId,
        candidate_id: CandidateId,
    ) -> Result<PromotionResult, Error> {
        let candidate = self.get(project_id, candidate_id)?;
        if let Some(canonical_question_id) = candidate.promotion_target {
            let _ = context
                .get_question(project_id, canonical_question_id)
                .map_err(canonical_error)?;
            return Ok(PromotionResult {
                candidate_id,
                question_id: canonical_question_id,
                canonical_replayed: true,
                candidate_reconciled: true,
            });
        }
        if let CandidateDisposition::Promoted {
            canonical_question_id,
            ..
        } = candidate.disposition
        {
            let _ = context
                .get_question(project_id, canonical_question_id)
                .map_err(canonical_error)?;
            return Ok(PromotionResult {
                candidate_id,
                question_id: canonical_question_id,
                canonical_replayed: true,
                candidate_reconciled: true,
            });
        }
        if candidate.disposition != CandidateDisposition::PendingOrRetained {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "only a pending retained Candidate can be promoted",
            ));
        }
        if candidate.kind != CandidateKind::QuestionCandidate || candidate.project_id != project_id
        {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "Question Candidate promotion Project or kind does not match",
            ));
        }
        if canonical.project.id != project_id {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "canonical read basis belongs to a different Project",
            ));
        }
        let question = candidate
            .content
            .as_ref()
            .and_then(|content| content.question.as_ref())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::CorruptState,
                    "Question Candidate content is missing",
                )
            })?;
        if question.materiality.status != MaterialityStatus::Material {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "Question Candidate is not currently material",
            ));
        }
        if !matches!(
            question.duplicate_assessment,
            DuplicateAssessment::NoDuplicate { .. }
        ) {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "Question Candidate duplicate or supersession assessment does not permit promotion",
            ));
        }
        let presentation_order = question.presentation_order.ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "Question Candidate requires an explicit presentation order",
            )
        })?;
        validate_sources(canonical, &question.source_basis)?;
        validate_sources(canonical, &question.materiality.source_basis)?;
        for dependency in &question.possible_prerequisites {
            validate_sources(canonical, &dependency.assessment_source_basis)?;
            validate_sources(canonical, &dependency.required_source_basis)?;
        }

        let draft = QuestionDraft {
            expected_project_revision: canonical.project.revision,
            prompt_basis: question.prompt_basis.clone(),
            source_basis: question.source_basis.clone(),
            dependencies: question.possible_prerequisites.clone(),
            alternatives: question.alternatives.clone(),
            recommendation: question.recommendation.clone(),
            trade_offs: question.trade_offs.clone(),
            uncertainty: question.uncertainty.clone(),
            material_scope: question.affected_scope.clone(),
            materiality: QuestionMateriality::Material,
            presentation_order,
            why_it_matters_now: question.why_it_matters_now.clone(),
            established_facts: question.known_facts.clone(),
            assumptions: question.assumptions.clone(),
            known_limits: question.known_limits.clone(),
            what_the_answer_unlocks: question.what_the_answer_unlocks.clone(),
            allowed_non_choice_dispositions: question.allowed_non_choice_dispositions.clone(),
            research_state: question.research_state,
        };
        let operation_id = OperationId::from_bytes(*candidate_id.as_bytes());
        let canonical_result = context
            .create_question(operation_id, project_id, draft)
            .map_err(canonical_error)?;
        let question_id = canonical_result.value.id;
        let now = self.clock.now().map_err(clock_error)?;
        self.mutate_pending(project_id, candidate_id, |record| {
            record.promotion_target = Some(question_id);
            record.disposition = CandidateDisposition::Promoted {
                canonical_question_id: question_id,
                promoted_at: now,
            };
            Ok(())
        })?;
        Ok(PromotionResult {
            candidate_id,
            question_id,
            canonical_replayed: canonical_result.replayed,
            candidate_reconciled: true,
        })
    }

    fn mutate_pending(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        mutation: impl FnOnce(&mut CandidateRecord) -> Result<(), Error>,
    ) -> Result<CandidateRecord, Error> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(write_error)?;
        let mut record = load_record(&transaction, project_id, candidate_id)?;
        if record.disposition != CandidateDisposition::PendingOrRetained {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "Candidate is no longer pending or retained",
            ));
        }
        let previous_revision = record.revision;
        mutation(&mut record)?;
        record.revision = record
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorKind::CorruptState, "Candidate revision overflow"))?;
        let encoded = encode_record(&record)?;
        let count = transaction
            .execute(
                "UPDATE candidates SET revision = ?4, record_json = ?5
                 WHERE id = ?1 AND project_id = ?2 AND revision = ?3",
                params![
                    candidate_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    i64::try_from(previous_revision).map_err(|_| Error::new(
                        ErrorKind::CorruptState,
                        "Candidate revision is outside the supported range",
                    ))?,
                    i64::try_from(record.revision).map_err(|_| Error::new(
                        ErrorKind::CorruptState,
                        "Candidate revision is outside the supported range",
                    ))?,
                    encoded,
                ],
            )
            .map_err(write_error)?;
        if count != 1 {
            return Err(Error::new(
                ErrorKind::StaleBasis,
                "Candidate changed concurrently",
            ));
        }
        transaction.commit().map_err(write_error)?;
        Ok(record)
    }

    fn cleanup_candidate_content(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        cleanup: CandidateCleanup,
    ) -> Result<(CandidateRecord, bool), Error> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(write_error)?;
        let mut record = load_record(&transaction, project_id, candidate_id)?;
        if record.content.is_none() || record.cleanup.is_some() {
            return Ok((record, false));
        }
        if record.disposition == CandidateDisposition::PendingOrRetained {
            record.disposition = CandidateDisposition::ExpiredOrRetentionCleaned;
        }
        record.content = None;
        record.cleanup = Some(cleanup);
        let previous_revision = record.revision;
        record.revision = record
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorKind::CorruptState, "Candidate revision overflow"))?;
        let encoded = encode_record(&record)?;
        let count = transaction
            .execute(
                "UPDATE candidates SET revision = ?4, record_json = ?5
                 WHERE id = ?1 AND project_id = ?2 AND revision = ?3",
                params![
                    candidate_id.as_bytes().as_slice(),
                    project_id.as_bytes().as_slice(),
                    i64::try_from(previous_revision).map_err(|_| Error::new(
                        ErrorKind::CorruptState,
                        "Candidate revision is outside the supported range",
                    ))?,
                    i64::try_from(record.revision).map_err(|_| Error::new(
                        ErrorKind::CorruptState,
                        "Candidate revision is outside the supported range",
                    ))?,
                    encoded,
                ],
            )
            .map_err(write_error)?;
        if count != 1 {
            return Err(Error::new(
                ErrorKind::StaleBasis,
                "Candidate changed concurrently",
            ));
        }
        transaction.commit().map_err(write_error)?;
        Ok((record, true))
    }

    fn read_policies(&self, project_id: ProjectId) -> Result<Vec<CollectionOptOut>, Error> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT policy_json FROM collection_policies
                 WHERE project_id = ?1 ORDER BY scope_key",
            )
            .map_err(read_error)?;
        let rows = statement
            .query_map([project_id.as_bytes().as_slice()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(read_error)?;
        let mut policies = Vec::new();
        for row in rows {
            policies.push(serde_json::from_str(&row.map_err(read_error)?).map_err(decode_error)?);
        }
        Ok(policies)
    }

    fn applicable_policies(
        &self,
        scope: &crate::CandidateCollectionScope,
    ) -> Result<Vec<CollectionOptOut>, Error> {
        Ok(self
            .read_policies(scope.project_id)?
            .into_iter()
            .filter(|policy| scope_matches(&policy.scope, scope))
            .collect())
    }
}

fn sanitize_deleted_content(connection: &Connection) -> Result<(), Error> {
    let (busy, _, _): (i64, i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(write_error)?;
    if busy != 0 {
        return Err(Error::new(
            ErrorKind::StorageUnavailable,
            "Candidate content cleanup committed but WAL truncation is busy",
        ));
    }
    connection
        .execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(write_error)
}

fn candidate_refers_to(candidate: &CandidateRecord, record: CanonicalRecordId) -> bool {
    match record {
        CanonicalRecordId::Project(project_id) => candidate.project_id == project_id,
        CanonicalRecordId::Source(source_id) => {
            candidate
                .observation_basis
                .source_basis
                .contains(&source_id)
                || candidate.content.as_ref().is_some_and(|content| {
                    content.question.as_ref().is_some_and(|question| {
                        question.source_basis.contains(&source_id)
                            || question
                                .repository_basis
                                .iter()
                                .any(|basis| basis.source_basis.contains(&source_id))
                    })
                })
        }
        CanonicalRecordId::Question(question_id) => {
            candidate.promotion_target == Some(question_id)
                || candidate.content.as_ref().is_some_and(|content| {
                    content.question.as_ref().is_some_and(|question| {
                        question
                            .possible_prerequisites
                            .iter()
                            .any(|dependency| dependency.question_id == question_id)
                    })
                })
        }
        CanonicalRecordId::Decision(_)
        | CanonicalRecordId::ContextItem(_)
        | CanonicalRecordId::Checkpoint(_) => false,
    }
}

pub fn resolve_question_by_research(
    context: &mut ContextStore,
    canonical: &CanonicalReadBasis,
    operation_id: OperationId,
    project_id: ProjectId,
    draft: QuestionDispositionDraft,
) -> Result<OperationResult<Question>, Error> {
    if canonical.project.id != project_id
        || draft.outcome != volicord_context::NonUserQuestionOutcome::ResolvedByResearch
    {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "research disposition must match the canonical Project and outcome",
        ));
    }
    validate_sources(canonical, &draft.source_basis)?;
    context
        .dispose_question(operation_id, project_id, draft)
        .map_err(canonical_error)
}

fn question_mut(record: &mut CandidateRecord) -> Result<&mut crate::QuestionCandidate, Error> {
    if record.kind != CandidateKind::QuestionCandidate {
        return Err(Error::new(
            ErrorKind::DomainConflict,
            "operation requires a Question Candidate",
        ));
    }
    record
        .content
        .as_mut()
        .and_then(|content| content.question.as_mut())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Question Candidate content is missing",
            )
        })
}

fn scope_matches(
    policy: &CollectionOptOutScope,
    candidate: &crate::CandidateCollectionScope,
) -> bool {
    policy.project_id == candidate.project_id
        && policy
            .session
            .as_ref()
            .is_none_or(|value| candidate.session.as_ref() == Some(value))
        && policy
            .source_operation
            .as_ref()
            .is_none_or(|value| candidate.source_operation.as_ref() == Some(value))
        && policy
            .candidate_kind
            .is_none_or(|value| candidate.candidate_kind == value)
}

fn validate_candidate_draft(draft: &CandidateDraft) -> Result<(), Error> {
    if draft.project_id != draft.collection_scope.project_id
        || draft.kind != draft.collection_scope.candidate_kind
    {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "Candidate Project, kind, and collection scope must match",
        ));
    }
    validate_text("origin subsystem", &draft.origin.subsystem)?;
    validate_text("origin provenance", &draft.origin.provenance_summary)?;
    validate_text("origin actor identity", &draft.origin.actor.identity)?;
    validate_optional_text("origin session", draft.origin.session.as_deref())?;
    validate_optional_text(
        "collection session",
        draft.collection_scope.session.as_deref(),
    )?;
    validate_optional_text(
        "collection source operation",
        draft.collection_scope.source_operation.as_deref(),
    )?;
    validate_id_list(&draft.observation_basis.source_basis)?;
    for (label, value) in [
        (
            "observation repository snapshot",
            draft.observation_basis.repository_snapshot.as_deref(),
        ),
        (
            "observation analysis snapshot",
            draft.observation_basis.analysis_snapshot.as_deref(),
        ),
        (
            "observation execution",
            draft.observation_basis.execution.as_deref(),
        ),
        (
            "observation host turn",
            draft.observation_basis.host_turn.as_deref(),
        ),
        (
            "observation other basis",
            draft.observation_basis.other.as_deref(),
        ),
    ] {
        validate_optional_text(label, value)?;
    }
    validate_text("retention basis", &draft.retention.basis)?;
    validate_text("bounded Candidate summary", &draft.content.bounded_summary)?;
    if (draft.kind == CandidateKind::QuestionCandidate) != draft.content.question.is_some() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question Candidate kind and content must agree",
        ));
    }
    if let Some(question) = &draft.content.question {
        validate_question_candidate(question)?;
    }
    let encoded = serde_json::to_vec(&draft.content).map_err(encode_error)?;
    if encoded.len() > MAX_RECORD_BYTES {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "bounded Candidate content exceeds the retained record limit",
        ));
    }
    Ok(())
}

fn validate_question_candidate(question: &crate::QuestionCandidate) -> Result<(), Error> {
    validate_text("Question Candidate prompt", &question.prompt_basis)?;
    validate_text(
        "Question Candidate material reason",
        &question.why_it_matters_now,
    )?;
    if question.source_basis.is_empty() || question.affected_scope.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question Candidate requires Source and affected-scope basis",
        ));
    }
    if question.alternatives.is_empty() || question.allowed_non_choice_dispositions.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question Candidate requires alternatives and allowed dispositions",
        ));
    }
    validate_id_list(&question.source_basis)?;
    validate_text(
        "Question Candidate recommendation rationale",
        &question.recommendation.rationale,
    )?;
    validate_id_list(&question.recommendation.source_basis)?;
    if question.recommendation.source_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question Candidate recommendation requires a Source basis",
        ));
    }
    let mut alternatives = BTreeSet::new();
    for alternative in &question.alternatives {
        validate_text("Question Candidate alternative key", &alternative.key)?;
        validate_text("Question Candidate alternative label", &alternative.label)?;
        validate_text(
            "Question Candidate alternative consequence",
            &alternative.consequence,
        )?;
        if !alternatives.insert(alternative.key.as_str()) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Question Candidate alternative keys must be unique",
            ));
        }
    }
    if question
        .recommendation
        .alternative_key
        .as_ref()
        .is_some_and(|key| !alternatives.contains(key.as_str()))
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question Candidate recommendation must name an alternative",
        ));
    }
    for fact in &question.known_facts {
        validate_text("Question Candidate established fact", &fact.statement)?;
        validate_id_list(&fact.source_basis)?;
        if fact.source_basis.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Question Candidate established fact requires a Source basis",
            ));
        }
        validate_optional_text(
            "Question Candidate established capability",
            fact.capability.as_deref(),
        )?;
    }
    let mut dependencies = BTreeSet::new();
    for dependency in &question.possible_prerequisites {
        if dependency.required_revision == 0
            || dependency.assessment_source_basis.is_empty()
            || !dependencies.insert((dependency.question_id, dependency.required_revision))
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Question Candidate prerequisite requires a unique explicit revision and assessment basis",
            ));
        }
        validate_id_list(&dependency.required_source_basis)?;
        validate_id_list(&dependency.assessment_source_basis)?;
    }
    for basis in &question.repository_basis {
        validate_research_basis(basis)?;
    }
    let disposition_count = question
        .allowed_non_choice_dispositions
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .len();
    if disposition_count != question.allowed_non_choice_dispositions.len() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Question Candidate allowed dispositions must be unique",
        ));
    }
    match question.materiality.status {
        MaterialityStatus::Unassessed => {
            if question.materiality.rationale.is_some()
                || !question.materiality.source_basis.is_empty()
                || question.materiality.assessed_by.is_some()
                || question.materiality.assessed_at.is_some()
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "unassessed materiality cannot carry assessment provenance",
                ));
            }
        }
        _ => validate_materiality_assessment(&question.materiality)?,
    }
    for list in [
        &question.assumptions,
        &question.uncertainty,
        &question.affected_scope,
        &question.trade_offs,
        &question.known_limits,
        &question.what_the_answer_unlocks,
    ] {
        validate_list(list)?;
    }
    Ok(())
}

fn validate_research_basis(basis: &RepositoryResearchBasis) -> Result<(), Error> {
    validate_text("repository snapshot", &basis.repository_snapshot)?;
    validate_text("research capability", &basis.capability)?;
    validate_text("research coverage", &basis.coverage)?;
    if basis.source_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "research evidence requires canonical Source basis",
        ));
    }
    validate_id_list(&basis.source_basis)?;
    validate_optional_text("analysis snapshot", basis.analysis_snapshot.as_deref())?;
    validate_list(&basis.limits)
}

fn validate_materiality_assessment(assessment: &MaterialityAssessment) -> Result<(), Error> {
    let rationale = assessment.rationale.as_deref().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "materiality assessment requires a rationale",
        )
    })?;
    validate_text("materiality rationale", rationale)?;
    let actor = assessment.assessed_by.as_ref().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "materiality assessment requires an actor",
        )
    })?;
    validate_text("materiality actor identity", &actor.identity)?;
    if assessment.assessed_at.is_none() || assessment.source_basis.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "materiality assessment requires time and Source basis",
        ));
    }
    validate_id_list(&assessment.source_basis)
}

fn validate_sources(canonical: &CanonicalReadBasis, source_ids: &[SourceId]) -> Result<(), Error> {
    let available = canonical
        .sources
        .iter()
        .map(|source| source.source.id)
        .collect::<BTreeSet<_>>();
    if source_ids.iter().any(|source| !available.contains(source)) {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            "Candidate references a Source absent from the current canonical read basis",
        ));
    }
    Ok(())
}

fn validate_scope(scope: &CollectionOptOutScope) -> Result<(), Error> {
    if let Some(value) = &scope.session {
        validate_text("collection session scope", value)?;
    }
    if let Some(value) = &scope.source_operation {
        validate_text("collection source-operation scope", value)?;
    }
    Ok(())
}

fn validate_list(values: &[String]) -> Result<(), Error> {
    if values.len() > MAX_LIST_ITEMS {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "bounded Candidate list exceeds the retained item limit",
        ));
    }
    for value in values {
        validate_text("bounded Candidate item", value)?;
    }
    Ok(())
}

fn validate_id_list<T>(values: &[T]) -> Result<(), Error> {
    if values.len() > MAX_LIST_ITEMS {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "bounded Candidate identity list exceeds the retained item limit",
        ));
    }
    Ok(())
}

fn validate_optional_text(label: &str, value: Option<&str>) -> Result<(), Error> {
    if let Some(value) = value {
        validate_text(label, value)?;
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must be non-empty and at most {MAX_TEXT_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn encode_scope(scope: &CollectionOptOutScope) -> Result<String, Error> {
    serde_json::to_string(scope).map_err(encode_error)
}

fn encode_record(record: &CandidateRecord) -> Result<String, Error> {
    let encoded = serde_json::to_string(record).map_err(encode_error)?;
    if encoded.len() > MAX_RECORD_BYTES {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Candidate record exceeds the durable bounded-record limit",
        ));
    }
    Ok(encoded)
}

fn decode_record(value: &str) -> Result<CandidateRecord, Error> {
    if value.len() > MAX_RECORD_BYTES {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "stored Candidate exceeds the bounded-record limit",
        ));
    }
    serde_json::from_str(value).map_err(decode_error)
}

fn load_record(
    connection: &Connection,
    project_id: ProjectId,
    candidate_id: CandidateId,
) -> Result<CandidateRecord, Error> {
    let row = connection
        .query_row(
            "SELECT project_id, revision, record_json FROM candidates WHERE id = ?1",
            [candidate_id.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(read_error)?
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Candidate was not found"))?;
    let owner = ProjectId::from_slice(&row.0).map_err(canonical_error)?;
    if owner != project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "Candidate belongs to a different Project",
        ));
    }
    let record = decode_record(&row.2)?;
    let stored_revision = u64::try_from(row.1).map_err(|_| {
        Error::new(
            ErrorKind::CorruptState,
            "stored Candidate revision is invalid",
        )
    })?;
    if record.id != candidate_id
        || record.project_id != project_id
        || record.revision != stored_revision
    {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "Candidate row identity or revision disagrees with its bounded record",
        ));
    }
    Ok(record)
}

fn initialize_or_validate(connection: &mut Connection) -> Result<(), Error> {
    let has_metadata = connection
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'metadata'",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(read_error)?
        .is_some();
    if !has_metadata {
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(write_error)?;
        let existing_tables: i64 = transaction
            .query_row(
                "SELECT count(*) FROM sqlite_schema
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(read_error)?;
        if existing_tables != 0 {
            return Err(Error::new(
                ErrorKind::UnsupportedVersion,
                "Candidate storage requires a separate empty or Candidate-owned database",
            ));
        }
        transaction
            .execute_batch(
                "CREATE TABLE metadata(
                     key TEXT PRIMARY KEY NOT NULL,
                     value TEXT NOT NULL
                 ) WITHOUT ROWID;
                 CREATE TABLE candidates(
                     id BLOB PRIMARY KEY NOT NULL CHECK(length(id) = 16),
                     project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                     revision INTEGER NOT NULL CHECK(revision >= 1),
                     record_json TEXT NOT NULL CHECK(length(record_json) > 0 AND length(record_json) <= 131072),
                     created_at INTEGER NOT NULL,
                     UNIQUE(project_id, id)
                 );
                 CREATE TABLE collection_policies(
                     scope_key TEXT PRIMARY KEY NOT NULL,
                     project_id BLOB NOT NULL CHECK(length(project_id) = 16),
                     policy_json TEXT NOT NULL CHECK(length(policy_json) > 0)
                 ) WITHOUT ROWID;",
            )
            .map_err(write_error)?;
        transaction
            .execute(
                "INSERT INTO metadata(key, value)
                 VALUES ('schema_kind', ?1), ('schema_version', ?2)",
                params![CANDIDATE_SCHEMA_KIND, CANDIDATE_SCHEMA_VERSION.to_string()],
            )
            .map_err(write_error)?;
        transaction.commit().map_err(write_error)?;
        return Ok(());
    }
    let kind: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'schema_kind'",
            [],
            |row| row.get(0),
        )
        .map_err(read_error)?;
    let version: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .map_err(read_error)?;
    if kind != CANDIDATE_SCHEMA_KIND || version != CANDIDATE_SCHEMA_VERSION.to_string() {
        return Err(Error::new(
            ErrorKind::UnsupportedVersion,
            format!(
                "unsupported Candidate store {kind} version {version}; current version is {CANDIDATE_SCHEMA_VERSION}"
            ),
        ));
    }
    for required in ["metadata", "candidates", "collection_policies"] {
        let exists = connection
            .query_row(
                "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                [required],
                |_| Ok(()),
            )
            .optional()
            .map_err(read_error)?
            .is_some();
        if !exists {
            return Err(Error::new(
                ErrorKind::CorruptState,
                format!("Candidate store is missing required table {required}"),
            ));
        }
    }
    Ok(())
}

fn open_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::StorageUnavailable,
        "cannot open Candidate store",
        error,
    )
}

fn read_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::CorruptState,
        "cannot read Candidate store",
        error,
    )
}

fn write_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::TransactionFailure,
        "Candidate store transaction failed",
        error,
    )
}

fn encode_error(error: serde_json::Error) -> Error {
    Error::with_source(
        ErrorKind::InvalidInput,
        "Candidate bounded record cannot be encoded",
        error,
    )
}

fn decode_error(error: serde_json::Error) -> Error {
    Error::with_source(
        ErrorKind::CorruptState,
        "Candidate bounded record cannot be decoded",
        error,
    )
}

fn canonical_error(error: volicord_context::Error) -> Error {
    Error::with_source(
        ErrorKind::CanonicalFailure,
        "Canonical Context rejected the Inquiry operation",
        error,
    )
}

fn id_error(error: volicord_context::Error) -> Error {
    Error::with_source(
        ErrorKind::StorageUnavailable,
        "Candidate identity generation failed",
        error,
    )
}

fn clock_error(error: volicord_context::Error) -> Error {
    Error::with_source(
        ErrorKind::StorageUnavailable,
        "Candidate observation clock failed",
        error,
    )
}
