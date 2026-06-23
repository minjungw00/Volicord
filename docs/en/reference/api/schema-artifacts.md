# API artifact schemas

This document owns API artifact-shaped schemas for the baseline scope. The schemas define request and response shapes; they do not grant local file access, create artifact bytes, define storage rows, or prove evidence sufficiency.

## Owns / Does not own

This document owns:

- `ArtifactRef`
- `ArtifactInput`
- `StagedArtifactHandle`
- staged versus existing artifact input distinctions
- artifact-shaped request and response fields for staging, linking, and body-read references
- artifact-shaped reference constraints needed to validate the schema
- redaction, availability, integrity, checksum, and size fields that appear on artifact-shaped API responses

This document does not own:

- artifact storage layout, staging records, promotion persistence, retention, or body-read storage eligibility; see [Artifact Storage](../storage-artifacts.md)
- method behavior for `volicord.stage_artifact` and `volicord.record_run`; see [Stage-artifact method](method-stage-artifact.md), [Record-run method](method-record-run.md), and the [API Methods](methods.md)
- supported artifact value sets; see [API Value Sets](schema-value-sets.md)
- evidence sufficiency; see [Core Model](../core-model.md) and [API State Schemas](schema-state.md)
- security claims about access, blocking, or isolation; see [Security](../security.md)

## Boundary

Artifact schemas do not make a caller-supplied path authoritative.

This document describes the request and response shapes used by artifact-related methods and owners.

Owner links:
- method validation, staging, promotion, and linking behavior: method owner documents routed from [API Methods](methods.md)
- body-read eligibility and artifact lifecycle: [Artifact Storage](../storage-artifacts.md)

## `ArtifactRef`

`ArtifactRef` is the public artifact reference and metadata shape.

```yaml
ArtifactRef:
  artifact_id: string
  project_id: string
  task_id: string
  display_name: string
  content_type: string | null
  sha256: string | null
  size_bytes: integer | null
  integrity_status: string
  redaction_state: string
  availability: string
  created_by_run_ref: StateRecordRef | null
  created_by_surface_id: string | null
  created_by_surface_instance_id: string | null
  storage_ref: string | null
```

`ArtifactRef` is a reference and metadata shape. It does not make artifact body content readable by default and does not prove that the content is sufficient evidence for close.

`artifact_id`, `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, and `storage_ref` are opaque identifiers. `display_name` is a free-form display string. `content_type` is media-type metadata when known, `sha256` is a checksum string when known, and `size_bytes` is byte-size metadata when known. `integrity_status`, `redaction_state`, and `availability` are controlled value strings owned by [artifact values](schema-value-sets.md#artifact-values).

`integrity_status` is required. Null `content_type`, `sha256`, or `size_bytes` means the fact is unknown, not empty, not zero, and not defaulted. Missing facts must not be represented as an empty hash, zero-byte size, or invented content type. A real zero-byte artifact has `size_bytes: 0` and the SHA-256 of empty bytes, `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

For `integrity_status=verified`, `content_type` is non-empty, `sha256` is a valid lowercase hexadecimal SHA-256 string, and `size_bytes` is nonnegative. Authority-bearing evidence and close use also require current-byte verification by [Artifact Storage](../storage-artifacts.md). `integrity_status=corrupt` records a known mismatch or invalid verified-fact relationship. Missing, unreadable, unavailable, or unusable backing bytes are represented through `availability`, not a third integrity value.

## `StagedArtifactHandle`

`StagedArtifactHandle` is the transient-handle shape associated with `volicord.stage_artifact` results. It is not the persistent `ArtifactRef` shape.

```yaml
StagedArtifactHandle:
  handle_id: string
  project_id: string
  task_id: string
  created_by_surface_id: string
  created_by_surface_instance_id: string
  content_type: string
  sha256: string
  size_bytes: integer
  redaction_state: string
  expires_at: string
  consumed: boolean
```

The caller does not submit `created_by_surface_id` or `created_by_surface_instance_id` as authority claims. Staged-handle lifecycle, provenance validation, expiry, and promotion are owned by [Artifact Storage](../storage-artifacts.md) and method owner documents.

`handle_id`, `project_id`, `task_id`, `created_by_surface_id`, and `created_by_surface_instance_id` are opaque identifiers. `content_type` is media-type metadata, `sha256` is a checksum string, and `redaction_state` is a controlled value string.

## `ArtifactInput`

`ArtifactInput` is the request-side shape for methods that accept artifact links for run or evidence output.

```yaml
ArtifactInput:
  artifact_input_id: string
  source_kind: string
  staged_artifact_handle: StagedArtifactHandle | null
  existing_artifact_ref: ArtifactRef | null
  relation_hint: string | null
  claim: string | null
  expected_sha256: string | null
  expected_size_bytes: integer | null
  redaction_state: string | null
```

For each input, exactly one source field is populated and the other source field is `null`. `ArtifactInput.source_kind` selects which source field applies; supported source-kind values and value meanings are owned by [artifact values](schema-value-sets.md#artifact-values).

`artifact_input_id` is an opaque request-local input identifier. `relation_hint` and `claim` are free-form display or claim strings. `expected_sha256` is a checksum string. `redaction_state`, when present, is a controlled value string.

Shape rules:
- If `staged_artifact_handle` is populated, `existing_artifact_ref` is `null`.
- If `existing_artifact_ref` is populated, `staged_artifact_handle` is `null`.

Caller-supplied paths, logs, capture claims, or local file references are not artifact authority.

## Reference constraints

`ArtifactInput[]` selects one artifact source shape per input. It does not add a second request-level access class to a public API request.

Public error semantics and response routing for invalid source-field shape are owned by [API error codes](error-codes.md) and [API error routing](error-routing.md). Staged-handle validation, promotion, body-read eligibility, and persistent linking are owned by [Artifact Storage](../storage-artifacts.md) and method owner documents.

## Related owners

- [Stage-artifact method](method-stage-artifact.md), [Record-run method](method-record-run.md), and [API Methods](methods.md) for artifact-related method behavior.
- [Artifact Storage](../storage-artifacts.md) for staging, promotion, persistent linking, and body-read lifecycle.
- [API Value Sets](schema-value-sets.md) for `ArtifactInput.source_kind`, `redaction_state`, availability, and related values.
- [API State Schemas](schema-state.md) for evidence summaries that mention `ArtifactRef`.
- [Runtime Boundaries](../runtime-boundaries.md) and [Security](../security.md) for local access and non-claim boundaries.
