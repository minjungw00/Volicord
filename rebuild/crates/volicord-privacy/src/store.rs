use crate::model::PreparedSource;
use crate::{
    AuthorityKind, AuthorityObservation, AuthorizedProviderDispatch, BackgroundSemanticRequest,
    CanonicalForgettingCleanup, Error, ErrorKind, FilterOutcome, LocalDeletion,
    ManagedCanonicalLink, ManagedDeletionResult, ManagedDeletionScope, ManagedDerivedDraft,
    ManagedDerivedId, ManagedDerivedKind, ManagedDerivedRecord, ManagedDerivedState,
    PreparationOutcome, ProjectPrivacyInspection, ProviderAvailability, ProviderConfigurationState,
    ProviderDeletionOutcome, ProviderDeletionRequest, ProviderExecution, ProviderIdentity,
    ProviderIntentProvenance, ProviderInvocation, ProviderInvocationSource, ProviderOptInEvent,
    ProviderOptInPolicy, ProviderOptInState, ProviderRequestId, ProviderRequestOutcome,
    ProviderRequestRecord, ScopeOutcome, SourceManifestEntry, TransmissionOutcome,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use volicord_context::{
    CanonicalInvalidation, CanonicalReadBasis, Clock, IdGenerator, PrincipalKind, ProjectId,
    SourcePayload, SystemClock, SystemIdGenerator,
};

pub const PRIVACY_SCHEMA_KIND: &str = "volicord-project-privacy";
pub const PRIVACY_SCHEMA_VERSION: u32 = 1;

const MAX_TEXT_BYTES: usize = 16_384;
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_SOURCES: usize = 4_096;

pub trait BackgroundSemanticProvider {
    fn identity(&self) -> ProviderIdentity;
    fn availability(&self) -> ProviderAvailability;
    fn invoke(&mut self, request: ProviderInvocation) -> ProviderExecution;
    fn delete(&mut self, request: ProviderDeletionRequest) -> ProviderDeletionOutcome;
}

pub struct PrivacyStore {
    connection: Connection,
    ids: Box<dyn IdGenerator>,
    clock: Box<dyn Clock>,
}

impl PrivacyStore {
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
                "privacy store path must be explicit",
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

    pub fn enable(
        &mut self,
        canonical: &CanonicalReadBasis,
        policy: ProviderOptInPolicy,
        intent: ProviderIntentProvenance,
    ) -> Result<ProviderOptInEvent, Error> {
        self.record_opt_in(canonical, policy, ProviderOptInState::Enabled, intent)
    }

    pub fn disable(
        &mut self,
        canonical: &CanonicalReadBasis,
        project_id: ProjectId,
        intent: ProviderIntentProvenance,
    ) -> Result<ProviderOptInEvent, Error> {
        let current = self.current_opt_in(project_id)?.ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                "a never-enabled Project has no provider policy to disable",
            )
        })?;
        self.record_opt_in(
            canonical,
            current.policy,
            ProviderOptInState::Disabled,
            intent,
        )
    }

    pub fn revoke(
        &mut self,
        canonical: &CanonicalReadBasis,
        project_id: ProjectId,
        intent: ProviderIntentProvenance,
    ) -> Result<ProviderOptInEvent, Error> {
        let current = self.current_opt_in(project_id)?.ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                "a never-enabled Project has no provider policy to revoke",
            )
        })?;
        self.record_opt_in(
            canonical,
            current.policy,
            ProviderOptInState::Revoked,
            intent,
        )
    }

    pub fn record_authority_observation(
        &mut self,
        mut observation: AuthorityObservation,
    ) -> Result<AuthorityObservation, Error> {
        validate_text("authority purpose", &observation.purpose)?;
        validate_text(
            "authority request or operation",
            &observation.request_or_operation,
        )?;
        if observation.kind == AuthorityKind::InteractiveCurrentHostAccess
            && (observation.host.as_deref().is_none_or(str::is_empty)
                || observation.session.as_deref().is_none_or(str::is_empty))
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "interactive authority requires a current host and session",
            ));
        }
        observation.observed_at = self.clock.now().map_err(clock_error)?;
        let encoded = encode(&observation)?;
        self.connection
            .execute(
                "INSERT INTO authority_observations(project_id, observed_at, observation_json)
                 VALUES (?1, ?2, ?3)",
                params![
                    observation.project_id.as_bytes().as_slice(),
                    observation.observed_at.as_unix_micros(),
                    encoded,
                ],
            )
            .map_err(write_error)?;
        Ok(observation)
    }

    pub fn prepare_background_request(
        &mut self,
        request: BackgroundSemanticRequest,
    ) -> Result<PreparationOutcome, Error> {
        validate_request(&request)?;
        let request_id = ProviderRequestId::from_bytes(self.ids.next_id().map_err(id_error)?);
        let now = self.clock.now().map_err(clock_error)?;
        let current = self.current_opt_in(request.project_id)?;
        let rejection = authorization_rejection(current.as_ref(), &request);

        if let Some(diagnostic) = rejection {
            let manifest = request
                .sources
                .iter()
                .map(|source| SourceManifestEntry {
                    source: source.source.clone(),
                    locator: source.locator.clone(),
                    class: source.class,
                    scope_outcome: ScopeOutcome::OutsideOptInScope,
                    filter_outcome: FilterOutcome::NotApplied,
                    transmission_outcome: TransmissionOutcome::NotTransmitted,
                    original_bytes: source.body.len() as u64,
                    transmitted_bytes: 0,
                    filtered_line_count: 0,
                    reason: Some(diagnostic.clone()),
                })
                .collect();
            let record = ProviderRequestRecord {
                id: request_id,
                project_id: request.project_id,
                opt_in_revision: current.as_ref().map(|event| event.revision),
                repository_snapshot: request.repository_snapshot,
                analysis_snapshot: request.analysis_snapshot,
                provider: request.provider,
                model: request.model,
                purpose: request.purpose,
                requested_capability: request.requested_capability,
                requested_source_scopes: request.requested_source_scopes,
                manifest,
                outcome: ProviderRequestOutcome::NotAuthorized,
                diagnostic: Some(diagnostic),
                requested_at: now,
                completed_at: Some(now),
            };
            self.insert_request(&record)?;
            return Ok(PreparationOutcome::Rejected(record));
        }

        let event = current.ok_or_else(|| {
            Error::new(
                ErrorKind::CorruptState,
                "authorized request lost its opt-in",
            )
        })?;
        let mut manifest = Vec::with_capacity(request.sources.len());
        let mut prepared = Vec::new();
        for source in request.sources {
            let mut entry = SourceManifestEntry {
                source: source.source.clone(),
                locator: source.locator.clone(),
                class: source.class,
                scope_outcome: ScopeOutcome::Included,
                filter_outcome: FilterOutcome::NotApplied,
                transmission_outcome: TransmissionOutcome::NotTransmitted,
                original_bytes: source.body.len() as u64,
                transmitted_bytes: 0,
                filtered_line_count: 0,
                reason: None,
            };
            if !matches_any_scope(&source.locator, &request.requested_source_scopes) {
                entry.scope_outcome = ScopeOutcome::OutsideRequestedScope;
                entry.reason = Some("source is outside the requested transmission scope".into());
            } else if !matches_any_scope(&source.locator, &event.policy.allowed_source_scopes) {
                entry.scope_outcome = ScopeOutcome::OutsideOptInScope;
                entry.reason = Some("source is outside the current Project opt-in".into());
            } else if event.policy.exclusions.file_classes.contains(&source.class)
                || event
                    .policy
                    .exclusions
                    .path_prefixes
                    .iter()
                    .any(|prefix| path_matches_scope(&source.locator, prefix))
            {
                entry.scope_outcome = ScopeOutcome::Excluded;
                entry.reason = Some(event.policy.exclusions.basis.clone());
            } else {
                let filtered = filter_body(&source.body, &event.policy.filtering);
                entry.filter_outcome = filtered.outcome;
                entry.filtered_line_count = filtered.filtered_line_count;
                prepared.push(PreparedSource {
                    source: source.source,
                    locator: source.locator,
                    filtered_body: filtered.body,
                });
            }
            manifest.push(entry);
        }

        let outcome = if prepared.is_empty() {
            ProviderRequestOutcome::NotTransmitted
        } else {
            ProviderRequestOutcome::Prepared
        };
        let diagnostic = (prepared.is_empty()).then(|| {
            "no Source remained after requested scope, opt-in scope, and exclusion checks".into()
        });
        let record = ProviderRequestRecord {
            id: request_id,
            project_id: request.project_id,
            opt_in_revision: Some(event.revision),
            repository_snapshot: request.repository_snapshot,
            analysis_snapshot: request.analysis_snapshot,
            provider: request.provider,
            model: request.model,
            purpose: request.purpose,
            requested_capability: request.requested_capability,
            requested_source_scopes: request.requested_source_scopes,
            manifest,
            outcome,
            diagnostic,
            requested_at: now,
            completed_at: prepared.is_empty().then_some(now),
        };
        self.insert_request(&record)?;
        if prepared.is_empty() {
            Ok(PreparationOutcome::Rejected(record))
        } else {
            Ok(PreparationOutcome::Ready(AuthorizedProviderDispatch {
                request: record,
                sources: prepared,
            }))
        }
    }

    /// Dispatch is intentionally separate from preparation. A later Local
    /// Operations layer can place this exact call behind Guarded confirmation.
    /// Provider opt-in is revalidated here and is not itself dispatch approval.
    pub fn dispatch_background(
        &mut self,
        prepared: AuthorizedProviderDispatch,
        provider: &mut dyn BackgroundSemanticProvider,
    ) -> Result<ProviderRequestRecord, Error> {
        let mut record = prepared.request;
        let now = self.clock.now().map_err(clock_error)?;
        let current = self.current_opt_in(record.project_id)?;
        let current_revision = current.as_ref().map(|event| event.revision);
        let still_authorized = current.as_ref().is_some_and(|event| {
            event.state == ProviderOptInState::Enabled
                && Some(event.revision) == record.opt_in_revision
                && event.policy.provider == record.provider
                && event.policy.model == record.model
                && event.policy.purpose == record.purpose
                && event.policy.requested_capability == record.requested_capability
        });
        if !still_authorized {
            record.outcome = ProviderRequestOutcome::NotAuthorized;
            record.diagnostic = Some(format!(
                "Project opt-in changed before dispatch (prepared {:?}, current {:?})",
                record.opt_in_revision, current_revision
            ));
            record.completed_at = Some(now);
            self.update_request(&record)?;
            return Ok(record);
        }

        let identity = provider.identity();
        if identity.provider != record.provider || identity.model != record.model {
            record.outcome = ProviderRequestOutcome::ProviderUnavailable;
            record.diagnostic = Some("configured adapter identity does not match opt-in".into());
            record.completed_at = Some(now);
            self.update_request(&record)?;
            return Ok(record);
        }
        if let ProviderAvailability::Unavailable { diagnostic } = provider.availability() {
            record.outcome = ProviderRequestOutcome::ProviderUnavailable;
            record.diagnostic = Some(diagnostic);
            record.completed_at = Some(now);
            self.update_request(&record)?;
            return Ok(record);
        }

        let transmitted_ids = prepared
            .sources
            .iter()
            .map(|source| source.source.identity())
            .collect::<BTreeSet<_>>();
        let transmitted_lengths = prepared
            .sources
            .iter()
            .map(|source| (source.source.identity(), source.filtered_body.len() as u64))
            .collect::<BTreeMap<_, _>>();
        for entry in &mut record.manifest {
            if let Some(length) = transmitted_lengths.get(&entry.source.identity()) {
                entry.transmission_outcome = TransmissionOutcome::Transmitted;
                entry.transmitted_bytes = *length;
            }
        }
        let invocation = ProviderInvocation {
            request_id: record.id,
            project_id: record.project_id,
            repository_snapshot: record.repository_snapshot,
            analysis_snapshot: record.analysis_snapshot,
            provider: record.provider.clone(),
            model: record.model.clone(),
            purpose: record.purpose.clone(),
            requested_capability: record.requested_capability.clone(),
            sources: prepared
                .sources
                .iter()
                .map(|source| ProviderInvocationSource {
                    source: source.source.clone(),
                    locator: source.locator.clone(),
                    filtered_body: source.filtered_body.clone(),
                })
                .collect(),
        };
        let result = provider.invoke(invocation);
        let (outcome, annotations, diagnostic, derived_state) = match result {
            ProviderExecution::Completed { annotations } => (
                ProviderRequestOutcome::Completed,
                annotations,
                None,
                ManagedDerivedState::Current,
            ),
            ProviderExecution::Partial {
                annotations,
                diagnostic,
            } => (
                ProviderRequestOutcome::Partial,
                annotations,
                Some(diagnostic),
                ManagedDerivedState::Current,
            ),
            ProviderExecution::Stale {
                annotations,
                diagnostic,
            } => (
                ProviderRequestOutcome::Stale,
                annotations,
                Some(diagnostic),
                ManagedDerivedState::Stale,
            ),
            ProviderExecution::Failed { diagnostic } => (
                ProviderRequestOutcome::ProviderFailed,
                Vec::new(),
                Some(diagnostic),
                ManagedDerivedState::Invalidated,
            ),
        };
        let invalid_annotation = annotations.iter().find_map(|annotation| {
            annotation
                .included_sources
                .iter()
                .find(|source| !transmitted_ids.contains(source))
                .copied()
        });
        if let Some(source) = invalid_annotation {
            record.outcome = ProviderRequestOutcome::ProviderFailed;
            record.diagnostic = Some(format!(
                "provider response referenced non-transmitted Source {source}"
            ));
        } else {
            let current_policy = current.ok_or_else(|| {
                Error::new(ErrorKind::CorruptState, "dispatch policy disappeared")
            })?;
            let by_id = prepared
                .sources
                .iter()
                .map(|source| (source.source.identity(), source.source.clone()))
                .collect::<BTreeMap<_, _>>();
            for annotation in annotations {
                let included_sources = annotation
                    .included_sources
                    .iter()
                    .filter_map(|source| by_id.get(source).cloned())
                    .collect::<Vec<_>>();
                let links = annotation
                    .included_sources
                    .iter()
                    .copied()
                    .map(ManagedCanonicalLink::Source)
                    .collect();
                self.record_derived_with_state(
                    ManagedDerivedDraft {
                        project_id: record.project_id,
                        kind: ManagedDerivedKind::SemanticAnnotation,
                        provider: Some(record.provider.clone()),
                        model: Some(record.model.clone()),
                        purpose: record.purpose.clone(),
                        analysis_snapshot: Some(record.analysis_snapshot),
                        included_sources,
                        canonical_links: links,
                        content: annotation.text,
                        uncertainty: Some(annotation.uncertainty),
                        retained_until: current_policy
                            .policy
                            .retention
                            .local_annotation_retained_until,
                        retention_basis: current_policy.policy.retention.local_basis.clone(),
                    },
                    derived_state,
                )?;
            }
            record.outcome = outcome;
            record.diagnostic = diagnostic;
        }
        record.completed_at = Some(self.clock.now().map_err(clock_error)?);
        self.update_request(&record)?;
        Ok(record)
    }

    pub fn record_managed_derived(
        &mut self,
        draft: ManagedDerivedDraft,
    ) -> Result<ManagedDerivedRecord, Error> {
        self.record_derived_with_state(draft, ManagedDerivedState::Current)
    }

    pub fn delete_managed(
        &mut self,
        scope: &ManagedDeletionScope,
        basis: impl Into<String>,
        provider: Option<&mut dyn BackgroundSemanticProvider>,
    ) -> Result<ManagedDeletionResult, Error> {
        let basis = basis.into();
        validate_text("managed deletion basis", &basis)?;
        let now = self.clock.now().map_err(clock_error)?;
        let records = self.read_derived(scope.project_id)?;
        let mut deleted = Vec::new();
        let mut provider_sources = BTreeSet::new();
        let mut provider_name = scope.provider.clone();
        for mut record in records {
            if record.state == ManagedDerivedState::Deleted || !scope_matches(scope, &record) {
                continue;
            }
            record.content = None;
            record.state = ManagedDerivedState::Deleted;
            record.local_deletion = Some(LocalDeletion {
                deleted_at: now,
                basis: basis.clone(),
            });
            for source in &record.included_sources {
                provider_sources.insert(source.identity());
            }
            if provider_name.is_none() {
                provider_name.clone_from(&record.provider);
            }
            self.update_derived(&record)?;
            deleted.push(record.id);
        }

        let provider_outcome = if deleted.is_empty() {
            ProviderDeletionOutcome::NotRequested
        } else if let (Some(adapter), Some(expected_provider)) = (provider, provider_name) {
            let identity = adapter.identity();
            if identity.provider != expected_provider {
                ProviderDeletionOutcome::Unknown {
                    diagnostic: "deletion adapter identity does not match retained provider".into(),
                }
            } else {
                adapter.delete(ProviderDeletionRequest {
                    project_id: scope.project_id,
                    managed_ids: deleted.clone(),
                    source_ids: provider_sources.into_iter().collect(),
                    provider: expected_provider,
                })
            }
        } else {
            ProviderDeletionOutcome::NotRequested
        };
        for id in &deleted {
            let mut record = self.get_derived(scope.project_id, *id)?;
            record.provider_deletion = provider_outcome.clone();
            self.update_derived(&record)?;
        }
        Ok(ManagedDeletionResult {
            locally_deleted: deleted,
            provider_outcome,
        })
    }

    pub fn cleanup_expired(
        &mut self,
        project_id: ProjectId,
    ) -> Result<Vec<ManagedDerivedId>, Error> {
        let now = self.clock.now().map_err(clock_error)?;
        let mut deleted = Vec::new();
        for mut record in self.read_derived(project_id)? {
            if record.state != ManagedDerivedState::Deleted
                && record.retained_until.is_some_and(|expiry| expiry <= now)
            {
                record.content = None;
                record.state = ManagedDerivedState::Deleted;
                record.local_deletion = Some(LocalDeletion {
                    deleted_at: now,
                    basis: record.retention_basis.clone(),
                });
                self.update_derived(&record)?;
                deleted.push(record.id);
            }
        }
        Ok(deleted)
    }

    pub fn invalidate_snapshot(
        &mut self,
        project_id: ProjectId,
        current_snapshot: volicord_repository_intelligence::AnalysisSnapshotId,
    ) -> Result<Vec<ManagedDerivedId>, Error> {
        let mut invalidated = Vec::new();
        for mut record in self.read_derived(project_id)? {
            if record.state == ManagedDerivedState::Current
                && record
                    .analysis_snapshot
                    .is_some_and(|snapshot| snapshot != current_snapshot)
            {
                record.state = ManagedDerivedState::Stale;
                self.update_derived(&record)?;
                invalidated.push(record.id);
            }
        }
        Ok(invalidated)
    }

    pub fn apply_canonical_forgetting(
        &mut self,
        candidates: &mut volicord_inquiry::CandidateStore,
        invalidation: &CanonicalInvalidation,
        basis: impl Into<String>,
    ) -> Result<CanonicalForgettingCleanup, Error> {
        let basis = basis.into();
        validate_text("canonical forgetting propagation basis", &basis)?;
        let candidate_ids = candidates
            .cleanup_related_to_canonical(invalidation, basis.clone())
            .map_err(|error| {
                Error::with_source(
                    ErrorKind::StorageUnavailable,
                    "related Candidate cleanup failed",
                    error,
                )
            })?;
        let target = ManagedCanonicalLink::from_invalidation(invalidation);
        let now = self.clock.now().map_err(clock_error)?;
        let mut derived_ids = Vec::new();
        for mut record in self.read_derived(invalidation.project_id)? {
            if record.state != ManagedDerivedState::Deleted
                && record.canonical_links.contains(&target)
            {
                record.content = None;
                record.state = ManagedDerivedState::Deleted;
                record.local_deletion = Some(LocalDeletion {
                    deleted_at: now,
                    basis: basis.clone(),
                });
                self.update_derived(&record)?;
                derived_ids.push(record.id);
            }
        }
        sanitize_deleted_content(&self.connection)?;
        Ok(CanonicalForgettingCleanup {
            candidate_ids,
            derived_ids,
        })
    }

    pub fn inspect_project(
        &self,
        project_id: ProjectId,
    ) -> Result<ProjectPrivacyInspection, Error> {
        let current_opt_in = self.current_opt_in(project_id)?;
        let configuration_state = match current_opt_in.as_ref().map(|event| event.state) {
            None => ProviderConfigurationState::NeverEnabled,
            Some(ProviderOptInState::Enabled) => ProviderConfigurationState::Enabled,
            Some(ProviderOptInState::Disabled) => ProviderConfigurationState::Disabled,
            Some(ProviderOptInState::Revoked) => ProviderConfigurationState::Revoked,
        };
        Ok(ProjectPrivacyInspection {
            project_id,
            configuration_state,
            current_opt_in,
            authority_observations: self.read_authority_observations(project_id)?,
            requests: self.read_requests(project_id)?,
            managed_derived: self.read_derived(project_id)?,
            withheld_for_canonical_forgetting: Vec::new(),
        })
    }

    pub fn inspect_project_with_invalidations(
        &self,
        project_id: ProjectId,
        invalidations: &[CanonicalInvalidation],
    ) -> Result<ProjectPrivacyInspection, Error> {
        let mut inspection = self.inspect_project(project_id)?;
        for record in &mut inspection.managed_derived {
            let related = invalidations.iter().any(|invalidation| {
                invalidation.project_id == project_id
                    && record
                        .canonical_links
                        .contains(&ManagedCanonicalLink::from_invalidation(invalidation))
            });
            if related && record.state != ManagedDerivedState::Deleted {
                record.content = None;
                record.state = ManagedDerivedState::Invalidated;
                inspection.withheld_for_canonical_forgetting.push(record.id);
            }
        }
        inspection.withheld_for_canonical_forgetting.sort();
        inspection.withheld_for_canonical_forgetting.dedup();
        Ok(inspection)
    }

    pub fn verify_canonical_forgetting(
        &self,
        invalidation: &CanonicalInvalidation,
    ) -> Result<bool, Error> {
        let target = ManagedCanonicalLink::from_invalidation(invalidation);
        Ok(self
            .read_derived(invalidation.project_id)?
            .iter()
            .filter(|record| record.canonical_links.contains(&target))
            .all(|record| record.content.is_none() && record.state == ManagedDerivedState::Deleted))
    }

    pub fn provider_request(
        &self,
        project_id: ProjectId,
        request_id: ProviderRequestId,
    ) -> Result<ProviderRequestRecord, Error> {
        let encoded = self
            .connection
            .query_row(
                "SELECT record_json FROM provider_requests WHERE project_id = ?1 AND id = ?2",
                params![
                    project_id.as_bytes().as_slice(),
                    request_id.as_bytes().as_slice()
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(read_error)?
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "provider request was not found"))?;
        decode(&encoded)
    }

    pub fn get_derived(
        &self,
        project_id: ProjectId,
        id: ManagedDerivedId,
    ) -> Result<ManagedDerivedRecord, Error> {
        let encoded = self
            .connection
            .query_row(
                "SELECT record_json FROM managed_derived WHERE project_id = ?1 AND id = ?2",
                params![project_id.as_bytes().as_slice(), id.as_bytes().as_slice()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(read_error)?
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "managed Derived record not found"))?;
        decode(&encoded)
    }

    fn record_opt_in(
        &mut self,
        canonical: &CanonicalReadBasis,
        policy: ProviderOptInPolicy,
        state: ProviderOptInState,
        intent: ProviderIntentProvenance,
    ) -> Result<ProviderOptInEvent, Error> {
        validate_policy(&policy)?;
        validate_intent(canonical, policy.project_id, &intent)?;
        let revision = self
            .current_opt_in(policy.project_id)?
            .map_or(Ok(1), |event| {
                event.revision.checked_add(1).ok_or_else(|| {
                    Error::new(ErrorKind::CorruptState, "provider opt-in revision overflow")
                })
            })?;
        let event = ProviderOptInEvent {
            revision,
            state,
            policy,
            intent,
            recorded_at: self.clock.now().map_err(clock_error)?,
        };
        let encoded = encode(&event)?;
        self.connection
            .execute(
                "INSERT INTO opt_in_events(project_id, revision, event_json)
                 VALUES (?1, ?2, ?3)",
                params![
                    event.policy.project_id.as_bytes().as_slice(),
                    i64::try_from(revision).map_err(|_| Error::new(
                        ErrorKind::CorruptState,
                        "provider opt-in revision exceeds storage range"
                    ))?,
                    encoded,
                ],
            )
            .map_err(write_error)?;
        Ok(event)
    }

    fn current_opt_in(&self, project_id: ProjectId) -> Result<Option<ProviderOptInEvent>, Error> {
        let encoded = self
            .connection
            .query_row(
                "SELECT event_json FROM opt_in_events
                 WHERE project_id = ?1 ORDER BY revision DESC LIMIT 1",
                [project_id.as_bytes().as_slice()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(read_error)?;
        encoded.map(|value| decode(&value)).transpose()
    }

    fn insert_request(&self, record: &ProviderRequestRecord) -> Result<(), Error> {
        self.connection
            .execute(
                "INSERT INTO provider_requests(id, project_id, requested_at, record_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    record.id.as_bytes().as_slice(),
                    record.project_id.as_bytes().as_slice(),
                    record.requested_at.as_unix_micros(),
                    encode(record)?,
                ],
            )
            .map_err(write_error)?;
        Ok(())
    }

    fn update_request(&self, record: &ProviderRequestRecord) -> Result<(), Error> {
        let count = self
            .connection
            .execute(
                "UPDATE provider_requests SET record_json = ?3 WHERE id = ?1 AND project_id = ?2",
                params![
                    record.id.as_bytes().as_slice(),
                    record.project_id.as_bytes().as_slice(),
                    encode(record)?,
                ],
            )
            .map_err(write_error)?;
        if count != 1 {
            return Err(Error::new(
                ErrorKind::NotFound,
                "prepared provider request no longer exists",
            ));
        }
        Ok(())
    }

    fn record_derived_with_state(
        &mut self,
        draft: ManagedDerivedDraft,
        state: ManagedDerivedState,
    ) -> Result<ManagedDerivedRecord, Error> {
        validate_derived_draft(&draft)?;
        let id = ManagedDerivedId::from_bytes(self.ids.next_id().map_err(id_error)?);
        let created_at = self.clock.now().map_err(clock_error)?;
        let record = ManagedDerivedRecord {
            id,
            project_id: draft.project_id,
            kind: draft.kind,
            provider: draft.provider,
            model: draft.model,
            purpose: draft.purpose,
            analysis_snapshot: draft.analysis_snapshot,
            included_sources: draft.included_sources,
            canonical_links: draft.canonical_links,
            content: Some(draft.content),
            uncertainty: draft.uncertainty,
            created_at,
            retained_until: draft.retained_until,
            retention_basis: draft.retention_basis,
            state,
            local_deletion: None,
            provider_deletion: ProviderDeletionOutcome::NotRequested,
        };
        self.connection
            .execute(
                "INSERT INTO managed_derived(id, project_id, created_at, record_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    id.as_bytes().as_slice(),
                    record.project_id.as_bytes().as_slice(),
                    created_at.as_unix_micros(),
                    encode(&record)?,
                ],
            )
            .map_err(write_error)?;
        Ok(record)
    }

    fn update_derived(&self, record: &ManagedDerivedRecord) -> Result<(), Error> {
        let count = self
            .connection
            .execute(
                "UPDATE managed_derived SET record_json = ?3 WHERE id = ?1 AND project_id = ?2",
                params![
                    record.id.as_bytes().as_slice(),
                    record.project_id.as_bytes().as_slice(),
                    encode(record)?,
                ],
            )
            .map_err(write_error)?;
        if count != 1 {
            return Err(Error::new(
                ErrorKind::NotFound,
                "managed Derived record no longer exists",
            ));
        }
        Ok(())
    }

    fn read_authority_observations(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<AuthorityObservation>, Error> {
        read_json_rows(
            &self.connection,
            "SELECT observation_json FROM authority_observations
             WHERE project_id = ?1 ORDER BY observed_at, rowid",
            project_id,
        )
    }

    fn read_requests(&self, project_id: ProjectId) -> Result<Vec<ProviderRequestRecord>, Error> {
        read_json_rows(
            &self.connection,
            "SELECT record_json FROM provider_requests
             WHERE project_id = ?1 ORDER BY requested_at, id",
            project_id,
        )
    }

    fn read_derived(&self, project_id: ProjectId) -> Result<Vec<ManagedDerivedRecord>, Error> {
        read_json_rows(
            &self.connection,
            "SELECT record_json FROM managed_derived
             WHERE project_id = ?1 ORDER BY created_at, id",
            project_id,
        )
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
            "managed Derived cleanup committed but WAL truncation is busy",
        ));
    }
    connection
        .execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(write_error)
}

