# User guide

Harness is meant to let you work in ordinary language while keeping decision boundaries visible. You decide the work and the risky calls. Harness records scope, evidence, approvals, user judgment, and close basis. The agent must not present inference as if it were your decision.

This guide describes intended user behavior for a local Harness Server. This repository is documentation: runtime state, generated artifacts, evidence records, acceptance records, close records, and conformance output belong outside this documentation tree. Exact state, API, schema, storage, and security contracts are owned by the [Reference Index](../reference/README.md).

## Starting a task

Start the way you normally would:

```text
Help me make this plan concrete before implementation.
Add email login, but keep password reset and account creation out of scope.
Fix only typos in this document.
Show me what still blocks the first safe change.
Close this only if the evidence is sufficient.
```

You do not need to choose an internal mode or know API names before work begins. The agent should turn the request into a visible working shape before it acts.

You decide:
- The goal in ordinary language.
- The first important outcome you want.
- Any non-goals, path limits, or "ask me before..." rules.
- Whether the request is only advice, a small change, or work that needs tracking when that distinction matters to you.

Harness records:
- The current goal, scope, and non-goals.
- Known facts, unknowns, and pending user judgment.
- The next safe action.
- Whether the work shape is still too vague to start safely.

The agent must not:
- Make you learn schema names before helping.
- Treat a broad request for help as permission to write files.
- Infer product behavior, technical direction, final acceptance, or residual-risk acceptance from your opening request.
- Create one-off planning files or artifacts just because the task needs shaping.

## Keeping scope current

Scope changes when the goal, non-goals, affected area, acceptance criteria, allowed paths, or current work piece changes. Say the change plainly; the agent should update the visible record before relying on an old write check or old status.

You decide:
- Whether to expand, narrow, pause, cancel, or supersede the task.
- Whether a new file path, dependency, service, command, or user-visible behavior belongs in scope.
- Which acceptance criteria or non-goals should change.
- Whether a new question is yours to decide or a local implementation detail the agent may handle.

Harness records:
- The accepted scope update and the current boundary.
- The facts or user judgment that caused the update.
- Any old approval or write basis that is now stale.
- The next safe action under the updated scope.

The agent must not:
- Treat enthusiasm, "sounds good," or "go ahead" as a scope expansion unless that exact expansion was named.
- Reuse a write approval after the scope it depended on changed.
- Hide a new product direction, external dependency, service, migration, security choice, or public-interface change as a small implementation detail.
- Remove a non-goal without asking.

## Reviewing status

At any point, you can ask for the current status in plain terms:

```text
What is known, what is still blocked, and what can safely happen next?
```

You decide:
- Which pending decision to answer.
- Whether to continue, defer, narrow, cancel, or ask for more inspection.
- Whether the status gives you enough context to make the next judgment.

Harness records:
- The current scope, status, and next safe action.
- Facts the agent inspected and facts still unknown.
- Pending user judgment, approval needs, evidence gaps, and close blockers.
- Changed paths or a clear no-file-change result when relevant.

The agent must not:
- Mix inspected facts with user-owned judgment.
- Ask you to restate facts it can safely inspect.
- Present stale status as current.
- Treat passing tests, a summary, or its own confidence as final acceptance.

## Approving a write

A write approval is bounded permission for a named write attempt.

It is not:

- approval of the whole plan
- final acceptance
- residual-risk acceptance
- a guarantee that Harness can stop every tool before it acts

You decide:
- The specific write or set of writes you are allowing.
- The paths, commands, dependency changes, hosts, or external actions included in that approval.
- Whether a separate sensitive action is allowed, such as dependency installation, deployment, secret access, or destructive command use.
- What is explicitly not authorized.

Harness records:
- The intended write and the scope it is checked against.
- The approval limit, such as path, action, time, task, or single-use boundary.
- Whether the write is allowed, blocked, needs more user judgment, or needs separate sensitive-action approval.
- When the write basis is stale or no longer compatible with current scope.

The agent must not:
- Write outside the named scope.
- Treat write approval as product approval, final acceptance, residual-risk acceptance, or blanket permission for later writes.
- Treat sensitive-action approval as the same thing as product-file write approval.
- Claim stronger security behavior than the security owner supports; see [Security](../reference/security.md).

## Recording runs and evidence

After meaningful action, the agent should show what happened and what supports each important claim. Evidence is support for a claim; it is not your judgment.

You decide:
- Which user-visible result, product choice, technical choice, or risk you are judging.
- Whether to provide a manual observation or ask for more evidence.
- Whether a missing item is acceptable only through a separate close or residual-risk path.

Harness records:
- What ran or changed.
- Evidence references, including persisted artifacts when they are used.
- Which claim each evidence item supports.
- What passed, failed, was skipped, was not applicable, was redacted, was blocked, or is still missing.
- Whether required close evidence is sufficient.

