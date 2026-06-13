# Template bodies

This document owns display-facing wording for current rendered template bodies: status cards, public error messages, judgment requests, run/evidence summaries, close results, and agent context packets. It owns rendered body guidance, user-facing labels, and display phrasing only; authority, storage records, API error semantics, and close-readiness blocker semantics stay with the linked owner records.

## Owns / Does not own

This document owns:

- rendered template body guidance for current status and support displays
- user-facing label wording and display phrasing for those bodies
- locale-aware rendered labels where a body needs them
- user-facing public-error display labels and recovery cues as display text
- links from body placeholders to schema and authority owners

This document does not own:

- projection authority, freshness, or read-only derived-display rules; see [Projection Authority Reference](projection-and-templates.md)
- source-of-truth state, storage record authority, or storage record layout; see [Core Model](core-model.md) and storage owners
- API schemas, value sets, public `ErrorCode` identifiers, or public `ErrorCode` semantics; see API schema owners and [API error codes](api/error-codes.md)
- error precedence, rejected-response behavior, response branch routing, or machine-readable `ToolError.details`; see [API error precedence](api/error-precedence.md), [API error routing](api/error-routing.md), and [API error details](api/error-details.md)
- close-readiness blocker semantics, blocker-code routing, or `CloseReadinessBlocker` shape; see [Core Model](core-model.md), [API State Schemas](api/schema-state.md), and [API blocker routing](api/blocker-routing.md)
- display packages outside the current bodies listed above; see [Scope Reference](scope.md) for support boundaries

## Boundary

Template text is display text. It can summarize owner records, but it must route authority questions back to those records.

Public `ErrorCode` values may appear as input conditions for label selection, but those identifiers and their meanings remain API-owned by [API error codes](api/error-codes.md). Error precedence, response branch routing, blocker mappings, and machine-readable details remain with their API owners.

Template output cannot by wording alone:

- authorize writes
- create evidence or persistent artifacts
- satisfy evidence, QA, verification, acceptance, or close gates
- create final acceptance or accept residual risk
- close a Task or create close readiness
- mutate owner records
- define storage record layout or make a rendered body the storage authority
- define, rename, localize, or change the semantics of public `ErrorCode` identifiers or machine-readable detail keys
- define close-readiness blocker semantics, blocker codes, or blocker routing
- convert rejected-response errors into blockers or blocked results

## Public error display labels

Use this section when rendering public API errors for a user or agent-facing surface. The public `ErrorCode` stays unchanged, and its meaning stays with the API owner. A label or recovery cue is display text only.

Rendered error copy must:

- Preserve the public `ErrorCode` when the exact diagnostic identifier is shown.
- Pair a concise label with one recovery cue when the surface has room.
- Keep labels separate from `CloseReadinessBlocker.code`, `WriteDecisionReason.code`, `PlannedBlocker.code`, and `ToolError.details` keys.
- Route public code meanings to [API error codes](api/error-codes.md), precedence or conflict selection to [API error precedence](api/error-precedence.md), response branch routing to [API error routing](api/error-routing.md), close-readiness blocker routing to [API blocker routing](api/blocker-routing.md), and machine-readable details to [API error details](api/error-details.md). Use [API errors](api/errors.md) only as the family index.

Rendered error copy must not:

- Replace a public `ErrorCode` with a localized label.
- Define or change public `ErrorCode` semantics.
- Reuse a label as a machine-readable code.
- Hide close blockers or turn rejected responses into blocked results.

<a id="label-validation-failed"></a>
### `VALIDATION_FAILED`

Label-selection input:
- `VALIDATION_FAILED`.

Suggested label:
- invalid request

Recovery cue:
- Fix the payload, enum value, activation rule, profile value, or field set before retrying.

<a id="label-state-version-conflict"></a>
### `STATE_VERSION_CONFLICT`

Label-selection input:
- `STATE_VERSION_CONFLICT`.

Suggested label:
- state version conflict

Recovery cue:
- Refresh current state and retry with the current `project_state.state_version`, or replay the original idempotent request.

<a id="label-mcp-unavailable"></a>
### `MCP_UNAVAILABLE`

Label-selection input:
- `MCP_UNAVAILABLE`.

Suggested label:
- Core or surface unavailable

Recovery cue:
- Reconnect Core, MCP, or the selected surface, or show that the route is unavailable.

<a id="label-local-access-mismatch"></a>
### `LOCAL_ACCESS_MISMATCH`

Label-selection input:
- `LOCAL_ACCESS_MISMATCH`.