struct FilteredBody {
    body: String,
    outcome: FilterOutcome,
    filtered_line_count: u64,
}

fn filter_body(body: &str, policy: &crate::SecretFilteringPolicy) -> FilteredBody {
    if !policy.enabled {
        return FilteredBody {
            body: body.to_owned(),
            outcome: FilterOutcome::NotApplied,
            filtered_line_count: 0,
        };
    }
    let mut output = String::new();
    let mut count = 0_u64;
    for line in body.split_inclusive('\n') {
        if policy
            .line_markers
            .iter()
            .any(|marker| !marker.is_empty() && line.contains(marker))
        {
            output.push_str(&policy.replacement);
            if line.ends_with('\n') {
                output.push('\n');
            }
            count += 1;
        } else {
            output.push_str(line);
        }
    }
    FilteredBody {
        body: output,
        outcome: if count == 0 {
            FilterOutcome::NoMatch
        } else {
            FilterOutcome::Filtered
        },
        filtered_line_count: count,
    }
}

fn authorization_rejection(
    current: Option<&ProviderOptInEvent>,
    request: &BackgroundSemanticRequest,
) -> Option<String> {
    let Some(event) = current else {
        return Some("Project has never enabled background provider processing".into());
    };
    if event.state != ProviderOptInState::Enabled {
        return Some(format!("Project provider state is {:?}", event.state));
    }
    if event.policy.provider != request.provider
        || event.policy.model != request.model
        || event.policy.purpose != request.purpose
        || event.policy.requested_capability != request.requested_capability
    {
        return Some("request provider, model, purpose, or capability differs from opt-in".into());
    }
    if request.requested_source_scopes.iter().any(|requested| {
        !event
            .policy
            .allowed_source_scopes
            .iter()
            .any(|allowed| scope_is_within(requested, allowed))
    }) {
        return Some("requested source scope exceeds the current Project opt-in".into());
    }
    None
}

