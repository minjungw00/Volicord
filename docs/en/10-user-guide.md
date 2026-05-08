# User Guide

## Document Role

This document explains how users talk to the agent, how to read state, and which judgments to make at which points.

It does not cover implementation internals or installation details.

## Starting Phrase

Everyday work starts as a conversation, not as a command.

```text
Run this work under the harness.
```

This means: check state, shape the scope, confirm allowed boundaries before writing, and proceed while recording evidence, verification, and user judgment.

Common phrases:

```text
Show me the status.
Continue this work. Check harness state first.
Show me the Journey Card before resuming.
Start with the scope and questions.
If this is small, handle it as direct; if it grows, move it to work.
Show the Decision Packet with options, recommendation, and uncertainty.
Approved. The scope is only what you just described.
Proceed AFK only when active Change Unit scope and Autonomy Boundary latitude both apply; sensitive categories still need granted approval, and actual product writes still need a compatible `prepare_write` Write Authorization.
Start detached verify.
Decide whether Manual QA is needed.
Show residual risk before I accept.
Accepted. Close this task.
```

## Basic Flow

The normal path should feel like a short conversation, not a work-management system. Users usually see a compact status card and the next safe action, not every internal record.

1. Check status or intake.
2. Classify as `advisor`, `direct`, or `work`.
3. Confirm scope and the Change Unit.
4. If product judgment blocks progress, read and answer the Decision Packet.
5. Before writing, the agent or Harness checks `prepare_write`.
6. After changes, the agent records run and evidence.
7. Verify, record Manual QA, show residual risk, and ask for acceptance when needed.
8. Close.

Gates should be explained as why the task cannot safely proceed or close yet. Evidence insufficiency should be shown by acceptance criterion, not as an abstract internal condition. If a cooperative guarantee is shown, explain plainly that the surface is expected to follow Harness decisions but may not physically block every violating write before it happens.

```text
Close blocked:
- AC-02 evidence missing
- Manual QA pending for UI copy
- Verification waived would close as risk accepted, not detached verified
```

## What You Usually Decide

Most sessions should reserve your attention for scope when it is not obvious, product or design trade-offs, sensitive approval, and QA, risk, or acceptance judgment.

You own the work direction and acceptable risk; you should not need to operate internal records by hand.

## What Harness Should Handle

Harness should handle state recording, `prepare_write` checks, artifact registration, evidence mapping, projection freshness, and close blockers.

Harness should translate your judgments into recorded state and clear blockers so you can stay focused on ownership, not bookkeeping.

If Harness or the connected surface cannot use MCP reliably, product/runtime/code changes should pause until the connection or surface setup is diagnosed. A documentation-only bootstrap override, when explicitly granted for exact paths, is not the same thing as Harness authorization.

## Reading A Status Card

A good harness session first shows a short status card. When significant work resumes, that card should be the Journey Card or an equivalent current-position view.

```text
TASK-0044 Add email login flow
Mode: work
State: shaping
Next action: decide failed-login UX
Scope: login form, login API call, session storage
Decision Gate: pending
Decision Packet: DEC-0012 failed-login UX
Autonomy Boundary: may implement agreed login flow details only
Write Authority: not yet requested
Approval: dependency_change required
Evidence: none
Verification: not started
Manual QA: pending
Acceptance: pending
Residual risk: none recorded
Projection: current
```

Look for these things.

- Does the request match the scope?
- What Decision Packet, if any, do I need to answer?
- What may the agent do inside the Autonomy Boundary?
- Is Write Authority not yet requested, blocked, or allowed for the intended write?
- What remains among approval, evidence, verification, Manual QA, residual risk, and acceptance?
- Is the next action safe to proceed with?

If the status looks wrong, say:

```text
Show the current status and next action again from state.
```

## Following The Journey Card

The Journey Card tells you where the work is right now. Use it before resuming, after long pauses, and near close.

Look for these lines:

