# Template bodies

This document owns display-facing wording and presentation packet/body shape for current rendered template bodies:

- status cards
- public error messages
- judgment requests
- run/evidence summaries
- close results
- managed `AGENTS.md` guidance blocks
- agent context packets

It owns only rendered body guidance, user-facing labels, and display phrasing.

Authority, storage records, API error semantics, and close-readiness blocker semantics stay with the linked owners.

## Owner boundaries

This document owns display presentation only:

- rendered template body guidance and presentation packet/body shape for current status and support displays
- the exact generated managed `AGENTS.md` guidance body
- user-facing labels, display phrasing, localized labels, and recovery cues for those bodies
- public-error display labels as display text
- links from body placeholders to schema and authority owners

Neighboring owners stay authoritative:

- projection freshness and read-only derived-display rules: [Projection and template display boundaries](projection-and-templates.md)
- Core state, user-owned judgment, evidence, close readiness, acceptance, and residual risk: [Core Model](core-model.md)
- API schemas and value sets: schema owners and [API Value Sets](api/schema-value-sets.md)
- public `ErrorCode` meanings: [API error codes](api/error-codes.md)
- response branches: [API error routing](api/error-routing.md)
- error precedence: [API error precedence](api/error-precedence.md)
- blocker routing: [API blocker routing](api/blocker-routing.md)
- `ToolError.details`: [API error details](api/error-details.md)
- storage record layout, persistence, artifact lifecycle, and storage effects: storage owners through [Reference Index](README.md)
- support boundaries, security guarantees, and connection context: [Scope Reference](scope.md), [Security](security.md), and [Agent Connection](agent-connection.md)

## Authority boundary

Template text is display text. It can summarize owner records and refer to semantic owners, but it must not redefine those semantics or become authority.

Owner-owned inputs may be used to choose or fill display text:

- public `ErrorCode`
- `CloseReadinessBlocker`
- `state_version`
- `ArtifactRef`

Their meanings, precedence, routing, storage effects, and schema authority remain with their owners.

Template wording must not, by itself:

- create write tickets or mutate owner records
- create evidence, persistent artifacts, final acceptance, or residual-risk acceptance
- satisfy evidence, QA, verification, acceptance, close-readiness, or close gates
- define storage layout, storage effects, or make a rendered body the storage authority
- define, rename, localize, or change public `ErrorCode` identifiers or meanings
- define or change response-branch behavior, error precedence, or machine-readable detail keys
- define or change close-readiness blocker semantics, blocker codes, or blocker routing
- convert rejected-response errors into blockers or blocked results

## Public error display labels

Use this section to choose display labels and recovery cues when rendering public API errors for a user or agent-facing display.

It does not define:

- which errors exist
- what they mean
- which branch wins
- how blocked results are routed

Rendered error copy must:

- Preserve the public `ErrorCode` when the exact diagnostic identifier is shown.
- Pair a concise label with one recovery cue when the display has room.
- Keep labels separate from `CloseReadinessBlocker.code`, `WriteDecisionReason.code`, `PlannedBlocker.code`, and `ToolError.details` keys.
- Link to the API owner when explaining code meaning, precedence, response branches, blocker routing, or machine-readable details.

Rendered error copy must not:

- Replace a public `ErrorCode` with a localized label.
- Define or change public `ErrorCode` semantics.
- Treat a label as a semantic owner or machine-readable code.
- Hide close blockers or turn rejected responses into blocked results.

Owner links:
- [API error codes](api/error-codes.md) owns public code meanings.
- [API error precedence](api/error-precedence.md): error precedence.
- [API error routing](api/error-routing.md): API response branch routing.
- [API blocker routing](api/blocker-routing.md): blocker routing.
- [API error details](api/error-details.md): machine-readable detail rules.

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

<a id="label-invocation-context-mismatch"></a>
### `INVOCATION_CONTEXT_MISMATCH`

Label-selection input:
- `INVOCATION_CONTEXT_MISMATCH`.

Suggested label:
- invocation context mismatch

Recovery cue:
- Use the registered Agent Connection, User Channel, project routing, or method-compatible invocation context.
- Repair connection binding or invocation context settings when needed.

<a id="label-capability-insufficient"></a>
### `CAPABILITY_INSUFFICIENT`

Label-selection input:
- `CAPABILITY_INSUFFICIENT`.

