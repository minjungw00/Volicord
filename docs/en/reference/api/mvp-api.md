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

| Method | Active role | Owner |
|---|---|---|
| <a id="harnessintake"></a>`harness.intake` | Start, resume, or classify ordinary user work. | [Intake method](method-intake.md) |
| <a id="harnessupdate_scope"></a>`harness.update_scope` | Update active Task scope and the active Change Unit after intake. | [Update-scope method](method-update-scope.md) |
| <a id="harnessstatus"></a>`harness.status` | Return current state summary, blockers, pending judgments, evidence summary, close state, and next safe actions. | [Status method](method-status.md) |
| <a id="harnessprepare_write"></a>`harness.prepare_write` | Check product-file write compatibility before Write Authorization. | [Prepare-write method](method-prepare-write.md) |
| <a id="harnessstage_artifact"></a>`harness.stage_artifact` | Stage safe bytes or a safe notice for later `record_run` promotion. | [Stage-artifact method](method-stage-artifact.md) |
| <a id="harnessrecord_run"></a>`harness.record_run` | Record shaping, direct, or implementation work plus compact evidence and artifact refs. | [Record-run method](method-record-run.md) |
| <a id="harnessrequest_user_judgment"></a>`harness.request_user_judgment` | Create one pending user-owned judgment request. | [User-judgment methods](method-user-judgment.md#harnessrequest_user_judgment) |
| <a id="harnessrecord_user_judgment"></a>`harness.record_user_judgment` | Record the user's answer to an existing pending `UserJudgment`. | [User-judgment methods](method-user-judgment.md#harnessrecord_user_judgment) |
| <a id="harnessclose_task"></a>`harness.close_task` | Check close readiness and close, cancel, or supersede only when blockers allow it. | [Close-task method](method-close-task.md) |

## Method owner routing table

Use this table for method behavior questions. Method-specific payload field questions start with the affected method owner. Shared response branch schemas, shared envelope fields, nested schema fields, storage effects, and public errors route to the owner links below.

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
| method-specific payload fields | affected method owner above |
| shared envelope fields and nested schema fields | [schema owner links](#schema-owner-links) |

<a id="shared-request-rules"></a>

## Shared envelope and response branch routes

All public methods use [`ToolEnvelope`](schema-core.md#tool-envelope). Each public method response has exactly one branch:

- concrete method-specific `MethodResult`
- `ToolRejectedResponse`
- `ToolDryRunResponse`

Method results use `ToolResultBase` from [common response branches](schema-core.md#common-response), set `response_kind=result`, and name the concrete result for read, staging, Core committed, or committed blocked outcomes when the method owner allows that branch.

`ToolRejectedResponse` and `ToolDryRunResponse` use the shared response schemas from [common response branches](schema-core.md#common-response). They do not inherit method-specific result-only fields.

Committed non-dry-run state-changing calls require non-null `idempotency_key` and current project-wide `expected_state_version`. Read-only calls, valid dry-run previews, and staging utility calls follow the exception rules in their method owners.

When a method has a method-specific `task_id`, Core resolves the primary Task in this order:

1. Method field.
2. `ToolEnvelope.task_id`.
3. Active Task.

Non-claim: Task resolution selects owner records; it does not create a separate state clock.

## Schema owner links

| Schema area | Owner |
|---|---|
| Common request envelope, common response branches, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`, `ToolError`, and `EventRef` | [API Schema Core](schema-core.md) |
| State summaries, refs, close-readiness shapes, evidence summaries, and write-authority summaries | [API State Schemas](schema-state.md) |
| Artifact inputs, staged artifact handles, and artifact refs | [API Artifact Schemas](schema-artifacts.md) |
| User judgment, judgment options, judgment answers, sensitive-action scopes, and accepted-risk inputs | [API Judgment Schemas](schema-judgment.md) |
| Active method names, method-local values, response/effect kinds, access classes, and lifecycle values | [API Value Sets](schema-value-sets.md) |
| Public error codes, stale-state precedence, and close blocker routing | [API Errors](errors.md) |

## Storage-effect owner links

| Storage area | Owner |
|---|---|
| Method-to-storage effects and no-effect boundaries | [Storage Effects](../storage-effects.md) |
| Persistent record layouts, DDL ownership, record-column meaning, and storage-owned JSON placement | [Storage Records](../storage-records.md) |
| State clocks, idempotency replay behavior, and version conflict storage rules | [Storage Versioning](../storage-versioning.md) |
| Artifact staging, validation, promotion, linking, and body-read lifecycle | [Artifact Storage](../storage-artifacts.md) |

## Stable API example scenario summary

Method owner examples use a durable account export confirmation scenario:

- Task summary: add explicit confirmation before account data export.
- Scope: account export flow and account export confirmation tests.
- Out of scope: account deletion behavior.
- Acceptance: explicit confirmation is required before account data export download.
- Extension: method examples may add representative account export confirmation test run and evidence data.

Examples are compact branch examples, not full schema definitions.

Consistency requirements:
- Nested shapes stay with schema owners.
- Shared scenario refs must stay aligned across `state_version`, artifact refs, run refs, judgment refs, close-readiness evidence, sensitive-action approval reasons, and expiration timestamps.

Maintenance rules for replacing or reviewing API examples live in [Authoring Guide](../../maintain/authoring-guide.md) and [Checks](../../maintain/checks.md).

API example consistency questions and field-name consistency questions route to [Authoring Guide](../../maintain/authoring-guide.md) and [Checks](../../maintain/checks.md). Concrete example field definitions then route to the affected method, schema, or storage owner.

## Method owner documents

- [Intake method](method-intake.md)
- [Update-scope method](method-update-scope.md)
- [Status method](method-status.md)
- [Prepare-write method](method-prepare-write.md)
- [Stage-artifact method](method-stage-artifact.md)
- [Record-run method](method-record-run.md)
- [User-judgment methods](method-user-judgment.md)
- [Close-task method](method-close-task.md)
