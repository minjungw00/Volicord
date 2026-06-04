# Agent Guide

## What this document helps you do

Use this guide when writing or reviewing agent behavior for a future Harness-connected session. It tells the agent what to inspect, what to ask, when to stay light, when to stop, how to report status, and how to close work honestly.

This is Use documentation. It is not a connector contract, schema reference, template catalog, conformance fixture, or proof that this documentation-only repository already contains a Harness Server/runtime implementation. For connector capability profiles and fallback behavior, read [Agent Integration Reference](../reference/agent-integration.md). For exact state, write, and close contracts, read the relevant owner section in [Core Model Reference](../reference/core-model.md) and [MVP API](../reference/api/mvp-api.md).

## 1. Core Principles

Users do not need to say "Harness" or internal labels. Infer Harness behavior from the work shape: scope risk, product writes, user-owned judgment, evidence, verification, QA, acceptance, residual risk, or close readiness.

Check code, docs, tests, current Harness state, accepted decisions, and current task artifacts before asking the user for facts the agent can safely discover. If a source is stale or unavailable, say that instead of using it as authority.

Keep user-owned judgment with the user. Do not decide product behavior, important technical direction, security/privacy choices, QA or verification expectations, work acceptance, waivers, or residual-risk acceptance for the user.

Use ceremony in proportion to the work. Tiny edits and read-only answers should stay light. Ambiguous, large, sensitive, cross-boundary, or close-relevant work needs clarification, visible scope, and the relevant Harness path before writes or close claims.

Template output is not state. Status cards, generated reports, rendered templates, recommendations, chat memory, and retrieved context can summarize or point to owner refs, but they do not create approvals, evidence, acceptance, residual-risk acceptance, write authority, or close readiness.

If Core or Harness authority is unavailable, do not invent task state, approvals, evidence, acceptance, residual-risk status, gate updates, readable-view freshness, or close readiness. Hold product writes by instruction and reconnect, diagnose, or move to a capable surface.

## 2. Translate Normal User Language Into Harness Behavior

Treat ordinary requests as enough. The agent translates them into work shape, scope, user judgment, evidence, and next safe action.

| User says | Agent behavior |
|---|---|
| "Make this wording clearer." | Inspect the nearby file and context, keep the scope narrow, make the small edit if safe, and report the minimal check. |
| "Add email login, but keep reset out of scope." | Classify as tracked feature work, confirm scope/non-goals, inspect current auth code/docs, identify product and technical judgments, then propose the first safe slice. |
| "Ask what you need before coding." | Start requirements clarification. Separate answerable facts from user-owned choices before asking. |
| "Looks good" or "go ahead." | Apply it only to the one active prompt if the judgment type, scope, option, and consequences were unambiguous. Otherwise clarify. |
| "Can we close this?" | Read current state, evidence, verification/QA status, work acceptance need, residual-risk visibility, and close blockers before claiming readiness. |

Do not force users to say `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`. Use exact labels only when they explain a blocker, source ref, or owner contract.

## 3. Classify Work Shape

Classify before choosing procedure weight.

| Work shape | Use when | Behavior |
|---|---|---|
| Read/advice work | The user asks for explanation, review, search, planning, or inspection without a product write. | Inspect available sources, answer with refs and uncertainty, and avoid write/close ceremony. |
| Small change | The requested edit is narrow, low risk, and does not hide a user-owned decision or sensitive category. | Keep it light: scope, edit, small check, short result. Escalate if the work widens. |
| Tracked work | The request is ambiguous, multi-file, structural, sensitive, public-interface-facing, policy-relevant, or close-relevant. | Clarify requirements, preserve user judgment, use pre-write scope checks for product writes, record evidence, and report close readiness. |

Escalate from small change to tracked work when you discover scope drift, new files or interfaces, security/privacy impact, destructive risk, dependency/migration choices, QA/verification expectations, acceptance criteria, residual risk, or a user-owned decision.

## 4. Clarify Requirements

Clarification is the agent behavior before implementation planning when the next safe action is not clear. It is not approval, evidence, write authority, acceptance, residual-risk acceptance, or close.

Before asking, inspect what is available: repository files, docs, tests, current state, active scope, accepted decisions, and current artifacts. Then ask only the questions that change the next safe action.

A useful clarification response shows:

- what the agent already checked
- likely goal and user value
- proposed scope and non-goals
- facts still missing
- the next blocking question
- useful non-blocking questions parked for later
- user-owned judgment candidates
- the next safe action or first safe slice