Suggested label:
- insufficient connection capability

Recovery cue:
- Use a compatible Agent Connection.
- Reduce the operation or avoid the missing capability.

<a id="label-no-active-task"></a>
### `NO_ACTIVE_TASK`

Label-selection input:
- `NO_ACTIVE_TASK`.

Suggested label:
- no current `Task`

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

<a id="label-write-ticket"></a>
### Write Ticket

Label-selection input:
- `WRITE_TICKET_REQUIRED` or `WRITE_TICKET_INVALID`.

Suggested label:
- missing or unusable write ticket

Recovery cue:
- Call or retry `volicord.prepare_write` for the exact operation, current scope, and current state.

<a id="label-judgment"></a>
### Judgment

Label-selection input:
- `DECISION_REQUIRED` or `DECISION_UNRESOLVED`.

Suggested label:
- judgment needed

Recovery cue:
- Request or resolve the focused `UserActionRequest` through its owned workflow.

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
- stale view

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

<a id="managed-agents-guidance-body"></a>
## Managed `AGENTS.md` guidance body

Init generates this exact block. It states durable authority and safety
boundaries without requiring a fixed status/intake/check-close call ritual.

```markdown
<!-- BEGIN VOLICORD MANAGED GUIDANCE -->
# Volicord

- Treat Volicord's recorded scope and Store-derived tagged current effective shaping authority graph as authoritative. Superseded checkpoint, request, resolution, and application history is immutable audit history, never actionable authority.
- Do not modify Product Repository files outside an active compatible write authorization.
- Do not infer, resolve, approve, or record user-owned trust judgments on the user's behalf.
- For the request `Run the Volicord integration verification.`, call `volicord.list_projects`, then `volicord.begin_integration_verification`. Follow its returned `workflow` tagged state: `awaiting_probe` calls its exact `volicord.guard_probe` tool once; `awaiting_observation` calls its exact `volicord.get_integration_verification` status tool once; `repair_required` and `complete` call no verification tool. Do not use shell sleep or poll loops, make repeated status calls, or automatically restart the workflow in the same turn. Begin, probe, and status expose the same workflow state.
- Only that first-party state-directed workflow proves current MCP and Guard correlation. Manual stdio and CLI MCP preflight are diagnostic and are not managed-host evidence.
- If Volicord tools are not exposed, report the managed MCP connection as unavailable. Do not substitute raw stdio, hand-author Codex `_meta`, or treat resources/list or resource templates as tool proof; use read-only connection status or MCP preflight only for diagnosis.
- `volicord connection verify` is optional active diagnostics only; it does not replace the managed-host workflow.
- Follow the tagged workflow's `required_action`; do not select progression from top-level array order. The tagged workflow action catalog is mutation admission authority: call only a currently allowed task-bound method, never speculate with a different shaping or implementation method, and do not treat read-only status as mutation authority.
- Use the exact method-specific action form and `action_form_ref` for the called method; an action form for one method never authorizes another. On failure, use the MCP tool schema and returned retry contract; ordinary CLI `--help` and binary strings are not MCP request schemas.
- Preserve JSON primitive types: never convert null, booleans, or numbers into strings, and never infer another tagged-union branch after validation rejects the selected branch.
- An MCP argument error does not diagnose Task-state corruption. Use the current authoritative context and action-form catalog, and recommend repair only when Volicord reports persisted-data corruption or an explicit repair workflow.
- Do not modify Product Repository files before the required authority mutation succeeds. If checkpoint recording or UserAction creation fails, report that neither was created and that Core state and Product Repository files are unchanged.
- Never replace the current shaping checkpoint to remove a pending or accepted-but-unapplied decision. Preserve its UserAction authority and follow the tagged recovery method.
- Carry every current compatible applied decision explicitly through `carry_forward_application_refs` when revising a checkpoint. Never replace a checkpoint to discard applied authority.
- Stale shaping authority grants no permission. Never carry a stale application forward or reuse its accepted resolution. For `application_authority_stale`, follow the exact `stale_authority_actions`: `retire` it or `reauthorize` it through a fresh successor gap and fresh `UserActionRequest`, preserving immutable `ShapingAuthorityReauthorization` lineage.
- Inspect the exact User Channel resolution outcome. Resolution does not apply a shaping decision: apply only accepted, current, compatible authority through its `application_owner`, using the exact current resolution refs.
- After rejection, deferral, or expiration, follow `decision_recovery_required` and revise shaping. Never retry resolution of a terminal or expired request. If the revised plan still needs that judgment, create a successor UserActionRequest with an independent identity; chat text cannot replace it.
- A rejected, deferred, or expired decision grants no authority and keeps Product Repository mutation unavailable. Surface that outcome and do not hide it as success.
- During `work/implementation`, an authority-invalidating scope, baseline, or Change Unit update is rejected before mutation. Follow the tagged `volicord.close_task` recovery to leave implementation; implementation work never returns silently to shaping.
- Do not invent a scope decision or pass a scope-decision ref for product-only or technical-only work; follow that decision's mode-specific application owner.
- Creating or replacing a Change Unit does not advance the Task phase. For work, call `volicord.advance_task` only when the tagged workflow requires explicit advance and never while a UserAction is pending. Do not call `volicord.prepare_write` before the Task enters implementation.
- Advisor work uses only a non-write Change Unit. On `ready_to_finalize_advice`, finalize the current advisor result with `volicord.finalize_advice`; do not use `volicord.record_run`, `volicord.advance_task`, or `volicord.prepare_write` for advisor.
- Create current UserAction requests before presenting user-owned choices. A chat reply is not a User Channel resolution; surface the canonical CLI inbox instruction.
- Never hide or paraphrase a rejected mutation as success. Surface pre-Core admission rejection accurately, including that Core was not reached and state did not change, plus the tagged workflow and every rejection and recovery fact in `presentation.must_surface`.
- Evaluate close readiness only during close review. Close blockers do not replace tagged workflow progression.
- Call `volicord.status` only when the current Task state is unknown or an authoritative refresh is required.
- Do not claim completion while Volicord reports close blockers. If Volicord is unavailable, disclose that its state was not updated or verified.
<!-- END VOLICORD MANAGED GUIDANCE -->
```

