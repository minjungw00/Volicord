<a id="volicordrecord_run"></a>

# `volicord.record_run` reference

## What this document owns

This document owns baseline method behavior for `volicord.record_run`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- run recording, current close-basis update, evidence update, blocker update, and artifact promotion method behavior
- record-run examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, artifact, value-set, or error schema definitions
- Core evidence meaning, Core authority semantics, storage DDL, storage record layouts, exact storage effects, artifact lifecycle, or security guarantees
- public error code meaning, public error precedence, machine-readable error details, or shared response-branch routing

## Purpose

`volicord.record_run` records:

- shaping work
- a direct answer or result
- implementation work

The method may also update the current close basis, update compact evidence coverage, consume a compatible `Write Authorization` when recording a product write, link existing artifacts, and promote eligible staged handles to persistent `ArtifactRef` records where allowed.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_authorization_id`, `summary`, `observed_changes`, `artifact_inputs`, `evidence_updates`, and `close_assessment`.
- Product-write runs require a compatible `status=active` `Write Authorization` from `volicord.prepare_write`.
- New artifact bytes must already be represented by a valid `StagedArtifactHandle`; `volicord.record_run` does not stage new bytes.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

All fields shown in this method-owned request block are required members of `params` unless a field note explicitly marks a member optional; `T | null` means the member must be present and may contain JSON `null`.

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string
  kind: string
  run_id: string | null
  baseline_ref: string
  write_authorization_id: string | null
  summary: string
  observed_changes: ObservedChanges
  artifact_inputs: ArtifactInput[]
  evidence_updates: EvidenceCoverageItem[]
  close_assessment: CloseAssessmentInput | null

CloseAssessmentInput:
  result_summary: string
  result_refs: StateRecordRef[]
  residual_risks: ResidualRiskInput[]
  sensitive_categories: string[]
  recovery_constraints: string[]

ResidualRiskInput:
  summary: string
  consequence: string
  acceptance_required: boolean
  source_refs: StateRecordRef[]
```

