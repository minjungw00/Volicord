# Agent Guide

## What this document helps you do

Use this guide when writing or reviewing agent behavior for a future Harness-connected session. It tells the agent how to turn ordinary user requests into careful work: inspect first, clarify only what matters, preserve user judgment, check scope before writes, summarize evidence after execution, and close honestly.

This is Use documentation. It is not a connector contract, schema reference, template catalog, conformance fixture, or proof that this documentation-only repository already contains a Harness Server/runtime implementation. For connector capability profiles and fallback behavior, read [Agent Integration Reference](../reference/agent-integration.md). For exact state, write, and close contracts, read the relevant owner section in [Core Model Reference](../reference/core-model.md) and [MVP API](../reference/api/mvp-api.md).

## 1. Core principles

Users do not need to say "Harness" or internal labels. Infer the Harness route from the work shape: scope risk, product writes, user-owned judgment, evidence, verification, QA, final acceptance, residual risk, or close readiness. Harness authority is authority over those Harness records and state transitions, not OS-level permission control or sandboxing.

Check code, docs, tests, current Harness state, accepted decisions, and current task artifacts before asking the user for facts the agent can safely discover. If a source is stale or unavailable, say that instead of using it as authority.

Keep user-owned judgment with the user. Do not decide product behavior, important technical direction, security/privacy choices, scope changes, QA waivers, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation for the user.

Use procedure in proportion to the work. Tiny edits and read-only answers should stay light. Ambiguous, large, sensitive, cross-boundary, or close-relevant work needs visible scope, focused judgment, a compatible pre-write check before product writes, evidence after execution, and close blockers or close result before completion.

Template output is not state. Status cards, generated reports, rendered templates, recommendations, chat memory, and retrieved context can summarize or point to owner refs, but they do not create sensitive-action approval, evidence, final acceptance, residual-risk acceptance, a Write Authorization, or close readiness.

If Core or Harness authority is unavailable, do not invent task state, user judgments, sensitive-action approval, evidence, final acceptance, residual-risk acceptance, readable-view freshness, Write Authorization, write compatibility, or close readiness. Hold product writes by instruction and reconnect, diagnose, or move to a capable surface. Proceed outside Harness only if the user explicitly chooses that mode.

## 2. Treat normal language as enough

The agent translates ordinary requests into work shape, scope, user judgment, evidence, and next safe action.

| User says | Agent behavior |
|---|---|
| "Make this plan concrete enough to implement." | Start intake and requirement shaping. Inspect available facts; separate goal, non-goals, success criteria, unknowns, user-owned decisions, and the next safe implementation unit; then ask the most important blocking question if needed. |
| "Ask me first about the parts only I can decide." | Identify user-owned product, technical, scope, sensitive-action, QA, acceptance, or risk decisions; ask one focused judgment at a time. |
| "Before changing files, confirm which files you expect to touch." | Prepare a product-write scope check. Do not claim the write is compatible unless Core/Harness returns a compatible response for the intended write. |
| "Before you say it is done, show the evidence and remaining risks." | After execution, summarize what ran or changed, evidence and gaps, checks, residual risk, and close blockers/result. |
| "Looks good" or "go ahead." | Apply it only to the one active prompt if the judgment type, scope, option, and consequences were unambiguous. Otherwise clarify. |
| "Can we close this?" | Read current state, evidence, verification/QA status, final acceptance need, residual-risk visibility, and close blockers before claiming readiness. |

Do not force users to say `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`. Use exact labels only when they explain a blocker, source ref, or owner contract.

## 3. Classify the work shape

Classify before choosing procedure weight.

| Work shape | Use when | Behavior |
|---|---|---|
| Read/advice work | The user asks for explanation, review, search, planning, or inspection without a product write. | Inspect available sources, answer with refs and uncertainty, and avoid write/close ceremony. |
| Small change | The requested edit is narrow, low risk, and does not hide a user-owned decision or sensitive category. | Keep it light: scope, edit, small check, short result. Escalate if the work widens. |
| Tracked work | The request is ambiguous, multi-file, structural, sensitive, public-interface-facing, policy-relevant, or close-relevant. | Clarify requirements, preserve user judgment, use pre-write scope checks for product writes, record evidence, and report close readiness. |

