# Active MVP API

## What this document owns

This document is the stable route document for the active MVP API method family. It owns:

- the active public API method list
- method owner routing
- shared envelope and response branch owner links
- schema owner links
- storage-effect owner links
- the stable API example scenario summary used by method owner documents

## What this document does not own

This document does not own:

- full per-method behavior details, including method-specific required inputs, access requirements, result fields, dry-run behavior, or representative request and response bodies
- common API envelope bodies, response branch schema bodies, or schema field definitions
- state, artifact, judgment, value-set, or error schema definitions
- API example consistency rules or field-name consistency rules
- storage-effect details, storage DDL, storage record layouts, artifact lifecycle, state-version storage rules, or security guarantees
- public error-code semantics
- future or later-candidate API methods

## API method family boundary

The active MVP API is a small local MCP surface for one user work loop. It can intake work, show status, update active scope, check proposed product writes, stage artifacts, record runs and evidence refs, ask and record user-owned judgment, and close only when active blockers allow it.

The API returns cooperative Harness record and check behavior only. Security non-claims and guarantee wording belong to [Security](../security.md). Future API or schema candidates are cataloged in [Later Candidate Index](../../later/index.md), not in this active reference.

<a id="active-mvp-method-behavior"></a>

## Active MVP API method list

This document owns the active public method list and owner routing. The exact active method-name value set is owned by [API Value Sets](schema-value-sets.md). The active methods route to these owner documents:

<a id="harnessintake"></a>
<a id="harnessupdate_scope"></a>
<a id="harnessstatus"></a>
<a id="harnessprepare_write"></a>
<a id="harnessstage_artifact"></a>
<a id="harnessrecord_run"></a>
<a id="harnessrequest_user_judgment"></a>
<a id="harnessrecord_user_judgment"></a>
<a id="harnessclose_task"></a>

| Method | Active role | Owner |
|---|---|---|
| `harness.intake` | Start, resume, or classify ordinary user work. | [Intake method](method-intake.md) |
| `harness.update_scope` | Update active Task scope and the active Change Unit after intake. | [Update-scope method](method-update-scope.md) |
| `harness.status` | Return current state and next safe actions. | [Status method](method-status.md) |
| `harness.prepare_write` | Check product-file write compatibility before Write Authorization. | [Prepare-write method](method-prepare-write.md) |
| `harness.stage_artifact` | Stage safe bytes or a safe notice. | [Stage-artifact method](method-stage-artifact.md) |
| `harness.record_run` | Record work, evidence, and artifact refs. | [Record-run method](method-record-run.md) |
| `harness.request_user_judgment` | Create one pending user-owned judgment. | [User-judgment methods](method-user-judgment.md#harnessrequest_user_judgment) |
| `harness.record_user_judgment` | Record the user's answer to a pending judgment. | [User-judgment methods](method-user-judgment.md#harnessrecord_user_judgment) |
| `harness.close_task` | Check close readiness or close when allowed. | [Close-task method](method-close-task.md) |

<a id="method-owner-routing-table"></a>

## Method owner routing

Use this table for method behavior routing.

| Question | Owner |
|---|---|
| `harness.intake` behavior | [Intake method](method-intake.md) |
| `harness.update_scope` behavior | [Update-scope method](method-update-scope.md) |
| `harness.status` behavior | [Status method](method-status.md) |
| `harness.prepare_write` behavior | [Prepare-write method](method-prepare-write.md) |
| `harness.stage_artifact` behavior | [Stage-artifact method](method-stage-artifact.md) |
| `harness.record_run` behavior | [Record-run method](method-record-run.md) |
| user judgment methods | [User-judgment methods](method-user-judgment.md) |
| `harness.close_task` behavior | [Close-task method](method-close-task.md) |

Method-specific questions:

- Method behavior: use the method owner document above.
- Method-specific payload fields: use the affected method owner document.
- Request and response branch shape: use [`schema-core.md`](schema-core.md).
- Nested state fields: use [`schema-state.md`](schema-state.md).
- Artifact fields: use [`schema-artifacts.md`](schema-artifacts.md).
- Judgment fields: use [`schema-judgment.md`](schema-judgment.md).
- Value sets: use [`schema-value-sets.md`](schema-value-sets.md).
- Storage effects: use [`../storage-effects.md`](../storage-effects.md).
- Public errors: use [`errors.md`](errors.md).
- API example consistency: use [Authoring Guide](../../maintain/authoring-guide.md) and [Checks](../../maintain/checks.md).

<a id="shared-request-rules"></a>

## Shared envelope and response branch routes

Shared API shapes route to [API Schema Core](schema-core.md).

- Request envelope: [`ToolEnvelope`](schema-core.md#tool-envelope).
- Response branches: [common response branches](schema-core.md#common-response).
- Shared result base: [`ToolResultBase`](schema-core.md#common-response).
- Rejected and dry-run branches: `ToolRejectedResponse` and `ToolDryRunResponse` in [common response branches](schema-core.md#common-response).
- Method result branch availability: use the affected method owner document.
- `idempotency_key`, `expected_state_version`, and `dry_run` exceptions: use the affected method owner document.
- Task selector precedence: method-specific `task_id`, `ToolEnvelope.task_id`, then active Task.
- Method-specific `task_id` fields: use the affected method owner document.

## Schema owner links

- Request and response branch shape: [API Schema Core](schema-core.md).
- Nested state fields: [API State Schemas](schema-state.md).
- Artifact fields: [API Artifact Schemas](schema-artifacts.md).
- Judgment fields: [API Judgment Schemas](schema-judgment.md).
- Value sets: [API Value Sets](schema-value-sets.md).
- Public errors: [API Errors](errors.md).

## Storage-effect owner links

- Method storage effects and no-effect boundaries: [Storage Effects](../storage-effects.md).
- Record layouts and DDL ownership: [Storage Records](../storage-records.md).
- State clocks and version conflict storage rules: [Storage Versioning](../storage-versioning.md).
- Artifact staging, promotion, and lifecycle: [Artifact Storage](../storage-artifacts.md).

## Stable API example scenario summary

Method owner examples use a durable account data export confirmation scenario:

- Task summary: add explicit confirmation before account data export.
- Scope: account data export flow and account data export confirmation tests.
- Out of scope: account deletion behavior.
- Acceptance: explicit confirmation is required before account data export download.
- Extension: method examples may add representative account data export confirmation test run and evidence data.

Examples are compact branch examples, not schema definitions.

API example questions:

- Consistency rules and replacement guidance: [Authoring Guide](../../maintain/authoring-guide.md) and [Checks](../../maintain/checks.md).
- Nested shapes: use the schema owner links above.
- Method payload fields: use the affected method owner document.
- Schema fields: use the affected schema owner document.
- Storage-owned example fields: use the affected storage owner document.
- Shared scenario ref alignment: use [Authoring Guide](../../maintain/authoring-guide.md) and [Checks](../../maintain/checks.md).

## Method owner documents

- [Intake method](method-intake.md)
- [Update-scope method](method-update-scope.md)
- [Status method](method-status.md)
- [Prepare-write method](method-prepare-write.md)
- [Stage-artifact method](method-stage-artifact.md)
- [Record-run method](method-record-run.md)
- [User-judgment methods](method-user-judgment.md)
- [Close-task method](method-close-task.md)
