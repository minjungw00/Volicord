<a id="harnessrecord_run"></a>

# `harness.record_run` reference

## What this document owns

This document owns baseline method behavior for `harness.record_run`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- run recording, evidence update, blocker update, and artifact promotion method behavior
- record-run examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, artifact, value-set, or error schema definitions
- Core evidence meaning, Core authority semantics, storage DDL, storage record layouts, exact storage effects, artifact lifecycle, or security guarantees
- public error code meaning, public error precedence, machine-readable error details, or shared response-branch routing

## Purpose

`harness.record_run` records:

- shaping work
- a direct answer or result
- implementation work

The method may also update compact evidence coverage, consume a compatible `Write Authorization` when recording a product write, link existing artifacts, and promote eligible staged handles to persistent `ArtifactRef` records where allowed.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_authorization_id`, `summary`, `observed_changes`, `artifact_inputs`, and `evidence_updates`.
- Product-write runs require a compatible `status=active` `Write Authorization` from `harness.prepare_write`.
- New artifact bytes must already be represented by a valid `StagedArtifactHandle`; `harness.record_run` does not stage new bytes.

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

Product-write recording consumes the `Write Authorization` only when:

- the current state version still matches the authorization basis
- observed changed paths are compatible with the authorized attempt

The method rejects stale `expected_state_version` and stale authorization basis before consuming the `Write Authorization`.

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

Not allowed:

- A committed blocked result must not hide invalid staged handles, missing `Write Authorization`, stale state, stale authorization basis, or local access failures.

Those failures are rejected before commit.

## Rejected result

Returns `ToolRejectedResponse` for:

- stale `expected_state_version`
- stale `Write Authorization` basis
- missing or invalid `Write Authorization` for product writes
- invalid staged handle
- incompatible staged-handle provenance
- missing artifact
- scope violation
- baseline staleness
- local access failure
- insufficient capability
- validator failure

Non-claim: invalid staged handles are validation failures with artifact-input details owned by [API error details](error-details.md#artifact-input-error-reason), not local access mismatch unless request-level local access itself failed.

Public error code meaning, precedence, details, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- creates no Run, evidence update, blocker update, artifact link, artifact promotion, or `Write Authorization` consumption

## Storage effect

On commit, the method may persist run, evidence, blocker, authorization-consumption, and artifact-linking results. Exact storage effects and artifact promotion details are owned by the storage documents linked below.

## Minimal valid request

```yaml
method: harness.record_run
params:
  envelope:
    project_id: proj_validation_001
    task_id: task_validation_001
    actor_kind: agent
    surface_id: surface_run
    request_id: req_run_validation_001
    idempotency_key: idem_run_validation_001
    expected_state_version: 31
    dry_run: false
    locale: en-US
  task_id: task_validation_001
  change_unit_id: cu_validation_001
  kind: implementation
  run_id: null
  baseline_ref: baseline_validation_001
  write_authorization_id: null
  summary: "Shipping-rate preview validation passed."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_validation_001
  artifact_inputs:
    - artifact_input_id: artifact_input_validation_001
      source_kind: existing_artifact
      staged_artifact_handle: null
      existing_artifact_ref:
        artifact_id: artifact_validation_report_001
        project_id: proj_validation_001
        task_id: task_validation_001
        display_name: "shipping-rate-validation.json"
        content_type: application/json
        sha256: sha256:example-validation
        size_bytes: 128
        redaction_state: none
        availability: available
        created_by_run_ref: null
        created_by_surface_id: surface_run
        created_by_surface_instance_id: surface_instance_run_01
        storage_ref: "artifact-storage://shipping-rate-validation"
      relation_hint: "validation_report"
      claim: "Shipping-rate preview validation passed."
      expected_sha256: "sha256:example-validation"
      expected_size_bytes: 128
      redaction_state: none
  evidence_updates:
    - claim: "Shipping-rate preview validation passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs: []
      supporting_artifact_refs:
        - artifact_id: artifact_validation_report_001
          project_id: proj_validation_001
          task_id: task_validation_001
          display_name: "shipping-rate-validation.json"
          content_type: application/json
          sha256: sha256:example-validation
          size_bytes: 128
          redaction_state: none
          availability: available
          created_by_run_ref: null
          created_by_surface_id: surface_run
          created_by_surface_instance_id: surface_instance_run_01
          storage_ref: "artifact-storage://shipping-rate-validation"
      gap_refs: []
