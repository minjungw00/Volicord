use super::*;

impl CoreService {
    /// Executes `volicord.intake` through the shared Core mutation pipeline.
    pub fn intake(
        &self,
        request: volicord_types::IntakeRequest,
        invocation: InvocationContext,
    ) -> CoreResult<PipelineResponse> {
        let request_json = serde_json::to_value(&request)?;
        let policy = mutation_method_policy(
            request.operation_category(),
            TaskRequirement::None,
            request.envelope.dry_run,
        );
        let prepared = match prepare_or_response(
            self,
            MethodName::Intake,
            request.envelope.clone(),
            request_json,
            invocation,
            policy,
        )? {
            Ok(prepared) => prepared,
            Err(response) => return Ok(response),
        };
        let store = &prepared.store;
        let project_state = &prepared.context.project_state;
        if request.resume_policy == ResumePolicy::RejectIfActive
            && project_state.active_task_id.is_some()
        {
            return validation_rejected(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "resume_policy",
                "resume_policy=reject_if_active cannot proceed while a Task is active",
            );
        }

        let plan = match plan_intake(
            self,
            store,
            project_state,
            request.clone(),
            &prepared.context.verified_invocation,
            &prepared.operation_now,
        ) {
            Ok(plan) => plan,
            Err(error) => return plan_error_response(&request.envelope, project_state, error),
        };

        if request.envelope.dry_run {
            return self.execute_prepared_request(
                prepared,
                OwnerPipelineBranch::DryRunPreview {
                    dry_run_summary: dry_run_summary(
                        "task",
                        "commit",
                        "Intake would select or create a Task.",
                        plan.next_actions,
                    ),
                },
            );
        }

        self.execute_prepared_request(
            prepared,
            OwnerPipelineBranch::CommitMutation {
                result_fields: plan.result_fields,
                event_kind: "task_intake".to_owned(),
                event_payload: plan.event_payload,
                task_id: Some(plan.task_id),
                change_unit_id: None,
                storage_mutations: plan.storage_mutations,
            },
        )
    }
}

