# Artifact storage

This document owns the artifact storage lifecycle for the current MVP source design. It is documentation source material only and does not create artifact bytes, artifact directories, runtime storage, evidence records, QA records, acceptance records, or close records.

## Owns / Does not own

This document owns:

- staged artifact storage lifecycle
- `StagedArtifactHandle` validation against stored staging records
- promotion from a compatible staged handle to a persistent `ArtifactRef`
- persistent `existing_artifact` linking eligibility
- artifact body-read storage eligibility, availability, redaction, retention, and integrity boundaries

This document does not own:

- API artifact schemas; see [API Artifact Schemas](api/schema-artifacts.md)
- API method behavior; see [MVP API](api/mvp-api.md)
- general record layout or DDL; see [Storage Records](storage-records.md)
- generic method storage effects; see [Storage Effects](storage-effects.md)
- local-access security claims; see [Security](security.md) and [Runtime Boundaries](runtime-boundaries.md)

## Lifecycle boundary

Artifact storage distinguishes staging, promotion, persistent linking, and body reads.

`ArtifactRef` is the public API pointer to a registered persistent artifact, but its shape is owned by [API Artifact Schemas](api/schema-artifacts.md). Storage implements persistent artifact authority through `artifacts` plus `artifact_links`.

`StagedArtifactHandle` is a temporary handle returned by successful `harness.stage_artifact`, but the handle shape is not authority unless it resolves to a compatible stored `artifact_staging` row or equivalent storage-owned staging manifest. `existing_artifact` links an existing persistent artifact; it does not register a new artifact body.

Caller-supplied raw filesystem paths, arbitrary local path strings, raw logs as authority claims, `captured_artifact` handles, raw capture-adapter outputs, and native capture claims are not registration authority in the active MVP.

## Staging

Temporary staging is not artifact authority. `artifact_staging` or an equivalent storage-owned staging manifest tracks at least:

- `handle_id`
- `project_id`
- `task_id`
- `created_by_surface_id`
- `created_by_surface_instance_id`
- `sha256`
- `size_bytes`
- `content_type`
- `redaction_state`
- `status`
- `expires_at`
- consumption facts such as `consumed_by_run_id`, `promoted_artifact_id`, and `consumed_at`

A future server records the `created_by_surface_*` fields from the successful `harness.stage_artifact` request's `VerifiedSurfaceContext`. They are not caller-provided authority claims and must be checked against the staging row, not trusted merely because a submitted handle has the right shape.

A successful `harness.stage_artifact` returns `StageArtifactResult` with `base.effect_kind=staging_created`. It may write safe bytes or a safe notice under `artifacts/tmp/` and create the temporary staging row.

Staging creates only temporary artifact storage. Method-effect questions such as evidence creation, replay rows, and state-version increments are owned by [Storage Effects](storage-effects.md).

`artifact_staging.status` is a storage-owned temporary handle lifecycle:

| Value | Storage meaning |
|---|---|
| `staged` | The handle is unexpired, unconsumed, and potentially consumable by a compatible `harness.record_run`. |
| `consumed` | A compatible `harness.record_run` consumed the handle and recorded the consuming Run and promoted artifact ids. |
| `expired` | The handle passed its usable lifetime and cannot be consumed. |
| `discarded` | The temporary staging object was discarded before persistent registration. |

Only `staged` is consumable. Terminal values cannot return to `staged`.

## Promotion

Only a compatible `harness.record_run` may consume an unexpired same-project same-Task handle with `artifact_staging.status=staged` and promote it to a persistent `ArtifactRef`. The current verified `surface_id` and `surface_instance_id` must match `created_by_surface_id` and `created_by_surface_instance_id`. The active MVP does not support cross-surface staged artifact handoff, and `StagedArtifactHandle` is not a bearer token that any local caller may use.

The consuming transaction must:

- validate stored `project_id`, `task_id`, `created_by_surface_id`, and `created_by_surface_instance_id`
- validate expiration and consumed status
- validate `sha256`, `size_bytes`, and `redaction_state`
- promote only validated staged handles
- mark promoted handles `consumed`
- set the consuming Run and promoted artifact ids
- commit the durable `artifacts` row and required `artifact_links`
- update evidence coverage only as allowed by the method owner

These staging handles must be rejected before mutation with API-owned validation error routing:

- missing
- expired
- mismatched
- already consumed
- discarded
- cross-surface
- wrong `created_by_surface_id`
- wrong `created_by_surface_instance_id`
- wrong `sha256`
- wrong `size_bytes`
- wrong `redaction_state`
- integrity-incompatible
- cross-task

They must not be hidden as evidence sufficiency, local access mismatch, or capability insufficiency.

For `harness.record_run`, storage effects follow this API-owned validation order:

1. request-level `VerifiedSurfaceContext.access_class=run_recording`
2. project-wide `ToolEnvelope.expected_state_version`
3. referenced Task and Change Unit
4. compatible Write Authorization when product-file writes are recorded
5. staged-handle validation
6. staged-handle field checks
7. staged promotion
8. staged consumption
9. existing-artifact link validation
10. no artifact body read

