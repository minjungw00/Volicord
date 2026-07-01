# Storage Records

This document owns the baseline persistent storage record families, placement, relationship layout, storage-owned values, and storage-owned JSON placement. Persistent records are local records committed for later reads inside the `Volicord Runtime Home`.

Persistent records are the local Core storage authority for Volicord records. Security guarantees, external audit guarantees, anti-forgery claims, and `Product Repository` file write authority remain with their owners.

## Owner Boundaries

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

## Storage Locations

Volicord stores baseline records in one local `Volicord Runtime Home` and one project-local state database per registered project. `volicord setup` establishes the selected Runtime Home and installation profile; ordinary user flows do not need to provide the Runtime Home path again.

The tree is representative after the relevant storage features have been used; it is not an initial-directory checklist. Project registration creates or opens project state, while artifact-store directories may be created lazily.

```text
~/.volicord/
  registry.sqlite
  projects/
    prj_<internal>/
      state.sqlite
      artifacts/        # created when artifact storage is used
        tmp/            # created when artifact staging occurs
```

Storage placement:

- `registry.sqlite` stores Runtime Home identity, installation profile records, project registration mapping, project aliases, Agent Connection records, Connection Projects membership, guard installation records, and registry metadata. The installation profile includes the setup-time `volicord` command, MCP launch command, bin directory, default connection mode, metadata, and timestamps. Project registration includes `project_internal_id`, display name, CLI selection alias, Runtime Home relationship, registered `repo_root`, `project_home`, project `state.sqlite` path, status, metadata, and timestamps.
- `projects/{project_internal_id}/` is the default Volicord project home shape for one registered project. It is not the same location or authority as `repo_root`.
- `state.sqlite` stores project-local Core state and project-scoped guarded-operation records for the registered project.
- `artifacts/` is the project artifact store when artifact storage is used; it may be created lazily when artifact storage is first needed. `artifacts/tmp/` is transient staging space when artifact staging requires it, not evidence authority; it may be created lazily when staging occurs. These directories need not exist immediately after project registration.

Artifact path bases:

- `artifact_staging.tmp_path` is stored relative to `project_home`; staged bytes or notices under the transient staging area use a shape such as `artifacts/tmp/<file>`.
- `artifacts.body_path` is stored relative to the artifact-store root, normally `project_home/artifacts`; persistent bodies use a shape such as `tmp/<file>` and are resolved as `artifact_store_root.join(body_path)`.

For operational project records, `project_home` is the location owner for project-local runtime state. The executable project state database path is derived from the validated project home as `project_home/state.sqlite`. The stored `state_db_path` remains in `registry.sqlite` for persistence and diagnostics, but it must match that derived path before Store returns a normal `ProjectRecord`, opens or migrates project-local state, resolves Agent Connection project access, enters Core execution, or reports MCP project availability. A mismatching registration remains inspectable as raw registry content for diagnosis, but operational lookup and listing must reject it rather than omit it or return it as a normal project. Inspection must not open, create, migrate, or repair the alternate `state_db_path`.

The `Product Repository` is the user product-file boundary registered by `repo_root`. It is not a Volicord runtime home, not Core authority storage, and not where runtime records, replay rows, judgments, Write Checks, guard records, or Agent Connection registry state are stored.

Baseline SQLite table shape, indexes, foreign keys, migration tables, and constraints belong to [Storage DDL](storage-ddl.md). The current baseline SQLite storage profile for these records is `baseline_sqlite_v3`; profile/version boundary behavior belongs to [Storage Versioning](storage-versioning.md).

Runtime Home identity must not depend only on a filesystem path. A copied or moved Runtime Home may carry the same stored `runtime_home_id`, while a newly created Runtime Home gets a new id. The id can help detect suspicious copies, duplicate registrations, or path drift; it is not a security guarantee.

## API Schemas Versus Storage Records

API schema shape and storage record layout have separate owners.

- API schema owners define request and response data shape and response branches. The [API Value Sets](api/schema-value-sets.md) owner defines public API values, and [API error codes](api/error-codes.md) defines public `ErrorCode` identifiers and meanings.
- This document defines what the baseline storage contract persists: record families, placement, stored categories, relationship layout, storage-owned values, and storage-owned JSON `TEXT`.
- Similar names do not create shared authority. `ArtifactRef` is an API shape; `artifacts` and `artifact_links` are storage records. `CloseReadinessBlocker` shape belongs to [API State Schemas](api/schema-state.md); `blockers` is a storage record family.
- A response shape does not prove persistence. The selected method branch and [Storage Effects](storage-effects.md) define whether a call creates, updates, observes, or leaves records untouched.
- Rendered status cards, judgment prompts, run/evidence summaries, close-readiness output, and agent context packets are read-time views over records. Template prose belongs to [Template Bodies](template-bodies.md), and projection authority belongs to [Projection Authority Reference](projection-and-templates.md).

