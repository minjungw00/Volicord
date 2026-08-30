use crate::{
    CandidateCleanup, CandidateCleanupKind, CandidateCollectionMode, CandidateDisposition,
    CandidateDraft, CandidateId, CandidateKind, CandidateReadBasis, CandidateRecord,
    CollectionOptOut, CollectionOptOutScope, DuplicateAssessment, EngineeringChoiceDiscovery,
    Error, ErrorKind, LateAuthorityCorrection, LearningDeliberationState, LearningInitialResponse,
    LearningRecommendation, MaterialityAssessment, MaterialityDisposition, MaterialityReview,
    MaterialityStatus, PromotionResult, RepositoryResearchBasis, SubmissionOutcome,
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
pub const CANDIDATE_SCHEMA_VERSION: u32 = 8;

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
        if matches!(
            draft.kind,
            CandidateKind::EngineeringChoiceDiscovery
                | CandidateKind::MaterialityReview
                | CandidateKind::LearningDeliberation
        ) {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "pre-work discovery, review, and learning deliberation require typed submission operations",
            ));
        }
        self.submit_validated(draft)
    }

    pub fn submit_engineering_choice_discovery(
        &mut self,
        draft: CandidateDraft,
        canonical: &CanonicalReadBasis,
        baseline: &AnalysisSnapshot,
    ) -> Result<SubmissionOutcome, Error> {
        if draft.kind != CandidateKind::EngineeringChoiceDiscovery
            || canonical.project.id != draft.project_id
            || baseline.project.identity() != draft.project_id
        {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "Engineering Choice Discovery Project basis does not match",
            ));
        }
        let discovery = draft
            .content
            .engineering_choice_discovery
            .as_ref()
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "Engineering Choice Discovery content is missing",
                )
            })?;
        if discovery.baseline_analysis_snapshot_id != baseline.identity {
            return Err(Error::new(
                ErrorKind::StaleBasis,
                "Engineering Choice Discovery does not use the exact pre-work Analysis Snapshot",
            ));
        }
        validate_discovery_against_canonical(canonical, discovery)?;
        self.submit_validated(draft)
    }

    pub fn submit_materiality_review(
        &mut self,
        mut draft: CandidateDraft,
        canonical: &CanonicalReadBasis,
        baseline: &AnalysisSnapshot,
        current: &AnalysisSnapshot,
        discovery_candidate: &CandidateRecord,
    ) -> Result<SubmissionOutcome, Error> {
        if draft.kind != CandidateKind::MaterialityReview
            || canonical.project.id != draft.project_id
            || baseline.project.identity() != draft.project_id
            || current.project.identity() != draft.project_id
        {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "Materiality Review Project basis does not match",
            ));
        }
        let review =
            draft.content.materiality_review.as_mut().ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput, "Materiality Review is missing")
            })?;
        if review.baseline_analysis_snapshot_id != baseline.identity {
            return Err(Error::new(
                ErrorKind::StaleBasis,
                "Materiality Review does not use the exact retained pre-work Analysis Snapshot",
            ));
        }
        match crate::attribute_repository_changes(
            draft.project_id,
            &crate::RepositoryWorkBasis {
                baseline,
                current,
                pre_existing_dirty_paths: baseline.repository_worktree.dirty_paths().to_vec(),
            },
        ) {
            crate::ChangeAttribution::Attributed { changed_paths, .. }
                if changed_paths.is_empty() => {}
            crate::ChangeAttribution::Attributed { changed_paths, .. } => {
                return Err(Error::new(
                    ErrorKind::StaleBasis,
                    format!(
                        "first Materiality Review is late; meaningful repository paths already changed: {}",
                        changed_paths.join(", ")
                    ),
                ));
            }
            crate::ChangeAttribution::Unavailable { reason, .. } => {
                return Err(Error::new(ErrorKind::StaleBasis, reason));
            }
        }
        review.first_review_analysis_snapshot_id = current.identity;
        review.current_review_analysis_snapshot_id = current.identity;
        review.first_review_preceded_meaningful_mutation = true;
        validate_review_against_canonical(canonical, review)?;
        validate_review_against_discovery(review, discovery_candidate)?;
        self.submit_validated(draft)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn revise_materiality_review(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        canonical: &CanonicalReadBasis,
        baseline: &AnalysisSnapshot,
        current: &AnalysisSnapshot,
        discovery_candidate: &CandidateRecord,
        revision: crate::MaterialityReviewRevision,
    ) -> Result<CandidateRecord, Error> {
        validate_text("Materiality Review rationale", &revision.rationale)?;
        if canonical.project.id != project_id
            || baseline.project.identity() != project_id
            || current.project.identity() != project_id
        {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "revised Materiality Review Project basis does not match",
            ));
        }
        self.mutate_pending(project_id, candidate_id, |record| {
            if record.kind != CandidateKind::MaterialityReview {
                return Err(Error::new(
                    ErrorKind::DomainConflict,
                    "operation requires a Materiality Review Candidate",
                ));
            }
            let review = record
                .content
                .as_mut()
                .and_then(|content| content.materiality_review.as_mut())
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::CorruptState,
                        "Materiality Review content is missing",
                    )
                })?;
            let late_corrections =
                detect_late_authority_corrections(review, &revision.dimensions, baseline, current)?;
            let learning_value_revisions = validate_learning_value_revisions(
                review,
                &revision.dimensions,
                &revision.learning_value_revision_bases,
                canonical,
                current,
            )?;
            review.current_review_analysis_snapshot_id = current.identity;
            review.rationale = revision.rationale;
            review.learning_participation = revision.learning_participation;
            review.dimensions = revision.dimensions;
            review.late_authority_corrections.extend(late_corrections);
            review
                .learning_value_revisions
                .extend(learning_value_revisions);
            review
                .late_authority_corrections
                .sort_by(|left, right| left.dimension_id.cmp(&right.dimension_id));
            review
                .late_authority_corrections
                .dedup_by(|left, right| left.dimension_id == right.dimension_id);
            validate_materiality_review(review)?;
            validate_review_against_canonical(canonical, review)?;
            validate_review_against_discovery(review, discovery_candidate)
        })
    }

    pub fn submit_learning_deliberation(
        &mut self,
        draft: CandidateDraft,
        canonical: &CanonicalReadBasis,
        discovery_candidate: &CandidateRecord,
        review_candidate: &CandidateRecord,
    ) -> Result<SubmissionOutcome, Error> {
        if draft.kind != CandidateKind::LearningDeliberation
            || draft.project_id != canonical.project.id
            || discovery_candidate.project_id != draft.project_id
            || review_candidate.project_id != draft.project_id
        {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "Learning Deliberation Project or kind does not match",
            ));
        }
        let deliberation = draft
            .content
            .learning_deliberation
            .as_ref()
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "Learning Deliberation content is missing",
                )
            })?;
        validate_learning_deliberation_basis(
            canonical,
            deliberation,
            discovery_candidate,
            review_candidate,
        )?;
        self.submit_validated(draft)
    }

    pub fn record_learning_response(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        canonical: &CanonicalReadBasis,
        user_turn_source_id: volicord_context::SourceId,
        response: LearningInitialResponse,
        user_rationale: Option<String>,
    ) -> Result<CandidateRecord, Error> {
        validate_current_host_user_source(canonical, user_turn_source_id)?;
        if let Some(rationale) = &user_rationale {
            validate_text("learning response rationale", rationale)?;
        }
        self.mutate_pending(project_id, candidate_id, |record| {
            let deliberation = learning_deliberation_mut(record)?;
            if !matches!(
                deliberation.state,
                LearningDeliberationState::AwaitingInitialResponse
                    | LearningDeliberationState::ReconsiderationRequested { .. }
            ) {
                return Err(Error::new(
                    ErrorKind::DomainConflict,
                    "Learning Deliberation is not awaiting an initial user response",
                ));
            }
            validate_learning_response(deliberation, &response)?;
            let round = u32::try_from(deliberation.rounds.len()).map_err(|_| {
                Error::new(
                    ErrorKind::CorruptState,
                    "Learning Deliberation round count is outside the supported range",
                )
            })?;
            let next_state = match &response {
                LearningInitialResponse::Select { .. } => {
                    LearningDeliberationState::AwaitingAgentFeedback { round }
                }
                LearningInitialResponse::DelegateToAgent => {
                    LearningDeliberationState::Delegated { round }
                }
                LearningInitialResponse::Skip => LearningDeliberationState::Skipped { round },
                LearningInitialResponse::RequestResearchOrPrototype { evidence_state } => {
                    LearningDeliberationState::ResearchOrPrototypeRequired {
                        round,
                        evidence_state: *evidence_state,
                    }
                }
            };
            deliberation.rounds.push(crate::LearningDeliberationRound {
                initial_response_source_id: user_turn_source_id,
                response,
                user_rationale,
                agent_feedback: None,
                agent_recommendation: None,
                reconsideration_source_id: None,
                reconsideration_rationale: None,
            });
            deliberation.state = next_state;
            validate_learning_deliberation(deliberation)
        })
    }

    pub fn provide_learning_feedback(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        feedback: String,
        recommendation: LearningRecommendation,
    ) -> Result<CandidateRecord, Error> {
        validate_text("learning feedback", &feedback)?;
        self.mutate_pending(project_id, candidate_id, |record| {
            let deliberation = learning_deliberation_mut(record)?;
            let LearningDeliberationState::AwaitingAgentFeedback { round } = deliberation.state
            else {
                return Err(Error::new(
                    ErrorKind::DomainConflict,
                    "agent feedback requires a prior selected user response",
                ));
            };
            validate_learning_recommendation(deliberation, &recommendation)?;
            let round_record = deliberation.rounds.get_mut(round as usize).ok_or_else(|| {
                Error::new(
                    ErrorKind::CorruptState,
                    "Learning Deliberation feedback round is missing",
                )
            })?;
            round_record.agent_feedback = Some(feedback);
            round_record.agent_recommendation = Some(recommendation);
            deliberation.state = LearningDeliberationState::FeedbackProvided { round };
            validate_learning_deliberation(deliberation)
        })
    }

    pub fn complete_learning_deliberation(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
    ) -> Result<CandidateRecord, Error> {
        self.mutate_pending(project_id, candidate_id, |record| {
            let deliberation = learning_deliberation_mut(record)?;
            let LearningDeliberationState::FeedbackProvided { round } = deliberation.state else {
                return Err(Error::new(
                    ErrorKind::DomainConflict,
                    "Learning Deliberation completion requires post-response agent feedback",
                ));
            };
            let round_record = deliberation.rounds.get(round as usize).ok_or_else(|| {
                Error::new(
                    ErrorKind::CorruptState,
                    "Learning Deliberation completion round is missing",
                )
            })?;
            let LearningInitialResponse::Select { selections } = &round_record.response else {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "Learning Deliberation feedback does not follow a selection",
                ));
            };
            deliberation.state = LearningDeliberationState::Completed {
                round,
                selected_alternatives: selections.clone(),
            };
            validate_learning_deliberation(deliberation)
        })
    }

    pub fn reconsider_learning_deliberation(
        &mut self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        canonical: &CanonicalReadBasis,
        user_turn_source_id: volicord_context::SourceId,
        rationale: String,
    ) -> Result<CandidateRecord, Error> {
        validate_current_host_user_source(canonical, user_turn_source_id)?;
        validate_text("learning reconsideration rationale", &rationale)?;
        self.mutate_pending(project_id, candidate_id, |record| {
            let deliberation = learning_deliberation_mut(record)?;
            let round = match deliberation.state {
                LearningDeliberationState::FeedbackProvided { round }
                | LearningDeliberationState::Completed { round, .. } => round,
                _ => {
                    return Err(Error::new(
                        ErrorKind::DomainConflict,
                        "reconsideration requires completed post-response feedback",
                    ))
                }
            };
            let round_record = deliberation.rounds.get_mut(round as usize).ok_or_else(|| {
                Error::new(
                    ErrorKind::CorruptState,
                    "Learning Deliberation reconsideration round is missing",
                )
            })?;
            round_record.reconsideration_source_id = Some(user_turn_source_id);
            round_record.reconsideration_rationale = Some(rationale);
            deliberation.state = LearningDeliberationState::ReconsiderationRequested { round };
            validate_learning_deliberation(deliberation)
        })
    }

    fn submit_validated(&mut self, draft: CandidateDraft) -> Result<SubmissionOutcome, Error> {
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
        if question.research_state != QuestionResearchState::ReadyToAsk {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "Question Candidate research is not ready for promotion",
            ));
        }
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
                    }) || content
                        .engineering_choice_discovery
                        .as_ref()
                        .is_some_and(|discovery| {
                            discovery
                                .choices
                                .iter()
                                .any(|choice| choice.source_basis.contains(&source_id))
                        })
                        || content.materiality_review.as_ref().is_some_and(|review| {
                            matches!(
                                review.learning_participation,
                                crate::LearningParticipation::Active {
                                    user_turn_source_id,
                                    ..
                                } if user_turn_source_id == source_id
                            ) || review
                                .dimensions
                                .iter()
                                .any(|dimension| dimension.basis.source_basis.contains(&source_id))
                        })
                        || content
                            .learning_deliberation
                            .as_ref()
                            .is_some_and(|deliberation| {
                                deliberation.rounds.iter().any(|round| {
                                    round.initial_response_source_id == source_id
                                        || round.reconsideration_source_id == Some(source_id)
                                })
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
        CanonicalRecordId::Decision(decision_id) => {
            candidate.content.as_ref().is_some_and(|content| {
                content.materiality_review.as_ref().is_some_and(|review| {
                    review
                        .dimensions
                        .iter()
                        .any(|dimension| dimension.basis.decision_basis.contains(&decision_id))
                })
            })
        }
        CanonicalRecordId::ContextItem(context_item_id) => {
            candidate.content.as_ref().is_some_and(|content| {
                content
                    .engineering_choice_discovery
                    .as_ref()
                    .is_some_and(|discovery| discovery.goal_context_id == context_item_id)
                    || content
                        .materiality_review
                        .as_ref()
                        .is_some_and(|review| review.goal_context_id == context_item_id)
                    || content
                        .learning_deliberation
                        .as_ref()
                        .is_some_and(|deliberation| deliberation.goal_context_id == context_item_id)
            })
        }
        CanonicalRecordId::Checkpoint(_) => false,
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

fn learning_deliberation_mut(
    record: &mut CandidateRecord,
) -> Result<&mut crate::LearningDeliberation, Error> {
    if record.kind != CandidateKind::LearningDeliberation {
        return Err(Error::new(
            ErrorKind::DomainConflict,
            "operation requires a Learning Deliberation Candidate",
        ));
    }
    record
        .content
        .as_mut()
        .and_then(|content| content.learning_deliberation.as_mut())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Learning Deliberation content is missing",
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
    if (draft.kind == CandidateKind::EngineeringChoiceDiscovery)
        != draft.content.engineering_choice_discovery.is_some()
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Engineering Choice Discovery kind and content must agree",
        ));
    }
    if let Some(discovery) = &draft.content.engineering_choice_discovery {
        validate_engineering_choice_discovery(discovery)?;
    }
    if (draft.kind == CandidateKind::MaterialityReview)
        != draft.content.materiality_review.is_some()
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Materiality Review kind and content must agree",
        ));
    }
    if let Some(review) = &draft.content.materiality_review {
        validate_materiality_review(review)?;
    }
    if (draft.kind == CandidateKind::LearningDeliberation)
        != draft.content.learning_deliberation.is_some()
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Learning Deliberation kind and content must agree",
        ));
    }
    if let Some(deliberation) = &draft.content.learning_deliberation {
        validate_learning_deliberation(deliberation)?;
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

fn validate_materiality_review(review: &MaterialityReview) -> Result<(), Error> {
    validate_text("Materiality Review rationale", &review.rationale)?;
    if let crate::LearningParticipation::Active {
        verbatim_statement, ..
    } = &review.learning_participation
    {
        validate_text("learning participation statement", verbatim_statement)?;
    }
    if review.dimensions.is_empty() || review.dimensions.len() > MAX_LIST_ITEMS {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Materiality Review requires a bounded set of independently material dimensions",
        ));
    }
    let mut identities = BTreeSet::new();
    for dimension in &review.dimensions {
        validate_text("materiality dimension identity", &dimension.dimension_id)?;
        validate_text("materiality dimension summary", &dimension.summary)?;
        validate_text("work-authority basis summary", &dimension.basis.summary)?;
        validate_list(&dimension.affected_scope)?;
        validate_list(&dimension.material_consequences)?;
        validate_list(&dimension.basis.contract_basis)?;
        validate_list(&dimension.basis.research_basis)?;
        validate_id_list(&dimension.basis.source_basis)?;
        validate_id_list(&dimension.basis.decision_basis)?;
        validate_list(&dimension.discovered_choice_ids)?;
        match &dimension.learning_value {
            crate::LearningValueAssessment::Routine { rationale } => {
                validate_text("routine learning assessment rationale", rationale)?;
            }
            crate::LearningValueAssessment::DeliberationWorthy {
                rationale,
                consequence_significance,
                transferable_principles,
                non_obvious_trade_offs,
            } => {
                validate_text("learning deliberation rationale", rationale)?;
                validate_list(consequence_significance)?;
                validate_list(transferable_principles)?;
                validate_list(non_obvious_trade_offs)?;
                if consequence_significance.is_empty()
                    || transferable_principles.is_empty()
                    || non_obvious_trade_offs.is_empty()
                {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "deliberation-worthy learning assessment requires significance, transferability, and non-obvious trade-off evidence",
                    ));
                }
            }
        }
        if let Some(delegation) = &dimension.basis.explicit_delegation {
            validate_text(
                "explicit delegation verbatim statement",
                &delegation.verbatim_statement,
            )?;
            validate_text(
                "explicit delegation dimension identity",
                &delegation.dimension_id,
            )?;
            validate_list(&delegation.discovered_choice_ids)?;
            validate_list(&delegation.affected_scope)?;
            validate_list(&delegation.material_consequences)?;
            if delegation.discovered_choice_ids.is_empty()
                || delegation.affected_scope.is_empty()
                || delegation.material_consequences.is_empty()
                || delegation.effect_categories.is_empty()
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "explicit delegation evidence requires exact dimension, discovered-choice, scope, consequence, and effect-category boundaries",
                ));
            }
        }
        if !identities.insert(dimension.dimension_id.as_str())
            || dimension.discovered_choice_ids.is_empty()
            || dimension.affected_scope.is_empty()
            || dimension.material_consequences.is_empty()
            || dimension.basis.source_basis.is_empty()
            || dimension.basis.kinds.is_empty()
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "each materiality dimension requires a unique identity, scope, consequence, and bounded evidence basis",
            ));
        }
        if dimension
            .basis
            .kinds
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != dimension.basis.kinds.len()
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "work-authority evidence kinds must be unique",
            ));
        }
    }
    let mut correction_dimensions = BTreeSet::new();
    for correction in &review.late_authority_corrections {
        validate_text(
            "late authority correction dimension identity",
            &correction.dimension_id,
        )?;
        validate_list(&correction.affected_changed_paths)?;
        if correction.affected_changed_paths.is_empty()
            || !correction_dimensions.insert(correction.dimension_id.as_str())
            || !identities.contains(correction.dimension_id.as_str())
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "late authority correction requires one current dimension and deterministically affected changed paths",
            ));
        }
    }
    for revision in &review.learning_value_revisions {
        validate_text(
            "learning-value revision dimension identity",
            &revision.dimension_id,
        )?;
        validate_learning_value_revision_basis(&revision.basis)?;
        if !identities.contains(revision.dimension_id.as_str())
            || !matches!(
                revision.previous,
                crate::LearningValueAssessment::DeliberationWorthy { .. }
            )
            || !matches!(
                revision.current,
                crate::LearningValueAssessment::Routine { .. }
            )
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "persisted learning-value revision must describe a supported deliberation-worthy-to-routine downgrade for a current dimension",
            ));
        }
    }
    Ok(())
}

