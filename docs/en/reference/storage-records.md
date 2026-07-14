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

- baseline SQLite DDL, indexes, foreign keys, canonical SQL sources, or constraints; see [Storage DDL](storage-ddl.md)
- method branch persistence effects; see [Storage Effects](storage-effects.md)
- artifact staging, promotion, linking, body reads, retention, or integrity lifecycle; see [Artifact Storage](storage-artifacts.md)
- `project_state.state_version`, the canonical Core UTC clock and its persisted
  floor, idempotency, replay, events, lock, and incompatible-storage handling;
  see [Storage Versioning](storage-versioning.md)
- API request or response shape; see [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md)
- API method behavior; see [API Methods](api/methods.md) and the method owner documents
- runtime location and repository boundaries; see [Runtime Boundaries](runtime-boundaries.md)
- security guarantee levels and security boundaries; see [Security](security.md)

## Storage Locations

Volicord stores baseline records in one local `Volicord Runtime Home` and one project-local state database per registered project. `volicord init` can establish or reuse the selected Runtime Home and installation profile during first-run repository setup. Ordinary user flows do not need to provide the Runtime Home path again.

The tree is representative after the relevant storage features have been used; it is not an initial-directory checklist. Project registration creates or opens project state, while artifact-store directories may be created lazily.

```text
~/.volicord/
  registry.sqlite
  diagnostics.sqlite   # created lazily after a diagnostic session is observed
  projects/
    prj_<internal>/
      state.sqlite
      artifacts/        # created when artifact storage is used
        tmp/            # created when artifact staging occurs
```

Storage placement:

- `registry.sqlite` stores Runtime Home identity, installation profile records, project registration mapping, project aliases, Agent Connection records, Connection Projects membership, host-capability verification history and current pointers, host-hook installation records, and registry metadata. The installation profile includes the selected `volicord` command, MCP launch command, bin directory, default connection mode, metadata, and timestamps. Project registration includes `project_internal_id`, display name, CLI selection alias, Runtime Home relationship, registered `repo_root`, `project_home`, project `state.sqlite` path, status, metadata, and timestamps.
- `diagnostics.sqlite` is a lazily created, bounded, non-authority local operability store. It is separate from `registry.sqlite` and every project `state.sqlite` and has no foreign keys into either database.
- `projects/{project_internal_id}/` is the default Volicord project home shape for one registered project. It is not the same location or authority as `repo_root`.
- `state.sqlite` stores project-local Core state and project-scoped host-observation records for the registered project.
- `artifacts/` is the project artifact store when artifact storage is used; it may be created lazily when artifact storage is first needed. `artifacts/tmp/` is transient staging space when artifact staging requires it, not evidence authority; it may be created lazily when staging occurs. These directories need not exist immediately after project registration.

Artifact path bases:

- `artifact_staging.tmp_path` is stored relative to `project_home`; staged bytes or notices under the transient staging area use a shape such as `artifacts/tmp/<file>`.
- `artifacts.body_path` is stored relative to the artifact-store root, normally `project_home/artifacts`; persistent bodies use a shape such as `tmp/<file>` and are resolved as `artifact_store_root.join(body_path)`.

For operational project records, `project_home` is the location owner for project-local runtime state. The executable project state database path is derived from the validated project home as `project_home/state.sqlite`. The stored `state_db_path` remains in `registry.sqlite` for persistence and diagnostics, but it must match that derived path before Store returns a normal `ProjectRecord`, opens project-local state, resolves Agent Connection project access, enters Core execution, or reports MCP project availability. A mismatching registration remains inspectable as raw registry content for diagnosis, but operational lookup and listing must reject it rather than omit it or return it as a normal project. Inspection must not open, create, initialize, or repair the alternate `state_db_path`.

The `Product Repository` is the user product-file boundary registered by `repo_root`. It is not a Volicord runtime home, not Core authority storage, and not where runtime records, replay rows, judgments, write tickets, guard records, or Agent Connection registry state are stored.

