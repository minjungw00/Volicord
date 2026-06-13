# Storage records

This document owns the baseline persistent storage record layout and stored record families. Persistent records are local records committed by Core for later reads inside the `Harness Runtime Home`.

Persistent records are not tamper-proof storage, anti-forgery proof, external audit guarantees, or `Product Repository` write authority.

## Owns / does not own

This document owns:

- baseline persisted record families and where they fit in the local storage model
- record-family layout, stored categories, and table/file placement
- storage-owned value sets and storage-owned JSON `TEXT` placement
- record-level exclusions for unsupported persistent families

This document does not own:

- method-to-storage effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- `project_state.state_version`, idempotency, event meaning, locks, or migrations; see [Storage Versioning](storage-versioning.md)
- API request or response schemas; see [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md)
- API method behavior; see [API Methods](api/methods.md) and the method owner documents
- runtime location and repository boundaries; see [Runtime Boundaries](runtime-boundaries.md)
- security guarantee levels and security boundaries; see [Security](security.md)

## Storage model

Harness stores baseline records in one local `Harness Runtime Home` and one project-local state database per registered project. The default reference root is `~/.harness`; an implementation may choose an equivalent configured root.

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      artifacts/
        tmp/
```

Storage placement:

- `registry.sqlite` stores Runtime Home identity and minimal project registration.
- `projects/{project_id}/` is the Harness project home for one registered project. It is not the same location or authority as `repo_root`.
- `project.yaml` stores static project configuration only.
- `state.sqlite` stores project-local Core state for the registered project.
- `artifacts/` is the project artifact store. `artifacts/tmp/` is transient staging space, not evidence authority.

Runtime Home identity must not depend only on a filesystem path. A copied or moved Runtime Home may carry the same stored `runtime_home_id`, while a newly created Runtime Home gets a new id. The id can help detect suspicious copies, duplicate registrations, or path drift; it is not a security guarantee.

Runtime Home files are local operational control data and may contain sensitive support data. They are not product source files, build outputs, generated projections, acceptance records, residual-risk records, close records, or a substitute for product-repository write authority.

## Records versus API schemas

Storage records and API schemas have different owner boundaries.

- API schema owners define request and response data shape, public API values, public errors, and response branches.
- Storage records define persistent record families, file/table placement, storage-owned JSON `TEXT` placement, stored relationships, and commit-time validation expectations.
- Similar names do not mean shared authority. `ArtifactRef` is an API shape; `artifacts` and `artifact_links` are storage records. `CloseReadinessBlocker` is an API shape; `blockers` is a stored blocker family.
- Rendered status cards, judgment requests, run/evidence summaries, close results, and agent context packets are read-time views. Template prose belongs to [Template Bodies](template-bodies.md), and projection authority belongs to [Projection Authority Reference](projection-and-templates.md).

## Persisted record families

Baseline storage persists only the supported Core record families defined by the baseline storage contract. The exact branches that create, update, observe, or leave records untouched belong to [Storage Effects](storage-effects.md).

Stored families:

- Runtime Home identity in `registry.sqlite`.
- Project registration in `registry.sqlite`.
- Static project configuration in `project.yaml`.
- Project-local state records in `state.sqlite`.
- Artifact metadata, durable artifact bodies, and transient staging bytes under the project artifact store as defined by [Artifact Storage](storage-artifacts.md).

Unsupported persistent families:

- No other persisted table family or transient handle family is baseline scope unless [Scope](scope.md) and the relevant storage owner promote it.
- Unsupported planning records and unsupported auxiliary workflow tables must not be introduced under alternate names.
- Generated projection bodies, expanded evidence packages, QA workflow records, acceptance records, residual-risk records, and close records are not separate baseline storage families.

## Record family overview

This table names the baseline storage record families. It is not full DDL and does not duplicate API schemas, method effects, artifact lifecycle rules, or rendered template bodies.

| Record family | Stored in | Stored category |
|---|---|---|
| Runtime Home identity | `registry.sqlite` | Runtime Home id, schema/storage profile, and local registry metadata. |
| Project registration | `registry.sqlite` | Registered project mapping to `repo_root` and `project_home`. |
| `project.yaml` | project home | Static project configuration for one registered project. |
| `project_state` | `state.sqlite` | Project-local state header, storage profile, state clock field, active Task pointer, and default surface pointer. |
| `surfaces` | `state.sqlite` | Registered local surface facts needed for API envelope compatibility, capability display, and local-access posture. |
| `tasks` | `state.sqlite` | User-value work unit, shaping summary, lifecycle/result/close summary, active `CompletionPolicy`, and active Change Unit pointer. |
| `change_units` | `state.sqlite` | Scoped work boundary, write/close basis, scope summaries, and Change Unit lifecycle. |
| `user_judgments` | `state.sqlite` | Pending and resolved user-owned judgments, including separate sensitive-action approval scope when relevant. |
| `write_authorizations` | `state.sqlite` | Single-use cooperative `Write Authorization` record, basis version, attempt scope, expiration, and consumption state. |
| `runs` | `state.sqlite` | Committed execution or observation records, compatible authorization consumption, and compact evidence updates. |
| `artifact_staging` | `state.sqlite` plus `artifacts/tmp/` | Transient staged artifact handles, safe staging metadata, and transient bytes or notices. |
| `artifacts` | `state.sqlite` plus artifact store | Durable artifact metadata or body location, integrity, redaction, retention, producer, and availability facts. |
| `artifact_links` | `state.sqlite` | Owner relation between an artifact and a supported Core/API record. |
| `evidence_summaries` | `state.sqlite` | Compact evidence coverage, supporting references, and gap references. |
| `blockers` | `state.sqlite` | Structured blocker state for next action, write compatibility, evidence gaps, close readiness, or recovery. |
| `task_events` | `state.sqlite` | Append-only ordering and audit trail for committed Core mutations. |
| `tool_invocations` | `state.sqlite` | Replay rows for committed non-dry-run Core method results when the storage-effect owner creates replay. |

## Record layout rules

### Identity and scope

Baseline records use opaque stable ids as primary keys or equivalent unique keys. Uniqueness is scoped by the owning record family:

- Runtime Home identity stores one `runtime_home_id` for the Runtime Home.
- Project registration requires unique project identity and a unique project home.
- Project-scoped rows belong to a registered project.
- Task-scoped rows belong to the same project and Task as their owning `tasks` row.
- Active pointers, default surface pointers, and owner references must point to same-project records.
- A Task has at most one active Change Unit.
- Single-use relations such as consumed `Write Authorization` rows, consumed staging handles, promoted staged artifacts, artifact owner links, and replay keys must not fork into multiple committed meanings.

### Current rows, event rows, and replay rows

Current record families hold the current Core state for ordinary reads. `task_events` is an append-only ordering and audit trail for committed Core mutations. `tool_invocations` stores committed replay rows only where [Storage Effects](storage-effects.md) says replay is created.

Event meaning, idempotency, replay conflict handling, locks, and migration behavior belong to [Storage Versioning](storage-versioning.md).

### Relationship validation

Storage must validate stored relationships before commit, including:

- same-project and same-Task ownership
- active pointer targets
- compatible `Write Authorization` consumption
- artifact staging consumption and promotion targets
- artifact owner relations
- JSON reference arrays that SQLite cannot express as direct foreign keys

### Deletion boundary

Ordinary baseline Core operations do not hard-delete authority rows. Rows move through status or lifecycle fields, Core appends events, and replay rows plus artifact metadata remain available for audit and recovery.

Closing, cancelling, or superseding a Task must not cascade-delete authority rows such as `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, or `tool_invocations`.

