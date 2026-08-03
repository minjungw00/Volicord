# Storage Records

`shaping_checkpoints`, `shaping_checkpoint_gaps`,
`shaping_checkpoint_user_actions`, `shaping_decision_applications`, and
`shaping_checkpoint_applications` form one Task-scoped authority aggregate. Exactly one
non-superseded checkpoint can be current. User-owned gaps require same-Task
request links, and ready checkpoints have no current gap. Advisor finalization
creates application authority in this aggregate without replacing the checkpoint.

This document owns semantic meaning and cross-record invariants for the
supported storage contract. Exact tables, columns, constraints, indexes,
and canonical SQL remain with [Storage DDL](storage-ddl.md).

## Storage Locations

| Location | Purpose |
|---|---|
| `registry.sqlite` | Runtime Home identity, installation profile, projects, aliases, Agent Connections, explicit project memberships, canonical connection verification reports, structured diagnostic findings, and authoritative MCP runtime sessions. |
| project `state.sqlite` | Project-local Core state, replay, authority events, UserAction, evidence, artifacts, continuity, normalized host correlation, managed MCP project sessions, Guard observations, and reconciliation. |
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
missing, changed, or unexpected schema object fails exact-open validation.

At an absent final path, Store creates one invocation-owned staging file in the
same directory with an opaque unique identity. It initializes the entire
canonical schema and manifest, validates and closes the database, verifies that
no SQLite journal, WAL, or SHM sidecar is required, hardens and synchronizes the
file, and then publishes it to `diagnostics.sqlite` with one atomic no-replace
operation. The final path therefore names only a fully validated diagnostics
carrier. Concurrent `SharedWriter` creators may prepare independently; one
publishes, while every other creator removes only its own staging files and
validates the published winner. Each caller inserts its diagnostic session
only after opening that final database.

An existing final `diagnostics.sqlite` is validated through the current
read-write diagnostics path and permission-hardened without initialization or
repair. Read-only diagnostics operations inspect only that final path; they
return an empty result when it is absent and never read or create a staging
file.

This diagnostics manifest is not the authority `StorageManifest`, and the
diagnostics database never uses a numeric schema version as a compatibility
identity. Reads do not create it. Diagnostics failure cannot change a Core or
User Channel result. Operational MCP evidence is never read from this database.

## Record Families

Registry records include:

- one Runtime Home identity, opaque publication ID, and current
  `StorageManifest` carrier;
- installation and executable selection;
- project registrations and aliases;
- Agent Connections and Connection Projects memberships;
- stable project-scoped Guard installation identities and their canonical typed
  Guard manifests;
- at most one canonical connection verification report per Agent Connection;
- bounded structured diagnostic findings and their directed cause edges;
- MCP runtime sessions and their process, initialization, discovery, safe-call,
  terminal-finding, and graceful-close facts;
- cross-database reservations that bind one runtime/host session to one
  Connection Project.

For a fresh Runtime Home, one UUID-backed publication ID is generated for the
preparation and inserted with the identity row and initial installation
profile in the staged Registry before its final directory is published. The
publication ID is invocation provenance, not a credential, actor identity, or
schema version. Exact manifest and schema validation must succeed before the
records become reachable at the selected Runtime Home path. An existing
Registry is inspected read-only first; incompatible record or relation facts
are reported without rewriting any Registry record.

Read-only Registry inspection retains the strictly decoded current
`StorageManifest` as a typed value for diagnostic consumers. Doctor therefore
serializes `storage_profile` as the manifest object itself and renders its
fields and capability collection as structured human detail. It does not carry
the physical JSON text across the Store boundary or offer that text after a
decode, canonical-encoding, or current-contract failure.

Doctor JSON retains every collected check ID and complete typed detail object.
Verbose human grouping, human titles, healthy command aggregation,
failure-first expansion, and Hook evidence compaction are projections of those
records and do not add, rewrite, or persist a storage record. The finalized
Doctor remediation plan remains the sole action source for both projections.

Publication guards and their rollback results are process-local typed lifecycle
facts, not Registry rows. Confirmation failure retains its primary error,
rollback outcome, final-path observation, and parent-entry durability together.
An observed complete removal remains complete if parent synchronization fails;
an incomplete effect cannot be promoted to preserved intact state.

The per-canonical-Runtime-Home setup lease and its persistent coordination file
are also outside the storage record model. They carry no actor identity,
publication authority, recovery state, schema version, or stale-owner
metadata. Only the live OS file lock denotes lease ownership; setup continues
to use the stored publication ID and process-local publication guard for exact
rollback identity.

Every supported Store mutation requires a live, permit-derived
`RuntimeHomeMutationContext` for the exact canonical Runtime Home. The context
is process-local capability state rather than a persisted record and conveys
neither user authority nor Product Repository write permission. Ordinary
writers derive it from `SharedWriter`; setup derives it from
`ExclusiveSetup`. A setup conflict is rejected before Registry, project,
diagnostic, artifact, or operational-session records change.

Project-state records include:

