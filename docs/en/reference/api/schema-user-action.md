# API user-action schemas

This document owns the common public schema for a user action requested by an
Agent Connection and resolved only through the `User Channel`. Choice judgments
and evidence observations share one request, basis, effective-status,
adapter-neutral resolution-form, and immutable-resolution envelope while
retaining distinct authority meanings.

## Closed action families

The request-side `UserActionDraft` is a closed tagged union whose discriminator
is `action_type`:

```schema
UserActionDraft:
  # choice variant
  action_type: choice
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserActionOptionInput[] | null
  context: UserActionContext
  affected_refs: StateRecordRef[]
  sensitive_action_scope: SensitiveActionScope | null

  # evidence-observation variant
  action_type: evidence_observation
  question: string
  context_summary: string
  target_candidates: EvidenceTarget[]
  artifact_candidate_ids: string[]
```

Exactly one variant is present. Unknown fields, unknown discriminators, mixed
choice/observation fields, and missing variant fields reject before commit.
The caller does not submit `action_kind`: Core derives its eight-value durable
form from `action_type` plus `judgment_kind`.

`UserActionRequestBody` is the corresponding stored union with the same
`action_type`. Its choice variant replaces caller options with current
Core-owned `UserActionOption[]`. Its observation variant replaces artifact IDs
with exact canonical `artifact_candidates: ArtifactRef[]`. Judgment-specific
option, context, and sensitive-scope shapes are owned by [API Judgment
Schemas](schema-judgment.md).

## Durable request and basis

```schema
UserActionRequest:
  user_action_request_id: string
  project_id: string
  task_id: string
  change_unit_id: string | null
  action_kind: string
  status: string
  body: UserActionRequestBody
  basis: UserActionBasis
  required_for: string[]
  user_action_resolution_ref: StateRecordRef | null
  expires_at: string | null
  created_at: string

UserActionBasisCoordinates:
  task_id: string
  change_unit_id: string | null
  scope_revision: integer
  baseline_ref: string | null
  created_at_state_version: integer
  compatibility_status: current | stale | superseded

UserActionBasis:
  # choice variant
  action_type: choice
  coordinates: UserActionBasisCoordinates
  close_basis_revision: integer | null
  result_refs: StateRecordRef[]
  residual_risk_ids: string[]
  sensitive_action_scope: SensitiveActionScope | null

  # evidence-observation variant
  action_type: evidence_observation
  coordinates: UserActionBasisCoordinates
  target_candidates: EvidenceTarget[]
  artifact_candidates: ArtifactRef[]
```

Core derives the durable `action_kind`, basis, exact artifact refs, and
compatibility. Callers cannot submit revisions, baseline, canonical refs,
compatibility, actor provenance, or capture time.

For a choice request, every `affected_refs` entry belongs to the request
project. A Task-scoped entry belongs to the request Task. Core validates this
before canonicalization because `affected_refs` participates in operation
relevance and blocker overlap. `context.related_refs` remains display and audit
context; it is not substituted for `affected_refs` and does not participate in
operation-blocker overlap.

`required_for` is non-empty, contains no duplicates, and must be compatible
with the action kind. It is stored unchanged and participates in operation
relevance; neither Store nor an adapter may silently add or discard entries.

The closed compatibility matrix is:

| Action kind | Compatible `required_for` values |
|---|---|
| `product_decision`, `technical_decision` | `scope_update`, `advance_task`, `prepare_write`, `record_run`, `close_complete`, `close_supersede`, `informational` |
| `scope_decision` | `scope_update`, `advance_task`, `prepare_write`, `record_run`, `close_complete`, `close_supersede`, `informational` |
| `sensitive_approval` | `advance_task`, `prepare_write`, `record_run`, `close_complete`, `close_supersede`, `informational` |
| `final_acceptance`, `residual_risk_acceptance` | `close_complete`, `informational` |
| `cancellation` | `close_cancel`, `informational` |
| `evidence_observation` | `record_run`, `close_complete`, `informational` |

The request validator and operation-blocker projection use this one matrix.
`informational` never keeps the Task waiting or blocks an operation by itself.

The generic compatibility matrix does not choose shaping application owners.
A request created by `volicord.record_shaping` uses the exact per-gap policy:
product and technical use `[advance_task]`; scope uses `[scope_update]`; and
sensitive approval uses `[advance_task, prepare_write, record_run,
close_complete]`. Its immutable resolution changes the linked gap to
the exact `accepted`, `rejected`, or `deferred` disposition. The semantic owner
method may change only `accepted` to `applied`.

