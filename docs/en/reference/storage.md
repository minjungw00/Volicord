# Storage

Use this human-readable storage-family router to find the focused storage owner. For the exact machine-readable owner route, use [`docs/doc-index.yaml`](../../doc-index.yaml).

This page does not define storage record layouts, SQLite DDL, storage effects, artifact lifecycle, versioning, API shapes, security guarantees, runtime locations, or Core authority semantics.

## Storage Routes

| Need | Owner |
|---|---|
| Records and storage-owned values | [Storage Records](storage-records.md) |
| Baseline SQLite table shape, indexes, foreign keys, migration tables, and constraints | [Storage DDL](storage-ddl.md) |
| Method or branch storage effects | [Storage Effects](storage-effects.md) |
| Artifact storage lifecycle | [Artifact Storage](storage-artifacts.md) |
| Versioning, replay, locking, and migrations | [Storage Versioning](storage-versioning.md) |
| Runtime and repository location boundaries | [Runtime Boundaries](runtime-boundaries.md) |

## Nearby Routes

- API method behavior: [API Methods](api/methods.md), then the linked method owner.
- API schema shapes: [API Schema Core](api/schema-core.md) and sibling schema owners.
- Core authority concepts: [Core Model](core-model.md).
- Security wording and guarantee semantics: [Security](security.md).
- API error families: [API Errors](api/errors.md).
