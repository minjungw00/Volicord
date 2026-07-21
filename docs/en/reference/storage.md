# Storage

Use this storage-family router to find the focused storage Reference page for a storage question. Exact storage contracts live in the linked storage owner pages.

Except for repeating the current canonical manifest identity below for owner
routing, this page does not define storage record layouts, SQLite DDL, storage
effects, artifact lifecycle, API shapes, security guarantees, runtime
locations, or Core authority semantics.

## Current Canonical Manifest Identity

```yaml
contract_id: volicord.sqlite.canonical
enabled_capabilities:
  - artifact_storage
  - authority_event_chain
  - exact_operation_result
  - guard_reconciliation
  - managed_codex_connection
  - operational_mcp_sessions
  - project_continuity
  - user_action_cli_resolution
```

The values and order are exact. Unknown, missing, reordered, subset, or
noncurrent values are invalid; no default or conversion applies.
[Storage Versioning](storage-versioning.md#storagemanifest) owns the complete
shape, digests, validation, and failure classification.

## Storage Routes

| Need | Owner |
|---|---|
| Records and storage-owned values | [Storage Records](storage-records.md) |
| Baseline SQLite table shape, indexes, foreign keys, constraints, and canonical SQL sources | [Storage DDL](storage-ddl.md) |
| Method or branch storage effects | [Storage Effects](storage-effects.md) |
| Artifact storage lifecycle | [Artifact Storage](storage-artifacts.md) |
| State-version clock, replay, locking, and incompatible storage handling | [Storage Versioning](storage-versioning.md) |
| The single canonical SQLite contract, exact manifest validation, and unsupported-format rejection | [Storage Versioning](storage-versioning.md) |
| Project policy copies, control fields, state-bound ticket records, and separate non-authority workflow metrics | [Storage Records](storage-records.md) and [Storage DDL](storage-ddl.md) |
| Runtime and repository location boundaries | [Runtime Boundaries](runtime-boundaries.md) |

## Nearby Routes

- API method behavior: [API Methods](api/methods.md), then the linked method owner.
- API schema shapes: [API Schema Core](api/schema-core.md) and sibling schema owners.
- Core authority concepts: [Core Model](core-model.md).
- Security wording and guarantee semantics: [Security](security.md).
- API error families: [API Errors](api/errors.md).
- Administrative command, file, and host integration for policy application:
  [Administrative CLI](admin-cli.md).
