# Judgment examples

Use this compact catalog after the [User Guide](user-guide.md) when a task is blocked by a choice the agent should not make alone. The examples show judgment-boundary guidance for a Harness-connected session. They are not runtime records, generated evidence, acceptance records, close records, or conformance outputs from this documentation repository.

Each example separates the user's decision from agent action, the Harness record, and the implication Harness must avoid. The examples are intentionally brief; contract details stay in the linked owner documents.

## Where to go next

Working users can read these examples after the [User Guide](user-guide.md), then check [Scope](../reference/scope.md) when they need to know whether a behavior is supported, profile-gated, or out of scope.

Use per-example reference links only when you need the exact owner. Do not start a user-facing task by asking the user to read schema internals.

## Product choice

User decides:
- Which user-visible save feedback pattern to use: inline message, toast, or modal.
- Whether the chosen behavior fits the product tone, flow, accessibility expectation, and user-facing trade-off.

Agent may do:
- Inspect the existing UI pattern and recommend the option that best fits current product behavior.
- Explain consequences, such as a modal interrupting flow or a toast leaving the form unobstructed.

Harness records:
- The specific product choice and any user-stated basis for that choice.
- The fact that this answer resolves only the named product decision.

Harness must not imply:
- The agent inferred product intent from surrounding code without asking.
- The product choice also grants final acceptance, residual-risk acceptance, sensitive-action approval, scope expansion, or write permission.

Reference links:
- [Core Model](../reference/core-model.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)

## Technical choice

User decides:
- Which material technical direction to take when architecture, dependency, authentication, migration, security, privacy, retention, or compatibility is at stake.
- Whether the cost and reversibility of the choice fit the task.

Agent may do:
- Inspect the current design and narrow the options to a recommendation with trade-offs.
- Continue read-only investigation while the technical direction is unresolved.

Harness records:
- The named technical decision and its scope.
- The fact that the decision does not settle separate product, scope, approval, acceptance, or residual-risk questions.

Harness must not imply:
- The agent can turn a strong recommendation into user-owned technical judgment.
- A technical choice approves dependency installation, product-file writes, or future migrations by itself.

Reference links:
- [Core Model](../reference/core-model.md)
- [Agent Guide](agent-guide.md)

## Scope change

User decides:
- Whether the task may expand beyond the accepted boundary, such as adding one named helper path outside `src/auth`.
- Whether to keep the original scope, expand it narrowly, or convert the work to read-only investigation.

Agent may do:
- Name the exact path or behavior that appears necessary and explain why the current boundary blocks it.
- Continue inspection inside accepted scope while writes outside scope remain blocked.

Harness records:
- The user's scope decision and the exact added or rejected boundary.
- Any remaining blocker if the task cannot proceed within the accepted scope.

Harness must not imply:
- Scope expansion was inferred from implementation convenience.
- Scope change also creates sensitive-action approval, final acceptance, residual-risk acceptance, or product-file write authorization.

Reference links:
- [Core Model](../reference/core-model.md)
- [Update-scope method](../reference/api/method-update-scope.md)

## Sensitive approval

User decides:
- Whether to permit one named sensitive action, such as a specific dependency install or destructive command.
- The action's command/tool, intended paths, host, dependency or target, secret handle if any, and time window.

Agent may do:
- Present the smallest useful approval request and a no-sensitive-action fallback when one exists.
- Stop or choose a safer path if the user denies or narrows the approval.

Harness records:
- The scoped sensitive-action approval, denial, or deferral.
- The honest capability claim for what the active surface can observe or enforce.

Harness must not imply:
- Sensitive approval is product-file write authorization, final acceptance, residual-risk acceptance, or security authority; see [Security](../reference/security.md).
- A broad "go ahead" approves unrelated installs, future upgrades, deploys, secret printing, or product decisions.

Reference links:
- [Core Model](../reference/core-model.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)
- [Security](../reference/security.md)

## Evidence sufficiency

User decides:
- Whether to add missing evidence, narrow the claim, keep the task open, or stop.
- Which visible result the evidence should support when the claim is ambiguous.

Agent may do:
- Identify missing or stale evidence and explain what claim cannot be supported yet.
- Attach or reference eligible artifacts only through the active owner path.

Harness records:
- Evidence references, supported or missing coverage, and evidence gaps from the owner path.
- Close-relevant evidence blockers when required coverage is absent, stale, or unusable.

Harness must not imply:
- Agent confidence, passing tests alone, a chat summary, or a user's broad approval proves evidence sufficiency.
- Artifact availability by itself creates evidence, final acceptance, residual-risk acceptance, QA, or close readiness.

Reference links:
- [Core Model](../reference/core-model.md)
- [API State Schemas](../reference/api/schema-state.md)
- [Artifact Storage](../reference/storage-artifacts.md)

## Final acceptance

