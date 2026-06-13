<a id="harnessstage_artifact"></a>

# `harness.stage_artifact` reference

## What this document owns

This document owns baseline method behavior for `harness.stage_artifact`:

- method-specific required inputs, access requirements, state-version behavior, result branches, and dry-run behavior
- the minimal request and representative response for the shared account data export confirmation scenario
- method-level storage-effect summary and links to storage owners

## What this document does not own

This document does not own:

- common `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, or `ToolDryRunResponse` schema bodies
- nested state, artifact, judgment, value-set, or error schema definitions
- storage DDL, storage record layouts, artifact lifecycle, security guarantees, or Core product meaning

## Purpose

Stage caller-provided safe artifact bytes or a safe notice into a transient `StagedArtifactHandle` for the same project and Task.

Staging is input preparation only. It does not create canonical evidence, a persistent `ArtifactRef`, gate satisfaction, final acceptance, residual-risk acceptance, or close readiness.

## Required inputs

- `ToolEnvelope` with `project_id`, `task_id`, `surface_id`, `request_id`, and `dry_run`; `idempotency_key` and `expected_state_version` may be `null`.
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

Non-claim: the result does not contain a persistent `ArtifactRef`.

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

Public error code meaning is owned by [API error codes](error-codes.md). Public error precedence is owned by [API error precedence](error-precedence.md).

## Dry-run behavior

For `dry_run=true`, a valid staging preview:

- returns `ToolDryRunResponse`
- does not return `StageArtifactResult`

Branch shape is owned by [API Schema Core](schema-core.md); no-effect staging semantics are owned by [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).

## Storage effect

On success, the method creates a transient staging result only. Exact storage effects are owned by [Storage Effects](../storage-effects.md), and artifact lifecycle details are owned by [Artifact Storage](../storage-artifacts.md).

Artifact data example:

The staged artifact is stable product test output. `harness.record_run` may consume the transient handle when recording evidence, but staging alone does not create canonical evidence.

```yaml
artifact:
  kind: test_log
  name: account_export_confirmation_test.log
  description: "Test output for account data export confirmation tests."
staged_artifact_handle: staged_artifact_account_export_test_log_001
expires_at: "<future-expiration-timestamp>"
```

## Minimal valid request

```yaml
method: harness.stage_artifact
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_stage_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  task_id: task_456
  display_name: "account_export_confirmation_test.log"
  content_type: text/plain
  redaction_state: none
  safe_bytes_or_notice: "Test output for account data export confirmation tests."
  expected_sha256: null
  expected_size_bytes: null
  relation_hint: "test_log"
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
expires_at: "<future-expiration-timestamp>"
```

## Owner links

- Request envelope, response branches, and dry-run summaries: [API Schema Core](schema-core.md).
- `StagedArtifactHandle`, `ArtifactInput`, and `ArtifactRef`: [API Artifact Schemas](schema-artifacts.md).
- Supported artifact values and access classes: [API Value Sets](schema-value-sets.md).
- Public errors: [API error codes](error-codes.md) and [API error precedence](error-precedence.md).
- Persistence effects and artifact lifecycle: [Storage Effects](../storage-effects.md) and [Artifact Storage](../storage-artifacts.md).