- `Next action`: what the agent thinks is safe to do next
- `Decision Packet`: whether a product judgment is waiting for you
- `Autonomy Boundary`: what the agent may do without another question
- `Write Authority`: whether a specific `prepare_write` authorization exists for the intended write; it is separate from Autonomy Boundary
- `Evidence`, `Verification`, and `Manual QA`: what has been checked
- `Residual risk`: what uncertainty or trade-off remains
- `Projection`: whether the readable view is current enough to trust

Useful phrases:

```text
Pause there. Show the Decision Packet first.
That next action is fine. Continue inside that boundary.
Refresh the Journey Card after this run.
```

When a write is already authorized, the line should stay specific:

```text
Write Authority: WA-0017 allowed for src/auth/login.ts and tests/auth/login.test.ts
Guarantee: cooperative; changed-path validation detects violations after the fact
```

## Reading A Decision Packet

Start from the user question: "Given this context, do I choose this direction, defer it, or ask for a smaller Change Unit?"

A Decision Packet is used when the work needs human judgment before it can safely proceed, close, waive QA, accept verification risk, or accept remaining risk. It is not a request for broad approval.

Read it in this order:

- Why is this decision needed now?
- What exactly am I deciding?
- What are the options and trade-offs?
- What does the agent recommend, and how uncertain is it?
- What may the agent decide without me?
- What happens if I defer?
- What residual risk or follow-up would remain?

Good answers are specific:

```text
Choose Option A. Keep the failed-login message generic and record the security trade-off.
Defer this decision until after the smoke test. Record the follow-up risk.
I do not accept this trade-off. Propose a smaller Change Unit.
```

## advisor, direct, work

`advisor` is for reading, explaining, comparing, and reviewing. It does not write product files.

```text
Explain this module's role.
Summarize the trade-offs of this design choice.
```

`direct` handles small, low-risk changes quickly. Direct still needs an active scoped Change Unit before writing product files, and its default assurance is `self_checked`.

```text
Fix the typo on the profile save button. If it is small, handle it as direct.
```

`work` is for feature additions, structural changes, risky fixes, or multi-file work that needs scope shaping, evidence, and independent verification.

```text
Add the email login flow. Run it under the harness.
```

If the work starts small but grows, the agent should say that it is moving the same Task to `work`.

## Small Direct Work Should Stay Light

For small obvious work, Harness should define narrow scope as an active Change Unit, check write permission with `prepare_write`, record changed paths and self-check evidence, and close when no blockers appear.

If the work grows, the same Task should move to `work` and show scope, decisions, evidence, and risk instead of turning direct mode into silent broad autonomy.

## User Judgments

Product judgment, approval, assurance, Manual QA, residual-risk acceptance, and final acceptance answer different questions.

| Judgment | Question it answers | It cannot replace |
|---|---|---|
| Product judgment / Decision Packet | Which product direction, trade-off, waiver, or close-relevant decision should be taken? | approval, implementation, verification, QA, acceptance |
| Approval | May this sensitive change proceed? | product judgment, verification, QA, acceptance |
| Assurance | How far was this technically checked? | approval, QA, acceptance |
| Manual QA | Did a human inspect the actual experience quality? | verification, acceptance |
| Residual-risk acceptance | Does the user accept a known remaining risk or limitation? | approval, evidence, verification, Manual QA, final acceptance |
| Final acceptance | Does the user accept the result and remaining trade-offs? | approval, verification, QA |

Examples that need approval include dependency additions, auth/permission changes, data model changes, public API changes, destructive writes, secret access, and production config changes. Approval does not mean correctness or acceptance.

When approval itself needs your judgment, Harness may show it as an approval-shaped Decision Packet. In that case you are deciding whether the sensitive scope is allowed. That answer does not pick a product option, waive QA or verification, accept residual risk, or let the agent edit without the write check passing afterward.

Product judgment should appear as a Decision Packet when it blocks progress. That packet should show options, trade-offs, recommendation, uncertainty, and what happens if the decision is deferred.

Assurance usually appears as `none`, `self_checked`, or `detached_verified`. `detached_verified` means the result passed a separate verification boundary, not a same-session self-review.