## Persisted Record Families

Baseline storage persists only the record families defined by this baseline storage contract. Any other durable record family requires [Scope](scope.md) and the affected storage owner to define support.

| Stored area | Record family | Stored category | Layout summary |
|---|---|---|---|
| `registry.sqlite` | Runtime Home identity | Runtime identity | One stored `runtime_home_id`, Runtime Home path, registry database path, schema/storage profile, metadata, and timestamps. |
| `registry.sqlite` | Installation profile | Setup-time executable profile | Setup-time `volicord` command, MCP launch command, bin directory, default connection mode, metadata, and timestamps established by `volicord setup`. |
| `registry.sqlite` | Project registration and aliases | Project mapping | `project_internal_id`, display name, CLI selection alias, Runtime Home relationship, unique `repo_root`, location-owning `project_home`, stored `state_db_path` that must match `project_home/state.sqlite` for execution, status, metadata, and alias-to-internal-identity mappings. |
| `registry.sqlite` | Agent Connection | MCP host connection unit | Durable `connection_internal_id`, host kind, connection intent, host scope, optional `project_internal_id`, internal server name, config target, mode, enabled state, managed fingerprint, verification summary status, verification report JSON, user actions JSON, metadata, and timestamps. |
| `registry.sqlite` | Connection Projects | Connection project allowlist | Explicit many-to-many membership between an Agent Connection and registered projects using `connection_internal_id` and `project_internal_id`. |
| `registry.sqlite` | Guard installation | Guard setup and host capability record | Runtime Home, Agent Connection, optional project scope, host kind, guard mode, host capability JSON, installation health, timestamps, and metadata. |
| `state.sqlite` | `project_state` | Project state header | Storage profile, `state_version`, current `Task` pointer, and project enforcement profile. |
| `state.sqlite` | `agent_sessions` | Guarded Agent Session | Project-scoped session for one Agent Connection, optional guard installation, host kind, guard mode, start/end timestamps, and metadata. |
| `state.sqlite` | `guard_events` | Guard decision event | Project-scoped guard event tied to a connection and optional session or installation, with decision, subject JSON, result JSON, timestamp, and metadata. |
| `state.sqlite` | `prompt_captures` | Prompt capture | Project-scoped prompt capture for a session, including connection, capture kind, prompt hash, optional prompt text, timestamp, and metadata. |
| `state.sqlite` | `expected_writes` | Expected Product Repository write | Project-scoped expected-write correlation record created by an allowed guarded pre-tool write, with connection/session identity, optional host invocation identity, exact path policy, active task/change-unit/write-check basis, timestamps, and matched post-tool metadata. |
| `state.sqlite` | `unrecorded_changes` | Unrecorded Product Repository change | Project-scoped unresolved or resolved record for observed Product Repository changes that are not yet matched to a Core run or other owner-defined record. |
| `state.sqlite` | `tasks` | Work-unit state | User-value work unit, shaping summary, scope and close-basis revisions, nullable current close basis, lifecycle/result/terminal close summary, current `CompletionPolicy`, current Change Unit pointer, and creator actor source. |
| `state.sqlite` | `change_units` | Scoped work boundary | Scope summaries, write basis, Change Unit lifecycle, and owning `Task` relation. |
| `state.sqlite` | `user_judgments` | User-owned judgment state | Pending, resolved, stale, superseded, and expired user-owned judgments, including basis snapshot, request context, options, sensitive-action scope, resolution machine action and outcome, rationale metadata, User Channel actor source, verification basis, and assurance level. |
| `state.sqlite` | `project_continuity_records` | Project continuity context | Durable project-level decisions, obligations, known limits, accepted residual risks, and constraints that remain addressable after the source `Task` closes. |
| `state.sqlite` | `write_checks` | Core-state write compatibility | Single-use Write Check, basis version, attempt scope, expiration, actor source, optional originating judgment, and consumption state. |
| `state.sqlite` | `runs` | Execution or observation record | Committed execution or observation record, optional compatible Write Check consumption, actor source, and compact evidence updates. |
| `state.sqlite` plus `artifacts/tmp/` | `artifact_staging` | Transient artifact staging | Staged handle metadata, creator actor source, safe staging facts, and transient bytes or notices. |
| `state.sqlite` plus artifact store | `artifacts` | Persistent artifact record | Durable artifact metadata or body location, content type, SHA-256, size, integrity status, redaction, retention, producer, and availability facts. |
| `state.sqlite` | `artifact_links` | Artifact owner relation | Owner relation between an artifact and a baseline Core/API record family. |
| `state.sqlite` | `evidence_summaries` | Evidence summary | Compact evidence coverage, supporting references, and gap references. |
| `state.sqlite` | `evidence_observations` | Evidence observation | Durable provenance record for one reported or observed evidence claim, including source kind, assurance level, observer actor source, tool metadata, input refs, output artifact refs, limitations, and timestamps. |
| `state.sqlite` | `blockers` | Blocker state | Structured blocker state for next action, write compatibility, evidence gaps, close readiness, or recovery. |
| `state.sqlite` | `task_events` | Event trail | Append-only ordering and audit trail for committed Core mutations. |
| `state.sqlite` | `tool_invocations` | Replay row | Replay rows for committed non-dry-run Core method results when [Storage Effects](storage-effects.md) says replay is created, including actor source and operation category. |