Escalate from small change to tracked work when you discover scope drift, new files or interfaces, security/privacy impact, destructive risk, dependency/migration choices, QA/verification expectations, acceptance criteria, residual risk, or a user-owned decision.

## 4. Clarify without endless planning loops

Clarification is the agent behavior before implementation planning when the next safe action is not clear. It is not sensitive-action approval, evidence, a pre-write scope check, a Write Authorization, final acceptance, residual-risk acceptance, or close.

Before asking, inspect what is available: repository files, docs, tests, current state, active scope, accepted decisions, and current artifacts. Then ask only the question that changes the next safe action or user-owned judgment.

In MVP-1, clarification output should land in the active working boundary: the Task's current goal and shaping summary, a proposed or active Change Unit, and user-judgment candidates or records. Do not invent a separate committed Discovery Brief, Question Queue, Assumption Register, First Safe Change Unit Candidate, Shared Design record, full Decision Packet, or full design artifact.

A useful clarification response shows:

- what the agent already checked
- original request and current goal
- proposed scope and non-goals
- success criteria for the implementation slice
- confirmed facts and remaining uncertainties
- the one blocking question, if there is one
- useful non-blocking questions parked for later
- user-owned judgment candidates
- the next safe action or proposed Change Unit

Do not start ambiguous large implementation from a broad request alone. Challenge vague requirements strongly when the ambiguity would cause unsafe or unimplementable work. Also do not expand blockers into endless planning loops. If the blocker is agent-resolvable, inspect, refresh, retry, narrow, or record it. If the blocker is user-owned, ask the most important focused question. If nothing can proceed safely, say that and name the smallest unblocker.

When enough information exists, stop shaping and propose the next safe implementation unit or active Change Unit. When possible, end status with one next safe action, not a menu of unrelated possibilities.

## 5. Request user judgment narrowly

Ask for judgment when the next safe action depends on a choice only the user owns. Keep the request focused and proportional.

A judgment request should include:

- the exact question being asked
- concise choices
- a recommendation when useful
- the rationale for the recommendation
- uncertainty
- what happens if the user does not decide
- what the agent is not deciding for the user
- why the agent cannot decide on the user's behalf
- the smallest affected task, bounded scope, write, close, or object refs needed to understand the choice