- `project_state`, project workflow policy, Tasks, acceptance criteria,
  supplemental claims, Change Units, shaping checkpoints, shaping gaps, and
  their UserAction links;
- Write Tickets, Runs, current close basis, blockers, authority events, and
  immutable replay rows;
- evidence-capture intents and receipts, artifacts and links, evidence
  summaries, observations, and producers;
- `UserActionRequest`, immutable `UserActionResolution`, and project continuity;
- expected writes, Guard observations, prompt observations, and unrecorded
  changes used by reconciliation;
- normalized host sessions, turns, and hook tool invocations, plus managed MCP
  project sessions that retain their required thread and may precede their
  Registry runtime binding.

### Shaping checkpoint aggregate

`shaping_checkpoints` owns the Task, exact nullable predecessor, current scope
revision, optional baseline, summary, optional implementation boundary,
readiness, source/evidence refs, creation time, and optional supersession time.
The partial unique index permits at most one non-superseded checkpoint per
Task. Replacement requires the exact current checkpoint, marks that predecessor
`readiness=superseded`, and records its supersession time equal to successor
creation time. The predecessor foreign key enforces same-project and same-Task
lineage; predecessor identity is immutable and cannot self-reference.

`shaping_checkpoint_gaps` owns each gap's closed kind, summary, affected refs,
nullable stale-application reauthorization origin, and
`current|accepted|rejected|deferred|applied` status. A `ready` checkpoint cannot receive or
retain a `current` gap. `shaping_checkpoint_user_actions` is a same-project,
same-Task, same-checkpoint, same-gap link to the exact UserAction request and,
after User Channel resolution, its immutable resolution. A user-owned gap must
have exactly one such request link; a non-user-owned gap has none. Resolving a
linked request changes only its gap to the outcome-specific `accepted`,
`rejected`, or `deferred` disposition and may make a structurally complete
checkpoint `ready`. `volicord.update_scope` applies exact selected accepted
scope gaps; `volicord.advance_task` applies exact selected product, technical,
and sensitive gaps atomically with phase transition. Neither operation updates
every accepted gap by a Task-wide predicate. Applied gaps are terminal, while
sensitive resolution authority remains available to downstream policy.

`shaping_decision_applications` is the authoritative accepted-decision
application aggregate. Its deterministic identity binds one resolution and
semantic owner to the source checkpoint/gap/request, judgment kind, applying
scope revision, baseline, optional Change Unit, application timestamp, and
closed `current|stale|superseded` authority status. All semantic fields are
immutable. `stale_at` records only the current-to-stale boundary;
`superseded_at` records current-to-superseded or stale-to-superseded. No
transition returns an application to `current`.
`shaping_checkpoint_applications` links an
initial application to its source checkpoint and records every exact
predecessor-to-successor carry edge. Current authority therefore may originate
at an ancestor without copying its gap into the current checkpoint.

Every aggregate read strictly decodes identifiers, closed values, canonical
JSON arrays, timestamps, task ownership, predecessor ownership and timestamps,
and link consistency. A malformed or inconsistent member is corrupt persisted
owner data; Store does not omit it, invent a default, choose a checkpoint by
row ordering, or detach a linked current-basis UserAction through checkpoint
replacement.

`shaping_authority_reauthorizations` is immutable lineage keyed by one stale
application. Its closed outcome is `retired` or `reissued`. Every row binds the
same-project, same-Task stale application and old request to the exact successor
checkpoint. A `reissued` row additionally binds the fresh successor gap and
fresh request; a `retired` row has neither. Update and delete are forbidden.

Exact checkpoint replacement requires `carry_forward_application_refs` to be
the complete exact set of current compatible applications, rejects missing or
extra refs and authority-boundary conflicts, and writes successor lineage in
the same transaction. It may supersede the basis of rejected, deferred,
or expired linked requests only when `retired_non_authorizing_request_refs` is the
complete predecessor-owned set. Pending, accepted, applied, stale, foreign,
omitted, or extra refs are rejected. Retirement and successor aggregate
creation share the same transaction, while the request and immutable
resolution rows remain audit history.
The separate `stale_authority_actions` set must exactly consume every relevant
stale application. Each retire action writes terminal lineage. Each
reauthorize action supersedes the stale application and request basis, writes
the successor gap with the stale application origin, creates a distinct
current request with no resolution, and writes exact lineage in the same
transaction. Missing, duplicate, extra, cross-Task, cross-project,
wrong-status, owner-mismatched, or reused-identity members reject the whole
mutation.

`project_workflow_policies` is the authoritative project workflow-policy
record family. Its canonical aggregate contains the project identity, exact
`volicord.workflow_policy` schema identity, positive policy version, canonical
JSON for the complete `ProjectWorkflowPolicy`, matching canonical fingerprint,
closed source, and timestamps. Every read strictly decodes and validates the
complete typed policy, canonical JSON, fingerprint, source, version, and
timestamps before returning the record across the Store boundary. Any invalid
or contradictory member is persisted-data `Corrupt`; Store does not return a
partial policy, a default, or the repository-managed policy file as fallback.

