<a id="harnessrecord_run"></a>

# `harness.record_run` reference

## What this document owns

This document owns baseline method behavior for `harness.record_run`:

- method-specific required inputs, access requirements, state-version behavior, result branches, and dry-run behavior
- the minimal request and representative response for the shared account data export confirmation scenario
- method-level storage-effect expectations before storage owners define record-level details

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, artifact lifecycle, security guarantees, or Core product meaning

## Purpose

`harness.record_run` records:

- shaping work
- a direct answer or result
- implementation work

Additional results:

- updates compact evidence coverage
- consumes a compatible Write Authorization when recording a product write
- links existing artifacts
- promotes eligible staged handles to persistent `ArtifactRef` records where allowed

## Required inputs

- `ToolEnvelope` with non-null `idempotency_key` and current `expected_state_version` for non-dry-run commits.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_authorization_id`, `summary`, `observed_changes`, `artifact_inputs`, and `evidence_updates`.
- Product-write runs require a compatible active Write Authorization from `harness.prepare_write`.
- New artifact bytes must already be represented by a valid `StagedArtifactHandle`; `record_run` does not stage new bytes.

## Access requirements

Requires:

- `VerifiedSurfaceContext.access_class=run_recording`
- `verified=true`

For `source_kind=staged_artifact`:

- the current verified `surface_id` must match the staged handle's recorded provenance
- the current verified `surface_instance_id` must match the staged handle's recorded provenance

Non-claims:

- `ArtifactInput[]` does not add `artifact_registration`.
- Cross-surface staged artifact transfer is outside the baseline scope.

## State version behavior

A compatible committed result increments `project_state.state_version` exactly once.

Product-write recording consumes the active Write Authorization only when:

- the current state version still matches the authorization basis
- observed changed paths are compatible with the authorized attempt

Rejected before consumption:

- stale `expected_state_version`
- stale authorization basis

## Success result

Returns `RecordRunResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `run_summary`
- any `registered_artifacts`
- updated `evidence_summary`
- `blocker_refs`
- current `state`

## Blocked result

The method may commit compatible run-related blocker state when the run is recordable but the result creates or preserves blockers, such as evidence gaps.

Non-claim: a committed blocked result must not hide:

- invalid staged handles
- missing Write Authorization
- stale state
- stale authorization basis
- local access failures

Those failures are rejected before commit.

## Rejected result

Returns `ToolRejectedResponse` for:

- stale `expected_state_version`
- stale Write Authorization basis
- missing or invalid Write Authorization for product writes
- invalid staged handle
- incompatible staged-handle provenance
- missing artifact
- scope violation
- baseline staleness
- local access failure
- insufficient capability
- validator failure

Non-claim: invalid staged handles are validation failures with artifact-input details, not local access mismatch unless request-level local access itself failed.

## Dry-run behavior

For `dry_run=true`, a valid preview returns `ToolDryRunResponse`.