fn matches_any_scope(locator: &str, scopes: &[String]) -> bool {
    scopes
        .iter()
        .any(|scope| path_matches_scope(locator, scope))
}

fn scope_is_within(requested: &str, allowed: &str) -> bool {
    requested == allowed || path_matches_scope(requested, allowed)
}

fn path_matches_scope(locator: &str, scope: &str) -> bool {
    scope == "."
        || locator == scope
        || locator
            .strip_prefix(scope)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

fn scope_matches(scope: &ManagedDeletionScope, record: &ManagedDerivedRecord) -> bool {
    (scope.kinds.is_empty() || scope.kinds.contains(&record.kind))
        && scope
            .provider
            .as_ref()
            .is_none_or(|provider| record.provider.as_ref() == Some(provider))
        && (scope.source_ids.is_empty()
            || record
                .included_sources
                .iter()
                .any(|source| scope.source_ids.contains(&source.identity())))
}

fn validate_request(request: &BackgroundSemanticRequest) -> Result<(), Error> {
    for (label, value) in [
        ("provider", request.provider.as_str()),
        ("model", request.model.as_str()),
        ("purpose", request.purpose.as_str()),
        (
            "requested capability",
            request.requested_capability.as_str(),
        ),
    ] {
        validate_text(label, value)?;
    }
    if request.requested_source_scopes.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "background request requires an explicit source scope",
        ));
    }
    validate_scopes(&request.requested_source_scopes)?;
    if request.sources.len() > MAX_SOURCES {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "background request exceeds the Source manifest limit",
        ));
    }
    for source in &request.sources {
        if source.source.project() != request.project_id {
            return Err(Error::new(
                ErrorKind::WrongProject,
                "background Source belongs to a different Project",
            ));
        }
        validate_locator(&source.locator)?;
        if source.body.len() > MAX_BODY_BYTES {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "one background Source body exceeds the in-memory request limit",
            ));
        }
    }
    Ok(())
}

