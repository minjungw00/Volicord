# Storage Effects

This document defines baseline method-to-storage effect semantics.

## Owns / Does not own

Exact identifiers used in this section: `project_state.state_version`, `state_version`.

This document owns:

- read-only, `dry_run`, rejected, staging-created, Core-committed, and committed-blocked storage-effect distinctions
- whether a method branch creates replay rows, `authority_events`, record changes, state-version increments, staged-handle creation or consumption, artifact promotion, or write-ticket compatibility changes
- the persistence boundary for blocker-like response data
- no-effect guarantees for rejected branches and valid dry-run preview branches

This document does not own:

- record-family overview; see [Storage Records](storage-records.md)
- baseline SQLite DDL, constraints, indexes, foreign keys, or canonical SQL sources; see [Storage DDL](storage-ddl.md)
- artifact lifecycle details; see [Artifact Storage](storage-artifacts.md)
- idempotency, locks, state-version clocks, event ordering, or incompatible-storage handling; see [Storage Versioning](storage-versioning.md)
- public response branches or schemas; see [API Schema Core](api/schema-core.md)
- API method behavior; see the [API Methods](api/methods.md) and method owner documents
- public error code precedence; see [API error precedence](api/error-precedence.md)

## Shape versus effect

Response data shape and storage effect are separate.

API data shapes belong to API schema owners, including [API State Schemas](api/schema-state.md) for blocker-like state shapes and [API Artifact Schemas](api/schema-artifacts.md) for artifact shapes. Examples include:
- `CloseReadinessBlocker`
- `WriteDecisionReason`
- `PlannedBlocker`
- `ArtifactRef`
- `StagedArtifactHandle`

Non-claim: their presence in a response does not by itself prove persistence, artifact promotion, staged-handle consumption, replay storage, close-state mutation, or `project_state.state_version` increment.

Effects come from the selected method behavior and response branch. The table summarizes each branch; the detail blocks separate allowed effects from forbidden effects.

