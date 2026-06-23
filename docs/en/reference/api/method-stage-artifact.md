<a id="volicordstage_artifact"></a>

# `volicord.stage_artifact` reference

## What this document owns

This document owns baseline method behavior for `volicord.stage_artifact`:

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

`volicord.stage_artifact` stages caller-provided safe artifact bytes or a safe notice into a transient `StagedArtifactHandle` for the same project and Task.

Staging is input preparation only. Evidence, persistent artifact links, acceptance, residual-risk, and close-readiness effects are owned by the relevant method and storage owners.

## Required inputs

- A valid `ToolEnvelope`; `idempotency_key` and `expected_state_version` may be `null`.
- `task_id`, `display_name`, `content_type`, `redaction_state`, `safe_bytes_or_notice`, `expected_sha256`, `expected_size_bytes`, and `relation_hint`.

## Request schema

This method owns the top-level `params` request shape below. `envelope` is the shared [`ToolEnvelope`](schema-core.md#tool-envelope); this block does not redefine `ToolEnvelope` fields.

All fields shown in this method-owned request block are required members of `params` unless a field note explicitly marks a member optional; `T | null` means the member must be present and may contain JSON `null`.

```yaml
StageArtifactRequest:
  envelope: ToolEnvelope
  task_id: string
  display_name: string
  content_type: string
  redaction_state: string
  safe_bytes_or_notice: string
  expected_sha256: string | null
  expected_size_bytes: integer | null
  relation_hint: string | null
```

Nested owner links:
- `redaction_state` values are owned by [API Value Sets artifact values](schema-value-sets.md#artifact-values).
- The result-side `StagedArtifactHandle` shape stays with [API Artifact Schemas](schema-artifacts.md#stagedartifacthandle).

## Staging admission defaults

This method applies the baseline staging defaults owned by [Artifact Storage](../storage-artifacts.md):

- the returned handle expires 24 hours after staging creation
- the stored staged artifact body or safe notice is capped at 10 MiB (10,485,760 bytes)
- stored body bytes are limited to safe text, JSON, Markdown, XML, or equivalent textual media types
- binary input is represented only by a safe textual notice unless a future owner defines a profile-gated safe binary body path
- raw secrets must not be stored; use `redaction_state=secret_omitted` or `redaction_state=blocked` safe notices where applicable

Requests that fail these admission requirements use the existing rejected-result behavior for invalid or unsafe artifact input. This section applies and routes storage-owned defaults; artifact lifecycle, retention, redaction-state value meaning, and body-read eligibility remain owned by [Artifact Storage](../storage-artifacts.md) and [API Value Sets](schema-value-sets.md#artifact-values).

## Access requirements

Requires:

- server-derived `VerifiedSurfaceContext` with `access_class=artifact_registration`
- compatible `project_id` and `task_id`
- `manual_artifact_attachment_supported=true`

A server records `created_by_surface_id` and `created_by_surface_instance_id` from the derived `VerifiedSurfaceContext`. The caller does not provide those fields as authority.

## State version behavior

A successful staging result:

- does not change Core state
- does not increment `project_state.state_version`
- reports the current project-wide `project_state.state_version` observed by the call in `base.state_version`
- creates no `tool_invocations` replay row

Rejected and dry-run requests have no storage effect.

## Method result fields

`StageArtifactResult` is the method-specific result branch for a successful staging operation. It carries `base: ToolResultBase` and these method-owned top-level fields:

| Field | Result-field meaning |
|---|---|
| `base` | Common result metadata. The `ToolResultBase` shape, including `events`, is owned by [API Schema Core](schema-core.md#common-response). Successful staging uses `base.response_kind=result`, `base.effect_kind=staging_created`, the current project-wide `project_state.state_version` observed by the call in `base.state_version`, and `events: []`. |
| `staged_artifact_handle` | Transient `StagedArtifactHandle` for the staged safe bytes or safe notice. The shape is owned by [API Artifact Schemas](schema-artifacts.md#stagedartifacthandle). |
| `expires_at` | Expiration timestamp for the transient handle. It mirrors `staged_artifact_handle.expires_at`; lifecycle, expiry, and consumption details are owned by [Artifact Storage](../storage-artifacts.md). |

`StageArtifactResult` does not include a persistent `ArtifactRef`, run summary, evidence summary, blocker refs, or current state snapshot.

## Success result

Returns `StageArtifactResult` with:

- `base.response_kind=result`
- `base.effect_kind=staging_created`
- `base.state_version` set to the current project-wide version observed by the call
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

The examples are intentionally compact and method-local. The representative response shows the full `StageArtifactResult` top-level shape and one `StagedArtifactHandle`.

## Minimal valid request

```yaml
method: volicord.stage_artifact
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
  state_version: 42
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
