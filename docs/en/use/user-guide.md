# User Guide

Harness lets you work in ordinary language while keeping decision boundaries visible. Harness is the local work-authority product/system; Core is the local authority record for Harness state. You decide the work and the risky calls. The agent should keep scope, judgment, evidence, approvals, and close basis separate instead of presenting inference as your decision.

This guide is the user workflow path. Exact API behavior, schemas, storage effects, security wording, and reference-level close readiness rules live in the owners linked from the [Reference Index](../reference/README.md).

## Start a task

Start the way you normally would:

```text
Help me make this plan concrete before implementation.
Add email login, but keep password reset and account creation out of scope.
Fix only typos in this document.
Show me what still blocks the first safe change.
Close this only if the evidence is sufficient.
```

You do not need internal mode names or API names. The agent should turn the request into a visible work boundary before it acts.

You decide:

- the goal in ordinary language
- the first important outcome
- non-goals, path limits, or "ask me before..." rules
- whether the request is advice, a small change, or tracked work when that distinction matters

The agent should show:

- current goal, current scope, and non-goals
- known facts, unknowns, and pending user-owned judgment
- the next safe action
- whether the request is still too vague to start safely

The agent should not treat a broad request for help as permission to write files, infer product behavior, infer final acceptance, or create one-off planning artifacts just because the task needs shaping.

## Keep scope current

Scope changes when the goal, non-goals, affected area, verification criteria, allowed paths, or current work slice changes. Say the change plainly. The agent should refresh the visible boundary before relying on old status or old write approval.

You decide:

- whether to expand, narrow, pause, cancel, or supersede the task
- whether a new path, dependency, service, command, migration choice, or user-visible behavior belongs in scope
- which verification criteria or non-goals should change
- whether a new question is yours to decide or a local implementation detail

The agent should show the accepted boundary, the reason it changed, any stale approval or status, and the next safe action under the updated scope.

The agent should not treat "sounds good" or "go ahead" as scope expansion unless the exact expansion was named.

## Review status

At any point, you can ask:

```text
What is known, what is still blocked, and what can safely happen next?
```

You decide which pending decision to answer and whether to continue, defer, narrow, cancel, or ask for more inspection.

A useful status summary says:

- current `Task` or work boundary
- current scope
- inspected facts and unknowns
- primary blocker
- pending user judgment or approval need
- evidence gaps and close blockers when relevant
- one next safe action

The agent should not mix inspected facts with user-owned judgment, ask you to restate facts it can safely inspect, present stale status as current, or treat passing tests as final acceptance.

## Approve writes and sensitive actions

A user-facing write approval is bounded permission for a named write attempt. In this guide, write approval means ordinary user approval for a write flow; it is separate from the exact product label `Write Authorization`.

Write approval is not whole-plan approval, final acceptance, residual-risk acceptance, sensitive-action approval, or a guarantee that Harness can prevent every unsafe action.

You decide:

- the specific write or set of writes you allow
- paths, commands, dependency changes, hosts, or external actions included in that approval
- whether a separate sensitive action is allowed, such as dependency installation, deployment, secret access, or destructive command use
- what is explicitly not authorized

The agent should show the intended write, the current scope checked for that write, the approval limit, whether a separate sensitive-action approval is needed, and whether the approval basis has gone stale.

The agent should not write outside the named scope, treat sensitive-action approval as product-file write approval, or claim stronger security behavior than [Security](../reference/security.md) supports.

## Provide user-owned judgment

User-owned judgment is a choice that belongs to you. The agent may recommend a bounded option when the facts support one, but it must keep your decision separate from its inference.

You decide:

- product behavior, UX, copy, user flow, accessibility trade-offs, and user-visible outcomes
- material technical direction, including architecture, dependencies, external services, authentication, migration, security, privacy, retention, compatibility, and public interfaces
- scope changes, final acceptance, residual-risk acceptance, cancellation, and supersession
- whether to defer a judgment and what may continue while it is deferred

The agent should ask the exact question, present concise options, name any bounded recommendation, record what your answer settles, and state what remains unsettled.

The agent should not turn "approved" into every pending judgment or combine product judgment, technical judgment, scope judgment, sensitive-action approval, final acceptance, and residual-risk acceptance into one broad approval.

For examples, see [Judgment Examples](judgment-examples.md). For exact authority boundaries, see [Core Model](../reference/core-model.md).

## Use evidence without replacing judgment

After meaningful action, the agent should show what happened and what supports each important claim. Evidence is support for a claim; it is not your judgment.

You decide:

- which visible result, product choice, technical choice, or risk you are judging
- whether to provide a manual observation or ask for more evidence
- whether a missing item must be gathered rather than accepted as risk

The agent should show what ran or changed, which claim each evidence item supports, what passed or failed, what is missing or stale, and which claim remains unsupported.

The agent should not treat a staged artifact, raw local path, copied log location, screenshot alone, generated summary, or test pass as broader proof than it is. It also should not expose raw secrets, tokens, or full sensitive logs.

## Review close readiness

Before larger work is called done, ask in ordinary language:

```text
Show what changed, what was checked, what residual risk is visible, and what still blocks close.
```

For users, close readiness means whether the task can honestly finish now. In reference terms, close readiness meaning belongs to [Core Model](../reference/core-model.md), and close method behavior belongs to [Close-task Method](../reference/api/method-close-task.md).

You decide:

- which blocker to address next
- whether to provide final acceptance when the close basis is visible
- whether to accept a named residual risk when the applicable close path requires that judgment
- whether the task should complete, cancel, or be superseded

The agent should show scope, evidence, checks, pending judgments, final-acceptance needs, residual-risk needs, known blockers, and the next action that would unblock close.

The agent should not call the task done while required scope, evidence, user judgment, final acceptance, residual-risk handling, or close blockers remain unresolved.

## Close or accept residual risk

Closing and accepting residual risk are separate user judgments. Final acceptance means you accept the visible result. Residual-risk acceptance means you accept a named remaining risk that is still visible.

You decide:

- whether the task should complete, cancel, or be superseded
- whether you accept the named final result
- whether you accept a named residual risk, including its affected area and consequence
- whether missing required evidence must be gathered instead of accepted as risk

The agent should not use residual-risk acceptance to cover missing required evidence, treat "looks good" as risk acceptance unless the risk was named, or present cancelled or superseded work as successful completion.

## Use reference owners for contract detail

Use the use docs for workflow. Use owner reference docs for exact contracts:

| Need | Owner Route |
|---|---|
| Baseline and out-of-scope boundary | [Scope](../reference/scope.md) |
| Core authority, user-owned judgment, close readiness meaning | [Core Model](../reference/core-model.md) |
| Security wording and guarantee levels | [Security](../reference/security.md) |
| API methods and schemas | [Reference Index](../reference/README.md) |
| Surface and connector behavior | [Agent Integration Reference](../reference/agent-integration.md) |

Do not treat this guide as the API contract. Do not copy detailed contract rules back into the user-facing path.

## Where to go next

| Reader | Path |
|---|---|
| Working user | [Judgment Examples](judgment-examples.md) -> [Scope](../reference/scope.md) |
| Agent author or operator | [Agent Guide](agent-guide.md) -> [Agent Integration Reference](../reference/agent-integration.md) |
| Implementer | [Reference Index](../reference/README.md) -> baseline scope -> API methods -> schema owners -> storage effects |
| Documentation maintainer | [Authoring Guide](../maintain/authoring-guide.md) -> [Translation Guide](../maintain/translation-guide.md) -> [Checks](../maintain/checks.md) |