The block is generated repository guidance, not Core state, a write ticket,
user authority, proof that an Agent followed it, or permission to edit product
files. Its closed semantic fact set has a canonical digest included in the
project integration revision, so changing one of these guidance semantics
invalidates revision-scoped managed evidence even when rendering punctuation
or the hook command is otherwise unchanged.

## Status card body

### Input state

- Current read-only state returned by `volicord.status`.
- Display inputs such as `StateSummary`, blockers, pending
  `AgentSafeUserActionRequestSummary` entries, evidence summary and provenance
  state, close-readiness observations, residual-risk coverage, project
  continuity summary, guarantee display, and next safe action.
- Freshness cues such as source refs, `state_version`, observation time, stale markers, unavailable markers, or capability-limited markers when present.
- Artifact availability only through owner-approved `ArtifactRef` display data or an owner-approved unavailable/redacted note.

### Must show

- A compact current-position card.
- Separate regions for state and current scope.
- Current goal, current scope, out-of-scope items, and allowed action state when those fields are present.
- Separate regions for blockers and pending user actions.
- Separate regions for run/evidence summary, evidence provenance limits, and gaps.
- Separate regions for close-readiness summary, next safe action, source refs, and freshness.
- Separate regions for residual risks and continuity records carried forward.
- That the card is read-only derived display.
- Any stale, partial, unavailable, redacted, or capability-limited source condition.
- Required blockers, unresolved user actions, and required evidence gaps.
- Close readiness as a current observation, not as a close action.
- Artifact limits, including unavailable or redacted artifact content.

### Must not imply

- The card creates write tickets, records evidence, accepts risk, or closes the `Task`.
- A green or positive label is a canonical enum value without support from [API Value Sets](api/schema-value-sets.md).
- Artifact availability alone proves evidence sufficiency.
- Missing source data can be replaced by optimistic wording.
- An existing pending request's question, options, context, resolution form,
  capture path, command, URL, or credential can be reconstructed from blocker,
  next-action, or template text. Display that request only by request ID,
  `pending`, and next actor/User Channel. A pre-request
  `missing_final_acceptance` action may still show the blocking question needed
  for the Agent to create the request; after creation, the generic pending rule
  applies.

### User-facing wording

Use direct status language:

- `Status as of {observed_at} from state {state_version}.`
- `Needs your action: {pending_user_action_summary}.`
- `Close is blocked by: {close_blocker_summary}.`
- `Evidence provenance: {provenance_summary}.`
- `Continuity carried forward: {continuity_summary}.`
- `Next safe action: {next_action}.`