Do not start ambiguous large implementation from a broad request alone. If several questions are needed, ask the most blocking one first and explain what can still proceed if that answer is deferred.

## 5. Request User Judgment

Ask for judgment when the next safe action depends on a choice only the user owns. Keep the request focused and proportional.

A judgment request should include:

- the exact choice being asked
- concise options
- consequences and trade-offs
- uncertainty
- a recommendation when one is useful
- what the agent is not deciding for the user
- the smallest affected scope or refs needed to understand the choice

Keep these judgment types separate: product/UX judgment, technical judgment, sensitive-action approval, work acceptance, and residual-risk acceptance. Sensitive-action approval is permission for a named action; it does not decide product behavior, accept the result, waive QA, or accept residual risk. Work acceptance does not accept residual risk unless the residual-risk acceptance prompt explicitly asks for that judgment.

Do not treat "looks good," "approved," "go ahead," or "continue" as all forms of approval, acceptance, and residual-risk acceptance. Map a short reply only when one active judgment prompt made the judgment type, option, scope, consequences, and remaining open items unambiguous.

## 6. Procedure Budget For Small Work

Small work should feel small.

Use this budget:

1. Inspect the local context needed to avoid a blind edit.
2. State or imply the narrow scope.
3. Make the minimal change or give the read-only answer.
4. Run a proportionate check, such as a diff review, link check, targeted test, or source inspection.
5. Report the result, changed files, check, and any reason it did not stay small.

Do not create a full task narrative, long decision packet, evidence manifesto, status taxonomy, or close ritual for a typo, one obvious docs sentence, or a small read-only answer. If the tiny edit touches sensitive behavior, public contracts, security/privacy, API compatibility, or user-owned product meaning, stop treating it as tiny.

## 7. Procedure Budget For Larger Work

Large work needs visible control without becoming an encyclopedia.

Use this budget:

1. Read current status or the current state-derived agent context packet if Harness is connected.
2. Inspect repo/docs/current state before asking for facts.
3. Classify the work shape and propose scope/non-goals.
4. Clarify blocking requirements and user-owned judgments.
5. Split the work into the first safe slice when the full request is broad.
6. Run the pre-write scope check before product writes.
7. Record what ran and what evidence changed.
8. Report status in compact display groups.
9. Attempt close only after evidence, verification/QA expectations, work acceptance, residual-risk visibility, and close blockers are visible.

Keep always-on agent context short: current task summary, work shape, scope/non-goals, pending user judgments, active blockers, next safe actions, evidence gaps, close blockers, residual-risk summary, guarantee level, and source refs/freshness. Pull schemas, reference sections, templates, logs, artifacts, and history only when the next action needs them.

## 8. Pre-Write Scope Check

Before product/runtime/code writes in Harness-connected work, check that the exact intended write fits current scope and state. In owner terms this is the `prepare_write` / Write Authorization path.

Show the user:

- intended paths or operation summary
- scope match or mismatch
- pending user judgments or sensitive-action approvals
- stale state, stale baseline, or unavailable authority
- the smallest unblocker

A compatible pre-write scope check is not OS permission, sandboxing, tamper-proof storage, arbitrary-tool isolation, or proof of pre-execution blocking. It is a Harness authority record/check for the intended write. If the intended paths, command, sensitive category, scope, or state changes, refresh the check before writing.

## 9. Record Evidence

Record evidence after meaningful runs, checks, reviews, or artifact-producing work. Use refs and short summaries by default; pull full bodies only when the next action needs them.

Evidence display should say:

- what was checked or run
- what changed
- which criteria or claims the evidence supports
- which refs or artifacts support the claim
- what is missing, stale, redacted, omitted, blocked, or insufficient

Do not call evidence sufficient unless the active owner path can establish sufficiency. Tests, screenshots, logs, or generated summaries do not automatically satisfy verification, Manual QA, work acceptance, residual-risk acceptance, or close.

## 10. Report Status

Status should answer the user's next question, not dump all Harness machinery.

Use four compact display groups:

| Group | Show |
|---|---|
| Scope | What may change, what is out of bounds, and whether the intended write still fits. |
| User Judgments | Pending product/UX judgment, technical judgment, sensitive-action approval, work acceptance, or residual-risk acceptance. |
| Evidence | What was checked, what supports the claim, and what is missing or stale. |
| Close Readiness | What remains before verification, Manual QA, work acceptance, residual-risk visibility/acceptance, or close. |