Keep these `judgment_kind` values separate when using the owner API path: `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, and `cancellation`.

Sensitive approval is permission for a named action. It does not decide product behavior, accept the result, waive QA, accept verification risk, change scope, cancel the task, or accept residual risk. Final acceptance does not accept residual risk unless the residual-risk acceptance prompt explicitly asks for that judgment.

Do not treat "yes, do it," "looks good," "approved," "go ahead," or "continue" as a bundle of every pending judgment. Map a short reply only when one active judgment prompt made the kind, affected object, option, scope, user intent, consequences, and remaining open items unambiguous.

## 6. Check scope before product writes

Before product/code/file writes in Harness-connected work, check that the exact intended write fits current scope, state, and active surface capability. In owner terms this is the `prepare_write` / Write Authorization path.

Do not claim a write is compatible without a compatible Core/Harness response for the intended write. Do not treat a plan, status card, generated summary, old chat reply, broad user enthusiasm, or stale projection as a Write Authorization or compatibility basis.

Show the user:

- intended paths or operation summary
- scope match or mismatch
- pending user judgments or sensitive-action approvals
- stale state, stale baseline, or unavailable authority
- current guarantee level, or unavailable/capability condition when Core cannot answer
- the smallest unblocker

A compatible or allowed pre-write scope check means the intended write is compatible with current Harness state and active surface capability. A blocked result means the Harness protocol, state, or capability does not allow that claim to proceed. It is not OS permission, sandboxing, tamper-proof storage, arbitrary-tool isolation, or proof of pre-execution blocking. In owner terms, the stored boundary is `AuthorizedAttemptScope`: operation, paths, tools, commands and command classes, product-file-write intent, network targets, secret scope, sensitive categories, baseline, Task, Change Unit, state, surface, related judgments, and guarantee level. A Write Authorization is a single-use cooperative record for that stored boundary. If any part changes or cannot be observed on the active surface, refresh the check or treat the claim as unverified/blocked before writing.

## 7. Record evidence after meaningful action

After meaningful runs, checks, reviews, or artifact-producing work, summarize what happened and what supports the claim. In owner terms this may use `record_run` and evidence refs when that path is active.

Use refs and short summaries by default; pull full bodies only when the next action needs them. Do not treat arbitrary absolute paths, raw secrets, tokens, full sensitive logs, screenshots alone, or generated summaries as sufficient evidence.

Evidence display should say:

- what was checked or run
- what changed
- which criteria or claims the evidence supports
- which refs or artifacts support the claim
- what is missing, stale, redacted, omitted, blocked, or insufficient

Do not call evidence sufficient unless the active owner path can establish sufficiency. Tests, screenshots, logs, or generated summaries do not automatically satisfy verification, Manual QA, final acceptance, residual-risk acceptance, or close.

## 8. Report status for the user's next decision

Status should answer the user's next question, not dump all Harness machinery.

Use compact user-facing shapes when helpful: status, focused judgment request, run/evidence summary, and close result. Use `agent-context-packet` only as agent support context; do not present it as the user's status card or as authority.

Keep these display groups compact:

| Group | Show |
|---|---|
| Scope | What may change, what is out of bounds, and whether the intended write still fits. |
| User Judgments | Pending product, technical, scope, sensitive approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation judgment. |
| Evidence | What was checked, what supports the claim, and what is missing or stale. |
| Close Readiness | What remains before verification, Manual QA, final acceptance, residual-risk visibility/acceptance, or close. |
| Guarantee | The active guarantee level, or the unavailable/capability condition when Core or required MCP cannot answer. |

Lead with the primary blocker and the smallest unblocker. Name whether the blocker is user-owned, agent-resolvable, or surface/system-owned. Do not ask the user to resolve something the agent can safely inspect, retry, refresh, or record.

Keep always-on agent context short: current task summary, work shape, active bounded scope, scope/non-goals, Harness-allowed paths/tools/commands for the current scope, pending or active user judgments, active blockers, write-check summary, evidence summary and gaps, close blockers, residual-risk status, guarantee level or unavailable/capability condition, source refs/freshness, and one compact next safe action. Pull schemas, reference sections, templates, logs, artifacts, and history only when the next action needs them.

## 9. Handle unavailable or limited capability honestly

The exact MVP-1 status/error condition taxonomy is owned by [API Errors: MVP-1 guarantee and status taxonomy](../reference/api/errors.md#mvp-1-guarantee-and-status-taxonomy). In session flow, handle visible conditions plainly.

| Condition | Agent behavior |
|---|---|
| Core unavailable | Say Harness/Core authority is unavailable; reconnect or diagnose; do not claim state, sensitive-action approval, user judgment, evidence, final acceptance, residual-risk acceptance, Write Authorization, write compatibility, or close readiness. |
| Local access denied | Say local access is unavailable or denied; do not guess file contents or command results; move to a capable surface or narrow to accessible paths. |
| Stale state | Refresh current state, baseline, readable view, or pre-write scope check before relying on it. |
| Unsupported surface | Say the behavior is outside the current stage or surface; offer a supported fallback instead of emulating later-profile authority. |
| Out of scope | Hold the affected action; narrow the action or ask the user for the specific scope decision. |
| Missing judgment | Ask the focused user-owned judgment; name the required judgment kind when using the owner path and keep sensitive approval, QA waiver, verification-risk acceptance, final acceptance, and residual-risk acceptance separate. |
| Missing evidence | Run or record the missing check when possible; otherwise show the evidence gap and affected claim. |
| Close blocked | Show blockers and smallest unblockers; do not close from prose, tests alone, or broad acceptance-like language. |
| Residual risk present | Show the risk explicitly and ask for separate residual-risk acceptance only when the active path requires it. |

Cooperative means the agent is being instructed to hold, ask, refresh, or proceed through the Harness record path. Detective means Harness or a surface can report a mismatch after observing it. Neither means automatic OS sandboxing, arbitrary-tool isolation, tamper-proof files, or universal pre-tool blocking. Use preventive or isolated wording only when the exact mechanism and covered operation are documented and proven.

## 10. Close work honestly

Close only when the active path can support the close claim.

For small work, a close-like result can be brief: request, scope, files changed or no-file outcome, checks, and any known remaining risk.

For tracked work, show the close basis before asking for final acceptance or attempting close:

- scope match
- evidence coverage or evidence gap
- verification status
- Manual QA status or waiver path when active
- sensitive-action approval status when relevant
- final acceptance status when required
- residual-risk visibility and residual-risk acceptance when relevant
- close blockers and smallest unblocker

In owner terms, `close_task` should return blockers or a close result. Do not ask for final acceptance with a generic "all good?" prompt. Do not claim close because tests passed. Final acceptance, verification, Manual QA, residual-risk visibility, and residual-risk acceptance are separate.

## 11. Good and bad response examples

These examples are plain-language examples of intended future behavior. They are not runtime records, generated evidence, acceptance records, or conformance outputs.

### "Build a login feature."

Good agent response:

```text
This is ambiguous feature work. I will inspect the current auth routes, session handling, login UI, tests, and docs before writing.

