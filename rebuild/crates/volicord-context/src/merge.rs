use crate::portable::{
    bundle_bytes, insert_row, sha256_hex, validate_bundle, validate_tables, value_bytes, value_key,
    value_text, Lineage, Payload, PortableTable, PortableValue, SemanticState, TABLES,
};
use crate::{
    Availability, Error, ErrorKind, OperationId, OperationResult, ProjectId, SourceId, Store,
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleBasis {
    pub checksum: String,
    pub history_basis: String,
    pub common_base_basis: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BundleConflictClass {
    IndependentAdditions,
    SameRecordRevision,
    SemanticDecisionConflict,
    DeleteModifyConflict,
    SourceBindingConflict,
    CommonBaseUnavailable,
}

impl BundleConflictClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::IndependentAdditions => "independent_additions",
            Self::SameRecordRevision => "same_record_revision",
            Self::SemanticDecisionConflict => "semantic_decision_conflict",
            Self::DeleteModifyConflict => "delete_modify_conflict",
            Self::SourceBindingConflict => "source_binding_conflict",
            Self::CommonBaseUnavailable => "common_base_unavailable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictSourceBasis {
    pub source_identity: String,
    pub base: Option<Availability>,
    pub local: Option<Availability>,
    pub incoming: Option<Availability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleConflict {
    pub conflict_identity: String,
    pub class: BundleConflictClass,
    pub affected_identities: Vec<String>,
    pub base_basis: Option<String>,
    pub local_basis: String,
    pub incoming_basis: String,
    pub sources: Vec<ConflictSourceBasis>,
    pub consequence: String,
    pub uncertainty: Vec<String>,
    pub automatic_resolution_allowed: bool,
    pub user_judgment_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceBindingCandidate {
    pub repository_identity: String,
    pub source_basis: Vec<SourceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleComparison {
    pub project_id: ProjectId,
    pub conflict_set_identity: String,
    pub conflict_revision: u64,
    pub common_base: Option<BundleBasis>,
    pub local: BundleBasis,
    pub incoming: BundleBasis,
    pub conflicts: Vec<BundleConflict>,
    pub already_present: bool,
}

impl BundleComparison {
    pub fn requires_user_resolution(&self) -> bool {
        self.conflicts
            .iter()
            .any(|conflict| !conflict.automatic_resolution_allowed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MergeResolutionMode {
    ChooseLocal,
    ChooseIncoming,
    ExplicitMerged { bundle_path: PathBuf },
    ContextBranch,
}

impl MergeResolutionMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ChooseLocal => "choose_local",
            Self::ChooseIncoming => "choose_incoming",
            Self::ExplicitMerged { .. } => "explicit_merged",
            Self::ContextBranch => "context_branch",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeResolution {
    pub conflict_set_identity: String,
    pub conflict_revision: u64,
    pub user_turn_source_id: SourceId,
    pub mode: MergeResolutionMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleMergeStatus {
    AlreadyPresent,
    MergedAutomatically,
    Resolved,
    Branched,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleMerge {
    pub project_id: ProjectId,
    pub conflict_set_identity: String,
    pub conflict_revision: u64,
    pub common_base_basis: Option<String>,
    pub local_history_basis: String,
    pub incoming_history_basis: String,
    pub result_history_basis: String,
    pub status: BundleMergeStatus,
    pub resolution_source_id: Option<SourceId>,
    pub affected_identities: Vec<String>,
    pub branch_history_basis: Option<String>,
}

type RowMap = BTreeMap<String, Vec<PortableValue>>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CanonicalRecordIdentity {
    kind: String,
    identity: String,
}

type RecordClosureMap = BTreeMap<CanonicalRecordIdentity, RowMap>;

struct ComparedState {
    comparison: BundleComparison,
    base: Option<Payload>,
    local: Payload,
    incoming: Payload,
}

impl Store {
    pub fn compare_bundle(
        &self,
        common_base_path: Option<&Path>,
        incoming_path: &Path,
        incoming_binding: Option<SourceBindingCandidate>,
    ) -> Result<BundleComparison, Error> {
        Ok(compare_state(self, common_base_path, incoming_path, incoming_binding)?.comparison)
    }

    pub fn merge_bundle(
        &mut self,
        operation_id: OperationId,
        common_base_path: Option<&Path>,
        incoming_path: &Path,
        incoming_binding: Option<SourceBindingCandidate>,
        resolution: Option<MergeResolution>,
    ) -> Result<OperationResult<BundleMerge>, Error> {
        let incoming = read_bundle(incoming_path)?;
        let base = common_base_path.map(read_bundle).transpose()?;
        let explicit = match resolution.as_ref().map(|value| &value.mode) {
            Some(MergeResolutionMode::ExplicitMerged { bundle_path }) => {
                Some(read_bundle(bundle_path)?)
            }
            _ => None,
        };
        let request_basis = merge_request_basis(
            &incoming,
            base.as_ref(),
            explicit.as_ref(),
            &incoming_binding,
            resolution.as_ref(),
        )?;
        if let Some(value) = load_replayed_merge(&self.connection, operation_id, &request_basis)? {
            return Ok(OperationResult {
                value,
                replayed: true,
            });
        }

        let compared = compare_state(self, common_base_path, incoming_path, incoming_binding)?;
        if compared.comparison.already_present {
            return self.commit_merge(
                operation_id,
                request_basis,
                compared,
                resolution,
                None,
                BundleMergeStatus::AlreadyPresent,
            );
        }
        if compared.comparison.requires_user_resolution() && resolution.is_none() {
            return Ok(OperationResult {
                value: merge_view(
                    &compared.comparison,
                    BundleMergeStatus::Unresolved,
                    None,
                    None,
                ),
                replayed: false,
            });
        }
        if let Some(resolution) = &resolution {
            validate_resolution(&self.connection, &compared.comparison, resolution)?;
        } else if compared.comparison.requires_user_resolution() {
            return Err(Error::new(
                ErrorKind::DomainConflict,
                "bundle conflict requires explicit user resolution",
            ));
        }

        let (target, status, branch_basis) = match resolution.as_ref().map(|value| &value.mode) {
            Some(MergeResolutionMode::ExplicitMerged { .. }) => {
                let validated = explicit.ok_or_else(|| {
                    Error::new(ErrorKind::InvalidInput, "explicit merged bundle is missing")
                })?;
                ensure_project(validated.project_id, compared.comparison.project_id)?;
                validate_tables(&validated.payload, validated.project_id)?;
                (validated.payload, BundleMergeStatus::Resolved, None)
            }
            Some(MergeResolutionMode::ContextBranch) => (
                build_target(&compared, SideChoice::Local)?,
                BundleMergeStatus::Branched,
                Some(compared.comparison.incoming.history_basis.clone()),
            ),
            Some(MergeResolutionMode::ChooseIncoming) => (
                build_target(&compared, SideChoice::Incoming)?,
                BundleMergeStatus::Resolved,
                None,
            ),
            Some(MergeResolutionMode::ChooseLocal) => (
                build_target(&compared, SideChoice::Local)?,
                BundleMergeStatus::Resolved,
                None,
            ),
            None => (
                build_target(&compared, SideChoice::Local)?,
                BundleMergeStatus::MergedAutomatically,
                None,
            ),
        };
        self.commit_merge(
            operation_id,
            request_basis,
            compared,
            resolution,
            Some(target),
            status,
        )
        .map(|mut result| {
            result.value.branch_history_basis = branch_basis;
            result
        })
    }

    fn commit_merge(
        &mut self,
        operation_id: OperationId,
        request_basis: Vec<u8>,
        compared: ComparedState,
        resolution: Option<MergeResolution>,
        target: Option<Payload>,
        status: BundleMergeStatus,
    ) -> Result<OperationResult<BundleMerge>, Error> {
        let project_id = compared.comparison.project_id;
        let result_basis = if let Some(target) = &target {
            semantic_history_basis(target)?
        } else {
            compared.comparison.local.history_basis.clone()
        };
        let branch_basis = (status == BundleMergeStatus::Branched)
            .then(|| compared.comparison.incoming.history_basis.clone());
        let resolution_kind = match (&resolution, status) {
            (_, BundleMergeStatus::AlreadyPresent) => "already_present",
            (None, _) => "automatic",
            (Some(value), _) => value.mode.as_str(),
        };
        let resolution_source = resolution.as_ref().map(|value| value.user_turn_source_id);
        let classes = sorted_classes(&compared.comparison);
        let affected = affected_identities(&compared.comparison);
        let operation_dependencies =
            crate::portable::payload_dependencies(target.as_ref().unwrap_or(&compared.local))?;
        let now = self.clock.now()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_write)?;
        if load_replayed_merge(&transaction, operation_id, &request_basis)?.is_some() {
            return Err(Error::new(
                ErrorKind::RepairRequired,
                "merge replay changed during transaction",
            ));
        }
        if let Some(target) = &target {
            replace_project_state(&transaction, project_id, target)?;
        }
        transaction.execute(
            "INSERT INTO merge_events(
                 operation_id, project_id, conflict_set_id, conflict_revision, common_base_basis,
                 local_history_basis, incoming_history_basis, result_history_basis, resolution_kind,
                 resolution_source_id, conflict_classes, affected_identities, branch_history_basis, committed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                operation_id.as_bytes().as_slice(), project_id.as_bytes().as_slice(),
                compared.comparison.conflict_set_identity, 1_i64,
                compared.comparison.common_base.as_ref().map(|value| value.history_basis.as_str()),
                compared.comparison.local.history_basis, compared.comparison.incoming.history_basis,
                result_basis, resolution_kind,
                resolution_source.map(|value| value.as_bytes().to_vec()),
                encode_strings(&classes), encode_strings(&affected), branch_basis,
                now.as_unix_micros(),
            ],
        ).map_err(storage_write)?;
        crate::store::record_operation(
            &transaction,
            operation_id,
            project_id,
            "merge_bundle",
            &request_basis,
            "bundle_merge",
            project_id.as_bytes(),
            1,
            now,
            &operation_dependencies,
        )?;
        transaction
            .execute(
                "DELETE FROM bundle_lineage WHERE project_id = ?1",
                [project_id.as_bytes().as_slice()],
            )
            .map_err(storage_write)?;
        transaction.commit().map_err(storage_commit)?;
        Ok(OperationResult {
            value: BundleMerge {
                project_id,
                conflict_set_identity: compared.comparison.conflict_set_identity,
                conflict_revision: 1,
                common_base_basis: compared
                    .comparison
                    .common_base
                    .map(|value| value.history_basis),
                local_history_basis: compared.comparison.local.history_basis,
                incoming_history_basis: compared.comparison.incoming.history_basis,
                result_history_basis: result_basis,
                status,
                resolution_source_id: resolution_source,
                affected_identities: affected,
                branch_history_basis: branch_basis,
            },
            replayed: false,
        })
    }
}

fn compare_state(
    store: &Store,
    common_base_path: Option<&Path>,
    incoming_path: &Path,
    incoming_binding: Option<SourceBindingCandidate>,
) -> Result<ComparedState, Error> {
    let incoming = read_bundle(incoming_path)?;
    let (local_bytes, _, _) = bundle_bytes(store, incoming.project_id)?;
    let local = validate_bundle(&local_bytes)?;
    ensure_project(incoming.project_id, local.project_id)?;
    let base = common_base_path.map(read_bundle).transpose()?;
    if let Some(base) = &base {
        ensure_project(base.project_id, local.project_id)?;
    }
    let trustworthy_base = base.as_ref().is_some_and(|base| {
        base.payload.lineage.history_basis == incoming.payload.lineage.common_base_basis
            && (base.payload.lineage.history_basis == local.payload.lineage.common_base_basis
                || base.payload.lineage.history_basis == local.payload.lineage.history_basis)
    });
    let base_payload = trustworthy_base
        .then(|| base.as_ref().map(|value| value.payload.clone()))
        .flatten();
    let mut conflicts = Vec::new();
    let mut ambiguous_keys = BTreeSet::new();
    let sources = conflict_sources(base_payload.as_ref(), &local.payload, &incoming.payload)?;
    if !trustworthy_base {
        push_conflict(
            &mut conflicts,
            BundleConflictClass::CommonBaseUnavailable,
            vec![format!("project:{}", local.project_id)],
            false,
            "The histories cannot be joined as a verified three-way merge.",
            Some("Only the user can decide whether to retain a side or create a context branch."),
            &sources,
            base.as_ref()
                .map(|value| value.payload.lineage.history_basis.as_str()),
            &local.payload.lineage.history_basis,
            &incoming.payload.lineage.history_basis,
        );
    } else if let Some(base_payload) = &base_payload {
        classify_rows(
            base_payload,
            &local.payload,
            &incoming.payload,
            &sources,
            &mut conflicts,
            &mut ambiguous_keys,
        )?;
        classify_delete_modify(
            base_payload,
            &local.payload,
            &incoming.payload,
            &sources,
            &mut conflicts,
            &mut ambiguous_keys,
        )?;
        classify_competing_decisions(
            base_payload,
            &local.payload,
            &incoming.payload,
            &sources,
            &mut conflicts,
            &mut ambiguous_keys,
        )?;
    }
    if let Some(candidate) = incoming_binding {
        if candidate.repository_identity.trim().is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "incoming binding identity must not be empty",
            ));
        }
        if Path::new(&candidate.repository_identity).is_absolute() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "incoming binding identity must not contain a local absolute path",
            ));
        }
        let local_binding_exists: bool = store
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM local_bindings WHERE project_id = ?1)",
                [local.project_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .map_err(storage_read)?;
        if local_binding_exists {
            let mut affected = vec![format!("binding:{}", candidate.repository_identity)];
            affected.extend(
                candidate
                    .source_basis
                    .iter()
                    .map(|value| format!("source:{value}")),
            );
            push_conflict(&mut conflicts, BundleConflictClass::SourceBindingConflict, affected, false,
                "The incoming repository identity cannot replace the current local binding automatically.",
                Some("Path, remote, and name similarity do not prove Source or Project identity."), &sources,
                base_payload.as_ref().map(|value| value.lineage.history_basis.as_str()),
                &local.payload.lineage.history_basis, &incoming.payload.lineage.history_basis);
        }
    }
    conflicts.sort_by(|left, right| {
        (left.class, &left.affected_identities).cmp(&(right.class, &right.affected_identities))
    });
    conflicts.dedup_by(|left, right| {
        left.class == right.class && left.affected_identities == right.affected_identities
    });
    for conflict in &mut conflicts {
        conflict.conflict_identity = conflict_identity(conflict);
    }
    let conflict_set_identity = conflict_set_identity(
        local.project_id,
        base_payload.as_ref(),
        &local.payload,
        &incoming.payload,
        &conflicts,
    )?;
    let comparison = BundleComparison {
        project_id: local.project_id,
        conflict_set_identity,
        conflict_revision: 1,
        common_base: base.as_ref().filter(|_| trustworthy_base).map(bundle_basis),
        local: bundle_basis(&local),
        incoming: bundle_basis(&incoming),
        already_present: local.payload.lineage.history_basis
            == incoming.payload.lineage.history_basis,
        conflicts,
    };
    Ok(ComparedState {
        comparison,
        base: base_payload,
        local: local.payload,
        incoming: incoming.payload,
    })
}

fn classify_rows(
    base: &Payload,
    local: &Payload,
    incoming: &Payload,
    sources: &[ConflictSourceBasis],
    conflicts: &mut Vec<BundleConflict>,
    ambiguous: &mut BTreeSet<String>,
) -> Result<(), Error> {
    let base_rows = rows(base);
    let local_rows = rows(local);
    let incoming_rows = rows(incoming);
    let keys = base_rows
        .keys()
        .chain(local_rows.keys())
        .chain(incoming_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in keys {
        let b = base_rows.get(&key);
        let l = local_rows.get(&key);
        let i = incoming_rows.get(&key);
        if l == i {
            continue;
        }
        let (class, automatic, consequence, reason) = if l == b || i == b {
            if b.is_none() {
                (
                    BundleConflictClass::IndependentAdditions,
                    true,
                    "A non-colliding addition can be retained with the other history.",
                    None,
                )
            } else {
                (
                    BundleConflictClass::SameRecordRevision,
                    true,
                    "A change on one side can be fast-forwarded from the verified base.",
                    None,
                )
            }
        } else {
            ambiguous.insert(key.clone());
            let table = key.split('|').next().unwrap_or_default();
            match table {
                "decisions" | "decision_revisions" => (BundleConflictClass::SemanticDecisionConflict, false,
                    "Choosing a side changes user judgment, rationale, applicability, or Decision history.",
                    Some("Decision meaning is user-owned and cannot be selected by ordering or timestamp.")),
                "questions" | "question_revisions" | "question_response_sources" => (BundleConflictClass::SameRecordRevision, false,
                    "The same Question history changed differently on both sides.",
                    Some("Question meaning, dependencies, or terminal state require user judgment.")),
                "tombstones" => (BundleConflictClass::DeleteModifyConflict, false,
                    "A forgotten identity and retained history cannot both become current automatically.",
                    Some("Privacy deletion and retention consequences require an explicit choice.")),
                _ => (BundleConflictClass::SameRecordRevision, false,
                    "The same canonical identity or protected relation changed differently on both sides.",
                    Some("Semantic equivalence is not proven by text, time, import order, or model recommendation.")),
            }
        };
        push_conflict(
            conflicts,
            class,
            vec![key.clone()],
            automatic,
            consequence,
            reason,
            sources,
            Some(&base.lineage.history_basis),
            &local.lineage.history_basis,
            &incoming.lineage.history_basis,
        );
    }
    Ok(())
}

fn classify_delete_modify(
    base: &Payload,
    local: &Payload,
    incoming: &Payload,
    sources: &[ConflictSourceBasis],
    conflicts: &mut Vec<BundleConflict>,
    ambiguous: &mut BTreeSet<String>,
) -> Result<(), Error> {
    let base_state = record_states(base)?;
    let local_state = record_states(local)?;
    let incoming_state = record_states(incoming)?;
    let base_closures = record_closures(base)?;
    let local_closures = record_closures(local)?;
    let incoming_closures = record_closures(incoming)?;
    let ids = base_state
        .keys()
        .chain(local_state.keys())
        .chain(incoming_state.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for id in ids {
        let b = base_state.get(&id);
        let l = local_state.get(&id);
        let i = incoming_state.get(&id);
        let record = id
            .split_once(':')
            .map(|(kind, identity)| CanonicalRecordIdentity {
                kind: kind.to_owned(),
                identity: identity.to_owned(),
            });
        let conflict = record.is_some_and(|record| match (b, l, i) {
            (
                Some(RecordState::Active),
                Some(RecordState::Tombstone),
                Some(RecordState::Active),
            ) => incoming_closures.get(&record) != base_closures.get(&record),
            (
                Some(RecordState::Active),
                Some(RecordState::Active),
                Some(RecordState::Tombstone),
            ) => local_closures.get(&record) != base_closures.get(&record),
            _ => false,
        });
        if conflict {
            ambiguous.insert(format!("record|{id}"));
            push_conflict(conflicts, BundleConflictClass::DeleteModifyConflict, vec![id], false,
                "One history forgot the record while the other retained or modified it.",
                Some("The merge must not restore forgotten content or discard retained meaning silently."), sources,
                Some(&base.lineage.history_basis), &local.lineage.history_basis, &incoming.lineage.history_basis);
        }
    }
    Ok(())
}

fn classify_competing_decisions(
    base: &Payload,
    local: &Payload,
    incoming: &Payload,
    sources: &[ConflictSourceBasis],
    conflicts: &mut Vec<BundleConflict>,
    ambiguous: &mut BTreeSet<String>,
) -> Result<(), Error> {
    let base_decisions = decisions_by_question(base)?;
    let local_decisions = decisions_by_question(local)?;
    let incoming_decisions = decisions_by_question(incoming)?;
    for question in local_decisions
        .keys()
        .chain(incoming_decisions.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
    {
        let base_ids = base_decisions.get(&question).cloned().unwrap_or_default();
        let local_new = local_decisions
            .get(&question)
            .cloned()
            .unwrap_or_default()
            .difference(&base_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let incoming_new = incoming_decisions
            .get(&question)
            .cloned()
            .unwrap_or_default()
            .difference(&base_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        if !local_new.is_empty() && !incoming_new.is_empty() && local_new != incoming_new {
            let affected = local_new
                .union(&incoming_new)
                .map(|value| format!("decision:{value}"))
                .chain(std::iter::once(format!("question:{question}")))
                .collect::<Vec<_>>();
            ambiguous.extend(affected.iter().cloned());
            push_conflict(conflicts, BundleConflictClass::SemanticDecisionConflict, affected, false,
                "Competing Decisions or supersessions apply to the same Question.",
                Some("Choice, delegation, rationale, applicability, and supersession are user-owned."), sources,
                Some(&base.lineage.history_basis), &local.lineage.history_basis, &incoming.lineage.history_basis);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RecordState {
    Active,
    Tombstone,
}

fn record_states(payload: &Payload) -> Result<BTreeMap<String, RecordState>, Error> {
    let mut values = BTreeMap::new();
    for table in &payload.tables {
        let kind = match table.name.as_str() {
            "sources" => Some("source"),
            "questions" => Some("question"),
            "decisions" => Some("decision"),
            "context_items" => Some("context_item"),
            "checkpoints" => Some("checkpoint"),
            _ => None,
        };
        if let Some(kind) = kind {
            for row in &table.rows {
                values.insert(
                    format!("{kind}:{}", value_key(&row[0])),
                    RecordState::Active,
                );
            }
        }
        if table.name == "tombstones" {
            for row in &table.rows {
                values.insert(
                    format!("{}:{}", value_text(&row[1])?, value_key(&row[2])),
                    RecordState::Tombstone,
                );
            }
        }
    }
    Ok(values)
}

fn decisions_by_question(payload: &Payload) -> Result<BTreeMap<String, BTreeSet<String>>, Error> {
    let mut values = BTreeMap::<String, BTreeSet<String>>::new();
    let table = payload
        .tables
        .iter()
        .find(|table| table.name == "decisions")
        .ok_or_else(|| Error::new(ErrorKind::CorruptState, "Decision table is missing"))?;
    for row in &table.rows {
        values
            .entry(value_key(&row[3]))
            .or_default()
            .insert(value_key(&row[0]));
    }
    Ok(values)
}

fn conflict_sources(
    base: Option<&Payload>,
    local: &Payload,
    incoming: &Payload,
) -> Result<Vec<ConflictSourceBasis>, Error> {
    fn add(
        target: &mut BTreeMap<String, [Option<Availability>; 3]>,
        payload: Option<&Payload>,
        position: usize,
    ) -> Result<(), Error> {
        let Some(payload) = payload else {
            return Ok(());
        };
        let table = payload
            .tables
            .iter()
            .find(|table| table.name == "sources")
            .ok_or_else(|| Error::new(ErrorKind::CorruptState, "Source table is missing"))?;
        for row in &table.rows {
            let availability = Availability::parse(value_text(&row[14])?).ok_or_else(|| {
                Error::new(ErrorKind::CorruptState, "Source availability is invalid")
            })?;
            target.entry(value_key(&row[0])).or_insert([None; 3])[position] = Some(availability);
        }
        Ok(())
    }
    let mut values = BTreeMap::new();
    add(&mut values, base, 0)?;
    add(&mut values, Some(local), 1)?;
    add(&mut values, Some(incoming), 2)?;
    Ok(values
        .into_iter()
        .map(|(source_identity, states)| ConflictSourceBasis {
            source_identity,
            base: states[0],
            local: states[1],
            incoming: states[2],
        })
        .collect())
}

fn rows(payload: &Payload) -> RowMap {
    let mut values = BTreeMap::new();
    for (spec, table) in TABLES.iter().zip(&payload.tables) {
        for row in &table.rows {
            let key = spec
                .primary_key
                .iter()
                .map(|index| value_key(&row[*index]))
                .collect::<Vec<_>>()
                .join("|");
            values.insert(format!("{}|{key}", spec.name), row.clone());
        }
    }
    values
}

fn canonical_record_identity(kind: &str, identity: &PortableValue) -> CanonicalRecordIdentity {
    CanonicalRecordIdentity {
        kind: kind.to_owned(),
        identity: value_key(identity),
    }
}

fn row_owner(table: &str, row: &[PortableValue]) -> Result<Option<CanonicalRecordIdentity>, Error> {
    let owner = match table {
        "projects" => Some(canonical_record_identity("project", &row[0])),
        "project_revisions" => Some(canonical_record_identity("project", &row[0])),
        "sources" => Some(canonical_record_identity("source", &row[0])),
        "source_relations" => Some(canonical_record_identity("source", &row[1])),
        "questions" => Some(canonical_record_identity("question", &row[0])),
        "question_revisions" => Some(canonical_record_identity("question", &row[0])),
        "question_response_sources" => Some(canonical_record_identity("question", &row[1])),
        "decisions" => Some(canonical_record_identity("decision", &row[0])),
        "decision_revisions" => Some(canonical_record_identity("decision", &row[0])),
        "context_items" => Some(canonical_record_identity("context_item", &row[0])),
        "context_item_sources" => Some(canonical_record_identity("context_item", &row[1])),
        "context_item_revisions" => Some(canonical_record_identity("context_item", &row[0])),
        "checkpoints" => Some(canonical_record_identity("checkpoint", &row[0])),
        "checkpoint_source_relations"
        | "checkpoint_decisions"
        | "checkpoint_questions"
        | "checkpoint_verifications" => Some(canonical_record_identity("checkpoint", &row[1])),
        "canonical_relations" => Some(canonical_record_identity(value_text(&row[1])?, &row[2])),
        "review_due" => Some(canonical_record_identity("decision", &row[1])),
        "tombstones" => Some(canonical_record_identity(value_text(&row[1])?, &row[2])),
        "merge_events" => None,
        _ => {
            return Err(Error::new(
                ErrorKind::CorruptState,
                format!("portable table {table} has no canonical closure ownership"),
            ))
        }
    };
    Ok(owner)
}

fn record_closures(payload: &Payload) -> Result<RecordClosureMap, Error> {
    let mut closures = RecordClosureMap::new();
    for (spec, table) in TABLES.iter().zip(&payload.tables) {
        for row in &table.rows {
            let Some(owner) = row_owner(spec.name, row)? else {
                continue;
            };
            let key = spec
                .primary_key
                .iter()
                .map(|index| value_key(&row[*index]))
                .collect::<Vec<_>>()
                .join("|");
            closures
                .entry(owner)
                .or_default()
                .insert(format!("{}|{key}", spec.name), row.clone());
        }
    }
    Ok(closures)
}

fn choose_record_closure<'a>(
    base: Option<&'a RowMap>,
    local: Option<&'a RowMap>,
    incoming: Option<&'a RowMap>,
    choice: SideChoice,
) -> Option<&'a RowMap> {
    if local == incoming {
        local
    } else if local == base {
        incoming
    } else if incoming == base {
        local
    } else {
        match choice {
            SideChoice::Local => local,
            SideChoice::Incoming => incoming,
        }
    }
}

fn selected_record_states(
    selected: &RowMap,
) -> Result<
    (
        BTreeSet<CanonicalRecordIdentity>,
        BTreeSet<CanonicalRecordIdentity>,
    ),
    Error,
> {
    let mut active = BTreeSet::new();
    let mut tombstones = BTreeSet::new();
    for (key, row) in selected {
        let table = key.split('|').next().unwrap_or_default();
        let kind = match table {
            "projects" => Some("project"),
            "sources" => Some("source"),
            "questions" => Some("question"),
            "decisions" => Some("decision"),
            "context_items" => Some("context_item"),
            "checkpoints" => Some("checkpoint"),
            _ => None,
        };
        if let Some(kind) = kind {
            active.insert(canonical_record_identity(kind, &row[0]));
        } else if table == "tombstones" {
            tombstones.insert(canonical_record_identity(value_text(&row[1])?, &row[2]));
        }
    }
    if active.iter().any(|record| tombstones.contains(record)) {
        return Err(Error::new(
            ErrorKind::CorruptState,
            "merged canonical state contains both active content and a tombstone",
        ));
    }
    Ok((active, tombstones))
}

fn record_present(
    active: &BTreeSet<CanonicalRecordIdentity>,
    tombstones: &BTreeSet<CanonicalRecordIdentity>,
    kind: &str,
    identity: &PortableValue,
) -> bool {
    let record = canonical_record_identity(kind, identity);
    active.contains(&record) || tombstones.contains(&record)
}

fn retain_relation_integrity(selected: &mut RowMap) -> Result<(), Error> {
    let (active, tombstones) = selected_record_states(selected)?;
    selected.retain(|key, row| {
        let table = key.split('|').next().unwrap_or_default();
        match table {
            "source_relations" => {
                active.contains(&canonical_record_identity("source", &row[1]))
                    && active.contains(&canonical_record_identity("source", &row[3]))
            }
            "question_response_sources" => {
                active.contains(&canonical_record_identity("question", &row[1]))
                    && active.contains(&canonical_record_identity("source", &row[3]))
            }
            "context_item_sources" => {
                active.contains(&canonical_record_identity("context_item", &row[1]))
                    && active.contains(&canonical_record_identity("source", &row[2]))
            }
            "checkpoint_source_relations" => {
                active.contains(&canonical_record_identity("checkpoint", &row[1]))
                    && active.contains(&canonical_record_identity("source", &row[3]))
            }
            "checkpoint_decisions" => {
                active.contains(&canonical_record_identity("checkpoint", &row[1]))
                    && active.contains(&canonical_record_identity("decision", &row[2]))
            }
            "checkpoint_questions" => {
                active.contains(&canonical_record_identity("checkpoint", &row[1]))
                    && active.contains(&canonical_record_identity("question", &row[2]))
            }
            "checkpoint_verifications" => {
                active.contains(&canonical_record_identity("checkpoint", &row[1]))
                    && (matches!(row[4], PortableValue::Null)
                        || active.contains(&canonical_record_identity("source", &row[4])))
            }
            "canonical_relations" => {
                let from = value_text(&row[1])
                    .ok()
                    .is_some_and(|kind| record_present(&active, &tombstones, kind, &row[2]));
                let to = value_text(&row[4])
                    .ok()
                    .is_some_and(|kind| record_present(&active, &tombstones, kind, &row[5]));
                from && to
            }
            _ => true,
        }
    });
    selected_record_states(selected).map(|_| ())
}

#[derive(Clone, Copy)]
enum SideChoice {
    Local,
    Incoming,
}

fn build_target(compared: &ComparedState, choice: SideChoice) -> Result<Payload, Error> {
    let Some(base) = &compared.base else {
        return Ok(match choice {
            SideChoice::Local => compared.local.clone(),
            SideChoice::Incoming => compared.incoming.clone(),
        });
    };
    let base_rows = rows(base);
    let local_rows = rows(&compared.local);
    let incoming_rows = rows(&compared.incoming);
    let base_closures = record_closures(base)?;
    let local_closures = record_closures(&compared.local)?;
    let incoming_closures = record_closures(&compared.incoming)?;
    let record_identities = base_closures
        .keys()
        .chain(local_closures.keys())
        .chain(incoming_closures.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let closure_row_keys = base_closures
        .values()
        .chain(local_closures.values())
        .chain(incoming_closures.values())
        .flat_map(|closure| closure.keys().cloned())
        .collect::<BTreeSet<_>>();
    let keys = base_rows
        .keys()
        .chain(local_rows.keys())
        .chain(incoming_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut selected = BTreeMap::new();
    for key in keys {
        if closure_row_keys.contains(&key) {
            continue;
        }
        let b = base_rows.get(&key);
        let l = local_rows.get(&key);
        let i = incoming_rows.get(&key);
        let value = if l == i {
            l
        } else if l == b {
            i
        } else if i == b {
            l
        } else {
            match choice {
                SideChoice::Local => l,
                SideChoice::Incoming => i,
            }
        };
        if let Some(value) = value {
            selected.insert(key, value.clone());
        }
    }
    for record in record_identities {
        if let Some(closure) = choose_record_closure(
            base_closures.get(&record),
            local_closures.get(&record),
            incoming_closures.get(&record),
            choice,
        ) {
            selected.extend(closure.clone());
        }
    }
    let losing_decisions = losing_decision_ids(base, &compared.local, &compared.incoming, choice)?;
    selected.retain(|key, row| !row_mentions_losing_decision(key, row, &losing_decisions));
    retain_relation_integrity(&mut selected)?;
    let mut tables = Vec::with_capacity(TABLES.len());
    for spec in TABLES {
        let prefix = format!("{}|", spec.name);
        let rows = selected
            .range(prefix.clone()..)
            .take_while(|(key, _)| key.starts_with(&prefix))
            .map(|(_, row)| row.clone())
            .collect();
        tables.push(PortableTable {
            name: spec.name.to_owned(),
            columns: spec
                .columns
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            rows,
        });
    }
    let history_basis = semantic_history_basis_parts(&compared.local.project_id, &tables)?;
    let payload = Payload {
        project_id: compared.local.project_id.clone(),
        lineage: Lineage {
            common_base_basis: history_basis.clone(),
            history_basis,
        },
        tables,
    };
    validate_tables(&payload, compared.comparison.project_id)?;
    Ok(payload)
}

fn losing_decision_ids(
    base: &Payload,
    local: &Payload,
    incoming: &Payload,
    choice: SideChoice,
) -> Result<BTreeSet<String>, Error> {
    let base_values = decisions_by_question(base)?;
    let local_values = decisions_by_question(local)?;
    let incoming_values = decisions_by_question(incoming)?;
    let mut losing = BTreeSet::new();
    for question in local_values
        .keys()
        .chain(incoming_values.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
    {
        let base_ids = base_values.get(&question).cloned().unwrap_or_default();
        let local_new = local_values
            .get(&question)
            .cloned()
            .unwrap_or_default()
            .difference(&base_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let incoming_new = incoming_values
            .get(&question)
            .cloned()
            .unwrap_or_default()
            .difference(&base_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        if !local_new.is_empty() && !incoming_new.is_empty() && local_new != incoming_new {
            losing.extend(match choice {
                SideChoice::Local => incoming_new,
                SideChoice::Incoming => local_new,
            });
        }
    }
    Ok(losing)
}

fn row_mentions_losing_decision(
    key: &str,
    row: &[PortableValue],
    losing: &BTreeSet<String>,
) -> bool {
    let table = key.split('|').next().unwrap_or_default();
    match table {
        "decisions" | "decision_revisions" => losing.contains(&value_key(&row[0])),
        "checkpoint_decisions" => losing.contains(&value_key(&row[2])),
        "review_due" => losing.contains(&value_key(&row[1])),
        "canonical_relations" => {
            matches!(value_text(&row[1]), Ok("decision")) && losing.contains(&value_key(&row[2]))
                || matches!(value_text(&row[4]), Ok("decision"))
                    && losing.contains(&value_key(&row[5]))
        }
        "tombstones" => {
            matches!(value_text(&row[1]), Ok("decision")) && losing.contains(&value_key(&row[2]))
        }
        _ => false,
    }
}

fn replace_project_state(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
    target: &Payload,
) -> Result<(), Error> {
    validate_tables(target, project_id)?;
    for spec in TABLES.iter().rev() {
        if spec.name == "projects" {
            continue;
        }
        let sql = format!(
            "DELETE FROM {} WHERE {} = ?1",
            spec.name, spec.columns[spec.project_column]
        );
        transaction
            .execute(&sql, [project_id.as_bytes().as_slice()])
            .map_err(storage_write)?;
    }
    let project = target
        .tables
        .first()
        .and_then(|table| table.rows.first())
        .ok_or_else(|| Error::new(ErrorKind::CorruptState, "target Project row is missing"))?;
    transaction.execute(
        "UPDATE projects SET display_name = ?2, revision = ?3, created_at = ?4, updated_at = ?5 WHERE id = ?1",
        params![value_bytes(&project[0])?, portable_sql(&project[1])?, portable_sql(&project[2])?, portable_sql(&project[3])?, portable_sql(&project[4])?],
    ).map_err(storage_write)?;
    for (spec, table) in TABLES.iter().zip(&target.tables) {
        if spec.name == "projects" {
            continue;
        }
        for row in &table.rows {
            insert_row(transaction, spec, row)?;
        }
    }
    Ok(())
}

fn portable_sql(value: &PortableValue) -> Result<rusqlite::types::Value, Error> {
    crate::portable::value_to_sql(value)
}

fn validate_resolution(
    connection: &rusqlite::Connection,
    comparison: &BundleComparison,
    resolution: &MergeResolution,
) -> Result<(), Error> {
    if resolution.conflict_set_identity != comparison.conflict_set_identity
        || resolution.conflict_revision != comparison.conflict_revision
    {
        return Err(Error::new(
            ErrorKind::StaleBasis,
            "merge resolution does not match the exact current conflict set and revision",
        ));
    }
    let source: Option<(Vec<u8>, String, String)> = connection
        .query_row(
            "SELECT project_id, source_kind, actor_kind FROM sources WHERE id = ?1",
            [resolution.user_turn_source_id.as_bytes().as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(storage_read)?;
    let Some((project, kind, actor)) = source else {
        return Err(Error::new(
            ErrorKind::NotFound,
            "merge resolution Source was not found",
        ));
    };
    if project != comparison.project_id.as_bytes()
        || kind != "current_host_user_turn"
        || actor != "user"
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "merge resolution requires an exact current-host user-turn Source for this Project",
        ));
    }
    Ok(())
}

fn read_bundle(path: &Path) -> Result<crate::portable::ValidatedBundle, Error> {
    let bytes = fs::read(path).map_err(|error| {
        Error::with_source(
            ErrorKind::StorageUnavailable,
            format!("cannot read bundle {}", path.display()),
            error,
        )
    })?;
    validate_bundle(&bytes)
}

fn ensure_project(actual: ProjectId, expected: ProjectId) -> Result<(), Error> {
    if actual != expected {
        return Err(Error::new(
            ErrorKind::WrongProject,
            "bundle Project identity is incompatible with the local Project",
        ));
    }
    Ok(())
}

fn bundle_basis(bundle: &crate::portable::ValidatedBundle) -> BundleBasis {
    BundleBasis {
        checksum: bundle.checksum.clone(),
        history_basis: bundle.payload.lineage.history_basis.clone(),
        common_base_basis: bundle.payload.lineage.common_base_basis.clone(),
    }
}

fn semantic_history_basis(payload: &Payload) -> Result<String, Error> {
    semantic_history_basis_parts(&payload.project_id, &payload.tables)
}
fn semantic_history_basis_parts(
    project_id: &str,
    tables: &[PortableTable],
) -> Result<String, Error> {
    let bytes = serde_json::to_vec(&SemanticState {
        project_id: project_id.to_owned(),
        tables: tables.to_vec(),
    })
    .map_err(|error| {
        Error::with_source(
            ErrorKind::CorruptState,
            "cannot serialize merged canonical state",
            error,
        )
    })?;
    Ok(sha256_hex(&bytes))
}

#[allow(clippy::too_many_arguments)]
fn push_conflict(
    conflicts: &mut Vec<BundleConflict>,
    class: BundleConflictClass,
    affected_identities: Vec<String>,
    automatic: bool,
    consequence: &str,
    reason: Option<&str>,
    sources: &[ConflictSourceBasis],
    base: Option<&str>,
    local: &str,
    incoming: &str,
) {
    conflicts.push(BundleConflict {
        conflict_identity: String::new(),
        class,
        affected_identities,
        base_basis: base.map(str::to_owned),
        local_basis: local.to_owned(),
        incoming_basis: incoming.to_owned(),
        sources: sources.to_vec(),
        consequence: consequence.to_owned(),
        uncertainty: if automatic {
            Vec::new()
        } else {
            vec![
                "Semantic equivalence is not proven by canonical identity and relations."
                    .to_owned(),
            ]
        },
        automatic_resolution_allowed: automatic,
        user_judgment_reason: reason.map(str::to_owned),
    });
}

fn conflict_identity(conflict: &BundleConflict) -> String {
    sha256_hex(
        format!(
            "{}|{}|{}|{}|{}",
            conflict.class.as_str(),
            conflict.affected_identities.join(","),
            conflict.base_basis.as_deref().unwrap_or("unavailable"),
            conflict.local_basis,
            conflict.incoming_basis
        )
        .as_bytes(),
    )
}

fn conflict_set_identity(
    project: ProjectId,
    base: Option<&Payload>,
    local: &Payload,
    incoming: &Payload,
    conflicts: &[BundleConflict],
) -> Result<String, Error> {
    let mut basis = format!(
        "{project}|{}|{}|{}",
        base.map(|value| value.lineage.history_basis.as_str())
            .unwrap_or("unavailable"),
        local.lineage.history_basis,
        incoming.lineage.history_basis
    );
    for conflict in conflicts {
        basis.push('|');
        basis.push_str(&conflict.conflict_identity);
    }
    Ok(sha256_hex(basis.as_bytes()))
}

fn merge_request_basis(
    incoming: &crate::portable::ValidatedBundle,
    base: Option<&crate::portable::ValidatedBundle>,
    explicit: Option<&crate::portable::ValidatedBundle>,
    binding: &Option<SourceBindingCandidate>,
    resolution: Option<&MergeResolution>,
) -> Result<Vec<u8>, Error> {
    let mut value = format!(
        "merge_bundle|{}|{}|{}",
        incoming.checksum,
        base.map(|value| value.checksum.as_str()).unwrap_or("none"),
        explicit
            .map(|value| value.checksum.as_str())
            .unwrap_or("none")
    );
    if let Some(binding) = binding {
        value.push_str("|binding:");
        value.push_str(&binding.repository_identity);
        for source in &binding.source_basis {
            value.push('|');
            value.push_str(&source.to_string());
        }
    }
    if let Some(resolution) = resolution {
        value.push_str(&format!(
            "|{}|{}|{}|{}",
            resolution.conflict_set_identity,
            resolution.conflict_revision,
            resolution.user_turn_source_id,
            resolution.mode.as_str()
        ));
    }
    Ok(value.into_bytes())
}

fn load_replayed_merge(
    connection: &rusqlite::Connection,
    operation_id: OperationId,
    expected_basis: &[u8],
) -> Result<Option<BundleMerge>, Error> {
    let operation: Option<(String, Vec<u8>, String)> = connection
        .query_row(
            "SELECT operation_kind, input_basis, replay_state FROM operations WHERE operation_id = ?1",
            [operation_id.as_bytes().as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(storage_read)?;
    let Some((kind, basis, replay_state)) = operation else {
        return Ok(None);
    };
    if replay_state == "forgotten_dependency" {
        return Err(Error::new(
            ErrorKind::NotFound,
            "merge replay input depended on forgotten canonical content",
        ));
    }
    if kind != "merge_bundle" || basis != expected_basis {
        return Err(Error::new(
            ErrorKind::DomainConflict,
            "OperationId was already committed with different merge input",
        ));
    }
    let row = connection.query_row(
        "SELECT project_id, conflict_set_id, conflict_revision, common_base_basis, local_history_basis,
                incoming_history_basis, result_history_basis, resolution_kind, resolution_source_id,
                affected_identities, branch_history_basis FROM merge_events WHERE operation_id = ?1",
        [operation_id.as_bytes().as_slice()], |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?, row.get::<_, String>(5)?, row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, Option<Vec<u8>>>(8)?, row.get::<_, Vec<u8>>(9)?, row.get::<_, Option<String>>(10)?)),
    ).optional().map_err(storage_read)?.ok_or_else(|| Error::new(ErrorKind::RepairRequired, "merge operation is missing its durable result"))?;
    let status = match row.7.as_str() {
        "already_present" => BundleMergeStatus::AlreadyPresent,
        "automatic" => BundleMergeStatus::MergedAutomatically,
        "context_branch" => BundleMergeStatus::Branched,
        _ => BundleMergeStatus::Resolved,
    };
    Ok(Some(BundleMerge {
        project_id: ProjectId::from_slice(&row.0)?,
        conflict_set_identity: row.1,
        conflict_revision: u64::try_from(row.2)
            .map_err(|_| Error::new(ErrorKind::CorruptState, "merge revision is invalid"))?,
        common_base_basis: row.3,
        local_history_basis: row.4,
        incoming_history_basis: row.5,
        result_history_basis: row.6,
        status,
        resolution_source_id: row
            .8
            .map(|value| SourceId::from_slice(&value))
            .transpose()?,
        affected_identities: decode_strings(&row.9)?,
        branch_history_basis: row.10,
    }))
}

fn merge_view(
    comparison: &BundleComparison,
    status: BundleMergeStatus,
    source: Option<SourceId>,
    branch: Option<String>,
) -> BundleMerge {
    BundleMerge {
        project_id: comparison.project_id,
        conflict_set_identity: comparison.conflict_set_identity.clone(),
        conflict_revision: comparison.conflict_revision,
        common_base_basis: comparison
            .common_base
            .as_ref()
            .map(|value| value.history_basis.clone()),
        local_history_basis: comparison.local.history_basis.clone(),
        incoming_history_basis: comparison.incoming.history_basis.clone(),
        result_history_basis: comparison.local.history_basis.clone(),
        status,
        resolution_source_id: source,
        affected_identities: affected_identities(comparison),
        branch_history_basis: branch,
    }
}

fn sorted_classes(comparison: &BundleComparison) -> Vec<String> {
    comparison
        .conflicts
        .iter()
        .map(|value| value.class.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn affected_identities(comparison: &BundleComparison) -> Vec<String> {
    comparison
        .conflicts
        .iter()
        .flat_map(|value| value.affected_identities.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn encode_strings(values: &[String]) -> Vec<u8> {
    serde_json::to_vec(values).unwrap_or_default()
}
fn decode_strings(bytes: &[u8]) -> Result<Vec<String>, Error> {
    serde_json::from_slice(bytes).map_err(|error| {
        Error::with_source(
            ErrorKind::CorruptState,
            "merge identity list is malformed",
            error,
        )
    })
}
fn storage_read(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::CorruptState,
        "cannot read bundle merge state",
        error,
    )
}
fn storage_write(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::TransactionFailure,
        "bundle merge transaction failed",
        error,
    )
}
fn storage_commit(error: rusqlite::Error) -> Error {
    Error::with_source(
        ErrorKind::IndeterminateOutcome,
        "bundle merge commit outcome is indeterminate",
        error,
    )
}