Baseline SQLite table shape, indexes, foreign keys, constraints, and canonical SQL sources belong to [Storage DDL](storage-ddl.md). The current baseline SQLite storage profile for these records is `baseline_sqlite_v6`; storage-profile and incompatible-storage boundary behavior belongs to [Storage Versioning](storage-versioning.md).

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
| `diagnostics.sqlite` | `diagnostic_sessions` | Bounded local operability session | Session, optional connection and project identifiers, transport, optional host kind, producer package/build identity, and start/update timestamps. |
| `diagnostics.sqlite` | `diagnostic_events` | Content-free operability observation | Session relation, event/tool category, latency and byte counters, validation/retry/Core/replay flags, optional User Channel or fallback category, observed product-write count, authoritative-refresh-failure flag, categorical outcome, and timestamp. |
| `registry.sqlite` | Runtime Home identity | Runtime identity | One stored `runtime_home_id`, Runtime Home path, registry database path, schema/storage profile, metadata, and timestamps. |
| `registry.sqlite` | Installation profile | Executable profile | Selected `volicord` command, MCP launch command, bin directory, default connection mode, metadata, and timestamps established by `volicord init`. |
| `registry.sqlite` | Project registration and aliases | Project mapping | `project_internal_id`, display name, CLI selection alias, Runtime Home relationship, unique `repo_root`, location-owning `project_home`, stored `state_db_path` that must match `project_home/state.sqlite` for execution, status, metadata, and alias-to-internal-identity mappings. |
| `registry.sqlite` | Agent Connection | MCP host connection unit | Durable `connection_internal_id`, host kind, connection intent, host scope, optional `project_internal_id`, internal server name, config target, mode, enabled state, managed fingerprint, verification summary status, verification report JSON, user actions JSON, metadata, and timestamps. |
| `registry.sqlite` | Connection Projects | Connection project allowlist | Explicit many-to-many membership between an Agent Connection and registered projects using `connection_internal_id` and `project_internal_id`. |
| `registry.sqlite` | `host_capability_verifications` | Immutable host-capability validation history | Exact connection/capability, outcome, host/client version, adapter profile, managed fingerprint, Volicord build/source/target/executable digest, bounded evidence-artifact digest, observation/expiry interval, strict canonical `{}` metadata, and creation time. |
| `registry.sqlite` | `host_capability_state` | Current host-capability pointer | One current immutable verification row per connection and capability, replaced atomically by later passing, failed, unavailable, or revoked observations. |
| `registry.sqlite` | Host-hook installation | Host-hook setup and host capability record | Runtime Home, Agent Connection, optional project scope, host kind, integration mode, host capability JSON, installation lifecycle status, observed hook metadata, timestamps, and metadata. |
| `state.sqlite` | `project_state` | Project state header | Storage profile, `state_version`, current `Task` pointer, project enforcement profile, and `updated_at` as the persisted floor of the canonical Core UTC clock. |
| `state.sqlite` | `agent_sessions` | Observed Agent Session | Project-scoped session for one Agent Connection, optional host-hook installation, host kind, integration profile, start/end timestamps, and metadata. |
| `state.sqlite` | `guard_events` | Host-hook decision event | Project-scoped host-hook event tied to a connection and optional session or installation, with decision, subject JSON, result JSON, timestamp, and metadata. |
| `state.sqlite` | `prompt_captures` | Prompt capture | Project-scoped prompt capture for a session, including connection, capture kind, prompt hash, optional prompt text, timestamp, and metadata. |
| `state.sqlite` | `expected_writes` | Expected Product Repository write | Project-scoped expected-write correlation record created by an allowed detective pre-tool write, with connection/session identity, optional host invocation identity, exact path policy, active task/Change Unit/write-ticket basis, timestamps, and matched post-tool metadata. |
| `state.sqlite` | `unrecorded_changes` | Unrecorded Product Repository change | Project-scoped unresolved or resolved record for detected Product Repository changes that are not yet matched to a Core run or other owner-defined record. |
| `state.sqlite` | `session_watch_baselines` | Session watch baseline | Project-scoped session watch status and baseline snapshot for a registered Product Repository or watched path set, including effective exclusions, snapshot digest metadata, and compact snapshot entries. |
| `state.sqlite` | `session_watch_observations` | Session watch observation | Project-scoped detective observation derived from comparing a later safe snapshot to a baseline, with observed changed paths, optional expected-write or write-ticket correlation, and optional link to an existing unrecorded-change row. |
| `state.sqlite` | `tasks` | Work-unit state | User-value unit with mode and work phase, Task-owned acceptance policy and reason, optional predecessor relation and carry-forward audit, shaping summary, scope and close-basis revisions, nullable current close basis, lifecycle/result/terminal summary, current Change Unit pointer, and creator actor source. |
| `state.sqlite` | `acceptance_criteria` | Acceptance criterion | Core-generated criterion identity, owning `Task`, statement, evidence requirement, replacement order, active/retired state, and timestamps. |
| `state.sqlite` | `evidence_claims` | Supplemental evidence claim | Caller-assigned `Task`-scoped claim identity with one immutable non-empty statement. |
| `state.sqlite` | `change_units` | Scoped work boundary | Scope summaries, write basis, Change Unit lifecycle, and owning `Task` relation. |
| `state.sqlite` | `evidence_capture_intents` | Evidence-capture intent | Immutable expiring request bound to current Task/Change Unit/scope/baseline/target/workspace, exact capture spec and command/tool input digest or Core-derived connection source-selector digest, requesting connection and actor, expected outcome, and timestamps. |
| `state.sqlite` | `user_action_requests` | User-action request | Closed action request JSON, Core-derived basis and compatibility, required-for targets, request actor, originating method/idempotency relation, and expiry. The capture form and effective lifecycle status are derived rather than stored as composite columns. |
| `state.sqlite` | `user_action_resolutions` | Immutable User Channel resolution | At most one resolution per request, with a closed kind-matching body, channel kind and bounded visible-ASCII submission replay identity, local-user provenance, verification basis, assurance, and Core capture time. Choice facts or full observation detail stay in the body. |
| `state.sqlite` | `user_action_channel_tokens` | User Channel fallback token | Hash-only one-time local-web token bound to one request, connection, expiry, capture basis, and closed creation metadata containing exactly the fallback kind, `delivery_surface=model_invisible_user_surface`, endpoint, and exact canonical-form digest. |
| `state.sqlite` | `project_continuity_records` | Project continuity context | Durable project-level decisions, obligations, known limits, accepted residual risks, and constraints that remain addressable after the source `Task` closes. |
| `state.sqlite` | `write_tickets` | Write-ticket authority | Physical storage table for single-use write ticket authority records, basis version, attempt scope, expiration, actor source, optional originating judgment, and consumption state. |
| `state.sqlite` | `runs` | Execution or observation record | Committed execution or observation record, optional compatible write-ticket consumption, actor source, and compact evidence updates. |
| `state.sqlite` plus `artifacts/tmp/` | `artifact_staging` | Transient artifact staging | Staged handle metadata, creator actor source, safe staging facts, and transient bytes or notices. |
| `state.sqlite` plus `artifacts/tmp/` | `evidence_capture_receipts` | Durable evidence-source fact receipt with transient staging | One immutable, complete, content-bound, redacted safe receipt and transient staging handle per capture intent, with exact source/result digests, observed outcome, registered source coordinates, limitations, and timestamps. The row remains addressable after staged bytes are promoted. |
| `state.sqlite` | `evidence_capture_source_claims` | Exclusive evidence-source claim | Project-scoped normalized identity for each host invocation, guard event, or session-watch observation consumed by one receipt, with its exact intent/receipt pair, capture kind, and claim timestamp. |
| `state.sqlite` plus artifact store | `artifacts` | Persistent artifact record | Durable artifact metadata or body location, content type, SHA-256, size, integrity status, redaction, retention, producer, and availability facts. |
| `state.sqlite` | `artifact_links` | Artifact owner relation | Owner relation between an artifact and a baseline Core/API record family. |
| `state.sqlite` | `evidence_summaries` | Evidence summary | Compact evidence coverage, supporting references, gap references, and the resulting project state version that produced the current row value. |
| `state.sqlite` | `evidence_observations` | Evidence observation | Durable provenance record for one target, including Core-derived source and assurance, producer anchor, separate relevance assessment, exact outputs, observer, refs, limitations, and timestamps. |
| `state.sqlite` | `evidence_producers` | Finalized evidence producer | Immutable one-to-one intent/receipt/observation/artifact authority record bound to one Run and current basis, with canonical producer JSON. |
| `state.sqlite` | `blockers` | Blocker state | Structured blocker state for next action, write compatibility, evidence gaps, close readiness, or recovery. |
| `state.sqlite` | `authority_events` | Authority event trail | Append-only ordering and local audit trail for committed Core authority mutations. |
| `state.sqlite` | `tool_invocations` | Replay and exact operation-result row | Replay rows for committed non-dry-run Core method results when [Storage Effects](storage-effects.md) says replay is created, including immutable `response_json`, actor source, operation category, optional verification basis, and the optional canonical Git workspace context captured from the verified invocation. Eligible `operation_category=agent_workflow` rows are also the storage source addressed by `OperationResultRef`. |

