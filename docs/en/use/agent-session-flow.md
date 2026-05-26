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

The always-on turn context should be a compact Harness envelope: active Task id and mode, scope, out of bounds, next safe action, primary blocker, smallest unblocker, active Change Unit summary, blocking decisions, write authority status, evidence, verification, Manual QA, residual risk, guarantee level, gate summary, and projection freshness. Evidence, Run, Eval, Manual QA, artifacts, logs, screenshots, diffs, and large traces should appear as refs and short outcomes by default, then be pulled only when the next action requires inspecting them.

## Session start

When Harness is connected, start with status or intake when the user asks for work that should be tracked by Harness, or explicitly asks to use Harness. The user does not need to say "Harness." Infer from the request shape and keep the first response short.

Track ordinary-language requests when their shape suggests scope, judgment, evidence, or close state should stay visible:

- product writes or state-changing project work
- scope drift risk or ambiguous requirements
- multi-file, structural, migration, or cross-boundary work
- sensitive or policy-relevant areas such as auth, security, billing, destructive/data-loss risk, privacy, compliance, accessibility, or design quality
- user-owned product judgment or material technical judgment with cost, compatibility, security, maintenance, migration, interface, dependency, or risk impact
- evidence, verification, Manual QA, acceptance, or residual-risk needs

Keep small direct tasks light. Do not add ceremony just to answer a question, inspect code, explain a result, or handle a tiny low-risk change with an already narrow shape.

Show:

- the active or likely Task id and mode: `advisor`, `direct`, or `work`
- the current or proposed scope
- what is out of bounds
- the next safe action
- any question that blocks progress
- the primary blocker, who owns the next move, and the smallest unblocker
- secondary blockers only when they still affect the follow-on path
- write authority status when writes are possible or near
- evidence, verification, Manual QA, residual-risk, and acceptance status when those affect the next decision or close readiness
- guarantee level and what the surface can actually block or only detect
- compact gate and projection freshness status
- when guard, freeze, or careful mode is relevant, what can actually be blocked before execution and what can only be detected after action

Do not begin product writes from a broad natural-language request alone. First establish scope and compatible write authority for the intended change.

## Resume

Before significant work resumes, read Harness state and show the current position. Do not reconstruct authority from old chat when state is available.

A good resume response says:

```text
I found the active task. Current scope is X. The next safe action is Y. Product writes are not authorized yet. One decision is pending: Z.
```

If projection, `source_state_version`, or readable status is stale or unknown, say that and refresh or reconcile before depending on it. If canonical state is available directly, the agent may continue from that state while warning that the readable projection is not the source of authority.

Keep display failures separate. A stale projection means the readable card/report may lag and needs refresh or reconcile before it becomes dependable context. Stale state, baseline, or evidence means the underlying inputs moved or became insufficient and may block writes or close. MCP unavailable means the agent cannot reach the required Harness/Core capability; do not claim authoritative state changes, Approval, result acceptance, residual-risk acceptance, gate updates, projection repairs, or close until that capability is available again.

If Core itself is unreachable, the display issue is `MCP_SERVER_UNAVAILABLE`: say Core cannot be reached and reconnect or diagnose before claiming state changed. If Core or the operator can tell that the current surface lacks usable MCP, the display issue is `SURFACE_MCP_UNAVAILABLE`: say this surface cannot use the required Harness tools, then hold writes by instruction or switch to a capable surface. Only say execution was blocked before action when a proven preventive guard covered that operation.

## Reading status and blockers

Use MCP results as the source, then speak in user terms.

The exact error taxonomy, complete mapping, and precedence stay in [MCP API And Schemas](../reference/mcp-api-and-schemas.md). This section gives short display examples for common session responses; it is intentionally not exhaustive.

- `harness.status` means "where are we now?"
- `harness.next` means "what is the next safe action or smallest unblocker?"
- `harness.prepare_write` means "may this exact product write happen now?"
- `harness.record_run` means "what happened, what evidence changed, and what is next?"
- `harness.close_task` means "can this Task finish or cancel now?"

When a response contains errors or blockers, lead with one primary blocker. Use the first `ToolError` chosen by API precedence, or the first `close_task` blocker when close returned blockers. Then show the smallest unblocker in ordinary language. Keep secondary blockers visible only when they will still matter after the primary blocker is resolved.

