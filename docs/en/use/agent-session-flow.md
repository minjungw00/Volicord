# Agent Session Flow

## What this document helps you do

This document describes how an agent session should behave for users. It is procedural: what to show, when to ask, when to continue, and when to stop.

It does not define connector contracts, full capability profiles, MCP schemas, or surface cookbooks. Those belong in [Reference: Agent Integration](../reference/agent-integration.md) and related reference documents once the reference path is created.

## Read this when

Read this when checking how the agent should present status, blockers, writes, checks, and close.

## Before you read

Read [User Guide](user-guide.md) first if you want the user-facing version.

## Main idea

Show only the state, blocker, judgment, and next action that affect the user's next decision.

## Session start

When the user asks to run work under Harness, start with status or intake. Keep the first response short.

Show:

- the likely mode: `advisor`, `direct`, or `work`
- the current or proposed scope
- what is out of bounds
- the next safe action
- any question that blocks progress

Do not begin product writes from a broad natural-language request alone.

## Resume

Before significant work resumes, read Harness state and show the current position. Do not reconstruct authority from old chat when state is available.

A good resume response says:

```text
I found the active task. Current scope is X. The next safe action is Y. Product writes are not authorized yet. One decision is pending: Z.
```

If projection or readable status is stale, say that and refresh or reconcile before depending on it.

## Intake

Intake turns an everyday request into a usable task shape.

Ask only questions that change the next safe action. Prefer one blocking question with a recommendation instead of a long form.

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

Autonomy Boundary is not write authority. It only describes what judgment the agent may exercise without asking again. Actual product writes still require a compatible write check.

## Blocking product judgment

When product judgment blocks progress, show or request a Decision Packet. Do not replace it with broad approval.

A user-facing Decision Packet should include:

- why the decision is needed now
- the exact question
- options and trade-offs
- recommendation and uncertainty
- what can continue if the decision is deferred
- residual risk or follow-up

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
I will run this under Harness. Current mode looks like direct because the requested change is one copy string. Scope: settings page label only. Out of bounds: account behavior and API changes. Next safe action: check write authority for that file, then make the edit and self-check.
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
