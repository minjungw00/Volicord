use super::*;

#[test]
fn intake_commits_once_and_replays_without_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let request = intake_request(
        "req_intake",
        "idem_intake",
        false,
        Some(0),
        RequestedMode::Auto,
    );

    let first = harness.service.intake(
        request.clone(),
        invocation(OperationCategory::AgentWorkflow),
    )?;
    let after_first = harness.counts()?;

    assert_eq!(first.response_value["base"]["response_kind"], "result");
    assert_eq!(
        first.response_value["base"]["effect_kind"],
        "core_committed"
    );
    assert_eq!(first.response_value["base"]["state_version"], 1);
    assert_eq!(first.response_value["state"]["mode"], "work");
    assert_eq!(after_first.state_version, before.state_version + 1);
    assert_eq!(after_first.tasks, before.tasks + 1);
    assert_eq!(after_first.task_events, before.task_events + 1);
    assert_eq!(after_first.tool_invocations, before.tool_invocations + 1);

    let second = harness
        .service
        .intake(request, invocation(OperationCategory::AgentWorkflow))?;
    assert!(second.replayed);
    assert_eq!(second.response_json, first.response_json);
    assert_eq!(harness.counts()?, after_first);
    Ok(())
}

#[test]
fn intake_dry_run_has_no_storage_effect() -> Result<(), Box<dyn Error>> {
    let harness = MethodHarness::new()?;
    let before = harness.counts()?;
    let response = harness.service.intake(
        intake_request(
            "req_intake_dry",
            "idem_intake_dry",
            true,
            Some(0),
            RequestedMode::Work,
        ),
        invocation(OperationCategory::AgentWorkflow),
    )?;

    assert_eq!(response.response_value["base"]["response_kind"], "dry_run");
    assert_eq!(response.response_value["base"]["effect_kind"], "no_effect");
    assert_eq!(harness.counts()?, before);
    Ok(())
}
