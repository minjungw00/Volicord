# Storage Records

This document owns semantic meaning and cross-record invariants for the one
first-release storage contract. Exact tables, columns, constraints, indexes,
and canonical SQL remain with [Storage DDL](storage-ddl.md).

## Storage Locations

| Location | Purpose |
|---|---|
| `registry.sqlite` | Runtime Home identity, installation profile, projects, aliases, Agent Connections, explicit project memberships, managed Codex binding/verification metadata, and authoritative MCP runtime sessions. |
| project `state.sqlite` | Project-local Core state, replay, authority events, UserAction, evidence, artifacts, continuity, project Agent Sessions, Guard observations, and reconciliation. |
| artifact store | Bytes and safe notices referenced by persistent artifact rows. |
| `diagnostics.sqlite` | Bounded non-authority operability counters. |

Each project state database belongs to one registered canonical Product
Repository. Cross-project rows, refs, replay, and current pointers are invalid.

### Local Diagnostics Contract

`diagnostics.sqlite` has one separate non-authority storage contract identified
by `contract_id=volicord.sqlite.diagnostics` and the exact canonical schema
digest derived from its current SQL table, column, and index inventory. The
manifest has exactly one singleton row. Fresh diagnostics storage is created
only when the database path is absent. An existing empty database, a missing or
extra manifest row, an unknown contract identifier, a noncurrent digest, or a
missing, changed, or unexpected schema object is rejected without migration,
repair, importer dispatch, or format inference.

This diagnostics manifest is not the authority `StorageManifest`, and the
diagnostics database never uses a numeric schema version as a compatibility
identity. Reads do not create it. Diagnostics failure cannot change a Core or
User Channel result. Operational MCP evidence is never read from this database.

## Record Families

Registry records include:

- one Runtime Home identity and current `StorageManifest` carrier;
- installation and executable selection;
- project registrations and aliases;
- Agent Connections and Connection Projects memberships;
- at most one canonical connection verification report per Agent Connection;
- MCP runtime sessions and their process, initialization, discovery, safe-call,
  terminal-failure, and graceful-close facts;
- cross-database reservations that bind one runtime/host session to one
  Connection Project;
- canonical `ManagedHostBinding` identity, generated-artifact identity, and
  current verification receipt coordinates.

Project-state records include:

- `project_state`, project workflow policy, Tasks, acceptance criteria,
  supplemental claims, and Change Units;
- Write Tickets, Runs, current close basis, blockers, authority events, and
  immutable replay rows;
- evidence-capture intents and receipts, artifacts and links, evidence
  summaries, observations, and producers;
- `UserActionRequest`, immutable `UserActionResolution`, and project continuity;
- expected writes, Guard observations, prompt observations, and unrecorded
  changes used by reconciliation;
- project Agent Sessions that reference a Registry runtime session, retain
  host session/thread/latest-turn correlation, and carry the current project
  integration revision.

Prompt-related Guard records are observations only. They are not a UserAction
resolution, user answer, verification basis, or authority source.

## Authoritative Operational Sessions

`mcp_runtime_sessions` is Agent Connection-owned application state. Volicord
creates its opaque `runtime_session_id` when the MCP process starts, before
host thread metadata exists. `session_source` is exactly `managed_host` or
`cli_preflight`; a CLI row can be inspected but can never satisfy a
managed-host operational-evidence lookup.

The connection integration revision is a domain-separated canonical digest of
the Connection identity, host kind, intent, scope, mode, server name,
configuration target, and exact managed-configuration fingerprint. The
managed fingerprint includes the current server command and entry. Observed
host version, executable digest, support-catalog coordinates, release
evidence, certified capability sets, and MCP client name/version are excluded.
Host and client version fields remain diagnostic observations, not identity or
allowlist inputs.

Milestone timestamps and facts express lifecycle state without a redundant
status enum. Store records successful `initialize`, the initialized
notification, every actual `tools/list` result and required-tool-set fact, a
successful designated safe/read-only Volicord call, terminal protocol failure,
and graceful close. A protocol success is not emitted when its authoritative
Store write fails. Best-effort diagnostics remain separate and cannot make an
otherwise valid tool result fail.

Project `agent_sessions` are the project-local correlation projection. Each
row names one runtime session and Connection, carries a project integration
revision that adds the current workflow-policy fingerprint and Guard ownership
pair, and keeps only the host session, thread, and latest turn needed for
workflow and Guard correlation. Composite project foreign keys prevent a
downstream Guard row from pairing a session with another Connection. Registry
`mcp_runtime_project_session_bindings` supplies the uniqueness boundary that a
foreign key cannot express across separate SQLite databases, so one
runtime/host session cannot be reused for another project.

## Identity And Ownership

Stored identifiers are exact, non-empty owner values. Store does not trim,
guess, reassign, or derive a replacement identifier from display text. Every
Task-scoped row names the same project and Task as its owner. Every Change
Unit, evidence target, Run, artifact link, blocker, and continuity ref is
validated against its owning coordinates.