Prompt-related Guard records are observations only. They are not a UserAction
resolution, user answer, verification basis, or authority source.

## Structured Diagnostic Findings

`diagnostic_findings` stores both explicit lifecycles and never infers one from
`runtime_session_id`. Every row has `lifecycle`; a current-state row also has
the full `current_identity_digest`, `diagnostic_scope_kind`, complete
`diagnostic_scope_identity`, validated opaque `current_subject_identity`,
`current_state_status`, and optional `resolved_at`. The subject identity uses
the exact `sha256:<64 lowercase hex>` token and never stores its canonical path
or other owner input bytes. Shared columns hold the namespaced code, domain,
stage, severity, source, bounded safe subject presentation and safe fact JSON,
bounded actions, applicable correlation coordinates, and canonical observation
time. Store validates the lifecycle type, subject identity token, and
serialized byte bounds before opening a write transaction. It never stores an
environment dump, raw request, unrestricted stderr, credential, canonical
subject input bytes, or unbounded fact object.

`insert_occurrence_finding` and `insert_occurrence_finding_graph` accept only
`OccurrenceDiagnosticFinding` values and insert immutable rows with generated
opaque occurrence IDs. `upsert_current_snapshot` accepts only
`CurrentDiagnosticFinding`, derives its ID from the complete key, compares all
persisted identity fields including `current_subject_identity`, and updates
only snapshot fields, including the safe subject presentation. It atomically
replaces outgoing causes and always leaves the condition active. Validation,
missing-cause, identity, or cycle failure preserves the previous snapshot.

`resolve_current_finding` addresses the row by `CurrentDiagnosticKey`, records
`resolved_at`, clears actions and outgoing causes, and retains facts and other
last-observed snapshot data. `active_current_findings_for_scope` returns only
active rows. `stored_diagnostic_findings_by_ids` and
`stored_diagnostic_finding_by_id` return `StoredDiagnosticFinding` values that
retain either the complete `OccurrenceDiagnosticFinding` or the complete
`CurrentDiagnosticFinding`. Exact current reads therefore retain
`current_state_status` and `resolved_at`. They reconstruct each current key
from the persisted subject identity rather than the safe subject presentation,
then recompute every current digest and ID before returning data; a malformed
subject identity or any mismatch is persisted-data corruption.
`reportable_diagnostic_findings_by_ids` projects only immutable occurrences and
active current-state rows into current-report findings. Resolved current-state
rows are excluded as current-report seeds but remain available through exact
ID lookup. Current identity columns and occurrence rows are also protected by
Registry update triggers.

`diagnostic_cause_edges` stores one directed finding-to-cause edge. Both ends
must name existing findings, the composite primary key rejects duplicates, and
the insert trigger plus Store graph validation reject cycles. Store inserts all
findings before their edges in one immediate transaction, so a rejected graph
leaves neither a partial finding set nor a dangling edge. Cause queries order
by depth and finding ID, reject a requested depth above 32, return at most 128
distinct findings, and report when the selected depth cut off another edge.
`bounded_stored_diagnostic_graph_from_seeds` returns the same
`StoredDiagnosticFinding` lifecycle shape for every entry, so occurrence,
active current, and resolved current causes retain their exact stored state.

Finding reads are available by explicit ID, by runtime occurrence session, and
by exact active current scope. A runtime-correlated occurrence must also carry
that runtime's Connection and integration revision. The current-Connection
convenience query filters the exact Connection scope to its current integration
revision. These Registry findings remain separate from bounded
non-authority counters in `diagnostics.sqlite`. A current Connection report
starts from the finding IDs explicitly referenced by its checks and resolves
their bounded cause chains through a provenance-bearing overlay. The overlay
uses an inline current-evaluation finding before Store lookup, then resolves an
explicitly persisted seed from occurrence or active current-state rows. A
missing-record finding is valid only when such an explicitly persisted seed is
absent from the Store. The report includes an otherwise independent current
finding only when that report operation deliberately selects it, and it does
not bulk-load every finding stored for the same revision.

## Authoritative Operational Sessions

`managed_mcp_launch_leases` is the Registry evidence-integrity boundary between
the hidden host launcher and MCP bootstrap. Each row stores an opaque lease ID,
Connection, `codex` host kind, expected Connection integration revision,
expected managed launch fingerprint, issue and expiry times, optional consumed
time, and exact `issued`, `consumed`, `cancelled`, or `expired` terminal state.
The short-lived lease is consumed once in the same transaction that creates its
`managed_host` runtime. Replay, expiry, Connection mismatch, revision mismatch,
fingerprint mismatch, or a non-current Connection creates no runtime. Launcher
failure cancels an unused lease, and bounded cleanup expires or removes old
terminal rows. Explicit final Connection removal deletes its remaining lease
inventory before deleting the Connection. Lease records are not OS actor
credentials or reusable secrets.

