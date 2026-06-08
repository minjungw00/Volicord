# User Guide

Use normal language. Harness should help the agent keep scope, user-owned judgment, evidence, close readiness, and residual risk visible without making you learn internal presentation terms first.

This guide describes intended active user behavior for a future local Harness Server. This repository is still documentation-only; no Harness runtime/server implementation exists here yet. Exact state, API, schema, storage, and template contracts are owned by the [Reference Index](../reference/README.md).

## 1. Start in ordinary language

You should be able to ask for careful work in everyday terms:

```text
Help me make this plan concrete before implementation.
Ask me before deciding product behavior.
Before changing files, tell me what you expect to touch.
Show what you verified before calling it done.
Tell me what still blocks close.
```

You can set scope the same way:

```text
Add email login, but keep password reset and account creation out of scope.
Fix only typos in this document.
Keep this as a small copy edit unless it becomes a product or technical decision.
```

The agent should translate that into visible working context: the current goal, what is in scope, what is out of scope, what is already known, what only you can decide, what can safely happen next, and what would block close.

Harness may classify ordinary requests as advice/review/planning without product writes, a small direct change, or tracked work. You do not need to choose or say `advisor`, `direct`, `work`, or `auto`; `auto` is only an API input that asks for classification, not a task label shown to you.

You do not need internal Harness labels or API method names before work can begin. The agent can explain exact routing only after the natural request is clear.

## 2. Clarify scope before write

Before product files change, the agent should make the working scope plain:

- what you asked for and what the current goal is
- the smallest useful result
- what may change
- what is out of bounds
- likely paths, commands, tools, or external actions
- facts the agent already verified
- facts still unknown
- decisions that belong to you
- the next safe action

For anything larger than a tiny obvious edit, the agent should inspect available files, tests, docs, current Harness state, and accepted judgments before asking you. If the answer is discoverable, the agent should discover it. If the work is still too vague to implement safely, the agent should name the ambiguity and ask the one question that changes the next safe action.

In a Harness-connected session, product writes go through a pre-write scope check. In owner terms, the active method is `harness.prepare_write`. A compatible result says the intended write matches current Harness state and active surface capability for the named operation. It is not OS permission, sandboxing, tamper-proof enforcement, arbitrary-tool isolation, or proof that every tool was blocked before action.

If you decide to change the goal, scope boundary, non-goals, acceptance criteria, autonomy boundary, baseline, or active work piece after `harness.intake`, the agent should apply that through `harness.update_scope` before relying on a write check. Existing pre-write checks that no longer match the updated scope become stale.

## 3. Separate facts from user judgment

Facts are things the agent can inspect, verify, or cite. User judgments are choices you own.

| Item | Agent should do | It must not do |
|---|---|---|
| Repository facts | Read files, tests, docs, config, history, or current state when available. | Ask you to restate facts it can safely inspect. |
| Evidence | Show what supports a claim and what is missing. | Treat evidence as your decision or final acceptance. |
| Product judgment | Ask you about product behavior, copy, UX, user flow, and accessibility trade-offs. | Pick a material product direction silently. |
| Technical judgment | Ask you about architecture, dependency, migration, interface, security, privacy, retention, or compatibility choices that matter. | Hide a material technical decision as an implementation detail. |
| Scope judgment | Ask before expanding scope or removing a non-goal, then apply the accepted scope change through `harness.update_scope`. | Treat enthusiasm as permission to broaden the task or as write approval. |
| Close judgment | Ask for final acceptance or residual-risk acceptance only when the close basis is visible. | Turn "looks good" into every pending judgment. |

Harness preserves this boundary so the agent can recommend without replacing your judgment.

## 4. Ask one narrow judgment at a time

A useful judgment request asks for one decision and fits on one screen when possible. It should include:

- the exact decision
- realistic options
- a recommendation when helpful
- why the recommendation is reasonable
- uncertainty
- what can continue if you defer
- what remains blocked
- what the answer does not settle

Example:

```text
Judgment needed: choose failed-login feedback.

Options:
- Inline message near the form.
- Toast after failed submit.
- Modal that interrupts the flow.

Recommendation: inline message. It stays visible and fits the form context.

If deferred: backend error mapping can continue, but final UI behavior, copy, screenshots, and user-visible inspection remain unresolved.

Does not settle: login architecture, account recovery, final acceptance, or residual-risk acceptance.
```

A broad reply such as "go ahead," "approved," or "looks good" applies only to the one active, clearly named judgment. It does not automatically grant sensitive-action approval, accept the final result, change scope, cancel the task, accept residual risk, or settle another future judgment candidate.

The compact active path asks through `harness.request_user_judgment` and records the answer through `harness.record_user_judgment` when a judgment is needed. Ordinary decisions do not require a special presentation format.

## 5. Treat sensitive action approval separately

Sensitive action approval is permission for a named action, not approval of the whole plan.

Examples include installing or updating dependencies, running commands with destructive potential, touching secrets, accessing restricted systems, deploying, sending network requests outside the current scope, or making a change with privacy/security impact.

