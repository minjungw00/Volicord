# Agent Session Flow

## What this document helps you do

This document describes how an agent session should behave for users. It is procedural: what to show, when to ask, when to continue, and when to stop.

It does not define connector contracts, full capability profiles, MCP schemas, or surface cookbooks. Those belong in [Agent Integration Reference](../reference/agent-integration.md) and [Surface Cookbook](../reference/surface-cookbook.md).

## Read this when

Read this when checking how the agent should present status, blockers, writes, checks, and close.

## Before you read

Read [User Guide](user-guide.md) first if you want the user-facing version.

## Main idea

Show only the state, blocker, judgment, and next action that affect the user's next decision.

## Session start

When Harness is connected, start with status or intake when the user asks for work that should be tracked by Harness, or explicitly asks to use Harness. The user does not need to say "Harness." Infer from the request shape and keep the first response short.

Track ordinary-language requests when their shape suggests scope, judgment, evidence, or close state should stay visible:

- product writes or state-changing project work
- scope drift risk or ambiguous requirements
- multi-file, structural, migration, or cross-boundary work
- sensitive or policy-relevant areas such as auth, security, billing, destructive/data-loss risk, privacy, compliance, accessibility, or design quality
- user-owned product judgment, technical direction with material cost, compatibility, security, maintenance, migration, interface, or risk impact, or trade-off decisions
- evidence, verification, Manual QA, acceptance, or residual-risk needs

Keep small direct tasks light. Do not add ceremony just to answer a question, inspect code, explain a result, or handle a tiny low-risk change with an already narrow shape.

Show:

- the likely mode: `advisor`, `direct`, or `work`
- the current or proposed scope
- what is out of bounds
- the next safe action
- any question that blocks progress

Do not begin product writes from a broad natural-language request alone. First establish scope and compatible write authority for the intended change.

## Resume

Before significant work resumes, read Harness state and show the current position. Do not reconstruct authority from old chat when state is available.

A good resume response says:

```text
I found the active task. Current scope is X. The next safe action is Y. Product writes are not authorized yet. One decision is pending: Z.
```

If projection or readable status is stale, say that and refresh or reconcile before depending on it.

## Intake

Intake turns an everyday request into a usable task shape without forcing the user to speak Harness.

Listen for the same task-shape triggers used at session start: product writes, scope drift risk, ambiguous requirements, multi-file or structural work, sensitive or policy-relevant areas, user-owned judgment, and evidence, verification, Manual QA, acceptance, or residual-risk needs. When one appears, translate the ordinary request into a proposed mode, scope, out-of-bounds area, and next safe action.

Ask only questions that change the next safe action. Prefer one blocking question with a recommendation instead of a long form.

Before asking, inspect repo, codebase, docs, and Harness state that are available and current for answers the agent can discover safely. Do not ask the user to restate existing file paths, behavior, terminology, or constraints that are already visible from current context. If a source is unavailable or stale, say so rather than relying on it as authority.

One blocking question at a time does not mean one clarification round total. Broad or design-heavy requests may need several short turns until the goal, scope, non-goals, acceptance criteria, affected product areas, user-facing screens or flows, modules, interfaces, sensitive categories, user-owned trade-offs, verification or Manual QA expectations, and known product, implementation, verification, QA, or follow-up risks are shaped enough to propose the first safe Change Unit.

Each blocking question should name the uncertainty, offer realistic options, include the agent's recommendation, and say what can continue if the decision is deferred, or why nothing should continue until the decision is made. Record assumptions the agent makes separately from choices, approvals, QA judgment, acceptance, or risk acceptance that belong to the user.

Good intake:

```text
I can treat this as direct if the change stays inside the settings copy. If it also changes account behavior, it becomes work. Recommendation: start direct with settings copy only. Is that the intended scope?
```

## Classify as advisor/direct/work

Use `advisor` for reading, explaining, comparing, and reviewing without product writes.

Use `direct` for small, low-risk work with a narrow scope. Direct work still needs active scope and write authority before product writes, but it should stay light.

Use `work` for feature work, structural changes, risky fixes, multi-file changes, unclear requirements, or anything that needs meaningful evidence and independent verification.

If a direct task grows, move the same task to `work` and show why.

## Scope and Change Unit

Before product writes, shape the active scope into a Change Unit. The user-facing explanation should answer:

- included behavior or files
- out-of-bounds behavior or files
- completion conditions
- known sensitive areas
- when the agent must stop and ask

Enough is known to propose the first safe Change Unit when the agent can state those items without hiding unresolved user judgment. If that cannot be done yet, continue intake with the next blocking question or propose a smaller Change Unit that avoids the unresolved area.

Autonomy Boundary is not write authority. It only describes what judgment the agent may exercise without asking again. Actual product writes still require a compatible write check.

## Blocking user-owned judgment

When product judgment or a user-owned technical choice blocks progress, show or request a Decision Packet. Do not replace it with broad approval or a vague "continue?" prompt.

A user-facing Decision Packet should include:

- why the decision is needed now
- the exact question
- options and trade-offs
- recommendation and uncertainty
- what can continue if the decision is deferred, or why nothing should continue until it is made
- residual risk or follow-up