fn validate_policy(policy: &ProviderOptInPolicy) -> Result<(), Error> {
    for (label, value) in [
        ("provider", policy.provider.as_str()),
        ("model", policy.model.as_str()),
        ("purpose", policy.purpose.as_str()),
        ("requested capability", policy.requested_capability.as_str()),
        ("exclusion basis", policy.exclusions.basis.as_str()),
        (
            "local retention basis",
            policy.retention.local_basis.as_str(),
        ),
        (
            "provider retention expectation",
            policy.retention.provider_expectation.as_str(),
        ),
    ] {
        validate_text(label, value)?;
    }
    if policy.allowed_source_scopes.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "provider opt-in requires an explicit allowed source scope",
        ));
    }
    validate_scopes(&policy.allowed_source_scopes)?;
    validate_scopes(&policy.exclusions.path_prefixes)?;
    if policy.filtering.enabled && policy.filtering.known_limits.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "secret filtering must state at least one known limit",
        ));
    }
    for value in &policy.filtering.known_limits {
        validate_text("filtering known limit", value)?;
    }
    for value in &policy.retention.provider_known_limits {
        validate_text("provider retention known limit", value)?;
    }
    validate_text("filter replacement", &policy.filtering.replacement)?;
    Ok(())
}

fn validate_intent(
    canonical: &CanonicalReadBasis,
    project_id: ProjectId,
    intent: &ProviderIntentProvenance,
) -> Result<(), Error> {
    if canonical.project.id != project_id {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "provider intent canonical basis belongs to a different Project",
        ));
    }
    if intent.actor.kind != PrincipalKind::User {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "provider opt-in changes require explicit user provenance",
        ));
    }
    for (label, value) in [
        ("intent actor", intent.actor.identity.as_str()),
        ("intent host", intent.host.as_str()),
        ("intent session", intent.session.as_str()),
        ("intent basis", intent.basis.as_str()),
    ] {
        validate_text(label, value)?;
    }
    let source = canonical
        .sources
        .iter()
        .find(|source| source.source.id == intent.user_turn_source)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "provider intent must reference a current canonical user-turn Source",
            )
        })?;
    match &source.source.payload {
        SourcePayload::CurrentHostUserTurn { host, session, .. }
            if source.source.project_id == project_id
                && source.source.actor.kind == PrincipalKind::User
                && source.source.actor.identity == intent.actor.identity
                && host == &intent.host
                && session == &intent.session => {}
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "provider intent Source is not the matching current-host user turn",
            ));
        }
    }
    Ok(())
}