fn validate_engineering_choice_discovery(
    discovery: &EngineeringChoiceDiscovery,
) -> Result<(), Error> {
    if discovery.choices.is_empty() || discovery.choices.len() > MAX_LIST_ITEMS {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Engineering Choice Discovery requires a bounded non-empty choice set",
        ));
    }
    let mut identities = BTreeSet::new();
    for choice in &discovery.choices {
        validate_text("engineering choice identity", &choice.choice_id)?;
        validate_text("engineering choice summary", &choice.summary)?;
        validate_list(&choice.affected_scope)?;
        validate_list(&choice.technical_consequences)?;
        validate_id_list(&choice.source_basis)?;
        if !identities.insert(choice.choice_id.as_str())
            || choice.affected_scope.is_empty()
            || choice.technical_consequences.is_empty()
            || choice.source_basis.is_empty()
            || choice.effect_categories.is_empty()
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "each engineering choice requires unique identity, scope, consequence, Source, and effect-category basis",
            ));
        }
        if choice.alternatives.len() < 2
            && matches!(
                choice.evidence_state,
                crate::EngineeringChoiceEvidenceState::Sufficient
            )
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "a discovery-worthy choice needs two credible alternatives or unresolved research/prototype evidence",
            ));
        }
        if choice.alternatives.len() > MAX_LIST_ITEMS {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "engineering choice alternatives exceed the bounded item limit",
            ));
        }
        let mut alternative_ids = BTreeSet::new();
        for alternative in &choice.alternatives {
            validate_text(
                "engineering alternative identity",
                &alternative.alternative_id,
            )?;
            validate_text("engineering alternative summary", &alternative.summary)?;
            validate_list(&alternative.technical_consequences)?;
            if !alternative_ids.insert(alternative.alternative_id.as_str())
                || alternative.technical_consequences.is_empty()
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "engineering alternatives require unique identity and technical consequences",
                ));
            }
        }
        if let crate::EngineeringChoiceRelationship::Coupled {
            choice_ids,
            rationale,
        } = &choice.relationship
        {
            validate_list(choice_ids)?;
            validate_text("engineering choice coupling rationale", rationale)?;
            if choice_ids.is_empty()
                || choice_ids.iter().any(|id| id == &choice.choice_id)
                || choice_ids.iter().collect::<BTreeSet<_>>().len() != choice_ids.len()
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "coupled choices require distinct peer identities and a necessary-joint-outcome rationale",
                ));
            }
        }
    }
    for choice in &discovery.choices {
        if let crate::EngineeringChoiceRelationship::Coupled { choice_ids, .. } =
            &choice.relationship
        {
            for peer_id in choice_ids {
                let peer = discovery
                    .choices
                    .iter()
                    .find(|candidate| &candidate.choice_id == peer_id)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::InvalidInput,
                            "coupled engineering choice references an unknown peer",
                        )
                    })?;
                let reciprocal = matches!(
                    &peer.relationship,
                    crate::EngineeringChoiceRelationship::Coupled { choice_ids, .. }
                        if choice_ids.contains(&choice.choice_id)
                );
                if !reciprocal {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "coupled engineering choices must declare their relationship symmetrically",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_review_against_canonical(
    canonical: &CanonicalReadBasis,
    review: &MaterialityReview,
) -> Result<(), Error> {
    let current_goal = canonical
        .context_items
        .iter()
        .filter(|item| item.role == volicord_context::ContextItemRole::Goal)
        .max_by_key(|item| (item.recorded_at, item.id));
    let goal = current_goal
        .filter(|item| item.id == review.goal_context_id)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::StaleBasis,
                "Materiality Review must bind the current Goal Context",
            )
        })?;
    if goal.provenance_role != volicord_context::StatementProvenanceRole::UserStatement
        || goal.author.kind != volicord_context::PrincipalKind::User
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Materiality Review Goal must be a current user-stated Goal Context",
        ));
    }
    let available = canonical
        .sources
        .iter()
        .filter(|basis| basis.freshness == volicord_context::SourceFreshness::Current)
        .map(|basis| basis.source.id)
        .collect::<BTreeSet<_>>();
    if review.dimensions.iter().any(|dimension| {
        dimension
            .basis
            .source_basis
            .iter()
            .any(|source| !available.contains(source))
    }) {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            "Materiality Review contains a missing or non-current Source basis",
        ));
    }
    if let Some(decision_id) = review
        .dimensions
        .iter()
        .flat_map(|dimension| dimension.basis.decision_basis.iter())
        .find(|decision_id| {
            !canonical
                .active_decisions
                .iter()
                .any(|lifecycle| lifecycle.decision.id == **decision_id)
        })
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Materiality Review authority Decision {decision_id} is not active in the current Project"
            ),
        ));
    }
    if let crate::LearningParticipation::Active {
        user_turn_source_id,
        verbatim_statement,
    } = &review.learning_participation
    {
        validate_current_host_user_source(canonical, *user_turn_source_id)?;
        let source = canonical
            .sources
            .iter()
            .find(|basis| basis.source.id == *user_turn_source_id)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "learning participation Source is missing",
                )
            })?;
        let volicord_context::SourcePayload::CurrentHostUserTurn { turn, .. } =
            &source.source.payload
        else {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "learning participation requires a current-host user-turn Source",
            ));
        };
        if !turn.contains(verbatim_statement) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "learning participation must preserve the explicit user statement verbatim",
            ));
        }
    }
    for dimension in &review.dimensions {
        let Some(delegation) = &dimension.basis.explicit_delegation else {
            continue;
        };
        if delegation.goal_context_id != review.goal_context_id
            || !dimension
                .basis
                .source_basis
                .contains(&delegation.user_turn_source_id)
            || !goal.source_basis.contains(&delegation.user_turn_source_id)
            || !goal.statement.contains(&delegation.verbatim_statement)
            || !delegation_scope_contains_dimension(
                &delegation.affected_scope,
                &dimension.affected_scope,
            )
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "explicit delegation evidence must bind the exact Goal, its user-turn Source, a verbatim Goal statement, and the dimension scope",
            ));
        }
        let source = canonical
            .sources
            .iter()
            .find(|basis| basis.source.id == delegation.user_turn_source_id)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "explicit delegation user-turn Source is missing",
                )
            })?;
        if source.freshness != volicord_context::SourceFreshness::Current
            || source.source.project_id != canonical.project.id
            || source.source.actor.kind != volicord_context::PrincipalKind::User
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "explicit delegation evidence requires the current Project user Source",
            ));
        }
        let volicord_context::SourcePayload::CurrentHostUserTurn { turn, .. } =
            &source.source.payload
        else {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "explicit delegation evidence requires a current-host user-turn Source",
            ));
        };
        if !turn.contains(&goal.statement) || !turn.contains(&delegation.verbatim_statement) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "explicit delegation statement must remain verbatim-grounded in the exact current-host user turn",
            ));
        }
    }
    Ok(())
}