Unconsumed or expired `artifact_staging` rows and `artifacts/tmp/` staging bytes or notices may be marked `expired` or `discarded`, and transient bytes may be cleaned before durable artifact registration. Once an `artifacts` row is committed, retention purge, project teardown, or destructive cleanup needs an owner-defined path.

## Storage-owned values

Closed storage-owned value sets are persistence constraints. Unknown values must not commit.

Storage owns these baseline storage values:

- Project registration `status`: `active`.
- `change_units.status`: `proposed`, `active`, `replaced`, `closed`.
- `write_authorizations.status`: `active`, `consumed`, `expired`, `stale`, `revoked`.
- `artifact_staging.status`: `staged`, `consumed`, `expired`, `discarded`.
- `artifacts.status`: `available`, `missing`, `integrity_failed`, `unavailable`.
- `artifact_links.owner_record_kind`: `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker`.
- `blockers.status`: `active`, `resolved`, `superseded`.
- `tool_invocations.status`: `committed`.

Rows that mirror public API schema values must match the API schema owner exactly. This document does not redefine public API values for fields such as `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result`, `runs.kind`, `runs.status`, `user_judgments.status`, or `evidence_summaries.status`; see [API Value Sets](api/schema-value-sets.md), [API State Schemas](api/schema-state.md), and method owners.

