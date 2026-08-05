<a id="volicordrecord_run"></a>

# `volicord.record_run` reference

## What this document owns

This document owns baseline method behavior for `volicord.record_run`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- run recording, current close-basis update, evidence update, evidence observation recording, blocker update, and artifact promotion method behavior
- record-run examples

## What this document does not own

This document does not own:

- common request envelope, response branch, `dry_run`, or rejected-response schema bodies
- nested state, artifact, value-set, or error schema definitions
- Core evidence meaning, Core authority semantics, storage DDL, storage record layouts, exact storage effects, artifact lifecycle, or security guarantees
- public error code meaning, public error precedence, machine-readable error details, or shared response-branch routing

## Implementation routes

The public entry point at
[`crates/volicord-core/src/methods/record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs)
owns request-specific pipeline orchestration, converts the public request to
semantic Recording input, maps typed recording failures, and converts semantic
result facts into the public method response.

Current Record Run implementation responsibilities route as follows:

- [`crates/volicord-core/src/recording/context.rs`](../../../../crates/volicord-core/src/recording/context.rs)
  normalizes the request and acquires the typed Task, Change Unit, workflow,
  and control facts.
- [`crates/volicord-core/src/recording/authority.rs`](../../../../crates/volicord-core/src/recording/authority.rs),
  [`evidence.rs`](../../../../crates/volicord-core/src/recording/evidence.rs),
  and [`artifact.rs`](../../../../crates/volicord-core/src/recording/artifact.rs)
  resolve capture authority and produce typed evidence-target, observation, and
  artifact plans through the shared evidence and artifact policies.
- [`crates/volicord-core/src/write_ticket/approval.rs`](../../../../crates/volicord-core/src/write_ticket/approval.rs)
  is the only consumer of raw current UserAction authority facts. It owns the
  canonical Write Ticket approval requirement, privately constructs the typed
  current sensitive-approval set, and returns the semantic basis assessment;
  [`current_validity.rs`](../../../../crates/volicord-core/src/write_ticket/current_validity.rs)
  receives non-approval current facts plus that typed assessment and converts
  an active stored candidate into `ReusableStoredWriteTicket`, while
  [`admission.rs`](../../../../crates/volicord-core/src/write_ticket/admission.rs)
  passes locally loaded raw approval authority directly to the approval owner,
  then combines that reusable type with the matching exact-attempt
  compatibility proof before returning `AdmissibleStoredWriteTicket`.
  Terminal stored states cannot enter this admission path.
- [`crates/volicord-core/src/close_readiness/recording.rs`](../../../../crates/volicord-core/src/close_readiness/recording.rs)
  constructs the typed current close basis and residual-risk facts used by the
  shared close-readiness service.
- [`crates/volicord-core/src/recording/plan.rs`](../../../../crates/volicord-core/src/recording/plan.rs)
  coordinates those owners, assembles the typed mutation plan, and returns the
  closed `RecordRunOperationPlan` with effect and result facts. It carries only
  `AdmissibleStoredWriteTicket` into protected mutation planning;
  [`state.rs`](../../../../crates/volicord-core/src/recording/state.rs) acquires
  the Store-aware post-operation state facts. The public entry point converts
  the returned facts to the neutral Core operation carrier and
  `RecordRunResultFields`.

The exact dependency and transaction boundaries are described in the
[Core architecture](../../architecture-guide/architecture.md),
[request lifecycle](../../architecture-guide/request-lifecycle.md), and
[source map](../../architecture-guide/source-map.md).

## Purpose

`volicord.record_run` records execution and its Evidence. Shaping analysis is recorded with `volicord.record_shaping_checkpoint`.

The current persisted `Task.mode`, `work_phase`, and requested `kind` must match
this exhaustive matrix:

| Current `Task.mode` | Current `work_phase` | Allowed `RecordRunRequest.kind` |
|---|---|---|
| `direct` | `implementation` | `direct` |
| `work` | `implementation` | `implementation` |

Core rejects every other mode, phase, or kind before commit. Advisor results
are finalized only by `volicord.finalize_advice` from
the exact durable shaping checkpoint; work shaping results remain checkpoint
authority until `volicord.advance_task`. There is no advisor Run fallback.

A work Task still in shaping receives `TASK_PHASE_TRANSITION_REQUIRED` with
typed `WorkflowRejectionDetails` and recovery owner
`volicord.advance_task`. Other mode/phase/kind mismatches receive
`RUN_KIND_INCOMPATIBLE` with the received kind, the closed allowed-kind set,
the current tagged workflow, and the exact recovery owner. These are workflow
rejections with current typed recovery data.

Every Run also requires the verified current Git workspace context to match the
current Change Unit write basis. This applies to non-write evidence and close
assessment Runs as well as product-write Runs, so changing branch, HEAD, or
worktree cannot move Run authority to a different workspace without an explicit
`replace_current` rebaseline.

The method may also update the current close basis, update compact
target-scoped evidence coverage, record evidence observations for stable
criterion or supplemental-claim targets, consume a compatible write ticket
when recording a product write or an effective `sensitive` Task's exact
approved non-product action, link existing Evidence attachments, and promote
eligible staged attachment inputs to persistent `ArtifactRef` records where
allowed. Input-only or staged-only items are not accepted Evidence and do not
establish close readiness until this method records the target, provenance, and
any attachment link or promotion according to the evidence rules below.

## Required inputs

- A valid `ToolEnvelope`; committed `dry_run=false` requests require non-null `idempotency_key` and current `expected_state_version`.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_ticket_id`, `performed_operation`, `summary`, `observed_changes`, `artifact_inputs`, `evidence_updates`, `evidence_observations`, and `close_assessment`.
- `performed_operation` is required-nullable. JSON `null` means no performed
  operation; omission is invalid. Every effective `sensitive` non-product Run
  must supply a non-empty value that
  exactly matches the operation stored in the consumed write ticket after
  trimming outer whitespace. An ordinary product-write Run may omit it.
- Product-write Runs and all effective `sensitive` Runs require a compatible
  `status=active` write ticket from `volicord.prepare_write`. A non-sensitive
  Run with no product-file write does not.
- New artifact bytes must already be represented by a valid `StagedArtifactHandle`; `volicord.record_run` does not stage new bytes. The handle remains an Evidence attachment input until accepted in a committed run result.
- A `supported` evidence update must be backed by a target-matching
  `EvidenceObservationInput`, a usable target-matching evidence observation ref,
  or `EvidenceCoverageUpdate.provenance` from which Core can create an evidence
  observation. Request-side `source_kind` and `assurance_level` select a claimed
  provenance pair; Core derives the committed pair from verified anchors.
- Acceptance-criterion targets must identify a current criterion for this `Task`.
  Supplemental targets use a caller-assigned Task-scoped `EvidenceClaimId`; its
  statement becomes immutable on first committed use. A required criterion
  rejects `coverage_state=not_applicable`.

Before creating a Run, changing evidence or close-basis state, promoting an
artifact, or consuming a write ticket, Core rejects with
`DECISION_UNRESOLVED` when a current pending user-action request includes
`record_run` in `required_for` and its action kind, Task, current Change Unit,
`scope_revision`, basis, and affected refs match this operation. A pending
`sensitive_approval` matches only when its bounded action scope overlaps the
validated write-ticket operation, actual normalized changed paths, sensitive
categories, and baseline for this Run. Informational, resolved, stale,
superseded, expired, non-matching, and action-kind-incompatible requests do not
block Run recording.

## Request schema

This method owns the top-level `params` request fields in the generated table
below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope);
the table does not redefine `ToolEnvelope` fields. Requiredness and nullability
come directly from the semantic request descriptor.

