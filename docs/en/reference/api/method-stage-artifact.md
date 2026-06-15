<a id="harnessstage_artifact"></a>

# `harness.stage_artifact` reference

## What this document owns

This document owns baseline method behavior for `harness.stage_artifact`:

- method-specific required inputs, access requirements, state version behavior, result branches, and `dry_run` behavior
- transient staged-handle creation behavior
- stage-artifact examples

## What this document does not own

This document does not own:

- common request envelope, response branch, dry-run, or rejected-response schema bodies
- `ArtifactInput`, `ArtifactRef`, `StagedArtifactHandle`, value-set, or error schema definitions
- storage DDL, storage record layouts, exact storage effects, artifact lifecycle, security guarantees, or Core authority semantics
- public error code meaning, public error precedence, or shared response-branch routing

## Purpose

`harness.stage_artifact` stages caller-provided safe artifact bytes or a safe notice into a transient `StagedArtifactHandle` for the same project and Task.

Staging is input preparation only. Evidence, persistent artifact links, acceptance, residual-risk, and close-readiness effects are owned by the relevant method and storage owners.

## Required inputs

- A valid `ToolEnvelope`; `idempotency_key` and `expected_state_version` may be `null`.
- `task_id`, `display_name`, `content_type`, `redaction_state`, `safe_bytes_or_notice`, `expected_sha256`, `expected_size_bytes`, and `relation_hint`.

## Access requirements

Requires:

- `VerifiedSurfaceContext.access_class=artifact_registration`
- `verified=true`
- compatible `project_id` and `task_id`
- `manual_artifact_attachment_supported=true`

A server records `created_by_surface_id` and `created_by_surface_instance_id` from the verified local surface. The caller does not provide those fields as authority.

## State version behavior

A successful staging result:

- does not change Core state
- does not increment `project_state.state_version`
- creates no `tool_invocations` replay row

Rejected and dry-run requests have no storage effect.

## Success result

Returns `StageArtifactResult` with:

- `base.response_kind=result`
- `base.effect_kind=staging_created`
- transient `staged_artifact_handle`
- `expires_at`

The result contains a transient handle, not a persistent `ArtifactRef`.

## Blocked result

There is no committed blocked branch.

- Invalid staging requests are rejected before any Core mutation.
- Staging availability or capability problems do not create blockers.

## Rejected result

Returns `ToolRejectedResponse` for:

- invalid request shape
- checksum or size mismatch
- unsafe artifact input
- unsupported redaction state
- unavailable Core or local surface
- local access mismatch
- insufficient artifact registration capability

Public error code meaning, precedence, and rejected-response routing are owned by the error documents linked below.

## Dry-run behavior

For `dry_run=true`, a valid staging preview:

- returns `ToolDryRunResponse`
- does not return `StageArtifactResult`
- creates no staged handle

## Storage effect

On success, the method creates a transient staging result only. Exact storage effects and artifact lifecycle details are owned by the storage documents linked below.

## Minimal valid request

```yaml
method: harness.stage_artifact
params:
  envelope:
    project_id: proj_trace_001
    task_id: task_trace_001
    actor_kind: agent
    surface_id: surface_artifact
    request_id: req_stage_trace_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  task_id: task_trace_001
  display_name: "diagnostic_trace.log"
  content_type: text/plain
  redaction_state: none
  safe_bytes_or_notice: "Local trace sample captured for debugging."
  expected_sha256: null
  expected_size_bytes: null
  relation_hint: "diagnostic_log"
```

## Representative response

Result branch (`StageArtifactResult`, staging created):

```yaml
base:
  response_kind: result
  effect_kind: staging_created
  dry_run: false
  state_version: null
  events: []
staged_artifact_handle:
  handle_id: staged_trace_log_001
  project_id: proj_trace_001
  task_id: task_trace_001
  created_by_surface_id: surface_artifact
  created_by_surface_instance_id: surface_instance_trace_01
  content_type: text/plain
  sha256: sha256:example-trace
  size_bytes: 42
  redaction_state: none
  expires_at: "<future-expiration-timestamp>"
  consumed: false
expires_at: "<future-expiration-timestamp>"
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `StagedArtifactHandle`, `ArtifactInput`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- Supported artifact values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors, precedence, and rejected-response routing: [API error codes](error-codes.md), [API error precedence](error-precedence.md), and [API error routing](error-routing.md).
- Persistence effects and artifact lifecycle: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).