| Effect category | Response or branch | Durable storage consequence | Details |
|---|---|---|---|
| Read-only | Read-only `MethodResult` | No Core authority-state mutation. Response data only; no replay row, authority event, artifact effect, write-ticket effect, close-state mutation, or `project_state.state_version` increment. | [Read-only result](#read-only-result) |
| No-effect | `ToolRejectedResponse` or a valid `MethodResult` with `effect_kind=no_effect` | No ordinary requested mutation and no Core commit. The response may carry errors or blocker-shaped data, but those values are not persisted by this branch. | [`ToolRejectedResponse`](#toolrejectedresponse-effect), [No-effect branches](#no-effect-branches) |
| `dry_run` | Valid `ToolDryRunResponse` | Preview only; no persistent refs, replay row, event, staged handle, artifact effect, or `project_state.state_version` increment. | [Valid dry-run preview](#valid-dry-run-preview) |
| Staging-created | `StageArtifactResult` with `effect_kind=staging_created` | Storage-owned transient staging plus an atomic non-decreasing advance of the persisted canonical-UTC floor; not the regular Core commit transaction. | [Staging-created artifact result](#staging-created-artifact-result) |
| Core commit | Core committed `MethodResult` | Method-owned effects through `CoreProjectStore::commit_mutation`, including the state-version increment, authority event, optional replay row, method-selected `CoreStorageMutation` values, and one canonical commit timestamp. | [Core committed result](#core-committed-result) |
| Committed blocker-shaped result | Committed `MethodResult` whose method owner allows blocked or non-allow persistence | Only the explicitly allowed event, replay, state-version, and blocker-state effects. A blocker-shaped response alone is not enough. | [Committed blocked result](#committed-blocked-result) |

Exact replay, rejected requests, valid dry runs, and read-only results do not
update the persisted canonical Core UTC floor. Storage-owned staging, registered
evidence-capture receipt fulfillment, and local User Channel token issuance are
the floor-only exceptions defined below. [Storage Versioning](storage-versioning.md#canonical-core-utc-clock)
owns the complete clock contract.

<a id="read-only-result"></a>
### Read-only result

Storage effect:

- No Core authority-state storage effect.
- Response data only.

Disallowed effects:

- replay row
- authority event
- Core current-row mutation
- close-state mutation
- artifact effect
- evidence update or evidence observation
- write-ticket effect
- `project_state.state_version` increment
- persisted canonical-UTC floor update

<a id="toolrejectedresponse-effect"></a>
### `ToolRejectedResponse`

Storage effect:

- None.

Disallowed effects:

- current row
- replay row
- event
- artifact effect
- write-ticket creation or consumption
- `project_state.state_version` increment
- persisted canonical-UTC floor update

<a id="valid-dry-run-preview"></a>
### Valid `dry_run` preview

Storage effect:

- Response preview only.

Disallowed effects:

- current row
- generated persistent ref
- replay row
- event
- staged-handle creation
- artifact promotion or link
- `project_state.state_version` increment
- persisted canonical-UTC floor update

<a id="staging-created-artifact-result"></a>
### Staging-created artifact result

Allowed effect:

- storage-owned transient staging
- an atomic `project_state.updated_at >= artifact_staging.created_at` floor
  update

This branch is separate from a regular Core committed mutation. It may create a storage-owned staged representation or handle, but that transient staging write is not a Core current-row mutation, persistent `ArtifactRef`, artifact link, or evidence record by itself.

Disallowed effects:

- Core authority current row other than the physical project-time floor
- replay row
- event
- persistent `ArtifactRef`
- `project_state.state_version` increment

<a id="core-committed-result"></a>
### Core committed result

Condition:

- The method owner allows the committed effect.

Allowed effects:

- current-row mutation
- `authority_events` append
- replay row creation
- exactly one `project_state.state_version` increment

The commit selects one canonical `committed_at` that is no earlier than the
prepared operation-time sample. It writes that exact value to
`project_state.updated_at`, every appended `authority_events.created_at`, the
optional replay-row `tool_invocations.created_at`, and Store transaction
metadata such as applicable `created_at`, `updated_at`, `retired_at`, and
`promoted_at` values that mutation application generates. Semantic operation
times such as `requested_at`, `resolved_at`, `closed_at`, `recorded_at`, and
`consumed_at`, and input- or observation-owned facts such as `observed_at` and
`started_at`, keep their owner-defined operation sample or validated source
meaning and are not rewritten to the commit timestamp.

Artifact promotion and `artifact_links` creation occur only when the method owner selects a committed mutation branch that explicitly includes those artifact effects. They do not follow automatically from earlier staging.

<a id="committed-blocked-result"></a>
### Committed blocked result

Condition:

- The method owner allows the blocked commit.

Allowed effects:

- explicitly allowed blocker-state effect
- explicitly allowed event effect
- explicitly allowed replay-row effect
- explicitly allowed `project_state.state_version` effect

Disallowed effects:

- creating the missing authority that the branch reports

<a id="no-effect-branches"></a>
## No-effect branches

No-effect branches include rejected responses and valid method results where the
method selected no durable mutation for the requested operation.

These failures return no-effect branches:

- malformed requests
- validation failures before commit
- `runtime_home.mutation.setup_in_progress` before a supported writer begins
- connection routing or mode-gating failures before a protected operation can proceed
- stale `expected_state_version`
- invalid, consumed, revoked, or state-bound incompatible write ticket on a consuming attempt
- idempotency request-hash conflicts
- rejected artifact inputs

No-effect branches must not:

- create current rows
- append `authority_events`
- write `tool_invocations.response_json`
- create replay rows
- update evidence summaries or create evidence observations
- mutate close state
- create or consume write-ticket compatibility rows
- change `artifact_staging.status`
- set `consumed_by_run_id` or `promoted_artifact_id`
- promote or link artifacts
- increment `project_state.state_version`

Setup-busy applies to CLI, MCP lifecycle and tool observations, Core commits,
Guard persistence, diagnostics, operational sessions, integration
verification, evidence capture, and artifact staging. It creates no database
row, artifact byte, receipt, checkpoint, or observation. After the exclusive
setup lease is released, a new admitted operation follows its ordinary effect
contract.

When preflight returns `ToolRejectedResponse`, the requested committed operation does not proceed. This principle applies to `dry_run` requests too. `dry_run` does not bypass validation, access, capability, or stale-state rejection.

A valid blocked method result can also be no-effect when the method owner
selects a response-only blocked branch. For example, a baseline `volicord.close_task`
blocked terminal attempt returns `CloseTaskResult` data without committing
blocker rows, authority events, replay rows, or a state-version increment. This is
separate from committed non-allow `volicord.prepare_write` results.

## `dry_run` preview effects

Valid dry-run previews may include `DryRunSummary.would_blockers: PlannedBlocker[]` or planned effects. Those preview entries do not create:

- `authority_events` append
- replay row or `tool_invocations.response_json`
- generated persistent ref
- `close_state` mutation
- write-ticket change
- staged-handle creation or consumption
- artifact effect
- evidence update or evidence observation
- `CloseReadinessBlocker` storage
- `project_state.state_version` increment

## Read-only effects

Read-only results have no Core authority-state storage effect, are not replay
rows, and are response-only.

For response computation, `volicord.status` and `volicord.check_close` may compute `CurrentCloseBasis`, close state, risk acceptance coverage, blockers, `CloseReadinessBlocker[]`, evidence summaries, artifact refs, project continuity summaries, diagnostics, and next actions for the response when the method owner selects those projections.

Storage must not persist those computed values merely because the read occurred.

Read-time projections must distinguish uncomputed, `unavailable`, empty, and verified state. Storage must not write empty arrays, empty hashes, zero sizes, invented content types, or stronger guarantee displays merely because a read path could not compute the underlying facts.

Read-time artifact checks may compute an effective missing, unavailable, or integrity-failed state for evidence, close, or status output when the current body cannot be verified against stored facts. That response computation does not mutate `artifacts.status`, `artifacts.integrity_status`, artifact links, or stored lifecycle rows unless a separate owner-defined mutation occurs.

`volicord.status` with `close_blockers: CloseReadinessBlocker[]` is a read-only observation. It does not create:

- `authority_events` append
- replay row or `tool_invocations.response_json`
- `close_state` mutation
- write-ticket change
- staged-handle consumption
- artifact effect
- evidence update or evidence observation
- `project_state.state_version` increment

For `volicord.check_close`, the response branch is owned by [`volicord.check_close`](api/method-close-task.md#volicordcheck_close). This storage page asserts that the check remains read-only for Core authority state and `project_state.state_version`, including with `dry_run=true` and with `blockers: CloseReadinessBlocker[]`.

## Committed blocked effects

Committed blocker-shaped outcomes are distinct from rejected responses and from
response-only blocked results.

Condition: a committed blocked or non-allow outcome is a `MethodResult` only
when the relevant method owner selects a committed branch for that outcome.

Owner links:
- [Prepare-write method](api/method-prepare-write.md)

<a id="volicordprepare_write-committed-non-allow-decision"></a>
### `volicord.prepare_write` committed non-allow decision

Conditions:

- The call is committed with `dry_run=false`.
- The result is `decision=blocked`, `decision=approval_required`, or `decision=decision_required`.

Allowed effects:

- append exactly one `authority_events` event containing the structured `write_decision_reasons: WriteDecisionReason[]`
- create a replay row when an idempotency key is present
- increment `project_state.state_version` exactly once
- record the method-owned decision and `write_decision_reasons` in the response and replay payload

Disallowed effects:

- issuing a write ticket
- creating a separate public history method
- adding a new public response field for historical non-allow decisions
- requiring `volicord.status` to expose historical non-allow decisions
- changing `close_state`
- evaluating close readiness
- storing `CloseReadinessBlocker`
- updating evidence or recording evidence observations
- changing artifacts
- consuming staged handles
- applying `close_task` effects

Persistence boundary:

- Request-side `volicord.prepare_write` payload fields belong to the [`volicord.prepare_write` reference](api/method-prepare-write.md).
- Stored `write_decision_reasons` remain `volicord.prepare_write` decision reasons.
- The durable local audit location for a valid committed non-allow decision is the committed authority event and, when keyed, the replay row.

Those stored reasons are not:

- close-readiness blockers
- `CloseReadinessBlocker[]`
- close-readiness blocker records

Project-policy application atomically writes the authoritative
`project_workflow_policies` canonical copy, monotonic version, canonical JSON,
fingerprint, and source. It also derives the normalized write-authority
fingerprint. When that fingerprint changes, the same transaction invalidates
with `explicit_revoke` every active ticket with a missing or different stored
binding, creates or updates the active Task's reevaluation mark even when its
current control and acceptance levels already satisfy the new minimums, and
also invalidates with `explicit_revoke` every active ticket for that marked
Task. It never silently
lowers effective control. A canonically identical policy, or a changed policy
whose normalized write-authority fingerprint is unchanged, has no ticket
invalidation effect solely because policy apply ran. Exact command, file, and
host behavior is an administrative-owner concern.

Workflow metrics writes store aggregate counters, durations, serialized tool
byte counts, and categorical outcomes only. These records never contain prompt,
file, answer, or command bodies.

## Administrative Connection Setup And Verification

An accepted setup, repair, or staged managed-configuration apply writes the
host configuration before its setup-owned Store path commits the resulting
`managed_fingerprint`. If an existing Connection's fingerprint changes, that
same immediate Registry transaction clears `verification_report_json`.
Compatible replay with the same fingerprint may retain the report. A host
write that succeeds before a later unrelated failure remains subject to the
administrative CLI's partial-application reporting and retry contract.

Active connection verification captures the exact typed Connection integration
revision that it verifies. Report persistence uses one immediate Registry
transaction to compare the current revision and, only on an exact match,
replace `verification_report_json` and the ordinary row update timestamp. It
does not write `managed_fingerprint`, integration instance or generation,
mode, metadata, memberships, Guard manifests, runtime sessions, or project
Agent Sessions. A revision conflict has no Registry effect.
`volicord connection status` remains read-only and does not use this mutation.

Before conformance, active verification probes Registry and selected project
writeability with bounded SQLite transactions that always roll back. The probe
may acquire write locks and therefore is an active storage operation, but it
does not retain schema objects or rows. Protocol and host-compatibility
conformance creates `manual_cli` sessions and possible findings only in a fresh
disposable per-command Runtime Home; disposal removes that fixture. It creates
no conformance session or finding in the selected user Runtime Home.

These active write results are stored only in
`last_active_verification.registry_write` and
`last_active_verification.project_writes` in the replacement report. They do
not update `preflight.evidence.writeability`, whose only state is
`not_checked`, and they do not add a side effect to preflight. The active
evidence records rollback-only Registry and project probes and disposable
conformance through its own closed side-effect values.

`volicord mcp preflight` opens the selected Registry and project databases
read-only. It does not probe writeability, create or update a runtime session,
persist a finding, reconcile diagnostics, or write either the Runtime Home or
Product Repository. Its JSON `side_effects` is therefore an empty array.

<a id="connection-integration-verification-effects"></a>
## Connection-Integration Verification Effects

`volicord.begin_integration_verification` validates the current managed runtime,
native session/turn, selected Connection Project, current Agent Session, Guard
Installation, policy, revision, hook contract, and prompt observation before
one immediate Registry transaction. It returns the existing row for an exact
semantic coordinate, including a terminal row, or inserts one new
`guard_integration_verification_runs` row for a genuinely new eligible
coordinate. If a prior nonterminal coordinate was superseded, begin records
typed terminal repair before applying its retry policy. Cleanup removes only
stale retained records and never creates a new ID. Rejected, manual, preflight,
ambiguous, prompt-missing, or retry-ineligible calls have no new-run effect.

`volicord.guard_probe` uses one immediate Registry transaction. It loads the
exact run, validates the complete caller coordinate, computes current effective
status, and returns an existing `probe_acknowledged_at` without updating. When
the field is absent, only an eligible active run may conditionally set it; the
Store then records `probe_acknowledged` and, if no pre-tool acquisition has
arrived, `hook_event_not_observed` in `guard_probe_observations`, and reads back
the authoritative timestamp and status before commit.
Concurrent identical first calls therefore converge on one timestamp. Exact
replay after `complete` or `repair_required` returns the original
acknowledgement without changing completion or matched events. Another caller
coordinate is rejected without disclosure, and a terminal run without an
acknowledgement cannot acquire one late. It has no project `state.sqlite`, Core
workflow, Task, or Product Repository effect.

`volicord.get_integration_verification` uses one immediate Registry
transaction. It validates the caller and current owner coordinate, consumes no
more than the stored host policy's allowed status reads, and returns an
existing terminal state unchanged. The current synchronous Codex policy allows
one read: if exact event correlation has not already completed the attempt,
that read persists `repair_required` with the most precise acquisition reason
and separate retry policy. Compatible Guard event persistence retains its
ordinary project-local effect; its subsequent Registry acquisition write can
record a bounded stage, and a matching pre/post stage can atomically finalize
`complete`. No branch fabricates missing Guard events, waits for cleanup
expiry, reactivates a terminal attempt, or alters MCP trust state.

## Managed Runtime Project-Session Binding

An actual managed MCP project call uses these ordered storage effects:

1. Runtime, Connection revision, project membership, observation time, and
   current project identity validation have no storage effect on failure.
2. One immediate project transaction creates or validates the exact unbound
   Agent Session anchor and applies only owner-defined observation updates.
3. One immediate Registry transaction revalidates the current owner facts and
   inserts or exactly reuses the matching
   `mcp_runtime_project_session_bindings` reservation.
4. One final immediate project transaction attaches the runtime to the exact
   anchor or accepts the same existing attachment as replay.

A deterministic Connection, project, Guard Installation, native-session,
thread, immutable-revision, or existing-runtime ownership conflict is rejected
in the first two stages and creates no Registry reservation. A Registry
uniqueness failure can leave the validated project anchor unbound; that row is
correlation state only. An interruption after Registry reservation can leave
the exact reservation without project attachment; that reservation is also not
authority. Exact replay under unchanged owner state reuses it and completes the
final attachment. No failure path compensates by deleting another runtime's
valid reservation.

## Administrative Connection Project Retirement

An accepted `volicord connection remove` apply uses one immediate Registry
transaction. It validates the Agent Connection and selected membership, rejects
pending-host-cleanup conflicts, deletes the selected membership's Registry
project-session bindings and integration-verification runs, then its Guard
Installation and membership, and counts remaining memberships before commit.
With remaining memberships it has
no effect on connection-wide runtime sessions or other projects' rows. With no
remaining membership it additionally deletes every remaining connection-owned
binding, integration-verification run, and Guard Installation, every
connection-owned MCP runtime session, and the Agent Connection.

Connection replacement cleanup uses the same owner-ordered project retirement. A
multi-project superseded Connection loses only the selected project's bindings,
integration-verification runs, Guard Installation, and membership in the same
Registry transaction that activates the replacement membership, Guard
Installation, and Connection. Its
other projects' rows and connection-wide runtime sessions remain.

A last-project superseded Connection instead remains disabled with its complete
membership, bindings, Guard Installation, and pending-host-cleanup marker while
external cleanup is pending or fails. After successful cleanup, one final
Registry transaction revalidates the exact replacement, marker, and membership
inventory, retires the project-owned rows before the membership, and clears the
marker. Revalidation failure leaves that complete Registry inventory unchanged
for retry. The disabled zero-membership historical Connection and its
connection-wide runtime sessions remain after successful replacement cleanup.

A rejected or failed Store transaction has no Registry effect. Dry run has no
Registry, host-configuration, or Product Repository effect. Project-local
Agent Sessions, Guard and workflow history, evidence, authority events, replay,
and other project authority rows are never part of this administrative
retirement. Retained historical rows cannot authorize a current call without
current Registry ownership.

<a id="method-effects"></a>
## Method effect summary

Exact identifiers used in this section: `Task`.

This table summarizes persistence effects. Method behavior and response unions remain owned by method owner documents routed from the [API Methods](api/methods.md).

| Method | Primary storage effect | Details |
|---|---|---|
| `volicord.intake` | creates task and shaping records | See [`volicord.intake`](#volicordintake) |
| `volicord.update_scope` | updates current scope records | See [`volicord.update_scope`](#volicordupdate_scope) |
| `volicord.status` | read-style response | See [`volicord.status`](#volicordstatus) |
| `volicord.get_operation_result` | reads immutable historical replay bytes without storage effects | See [`volicord.get_operation_result`](#volicordget_operation_result) |
| `volicord.prepare_write` | records write decision effects | See [`volicord.prepare_write`](#volicordprepare_write) |
| `volicord.prepare_evidence_capture` | creates one immutable expiring capture intent | See [`volicord.prepare_evidence_capture`](#volicordprepare_evidence_capture) |
| `volicord.stage_artifact` | creates transient staging only | See [`volicord.stage_artifact`](#volicordstage_artifact) |
| `volicord.record_run` | records run, current close-basis, evidence, and evidence-observation effects | See [`volicord.record_run`](#volicordrecord_run) |
| `volicord.request_user_action` | creates one pending user-action request and canonical capture form | See [`volicord.request_user_action`](#volicordrequest_user_action) |
| `volicord.resolve_user_action` | inserts one immutable User Channel resolution | See [`volicord.resolve_user_action`](#volicordresolve_user_action) |
| `volicord.reconcile_changes` | resolves Unrecorded Changes and creates pending user actions | See [`volicord.reconcile_changes`](#volicordreconcile_changes) |
| `volicord.check_close` | read-only close-readiness check | See [`volicord.check_close`](#volicordcheck_close) |
| `volicord.close_task intent=complete` | persists a successful `complete` terminal effect; blocked attempts return a no-effect result | See [`volicord.close_task intent=complete`](#volicordclose_task-intentcomplete) |
| `volicord.close_task intent=cancel` | persists a successful cancellation terminal effect; blocked attempts return a no-effect result | See [`volicord.close_task intent=cancel`](#volicordclose_task-intentcancel) |
| `volicord.close_task intent=supersede` | persists a successful supersession terminal effect; blocked attempts return a no-effect result | See [`volicord.close_task intent=supersede`](#volicordclose_task-intentsupersede) |

<a id="volicordintake"></a>
### `volicord.intake`

Committed `dry_run=false` may:

- create the `Task`
- store its mode, work phase, acceptance policy and reason, and optional
  predecessor relation with carry-forward dispositions
- create ordered active `acceptance_criteria` rows with Core-generated identities
- preserve validated `initial_source_refs` as non-authoritative Task context in the Task owner JSON
- create an optional Change Unit
- create shaping records
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid `dry_run=true`
- rejected attempts

Those branches create no Task, refs, event, replay row, or state-version increment.

Owner links:

- [`volicord.intake` method](api/method-intake.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordupdate_scope"></a>
### `volicord.update_scope`

Committed `dry_run=false` may:

- update current-scope `Task` fields
- for a non-null criterion replacement, update retained active same-Task criterion rows in replacement order, create rows for null IDs, and retire omitted active rows without reactivating retired identities
- create or replace current `change_units`, including effect-contract JSON when supplied by the method owner
- capture the verified optional Git workspace context in the Change Unit write
  basis and advance a non-advisor Task to `work_phase=implementation` when a
  current Change Unit is created or replaced
- increment `tasks.scope_revision` for material current-scope or current Change Unit changes
- invalidate `tasks.close_basis_json` and increment `tasks.close_basis_revision` for material scope changes
- mark incompatible user-action basis rows stale or superseded as owner-defined compatibility requires
- update blockers or stale write-ticket refs as the method owner allows
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts

Valid dry-run previews only describe scope, Change Unit, blocker, and stale write-ticket effects.

Semantically identical normalized updates do not increment `tasks.scope_revision` or invalidate the current close basis.

Owner links:

- [`volicord.update_scope` method](api/method-update-scope.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordstatus"></a>
### `volicord.status`

Read-only calls:

- return response data without Core authority-state mutation
- do not create replay rows
- do not create `project_continuity_records`
- do not mutate Core state
- do not increment `project_state.state_version`

`dry_run=true` remains `StatusResult` with `effect_kind=read_only`, not `ToolDryRunResponse`.

No-effect branches:

- rejected attempts

Owner links:

- [`volicord.status` method](api/method-status.md)

<a id="volicordget_operation_result"></a>
### `volicord.get_operation_result`

Successful calls read one eligible immutable `tool_invocations.response_json`
value in bounded UTF-8 pages. The read does not replay the original mutation,
recompute its response, or turn the historical result into current authority.

The method is response-only and must not create or change:

- replay rows or `tool_invocations.response_json`
- `authority_events` or Core current rows
- Task, Change Unit, user-action, blocker, or continuity state
- staging, artifact, evidence, or write-ticket state
- `project_state.state_version`

Rejected access, invalid cursors, unavailable rows, and integrity failures have
the same no-effect boundary and return no partial historical bytes.

Owner links:

- [`volicord.get_operation_result` method](api/method-get-operation-result.md)
- [Storage Records](storage-records.md#exact-operation-result-storage)
- [Storage Versioning](storage-versioning.md#exact-operation-result-retrieval)

<a id="volicordprepare_write"></a>
### `volicord.prepare_write`

An original committed `dry_run=false` call with `decision=allowed` may:

- issue one active write ticket or reuse one compatible active, unconsumed ticket stored in the physical `write_tickets` table
- append events
- create a replay row
- increment `project_state.state_version` once

Issue inserts one row. Reuse inserts no ticket and preserves its identifier;
the event/replay/state-version effects still occur exactly once. Neither this
increment nor an unrelated Core mutation invalidates the ticket. A non-allow
decision does not revoke unrelated active tickets.

Issue stores the current normalized project write-authority fingerprint in
`validity_basis_json`. Reuse requires an exact non-null match to that current
fingerprint. On any committed non-dry-run decision, every active ticket selected
as stale because its binding is missing or different is atomically set to
`status=invalidated,invalidation_reason=explicit_revoke`. An allowed decision
does so before issuing the new current ticket; a committed non-allow decision
persists the invalidation without issuing a replacement. Rejected and dry-run
paths do not mutate such an invalid row; it remains dynamically unusable.

Idempotent replay returns the stored original response under [Storage Versioning](storage-versioning.md) and does not repeat these effects.

Committed non-allowed decisions:

- See [`volicord.prepare_write` committed non-allow decision](#volicordprepare_write-committed-non-allow-decision).
- They append exactly one `authority_events` row, create a replay row when keyed, and increment `project_state.state_version` exactly once.
- They do not issue a write ticket, create a separate public history method, or create a product-file write authority record.
- `volicord.status` is not required to expose historical non-allow decisions.

No-effect branches:

- rejected attempts
- valid dry-run previews

Those branches do not create:

- replay row
- write ticket
- event
- `close_state` mutation
- artifact or evidence effect
- `project_state.state_version` increment

Owner links:

- [`volicord.prepare_write` method](api/method-prepare-write.md)
- [Storage Records](storage-records.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordprepare_evidence_capture"></a>
### `volicord.prepare_evidence_capture`

An original committed `dry_run=false` call:

- inserts one `evidence_capture_intents` row
- appends one authority event and creates one replay row
- increments `project_state.state_version` exactly once

Exact idempotent replay repeats none of those effects. A valid dry run and any
rejected request create no intent, receipt, staging row or bytes, producer,
source claim, event, replay row, or state-version change.

Registered source fulfillment is a separate Store transaction outside the Core
state commit. After revalidating the intent's source selector and the explicitly
selected registered source, it atomically creates one
`evidence_capture_receipts` row and one redacted `artifact_staging` row plus its
bounded safe JSON bytes, together with every required
`evidence_capture_source_claims` row. Command, guard-connection, and watcher
receipts create one claim; tool receipts create three claims for the normalized
host invocation and both distinct guard events. The project-scoped claim key
rejects reuse of any exact underlying source fact across intents or producer
classes. It creates no event or replay row and does not change
`project_state.state_version`. In the same transaction, it advances
`project_state.updated_at` to at least `receipt.created_at`; another concurrent
writer may already have established a later floor, so equality is not required.
For a registered connection observation, the intent binds only the pre-intent
selector. The source-owned receipt fixes the selected source identifier,
observation time, and raw-event or snapshot/selection digests. A guard event
must have the selected event kind. A watcher observation must belong to the
unique current active baseline for the exact connection and session. Zero or
multiple explicit source coordinates, a pre-intent source, an incomplete or
degraded source, and any receipt/source mismatch fail without effects.
The source observation must satisfy
`intent.created_at <= observed_at < intent.expires_at`; receipt creation must satisfy
`observed_at <= receipt.created_at < intent.expires_at`, and the staging handle expires exactly
at `intent.expires_at`. A failed or duplicate-claim transaction rolls back the
receipt and claims and removes any newly written staging file. One intent can be
fulfilled at most once.

The immutable receipt and source claims are durable source-fact rows. Only the
receipt staging handle and staged safe JSON bytes are transient; promotion does
not delete the receipt row used by the producer audit chain.

Owner links:

- [`volicord.prepare_evidence_capture` method](api/method-prepare-evidence-capture.md)
- [Storage Records](storage-records.md)
- [Artifact Storage](storage-artifacts.md)

<a id="volicordstage_artifact"></a>
### `volicord.stage_artifact`

Successful staging may:

- create `artifact_staging` or an equivalent storage-owned staging record
- store transient safe bytes or notices under `artifacts/tmp/`
- advance `project_state.updated_at` to at least the staging row's `created_at`
  atomically with that row

This branch creates only transient storage-owned staging. It is not the regular Core committed mutation branch, and temporary staging directories may be created when staging occurs rather than during project registration.

Because this branch has no replay row or `OperationResultRef`, Core must prove
that the complete serialized `StageArtifactResult` fits the supported staging
result bound before creating a staging record, staged handle, temporary
directory, bytes, or notice. A prospectively oversized result is rejected with
no staging effect; the size check must not be deferred until after staging.

It does not create:

- Core authority current row other than the physical project-time floor
- persistent `ArtifactRef`
- replay row
- `project_state.state_version` increment

No-effect branches:

- valid `dry_run=true`
- invalid staging requests

Valid `dry_run=true` does not create:

- bytes
- staging record
- `StagedArtifactHandle`
- replay row
- `project_state.state_version` increment

Owner links:

- [`volicord.stage_artifact` method](api/method-stage-artifact.md)
- [Artifact Storage](storage-artifacts.md)

<a id="volicordrecord_run"></a>
### `volicord.record_run`

Committed `dry_run=false` may:

- create `runs`
- consume a compatible `write_tickets` row when the Run records an actual
  Product Repository file write or when an effective `sensitive` `Task` records
  the exact approved non-product action bound by that ticket
- consume eligible `artifact_staging`
- promote or link `artifacts`
- create `evidence_claims` rows for new Task-scoped supplemental targets while preserving the immutable statement of an existing same-Task ID
- insert or update `evidence_summaries` with `produced_at_state_version` set to the transaction's resulting `project_state.state_version`, create `evidence_observations` with separately stored Core-record input refs and non-authoritative source refs, or update allowed blockers
- for each valid capture-intent observation, consume and promote its safe
  receipt staging handle, link the promoted artifact to a new immutable
  `evidence_producers` row, and create that producer and its one-to-one
  `evidence_observation`
- update `tasks.close_basis_revision` and `tasks.close_basis_json` according to `close_assessment`
- append events
- create a replay row
- increment `project_state.state_version` once

A non-sensitive Run with no product-file write leaves a compatible ticket active
for reuse. An effective `sensitive` non-product Run instead requires and consumes
its exact approval-bound ticket so close retains a Core-derived sensitive-action
basis.
Consumption, Run insertion, and all evidence/artifact effects are one atomic
commit. Rejection records no consumption. `basis_state_version` is audit-only;
validity uses the stored Task/Change Unit/scope/baseline/workspace/current
project write-authority/approval basis, status, and optional idle timeout.

No-effect branches:

- valid dry-run previews
- rejected attempts
- invalid staged handles before commit

Valid dry-run previews do not create:

- `run_summary`
- current close basis
- persistent residual-risk IDs
- persistent artifact
- artifact link
- evidence update or evidence observation
- blocker update
- event
- replay row
- staged-handle consumption
- write-ticket compatibility consumption
- `project_state.state_version` increment

Rejected attempts do not change:

- staging rows
- artifacts
- acceptance criteria, supplemental evidence claims, or evidence observations
- evidence-capture intents, receipts, producers, or receipt staging rows

Write-ticket consumption boundary:

- When the method owner allows a committed run that records a product file write,
  or an effective `sensitive` Run that records its exact approved non-product
  action, storage may consume the compatible `write_tickets` row in the same
  commit.
- Core reloads and verifies the current normalized write-authority fingerprint
  before planning consumption. Inside the commit transaction, Store rereads the
  policy and requires the active ticket's durable binding and the planned
  expected binding to match it. If policy authority changes between planning
  and consumption, the transaction rolls back: it consumes no ticket and
  creates no Run, evidence, artifact, event, replay row, or state-version
  effect.
- Test evidence persistence can promote staged artifacts, update evidence, and record evidence observations without implying a product file write observation.
- Exact run classification belongs to the [`volicord.record_run` method](api/method-record-run.md).

Current close-basis persistence boundary:

- A committed `volicord.record_run` increments `tasks.close_basis_revision` exactly once.
- A non-null `close_assessment` writes a new current `CurrentCloseBasis` in `tasks.close_basis_json` and stores Core-generated opaque residual-risk IDs.
- Sensitive action requirements stored in that `CurrentCloseBasis` are derived by Core from the committed Run and any consumed write-ticket compatibility row, preserving operation, normalized paths, sensitive categories, baseline, Change Unit, source Run ref, and source write-ticket ref through close.
- Category-only caller input cannot establish, satisfy, or erase a sensitive action requirement.
- `close_assessment=null` records that the committed Run does not establish a current close basis; any existing current basis becomes stale or absent.
- Evidence Summary recency is determined by `produced_at_state_version`, not by
  `created_at`, `updated_at`, or an opaque record ID; the canonical UTC clock
  does not substitute for authority commit order.
- Run, current close basis, evidence summary, evidence observation, capture
  producer, receipt artifact promotion/linking, write-ticket compatibility
  consumption, replay, event, and revision effects commit atomically.

Owner links:

- [`volicord.record_run` method](api/method-record-run.md)
- [Artifact Storage](storage-artifacts.md)
- [Storage Records](storage-records.md)

<a id="volicordrequest_user_action"></a>
### `volicord.request_user_action`

Committed `dry_run=false` may:

- create one `user_action_requests` row
- store the closed request from which Core derives the canonical capture form,
  plus the Core-derived basis, current basis status, required-for targets,
  candidates, expiry, and exact originating method/idempotency relation
- update affected blockers
- append events
- create a replay row
- increment `project_state.state_version` once

The direct origin `(project_id, source_idempotency_key)` is unique for
`source_method=volicord.request_user_action`. The MCP
`request.operation=resume` branch only reads that row and its immutable original
replay response for the same Agent Connection access scope, and only after the
whole response strict-decodes as the current closed agent-safe result shape.
Any stored replay row that violates that contract fails closed as
`PERSISTED_DATA_CORRUPT` and is not rewritten. Resume creates no
request, event, replay row, resolution, blocker update, or state
version, and does not update the persisted canonical-UTC floor.

No-effect branches:

- valid dry-run previews
- rejected attempts

Valid dry-run previews do not create:

- real `user_action_request_ref`
- pending user action
- blocker update
- event
- replay row
- `project_state.state_version` increment
- persisted canonical-UTC floor update

Owner links:

- [`volicord.request_user_action` method](api/method-request-user-action.md#volicordrequest_user_action)
- [Storage Records](storage-records.md)

<a id="volicordresolve_user_action"></a>
### `volicord.resolve_user_action`

Exact identifiers used in this section: `operation_category`, `user_only`.

Committed `dry_run=false` may:

- insert one immutable one-to-one `user_action_resolutions` row, causing the Core effective-status evaluator to return `resolved`
- store the matching closed resolution body, channel kind and submission id, derived local-user provenance, verification basis, assurance level, and Core capture time; the body carries either option-derived choice facts or the full evidence-observation detail
- create `project_continuity_records` for accepted product, technical, or scope decisions and for accepted current residual risks when selected by the method owner
- update dependent blockers or next actions
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- valid dry-run previews
- rejected attempts

Rejected CLI attempts, including validation failure, wrong binding, expiration, state race, and resolution write failure, must not insert a resolution or update the persisted canonical-UTC floor.

Valid dry-run previews do not create:

- user-action resolution or observation detail
- project continuity records
- blocker update
- event
- replay row
- `project_state.state_version` increment
- persisted canonical-UTC floor update

Resolving a user action does not increment `tasks.scope_revision` or `tasks.close_basis_revision`.

Effective `status=resolved` records that an immutable resolution exists; it is not acceptance or supporting evidence by itself. A choice resolution requires its stored option-derived action/outcome. An observation resolution requires exact current target/artifact detail while preserving the exact artifact refs stored by the request. Missing kind-specific authority facts are invalid owner state.

The replay row remains user-only. Its exact response and any free-form private
note are not eligible for Agent Connection retrieval through
`volicord.get_operation_result`.

Owner links:

- [`volicord.resolve_user_action` method](api/method-resolve-user-action.md#volicordresolve_user_action)
- [Storage Records](storage-records.md)

<a id="volicordreconcile_changes"></a>
### `volicord.reconcile_changes`

Committed `dry_run=false` may:

- set unresolved `unrecorded_changes` rows to `status='resolved'`
- store resolution JSON that names the resolution basis, capture basis, resolved method, and optional linked user-action ref
- store `resolved_at` and `resolved_by_actor_source`
- create pending `user_action_requests` rows for findings that require user acceptance, each carrying `source_method=volicord.reconcile_changes` and the reconciliation idempotency key
- append events
- create a replay row when an idempotency key is present
- increment `project_state.state_version` once

Read-only branches:

- A valid call with no planned resolution or pending user-action creation
  returns response data only and creates no reconciliation effect.

No-effect branches:

- rejected attempts
- valid dry-run previews

These branches do not resolve findings, create pending user actions, append events, create replay rows, or increment `project_state.state_version`.

Reconciliation effects do not prove product correctness, test sufficiency, review completion, final acceptance, residual-risk acceptance, or security. They only record why an Unrecorded Change is no longer unresolved or create a pending user-owned action for remaining acceptance. Reconciliation-created requests are not eligible for the direct-request MCP resume branch.

Owner links:

- [`volicord.reconcile_changes` method](api/method-reconcile-changes.md#volicordreconcile_changes)
- [Storage Records](storage-records.md)

<a id="volicordcheck_close"></a>
### `volicord.check_close`

Read-only calls have no Core authority-state storage effect:

- return computed close readiness
- use the same close-readiness calculation as `volicord.status include.close=true`
- do not create replay rows
- do not append events
- do not create blocker rows
- do not mutate close state
- do not touch artifacts or evidence
- do not increment `project_state.state_version`

`dry_run=true` remains `CloseTaskResult` with `effect_kind=read_only`.

No-effect branches:

- rejected attempts

Owner links:

- [`volicord.check_close` method](api/method-close-task.md#volicordcheck_close)

<a id="volicordclose_task-intentcomplete"></a>
### `volicord.close_task intent=complete`

Committed `dry_run=false` may:

- persist the method-selected terminal completion effect
- persist a terminal close summary distinct from `tasks.close_basis_json` when the method-selected completion effect succeeds
- create `project_continuity_records` with `kind='known_limit'` for current close-basis residual risks that are visible and do not require residual-risk acceptance when the method-selected completion effect succeeds
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- response-only blocked `complete` result
- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`. Preflight failures are no-effect `ToolRejectedResponse`.

A response-only blocked `complete` result uses `base.effect_kind=no_effect` and does not persist close blocker rows, an authority event, a replay row, a terminal mutation, or a state-version increment.

Owner links:

- [`volicord.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordclose_task-intentcancel"></a>
### `volicord.close_task intent=cancel`

Committed `dry_run=false` may:

- persist the method-selected cancellation effect
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- response-only blocked cancellation result
- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`.

Cancellation effects require the method-owned current cancellation judgment with `machine_action=accept`, `resolution_outcome=accepted`, compatible basis, `resolved_by_actor_source=local_user`, and compatible User Channel provenance. Missing or incompatible cancellation authority returns a response-only blocked result and must not fabricate acceptance or completion-only close evidence.

Owner links:

- [`volicord.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

<a id="volicordclose_task-intentsupersede"></a>
### `volicord.close_task intent=supersede`

Committed `dry_run=false` may:

- persist the method-selected supersession effect
- update `project_state.active_task_id` in the same mutation when the method-selected effect requires it
- append events
- create a replay row
- increment `project_state.state_version` once

No-effect branches:

- response-only blocked supersession result
- valid `dry_run=true`
- preflight failures

Valid `dry_run=true` returns `ToolDryRunResponse`.

Owner links:

- [`volicord.close_task` method](api/method-close-task.md)
- [Storage Versioning](storage-versioning.md)

## Related owners

Exact identifiers used in this section: `state_version`.

- [API Methods](api/methods.md) and method owner documents for selected method behavior and response unions.
- [API error routing](api/error-routing.md) and [API error codes](api/error-codes.md) for rejected-response public errors.
- [Storage Records](storage-records.md) for records that effects may touch.
- [Artifact Storage](storage-artifacts.md) for staged-handle and artifact lifecycle details.
- [Storage Versioning](storage-versioning.md) for state clocks and replay/idempotency semantics.