<!-- BEGIN GENERATED: contract-structures api.method.record_run.request[params] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `RecordRunRequest` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `artifact_inputs` | yes | no | `ArtifactInput[]` |
| `baseline_ref` | yes | no | `BaselineRef` |
| `change_unit_id` | yes | no | `ChangeUnitId` |
| `close_assessment` | yes | yes | `CloseAssessmentInput` |
| `envelope` | yes | no | `ToolEnvelope` |
| `evidence_observations` | yes | no | `EvidenceObservationInput[]` |
| `evidence_updates` | yes | no | `EvidenceCoverageUpdate[]` |
| `kind` | yes | no | `RunKind` |
| `observed_changes` | yes | no | `ObservedChanges` |
| `performed_operation` | yes | yes | `string` |
| `run_id` | yes | yes | `RunId` |
| `summary` | yes | no | `string` |
| `task_id` | yes | no | `TaskId` |
| `write_ticket_id` | yes | yes | `WriteTicketId` |
<!-- END GENERATED: contract-structures api.method.record_run.request[params] -->



Nested owner links:
- `observed_changes`, `evidence_updates`, and `evidence_observations` use
  `ObservedChanges`, `EvidenceCoverageUpdate`, and `EvidenceObservationInput`;
  those shapes are owned by [API State Schemas](schema-state.md#evidence-and-run-snapshot-shapes).
- `close_assessment.result_refs` and `ResidualRiskInput.source_refs` use `StateRecordRef`, owned by [API State Schemas](schema-state.md#state-references).
- `CurrentCloseBasis` and committed `ResidualRisk` output shapes are owned by [API State Schemas](schema-state.md#close-readiness-and-validation-shapes). `ResidualRiskInput` has no caller-authoritative `risk_id`; Core generates opaque `risk_id` values when committing a new current close basis.
- `artifact_inputs` uses `ArtifactInput[]`; `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef` shapes are owned by [API Artifact Schemas](schema-artifacts.md#artifactinput).
- `kind`, artifact source values, `redaction_state`, and evidence coverage values are owned by [API Value Sets](schema-value-sets.md).

Path and access notes:
- A non-null `performed_operation` is normalized by trimming outer whitespace
  only. Core does not perform case folding, semantic matching, or substitution
  from `summary`. Whenever a ticket-consuming Run supplies the field, the
  normalized value must exactly equal the ticket's normalized
  `WriteTicketAttemptScope.intended_operation`; an effective `sensitive`
  non-product Run cannot omit it.
- `observed_changes.changed_paths` entries are `Product Repository` API product paths. Product Repository path normalization is owned by [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- `ArtifactInput[]` and staged handles do not create a second request-level operation category or actor source; the invocation remains the one in the verified invocation context.
- `ArtifactInput[]` members are Evidence attachment inputs. Their optional
  `evidence_target` uses the same tagged target identity as coverage and
  observations. They support Evidence only when this method links them to
  target-scoped evidence or observations; their presence in the request is not
  evidence sufficiency.
- `EvidenceObservationInput.source_refs` and `EvidenceUpdateProvenance.source_refs` preserve structurally validated, non-authoritative provenance. Core performs no file read, Git resolution, command execution, URI fetch, or message lookup for these refs. Optional command or Git-diff artifact refs must canonicalize to an existing artifact owned by this project and `Task`. Source refs never establish evidence sufficiency or close authority.
- `EvidenceObservationInput.observed_by_actor_source` is not authoritative input.
  Core derives the committed observer from the validated producer record when
  one exists and otherwise from the verified invocation context.
- A capture-backed observation places exactly one current
  `record_kind=evidence_capture_intent` ref in `input_refs`. It leaves
  `observed_by_actor_source`, `tool_name`, and `tool_invocation_id` null and
  leaves `tool_metadata`, `source_refs`, `output_artifact_refs`, and
  `limitations` empty. It still supplies a syntactically valid `observed_at`,
  but Core ignores that caller value for capture-backed input and replaces it,
  along with the other listed members, from stored receipt facts. Command and
  tool capture request `external_tool` / `external_tool_result`; registered
  connection capture requests `connection_observation` /
  `registered_connection_observed`.

Close-assessment ref rules:
- Caller-supplied `close_assessment.result_refs` and `ResidualRiskInput.source_refs` are restricted to `record_kind=run`, `artifact`, `evidence_summary`, or `change_unit` unless an owner explicitly adds another kind.
- The method rejects or excludes caller-supplied `project_state`, `write_ticket`, `user_action_request`, `user_action_resolution`, `blocker`, `task_event`, and `task` refs from the close basis unless an owner explicitly adds them.
- Every accepted ref must exist and belong to the same project and Task. Artifact refs must be linked to the Task and pass current-byte verification with `integrity_status=verified`; evidence refs must identify the current Task evidence summary; Run refs used as current close-basis result refs must identify a recorded current Run compatible with the current Task, current Change Unit, current scope revision, compatible baseline, and recorded status.
- Historical Run refs are audit records for close-basis purposes unless this new current Run explicitly reuses `verified` artifacts or evidence from history and records that reuse in its committed evidence or close assessment.
- Core stores canonical refs in `CurrentCloseBasis` and never treats caller-supplied `produced_at_state_version` metadata as authority or concurrency input.
- Core may add the current Run, current Change Unit, and current EvidenceSummary refs while constructing the canonical close basis.

Evidence update provenance rules:
- `coverage_state=supported` is a claim about coverage, not sufficient provenance by itself.
- When `EvidenceCoverageUpdate.provenance` is supplied for a `supported` item and
  no explicit target-matching observation input is supplied, Core creates an
  `EvidenceObservation` for the current Run and links its ref into the committed
  evidence summary.
- Request-side `source_kind` and `assurance_level` must form a valid pair, but
  the pair cannot self-assert stronger provenance. Core derives the committed
  pair as follows:
  - A canonical artifact proves byte identity and current integrity only. It
    does not prove who produced the bytes or whether they support the target.
  - `user_observation` / `user_observed` is retained only when `input_refs`
    identifies a current target-bound `evidence_observation UserActionResolution` created by
    [`volicord.resolve_user_action`](method-resolve-user-action.md), and
    its exact output artifacts match. Core rechecks the local-user actor,
    verification basis, relevance, Task, Change Unit, scope, baseline, target,
    and current bytes. It preserves the resolution's exact stored `supported`
    or `contradicted` relevance in the committed `relevance_assessment`, and it
    replaces caller-supplied `EvidenceObservationInput.observed_at` with the
    enclosing resolution's `resolved_at`. Both relevance states retain the
    local-user producer provenance; `contradicted` remains negative relevance
    and cannot establish supported coverage or evidence sufficiency.
  - A current capture intent with a complete matching receipt allows Core to
    finalize a verified command, verified tool-invocation, or registered
    connection-observation producer. Without that exact intent ref, direct
    `external_tool` and `connection_observation` requests still downgrade even
    when an attached artifact is available and integrity-verified. Descriptive
    tool fields, `SourceRef`, staging metadata, and raw guard payloads cannot
    substitute for the stored receipt.
  - Direct caller-supplied `reused_evidence` is not a validated reuse path.
  - An unproved strong claim is committed as `agent_report` /
    `cooperative_report`; `unverified_claim` / `unverified` remains unverified.
- When a `supported` update relies only on strong, usable target-matching
  `observation_refs`, Core records a current-Run `source_kind=reused_evidence`
  observation. Its single `input_ref` retains the original observation ref, so the
  historical observation is a provenance input rather than being relabeled as
  current. The reuse observation carries the original observation's exact
  canonical artifact outputs and a reuse limitation. A current update cannot
  substitute different bytes into the inherited producer chain.
- Before creating that reuse observation, Core revalidates the original
  observation identity and target, Task and Change Unit ownership, source Run,
  current scope revision and baseline, inherited assurance, producer anchor,
  exact outputs, and separate relevance assessment. Every recursive hop
  strict-decodes its persisted authority metadata and must lead to the same
  current anchored assurance; a stale, missing, contradicted, corrupt,
  mismatched, output-substituted, or cyclic chain is rejected.
- The `contradicted` entry in that rejection list is a supported-reuse rule,
  not a producer-provenance downgrade. A current exact User Channel observation
  with `contradicted` relevance remains `user_observation` / `user_observed`,
  but it cannot qualify for validated reuse that establishes `supported`.
- For every strong `user_observation` and validated-reuse check above, an exact
  output set is non-empty and contains pairwise-distinct `artifact_id` values;
  Core rejects duplicate artifact ids instead of deduplicating them. Each
  historical output must match the current canonical typed `ArtifactRef` in
  `artifact_id`, `project_id`, `task_id`, `display_name`, `content_type`,
  `sha256`, `size_bytes`, `integrity_status`, `redaction_state`, `availability`,
  `created_by_run_ref` presence and identity (`record_kind`, `record_id`,
  `project_id`, and `task_id`), `created_by_actor_source`, and `storage_ref`.
  The sole allowed normalization when comparing a historical ref with the
  current canonical projection is rebasing the nested
  `created_by_run_ref.produced_at_state_version`; that field is projection
  freshness only and grants neither authority nor concurrency. No other typed
  field may be ignored, rebased, or substituted. A duplicate or mismatch
  rejects the request before commit; `dry_run` performs the same validation and
  neither branch records strong provenance or another effect.
- For non-supported states, current target-matching cooperative or unverified
  observation refs may be retained as descriptive support; the strong-reuse
  requirement applies only when refs are used to establish `supported`.
- Committed `source_kind` and `assurance_level` contain Core's derived
  provenance class, not a caller assurance grant.
- `unverified_claim`, `unverified`, and cooperative `agent_report` observations may be recorded as evidence observations, but close readiness evaluates them as weak provenance when stronger provenance is required.
- Evidence observations do not replace user-owned judgment, final acceptance, residual-risk acceptance, or close readiness.

Capture-backed observation rules:

- Core loads the intent and its one immutable receipt directly, revalidates the
  current project, Task, Change Unit, scope revision, baseline, target,
  workspace, connection/actor, expiry, exact digests, receipt bytes,
  completeness, and redaction state, and rejects a missing, stale, expired,
  already consumed, cross-scope, or corrupt intent or receipt before commit.
- Core automatically promotes the bounded safe receipt staging handle, links
  the resulting artifact to the new `EvidenceProducer`, creates that producer
  and its one-to-one `EvidenceObservation`, and records the stored source facts
  atomically with the Run. Caller-supplied output refs or metadata cannot
  replace those facts.
- An observed outcome equal to the intent expectation yields strong producer
  provenance and `relevance_assessment.status=unassessed`. Registered execution
  or observation does not decide that the result supports the selected target,
  so the capture-backed observation alone cannot make a required criterion
  sufficient. A separate owner-defined relevance authority is required for
  `supported`. A complete outcome that mismatches the stored expectation is preserved as
  `contradicted`; it is not silently changed to a cooperative success claim.
  In both capture classifications, `assessment_ref` identifies the immutable
  capture intent as the classification basis and
  `assessed_by_actor_source=null`; that ref is not an independent relevance
  authority and cannot turn `unassessed` into `supported`.
- Inputs without a capture-intent ref retain the existing unanchored downgrade
  and validated-reuse rules.

## Access requirements

Requires:

- verified invocation context with `operation_category=agent_workflow`

For `ArtifactInput.source_kind=staged_artifact`:

- the current verified `actor_source` must match the staged handle's recorded provenance

The recorded provenance was captured from the verified invocation context at staging time. This method compares it with the current verified context instead of accepting caller-submitted provenance as authority.

Non-claims:

- `ArtifactInput[]` does not add `artifact_registration`.
- Cross-actor staged artifact transfer is outside the baseline scope.

## State version behavior

A compatible committed result increments `project_state.state_version` exactly once.

A compatible committed result increments the selected `Task.close_basis_revision` exactly once. When `close_assessment` is non-null, the commit establishes a new `CurrentCloseBasis` from the committed current Run, the assessment fields, generated residual-risk IDs, current Task, current Change Unit, selected current scope revision, and compatible baseline. When `close_assessment=null`, the committed Run explicitly does not establish a current close basis, and any existing current close basis becomes stale or absent.

An empty `close_assessment.residual_risks` list explicitly means the current result has no identified residual risks. Core generates opaque `risk_id` values only for committed non-null assessments. A `dry_run` never reserves persistent `risk_id` values.

Sensitive action requirements in the resulting `CurrentCloseBasis` are derived by Core from the committed Run and any consumed write ticket. Category-only caller input in `close_assessment.sensitive_categories` can contribute display context but cannot establish, satisfy, or erase a sensitive approval requirement.

An effective `sensitive` Task requires a compatible ticket even when the Run
records no product-file write. That ticket is the exact action and approval
basis: it must have `product_file_write_intended=false`, match the Run's empty
product-path observation, and carry a current user-owned sensitive approval.
The matching Run consumes it and preserves its operation, Change Unit, scope,
baseline, and approval-bound sensitive-action requirement through close.
Ordinary non-sensitive Runs with no product-file write still require no ticket.

Category-only `observed_changes.sensitive_categories` is a caller report rather than a Core-confirmed approval basis. It does not by itself raise the Task's effective control level or create sensitive-action approval authority. It does atomically strengthen the Task's acceptance policy to `required`, so policy-dependent `light` auto-close cannot consume the signal and current final acceptance remains mandatory. A Core-confirmed `sensitive` control basis still requires both matching user approval and final acceptance; category-only input can provide neither.

Recording a successful Run, its close assessment, or later final acceptance
does not repair a missing pre-write approval and does not retroactively
authorize a write. When the current policy requires `sensitive` control and
sensitive-action approval, `record_run` requires the already approved, currently
policy-bound ticket before it creates the Run.

The Run, current close basis, evidence updates, evidence observations, artifact links or promotions, write-ticket consumption, and revision changes are committed atomically when the result commits.

Ticket-backed recording consumes the write ticket only when:

- the ticket has `status=active` and has not already been consumed or revoked
- its `WriteTicketValidityBasis` still matches the current `task_id`,
  `change_unit_id`, `scope_revision`, baseline, and workspace digest; its
  Store-valid approval basis receives a current or not-required result from the
  canonical typed approval assessment. Admission loads raw UserAction
  authority locally, sends it directly to the approval owner, and does not
  retain it in the current-validity facts
- its non-null `write_authority_fingerprint` exactly matches the fingerprint
  independently reloaded from the current authoritative project policy; the
  Store rechecks the same binding inside the ticket-consumption transaction
- it has not crossed a project-policy-selected optional `idle_expires_at`; the
  default idle timeout is `null`
- the ticket and its `WriteTicketAttemptScope` identify the same `task_id` and `change_unit_id` as the Run being recorded
- the checked `WriteTicketAttemptScope`'s `product_file_write_intended` exactly matches whether
  the Run observed a product-file write; an effective `sensitive` non-product
  Run uses `false`
- the checked `WriteTicketAttemptScope` `baseline_ref` matches the Run
  `baseline_ref`
- a supplied `performed_operation` exactly matches the checked attempt's
  normalized `intended_operation`; the field is mandatory for an effective
  `sensitive` non-product Run
- the verified current Git workspace context still exactly matches the current
  Change Unit write basis captured when the ticket was issued; a branch, HEAD,
  worktree, or fingerprint change after issuance rejects consumption
- observed sensitive categories match the checked attempt's normalized `sensitive_categories`
- for a product-file write, observed changed paths after Product Repository
  path normalization are compatible with the checked attempt; a non-product
  sensitive Run records no product changed paths

A ticket remains valid across unrelated `state_version` changes, including
status or close checks, evidence recording, diagnostics, operation-result
retrieval, unrelated user actions, and committed non-allow prepare-write
decisions. `WriteTicket.basis_state_version` records issuance order only.

An expired or otherwise no-longer-current well-formed approval produces the
semantic `approval_basis_changed` outcome. Persisted approval-reference owner
disagreement, missing required reference metadata, or duplicate full
resolution identity is Store corruption and cannot reach this admission
policy. The assessment distinguishes approval newly required, no current
resolution, changed approval scope, and a persisted basis resolution that is no
longer current. The canonical owner returns that typed assessment without
exposing raw UserAction authority facts to Record Run admission, which does not
reconstruct or compare approval reference identities independently.

The method rejects stale `expected_state_version` according to normal request
conflict precedence. It independently validates the ticket basis; a different
global state version never produces `STATE_VERSION_CONFLICT` for the ticket.

When an optional idle boundary is configured it is calculated from parsed UTC
timestamps, not lexical strings. A ticket invalidated by that boundary is never
consumed and returns `WRITE_TICKET_INVALID` with
`ToolError.details.write_ticket_reason=idle_timeout`.

Persisted invalidation uses `WRITE_TICKET_INVALID` with the stored reason:
`scope_revision_changed`, `change_unit_changed`, `baseline_changed`,
`workspace_changed`, `approval_basis_changed`, `idle_timeout`, `task_closed`,
or `explicit_revoke`. Attempt compatibility mismatch uses method-local detail
values such as `task_mismatch`, `change_unit_mismatch`,
`product_write_flag_mismatch`, `baseline_mismatch`,
`operation_mismatch`, `workspace_context_mismatch`, `sensitive_category_mismatch`, or
`path_mismatch`.

## Method result fields

`RecordRunResult` is the method-specific result branch for a committed run-recording operation. It carries `base: RecordRunResultBase`, whose only result effect is `core_committed`, and these method-owned top-level fields:

<!-- BEGIN GENERATED: contract-structures api.method.record_run.response[response_variants] api.method.record_run.response[result_body] api.method.record_run.response[result_metadata] api.method.record_run.response[rejection] api.method.record_run.response[dry_run] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `RecordRunResult` success fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `base` | yes | no | `RecordRunResultBase` |
| `blocker_refs` | yes | no | `StateRecordRef[]` |
| `current_close_basis` | no | yes | `CurrentCloseBasis` |
| `evidence_observations` | yes | no | `EvidenceObservation[]` |
| `evidence_producers` | yes | no | `EvidenceProducer[]` |
| `evidence_summary` | no | yes | `EvidenceSummary` |
| `registered_artifacts` | yes | no | `ArtifactRef[]` |
| `run_summary` | yes | no | `RunSummary` |
| `state` | yes | no | `StateSummary` |

### `Result Metadata: core_committed` fields

Contract: `dry_run` is `false`; `events` contains at least one event (`minItems: 1`).

| Field | Required | Nullable | Type |
|---|---|---|---|
| `disclosure` | yes | no | `GuaranteeDisclosure` |
| `dry_run` | yes | no | `boolean enum(false)` |
| `effect_kind` | yes | no | `string enum("core_committed")` |
| `events` | yes | no | `NonEmptyEventRefs` |
| `response_kind` | yes | no | `string enum("result")` |
| `state_version` | yes | no | `integer` |

### `dry_run` request policy

- `volicord.record_run`: `dry_run=true` selects the `ToolDryRunResponse` preview branch, whose `base.dry_run` is `true`. `dry_run=false` or an omitted `dry_run` does not select a preview branch.


### Shared response structures

The response descriptor defines success, rejection, and preview as an exact `anyOf` branch union. The rejection branch uses the generated [`ToolRejectedResponse`](schema-core.md#common-response) structure. When method behavior selects a preview branch, it uses the generated [`ToolDryRunResponse`](schema-core.md#common-response) structure. Shared rejection and preview fields remain distinct from the success fields above.
<!-- END GENERATED: contract-structures api.method.record_run.response[response_variants] api.method.record_run.response[result_body] api.method.record_run.response[result_metadata] api.method.record_run.response[rejection] api.method.record_run.response[dry_run] -->

The MCP compact result preserves `evidence_producer_refs` alongside
`evidence_observation_refs`; full detail carries the exact producer bodies.
If response budgeting omits full detail, the durable operation-result path
recovers the exact `RecordRunResult`.

Nested `StateRecordRef`, `RunSummary`, `ObservedChanges`, `EvidenceSummary`, `EvidenceCoverageItem`, `EvidenceObservation`, `EvidenceProducer`, `StateSummary`, and `ArtifactRef` field bodies stay with the schema owners linked above. Exact persistence effects, including staged-handle consumption, artifact promotion, evidence updates, evidence observation records, replay rows, and write-ticket consumption, stay with [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

## Success result

The committed `RecordRunResult` uses `base.response_kind=result` and
`base.effect_kind=core_committed`. Artifact presence alone does not establish
evidence sufficiency, and a non-null close basis means this Run established the
current close basis.

## Blocked result

The method may commit compatible run-related blocker state when the run is recordable but the result creates or preserves blockers, such as evidence gaps.

Not allowed:

- A committed blocked result must not hide invalid staged handles, missing or
  invalidated write tickets, stale request state, or invocation-context failures.

Those failures are rejected before commit.

## Rejected result

Returns `ToolRejectedResponse` for:

- `TASK_PHASE_TRANSITION_REQUIRED` when a work Task is still in shaping
- `RUN_KIND_INCOMPATIBLE` for every other `kind`, mode, or phase mismatch
- stale `expected_state_version`
- invalidated write-ticket validity basis
- missing or mismatched write-ticket policy-authority binding
- stale or mismatched current Git workspace context
- missing or invalid write ticket for product writes
- write ticket invalidated by optional idle timeout
- incompatible write-ticket operation, path, baseline, product-write flag, sensitivity category, Task, or Change Unit
- invalid staged handle
- incompatible staged-handle provenance
- missing, expired, already consumed, stale, cross-scope, or corrupt
  evidence-capture intent or receipt
- capture-intent/receipt source, digest, bytes, completeness, redaction,
  outcome, target, or connection mismatch
- `supported` evidence update without required observation provenance
- missing artifact
- scope violation
- baseline staleness
- actor-source or operation-category mismatch
- unsupported invocation context
- validator failure

Non-claim: invalid staged handles are validation failures with artifact-input details owned by [API error details](error-details.md#artifact-input-error-reason), not invocation-context mismatch unless the request invocation itself failed.

Public error code meaning, precedence, details, and rejected-response routing are owned by the error documents linked below.

For an invalidated write-ticket basis, rejection happens before consumption and creates no Run, evidence update, evidence observation, artifact link, artifact promotion, event, replay row, or `project_state.state_version` increment.

An otherwise active ticket whose policy-authority binding is missing or
mismatched returns
`WRITE_TICKET_INVALID` with
`ToolError.details.write_ticket_reason=policy_authority_mismatch`; it does not
consume the ticket or record an authorized Run. This check does not rely on
Guard having observed or denied the write. Normal policy application first
persists `status=invalidated,invalidation_reason=explicit_revoke`, so a later
attempt against that already-invalidated row reports `explicit_revoke` by
status precedence.

A missing required or mismatched `performed_operation` likewise rejects before
ticket consumption and creates none of those effects.

For an idle-timeout-invalidated write ticket, rejection happens before consumption and creates no Run, event, replay row, artifact promotion, evidence update, evidence observation, write-ticket consumption, or `project_state.state_version` increment.

Mode, phase, or Run-kind workflow rejection likewise creates no Run, close-basis revision, evidence update, evidence observation, artifact link or promotion, event, replay row, write-ticket effect, or state-version increment.

## `dry_run` behavior

For `dry_run=true`, a valid preview:

- returns `ToolDryRunResponse`
- creates no Run, current close basis, residual-risk IDs, evidence update, evidence observation, blocker update, artifact link, artifact promotion, or write-ticket consumption

## Storage effect

On commit, the method may persist run, current close-basis, evidence summary,
evidence observation, blocker, write-ticket consumption, and artifact-linking
results. A capture-backed observation also promotes the receipt artifact and
creates its immutable producer in the same transaction. Exact storage effects
and artifact promotion details are owned by the storage documents linked below.

The examples are intentionally compact and method-local. The representative response is abbreviated to the fields needed to show the committed run, promoted artifact ref, updated evidence summary, evidence observation, blocker refs, state version, and current state snapshot.


## Minimal valid request

This example records validation output from a method-local staged handle. Method-local precondition: `staged_runprobe_001` is unexpired, unconsumed, and belongs to `proj_runprobe_001` / `task_runprobe_001`; its recorded actor provenance, captured at staging time, is `agent_connection:conn_run_probe`. The target-linked staged artifact establishes byte integrity, but the request supplies no capture-intent producer anchor, so the requested external-tool classification is committed as a cooperative agent report. The request leaves `observed_by_actor_source=null`; the response shows the actor source derived from the verified invocation. The precondition is local to this document and does not reuse any other method example.

```yaml contract=api.method.record_run.request shape=complete_request
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
  performed_operation: null
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
        expires_at: "2030-01-01T00:00:00Z"
        consumed: false
      existing_artifact_ref: null
      relation_hint: "validation_report"
      evidence_target:
        target_kind: acceptance_criterion
        acceptance_criterion_id: criterion_runprobe_count_001
      expected_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
      expected_size_bytes: 96
      redaction_state: none
  evidence_updates:
    - target:
        target_kind: acceptance_criterion
        acceptance_criterion_id: criterion_runprobe_count_001
      coverage_state: supported
      supporting_run_refs: []
      observation_refs: []
      supporting_artifact_refs: []
      gap_refs: []
  evidence_observations:
    - target:
        target_kind: acceptance_criterion
        acceptance_criterion_id: criterion_runprobe_count_001
      source_kind: external_tool
      assurance_level: external_tool_result
      observed_by_actor_source: null
      tool_name: "search-count-validator"
      tool_invocation_id: null
      tool_metadata:
        validator: "search-count"
      input_refs: []
      source_refs: []
      output_artifact_refs: []
      limitations: []
      observed_at: "2026-07-28T11:59:00Z"
  close_assessment:
    result_summary: "Search-result count validation passed."
    result_refs: []
    residual_risks: []
    sensitive_categories: []
    recovery_constraints: []
```

## Representative response

Result branch (`RecordRunResult`, committed):

```schema
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
    produced_at_state_version: 32
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
        produced_at_state_version: 32
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
      produced_at_state_version: 32
    created_by_actor_source: agent_connection:conn_run_probe
    storage_ref: "artifact-storage://search-result-count-validation"
evidence_summary:
  evidence_state: accepted_for_close
  status: sufficient
  coverage_items:
    - target:
        target_kind: acceptance_criterion
        acceptance_criterion_id: criterion_runprobe_count_001
      coverage_state: supported
      supporting_run_refs:
        - record_kind: run
          record_id: run_runprobe_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          produced_at_state_version: 32
      observation_refs:
        - record_kind: evidence_observation
          record_id: evidence_observation_runprobe_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          produced_at_state_version: 32
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
            produced_at_state_version: 32
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
        produced_at_state_version: 32
      created_by_actor_source: agent_connection:conn_run_probe
      storage_ref: "artifact-storage://search-result-count-validation"
  observation_refs:
    - record_kind: evidence_observation
      record_id: evidence_observation_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
  updated_by_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
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
      produced_at_state_version: 32
    target:
      target_kind: acceptance_criterion
      acceptance_criterion_id: criterion_runprobe_count_001
    source_kind: agent_report
    assurance_level: cooperative_report
    observed_by_actor_source: agent_connection:conn_run_probe
    tool_name: "search-count-validator"
    tool_invocation_id: null
    tool_metadata:
      validator: "search-count"
    input_refs: []
    source_refs: []
    output_artifact_refs:
      - &runprobe_output
        artifact_id: artifact_runprobe_report_001
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
          produced_at_state_version: 32
        created_by_actor_source: agent_connection:conn_run_probe
        storage_ref: "artifact-storage://search-result-count-validation"
    producer_anchor:
      producer_kind: unverified_caller
      producer_ref: null
      output_artifact_refs:
        - *runprobe_output
      verification_basis: null
    relevance_assessment:
      status: unassessed
      assessment_ref: null
      assessed_by_actor_source: null
    limitations: []
    observed_at: "<example-observed-at>"
    recorded_at: "<example-recorded-at>"
evidence_producers: []
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
      produced_at_state_version: 32
    - record_kind: change_unit
      record_id: cu_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
    - record_kind: evidence_summary
      record_id: evidence_summary_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
  evidence_summary_ref:
    record_kind: evidence_summary
    record_id: evidence_summary_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
  residual_risks: []
  sensitive_categories: []
  sensitive_action_requirements: []
  recovery_constraints: []
  source_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
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
    produced_at_state_version: 32
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
    - acceptance_criterion_id: criterion_runprobe_count_001
      statement: "Search results show the expected count."
      evidence_requirement: required
  autonomy_boundary: "Stay within validation recording for search-result counts."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
  baseline_ref: baseline_runprobe_001
  workflow: {kind: implementation, next_actor: agent, required_action: null, allowed_actions: [volicord.update_scope, volicord.prepare_write, volicord.record_run, volicord.check_close], required_refs: [], expected_state_version: 32, blocking_reason: null, checkpoint: null}
  pending_user_action_summaries: []
  blocker_refs: []
  write_ticket_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
```

## Owner links

- Request envelope, response branches, and `dry_run` summaries: [API Schema Core](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `EvidenceObservation`, `CurrentCloseBasis`, `ResidualRisk`, `StateSummary`, and refs: [API State Schemas](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- Write-ticket and close-relevant evidence boundaries: [Core Model](../core-model.md).
- Product Repository path normalization: [Runtime Boundaries](../runtime-boundaries.md#product-repository-api-path-normalization).
- Supported values and operation categories: [API Value Sets](schema-value-sets.md#operation-category-values).
- Public errors, precedence, response routing, and artifact-input detail values: [API error codes](error-codes.md), [API error precedence](error-precedence.md), [API error routing](error-routing.md), and [artifact-input error details](error-details.md#artifact-input-error-reason).
- Persistence effects and artifact promotion: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).