The effective statuses are `pending`, `resolved`, `stale`, `superseded`, and
`expired`. One Core evaluator derives status from current basis compatibility,
immutable resolution presence, expiry, and current Core time. A stale or
superseded basis takes precedence over a stored resolution; a current request
with a resolution is `resolved`; only an otherwise-pending current request can
be `expired`. Expiry uses `created_at <= now < expires_at`, so
`now >= expires_at` cannot resolve. Reads do not mutate state to report expiry.
For a shaping-linked request, this effective lifecycle status is an input to
the separate `ShapingDecisionAuthorityState`; `resolved` therefore records a
terminal answer but does not by itself grant shaping authority.

Choice requests preserve the explicit nullable caller `expires_at`; `null`
means no time deadline while basis invalidation still applies.
Evidence-observation requests do not accept a caller deadline; Core assigns a
15-minute expiry.

### Canonical time sampling

Every `now` in the UserAction lifecycle is a sample of the project-scoped
canonical Core UTC clock, not the host or caller clock. After common preflight,
each request or resolution operation samples `operation_now` exactly once and
reuses it for all status, basis, expiry, and timestamp decisions in that
operation.

- Request creation exposes `UserActionRequest.created_at` and stores
  `user_action_requests.requested_at` as that one sample. Evidence-observation
  expiry is exactly 15 minutes after the same sample; explicit choice expiry is
  validated against it and must itself normalize to an instant representable as
  canonical four-digit RFC 3339 UTC. Dry run and commit apply the same explicit-
  expiry validation. Every derived expiry uses checked addition and the same
  representability rule; an invalid explicit value or derivation overflow
  rejects with no effect.
- Resolution derives effective status, validates the request and channel, and
  records `UserActionResolution.resolved_at` from one resolution-operation
  sample.
- `current_projection_observed_at` is the canonical Core-time sample for the
  one read snapshot named by the projection. Observing it does not by itself
  persist a later project-time floor.