fn validate_discovery_against_canonical(
    canonical: &CanonicalReadBasis,
    discovery: &EngineeringChoiceDiscovery,
) -> Result<(), Error> {
    let current_goal = canonical
        .context_items
        .iter()
        .filter(|item| item.role == volicord_context::ContextItemRole::Goal)
        .max_by_key(|item| (item.recorded_at, item.id));
    if current_goal.map(|goal| goal.id) != Some(discovery.goal_context_id) {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            "Engineering Choice Discovery must bind the current Goal Context",
        ));
    }
    let current_sources = canonical
        .sources
        .iter()
        .filter(|basis| basis.freshness == volicord_context::SourceFreshness::Current)
        .map(|basis| basis.source.id)
        .collect::<BTreeSet<_>>();
    if discovery.choices.iter().any(|choice| {
        choice
            .source_basis
            .iter()
            .any(|source| !current_sources.contains(source))
    }) {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            "Engineering Choice Discovery contains a missing or non-current Source basis",
        ));
    }
    Ok(())
}

fn validate_review_against_discovery(
    review: &MaterialityReview,
    discovery_candidate: &CandidateRecord,
) -> Result<(), Error> {
    if discovery_candidate.id != review.engineering_choice_discovery_candidate_id
        || discovery_candidate.kind != CandidateKind::EngineeringChoiceDiscovery
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Materiality Review must reference the exact Engineering Choice Discovery Candidate",
        ));
    }
    let discovery = discovery_candidate
        .content
        .as_ref()
        .and_then(|content| content.engineering_choice_discovery.as_ref())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Engineering Choice Discovery content is unavailable",
            )
        })?;
    if discovery.goal_context_id != review.goal_context_id
        || discovery.baseline_analysis_snapshot_id != review.baseline_analysis_snapshot_id
    {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            "Materiality Review and Engineering Choice Discovery Goal/baseline differ",
        ));
    }
    let discovered = discovery
        .choices
        .iter()
        .map(|choice| choice.choice_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut reviewed = BTreeSet::new();
    for dimension in &review.dimensions {
        for choice_id in &dimension.discovered_choice_ids {
            if !discovered.contains(choice_id.as_str()) || !reviewed.insert(choice_id.as_str()) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "each discovered choice must be referenced by exactly one Materiality dimension",
                ));
            }
        }
        if let Some(delegation) = &dimension.basis.explicit_delegation {
            let choices = dimension
                .discovered_choice_ids
                .iter()
                .filter_map(|choice_id| {
                    discovery
                        .choices
                        .iter()
                        .find(|choice| &choice.choice_id == choice_id)
                })
                .collect::<Vec<_>>();
            let effect_categories = choices
                .iter()
                .flat_map(|choice| choice.effect_categories.iter().copied())
                .collect::<BTreeSet<_>>();
            if delegation.dimension_id != dimension.dimension_id
                || delegation
                    .discovered_choice_ids
                    .iter()
                    .collect::<BTreeSet<_>>()
                    != dimension
                        .discovered_choice_ids
                        .iter()
                        .collect::<BTreeSet<_>>()
                || delegation
                    .material_consequences
                    .iter()
                    .collect::<BTreeSet<_>>()
                    != dimension
                        .material_consequences
                        .iter()
                        .collect::<BTreeSet<_>>()
                || delegation
                    .effect_categories
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    != effect_categories
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "explicit delegation evidence must name the exact dimension, discovered choices, material consequences, and discovery effect categories it claims to settle",
                ));
            }
        }
        if dimension.discovered_choice_ids.len() > 1 {
            let grouped = dimension
                .discovered_choice_ids
                .iter()
                .collect::<BTreeSet<_>>();
            for choice_id in &dimension.discovered_choice_ids {
                let Some(choice) = discovery
                    .choices
                    .iter()
                    .find(|choice| &choice.choice_id == choice_id)
                else {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "grouped authority dimension references an unknown discovered choice",
                    ));
                };
                let crate::EngineeringChoiceRelationship::Coupled { choice_ids, .. } =
                    &choice.relationship
                else {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "independent discovered choices cannot be collapsed into one authority dimension",
                    ));
                };
                let expected = choice_ids
                    .iter()
                    .chain(std::iter::once(choice_id))
                    .collect();
                if grouped != expected {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "a grouped authority dimension must contain the complete declared coupled-choice set",
                    ));
                }
            }
        }
    }
    if reviewed != discovered {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Materiality Review must classify every discovered engineering choice",
        ));
    }
    Ok(())
}