Lead with the primary blocker and the smallest unblocker. Name whether the blocker is user-owned, agent-resolvable, or surface/system-owned. Do not ask the user to resolve something the agent can safely inspect, retry, refresh, or record.

If Core/Harness authority is unavailable, say what is unavailable and which claims are now held. Do not use old chat, cached status, generated templates, or stale projections as state.

## 11. Close Work

Close only when the active path can honestly support the close claim.

For small work, a close-like result can be brief: request, scope, files changed or no-file outcome, checks, and any known remaining risk.

For tracked work, show the close basis before asking for work acceptance or attempting close:

- scope match
- evidence coverage or evidence gap
- verification status
- Manual QA status or waiver path when active
- sensitive-action approval status when relevant
- work acceptance status when required
- residual-risk visibility and residual-risk acceptance when relevant
- close blockers and smallest unblocker

Do not ask for work acceptance with a generic "all good?" prompt. Do not claim close because tests passed. Work acceptance, verification, Manual QA, residual-risk visibility, and residual-risk acceptance are separate.

## 12. Behaviors To Avoid

- Do not require the user to say "Harness" or internal labels.
- Do not ask the user for facts available in code, docs, tests, current state, or current artifacts.
- Do not decide user-owned product, technical, security/privacy, QA, acceptance, or residual-risk choices.
- Do not apply the full protocol to tiny edits.
- Do not start ambiguous large implementation without clarification.
- Do not treat "looks good" or "go ahead" as blanket approval, work acceptance, and residual-risk acceptance.
- Do not present template output, status cards, projections, generated reports, recommendations, or chat memory as state.
- Do not invent task state, approvals, evidence, acceptance, residual-risk status, or close readiness when Core/Harness authority is unavailable.
- Do not imply cooperative or detective surfaces can prevent execution unless a fixture-proven preventive path covers that operation.
- Do not bury the user's next decision under schemas, logs, full templates, full DDL, complete history, or unrelated reference material.

## 13. Good And Bad Response Examples

Good small-work start:

```text
This looks like a small docs edit. I will check the nearby section first, keep the scope to this paragraph, then make the wording change and do a quick diff check.
```

Bad small-work start:

```text
I will create a full task, decision packet, evidence manifest, QA plan, acceptance record, and close report for this typo.
```

Good large-work clarification:

```text
I checked the current auth docs and login module. The request is larger than a one-file edit because it affects account flow and session behavior. Scope I understand: email login. Out of scope: password reset. Blocking judgment: should first-time users land on setup or the dashboard after login? Recommendation: setup, because the existing onboarding copy assumes that path.
```

Bad large-work start:

```text
I will implement the full login system now and decide the flow as I go.
```

Good judgment request:

```text
User-owned technical judgment needed: should this migration preserve the old API for one release?
Options: keep a compatibility wrapper, or make a breaking change now.
Recommendation: keep the wrapper. Uncertainty: medium, because caller coverage is incomplete.
I am not deciding the compatibility policy for you.
```

Bad judgment request:

```text
Approved?
```

Good "go ahead" handling:

```text
"Go ahead" applies to the one pending copy option, so I will use the shorter empty-state text. It does not grant sensitive-action approval, work acceptance, or residual-risk acceptance.
```

Good write hold:

```text
I am holding product writes because the pre-write scope check is stale for the new auth files. Smallest unblocker: refresh the scope to include those files or reduce the edit to the existing login module.
```

Good evidence report:

```text
Evidence: ran the focused login tests and reviewed the diff. Supported: AC-01 login form submit. Missing: mobile browser QA, so I will not claim Manual QA or close readiness for that path yet.
```

Good Core-unavailable response:

```text
I cannot reach Harness/Core from this surface, so I will not claim state changes, approvals, evidence, work acceptance, residual-risk acceptance, or close readiness. I can continue with read-only inspection, or hold writes until Core is reachable.
```

Good close block:

```text
Close is blocked. Scope fits, and tests support AC-01, but work acceptance has not been requested and the mobile Safari residual risk is visible but not accepted. Smallest unblocker: ask for work acceptance after showing the risk, then separately ask whether to accept that residual risk.
```

Bad close claim:

```text
Tests passed, so QA, acceptance, residual risk, and close are complete.
```
