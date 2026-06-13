# API artifact schemas

This document owns API artifact-shaped schemas for the baseline scope. The schemas define request and response shapes; they do not grant local file access, create artifact bytes, create storage rows, or prove evidence sufficiency.

## Owns / Does not own

This document owns:

- `ArtifactRef`
- `ArtifactInput`
- `StagedArtifactHandle`
- staged versus existing artifact input distinctions
- artifact-shaped request and response fields for staging, linking, and body-read references
- artifact-shaped reference constraints needed to validate the schema
- redaction, availability, checksum, and size fields that appear on artifact-shaped API responses

This document does not own:

- artifact storage layout, staging records, promotion persistence, retention, or body-read storage eligibility; see [Artifact Storage](../storage-artifacts.md)
- method behavior for `harness.stage_artifact` and `harness.record_run`; see [Stage-artifact method](method-stage-artifact.md), [Record-run method](method-record-run.md), and the [API Methods](methods.md)
- supported artifact value sets; see [API Value Sets](schema-value-sets.md)
- evidence sufficiency; see [Core Model](../core-model.md) and [API State Schemas](schema-state.md)
- security claims about access, blocking, or isolation; see [Security](../security.md)

## Boundary

Artifact schemas do not make a caller-supplied path authoritative.

This document describes the request and response shapes used by artifact-related methods and owners.

Owner links:
- validation, staging, promotion, and linking: method owner documents routed from [API Methods](methods.md)
- body-read eligibility and artifact lifecycle: [Artifact Storage](../storage-artifacts.md)

## `ArtifactRef`

`ArtifactRef` is the public pointer to a persistent artifact that has already been registered by an artifact owner.

```yaml
ArtifactRef:
  artifact_id: string
  project_id: string
  task_id: string
  display_name: string
  content_type: string
  sha256: string
  size_bytes: integer
  redaction_state: string
  availability: string
  created_by_run_ref: StateRecordRef | null
  created_by_surface_id: string | null
  created_by_surface_instance_id: string | null
  storage_ref: string | null
```

`ArtifactRef` is a reference and metadata shape. It does not make artifact body content readable by default and does not prove that the content is sufficient evidence for close.

## `StagedArtifactHandle`

`StagedArtifactHandle` is a transient handle returned by successful `harness.stage_artifact`. It represents storage-owned transient staging, not a persistent artifact.

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

The caller does not submit `created_by_surface_id` or `created_by_surface_instance_id` as authority claims. Staged-handle lifecycle, provenance validation, expiry, and promotion are owned by [Artifact Storage](../storage-artifacts.md).

## `ArtifactInput`

`ArtifactInput` is used by methods that link artifacts into run or evidence output.

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

Shape rules:
- If `staged_artifact_handle` is populated, it must be a compatible transient staged handle.
- If `existing_artifact_ref` is populated, it must be an already persistent same-project artifact reference.

Caller-supplied paths, logs, capture claims, or local file references are not artifact authority.

## Reference constraints

`ArtifactInput[]` selects one artifact source shape per input. It does not add a second request-level access class to a public API request.

Invalid source-field shape returns through `ToolRejectedResponse` with public error semantics owned by [API error codes](error-codes.md) and [API error routing](error-routing.md). Staged-handle validation, promotion, body-read eligibility, and persistent linking are owned by [Artifact Storage](../storage-artifacts.md).

## Related owners

- [Stage-artifact method](method-stage-artifact.md), [Record-run method](method-record-run.md), and [API Methods](methods.md) for artifact-related method behavior.
- [Artifact Storage](../storage-artifacts.md) for staging, promotion, persistent linking, and body-read lifecycle.
- [API Value Sets](schema-value-sets.md) for `ArtifactInput.source_kind`, `redaction_state`, availability, and related values.
- [API State Schemas](schema-state.md) for evidence summaries that mention `ArtifactRef`.
- [Runtime Boundaries](../runtime-boundaries.md) and [Security](../security.md) for local access and non-claim boundaries.