A good sensitive-action prompt should name:

- the exact action
- the reason it is needed
- the scope and time window
- what command, tool, host, dependency, secret, or path is covered
- what safer alternative exists, if any
- what the approval does not settle

Approving a dependency install does not mean the dependency is the right architecture. Approving a deployment command does not mean the product result is accepted. Approving secret access does not accept residual risk. Each of those needs its own judgment when relevant.

## 6. Record evidence after meaningful action

After meaningful work, the agent should summarize what happened and what supports the claim. In owner terms, the active path may use `harness.record_run` and evidence references when that path is available.

Useful evidence can include changed paths, diffs, command output, test results, screenshots, logs, source links, and inspection notes. If a future owner promotes a separate user-visible inspection path, its notes remain separate later-path material. The summary should say:

- what ran or changed
- what claim each item supports
- what passed, failed, was skipped, or was not applicable
- what evidence is missing, stale, redacted, omitted, blocked, or insufficient
- what was not verified

Evidence does not replace your judgment or final acceptance. Tests do not replace user-visible inspection or any future promoted review path. A screenshot does not prove accessibility. A generated summary does not become operational truth. Raw secrets, tokens, and full sensitive logs should be redacted, omitted, blocked, or represented by safe handles.

## 7. Review blockers before close

Before larger work is called done, ask:

```text
Show what changed, what was checked, what residual risk is visible, and what still blocks close.
```

The agent should show:

- whether scope stayed in bounds
- user judgments already made
- unresolved user judgments
- changed paths or no-file result
- evidence supporting important completion claims
- checks and their status
- user-visible inspection result when relevant
- final acceptance need
- residual risk visibility and acceptance need
- the smallest unblocker

Tests can pass while close is still blocked. A UI change can still have a visible inspection gap. A security-sensitive change can need a risk decision. Missing evidence remains a blocker until it is gathered or honestly reported as unresolved.

In owner terms, `harness.close_task` returns blockers or a close result. In user terms, the agent should not claim close while required scope, evidence, user judgment, final acceptance, residual-risk handling, or close blockers remain unresolved. Separate quality routes are later candidates unless a future owner promotes them.

Close can end as completed, cancelled, or superseded. A failed command, failed derived view, missing artifact, evidence gap, or unresolved blocker should stay visible as the specific problem; it is not a generic failed Task result.

## 8. Accept final result separately from residual risk

Final acceptance means you accept the result you can see. Residual-risk acceptance means you accept a named residual risk that is still visible. They are separate judgments.

The agent should ask for final acceptance only after the close basis is visible: scope, result, evidence, checks, known gaps, user-visible inspection status when relevant, and blockers. The prompt should name exactly what result you are accepting.

The agent should ask for residual-risk acceptance only when a known residual risk is visible and the active close path requires that judgment. The prompt should name the risk, affected area, consequence, evidence gap or uncertainty, and any safer alternative.

"Looks good" may be final acceptance only when the agent has clearly asked for final acceptance of a named result. It is not residual-risk acceptance unless the risk was named and the prompt asked for that judgment.

## 9. Read current MVP guarantees honestly

Early Harness behavior is cooperative by default. Limited detective reporting applies only to supported observable facts after the relevant capability check has passed, unless a specific owner-documented mechanism proves more.

| Guarantee level | What it means | What it does not mean |
|---|---|---|
| Cooperative | The agent is instructed to hold, ask, refresh, or proceed through the Harness record path. | Harness is not automatically stopping every tool at the OS level. |
| Detective | Harness or a surface can report a mismatch after observing a supported fact and passing the relevant capability check. | The action was not necessarily blocked before it happened, and unsupported command, network, secret, or native-capture effects were not verified. |
| Preventive | A specific proven mechanism blocks a covered action before it occurs. | Do not assume this unless the exact mechanism and operation are named. |
| Isolated | A documented separation boundary exists. | Do not assume broader OS sandboxing, arbitrary-tool isolation, or tamper-proof storage. |

Common status messages should be read plainly:

| Message | Meaning |
|---|---|
| Harness/Core authority is unavailable. | The agent cannot confirm current Harness state, judgments, evidence, write compatibility, final acceptance, residual-risk acceptance, or close readiness. |
| Current state may be stale. | The agent should refresh before relying on state, baseline, readable view, or pre-write scope check. |
| Out of scope. | The agent should narrow the action or ask whether you want to change scope. |
| User judgment needed. | A specific user-owned decision blocks the affected action. |
| Evidence insufficient. | The claim needs more support before it can be relied on. |
| Close blocked. | The work cannot honestly be closed yet. |

If an agent says something is blocked, read that as "we cannot honestly proceed or close under the current Harness record" unless it also names a proven preventive control.

## 10. Examples

### Build a login feature