fn validate_derived_draft(draft: &ManagedDerivedDraft) -> Result<(), Error> {
    validate_text("managed Derived purpose", &draft.purpose)?;
    validate_text("managed Derived retention basis", &draft.retention_basis)?;
    validate_text("managed Derived content", &draft.content)?;
    if draft
        .included_sources
        .iter()
        .any(|source| source.project() != draft.project_id)
    {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "managed Derived Source belongs to a different Project",
        ));
    }
    if draft.kind == ManagedDerivedKind::SemanticAnnotation
        && (draft.provider.as_deref().is_none_or(str::is_empty)
            || draft.model.as_deref().is_none_or(str::is_empty)
            || draft.analysis_snapshot.is_none()
            || draft.uncertainty.is_none())
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Semantic Annotation requires provider, model, Analysis Snapshot, and uncertainty",
        ));
    }
    Ok(())
}

fn validate_scopes(scopes: &[String]) -> Result<(), Error> {
    for scope in scopes {
        validate_locator(scope)?;
    }
    Ok(())
}

fn validate_locator(locator: &str) -> Result<(), Error> {
    validate_text("portable source locator", locator)?;
    let path = Path::new(locator);
    if locator.contains('\\')
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "source scopes and locators must be portable relative paths",
        ));
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES || value.contains('\0') {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{label} must be non-empty bounded text"),
        ));
    }
    Ok(())
}