Suggested label:
- local access mismatch

Recovery cue:
- Use the registered local transport, session, or binding.
- Repair local access registration when needed.

<a id="label-capability-insufficient"></a>
### `CAPABILITY_INSUFFICIENT`

Label-selection input:
- `CAPABILITY_INSUFFICIENT`.

Suggested label:
- insufficient surface capability

Recovery cue:
- Use a capable surface.
- Reduce the operation or avoid the missing capability.

<a id="label-no-active-task"></a>
### `NO_ACTIVE_TASK`

Label-selection input:
- `NO_ACTIVE_TASK`.

Suggested label:
- no active Task

Recovery cue:
- Select or create a Task before a Task-scoped action.

<a id="label-scope-boundary-baseline"></a>
### Scope, boundary, or baseline

Label-selection input:
- `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, or `BASELINE_STALE`.

Suggested label:
- scope, boundary, or baseline issue

Recovery cue:
- Confirm or narrow scope.
- Use the appropriate scope or baseline owner-defined action.
- Request the needed user judgment.

<a id="label-write-authorization"></a>
### Write Authorization

Label-selection input:
- `WRITE_AUTHORIZATION_REQUIRED` or `WRITE_AUTHORIZATION_INVALID`.

Suggested label:
- missing or unusable pre-write check

Recovery cue:
- Call or retry `harness.prepare_write` for the exact operation, current scope, and current state.

<a id="label-judgment"></a>
### Judgment

Label-selection input:
- `DECISION_REQUIRED` or `DECISION_UNRESOLVED`.

Suggested label:
- judgment needed

Recovery cue:
- Request or resolve the focused `UserJudgment`.

<a id="label-sensitive-approval"></a>
### Sensitive-action approval

Label-selection input:
- `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, or `APPROVAL_EXPIRED`.

Suggested label:
- sensitive-action approval needed or not usable

Recovery cue:
- Request, resolve, or renew `judgment_kind=sensitive_approval`.

<a id="label-evidence-insufficient"></a>
### `EVIDENCE_INSUFFICIENT`

Label-selection input:
- `EVIDENCE_INSUFFICIENT`.

Suggested label:
- evidence needed

Recovery cue:
- Record, rerun, or show the missing evidence, then display the smallest next action needed.

<a id="label-acceptance-required"></a>
### `ACCEPTANCE_REQUIRED`

Label-selection input:
- `ACCEPTANCE_REQUIRED`.

Suggested label:
- final acceptance needed

Recovery cue:
- Request or resolve `judgment_kind=final_acceptance` for the visible result basis.

<a id="label-residual-risk-not-visible"></a>
### `RESIDUAL_RISK_NOT_VISIBLE`

Label-selection input:
- `RESIDUAL_RISK_NOT_VISIBLE`.

Suggested label:
- residual risk not visible

Recovery cue:
- Show the close-relevant residual risk before final acceptance or close.

<a id="label-projection-stale"></a>
### `PROJECTION_STALE`

Label-selection input:
- `PROJECTION_STALE`.

Suggested label:
- stale readable view

Recovery cue:
- Refresh the view before relying on it.

<a id="label-artifact-missing"></a>
### `ARTIFACT_MISSING`

Label-selection input:
- `ARTIFACT_MISSING`.

Suggested label:
- artifact issue

Recovery cue:
- Restore, regenerate, replace, or reconnect the missing or unusable artifact.

<a id="label-validator-failed"></a>
### `VALIDATOR_FAILED`

Label-selection input:
- `VALIDATOR_FAILED`.

Suggested label:
- check failed

Recovery cue:
- Show the specific validator or check result when available.
- Use this fallback label only when no typed public code gives a clearer label.

<a id="status-card-body"></a>
## Status card body

### Input state

- Current read-only state returned by `harness.status`, including `StateSummary`, blockers, pending `UserJudgment` items, evidence summary, close-readiness observations, guarantee display, and next safe action.
- Freshness cues such as source refs, `state_version`, observation time, stale markers, unavailable markers, or capability-limited markers when present.
- Artifact availability only through owner-approved `ArtifactRef` display data or an owner-approved unavailable/redacted note.

### Must show

- A compact current-position card with separate regions for state and active scope, blockers and pending user judgments, run/evidence summary and gaps, close-readiness summary, next safe action, and source refs and freshness.
- That the card is read-only derived display.
- Any stale, partial, unavailable, redacted, or capability-limited source condition.
- Required blockers, unresolved user judgments, and required evidence gaps.
- Close readiness as a current observation, not as a close action.
- Artifact limits, including unavailable or redacted artifact content.