Every blocker display should also name ownership in user-facing terms:

- User-owned: product direction, material technical direction, sensitive-action Approval, Manual QA judgment or waiver, residual-risk acceptance, final acceptance, or another choice the user must make.
- Agent-resolvable: refresh or reconcile status, retry `prepare_write`, collect missing evidence, run an in-scope check, repair or replace an artifact, or narrow the Change Unit without changing user-owned judgment.
- Surface or system: Core unavailable, surface MCP unavailable, capability insufficient, or another condition that needs reconnection, a different surface, or operator repair.

Do not ask the user to resolve an agent-resolvable blocker. Say what the agent will do next, unless that action would change scope, require Approval, or create new user-owned risk.

Common display examples:

| Raw condition | Say first | Smallest unblocker |
|---|---|---|
| `STATE_CONFLICT` | State changed since this view. | Refresh status and retry with the current state version. |
| `MCP_UNAVAILABLE` with `details.mcp_unavailable_kind=server_unavailable`, or diagnostic `MCP_SERVER_UNAVAILABLE` | Core cannot be reached. | Reconnect or diagnose Core access before claiming state changes. |
| `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with `details.mcp_unavailable_kind=surface_mcp_unavailable`, or diagnostic `SURFACE_MCP_UNAVAILABLE` | This surface cannot use the required Harness tools. | Repair the surface or switch to a capable surface; hold writes by instruction unless a proven guard blocks execution. |
| `MCP_UNAVAILABLE` with no useful detail | Harness/Core capability is unavailable. | Reconnect, repair the surface, or switch to a capable surface before claiming state changes. |
| `CAPABILITY_INSUFFICIENT` | This surface cannot provide the needed guarantee. | Use a capable profile, reduce the operation, or choose a path that does not need that capability. |
| `NO_ACTIVE_TASK` | No active Task is selected. | Select or create the Task before continuing. |
| `WRITE_AUTHORIZATION_REQUIRED` or `WRITE_AUTHORIZATION_INVALID` | Write authority is missing or stale. | Retry `harness.prepare_write` for the exact intended write. |
| `DECISION_REQUIRED` or `DECISION_UNRESOLVED` | User judgment is needed. | Show the Decision Packet or a focused decision prompt. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, or `APPROVAL_EXPIRED` | Sensitive-action Approval is needed or unusable. | Request, resolve, or renew the Approval, then retry the write check. |
| `PROJECTION_STALE` | The readable status view is stale. | Refresh or reconcile the projection before relying on that view. |
| `ARTIFACT_MISSING` | An artifact is missing or failed integrity. | Reattach, regenerate, or replace the artifact before using it as evidence. |

Prefer the plain phrase first and the exact Harness term in parentheses only when it helps: "Write authority is stale (`WRITE_AUTHORIZATION_INVALID`). Smallest unblocker: rerun `harness.prepare_write` for the current file list."

## Intake

Intake turns an everyday request into a usable task shape without forcing the user to speak Harness.

Listen for the same task-shape triggers used at session start: product writes, scope drift risk, ambiguous requirements, multi-file or structural work, sensitive or policy-relevant areas, user-owned judgment, and evidence, verification, Manual QA, acceptance, or residual-risk needs. When one appears, translate the ordinary request into a proposed mode, scope, out-of-bounds area, and next safe action.

Ask only questions that change the next safe action. Prefer one blocking question with a recommendation instead of a long form.

Before asking, inspect repo, codebase, docs, and Harness state that are available and current for answers the agent can discover safely. Do not ask the user to restate existing file paths, behavior, terminology, or constraints that are already visible from current context. If a source is unavailable or stale, say so rather than relying on it as authority.

One blocking question at a time does not mean one clarification round total. Broad or design-heavy requests may need several short turns until the goal, scope, non-goals, acceptance criteria, affected product areas, user-facing screens or flows, modules, interfaces, sensitive categories, user-owned product or material technical trade-offs, verification or Manual QA expectations, and known product, implementation, verification, QA, or follow-up risks are shaped enough to propose the first safe Change Unit.

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

Use this distinction when explaining stops and permissions:

| Concept | Plain question | Allows | Does not allow |
|---|---|---|---|
| Change Unit scope | What work area is in bounds? | Names the behavior, files, paths, tools, commands, network targets, and sensitive categories the work is scoped around. | Does not decide user-owned product or material technical judgment or create Write Authorization by itself. |
| Autonomy Boundary | What may the agent decide alone inside that scope? | Lets the agent choose covered implementation details without another user decision. | Does not grant paths, tools, commands, network, secrets, sensitive categories, approval, or write authority. |
| Approval | May this sensitive step proceed? | Allows a named sensitive action within its recorded scope and expiry. | Does not decide user-owned judgment, prove correctness, accept risk, or create Write Authorization. |
| Write Authorization | May this exact write attempt happen now? | Records that Core allowed one compatible write attempt after the required checks. | Is not reusable and does not expand scope, Autonomy Boundary, or Approval. |

When a prompt or status uses the word "approved," name the exact authority or recorded decision: sensitive-action Approval, scope confirmation, Decision Packet resolution, residual-risk acceptance, final acceptance, or Write Authorization status. Do not use "approved" as a catch-all label.

Examples:

- Dependency install approval: approval to run the install or update dependency files does not decide that the new dependency is the right architecture choice. If that choice affects compatibility, rollback, cost, or maintenance, use a Decision Packet.
- Secret access approval: approval to read or use a secret inside the requested scope does not permit exposing secret values in artifacts, projections, exports, logs, screenshots, or summaries.
- Auth/system change approval: approval to touch auth files, permissions, or system configuration does not choose session auth, JWT, social login, role model, lockout behavior, or user notice.
- Public API change decision: resolving the API direction decides the contract choice for the Task; it is not deployment authority, merge authority, or a reusable Write Authorization.
- Final acceptance: accepting the result does not authorize more writes, approve new sensitive actions, or retroactively satisfy missing evidence, QA, verification, or Write Authorization.

Inside the Autonomy Boundary, the agent may decide ordinary implementation details: whether to reuse an existing helper, how to split a private function, where to place focused tests, or which conservative internal approach best fits the agreed result. The agent must stop for user judgment before public API or module contract changes, security or privacy trade-offs, UX or product trade-offs, material technical direction such as dependency or migration choices, scope expansion, or residual-risk acceptance.

## Blocking user-owned judgment

When user-owned product or material technical judgment blocks progress, show or request a Decision Packet. Do not replace it with broad approval or a vague "continue?" prompt.

A user-facing Decision Packet should include:

- why the decision is needed now
- the exact question
- options and trade-offs
- recommendation and uncertainty
- what can continue if the decision is deferred, or why nothing should continue until it is made
- residual risk or follow-up

Useful examples:

- Failed-login UX: compare inline message, toast, and modal/layer; recommend one based on flow, accessibility, interruption, and copy risk. If deferred, backend auth work may continue, but the final failed-login experience should not be claimed done.
- Failed-login copy: compare generic, specific, and hybrid wording; recommend one based on account enumeration risk, clarity, recovery usefulness, support burden, and product tone. If deferred, validation wiring may continue, but release-ready copy and Manual QA should stay open.
- Product taste and Manual QA need: compare a polished interaction that needs human visual review with a simpler conservative behavior that can be checked by tests and browser smoke. Explain the taste trade-off, QA cost, user impact, and what can continue if Manual QA is deferred, or why nothing should continue until the decision is made.
- Auth approach: compare session cookie, JWT, and social login; explain revocation, CSRF/XSS exposure, client compatibility, operational complexity, and migration cost. If deferred, form scaffolding may continue only if it does not commit to the session model.
- Dependency choice: separate approval to install or update dependency files from the architecture decision to adopt the dependency. Compare adding the dependency, using existing utilities, or postponing the capability, and explain compatibility, rollback, cost, and maintenance impact.
- Schema/data-model migration: compare additive migration, compatibility shim, and breaking cleanup. Explain migration evidence, data-backfill risk, rollback path, test boundary, and maintenance cost.
- Public API/interface or module boundary: compare preserving the current interface, adding a narrow extension, or moving responsibility across a module boundary. Explain caller impact, compatibility or breaking-change risk, boundary tests, documentation promises, migration path, and future-change cost.
- Security-sensitive change: approval to access a secret, change permissions, or export data is only an approval boundary. Separate product or security judgment may still be needed for roles, fields, redaction, audit logging, retention, rollback, and user notice.
- QA or verification waiver: use the existing recording required for the Task. A QA waiver is recorded through Manual QA/gate state and `qa_gate=waived`; product/user risk or policy-required judgment uses a QA waiver Decision Packet. A verification waiver is recorded as `verification_gate=waived_by_user`; when user-owned judgment is needed, use the relevant Decision Packet. Name the skipped check or surface, accepted risk, follow-up, relevant refs, and close impact. Example: waive mobile Safari Manual QA for a copy-only change, accept wrapping risk, and keep a browser pass as release follow-up.
- Residual-risk acceptance before close: show the remaining limitation, the evidence that does exist, why close can still be acceptable, and the follow-up that remains.

Ask one blocking question at a time when possible.

## Review lenses and displays

When the user asks for a product, engineering, design, security, QA, or release-handoff perspective, treat `product-review`, `eng-review`, `design-review`, `security-review`, `qa-review`, and `release-handoff` as Role Lens or recommended playbook displays. The label chooses a review posture; it is not a new mode, gate, Approval, Write Authorization, evidence, verification, Manual QA, acceptance, residual-risk acceptance, or close.

For review output, keep the two questions separate:

- Spec Compliance Review: did we build the requested thing under current scope and authority?
- Code Quality / Stewardship Review: is the result maintainable and coherent in the codebase?

Same-session review is self-check or stewardship signal unless a qualifying independent Eval or verification record exists. It may find Decision Packet candidates, evidence gaps, Eval or verification needs, Manual QA needs, residual-risk candidates, Approval needs, Change Unit update recommendations, or close blockers, but those findings must route through the existing paths before affected writes or close proceed.

## Product writes

Before writing product files, the agent must check write authority for the intended operation.

Show a short Write Authority Summary:

```text
Write authority: allowed for src/auth/login.ts and tests/auth/login.test.ts
Scope basis: email login Change Unit
Limitation: cooperative surface; changed-path validation detects violations after the fact
```

For external side effects, separate the before-action claim from the after-action record. Before action, say the intended effect, sensitive category, Approval or Decision Packet need, and guarantee level. After action, say what actually happened, which Run/artifact/evidence refs were recorded, and whether anything was redacted, omitted, blocked, stale, or a violation. Exact guarantee-level semantics are owned by [Runtime Architecture Reference](../reference/runtime-architecture.md#guarantee-levels).

Do not describe a cooperative or detective hold as if it blocks execution. Say that writes are held by instruction, or that violations can be detected after action when the connected profile supports that validation. Use preventive wording only for proven pre-execution blocking on the covered operation.

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

Use refs-first evidence display. Cite Evidence, Run, Eval, Manual QA, artifact, log, screenshot, diff, or trace refs with a short outcome, and embed excerpts only when the user or evaluator needs to inspect the content to decide the next action.

## Verification, Manual QA, residual risk, acceptance

Keep these separate in the agent response.

| Item | What the user should understand |
|---|---|
| Evidence | What supports the claim that a result or acceptance criterion was met. |
| Verification | What checked correctness, and whether the verifier was independent enough for detached assurance. |
| Manual QA | What a person inspected because human judgment matters. |
| Acceptance | Whether the user accepts the result when that judgment is required. |
| Residual risk | What uncertainty, limitation, unchecked condition, or trade-off remains. |

Verification answers how the work was technically checked. Same-session self-review is useful, but it is not detached verification. Passing tests can be evidence and can support verification, but tests alone do not prove Manual QA happened.

Manual QA answers whether a person inspected qualities that need human judgment, such as UX, workflow, visual result, copy, or accessibility interpretation. Do not present a browser smoke run, screenshot capture, or verifier note as Manual QA unless a Manual QA result was actually recorded or validly waived.

Residual risk is a known remaining limitation, uncertainty, unchecked condition, or trade-off. It must be visible before risk-accepted close or final acceptance. Risk acceptance does not upgrade assurance and does not replace verification or QA.

Final acceptance is the user's acceptance of the result when the task path requires it. It is not the same as approval, verification, QA, residual-risk acceptance, or proof of correctness.

Applied close examples:

- Direct work: show changed files, evidence refs, self-check, and whether anything escalated. Do not call it detached verified without a qualifying Eval.
- UI/UX work: keep tests, browser smoke, Manual QA, and acceptance on separate lines. If Manual QA is waived, show the skipped surface, accepted risk, and follow-up.
- Auth or security work: show approval separately from the security or product decision, then show evidence and verification. Approval to touch a secret or permission does not settle redaction, audit, role, retention, or user-notice choices.
- Public API work: show caller compatibility, migration or documentation impact, evidence, and verification separately. Passing tests does not by itself settle the API contract decision.
- Risk-accepted close: show the limitation, existing evidence, missing or waived verification or QA, accepted risk, and follow-up. Do not present the result as detached verified.

## Close

Close only when blockers are clear for the active task path.

For small `direct` work, keep the result low-ceremony: request, scope, changed files or no-file outcome, checks, escalation status, and any close-relevant risk or follow-up.

For `work` tasks, the close summary must make the close basis visible. Show changed scope, evidence coverage, verification, Manual QA, residual risk, acceptance, and close reason when applicable. If a gate is waived, `not_required`, failed, pending, or blocked, the close display should say so instead of folding it into a generic success line.

Use the close display that matches the task shape: `DIRECT-RESULT` is the compact result display for direct work, `TASK` Close Summary is continuity display for active or recently closed `work` tasks, and Journey Card close context is compact status/resume display. None of these displays creates state, gates, acceptance, QA, verification, residual-risk acceptance, close, or write authority.

Before successful close, show or confirm:

- scope match
- evidence coverage or no evidence requirement
- verification status, or `verification_gate=waived_by_user` with the relevant Decision Packet when required
- Manual QA status or valid waiver
- close-relevant residual risk shown or no known close-relevant residual risk reported
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
Blocked on user-owned product judgment: the empty state can either invite setup now or keep the page quiet until data exists. Recommendation: invite setup now, low uncertainty. If you defer, I can finish the data-loading refactor but not the empty-state UX.
```