The user may accept verification risk and close the task, but that is a risk-accepted close, not `detached_verified`. Residual-risk acceptance can make a known risk acceptable for close, but it does not replace approval, evidence, verification, Manual QA, or final acceptance.

## What The Agent May Do AFK

AFK implementation means the agent may continue while you are away. It is allowed only when active Change Unit scope, Autonomy Boundary latitude, and granted sensitive approval where applicable all apply. Actual product writes also require a compatible `prepare_write` / Write Authorization before writing.

The Autonomy Boundary is not a scope grant or write permission. The agent still needs `prepare_write`, active Change Unit scope, allowed paths, allowed tools, allowed commands, network targets, secret access, and sensitive approval where applicable.

The agent may usually implement agreed details, run allowed checks, collect evidence, update summaries, and stop with a clear blocker.

The agent must stop for human-held judgment:

- planning direction
- product trade-offs
- scope expansion
- sensitive-change approval
- QA waiver
- verification risk acceptance
- final acceptance

Useful phrase:

```text
Proceed AFK inside this boundary. Stop for product trade-offs, QA waiver, verification risk, or acceptance.
```

## Missing Evidence

Evidence is not a statement that something was done. It is a record that supports acceptance criteria.

```text
Evidence: partial
Close blocked: AC-02 supporting evidence missing
```

Say:

```text
Show which acceptance criteria are missing evidence, and suggest what additional checks would be enough.
```

If evidence is stale, the work may need a fresh run, fresh logs, a fresh diff, a fresh verification bundle, or scope reconfirmation.

## Verify

Work does not become `detached_verified` from the implementer's self-report alone.

```text
Start detached verify.
```

When verification passes, the agent should summarize what was checked, why the verification boundary counts as independent, and whether any blockers remain.

If you need to close without verification now, say:

```text
Accept the verification risk and close. Record the remaining risk.
```

In that case, the task can close successfully, but assurance is not displayed as `detached_verified`.

## Accepting Residual Risk

Residual risk is known remaining uncertainty, limitation, unchecked condition, or trade-off. Before final acceptance or a risk-accepted close, the agent should show the close-relevant residual risk in plain language.

Accepting residual risk can allow close, but it does not replace approval, evidence, verification, Manual QA, or acceptance.

Useful phrases:

```text
Show close-relevant residual risk before acceptance.
I accept the residual risk shown here. Close with risk accepted.
I do not accept that risk. Rework or add verification.
```

## Manual QA

Manual QA is the user's judgment about qualities that a person needs to inspect, such as UX, workflow, copy, accessibility, and visual result.

When a card says `Manual QA: pending`, that is the `qa_gate` display. It means the required QA has not yet produced a satisfying Manual QA record, not that there is a pending Manual QA record result.

```text
Decide whether Manual QA is needed.
```

If the Manual QA judgment is "not acceptable yet," the task does not close and returns to rework or blocked. If Manual QA is not useful for this task shape, record the waiver reason.

```text
Mark Manual QA waived for this internal CLI work. Reason: there is no user UI, and tests/logs are enough to verify it.
```

If skipping QA carries product or user risk, the waiver may require a Decision Packet; a waiver reason alone may not be enough.

```text
Show the QA waiver Decision Packet before I decide.
```

## Acceptance

Acceptance is the final user judgment that says, "I accept this result." Even if technical verification passes and Manual QA is complete, the task does not close unless the user accepts the remaining trade-offs.

```text
Accepted. Close this task.
```

The user can also reject it.

```text
I do not accept it. Rework the session-expiration UX.
```

Acceptance is not approval, Manual QA, or residual-risk acceptance.

## Resuming Work

Resume from harness state instead of searching through old chat.

```text
Show the active task status for this project.
Continue TASK-0044. Check harness state first.
```

When resuming, check two questions.

```text
What is the next action now?
Why is the work stopped now?
```

If you left notes in a document, say:

```text
Check the user notes in the TASK document and reconcile anything that should be reflected in state.
```

Documents are human-readable projections. If state and documents seem out of sync, check projection freshness and ask for a state-based summary.