Branch shape is owned by [API Schema Core](schema-core.md); no-effect persistence and promotion semantics are owned by [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

## Storage effect

On commit, the method may persist:

- run results
- evidence results
- blocker results
- authorization-consumption results
- artifact-linking results

Exact storage effects are owned by [Storage Effects](../storage-effects.md), and artifact promotion details are owned by [Artifact Storage](../storage-artifacts.md).

Run data example:

The run records account data export confirmation test evidence and may consume the staged test log from the shared `harness.stage_artifact` example as evidence:

This example records test evidence after the write path has already been handled. It does not claim that this run observed the product file write itself.

```yaml
command: "npm test -- account-export"
summary: "Account data export confirmation tests passed."
artifacts:
  - staged_artifact_account_export_test_log_001
run_ref: run_account_export_tests_001
state_version: 21
```

## Minimal valid request

```yaml
method: harness.record_run
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_run_001
    idempotency_key: idem_run_001
    expected_state_version: 20
    dry_run: false
    locale: en-US
  task_id: task_456
  change_unit_id: cu_001
  kind: implementation
  run_id: null
  baseline_ref: baseline_account_export_001
  write_authorization_id: null
  summary: "Account data export confirmation tests passed."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_account_export_001
  artifact_inputs:
    - artifact_input_id: artifact_input_account_export_test_log_001
      source_kind: staged_artifact
      staged_artifact_handle:
        handle_id: staged_artifact_account_export_test_log_001
        project_id: proj_123
        task_id: task_456
        created_by_surface_id: surface_local
        created_by_surface_instance_id: surface_instance_01
        content_type: text/plain
        sha256: sha256:example
        size_bytes: 65
        redaction_state: none
        expires_at: "<future-expiration-timestamp>"
        consumed: false
      existing_artifact_ref: null
      relation_hint: "test_log"
      claim: "Test output for account data export confirmation tests."
      expected_sha256: null
      expected_size_bytes: null
      redaction_state: none
  evidence_updates:
    - claim: "Account data export confirmation tests passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs: []
      supporting_artifact_refs: []
      gap_refs: []
```

## Representative response

Result branch (`RecordRunResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 21
  events:
    - event_id: evt_1004
      event_kind: run_recorded
run_summary:
  run_ref:
    record_kind: run
    record_id: run_account_export_tests_001
    project_id: proj_123
    task_id: task_456
    state_version: 21
  kind: implementation
  summary: "Account data export confirmation tests passed."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_account_export_001
  artifact_refs:
    - artifact_id: artifact_account_export_test_log_001
      project_id: proj_123
      task_id: task_456
      display_name: "account_export_confirmation_test.log"
      content_type: text/plain
      sha256: sha256:example
      size_bytes: 65
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_account_export_tests_001
        project_id: proj_123
        task_id: task_456
        state_version: 21
      created_by_surface_id: surface_local
      created_by_surface_instance_id: surface_instance_01
      storage_ref: artifact://artifact_account_export_test_log_001
registered_artifacts:
  - artifact_id: artifact_account_export_test_log_001
    project_id: proj_123
    task_id: task_456
    display_name: "account_export_confirmation_test.log"
    content_type: text/plain
    sha256: sha256:example
    size_bytes: 65
    redaction_state: none
    availability: available
    created_by_run_ref:
      record_kind: run
      record_id: run_account_export_tests_001
      project_id: proj_123
      task_id: task_456
      state_version: 21
    created_by_surface_id: surface_local
    created_by_surface_instance_id: surface_instance_01
    storage_ref: artifact://artifact_account_export_test_log_001
evidence_summary:
  status: sufficient
  coverage_items:
    - claim: "Account data export confirmation tests passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_account_export_tests_001
          project_id: proj_123
          task_id: task_456
          state_version: 21
      supporting_artifact_refs:
        - artifact_id: artifact_account_export_test_log_001
          project_id: proj_123
          task_id: task_456
          display_name: "account_export_confirmation_test.log"
          content_type: text/plain
          sha256: sha256:example
          size_bytes: 65
          redaction_state: none
          availability: available
          created_by_run_ref:
            record_kind: run
            record_id: run_account_export_tests_001
            project_id: proj_123
            task_id: task_456
            state_version: 21
          created_by_surface_id: surface_local
          created_by_surface_instance_id: surface_instance_01
          storage_ref: artifact://artifact_account_export_test_log_001
      gap_refs: []
  artifact_refs:
    - artifact_id: artifact_account_export_test_log_001
      project_id: proj_123
      task_id: task_456
      display_name: "account_export_confirmation_test.log"
      content_type: text/plain
      sha256: sha256:example
      size_bytes: 65
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_account_export_tests_001
        project_id: proj_123
        task_id: task_456
        state_version: 21
      created_by_surface_id: surface_local
      created_by_surface_instance_id: surface_instance_01
      storage_ref: artifact://artifact_account_export_test_log_001
blocker_refs: []
state:
  project_id: proj_123
  state_version: 21
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 21
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `StateSummary`, and refs: [API State Schemas](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- Write Authorization and close-relevant evidence boundaries: [Core Model](../core-model.md).
- Active values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors: [API Errors](errors.md).
- Persistence effects and artifact promotion: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).