`mcp_runtime_sessions` is Agent Connection-owned application state. Volicord
creates its opaque `runtime_session_id` when the MCP process starts, before
host thread metadata exists. `session_source` is exactly `managed_host`,
`manual_cli`, `cli_preflight`, or `integration_probe`. `managed_host` is the
lease-bound managed launcher source; `manual_cli` is the public stdio or
disposable CLI-conformance source; `cli_preflight` and `integration_probe` are
non-managed diagnostic classifications. The current public preflight is
read-only and creates no runtime row. Only successful atomic launch-lease
consumption creates `managed_host`; every other source can be inspected but
can never satisfy a managed-host operational-evidence lookup or authorize a
managed call.

Each physical `agent_connections` row has one unique immutable opaque
integration-instance ID generated by Store when that row is inserted. Store
preserves it across compatible registration replay, enabled-state and
verification updates, staged activation and cleanup recovery, and mode
transitions. Physical deletion removes it with the row; later insertion of the
same deterministic Connection identity receives a new value.

The connection integration revision is a domain-separated canonical digest of
the Connection identity, immutable integration-instance ID, host kind, intent,
scope, mode, server name, configuration target, exact managed-configuration
fingerprint, and nonnegative Store-owned integration generation. The managed
fingerprint includes the current server command and entry and identifies the
Volicord-managed host configuration that the setup owner last successfully
applied or adopted. Only an explicit setup-owned managed-configuration write
may change it. When that write changes the fingerprint, the same Registry
transaction clears `verification_report_json`; replay with the same
fingerprint may retain the report. Host and client version fields remain
diagnostic observations. The current owner fields and Store-owned generation
derive the lifecycle revision. Store increments the generation exactly once in each
successful real mode transition and never for a same-mode no-op. The generation
distinguishes revisions within one physical Connection instance; the immutable
instance ID distinguishes physical deletion and recreation. Together they are
Store-owned local lifecycle and correlation coordinates.

A real mode transition atomically changes the Connection mode and generation,
clears its verification report, and replaces only the integration revision in
every owned strict Guard manifest. The complete current manifest inventory must
match one-for-one with Connection Project membership before mutation. Missing,
duplicate, stale, malformed, owner-mismatched, or partially writable inventory
causes the whole Registry transaction to fail without a partial Connection or
manifest update.

Milestone timestamps and facts express lifecycle state without a redundant
status enum. `attempted_client_name`, `attempted_client_version`, and
`requested_protocol_version` are recorded as soon as those bounded values are
parsed, including when later initialize validation fails.
`selected_protocol_version` is the revision selected and returned by the server
when initialize completes; `negotiated_protocol_version` remains null until the
valid initialized notification fully completes that selected profile's
handshake. Store separately records initialize completion, every actual
`tools/list` time, canonical sorted `returned_tool_identities_json`, required-
tool-set fact, and `required_tools_validated_at`, plus the exact successful
verification-tool identity/time pair `verification_tool_name` and
`verification_tool_observed_at`, one terminal structured finding ID, and
graceful close. Required-tool success requires the list observation and
returned inventory; verification-tool success requires same-session required-
tool validation. The verification pair is both null or both present. A present name is an
MCP-compatible 1 through 128 byte ASCII name, and its observation timestamp is
not earlier than required-tool validation. Store accepts this pair only
for a current enabled `managed_host` runtime and current Connection revision;
`cli_preflight` cannot write it. A protocol success is not emitted when its
authoritative Store write fails. Best-effort diagnostics remain separate and
cannot make an otherwise valid tool result fail.

Store converts one row to `McpSessionMilestones` only when all milestone
relationships are coherent. A `ManagedCapabilityProof` additionally requires
`session_source=managed_host` and the complete process, initialize,
initialized-notification, `tools/list`, required-tool, and canonical
verification-tool chain in that row. For one current integration revision,
selection names the newest managed row `latest_managed_attempt` and the newest
complete row `latest_managed_capability_proof`; it never merges rows. The selected peer's
`clientInfo` is the authoritative protocol peer observation. The separately
probed PATH executable version remains diagnostic and does not select the
proof. Persisted connection reports retain all selected session IDs and their
roles, deduplicating one ID that carries both roles.

Project host correlation is normalized by source. The
`CodexMcpTurnMetadata` decoder supplies MCP session/thread/turn correlation,
while the distinct `CodexCommandHooks` decoder supplies prompt session/turn or tool
session/turn/tool-use/tool-name correlation. The host-contract owner maps both
markers to their reviewed profile IDs. `host_sessions` names the
Connection, exact native host session, immutable project integration revision,
and first/last observation times. Store derives its revision-scoped local
session ID from the Connection internal ID, exact revision, and native session.
`host_turns` records exact turns for that local session.
`host_tool_invocations` records a hook tool-use ID and canonical tool name
under its exact session and turn. Reusing a tool-use ID with another turn or
tool name is rejected.

