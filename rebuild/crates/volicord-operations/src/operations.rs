use crate::{
    AnalysisOutcome, BindingOutcome, CandidateRepositoryResearchDraft, CanonicalMutationOutcome,
    ChildProcessOutcome, CommandVerificationDraft, Error, GroundedCheckpointDraft,
    GroundedCheckpointOutcome, HealthIssue, HealthIssueKind, HealthReport, HealthState,
    LongOperationResult, OperationState, PartialOutcome, ProgressState, ProjectInitialization,
    PublicationOutcome, RepairKind, RepairOutcome, RuntimeLayout, UserContextRecordingOutcome,
};
use crate::{
    BackgroundProviderDispatcher, BackgroundProviderOperationDraft, ConfirmationDecision,
    ConfirmationRequestId, ConfirmationResponse, DispatchExpectation, GuardedEffectCandidate,
    GuardedEffectCategory, GuardedEffectDispatcher, GuardedEffectDraft, GuardedOperationId,
    GuardedOperationResult, GuardedProviderInspection, GuardedProviderPreparation,
    GuardedProviderPreparationOutcome, GuardedRisk, GuardedStore,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use volicord_context::{
    Availability, BundleComparison, BundleMerge, CanonicalReadBasis, CanonicalReadOptions,
    CanonicalRecordId, CheckpointDraft, Clock, CommandOutcome, ContextItemCorrectionDraft,
    ContextItemDraft, ContextItemId, ContextItemRole, DecisionChoice, DecisionCorrectionDraft,
    DecisionId, DecisionSupersessionDraft, MergeResolution, OperationId, OperationResult,
    Principal, PrincipalKind, ProjectId, SourceDraft, SourceId, SourcePayload,
    StatementProvenanceRole, Store, SystemClock, TimestampMicros, UserAcceptanceFact,
    UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource, VerificationFact,
    VerificationState,
};
use volicord_inquiry::{
    attribute_repository_changes, compute_frontier, evaluate_checkpoint_candidate,
    evaluate_decision_applicability, record_checkpoint as persist_evaluated_checkpoint,
    record_response_batch, ApplicabilityQuery, BatchResponseItem, BatchResponseResult,
    CandidateDraft, CandidateId, CandidateReadBasis, CandidateRecord, CandidateStore,
    ChangeAttribution, CheckpointCandidate, CheckpointEvaluation, DecisionApplicabilityState,
    FrontierRead, InquiryScope, PromotionResult, RepositoryResearchBasis, RepositoryWorkBasis,
    SubmissionOutcome,
};
use volicord_local_platform::{
    publish_file_no_replace, CancellationFlag, DirectoryEntryDurability, DirtyObservation,
    GitWorktreeLayout, NoReplacePublicationOutcome, ProcessCompletion, ProcessRequest,
    ProcessStopTrigger, ProcessTermination, ProcessTreeCleanup, RepositoryPathState,
    RepositoryRoot,
};
use volicord_privacy::{
    BackgroundSemanticProvider, BackgroundSemanticRequest, BackgroundSource, PreparationOutcome,
    PrivacyStore, ProjectPrivacyInspection, ProviderAvailability, ProviderDeletionOutcome,
    ProviderDeletionRequest, ProviderExecution, ProviderIdentity, ProviderIntentProvenance,
    ProviderInvocation, ProviderOptInEvent, ProviderOptInPolicy, ProviderRequestId,
    ProviderRequestRecord, SourceClass,
};
use volicord_projections::{
    build_project_projection, generate_documents, CandidateContentAccess, DocumentKind,
    DocumentRequest, DocumentSet, GeneratedDocument, OutputFormat, ProjectProjection,
    ProjectProjectionInputs, ProjectionBound, RecallBound, RecallInputs, ResumeBrief,
};
use volicord_repository_intelligence::{
    analyze_repository, AnalysisSnapshot, AnalysisSnapshotId, CanonicalGrounding, CapabilityState,
    EntryKind, InventoryClassification, InventoryEntry, InventoryRequest,
    RepositoryWorktreeObservation, StructuralAnalysisRequest,
};

pub struct LocalOperations {
    layout: RuntimeLayout,
}

struct PreparedAnalysisBasis {
    root: RepositoryRoot,
    requested_path: PathBuf,
    canonical: CanonicalReadBasis,
    repository_source: SourceId,
    repository_worktree: RepositoryWorktreeObservation,
}

impl LocalOperations {
    pub fn new(layout: RuntimeLayout) -> Self {
        Self { layout }
    }
    pub fn layout(&self) -> &RuntimeLayout {
        &self.layout
    }

    pub fn initialize_runtime(&self) -> Result<(), Error> {
        fs::create_dir_all(self.layout.analysis_dir()).map_err(|error| {
            Error::with_source("cannot create analysis runtime directory", error)
        })?;
        fs::create_dir_all(self.layout.artifacts_dir()).map_err(|error| {
            Error::with_source("cannot create operation artifact directory", error)
        })?;
        Store::open(self.layout.canonical_store())
            .map_err(|error| Error::with_source("cannot initialize canonical store", error))?;
        CandidateStore::open(self.layout.candidate_store())
            .map_err(|error| Error::with_source("cannot initialize Candidate store", error))?;
        PrivacyStore::open(self.layout.privacy_store())
            .map_err(|error| Error::with_source("cannot initialize privacy store", error))?;
        GuardedStore::open(self.layout.guarded_store())?;
        Ok(())
    }

    pub fn initialize_project(
        &self,
        display_name: impl Into<String>,
        repository: Option<&Path>,
    ) -> Result<ProjectInitialization, Error> {
        self.initialize_runtime()?;
        let mut canonical = self.open_canonical()?;
        let created = canonical
            .create_project(new_operation_id()?, display_name)
            .map_err(|error| Error::with_source("project initialization failed", error))?;
        let binding = repository
            .map(|path| self.bind_with_store(&mut canonical, created.value.id, None, path))
            .transpose()?;
        Ok(ProjectInitialization {
            project: created.value,
            binding,
        })
    }

    pub fn bind_project(
        &self,
        project_id: ProjectId,
        expected_binding_revision: Option<u64>,
        repository: &Path,
    ) -> Result<BindingOutcome, Error> {
        let mut canonical = self.open_canonical()?;
        self.bind_with_store(
            &mut canonical,
            project_id,
            expected_binding_revision,
            repository,
        )
    }

    fn bind_with_store(
        &self,
        canonical: &mut Store,
        project_id: ProjectId,
        expected_binding_revision: Option<u64>,
        repository: &Path,
    ) -> Result<BindingOutcome, Error> {
        let root = RepositoryRoot::open(repository)
            .map_err(|error| Error::with_source("repository binding path is invalid", error))?;
        let layout = GitWorktreeLayout::resolve(root.canonical_path())
            .map_err(|error| Error::with_source("cannot observe repository identity", error))?;
        let coordinate = layout.as_ref().map(GitWorktreeLayout::coordinate);
        let result = canonical
            .bind_clone(
                new_operation_id()?,
                project_id,
                expected_binding_revision,
                root.canonical_path().to_path_buf(),
                Availability::Available,
            )
            .map_err(|error| Error::with_source("project binding failed", error))?;
        Ok(BindingOutcome {
            binding: result.value,
            clone_identity: coordinate
                .as_ref()
                .map(|value| value.clone_identity().to_owned()),
            worktree_identity: coordinate.map(|value| value.worktree_identity().to_owned()),
        })
    }

    pub fn health(&self, project_id: Option<ProjectId>) -> HealthReport {
        let mut issues = Vec::new();
        let canonical = Store::open(self.layout.canonical_store());
        let candidates = CandidateStore::open(self.layout.candidate_store());
        let privacy = PrivacyStore::open(self.layout.privacy_store());
        let guarded = GuardedStore::open(self.layout.guarded_store());
        let canonical_available = canonical.is_ok();
        let candidate_available = candidates.is_ok();
        let privacy_available = privacy.is_ok();
        let guarded_available = guarded.is_ok();
        if let Err(error) = &canonical {
            issues.push(HealthIssue {
                kind: classify_context_error(error.kind()),
                scope: "canonical".into(),
                detail: error.to_string(),
            });
        }
        if let Err(error) = &candidates {
            issues.push(HealthIssue {
                kind: HealthIssueKind::Failed,
                scope: "candidates".into(),
                detail: error.to_string(),
            });
        }
        if let Err(error) = &privacy {
            issues.push(HealthIssue {
                kind: HealthIssueKind::Failed,
                scope: "privacy".into(),
                detail: error.to_string(),
            });
        }
        if let Err(error) = &guarded {
            issues.push(HealthIssue {
                kind: HealthIssueKind::Failed,
                scope: "guarded_operations".into(),
                detail: error.to_string(),
            });
        }
        let repository_available = project_id.and_then(|id| match &canonical {
            Ok(store) => match store.get_local_binding(id) {
                Ok(binding) => {
                    let available = binding.absolute_path.is_dir();
                    if !available {
                        issues.push(HealthIssue {
                            kind: HealthIssueKind::Unavailable,
                            scope: "repository".into(),
                            detail: format!(
                                "bound repository {} is unavailable",
                                binding.absolute_path.display()
                            ),
                        });
                    }
                    Some(available)
                }
                Err(error) => {
                    issues.push(HealthIssue {
                        kind: HealthIssueKind::Unavailable,
                        scope: "repository_binding".into(),
                        detail: error.to_string(),
                    });
                    Some(false)
                }
            },
            Err(_) => None,
        });
        if let Some(project_id) = project_id {
            if let Err(error) = self.load_analyses(project_id) {
                issues.push(HealthIssue {
                    kind: HealthIssueKind::Corrupt,
                    scope: format!("derived_analysis:{project_id}"),
                    detail: error.to_string(),
                });
            }
        }
        let state = if !canonical_available {
            HealthState::Failed
        } else if issues.is_empty() {
            HealthState::Healthy
        } else {
            HealthState::Degraded
        };
        HealthReport {
            state,
            runtime_root: self.layout.root().to_path_buf(),
            canonical_available,
            candidate_available,
            privacy_available,
            guarded_available,
            repository_available,
            issues,
        }
    }

    pub fn analyze(
        &self,
        project_id: ProjectId,
        excluded_paths: Vec<String>,
    ) -> Result<LongOperationResult<AnalysisOutcome>, Error> {
        let operation_id = new_operation_id()?;
        let started_at = now_micros()?;
        let monotonic = Instant::now();
        let prepared = self.prepare_analysis_basis(project_id, started_at)?;
        self.analyze_from_basis(
            operation_id,
            started_at,
            monotonic,
            prepared.root,
            prepared.requested_path,
            prepared.canonical,
            prepared.repository_source,
            prepared.repository_worktree,
            excluded_paths,
            false,
        )
    }

    fn observe_repository_worktree(
        &self,
        root: &Path,
    ) -> Result<RepositoryWorktreeObservation, Error> {
        if GitWorktreeLayout::resolve(root)
            .map_err(|error| Error::with_source("cannot resolve Git worktree for analysis", error))?
            .is_none()
        {
            return Ok(RepositoryWorktreeObservation::NonGit);
        }
        let result = self.run_child(
            "git",
            [
                OsString::from("--no-optional-locks"),
                OsString::from("status"),
                OsString::from("--porcelain=v2"),
                OsString::from("-z"),
                OsString::from("--untracked-files=all"),
            ],
            Some(root),
            vec!["git_worktree_status".into()],
            Duration::from_secs(30),
            CancellationFlag::default(),
        )?;
        if result.state != OperationState::Succeeded {
            return Err(Error::new(format!(
                "Git worktree status observation failed; operation {}; diagnostic: {}",
                result.operation_id,
                result
                    .diagnostic
                    .as_deref()
                    .unwrap_or("process did not exit successfully")
            )));
        }
        let outcome = result.value.ok_or_else(|| {
            Error::new("Git worktree status observation has no preserved process result")
        })?;
        let bytes = fs::read(outcome.stdout.path()).map_err(|error| {
            Error::with_source("cannot read preserved Git worktree status stdout", error)
        })?;
        let observation = DirtyObservation::from_porcelain_v2(&bytes)
            .map_err(|error| Error::with_source("cannot parse Git worktree status", error))?;
        Ok(RepositoryWorktreeObservation::Git {
            status_fingerprint: observation.fingerprint().to_owned(),
            dirty_paths: observation.dirty_paths().to_vec(),
        })
    }

    fn prepare_analysis_basis(
        &self,
        project_id: ProjectId,
        observed_at: TimestampMicros,
    ) -> Result<PreparedAnalysisBasis, Error> {
        let mut canonical = self.open_canonical()?;
        let project = canonical
            .get_project(project_id)
            .map_err(|error| Error::with_source("cannot read Project for analysis", error))?;
        let binding = canonical.get_local_binding(project_id).map_err(|error| {
            Error::with_source("Project has no usable repository binding", error)
        })?;
        let root = RepositoryRoot::open(&binding.absolute_path)
            .map_err(|error| Error::with_source("bound repository is unavailable", error))?;
        let repository_worktree = self.observe_repository_worktree(root.canonical_path())?;
        let revision =
            repository_observation_basis(root.canonical_path(), observed_at.as_unix_micros())?;
        let source = canonical
            .record_source(
                new_operation_id()?,
                project_id,
                SourceDraft {
                    expected_project_revision: project.revision,
                    payload: SourcePayload::RepositorySnapshot { revision },
                    actor: Principal {
                        kind: PrincipalKind::Repository,
                        identity: "local-repository-observer".into(),
                    },
                    observer: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: "volicord-local-operations".into(),
                    }),
                    availability: Availability::Available,
                },
            )
            .map_err(|error| {
                Error::with_source("cannot record repository observation Source", error)
            })?;
        let basis = canonical
            .read_canonical_basis(project_id, CanonicalReadOptions::default())
            .map_err(|error| Error::with_source("cannot read canonical analysis basis", error))?;
        Ok(PreparedAnalysisBasis {
            root,
            requested_path: binding.absolute_path,
            canonical: basis,
            repository_source: source.value.id,
            repository_worktree,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn analyze_from_basis(
        &self,
        operation_id: OperationId,
        started_at: TimestampMicros,
        monotonic: Instant,
        root: RepositoryRoot,
        requested_path: PathBuf,
        basis: CanonicalReadBasis,
        repository_source: SourceId,
        repository_worktree: RepositoryWorktreeObservation,
        excluded_paths: Vec<String>,
        replace_existing: bool,
    ) -> Result<LongOperationResult<AnalysisOutcome>, Error> {
        let grounding = CanonicalGrounding::from_read_basis(&basis).map_err(|error| {
            Error::with_source("canonical analysis grounding is invalid", error)
        })?;
        let mut inventory = InventoryRequest::new(
            root.canonical_path(),
            &grounding,
            repository_source,
            started_at.as_unix_micros(),
        )
        .map_err(|error| Error::with_source("cannot create repository inventory request", error))?
        .with_repository_worktree(repository_worktree);
        inventory.excluded_paths = excluded_paths;
        let (repository, analysis) = analyze_repository(StructuralAnalysisRequest::new(inventory))
            .map_err(|error| Error::with_source("repository analysis failed", error))?;
        let stored_at = if replace_existing {
            self.replace_analysis(&analysis)?
        } else {
            self.store_analysis(&analysis)?
        };
        let mut completed_scopes = Vec::new();
        let mut failed_scopes = Vec::new();
        let mut omitted_scopes = Vec::new();
        for capability in &analysis.capabilities {
            let scope = format!(
                "{:?}:{:?}:{}",
                capability.capability, capability.language, capability.area.path
            );
            match capability.state {
                CapabilityState::Available => completed_scopes.push(scope),
                CapabilityState::Partial | CapabilityState::Failed => failed_scopes.push(scope),
                CapabilityState::Unavailable
                | CapabilityState::Unsupported
                | CapabilityState::Stale => omitted_scopes.push(scope),
            }
        }
        let state = if failed_scopes.is_empty() && omitted_scopes.is_empty() {
            OperationState::Succeeded
        } else {
            OperationState::Partial
        };
        let duration = duration_micros(monotonic.elapsed());
        Ok(LongOperationResult {
            operation_id,
            requested_scope: vec![requested_path.display().to_string()],
            state,
            started_at_unix_micros: started_at.as_unix_micros(),
            ended_at_unix_micros: now_micros()?.as_unix_micros(),
            duration_micros: duration,
            progress: ProgressState {
                phase: "analysis_complete".into(),
                unit: None,
                completed: analysis.inventory.entries.len() as u64,
                total: Some(analysis.inventory.entries.len() as u64),
                active: false,
            },
            partial: PartialOutcome {
                completed_scopes,
                failed_scopes,
                omitted_scopes,
            },
            value: Some(AnalysisOutcome {
                repository,
                analysis,
                stored_at,
            }),
            diagnostic: None,
        })
    }

    pub fn rebuild_analysis(
        &self,
        project_id: ProjectId,
        excluded_paths: Vec<String>,
    ) -> Result<LongOperationResult<AnalysisOutcome>, Error> {
        let operation_id = new_operation_id()?;
        let started_at = now_micros()?;
        let monotonic = Instant::now();
        let prepared = self.prepare_analysis_basis(project_id, started_at)?;
        self.analyze_from_basis(
            operation_id,
            started_at,
            monotonic,
            prepared.root,
            prepared.requested_path,
            prepared.canonical,
            prepared.repository_source,
            prepared.repository_worktree,
            excluded_paths,
            true,
        )
    }

    pub fn repair(
        &self,
        project_id: ProjectId,
        scope: impl Into<String>,
        excluded_paths: Vec<String>,
    ) -> Result<RepairOutcome, Error> {
        let scope = scope.into();
        if scope != "derived-analysis" {
            return Err(Error::new(format!(
                "unsupported repair scope {scope:?}; supported scope: derived-analysis"
            )));
        }
        let diagnosis = match self.load_analyses(project_id) {
            Ok(values) if values.is_empty() => "derived analysis is missing".to_owned(),
            Ok(_) => {
                "derived analysis is readable; forced verification rebuild requested".to_owned()
            }
            Err(error) => format!("derived analysis is corrupt: {error}"),
        };
        let discarded_entries = self.project_analysis_entry_count(project_id)?;
        let operation = self.rebuild_analysis(project_id, excluded_paths)?;
        Ok(RepairOutcome {
            kind: RepairKind::DerivedAnalysisRepair,
            affected_scope: format!("project:{project_id}:derived-analysis"),
            diagnosis,
            discarded_entries,
            operation,
        })
    }

    pub fn reindex(
        &self,
        project_id: ProjectId,
        excluded_paths: Vec<String>,
    ) -> Result<RepairOutcome, Error> {
        let discarded_entries = self.project_analysis_entry_count(project_id)?;
        let operation = self.rebuild_analysis(project_id, excluded_paths)?;
        Ok(RepairOutcome {
            kind: RepairKind::DerivedRebuild,
            affected_scope: format!("project:{project_id}:derived-analysis"),
            diagnosis: "forced derived-analysis reconstruction requested".into(),
            discarded_entries,
            operation,
        })
    }

    pub fn run_child(
        &self,
        program: impl AsRef<OsStr>,
        arguments: impl IntoIterator<Item = OsString>,
        current_dir: Option<&Path>,
        requested_scope: Vec<String>,
        timeout: Duration,
        cancellation: CancellationFlag,
    ) -> Result<LongOperationResult<ChildProcessOutcome>, Error> {
        self.initialize_runtime()?;
        let operation_id = new_operation_id()?;
        let started_at = now_micros()?;
        let operation_dir = self.layout.artifacts_dir().join(operation_id.to_string());
        fs::create_dir(&operation_dir).map_err(|error| {
            Error::with_source("cannot create child-process artifact directory", error)
        })?;
        let mut request = ProcessRequest::new(
            program,
            operation_dir.join("stdout"),
            operation_dir.join("stderr"),
            timeout,
            Duration::from_secs(2),
        )
        .args(arguments)
        .cancellation(cancellation);
        if let Some(directory) = current_dir {
            request = request.current_dir(directory);
        }
        let observation = match request.run() {
            Ok(value) => value,
            Err(error) => {
                return Ok(LongOperationResult {
                    operation_id,
                    requested_scope,
                    state: OperationState::Failed,
                    started_at_unix_micros: started_at.as_unix_micros(),
                    ended_at_unix_micros: now_micros()?.as_unix_micros(),
                    duration_micros: duration_micros(error.duration()),
                    progress: ProgressState {
                        phase: "spawn_failed".into(),
                        unit: None,
                        completed: 0,
                        total: None,
                        active: false,
                    },
                    partial: empty_partial(),
                    value: None,
                    diagnostic: Some(error.detail().to_owned()),
                });
            }
        };
        let state = process_state(&observation);
        let progress_phase = format!("{:?}", state).to_lowercase();
        let value = ChildProcessOutcome::from_observation(&observation);
        Ok(LongOperationResult {
            operation_id,
            requested_scope,
            state,
            started_at_unix_micros: started_at.as_unix_micros(),
            ended_at_unix_micros: now_micros()?.as_unix_micros(),
            duration_micros: duration_micros(observation.duration()),
            progress: ProgressState {
                phase: progress_phase,
                unit: None,
                completed: 1,
                total: Some(1),
                active: false,
            },
            partial: empty_partial(),
            value: Some(value),
            diagnostic: match observation.completion() {
                ProcessCompletion::ObservationFailed { detail, .. } => Some(detail.clone()),
                ProcessCompletion::Exited(_) => None,
            },
        })
    }

    pub fn export_bundle(
        &self,
        project_id: ProjectId,
        destination: &Path,
    ) -> Result<volicord_context::BundleExport, Error> {
        let mut canonical = self.open_canonical()?;
        canonical
            .export_bundle(project_id, destination)
            .map_err(|error| Error::with_source("portable export failed", error))
    }

    pub fn import_bundle(&self, source: &Path) -> Result<volicord_context::BundleImport, Error> {
        self.initialize_runtime()?;
        let mut canonical = self.open_canonical()?;
        canonical
            .import_bundle(new_operation_id()?, source)
            .map(|result| result.value)
            .map_err(|error| Error::with_source("portable import failed", error))
    }

    pub fn compare_portable_bundle(
        &self,
        common_base: Option<&Path>,
        incoming: &Path,
    ) -> Result<BundleComparison, Error> {
        self.open_canonical()?
            .compare_bundle(common_base, incoming, None)
            .map_err(|error| Error::with_source("portable comparison failed", error))
    }

    pub fn merge_portable_bundle(
        &self,
        common_base: Option<&Path>,
        incoming: &Path,
        resolution: Option<MergeResolution>,
    ) -> Result<OperationResult<BundleMerge>, Error> {
        self.open_canonical()?
            .merge_bundle(new_operation_id()?, common_base, incoming, None, resolution)
            .map_err(|error| Error::with_source("portable merge failed", error))
    }

    pub fn canonical_basis(&self, project_id: ProjectId) -> Result<CanonicalReadBasis, Error> {
        self.open_canonical()?
            .read_canonical_basis(
                project_id,
                CanonicalReadOptions {
                    include_checkpoint_history: true,
                },
            )
            .map_err(|error| Error::with_source("canonical inspection failed", error))
    }

    pub fn candidate_basis(&self, project_id: ProjectId) -> Result<CandidateReadBasis, Error> {
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|store| store.read_basis(project_id))
            .map_err(|error| Error::with_source("Candidate inspection failed", error))
    }

    pub fn submit_candidate(&self, draft: CandidateDraft) -> Result<SubmissionOutcome, Error> {
        self.initialize_runtime()?;
        let _ = self.canonical_basis(draft.project_id)?;
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| store.submit(draft))
            .map_err(|error| Error::with_source("Candidate submission failed", error))
    }

    pub fn attach_candidate_repository_research(
        &self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        draft: CandidateRepositoryResearchDraft,
    ) -> Result<CandidateRecord, Error> {
        let canonical = self.canonical_basis(project_id)?;
        let analysis = self.load_analyses(project_id)?.pop().ok_or_else(|| {
            Error::new("Candidate repository research requires a current Analysis Snapshot")
        })?;
        let basis = RepositoryResearchBasis {
            repository_snapshot: analysis.repository_snapshot.to_string(),
            analysis_snapshot: Some(analysis.identity.to_string()),
            capability: draft.capability,
            coverage: draft.coverage,
            freshness: draft.freshness,
            source_basis: draft.source_basis,
            sufficient: draft.sufficient,
            limits: draft.limits,
        };
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| {
                store.attach_repository_research(
                    project_id,
                    candidate_id,
                    &canonical,
                    &analysis,
                    basis,
                )
            })
            .map_err(|error| Error::with_source("Candidate research attachment failed", error))
    }

    pub fn mark_candidate_ready_to_ask(
        &self,
        project_id: ProjectId,
        candidate_id: CandidateId,
    ) -> Result<CandidateRecord, Error> {
        let _ = self.canonical_basis(project_id)?;
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| {
                store.set_research_state(
                    project_id,
                    candidate_id,
                    volicord_context::QuestionResearchState::ReadyToAsk,
                )
            })
            .map_err(|error| Error::with_source("Candidate research transition failed", error))
    }

    pub fn promote_question_candidate(
        &self,
        project_id: ProjectId,
        candidate_id: CandidateId,
    ) -> Result<PromotionResult, Error> {
        let mut canonical = self.open_canonical()?;
        let basis = canonical
            .read_canonical_basis(
                project_id,
                CanonicalReadOptions {
                    include_checkpoint_history: true,
                },
            )
            .map_err(|error| Error::with_source("canonical inspection failed", error))?;
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| {
                store.promote_question(&mut canonical, &basis, project_id, candidate_id)
            })
            .map_err(|error| Error::with_source("Question Candidate promotion failed", error))
    }

    pub fn dismiss_candidate(
        &self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        reason: impl Into<String>,
    ) -> Result<CandidateRecord, Error> {
        let _ = self.canonical_basis(project_id)?;
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| store.dismiss(project_id, candidate_id, reason))
            .map_err(|error| Error::with_source("Candidate dismissal failed", error))
    }

    pub fn delete_candidate(
        &self,
        project_id: ProjectId,
        candidate_id: CandidateId,
        basis: impl Into<String>,
    ) -> Result<CandidateRecord, Error> {
        let _ = self.canonical_basis(project_id)?;
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| store.delete_candidate(project_id, candidate_id, basis))
            .map_err(|error| Error::with_source("Candidate deletion failed", error))
    }

    pub fn privacy_status(&self, project_id: ProjectId) -> Result<ProjectPrivacyInspection, Error> {
        PrivacyStore::open(self.layout.privacy_store())
            .and_then(|store| store.inspect_project(project_id))
            .map_err(|error| Error::with_source("privacy inspection failed", error))
    }

    pub fn enable_provider(
        &self,
        policy: ProviderOptInPolicy,
        intent: ProviderIntentProvenance,
    ) -> Result<ProviderOptInEvent, Error> {
        let canonical = self.canonical_basis(policy.project_id)?;
        let mut privacy = PrivacyStore::open(self.layout.privacy_store())
            .map_err(|error| Error::with_source("cannot open privacy store", error))?;
        privacy
            .enable(&canonical, policy, intent)
            .map_err(|error| Error::with_source("provider opt-in failed", error))
    }

    pub fn disable_provider(
        &self,
        project_id: ProjectId,
        intent: ProviderIntentProvenance,
    ) -> Result<ProviderOptInEvent, Error> {
        let canonical = self.canonical_basis(project_id)?;
        let mut privacy = PrivacyStore::open(self.layout.privacy_store())
            .map_err(|error| Error::with_source("cannot open privacy store", error))?;
        privacy
            .disable(&canonical, project_id, intent)
            .map_err(|error| Error::with_source("provider disable failed", error))
    }

    pub fn revoke_provider(
        &self,
        project_id: ProjectId,
        intent: ProviderIntentProvenance,
    ) -> Result<ProviderOptInEvent, Error> {
        let canonical = self.canonical_basis(project_id)?;
        let mut privacy = PrivacyStore::open(self.layout.privacy_store())
            .map_err(|error| Error::with_source("cannot open privacy store", error))?;
        privacy
            .revoke(&canonical, project_id, intent)
            .map_err(|error| Error::with_source("provider revoke failed", error))
    }

    pub fn prepare_guarded_provider_operation(
        &self,
        draft: BackgroundProviderOperationDraft,
    ) -> Result<GuardedProviderPreparationOutcome, Error> {
        self.initialize_runtime()?;
        if draft.source_paths.is_empty() {
            return Err(Error::new(
                "background provider operation requires at least one explicit source path",
            ));
        }
        let unique_paths = draft.source_paths.iter().collect::<BTreeSet<_>>();
        if unique_paths.len() != draft.source_paths.len() {
            return Err(Error::new(
                "background provider source paths must not contain duplicates",
            ));
        }
        let canonical = self.canonical_basis(draft.project_id)?;
        let binding = self
            .open_canonical()?
            .get_local_binding(draft.project_id)
            .map_err(|error| {
                Error::with_source("Project has no usable repository binding", error)
            })?;
        let root = RepositoryRoot::open(&binding.absolute_path)
            .map_err(|error| Error::with_source("bound repository is unavailable", error))?;
        let analysis = self.load_analyses(draft.project_id)?.pop().ok_or_else(|| {
            Error::new("background semantic preparation requires a current local Analysis Snapshot")
        })?;
        let mut sources = Vec::with_capacity(draft.source_paths.len());
        for locator in &draft.source_paths {
            sources.push(background_source(&root, &analysis, locator)?);
        }

        let mut privacy = PrivacyStore::open(self.layout.privacy_store())
            .map_err(|error| Error::with_source("cannot open privacy store", error))?;
        let preparation = privacy
            .prepare_background_request(BackgroundSemanticRequest {
                project_id: draft.project_id,
                repository_snapshot: analysis.repository_snapshot,
                analysis_snapshot: analysis.identity,
                provider: draft.provider,
                model: draft.model,
                purpose: draft.purpose,
                requested_capability: draft.requested_capability,
                requested_source_scopes: draft.source_paths,
                sources,
            })
            .map_err(|error| {
                Error::with_source("background provider request preparation failed", error)
            })?;
        let authorized = match preparation {
            PreparationOutcome::Ready(value) => value,
            PreparationOutcome::Rejected(record) => {
                return Ok(GuardedProviderPreparationOutcome::Rejected(Box::new(
                    record,
                )))
            }
        };
        let provider_request = authorized.record().clone();
        let candidate = self.create_guarded_request(GuardedEffectDraft {
            project_id: draft.project_id,
            exact_action: "transmit_source_for_background_semantic_processing".into(),
            target: format!(
                "provider:{}/model:{}",
                provider_request.provider, provider_request.model
            ),
            expected_effect: format!(
                "transmit {} privacy-filtered source file(s) for {} using provider request {}",
                provider_request
                    .manifest
                    .iter()
                    .filter(|entry| {
                        entry.scope_outcome == volicord_privacy::ScopeOutcome::Included
                    })
                    .count(),
                provider_request.purpose,
                provider_request.id
            ),
            risk: GuardedRisk {
                category: GuardedEffectCategory::PersonalDataOrSourceCodeExternalTransmission,
                concrete_consequence:
                    "the authorized, filtered source manifest may leave the local environment"
                        .into(),
            },
            scope: provider_guarded_scope(&provider_request),
            expires_at: draft.expires_at,
            requesting_provenance: draft.requesting_provenance,
        })?;
        if canonical.project.id != candidate.project_id {
            return Err(Error::new(
                "prepared provider request changed Project before Guarded creation",
            ));
        }
        Ok(GuardedProviderPreparationOutcome::Ready(Box::new(
            GuardedProviderPreparation {
                candidate,
                provider_request,
                authorized: Some(authorized),
            },
        )))
    }

    pub fn dispatch_guarded_provider(
        &self,
        preparation: &mut GuardedProviderPreparation,
        request_revision: u64,
        effect_fingerprint: &str,
        provider: &mut dyn BackgroundSemanticProvider,
    ) -> Result<GuardedOperationResult, Error> {
        let mut expectation = DispatchExpectation::from(&preparation.candidate);
        expectation.effect_fingerprint = effect_fingerprint.to_owned();
        let mut privacy = PrivacyStore::open(self.layout.privacy_store())
            .map_err(|error| Error::with_source("cannot open privacy store", error))?;
        let mut dispatcher = BackgroundProviderDispatcher::new(
            &mut privacy,
            &mut preparation.authorized,
            &preparation.candidate,
            provider,
        );
        self.dispatch_guarded(
            preparation.candidate.confirmation_request_identity,
            request_revision,
            &expectation,
            &mut dispatcher,
        )
    }

    pub fn dispatch_guarded_provider_with_configured_adapter(
        &self,
        preparation: &mut GuardedProviderPreparation,
        request_revision: u64,
        effect_fingerprint: &str,
    ) -> Result<GuardedOperationResult, Error> {
        let mut provider = UnavailableConfiguredProvider {
            identity: ProviderIdentity {
                provider: preparation.provider_request.provider.clone(),
                model: preparation.provider_request.model.clone(),
            },
        };
        self.dispatch_guarded_provider(
            preparation,
            request_revision,
            effect_fingerprint,
            &mut provider,
        )
    }

    pub fn guarded_operation(
        &self,
        operation: GuardedOperationId,
    ) -> Result<GuardedOperationResult, Error> {
        GuardedStore::open(self.layout.guarded_store())?.operation(operation)
    }

    pub fn inspect_guarded_provider_operation(
        &self,
        project_id: ProjectId,
        operation_id: GuardedOperationId,
        provider_request_id: ProviderRequestId,
    ) -> Result<GuardedProviderInspection, Error> {
        let operation = self.guarded_operation(operation_id)?;
        let request = GuardedStore::open(self.layout.guarded_store())?.request(
            operation.confirmation_request_identity,
            operation.request_revision,
        )?;
        if request.project_id != project_id {
            return Err(Error::new(
                "Guarded provider operation belongs to another Project",
            ));
        }
        let provider_request = PrivacyStore::open(self.layout.privacy_store())
            .and_then(|store| store.provider_request(project_id, provider_request_id))
            .map_err(|error| Error::with_source("provider request inspection failed", error))?;
        if !request
            .scope
            .contains(&format!("provider_request:{}", provider_request.id))
        {
            return Err(Error::new(
                "provider request is not bound to the inspected Guarded operation",
            ));
        }
        Ok(GuardedProviderInspection {
            request,
            operation,
            provider_request,
        })
    }

    pub fn project_projection(&self, project_id: ProjectId) -> Result<ProjectProjection, Error> {
        let canonical = self.canonical_basis(project_id)?;
        let analyses = self.load_analyses(project_id)?;
        let analysis_refs = analyses.iter().collect::<Vec<_>>();
        let candidates = self.candidate_basis(project_id).ok();
        Ok(build_project_projection(ProjectProjectionInputs {
            canonical: &canonical,
            analyses: &analysis_refs,
            applicability: empty_applicability(project_id),
            candidates: candidates.as_ref(),
            candidate_content_access: CandidateContentAccess::AllowBoundedSummary,
            observed_at: now_micros()?,
            bound: ProjectionBound::default(),
        }))
    }

    pub fn recall(&self, project_id: ProjectId) -> Result<ResumeBrief, Error> {
        let canonical = self.canonical_basis(project_id)?;
        let analyses = self.load_analyses(project_id)?;
        let analysis_refs = analyses.iter().collect::<Vec<_>>();
        Ok(volicord_projections::build_resume_brief(RecallInputs {
            canonical: &canonical,
            analyses: &analysis_refs,
            scope: empty_applicability(project_id),
            bound: RecallBound::default(),
        }))
    }

    pub fn documents(
        &self,
        project_id: ProjectId,
        request: &DocumentRequest,
    ) -> Result<DocumentSet, Error> {
        let projection = self.project_projection(project_id)?;
        generate_documents(&projection, request)
            .map_err(|error| Error::with_source("document generation failed", error))
    }

    pub fn publish_document(
        &self,
        document: &GeneratedDocument,
        format: OutputFormat,
        destination: &Path,
    ) -> Result<PublicationOutcome, Error> {
        let artifact = match format {
            OutputFormat::Markdown => &document.markdown,
            OutputFormat::Html => &document.html,
        };
        publish_bytes_no_replace(destination, artifact.content.as_bytes())
    }

    pub fn inquiry_frontier(
        &self,
        project_id: ProjectId,
        material_scope: Vec<String>,
    ) -> Result<FrontierRead, Error> {
        let canonical = self.canonical_basis(project_id)?;
        Ok(compute_frontier(
            &canonical,
            &InquiryScope {
                project_id,
                material_scope,
            },
        ))
    }

    pub fn record_inquiry_responses(
        &self,
        project_id: ProjectId,
        responses: Vec<BatchResponseItem>,
    ) -> Result<BatchResponseResult, Error> {
        let mut canonical = self.open_canonical()?;
        Ok(record_response_batch(&mut canonical, project_id, responses))
    }

    pub fn record_user_source(
        &self,
        project_id: ProjectId,
        host: String,
        session: String,
        turn: String,
    ) -> Result<CanonicalMutationOutcome, Error> {
        let source = self.create_user_source(project_id, host, session, turn)?;
        Ok(CanonicalMutationOutcome {
            record_kind: "source".into(),
            identity: source.id.to_string(),
            revision: Some(1),
            replayed: false,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_current_host_user_context(
        &self,
        project_id: ProjectId,
        host: String,
        session: String,
        user_turn: String,
        role: ContextItemRole,
        statement: String,
    ) -> Result<UserContextRecordingOutcome, Error> {
        if !user_turn.contains(&statement) {
            return Err(Error::new(
                "user Context statement must occur verbatim in the exact current-host user turn",
            ));
        }
        let source = self.create_user_source(project_id, host, session, user_turn)?;
        let mut canonical = self.open_canonical()?;
        let project = canonical
            .get_project(project_id)
            .map_err(|error| Error::with_source("cannot read Project", error))?;
        let item = canonical
            .record_context_item(
                new_operation_id()?,
                project_id,
                ContextItemDraft {
                    expected_project_revision: project.revision,
                    role,
                    statement,
                    provenance_role: StatementProvenanceRole::UserStatement,
                    author: Principal {
                        kind: PrincipalKind::User,
                        identity: "current-host-user".into(),
                    },
                    source_basis: vec![source.id],
                    applicability: Default::default(),
                },
            )
            .map_err(|error| Error::with_source("user Context recording failed", error))?
            .value;
        Ok(UserContextRecordingOutcome {
            source_id: source.id,
            context_item_id: item.id,
            context_item_revision: item.revision,
            role: item.role,
        })
    }

    pub fn create_guarded_request(
        &self,
        draft: GuardedEffectDraft,
    ) -> Result<GuardedEffectCandidate, Error> {
        self.initialize_runtime()?;
        GuardedStore::open(self.layout.guarded_store())?.create_request(draft, now_micros()?)
    }

    pub fn revise_guarded_request(
        &self,
        request: ConfirmationRequestId,
        expected_revision: u64,
        draft: GuardedEffectDraft,
    ) -> Result<GuardedEffectCandidate, Error> {
        GuardedStore::open(self.layout.guarded_store())?.revise_request(
            request,
            expected_revision,
            draft,
            now_micros()?,
        )
    }

    pub fn guarded_request(
        &self,
        request: ConfirmationRequestId,
    ) -> Result<GuardedEffectCandidate, Error> {
        GuardedStore::open(self.layout.guarded_store())?.current_request(request)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_confirmation(
        &self,
        request_identity: ConfirmationRequestId,
        request_revision: u64,
        effect_fingerprint: &str,
        decision: ConfirmationDecision,
        host: String,
        session: String,
        user_turn: String,
    ) -> Result<ConfirmationResponse, Error> {
        let mut guarded = GuardedStore::open(self.layout.guarded_store())?;
        let request = guarded.current_request(request_identity)?;
        if request.request_revision != request_revision
            || request.effect_fingerprint != effect_fingerprint
        {
            return Err(Error::new(
                "confirmation does not match the current logical request identity, revision, and fingerprint",
            ));
        }
        let source = self.create_user_source(request.project_id, host, session, user_turn)?;
        let response =
            ConfirmationResponse::exact_for(&request, decision, source.id, now_micros()?)?;
        guarded.record_response(response)
    }

    pub fn dispatch_guarded(
        &self,
        request_identity: ConfirmationRequestId,
        request_revision: u64,
        expectation: &DispatchExpectation,
        dispatcher: &mut dyn GuardedEffectDispatcher,
    ) -> Result<GuardedOperationResult, Error> {
        let request = self.guarded_request(request_identity)?;
        let canonical = self.canonical_basis(request.project_id)?;
        GuardedStore::open(self.layout.guarded_store())?.dispatch(
            request_identity,
            request_revision,
            expectation,
            &canonical,
            now_micros()?,
            dispatcher,
        )
    }

    pub fn correct_context_item(
        &self,
        project_id: ProjectId,
        item_id: ContextItemId,
        draft: ContextItemCorrectionDraft,
    ) -> Result<CanonicalMutationOutcome, Error> {
        let mut canonical = self.open_canonical()?;
        let result = canonical
            .correct_context_item(new_operation_id()?, project_id, item_id, draft)
            .map_err(|error| Error::with_source("Context Item correction failed", error))?;
        Ok(CanonicalMutationOutcome {
            record_kind: "context_item".into(),
            identity: result.value.id.to_string(),
            revision: Some(result.value.revision),
            replayed: result.replayed,
        })
    }

    pub fn correct_decision(
        &self,
        project_id: ProjectId,
        decision_id: DecisionId,
        draft: DecisionCorrectionDraft,
    ) -> Result<CanonicalMutationOutcome, Error> {
        let mut canonical = self.open_canonical()?;
        let result = canonical
            .correct_decision(new_operation_id()?, project_id, decision_id, draft)
            .map_err(|error| Error::with_source("Decision correction failed", error))?;
        Ok(CanonicalMutationOutcome {
            record_kind: "decision".into(),
            identity: result.value.id.to_string(),
            revision: Some(result.value.revision),
            replayed: result.replayed,
        })
    }

    pub fn supersede_decision(
        &self,
        project_id: ProjectId,
        draft: DecisionSupersessionDraft,
    ) -> Result<CanonicalMutationOutcome, Error> {
        let mut canonical = self.open_canonical()?;
        let result = canonical
            .supersede_decision(new_operation_id()?, project_id, draft)
            .map_err(|error| Error::with_source("Decision supersession failed", error))?;
        Ok(CanonicalMutationOutcome {
            record_kind: "decision".into(),
            identity: result.value.id.to_string(),
            revision: Some(result.value.revision),
            replayed: result.replayed,
        })
    }

    pub fn supersede_decision_choice(
        &self,
        project_id: ProjectId,
        previous_decision_id: DecisionId,
        user_source_id: SourceId,
        alternative_key: String,
        user_rationale: Option<String>,
    ) -> Result<CanonicalMutationOutcome, Error> {
        let canonical = self.canonical_basis(project_id)?;
        let previous = canonical
            .active_decisions
            .iter()
            .chain(canonical.superseded_decisions.iter())
            .find(|lifecycle| lifecycle.decision.id == previous_decision_id)
            .ok_or_else(|| Error::new("previous Decision was not found"))?;
        self.supersede_decision(
            project_id,
            DecisionSupersessionDraft {
                expected_project_revision: canonical.project.revision,
                previous_decision_id,
                user_turn_source: UserTurnSource::Existing(user_source_id),
                choice: DecisionChoice::Alternative { alternative_key },
                user_rationale,
                applicability: previous.decision.applicability.clone(),
                assumptions: previous.decision.assumptions.clone(),
                revisit_triggers: previous.decision.revisit_triggers.clone(),
            },
        )
    }

    pub fn forget_record(
        &self,
        project_id: ProjectId,
        record: CanonicalRecordId,
        authorization: SourceId,
    ) -> Result<CanonicalMutationOutcome, Error> {
        let mut canonical = self.open_canonical()?;
        let operation = new_operation_id()?;
        let result = match record {
            CanonicalRecordId::Source(id) => {
                canonical.forget_source(operation, project_id, id, authorization)
            }
            CanonicalRecordId::Question(id) => {
                canonical.forget_question(operation, project_id, id, authorization)
            }
            CanonicalRecordId::Decision(id) => {
                canonical.forget_decision(operation, project_id, id, authorization)
            }
            CanonicalRecordId::ContextItem(id) => {
                canonical.forget_context_item(operation, project_id, id, authorization)
            }
            CanonicalRecordId::Checkpoint(id) => {
                canonical.forget_checkpoint(operation, project_id, id, authorization)
            }
            CanonicalRecordId::Project(_) => {
                return Err(Error::new(
                    "Project forgetting is not exposed by the Canonical Context owner",
                ))
            }
        }
        .map_err(|error| Error::with_source("canonical forgetting failed", error))?;
        let (record_kind, identity) = canonical_record_parts(result.value.tombstone.record);
        Ok(CanonicalMutationOutcome {
            record_kind,
            identity,
            revision: None,
            replayed: result.replayed,
        })
    }

    pub fn record_checkpoint(
        &self,
        project_id: ProjectId,
        draft: CheckpointDraft,
    ) -> Result<CanonicalMutationOutcome, Error> {
        let mut canonical = self.open_canonical()?;
        let result = canonical
            .record_checkpoint(new_operation_id()?, project_id, draft)
            .map_err(|error| Error::with_source("Checkpoint recording failed", error))?;
        Ok(CanonicalMutationOutcome {
            record_kind: "checkpoint".into(),
            identity: result.value.id.to_string(),
            revision: Some(1),
            replayed: result.replayed,
        })
    }

    pub fn record_grounded_checkpoint(
        &self,
        draft: GroundedCheckpointDraft,
    ) -> Result<GroundedCheckpointOutcome, Error> {
        validate_verification_drafts(&draft.verification)?;
        let boundary_valid = match draft.kind {
            volicord_context::CheckpointKind::Completion => {
                draft.work_state == volicord_context::WorkState::Completed
                    && draft.handoff_to.is_none()
            }
            volicord_context::CheckpointKind::Pause => {
                draft.work_state == volicord_context::WorkState::Paused
                    && draft.handoff_to.is_none()
            }
            volicord_context::CheckpointKind::Handoff => draft.handoff_to.is_some(),
        };
        if !boundary_valid {
            return Err(Error::new(
                "Checkpoint kind, work state, and handoff target are inconsistent",
            ));
        }
        let initial_canonical = self.canonical_basis(draft.project_id)?;
        let goal = initial_canonical
            .context_items
            .iter()
            .find(|item| item.id == draft.goal_context_id)
            .ok_or_else(|| Error::new("Checkpoint Goal Context was not found in the Project"))?
            .clone();
        if goal.role != ContextItemRole::Goal
            || goal.provenance_role != StatementProvenanceRole::UserStatement
            || goal.author.kind != PrincipalKind::User
        {
            return Err(Error::new(
                "Checkpoint goal must reference a current user-stated Goal Context Item",
            ));
        }
        for source_id in &goal.source_basis {
            let source = initial_canonical
                .sources
                .iter()
                .find(|basis| basis.source.id == *source_id)
                .ok_or_else(|| Error::new("Checkpoint Goal Context Source is unavailable"))?;
            if source.freshness != volicord_context::SourceFreshness::Current
                || source.source.actor.kind != PrincipalKind::User
                || !matches!(
                    source.source.payload,
                    SourcePayload::CurrentHostUserTurn { .. }
                )
            {
                return Err(Error::new(
                    "Checkpoint Goal Context is not grounded by a current-host user Source",
                ));
            }
        }

        let mut unique_decisions = BTreeSet::new();
        for decision_id in &draft.applied_decisions {
            if !unique_decisions.insert(*decision_id) {
                return Err(Error::new("Checkpoint applied Decision IDs must be unique"));
            }
            if !initial_canonical
                .active_decisions
                .iter()
                .any(|lifecycle| lifecycle.decision.id == *decision_id)
            {
                return Err(Error::new(
                    "Checkpoint applied Decision is not current in this Project",
                ));
            }
        }

        let baseline =
            self.load_analysis_snapshot(draft.project_id, draft.baseline_analysis_snapshot_id)?;
        let excluded_paths = baseline
            .inventory
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .classifications
                    .contains(&InventoryClassification::Excluded)
            })
            .map(|entry| entry.area.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let current_result = self.analyze(draft.project_id, excluded_paths)?;
        let current_outcome = current_result.value.ok_or_else(|| {
            Error::new("current repository analysis completed without a usable snapshot")
        })?;
        let repository_work = RepositoryWorkBasis {
            baseline: &baseline,
            current: &current_outcome.analysis,
            pre_existing_dirty_paths: baseline.repository_worktree.dirty_paths().to_vec(),
        };
        let changed_paths = match attribute_repository_changes(draft.project_id, &repository_work) {
            ChangeAttribution::Attributed { changed_paths, .. } => changed_paths,
            ChangeAttribution::Ambiguous { reason, .. }
            | ChangeAttribution::Unavailable { reason, .. } => return Err(Error::new(reason)),
        };

        let current_assumptions = initial_canonical
            .context_items
            .iter()
            .filter(|item| item.role == ContextItemRole::Assumption)
            .map(|item| item.statement.clone())
            .collect::<Vec<_>>();
        let applicability = ApplicabilityQuery {
            project_id: draft.project_id,
            paths: changed_paths.clone(),
            components: draft.decision_components.clone(),
            work_contexts: draft.work_contexts.clone(),
            current_assumptions,
            met_revisit_triggers: draft.met_revisit_triggers.clone(),
        };
        for decision_id in &draft.applied_decisions {
            let lifecycle = initial_canonical
                .active_decisions
                .iter()
                .find(|lifecycle| lifecycle.decision.id == *decision_id)
                .ok_or_else(|| Error::new("Checkpoint applied Decision is not current"))?;
            let evaluation =
                evaluate_decision_applicability(&initial_canonical, lifecycle, &applicability);
            if evaluation.state != DecisionApplicabilityState::ReusableCurrent {
                return Err(Error::new(format!(
                    "Checkpoint applied Decision {} is not currently applicable: {:?}",
                    decision_id, evaluation.issues
                )));
            }
        }

        let mut verification = Vec::with_capacity(draft.verification.len());
        let mut verification_source_ids = Vec::new();
        for fact in &draft.verification {
            if fact.state == VerificationState::NotRun {
                verification.push(VerificationFact {
                    state: VerificationState::NotRun,
                    source_id: None,
                    outcome: None,
                });
                continue;
            }
            let source = self.record_command_source(draft.project_id, fact)?;
            verification_source_ids.push(source.id);
            verification.push(VerificationFact {
                state: fact.state,
                source_id: Some(source.id),
                outcome: fact.outcome.clone(),
            });
        }

        let canonical = self.canonical_basis(draft.project_id)?;
        let evaluation = evaluate_checkpoint_candidate(
            &canonical,
            CheckpointCandidate {
                project_id: draft.project_id,
                kind: draft.kind,
                goal: goal.statement,
                work_state: draft.work_state,
                state_change: draft.state_change,
                repository_work: Some(repository_work),
                supporting_sources: goal.source_basis,
                applied_decisions: draft.applied_decisions.clone(),
                verification,
                user_review: UserReviewFact {
                    state: UserReviewState::NotRequested,
                    source_id: None,
                },
                user_acceptance: UserAcceptanceFact {
                    state: UserAcceptanceState::NotRequested,
                    source_id: None,
                },
                known_limits: draft.known_limits,
                non_goals: draft.non_goals,
                next_step: draft.next_step,
                handoff_to: draft.handoff_to,
                status_only: false,
            },
        );
        if let CheckpointEvaluation::Rejected { detail, .. } = &evaluation {
            return Err(Error::new(format!(
                "source-grounded Checkpoint was rejected: {detail}"
            )));
        }
        let mut store = self.open_canonical()?;
        let checkpoint = persist_evaluated_checkpoint(
            &mut store,
            new_operation_id()?,
            draft.project_id,
            evaluation,
        )
        .map_err(|error| Error::with_source("Checkpoint recording failed", error))?
        .value;
        Ok(GroundedCheckpointOutcome {
            checkpoint_id: checkpoint.id,
            checkpoint_revision: checkpoint.revision,
            goal_context_id: draft.goal_context_id,
            baseline_analysis_snapshot_id: baseline.identity,
            current_analysis_snapshot_id: current_outcome.analysis.identity,
            baseline_repository_snapshot_id: baseline.repository_snapshot,
            current_repository_snapshot_id: current_outcome.analysis.repository_snapshot,
            changed_paths: checkpoint.changed_paths,
            applied_decisions: checkpoint.applied_decisions,
            verification_source_ids,
        })
    }

    fn record_command_source(
        &self,
        project_id: ProjectId,
        draft: &CommandVerificationDraft,
    ) -> Result<volicord_context::Source, Error> {
        let mut canonical = self.open_canonical()?;
        let project = canonical
            .get_project(project_id)
            .map_err(|error| Error::with_source("cannot read Project", error))?;
        canonical
            .record_source(
                new_operation_id()?,
                project_id,
                SourceDraft {
                    expected_project_revision: project.revision,
                    payload: SourcePayload::CommandExecution {
                        command_label: draft.command_label.clone().ok_or_else(|| {
                            Error::new("executed verification needs a command label")
                        })?,
                        outcome: CommandOutcome {
                            exit_code: draft.exit_code,
                            termination: draft.termination.ok_or_else(|| {
                                Error::new("executed verification needs a command termination")
                            })?,
                        },
                    },
                    actor: Principal {
                        kind: PrincipalKind::Command,
                        identity: "current-host-reported-command".into(),
                    },
                    observer: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: "codex".into(),
                    }),
                    availability: Availability::Available,
                },
            )
            .map(|result| result.value)
            .map_err(|error| Error::with_source("cannot record verification Source", error))
    }

    fn open_canonical(&self) -> Result<Store, Error> {
        Store::open(self.layout.canonical_store())
            .map_err(|error| Error::with_source("cannot open canonical store", error))
    }

    fn create_user_source(
        &self,
        project_id: ProjectId,
        host: String,
        session: String,
        turn: String,
    ) -> Result<volicord_context::Source, Error> {
        let mut canonical = self.open_canonical()?;
        let project = canonical
            .get_project(project_id)
            .map_err(|error| Error::with_source("cannot read Project", error))?;
        canonical
            .record_source(
                new_operation_id()?,
                project_id,
                SourceDraft {
                    expected_project_revision: project.revision,
                    payload: SourcePayload::CurrentHostUserTurn {
                        host: host.clone(),
                        session,
                        turn,
                    },
                    actor: Principal {
                        kind: PrincipalKind::User,
                        identity: "current-host-user".into(),
                    },
                    observer: Some(Principal {
                        kind: PrincipalKind::Agent,
                        identity: host,
                    }),
                    availability: Availability::Available,
                },
            )
            .map(|result| result.value)
            .map_err(|error| Error::with_source("cannot record current-host user Source", error))
    }

    fn store_analysis(&self, analysis: &AnalysisSnapshot) -> Result<PathBuf, Error> {
        let directory = self
            .layout
            .analysis_project_dir(analysis.project.identity());
        fs::create_dir_all(&directory).map_err(|error| {
            Error::with_source("cannot create Project analysis directory", error)
        })?;
        let bytes = serde_json::to_vec_pretty(analysis)
            .map_err(|error| Error::with_source("cannot serialize Analysis Snapshot", error))?;
        let path = directory.join(format!("{}.json", analysis.identity));
        publish_bytes_no_replace(&path, &bytes)?;
        Ok(path)
    }

    fn replace_analysis(&self, analysis: &AnalysisSnapshot) -> Result<PathBuf, Error> {
        let project_id = analysis.project.identity();
        let base = self.layout.analysis_dir();
        fs::create_dir_all(&base)
            .map_err(|error| Error::with_source("cannot create analysis directory", error))?;
        let destination = self.layout.analysis_project_dir(project_id);
        let staging = base.join(format!(".staging-{project_id}-{}", analysis.identity));
        let replaced = base.join(format!(".replaced-{project_id}-{}", analysis.identity));
        fs::create_dir(&staging).map_err(|error| {
            Error::with_source("cannot stage Project analysis replacement", error)
        })?;
        let bytes = serde_json::to_vec_pretty(analysis)
            .map_err(|error| Error::with_source("cannot serialize Analysis Snapshot", error))?;
        let file_name = format!("{}.json", analysis.identity);
        if let Err(error) = publish_bytes_no_replace(&staging.join(&file_name), &bytes) {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        let had_previous = destination.exists();
        if had_previous {
            fs::rename(&destination, &replaced).map_err(|error| {
                Error::with_source("cannot isolate prior Project analysis", error)
            })?;
        }
        if let Err(error) = fs::rename(&staging, &destination) {
            if had_previous {
                let _ = fs::rename(&replaced, &destination);
            }
            let _ = fs::remove_dir_all(&staging);
            return Err(Error::with_source(
                "cannot publish Project analysis replacement",
                error,
            ));
        }
        if had_previous {
            fs::remove_dir_all(&replaced).map_err(|error| {
                Error::with_source("cannot discard replaced Project analysis", error)
            })?;
        }
        Ok(destination.join(file_name))
    }

    fn project_analysis_entry_count(&self, project_id: ProjectId) -> Result<u64, Error> {
        let directory = self.layout.analysis_project_dir(project_id);
        if !directory.exists() {
            return Ok(0);
        }
        fs::read_dir(directory)
            .map_err(|error| Error::with_source("cannot inspect Project analysis directory", error))
            .map(|entries| entries.count() as u64)
    }

    fn load_analyses(&self, project_id: ProjectId) -> Result<Vec<AnalysisSnapshot>, Error> {
        let directory = self.layout.analysis_project_dir(project_id);
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut values = Vec::new();
        let entries = fs::read_dir(&directory)
            .map_err(|error| Error::with_source("cannot inspect analysis directory", error))?;
        for entry in entries {
            let entry = entry
                .map_err(|error| Error::with_source("cannot inspect analysis entry", error))?;
            if entry.path().extension().and_then(OsStr::to_str) != Some("json") {
                continue;
            }
            let bytes = fs::read(entry.path())
                .map_err(|error| Error::with_source("cannot read Analysis Snapshot", error))?;
            let value: AnalysisSnapshot = serde_json::from_slice(&bytes).map_err(|error| {
                Error::with_source(
                    format!(
                        "Analysis Snapshot {} is unsupported or corrupt",
                        entry.path().display()
                    ),
                    error,
                )
            })?;
            if value.project.identity() != project_id {
                return Err(Error::new(format!(
                    "Analysis Snapshot {} belongs to another Project",
                    entry.path().display()
                )));
            }
            values.push(value);
        }
        values.sort_by_key(|value| (value.generated_at_unix_micros, value.identity));
        if values.len() > 1 {
            let latest = values
                .pop()
                .ok_or_else(|| Error::new("analysis ordering failed"))?;
            values.clear();
            values.push(latest);
        }
        Ok(values)
    }

    fn load_analysis_snapshot(
        &self,
        project_id: ProjectId,
        analysis_id: AnalysisSnapshotId,
    ) -> Result<AnalysisSnapshot, Error> {
        let path = self
            .layout
            .analysis_project_dir(project_id)
            .join(format!("{analysis_id}.json"));
        let bytes = fs::read(&path).map_err(|error| {
            Error::with_source(
                format!("baseline Analysis Snapshot {analysis_id} is unavailable"),
                error,
            )
        })?;
        let value: AnalysisSnapshot = serde_json::from_slice(&bytes).map_err(|error| {
            Error::with_source(
                format!("baseline Analysis Snapshot {analysis_id} is unsupported or corrupt"),
                error,
            )
        })?;
        if value.identity != analysis_id || value.project.identity() != project_id {
            return Err(Error::new(
                "baseline Analysis Snapshot identity or Project binding is incompatible",
            ));
        }
        Ok(value)
    }
}

fn validate_verification_drafts(values: &[CommandVerificationDraft]) -> Result<(), Error> {
    for value in values {
        if value.state == VerificationState::NotRun {
            if value.command_label.is_some()
                || value.exit_code.is_some()
                || value.termination.is_some()
                || value.outcome.is_some()
            {
                return Err(Error::new(
                    "not-run verification cannot claim command execution or an outcome",
                ));
            }
            continue;
        }
        let label = value
            .command_label
            .as_deref()
            .ok_or_else(|| Error::new("executed verification needs a command label"))?;
        if label.is_empty() || label.chars().count() > 1_024 {
            return Err(Error::new(
                "verification command label must contain 1 to 1024 characters",
            ));
        }
        let outcome = value
            .outcome
            .as_deref()
            .ok_or_else(|| Error::new("executed verification needs an outcome"))?;
        if outcome.is_empty() || outcome.chars().count() > 16_384 {
            return Err(Error::new(
                "verification outcome must contain 1 to 16384 characters",
            ));
        }
        let termination = value
            .termination
            .ok_or_else(|| Error::new("executed verification needs a command termination"))?;
        match termination {
            volicord_context::CommandTermination::Exited if value.exit_code.is_none() => {
                return Err(Error::new(
                    "an exited verification command requires its actual exit code",
                ));
            }
            volicord_context::CommandTermination::Signaled
            | volicord_context::CommandTermination::SpawnFailed
            | volicord_context::CommandTermination::Indeterminate
                if value.exit_code.is_some() =>
            {
                return Err(Error::new(
                    "a non-exited verification command cannot claim an exit code",
                ));
            }
            _ => {}
        }
        if value.state == VerificationState::Passed
            && !(termination == volicord_context::CommandTermination::Exited
                && value.exit_code == Some(0))
        {
            return Err(Error::new(
                "passed verification requires an executed command that exited with code 0",
            ));
        }
        if value.state == VerificationState::Failed
            && termination == volicord_context::CommandTermination::Exited
            && value.exit_code == Some(0)
        {
            return Err(Error::new(
                "failed verification cannot claim a command that exited with code 0",
            ));
        }
    }
    Ok(())
}

fn background_source(
    root: &RepositoryRoot,
    analysis: &AnalysisSnapshot,
    locator: &str,
) -> Result<BackgroundSource, Error> {
    let resolved = root
        .resolve(locator)
        .map_err(|error| Error::with_source("background source path is invalid", error))?;
    if resolved.state() != RepositoryPathState::Existing || resolved.traversed_symlink() {
        return Err(Error::new(format!(
            "background source {locator:?} must be an existing non-symlink repository file"
        )));
    }
    let entry = analysis
        .inventory
        .entries
        .iter()
        .find(|entry| entry.area.path == locator && entry.entry_kind == EntryKind::File)
        .ok_or_else(|| {
            Error::new(format!(
                "background source {locator:?} is not a file in the current Analysis Snapshot"
            ))
        })?;
    if !entry
        .classifications
        .contains(&InventoryClassification::Included)
    {
        return Err(Error::new(format!(
            "background source {locator:?} is not included in the current Analysis Snapshot"
        )));
    }
    let bytes = fs::read(root.canonical_path().join(resolved.relative()))
        .map_err(|error| Error::with_source("cannot read background source", error))?;
    let content_sha256 = format!("{:x}", Sha256::digest(&bytes));
    if entry.content_sha256.as_deref() != Some(content_sha256.as_str()) {
        return Err(Error::new(format!(
            "background source {locator:?} changed after the current Analysis Snapshot; analyze again before preparing transmission"
        )));
    }
    let body = String::from_utf8(bytes)
        .map_err(|error| Error::with_source("background source is not UTF-8 text", error))?;
    Ok(BackgroundSource {
        source: analysis.repository_source.clone(),
        locator: locator.to_owned(),
        class: provider_source_class(entry),
        body,
    })
}

fn provider_source_class(entry: &InventoryEntry) -> SourceClass {
    if entry
        .classifications
        .contains(&InventoryClassification::Binary)
    {
        SourceClass::Binary
    } else if entry
        .classifications
        .contains(&InventoryClassification::Generated)
    {
        SourceClass::Generated
    } else if entry
        .classifications
        .contains(&InventoryClassification::Vendor)
    {
        SourceClass::Vendor
    } else if entry
        .classifications
        .contains(&InventoryClassification::Configuration)
        || entry
            .classifications
            .contains(&InventoryClassification::Manifest)
        || entry
            .classifications
            .contains(&InventoryClassification::WorkspaceManifest)
    {
        SourceClass::Configuration
    } else if entry
        .classifications
        .contains(&InventoryClassification::Document)
    {
        SourceClass::Document
    } else {
        SourceClass::Source
    }
}

fn provider_guarded_scope(record: &ProviderRequestRecord) -> Vec<String> {
    let mut scope = vec![
        format!("provider_request:{}", record.id),
        format!("repository_snapshot:{}", record.repository_snapshot),
        format!("analysis_snapshot:{}", record.analysis_snapshot),
        format!("purpose:{}", record.purpose),
        format!("capability:{}", record.requested_capability),
    ];
    scope.extend(record.manifest.iter().map(|entry| {
        format!(
            "source:{};class:{};scope:{};filter:{};original_bytes:{};filtered_lines:{}",
            entry.locator,
            source_class_slug(entry.class),
            scope_outcome_slug(entry.scope_outcome),
            filter_outcome_slug(entry.filter_outcome),
            entry.original_bytes,
            entry.filtered_line_count,
        )
    }));
    scope
}

const fn source_class_slug(class: SourceClass) -> &'static str {
    match class {
        SourceClass::Source => "source",
        SourceClass::Generated => "generated",
        SourceClass::Vendor => "vendor",
        SourceClass::Binary => "binary",
        SourceClass::Configuration => "configuration",
        SourceClass::Document => "document",
    }
}

