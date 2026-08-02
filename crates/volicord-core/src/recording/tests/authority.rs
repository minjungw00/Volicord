use super::*;

#[test]
fn record_run_blocks_only_matching_pending_observation_actions_without_effect(
) -> Result<(), Box<dyn Error>> {
    for (required_for, should_block, suffix) in [
        (
            volicord_types::values::UserActionRequiredFor::RecordRun,
            true,
            "matching",
        ),
        (
            volicord_types::values::UserActionRequiredFor::Informational,
            false,
            "informational",
        ),
        (
            volicord_types::values::UserActionRequiredFor::CloseComplete,
            false,
            "nonmatching_close",
        ),
    ] {
        let harness = MethodHarness::new()?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("run_pending_observation_{suffix}"))?;
        let (after_artifact, artifact_ref) = promote_artifact_for_record_run(
            &harness,
            &task_id,
            &change_unit_id,
            2,
            &format!("pending_observation_{suffix}"),
        )?;
        let mut pending = observation_action_request(
            &format!("req_run_pending_observation_{suffix}"),
            &format!("idem_run_pending_observation_{suffix}"),
            after_artifact,
            &task_id,
            &change_unit_id,
            supplemental_evidence_target("Artifact registered for corruption coverage."),
            vec![artifact_ref.artifact_id],
        );
        pending.required_for = vec![required_for];
        let requested = harness
            .service
            .request_user_action(pending, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(requested.response_value["base"]["response_kind"], "result");
        let before = harness.counts()?;

        let response = harness.service.record_run(
            record_run_request(
                &format!("req_run_pending_observation_record_{suffix}"),
                &format!("idem_run_pending_observation_record_{suffix}"),
                false,
                Some(before.state_version),
                &task_id,
                &change_unit_id,
            ),
            invocation(OperationCategory::AgentWorkflow),
        )?;

        if should_block {
            assert_eq!(response.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                response.response_value["errors"][0]["code"],
                "USER_DECISION_UNRESOLVED"
            );
            assert_eq!(harness.counts()?, before);

            let dry_run = harness.service.record_run(
                record_run_request(
                    &format!("req_run_pending_observation_dry_{suffix}"),
                    &format!("idem_run_pending_observation_dry_{suffix}"),
                    true,
                    Some(before.state_version),
                    &task_id,
                    &change_unit_id,
                ),
                invocation(OperationCategory::AgentWorkflow),
            )?;
            assert_eq!(dry_run.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                dry_run.response_value["errors"][0]["code"],
                "USER_DECISION_UNRESOLVED"
            );
            assert_eq!(harness.counts()?, before);
        } else {
            assert_eq!(response.response_value["base"]["response_kind"], "result");
            let after = harness.counts()?;
            assert_eq!(after.state_version, before.state_version + 1);
            assert_eq!(after.runs, before.runs + 1);
        }
    }
    Ok(())
}