`guard_events.correlation_kind` is `codex_hook_prompt` or
`codex_hook_tool` when the event is compatible. `prompt_capture` requires a
session and turn and forbids tool-use fields. `pre_tool` and `post_tool`
require session, turn, tool-use ID, and canonical tool name. Prompt captures
reference the exact host turn. Repository observations reference the exact
host tool invocation; expected writes and correlated Unrecorded Changes
reference that exact observation. Rust inputs carry the corresponding
`HostNativeCorrelation` variant, and SQL checks plus composite foreign keys
reject incomplete, cross-phase, and cross-Connection shapes. None of these
hook records has a host-thread field.

`managed_mcp_sessions` is the separate MCP-only project anchor. It references
the normalized host session and latest host turn, requires a host thread, and
has an optional Registry runtime attachment. A Guard observation never creates
this row. Empty, sentinel, fabricated, or CLI-preflight runtime coordinates do
not represent an unattached MCP anchor.

The first actual managed MCP tool call for the same host session uses four
ordered stages. Store validates the current `managed_host` runtime and its
Connection revision without mutation. In an immediate project transaction it
then creates or validates an unbound `managed_mcp_sessions` anchor and rejects any
Connection, native-session, thread, immutable-revision, or existing-runtime
conflict. Only after that commit does an immediate Registry transaction
revalidate the runtime, Connection, project membership, current project
identity, and exact anchor coordinates before inserting
`mcp_runtime_project_session_bindings`. A final immediate project transaction
revalidates the anchor and attaches the runtime only when the field is null or
already names the same runtime.

The Registry reservation supplies the uniqueness boundary that a foreign key
cannot express across separate SQLite databases. A deterministic project
ownership conflict leaves no new reservation. A valid anchor can remain
unbound when Registry reservation fails, but it cannot authorize Core. A
reservation can remain without project attachment after an interrupted final
write, but it also cannot authorize Core. Exact replay under unchanged owner
state reuses that reservation and completes the attachment. The reservation
stores the same exact project integration revision as the project row, and
authorization validates that match. Conflicting runtime, Connection, project,
revision, or host-session Registry claims fail. A partial project index makes
non-null runtime attachment unique while allowing any number of unbound MCP
anchors.

Only an attached `managed_mcp_sessions` row with an exact current Registry
binding can authorize Core. Hook-only normalized rows can retain Guard events
and prompt captures but cannot authorize a managed call. Runtime rows
themselves are historical process observations: a crashed row
may remain apparently open, and concurrent current rows may coexist without
blocking or selecting Guard correlation.

Runtime authorization reads these current records directly. It accepts only an
enabled Connection, a current Connection Project membership, a
`session_source=managed_host` nonterminal runtime session for that Connection, and a
project managed MCP session owned by the same runtime session, Connection, and
project. The stored Connection and project integration revisions must equal the
revisions derived from current owner inputs. The Connection mode must allow the
requested operation category. `cli_preflight` rows, diagnostic version fields,
and best-effort diagnostics cannot satisfy this boundary.

Registry storage authorizes managed operations only through the rows above. A
Runtime Home with a noncurrent authorization schema belongs to a different
`StorageManifest` and is rejected by exact-open validation.

Connection Project retirement treats these rows as connection-owned Registry
integration state. Explicit removal and replacement cleanup atomically delete the
selected membership's `mcp_runtime_project_session_bindings`, project-scoped
`guard_installations`, and then the `connection_projects` row. If a superseded
multi-project Connection retains memberships, its Agent Connection, every
`mcp_runtime_sessions` row, and other projects' bindings and Guard Installations
remain. Explicit final-membership removal deletes every remaining binding and
Guard Installation owned by the Connection, then its runtime sessions and
`agent_connections` row.

Last-project replacement cleanup instead keeps the disabled old membership, its bindings
and Guard Installation, and the pending-host-cleanup marker as one retry
inventory until host cleanup and final Registry revalidation succeed. Final
cleanup deletes those project-owned rows and membership together and clears the
marker, while the disabled zero-membership historical Connection and its
connection-wide runtime sessions remain. Project registrations, installation
profiles, Runtime Home records, and all project `state.sqlite` rows remain
outside every retirement set.

Retained project-local Agent Sessions and Guard or workflow history do not
become future authority. Runtime authorization still requires the current
Registry membership, runtime session, and project-session validation described
above. Reusing one native host session after a real mode transition or after
physical Connection deletion and recreation therefore selects a new
revision-scoped project row without colliding with retained history.

## Identity And Ownership

Stored identifiers are exact, non-empty owner values. Store does not trim,
guess, reassign, or derive a replacement identifier from display text. Every
Task-scoped row names the same project and Task as its owner. Every Change
Unit, evidence target, Run, artifact link, blocker, and continuity ref is
validated against its owning coordinates.

Current pointers must reference current same-project records. Immutable history
may remain after current state advances, but it never becomes current through
timestamp comparison or record-name ordering.