const fn scope_outcome_slug(outcome: volicord_privacy::ScopeOutcome) -> &'static str {
    match outcome {
        volicord_privacy::ScopeOutcome::Included => "included",
        volicord_privacy::ScopeOutcome::Excluded => "excluded",
        volicord_privacy::ScopeOutcome::OutsideRequestedScope => "outside_requested_scope",
        volicord_privacy::ScopeOutcome::OutsideOptInScope => "outside_opt_in_scope",
    }
}

const fn filter_outcome_slug(outcome: volicord_privacy::FilterOutcome) -> &'static str {
    match outcome {
        volicord_privacy::FilterOutcome::NotApplied => "not_applied",
        volicord_privacy::FilterOutcome::NoMatch => "no_match",
        volicord_privacy::FilterOutcome::Filtered => "filtered",
    }
}

struct UnavailableConfiguredProvider {
    identity: ProviderIdentity,
}

impl BackgroundSemanticProvider for UnavailableConfiguredProvider {
    fn identity(&self) -> ProviderIdentity {
        self.identity.clone()
    }

    fn availability(&self) -> ProviderAvailability {
        ProviderAvailability::Unavailable {
            diagnostic:
                "no production external semantic-provider transport is configured in this build"
                    .into(),
        }
    }

    fn invoke(&mut self, _request: ProviderInvocation) -> ProviderExecution {
        ProviderExecution::Failed {
            diagnostic: "unavailable provider adapter cannot invoke a transport".into(),
        }
    }