Bad decision stop:

```text
Need approval to continue.
```

Good approval summary:

```text
Approved for: install zod and update the package lock within CU-03. Not decided: whether validation should become part of the public API. Next safe action: retry prepare_write for the exact dependency-file write.
```

Bad approval summary:

```text
Approved, so I will finish the auth design, implement it, deploy it, and close.
```

Good close block:

```text
Close is blocked by a user-owned Manual QA judgment for the onboarding copy. Smallest unblocker: record whether the copy is acceptable. Secondary blocker: AC-02 evidence is still missing, which the agent can collect after Manual QA.
```

Bad close block:

```text
The state model does not satisfy gates.
```

Bad close claim:

```text
Tests passed, so Manual QA and acceptance are complete.
```

Good direct result:

```text
Done as direct. Scope was one settings label; account behavior stayed out of bounds. Changed `src/settings/Profile.tsx`. Self-check passed with the existing copy test. No escalation, no known close-relevant residual risk.
```

Good work close summary:

```text
Close summary: changed scope stayed inside login form, login API call, and session storage. Evidence covers AC-01 and AC-02 via RUN-018 and EVIDENCE-009. Verification is self-checked only; no detached Eval was required for this path. Manual QA passed for final copy and layout. Residual risk: mobile Safari was not checked, accepted in DEC-022 with follow-up. Final acceptance recorded. Close reason: completed with accepted residual risk.
```

Good write hold:

```text
I am holding product writes because write authority is not available for the new auth files. Smallest unblocker: update scope to include those files or reduce the change to the existing login module.
```

Bad write hold:

```text
Harness is unhappy, so I stopped.
```

Good MCP-unavailable hold:

```text
I cannot reach Harness/Core from this surface, so I am holding product writes and will not claim state changes, gate updates, Approval, result acceptance, residual-risk acceptance, or close. Smallest unblocker: reconnect Core or continue from a capable surface.
```

Good projection-stale recovery:

```text
The Journey Card is stale, but Core state is current. I will refresh or reconcile the readable view, then continue from Core state instead of treating the old Markdown as authority.
```

Good managed-block response:

```text
The generated report's managed block was edited by hand. I will treat that edit as a reconcile proposal; it is not state until Reconcile or another Core state-changing path records it.
```

Good review boundary:

```text
Same-session review found one stewardship concern and no scope mismatch. This is a self-check signal, not detached verification; detached verification would need a qualifying Eval or independent review boundary.
```