The Store-owned current effective shaping authority graph is a typed relational
read, not a creation-order selection. It binds one open Task to its exact current
checkpoint gaps, current UserAction bases and immutable resolutions, current
checkpoint-application lineage, current applications, stale application
recovery obligations, scope revision, baseline, and Change Unit. The graph
rejects detached or cross-owner current authority, duplicate current
request/resolution authority, a current application backed by a superseded
request, a superseded application linked as current, and malformed immutable
source coordinates. Explicit history reads retain every superseded request and
application for diagnostics, lineage inspection, and export without supplying
workflow blockers. A terminal Task has no current graph; its exact shaping
lineage remains in those history reads.

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
read. `StoredUserActionRequest`, `StoredUserActionResolution`, and
`StoredUserActionRecordSet` carry the validated boundary. Their
invariant-bearing fields are private; public typed accessors expose semantic
facts without permitting callers to assemble a contradictory persisted
record. Store rejects:

- unknown or mixed union tags and extra fields;
- missing kind-specific fields;
- an `action_kind` inconsistent with the request body;
- a basis, `required_for`, or expiry duplicated representation that disagrees
  with the closed request;
- option or evidence selections outside the stored candidates;
- a non-CLI channel or non-local-user provenance;
- invalid limits, timestamps, refs, or submission identity; and
- a resolution whose request identity, action kind, project, Task, or current
  basis does not match.

Reads that need a request and resolution together return one validated
`StoredUserActionRecordSet`. The normal public Store API cannot return or
construct an invalid set. A canonical typed in-memory projection used before
commit passes the same Store-owned consistency checks and does not expose an
unchecked constructor.

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
expectations, required typed hook phases, `host_contract_profile`, and
`host_contract_digest`. The current Guard profile is explicitly
`codex-command-hooks`; audit requires its deterministic reviewed digest rather than
choosing a parser from an incoming payload. File audit and required-phase
observation derive the current Guard checks from this manifest and current
owner-matched facts.

Policy commands and runtime commands are intentionally different projections.
The canonical policy commands omit `--policy-hash`; after hashing that policy,
runtime commands add `--policy-hash <exact-hash>`. Hook wrappers and the Guard
manifest use the same typed runtime commands. Audit compares their shared owner
fields and command segments individually and never compares the two complete
command objects for equality.

Project `guard_events` bind every observation to the Guard installation,
policy hash, integration revision, typed hook phase, and contract status.
`UserPromptSubmit` becomes `prompt_capture` with session/turn correlation.
`PreToolUse` and `PostToolUse` become `pre_tool` and `post_tool` with
session/turn/tool-use/tool-name correlation. They never require or store a
thread coordinate. The same typed tool correlation, including canonical tool
name, must match across related pre/post records.
Current compatible events derive `guard_observation`; older hashes or revisions
remain historical and cannot satisfy it. A current malformed or incompatible
event makes that check fail. Prompt capture remains a fact within the same
observation summary, not a separate installation state. Routed MCP hook events
retain hashes and bounded normalized correlation, but not the unrestricted raw
event, tool input, or tool result.

Registry `guard_integration_verification_runs` are durable bounded
Connection-integration records, not Core or Task records. Each row stores its
opaque verification ID and immutable semantic coordinate: Connection, project,
managed runtime session, native host session and turn, integration revision,
Guard Installation, host-contract profile, hook-definition digest, and policy
digest. An unconditional unique constraint permits exactly one row for that
coordinate, including terminal rows. A prompt event is likewise owned by only
one attempt in the turn. Exact begin replay therefore returns the same ID and
current state; elapsed time never replaces it.

The row also stores the expected typed probe and host callable, semantic
observation-policy kind, optional deferred deadline, allowed and consumed
status reads, creation and cleanup times, acknowledgement and completion,
matched prompt/pre/post events, and typed terminal repair reason, retry policy,
code, and summary. Status is exactly `awaiting_probe`,
`awaiting_observation`, `complete`, or `repair_required`. SQL checks enforce
each state's exact nullable fields. Coordinate updates, a second probe
acknowledgement, terminal mutation, and terminal-to-active transition are
rejected. Cleanup time only bounds retention.

Strict projection parses these lifecycle fields as `UtcTimestamp` values and
validates their order. Creation cannot follow acknowledgement or terminal
completion; acknowledgement cannot precede creation or follow terminal
completion; and a persisted acquisition observation cannot precede creation or
follow the terminal transition of its owning run. `complete` requires its
completion timestamp and matching completed proof. `repair_required` requires
its terminal completion timestamp. Missing, malformed, or inconsistent stored
times are corrupt persisted owner data rather than an invitation to use a
caller observation or report-evaluation time.

Registry `guard_probe_observations` records the acquisition boundary separately
from correlated completion. Its closed `stage` values are
`probe_acknowledged`, `unrelated_routed_tool`, `hook_event_not_observed`,
`hook_payload_incompatible`,
`callable_identity_unknown`, `callable_identity_mismatch`,
`verification_id_mismatch`, `session_mismatch`, `turn_mismatch`,
`tool_use_mismatch`, `pre_tool_matched`, and `post_tool_matched`. Each row
stores the expected `volicord.guard_probe` agent-tool identity and expected
host callable, an optional bounded observed callable, optional hook kind,
verification-ID presence/match booleans, Guard Installation, integration
revision, and observation time. It stores no prompt, full payload, tool input,
or tool output. An absent event therefore records only
`hook_event_not_observed`, without claiming a proven routing cause.
`unrelated_routed_tool` is nonterminal bounded trace for workflow controls and
other known routed tools, plus an unknown same-server callable that does not
claim the exact current verification ID. It cannot supply proof, consume the
status-read budget, or select acknowledgement, repair, retry, or root finding.

