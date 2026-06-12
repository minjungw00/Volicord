# Template bodies

This document owns active current MVP rendered body expectations for status cards, judgment requests, run/evidence summaries, close results, and agent context packets. It is documentation reference material only and does not generate projection files, runtime artifacts, QA records, acceptance records, or close records.

## Owns / Does not own

This document owns:

- exact active template body structure for rendered status or support displays
- user-facing wording boundaries for those bodies
- locale-aware rendered labels where a body needs them
- links from body placeholders to schema and authority owners

This document does not own:

- projection authority, freshness, or read-only derived-display rules; see [Projection Authority Reference](projection-and-templates.md)
- source-of-truth state or storage; see [Core Model](core-model.md) and storage owners
- API schemas or value sets; see API schema owners
- later template candidates; see [Scope Reference](scope.md)

## Boundary

Template text is display text. It can summarize owner records, but it must route authority questions back to those records.

Template output cannot by wording alone:

- authorize writes
- create evidence or persistent artifacts
- satisfy evidence, QA, verification, acceptance, or close gates
- create final acceptance or accept residual risk
- close a Task or create close readiness
- mutate owner records

## Status card body

### Input state

- Current read-only state returned by the status owner path, including `StateSummary`, blockers, pending `UserJudgment` items, evidence summary, close-readiness observations, guarantee display, and next safe action.
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

## Judgment request body

### Input state

- One pending user-owned judgment request from the judgment owner path.
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
- A summary creates evidence that the owner path did not record.
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

## Close result body

### Input state

- `CloseTaskResult` or close-readiness observations returned by the close owner path.
- `CloseReadinessBlocker[]`, evidence summary, pending user judgments, final-acceptance state, residual-risk state, artifact availability, source refs, freshness cues, and the requested close intent.
- The owner result that distinguishes a read-only close check from a state-changing close attempt.

### Must show

- Whether the body is showing a read-only close check, blocked close attempt, or owner-recorded close result.
- The close intent and whether the owner result was read-only or state-changing.
- Every returned close blocker and the owner route for resolving it.
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

Use `Closed by owner result` only when the close owner path returned an actual close result.

### Owner links

- [Core Model](core-model.md) for close readiness, close honesty, final acceptance, and residual-risk boundaries.
- [Close-task method](api/method-close-task.md) for `harness.close_task` behavior.
- [API State Schemas](api/schema-state.md) for `CloseReadinessBlocker`.
- [API Judgment Schemas](api/schema-judgment.md) for final acceptance and accepted-risk input shapes.
- [API Errors](api/errors.md) for close rejection and blocker routing.

## Agent context packet body

### Input state

- Current task summary, active scope, out-of-scope items, pending user judgments, blockers, next safe actions, evidence gaps, artifact availability summary, close readiness, residual-risk summary, guarantee level, source refs, and freshness cues.
- Connector or surface capability context when it affects what the agent may safely infer.
- Only the language and owner sections needed for the next action.

### Must show

- A compact support packet for an agent, not a replacement for owner records.
- A readable connector-approved structure when the connector uses Markdown, JSON-like text, or another display shape.
- Authority and freshness cues visible in the packet.
- Current task and scope in a compact form.
- Pending user-owned judgments and blockers.
- Next safe action and any action the agent must not take yet.
- Evidence, artifact, close-readiness, residual-risk, and guarantee limits.
- Source refs, source freshness, and unavailable or capability-limited conditions.

### Must not imply

- The packet is Core state, storage state, evidence, acceptance, residual-risk acceptance, or close output.
- A stale packet overrides current owner state.
- The agent may bypass user judgment, write authorization, artifact rules, or close blockers.
- Full schemas, DDL, logs, artifact bodies, unrelated contract material, future catalog material, or paired bilingual docs should be injected by default.

### User-facing wording

If the packet is visible to a user or chat surface, label it as read-only support context:

- `Agent context packet, read-only support context.`
- `Source state: {state_version}; observed at {observed_at}.`
- `Do not proceed without: {blocked_items}.`
- `Next safe action: {next_action}.`

Avoid wording that presents the packet as a record, approval, or close result.

### Owner links

- [Agent Integration](agent-integration.md) for connector context discipline and capability context.
- [Projection Authority Reference](projection-and-templates.md) for read-only display and freshness boundaries.
- [Core Model](core-model.md) for authority, user-owned judgment, close readiness, and residual-risk boundaries.
- [API State Schemas](api/schema-state.md), [API Judgment Schemas](api/schema-judgment.md), and [API Artifact Schemas](api/schema-artifacts.md) for packet input shapes.
- [Security](security.md) for guarantee wording.