Use wording such as `approved`, `accepted`, `verified`, or `closed` only when the corresponding owner record exists and is linked.

Otherwise, avoid those words.

### Owner links

- [Projection and template display boundaries](projection-and-templates.md) for read-only display and freshness boundaries.
- [Core Model](core-model.md) for Core authority and close-readiness meaning.
- [API State Schemas](api/schema-state.md) for state-shaped display inputs.
- [API User-Action Schemas](api/schema-user-action.md) for the shared request,
  adapter-neutral resolution form, status, and resolution shapes.
- [API Judgment Schemas](api/schema-judgment.md) for choice-specific judgment
  detail.
- [API Artifact Schemas](api/schema-artifacts.md) for `ArtifactRef` display inputs.

<a id="judgment-request-body"></a>
## Judgment request body

### Input state

- One typed `CliUserActionInboxItem` supplied to a verified User
  Channel renderer. This template is never built from the Agent-facing
  `volicord.request_user_action` result, which contains only the safe summary.
- Exact question and bounded options.
- Rationale, uncertainty, affected scope, consequence of deferral, and non-substitution notes.
- Any linked source refs, `state_version`, and freshness or capability-limited notes.

### Must show

- One focused decision request that separates the user's answer from evidence, acceptance, residual-risk acceptance, and write ticket.
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
- This complete question/options template may be rendered in Agent Connection
  output, fallback text, status, close, replay, or operation-result bytes.

### User-facing wording

Use one-question wording:

- `I need your judgment on {decision_scope}.`
- `Choose one: {option_list}.`
- `This decides {settled_scope}. It does not decide {non_settled_scope}.`
- `If you defer, the next safe action is {deferral_action}.`

Avoid pressure wording such as `obviously`, `just approve`, or `I can decide this for you`.

### Owner links

- [Core Model](core-model.md) for user-owned judgment and non-substitution rules.
- [Request-user-action method](api/method-request-user-action.md) for request behavior.
- [Resolve-user-action method](api/method-resolve-user-action.md) for immutable User Channel resolution behavior.
- [API user-action schemas](api/schema-user-action.md) for the common request, resolution, status, and adapter-neutral resolution form.
- [API Judgment Schemas](api/schema-judgment.md) for choice payloads, `SensitiveActionScope`, and accepted-risk shapes.
- [Security](security.md) for sensitive-action approval boundaries.

<a id="run--evidence-summary-body"></a>
## Run / evidence summary body

### Input state

- Run and evidence owner records for the current `Task` or Change Unit.
- Evidence coverage items and required/optional/not-applicable status.
- Supporting run refs, supporting `ArtifactRef` links, blockers, and validator results when present.
- Freshness cues.
- Artifact availability, redaction, blocked-artifact, or unavailable notes from artifact owners.

### Must show

- A concise evidence-position summary.
- A separate region for what was run or checked.
- Separate regions for result and confidence limits.
- Separate regions for required evidence coverage and optional supporting evidence.
- Separate regions for artifacts and source refs.
- Separate regions for gaps, blockers, and next safe action.
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

Use `fully verified`, `QA passed`, or `accepted` only when the relevant owner record exists and is linked.

Otherwise, avoid those words.

### Owner links

- [Core Model](core-model.md) for evidence meaning and non-substitution rules.
- [Record-run method](api/method-record-run.md) for run/evidence method behavior.
- [API State Schemas](api/schema-state.md) for evidence summary and validator-shaped display data.
- [API Artifact Schemas](api/schema-artifacts.md) and [Artifact Storage](storage-artifacts.md) for artifact refs, availability, and body-read eligibility.
- [Storage Effects](storage-effects.md) for what does and does not mutate storage.

<a id="close-result-body"></a>
## Close result body

### Input state

- `CheckCloseResult` returned by `volicord.check_close` or `CloseTaskResult`
  returned by `volicord.close_task`.
- `CloseReadinessBlocker[]`, evidence summary, and pending
  `AgentSafeUserActionRequestSummary` entries.
- Final-acceptance state, residual-risk state, and artifact availability.
- Project continuity records returned by the close result.
- Source refs, freshness cues, and the requested method or close intent.
- The owner result that distinguishes a read-only close check from a state-changing close attempt.