When managed Codex or Claude Code correlation contributes a session identity,
storage receives only the opaque `managed_host_session_id` defined by
[Host Release Evidence](host-release-evidence.md).
The `mhs_` namespace is reserved for that mapping. An existing mapped session
is reusable only for the exact registered connection and host kind; an
attempted cross-connection or cross-host reuse is rejected without changing
the existing row, and generic or manual paths cannot preseed the namespace.
Raw native session, event, tool-call, capture, turn, and invocation identifiers
are never storage records, JSON metadata, log payloads, or diagnostic session
identifiers. Managed ingestion replaces correlation identifiers with
domain-separated opaque values before persistence. A missing, invalid, or
mismatched mapping remains missing, invalid, or mismatched; storage must not
manufacture a replacement or create diagnostic state for an invalid marker.

## Record Layout Rules

### Identity And Ownership

Baseline records use opaque stable ids as primary keys or equivalent unique keys. Uniqueness is scoped by the owning record family:

- Runtime Home identity stores one `runtime_home_id` for the Runtime Home.
- Project registration requires a unique `project_internal_id`, unique project alias, unique repository root, unique project home, and unique state database path. `project_name` is the display name and `project_alias` is the CLI selection aid.
- Agent Connection identity is unique by `connection_internal_id`.
- Connection Projects membership is unique by `connection_internal_id` and `project_internal_id`, and is the only registry membership that lets one connection address a registered project.
- Host-capability verification identity is unique by
  `verification_internal_id`; each history row belongs to one Agent Connection
  and exact capability. `host_capability_state` can point only to a row with the
  same connection and capability. A current non-passing row prevents fallback
  to an older passing row. Canonical UTC interval values must satisfy
  `observed_at <= created_at` and
  `observed_at < expires_at <= observed_at + 86,400 seconds`; a passing row also
  requires `created_at < expires_at`. A row is fresh only at
  `observed_at <= now < expires_at`. Twenty-four hours is a maximum freshness
  window, not a default lifetime or attestation period. A passing
  built-in stdio row requires `host_version = client_version`, and that single
  version must equal both the exact runtime `clientInfo.version` and the live
  artifact's installed-host version. Its `source_revision` is exact lowercase
  40- or 64-hex; `unknown` cannot pass.
- Publishing the exact same verification ID and content is idempotent. If that
  history row is no longer current, the duplicate does not move the newer
  pointer backward. The same ID with different content is a conflict.
- A disabled connection may ordinarily retain membership and is not thereby a migration-cleanup record. Last-project host migration cleanup is identified only by the exact `agent_connections.metadata_json.pending_host_cleanup` object with `project_id` and `replacement_connection_id`. The cleanup transaction must match that marker, disabled state, and the one retained membership before host retirement and membership removal.
- `agent_connections.metadata_json.pending_host_cleanup` is Store-owned recovery state. Generic Agent Connection registration and update inputs must reject that reserved key, and generic enable/disable or Connection Projects membership mutations must reject a marked row. A migration must not activate a marked row as its requested target. Migration transition and cleanup operations may rebind or remove the marker on superseded inventory only while revalidating its project membership.
- A present `pending_host_cleanup` value with missing, extra, empty, or wrongly typed members is not resumable cleanup. Doctor must report it as an invalid reserved marker; cleanup and migration discovery must not interpret it as valid inventory.
- Host-hook installation identity is unique by `guard_installation_id`. Project-scoped host-hook installations must name a registered project and an Agent Connection that has Connection Projects membership for that project.
- Local web consent token identity is the stored domain-separated token hash within one project-state database. The raw token must not be stored, and a pending token must name the project, selected Agent Connection, pending `UserActionRequest`, capture basis, and expiration. Consuming a token and inserting the corresponding `UserActionResolution` must be one project-state transaction or equivalent atomic operation.
- `user_action_resolutions.channel_submission_id` is 1 through 256 bytes of
  visible ASCII `0x21..=0x7e` and is unique within its project and channel kind.
  A local-web value is a digest-only identity bound to the project, request,
  bearer-token credential, expected connection, and closed completion metadata.
  The corresponding replay request hash separately incorporates the
  domain-separated token digest, expected connection, and typed canonical
  metadata; neither durable record stores the raw token or the internal binding
  object.
- Every `user_action_requests` row stores its exact `source_method` and
  `source_idempotency_key`. A direct `volicord.request_user_action` origin maps
  to exactly one request per project, which lets the same Agent Connection
  resume that exact result without creating a second request. One
  `volicord.reconcile_changes` commit may create several requests, so its rows
  may intentionally share the reconciliation idempotency key.
- Project-scoped rows belong to a registered project.
- Agent Sessions, host-hook events, prompt captures, expected writes, unrecorded changes, session watch baselines, and session watch observations belong to one project-local `state.sqlite` and name the Agent Connection that observed or produced the record.
- Task-scoped rows belong to the same project and `Task` as their owning `tasks` row.
- A Task has at most one same-project predecessor. Predecessor id, relation, and
  non-empty reason are either all absent or all present, and self-predecessor
  edges are rejected. `carry_forward_json` is the explicit disposition audit;
  it does not bypass current authority checks.
- An `AcceptanceCriterionId` is Core-generated and project-unique. Its composite same-Task key supports target foreign keys; once a criterion is retired, the row remains retired and is not reused as an active identity.
- An `EvidenceClaimId` is caller-assigned and unique only within its owning `Task`. The same spelling may exist independently in another `Task`, while the statement for an existing same-Task ID is immutable.
- Each evidence observation names exactly one same-Task acceptance criterion or supplemental evidence claim. The two target columns cannot both be null or both be populated.
- Current pointers and owner references must point to same-project records.
- A `Task` has at most one current Change Unit.
- Single-use relations such as consumed write-ticket rows, consumed staging handles, promoted staged artifacts, artifact owner links, and replay keys must not fork into multiple committed meanings.

### Current, Event, And Replay Rows

Current record families hold the current Core state for ordinary reads. `authority_events` is an append-only ordering and local audit trail for committed Core authority mutations. Each authority event row stores `event_id`, `project_id`, resulting `state_version`, `event_type`, `actor_source`, `operation_category`, `payload_json`, `request_hash`, `previous_event_hash`, `event_hash`, and `created_at`. The event hashes are local integrity and export-correlation fields; local SQLite storage is not a tamper-proof audit log. `tool_invocations` stores committed replay rows only where [Storage Effects](storage-effects.md) says replay is created.

For a normal Core authority commit, `project_state.updated_at`, every
`authority_events.created_at` row in the event batch, and the optional
`tool_invocations.created_at` replay-row value store one exact transaction
timestamp. That timestamp is no earlier than the prepared operation-time
sample. It can equal a prior floor and therefore does not imply that distinct
`state_version` values always have distinct timestamps.

