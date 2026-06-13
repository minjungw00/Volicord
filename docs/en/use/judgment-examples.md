# Judgment Examples

Use these examples after the [User Guide](user-guide.md) when a task is blocked by a choice the agent should not make alone.

The examples are illustrative. They are not an exhaustive policy, a schema reference, a close-readiness contract, or proof that every similar case uses the same route. For exact owner boundaries, use [Core Model](../reference/core-model.md), [Scope](../reference/scope.md), and the relevant owners from the [Reference Index](../reference/README.md).

Each example separates the user's decision from what the agent may do and what the agent must not imply.

## Product choice

Scenario:

- The agent is adding save feedback to an account settings form.

User decides:

- Whether the feedback should be inline text, a toast, or a modal.
- Whether the save-feedback behavior fits the product tone, flow, accessibility expectation, and user-facing trade-off.

Agent may do:

- Inspect existing UI patterns.
- Recommend the option that best fits the inspected product behavior.
- Explain consequences, such as a modal interrupting flow or a toast leaving the form unobstructed.

Agent must not imply:

- Surrounding code is enough to infer product intent.
- This product choice also grants final acceptance, residual-risk acceptance, sensitive-action approval, scope expansion, or write permission.

Owner links:

- [Core Model](../reference/core-model.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)

## Technical direction

Scenario:

- The agent needs to choose how account data export confirmation should be implemented.

User decides:

- Which material technical direction to take when architecture, dependency, authentication, migration, security, privacy, retention, or compatibility is at stake.
- Whether the cost and reversibility of the choice fit the task.

Agent may do:

- Inspect the existing design.
- Narrow options to a bounded recommendation with trade-offs.
- Continue read-only investigation while the direction is unresolved.

Agent must not imply:

- A strong recommendation is the same thing as user-owned technical judgment.
- A technical choice approves dependency installation, product-file writes, or migration work by itself.

Owner links:

- [Core Model](../reference/core-model.md)
- [Agent Guide](agent-guide.md)

## Scope change

Scenario:

- The accepted task is limited to `src/auth`, but the agent finds a helper path outside the accepted boundary.

User decides:

- Whether the task may expand to the named helper path.
- Whether to keep the original scope, expand it narrowly, or convert the work to read-only investigation.

Agent may do:

- Name the exact path or behavior that appears necessary.
- Explain why the accepted boundary blocks the next safe action.
- Continue inspection inside accepted scope while writes outside scope remain blocked.

Agent must not imply:

- Scope expansion can be inferred from implementation convenience.
- Scope change also creates sensitive-action approval, final acceptance, residual-risk acceptance, or write authorization.

Owner links:

- [Core Model](../reference/core-model.md)
- [Update-scope Method](../reference/api/method-update-scope.md)

## Sensitive action

Scenario:

- A focused fix may require one dependency install, network action, secret access, deployment step, or destructive command.

User decides:

- Whether to permit the named sensitive action.
- The command or tool, intended paths, host, dependency or target, secret handle if any, and time window.

Agent may do:

- Present the smallest useful request.
- Offer a no-sensitive-action fallback when one exists.
- Stop or choose a safer path if the user denies or narrows the approval.

Agent must not imply:

- Sensitive approval is product-file write authorization, final acceptance, residual-risk acceptance, or security authority.
- A broad "go ahead" approves unrelated installs, upgrades, deploys, secret printing, or product decisions.

Owner links:

- [Core Model](../reference/core-model.md)
- [Security](../reference/security.md)

## Evidence gap

Scenario:

- Tests pass, but the claim also depends on user-visible behavior that the agent did not inspect.

User decides:

- Whether to add missing evidence, narrow the claim, keep the task open, or stop.
- Which visible result the evidence should support when the claim is ambiguous.

Agent may do:

- Identify missing or stale evidence.
- Explain what claim cannot be supported yet.
- Follow the artifact and evidence owners when attaching or referencing eligible artifacts.

Agent must not imply:

- Agent confidence, passing tests alone, a chat summary, or broad approval proves evidence sufficiency.
- Artifact availability by itself creates evidence, final acceptance, residual-risk acceptance, QA, or close readiness.

Owner links:

- [Core Model](../reference/core-model.md)
- [API State Schemas](../reference/api/schema-state.md)
- [Artifact Storage](../reference/storage-artifacts.md)

## Final acceptance

Scenario:

- The agent has completed the named result and is ready for the user to judge the visible outcome.

User decides:

- Whether the visible completed result is accepted.
- Whether to reject the result, name required changes, or keep the task open for broader review.

Agent may do:

- Present completed scope, changed files or no-file outcome, checks, known gaps, and visible residual risk.
- Ask for final acceptance only when the result basis is visible enough for the user to judge.

Agent must not imply:

- "Looks good" is final acceptance unless that exact question was pending.
- Final acceptance supplies missing evidence, accepts residual risk, expands scope, or accepts unrelated files.

Owner links:

- [Core Model](../reference/core-model.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)

## Residual risk

Scenario:

- Password reset remains out of scope for a login slice, and the remaining risk is visible.

User decides:

- Whether to accept the named residual risk.
- Whether to reject the risk, add work to remove it, or narrow the close claim.

Agent may do:

- Explain the risk, affected area, consequence, and why it remains.
- Offer alternatives that remove, reduce, or keep the risk visible without accepting it.

Agent must not imply:

- Residual-risk acceptance is final acceptance, evidence sufficiency, verification, QA, or proof that no risk remains.
- Acceptance of one risk accepts other risks or hides the risk from close reporting.

Owner links:

- [Core Model](../reference/core-model.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)

## Close readiness

Scenario:

- The user asks whether the task can honestly finish now.

User decides:

- Whether to handle blockers, provide required final acceptance, accept a named residual risk when asked, cancel, or keep the task open.

Agent may do:

- Summarize changed scope, evidence, checks, known blockers, visible residual risk, and next safe action.
- Ask only the missing judgment that changes close readiness.

Agent must not imply:

- A status summary, passing checks, final acceptance alone, residual-risk acceptance alone, or chat text closes the task.
- A read-only close-readiness review creates evidence, acceptance, residual-risk acceptance, or task state.

Owner links:

- [Core Model](../reference/core-model.md)
- [Close-task Method](../reference/api/method-close-task.md)
- [API blocker routing](../reference/api/blocker-routing.md)

## Ordinary implementation detail

Scenario:

- The agent needs to pick a local variable name or place a focused test beside an existing related test.

User decides:

- No new user judgment is needed when the detail stays inside accepted scope and acceptance criteria.
- Prior accepted product, technical, and scope decisions remain the boundary.

Agent may do:

- Choose ordinary local details that follow project style.
- Escalate only if the detail changes user-visible behavior, public interfaces, privacy or security behavior, dependencies, scope, sensitive action, acceptance, or residual risk.

Agent must not imply:

- Every implementation detail needs a user decision.
- Agent-owned latitude can change product behavior, scope, sensitive actions, acceptance, or residual risk without asking.

Owner links:

- [Agent Guide](agent-guide.md)
- [Core Model](../reference/core-model.md)

## Where to go next

Use [Agent Guide](agent-guide.md) for operating patterns and [Scope](../reference/scope.md) when an example raises a baseline, profile-gated, or out-of-scope question. Use [Reference Index](../reference/README.md) for exact owner routing.