### Must not imply

- The card authorizes a write, records evidence, accepts risk, or closes the Task.
- A green or positive label is a canonical enum value unless a schema owner says so.
- Artifact availability alone proves evidence sufficiency.
- Missing source data can be replaced by optimistic wording.

### User-facing wording

Use direct status language:

- `Status as of {observed_at} from state {state_version}.`
- `Needs your judgment: {pending_judgment_summary}.`
- `Close is blocked by: {close_blocker_summary}.`
- `Next safe action: {next_action}.`

Avoid wording such as `approved`, `accepted`, `verified`, or `closed` unless the corresponding owner record exists and is linked.

### Owner links

- [Projection Authority Reference](projection-and-templates.md) for read-only display and freshness boundaries.
- [Core Model](core-model.md) for Core authority and close-readiness meaning.
- [API State Schemas](api/schema-state.md) for state-shaped display inputs.
- [API Judgment Schemas](api/schema-judgment.md) for user-judgment references.
- [API Artifact Schemas](api/schema-artifacts.md) for `ArtifactRef` display inputs.

<a id="judgment-request-body"></a>
## Judgment request body

### Input state

- One pending user-owned judgment request returned by the user-judgment method.
- Exact question, bounded options, rationale, uncertainty, affected scope, consequence of deferral, and non-substitution notes.
- Any linked source refs, `state_version`, and freshness or capability-limited notes.

### Must show

- One focused decision request that separates the user's answer from evidence, acceptance, residual-risk acceptance, and write authorization.
- The exact question the user is being asked to decide.
- Why this is a user-owned judgment rather than an agent inference.
- Options that are short, distinct, and compatible with the current facts.
- What the answer settles and what it does not settle.
- The consequence of waiting or declining to answer.

### Must not imply

- The agent may choose for the user because an option looks obvious.
- A broad yes replaces sensitive-action approval, final acceptance, residual-risk acceptance, or any other distinct judgment.
- The answer creates evidence, verifies work, or authorizes unrelated writes.
- Grouped questions can be recorded as one answer when the decisions are separate.

### User-facing wording

Use one-question wording:

- `I need your judgment on {decision_scope}.`
- `Choose one: {option_list}.`
- `This decides {settled_scope}. It does not decide {non_settled_scope}.`
- `If you defer, the next safe action is {deferral_action}.`

Avoid pressure wording such as `obviously`, `just approve`, or `I can decide this for you`.

### Owner links

- [Core Model](core-model.md) for user-owned judgment and non-substitution rules.
- [User-judgment methods](api/method-user-judgment.md) for judgment request and record method behavior.
- [API Judgment Schemas](api/schema-judgment.md) for `UserJudgment`, `SensitiveActionScope`, and accepted-risk shapes.
- [Security](security.md) for sensitive-action approval boundaries.

<a id="run--evidence-summary-body"></a>
## Run / evidence summary body

### Input state

- Run and evidence owner records for the active Task or Change Unit.
- Evidence coverage items, required/optional/not-applicable status, supporting run refs, supporting `ArtifactRef` links, blockers, validator results when present, and freshness cues.
- Artifact availability, redaction, blocked-artifact, or unavailable notes from artifact owners.

### Must show

- A concise evidence-position summary with separate regions for what was run or checked, result and confidence limits, required evidence coverage, optional supporting evidence, artifacts and source refs, and gaps, blockers, and next safe action.
- Required evidence separately from optional support.
- Unsupported, partial, stale, blocked, or missing required evidence.
- Which run or artifact supports which claim when that link exists.
- Artifact availability limits, including redaction and body-read limits.
- Freshness or source-state limits that affect evidence use.

### Must not imply

- A run result alone is final acceptance, QA, verification, or residual-risk acceptance.
- An available artifact is automatically sufficient evidence.
- A summary creates evidence that the Run or evidence owner did not record.
- Redacted, omitted, unavailable, or blocked artifact values can be reconstructed.

### User-facing wording

Use coverage language:

- `Checked: {run_or_check_summary}.`
- `Required evidence covered: {covered_items}.`
- `Required evidence still missing: {gap_items}.`
- `Artifact available: {artifact_ref}; content status: {availability_note}.`

Avoid `fully verified`, `QA passed`, or `accepted` unless the relevant owner record exists and is linked.

### Owner links