#[test]
fn sensitive_pending_action_blocks_record_run_only_on_validated_matching_scope(
) -> Result<(), Box<dyn Error>> {
    for (
        suffix,
        action_operation,
        action_paths,
        action_categories,
        run_operation,
        run_paths,
        run_categories,
        mismatched_baseline,
        should_block,
    ) in [
        (
            "matching",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            false,
            true,
        ),
        (
            "no_sensitive_categories",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["src/export.rs"][..],
            &[][..],
            false,
            false,
        ),
        (
            "operation_mismatch",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "other_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            false,
            false,
        ),
        (
            "path_mismatch",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["tests/export.rs"][..],
            &["network"][..],
            false,
            false,
        ),
        (
            "category_mismatch",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["credential"][..],
            false,
            false,
        ),
        (
            "baseline_mismatch",
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            "local_sensitive_step",
            &["src/export.rs"][..],
            &["network"][..],
            true,
            false,
        ),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_change_unit(&harness, &format!("run_pending_sensitive_{suffix}"))?;

        if !run_categories.is_empty() {
            let mut ticket_approval = user_action_request(
                &format!("req_run_ticket_approval_{suffix}"),
                &format!("idem_run_ticket_approval_{suffix}"),
                false,
                Some(harness.counts()?.state_version),
                &task_id,
                Some(&change_unit_id),
                JudgmentKind::SensitiveApproval,
            );
            let volicord_types::schema::UserActionDraft::Choice(choice) =
                &mut ticket_approval.action
            else {
                unreachable!("sensitive approval fixture is choice-shaped")
            };
            choice.sensitive_action_scope = Some(sensitive_scope(
                run_operation,
                run_paths.to_vec(),
                run_categories.to_vec(),
            ))
            .into();
            let requested = harness.service.request_user_action(
                ticket_approval,
                invocation(OperationCategory::AgentWorkflow),
            )?;
            let approval_id =
                response_record_id(&requested.response_value, "user_action_request_ref");
            harness.service.resolve_user_action(
                resolve_user_action_request(
                    &format!("req_run_ticket_approval_resolve_{suffix}"),
                    &format!("submission_run_ticket_approval_{suffix}"),
                    None,
                    &task_id,
                    &approval_id,
                    "accept",
                ),
                invocation(OperationCategory::UserOnly),
            )?;
        }
        let mut prepare = prepare_write_request(
            &format!("req_run_ticket_prepare_{suffix}"),
            &format!("idem_run_ticket_prepare_{suffix}"),
            Some(harness.counts()?.state_version),
            Some(&task_id),
            Some(&change_unit_id),
        );
        prepare.intended_operation = run_operation.to_owned();
        prepare.intended_paths = run_paths.iter().map(|path| (*path).to_owned()).collect();
        prepare.sensitive_categories = run_categories
            .iter()
            .map(|category| (*category).to_owned())
            .collect();
        let prepared = harness
            .service
            .prepare_write(prepare, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(prepared.response_value["decision"], "allowed");
        let write_ticket_id = response_record_id(&prepared.response_value, "write_ticket_ref");

        let mut pending = user_action_request(
            &format!("req_run_pending_sensitive_{suffix}"),
            &format!("idem_run_pending_sensitive_{suffix}"),
            false,
            Some(harness.counts()?.state_version),
            &task_id,
            Some(&change_unit_id),
            JudgmentKind::SensitiveApproval,
        );
        pending.required_for = vec![volicord_types::values::UserActionRequiredFor::RecordRun];
        let volicord_types::schema::UserActionDraft::Choice(choice) = &mut pending.action else {
            unreachable!("sensitive approval fixture is choice-shaped")
        };
        choice.sensitive_action_scope = Some(sensitive_scope(
            action_operation,
            action_paths.to_vec(),
            action_categories.to_vec(),
        ))
        .into();
        let requested = harness
            .service
            .request_user_action(pending, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(requested.response_value["base"]["response_kind"], "result");
        let user_action_request_id =
            response_record_id(&requested.response_value, "user_action_request_ref");
        if mismatched_baseline {
            mutate_user_action_basis_json(&harness, &user_action_request_id, |basis| {
                basis["coordinates"]["baseline_ref"] = json!("baseline_other");
            })?;
        }

        let before = harness.counts()?;
        let mut request = product_write_record_run_request(
            &format!("req_run_pending_sensitive_record_{suffix}"),
            &format!("idem_run_pending_sensitive_record_{suffix}"),
            before.state_version,
            &task_id,
            &change_unit_id,
            &write_ticket_id,
            &format!("run_pending_sensitive_{suffix}"),
        );
        request.observed_changes.changed_paths =
            run_paths.iter().map(|path| (*path).to_owned()).collect();
        request.observed_changes.sensitive_categories = run_categories
            .iter()
            .map(|category| (*category).to_owned())
            .collect();
        request.performed_operation = Some(run_operation.to_owned()).into();
        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;

        if should_block {
            assert_eq!(response.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                response.response_value["errors"][0]["code"],
                "USER_DECISION_UNRESOLVED"
            );
            assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
            assert_eq!(harness.counts()?, before);

            let mut dry_run = product_write_record_run_request(
                &format!("req_run_pending_sensitive_dry_{suffix}"),
                &format!("idem_run_pending_sensitive_dry_{suffix}"),
                before.state_version,
                &task_id,
                &change_unit_id,
                &write_ticket_id,
                &format!("run_pending_sensitive_dry_{suffix}"),
            );
            dry_run.envelope.dry_run = volicord_types::schema::DryRunIntent::Requested;
            dry_run.observed_changes.changed_paths =
                run_paths.iter().map(|path| (*path).to_owned()).collect();
            dry_run.observed_changes.sensitive_categories = run_categories
                .iter()
                .map(|category| (*category).to_owned())
                .collect();
            dry_run.performed_operation = Some(run_operation.to_owned()).into();
            let dry_run = harness
                .service
                .record_run(dry_run, invocation(OperationCategory::AgentWorkflow))?;
            assert_eq!(dry_run.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                dry_run.response_value["errors"][0]["code"],
                "USER_DECISION_UNRESOLVED"
            );
            assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "active");
            assert_eq!(harness.counts()?, before);
        } else {
            assert_eq!(response.response_value["base"]["response_kind"], "result");
            let after = harness.counts()?;
            assert_eq!(after.state_version, before.state_version + 1);
            assert_eq!(after.runs, before.runs + 1);
            assert_eq!(write_ticket_status(&harness, &write_ticket_id)?, "consumed");
        }
    }
    Ok(())
}

#[test]
fn record_run_observations_derive_provenance_and_actor_fail_closed() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    enable_record_run_capabilities(&harness)?;
    let (task_id, change_unit_id) =
        create_task_with_change_unit(&harness, "run_observation_classes")?;
    let classes = [
        (
            "Agent cooperative report.",
            EvidenceSourceKind::AgentReport,
            EvidenceAssuranceLevel::CooperativeReport,
            "agent_report",
            "cooperative_report",
        ),
        (
            "External tool result.",
            EvidenceSourceKind::ExternalTool,
            EvidenceAssuranceLevel::ExternalToolResult,
            "agent_report",
            "cooperative_report",
        ),
        (
            "User observation.",
            EvidenceSourceKind::UserObservation,
            EvidenceAssuranceLevel::UserObserved,
            "agent_report",
            "cooperative_report",
        ),
        (
            "Caller-declared reused evidence.",
            EvidenceSourceKind::ReusedEvidence,
            EvidenceAssuranceLevel::ExternalToolResult,
            "agent_report",
            "cooperative_report",
        ),
        (
            "Unverified claim.",
            EvidenceSourceKind::UnverifiedClaim,
            EvidenceAssuranceLevel::Unverified,
            "unverified_claim",
            "unverified",
        ),
    ];
    let mut request = record_run_request(
        "req_run_observation_classes",
        "idem_run_observation_classes",
        false,
        Some(2),
        &task_id,
        &change_unit_id,
    );
    request.evidence_updates = classes
        .iter()
        .map(|(claim, source_kind, assurance_level, _, _)| {
            supported_evidence_update_with_provenance(claim, *source_kind, *assurance_level)
        })
        .collect();
    request.evidence_observations = classes
        .iter()
        .map(
            |(claim, source_kind, assurance_level, _, _)| EvidenceObservationInput {
                target: supplemental_evidence_target(claim),
                source_kind: *source_kind,
                assurance_level: *assurance_level,
                observed_by_actor_source: Some(ActorSource::LocalUser).into(),
                tool_name: Some("fixture-evidence-check".to_owned()).into(),
                tool_invocation_id: None.into(),
                tool_metadata: JsonObject::new(),
                input_refs: Vec::new(),
                source_refs: Vec::new(),
                output_artifact_refs: Vec::new(),
                limitations: Vec::new(),
                observed_at: volicord_types::values::UtcTimestamp::parse("2026-06-18T00:00:00Z")
                    .expect("fixture timestamp should parse"),
            },
        )
        .collect();

    let response = harness
        .service
        .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
    let observations = response.response_value["evidence_observations"]
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "evidence observations should be present: {}",
                response.response_value
            )
        });

    assert_eq!(observations.len(), classes.len());
    for (observation, (_, _, _, source_value, assurance_value)) in observations.iter().zip(classes)
    {
        assert_eq!(observation["source_kind"], source_value);
        assert_eq!(observation["assurance_level"], assurance_value);
        assert_eq!(observation["observed_by_actor_source"], AGENT_ACTOR_SOURCE);
        assert!(observation.get("guarantee_display").is_none());
    }
    Ok(())
}
