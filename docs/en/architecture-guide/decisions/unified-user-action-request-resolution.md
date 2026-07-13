# Unified user-action request and resolution

## Context

Volicord previously stored pending judgments in `user_judgments`, accepted
judgment answers through judgment-specific host and CLI paths, and recorded
user evidence observations immediately through a separate CLI-only method and
`user_evidence_observations`. Final acceptance and residual-risk acceptance
were judgment kinds, but user observation had no pending request, host-native
capture form, or shared channel lifecycle.

## Decision

Use one Core-owned `UserActionRequest` lifecycle and one immutable one-to-one
`UserActionResolution` for every supported user action. The public methods are
`volicord.request_user_action` for `agent_workflow` creation and
`volicord.resolve_user_action` for `user_only` resolution. Only the request
method is an MCP tool.

The MCP tool has an explicit nested `request.operation=create|resume` union.
`create` runs the public mutation once. `resume` names an existing direct
request and reads the exact original request result for the same Agent
Connection access scope; it creates no request, emits no authority event, and
does not resolve the action. After either branch, Core rereads the effective
status and agent-safe resolution in one SQLite read snapshot. The MCP result
keeps that projection's state version and observation time separate from the
historical request result and any later generic authority receipt.

Request creation and resolution each use one canonical prepared-operation time
sample for status, expiry, and their semantic timestamps. A later Core commit
timestamp does not rewrite that sample. Local-web token validation uses the
half-open interval `created_at <= now < expires_at` from the same project clock.

The closed action kinds are the seven judgment kinds and
`evidence_observation`. Tagged payloads keep judgment action/outcome authority
separate from observation relevance. Core derives one capture form, basis,
effective status, candidate set, and expiry result. MCP elicitation, prompt
capture, local web consent, and CLI inbox adapters render and submit that same
form. An ordinary channel adapter supplies a bounded replay-bound
`channel_submission_id`; it does not recompute candidates or user authority.
For local-web consent, Core owns the digest-only identity derivation and exact
revalidation over the project, request, bearer-token credential, expected
connection, and closed canonical completion metadata. Its mutation replay
identity uses a domain-separated token digest with the same connection and
metadata, never the raw token.

Durable storage uses `user_action_requests`, immutable one-to-one
`user_action_resolutions` with a closed tagged evidence-observation body, and
request-bound local channel tokens. The request row also stores the exact
originating method and idempotency key. A partial uniqueness rule gives a
direct request exactly one origin while still allowing one reconciliation
commit to create several requests. The
old `user_judgments`, `user_evidence_observations`, and judgment-bound token
shape are removed.

## Invariants and consequences

- An Agent Connection can create a request but cannot resolve it.
- The user selects only stored candidates through a verified `User Channel`.
- Resolution kind, request kind, basis, Task, Change Unit, scope, baseline,
  target, artifact bytes, expiry, and channel binding are revalidated before one
  atomic commit.
- One request has at most one immutable resolution. Concurrent or conflicting
  replay cannot fork it.
- A local-web duplicate returns the original safe completion only when its
  token credential, project/request coordinates, expected connection, closed
  completion context, submission identity, and canonical resolution all match.
  A hand-crafted identity or changed binding cannot open that replay.
- `resolved` means a resolution exists, not that a judgment was accepted or an
  observation supports a claim. Kind-specific payloads retain those meanings.
- Observation capture time is Core time. Agents and channel forms do not submit
  it.
- Request and resolution planners each reuse one canonical Core-time sample;
  adapters and host timestamps do not supply temporal authority.
- Judgment capture submits only a stored option ID and optional note. Core
  derives structured authority facts from stored state and does not fabricate
  user rationale.
- Resolution adapters do not guess a request-time state version. Core pins
  current state at preflight and the transaction detects a preflight-to-commit
  race.
- A user evidence resolution is still a producer/relevance record only; a later
  `record_run` must reference it before evidence coverage can use it.
- Free-form user text remains in the user-only result. Agent-safe MCP
  projections expose only structured selected outcomes and refs.
- Cross-channel continuation replays the byte-exact original Agent Workflow
  result and marks it as replayed. It then attaches a separately observed,
  agent-safe current projection. It neither fabricates a new idempotency key nor
  exposes the exact user-only resolution response.
- Current status, safe resolution, and historical resolution-derived refs are
  read from one Core/Store snapshot. Their observed state version and time make
  freshness explicit if a newer authority receipt is produced later.

## Compatibility and migration

This is an intentional pre-major clean break. The three old public methods and
their request/response schemas, MCP names, CLI `inbox answer` and direct-observe
forms, record kinds, tables, and aliases are not retained. The public contract
batch is version `0.8.0`. The nested MCP create/resume union replaces the
earlier flat create-only arguments in the same clean-break batch; no ambiguous
flat compatibility decoder is retained. Residual-risk coverage now names its exact authority
refs `accepted_by_user_action_resolution_refs`.

The storage profile is `baseline_sqlite_v5`. There is no v4-to-v5 conversion or
legacy read path. A v4 Runtime Home is incompatible and must be recreated.

## Rejected alternatives

- Adding only `request_user_observation` would preserve separate judgment and
  observation lifecycles and repeat the root cause.
- Keeping the old methods as wrappers would create competing stable paths and
  legacy compatibility code before the first major release.
- Letting adapters construct forms or infer user answers would split authority
  from Core and permit projection drift.
- Storing observation resolutions only as untyped JSON would lose target
  relationship checks needed by evidence reuse.
- Treating a later retry as a new create call would depend on a newly generated
  adapter idempotency key and could duplicate the user request.
- Deterministically deriving a replacement idempotency key from public request
  content would conflate distinct intentional requests and would still not
  identify the exact historical result after another channel resolved it.
- Keeping flat create fields beside an optional resume ID would admit mixed or
  ambiguous requests instead of a closed operation union.

## Implementation and tests

The shared shapes belong in `volicord-types`; Core owns status, basis, and
capture-form evaluation; Store owns the atomic request-resolution-token
transaction; MCP and CLI remain adapters. Durable tests cover strict tagged
decoding, every action kind, candidate and size bounds, actor denial,
stale/expiry boundaries, idempotent channel submission, concurrent resolution,
rollback, channel fallback equivalence, private-text redaction, and the later
`record_run` evidence path.

Exact behavior remains in [Core Model](../../reference/core-model.md), [API User
Action Schemas](../../reference/api/schema-user-action.md), [Request-user-action
method](../../reference/api/method-request-user-action.md), [Resolve-user-action
method](../../reference/api/method-resolve-user-action.md), and [Storage
Versioning](../../reference/storage-versioning.md).
The clock rationale is recorded in
[Canonical Core UTC clock](canonical-core-utc-clock.md).