- [Core Model](core-model.md) for evidence meaning and non-substitution rules.
- [Record-run method](api/method-record-run.md) for run/evidence method behavior.
- [API State Schemas](api/schema-state.md) for evidence summary and validator-shaped display data.
- [API Artifact Schemas](api/schema-artifacts.md) and [Artifact Storage](storage-artifacts.md) for artifact refs, availability, and body-read eligibility.
- [Storage Effects](storage-effects.md) for what does and does not mutate storage.

<a id="close-result-body"></a>
## Close result body

### Input state

- `CloseTaskResult` or close-readiness observations returned by `harness.close_task`.
- `CloseReadinessBlocker[]`, evidence summary, pending user judgments, final-acceptance state, residual-risk state, artifact availability, source refs, freshness cues, and the requested close intent.
- The owner result that distinguishes a read-only close check from a state-changing close attempt.

### Must show

- Whether the body is showing a read-only close check, blocked close attempt, or owner-recorded close result.
- The close intent and whether the owner result was read-only or state-changing.
- Every returned close blocker and its responsible blocker category or next action.
- Remaining evidence, user judgment, final acceptance, residual-risk, or artifact availability gaps.
- Source state version or equivalent freshness cue when available.
- The next safe action when close is blocked.

### Must not imply

- A close check closed the Task.
- A `ready` label closes the Task or removes blockers.
- Broad approval substitutes for final acceptance or residual-risk acceptance.
- The body may hide blockers inside successful-looking prose.
- Missing evidence or unavailable artifacts can be satisfied by close wording.

### User-facing wording

Use close-position wording:

- `Close check: {blocked_or_ready}.`
- `Not closed: {blocker_summary}.`
- `Ready to attempt close, but not closed by this check.`
- `Closed by owner result: {close_ref}.`

Use `Closed by owner result` only when `harness.close_task` returned an actual state-changing close result.

### Owner links

- [Core Model](core-model.md) for close readiness, close honesty, final acceptance, and residual-risk boundaries.
- [Close-task method](api/method-close-task.md) for `harness.close_task` behavior.
- [API State Schemas](api/schema-state.md) for `CloseReadinessBlocker`.
- [API Judgment Schemas](api/schema-judgment.md) for final acceptance and accepted-risk input shapes.
- [API error routing](api/error-routing.md) for close rejection response branch routing.
- [API blocker routing](api/blocker-routing.md) for close-readiness blocker routing.

<a id="agent-context-packet-body"></a>
## Agent context packet body

### Input state

- Current task summary, active scope, out-of-scope items, pending user judgments, blockers, next safe actions, evidence gaps, artifact availability summary, close readiness, residual-risk summary, guarantee level, source refs, and freshness cues.
- Active surface capability context when it affects what the agent may safely infer.
- Only the language and owner sections needed for the next action.

### Must show

- A compact support packet for an agent, not a replacement for owner records.
- A readable surface-supported structure when the surface uses Markdown, JSON-like text, or another display shape.
- Authority and freshness cues visible in the packet.
- Current task and scope in a compact form.
- Pending user-owned judgments and blockers.
- Next safe action and any action the agent must not take yet.
- Evidence, artifact, close-readiness, residual-risk, and guarantee limits.
- Source refs, source freshness, and unavailable or capability-limited conditions.

### Must not imply

- The packet is Core state, storage state, evidence, acceptance, residual-risk acceptance, or close output.
- A stale packet overrides newer state returned by an owner method.
- The agent may bypass user judgment, write authorization, artifact rules, or close blockers.
- The packet should include full schemas, DDL, logs, artifact bodies, unrelated contract material, out-of-scope capability catalogs, or paired bilingual docs by default.

### User-facing wording

If the packet is visible to a user or chat surface, label it as read-only support context:

- `Agent context packet, read-only support context.`
- `Source state: {state_version}; observed at {observed_at}.`
- `Do not proceed without: {blocked_items}.`
- `Next safe action: {next_action}.`

Avoid wording that presents the packet as a record, approval, or close result.

### Owner links

- [Agent Integration](agent-integration.md) for active surface context and capability declarations.
- [Projection Authority Reference](projection-and-templates.md) for read-only display and freshness boundaries.
- [Core Model](core-model.md) for authority, user-owned judgment, close readiness, and residual-risk boundaries.
- [API State Schemas](api/schema-state.md), [API Judgment Schemas](api/schema-judgment.md), and [API Artifact Schemas](api/schema-artifacts.md) for packet input shapes.
- [Security](security.md) for guarantee wording.
