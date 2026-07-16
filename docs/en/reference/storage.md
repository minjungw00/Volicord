# Storage

Use this storage-family router to find the focused storage Reference page for a storage question. Exact storage contracts live in the linked storage owner pages.

This page does not define storage record layouts, SQLite DDL, storage effects, artifact lifecycle, versioning, API shapes, security guarantees, runtime locations, or Core authority semantics.

## Storage Routes

| Need | Owner |
|---|---|
| Records and storage-owned values | [Storage Records](storage-records.md) |
| Baseline SQLite table shape, indexes, foreign keys, constraints, and canonical SQL sources | [Storage DDL](storage-ddl.md) |
| Method or branch storage effects | [Storage Effects](storage-effects.md) |
| Artifact storage lifecycle | [Artifact Storage](storage-artifacts.md) |
| State-version clock, replay, locking, and incompatible storage handling | [Storage Versioning](storage-versioning.md) |
| `baseline_sqlite_v7`, normal-open rejection of v6, and offline read-only v6-to-v7 copy/validation | [Storage Versioning](storage-versioning.md) |
| Project policy copies, session-end receipts, control fields, state-bound ticket records, and separate non-authority workflow metrics | [Storage Records](storage-records.md) and [Storage DDL](storage-ddl.md) |
| Runtime and repository location boundaries | [Runtime Boundaries](runtime-boundaries.md) |

## Nearby Routes

- API method behavior: [API Methods](api/methods.md), then the linked method owner.
- API schema shapes: [API Schema Core](api/schema-core.md) and sibling schema owners.
- Core authority concepts: [Core Model](core-model.md).
- Security wording and guarantee semantics: [Security](security.md).
- API error families: [API Errors](api/errors.md).
- Administrative command, file, and host integration for policy application or
  storage upgrade: [Administrative CLI](admin-cli.md). Storage pages own only
  the database records and conversion effects.
