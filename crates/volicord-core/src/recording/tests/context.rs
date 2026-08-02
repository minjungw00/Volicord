use super::*;

#[test]
fn task_mode_run_kind_matrix_is_enforced_before_commit() -> Result<(), Box<dyn Error>> {
    for (requested_mode, task_mode, run_kind, run_kind_value, allowed, suffix) in [
        (
            RequestedMode::Advisor,
            "advisor",
            RunKind::Implementation,
            "implementation",
            false,
            "advisor_implementation",
        ),
        (
            RequestedMode::Advisor,
            "advisor",
            RunKind::Direct,
            "direct",
            false,
            "advisor_direct",
        ),
        (
            RequestedMode::Direct,
            "direct",
            RunKind::Implementation,
            "implementation",
            false,
            "direct_implementation",
        ),
        (
            RequestedMode::Direct,
            "direct",
            RunKind::Direct,
            "direct",
            true,
            "direct_direct",
        ),
        (
            RequestedMode::Work,
            "work",
            RunKind::Implementation,
            "implementation",
            true,
            "work_implementation",
        ),
        (
            RequestedMode::Work,
            "work",
            RunKind::Direct,
            "direct",
            false,
            "work_direct",
        ),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_mode_and_change_unit(&harness, suffix, requested_mode)?;
        let before = harness.counts()?;
        let mut request = record_run_request(
            &format!("req_mode_kind_{suffix}"),
            &format!("idem_mode_kind_{suffix}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.kind = run_kind;

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        let after = harness.counts()?;
        if allowed {
            assert_eq!(
                response.response_value["base"]["response_kind"], "result",
                "{suffix}: {:?}",
                response.response_value
            );
            assert_eq!(response.response_value["state"]["mode"], task_mode);
            assert_eq!(
                response.response_value["run_summary"]["kind"],
                run_kind_value
            );
            let run_id = run_id_from_record_run(&response.response_value);
            assert_eq!(stored_run_kind(&harness, &run_id)?, run_kind_value);
            assert_eq!(after.state_version, before.state_version + 1);
            assert_eq!(after.runs, before.runs + 1);
        } else {
            assert_eq!(response.response_value["base"]["response_kind"], "rejected");
            assert_eq!(
                response.response_value["errors"][0]["code"],
                "RUN_KIND_INCOMPATIBLE"
            );
            assert_eq!(after, before, "{suffix} must have no storage effect");
        }
    }
    Ok(())
}

#[test]
fn advisor_run_rejects_write_and_sensitive_effects_without_effect() -> Result<(), Box<dyn Error>> {
    for (
        suffix,
        product_write_observed,
        changed_paths,
        write_ticket_id,
        sensitive_categories,
        expected_code,
    ) in [
        (
            "advisor_observed_write",
            true,
            vec!["src/export.rs".to_owned()],
            None,
            Vec::new(),
            "RUN_KIND_INCOMPATIBLE",
        ),
        (
            "advisor_changed_paths",
            false,
            vec!["src/export.rs".to_owned()],
            None,
            Vec::new(),
            "VALIDATION_FAILED",
        ),
        (
            "advisor_write_ticket",
            false,
            Vec::new(),
            Some(WriteTicketId::new("wt_advisor_forbidden")),
            Vec::new(),
            "RUN_KIND_INCOMPATIBLE",
        ),
        (
            "advisor_sensitive_effect",
            false,
            Vec::new(),
            None,
            vec!["network".to_owned()],
            "RUN_KIND_INCOMPATIBLE",
        ),
    ] {
        let harness = MethodHarness::new()?;
        enable_record_run_capabilities(&harness)?;
        let (task_id, change_unit_id) =
            create_task_with_mode_and_change_unit(&harness, suffix, RequestedMode::Advisor)?;
        let before = harness.counts()?;
        let mut request = record_run_request(
            &format!("req_{suffix}"),
            &format!("idem_{suffix}"),
            false,
            Some(2),
            &task_id,
            &change_unit_id,
        );
        request.kind = RunKind::Implementation;
        request.observed_changes.product_file_write_observed = product_write_observed;
        request.observed_changes.changed_paths = changed_paths;
        request.observed_changes.sensitive_categories = sensitive_categories;
        request.write_ticket_id = write_ticket_id.into();

        let response = harness
            .service
            .record_run(request, invocation(OperationCategory::AgentWorkflow))?;
        assert_eq!(response.response_value["base"]["response_kind"], "rejected");
        assert_eq!(
            response.response_value["errors"][0]["code"], expected_code,
            "{suffix}"
        );
        assert_eq!(
            harness.counts()?,
            before,
            "{suffix} must have no storage effect"
        );
    }
    Ok(())
}