### Must show

- Whether the body is showing a read-only close check, blocked close attempt, or owner-recorded close result.
- The close intent when present, and whether the owner result was read-only or state-changing.
- Every returned close blocker and its responsible blocker category or next action.
- Remaining evidence, user-action, final-acceptance, residual-risk, or artifact availability gaps.
- Source state version or equivalent freshness cue when available.
- Continuity records that remain relevant after a successful close.
- The next safe action when close is blocked.

### Must not imply

- A close check closed the `Task`.
- A `ready` label closes the Task or removes blockers.
- Broad approval substitutes for final acceptance or residual-risk acceptance.
- The body may hide blockers inside successful-looking prose.
- Missing evidence or unavailable artifacts can be satisfied by close wording.
- An existing pending request's question, options, context, form, capture path,
  command, URL, or credential can appear in blocker or next-action prose. Use
  only its agent-safe summary and generic User Channel guidance. The separate
  pre-request `missing_final_acceptance` state may show the question required to
  create a request.

### User-facing wording

Use close-position wording:

- `Close check: {blocked_or_ready}.`
- `Not closed: {blocker_summary}.`
- `Ready to attempt close, but not closed by this check.`
- `Closed by recorded close result: {close_ref}.`
- `Continuity carried forward: {continuity_summary}.`

Use `Closed by recorded close result` only when `volicord.close_task` returned
an actual state-changing close result.

### Owner links

- [Core Model](core-model.md) for close readiness, close honesty, final acceptance, and residual-risk boundaries.
- [Close method](api/method-close-task.md) for `volicord.check_close` and `volicord.close_task` behavior.
- [API State Schemas](api/schema-state.md) for `CloseReadinessBlocker`.
- [API Judgment Schemas](api/schema-judgment.md) for final acceptance and accepted-risk input shapes.
- [API error routing](api/error-routing.md) for close rejection response branch routing.
- [API blocker routing](api/blocker-routing.md) for close-readiness blocker routing.

<a id="agent-context-packet-body"></a>
## Agent context packet body

### Input state

- Current task summary, current scope, and out-of-scope items.
- Pending `AgentSafeUserActionRequestSummary` entries, blockers, and next safe
  actions.
- Evidence gaps and artifact availability summary.
- Close readiness, residual-risk summary, and guarantee level.
- Source refs and freshness cues.
- Current connection context and capability limits when they affect what the agent may safely infer.
- Only the language and owner sections needed for the next action.

### Must show

- A compact support packet for an agent, not a replacement for owner records.
- A readable display-supported structure when the display uses Markdown, JSON-like text, or another display shape.
- Authority and freshness cues visible in the packet.
- Current task and scope in a compact form.
- Pending user-owned actions and blockers.
- Next safe action and any action the agent must not take yet.
- Evidence, artifact, close-readiness, residual-risk, and guarantee limits.
- Source refs, source freshness, and unavailable or capability-limited conditions.

### Must not imply

- The packet is Core state, storage state, evidence, acceptance, residual-risk acceptance, or close output.
- A stale packet overrides newer state returned by an owner method.
- The agent may bypass user judgment, write-ticket, artifact rules, or close blockers.
- The packet should include full schemas, DDL, logs, artifact bodies, or unrelated contract material by default.
- The packet should include out-of-scope capability catalogs or paired bilingual docs by default.
- The packet should include or reconstruct a pending request's question,
  options, context, resolution form, capture path, command, URL, or credential.

### User-facing wording

If the packet is visible to a user or chat display, label it as read-only support context:

- `Agent context packet, read-only support context.`
- `Source state: {state_version}; observed at {observed_at}.`
- `Do not proceed without: {blocked_items}.`
- `Next safe action: {next_action}.`

Avoid wording that presents the packet as a record, approval, or close result.

### Owner links

- [Agent Connection](agent-connection.md) for current connection context and connection capability declarations.
- [Projection and template display boundaries](projection-and-templates.md) for read-only display and freshness boundaries.
- [Core Model](core-model.md) for authority, user-owned judgment, close readiness, and residual-risk boundaries.
- [API State Schemas](api/schema-state.md), [API Judgment Schemas](api/schema-judgment.md), and [API Artifact Schemas](api/schema-artifacts.md) for packet input shapes.
- [Security](security.md) for guarantee wording.