## Record Layout Rules

### Identity And Ownership

Baseline records use opaque stable ids as primary keys or equivalent unique keys. Uniqueness is scoped by the owning record family:

- Runtime Home identity stores one `runtime_home_id` for the Runtime Home.
- Project registration requires a unique `project_internal_id`, unique project alias, unique repository root, unique project home, and unique state database path. `project_name` is the display name and `project_alias` is the CLI selection aid.
- Agent Connection identity is unique by `connection_internal_id`.
- Connection Projects membership is unique by `connection_internal_id` and `project_internal_id`, and is the only registry membership that lets one connection address a registered project.
- Guard installation identity is unique by `guard_installation_id`. Project-scoped guard installations must name a registered project and an Agent Connection that has Connection Projects membership for that project.
- Project-scoped rows belong to a registered project.
- Guard sessions, guard events, prompt captures, expected writes, and unrecorded changes belong to one project-local `state.sqlite` and name the Agent Connection that observed or produced the record.
- Task-scoped rows belong to the same project and `Task` as their owning `tasks` row.
- Current pointers and owner references must point to same-project records.
- A `Task` has at most one current Change Unit.
- Single-use relations such as consumed Write Check rows, consumed staging handles, promoted staged artifacts, artifact owner links, and replay keys must not fork into multiple committed meanings.

### Current, Event, And Replay Rows

Current record families hold the current Core state for ordinary reads. `task_events` is an append-only ordering and audit trail for committed Core mutations. `tool_invocations` stores committed replay rows only where [Storage Effects](storage-effects.md) says replay is created.

State-version behavior, idempotency, event meaning, replay conflict handling, locks, and migration contracts belong to [Storage Versioning](storage-versioning.md).

### Relationship Validation

Storage must validate stored relationships before commit, including:

- same-project and same-`Task` ownership
- active pointer targets
- compatible Write Check consumption
- artifact staging consumption and promotion targets
- artifact owner relations
- Connection Projects membership and enabled-state consistency for Agent Connection routing
- guard installation, Agent Session, guard event, prompt capture, expected-write, and unrecorded-change project and connection scope
- JSON reference arrays that SQLite cannot express as direct foreign keys

### Authority Row Preservation

Ordinary baseline Core operations preserve authority rows through lifecycle or status transitions. Completing, cancelling, or superseding a `Task` changes the relevant lifecycle/status meaning while keeping committed authority rows addressable for audit and recovery.

This preservation applies to `tasks`, `change_units`, `user_judgments`, `project_continuity_records`, `write_checks`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `evidence_observations`, `blockers`, `task_events`, `tool_invocations`, `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, and `unrecorded_changes`. Artifact-specific transient and durable retention rules belong to [Artifact Storage](storage-artifacts.md).

### Guarded Operation Records

Guarded-operation records preserve local authority facts about host integration state. They can help Core and Store code determine whether work can honestly proceed or close, but they are not OS-level sandboxing, filesystem ACLs, external policy enforcement, anti-forgery proof, or proof that a write was prevented.

`guard_installations` records setup lifecycle state, observed hook metadata, and host capability by Runtime Home, Agent Connection, and optional project scope. `configured` and `reload_required` mean files or metadata are installed but no matching guard hook has yet been observed. `active` means Volicord observed a valid guard hook for the recorded project, Agent Connection, host kind, guard mode, and policy hash; it does not prove OS-level enforcement or sandboxing. `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, and `unrecorded_changes` are project-local rows and must not leak across project `state.sqlite` databases. A pending `expected_writes` row means guarded pre-tool allowed a concrete expected write for bounded project, connection, session, time, path, task, Change Unit, and Write Check coordinates. A matched row means post-tool observation was correlated to that expected write; it is not proof of product correctness. An unresolved `unrecorded_changes` row means an observed Product Repository change still needs owner-defined reconciliation. Resolving the row records the local resolution basis, actor source, capture basis, resolution timestamp, and optional linked user judgment while preserving the row.

