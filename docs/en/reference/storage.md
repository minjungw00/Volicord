# Storage

This page is a short router for the storage document family. It is documentation source material only; it does not create a Harness Server, Runtime Home, database, artifact store, migration runner, generated projection, runtime state, or implementation-complete DDL in this repository.

## How to read

The storage family separates conditions, results, exceptions, and non-claims. Do not infer storage effects from API schema shape or user-visible display prose.

- Conditions identify the method branch, record state, or artifact lifecycle stage where a storage rule applies.
- Results identify stored rows, artifacts, replay rows, events, and state-version impact.
- Exceptions identify rejected, dry-run, read-only, or compatibility-failure branches where storage effects are absent or limited.
- Non-claims mark boundaries where response shape, projections, report prose, or path strings do not create storage authority.

## Owns / Does not own

The storage family owns:

- where future Harness records persist
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

Use [MVP API](api/mvp-api.md), the API schema owners, [Core Model](core-model.md), [API Errors](api/errors.md), [Security](security.md), and [Runtime Boundaries](runtime-boundaries.md) for those contracts.

API data shape and persistence effect are separate. A response field such as `CloseReadinessBlocker[]`, `ArtifactRef`, or `StagedArtifactHandle` describes API data; it does not by itself prove that a row was written, an artifact was promoted, a handle was consumed, or `project_state.state_version` changed.

## Storage owner routes

| Need | Owner |
|---|---|
| Runtime Home layout, local store assumptions, persisted record categories, table overview, storage-owned JSON, record-level active/later exclusions | [Storage Records](storage-records.md) |
| Read-only effects, dry-run effects, rejected no-effect branches, committed blocked effects, method-by-method persistence effects | [Storage Effects](storage-effects.md) |
| Staged artifacts, `ArtifactRef`, existing artifact links, staged-handle promotion, artifact body-read boundaries, retention, integrity | [Artifact Storage](storage-artifacts.md) |
| `project_state.state_version`, idempotency and replay rows, event meaning, locks, migrations | [Storage Versioning](storage-versioning.md) |

Storage owners describe future Harness Runtime Home records only. This documentation repository is not a Runtime Home and must not contain generated runtime state.
