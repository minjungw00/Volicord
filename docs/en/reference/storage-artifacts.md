# Artifact storage

Rule:

- This document owns the artifact storage lifecycle for the baseline scope source design.

Not allowed:

- This document does not create artifact bytes, artifact directories, runtime storage, evidence records, QA records, acceptance records, or close records.

## Owns / Does not own

This document owns:

- staged artifact storage lifecycle
- `StagedArtifactHandle` validation against stored staging records
- promotion from a compatible staged handle to a persistent `ArtifactRef`
- persistent `existing_artifact` linking eligibility
- artifact body-read storage eligibility, availability, redaction, retention, and integrity boundaries

This document does not own:

- API artifact schemas; see [API Artifact Schemas](api/schema-artifacts.md)
- API method behavior; see the [API Methods](api/methods.md), [Stage-artifact method](api/method-stage-artifact.md), and [Record-run method](api/method-record-run.md)
- general record layout or DDL; see [Storage Records](storage-records.md)
- generic method storage effects; see [Storage Effects](storage-effects.md)
- local-access security claims; see [Security](security.md) and [Runtime Boundaries](runtime-boundaries.md)

## Lifecycle boundary

Rule:

- Artifact storage distinguishes staging, promotion, persistent linking, and body reads.
- `ArtifactRef` is the public API pointer to a registered persistent artifact.
- Storage implements persistent artifact authority through `artifacts` plus `artifact_links`.

Owner links:

- `ArtifactRef` shape is owned by [API Artifact Schemas](api/schema-artifacts.md).

Allowed:

- `StagedArtifactHandle` is a transient handle returned by successful `harness.stage_artifact`.
- `existing_artifact` links an existing persistent artifact.

Not allowed:

- A `StagedArtifactHandle` shape is not authority unless it resolves to a compatible stored `artifact_staging` row or equivalent storage-owned staging manifest.
- `existing_artifact` does not register a new artifact body.
- Caller-supplied paths, logs, capture claims, or local file references are not registration authority in the baseline.

## Staging

Rule:

- transient staging is not artifact authority.
- `artifact_staging` or an equivalent storage-owned staging manifest tracks staging facts.

Tracked facts:

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

Rule:

- A server records the `created_by_surface_*` fields from the successful `harness.stage_artifact` request's `VerifiedSurfaceContext`.
- The fields must be checked against the staging row.

Not allowed:

- The fields are not caller-provided authority claims.
- A submitted handle must not be trusted merely because it has the right shape.

Allowed:

- A successful `harness.stage_artifact` returns `StageArtifactResult` with `base.effect_kind=staging_created`.
- It may write safe bytes or a safe notice under `artifacts/tmp/`.
- It may create the transient staging row.

Example staged artifact data:

```yaml
artifact:
  kind: test_log
  name: account_export_confirmation_test.log
  description: "Test output for account data export confirmation tests."
staged_artifact_handle: staged_artifact_account_export_test_log_001
expires_at: "<future-expiration-timestamp>"
```

Rule:

- The example represents product test output prepared for staging.
- Staging creates only transient artifact storage.

Not allowed:

- The example is not a persistent `ArtifactRef`.
- The example does not become canonical evidence until a compatible owner method records and promotes it under that method's contract.

Owner links:

- Method-effect questions such as evidence creation, replay rows, and state-version increments are owned by [Storage Effects](storage-effects.md).

`artifact_staging.status` is a storage-owned transient handle lifecycle. The summary table stays short; detail blocks define the value meanings.

