# Storage records

This document owns the baseline persistent storage record families, placement, and storage-owned values. Persistent records are local records committed by Core for later reads inside the `Harness Runtime Home`.

Persistent records are the local Core storage authority for Harness records. Security guarantees, external audit guarantees, anti-forgery claims, and `Product Repository` write authority remain with their owners.

## Owner boundaries

This document owns:

- baseline persisted record families
- table, file, and artifact-store placement for those families
- stored categories and relationship layout
- storage-owned value sets
- storage-owned SQLite JSON `TEXT` placement
- record-layout validation requirements before commit

This document does not own:

- baseline SQLite DDL, indexes, foreign keys, migration tables, or constraints; see [Storage DDL](storage-ddl.md)
- method branch persistence effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- `project_state.state_version`, idempotency, replay, events, lock, and migration contracts; see [Storage Versioning](storage-versioning.md)
- API request or response shape; see [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md)
- API method behavior; see [API Methods](api/methods.md) and the method owner documents
- runtime location and repository boundaries; see [Runtime Boundaries](runtime-boundaries.md)
- security guarantee levels and security boundaries; see [Security](security.md)

## Storage locations

Harness stores baseline records in one local `Harness Runtime Home` and one project-local state database per registered project. The default reference root is `~/.harness`; an implementation may choose an equivalent configured root.

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      state.sqlite
      artifacts/
        tmp/