Useful examples:

- Failed-login UX: compare inline message, toast, and modal/layer; recommend one based on flow, accessibility, interruption, and copy risk. If deferred, backend auth work may continue, but the final failed-login experience should not be claimed done.
- Failed-login copy: compare terse security-focused wording, plain recovery wording, and more specific field-level guidance; recommend one based on account enumeration risk, clarity, support burden, and product tone. If deferred, validation wiring may continue, but release-ready copy and Manual QA should stay open.
- Product taste and Manual QA need: compare a polished interaction that needs human visual review with a simpler conservative behavior that can be checked by tests and browser smoke. Explain the taste trade-off, QA cost, user impact, and what can continue if Manual QA is deferred, or why nothing should continue until the decision is made.
- Session approach: compare session auth, token auth, and social login; explain revocation, CSRF/XSS exposure, client compatibility, operational complexity, and migration cost. If deferred, form scaffolding may continue only if it does not commit to the session model.
- Dependency or migration choice: compare adding a dependency, using existing utilities, or postponing the capability; for schema/data-model migration, compare additive migration, compatibility shim, and breaking cleanup. Explain blast radius, rollback, test boundary, and maintenance cost.
- Public API/interface or module boundary: compare preserving the current interface, adding a narrow extension, or moving responsibility across a module boundary. Explain caller impact, compatibility risk, boundary tests, and future-change cost.
- Security-sensitive change: approval to access a secret, change permissions, or export data is only an approval boundary. Separate product or security judgment may still be needed for roles, fields, redaction, audit logging, retention, rollback, and user notice.
- QA or verification waiver: name the skipped check, accepted risk, and follow-up. Example: waive mobile Safari Manual QA for a copy-only change, accept wrapping risk, and keep a browser pass as release follow-up.
- Residual-risk acceptance before close: show the remaining limitation, the evidence that does exist, why close can still be acceptable, and the follow-up that remains.

Ask one blocking question at a time when possible.

## Product writes

Before writing product files, the agent must check write authority for the intended operation.

Show a short Write Authority Summary:

```text
Write authority: allowed for src/auth/login.ts and tests/auth/login.test.ts
Scope basis: email login Change Unit
Limitation: cooperative surface; changed-path validation detects violations after the fact
```

If write authority is blocked, unavailable, stale, or incompatible with the intended change, hold product writes and explain the smallest unblocker.

Documentation-maintenance edits are a separate docs-only workflow. They are governed by
[Authoring Guide](../maintain/authoring-guide.md), not by the product-write flow described here.

## Evidence and checks

After advice, changes, runs, or review, record the result at the right level of detail. User-facing evidence should map to acceptance criteria or the stated task goal.

Good evidence display:

```text
Evidence:
- AC-01: login form renders with email field, supported by test run RUN-008.
- AC-02: failed login message still needs Manual QA.
```

When evidence is missing, name the criterion or claim that lacks support. Do not say only "evidence gate failed."

## Verification, Manual QA, residual risk, acceptance

Keep these separate in the agent response.

Verification answers how the work was technically checked. Same-session self-review is useful, but it is not detached verification.

Manual QA answers whether a person inspected qualities that need human judgment, such as UX, workflow, visual result, copy, or accessibility interpretation.

Residual risk is a known remaining limitation, uncertainty, unchecked condition, or trade-off. It must be visible before risk-accepted close or final acceptance.

Final acceptance is the user's acceptance of the result when the task path requires it. It is not the same as approval, verification, QA, or residual-risk acceptance.

## Close

Close only when blockers are clear for the active task path.

Before successful close, show or confirm:

- scope match
- evidence coverage or no evidence requirement
- verification status or accepted verification risk
- Manual QA status or valid waiver
- close-relevant residual risk shown or `ResidualRiskSummary.status=none`
- final acceptance recorded when required

If close is blocked, say exactly why and name the smallest next unblocker.

## Good and bad agent response examples

Good start:

```text
This looks like a small task Harness should track lightly. Current mode looks like direct because the requested change is one copy string. Scope: settings page label only. Out of bounds: account behavior and API changes. Next safe action: check write authority for that file, then make the edit and self-check.
```

Bad start:

```text
Approved, I will update whatever is needed and close the task.
```

Good decision stop:

```text
Blocked on product judgment: the empty state can either invite setup now or keep the page quiet until data exists. Recommendation: invite setup now, low uncertainty. If you defer, I can finish the data-loading refactor but not the empty-state UX.
```

Bad decision stop:

```text
Need approval to continue.
```

Good close block:

```text
Close is blocked by Manual QA for the onboarding copy and missing AC-02 evidence. Smallest unblocker: run the browser smoke check and record whether the copy is acceptable.
```

Bad close block:

```text
The state model does not satisfy gates.
```

Good write hold:

```text
I am holding product writes because write authority is not available for the new auth files. Smallest unblocker: update scope to include those files or reduce the change to the existing login module.
```

Bad write hold:

```text
Harness is unhappy, so I stopped.
```