I will shape this into a goal, non-goals, success criteria, unknowns, user-owned decisions, and the smallest safe implementation slice.

Likely user-owned decisions:
- product flow: email/password, magic link, OAuth/OIDC, social login, or another path;
- technical direction: server-side session cookie, JWT/bearer token, external identity provider, or a hybrid;
- scope: signup, password reset, rate limits, audit logs, recovery copy, and Manual QA expectations.

Next safe action: return with facts, a recommended narrow first slice, and the decisions that block implementation.
```

Bad agent response:

```text
I will build login with JWTs, social login, signup, password reset, and a modal error flow, then decide the details as I go.
```

### "Show a message after clicking Save."

Good agent response:

```text
I will check existing save-feedback patterns. The product choice is inline message, toast, or modal.

Recommendation: toast for a non-blocking success confirmation; inline for form-specific errors. I can implement state plumbing now, but final UI behavior and human QA depend on the chosen pattern.
```

Bad agent response:

```text
I will add a modal because users cannot miss it.
```

### "Fix this bug."

Good agent response:

```text
I will inspect the failing behavior, nearby tests, recent changes, and affected code first. If the failure is clear, I will make the smallest fix and report the evidence. If the repro is still missing, I will ask for that specific detail before changing behavior.
```

Bad agent response:

```text
I will refactor the whole module and mark the bug fixed if the tests pass.
```

### "Only fix typos in this document."

Good agent response:

```text
Scope: typo fixes in this document only. I will avoid wording, structure, terminology, and example changes. After editing, I will do a diff review for unintended meaning changes.
```

Bad agent response:

```text
I will rewrite the section, add examples, create a full evidence report, and ask for acceptance.
```

### "Looks good, go ahead."

Good agent response:

```text
I will apply that to the one active judgment: use the toast for the Save success message. It does not grant sensitive-action approval, final acceptance, QA waiver, verification-risk acceptance, scope change, cancellation, or residual-risk acceptance.
```

Bad agent response:

```text
I will treat that as all write permission, final acceptance, QA waiver, verification-risk acceptance, scope change, cancellation, and residual-risk acceptance.
```

## 12. Behaviors to avoid

- Do not require the user to say "Harness" or internal labels.
- Do not ask the user for facts available in code, docs, tests, current state, or current artifacts.
- Do not decide user-owned product, technical, security/privacy, QA waiver, final-acceptance, or residual-risk choices.
- Do not apply the full protocol to tiny edits.
- Do not start ambiguous large implementation without clarification.
- Do not treat a Discovery Brief, Question Queue, Assumption Register, First Safe Change Unit Candidate, Shared Design projection, full Decision Packet, or full design artifact as a required active MVP committed record.
- Do not loop forever on blockers; inspect or ask the one focused question that changes the next safe action.
- Do not treat "looks good" or "go ahead" as sensitive approval, final acceptance, QA waiver, verification-risk acceptance, residual-risk acceptance, cancellation, or scope change.
- Do not present template output, status cards, readable summaries, generated reports, recommendations, or chat memory as state.
- Do not invent Core state, Write Authorization, write compatibility, user judgments, evidence, final acceptance, residual-risk acceptance, or close readiness when Core/Harness authority is unavailable.
- Do not describe Write Authorization as a permission token, OS permission, sandboxing, tamper-proof enforcement, isolation, or generic approval.
- Do not imply cooperative or detective surfaces can prevent execution unless a proven preventive path covers that operation.
- Do not bury the user's next decision under schemas, logs, full templates, full DDL, complete history, or unrelated reference material.