```

## Representative response

Result branch (`RecordRunResult`, committed):

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 32
  events:
    - event_id: evt_validation_001
      event_kind: run_recorded
run_summary:
  run_ref:
    record_kind: run
    record_id: run_validation_001
    project_id: proj_validation_001
    task_id: task_validation_001
    state_version: 32
  kind: implementation
  summary: "Shipping-rate preview validation passed."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_validation_001
  artifact_refs:
    - artifact_id: artifact_validation_report_001
      project_id: proj_validation_001
      task_id: task_validation_001
      display_name: "shipping-rate-validation.json"
      content_type: application/json
      sha256: sha256:example-validation
      size_bytes: 128
      redaction_state: none
      availability: available
      created_by_run_ref: null
      created_by_surface_id: surface_run
      created_by_surface_instance_id: surface_instance_run_01
      storage_ref: "artifact-storage://shipping-rate-validation"
registered_artifacts:
  - artifact_id: artifact_validation_report_001
    project_id: proj_validation_001
    task_id: task_validation_001
    display_name: "shipping-rate-validation.json"
    content_type: application/json
    sha256: sha256:example-validation
    size_bytes: 128
    redaction_state: none
    availability: available
    created_by_run_ref: null
    created_by_surface_id: surface_run
    created_by_surface_instance_id: surface_instance_run_01
    storage_ref: "artifact-storage://shipping-rate-validation"
evidence_summary:
  status: sufficient
  completion_policy:
    evidence_required: true
    required_claims:
      - "Shipping-rate preview validation passed."
  coverage_items:
    - claim: "Shipping-rate preview validation passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_validation_001
          project_id: proj_validation_001
          task_id: task_validation_001
          state_version: 32
      supporting_artifact_refs:
        - artifact_id: artifact_validation_report_001
          project_id: proj_validation_001
          task_id: task_validation_001
          display_name: "shipping-rate-validation.json"
          content_type: application/json
          sha256: sha256:example-validation
          size_bytes: 128
          redaction_state: none
          availability: available
          created_by_run_ref: null
          created_by_surface_id: surface_run
          created_by_surface_instance_id: surface_instance_run_01
          storage_ref: "artifact-storage://shipping-rate-validation"
      gap_refs: []
  artifact_refs:
    - artifact_id: artifact_validation_report_001
      project_id: proj_validation_001
      task_id: task_validation_001
      display_name: "shipping-rate-validation.json"
      content_type: application/json
      sha256: sha256:example-validation
      size_bytes: 128
      redaction_state: none
      availability: available
      created_by_run_ref: null
      created_by_surface_id: surface_run
      created_by_surface_instance_id: surface_instance_run_01
      storage_ref: "artifact-storage://shipping-rate-validation"
  updated_by_run_ref:
    record_kind: run
    record_id: run_validation_001
    project_id: proj_validation_001
    task_id: task_validation_001
    state_version: 32
blocker_refs: []
state:
  project_id: proj_validation_001
  state_version: 32
  task_ref:
    record_kind: task
    record_id: task_validation_001
    project_id: proj_validation_001
    task_id: task_validation_001
    state_version: 32
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `StateSummary`, and refs: [API State Schemas](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- `Write Authorization` and close-relevant evidence boundaries: [Core Model](../core-model.md).
- Supported values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, precedence, response routing, and artifact-input detail values: [API error codes](error-codes.md), [API error precedence](error-precedence.md), [API error routing](error-routing.md), and [artifact-input error details](error-details.md#artifact-input-error-reason).
- Persistence effects and artifact promotion: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).
