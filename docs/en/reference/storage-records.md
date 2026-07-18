# Storage Records

This document owns semantic meaning and cross-record invariants for the one
first-release storage contract. Exact tables, columns, constraints, indexes,
and canonical SQL remain with [Storage DDL](storage-ddl.md).

## Storage Locations

| Location | Purpose |
|---|---|
| `registry.sqlite` | Runtime Home identity, installation profile, projects, aliases, Agent Connections, explicit project memberships, canonical connection verification reports, and authoritative MCP runtime sessions. |
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
- stable project-scoped Guard installation identities and their canonical typed
  Guard manifests;
- at most one canonical connection verification report per Agent Connection;
- MCP runtime sessions and their process, initialization, discovery, safe-call,
  terminal-failure, and graceful-close facts;
- cross-database reservations that bind one runtime/host session to one
  Connection Project.

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
- project Agent Sessions that may precede their Registry runtime binding,
  retain host session/thread/latest-turn correlation plus first/last
  observations, and carry the current project integration revision.

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
configuration target, exact managed-configuration fingerprint, and nonnegative
Store-owned integration generation. The managed fingerprint includes the
current server command and entry. Host and client version fields are excluded
and remain diagnostic observations, not identity or allowlist inputs. The
fingerprint contains no host executable identity or provenance inputs. Store
increments the generation exactly once in each successful real mode transition
and never for a same-mode no-op, so a later return to an earlier mode cannot
revive earlier runtime, session, or Guard evidence.

A real mode transition atomically changes the Connection mode and generation,
clears its verification report, and replaces only the integration revision in
every owned strict Guard manifest. The complete current manifest inventory must
match one-for-one with Connection Project membership before mutation. Missing,
duplicate, stale, malformed, owner-mismatched, or partially writable inventory
causes the whole Registry transaction to fail without a partial Connection or
manifest update.

Milestone timestamps and facts express lifecycle state without a redundant
status enum. Store records successful `initialize`, the initialized
notification, every actual `tools/list` result and required-tool-set fact, a
successful designated safe/read-only Volicord call, terminal protocol failure,
and graceful close. A protocol success is not emitted when its authoritative
Store write fails. Best-effort diagnostics remain separate and cannot make an
otherwise valid tool result fail.

Project `agent_sessions` are the project-local correlation projection. Each
row names one Connection, carries a project integration revision that adds the
current workflow-policy fingerprint and Guard ownership pair, and keeps the
deterministic Connection-bound session ID, host session, thread, latest turn,
and first/last observations needed for workflow and Guard correlation. A Guard
observation can create the row with `runtime_session_id=NULL`; no empty,
sentinel, fabricated, or CLI-preflight runtime represents that state.
Composite project foreign keys prevent a downstream Guard row from pairing a
session with another Connection.

The first actual managed MCP tool call for the same host session reserves
Registry `mcp_runtime_project_session_bindings` and then attaches the runtime
to the project row. The Registry reservation supplies the uniqueness boundary
that a foreign key cannot express across separate SQLite databases. Exact
replay is idempotent, including recovery after reservation but before attach;
conflicting runtime, Connection, project, or host-session claims fail. A
partial project index makes non-null runtime attachment unique while allowing
any number of unbound Guard-first sessions.

Only an attached project row with an exact current Registry binding can
authorize Core. Unbound rows can retain Guard events and prompt captures.
Runtime rows themselves are historical process observations: a crashed row
may remain apparently open, and concurrent current rows may coexist without
blocking or selecting Guard correlation.

Runtime authorization reads these current records directly. It accepts only an
enabled Connection, a current Connection Project membership, a
`session_source=managed_host` runtime session for that Connection, and a
project Agent Session owned by the same runtime session, Connection, and
project. The stored Connection and project integration revisions must equal the
revisions derived from current owner inputs. The Connection mode must allow the
requested operation category. `cli_preflight` rows, diagnostic version fields,
and best-effort diagnostics cannot satisfy this boundary.

Registry storage authorizes managed operations only through the rows above. A
Runtime Home with a noncurrent authorization schema belongs to a different
`StorageManifest` and is rejected without migration.

Explicit Connection Project removal treats these rows as connection-owned
Registry integration state. The Store atomically deletes the selected
membership's `mcp_runtime_project_session_bindings`, project-scoped
`guard_installations`, and `connection_projects` row. If memberships remain,
it retains the Agent Connection, every `mcp_runtime_sessions` row, and other
projects' bindings and Guard Installations. If none remain, it deletes every
remaining binding and Guard Installation owned by the Connection, then its
runtime sessions and `agent_connections` row. Project registrations,
installation profiles, Runtime Home records, and all project `state.sqlite`
rows remain outside this deletion set.

Retained project-local Agent Sessions and Guard or workflow history do not
become future authority. Runtime authorization still requires the current
Registry membership, runtime session, and project-session validation described
above.

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

Each Registry `guard_installations` row retains only stable installation and
owner identity, canonical `manifest_json`, and creation/update timestamps. The
manifest is strict and owner-bound. It carries the exact policy hash,
integration revision, typed runtime commands, complete Volicord-managed file
expectations, and required typed hook phases. It is neither a host-capability
certificate nor an installation-status state machine.

Policy commands and runtime commands are intentionally different projections.
The canonical policy commands omit `--policy-hash`; after hashing that policy,
runtime commands add `--policy-hash <exact-hash>`. Hook wrappers and the Guard
manifest use the same typed runtime commands. Audit compares their shared owner
fields and command segments individually and never compares the two complete
command objects for equality.

Project `guard_events` bind every observation to the Guard installation,
policy hash, integration revision, typed hook phase, and contract status.
Current compatible events derive `guard_observation`; older hashes or revisions
remain historical and cannot satisfy it. A current malformed or incompatible
event makes that check fail. Prompt capture remains a fact within the same
observation summary, not a separate installation state.

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

Absent optional check or action members are omitted from canonical JSON rather
than stored as explicit null. This persisted report remains the sole stored
check/action state; CLI command output projects those members at top level and
does not persist a second command-output tree.

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