```

Storage placement:

- `registry.sqlite` stores Runtime Home identity, project registration mapping, Agent Integration Profile records, integration project membership, Host Installation inventory, and registry metadata. Project registration includes the registered `repo_root`, `project_home`, project `state.sqlite` path, and status.
- `projects/{project_id}/` is the Harness project home for one registered project. It is not the same location or authority as `repo_root`.
- `state.sqlite` stores project-local Core state for the registered project.
- `artifacts/` is the project artifact store when artifact storage is used. `artifacts/tmp/` is transient staging space when artifact staging requires it, not evidence authority. These directories need not exist immediately after project registration.

Artifact path bases:

- `artifact_staging.tmp_path` is stored relative to `project_home`; staged bytes or notices under the transient staging area use a shape such as `artifacts/tmp/<file>`.
- `artifacts.body_path` is stored relative to the artifact-store root, normally `project_home/artifacts`; persistent bodies use a shape such as `tmp/<file>` and are resolved as `artifact_store_root.join(body_path)`.

For project execution, `project_home` is the location owner for project-local runtime state. The executable project state database path is derived from the validated project home as `project_home/state.sqlite`. The stored `state_db_path` remains in `registry.sqlite` for persistence and diagnostics, but it must match that derived path before Store execution opens, inspects, migrates, or uses project-local state. A mismatching registration remains readable through registry-level lookup and listing for diagnosis, but it is not eligible for Core execution, surface management, setup reuse, or MCP project startup.

Baseline SQLite table shape, indexes, foreign keys, migration tables, and constraints belong to [Storage DDL](storage-ddl.md). The current baseline SQLite storage profile for these records is `baseline_sqlite_v2`; profile/version boundary behavior belongs to [Storage Versioning](storage-versioning.md).

Runtime Home identity must not depend only on a filesystem path. A copied or moved Runtime Home may carry the same stored `runtime_home_id`, while a newly created Runtime Home gets a new id. The id can help detect suspicious copies, duplicate registrations, or path drift; it is not a security guarantee.

## API schemas versus storage records

API schema shape and storage record layout have separate owners.

- API schema owners define request and response data shape and response branches. The [API Value Sets](api/schema-value-sets.md) owner defines public API values, and [API error codes](api/error-codes.md) defines public `ErrorCode` identifiers and meanings.
- This document defines what the baseline storage contract persists: record families, placement, stored categories, relationship layout, storage-owned values, and storage-owned JSON `TEXT`.
- Similar names do not create shared authority. `ArtifactRef` is an API shape; `artifacts` and `artifact_links` are storage records. `CloseReadinessBlocker` shape belongs to [API State Schemas](api/schema-state.md); `blockers` is a storage record family.
- A response shape does not prove persistence. The selected method branch and [Storage Effects](storage-effects.md) define whether a call creates, updates, observes, or leaves records untouched.
- Rendered status cards, judgment prompts, run/evidence summaries, close-readiness output, and agent context packets are read-time views over records. Template prose belongs to [Template Bodies](template-bodies.md), and projection authority belongs to [Projection Authority Reference](projection-and-templates.md).

## Persisted record families and categories

Baseline storage persists only the record families defined by this baseline storage contract. Any other durable record family requires [Scope](scope.md) and the affected storage owner to define support.

| Stored area | Record family | Stored category | Layout summary |
|---|---|---|---|
| `registry.sqlite` | Runtime Home identity | Runtime identity | One stored `runtime_home_id`, schema/storage profile, and local registry metadata. |
| `registry.sqlite` | Project registration | Project mapping | Registered project identity mapped to `repo_root`, location-owning `project_home`, and stored `state_db_path` that must match `project_home/state.sqlite` for execution. |
| `registry.sqlite` | Agent Integration Profile | Coding-agent integration binding | Durable integration identity, interaction role, bound surface identifiers, enabled state, optional default project, and integration metadata. |
| `registry.sqlite` | Integration project membership | Integration project allowlist | Explicit many-to-many membership between an Agent Integration Profile and registered projects. |
| `registry.sqlite` | Host Installation | Host setup inventory | Host kind, host scope, server name, config target, managed fingerprint, last verification status, and installation metadata for a configured or exported coding-agent host entry. |
| `state.sqlite` | `project_state` | Project state header | Storage profile, `state_version`, current `Task` pointer, default surface pointer, and project enforcement profile. |
| `state.sqlite` | `surfaces` | Surface facts | Registered local surface facts needed for API envelope compatibility, actor-provenance role, capability display, and local-access posture. |
| `state.sqlite` | `tasks` | Work-unit state | User-value work unit, shaping summary, scope and close-basis revisions, nullable current close basis, lifecycle/result/terminal close summary, current `CompletionPolicy`, and current Change Unit pointer. |
| `state.sqlite` | `change_units` | Scoped work boundary | Scope summaries, write basis, Change Unit lifecycle, and owning `Task` relation. |
| `state.sqlite` | `user_judgments` | User-owned judgment state | Pending, resolved, stale, superseded, and expired user-owned judgments, including required basis snapshot, basis status, selected option, machine action, resolution outcome, resolution actor, verified actor provenance for resolved rows, and sensitive-action approval scope when relevant. |
| `state.sqlite` | `write_authorizations` | Cooperative write authority | Single-use `Write Authorization`, basis version, attempt scope, expiration, and consumption state. |
| `state.sqlite` | `runs` | Execution or observation record | Committed execution or observation record, compatible authorization consumption, and compact evidence updates. |
| `state.sqlite` plus `artifacts/tmp/` | `artifact_staging` | Transient artifact staging | Staged handle metadata, safe staging facts, and transient bytes or notices. |
| `state.sqlite` plus artifact store | `artifacts` | Persistent artifact record | Durable artifact metadata or body location, content type, SHA-256, size, integrity status, redaction, retention, producer, and availability facts. |
| `state.sqlite` | `artifact_links` | Artifact owner relation | Owner relation between an artifact and a baseline Core/API record family. |
| `state.sqlite` | `evidence_summaries` | Evidence summary | Compact evidence coverage, supporting references, and gap references. |
| `state.sqlite` | `blockers` | Blocker state | Structured blocker state for next action, write compatibility, evidence gaps, close readiness, or recovery. |
| `state.sqlite` | `task_events` | Event trail | Append-only ordering and audit trail for committed Core mutations. |
| `state.sqlite` | `tool_invocations` | Replay row | Replay rows for committed non-dry-run Core method results when [Storage Effects](storage-effects.md) says replay is created. |

## Record layout rules

### Identity and ownership

Baseline records use opaque stable ids as primary keys or equivalent unique keys. Uniqueness is scoped by the owning record family:

- Runtime Home identity stores one `runtime_home_id` for the Runtime Home.
- Project registration requires unique project identity and a unique project home.
- Agent Integration Profile identity is unique by `integration_id` and binds one registered coding-agent surface identity for adapter calls.
- Integration project membership is unique by `(integration_id, project_id)`. A profile default project, when present, must also be present in that membership set.
- Host Installation inventory is unique for the managed host target, host scope, and server name. It records managed setup state; it is not the external host configuration source of truth.
- Project-scoped rows belong to a registered project.
- Task-scoped rows belong to the same project and `Task` as their owning `tasks` row.
- Current pointers, default surface pointers, and owner references must point to same-project records.
- A `Task` has at most one current Change Unit.
- Single-use relations such as consumed `Write Authorization` rows, consumed staging handles, promoted staged artifacts, artifact owner links, and replay keys must not fork into multiple committed meanings.

### Current, event, and replay rows

Current record families hold the current Core state for ordinary reads. `task_events` is an append-only ordering and audit trail for committed Core mutations. `tool_invocations` stores committed replay rows only where [Storage Effects](storage-effects.md) says replay is created.

State-version behavior, idempotency, event meaning, replay conflict handling, locks, and migration contracts belong to [Storage Versioning](storage-versioning.md).

### Relationship validation

Storage must validate stored relationships before commit, including:

- same-project and same-`Task` ownership
- active pointer targets
- compatible `Write Authorization` consumption
- artifact staging consumption and promotion targets
- artifact owner relations
- Agent Integration Profile default-project membership and enabled-state consistency
- Host Installation references to an existing Agent Integration Profile
- JSON reference arrays that SQLite cannot express as direct foreign keys

### Authority row preservation

Ordinary baseline Core operations preserve authority rows through lifecycle or status transitions. Completing, cancelling, or superseding a `Task` changes the relevant lifecycle/status meaning while keeping committed authority rows addressable for audit and recovery.

This preservation applies to `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, and `tool_invocations`. Artifact-specific transient and durable retention rules belong to [Artifact Storage](storage-artifacts.md).

### Current close basis

The current close basis is Task-owned current state stored with the `tasks` family. It is distinct from the terminal close summary stored for a successful terminal close result.