## Storage-owned JSON

SQLite `TEXT` columns that store JSON are a storage representation choice, not permission to persist arbitrary JSON.

Rules:

- Core must parse and validate JSON before commit.
- API-shaped stored JSON validates against the API schema owners.
- Storage-only JSON validates against this document or the owner document named by this document.
- SQLite defaults such as `'{}'` and `'[]'` are storage defaults only; they do not make API fields optional.

Baseline JSON `TEXT` columns store compact owner-shaped data for:

- surface capability profile data
- Task and Change Unit shaping summaries, bounded lists, autonomy boundary, and `CompletionPolicy`
- user-judgment request, context, option, affected-ref, artifact-ref, sensitive-action scope, and resolution data
- `Write Authorization` attempt scope
- run observation and evidence-update data
- evidence coverage and gap references
- blocker owner and related references
- event payloads
- committed replay responses

Task and Change Unit shaping JSON stores compact summaries and bounded lists only. It must not store unsupported planning records, generated projection bodies, expanded evidence-package bodies, QA workflow records, acceptance records, residual-risk records, or close records under another name.

## Baseline / out-of-scope boundary

Profile-gated storage is outside the baseline unless [Scope](scope.md) and the relevant storage owner define a supported contract with fallback behavior and proof-path expectations. Reference-schema presence alone does not make storage supported.

Baseline storage excludes capability families outside the supported scope, generated operational outputs, expanded evidence packages, hosted services, cross-surface orchestration, unsupported planning records, unsupported auxiliary workflow tables, and long-term design-support records.

Status, close readiness, run/evidence summaries, next actions, readable cards, `agent-context-packet`, and guarantee display are read-time derived views over baseline persisted records. They may be stale, absent, failed, or recomputed without changing storage authority.

## Related owners

- [Storage Effects](storage-effects.md) for which methods create, update, observe, or leave records untouched.
- [Artifact Storage](storage-artifacts.md) for artifact-specific storage lifecycle.
- [Storage Versioning](storage-versioning.md) for clocks, idempotency, locks, event meaning, replay, and migration semantics.
- [API Methods](api/methods.md) and method owner documents for public method behavior that uses records.
- [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md) for request/response shape and public API values.
- [Template Bodies](template-bodies.md) for user-visible status cards, judgment requests, run/evidence summaries, close results, and agent context packets.
- [Projection Authority Reference](projection-and-templates.md) for read-only projection authority, source records, and freshness boundaries.
- [Runtime Boundaries](runtime-boundaries.md) for Runtime Home and Product Repository boundaries.
- [Security](security.md) for security boundaries and guarantee levels.