fn detect_late_authority_corrections(
    existing: &MaterialityReview,
    revised_dimensions: &[crate::MaterialityDimension],
    baseline: &AnalysisSnapshot,
    current: &AnalysisSnapshot,
) -> Result<Vec<LateAuthorityCorrection>, Error> {
    let changed_paths = match crate::attribute_repository_changes(
        baseline.project.identity(),
        &crate::RepositoryWorkBasis {
            baseline,
            current,
            pre_existing_dirty_paths: baseline.repository_worktree.dirty_paths().to_vec(),
        },
    ) {
        crate::ChangeAttribution::Attributed { changed_paths, .. } => changed_paths,
        crate::ChangeAttribution::Unavailable { .. } => return Ok(Vec::new()),
    };
    let already_recorded = existing
        .late_authority_corrections
        .iter()
        .map(|correction| correction.dimension_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut corrections = Vec::new();
    for revised in revised_dimensions {
        let Some(previous) = existing
            .dimensions
            .iter()
            .find(|dimension| dimension.dimension_id == revised.dimension_id)
        else {
            continue;
        };
        if already_recorded.contains(revised.dimension_id.as_str())
            || !matches!(
                previous.disposition,
                MaterialityDisposition::AgentOwnedImplementationChoice
                    | MaterialityDisposition::DelegatedImplementationChoice
            )
            || !matches!(
                revised.disposition,
                MaterialityDisposition::UnresolvedUserOwnedOutcome { .. }
            )
        {
            continue;
        }
        let affected_changed_paths = changed_paths
            .iter()
            .filter(|path| scope_overlaps_path(&revised.affected_scope, path))
            .cloned()
            .collect::<Vec<_>>();
        if !affected_changed_paths.is_empty() {
            corrections.push(LateAuthorityCorrection {
                dimension_id: revised.dimension_id.clone(),
                detected_analysis_snapshot_id: current.identity,
                affected_changed_paths,
            });
        }
    }
    Ok(corrections)
}

fn validate_learning_value_revisions(
    existing: &MaterialityReview,
    revised_dimensions: &[crate::MaterialityDimension],
    requests: &[crate::LearningValueRevisionRequest],
    canonical: &CanonicalReadBasis,
    current: &AnalysisSnapshot,
) -> Result<Vec<crate::LearningValueRevision>, Error> {
    let mut indexed = std::collections::BTreeMap::new();
    for request in requests {
        validate_text(
            "learning-value revision dimension identity",
            &request.dimension_id,
        )?;
        validate_learning_value_revision_basis(&request.basis)?;
        if indexed
            .insert(request.dimension_id.as_str(), request)
            .is_some()
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "learning-value revision basis must be unique per dimension",
            ));
        }
    }
    let mut revisions = Vec::new();
    for revised in revised_dimensions {
        let Some(previous) = existing
            .dimensions
            .iter()
            .find(|dimension| dimension.dimension_id == revised.dimension_id)
        else {
            continue;
        };
        let downgrade = matches!(
            previous.learning_value,
            crate::LearningValueAssessment::DeliberationWorthy { .. }
        ) && matches!(
            revised.learning_value,
            crate::LearningValueAssessment::Routine { .. }
        );
        let request = indexed.remove(revised.dimension_id.as_str());
        if !downgrade {
            if request.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "learning-value revision basis is accepted only for a deliberation-worthy-to-routine downgrade",
                ));
            }
            continue;
        }
        let request = request.ok_or_else(|| {
            Error::new(
                ErrorKind::DomainConflict,
                "deliberation-worthy learning cannot be downgraded to routine without a supported research, prototype, or current-user withdrawal basis",
            )
        })?;
        validate_learning_value_revision_basis_against_canonical(&request.basis, canonical)?;
        revisions.push(crate::LearningValueRevision {
            dimension_id: revised.dimension_id.clone(),
            previous: previous.learning_value.clone(),
            current: revised.learning_value.clone(),
            basis: request.basis.clone(),
            revised_analysis_snapshot_id: current.identity,
        });
    }
    if !indexed.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "learning-value revision basis references a missing Materiality dimension",
        ));
    }
    Ok(revisions)
}