The agent must not:
- Treat a staged artifact as persistent evidence until it is recorded and linked to a claim.
- Treat evidence as user judgment or final acceptance.
- Use a raw local path, copied log location, or "the file is over there" as evidence authority by itself.
- Expose raw secrets, tokens, or full sensitive logs.
- Claim that tests prove user-visible behavior, accessibility, security, privacy, or completeness beyond what they actually cover.

## Providing user judgment

User judgment is a choice that belongs to you. The agent may recommend a bounded option when the facts support one, but it must keep your decision separate from its inference.

You decide:
- Product behavior, UX, copy, user flow, accessibility trade-offs, and user-visible outcomes.
- Material technical direction, including architecture, dependencies, external services, authentication, migration, security, privacy, retention, compatibility, and public interfaces.
- Scope changes, final acceptance, residual-risk acceptance, cancellation, and supersession.
- Whether to defer a judgment and what may continue while it is deferred.

Harness records:
- The exact question asked.
- The available options and any bounded recommendation.
- Your answer and what it settles.
- What the answer does not settle.

The agent must not:
- Turn "go ahead," "approved," or "looks good" into every pending judgment.
- Collapse product judgment, technical judgment, scope judgment, sensitive-action approval, final acceptance, and residual-risk acceptance into one broad approval.
- Ask a vague multi-decision question when one narrow decision would unblock the next safe action.
- Decide a material user-owned choice silently.

## Reviewing close readiness

Before larger work is called done, ask for close readiness in ordinary language:

```text
Show what changed, what was checked, what residual risk is visible, and what still blocks close.
```

You decide:
- Which blocker to address next.
- Whether to provide final acceptance when the close basis is visible.
- Whether a named residual risk is acceptable when the active close path requires that judgment.
- Whether the task should complete, cancel, or be superseded.

Harness records:
- Whether scope stayed in bounds.
- Required and optional evidence support.
- Pending user judgment, final acceptance need, and residual-risk need.
- Known blockers and the next action that would unblock close.
- A read-only close-readiness check when the user only asks whether close would be blocked.

The agent must not:
- Call the task done while required scope, evidence, user judgment, final acceptance, residual-risk handling, or close blockers remain unresolved.
- Treat passing tests as close readiness.
- Treat a read-only close review as if it changed task state.
- Hide a UI inspection gap, evidence gap, security concern, or unresolved user decision behind a generic "done."

## Closing or accepting residual risk

Closing and accepting residual risk are separate user judgments. Final acceptance means you accept the visible result. Residual-risk acceptance means you accept a named remaining risk that is still visible.

You decide:
- Whether the task should complete, cancel, or be superseded.
- Whether you accept the named final result.
- Whether you accept a named residual risk, including its affected area and consequence.
- Whether missing required evidence must be gathered instead of accepted as risk.

Harness records:
- The close attempt and its outcome.
- The final acceptance basis when completion needs it.
- The named residual risk, its scope, its consequence, and the reason it remains.
- Any blocker that prevents honest close.

The agent must not:
- Use residual-risk acceptance to cover missing required evidence.
- Treat "looks good" as residual-risk acceptance unless the risk was named and you were asked that exact question.
- Present cancelled or superseded work as successful completion.
- Turn a failed command, missing artifact, stale evidence, or unresolved blocker into a vague failed task result.

## What Harness does not guarantee

Harness makes scope, judgment, evidence, and close basis visible. That visibility is useful, but it is not the same thing as enforcement or proof. For canonical security non-claims and guarantee levels, see [Security](../reference/security.md).

You decide:
- Whether to continue when Harness authority, evidence, or observable surface information is unavailable.
- Whether a weaker record is acceptable for the task at hand.
- Whether to move to a safer surface, gather more evidence, or stop.

Harness records:
- The facts it can honestly observe or record.
- Missing, stale, redacted, blocked, or unsupported evidence.
- Whether the agent is working with a cooperative record rather than a proven stronger control.
- Known limits that affect write, evidence, judgment, or close claims.

The agent must not:
- Imply stronger security, observation, or prevention than the relevant owner and active surface support.
- Claim Harness proves correctness, completeness, accessibility, privacy, security, or production safety by itself.
- Turn "Harness recorded this" into "the user accepted this."

## Where to go next

This guide is the user-facing workflow. Continue by role instead of starting with internal schema details.

| Reader | Path |
|---|---|
| Working user | [Judgment Examples](judgment-examples.md) -> [Scope](../reference/scope.md) |
| Agent author/operator | [Agent Guide](agent-guide.md) -> [Agent Integration Reference](../reference/agent-integration.md) |
| Implementer | [Reference Index](../reference/README.md) -> active scope -> API methods -> schema owners -> storage effects |
| Documentation maintainer | [Authoring Guide](../maintain/authoring-guide.md) -> [Translation Guide](../maintain/translation-guide.md) -> [Checks](../maintain/checks.md) |

Use the [Reference Index](../reference/README.md) when you need exact method behavior, schema shape, storage effect, error behavior, security wording, or close-readiness rules. Do not treat this user guide as the API contract, and do not copy detailed contract rules back into the user-facing path.

Reading these docs does not create runtime state, evidence, acceptance, close records, or implementation authority.