    fn delete(&mut self, _request: ProviderDeletionRequest) -> ProviderDeletionOutcome {
        ProviderDeletionOutcome::Unsupported {
            diagnostic: "no production external semantic-provider transport is configured".into(),
        }
    }
}

fn empty_applicability(project_id: ProjectId) -> ApplicabilityQuery {
    ApplicabilityQuery {
        project_id,
        paths: Vec::new(),
        components: Vec::new(),
        work_contexts: Vec::new(),
        current_assumptions: Vec::new(),
        met_revisit_triggers: Vec::new(),
    }
}

fn empty_partial() -> PartialOutcome {
    PartialOutcome {
        completed_scopes: Vec::new(),
        failed_scopes: Vec::new(),
        omitted_scopes: Vec::new(),
    }
}

fn process_state(observation: &volicord_local_platform::ProcessObservation) -> OperationState {
    if observation.stop_trigger() == Some(ProcessStopTrigger::Timeout) {
        return OperationState::TimedOut;
    }
    if observation.stop_trigger() == Some(ProcessStopTrigger::Cancellation) {
        return OperationState::Cancelled;
    }
    if matches!(observation.cleanup(), ProcessTreeCleanup::Incomplete { .. }) {
        return OperationState::Indeterminate;
    }
    match observation.completion() {
        ProcessCompletion::Exited(ProcessTermination::ExitCode(0)) => OperationState::Succeeded,
        ProcessCompletion::Exited(ProcessTermination::Unknown)
        | ProcessCompletion::ObservationFailed {
            termination: ProcessTermination::Unknown,
            ..
        } => OperationState::Indeterminate,
        ProcessCompletion::Exited(_) | ProcessCompletion::ObservationFailed { .. } => {
            OperationState::Failed
        }
    }
}

