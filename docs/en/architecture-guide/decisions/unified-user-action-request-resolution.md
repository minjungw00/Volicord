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
request and reads the exact original agent-safe request result for the same
Agent Connection access scope; it creates no request, emits no authority event,
and does not resolve the action. The stored Agent Workflow result contains only
`AgentSafeUserActionRequestSummary`, never the full request, inbox item, or
capture form. After either branch, Core rereads the effective status and agent-
safe resolution in one SQLite read snapshot. The MCP result keeps that
projection's state version and observation time separate from the historical
request result and any later generic authority receipt.

User Channel presentation is a distinct projection. Native elicitation uses
its protocol-owned user input surface. A local-web bearer URL may be issued only
when a loopback listener is available and the initialized client declares exact
boolean `true` at
`params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`.
The server places that URL only in the namespaced top-
level tool-result `_meta` handoff promised to remain outside model context. A
missing, false, or malformed capability selects the CLI inbox without issuing a
token. No User Channel credential or credential-bearing URL may enter Agent
Connection `content`, `structuredContent`, compatibility or diagnostic text,
exact replay, or operation-result bytes.

The loopback browser submission adds an independent transport defense. Every
`POST /consent` must carry exactly one syntactically valid `Origin` whose
scheme and authority equal the listener's own origin. The server rejects a
missing, empty, `null`, malformed, comma-joined, repeated, or different origin
before reading the form body, looking up the token, or invoking Core. A
`GET /consent` does not require `Origin`, but a supplied origin must satisfy the
same exact validation. This gate is defense in depth against browser cross-
origin submission; it is neither user authentication nor a substitute for the
model-invisible credential boundary.

Request creation and resolution each use one canonical prepared-operation time
sample for status, expiry, and their semantic timestamps. A later Core commit
timestamp does not rewrite that sample. Local-web token validation uses the
half-open interval `created_at <= now < expires_at` from the same project clock.

The closed action kinds are the seven judgment kinds and
`evidence_observation`. Tagged payloads keep judgment action/outcome authority
separate from observation relevance. Core derives one capture form, basis,
effective status, candidate set, and expiry result. Separately verified User
Channel renderers—including native MCP elicitation, negotiated model-invisible
local web, and CLI inbox—render and submit that same form. Agent-visible prompt
capture receives only the safe pending summary and generic CLI guidance; it is
not a complete-form surface. An ordinary channel adapter supplies a bounded replay-bound
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
- An Agent Connection receives only the canonical request summary for a pending
  user action. The complete request, question, context, candidates, capture
  form, capture path, command, URL, and User Channel credential remain on User
  Channel projections.
- Listener existence is not a delivery capability. Local web is available only
  when the listener and negotiated model-invisible host surface are both
  available; capability omission or negotiation failure falls back to CLI
  without token issuance.
- Local-web browser submission requires the exact same-origin transport gate
  before request-body, token, replay, or resolution processing. Rejection has
  no token or Core state effect, and the same valid token can be retried from
  the correct origin.
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
- Cross-channel continuation replays the byte-exact original safe Agent
  Workflow result and marks it as replayed. It then attaches a separately
  observed, agent-safe current projection. It neither fabricates a new
  idempotency key nor exposes the complete User Channel form or exact user-only
  resolution response.
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

The safe request projection and model-invisible handoff are corrections inside
the same unreleased `0.8.0` clean-break batch, so they do not introduce another
SemVer or storage profile. No DDL changes. Every stored public-method result is
strictly checked against its current closed result type before replay or
operation-result paging. Stored pre-correction request results using the full-
form shape, close results containing `pending_user_action_inbox_items`, and any
result embedding the superseded `StateSummary` pending-action projection are
therefore ineligible. They are never rewritten or adapted.
Pre-correction local-web tokens omit the required
`delivery_surface=model_invisible_user_surface` creation marker and are
permanently unusable under corrected code: GET and POST fail closed without
rendering or effects. The row is never upgraded; the pending action remains
resolvable through another valid User Channel such as CLI. No compatibility
alias restores the unsafe projection.

Requiring same-origin `Origin` on browser `POST /consent` is also a correction
inside that unreleased batch. It changes no public method schema, DDL, or
storage profile and needs no separate SemVer change. Browser form submissions
already supply `Origin`; a non-browser caller that omitted it now receives the
transport-owned `403 ORIGIN_NOT_ALLOWED` response and must use a conforming
same-origin request or another User Channel. Rejection occurs before token
lookup, so it neither consumes nor invalidates an otherwise valid token.

The corrected Store and adapter fences apply only after the pre-correction
process has been replaced or restarted. A still-running old process can retain
an already-issued raw credential and is bounded only by that token's existing
TTL; operators must replace it before relying on the corrected fence.

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
- Removing only the bearer string from fallback text would leave full User
  Channel forms reachable through full-detail results, status/close
  projections, resume, and exact operation-result retrieval.
- Treating listener startup as proof of a safe host surface would issue an
  authority-bearing bearer credential to clients that can expose it to the
  model.
- Putting the URL in ordinary MCP content with instructions not to use it would
  preserve the authority bypass rather than enforce the channel boundary.
- Treating `Origin` as optional when absent was rejected because it makes the
  browser defense depend on attacker-controlled header omission.
- Treating the bearer token as sufficient browser-request protection was
  rejected because possession of the token and same-origin request provenance
  are separate defenses, and neither one replaces the model-invisible delivery
  invariant.

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
