# Judgment Examples

Use these examples after the [User Guide](user-guide.md) when a task is blocked by a choice the agent should not make alone.

The examples are illustrative. They help readers recognize boundary shapes; they are not an exhaustive policy, a schema reference, a close-readiness contract, or proof that every similar case uses the same route. No scenario here is a required shared sample task. For exact owner boundaries, use [Core Model](../reference/core-model.md), [Scope](../reference/scope.md), and the relevant owners from the [Reference Index](../reference/README.md).

Each example separates the user's decision from what the agent may do and what the agent must not imply. User-owned judgment, sensitive-action approval, final acceptance, residual-risk acceptance, verification criteria, evidence, close readiness, and `Write Authorization` stay distinct.

## Product choice

Scenario:

- The agent is adding save feedback to a profile settings form.

User decides:

- Whether the feedback should be inline text, a toast, or a modal.
- Whether the save-feedback behavior fits the product tone, flow, accessibility needs, and user-facing trade-off.

Agent may do:

- Inspect existing UI patterns.
- Recommend the option that best fits the inspected product behavior.
- Explain consequences, such as a modal interrupting flow or a toast leaving the form unobstructed.

Agent must not imply:

- Surrounding code is enough to infer product intent.
- This product choice also grants final acceptance, residual-risk acceptance, sensitive-action approval, scope expansion, evidence sufficiency, or `Write Authorization`.

Owner links:

- [Core Model](../reference/core-model.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)

## Technical direction

Scenario:

- The agent needs to choose whether a new report-export confirmation should live in the UI flow, an existing service boundary, or a new shared helper.

This scenario is only about who owns a material technical direction. It does not make report export confirmation a product policy or a required sample task.

User decides:

- Which material technical direction to take when architecture, dependency, authentication, security, privacy, retention, or compatibility is at stake.
- Whether the cost and reversibility of the choice fit the current task.

Agent may do:

- Inspect the existing design.
- Narrow options to a bounded recommendation with trade-offs.
- Continue read-only investigation while the direction is unresolved.

Agent must not imply:

- A strong recommendation is the same thing as user-owned technical judgment.
- A technical choice approves dependency installation, product-file writes, data-shape changes, sensitive-action approval, final acceptance, residual-risk acceptance, or `Write Authorization` by itself.

Owner links:

- [Core Model](../reference/core-model.md)
- [Agent Guide](agent-guide.md)

## Scope change

Scenario:

- The current scope is limited to `src/auth`, but the agent finds a helper path outside that boundary.

User decides:

- Whether the current scope may expand to the named helper path.
- Whether to keep the current scope, expand it narrowly, or convert the work to read-only investigation.

Agent may do:

- Name the exact path or behavior that appears necessary.
- Explain why the currently applied scope blocks the next safe action.
- Continue inspection inside current scope while writes outside scope remain blocked.

Agent must not imply:

- Scope expansion can be inferred from implementation convenience.
- Scope change also creates sensitive-action approval, final acceptance, residual-risk acceptance, evidence sufficiency, or `Write Authorization`.

Owner links:

- [Core Model](../reference/core-model.md)
- [Update-scope Method](../reference/api/method-update-scope.md)

## Verification criteria

Scenario:

- The task asks for clearer import errors, but no one has named what "clearer" means for this slice.

User decides:

- Which visible outcomes count as the criteria for checking the work.
- Whether the criteria are narrow enough for the requested slice or require a broader product review.

Agent may do:

- Propose criteria from current product behavior and ask the user to confirm or revise them.
- Use agreed criteria to guide checks and evidence gathering.
- Name any claim that still lacks supporting evidence.

Agent must not imply:

- Verification criteria are evidence, QA completion, final acceptance, residual-risk acceptance, sensitive-action approval, scope expansion, or `Write Authorization`.
- Meeting criteria in an agent summary alone makes the task close-ready.

Owner links:

- [Core Model](../reference/core-model.md)
- [Agent Guide](agent-guide.md)

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

- Sensitive-action approval is write approval, `Write Authorization`, final acceptance, residual-risk acceptance, verification criteria, or security authority.
- A broad "go ahead" approves unrelated installs, upgrades, deploys, secret printing, product decisions, or product-file writes.

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
- Artifact availability by itself creates evidence, verification criteria, final acceptance, residual-risk acceptance, QA, or close readiness.

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

- Present current scope, changed files or no-file outcome, checks, evidence coverage, agreed verification criteria, known gaps, and visible residual risk.
- Ask for final acceptance only when the result basis is visible enough for the user to judge.

Agent must not imply:

- "Looks good" is final acceptance unless that exact question was pending.
- Final acceptance supplies missing evidence, changes verification criteria, accepts residual risk, expands current scope, creates sensitive-action approval or `Write Authorization`, or accepts unrelated files.

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

- Residual-risk acceptance is final acceptance, evidence sufficiency, verification criteria satisfaction, QA, or proof that no risk remains.
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

- Summarize current scope, evidence, checks, known blockers, visible residual risk, and next safe action.
- Ask only the missing judgment that changes close readiness.

Agent must not imply:

- A status summary, passing checks, final acceptance alone, residual-risk acceptance alone, or chat text closes the task.
- A read-only close-readiness review creates evidence, final acceptance, residual-risk acceptance, or task state.

Owner links:

- [Core Model](../reference/core-model.md)
- [Close-task Method](../reference/api/method-close-task.md)
- [API blocker routing](../reference/api/blocker-routing.md)

## Ordinary implementation detail

Scenario:

- The agent needs to pick a local variable name or place a focused test beside an existing related test.

User decides:

- No new user judgment is normally apparent when the detail stays inside current scope and agreed verification criteria.
- Prior accepted product, technical, and scope decisions still provide the boundary.

Agent may do:

- Choose ordinary local details that follow project style.
- Escalate only if the detail changes user-visible behavior, public interfaces, privacy or security behavior, dependencies, current scope, sensitive action, verification criteria, final acceptance, or residual risk.

Agent must not imply:

- Every implementation detail needs a user decision.
- Agent-owned latitude can change product behavior, current scope, sensitive actions, verification criteria, final acceptance, or residual risk without asking.

Owner links:

- [Agent Guide](agent-guide.md)
- [Core Model](../reference/core-model.md)

## Where to go next

Use [Agent Guide](agent-guide.md) for operating patterns and [Scope](../reference/scope.md) when an example raises a baseline, profile-gated, or out-of-scope question. Use [Reference Index](../reference/README.md) for exact owner routing.