| Value | Summary | Details |
|---|---|---|
| `staged` | consumable candidate | [`staged`](#artifact-staging-status-staged) |
| `consumed` | consumed by owner method | [`consumed`](#artifact-staging-status-consumed) |
| `expired` | usable lifetime passed | [`expired`](#artifact-staging-status-expired) |
| `discarded` | transient object discarded | [`discarded`](#artifact-staging-status-discarded) |

<a id="artifact-staging-status-staged"></a>
**`artifact_staging.status=staged`**

Storage meaning:

- The handle is unexpired and unconsumed.
- A compatible `harness.record_run` may consume it.

<a id="artifact-staging-status-consumed"></a>
**`artifact_staging.status=consumed`**

Storage meaning:

- A compatible `harness.record_run` consumed the handle.
- Storage recorded the consuming Run and promoted artifact ids.

<a id="artifact-staging-status-expired"></a>
**`artifact_staging.status=expired`**

Storage meaning:

- The handle passed its usable lifetime.
- The handle cannot be consumed.

<a id="artifact-staging-status-discarded"></a>
**`artifact_staging.status=discarded`**

Storage meaning:

- The transient staging object was discarded before persistent registration.

Only `staged` is consumable. Terminal values cannot return to `staged`.

## Promotion

Rule:

- Only a compatible `harness.record_run` may consume a staged handle and promote it to a persistent `ArtifactRef`.

Required conditions:

- `artifact_staging.status=staged`.
- The handle is unexpired.
- The handle belongs to the same project.
- The handle belongs to the same Task.
- The current verified `surface_id` matches `created_by_surface_id`.
- The current verified `surface_instance_id` matches `created_by_surface_instance_id`.

Not allowed:

- Cross-surface staged artifact transfer is outside the baseline scope.
- `StagedArtifactHandle` is not a bearer token that any local caller may use.

The consuming transaction must validate:

- stored `project_id`, `task_id`, `created_by_surface_id`, and `created_by_surface_instance_id`
- expiration and consumed status
- `sha256`, `size_bytes`, and `redaction_state`

The consuming transaction may commit only after validation:

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

Rule:

- `existing_artifact` reuses the persisted artifact row only when the existing artifact remains compatible with the new use.

Required conditions:

- availability
- integrity facts
- redaction state
- same-project identity
- allowed Task scope

Allowed:

- A compatible `existing_artifact` may add a new `artifact_links` row for the new owner relation.
- The new link remains subject to uniqueness and same-project/same-Task rules.

Not allowed:

- `existing_artifact` must not clone bytes.
- `existing_artifact` must not register a new artifact body.
- `existing_artifact` must not skip integrity checks.
- `existing_artifact` must not use a raw artifact path as authority.

## Evidence eligibility

An artifact is evidence-eligible only when storage has:

- registered bytes or a safe metadata notice under the artifact store
- integrity facts such as `sha256`, `size_bytes`, and `content_type`
- a `redaction_state`
- producer and retention facts
- an availability `status`
- an owner link to an active record such as `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, or `blocker`

Rule:

- Evidence eligibility, artifact availability, and evidence sufficiency remain separate.
- Artifact owner relation integrity is required even though `artifact_links` is a polymorphic owner table.

Allowed:

- An `artifacts.status=available` row with a valid owner link can support a coverage item.
- The coverage item can make `EvidenceSummary.status=sufficient` only when the required coverage item links that artifact to the claim and the item is `supported` or `not_applicable`.

Required validation:

- `owner_record_kind` is one of `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, or `blocker`.
- `owner_record_id` exists in the matching active table.
- The owner belongs to the same `project_id` and `task_id`.
- The relation is compatible with the way the artifact is used.

Not allowed:

- Missing, unavailable, integrity-failed, or otherwise unusable artifacts do not stop being artifact-availability problems.
- A raw `artifact_id` without a valid owner link is not evidence support.

An artifact link does not:

- create the owner record
- satisfy a gate by itself
- prove evidence sufficiency
- perform QA
- create final acceptance
- accept residual risk
- close a Task

## Availability, redaction, and integrity

`artifacts.status` is an availability state. The summary table stays short; detail blocks define the value meanings.

| Value | Summary | Details |
|---|---|---|
| `available` | present and integrity-matched | [`available`](#artifacts-status-available) |
| `missing` | row remains, payload missing | [`missing`](#artifacts-status-missing) |
| `integrity_failed` | integrity facts mismatch | [`integrity_failed`](#artifacts-status-integrity_failed) |
| `unavailable` | retrieval path unavailable | [`unavailable`](#artifacts-status-unavailable) |

<a id="artifacts-status-available"></a>
**`artifacts.status=available`**

Storage meaning:

- The registered safe bytes or safe metadata notice is present.
- The stored payload matches stored integrity metadata.

<a id="artifacts-status-missing"></a>
**`artifacts.status=missing`**

Storage meaning:

- The artifact row remains.
- The registered bytes or safe metadata notice cannot be found.

<a id="artifacts-status-integrity_failed"></a>
**`artifacts.status=integrity_failed`**

Storage meaning:

- Available bytes or metadata do not match stored integrity facts such as `sha256` or `size_bytes`.

<a id="artifacts-status-unavailable"></a>
**`artifacts.status=unavailable`**

Storage meaning:

- The artifact store or required retrieval path cannot currently provide the registered bytes or safe metadata notice.

Rule:

- `artifacts.redaction_state` uses the active `ArtifactRef.redaction_state` values from [API Artifact Schemas](api/schema-artifacts.md).
- `sha256`, `size_bytes`, and `content_type` are artifact integrity facts for comparison and availability handling.

Allowed:

- A `blocked`, `secret_omitted`, or `redacted` artifact may still have `artifacts.status=available` when the committed safe notice or redacted bytes are present and integrity-aware.
- `uri` resolves through Harness storage, normally as `harness-artifact://{project_id}/{artifact_id}`.
- Store redacted bytes, `secret_omitted` or `blocked` notices, safe handles, or other owner-approved safe representations instead of unsafe evidence bytes.

Not allowed:

- `blocked` is a redaction/omission state, not an artifact availability status.
- `sha256`, `size_bytes`, and `content_type` are not security guarantee claims.
- `uri` is not a caller-supplied arbitrary filesystem path.
- Raw secrets, tokens, and full sensitive logs must not be stored as evidence bytes.

Owner links:

- Security guarantee claims belong to [Security](security.md).

## Body reads

Artifact body reads are separate from staged artifact promotion. Raw artifact path reads are not granted by default.

Artifact metadata or content reads require:

- a registered `ArtifactRef`
- the matching same-project `task_id`
- the required `artifact_links` owner relation
- the redaction/availability state needed by the caller's access class
- the API/security owner requirements for `access_class=artifact_read`
- any documented surface or connector capability boundary

Not allowed:

- A local path under the artifact store, an artifact `uri`, a staged path, or a copied file is not enough by itself to read or rely on artifact bytes.

## Retention boundary

Allowed:

- Unconsumed or expired `artifact_staging` rows and `artifacts/tmp/` staging bytes or notices may be marked `expired` or `discarded`.
- transient bytes may be cleaned before registration.

Rule:

- These transient staging materials are not evidence authority.
- Once an `artifacts` row is committed, retention purge, project teardown, or destructive cleanup is outside ordinary baseline mutation behavior and needs an owner-defined path.
- A retention or migration path must preserve artifact hashes, owner links, events, and replay rows, or mark affected refs invalid for recovery.

Not allowed:

- A retention or migration path must not silently delete evidence support that current records still name.

## Related owners

- [API Artifact Schemas](api/schema-artifacts.md) for `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` shapes.
- [Stage-artifact method](api/method-stage-artifact.md), [Record-run method](api/method-record-run.md), and [API Methods](api/methods.md) for `harness.stage_artifact`, `harness.record_run`, and artifact read behavior.
- [Storage Effects](storage-effects.md) for whether a response branch creates storage effects.
- [Storage Records](storage-records.md) for `artifact_staging`, `artifacts`, and `artifact_links` table overview.
- [Security](security.md) for access and guarantee non-claims.