The Core commit timestamp may be later than `operation_now`, but it must not
rewrite these owner-defined semantic timestamps. The physical floor and commit
rules belong to [Storage Versioning](../storage-versioning.md#canonical-core-utc-clock).

## Resolution input and immutable body

`UserActionResolutionInput` is a separate closed union whose discriminator is
`resolution_type`:

```schema
UserActionResolutionInput:
  # choice variant
  resolution_type: choice
  selected_option_id: string
  note: string | null

  # evidence-observation variant
  resolution_type: evidence_observation
  target: EvidenceTarget
  artifact_ids: string[]
  relevance_status: supported | contradicted
  summary: string

UserActionResolutionBody:
  # choice variant
  resolution_type: choice
  selected_option_id: string
  machine_action: accept | reject | defer
  resolution_outcome: accepted | rejected | deferred
  note: string | null
  accepted_risk_ids: string[]

  # evidence-observation variant
  resolution_type: evidence_observation
  observation: UserActionEvidenceObservation

UserActionEvidenceObservation:
  target: EvidenceTarget
  output_artifact_refs: ArtifactRef[]
  relevance_status: supported | contradicted
  summary: string
```

The choice user submits only a stored option ID and optional note. Core copies
the machine action and outcome from that stored option and derives current
accepted residual-risk IDs from the stored request and compatible basis. The
caller cannot submit `judgment_kind`, `action_kind`, machine action, outcome,
risk objects, an answer branch, sensitive scope, or rationale. Core does not
fabricate an uncaptured rationale or synthetic user answer.

For an evidence observation, the user chooses one stored target candidate and
a non-empty unique subset of stored artifact candidates. Core canonicalizes the
selection and validates each selected artifact candidate against current
authoritative artifact identity and freshness. After that validation, the
immutable resolution preserves the exact `ArtifactRef` values from the stored
request candidates, including nested `created_by_run_ref` versions; it does not
reconstruct or upgrade them from a later current projection. The request
`UserActionBasis` solely owns Task, Change Unit, scope, and
baseline coordinates. Resolution identity, project/Task, channel, actor
provenance, assurance, verification basis, and capture time occur only on the
enclosing `UserActionResolution`. The nested observation contains only the
selected target, artifact refs, relevance, and summary; it does not duplicate
coordinates or create an orphan observation identity.

The semantic UserAction resolution identity is the typed tuple
(`project_id`, `task_id`, `user_action_resolution_id`). Authority consumers
obtain that full identity from the resolved UserAction facts instead of
reconstructing it from an unscoped resolution ID and ambient owner state.
The Write Ticket approval owner applies the current sensitive-approval
semantics to those authority facts and exposes a typed assessment to ticket
consumers; summary, reuse, admission, and close readiness do not rebuild a
resolution-ID set.

```schema
UserActionResolution:
  user_action_resolution_id: string
  user_action_request_id: string
  project_id: string
  task_id: string
  action_kind: string
  body: UserActionResolutionBody
  resolved_by_actor_source: local_user
  resolved_verification_basis: cli_direct_user_channel
  resolved_assurance_level: string
  channel_kind: cli
  channel_submission_id: string
  resolved_at: string
```

The resolution `action_kind` is copied from the request; it is not resolution
input. `resolved` means only that one immutable User Channel resolution exists.
Choice acceptance and evidence relevance never substitute for one another.

## Bounds

Choice options, observation target candidates, and observation artifact
candidates are each limited to 32 entries. Product and technical choices require
non-empty caller options with unique IDs and at most one default; authority-bearing
choice kinds reject caller options and use Core-owned options. Observation
candidate lists and the selected artifact list must be non-empty and unique.
Questions and context summaries must not be blank. User note-like text is limited to
1,000 Unicode scalar values, observation `summary` to 4,000 Unicode scalar
values, and a canonical serialized action or adapter-neutral resolution form to
32 KiB. Core checks the bounds before request commit and resolution commit.
Adapters check again before rendering or accepting a form and never truncate
into validity.

`ResolveUserActionRequest.channel_submission_id` and the value preserved on
`UserActionResolution` are 1 through 256 bytes of visible ASCII
`0x21..=0x7e`. Empty values, whitespace, NUL, non-ASCII, and longer values are
invalid. The public JSON Schema expresses the matching non-empty, maximum-
length, visible-ASCII shape, and Core validates the exact byte bound before
replay lookup or commit.

<a id="resolution-form"></a>
## Adapter-neutral resolution form

```schema
AgentSafeUserActionRequestSummary:
  user_action_request_id: string
  status: pending
  next_actor: user

UserActionResolutionForm:
  # choice variant
  form_type: choice
  choices: UserActionResolutionChoice[]
  note_allowed: boolean
  note_max_chars: integer

  # evidence-observation variant
  form_type: evidence_observation
  target_candidates: EvidenceTarget[]
  artifact_candidates: ArtifactRef[]
  relevance_options: [supported, contradicted]
  summary_max_chars: integer

UserActionResolutionChoice:
  choice_id: string
  label: string
  description: string
  consequence: string
  is_default: boolean
```

`AgentSafeUserActionRequestSummary` is the only pending-request projection
allowed in an Agent Connection result. It identifies only the request, its
historical pending status, and the user as next actor. It does not carry the
request ref, action kind, expiry, request body, basis, question, context,
candidates, resolution form, capture path, command, URL, or any User Channel
credential. Current non-pending status belongs to the separately refreshed
current projection rather than this historical pending summary.

This is a closed object with exactly three required fields and no unknown or
additional fields. `user_action_request_id` must satisfy its non-empty bounded
identifier contract, while `status` and `next_actor` are the literal values
`pending` and `user`. A missing, additional, wrong-typed, or wrong-literal field
is invalid in ordinary output, replay, resume, and operation-result eligibility.

`UserActionResolutionForm` is a closed semantic projection derived by
`UserActionRequestBody.resolution_form()` from the stored request body. It
copies only the exact selectable choices or evidence candidates, the closed
relevance values, and canonical input limits. It carries no channel
availability, CLI label, command, terminal or Markdown layout, protocol field,
credential, or adapter status. Adapters must not reconstruct candidates from
arguments, prose, or adapter-local state.

The exact CLI inbox document, channel availability, capture path, and CLI JSON
schemas are owned by [Administrative CLI](../admin-cli.md#user-channel-commands).
MCP maps neutral current facts into its own safe protocol projection. It may
create or resume the request, but it cannot receive or submit the resolution
form.


The exact MCP compound response, compact projection, safe resolution, and
wire serialization are owned by
[MCP transport](../mcp-transport.md#user-action-wire-projection). This public
schema owner supplies only the adapter-neutral request, resolution, reference,
and current-state facts consumed by that projection.

## Related owners

- [Request-user-action method](method-request-user-action.md).
- [Resolve-user-action method](method-resolve-user-action.md).
- [API Judgment Schemas](schema-judgment.md).
- [API State Schemas](schema-state.md) for evidence targets and refs.
- [Core Model](../core-model.md) for authority and non-substitution meaning.
- [Storage Versioning](../storage-versioning.md#canonical-core-utc-clock) for
  the canonical clock, persisted floor, and commit timestamp.
