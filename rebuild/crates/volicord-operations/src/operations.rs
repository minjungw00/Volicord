use crate::forgetting::{ForgettingOperationRecord, ForgettingState, ForgettingStore};
use crate::{
    AnalysisOutcome, BindingOutcome, CandidateRepositoryResearchDraft, CanonicalMutationOutcome,
    ChildProcessOutcome, CodexCliProviderConfig, CodexCliSemanticProvider,
    CommandVerificationDraft, Error, ForgettingOutcome, GroundedCheckpointDraft,
    GroundedCheckpointOutcome, HealthIssue, HealthIssueKind, HealthReport, HealthState,
    LongOperationResult, MaterialityReviewDraft, MaterialityReviewOutcome,
    MaterialityReviewRevisionDraft, OperationState, PartialOutcome, ProgressState,
    ProjectInitialization, ProjectResolution, PublicationOutcome, RepairKind, RepairOutcome,
    RuntimeLayout, UserContextRecordingOutcome,
};
use crate::{
    BackgroundProviderDispatcher, BackgroundProviderOperationDraft, ConfirmationDecision,
    ConfirmationRequestId, ConfirmationResponse, DispatchExpectation, GuardedEffectCandidate,
    GuardedEffectCategory, GuardedEffectDispatcher, GuardedEffectDraft, GuardedOperationId,
    GuardedOperationResult, GuardedProviderInspection, GuardedProviderPreparation,
    GuardedProviderPreparationOutcome, GuardedRisk, GuardedStore,
};
use sha2::{Digest, Sha256};
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::BTreeSet,
    error::Error as StdError,
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use volicord_context::{
    Availability, BundleComparison, BundleMerge, CanonicalInvalidation, CanonicalReadBasis,
    CanonicalReadOptions, CanonicalRecordId, CheckpointDraft, Clock, CommandOutcome,
    ContextItemCorrectionDraft, ContextItemDraft, ContextItemId, ContextItemRole, DecisionChoice,
    DecisionCorrectionDraft, DecisionId, DecisionSupersessionDraft, MergeResolution, OperationId,
    OperationResult, Principal, PrincipalKind, ProjectId, SourceDraft, SourceId, SourcePayload,
    StatementProvenanceRole, Store, SystemClock, TimestampMicros, UserAcceptanceFact,
    UserAcceptanceState, UserReviewFact, UserReviewState, UserTurnSource, VerificationFact,
    VerificationState,
};
use volicord_inquiry::{
    attribute_repository_changes, compute_frontier, evaluate_checkpoint_candidate,
    evaluate_decision_applicability, evaluate_work_authority,
    record_checkpoint as persist_evaluated_checkpoint, record_response_batch, ApplicabilityQuery,
    BatchResponseItem, BatchResponseResult, CandidateCollectionMode, CandidateCollectionScope,
    CandidateContent, CandidateDisposition, CandidateDraft, CandidateId, CandidateKind,
    CandidateObservationBasis, CandidateOrigin, CandidateReadBasis, CandidateRecord,
    CandidateRetention, CandidateStore, ChangeAttribution, CheckpointCandidate,
    CheckpointEvaluation, DecisionApplicabilityState, FrontierRead, InquiryScope,
    MaterialityReview, PromotionResult, RepositoryResearchBasis, RepositoryWorkBasis,
    SubmissionOutcome, WorkAuthorityDisposition, WorkAuthorityResult,
};
use volicord_local_platform::{
    publish_file_no_replace, CancellationFlag, DirectoryEntryDurability, DirtyObservation,
    GitWorktreeLayout, NoReplacePublicationOutcome, ProcessCompletion, ProcessRequest,
    ProcessStopTrigger, ProcessTermination, ProcessTreeCleanup, RepositoryPathState,
    RepositoryRoot,
};
use volicord_privacy::{
    BackgroundSemanticProvider, BackgroundSemanticRequest, BackgroundSource, PreparationOutcome,
    PrivacyStore, ProjectPrivacyInspection, ProviderDeletionOutcome, ProviderIdentity,
    ProviderIntentProvenance, ProviderOptInEvent, ProviderOptInPolicy, ProviderRequestId,
    ProviderRequestRecord, SourceClass,
};
use volicord_projections::{
    build_project_projection, generate_documents, prepare_narrative_plan, realize_narrative,
    CandidateContentAccess, CandidateDependencyFailure, CandidateDependencyFailureKind,
    CandidateProjectionInput, DocumentKind, DocumentRequest, DocumentSet, GeneratedDocument,
    NarrativePlan, NarrativeRealization, OutputFormat, ProjectProjection, ProjectProjectionInputs,
    ProjectionBound, RecallBound, RecallInputs, ResumeBrief,
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
        let _mutation = self.layout.acquire_mutation_lock()?;
        self.initialize_runtime_unlocked()
    }

    fn initialize_runtime_unlocked(&self) -> Result<(), Error> {
        self.layout.prepare_private_paths()?;
        let canonical = Store::open(self.layout.canonical_store())
            .map_err(|error| Error::with_source("cannot initialize canonical store", error))?;
        let candidates = CandidateStore::open(self.layout.candidate_store())
            .map_err(|error| Error::with_source("cannot initialize Candidate store", error))?;
        let privacy = PrivacyStore::open(self.layout.privacy_store())
            .map_err(|error| Error::with_source("cannot initialize privacy store", error))?;
        let guarded = GuardedStore::open(self.layout.guarded_store())?;
        let forgetting = ForgettingStore::open(&self.layout.forgetting_store())?;
        self.layout.enforce_private_store_files()?;
        drop((canonical, candidates, privacy, guarded, forgetting));
        Ok(())
    }

    pub fn initialize_project(
        &self,
        display_name: impl Into<String>,
        repository: Option<&Path>,
    ) -> Result<ProjectInitialization, Error> {
        let _mutation = self.layout.acquire_mutation_lock()?;
        self.initialize_runtime_unlocked()?;
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

    pub fn initialize_project_from_repository(
        &self,
        repository: &Path,
    ) -> Result<ProjectInitialization, Error> {
        let root = RepositoryRoot::open(repository).map_err(|error| {
            Error::with_source("repository initialization path is invalid", error)
        })?;
        let display_name = GitWorktreeLayout::resolve(root.canonical_path())
            .ok()
            .flatten()
            .and_then(|layout| layout.repository_name_hint())
            .or_else(|| {
                root.canonical_path()
                    .file_name()
                    .and_then(OsStr::to_str)
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned)
            })
            .ok_or_else(|| {
                Error::new(
                    "canonical repository root has no non-empty UTF-8 basename for the Project display name",
                )
            })?;
        self.initialize_project(display_name, Some(root.canonical_path()))
    }

    pub fn bind_project(
        &self,
        project_id: ProjectId,
        expected_binding_revision: Option<u64>,
        repository: &Path,
    ) -> Result<BindingOutcome, Error> {
        let _mutation = self.layout.acquire_mutation_lock()?;
        let mut canonical = self.open_canonical()?;
        self.bind_with_store(
            &mut canonical,
            project_id,
            expected_binding_revision,
            repository,
        )
    }

    pub fn resolve_project(&self, repository: &Path) -> Result<ProjectResolution, Error> {
        let root = RepositoryRoot::open(repository)
            .map_err(|error| Error::with_source("repository resolution path is invalid", error))?;
        let canonical_repository_path = root.canonical_path().to_path_buf();
        let canonical = match Store::open_read_only(self.layout.canonical_store()) {
            Ok(canonical) => canonical,
            Err(error) if error.kind() == volicord_context::ErrorKind::NotFound => {
                return Ok(ProjectResolution::NotFound {
                    canonical_repository_path,
                });
            }
            Err(error) => {
                return Err(Error::with_source(
                    "cannot open canonical store for Project resolution",
                    error,
                ));
            }
        };
        let Some(binding) = canonical
            .resolve_local_binding(&canonical_repository_path)
            .map_err(|error| Error::with_source("cannot resolve repository binding", error))?
        else {
            return Ok(ProjectResolution::NotFound {
                canonical_repository_path,
            });
        };
        let project = canonical
            .get_project(binding.project_id)
            .map_err(|error| Error::with_source("cannot read resolved Project", error))?;
        let layout = GitWorktreeLayout::resolve(root.canonical_path())
            .map_err(|error| Error::with_source("cannot observe repository identity", error))?;
        let coordinate = layout.as_ref().map(GitWorktreeLayout::coordinate);
        Ok(ProjectResolution::Found {
            project,
            binding: BindingOutcome {
                binding,
                clone_identity: coordinate
                    .as_ref()
                    .map(|value| value.clone_identity().to_owned()),
                worktree_identity: coordinate.map(|value| value.worktree_identity().to_owned()),
            },
        })
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
        if let Err(error) = self.initialize_runtime() {
            issues.push(HealthIssue {
                kind: HealthIssueKind::Failed,
                scope: "runtime_access".into(),
                detail: error.to_string(),
            });
            return HealthReport {
                state: HealthState::Failed,
                runtime_root: self.layout.root().to_path_buf(),
                canonical_available: false,
                candidate_available: false,
                privacy_available: false,
                guarded_available: false,
                forgetting_available: false,
                repository_available: None,
                issues,
            };
        }
        let canonical = Store::open(self.layout.canonical_store());
        let candidates = CandidateStore::open(self.layout.candidate_store());
        let privacy = PrivacyStore::open(self.layout.privacy_store());
        let guarded = GuardedStore::open(self.layout.guarded_store());
        let forgetting = ForgettingStore::open(&self.layout.forgetting_store());
        let canonical_available = canonical.is_ok();
        let candidate_available = candidates.is_ok();
        let privacy_available = privacy.is_ok();
        let guarded_available = guarded.is_ok();
        let forgetting_available = forgetting.is_ok();
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
        if let Err(error) = &forgetting {
            issues.push(HealthIssue {
                kind: HealthIssueKind::Failed,
                scope: "forgetting_operations".into(),
                detail: error.to_string(),
            });
        }
        if let Ok(store) = &forgetting {
            match store.incomplete(project_id) {
                Ok(operations) => {
                    for operation in operations {
                        issues.push(HealthIssue {
                            kind: HealthIssueKind::RepairRequired,
                            scope: format!("forgetting:{}", operation.operation_id),
                            detail: format!(
                                "canonical forgetting cleanup is {:?}; safe next action from the bound repository: volicord doctor repair --forgetting {} (or add --project {} when repository resolution is unavailable)",
                                operation.state, operation.operation_id, operation.project_id
                            ),
                        });
                    }
                }
                Err(error) => issues.push(HealthIssue {
                    kind: HealthIssueKind::Failed,
                    scope: "forgetting_operations".into(),
                    detail: error.to_string(),
                }),
            }
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
            forgetting_available,
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
        let canonical = self.open_canonical()?;
        let binding = canonical.get_local_binding(project_id).map_err(|error| {
            Error::with_source("Project has no usable repository binding", error)
        })?;
        let root = RepositoryRoot::open(&binding.absolute_path)
            .map_err(|error| Error::with_source("bound repository is unavailable", error))?;
        drop(canonical);
        let repository_worktree = self.observe_repository_worktree(root.canonical_path())?;
        let revision =
            repository_observation_basis(root.canonical_path(), observed_at.as_unix_micros())?;
        let _mutation = self.layout.acquire_mutation_lock()?;
        let mut canonical = self.open_canonical()?;
        let project = canonical
            .get_project(project_id)
            .map_err(|error| Error::with_source("cannot refresh Project for analysis", error))?;
        let current_binding = canonical.get_local_binding(project_id).map_err(|error| {
            Error::with_source(
                "Project binding changed during repository observation",
                error,
            )
        })?;
        if current_binding.absolute_path != binding.absolute_path {
            return Err(Error::new(
                "Project binding changed during repository observation; analyze again",
            ));
        }
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
        volicord_local_platform::ensure_private_directory(&operation_dir).map_err(|error| {
            Error::with_source(
                "cannot create private child-process artifact directory",
                error,
            )
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
        let _mutation = self.layout.acquire_mutation_lock()?;
        let mut canonical = self.open_canonical()?;
        canonical
            .export_bundle(project_id, destination)
            .map_err(|error| Error::with_source("portable export failed", error))
    }

    pub fn import_bundle(&self, source: &Path) -> Result<volicord_context::BundleImport, Error> {
        self.initialize_runtime()?;
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let invalidations = self.incomplete_committed_invalidations(project_id)?;
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|store| store.read_basis_with_invalidations(project_id, &invalidations))
            .map_err(|error| Error::with_source("Candidate inspection failed", error))
    }

    pub fn submit_candidate(&self, draft: CandidateDraft) -> Result<SubmissionOutcome, Error> {
        self.initialize_runtime()?;
        let _mutation = self.layout.acquire_mutation_lock()?;
        let _ = self.canonical_basis(draft.project_id)?;
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| store.submit(draft))
            .map_err(|error| Error::with_source("Candidate submission failed", error))
    }

    pub fn record_materiality_review(
        &self,
        draft: MaterialityReviewDraft,
    ) -> Result<MaterialityReviewOutcome, Error> {
        self.initialize_runtime()?;
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
        let current = self
            .analyze(draft.project_id, excluded_paths)?
            .value
            .ok_or_else(|| Error::new("Materiality Review analysis produced no usable snapshot"))?
            .analysis;
        let _mutation = self.layout.acquire_mutation_lock()?;
        let canonical = self.canonical_basis(draft.project_id)?;
        let goal = canonical
            .context_items
            .iter()
            .find(|item| item.id == draft.goal_context_id)
            .ok_or_else(|| Error::new("Materiality Review Goal Context was not found"))?;
        let mut source_basis = goal.source_basis.clone();
        source_basis.push(baseline.repository_source.identity());
        source_basis.push(current.repository_source.identity());
        for dimension in &draft.dimensions {
            source_basis.extend(dimension.basis.source_basis.iter().copied());
        }
        source_basis.sort_unstable();
        source_basis.dedup();
        let observed_at = SystemClock
            .now()
            .map_err(|error| Error::with_source("cannot timestamp Materiality Review", error))?;
        let candidate = CandidateDraft {
            project_id: draft.project_id,
            kind: CandidateKind::MaterialityReview,
            collection_mode: CandidateCollectionMode::ExplicitUserDirected,
            origin: CandidateOrigin {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "codex".to_owned(),
                },
                subsystem: "inquiry".to_owned(),
                session: Some(draft.session.clone()),
                provenance_summary: "typed pre-work Materiality Review".to_owned(),
            },
            collection_scope: CandidateCollectionScope {
                project_id: draft.project_id,
                session: Some(draft.session),
                source_operation: Some(draft.source_operation),
                candidate_kind: CandidateKind::MaterialityReview,
            },
            observation_basis: CandidateObservationBasis {
                source_basis,
                repository_snapshot: Some(baseline.repository_snapshot.to_string()),
                analysis_snapshot: Some(baseline.identity.to_string()),
                execution: None,
                host_turn: None,
                other: Some("pre-work work-authority classification".to_owned()),
            },
            observed_at,
            retention: CandidateRetention {
                retained_until: None,
                basis: "retain through bounded work and Checkpoint validation".to_owned(),
            },
            content: CandidateContent {
                bounded_summary: draft.rationale.clone(),
                question: None,
                materiality_review: Some(MaterialityReview {
                    goal_context_id: draft.goal_context_id,
                    baseline_analysis_snapshot_id: baseline.identity,
                    first_review_analysis_snapshot_id: current.identity,
                    current_review_analysis_snapshot_id: current.identity,
                    first_review_preceded_meaningful_mutation: false,
                    rationale: draft.rationale,
                    dimensions: draft.dimensions,
                }),
            },
        };
        let stored = CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| {
                store.submit_materiality_review(candidate, &canonical, &baseline, &current)
            })
            .map_err(|error| Error::new(format!("Materiality Review failed: {error}")))?;
        let SubmissionOutcome::Stored(record) = stored else {
            return Err(Error::new(
                "typed Materiality Review was unexpectedly disabled",
            ));
        };
        Ok(MaterialityReviewOutcome {
            review_candidate_id: record.id,
            review_revision: record.revision,
            goal_context_id: draft.goal_context_id,
            baseline_analysis_snapshot_id: baseline.identity,
            review_analysis_snapshot_id: current.identity,
        })
    }

    pub fn revise_materiality_review(
        &self,
        draft: MaterialityReviewRevisionDraft,
    ) -> Result<MaterialityReviewOutcome, Error> {
        let existing = CandidateStore::open(self.layout.candidate_store())
            .and_then(|store| store.get(draft.project_id, draft.review_candidate_id))
            .map_err(|error| Error::with_source("Materiality Review lookup failed", error))?;
        let review = existing
            .content
            .as_ref()
            .and_then(|content| content.materiality_review.as_ref())
            .ok_or_else(|| Error::new("Materiality Review content is unavailable"))?;
        let baseline =
            self.load_analysis_snapshot(draft.project_id, review.baseline_analysis_snapshot_id)?;
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
        let current = self
            .analyze(draft.project_id, excluded_paths)?
            .value
            .ok_or_else(|| Error::new("revised Materiality Review analysis produced no snapshot"))?
            .analysis;
        let _mutation = self.layout.acquire_mutation_lock()?;
        let canonical = self.canonical_basis(draft.project_id)?;
        let record = CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| {
                store.revise_materiality_review(
                    draft.project_id,
                    draft.review_candidate_id,
                    &canonical,
                    &current,
                    draft.rationale,
                    draft.dimensions,
                )
            })
            .map_err(|error| Error::new(format!("Materiality Review revision failed: {error}")))?;
        let review = record
            .content
            .as_ref()
            .and_then(|content| content.materiality_review.as_ref())
            .ok_or_else(|| Error::new("revised Materiality Review content is unavailable"))?;
        Ok(MaterialityReviewOutcome {
            review_candidate_id: record.id,
            review_revision: record.revision,
            goal_context_id: review.goal_context_id,
            baseline_analysis_snapshot_id: review.baseline_analysis_snapshot_id,
            review_analysis_snapshot_id: review.current_review_analysis_snapshot_id,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn work_readiness(
        &self,
        project_id: ProjectId,
        goal_context_id: ContextItemId,
        baseline_analysis_snapshot_id: AnalysisSnapshotId,
        review_candidate_id: CandidateId,
        paths: Vec<String>,
        components: Vec<String>,
        work_contexts: Vec<String>,
        met_revisit_triggers: Vec<String>,
    ) -> Result<WorkAuthorityResult, Error> {
        let canonical = self.canonical_basis(project_id)?;
        let candidate = CandidateStore::open(self.layout.candidate_store())
            .and_then(|store| store.get(project_id, review_candidate_id))
            .map_err(|error| Error::with_source("Materiality Review lookup failed", error))?;
        let current_assumptions = canonical
            .context_items
            .iter()
            .filter(|item| item.role == ContextItemRole::Assumption)
            .map(|item| item.statement.clone())
            .collect();
        Ok(evaluate_work_authority(
            &canonical,
            Some(&candidate),
            project_id,
            goal_context_id,
            baseline_analysis_snapshot_id,
            &ApplicabilityQuery {
                project_id,
                paths,
                components,
                work_contexts,
                current_assumptions,
                met_revisit_triggers,
            },
        ))
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
        if self
            .candidate_basis(project_id)?
            .withheld_for_canonical_forgetting
            .contains(&candidate_id)
        {
            return Err(Error::new(
                "Question Candidate promotion is blocked while related canonical forgetting requires repair",
            ));
        }
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
        let _ = self.canonical_basis(project_id)?;
        CandidateStore::open(self.layout.candidate_store())
            .and_then(|mut store| store.delete_candidate(project_id, candidate_id, basis))
            .map_err(|error| Error::with_source("Candidate deletion failed", error))
    }

    pub fn privacy_status(&self, project_id: ProjectId) -> Result<ProjectPrivacyInspection, Error> {
        let invalidations = self.incomplete_committed_invalidations(project_id)?;
        PrivacyStore::open(self.layout.privacy_store())
            .and_then(|store| store.inspect_project_with_invalidations(project_id, &invalidations))
            .map_err(|error| Error::with_source("privacy inspection failed", error))
    }

    pub fn enable_provider(
        &self,
        policy: ProviderOptInPolicy,
        intent: ProviderIntentProvenance,
    ) -> Result<ProviderOptInEvent, Error> {
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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

        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let candidate = self.create_guarded_request_unlocked(GuardedEffectDraft {
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
        let mut provider = CodexCliSemanticProvider::new(
            ProviderIdentity {
                provider: preparation.provider_request.provider.clone(),
                model: preparation.provider_request.model.clone(),
            },
            CodexCliProviderConfig::production(self.layout.artifacts_dir()),
        );
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
        let mut candidate_basis = None;
        let candidate_failure;
        match self.incomplete_committed_invalidations(project_id) {
            Ok(invalidations) => {
                match CandidateStore::open(self.layout.candidate_store()).and_then(|store| {
                    store.read_basis_with_invalidations(project_id, &invalidations)
                }) {
                    Ok(basis) => {
                        candidate_basis = Some(basis);
                        candidate_failure = (!invalidations.is_empty()).then(|| {
                            CandidateDependencyFailure {
                                kind: CandidateDependencyFailureKind::RepairRequired,
                                affected_scope: "candidate_inspection".to_owned(),
                                reason: format!(
                                    "Candidate data is partially unavailable while {} canonical forgetting operation(s) require repair",
                                    invalidations.len()
                                ),
                            }
                        });
                    }
                    Err(error) => {
                        candidate_failure = Some(CandidateDependencyFailure {
                            kind: candidate_dependency_failure_from_inquiry(error.kind()),
                            affected_scope: "candidate_inspection".to_owned(),
                            reason: error.to_string(),
                        });
                    }
                }
            }
            Err(error) => {
                candidate_failure = Some(CandidateDependencyFailure {
                    kind: candidate_dependency_failure_from_error(&error),
                    affected_scope: "candidate_inspection".to_owned(),
                    reason: error.to_string(),
                });
            }
        }
        let candidates = match (candidate_basis.as_ref(), candidate_failure) {
            (basis, Some(failure)) => CandidateProjectionInput::Degraded {
                usable_basis: basis,
                failure,
            },
            (Some(basis), None) => CandidateProjectionInput::Available(basis),
            (None, None) => CandidateProjectionInput::Degraded {
                usable_basis: None,
                failure: CandidateDependencyFailure {
                    kind: CandidateDependencyFailureKind::Failed,
                    affected_scope: "candidate_inspection".to_owned(),
                    reason: "Candidate read completed without a readable basis".to_owned(),
                },
            },
        };
        Ok(build_project_projection(ProjectProjectionInputs {
            canonical: &canonical,
            analyses: &analysis_refs,
            applicability: empty_applicability(project_id),
            candidates,
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

    pub fn document_narrative_plan(
        &self,
        project_id: ProjectId,
        request: &DocumentRequest,
        kind: DocumentKind,
    ) -> Result<NarrativePlan, Error> {
        let projection = self.project_projection(project_id)?;
        prepare_narrative_plan(&projection, request, kind)
            .map_err(|error| Error::with_source("narrative plan generation failed", error))
    }

    pub fn realize_document_narrative(
        &self,
        project_id: ProjectId,
        request: &DocumentRequest,
        kind: DocumentKind,
        realization: &NarrativeRealization,
    ) -> Result<GeneratedDocument, Error> {
        let projection = self.project_projection(project_id)?;
        realize_narrative(&projection, request, kind, realization)
            .map_err(|error| Error::with_source("narrative realization failed", error))
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

    /// Publishes one already-rendered, read-only Viewer snapshot to an exact
    /// user destination. This operation owns only atomic no-replace local
    /// publication; it does not interpret the HTML or mutate Project state.
    pub fn publish_viewer_snapshot(
        &self,
        html: &str,
        destination: &Path,
    ) -> Result<PublicationOutcome, Error> {
        publish_bytes_no_replace(destination, html.as_bytes())
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
        self.create_guarded_request_unlocked(draft)
    }

    fn create_guarded_request_unlocked(
        &self,
        draft: GuardedEffectDraft,
    ) -> Result<GuardedEffectCandidate, Error> {
        GuardedStore::open(self.layout.guarded_store())?.create_request(draft, now_micros()?)
    }

    pub fn revise_guarded_request(
        &self,
        request: ConfirmationRequestId,
        expected_revision: u64,
        draft: GuardedEffectDraft,
    ) -> Result<GuardedEffectCandidate, Error> {
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
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
        let _mutation = self.layout.acquire_mutation_lock()?;
        self.supersede_decision_unlocked(project_id, draft)
    }

    fn supersede_decision_unlocked(
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
        let _mutation = self.layout.acquire_mutation_lock()?;
        let canonical = self.canonical_basis(project_id)?;
        let previous = canonical
            .active_decisions
            .iter()
            .chain(canonical.superseded_decisions.iter())
            .find(|lifecycle| lifecycle.decision.id == previous_decision_id)
            .ok_or_else(|| Error::new("previous Decision was not found"))?;
        self.supersede_decision_unlocked(
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
    ) -> Result<ForgettingOutcome, Error> {
        let _mutation = self.layout.acquire_mutation_lock()?;
        self.initialize_runtime_unlocked()?;
        let requested_operation = new_operation_id()?;
        let observed_at = now_micros()?.as_unix_micros();
        let mut forgetting = self.open_forgetting()?;
        let mut operation = forgetting.prepare(
            requested_operation,
            project_id,
            record,
            authorization,
            observed_at,
        )?;
        let replayed = operation.operation_id != requested_operation
            || operation.state != ForgettingState::Prepared;
        if operation.state == ForgettingState::Completed {
            return Ok(forgetting_outcome(&operation, true, None));
        }

        let mut canonical = self.open_canonical()?;
        let canonical_result = match operation.record {
            CanonicalRecordId::Source(id) => canonical.forget_source(
                operation.operation_id,
                project_id,
                id,
                operation.authorization_source_id,
            ),
            CanonicalRecordId::Question(id) => canonical.forget_question(
                operation.operation_id,
                project_id,
                id,
                operation.authorization_source_id,
            ),
            CanonicalRecordId::Decision(id) => canonical.forget_decision(
                operation.operation_id,
                project_id,
                id,
                operation.authorization_source_id,
            ),
            CanonicalRecordId::ContextItem(id) => canonical.forget_context_item(
                operation.operation_id,
                project_id,
                id,
                operation.authorization_source_id,
            ),
            CanonicalRecordId::Checkpoint(id) => canonical.forget_checkpoint(
                operation.operation_id,
                project_id,
                id,
                operation.authorization_source_id,
            ),
            CanonicalRecordId::Project(_) => {
                return Err(Error::new(
                    "Project forgetting is not exposed by the Canonical Context owner",
                ))
            }
        };
        match canonical_result {
            Ok(_) => {
                operation = forgetting.mark_canonical_committed(
                    operation.operation_id,
                    now_micros()?.as_unix_micros(),
                )?;
            }
            Err(error) => match canonical.get_tombstone(project_id, operation.record) {
                Ok(_) => {
                    operation = forgetting.mark_repair_required(
                        operation.operation_id,
                        false,
                        false,
                        false,
                        now_micros()?.as_unix_micros(),
                    )?;
                    return Ok(forgetting_outcome(
                        &operation,
                        replayed,
                        Some(format!(
                            "canonical content is forgotten, but reconciliation failed: {error}"
                        )),
                    ));
                }
                Err(tombstone_error)
                    if tombstone_error.kind() == volicord_context::ErrorKind::NotFound =>
                {
                    return Err(Error::with_source("canonical forgetting failed", error));
                }
                Err(tombstone_error) => {
                    return Err(Error::with_source(
                        "canonical forgetting commit state is indeterminate",
                        tombstone_error,
                    ));
                }
            },
        }
        drop(canonical);

        let invalidation = operation.invalidation();
        let mut candidates = match CandidateStore::open(self.layout.candidate_store()) {
            Ok(store) => store,
            Err(error) => {
                return mark_forgetting_repair_required(
                    &mut forgetting,
                    operation,
                    replayed,
                    false,
                    false,
                    format!("cannot open Candidate store: {error}"),
                )
            }
        };
        let mut privacy = match PrivacyStore::open(self.layout.privacy_store()) {
            Ok(store) => store,
            Err(error) => {
                return mark_forgetting_repair_required(
                    &mut forgetting,
                    operation,
                    replayed,
                    false,
                    false,
                    format!("cannot open privacy store: {error}"),
                )
            }
        };
        let cleanup_basis = format!("forgetting-operation:{}", operation.operation_id);
        if let Err(error) =
            privacy.apply_canonical_forgetting(&mut candidates, &invalidation, cleanup_basis)
        {
            let candidate_complete = candidates
                .verify_canonical_forgetting(&invalidation)
                .unwrap_or(false);
            let derived_complete = privacy
                .verify_canonical_forgetting(&invalidation)
                .unwrap_or(false);
            return mark_forgetting_repair_required(
                &mut forgetting,
                operation,
                replayed,
                candidate_complete,
                derived_complete,
                format!("related local cleanup is incomplete: {error}"),
            );
        }
        let candidate_complete = match candidates.verify_canonical_forgetting(&invalidation) {
            Ok(complete) => complete,
            Err(error) => {
                return mark_forgetting_repair_required(
                    &mut forgetting,
                    operation,
                    replayed,
                    false,
                    false,
                    format!("cannot verify Candidate forgetting: {error}"),
                )
            }
        };
        let derived_complete = match privacy.verify_canonical_forgetting(&invalidation) {
            Ok(complete) => complete,
            Err(error) => {
                return mark_forgetting_repair_required(
                    &mut forgetting,
                    operation,
                    replayed,
                    candidate_complete,
                    false,
                    format!("cannot verify managed Derived forgetting: {error}"),
                )
            }
        };
        if !candidate_complete || !derived_complete {
            return mark_forgetting_repair_required(
                &mut forgetting,
                operation,
                replayed,
                candidate_complete,
                derived_complete,
                "related local cleanup postconditions are incomplete".into(),
            );
        }
        operation =
            forgetting.mark_completed(operation.operation_id, now_micros()?.as_unix_micros())?;
        Ok(forgetting_outcome(&operation, replayed, None))
    }

    pub fn repair_forgetting(
        &self,
        project_id: ProjectId,
        operation_id: OperationId,
    ) -> Result<ForgettingOutcome, Error> {
        self.initialize_runtime()?;
        let operation = self.open_forgetting()?.get(operation_id)?;
        if operation.project_id != project_id {
            return Err(Error::new(
                "forgetting repair operation belongs to a different Project",
            ));
        }
        let outcome = self.forget_record(
            project_id,
            operation.record,
            operation.authorization_source_id,
        )?;
        if outcome.operation_id != operation_id {
            return Err(Error::new(
                "forgetting repair resolved a different durable operation",
            ));
        }
        Ok(outcome)
    }

    pub fn record_checkpoint(
        &self,
        _project_id: ProjectId,
        _draft: CheckpointDraft,
    ) -> Result<CanonicalMutationOutcome, Error> {
        Err(Error::new(
            "ungrounded Checkpoint recording is unavailable; a meaningful Checkpoint requires the current Goal, exact pre-work Analysis Snapshot, and resolved work authority",
        ))
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
        let (pre_existing_dirty_paths, changed_paths) =
            match attribute_repository_changes(draft.project_id, &repository_work) {
                ChangeAttribution::Attributed {
                    pre_existing_paths,
                    changed_paths,
                } => (pre_existing_paths, changed_paths),
                ChangeAttribution::Unavailable { reason, .. } => return Err(Error::new(reason)),
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

        // Repository observation and analysis may be long-running. Serialize
        // only the final durable Source and Checkpoint publication.
        let _mutation = self.layout.acquire_mutation_lock()?;
        let authority_canonical = self.canonical_basis(draft.project_id)?;
        let candidate_basis = self.candidate_basis(draft.project_id)?;
        let latest_review = |exact: bool| {
            candidate_basis
                .candidates
                .iter()
                .filter(|candidate| {
                    candidate.kind == CandidateKind::MaterialityReview
                        && matches!(
                            candidate.disposition,
                            CandidateDisposition::PendingOrRetained
                        )
                        && candidate.content.is_some()
                })
                .filter(|candidate| {
                    if !exact {
                        return true;
                    }
                    candidate
                        .content
                        .as_ref()
                        .and_then(|content| content.materiality_review.as_ref())
                        .is_some_and(|review| {
                            review.goal_context_id == draft.goal_context_id
                                && review.baseline_analysis_snapshot_id
                                    == draft.baseline_analysis_snapshot_id
                        })
                })
                .max_by_key(|candidate| (candidate.created_at, candidate.id))
        };
        let review_candidate = latest_review(true).or_else(|| latest_review(false));
        let authority = evaluate_work_authority(
            &authority_canonical,
            review_candidate,
            draft.project_id,
            draft.goal_context_id,
            draft.baseline_analysis_snapshot_id,
            &applicability,
        );
        if authority.disposition != WorkAuthorityDisposition::ReadyForWork {
            return Err(Error::new(format!(
                "Checkpoint work authority is not resolved: {}",
                authority.reason
            )));
        }
        let required_decisions = authority
            .satisfied_requirements
            .iter()
            .flat_map(|requirement| requirement.decision_basis.iter().copied())
            .collect::<BTreeSet<_>>();
        if !required_decisions.is_subset(&unique_decisions) {
            return Err(Error::new(format!(
                "Checkpoint must name every Decision in its resolved work-authority basis: {:?}",
                required_decisions
                    .difference(&unique_decisions)
                    .collect::<Vec<_>>()
            )));
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
            pre_existing_dirty_paths,
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
                        invocation_fingerprint: command_invocation_fingerprint(
                            draft.command_invocation.as_deref().ok_or_else(|| {
                                Error::new(
                                    "executed verification needs the exact command invocation",
                                )
                            })?,
                        ),
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
        let path = self.layout.canonical_store();
        let store = Store::open(&path)
            .map_err(|error| Error::with_source("cannot open canonical store", error))?;
        volicord_local_platform::ensure_private_file(&path)
            .map_err(|error| Error::with_source("canonical store is not private", error))?;
        Ok(store)
    }

    fn open_forgetting(&self) -> Result<ForgettingStore, Error> {
        let path = self.layout.forgetting_store();
        let store = ForgettingStore::open(&path)?;
        volicord_local_platform::ensure_private_file(&path)
            .map_err(|error| Error::with_source("forgetting store is not private", error))?;
        Ok(store)
    }

    fn incomplete_committed_invalidations(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<CanonicalInvalidation>, Error> {
        let operations = self.open_forgetting()?.incomplete(Some(project_id))?;
        let canonical = self.open_canonical()?;
        let mut invalidations = Vec::new();
        for operation in operations {
            match canonical.get_tombstone(project_id, operation.record) {
                Ok(_) => invalidations.push(operation.invalidation()),
                Err(error) if error.kind() == volicord_context::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(Error::with_source(
                        "cannot establish canonical forgetting read barrier",
                        error,
                    ))
                }
            }
        }
        Ok(invalidations)
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
        let _mutation = self.layout.acquire_mutation_lock()?;
        let directory = self
            .layout
            .analysis_project_dir(analysis.project.identity());
        volicord_local_platform::ensure_private_directory(&directory).map_err(|error| {
            Error::with_source("cannot create private Project analysis directory", error)
        })?;
        let bytes = serde_json::to_vec_pretty(analysis)
            .map_err(|error| Error::with_source("cannot serialize Analysis Snapshot", error))?;
        let path = directory.join(format!("{}.json", analysis.identity));
        publish_bytes_no_replace(&path, &bytes)?;
        Ok(path)
    }

    fn replace_analysis(&self, analysis: &AnalysisSnapshot) -> Result<PathBuf, Error> {
        let _mutation = self.layout.acquire_mutation_lock()?;
        let project_id = analysis.project.identity();
        let base = self.layout.analysis_dir();
        fs::create_dir_all(&base)
            .map_err(|error| Error::with_source("cannot create analysis directory", error))?;
        let destination = self.layout.analysis_project_dir(project_id);
        let staging = base.join(format!(".staging-{project_id}-{}", analysis.identity));
        let replaced = base.join(format!(".replaced-{project_id}-{}", analysis.identity));
        volicord_local_platform::ensure_private_directory(&staging).map_err(|error| {
            Error::with_source("cannot stage private Project analysis replacement", error)
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

fn candidate_dependency_failure_from_inquiry(
    kind: volicord_inquiry::ErrorKind,
) -> CandidateDependencyFailureKind {
    match kind {
        volicord_inquiry::ErrorKind::UnsupportedVersion => {
            CandidateDependencyFailureKind::Unsupported
        }
        volicord_inquiry::ErrorKind::CorruptState => CandidateDependencyFailureKind::Corrupt,
        volicord_inquiry::ErrorKind::StorageUnavailable | volicord_inquiry::ErrorKind::NotFound => {
            CandidateDependencyFailureKind::Unavailable
        }
        _ => CandidateDependencyFailureKind::Failed,
    }
}

fn candidate_dependency_failure_from_error(error: &Error) -> CandidateDependencyFailureKind {
    let mut current: Option<&(dyn StdError + 'static)> = Some(error);
    while let Some(cause) = current {
        if let Some(candidate) = cause.downcast_ref::<volicord_inquiry::Error>() {
            return candidate_dependency_failure_from_inquiry(candidate.kind());
        }
        if let Some(canonical) = cause.downcast_ref::<volicord_context::Error>() {
            return match canonical.kind() {
                volicord_context::ErrorKind::UnsupportedVersion => {
                    CandidateDependencyFailureKind::Unsupported
                }
                volicord_context::ErrorKind::CorruptState => {
                    CandidateDependencyFailureKind::Corrupt
                }
                volicord_context::ErrorKind::RepairRequired => {
                    CandidateDependencyFailureKind::RepairRequired
                }
                volicord_context::ErrorKind::StorageUnavailable
                | volicord_context::ErrorKind::NotFound => {
                    CandidateDependencyFailureKind::Unavailable
                }
                _ => CandidateDependencyFailureKind::Failed,
            };
        }
        current = cause.source();
    }
    CandidateDependencyFailureKind::Failed
}

fn validate_verification_drafts(values: &[CommandVerificationDraft]) -> Result<(), Error> {
    for value in values {
        if value.state == VerificationState::NotRun {
            if value.command_label.is_some()
                || value.command_invocation.is_some()
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
        let invocation = value.command_invocation.as_deref().ok_or_else(|| {
            Error::new("executed verification needs the exact command invocation")
        })?;
        if invocation.is_empty() || invocation.len() > 16_384 {
            return Err(Error::new(
                "verification command invocation must contain 1 to 16384 UTF-8 bytes",
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

fn command_invocation_fingerprint(invocation: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(invocation.as_bytes()))
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
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(target_os = "linux")]
    options.mode(0o600);
    let mut file = options
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

fn mark_forgetting_repair_required(
    forgetting: &mut ForgettingStore,
    operation: ForgettingOperationRecord,
    replayed: bool,
    candidate_cleanup_completed: bool,
    managed_derived_cleanup_completed: bool,
    diagnostic: String,
) -> Result<ForgettingOutcome, Error> {
    let operation = forgetting.mark_repair_required(
        operation.operation_id,
        candidate_cleanup_completed,
        managed_derived_cleanup_completed,
        false,
        now_micros()?.as_unix_micros(),
    )?;
    Ok(forgetting_outcome(&operation, replayed, Some(diagnostic)))
}

fn forgetting_outcome(
    operation: &ForgettingOperationRecord,
    replayed: bool,
    diagnostic: Option<String>,
) -> ForgettingOutcome {
    let (record_kind, identity) = canonical_record_parts(operation.record);
    ForgettingOutcome {
        operation_id: operation.operation_id,
        record_kind,
        identity,
        state: operation.state,
        canonical_committed: operation.state != ForgettingState::Prepared,
        candidate_cleanup_completed: operation.candidate_cleanup_completed,
        managed_derived_cleanup_completed: operation.managed_derived_cleanup_completed,
        residue_verified: operation.residue_verified,
        replayed,
        provider_deletion: ProviderDeletionOutcome::NotRequested,
        diagnostic,
    }
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