The authoritative current `CurrentCloseBasis` record is `tasks.close_basis_json`, interpreted with the Task-owned close-basis coordinates.

Existing open Tasks do not automatically convert terminal close summary JSON into a current close basis. Absence of a current close basis is represented as absence in `tasks.close_basis_json`, not as an empty generated basis. Change Unit records do not store or satisfy current `CurrentCloseBasis` authority.

Stored judgments require a `JudgmentBasis`. Resolved stored judgments require a complete machine-readable resolution, actor provenance, and verified resolved surface provenance. Rows missing those facts are invalid owner state, not audit-compatible authority records.

For stored judgment authority, `user_judgments.status='resolved'` records that an answer exists. It does not mean the user approved. Current authority-bearing judgment use requires the selected option, stored `resolution_machine_action`, stored `resolution_outcome`, applicable actor provenance, and applicable verified resolved surface provenance. Absence of an outcome, machine action, applicable actor provenance, or verified resolved surface provenance is invalid owner state and is never acceptance.

## Storage-owned values

Closed storage-owned value sets are persistence constraints. Unknown values must not commit.

| Stored field | Baseline values |
|---|---|
| Project registration `status` | `active` |
| Agent Integration Profile `interaction_role` | `agent` |
| Agent Integration Profile `enabled` | `0`, `1` |
| Host Installation `host_kind` | `codex`, `claude_code`, `generic` |
| Host Installation `host_scope` | `user`, `project`, `local`, `export` |
| Host Installation `last_verified_status` | `not_verified`, `complete`, `action_required`, `partial_failure`, `failed` |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` |
| `write_authorizations.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `user_judgments.basis_status` | `current`, `stale`, `superseded` |
| `user_judgments.resolution_machine_action` | `accept`, `reject`, `defer` in complete resolution groups |
| `user_judgments.resolution_outcome` | `accepted`, `rejected`, `deferred` in complete resolution groups |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` |
| `artifacts.integrity_status` | `verified`, `corrupt` |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` |
| `blockers.status` | `active`, `resolved`, `superseded` |
| `tool_invocations.status` | `committed` |

Rows that mirror public API values must match [API Value Sets](api/schema-value-sets.md), the relevant schema owner, and the method owner exactly. This document does not redefine public API values for fields such as `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result`, `runs.kind`, `runs.status`, `user_judgments.status`, or `evidence_summaries.status`; see [API Value Sets](api/schema-value-sets.md), [API State Schemas](api/schema-state.md), and method owners.

## Storage-owned JSON

SQLite `TEXT` columns that store JSON are a storage representation choice, not permission to persist arbitrary JSON.

Rules:

- Core must parse and validate JSON before commit.
- API-shaped stored JSON validates against the API schema owners.
- Storage-only JSON validates against this storage contract or the referenced storage owner.
- SQLite defaults such as `'{}'` and `'[]'` are storage defaults only; they do not make API fields optional.

| Record family | JSON `TEXT` category |
|---|---|
| Agent Integration Profile | Integration metadata that is not used as authority, access grant, project selection, or host trust proof. |
| Host Installation | Installation metadata that is not used as authority, host trust proof, or replacement for the external host configuration. |
| `surfaces` | Surface capability profile data. |
| `tasks` | Shaping summary, bounded lists, autonomy boundary, current close basis, terminal close summary, lifecycle summary, and `CompletionPolicy`. |
| `change_units` | Scope summaries, bounded lists, write basis summaries, and lifecycle support data. |
| `user_judgments` | Judgment request, context, option, affected-ref, artifact-ref, basis snapshot, sensitive-action scope, selected option, machine action, resolution outcome, actor provenance, and resolution data. |
| `write_authorizations` | `Write Authorization` attempt scope. |
| `runs` | Observation and evidence-update data. |
| `evidence_summaries` | Evidence coverage and gap references. |
| `blockers` | Blocker owner references and related references. |
| `task_events` | Event payloads for committed Core mutations. |
| `tool_invocations` | Committed replay responses. |

Task and Change Unit shaping JSON stores compact summaries and bounded lists only. It does not create an additional persisted record family.

## Related owners

- [Storage Effects](storage-effects.md) defines which method branches create, update, observe, or leave records untouched.
- [Storage DDL](storage-ddl.md) defines baseline SQLite table shape, indexes, foreign keys, migration tables, and constraints.
- [Artifact Storage](storage-artifacts.md) defines artifact staging, promotion, linking, body reads, retention, and integrity lifecycle.
- [Storage Versioning](storage-versioning.md) defines state versioning, idempotency, replay, events, locks, and migration contracts.
- [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md) define API shape and public API values.
- [API Methods](api/methods.md) and method owner documents define public method behavior that uses records.
- [Runtime Boundaries](runtime-boundaries.md) defines `Product Repository`, Harness installation or runtime process, and `Harness Runtime Home` location boundaries.
- [Projection Authority Reference](projection-and-templates.md) and [Template Bodies](template-bodies.md) define read-time projection authority and rendered template bodies.
- [Security](security.md) defines security boundaries and guarantee levels.
