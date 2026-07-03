<a id="volicordrecord_run"></a>

# `volicord.record_run` reference

## What this document owns

This document owns baseline method behavior for `volicord.record_run`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- run recording, current close-basis update, evidence update, evidence observation recording, blocker update, and artifact promotion method behavior
- record-run examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- nested state, artifact, value-set, or error schema definitions
- Core evidence meaning, Core authority semantics, storage DDL, storage record layouts, exact storage effects, artifact lifecycle, or security guarantees
- public error code meaning, public error precedence, machine-readable error details, or shared response-branch routing

## Purpose

`volicord.record_run` is the baseline public method for recording Evidence after meaningful work. It records a Run for:

- shaping work
- a direct answer or result
- implementation work

The method may also update the current close basis, update compact claim-scoped evidence coverage, record evidence observations for reported or observed claims, consume a compatible write ticket when recording a product write, link existing Evidence attachments, and promote eligible staged attachment inputs to persistent `ArtifactRef` records where allowed. Input-only or staged-only items are not accepted Evidence and do not establish close readiness until this method records the claim, provenance, and any attachment link or promotion according to the evidence rules below.

## Required inputs

- A valid `ToolEnvelope`; committed non-dry-run requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_ticket_id`, `summary`, `observed_changes`, `artifact_inputs`, `evidence_updates`, `evidence_observations`, and `close_assessment`.
- Product-write runs require a compatible `status=active` write ticket from `volicord.prepare_write`.
- New artifact bytes must already be represented by a valid `StagedArtifactHandle`; `volicord.record_run` does not stage new bytes. The handle remains an Evidence attachment input until accepted in a committed run result.
- A supported evidence update must be backed by a same-claim `EvidenceObservationInput`, a usable same-claim evidence observation ref, or `EvidenceCoverageItem.provenance` from which Core can create an evidence observation with explicit `source_kind` and `assurance_level`.

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
  write_ticket_id: string | null
  summary: string
  observed_changes: ObservedChanges
  artifact_inputs: ArtifactInput[]
  evidence_updates: EvidenceCoverageItem[]
  evidence_observations: EvidenceObservationInput[]
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
- `observed_changes`, `evidence_updates`, and `evidence_observations` use `ObservedChanges`, `EvidenceCoverageItem`, and `EvidenceObservationInput`; those shapes are owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes).
- `close_assessment.result_refs` and `ResidualRiskInput.source_refs` use `StateRecordRef`, owned by [API State Schemas](schema-state.md#state-references).
- `CurrentCloseBasis` and committed `ResidualRisk` output shapes are owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). `ResidualRiskInput` has no caller-authoritative `risk_id`; Core generates opaque `risk_id` values when committing a new current close basis.
- `artifact_inputs` uses `ArtifactInput[]`; `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef` shapes are owned by [API Artifact Schemas](schema-artifacts.md#artifactinput).
- `kind`, artifact source values, `redaction_state`, and evidence coverage values are owned by [API Value Sets](schema-value-sets.md).

Path and access notes:
- `observed_changes.changed_paths` entries are `Product Repository` API product paths. Product Repository path normalization is owned by [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- `ArtifactInput[]` and staged handles do not create a second request-level operation category or actor source; the invocation remains the one in the verified invocation context.
- `ArtifactInput[]` members are Evidence attachment inputs. They support Evidence only when this method links them to recorded claim-scoped evidence or observations; their presence in the request is not evidence sufficiency.

Close-assessment ref rules:
- Caller-supplied `close_assessment.result_refs` and `ResidualRiskInput.source_refs` are restricted to `record_kind=run`, `artifact`, `evidence_summary`, or `change_unit` unless an owner explicitly adds another kind.
- The method rejects or excludes caller-supplied `project_state`, `write_ticket`, `user_judgment`, `blocker`, `task_event`, and `task` refs from the close basis unless an owner explicitly adds them.
- Every accepted ref must exist and belong to the same project and Task. Artifact refs must be linked to the Task and pass current-byte verification with `integrity_status=verified`; evidence refs must identify the current Task evidence summary; Run refs used as current close-basis result refs must identify a recorded current Run compatible with the current Task, current Change Unit, current scope revision, compatible baseline, and recorded status.
- Historical Run refs are audit records for close-basis purposes unless this new current Run explicitly reuses verified artifacts or evidence from history and records that reuse in its committed evidence or close assessment.
- Core stores canonical refs in `CurrentCloseBasis` and never preserves caller-supplied `state_version` metadata as authority.
- Core may add the current Run, current Change Unit, and current EvidenceSummary refs while constructing the canonical close basis.

Evidence update provenance rules:
- `coverage_state=supported` is a claim about coverage, not sufficient provenance by itself.
- When `EvidenceCoverageItem.provenance` is supplied for a supported item and no explicit same-claim observation input is supplied, Core creates an `EvidenceObservation` for the current Run and links its ref into the committed evidence summary.
- Committed evidence observations keep the explicit provenance class through `source_kind` and `assurance_level`, including `agent_report`, `connection_observation`, `external_tool`, `user_observation`, and `unverified_claim`.
- `unverified_claim`, `unverified`, and cooperative `agent_report` observations may be recorded as evidence observations, but close readiness evaluates them as weak provenance when stronger provenance is required.
- Evidence observations do not replace user-owned judgment, final acceptance, residual-risk acceptance, or close readiness.

## Access requirements

Requires:

- verified invocation context with `operation_category=agent_workflow`

For `source_kind=staged_artifact`:

- the current verified `actor_source` must match the staged handle's recorded provenance

The recorded provenance was captured from the verified invocation context at staging time. This method compares it with the current verified context instead of accepting caller-submitted provenance as authority.

Non-claims:

- `ArtifactInput[]` does not add `artifact_registration`.
- Cross-actor staged artifact transfer is outside the baseline scope.

## State version behavior

A compatible committed result increments `project_state.state_version` exactly once.

A compatible committed result increments the selected `Task.close_basis_revision` exactly once. When `close_assessment` is non-null, the commit establishes a new `CurrentCloseBasis` from the committed current Run, the assessment fields, generated residual-risk IDs, current Task, current Change Unit, selected current scope revision, and compatible baseline. When `close_assessment=null`, the committed Run explicitly does not establish a current close basis, and any existing current close basis becomes stale or absent.

An empty `close_assessment.residual_risks` list explicitly means the current result has no identified residual risks. Core generates opaque `risk_id` values only for committed non-null assessments. A dry-run never reserves persistent `risk_id` values.

Sensitive action requirements in the resulting `CurrentCloseBasis` are derived by Core from the committed Run and any consumed write ticket. Category-only caller input in `close_assessment.sensitive_categories` can contribute display context but cannot establish, satisfy, or erase a sensitive approval requirement.

The Run, current close basis, evidence updates, evidence observations, artifact links or promotions, write-ticket consumption, and revision changes are committed atomically when the result commits.

Product-write recording consumes the write ticket only when:

- the ticket has `status=active` and has not already been consumed or revoked
- the current `project_state.state_version` equals `WriteTicket.basis_state_version` immediately before consumption
- the ticket is not expired under the effective expiration rule: the earlier of stored `expires_at` and `created_at + 15 minutes`
- the ticket and its `WriteTicketAttemptScope` identify the same `task_id` and `change_unit_id` as the Run being recorded
- the checked attempt has `product_file_write_intended=true`
- the checked attempt `baseline_ref` matches the Run `baseline_ref`
- observed sensitive categories match the checked attempt's normalized `sensitive_categories`
- observed changed paths, after Product Repository path normalization, are compatible with the checked attempt

A write ticket issued by `volicord.prepare_write` is not stale immediately after issuance when no intervening project state change has occurred. If `volicord.prepare_write` commits from version `19` to version `20`, `volicord.record_run` may consume that write ticket while the current `project_state.state_version` and `WriteTicket.basis_state_version` are both `20`.

The method rejects stale `expected_state_version` and stale write-ticket basis before consuming the ticket. A stale `WriteTicket.basis_state_version` retains higher-priority `STATE_VERSION_CONFLICT` routing even if the same ticket is also expired.

Expiration is calculated using parsed UTC timestamps, not lexical string comparison. An expired write ticket is never consumed. Expired write-ticket use returns `WRITE_TICKET_INVALID` with `ToolError.details.write_ticket_reason=expired`.

Compatibility mismatch rejections use `WRITE_TICKET_INVALID` with `ToolError.details.write_ticket_reason` values such as `task_mismatch`, `change_unit_mismatch`, `product_write_flag_mismatch`, `baseline_mismatch`, `sensitive_category_mismatch`, or `path_mismatch`.

## Method result fields

`RecordRunResult` is the method-specific result branch for a committed run-recording operation. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). Committed `RecordRunResult` branches use `base.response_kind=result` and `base.effect_kind=core_committed`. `base.events[].event_kind`, when present, is an opaque illustrative classification string. |
| `run_summary` | `RunSummary` for the recorded Run. `RunSummary.kind` mirrors the request `kind`; supported run-kind values are owned by [API Value Sets](schema-value-sets.md#method-local-values). |
| `registered_artifacts` | `ArtifactRef[]` for persistent artifact refs produced or linked for this run result. These refs are Evidence attachments only when the committed evidence summary or observations link them to claims; their existence alone is not evidence sufficiency. `ArtifactRef` shape is owned by [API Artifact Schemas](schema-artifacts.md#artifactref); promotion and linking lifecycle details are owned by [Artifact Storage](../storage-artifacts.md). |
| `evidence_summary` | `EvidenceSummary | null` for evidence coverage updated by this run result, or `null` when the run records no evidence update. When present, `evidence_summary.evidence_state` is `attached` unless this result establishes a current close basis that accepts the summary for close-readiness display. Shape is owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes); evidence authority meaning is owned by [Core Model](../core-model.md#9-evidence-and-run-authority). |
| `evidence_observations` | `EvidenceObservation[]` for observation records committed by this run result. Empty when the request records no observations. Shape is owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes); observation source and assurance values are owned by [API Value Sets](schema-value-sets.md#evidence-observation-values). |
| `current_close_basis` | `CurrentCloseBasis | null` after this run is recorded. Non-null means this Run established the current close basis; `null` means this Run did not establish one. Shape is owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). |
| `blocker_refs` | `StateRecordRef[]` for run- or evidence-related blockers committed or still relevant because of this result. |
| `state` | Current `StateSummary` after the run is recorded. Nested state fields, including `write_ticket_summary` after any write-ticket consumption, are owned by [API State Schemas](schema-state.md). When a product-write Run consumes a write ticket, that summary can expose `status=consumed`, `consumed_by_run_ref`, and observation refs created by the consuming Run. |

Nested `StateRecordRef`, `RunSummary`, `ObservedChanges`, `EvidenceSummary`, `EvidenceCoverageItem`, `EvidenceObservation`, `StateSummary`, and `ArtifactRef` field bodies stay with the schema owners linked above. Exact persistence effects, including staged-handle consumption, artifact promotion, evidence updates, evidence observation records, replay rows, and write-ticket consumption, stay with [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

## Success result

Returns `RecordRunResult` with:

- `base.response_kind=result`
- `base.effect_kind=core_committed`
- `run_summary`
- any `registered_artifacts`
- updated `evidence_summary`
- committed `evidence_observations`
- `current_close_basis` when established, otherwise `null`
- `blocker_refs`
- current `state`

## Blocked result

The method may commit compatible run-related blocker state when the run is recordable but the result creates or preserves blockers, such as evidence gaps.

Not allowed:

- A committed blocked result must not hide invalid staged handles, missing write ticket, stale state, stale write-ticket basis, or invocation-context failures.

Those failures are rejected before commit.

## Rejected result

Returns `ToolRejectedResponse` for:

- stale `expected_state_version`
- stale write-ticket basis
- missing or invalid write ticket for product writes
- expired write ticket
- incompatible write-ticket path, baseline, product-write flag, sensitivity category, Task, or Change Unit
- invalid staged handle
- incompatible staged-handle provenance
- supported evidence update without required observation provenance
- missing artifact
- scope violation
- baseline staleness
- actor-source or operation-category mismatch
- unsupported invocation context
- validator failure

Non-claim: invalid staged handles are validation failures with artifact-input details owned by [API error details](error-details.md#artifact-input-error-reason), not invocation-context mismatch unless the request invocation itself failed.

Public error code meaning, precedence, details, and rejected-response routing are owned by the error documents linked below.

For a stale write-ticket basis, rejection happens before consumption and creates no Run, evidence update, evidence observation, artifact link, artifact promotion, event, replay row, or `project_state.state_version` increment.

For an expired write ticket, rejection happens before consumption and creates no Run, event, replay row, artifact promotion, evidence update, evidence observation, write-ticket consumption, or `project_state.state_version` increment.

## Dry-run behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- creates no Run, current close basis, residual-risk IDs, evidence update, evidence observation, blocker update, artifact link, artifact promotion, or write-ticket consumption

## Storage effect

On commit, the method may persist run, current close-basis, evidence summary, evidence observation, blocker, write-ticket consumption, and artifact-linking results. Exact storage effects and artifact promotion details are owned by the storage documents linked below.

The examples are intentionally compact and method-local. The representative response is abbreviated to the fields needed to show the committed run, promoted artifact ref, updated evidence summary, evidence observation, blocker refs, state version, and current state snapshot.

## Minimal valid request

This example records validation output from a method-local staged handle. Method-local precondition: `staged_runprobe_001` is unexpired, unconsumed, and belongs to `proj_runprobe_001` / `task_runprobe_001`; its recorded actor provenance, captured at staging time, is `agent_connection:conn_run_probe`. The precondition is local to this document and does not reuse any other method example.

```yaml
method: volicord.record_run
params:
  envelope:
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
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
  write_ticket_id: null
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
        created_by_actor_source: agent_connection:conn_run_probe
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
      observation_refs: []
      supporting_artifact_refs: []
      gap_refs: []
  evidence_observations:
    - claim: "Search-result count validation passed."
      source_kind: external_tool
      assurance_level: external_tool_result
      observed_by_actor_source: agent_connection:conn_run_probe
      tool_name: "search-count-validator"
      tool_invocation_id: null
      tool_metadata:
        validator: "search-count"
      input_refs: []
      output_artifact_refs: []
      limitations: []
      observed_at: "<example-observed-at>"
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
      created_by_actor_source: agent_connection:conn_run_probe
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
    created_by_actor_source: agent_connection:conn_run_probe
    storage_ref: "artifact-storage://search-result-count-validation"