fn publish_bytes_no_replace(destination: &Path, bytes: &[u8]) -> Result<PublicationOutcome, Error> {
    if !destination.is_absolute() {
        return Err(Error::new("publication destination must be absolute"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| Error::new("publication destination has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| Error::with_source("cannot create publication directory", error))?;
    let temporary = parent.join(format!(".volicord-publish-{}.tmp", new_operation_id()?));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| Error::with_source("cannot create publication temporary file", error))?;
    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(Error::with_source(
            "cannot durably write publication temporary file",
            error,
        ));
    }
    drop(file);
    let outcome = publish_file_no_replace(&temporary, destination)
        .map_err(|error| Error::with_source("no-replace publication failed", error))?;
    let durability = match outcome {
        NoReplacePublicationOutcome::DestinationExists => {
            let _ = fs::remove_file(&temporary);
            return Err(Error::new("publication destination already exists"));
        }
        NoReplacePublicationOutcome::Published {
            durability: DirectoryEntryDurability::ParentSynchronized,
        } => "parent_synchronized",
        NoReplacePublicationOutcome::Published {
            durability: DirectoryEntryDurability::ParentSynchronizationFailed,
        } => "parent_synchronization_failed",
        NoReplacePublicationOutcome::Published {
            durability: DirectoryEntryDurability::NotApplicable,
        } => "not_applicable",
    };
    Ok(PublicationOutcome {
        destination: destination.to_path_buf(),
        bytes: bytes.len() as u64,
        durability: durability.into(),
    })
}

fn repository_observation_basis(root: &Path, observed_at: i64) -> Result<String, Error> {
    let layout = GitWorktreeLayout::resolve(root)
        .map_err(|error| Error::with_source("cannot resolve repository coordinate", error))?;
    let coordinate = layout
        .map(|value| {
            format!(
                "{}:{}",
                value.coordinate().clone_identity(),
                value.coordinate().worktree_identity()
            )
        })
        .unwrap_or_else(|| "non-git-repository".into());
    let mut digest = Sha256::new();
    digest.update((coordinate.len() as u64).to_be_bytes());
    digest.update(coordinate.as_bytes());
    Ok(format!(
        "local-observation:sha256:{:x}:at:{observed_at}",
        digest.finalize()
    ))
}

fn new_operation_id() -> Result<OperationId, Error> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| Error::with_source("operating-system randomness is unavailable", error))?;
    Ok(OperationId::from_bytes(bytes))
}