Store projects those authoritative row facts once into the shared tagged
`IntegrationVerificationWorkflowState`. It maps the four stored states
directly and exposes a canonical `AgentToolId` only for the two nonterminal
states. The repair reason is separate from
`no_automatic_retry`, `new_turn_required`, `host_reload_required`,
`hook_review_required`, or `repair_required`. Begin, probe, get/status,
adapters, and renderers consume that state. Persistence does not construct
user-facing next-action prose or own renderer wording.

Connection reporting queries the newest current-revision Guard run as
`guard_verification_attempt` and independently queries the newest completed
current-revision run as `guard_verification_proof`. The latest run alone
determines the correlated check: active is pending, complete is passed, and
`repair_required` is failed. An older completed row remains proof evidence but
cannot override a newer failed attempt. Report context deduplicates their
runtime session with managed MCP roles and retains all relevant verification
IDs.

The typed attempt and proof projections own the correlated check's evidence
timestamp selection. Attempt creation is decisive for `awaiting_probe`; the
latest applicable acquisition observation, acknowledgement, or creation is
decisive for `awaiting_observation`; the matching proof completion is decisive
for `complete`; and terminal completion is decisive for `repair_required`.
No run supplies no check observation time. Store does not receive a report
evaluation timestamp merely to project this persisted workflow state.

Repository observations, expected writes, and Unrecorded Changes are
project-local. One observation is unique on its normalized project,
Connection, host session, host turn, hook tool-use ID, and canonical tool-name
coordinate. `open` requires a verified repository baseline; `complete` requires
a verified repository outcome and deterministic repository delta;
`unavailable` requires a closed observation-unavailable reason and contains no
complete delta. Each expected write belongs to that exact observation. Each
Unrecorded Change links to it, contains a non-empty canonical path set, and
stores the unmatched-delta digest.

The canonical closed lifecycle reasons include `post_tool_not_observed` for
prior turns closed by the next accepted prompt in the exact project session,
and `managed_session_terminated` for remaining opens closed through an
authoritatively terminal managed runtime's exact Registry bindings. Both
terminal forms retain the baseline, completion time, and stable result while
leaving post-tool event, outcome, delta, expected-write matches, and
Unrecorded Changes absent. Prompt capture and prior-turn closure commit in one
immediate bounded project transaction. Managed-runtime cleanup commits one
bounded immediate transaction per exact bound project session and is
idempotent across recovery.

Prompt observations may be stored only under their bounded observation schema.
They do not carry a user choice, resolution body, private resolution form, or
credential.

## Current Close Basis And Continuity

The current close basis belongs to the Task and is distinct from terminal close
history. Absence is represented as absence, not a generated empty basis.
Evidence and acceptance refs must remain exact and current under their owners.
Direct/work bases identify an exact compatible source Run. Advisor bases have
no source Run and instead identify the exact current shaping checkpoint and
the exact set of current shaping decision applications linked to it. Store validates current Task,
scope revision, close-basis revision, baseline, Change Unit, mode-compatible
lineage, result/evidence refs, residual risks, and update time as one aggregate.
An advisor basis whose Change Unit violates the canonical non-write predicate
or whose checkpoint is not exact current state is corrupt persisted data.

Project continuity records are durable context, not a waiver. Their typed cursor
and ordering belong to the status method. Carry-forward never bypasses current
scope, baseline, Write Ticket, evidence, UserAction, or close checks.

## Storage-Owned Structured Values

Every Store-owned structured authority field uses a closed typed schema,
canonical encoding where a digest depends on bytes, and explicit size limits.
Unknown, missing, duplicate, wrongly typed, noncanonical, or owner-inconsistent
members are invalid input and corrupt persisted data.

Physical row shapes, serialized JSON, closed `TEXT` values, and persisted
timestamp parsing remain private to Store. Store decodes Task, Change Unit,
workflow-policy, Write Ticket, Run, evidence, artifact, project-state, replay
identity, reconciliation observation, project continuity, `StoredRecordRef`,
and UserAction owner state before any of those facts cross a public
Store-to-Core or Store-to-service interface. The resulting records carry typed
closed values, actors, timestamps, JSON objects, Product Repository paths,
workflow policies, Write Ticket validity bases, and attempt scopes. Mutation
interfaces accept the corresponding typed values and Store performs their
single physical serialization at the SQLite boundary.

