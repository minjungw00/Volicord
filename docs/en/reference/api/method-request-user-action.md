<a id="volicordrequest_user_action"></a>

# `volicord.request_user_action` reference

This document owns the agent-workflow method that creates one pending
`UserActionRequest`. It is the only public request method for the seven
judgment kinds and `evidence_observation`.

## Request

### `RequestUserActionRequest` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `action` | yes | no | `UserActionDraft` |
| `change_unit_id` | yes | yes | `string` |
| `envelope` | yes | no | `ToolEnvelope` |
| `expires_at` | yes | yes | `UtcTimestamp` |
| `required_for` | yes | no | `UserActionRequiredFor[]` |
| `task_id` | yes | no | `string` |



`action` is the closed `UserActionDraft` union. Its discriminator is
`action_type=choice|evidence_observation`; the choice variant carries a nested
seven-value `judgment_kind`. The caller cannot separately supply the durable
eight-value `action_kind`. Choice payloads are owned by [API Judgment
Schemas](schema-judgment.md). The evidence-observation variant carries:

```schema
UserActionEvidenceObservationDraft:
  action_type: evidence_observation
  question: string
  context_summary: string
  target_candidates: EvidenceTarget[]
  artifact_candidate_ids: string[]
```

The method requires `operation_category=agent_workflow`, a current compatible
Task and Change Unit where required, a committed request idempotency key, and
the current `expected_state_version`. The agent proposes the visible candidates
but cannot submit a resolution, user actor, relevance assessment, selected
option, selected target, selected artifacts, or capture time.

`change_unit_id`, `required_for`, and `expires_at` are common request fields.
`expires_at` must be null for an evidence-observation request; Core assigns its
15-minute expiry. For an observation request, Core validates current target identity and exact
persistent artifact bytes, canonicalizes candidate refs, captures Task, Change
Unit, scope revision, baseline, and state version, and sets a 15-minute expiry.
For a judgment request, Core derives the judgment basis and Core-owned authority
options as required by the judgment owner.

### Operation time

After common preflight, Core samples the project's canonical Core UTC clock
exactly once for this prepared operation. The resulting `operation_now` is used
for every current-time decision, the public request `created_at`, the stored
`requested_at`, explicit choice-expiry validation, and the derived 15-minute
evidence-observation expiry. A host timestamp or caller clock is not an input to
these decisions.

A non-null caller-supplied choice `expires_at` must normalize to an instant
that has a canonical four-digit RFC 3339 UTC representation and must be later
than `operation_now`. Core applies this same validation before either dry-run
planning or commit planning. An unrepresentable explicit expiry rejects with no
request, event, replay row, state-version change, or persisted clock-floor
update.

The 15-minute derivation and every other owner TTL use checked timestamp
addition and require a canonical RFC 3339 UTC result. Overflow or an
unrepresentable result rejects before commit and leaves no request, event,
replay row, state-version change, or persisted clock-floor update.

The eventual Core transaction selects a canonical commit timestamp that is no
earlier than `operation_now`. It may be later, but it does not rewrite the
request's semantic creation/request time. Commit-time storage metadata follows
[Storage Versioning](../storage-versioning.md#canonical-core-utc-clock).

### MCP create and resume operations

The MCP-visible adapter arguments wrap creation and continuation in one strict
nested operation union; they do not change the create-only Core request above:

```schema
McpRequestUserActionArguments:
  project_selector: string | null
  detail: summary | workflow | full
  request:
    # create variant
    operation: create
    task_id: string
    change_unit_id: string | null
    action: UserActionDraft
    required_for: string[]
    expires_at: string | null

    # resume variant
    operation: resume
    user_action_request_id: string
```

Exactly one variant is accepted. Missing or unknown operations, flat create
fields, and mixed create/resume fields reject before Core. The create variant
constructs the complete `RequestUserActionRequest` and requires writable
project state.

The resume variant is read-only continuation, not a second public mutation. It
addresses only a request created directly by `volicord.request_user_action`,
requires the same enabled workflow Agent Connection actor scope and an allowed
project, and replays the byte-exact original agent-safe Agent Workflow response
with the same `operation_result_ref`. The replayed result contains only the
canonical request summary; it never contains the full request, CLI inbox item,
resolution form, capture path, or User Channel credential. It creates no request,
event, replay row, prompt, token, resolution, or state-version increment. A
request created by another connection or by `volicord.reconcile_changes` is
unavailable through this branch. An unrelated later Git or authority-state
change does not rewrite or invalidate the historical result. A stored response with a duplicate JSON object member at any nesting level, a
non-result branch, a commit-coordinate mismatch, or any other current closed
result-contract violation is corrupt instead of being replayed. Resume applies
the same raw committed-result gate as direct replay and returns
`PERSISTED_DATA_CORRUPT` without any stored bytes when that gate fails.

After create or resume, the adapter asks Core for a separate current,
agent-safe projection. Core reads its status, optional safe resolution, exact
historical resolution-derived refs, and observation anchors from one SQLite
read snapshot. The adapter returns that projection without opening another
input surface. A pending action is delivered and resolved only through
`volicord inbox`. The exact result is marked
`agent_workflow_result_replayed=true` only for resume.
Its `current_projection_observed_at` is the canonical Core-time sample for that
read snapshot and is not persisted merely because the projection was read.

## Result and effects

```schema
RequestUserActionResult:
  base: ToolResultBase
  user_action_request_summary: AgentSafeUserActionRequestSummary
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

A committed call inserts one `user_action_requests` row, appends one authority
event, stores the exact replay result, increments `state_version` once, and may
move the current non-terminal Task to `waiting_user` when a current effective
pending request has a non-informational `required_for`. Idempotent replay
returns the original agent-safe request summary without recanonicalizing the
stored request. The summary contains only the request ID, historical `pending`
status, and `next_actor=user`. It omits refs, action kind, expiry, question,
context, body, basis, candidates, form, channel paths, commands, URLs, and credentials.
It does not update the persisted canonical-UTC floor.

Dry run returns no durable ref and has no effect. Invalid candidates, oversized
forms, stale state, wrong operation category, incompatible basis, unavailable
artifact bytes, and unsupported tagged payloads reject before commit.
Neither dry run nor rejection updates the persisted canonical-UTC floor.

MCP exposes create to a writable `workflow` Agent Connection. A workflow
connection whose project storage has degraded to readable-only may still
discover and use resume, while create rejects before Core mutation. A
`read_only` Agent Connection cannot use either branch. The CLI presentation may
render the adapter-neutral resolution form through a supported User Channel
only for a newly created request that is still pending; the Agent Connection
call itself never receives that form or resolves the action. Resume returns only the exact safe
replay and current safe projection and never opens a User Channel.

## Related owners

- Common shapes and limits: [API User Action Schemas](schema-user-action.md).
- Judgment payloads: [API Judgment Schemas](schema-judgment.md).
- Resolution: [`volicord.resolve_user_action`](method-resolve-user-action.md).
- Effects: [Storage Effects](../storage-effects.md#volicordrequest_user_action).
- Clock persistence: [Storage Versioning](../storage-versioning.md#canonical-core-utc-clock).