fn validate_learning_value_revision_basis(
    basis: &crate::LearningValueRevisionBasis,
) -> Result<(), Error> {
    match basis {
        crate::LearningValueRevisionBasis::ResearchEvidence {
            source_basis,
            evidence_basis,
            rationale,
        }
        | crate::LearningValueRevisionBasis::PrototypeEvidence {
            source_basis,
            evidence_basis,
            rationale,
        } => {
            validate_id_list(source_basis)?;
            validate_list(evidence_basis)?;
            validate_text("learning-value revision rationale", rationale)?;
            if source_basis.is_empty() || evidence_basis.is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "research/prototype learning-value revision requires current Source identities and bounded evidence",
                ));
            }
        }
        crate::LearningValueRevisionBasis::CurrentUserWithdrawal {
            verbatim_statement,
            rationale,
            ..
        } => {
            validate_text("learning withdrawal statement", verbatim_statement)?;
            validate_text("learning-value revision rationale", rationale)?;
        }
    }
    Ok(())
}

fn validate_learning_value_revision_basis_against_canonical(
    basis: &crate::LearningValueRevisionBasis,
    canonical: &CanonicalReadBasis,
) -> Result<(), Error> {
    match basis {
        crate::LearningValueRevisionBasis::ResearchEvidence { source_basis, .. }
        | crate::LearningValueRevisionBasis::PrototypeEvidence { source_basis, .. } => {
            if source_basis.iter().any(|source_id| {
                !canonical.sources.iter().any(|candidate| {
                    candidate.source.id == *source_id
                        && candidate.freshness == volicord_context::SourceFreshness::Current
                })
            }) {
                return Err(Error::new(
                    ErrorKind::StaleBasis,
                    "learning-value revision evidence contains a missing or non-current Source",
                ));
            }
        }
        crate::LearningValueRevisionBasis::CurrentUserWithdrawal {
            user_turn_source_id,
            verbatim_statement,
            ..
        } => {
            validate_current_host_user_source(canonical, *user_turn_source_id)?;
            let source = canonical
                .sources
                .iter()
                .find(|candidate| candidate.source.id == *user_turn_source_id)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "learning withdrawal Source is missing",
                    )
                })?;
            let volicord_context::SourcePayload::CurrentHostUserTurn { turn, .. } =
                &source.source.payload
            else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "learning withdrawal requires a current-host user-turn Source",
                ));
            };
            if !turn.contains(verbatim_statement) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "learning withdrawal must preserve the current user's statement verbatim",
                ));
            }
        }
    }
    Ok(())
}