A malformed JSON value, unknown closed string, invalid timestamp, invalid
Product Repository path, or contradiction among duplicated physical columns
produces the Store-owned persisted-data `Corrupt` failure. It is not
reinterpreted through an empty value, a default, or another column. Store
corruption constructors are internal to Store. Core and semantic services
consume the validated fields directly and do not parse their persisted
representations or construct Store corruption. A relationship failure among
multiple fields of one persisted aggregate is still Store corruption and uses
that aggregate's closed invariant identity rather than an arbitrary column.
A semantic contradiction among fully validated typed facts and current
operation or policy facts is instead owned by the consuming Core or service
policy.

### Write Ticket Physical Ownership

The Write Ticket Store aggregate is the only implementation owner of the
physical `write_tickets` table, its columns, row projection, JSON and closed
value decoding, timestamp decoding, Product Repository path decoding, and
persisted cross-field invariants. Normal reads and reads within an existing
Store transaction use the same canonical row projection, decoder, and
invariant validator. Filtering for active tickets occurs only after each
selected row has become a validated `StoredWriteTicket`. All
`StoredWriteTicket` fields are private outside the aggregate; consumers use
read-only semantic accessors and cannot construct, update, or destructure a
persisted record. No public compatibility alias exists.

The decoded `WriteTicketValidityBasis.approval_basis_refs` collection uses the
dedicated `UserActionResolutionRef` contract. Typed JSON decoding requires the
fixed `user_action_resolution` kind, complete project, `Task`, and resolution
identity, and a concrete produced state version. Aggregate validation then
requires every reference's project and `Task` to equal the physical ticket
owner and rejects duplicate full project/`Task`/resolution identities.

Other Store modules receive a complete typed Write Ticket or a focused typed
view produced from that record. In particular, workflow-policy persistence
uses a typed authority-binding view and does not query `write_tickets`, parse
`validity_basis_json`, or construct ticket corruption. It compares the
validated binding with current workflow-policy authority as a semantic policy
decision.

An unpersisted ticket proposal is a distinct Core-owned
`PlannedWriteTicket`, not a synthetic `StoredWriteTicket`. Core validates its
semantic identity, scope, authority, timestamp relationships, and one
invariant-bearing `WriteTicketPathScope` before projection. The same planned
value supplies both the projected ticket facts and the fully typed
`WriteTicketInsert`, including that exact path scope; only Store maps the
insertion input to physical columns and JSON. Store reconstructs one validated
`WriteTicketPathScope` from the physical allowed and denied columns before
returning an opaque stored ticket. A dry-run issue branch has no persisted
ticket identity and cannot produce a Store insertion input. Reuse reads an
already validated stored ticket and exposes its immutable path-scope view.

Corruption local to one physical Write Ticket field identifies that exact
field. A relationship among fields, including owner identity, scope revision,
baseline, timestamp ordering, path-set, path-coverage, write-intent, or status
lifecycle disagreement, identifies the Write Ticket aggregate and a closed
invariant code. Approval-owner disagreement and duplicate approval-resolution
identity are aggregate invariant failures rather than JSON-column
attribution. Expiry relative to an operation time, incompatibility with the
current policy, or lack of coverage for a requested operation remains a
semantic rejection when the persisted ticket itself is internally valid.

Exact committed method response bytes are a deliberate semantic-owner
exception. Store validates the persisted replay carrier and its typed replay
identity, preserves `response_json` exactly, and does not reinterpret the
method result. Core validates and decodes that exact result against the owning
method contract when replay or operation-result behavior requires it.
Core-owned event payload bytes follow their Core event contract and are not
exposed as typed Store authority-record fields.

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

Within the report's `mcp_server` check, `preflight` and
`last_active_verification` are separate sibling evidence records.
`preflight.evidence` is immutable after construction, always keeps
writeability `not_checked`, and always has an empty side-effect array.
`last_active_verification` is null when no active evidence exists or contains
the latest active Registry/project write results, disposable conformance
results, timestamp, source, and side effects. Store accepts no combined
preflight/write result; a combined shape is invalid stored owner data.

Report replacement accepts the exact expected typed Connection integration
revision, not a caller-supplied fingerprint. In one immediate Registry
transaction Store loads and validates the revision owner fields, compares the
current revision, and updates only `verification_report_json` and the ordinary
row update timestamp. A mismatch is a conflict with no write. This boundary
allows explicit replacement of a malformed stored report but does not repair
malformed metadata or other owner state. An action with missing or unknown
typed members, noncanonical check/root references, or owner/channel/check
metadata inconsistent with its ID is one such malformed report; strict reads
reject it, while active verification may replace the whole report under the
unchanged revision. Verification therefore cannot adopt managed configuration
or change the Connection integration revision.

Store validates the shared report type before write and after read, including
closed values, bounds, deterministic ordering, duplicate rejection, and the
derived aggregate. Malformed or noncanonical report JSON is corrupt persisted
owner state. It is not interpreted as no report and is not repaired from
another column.

Absent optional check members are omitted from canonical JSON rather than
stored as explicit null. Every action contains exactly its semantic `id` and
user `instruction`; no executable invocation is persisted. This report remains
the sole stored check/action state. CLI command output projects those members
at top level and does not persist a second command-output tree.

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