fn plan_intake(
    service: &CoreService,
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    mut request: volicord_types::IntakeRequest,
    verified_invocation: &VerifiedInvocationContext,
    operation_now: &UtcTimestamp,
) -> Result<MethodPlan, PlanError> {
    let plan_now = *operation_now.as_datetime();
    let user_action_now = operation_now.clone();
    let planned_state_version = project_state.state_version + 1;
    let mode = resolve_requested_mode(request.requested_mode);
    let active_task = store
        .active_task_record()
        .map_err(CorePipelineError::from)?;

    let create_new = match request.resume_policy {
        ResumePolicy::ResumeActive => active_task.is_none(),
        ResumePolicy::CreateNew | ResumePolicy::RejectIfActive => true,
        ResumePolicy::SupersedeActive => true,
    };
    if !create_new && (request.acceptance_policy.is_some() || request.lineage.is_some()) {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "acceptance_policy",
            "resume_active requires null acceptance_policy and lineage fields",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let planned_lineage = if create_new {
        plan_task_lineage(
            store,
            project_state,
            verified_invocation,
            &mut request,
            planned_state_version,
        )?
    } else {
        None
    };
    let (acceptance_policy, acceptance_policy_reason) = if create_new {
        resolve_acceptance_policy(mode, request.acceptance_policy.as_ref().copied(), &request)?
    } else {
        let active = active_task
            .as_ref()
            .expect("active_task exists when resume selects an existing Task");
        (
            parse_acceptance_policy(&active.acceptance_policy)?,
            active.acceptance_policy_reason.clone(),
        )
    };
    let task_id = if create_new {
        match request.envelope.task_id.as_ref().cloned() {
            Some(task_id) => task_id,
            None => allocate_task_id(service, store)?,
        }
    } else {
        TaskId::new(
            active_task
                .as_ref()
                .expect("active_task exists when create_new is false")
                .task_id
                .clone(),
        )
    };
    if planned_lineage
        .as_ref()
        .is_some_and(|lineage| lineage.predecessor_task_id == task_id.as_str())
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "lineage.predecessor_task_id",
            "a Task cannot name itself as its predecessor",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }

    let mut initial_source_refs = if create_new {
        normalize_source_refs(
            store,
            project_state,
            &request.envelope,
            &task_id,
            "initial_source_refs",
            &request.initial_source_refs,
        )?
    } else {
        Vec::new()
    };
    if let Some(lineage) = planned_lineage.as_ref() {
        let predecessor_task_id = TaskId::new(lineage.predecessor_task_id.clone());
        let carried_source_refs = normalize_source_refs_with_carried_artifact_task(
            store,
            project_state,
            &request.envelope,
            &task_id,
            "lineage.carry_forward.source_refs",
            &lineage.carried_source_refs,
            Some(&predecessor_task_id),
        )?;
        for source_ref in carried_source_refs {
            if !initial_source_refs.contains(&source_ref) {
                initial_source_refs.push(source_ref);
            }
        }
    }

    let mut storage_mutations = Vec::new();
    if request.resume_policy == ResumePolicy::SupersedeActive {
        if let Some(active) = &active_task {
            storage_mutations.push(CoreStorageMutation::SupersedeTask {
                task_id: active.task_id.clone(),
            });
        }
    }

    let acceptance_criteria = if create_new {
        let mut criteria = Vec::with_capacity(request.initial_scope.acceptance_criteria.len());
        let mut reserved_ids = BTreeSet::new();
        for input in &request.initial_scope.acceptance_criteria {
            let statement = normalize_display_text(&input.statement);
            if statement.is_empty() {
                validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "initial_scope.acceptance_criteria[].statement",
                    "acceptance criterion statements must not be empty",
                )?;
                unreachable!("validation_plan_error always returns Err");
            }
            let acceptance_criterion_id =
                allocate_acceptance_criterion_id(service, store, &reserved_ids)
                    .map_err(PlanError::Core)?;
            reserved_ids.insert(acceptance_criterion_id.as_str().to_owned());
            criteria.push(AcceptanceCriterion {
                acceptance_criterion_id,
                statement,
                evidence_requirement: input.evidence_requirement,
            });
        }
        criteria
    } else {
        active_acceptance_criteria_for_task(store, &task_id)?
    };

    let task_record = if create_new {
        let mut shaping_summary = task_shaping_json(
            Some(request.plain_language_request.clone()),
            Some(request.initial_scope.boundary.clone()),
            request.initial_scope.non_goals.clone(),
            None,
            None,
            Some(serde_json::to_value(&request.initial_context_refs)?),
        );
        shaping_summary["initial_source_refs"] = serde_json::to_value(&initial_source_refs)?;
        if let Some(lineage) = planned_lineage.as_ref() {
            if let Some(baseline_ref) = lineage.carried_baseline_ref.as_ref() {
                shaping_summary["baseline_ref"] = serde_json::to_value(baseline_ref)?;
            }
        }
        let work_phase = initial_work_phase(mode);
        let task = TaskRecord {
            project_id: request.envelope.project_id.as_str().to_owned(),
            task_id: task_id.as_str().to_owned(),
            mode: task_mode_storage(mode).to_owned(),
            work_phase: work_phase_storage(work_phase).to_owned(),
            acceptance_policy: acceptance_policy_storage(acceptance_policy).to_owned(),
            acceptance_policy_reason: acceptance_policy_reason.clone(),
            predecessor_task_id: planned_lineage
                .as_ref()
                .map(|lineage| lineage.predecessor_task_id.clone()),
            lineage_relation: planned_lineage
                .as_ref()
                .map(|lineage| task_lineage_relation_storage(lineage.relation).to_owned()),
            lineage_reason: planned_lineage
                .as_ref()
                .map(|lineage| lineage.creation_reason.clone()),
            carry_forward_json: serde_json::to_string(
                &planned_lineage
                    .as_ref()
                    .map(|lineage| lineage.dispositions.clone())
                    .unwrap_or_default(),
            )?,
            lifecycle_phase: "shaping".to_owned(),
            result: Some("none".to_owned()),
            title: Some(request.plain_language_request.clone()),
            summary: Some(request.plain_language_request.clone()),
            shaping_summary_json: serde_json::to_string(&shaping_summary)?,
            bounded_context_json: serde_json::to_string(&json!({
                "initial_context_refs": request.initial_context_refs,
                "initial_source_refs": initial_source_refs
            }))?,
            autonomy_boundary_json: serde_json::to_string(&json!({
                "autonomy_boundary": Value::Null
            }))?,
            scope_revision: 0,
            close_basis_revision: 0,
            close_basis_json: None,
            close_summary_json: serde_json::to_string(&json!({
                "close_reason": "none"
            }))?,
            current_change_unit_id: None,
            closed_at: None,
        };
        storage_mutations.push(CoreStorageMutation::InsertTask(TaskInsert {
            task_id: task.task_id.clone(),
            created_by_actor_source: verified_invocation.actor_source.to_canonical_string(),
            mode: task.mode.clone(),
            work_phase: task.work_phase.clone(),
            acceptance_policy: task.acceptance_policy.clone(),
            acceptance_policy_reason: task.acceptance_policy_reason.clone(),
            predecessor_task_id: task.predecessor_task_id.clone(),
            lineage_relation: task.lineage_relation.clone(),
            lineage_reason: task.lineage_reason.clone(),
            carry_forward_json: task.carry_forward_json.clone(),
            lifecycle_phase: task.lifecycle_phase.clone(),
            result: task.result.clone(),
            title: task.title.clone(),
            summary: task.summary.clone(),
            shaping_summary_json: task.shaping_summary_json.clone(),
            bounded_context_json: task.bounded_context_json.clone(),
            autonomy_boundary_json: task.autonomy_boundary_json.clone(),
            close_summary_json: task.close_summary_json.clone(),
            current_change_unit_id: None,
        }));
        storage_mutations.push(CoreStorageMutation::ReplaceAcceptanceCriteria(
            AcceptanceCriteriaReplace {
                task_id: task.task_id.clone(),
                criteria: acceptance_criteria
                    .iter()
                    .enumerate()
                    .map(|(position, criterion)| AcceptanceCriterionUpsert {
                        acceptance_criterion_id: criterion
                            .acceptance_criterion_id
                            .as_str()
                            .to_owned(),
                        statement: criterion.statement.clone(),
                        evidence_requirement: storage_value(criterion.evidence_requirement)
                            .expect("EvidenceRequirement serialization should be infallible"),
                        position: position as u64,
                    })
                    .collect(),
            },
        ));
        storage_mutations.push(CoreStorageMutation::SetActiveTask {
            task_id: task.task_id.clone(),
        });
        task
    } else {
        active_task.expect("active_task exists when create_new is false")
    };

    let current_change_unit = if create_new {
        None
    } else {
        store
            .current_change_unit(&task_id)
            .map_err(CorePipelineError::from)?
    };
    let task_ref = state_ref(
        StateRecordKind::Task,
        &task_record.task_id,
        &request.envelope.project_id,
        Some(&task_id),
        Some(planned_state_version),
    );
    let change_unit_ref = current_change_unit.as_ref().map(|record| {
        state_ref(
            StateRecordKind::ChangeUnit,
            &record.change_unit_id,
            &request.envelope.project_id,
            Some(&task_id),
            Some(record.basis_state_version.unwrap_or(planned_state_version)),
        )
    });
    let pending_refs = if create_new {
        Vec::new()
    } else {
        projected_pending_user_action_refs(
            store,
            &task_id,
            planned_state_version,
            &user_action_now,
        )?
    };
    let blocker_refs = if create_new {
        Vec::new()
    } else {
        projected_blocker_refs(store, &task_id, planned_state_version)?
    };
    let next_actions = next_actions_for_state(
        parse_task_mode(&task_record.mode)?,
        &task_ref,
        change_unit_ref.as_ref(),
        planned_state_version,
    );
    let evidence_summary = projected_evidence_summary_for_criteria(
        store,
        &request.envelope.project_id,
        planned_state_version,
        &task_record,
        &acceptance_criteria,
    )?;
    let projected_project_state = project_state_projection(
        project_state,
        planned_state_version,
        Some(task_record.task_id.clone()),
    );
    let close_plan = projected_close_check(
        store,
        &projected_project_state,
        verified_invocation,
        &request.envelope,
        &task_id,
        close_context_with_projected_acceptance_criteria(
            close_context_from_projection(
                task_record.clone(),
                current_change_unit.clone(),
                if create_new {
                    None
                } else {
                    projected_close_basis(store, &task_id)?
                },
                pending_refs.clone(),
                blocker_refs.clone(),
                evidence_summary.clone(),
                user_action_now.clone(),
            ),
            &acceptance_criteria,
        ),
        plan_now,
    )?;
    let guarantee_display =
        guarantee_display_for_invocation(store, verified_invocation, planned_state_version)?;
    let write_ticket_summary = if create_new {
        None
    } else {
        projected_write_ticket_summary(
            store,
            &task_id,
            planned_state_version,
            plan_now,
            Some(guarantee_display.clone()),
        )?
    };
    let state = build_state_summary(SummaryBuild {
        project_id: &request.envelope.project_id,
        state_version: planned_state_version,
        task: &task_record,
        current_change_unit: current_change_unit.as_ref(),
        acceptance_criteria,
        pending_user_action_refs: pending_refs,
        blocker_refs,
        write_ticket_summary,
        evidence_summary,
        evidence_gate: Some(close_plan.evidence_gate),
        close_state: Some(close_plan.close_state),
        close_blockers: close_plan.blockers,
        guard_health: close_plan.guard_health,
        guarantee_display: Some(guarantee_display),
    })?;
    let result = volicord_types::IntakeResult {
        base: placeholder_base(),
        task_ref: task_ref.clone(),
        change_unit_ref,
        state,
        next_actions: next_actions.clone(),
    };
    let event_payload = object_from_value(json!({
        "task_id": task_id,
        "resume_policy": request.resume_policy,
        "requested_mode": request.requested_mode,
        "resolved_mode": mode
        ,"acceptance_policy": acceptance_policy
        ,"lineage": planned_lineage.as_ref().map(|lineage| json!({
            "predecessor_task_id": lineage.predecessor_task_id,
            "relation": lineage.relation,
            "carry_forward": lineage.dispositions
        }))
    }))?;
    Ok(MethodPlan {
        task_id,
        change_unit_id: None,
        storage_mutations,
        event_payload,
        result_fields: strip_base(serde_json::to_value(result)?)?,
        next_actions,
    })
}

