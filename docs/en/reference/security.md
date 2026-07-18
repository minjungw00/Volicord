# Security

This document owns supported security guarantees and explicit non-guarantees
for the first-release local Codex workflow. It does not define method schemas,
storage effects, Codex configuration syntax, or operating-system policy.

## Boundary Summary

Volicord is a cooperative local authority record. It validates and records
owner-defined workflow state, but it is not a sandbox, access-control system,
host attestation service, malware defense, network isolation layer, or proof
that a model followed instructions.

## Supported Guarantees

Within the owner-defined local boundary, Volicord guarantees:

- strict typed validation before Core or Store commits;
- explicit Task, scope, Change Unit, Write Ticket, evidence, UserAction, and
  close-state transitions;
- no-effect behavior for owner-defined rejection branches;
- current Connection, project membership, mode, and authoritative managed-host
  session validation for Agent Connection calls;
- Runtime Home and Product Repository separation;
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

A Guard-only project session with no runtime binding is correlation history,
not invocation authority. Core authority additionally requires a current
managed-host runtime and an exact Registry runtime/project/host-session
reservation. Runtime rows are not process-liveness claims: an apparently open
crashed row is historical, and concurrent rows may coexist without authorizing
one another or being guessed for a Guard event.

Executable bytes, executable paths, process identity, client name/version,
host version, environment values, and host thread/turn metadata are not actor
or human identity credentials. Thread and turn metadata may correlate the
supported workflow, but cannot widen Connection or project authority.

The supported MCP process uses stdin/stdout and opens no network transport
listener. This is a process topology fact, not network sandboxing: Codex or
tools may use the network independently.

## Authority Boundaries

Product Repository files are user product data. Runtime Home rows are Volicord
authority records. Managed Codex configuration starts a process but is not
authority, approval, a Write Ticket, or proof that Codex loaded it.

Behavioral connection observations do not grant Core authority, identify a
user, certify executable provenance or identity, or prove future host behavior.
Production runtime authorization does not consult executable digests or host
version allowlists.

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

## Guard And Unrecorded Changes

Guard and reconciliation records are bounded observations. They do not prove
who changed a file, malicious intent, complete monitoring, or prevention.
Suppression may remove only exact owner-defined matching paths. An
`Unavailable` suppression outcome remains visible and cannot be treated as a
successful empty suppression.

## Explicit Non-Guarantees

Volicord does not guarantee:

- filesystem, process, shell, command, network, credential, or secret isolation;
- that Codex honors guidance, tool descriptions, or managed instructions;
- actor attribution from process, path, timing, prompt, or observation data;
- actor attribution from client name/version, host version, environment values,
  or local session metadata;
- complete detection or prevention of Product Repository changes;
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