fn scope_overlaps_path(scope: &[String], path: &str) -> bool {
    scope.iter().any(|item| {
        item == path
            || path
                .strip_prefix(item)
                .is_some_and(|suffix| suffix.starts_with('/'))
            || item
                .strip_prefix(path)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn delegation_scope_contains_dimension(
    delegation_scope: &[String],
    affected_scope: &[String],
) -> bool {
    affected_scope.iter().all(|affected| {
        delegation_scope.iter().any(|delegated| {
            delegated == affected
                || affected
                    .strip_prefix(delegated)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    })
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

fn validate_learning_deliberation_basis(
    canonical: &CanonicalReadBasis,
    deliberation: &crate::LearningDeliberation,
    discovery_candidate: &CandidateRecord,
    review_candidate: &CandidateRecord,
) -> Result<(), Error> {
    if discovery_candidate.id != deliberation.engineering_choice_discovery_candidate_id
        || discovery_candidate.kind != CandidateKind::EngineeringChoiceDiscovery
        || review_candidate.id != deliberation.materiality_review_candidate_id
        || review_candidate.kind != CandidateKind::MaterialityReview
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Learning Deliberation must reference its exact discovery and review Candidates",
        ));
    }
    let discovery = discovery_candidate
        .content
        .as_ref()
        .and_then(|content| content.engineering_choice_discovery.as_ref())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Learning Deliberation discovery content is unavailable",
            )
        })?;
    let review = review_candidate
        .content
        .as_ref()
        .and_then(|content| content.materiality_review.as_ref())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "Learning Deliberation review content is unavailable",
            )
        })?;
    if deliberation.goal_context_id != review.goal_context_id
        || deliberation.goal_context_id != discovery.goal_context_id
        || deliberation.baseline_analysis_snapshot_id != review.baseline_analysis_snapshot_id
        || deliberation.baseline_analysis_snapshot_id != discovery.baseline_analysis_snapshot_id
        || review.engineering_choice_discovery_candidate_id != discovery_candidate.id
    {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            "Learning Deliberation Goal, baseline, discovery, or review basis is stale",
        ));
    }
    let dimension = review
        .dimensions
        .iter()
        .find(|dimension| dimension.dimension_id == deliberation.dimension_id)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                "Learning Deliberation materiality dimension was not found",
            )
        })?;
    if !matches!(
        review.learning_participation,
        crate::LearningParticipation::Active { .. }
    ) || !matches!(
        dimension.disposition,
        MaterialityDisposition::AgentOwnedImplementationChoice
            | MaterialityDisposition::DelegatedImplementationChoice
    ) || !matches!(
        dimension.learning_value,
        crate::LearningValueAssessment::DeliberationWorthy { .. }
    ) {
        return Err(Error::new(
            ErrorKind::DomainConflict,
            "Learning Deliberation requires explicit participation and an agent-owned deliberation-worthy dimension",
        ));
    }
    let exact_choices = discovery
        .choices
        .iter()
        .filter(|choice| dimension.discovered_choice_ids.contains(&choice.choice_id))
        .cloned()
        .collect::<Vec<_>>();
    if deliberation.discovered_choice_ids != dimension.discovered_choice_ids
        || deliberation.affected_scope != dimension.affected_scope
        || deliberation.choices != exact_choices
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Learning Deliberation must preserve the exact discovered choices and affected work scope",
        ));
    }
    validate_review_against_canonical(canonical, review)?;
    validate_learning_deliberation(deliberation)
}