### Current Close Basis

The current close basis is Task-owned current state stored with the `tasks` family. It is distinct from the terminal close summary stored for a successful terminal close result.

The authoritative current `CurrentCloseBasis` record is `tasks.close_basis_json`, interpreted with the Task-owned close-basis coordinates.

Existing open Tasks do not automatically convert terminal close summary JSON into a current close basis. Absence of a current close basis is represented as absence in `tasks.close_basis_json`, not as an empty generated basis. Change Unit records do not store or satisfy current `CurrentCloseBasis` authority.

Stored judgments require a `JudgmentBasis`. Resolved stored judgments require a complete machine-readable resolution, structured descriptive rationale metadata, actor provenance, verification basis, and assurance level. Rows missing those facts are invalid owner state, not audit-compatible authority records.

For stored judgment authority, `user_judgments.status='resolved'` records that an answer exists. It does not mean the user approved. Current authority-bearing judgment use requires the selected option, stored `resolution_machine_action`, stored `resolution_outcome`, applicable User Channel actor provenance, and method-owned compatibility. Rationale metadata preserves the reason and context for the answer but is not itself authority, evidence, acceptance, close readiness, or residual-risk acceptance. Absence of an outcome, machine action, applicable actor provenance, verification basis, or assurance level is invalid owner state and is never acceptance.

### Project Continuity Records

`project_continuity_records` preserve durable project-level context from committed Core effects. Baseline records may represent decisions, obligations, known limits, accepted residual risks, or constraints.

The source `Task` and optional source Change Unit identify where the continuity record came from. They do not make that source path current again. `status='active'` keeps the record visible as live project context, while `superseded` and `closed` keep the record addressable for audit and recovery.

Project continuity records are not current authority for a new operation. A future write, Run, judgment requirement, close readiness check, final acceptance, residual-risk acceptance, or blocker decision must still use the current owner-defined Core state and compatibility rules.

## Storage-Owned Values

Closed storage-owned value sets are persistence constraints. Unknown values must not commit.