Store transaction metadata that Core mutation application itself generates for
that transaction also uses the exact transaction timestamp, including
applicable `created_at`, `updated_at`, `retired_at`, and `promoted_at` values.
This rule does not replace semantic operation times such as `requested_at`,
`resolved_at`, `closed_at`, `recorded_at`, or `consumed_at`, or input- and
observation-owned facts such as `observed_at` and `started_at`; those preserve
the single operation sample or owner-verified source time defined by their
owners.

Every committed `evidence_summaries` insert or update stores the transaction's
resulting `project_state.state_version` in `produced_at_state_version`.
The current Evidence Summary for a `Task` is the row with the greatest
`produced_at_state_version`, not the row with the greatest `created_at` or
`updated_at`. UTC timestamps retain their owner-defined temporal meaning and
must not be used as a substitute for authority commit order. An opaque record
ID is likewise not an authority-order key.

Artifact staging, registered evidence-capture receipt fulfillment, and local
User Channel token issuance are storage-owned temporal effects rather than Core
authority commits. Each updates `project_state.updated_at` to at least its own
`created_at` in the same transaction, but creates no authority event or replay
row and does not increment `state_version`. Exact replay, rejection, dry-run,
and read-only paths do not persist a later floor. The complete clock and
bootstrap-preservation rules belong to [Storage Versioning](storage-versioning.md#canonical-core-utc-clock).

<a id="exact-operation-result-storage"></a>
#### Exact operation-result storage

For an eligible `operation_category=agent_workflow` Core commit,
`tool_invocations.response_json` is the immutable exact serialized method result
used both for idempotent replay and for read-only
`volicord.get_operation_result` paging. `OperationResultRef`
addresses that existing row; it does not create another record family or copy
the response into page records. Pages read contiguous UTF-8-safe portions of the
stored bytes without rewriting, truncating, or recomputing the response.

The stored actor and project remain part of the row's ownership boundary.
Retrieval does not broaden that boundary, and historical response bytes are not
current Core authority. `operation_category=user_only` rows, including the exact
`volicord.resolve_user_action` response and its private user text, are not
eligible for Agent Connection result retrieval. `volicord.stage_artifact` has
no replay row and therefore no `OperationResultRef`.

This retrieval capability reuses the current `tool_invocations` row and
`response_json`; it adds no table, column, durable page record, record family,
or storage migration.

State-version behavior, idempotency, event meaning, replay conflict handling, locks, and migration contracts belong to [Storage Versioning](storage-versioning.md).

### Authority Bundle Export

Administrative CLI behavior for `volicord export authority-bundle` belongs to
[Administrative CLI](admin-cli.md#authority-bundle-export). Storage Records owns
the storage-row basis of that export: the bundle's `records.jsonl` represents
the selected project's baseline `state.sqlite` record families as storage rows,
and copied persistent artifact bodies are supplementary exported files for
`artifacts` rows that currently have readable local artifact-store bytes.

The export bundle preserves storage row names, column names, stored values, and
storage-owned JSON `TEXT` values as exported record data. Its SHA-256 checksum
manifest labels the exported files. It does not convert the Runtime Home into
tamper-proof storage, prove that the Runtime Home was never modified before
export, or prove correctness, test sufficiency, review completion, deployment,
final acceptance, or residual-risk acceptance.

### Relationship Validation

Storage must validate stored relationships before commit, including:

- same-project and same-`Task` ownership
- active pointer targets
- compatible write-ticket consumption
- artifact staging consumption and promotion targets
- artifact owner relations
- evidence-capture intent/receipt one-time fulfillment and producer one-to-one
  intent, receipt, observation, artifact, and Run relations
- exact capture-class source shape and an exclusive normalized claim for every
  underlying invocation, guard event, or watcher observation; staging, receipt,
  and claims commit or roll back together
- evidence-capture source timing with
  `intent.created_at <= observed_at < intent.expires_at`, receipt creation after
  observation and before expiry, and staging expiry exactly equal to intent
  expiry
- Connection Projects membership and enabled-state consistency for Agent Connection routing
- host-hook installation, Agent Session, host-hook event, prompt capture, expected-write, unrecorded-change, session watch baseline, and session watch observation project and connection scope
- session-watch baseline and observation snapshots whose strict stored entries,
  scope, paths, algorithm, and digest reconstruct canonically; the stored
  `observed_paths_json` and `change_summary_json` must equal a recomputed diff
- JSON reference arrays that SQLite cannot express as direct foreign keys

### Authority Row Preservation

Ordinary baseline Core operations preserve authority rows through lifecycle or status transitions. Completing, cancelling, or superseding a `Task` changes the relevant lifecycle/status meaning while keeping committed authority rows addressable for audit and recovery.

This preservation applies to `tasks`, `change_units`, `evidence_capture_intents`, `evidence_capture_receipts`, `evidence_capture_source_claims`, `user_action_requests`, `user_action_resolutions`, `user_action_channel_tokens`, `project_continuity_records`, `write_tickets`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `evidence_observations`, `evidence_producers`, `blockers`, `authority_events`, `tool_invocations`, `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, and `session_watch_observations`. Only the receipt's staging handle and staged bytes follow the transient artifact lifecycle. Artifact-specific transient and durable retention rules belong to [Artifact Storage](storage-artifacts.md).

### Host-Observation Records

Host-observation records preserve local authority facts about host integration state. They can help Core and Store decide whether work can honestly proceed or close. They do not provide OS sandboxing, filesystem ACLs, external policy enforcement, anti-forgery proof, actor identity proof, or proof that a write was prevented.

All `agent_sessions`, `guard_events`, `prompt_captures`, `expected_writes`, `unrecorded_changes`, `session_watch_baselines`, and `session_watch_observations` rows are project-local. They must not leak across project `state.sqlite` databases.

Whenever a host-observation read or projection derives the `latest` or most
recent `agent_sessions`, `guard_events`, `session_watch_baselines`, or
`session_watch_observations` authority fact, it strict-parses the applicable
canonical RFC 3339 timestamp, normalizes it to a UTC instant, and compares that
instant at nanosecond precision. Stored timestamp text, SQLite `julianday()`,
row insertion order, and opaque record IDs are not authority-order keys. Rows
at the same greatest instant are co-latest; an opaque ID must not select one as
newer.

Close- and security-relevant issue predicates from co-latest `guard_events`
are combined conservatively across the entire set: an issue present on any
co-latest event remains present, and an `allow` or issue-free sibling cannot
hide it. When a consumer requires one Agent Session, session-watch baseline,
or session-watch observation as authority, multiple distinct co-latest
candidates make that selection ambiguous and fail closed as unavailable owner
state unless a focused owner explicitly defines a set-valued aggregation.

`guard_installations` records setup lifecycle state, observed hook metadata, and host capability by Runtime Home, Agent Connection, and optional project scope:

- `configured` means files or metadata are installed and the setup lifecycle row is not itself asserting a new active observation. A same-identity refresh can preserve earlier observation metadata, and that preserved observation can still match the current host, policy, phase, and capability; consumers therefore evaluate those facts rather than inferring observation absence from the lifecycle status. `reload_required` means a host reload and a current matching observation are still required; any retained observation metadata remains diagnostic only.
- `active` means Volicord observed a valid host hook for the recorded project, Agent Connection, host kind, integration profile, and policy hash. It does not prove OS enforcement or sandboxing.

#### Closed host-hook capability v2 record

`guard_installations.host_capability_json` uses the internal closed
`volicord-host-hook-capability-v2` contract. Its top-level object has exactly
these 18 members and no others:

- `schema`, `policy_hash`, `selected_profile`, `connection_intent`
- `final_output_authority_disclosure_implementation_available`,
  `native_host_output_adapter`,
  `native_host_output_adapter_config_verified`,
  `bash_shell_mutation_coverage`, `direct_file_write_matcher_coverage`
- `host_capabilities`, `required_hook_phases`, `missing_required_hooks`,
  `prompt_capture`
- `files`, `host_hook_commands`, `hook_root_resolution`, `hook_path_safety`,
  `commands`

`schema` is exactly `volicord-host-hook-capability-v2`; `policy_hash` is a
nonempty string; `selected_profile` is `record` or `detective`; and
`connection_intent` is `personal`, `shared`, or `global`.
`native_host_output_adapter` is `none`, `codex`, or `claude-code`.
`final_output_authority_disclosure_implementation_available` is true exactly
when that adapter is `codex` or `claude-code`, while
`native_host_output_adapter_config_verified=true` is permitted only for one of
those implemented adapters. The two coverage members and `prompt_capture` are
Booleans. A `record` capability has `prompt_capture=false`.

`host_capabilities` is a closed object containing exactly these Boolean
members: `stdio_mcp`, `http_mcp`, `session_start_hook`, `pre_tool_hook`,
`post_tool_hook`, `user_prompt_submit_hook`, `stop_hook`, `rule_file_support`,
and `project_local_configuration`. `commands` is a closed map with exactly
`session_start`, `pre_tool`, `post_tool`, `prompt_capture`, and `stop`. Each
value is exactly `{command,args}`, where `command` is a nonempty string and
`args` is an array of strings.

`required_hook_phases` and `missing_required_hooks` are duplicate-free arrays.
For `detective`, `required_hook_phases` is exactly
`session_start_hook`, `pre_tool_hook`, `post_tool_hook`,
`user_prompt_submit_hook`, and `stop_hook`; `missing_required_hooks` is a subset
of that set. For `record`, both arrays are empty. This canonical stored rule is
distinct from the defensive absent-or-listed completeness projection defined
for `GuardHealthSummary` in [API State Schemas](api/schema-state.md).

Each `host_hook_commands[]` entry is a closed object with exactly
`host_kind`, `phase`, `purpose`, `policy_key`, `command_shape`, `command`,
`args`, `expected_wrapper_path`, `expected_phase_wrapper_path`,
`root_resolution_basis`, `hook_command_path_basis`, `cwd_independent`,
`subdirectory_safe`, `wrapper_resolution_status`, and `verification`.
`verification` is exactly `{basis_verified_by,host_contract_source}`.
`command_shape` is `shell_command_string` with `args=null`, or `exec_form` with
a string array for `args`. Root basis is `git_work_tree` or
`claude_project_dir`; path basis is `git_root_runtime` or
`claude_project_dir`; and wrapper status is one of `ok`,
`relative_path_unsafe`, `wrapper_missing`, `wrapper_not_executable`,
`dispatch_missing`, `placeholder_unsupported`, `absolute_path_stale`,
`policy_hash_mismatch`, `host_output_mismatch`, `authority_mismatch`, or
`metadata_missing`. Phases are unique and use the exact phase-to-policy-key
mapping. Purpose is `detective_guard` for `detective` and
`final_output_authority_disclosure` for `record`; all entries use one nonempty
host kind. A `detective` record has exactly the required phases not listed as
missing. A `record` record has no entry or only `stop_hook`.

Owner binding further requires the generated host command itself. Codex uses
exactly `shell_command_string` with `args=null`. Detective commands are exactly
`sh -c 'root=$(git rev-parse --show-toplevel) || exit $?; exec
"$root/.codex/hooks/volicord-dispatch.sh" <command-name>'`, where the final
argument is the command name mapped from that entry's `policy_key`; a Record
`stop` command names the exact phase wrapper instead of the dispatch and has no
phase argument. Claude Code uses exactly `exec_form`, an empty `args` array,
and `${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-<command-name>.sh`. A direct
Volicord invocation, absolute wrapper path, wrong phase argument, alternate
shell form, or legacy form is not an exact-v2 owner-bound command.

When there are no host-hook commands, `hook_root_resolution` and
`hook_path_safety` are both `null`. Otherwise, `hook_root_resolution` is exactly
`{basis,all_cwd_independent,all_subdirectory_safe,overall_status,phases}` and
each `phases[]` entry is exactly
`{phase,root_resolution_basis,hook_command_path_basis,cwd_independent,subdirectory_safe,wrapper_resolution_status}`.
`hook_path_safety` is exactly
`{overall_status,all_cwd_independent,all_subdirectory_safe,commands}` and each
`commands[]` entry is exactly
`{phase,hook_command_path_basis,cwd_independent,subdirectory_safe,wrapper_resolution_status}`.
Both arrays are one-to-one projections of `host_hook_commands`; their aggregate
Booleans, basis, and `ok` or `relative_path_unsafe` status must equal the
underlying entries.

Each `files[]` value is a closed ownership-tagged union. Every variant has
string `kind`, `path`, `status`, `content_hash`, and `ownership`. `kind` is one
of `volicord_policy`, `git_info_exclude`, `host_mcp_config`,
`host_hook_config`, `host_hook_dispatch`, `host_hook_wrapper`,
`host_rule_instruction`, or `agents_managed_block`; `status` is one of
`planned_create`, `planned_update`, `unchanged`, `created`, or `updated`.
`managed_json` has only the five common members. `managed_block` adds exactly
`managed_marker_start` and `managed_marker_end`.
`managed_json_projection` adds exactly `managed_projection` and
`managed_projection_json`. `managed_script` adds `managed_marker` and Boolean
`executable_required`, then is either a `host_hook_dispatch` with exactly
`managed_script_role=codex_dispatch`, `host_kind`, and `phase`, or a
`host_hook_wrapper` with exactly `managed_script_command`, `host_kind`,
`phase`, `purpose`, `connection_id`, `guard_installation_id`, `policy_hash`,
and `host_output`.

The JSON shape alone is insufficient authority. The capability profile and
intent must match the owning `guard_installations` row and Agent Connection;
the row and connection host kinds must match; the adapter, every host-hook
command host kind, and every managed-script host kind must agree with that
owner host. Repository inventory is also bound to the normalized absolute
`repo_root` of the exact owning project. `volicord_policy` is exactly
`.volicord/policy.json`; `agents_managed_block` is exactly `AGENTS.md`; the
Claude Code `host_mcp_config` is exactly `.mcp.json`; hook configuration is
exactly `.codex/hooks.json`, `.claude/settings.local.json` for a personal
Claude Code connection, or `.claude/settings.json` otherwise; phase wrappers
are exactly `.codex/hooks/volicord-<command-name>.sh` or
`.claude/hooks/volicord-<command-name>.sh`; the Codex Detective dispatch is
exactly `.codex/hooks/volicord-dispatch.sh`; and rule instructions are exactly
`.codex/rules/volicord.rules` or `.claude/rules/volicord.md`. Command wrapper
paths must name those same canonical paths.

For a project-scoped capability, all five top-level `commands` entries use one
identical nonempty executable. Their arguments are exactly `_hook`, the
phase-mapped command name, `--repo` and the owning normalized root,
`--connection` and the owning connection ID, `--guard-installation` and the
owning installation ID, `--host` and the public owner-host label, and
`--integration-profile` and the owning profile, followed by the generated
output pair. That pair is `--host-output codex` or `--host-output claude-code`
for the corresponding Detective adapter and `--output volicord-json`
otherwise. These policy commands omit `--policy-hash` because the policy hash
is computed over the policy containing them. Every managed wrapper command
must reuse that executable and the exact owner coordinates: a Detective
wrapper adds the capability `policy_hash` before its host-output pair, while a
Record `stop` wrapper uses `_final-output`, the same owner coordinates, the
capability `policy_hash`, and the exact host-output pair. Generated shell-word
quoting is part of this exact command text; there is no compatibility fallback.

`git_info_exclude` is the sole
under-`repo_root` location exception because a linked worktree's resolved
common-Git-directory `info/exclude` can be outside the worktree; it does not
authorize arbitrary-path retirement. A projectless capability cannot carry
host-hook commands or any repository inventory, including `git_info_exclude`.
Its required closed top-level `commands` map remains shape-validated but is not
repository authority and must not be consumed as project command facts.
Production writes reject a shape, semantic relation, or owner binding mismatch.
Existing mismatched rows remain visible to bounded raw inspection for diagnosis,
but Store, Core, final-output, connection, and Doctor fact consumers, including
guard-event evidence fulfillment, fail closed instead of consuming their
capability facts.

The removed `final_output_authority_disclosure_supported` Boolean is not a v2
member. The three current final-output members keep implementation,
`native_host_output_adapter`, and generated-configuration audit distinct. None
is `HostFeatureSupportStatus`, exact live-host evidence, or permission to
project `verified`. Missing members, unknown members at any closed level, a v1
schema, or a retired member make the value invalid current capability input.
Reads must not decode v1 as v2, copy its old Boolean into a new field, or infer
a support state from it. Supported same-identity repair and migration rejection
behavior belongs to [Administrative CLI](admin-cli.md).

`expected_writes` records deterministic write correlation:

- A pending row means the detective pre-tool path allowed one expected write bounded by project, connection, session, time, path, Task, Change Unit, and active write-ticket coordinates.
- A matched row means a post-tool observation was correlated with that expected write. It does not prove product correctness, actor identity, or OS-level write prevention.
- An unmatched, ambiguous, or ticket-out-of-scope Product Repository change creates an unresolved `unrecorded_changes` row.

An unresolved `unrecorded_changes` row means that an observed Product Repository change still needs owner-defined reconciliation. Resolving it preserves the row and records the local resolution basis, actor source, capture basis, resolution timestamp, and optional linked user-action resolution.

`session_watch_baselines` and `session_watch_observations` support detective session-level Product Repository watching. They are not a sandbox, filesystem permission boundary, pre-write block, or proof of who changed a file or why it changed.

- A baseline stores watch availability, the registered repository root or watched path set, effective exclusions, and deterministic snapshot-digest metadata.
- An observation stores changed product paths found by comparing a later safe snapshot with the baseline. It may include expected-write, write-ticket, and unrecorded-change correlation refs.
- Linking an observation to an expected write or one active matching write ticket is deterministic correlation only.
- Linking an observation to an `unrecorded_changes` row records local reconciliation context. It does not create a close blocker by itself.

<a id="local-diagnostics-store"></a>
### Local Diagnostics Store

`diagnostics.sqlite` is independent local operability storage, not a Core,
registry, evidence, User Channel, or host-observation authority database. Its
schema version is local to this store. `diagnostic_sessions.session_id` owns
each event through an internal cascading foreign key, but the database has no
cross-database relation to `registry.sqlite` or a project `state.sqlite`.
Connection and project identifiers are non-authority correlation labels only.

Default local collection records only bounded aggregates and categorical
observations:

- event kind: `mcp_tool_call`, `guard_hook`, or `session`
- outcome: `success`, `rejected`, `validation_failure`, `tool_error`,
  `transport_error`, or `unavailable`
- optional verified User Channel category: `mcp_elicitation`,
  `prompt_capture`, `local_web_consent`, or `cli_inbox`
- optional pending fallback category: `prompt_capture`,
  `local_web_consent`, or `cli_inbox`
- call, latency, request/response byte, validation failure, retry,
  Core-reached, Core-committed, replay, observed product-write, and
  authoritative-refresh-failure counters

The schema has no prompt, path, file-body, error-detail, secret, user-action
question or capture-form, choice-note, or evidence-observation-summary column.
The bounded tool field accepts an
identifier, not arbitrary request text. Content-bearing detailed trace is not
supported. Any future detailed trace requires a separate explicit opt-in,
retention, and redaction contract rather than widening these tables.

Retention is enforced on diagnostic writes: sessions older than 7 days are
removed, at most 64 sessions remain, and at most 1,024 events remain per
session. Time-based retention compares parsed timestamp values, not lexical
timestamp text. Absence, corruption, incompatible version, read-only storage,
or write failure in this database is nonfatal to MCP, guard, Core, and User
Channel outcomes. Diagnostics must never update `state_version`, evidence,
assurance, close readiness, judgments, authority events, or replay rows, and
the authority-bundle export excludes this database.

### Current Close Basis

The current close basis is Task-owned current state stored with the `tasks` family. It is distinct from the terminal close summary stored for a successful terminal close result.

The authoritative current `CurrentCloseBasis` record is `tasks.close_basis_json`, interpreted with the Task-owned close-basis coordinates.

Existing open Tasks do not automatically convert terminal close summary JSON into a current close basis. Absence of a current close basis is represented as absence in `tasks.close_basis_json`, not as an empty generated basis. Change Unit records do not store or satisfy current `CurrentCloseBasis` authority.

Stored user-action requests require a closed request body and `UserActionBasis`.
Resolved requests require one complete closed resolution body, actor provenance,
verification basis, and assurance level. Rows missing those facts are invalid
owner state, not audit-compatible authority records.

`user_action_channel_tokens.created_metadata_json` strict-decodes as exactly
`{fallback_kind, delivery_surface, endpoint, form_digest}`. The required values
are `fallback_kind=local_web_consent`,
`delivery_surface=model_invisible_user_surface`, and `endpoint=/consent`; the
digest must match the canonical form derived from the closed stored request.
Missing, extra, wrong-typed, or mismatched metadata—including every
pre-correction row without `delivery_surface`—is permanently unusable under
corrected code. Local-web GET, POST, token consumption, and resolution then fail
closed without rendering a form or changing token, project, UTC-floor, or
user-action state. Such a row is never upgraded; the pending action remains
resolvable through another valid User Channel such as CLI.

The presence of one `user_action_resolutions` row causes effective
`status=resolved` only while the request basis remains current. A stale or
superseded basis takes precedence over that immutable row. Resolution presence
does not mean approval or supporting evidence. Current authority-bearing choice
use requires the selected stored option, derived machine action/outcome,
applicable User Channel provenance, and current basis.
Evidence-observation use requires the nested selected target, exact canonical
artifact refs, relevance status, nonblank observation summary, and current exact
artifact bytes stored in the closed resolution body. The optional user note is
private descriptive text, not rationale or authority. Missing kind-specific
facts are invalid owner state.

### Project Continuity Records

`project_continuity_records` preserve durable project-level context from committed Core effects. Baseline records may represent decisions, obligations, known limits, accepted residual risks, or constraints.

The source `Task` and optional source Change Unit identify where the continuity record came from. They do not make that source path current again. `status='active'` keeps the record visible as live project context, while `superseded` and `closed` keep the record addressable for audit and recovery.

Project continuity records are not current authority for a new operation. A future write, Run, judgment requirement, close readiness check, final acceptance, residual-risk acceptance, or blocker decision must still use the current owner-defined Core state and compatibility rules.

## Storage-Owned Values

Closed storage-owned value sets are persistence constraints. Unknown values must not commit.

| Stored field | Baseline values |
|---|---|
| Project registration `status` | `active` |
| `installation_profile.default_connection_mode` | `read_only`, `workflow` |
| Agent Connection `host_kind` | `codex`, `claude_code`, `generic` |
| Agent Connection `intent` | `personal`, `shared`, `global` |
| Agent Connection `host_scope` | `user`, `project`, `local`, `export` according to the `host_kind` matrix |
| Agent Connection `mode` | `workflow`, `read_only` |
| Agent Connection `enabled` | `0`, `1` |
| Agent Connection `last_verification_status` | `not_verified`, `complete`, `action_required`, `failed` |
| Host-hook installation `guard_mode` | `record`, `detective` |
| Host-hook installation `installation_status` | `absent`, `configured`, `reload_required`, `active`, `degraded`, `stale`, `broken` |
| `agent_sessions.guard_mode` | `record`, `detective` |
| `guard_events.decision` | `allow`, `deny`, `warn`, `inject_context` |
| `expected_writes.path_policy` | `exact_paths` |
| `expected_writes.status` | `pending`, `matched` |
| `unrecorded_changes.status` | `unresolved`, `resolved` |
| `session_watch_baselines.status` | `disabled`, `active`, `degraded`, `unavailable` |
| `session_watch_baselines.scope_kind` | `repository`, `path_set` |
| `session_watch_observations.observation_status` | `unresolved`, `linked` |
| `change_units.status` | `proposed`, `active`, `replaced`, `closed` |
| `change_units.is_current` | `0`, `1` |
| `write_tickets.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `user_action_requests.action_kind` | seven judgment kinds plus `evidence_observation` |
| `user_action_requests.basis_status` | `current`, `stale`, `superseded` |
| `user_action_requests.source_method` | `volicord.request_user_action`, `volicord.reconcile_changes` |
| `user_action_channel_tokens.status` | `pending`, `consumed`, `expired` |
| `project_continuity_records.kind` | `decision`, `obligation`, `known_limit`, `accepted_risk`, `constraint` |
| `project_continuity_records.status` | `active`, `superseded`, `closed` |
| `artifact_staging.status` | `staged`, `consumed`, `expired`, `discarded` |
| `artifacts.status` | `available`, `missing`, `integrity_failed`, `unavailable` |
| `artifacts.integrity_status` | `verified`, `corrupt` |
| `artifact_links.owner_record_kind` | `task`, `change_unit`, `run`, `user_action_request`, `user_action_resolution`, `evidence_summary`, `evidence_observation`, `evidence_producer`, `blocker` |
| `evidence_capture_intents.capture_kind`, `evidence_capture_receipts.capture_kind`, `evidence_producers.producer_kind` | `verified_command_execution`, `verified_tool_invocation`, `registered_connection_observation` |
| `evidence_capture_receipts.completeness` | `complete` |
| `evidence_capture_source_claims.source_claim_kind` | `host_invocation`, `guard_event`, `session_watch_observation` |
| `evidence_observations.source_kind` | `agent_report`, `connection_observation`, `external_tool`, `user_observation`, `reused_evidence`, `unverified_claim` |
| `evidence_observations.assurance_level` | `cooperative_report`, `registered_connection_observed`, `external_tool_result`, `user_observed`, `unverified` |
| `blockers.status` | `active`, `resolved`, `superseded` |
| `tool_invocations.status` | `committed` |
| `authority_events.operation_category` and `tool_invocations.operation_category` | `read`, `agent_workflow`, `user_only`, `admin_local`, `local_recovery` |

Rows that mirror public API values must match [API Value Sets](api/schema-value-sets.md), the relevant schema owner, and the method owner exactly. This document does not redefine public API values for fields such as `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result`, `runs.kind`, `runs.status`, or `evidence_summaries.status`; see [API Value Sets](api/schema-value-sets.md), [API State Schemas](api/schema-state.md), and method owners.

An `evidence_observations.source_kind` / `assurance_level` pair stored in the row
is not sufficient strong provenance by enum value alone. Core records the pair
after method-owned derivation and records `observed_by_actor_source` from the
verified invocation rather than trusting the request member. Current close and
reuse evaluation fail closed and revalidate the target, Task and Change Unit,
source Run, current scope revision and baseline, exact current output bytes,
the typed producer anchor, and the separate relevance assessment. The capture
intent and complete receipt path can finalize an authority-owned external-tool
or registered-connection producer. Direct claims without that exact anchor
remain cooperative even when their artifact bytes are available and verified.
A `user_observation` row must point to a current
`evidence_observation` `user_action_resolutions` record with matching detail, exact outputs, and
`relevance_status=supported`. A `reused_evidence` row must point to exactly one
original evidence observation; Core recursively revalidates that original
identity, inherited assurance, outputs, producer, and relevance. Descriptive
tool metadata, raw guard payloads, artifact integrity, and `source_refs_json`
cannot substitute for a producer or relevance record.

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
| Host-capability verification | V1 `metadata_json` is strict canonical `{}` only. Every allowed evidence coordinate has a dedicated column; a bearer URL or token, prompt, transcript, screenshot, raw host artifact, private operator data, and arbitrary or additional members are invalid. |
| Host-hook installation | Closed internal `volicord-host-hook-capability-v2` JSON and metadata for local host-hook setup health. V2 separates implementation availability from configuration verification; v1 is invalid current input and is repaired only through init without inference. These records are not typed host support, exact live evidence, or OS enforcement proof. |
| `agent_sessions` | Non-authority metadata for a project-scoped Agent Session. |
| `guard_events` | Host-hook subject JSON, result JSON, and metadata for a local host decision request. |
| `prompt_captures` | Non-authority metadata for a captured prompt record; prompt text is a direct nullable text column. |
| `expected_writes` | Expected path arrays, write-ticket id arrays, matched path arrays, and metadata for detective expected-write correlation. |
| `unrecorded_changes` | Observed path arrays, detection JSON, resolution JSON, and metadata for unrecorded Product Repository changes. Resolution JSON stores compact resolution basis, capture basis, resolved method, and optional linked user-action resolution reference; it must not store full sensitive command or prompt content. |
| `session_watch_baselines` | Watched path arrays, effective exclusion arrays, snapshot entry arrays, and metadata for a session watch baseline. Snapshot entries store path, kind, size, hash, or skip reason metadata only; they do not store file contents. |
| `session_watch_observations` | Observed changed path arrays, compact change-summary JSON, snapshot entry arrays, and metadata for a session watch observation. Snapshot and change summaries do not prove actor identity, intent, product correctness, or close readiness. |
| `tasks` | Shaping summary, bounded lists, autonomy boundary, carry-forward dispositions, current close basis, terminal close summary, and lifecycle summary. Acceptance policy, work phase, and lineage edge identity use dedicated columns; acceptance criteria and supplemental evidence claims use their canonical relational tables. |
| `change_units` | Scope summaries, bounded lists, write basis summaries, optional effect contract data, and lifecycle support data. |
| `user_action_requests` | Closed request, required-for targets, Core-derived basis, request actor, exact originating method/idempotency relation, and expiry. |
| `user_action_resolutions` | Closed immutable resolution body, channel kind and bounded visible-ASCII submission id, derived actor/verification/assurance, Core capture time, optional private note, and choice or evidence-observation detail. Local-web rows store only the derived digest identity, never the raw token. |
| `user_action_channel_tokens` | Request-bound local-web hash-token lifecycle, capture basis, and closed delivery-surface creation metadata. |
| `project_continuity_records` | Applies-to paths, applies-to refs, source refs, artifact refs, superseded refs, review triggers, and non-authority metadata for durable project context. |
| `write_tickets` | Write-ticket attempt scope and non-authority metadata. |
| `runs` | Summary, observed changes, evidence updates, write-ticket effect data, and non-authority metadata. |
| `artifact_staging` | Staged artifact data, safe metadata, and non-authority metadata. |
| `evidence_capture_intents` | Exact target/capture JSON, command/tool input digest or Core-derived connection source-selector digest, expected outcome, registered session and Git workspace basis, actor/connection provenance, expiry, and non-authority metadata. Connection capture JSON contains no future source ID, observation timestamp, snapshot digest, or raw-event digest. |
| `evidence_capture_receipts` | Exact expected/observed outcomes, source refs, limitations, bounded safe receipt JSON and its digest/size, registered source coordinates in metadata, and non-authority metadata. The safe receipt is redacted and contains no raw command, environment, stdout, stderr, tool input, tool response, secret, or unbounded host payload. |
| `artifacts` | Retention, producer, and non-authority metadata. |
| `artifact_links` | Non-authority metadata. |
| `evidence_summaries` | Resulting authority state version, evidence coverage, supporting refs, gap refs, and non-authority metadata. |
| `evidence_observations` | Tool metadata, Core-record input refs, non-authoritative `SourceRef` JSON, output artifact refs, limitations, and typed Core-derived producer/relevance authority metadata. `source_refs_json` does not create authority. |
| `evidence_producers` | Strict canonical `EvidenceProducer` JSON plus the relational one-to-one authority keys and verification-basis metadata. |
| `blockers` | Blocker owner references, related references, details, and non-authority metadata. |
| `authority_events` | Event payloads for committed Core authority mutations. |
| `tool_invocations` | Immutable committed replay responses used for replay and eligible exact operation-result paging, plus the verified actor source, operation category, optional verification basis, and optional canonical Git workspace-context JSON used for exact replay compatibility. |

Task and Change Unit shaping JSON stores compact summaries and bounded lists only. It does not create an additional persisted record family.

## Related Owners

- [Storage Effects](storage-effects.md) defines which method branches create, update, observe, or leave records untouched.
- [Storage DDL](storage-ddl.md) defines baseline SQLite table shape, indexes, foreign keys, constraints, and canonical SQL sources.
- [Artifact Storage](storage-artifacts.md) defines artifact staging, promotion, linking, body reads, retention, and integrity lifecycle.
- [Storage Versioning](storage-versioning.md) defines the state-version clock,
  canonical Core UTC clock and persisted floor, idempotency, replay, events,
  locks, and incompatible-storage handling.
- [Agent Connection](agent-connection.md) defines Agent Connections, Connection Projects, mode-gated MCP tool access, and User Channel boundaries.
- [API Schema Core](api/schema-core.md), [API State Schemas](api/schema-state.md), [API Artifact Schemas](api/schema-artifacts.md), [API User Action Schemas](api/schema-user-action.md), [API Judgment Schemas](api/schema-judgment.md), and [API Value Sets](api/schema-value-sets.md) define API shape and public API values.
- [API Methods](api/methods.md) and method owner documents define public method behavior that uses records.
- [Runtime Boundaries](runtime-boundaries.md) defines `Product Repository`, Volicord installation or runtime process, and `Volicord Runtime Home` location boundaries.
- [Projection Authority Reference](projection-and-templates.md) and [Template Bodies](template-bodies.md) define read-time projection authority and rendered template bodies.
- [Security](security.md) defines security boundaries and guarantee levels.