fn now_micros() -> Result<TimestampMicros, Error> {
    SystemClock
        .now()
        .map_err(|error| Error::with_source("system clock is unavailable", error))
}

fn duration_micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn classify_context_error(kind: volicord_context::ErrorKind) -> HealthIssueKind {
    match kind {
        volicord_context::ErrorKind::UnsupportedVersion => HealthIssueKind::Unsupported,
        volicord_context::ErrorKind::CorruptState => HealthIssueKind::Corrupt,
        volicord_context::ErrorKind::RepairRequired => HealthIssueKind::RepairRequired,
        volicord_context::ErrorKind::StorageUnavailable | volicord_context::ErrorKind::NotFound => {
            HealthIssueKind::Unavailable
        }
        _ => HealthIssueKind::Failed,
    }
}

fn canonical_record_parts(record: CanonicalRecordId) -> (String, String) {
    let kind = match record {
        CanonicalRecordId::Project(_) => "project",
        CanonicalRecordId::Source(_) => "source",
        CanonicalRecordId::Question(_) => "question",
        CanonicalRecordId::Decision(_) => "decision",
        CanonicalRecordId::ContextItem(_) => "context_item",
        CanonicalRecordId::Checkpoint(_) => "checkpoint",
    };
    let mut identity = String::with_capacity(32);
    for byte in record.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(identity, "{byte:02x}");
    }
    (kind.to_owned(), identity)
}

pub(crate) fn parse_identity(value: &str) -> Result<[u8; 16], Error> {
    if value.len() != 32 {
        return Err(Error::new(
            "identity must contain exactly 32 hexadecimal characters",
        ));
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair)
            .map_err(|error| Error::with_source("identity is not UTF-8", error))?;
        bytes[index] = u8::from_str_radix(text, 16)
            .map_err(|error| Error::with_source("identity is not hexadecimal", error))?;
    }
    Ok(bytes)
}

pub(crate) fn select_document(set: &DocumentSet, kind: DocumentKind) -> &GeneratedDocument {
    match kind {
        DocumentKind::ProjectArchitectureGuide => &set.project_architecture_guide,
        DocumentKind::DecisionReport => &set.decision_report,
        DocumentKind::ImplementationPlan => &set.implementation_plan,
        DocumentKind::HandoffResume => &set.handoff_resume,
    }
}
