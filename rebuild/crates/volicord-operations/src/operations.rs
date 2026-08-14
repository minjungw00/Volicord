use crate::{
    AnalysisOutcome, BindingOutcome, CanonicalMutationOutcome, ChildProcessOutcome, Error,
    HealthIssue, HealthIssueKind, HealthReport, HealthState, LongOperationResult, OperationState,
    PartialOutcome, ProgressState, ProjectInitialization, PublicationOutcome, RepairKind,
    RepairOutcome, RuntimeLayout,
};
use crate::{
    ConfirmationDecision, ConfirmationRequestId, ConfirmationResponse, DispatchExpectation,
    GuardedEffectCandidate, GuardedEffectDispatcher, GuardedEffectDraft, GuardedOperationResult,
    GuardedStore,
};
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use volicord_context::{
    Availability, CanonicalReadBasis, CanonicalReadOptions, CanonicalRecordId, CheckpointDraft,
    Clock, ContextItemCorrectionDraft, ContextItemId, DecisionChoice, DecisionCorrectionDraft,
    DecisionId, DecisionSupersessionDraft, OperationId, Principal, PrincipalKind, ProjectId,
    SourceDraft, SourceId, SourcePayload, Store, SystemClock, TimestampMicros, UserTurnSource,
};
use volicord_inquiry::{
    compute_frontier, record_response_batch, ApplicabilityQuery, BatchResponseItem,
    BatchResponseResult, CandidateDraft, CandidateId, CandidateReadBasis, CandidateRecord,
    CandidateStore, FrontierRead, InquiryScope, PromotionResult, SubmissionOutcome,
};
use volicord_local_platform::{
    publish_file_no_replace, CancellationFlag, DirectoryEntryDurability, GitWorktreeLayout,
    NoReplacePublicationOutcome, ProcessCompletion, ProcessRequest, ProcessStopTrigger,
    ProcessTermination, ProcessTreeCleanup, RepositoryRoot,
};
use volicord_privacy::{
    PrivacyStore, ProjectPrivacyInspection, ProviderIntentProvenance, ProviderOptInEvent,
    ProviderOptInPolicy,
};
use volicord_projections::{
    build_project_projection, generate_documents, CandidateContentAccess, DocumentKind,
    DocumentRequest, DocumentSet, GeneratedDocument, OutputFormat, ProjectProjection,
    ProjectProjectionInputs, ProjectionBound, RecallBound, RecallInputs, ResumeBrief,
};
use volicord_repository_intelligence::{
    analyze_repository, AnalysisSnapshot, CanonicalGrounding, CapabilityState, InventoryRequest,
    StructuralAnalysisRequest,
};

pub struct LocalOperations {
    layout: RuntimeLayout,
}

struct PreparedAnalysisBasis {
    root: RepositoryRoot,
    requested_path: PathBuf,
    canonical: CanonicalReadBasis,
    repository_source: SourceId,
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
            excluded_paths,
            false,
        )
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
        .map_err(|error| Error::with_source("cannot create repository inventory request", error))?;
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
    digest.update(observed_at.to_be_bytes());
    Ok(format!("local-observation:sha256:{:x}", digest.finalize()))
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