fn validate_learning_deliberation(deliberation: &crate::LearningDeliberation) -> Result<(), Error> {
    validate_text(
        "Learning Deliberation dimension identity",
        &deliberation.dimension_id,
    )?;
    validate_text("Learning Deliberation problem", &deliberation.problem)?;
    validate_list(&deliberation.discovered_choice_ids)?;
    validate_list(&deliberation.affected_scope)?;
    validate_list(&deliberation.established_facts)?;
    if deliberation.discovered_choice_ids.is_empty()
        || deliberation.affected_scope.is_empty()
        || deliberation.established_facts.is_empty()
        || deliberation.choices.is_empty()
        || deliberation.rounds.len() > MAX_LIST_ITEMS
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Learning Deliberation requires bounded choice, scope, fact, and round state",
        ));
    }
    let choice_ids = deliberation
        .choices
        .iter()
        .map(|choice| choice.choice_id.as_str())
        .collect::<BTreeSet<_>>();
    if choice_ids
        != deliberation
            .discovered_choice_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
        || choice_ids.len() != deliberation.discovered_choice_ids.len()
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Learning Deliberation choice identities must be exact and unique",
        ));
    }
    for choice in &deliberation.choices {
        validate_text("Learning Deliberation choice summary", &choice.summary)?;
        if choice.alternatives.len() < 2 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Learning Deliberation requires credible alternatives",
            ));
        }
    }
    for round in &deliberation.rounds {
        if let Some(rationale) = &round.user_rationale {
            validate_text("learning user rationale", rationale)?;
        }
        if let Some(feedback) = &round.agent_feedback {
            validate_text("learning agent feedback", feedback)?;
        }
        if round.agent_feedback.is_some() != round.agent_recommendation.is_some() {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Learning Deliberation feedback and recommendation must be recorded together",
            ));
        }
        if let Some(recommendation) = &round.agent_recommendation {
            validate_learning_recommendation(deliberation, recommendation)?;
        }
        if round.reconsideration_source_id.is_some() != round.reconsideration_rationale.is_some() {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "Learning Deliberation reconsideration Source and rationale must be recorded together",
            ));
        }
        if let Some(rationale) = &round.reconsideration_rationale {
            validate_text("learning reconsideration rationale", rationale)?;
        }
        validate_learning_response(deliberation, &round.response)?;
    }
    match &deliberation.state {
        LearningDeliberationState::AwaitingInitialResponse if deliberation.rounds.is_empty() => {}
        LearningDeliberationState::AwaitingInitialResponse => {
            return Err(Error::new(
                ErrorKind::CorruptState,
                "initial pre-response state cannot contain a response or recommendation",
            ));
        }
        LearningDeliberationState::AwaitingAgentFeedback { round } => {
            let value = learning_round(deliberation, *round)?;
            if !matches!(value.response, LearningInitialResponse::Select { .. })
                || value.agent_feedback.is_some()
                || value.agent_recommendation.is_some()
            {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "agent feedback state must follow a selection and cannot predate it",
                ));
            }
        }
        LearningDeliberationState::FeedbackProvided { round } => {
            let value = learning_round(deliberation, *round)?;
            if !matches!(value.response, LearningInitialResponse::Select { .. })
                || value.agent_feedback.is_none()
                || value.agent_recommendation.is_none()
            {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "feedback-provided state requires a prior selection and later feedback",
                ));
            }
        }
        LearningDeliberationState::Completed {
            round,
            selected_alternatives,
        } => {
            let value = learning_round(deliberation, *round)?;
            let LearningInitialResponse::Select { selections } = &value.response else {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "completed Learning Deliberation requires a selected response",
                ));
            };
            if selections != selected_alternatives
                || value.agent_feedback.is_none()
                || value.agent_recommendation.is_none()
            {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "completed Learning Deliberation must retain its selection and post-response feedback",
                ));
            }
        }
        LearningDeliberationState::Delegated { round } => {
            if !matches!(
                learning_round(deliberation, *round)?.response,
                LearningInitialResponse::DelegateToAgent
            ) {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "delegated Learning Deliberation must retain the delegation response",
                ));
            }
        }
        LearningDeliberationState::Skipped { round } => {
            if !matches!(
                learning_round(deliberation, *round)?.response,
                LearningInitialResponse::Skip
            ) {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "skipped Learning Deliberation must retain the skip response",
                ));
            }
        }
        LearningDeliberationState::ResearchOrPrototypeRequired {
            round,
            evidence_state,
        } => {
            if !matches!(
                learning_round(deliberation, *round)?.response,
                LearningInitialResponse::RequestResearchOrPrototype {
                    evidence_state: state
                } if state == *evidence_state
            ) {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "research Learning Deliberation must retain the requested evidence state",
                ));
            }
        }
        LearningDeliberationState::ReconsiderationRequested { round } => {
            let value = learning_round(deliberation, *round)?;
            if value.agent_feedback.is_none()
                || value.agent_recommendation.is_none()
                || value.reconsideration_source_id.is_none()
            {
                return Err(Error::new(
                    ErrorKind::CorruptState,
                    "reconsideration requires a prior response, feedback, and explicit user Source",
                ));
            }
        }
    }
    Ok(())
}