#[derive(Debug, Clone)]
struct PlannedTaskLineage {
    predecessor_task_id: String,
    relation: TaskLineageRelation,
    creation_reason: String,
    dispositions: Vec<CarryForwardDisposition>,
    carried_baseline_ref: Option<BaselineRef>,
    carried_source_refs: Vec<SourceRef>,
}

fn plan_task_lineage(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    verified_invocation: &VerifiedInvocationContext,
    request: &mut volicord_types::IntakeRequest,
    planned_state_version: u64,
) -> Result<Option<PlannedTaskLineage>, PlanError> {
    let Some(mut lineage) = request.lineage.as_ref().cloned() else {
        return Ok(None);
    };
    lineage.creation_reason = normalize_display_text(&lineage.creation_reason);
    if lineage.creation_reason.is_empty() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "lineage.creation_reason",
            "lineage creation_reason must not be empty",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let predecessor = store
        .task_record(&lineage.predecessor_task_id)
        .map_err(CorePipelineError::from)?
        .ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "lineage predecessor must identify an existing same-project Task",
            )))
        })?;
    if lineage.relation == TaskLineageRelation::ImplementsAdviceFrom
        && !(predecessor.mode == "advisor"
            && predecessor.lifecycle_phase == "completed"
            && predecessor.result.as_deref() == Some("advice_only"))
    {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "lineage.relation",
            "implements_advice_from requires a completed advisor advice_only predecessor",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let selected = lineage
        .carry_forward
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if selected.len() != lineage.carry_forward.len() {
        validation_plan_error(
            request.envelope.dry_run,
            Some(project_state.state_version),
            "lineage.carry_forward",
            "carry_forward values must not contain duplicates",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let predecessor_ref = state_ref(
        StateRecordKind::Task,
        &predecessor.task_id,
        &request.envelope.project_id,
        Some(&lineage.predecessor_task_id),
        Some(planned_state_version),
    );

    let predecessor_scope = StoredScope::from_task(&predecessor)?;
    if selected.contains(&CarryForwardKind::Scope) {
        let predecessor_criteria = store
            .active_acceptance_criteria(&lineage.predecessor_task_id)
            .map_err(CorePipelineError::from)?
            .into_iter()
            .map(|record| {
                Ok(AcceptanceCriterionInput {
                    statement: record.statement,
                    evidence_requirement: parse_owner_storage_value(
                        "acceptance_criteria",
                        record.acceptance_criterion_id,
                        "evidence_requirement",
                        &record.evidence_requirement,
                    )?,
                })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        if predecessor_scope.scope_summary.is_none() && predecessor_criteria.is_empty() {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected scope carry-forward has no predecessor scope material",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        if let Some(scope) = predecessor_scope.scope_summary.as_ref() {
            let submitted = normalize_display_text(&request.initial_scope.boundary);
            if submitted.is_empty() {
                request.initial_scope.boundary = scope.clone();
            } else if submitted != normalize_display_text(scope) {
                validation_plan_error(
                    request.envelope.dry_run,
                    Some(project_state.state_version),
                    "initial_scope.boundary",
                    "carried scope must match an explicitly submitted new-Task boundary",
                )?;
                unreachable!("validation_plan_error always returns Err");
            }
        }
        if request.initial_scope.acceptance_criteria.is_empty() {
            request.initial_scope.acceptance_criteria = predecessor_criteria;
        } else if request.initial_scope.acceptance_criteria != predecessor_criteria {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "initial_scope.acceptance_criteria",
                "carried criteria must match explicitly submitted criterion statements and requirements",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
    }
    if selected.contains(&CarryForwardKind::NonGoals) {
        if predecessor_scope.non_goals.is_empty() {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected non_goals carry-forward has no predecessor non-goals",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        if request.initial_scope.non_goals.is_empty() {
            request.initial_scope.non_goals = predecessor_scope.non_goals.clone();
        } else if normalized_string_set(&request.initial_scope.non_goals)
            != normalized_string_set(&predecessor_scope.non_goals)
        {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "initial_scope.non_goals",
                "carried non-goals must match explicitly submitted non-goals",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
    }

    let shaping: PersistedTaskShaping = decode_required_json(
        "tasks",
        predecessor.task_id.clone(),
        "shaping_summary_json",
        Some(&predecessor.shaping_summary_json),
    )?;
    if selected.contains(&CarryForwardKind::ContextRefs) {
        let refs = shaping
            .initial_context_refs
            .map(serde_json::from_value::<Vec<StateRecordRef>>)
            .transpose()?
            .unwrap_or_default();
        if refs.is_empty() {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected context_refs carry-forward has no predecessor context refs",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        request.initial_context_refs.extend(refs);
        request.initial_context_refs =
            unique_state_record_refs(request.initial_context_refs.clone());
    }
    let carried_source_refs = if selected.contains(&CarryForwardKind::SourceRefs) {
        let refs = shaping
            .initial_source_refs
            .map(serde_json::from_value::<Vec<SourceRef>>)
            .transpose()?
            .unwrap_or_default();
        if refs.is_empty() {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected source_refs carry-forward has no predecessor source refs",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        refs
    } else {
        Vec::new()
    };
    let reference_only_sources = reference_only_carry_sources(
        store,
        project_state,
        request,
        &predecessor,
        &predecessor_scope,
        &selected,
        planned_state_version,
    )?;

    let carried_baseline_ref = if selected.contains(&CarryForwardKind::Baseline) {
        let baseline_ref = predecessor_scope.baseline_ref.as_ref().ok_or_else(|| {
            PlanError::Response(Box::new(decision_rejected_response(
                &request.envelope,
                Some(project_state.state_version),
                "selected baseline carry-forward has no predecessor baseline",
            )))
        })?;
        let change_unit = store
            .current_change_unit(&lineage.predecessor_task_id)
            .map_err(CorePipelineError::from)?
            .ok_or_else(|| {
                PlanError::Response(Box::new(decision_rejected_response(
                    &request.envelope,
                    Some(project_state.state_version),
                    "selected baseline carry-forward has no current predecessor Change Unit",
                )))
            })?;
        let write_basis: PersistedWriteBasis = decode_required_json(
            "change_units",
            change_unit.change_unit_id,
            "write_basis_json",
            Some(&change_unit.write_basis_json),
        )?;
        if write_basis.baseline_ref.as_ref().map(BaselineRef::as_str) != Some(baseline_ref.as_str())
        {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "baseline carry-forward requires matching predecessor Task and Change Unit baselines",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        if write_basis.git_workspace_context != verified_invocation.git_workspace_context {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "baseline carry-forward requires the exact current compatible Git workspace context",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        Some(BaselineRef::new(baseline_ref.clone()))
    } else {
        None
    };

    let dispositions = lineage
        .carry_forward
        .iter()
        .copied()
        .map(|kind| CarryForwardDisposition {
            kind,
            status: if matches!(
                kind,
                CarryForwardKind::UserDecisions
                    | CarryForwardKind::KnownLimitations
                    | CarryForwardKind::UnresolvedObligations
                    | CarryForwardKind::ResidualRisks
            ) {
                CarryForwardDispositionStatus::ReferenceOnly
            } else {
                CarryForwardDispositionStatus::Applied
            },
            source_refs: reference_only_sources
                .get(&kind)
                .cloned()
                .unwrap_or_else(|| vec![predecessor_ref.clone()]),
        })
        .collect();
    Ok(Some(PlannedTaskLineage {
        predecessor_task_id: predecessor.task_id,
        relation: lineage.relation,
        creation_reason: lineage.creation_reason,
        dispositions,
        carried_baseline_ref,
        carried_source_refs,
    }))
}

fn reference_only_carry_sources(
    store: &CoreProjectStore,
    project_state: &ProjectStateHeader,
    request: &volicord_types::IntakeRequest,
    predecessor: &TaskRecord,
    predecessor_scope: &StoredScope,
    selected: &BTreeSet<CarryForwardKind>,
    planned_state_version: u64,
) -> Result<BTreeMap<CarryForwardKind, Vec<StateRecordRef>>, PlanError> {
    let reference_only_kinds = [
        (CarryForwardKind::UserDecisions, "decision"),
        (CarryForwardKind::KnownLimitations, "known_limit"),
        (CarryForwardKind::UnresolvedObligations, "obligation"),
        (CarryForwardKind::ResidualRisks, "accepted_risk"),
    ];
    if !reference_only_kinds
        .iter()
        .any(|(kind, _)| selected.contains(kind))
    {
        return Ok(BTreeMap::new());
    }

    let continuity_records = store
        .project_continuity_records_for_task(&predecessor.task_id)
        .map_err(CorePipelineError::from)?;
    let needs_current_risks = selected.contains(&CarryForwardKind::KnownLimitations)
        || selected.contains(&CarryForwardKind::ResidualRisks);
    let current_close_basis = if needs_current_risks {
        projected_close_basis(store, &TaskId::new(predecessor.task_id.clone()))?
    } else {
        None
    };
    let current_change_unit = if current_close_basis.is_some() {
        store
            .current_change_unit(&TaskId::new(predecessor.task_id.clone()))
            .map_err(CorePipelineError::from)?
    } else {
        None
    };

    let mut result = BTreeMap::new();
    for (kind, continuity_kind) in reference_only_kinds {
        if !selected.contains(&kind) {
            continue;
        }
        let mut source_refs = continuity_records
            .iter()
            .filter(|record| record.status == "active" && record.kind == continuity_kind)
            .map(|record| project_continuity_ref(record, planned_state_version))
            .collect::<Vec<_>>();

        if matches!(
            kind,
            CarryForwardKind::KnownLimitations | CarryForwardKind::ResidualRisks
        ) {
            if let Some(close_basis) = current_close_basis.as_ref() {
                let relevant_risks = close_basis
                    .residual_risks
                    .iter()
                    .filter(|risk| {
                        kind == CarryForwardKind::ResidualRisks || !risk.acceptance_required
                    })
                    .collect::<Vec<_>>();
                if !relevant_risks.is_empty() {
                    let compatible = close_basis.task_id.as_str() == predecessor.task_id
                        && close_basis.scope_revision == predecessor.scope_revision
                        && close_basis.baseline_ref.as_ref().map(BaselineRef::as_str)
                            == predecessor_scope.baseline_ref.as_deref()
                        && current_change_unit.as_ref().is_some_and(|change_unit| {
                            change_unit.change_unit_id == close_basis.change_unit_id.as_str()
                        });
                    if !compatible {
                        validation_plan_error(
                            request.envelope.dry_run,
                            Some(project_state.state_version),
                            "lineage.carry_forward",
                            "reference-only risk carry-forward requires a current compatible predecessor close basis",
                        )?;
                        unreachable!("validation_plan_error always returns Err");
                    }
                    source_refs.push(close_basis.source_run_ref.clone());
                    source_refs.extend(
                        relevant_risks
                            .into_iter()
                            .flat_map(|risk| risk.source_refs.clone()),
                    );
                }
            }
        }
        source_refs = unique_state_record_refs(source_refs);
        if source_refs.is_empty() {
            validation_plan_error(
                request.envelope.dry_run,
                Some(project_state.state_version),
                "lineage.carry_forward",
                "selected reference-only carry-forward has no active compatible predecessor record",
            )?;
            unreachable!("validation_plan_error always returns Err");
        }
        result.insert(kind, source_refs);
    }
    Ok(result)
}

fn resolve_acceptance_policy(
    mode: TaskMode,
    requested: Option<AcceptancePolicy>,
    request: &volicord_types::IntakeRequest,
) -> Result<(AcceptancePolicy, String), PlanError> {
    let selected = requested.unwrap_or(match mode {
        TaskMode::Advisor => AcceptancePolicy::NotRequired,
        TaskMode::Direct | TaskMode::Work => AcceptancePolicy::Required,
    });
    if selected == AcceptancePolicy::NotRequired && mode != TaskMode::Advisor {
        validation_plan_error(
            request.envelope.dry_run,
            None,
            "acceptance_policy",
            "not_required acceptance is limited to advisor Tasks",
        )?;
        unreachable!("validation_plan_error always returns Err");
    }
    let reason = match (selected, mode) {
        (AcceptancePolicy::NotRequired, TaskMode::Advisor) => {
            "Pure advice does not require final result acceptance unless intake selects another policy."
        }
        (AcceptancePolicy::Required, _) => {
            "This Task requires final acceptance for its current close basis."
        }
        (AcceptancePolicy::PolicyDependent, _) => {
            "Core evaluates final acceptance from the current result and residual-risk basis."
        }
        (AcceptancePolicy::NotRequired, _) => unreachable!("validated above"),
    };
    Ok((selected, reason.to_owned()))
}