evidence_summary:
  evidence_state: accepted_for_close
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
      observation_refs:
        - record_kind: evidence_observation
          record_id: evidence_observation_runprobe_001
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
          created_by_actor_source: agent_connection:conn_run_probe
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
      created_by_actor_source: agent_connection:conn_run_probe
      storage_ref: "artifact-storage://search-result-count-validation"
  observation_refs:
    - record_kind: evidence_observation
      record_id: evidence_observation_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      state_version: 32
  updated_by_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
evidence_observations:
  - observation_id: evidence_observation_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    change_unit_id: cu_runprobe_001
    run_ref:
      record_kind: run
      record_id: run_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      state_version: 32
    claim: "Search-result count validation passed."
    source_kind: external_tool
    assurance_level: external_tool_result
    observed_by_actor_source: agent_connection:conn_run_probe
    tool_name: "search-count-validator"
    tool_invocation_id: null
    tool_metadata:
      validator: "search-count"
    input_refs: []
    output_artifact_refs:
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
        created_by_actor_source: agent_connection:conn_run_probe
        storage_ref: "artifact-storage://search-result-count-validation"
    limitations: []
    observed_at: "<example-observed-at>"
    recorded_at: "<example-recorded-at>"
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
    - record_kind: change_unit
      record_id: cu_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      state_version: 32
    - record_kind: evidence_summary
      record_id: evidence_summary_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      state_version: 32
  evidence_summary_ref:
    record_kind: evidence_summary
    record_id: evidence_summary_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    state_version: 32
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
  write_ticket_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `EvidenceObservation`, `CurrentCloseBasis`, `ResidualRisk`, `StateSummary`, and refs: [API State Schemas](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- Write-ticket and close-relevant evidence boundaries: [Core Model](../core-model.md).
- Product Repository path normalization: [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- Supported values and operation categories: [API Value Sets](schema-value-sets.md#operation-category-values).
- Public errors, precedence, response routing, and artifact-input detail values: [API error codes](error-codes.md), [API error precedence](error-precedence.md), [API error routing](error-routing.md), and [artifact-input error details](error-details.md#artifact-input-error-reason).
- Persistence effects and artifact promotion: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).