Nested owner links:
- `observed_changes` and `evidence_updates` use `ObservedChanges` and `EvidenceCoverageItem`; those shapes are owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes).
- `close_assessment.result_refs` and `ResidualRiskInput.source_refs` use `StateRecordRef`, owned by [API State Schemas](schema-state.md#state-references).
- `CurrentCloseBasis` and committed `ResidualRisk` output shapes are owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). `ResidualRiskInput` has no caller-authoritative `risk_id`; Core generates opaque `risk_id` values when committing a new current close basis.
- `artifact_inputs` uses `ArtifactInput[]`; `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef` shapes are owned by [API Artifact Schemas](schema-artifacts.md#artifactinput).
- `kind`, artifact source values, `redaction_state`, and evidence coverage values are owned by [API Value Sets](schema-value-sets.md).

Path and access notes:
- `observed_changes.changed_paths` entries are `Product Repository` API product paths. Product Repository path normalization is owned by [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- `ArtifactInput[]` and staged handles do not create a second request-level access class; the request-level access class remains the one in the derived `VerifiedSurfaceContext`.

Close-assessment ref rules:
- Caller-supplied `close_assessment.result_refs` and `ResidualRiskInput.source_refs` are restricted to `record_kind=run`, `artifact`, `evidence_summary`, or `change_unit` unless an owner explicitly adds another kind.
- The method rejects or excludes caller-supplied `project_state`, `write_authorization`, `user_judgment`, `blocker`, `task_event`, `local_surface_registration`, and `task` refs from the close basis unless an owner explicitly adds them.
- Every accepted ref must exist and belong to the same project and Task. Artifact refs must be linked to the Task and pass current-byte verification with `integrity_status=verified`; evidence refs must identify the current Task evidence summary; Run refs used as current close-basis result refs must identify a recorded current Run compatible with the current Task, current Change Unit, current scope revision, compatible baseline, and recorded status.
- Historical Run refs are audit records for close-basis purposes unless this new current Run explicitly reuses verified artifacts or evidence from history and records that reuse in its committed evidence or close assessment.
- Core stores canonical refs in `CurrentCloseBasis` and never preserves caller-supplied `state_version` metadata as authority.
- Core may add the current Run, current Change Unit, and current EvidenceSummary refs while constructing the canonical close basis.

## Access requirements

Requires:

- server-derived `VerifiedSurfaceContext` with `access_class=run_recording`

For `source_kind=staged_artifact`:

- the current derived `VerifiedSurfaceContext.surface_id` must match the staged handle's recorded provenance
- the current derived `VerifiedSurfaceContext.surface_instance_id` must match the staged handle's recorded provenance

The recorded provenance was captured from the derived `VerifiedSurfaceContext` at staging time. This method compares it with the current derived context instead of accepting caller-submitted provenance as authority.

Non-claims:

- `ArtifactInput[]` does not add `artifact_registration`.
- Cross-surface staged artifact transfer is outside the baseline scope.

## State version behavior

A compatible committed result increments `project_state.state_version` exactly once.

A compatible committed result increments the selected `Task.close_basis_revision` exactly once. When `close_assessment` is non-null, the commit establishes a new `CurrentCloseBasis` from the committed current Run, the assessment fields, generated residual-risk IDs, current Task, current Change Unit, selected current scope revision, and compatible baseline. When `close_assessment=null`, the committed Run explicitly does not establish a current close basis, and any existing current close basis becomes stale or absent.

An empty `close_assessment.residual_risks` list explicitly means the current result has no identified residual risks. Core generates opaque `risk_id` values only for committed non-null assessments. A dry-run never reserves persistent `risk_id` values.

Sensitive action requirements in the resulting `CurrentCloseBasis` are derived by Core from the committed Run and any consumed `Write Authorization`. Category-only caller input in `close_assessment.sensitive_categories` can contribute display context but cannot establish, satisfy, or erase a sensitive approval requirement.

The Run, current close basis, evidence updates, artifact links or promotions, `Write Authorization` consumption, and revision changes are committed atomically when the result commits.

Product-write recording consumes the `Write Authorization` only when:

- the current `project_state.state_version` equals `WriteAuthorization.basis_state_version` immediately before consumption
- the authorization is not expired under the effective expiration rule: the earlier of stored `expires_at` and `created_at + 15 minutes`
- observed changed paths, after Product Repository path normalization, are compatible with the authorized attempt

An authorization created by `volicord.prepare_write` is not stale immediately after creation when no intervening project state change has occurred. If `volicord.prepare_write` commits from version `19` to version `20`, `volicord.record_run` may consume that authorization while the current `project_state.state_version` and `WriteAuthorization.basis_state_version` are both `20`.

The method rejects stale `expected_state_version` and stale authorization basis before consuming the `Write Authorization`. A stale `WriteAuthorization.basis_state_version` retains higher-priority `STATE_VERSION_CONFLICT` routing even if the same authorization is also expired.

Expiration is calculated using parsed UTC timestamps, not lexical string comparison. An expired authorization is never consumed. Expired authorization use returns `WRITE_AUTHORIZATION_INVALID` with `ToolError.details.authorization_reason=expired`.

## Method result fields

`RecordRunResult` is the method-specific result branch for a committed run-recording operation. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). Committed `RecordRunResult` branches use `base.response_kind=result` and `base.effect_kind=core_committed`. `base.events[].event_kind`, when present, is an opaque illustrative classification string. |
| `run_summary` | `RunSummary` for the recorded Run. `RunSummary.kind` mirrors the request `kind`; supported run-kind values are owned by [API Value Sets](schema-value-sets.md#method-local-values). |
| `registered_artifacts` | `ArtifactRef[]` for persistent artifact refs produced or linked for this run result. `ArtifactRef` shape is owned by [API Artifact Schemas](schema-artifacts.md#artifactref); promotion and linking lifecycle details are owned by [Artifact Storage](../storage-artifacts.md). |
| `evidence_summary` | `EvidenceSummary | null` for evidence coverage updated by this run result, or `null` when the run records no evidence update. Shape is owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes); evidence authority meaning is owned by [Core Model](../core-model.md#9-evidence-and-run-authority). |
| `current_close_basis` | `CurrentCloseBasis | null` after this run is recorded. Non-null means this Run established the current close basis; `null` means this Run did not establish one. Shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). |
| `blocker_refs` | `StateRecordRef[]` for run- or evidence-related blockers committed or still relevant because of this result. |
| `state` | Current `StateSummary` after the run is recorded. Nested state fields, including `write_authority_summary` after any `Write Authorization` consumption, are owned by [API State Schemas](schema-state.md). |

Nested `StateRecordRef`, `RunSummary`, `ObservedChanges`, `EvidenceSummary`, `EvidenceCoverageItem`, `StateSummary`, and `ArtifactRef` field bodies stay with the schema owners linked above. Exact persistence effects, including staged-handle consumption, artifact promotion, evidence updates, replay rows, and `Write Authorization` consumption, stay with [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

## Success result

Returns `RecordRunResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `run_summary`
- any `registered_artifacts`
- updated `evidence_summary`
- `current_close_basis` when established, otherwise `null`
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
- expired `Write Authorization`
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

For a stale `Write Authorization` basis, rejection happens before consumption and creates no Run, evidence update, artifact link, artifact promotion, event, replay row, or `project_state.state_version` increment.

For an expired `Write Authorization`, rejection happens before consumption and creates no Run, event, replay row, artifact promotion, evidence update, authorization consumption, or `project_state.state_version` increment.

## Dry-run behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- creates no Run, current close basis, residual-risk IDs, evidence update, blocker update, artifact link, artifact promotion, or `Write Authorization` consumption

## Storage effect

On commit, the method may persist run, current close-basis, evidence, blocker, authorization-consumption, and artifact-linking results. Exact storage effects and artifact promotion details are owned by the storage documents linked below.

The examples are intentionally compact and method-local. The representative response is abbreviated to the fields needed to show the committed run, promoted artifact ref, updated evidence summary, blocker refs, state version, and current state snapshot.

## Minimal valid request

This example records validation output from a method-local staged handle. Method-local precondition: `staged_runprobe_001` is unexpired, unconsumed, and belongs to `proj_runprobe_001` / `task_runprobe_001`; its recorded surface provenance, captured at staging time, is `surface_run_probe` and `surface_instance_run_probe_01`. The precondition is local to this document and does not reuse any other method example.

```yaml
method: volicord.record_run
params:
  envelope:
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    actor_kind: agent
    surface_id: surface_run_probe
    request_id: req_runprobe_001
    idempotency_key: idem_runprobe_001
    expected_state_version: 31
    dry_run: false
    locale: en-US
  task_id: task_runprobe_001
  change_unit_id: cu_runprobe_001
  kind: implementation
  run_id: null
  baseline_ref: baseline_runprobe_001
  write_authorization_id: null
  summary: "Search-result count validation passed."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_runprobe_001
  artifact_inputs:
    - artifact_input_id: artifact_input_runprobe_001
      source_kind: staged_artifact
      staged_artifact_handle:
        handle_id: staged_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        created_by_surface_id: surface_run_probe
        created_by_surface_instance_id: surface_instance_run_probe_01
        content_type: application/json
        sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
        size_bytes: 96
        redaction_state: none
        expires_at: "<future-expiration-timestamp>"
        consumed: false
      existing_artifact_ref: null
      relation_hint: "validation_report"
      claim: "Search-result count validation passed."
      expected_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
      expected_size_bytes: 96
      redaction_state: none
  evidence_updates:
    - claim: "Search-result count validation passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs: []
      supporting_artifact_refs: []
      gap_refs: []
  close_assessment:
    result_summary: "Search-result count validation passed."
    result_refs: []
    residual_risks: []
    sensitive_categories: []
    recovery_constraints: []
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
    - event_id: evt_runprobe_001
      event_kind: run_recorded
run_summary:
  run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
  kind: implementation
  summary: "Search-result count validation passed."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_runprobe_001
  artifact_refs:
    - artifact_id: artifact_runprobe_report_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      display_name: "search-result-count-validation.json"
      content_type: application/json
      sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
      size_bytes: 96
      integrity_status: verified
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        state_version: 32
      created_by_surface_id: surface_run_probe
      created_by_surface_instance_id: surface_instance_run_probe_01
      storage_ref: "artifact-storage://search-result-count-validation"
registered_artifacts:
  - artifact_id: artifact_runprobe_report_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    display_name: "search-result-count-validation.json"
    content_type: application/json
    sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
    size_bytes: 96
    integrity_status: verified
    redaction_state: none
    availability: available
    created_by_run_ref:
      record_kind: run
      record_id: run_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      state_version: 32
    created_by_surface_id: surface_run_probe
    created_by_surface_instance_id: surface_instance_run_probe_01
    storage_ref: "artifact-storage://search-result-count-validation"
evidence_summary:
  status: sufficient
  completion_policy:
    evidence_required: true
    required_claims:
      - "Search-result count validation passed."
  coverage_items:
    - claim: "Search-result count validation passed."
      required_for_close: true
      coverage_state: supported
      supporting_refs:
        - record_kind: run
          record_id: run_runprobe_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          state_version: 32
      supporting_artifact_refs:
        - artifact_id: artifact_runprobe_report_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          display_name: "search-result-count-validation.json"
          content_type: application/json
          sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
          size_bytes: 96
          integrity_status: verified
          redaction_state: none
          availability: available
          created_by_run_ref:
            record_kind: run
            record_id: run_runprobe_001
            project_id: proj_runprobe_001
            task_id: task_runprobe_001
            state_version: 32
          created_by_surface_id: surface_run_probe
          created_by_surface_instance_id: surface_instance_run_probe_01
          storage_ref: "artifact-storage://search-result-count-validation"
      gap_refs: []
  artifact_refs:
    - artifact_id: artifact_runprobe_report_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      display_name: "search-result-count-validation.json"
      content_type: application/json
      sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
      size_bytes: 96
      integrity_status: verified
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        state_version: 32
      created_by_surface_id: surface_run_probe
      created_by_surface_instance_id: surface_instance_run_probe_01
      storage_ref: "artifact-storage://search-result-count-validation"
  updated_by_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
current_close_basis:
  close_basis_revision: 4
  scope_revision: 2
  task_id: task_runprobe_001
  change_unit_id: cu_runprobe_001
  baseline_ref: baseline_runprobe_001
  result_summary: "Search-result count validation passed."
  result_refs:
    - record_kind: run
      record_id: run_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      state_version: 32
  evidence_summary_ref: null
  residual_risks: []
  sensitive_categories: []
  sensitive_action_requirements: []
  recovery_constraints: []
  source_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
  updated_at: "<example-updated-at>"
blocker_refs: []
state:
  project_id: proj_runprobe_001
  state_version: 32
  task_ref:
    record_kind: task
    record_id: task_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Validate search-result count display."
  scope_summary: "Search-result count validation."
  non_goals:
    - "Changing search ranking."
  acceptance_criteria:
    - "Search results show the expected count."
  autonomy_boundary: "Stay within validation recording for search-result counts."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 31
  baseline_ref: baseline_runprobe_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_authority_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `CurrentCloseBasis`, `ResidualRisk`, `StateSummary`, and refs: [API State Schemas](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- `Write Authorization` and close-relevant evidence boundaries: [Core Model](../core-model.md).
- Product Repository path normalization: [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- Supported values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, precedence, response routing, and artifact-input detail values: [API error codes](error-codes.md), [API error precedence](error-precedence.md), [API error routing](error-routing.md), and [artifact-input error details](error-details.md#artifact-input-error-reason).
- Persistence effects and artifact promotion: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).