If any validation in this sequence fails before commit, storage must not change:

- `artifact_staging.status`
- `consumed_by_run_id`
- `promoted_artifact_id`
- `artifacts`
- `artifact_links`
- `evidence_summaries`
- `write_authorizations.status`
- `task_events`
- `tool_invocations`
- `project_state.state_version`

## Existing artifacts

Using an `existing_artifact` reuses the persisted artifact row only when its availability, integrity facts, redaction state, same-project identity, and allowed Task scope remain compatible with the new use. It may add a new `artifact_links` row for the new owner relation, subject to uniqueness and same-project/same-Task rules.

`existing_artifact` must not clone bytes, register a new artifact body, skip integrity checks, or use a raw artifact path as authority.

## Evidence eligibility

An artifact is evidence-eligible only when storage has:

- registered bytes or a safe metadata notice under the artifact store
- integrity facts such as `sha256`, `size_bytes`, and `content_type`
- a `redaction_state`
- producer and retention facts
- an availability `status`
- an owner link to an active record such as `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, or `blocker`

Evidence eligibility, artifact availability, and evidence sufficiency remain separate. An `artifacts.status=available` row with a valid owner link can support a coverage item, but it does not make `EvidenceSummary.status=sufficient` unless the required coverage item links that artifact to the claim and the item is `supported` or `not_applicable`. Missing, unavailable, integrity-failed, or otherwise unusable artifacts stay artifact-availability problems and can also keep required evidence coverage from being sufficient.

Artifact owner relation integrity is required even though `artifact_links` is a polymorphic owner table. Storage must validate that `owner_record_kind` is one of `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, or `blocker`; that `owner_record_id` exists in the matching active table; that the owner belongs to the same `project_id` and `task_id`; and that the relation is compatible with the way the artifact is used. A raw `artifact_id` without a valid owner link is not evidence support.

An artifact link does not create the owner record, satisfy a gate by itself, prove evidence sufficiency, perform QA, create final acceptance, accept residual risk, or close a Task.

## Availability, redaction, and integrity

`artifacts.status` is an availability state:

| Value | Storage meaning |
|---|---|
| `available` | The registered safe bytes or safe metadata notice is present and matches stored integrity metadata. |
| `missing` | The artifact row remains, but the registered bytes or safe metadata notice cannot be found. |
| `integrity_failed` | The available bytes or metadata do not match stored integrity facts such as `sha256` or `size_bytes`. |
| `unavailable` | The artifact store or required retrieval path cannot currently provide the registered bytes or safe metadata notice. |

`artifacts.redaction_state` uses the active `ArtifactRef.redaction_state` values from [API Artifact Schemas](api/schema-artifacts.md). `blocked` is a redaction/omission state, not an artifact availability status. A `blocked`, `secret_omitted`, or `redacted` artifact may still have `artifacts.status=available` when the committed safe notice or redacted bytes are present and integrity-aware.

`sha256`, `size_bytes`, and `content_type` are artifact integrity facts for comparison and availability handling. They are not security guarantee claims; see [Security](security.md).

`uri` resolves through Harness storage, normally as `harness-artifact://{project_id}/{artifact_id}`. It is not a caller-supplied arbitrary filesystem path. Raw secrets, tokens, and full sensitive logs must not be stored as evidence bytes. Store redacted bytes, `secret_omitted` or `blocked` notices, safe handles, or other owner-approved safe representations instead.

## Body reads

Artifact body reads are separate from staged artifact promotion. Raw artifact path reads are not granted by default.

Artifact metadata or content reads require a registered `ArtifactRef`, the matching same-project `task_id`, the required `artifact_links` owner relation, the redaction/availability state needed by the caller's access class, and the API/security owner requirements for `access_class=artifact_read`, including any documented surface or connector capability boundary. A local path under the artifact store, an artifact `uri`, a staged path, or a copied file is not enough by itself to read or rely on artifact bytes.

## Retention boundary

Unconsumed or expired `artifact_staging` rows and `artifacts/tmp/` staging bytes or notices may be marked `expired` or `discarded`, and temporary bytes may be cleaned before registration, because they are not evidence authority.

Once an `artifacts` row is committed, retention purge, project teardown, or destructive cleanup is outside ordinary active MVP mutation behavior and needs an owner-defined path. A future retention or migration path must preserve artifact hashes, owner links, events, and replay rows, or mark affected refs invalid for recovery. It must not silently delete evidence support that current records still name.

## Related owners

- [API Artifact Schemas](api/schema-artifacts.md) for `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes.
- [MVP API](api/mvp-api.md) for `harness.stage_artifact`, `harness.record_run`, and artifact read behavior.
- [Storage Effects](storage-effects.md) for whether a response branch creates storage effects.
- [Storage Records](storage-records.md) for `artifact_staging`, `artifacts`, and `artifact_links` table overview.
- [Security](security.md) for access and guarantee non-claims.