User decides:
- Whether the visible completed result is accepted.
- Whether to reject the result, name required changes, or keep the task open for broader review.

Agent may do:
- Present the completed scope, changed files or no-file outcome, checks, known gaps, and residual-risk visibility.
- Ask for final acceptance only when the result basis is visible enough for the user to judge.

Harness records:
- The final-acceptance answer for the named result.
- The basis that was visible when the user accepted or rejected the result.

Harness must not imply:
- "Looks good" is final acceptance unless that exact final-acceptance question was pending.
- Final acceptance supplies missing evidence, accepts residual risk, expands scope, or accepts unrelated files.

Reference links:
- [Core Model](../reference/core-model.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)

## Residual risk acceptance

User decides:
- Whether to accept one named visible residual risk, such as password reset remaining out of scope for a login slice.
- Whether to reject the risk, add work to remove it, or narrow the close claim.

Agent may do:
- Explain the risk, affected area, consequence, and why it remains.
- Offer alternatives that remove, reduce, or make the risk visible without accepting it.

Harness records:
- The user's answer for that specific residual risk.
- The risk scope and consequence that were visible at the time of the answer.

Harness must not imply:
- Residual-risk acceptance is final acceptance, evidence sufficiency, verification, QA, or proof that no risk remains.
- Acceptance of one risk accepts other risks or hides the risk from close reporting.

Reference links:
- [Core Model](../reference/core-model.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)

## Close readiness review

User decides:
- Whether to ask for a close-readiness review, handle blockers, provide required final acceptance, or accept a named residual risk when asked.
- Whether the task should remain open, narrow its claim, cancel, or proceed toward close.

Agent may do:
- Summarize changed scope, evidence, checks, known blockers, visible residual risk, and next safe action.
- Ask only the missing judgment that changes close readiness.

Harness records:
- Current close-readiness findings and close blockers from the owner path.
- A close result only when the active close path permits it and required blockers are resolved.

Harness must not imply:
- A status summary, passing checks, final acceptance alone, residual-risk acceptance alone, or chat text closes the task.
- A read-only close-readiness review creates generated evidence, acceptance, residual-risk acceptance, or runtime state in this documentation repository.

Reference links:
- [Core Model](../reference/core-model.md)
- [Close-task method](../reference/api/method-close-task.md)
- [API Errors](../reference/api/errors.md)

## Cancellation or defer decision

User decides:
- Whether to cancel the current task, defer the blocking choice, or narrow the task to read-only investigation.
- Whether stopping means no successful result or only a pause before later work.

Agent may do:
- Preserve inspected facts and blockers without claiming implementation completion.
- Return with a concrete follow-up when the work is narrowed or deferred.

Harness records:
- The cancellation, deferral, or narrowed-task decision when the owner path allows it.
- That no successful close judgment has been made unless a separate close path later succeeds.

Harness must not imply:
- Cancellation or deferral is product direction, technical direction, evidence sufficiency, final acceptance, residual-risk acceptance, or close readiness for a completed result.

Reference links:
- [Core Model](../reference/core-model.md)
- [Close-task method](../reference/api/method-close-task.md)

## Usually agent-owned implementation detail

User decides:
- No new user judgment is needed when the detail stays inside accepted scope and acceptance criteria.
- Prior accepted product, technical, and scope decisions remain the boundary.

Agent may do:
- Choose ordinary local details such as a clearer variable name or the existing nearby test location.
- Escalate only if the detail changes user-visible behavior, public interfaces, privacy/security behavior, dependencies, scope, sensitive action, acceptance, or residual risk.

Harness records:
- No new `UserJudgment` for ordinary implementation latitude.
- Any later escalation only if the detail crosses a user-owned boundary.

Harness must not imply:
- Every implementation detail needs a user decision.
- Agent-owned latitude can change product behavior, scope, sensitive actions, acceptance, or residual risk without asking.

Reference links:
- [Agent Guide](agent-guide.md)
- [Core Model](../reference/core-model.md)

## Design quality finding waiver

User decides:
- Whether to fix the design-quality finding, treat it as advisory, narrow scope, provide evidence, or route a named risk through an active residual-risk path.
- The user does not decide an active design-quality waiver in the baseline scope because that waiver route is not active.

Agent may do:
- State the finding, the active owner path it affects, and the next safe action.
- Route the issue to product judgment, scope change, evidence, residual-risk visibility, or no action when the active owners support that route.

Harness records:
- The active owner-path decision or blocker, if one exists.
- No separate design-quality waiver record in the baseline scope.

Harness must not imply:
- A design-quality finding creates its own active gate, validator family, close blocker category, waiver route, QA result, evidence record, acceptance record, or close authority.
- An out-of-scope design-policy waiver candidate can be used as a baseline waiver.

Reference links:
- [Design Quality](../reference/design-quality.md)
- [Core Model](../reference/core-model.md)
- [Scope Reference](../reference/scope.md)