fn learning_round(
    deliberation: &crate::LearningDeliberation,
    round: u32,
) -> Result<&crate::LearningDeliberationRound, Error> {
    if round as usize + 1 != deliberation.rounds.len() {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "Learning Deliberation state must reference the current response round",
        ));
    }
    deliberation.rounds.get(round as usize).ok_or_else(|| {
        Error::new(
            ErrorKind::CorruptState,
            "Learning Deliberation response round is missing",
        )
    })
}

fn validate_learning_response(
    deliberation: &crate::LearningDeliberation,
    response: &LearningInitialResponse,
) -> Result<(), Error> {
    match response {
        LearningInitialResponse::Select { selections } => {
            validate_learning_selections(deliberation, selections)
        }
        LearningInitialResponse::RequestResearchOrPrototype {
            evidence_state:
                crate::EngineeringChoiceEvidenceState::ResearchRequired
                | crate::EngineeringChoiceEvidenceState::PrototypeRequired,
        } => Ok(()),
        LearningInitialResponse::RequestResearchOrPrototype { .. } => Err(Error::new(
            ErrorKind::InvalidInput,
            "learning evidence request must require research or prototype work",
        )),
        LearningInitialResponse::DelegateToAgent | LearningInitialResponse::Skip => Ok(()),
    }
}

fn validate_learning_recommendation(
    deliberation: &crate::LearningDeliberation,
    recommendation: &LearningRecommendation,
) -> Result<(), Error> {
    validate_text(
        "learning recommendation rationale",
        &recommendation.rationale,
    )?;
    validate_learning_selections(deliberation, &recommendation.selections)
}

fn validate_learning_selections(
    deliberation: &crate::LearningDeliberation,
    selections: &[crate::LearningAlternativeSelection],
) -> Result<(), Error> {
    if selections.len() != deliberation.choices.len() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "learning selection must choose one alternative for every discovered choice",
        ));
    }
    let mut selected_choices = BTreeSet::new();
    for selection in selections {
        validate_text("learning selection choice identity", &selection.choice_id)?;
        validate_text(
            "learning selection alternative identity",
            &selection.alternative_id,
        )?;
        let choice = deliberation
            .choices
            .iter()
            .find(|choice| choice.choice_id == selection.choice_id)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    "learning selection references an unknown discovered choice",
                )
            })?;
        if !selected_choices.insert(selection.choice_id.as_str())
            || !choice
                .alternatives
                .iter()
                .any(|alternative| alternative.alternative_id == selection.alternative_id)
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "learning selection must reference one credible alternative per choice",
            ));
        }
    }
    Ok(())
}

fn validate_current_host_user_source(
    canonical: &CanonicalReadBasis,
    source_id: SourceId,
) -> Result<(), Error> {
    let source = canonical
        .sources
        .iter()
        .find(|basis| basis.source.id == source_id)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "current-host user Source is missing",
            )
        })?;
    if source.freshness != volicord_context::SourceFreshness::Current
        || source.source.project_id != canonical.project.id
        || source.source.actor.kind != volicord_context::PrincipalKind::User
        || !matches!(
            source.source.payload,
            volicord_context::SourcePayload::CurrentHostUserTurn { .. }
        )
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "learning participation and responses require a current-host user Source",
        ));
    }
    Ok(())
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