Current pointers must reference current same-project records. Immutable history
may remain after current state advances, but it never becomes current through
timestamp comparison or record-name ordering.

Normalized Product Repository paths follow
[Runtime Boundaries](runtime-boundaries.md#product-repository-api-path-normalization).
Git object IDs use the shared exact 40- or 64-lowercase-hex contract; other
lengths and non-hex values are invalid on write and corrupt on read.

## Strict Stored UserAction Validation

`user_action_requests` stores one closed request body, Core-derived typed basis,
`required_for`, source method/idempotency identity, actor, and expiry.
`user_action_resolutions` stores at most one closed kind-matching resolution,
`channel_kind=cli`, bounded visible-ASCII submission identity,
`resolved_by_actor_source=local_user`, verification basis, assurance, and Core
capture time.

Store validates the complete typed request and resolution on both write and
read. It rejects:

- unknown or mixed union tags and extra fields;
- missing kind-specific fields;
- an `action_kind` inconsistent with the request body;
- option or evidence selections outside the stored candidates;
- a non-CLI channel or non-local-user provenance;
- invalid limits, timestamps, refs, or submission identity; and
- a resolution whose request, project, Task, or current basis does not match.

A malformed stored value is `Corrupt` with the persisted-data machine-readable
code. It is not defaulted, silently skipped, repaired from another column, or
returned as a partial valid object. The CLI inbox fails closed; MCP may expose
only the safe failure and never resolves the row.

<a id="exact-operation-result-storage"></a>
## Replay And Effects

One committed non-dry-run Core mutation stores its exact eligible response with
method, project, actor, operation category, idempotency identity, request hash,
state version, and optional verified workspace coordinates. Exact retry returns
the original bytes; the same identity with different canonical input conflicts.

User-only resolution replay remains inaccessible to an Agent Connection.
Request-user-action resume may read only the original agent-safe request result
and a separately refreshed safe current projection.

## Guard And Reconciliation Records

Expected-write and unrecorded-change records are project-local. Guard
suppression reads only bounded canonical correlation data and returns the exact
`SuppressionOutcome`. Store-read failure, corrupt records, budget exhaustion,
or invalid correlation yields `Unavailable`; no observed path is hidden.

Prompt observations may be stored only under their bounded observation schema.
They do not carry a user choice, resolution body, private inbox form, or
credential.

## Current Close Basis And Continuity

The current close basis belongs to the Task and is distinct from terminal close
history. Absence is represented as absence, not a generated empty basis.
Evidence and acceptance refs must remain exact and current under their owners.

Project continuity records are durable context, not a waiver. Their typed cursor
and ordering belong to the status method. Carry-forward never bypasses current
scope, baseline, Write Ticket, evidence, UserAction, or close checks.

## Storage-Owned JSON

Every authority-relevant JSON field uses a closed typed schema, canonical
encoding where a digest depends on bytes, and explicit size limits. Unknown,
missing, duplicate, wrongly typed, noncanonical, or owner-inconsistent members
are invalid input and corrupt persisted data.

Metadata explicitly classified as non-authority remains non-authority. It cannot
create user judgment, evidence assurance, acceptance, Write Ticket authority,
or close readiness.

### Agent Connection Verification Report

`agent_connections.verification_report_json` is the only persisted
connection-verification state. A non-null value is one complete strict
`ConnectionVerificationReport`; its derived status, checks, and user actions
cannot be stored or changed independently. SQL null means no completed report.
Reads project that absence through the Agent Connection owner's synthesized
`verification_not_run` report without mutating Registry storage.

Store validates the shared report type before write and after read, including
closed values, bounds, deterministic ordering, duplicate rejection, and the
derived aggregate. Malformed or noncanonical report JSON is corrupt persisted
owner state. It is not interpreted as no report and is not repaired from
another column.

<a id="authority-bundle-export"></a>
## Authority Bundle Export

The non-mutating authority bundle reads a consistent owner-defined snapshot. It
does not include diagnostics, credentials, private UserAction notes, prompts,
transcripts, runtime logs, or artifact bytes not selected by the export owner.
Export never changes project state.

The exported record-table set is projected from the canonical project-state
`GeneratedSchemaMetadata`, not from a separately maintained table list. Every
canonical table relation is included, including `acceptance_criteria`,
`authority_events`, `evidence_claims`, and `project_workflow_policies`. The
canonical project-state schema contains record tables rather than derived
compatibility relations. Content redaction remains field-semantic; for example,
a `prompt_captures` row does not export
`prompt_text`, and a user-only replay row does not export its response body.

## Related Owners

- [Storage](storage.md)
- [Storage DDL](storage-ddl.md)
- [Storage Effects](storage-effects.md)
- [Storage Versioning](storage-versioning.md)
- [API User-Action Schemas](api/schema-user-action.md)
- [Failure Model](failure-model.md)
