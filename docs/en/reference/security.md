# Security

This document owns supported security guarantees and explicit non-guarantees
for the local Codex workflow. It does not define method schemas,
storage effects, Codex configuration syntax, or operating-system policy.

## Boundary Summary

Volicord is a cooperative local authority record. It validates and records
owner-defined workflow state, but it is not a sandbox, access-control system,
malware defense, network isolation layer, tamper-proof audit log, or proof that
a model followed instructions.

## Supported Guarantees

Within the owner-defined local boundary, Volicord guarantees:

- strict typed validation before Core or Store commits;
- explicit Task, scope, Change Unit, Write Ticket, evidence, UserAction, and
  close-state transitions;
- no-effect behavior for owner-defined rejection branches;
- current Connection, project membership, mode, and authoritative managed-host
  session validation for Agent Connection calls;
- Runtime Home and Product Repository separation;
- lexical Product Repository path validation separated from platform-owned
  canonical root, link, and containment observation before Core accepts a
  request path;
- CLI-only UserAction resolution with `resolved_by_actor_source=local_user`;
- managed stdio MCP without a network listener; and
- machine-readable failure categories instead of permissive fallback.

These guarantees apply only to Volicord processing. They do not control what
Codex, the shell, tools, the filesystem, or external systems do outside that
processing boundary.

## Sensitive Actions And User Judgment

A Write Ticket is Core authority state, not filesystem permission. Sensitive
approval, final acceptance, residual-risk acceptance, cancellation, and other
user-owned judgments cannot be supplied by an Agent Connection. An MCP agent
may create a pending request, but the user resolves it only through the local
CLI inbox.

Guard prompt-related observations do not become user answers. A stored
resolution is authoritative only when the strict typed request, selected stored
option or evidence candidates, CLI provenance, submission identity, and current
basis all validate.

## Local Connection Assumptions

The managed Codex process and `volicord` run under the local user's operating-
system account. Volicord does not authenticate that OS user or turn process
identity into human identity. Agent authorization proves only locally observed
cooperative runtime/project session ownership, current Connection Project
membership, current integration revisions, and permission under the current
Connection mode.

The immutable Store-owned Connection integration-instance ID and the
integration generation distinguish local Registry lifecycle revisions. Together
with current owner inputs, they derive local lifecycle and correlation
coordinates, and callers cannot select them.

A Guard-only project session with no runtime binding is correlation history,
not invocation authority. Core authority additionally requires a current
managed-host runtime and an exact Registry runtime/project/host-session
reservation attached to the exact current project row. Project ownership is
validated before that reservation is created, so a deterministic Connection,
project, Guard Installation, revision, native-session, thread, or attached-
runtime conflict leaves no new Registry reservation. An unbound project row is
not authority, and a reservation left by interruption before project
attachment is not authority. Exact replay may complete that attachment only
under unchanged owner state. Runtime rows are not process-liveness claims: an
apparently open crashed row is historical, and concurrent rows may coexist
without authorizing one another or being guessed for a Guard event.

Executable paths, process metadata, client name/version, host version,
environment values, and host thread/turn metadata are diagnostic or correlation
facts, not actor or human identity. Thread and turn metadata may correlate the
supported workflow, but cannot widen Connection or project authority. Internal
runtime and revision-scoped project session IDs are likewise private local
correlation coordinates.

Package version and structured build provenance are also diagnostic and
correlation facts. Profile-class precision alone does not weaken that boundary
when the other required provenance is known. A dirty tree explicitly limits
source reproducibility, and complete-looking build metadata is not proof that
the executable is trusted, untampered, correct, or built from unmodified source.

The supported MCP process uses stdin/stdout and opens no network transport
listener. This is a process topology fact, not network sandboxing: Codex or
tools may use the network independently.

## Authority Boundaries

Product Repository files are user product data. Runtime Home rows are Volicord
authority records. Managed Codex configuration starts a process but is not
authority, approval, a Write Ticket, or proof that Codex loaded it.

A validated relative Product Repository path is not filesystem permission or
proof of future containment. The platform observation is a bounded local fact
at the time Core obtains it. Semantic services and stored authority records do
not carry canonical absolute paths or independently reopen caller path text.

Behavioral connection observations establish compatibility for the current
managed configuration and observed protocol, tool, safe-call, and Guard
behavior. Core authority separately validates the current enabled Connection,
project membership, mode, managed runtime session, revision-scoped project
session, and exact Registry/project binding. These cooperative records do not
establish actor, client, operating-system-user, or human identity, complete
monitoring, or future host behavior.

<a id="historical-operation-result-access"></a>
## Historical Operation-Result Access