| Stored field | Baseline values |
|---|---|
| Project registration `status` | `active` |
| Agent Connection `host_kind` | `codex`, `claude_code`, `generic` |
| Agent Connection `intent` | `personal`, `shared`, `global` |
| Agent Connection `host_scope` | `user`, `project`, `local`, `export` according to the `host_kind` matrix |
| Agent Connection `mode` | `workflow`, `read_only` |
| Agent Connection `enabled` | `0`, `1` |
| Agent Connection `last_verification_status` | `not_verified`, `complete`, `action_required`, `failed` |
| Guard installation `guard_mode` | `mcp_only`, `guarded`, `managed` |
| Guard installation `installation_status` | `absent`, `configured`, `reload_required`, `active`, `degraded`, `stale`, `broken` |
| `agent_sessions.guard_mode` | `mcp_only`, `guarded`, `managed` |
| `guard_events.decision` | `allow`, `deny`, `warn`, `inject_context` |
| `expected_writes.path_policy` | `exact_paths` |
| `expected_writes.status` | `pending`, `matched` |
| `unrecorded_changes.status` | `unresolved`, `resolved` |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` |
| `write_checks.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `user_judgments.status` | `pending`, `resolved`, `stale`, `superseded`, `expired` |
| `user_judgments.basis_status` | `current`, `stale`, `superseded` |
| `user_judgments.resolution_machine_action` | `accept`, `reject`, `defer` in complete resolution groups |
| `user_judgments.resolution_outcome` | `accepted`, `rejected`, `deferred` in complete resolution groups |
| `project_continuity_records.kind` | `decision`, `obligation`, `known_limit`, `accepted_risk`, `constraint` |
| `project_continuity_records.status` | `active`, `superseded`, `closed` |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` |
| `artifacts.integrity_status` | `verified`, `corrupt` |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `evidence_observation`, `blocker` |
| `evidence_observations.source_kind` | `agent_report`, `connection_observation`, `external_tool`, `user_observation`, `reused_evidence`, `unverified_claim` |
| `evidence_observations.assurance_level` | `cooperative_report`, `registered_connection_observed`, `external_tool_result`, `user_observed`, `unverified` |
| `blockers.status` | `active`, `resolved`, `superseded` |
| `tool_invocations.status` | `committed` |
| `tool_invocations.operation_category` | `read`, `agent_workflow`, `user_only`, `admin_local` |

Rows that mirror public API values must match [API Value Sets](api/schema-value-sets.md), the relevant schema owner, and the method owner exactly. This document does not redefine public API values for fields such as `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result`, `runs.kind`, `runs.status`, or `evidence_summaries.status`; see [API Value Sets](api/schema-value-sets.md), [API State Schemas](api/schema-state.md), and method owners.

## Storage-Owned JSON

SQLite `TEXT` columns that store JSON are a storage representation choice, not permission to persist arbitrary JSON.

Rules:

- Core must parse and validate JSON before commit.
- API-shaped stored JSON validates against the API schema owners.
- Storage-only JSON validates against this storage contract or the referenced storage owner.
- SQLite defaults such as `'{}'` and `'[]'` are storage defaults only; they do not make API fields optional.

| Record family | JSON `TEXT` category |
|---|---|
| Installation profile | Installation-profile metadata that is not a host trust decision, user judgment, or public API schema. |
| Agent Connection | Verification report JSON, user-action JSON, and metadata that are not used as authority, host trust proof, or a replacement for external host configuration. |
| Guard installation | Host capability JSON and metadata for local guard setup health, not OS enforcement proof. |
| `agent_sessions` | Non-authority metadata for a project-scoped Agent Session. |
| `guard_events` | Guard subject JSON, result JSON, and metadata for a local guard decision event. |
| `prompt_captures` | Non-authority metadata for a captured prompt record; prompt text is a direct nullable text column. |
| `unrecorded_changes` | Observed path arrays, detection JSON, resolution JSON, and metadata for unrecorded Product Repository changes. Resolution JSON stores compact resolution basis, capture basis, resolved method, and optional linked user-judgment reference; it must not store full sensitive command or prompt content. |
| `tasks` | Shaping summary, bounded lists, autonomy boundary, current close basis, terminal close summary, lifecycle summary, and `CompletionPolicy`. |
| `change_units` | Scope summaries, bounded lists, write basis summaries, optional effect contract data, and lifecycle support data. |
| `user_judgments` | Judgment request, context, options, affected refs, artifact refs, basis snapshot, sensitive-action scope, machine-readable resolution, and descriptive rationale metadata. |
| `project_continuity_records` | Applies-to paths, applies-to refs, source refs, artifact refs, superseded refs, review triggers, and non-authority metadata for durable project context. |
| `write_checks` | Write Check attempt scope and non-authority metadata. |
| `runs` | Summary, observed changes, evidence updates, Write Check effect data, and non-authority metadata. |
| `artifact_staging` | Staged artifact data, safe metadata, and non-authority metadata. |
| `artifacts` | Retention, producer, and non-authority metadata. |
| `artifact_links` | Non-authority metadata. |
| `evidence_summaries` | Evidence coverage, supporting refs, gap refs, and non-authority metadata. |
| `evidence_observations` | Tool metadata, input refs, output artifact refs, limitations, and non-authority metadata for one evidence observation. |
| `blockers` | Blocker owner references, related references, details, and non-authority metadata. |
| `task_events` | Event payloads for committed Core mutations. |
| `tool_invocations` | Committed replay responses. |

Task and Change Unit shaping JSON stores compact summaries and bounded lists only. It does not create an additional persisted record family.

## Related Owners

- [Storage Effects](storage-effects.md) defines which method branches create, update, observe, or leave records untouched.
- [Storage DDL](storage-ddl.md) defines baseline SQLite table shape, indexes, foreign keys, migration tables, and constraints.
- [Artifact Storage](storage-artifacts.md) defines artifact staging, promotion, linking, body reads, retention, and integrity lifecycle.
- [Storage Versioning](storage-versioning.md) defines state versioning, idempotency, replay, events, locks, and migration contracts.
- [Agent Connection](agent-connection.md) defines Agent Connections, Connection Projects, mode-gated MCP tool access, and User Channel boundaries.
- [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md) define API shape and public API values.
- [API Methods](api/methods.md) and method owner documents define public method behavior that uses records.
- [Runtime Boundaries](runtime-boundaries.md) defines `Product Repository`, Volicord installation or runtime process, and `Volicord Runtime Home` location boundaries.
- [Projection Authority Reference](projection-and-templates.md) and [Template Bodies](template-bodies.md) define read-time projection authority and rendered template bodies.
- [Security](security.md) defines security boundaries and guarantee levels.
