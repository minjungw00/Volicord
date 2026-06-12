# Storage

This page is a short router for the storage document family. It is documentation reference material only; it does not create a Harness Server, Runtime Home, database, artifact store, migration runner, generated projection, runtime state, or implementation-complete DDL in this repository.

## How to read

The storage family separates conditions, results, exceptions, and non-claims. Do not infer storage effects from API schema shape or user-visible display prose.

- Conditions identify the method branch, record state, or artifact lifecycle stage where a storage rule applies.
- Results identify stored rows, artifacts, replay rows, events, and state-version impact.
- Exceptions identify rejected, dry-run, read-only, or compatibility-failure branches where storage effects are absent or limited.
- Non-claims mark boundaries where response shape, projections, report prose, or path strings do not create storage authority.

## Owns / Does not own

The storage family owns:

- where Harness records persist
- what committed records mean as storage authority
- how method branches affect persistence
- how artifacts move from temporary staging to persistent references
- how project-wide versioning, idempotency, locks, and migrations behave at the storage layer

Storage is authority only for rows or artifact records committed by Core and validated against the owning Core, API, artifact, and storage contracts. Chat, generated Markdown, status cards, projections, connector output, operator output, and report prose are not storage authority.

The storage family does not own:

- API request or response shapes
- public error precedence
- method behavior
- Core lifecycle meaning
- security guarantees
- Runtime Home deployment
- permission claims

Use the [API Methods](api/methods.md), method owner documents, the API schema owners, [Core Model](core-model.md), [API Errors](api/errors.md), [Security](security.md), and [Runtime Boundaries](runtime-boundaries.md) for those contracts.

API data shape and persistence effect are separate. A response field such as `CloseReadinessBlocker[]`, `ArtifactRef`, or `StagedArtifactHandle` describes API data; it does not by itself prove that a row was written, an artifact was promoted, a handle was consumed, or `project_state.state_version` changed.

## Storage owner routes

Use this summary table for first-hop routing. The detail blocks keep the routing conditions out of long table cells.

| Need | Owner |
|---|---|
| Record layout | [Storage Records](storage-records.md) |
| Method effects | [Storage Effects](storage-effects.md) |
| Artifact lifecycle | [Artifact Storage](storage-artifacts.md) |
| Versioning and operational boundaries | [Storage Versioning](storage-versioning.md) |

**Storage Records route**

Use [Storage Records](storage-records.md) for:

- Runtime Home layout.
- Local store assumptions.
- Persisted record categories.
- Table overview.
- Storage-owned record values and status fields.
- Storage-owned JSON.
- Record-level active/out-of-scope exclusions.

**Storage Effects route**

Use [Storage Effects](storage-effects.md) for:

- Read-only effects.
- Dry-run effects.
- Rejected no-effect branches.
- Committed blocked effects.
- Method-by-method persistence effects.

**Artifact Storage route**

Use [Artifact Storage](storage-artifacts.md) for:

- staged artifacts.
- `ArtifactRef`.
- existing artifact links.
- staged-handle promotion.
- artifact body-read boundaries.
- retention and integrity.

**Storage Versioning route**

Use [Storage Versioning](storage-versioning.md) for:

- `project_state.state_version`.
- idempotency and replay rows.
- event meaning.
- locks.
- migrations.

Storage owners describe Harness Runtime Home records only. This documentation repository is not a Runtime Home and must not contain generated runtime state.