fn initialize_or_validate(connection: &mut Connection) -> Result<(), Error> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(write_error)?;
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS privacy_metadata (
                 singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                 schema_kind TEXT NOT NULL,
                 schema_version INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS opt_in_events (
                 project_id BLOB NOT NULL,
                 revision INTEGER NOT NULL,
                 event_json TEXT NOT NULL,
                 PRIMARY KEY(project_id, revision)
             );
             CREATE TABLE IF NOT EXISTS authority_observations (
                 project_id BLOB NOT NULL,
                 observed_at INTEGER NOT NULL,
                 observation_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS provider_requests (
                 id BLOB PRIMARY KEY,
                 project_id BLOB NOT NULL,
                 requested_at INTEGER NOT NULL,
                 record_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS managed_derived (
                 id BLOB PRIMARY KEY,
                 project_id BLOB NOT NULL,
                 created_at INTEGER NOT NULL,
                 record_json TEXT NOT NULL
             );",
        )
        .map_err(write_error)?;
    let metadata = transaction
        .query_row(
            "SELECT schema_kind, schema_version FROM privacy_metadata WHERE singleton = 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
        )
        .optional()
        .map_err(read_error)?;
    match metadata {
        None => {
            transaction
                .execute(
                    "INSERT INTO privacy_metadata(singleton, schema_kind, schema_version)
                     VALUES (1, ?1, ?2)",
                    params![PRIVACY_SCHEMA_KIND, PRIVACY_SCHEMA_VERSION],
                )
                .map_err(write_error)?;
        }
        Some((kind, version))
            if kind == PRIVACY_SCHEMA_KIND && version == PRIVACY_SCHEMA_VERSION => {}
        Some((kind, version)) => {
            return Err(Error::new(
                ErrorKind::CorruptState,
                format!(
                    "unsupported privacy store metadata {kind} version {version}; expected {PRIVACY_SCHEMA_KIND} version {PRIVACY_SCHEMA_VERSION}"
                ),
            ));
        }
    }
    transaction.commit().map_err(write_error)
}

fn read_json_rows<T: serde::de::DeserializeOwned>(
    connection: &Connection,
    query: &str,
    project_id: ProjectId,
) -> Result<Vec<T>, Error> {
    let mut statement = connection.prepare(query).map_err(read_error)?;
    let rows = statement
        .query_map([project_id.as_bytes().as_slice()], |row| {
            row.get::<_, String>(0)
        })
        .map_err(read_error)?;
    let mut values = Vec::new();
    for row in rows {
        values.push(decode(&row.map_err(read_error)?)?);
    }
    Ok(values)
}

fn encode(value: &impl serde::Serialize) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|error| {
        Error::with_source(
            ErrorKind::CorruptState,
            "privacy record could not be encoded",
            error,
        )
    })
}

fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, Error> {
    serde_json::from_str(value).map_err(|error| {
        Error::with_source(
            ErrorKind::CorruptState,
            "privacy record could not be decoded",
            error,
        )
    })
}

fn open_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::StorageUnavailable,
        "privacy store could not be opened",
        error,
    )
}

fn read_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::StorageUnavailable,
        "privacy store read failed",
        error,
    )
}

fn write_error(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::StorageUnavailable,
        "privacy store write failed",
        error,
    )
}

fn id_error(error: volicord_context::Error) -> Error {
    Error::with_source(
        ErrorKind::StorageUnavailable,
        "privacy identity generation failed",
        error,
    )
}

fn clock_error(error: volicord_context::Error) -> Error {
    Error::with_source(
        ErrorKind::StorageUnavailable,
        "privacy clock observation failed",
        error,
    )
}