```text
User: Build a login feature.

Good agent behavior:
- inspect existing auth routes, session handling, login UI, tests, and docs first;
- separate scope, non-goals, facts, unknowns, and user-owned choices;
- propose the smallest safe login slice before writing.

Visible shaped result:
- Goal: add a basic email/password login path.
- Scope: login form, submit handling, session creation, and focused tests.
- Out of scope: account creation, password reset, social login, and production deployment.
- Acceptance: existing users can sign in, failed login shows a chosen error treatment, and tests cover the touched path.
- Blocking question: should failed login feedback be inline text or a toast?
- Next safe action: after that answer, update the active scope and check the intended login-form write.

Harness routing after the ordinary request is clear:
- `harness.intake`: turn "login feature" into the smallest useful goal, non-goals, acceptance, evidence expectation, and blocker.
- `harness.request_user_judgment`: ask the failed-login feedback choice instead of choosing silently.
- `harness.record_user_judgment`: record the answer to that one product choice.
- `harness.update_scope`: apply the accepted slice before writing.
- `harness.prepare_write`: check the intended login form, session, and test writes against the active scope.
- `harness.record_run`: summarize changed paths, tests, and missing evidence after meaningful action.
- `harness.close_task`: show evidence, blockers, final acceptance need, and any named residual risk before close.
```

### Make a plan concrete

```text
User: Make this plan more concrete before implementation.

Good agent behavior:
- read the plan and related docs before asking anything;
- identify the first writable slice, non-goals, acceptance criteria, evidence expectation, and close blockers;
- ask only the decision that changes the first safe slice.

Visible shaped result:
- Goal: turn the plan into one implementable first change.
- Scope: clarify the current objective, affected areas, acceptance criteria, and evidence expectation.
- Out of scope: new standalone artifacts, broad roadmap rewrite, and implementation.
- Evidence gap: no repository files or tests have been checked yet.
- Blocking question: which user outcome should the first slice prove?
- Next safe action: inspect the named owner docs or files, then update the active scope for the first implementable slice.

Harness routing after the ordinary request is clear:
- `harness.intake`: keep this as planning until the first implementable slice is specific.
- `harness.request_user_judgment`: ask only the choice that changes that slice.
- `harness.record_user_judgment`: record the chosen user outcome when the user decides.
- `harness.update_scope`: apply the chosen slice before write-capable work.
- `harness.prepare_write`: wait until implementation paths are specific.
- `harness.record_run`: record inspections or checks only after they happen.
- `harness.close_task`: do not call the plan done if the first slice, evidence expectation, or close blocker is still unresolved.
```

### Only change files inside this scope

```text
User: Only change files inside `src/auth` and its tests.

Good agent behavior:
- treat the listed paths as the allowed boundary;
- inspect enough to know whether the requested fix fits that boundary;
- ask before touching any path outside it.

Harness routing after the ordinary request is clear:
- `harness.intake`: convert the path instruction into scope and non-goals.
- `harness.request_user_judgment`: ask if the needed fix requires a path outside the boundary.
- `harness.record_user_judgment`: record the user's scope answer.
- `harness.update_scope`: apply any accepted boundary change before relying on it.
- `harness.prepare_write`: check intended writes against the allowed paths.
- `harness.record_run`: report changed paths and focused checks.
- `harness.close_task`: name out-of-scope needs or missing checks as blockers instead of hiding them.
```

### Split dependency choice from install approval

```text
User: This dependency choice needs my decision before you install anything.

Good agent behavior:
- inspect current charting/data-display needs first;
- ask the technical judgment about whether a new dependency is the right direction;
- ask a separate sensitive-action approval before installing or updating packages.

Harness routing after the ordinary request is clear:
- `harness.intake`: separate the product need from the dependency path.
- `harness.request_user_judgment`: ask the `technical_decision` about dependency direction, then ask separate `sensitive_approval` before installing.
- `harness.record_user_judgment`: record each answer as its own judgment.
- `harness.update_scope`: update scope only if the accepted decision changes the work.
- `harness.prepare_write`: check manifest or lockfile writes before package changes.
- `harness.record_run`: record install/check output after the approved action.
- `harness.close_task`: keep final acceptance and residual-risk acceptance separate from the dependency decision.
```

### Close honestly

```text
User: Check whether this change can be closed.

Good agent behavior:
- show scope, evidence, checks, user-visible inspection status when relevant, final acceptance need, and residual risk;
- name blockers before close;
- ask final acceptance and residual-risk acceptance separately when both are relevant.

Harness routing after the ordinary request is clear:
- `harness.intake`: confirm what result is being evaluated for close.
- `harness.request_user_judgment`: ask only unresolved `final_acceptance` or named `residual_risk_acceptance` questions.
- `harness.record_user_judgment`: record those answers separately when the user gives them.
- `harness.update_scope`: update scope only if the close check reveals accepted scope drift.
- `harness.prepare_write`: do not reuse stale write checks as close evidence.
- `harness.record_run`: rely on recorded actions and evidence summaries, or name the gap.
- `harness.close_task`: return blockers or a close result; broad "looks good" does not settle every judgment.
```

For compact judgment prompt patterns, see [Judgment Examples](judgment-examples.md).