`volicord.get_operation_result` returns only the exact eligible immutable
response bytes selected by its owner-defined identity and pagination rules.
Access never widens merely because the caller knows an ID. Cross-project,
cross-connection, ineligible, corrupt, or unavailable records fail without
revealing private content.

<a id="generated-displays-and-text"></a>
## Generated Displays And Text

Generated guidance, CLI prose, MCP text content, status summaries, and templates
are displays, not separate authority records. They must derive from current
typed state, omit secrets and private UserAction content, and preserve the
structured result's guarantee boundary. A stale display cannot authorize work.
Concise human output may select only facts applicable to its command and
current branch while verbose or JSON output exposes more diagnostic context.
Omitting an inapplicable field or repeating fewer non-guarantee explanations
does not create a positive guarantee; the typed disclosure remains
authoritative for the result.

Doctor action displays derive from one finalized typed remediation plan.
Finding and check provenance is bounded diagnostic context and does not prove
actor identity, grant write authority, approve a command, or show that a
remediation ran. Compact output may show only the primary action, while verbose
and JSON output show the plan's grouped or structured projections; every form
preserves the same action identity, urgency, and ordering.

Doctor privacy-footprint displays likewise derive from one canonical typed
definition and one typed report. Every claim belongs to exactly one of
`stores`, `does_not_store`, or `does_not_prove`; `doctor_output_scope` is a
separate scalar and the sole output-scope statement. A `does_not_store` claim is
bounded to the Store it names and does not imply that a Product Repository or
another Volicord Store cannot contain corresponding authority metadata. Human
and JSON output preserve the same canonical UTF-8 claim text.

Version and Doctor human output derive build-provenance labels from one typed
vocabulary. Missing recorded build facts are `not recorded`, while exact JSON
values remain unchanged, including `class_only`. Doctor keeps strictly decoded
storage metadata and Hook assessments structured in JSON and projects them as
focused verbose sections with explicit human states. It does not turn missing
evidence into `no`, flatten a structured value into text, or parse an unchecked
stored string in the renderer.

### Workflow-Policy Inspection

The complete authoritative workflow policy is inspectable local configuration,
not a credential. `policy show --verbose` and `policy show --json` may display
its exact MCP command, argument vector, and static `mcp.env` entries. The
current strict policy contract permits only string-valued `VOLICORD_HOME` in
that static environment object; this value is a local Runtime Home binding, not
a secret. The command does not enumerate the invoking process environment or
accept additional environment names. A policy carrying an unknown or
disallowed environment member fails strict decoding instead of being partially
displayed or silently redacted.

## Guard And Unrecorded Changes

Guard and reconciliation records are bounded observations. They do not prove
who changed a file, malicious intent, complete monitoring, or prevention.
One exact invocation-scoped observation records only the net Product Repository
transition between its persisted baseline and outcome. An exact expected write
may cover only matching paths in that complete delta. An `unavailable`
observation remains visible and cannot be treated as a complete empty delta or
as evidence that no change occurred. The invocation window observes a
transition; it does not establish actor identity or exclusive causation. Exact
capture, matching, and finding rules belong to
[Repository Observation](repository-observation.md).

Hook path safety is a bounded typed diagnostic assessment, not filesystem or
process enforcement. `verified` establishes that the current owner-bound Guard
manifest, Codex hook configuration, Git-root dispatch, every required phase
wrapper, managed invocation fields, content hashes, policy hash, host output,
and permissions match the reviewed current contract. It does not prove that
Codex executed the Hook or that another process cannot bypass it. `failed`
requires an observed current contract violation. `not_recorded` and
`not_checked` preserve missing or unavailable evidence and never claim that
path safety, CWD independence, or subdirectory safety was disproved. The audit
reads the current Store and managed artifacts without modifying authority or
repository state.

## Explicit Non-Guarantees

Volicord does not guarantee:

- filesystem, process, shell, command, network, credential, or secret isolation;
- that Codex honors guidance, tool descriptions, or managed instructions;
- actor attribution from process, path, timing, prompt, or observation data;
- actor attribution from client name/version, host version, environment values,
  or local session metadata;
- complete detection or prevention of Product Repository changes;
- continued path existence or containment after a completed platform
  observation;
- correctness, test sufficiency, QA, deployment readiness, or human review;
- that Close Status replaces final user acceptance where acceptance is required;
- that configuration presence proves active tool exposure;
- that a release result from one platform applies to another platform; or
- recovery, decoding, or automatic conversion of unsupported stored or external
  contract formats.

## Related Owners

- [Scope](scope.md)
- [Agent Connection](agent-connection.md)
- [MCP Transport](mcp-transport.md)
- [Failure Model](failure-model.md)
- [Runtime Boundaries](runtime-boundaries.md)
- [API User-Action Schemas](api/schema-user-action.md)
