<a id="volicordresolve_user_action"></a>

# `volicord.resolve_user_action` reference

This document owns the only public transition that resolves a pending
`UserActionRequest`. It is a direct `User Channel` method and is never an Agent
Connection MCP tool.

## Request

```yaml
ResolveUserActionRequest:
  envelope: ToolEnvelope
  user_action_request_id: string
  resolution: UserActionResolutionInput
  channel_submission_id: string
```

`resolution` is a closed tagged union with
`resolution_type=choice|evidence_observation` and must match the stored request
family.
Judgment resolution input contains only the selected stored option ID and an
optional user note. Core copies machine action and outcome from the stored
option and derives current accepted residual-risk IDs from the stored request
and compatible basis. `judgment_kind`, durable `action_kind`, sensitive scope,
and other authority coordinates remain request-owned. The caller cannot submit
rationale, risk objects or IDs, an answer branch, machine action, or outcome,
and Core does not fabricate user rationale or a synthetic answer.
Evidence-observation input contains the
selected stored target, selected candidate artifact IDs, `supported` or
`contradicted`, and the user's summary. It does not contain `observed_at`.

The invocation must be server-derived as `actor_source=local_user` and
`operation_category=user_only` with a recognized User Channel verification
basis. The request must be effectively `pending`, unexpired, and current for
its stored basis. Core re-reads the canonical capture form, candidate set,
current target, exact artifact bytes, Task/Change Unit, scope, baseline, and
close basis as applicable before commit.

The channel adapter sets `ToolEnvelope.expected_state_version` to explicit
`null`; omission is invalid for the required-nullable envelope field. A host or
user does not guess the state version captured
when the request was created. Core pins the current state version during
resolution preflight. If state changes before commit, the transaction returns
`STATE_VERSION_CONFLICT`; semantic freshness still comes from the basis
evaluator rather than an adapter-supplied version.

`channel_submission_id` is an opaque channel identity of 1 through 256 bytes.
Every byte must be visible ASCII `0x21..=0x7e`; whitespace, NUL, non-ASCII, an
empty value, and a longer value reject. The envelope `idempotency_key` must
exactly equal it. For ordinary User Channels the channel adapter generates the
identity. Replay under the same request, channel, actor context, submission id,
and canonical resolution returns the original committed response. Reuse with a
different resolution rejects. Concurrent distinct submissions cannot create a
second resolution.

Local-web consent is stricter. It can enter Core only through the token-bearing
local-web boundary. Core derives and then independently revalidates the one
accepted digest-only `local_web:<sha256>` submission identity over the exact
project, user-action request, raw bearer-token credential, expected Agent
Connection, and typed canonical completion metadata. The mutation replay
identity separately binds a domain-separated digest of that token, the expected
connection, and the same closed metadata. A hand-crafted identity or a changed
token, connection, or completion context cannot open the stored replay. The raw
token is transient validation input: it is absent from the public request,
resolution row, replay row, and response.

### Operation time

After common preflight, Core samples the project's canonical Core UTC clock
exactly once for the resolution operation. That `operation_now` is reused to
derive effective request status, check request expiry, validate any local-web
token, and set the public and stored `resolved_at`. A local-web token is valid
only when `token.created_at <= operation_now < token.expires_at`; a value before
creation is invalid and a value at expiry is expired.

The Core transaction may choose a later canonical commit timestamp, but it must
not replace the semantic `resolved_at` sample. The transaction timestamp and
persisted-floor rules belong to
[Storage Versioning](../storage-versioning.md#canonical-core-utc-clock).

## Result and effects

```yaml
ResolveUserActionResult:
  base: ToolResultBase
  user_action_request_ref: StateRecordRef
  user_action_resolution_ref: StateRecordRef
  user_action_request: UserActionRequest
  user_action_resolution: UserActionResolution
  derived_refs: StateRecordRef[]
  state: StateSummary
  next_actions: NextActionSummary[]
```

Commit atomically inserts one immutable `user_action_resolutions` row whose
closed `resolution_json` contains the applicable choice or evidence-observation
body, causing the common effective-status evaluator to return `resolved`. It consumes the matching channel token when present, updates dependent
blockers and Task lifecycle, appends one authority event, stores the user-only
replay result, and increments `state_version` once. A judgment resolution may
create owner-selected continuity records. An evidence-observation resolution
does not update evidence coverage or create a Run; a later `record_run` must
reference the exact `user_action_resolution_ref` and selected artifacts.
Every user-action-derived continuity record stores `rationale=null`; the
optional private user note remains only in the immutable resolution body and
must not be copied into continuity, agent-safe, status, export-derived, or
diagnostic projections.

Core supplies one resolution capture time on the enclosing resolution. The
nested evidence-observation body has no duplicate identity or timestamp.
`status=resolved` is not acceptance by itself. Judgment authority comes only
from the stored option action/outcome and current basis. Observation authority
comes only from the exact current `evidence_observation` resolution body and is
never final acceptance or another judgment.

Dry run, malformed or mixed payloads, an Agent Connection actor, stale or
superseded basis, `now >= expires_at`, changed candidates or bytes, replay
conflict, and wrong channel binding reject with no request, resolution, token
consumption, event, replay, blocker, lifecycle, or state-version effect. A
deadline check may persist the already-derived `expired` token status; it never
turns a rejected attempt into consumption. Exact replay, dry run, rejection,
and an expiry-status update do not update the persisted canonical-UTC floor.

## Related owners

- Common shapes and effective status: [API User Action Schemas](schema-user-action.md).
- Authority meaning: [Core Model](../core-model.md).
- Effects: [Storage Effects](../storage-effects.md#volicordresolve_user_action).
- Channel behavior: [MCP Transport](../mcp-transport.md) and [Administrative CLI](../admin-cli.md).
- Clock persistence: [Storage Versioning](../storage-versioning.md#canonical-core-utc-clock).
