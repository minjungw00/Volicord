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

Agents translate ordinary user requests into Harness procedures. Do not require users to say Discovery, Change Unit, Decision Packet, Write Authorization, Evidence Manifest, Projection, Autonomy Boundary, or `task_events` before the work can proceed. Use those internal terms where agent/runtime behavior needs precision, and place them after the plain-language explanation when showing user-facing status.

Treat requests like these as complete user input, not as invitations to demand Harness terminology:

```text
I want to add an email login flow. Keep password reset out of scope for now and help me clarify the decisions first.
Review this feature idea and ask the questions needed before implementation.
Make a small copy change, but tell me if it turns into a broader product decision.
Before changing code, separate the product decisions from the technical decisions.
```

The agent response should translate the request into understood scope, what the agent can inspect itself, what only the user can decide, what evidence would be needed, and what blocks close. Exact Harness labels can follow only when they clarify a boundary or source ref.

A useful status or next-action response answers four questions in ordinary language:

- Scope: what may change, and what is out of bounds?
- Judgment: what, if anything, must the user decide?
- Evidence: what has already been checked, and by which refs?
- Close Readiness: what remains before verification, Manual QA, acceptance, residual-risk handling, or close?

Render gate state through four user-facing display groups: Scope, Judgment, Evidence, and Close Readiness. Explain the easy concept first, then add exact internal terms or refs only when they clarify a boundary, blocker, source ref, or runtime rule. These are display groups only; they do not replace kernel gates, add schema fields, change recompute rules, authorize writes, satisfy gates, accept residual risk, or close the Task. Exact gate values, recompute behavior, and close semantics are owned by [Kernel Reference](../reference/kernel.md#gates) and [`close_task`](../reference/kernel.md#close_task).

The turn context should stay compact, current, and phase-filtered. Start from the ten-item compact context rule set in [Agent Integration Reference](../reference/agent-integration.md#context-pushpull-principles), then show only the current phase's relevant envelope fields: for example the current status or Journey Card, four display groups, next safe action, primary blocker, active Change Unit or Decision Packet refs, Write Authority Summary, evidence or residual-risk refs, guarantee/MCP availability, and projection freshness when they are relevant to the next safe action. Do not treat this as a checklist that sends the entire field set on each turn. Evidence, Run, Eval, Manual QA, artifacts, logs, screenshots, diffs, old projections, older PRDs or designs, module maps, and large traces should appear as refs and short outcomes by default, then be pulled only when the next action requires inspecting them. Stale chat memory and retrieved context can point to refs to inspect, but they cannot authorize writes, satisfy gates, accept results, close tasks, or replace current state.

Use progressive context loading instead of reading the whole documentation set into the agent prompt. The detailed context contract is in [Agent Integration Reference](../reference/agent-integration.md#context-pushpull-principles); in this user-facing flow, keep the phase bundle narrow:

| Phase | Push now | Pull only if needed |
|---|---|---|
| Intake | Current status or Journey Card, likely work shape, four display groups, next safe action, primary blocker. | Task history, user guide, or Reference docs needed to classify the request. |
| Discovery | Clarification summary, blocking questions grouped by decision area, inspectable facts, assumptions, user-owned judgment candidates, QA/verification expectations, current source refs, and first implementation candidate or work split. | Repo docs, module/interface/domain refs, older PRDs/designs, or decision guidance needed to separate inspectable facts from user decisions and scope safe next work. |
| Write | Active Change Unit, Autonomy Boundary, intended paths/tools/commands summary, Approval status, active Decision Packets, Write Authority Summary. | Exact `prepare_write`, Kernel, security, approval, or policy references needed for the intended write. |
| Evidence | Run summary, Evidence Manifest ref, artifact refs, evidence gaps, next evidence action. | Logs, diffs, screenshots, traces, or artifact/evidence contract details needed to interpret or repair support. |
| Verification | Acceptance criteria, changed files, evidence refs, artifact refs, relevant decisions, residual-risk summary, Manual QA requirement, independence/freshness notes. | Full evaluator bundle material, source files, logs, Eval/Manual QA contract details, or verification guidance. |
| Close | Close-readiness summary, blockers, evidence/verification/QA/acceptance status, residual-risk summary or accepted refs, projection freshness. | Exact close, acceptance, residual-risk, QA, verification, or artifact details behind a blocker. |

Retrieved, indexed, remembered, or summarized context stays read-only. It can suggest what to inspect, but it cannot authorize writes, satisfy gates, create evidence, perform verification, accept risk, close a Task, or make any other authority claim.

## Session start

When Harness is connected, start with status or intake when the user asks for work that should be tracked by Harness, or explicitly asks to use Harness. The user does not need to say "Harness." Infer from the request shape and keep the first response short.

Track ordinary-language requests when their shape suggests scope, judgment, evidence, or close state should stay visible:

- product writes or state-changing project work
- scope drift risk or ambiguous requirements
- multi-file, structural, migration, or cross-boundary work
- changes to public APIs, public interfaces, domain language, module boundaries, or shared design that other people, callers, docs, or future work may rely on
- sensitive or policy-relevant areas such as auth, security, billing, destructive/data-loss risk, privacy, compliance, accessibility, or design quality
- user-owned product judgment or material technical judgment with cost, compatibility, security, maintenance, migration, interface, dependency, or risk impact
- evidence, verification, Manual QA, acceptance, or residual-risk needs

Keep small changes light. Do not add ceremony just to answer a question, inspect code, explain a result, or handle a tiny low-risk change with an already narrow shape. A typo, one docs sentence, or an obvious rename can use the internal tiny profile under `direct` when no user-owned judgment, sensitive category, security boundary, or evidence beyond the tiny changed-path/self-check note is hiding inside it. User-facing display should say the plain scope, result, and check, not expose the internal profile unless it clarifies a boundary.

Show:

- the active or likely Task id when useful, plus the plain work shape: read/advice work, small change, or tracked work; include `advisor`, `direct`, or `work` only as diagnostic or power-user detail
- Scope: the current or proposed scope, what is out of bounds, and any active Change Unit or write-authority boundary that affects the next action
- Judgment: any user-owned question, Decision Packet, or sensitive-action Approval that blocks progress
- Evidence: supporting refs, missing support, stale support, or checks already run
- Close Readiness: verification, Manual QA, residual-risk, acceptance, and close-blocker status when those affect the next decision or close
- the next safe action
- the primary blocker, who owns the next move, and the smallest unblocker
- secondary blockers only when they still affect the follow-on path
- write authority status when writes are possible or near
- guarantee level and what the surface can actually block or only detect, as display and risk context rather than sensitive-action Approval, verification, acceptance, or a gate
- optional raw gate names or refs only when they clarify a boundary; do not make the user read the full gate taxonomy to understand the next action
- projection freshness status
- when guard, freeze, or careful mode is relevant, what can actually be blocked before execution and what can only be detected after action

Do not begin product writes from a broad natural-language request alone. First establish scope and compatible write authority for the intended change.

## Resume

Before significant work resumes, read Harness state and show the current position. Resume from current Core state and owner records, not old chat, stale status text, or remembered prior recommendations. Stale chat memory may identify refs to inspect, but it cannot authorize writes, close tasks, accept results, waive checks, accept residual risk, or replace current state.

A good resume response says:

```text
I found the active task. Current scope is X. The next safe action is Y. Product writes are not authorized yet. One decision is pending: Z.
```

If projection, `source_state_version`, or readable status is stale or unknown, say that and refresh or reconcile before depending on it. If canonical state is available directly, the agent may continue from that state while warning that the readable projection is not the source of authority.

Keep display failures separate. A stale projection means the readable card/report may lag and needs refresh or reconcile before it becomes dependable context. Stale state, baseline, or evidence means the underlying inputs moved or became insufficient and may block writes or close. MCP unavailable means the agent cannot reach the required Harness/Core capability; do not claim authoritative state changes, Approval, result acceptance (Acceptance), residual-risk acceptance, gate updates, projection repairs, or close until that capability is available again.

If Core itself is unreachable, the display issue is `MCP_SERVER_UNAVAILABLE`: say Core cannot be reached and reconnect or diagnose before claiming state changed. If Core or the operator can tell that the current surface lacks usable MCP, the display issue is `SURFACE_MCP_UNAVAILABLE`: say this surface cannot use the required Harness tools, then hold writes by instruction or switch to a capable surface. Surface name alone does not prove capability. Only say execution was blocked before action when a preventive guard has proven pre-tool blocking for that covered operation.

## Reading status and blockers

Use MCP results as the source, then speak in user terms.

The exact error taxonomy, complete mapping, and precedence stay in [MCP API And Schemas](../reference/mcp-api-and-schemas.md). This section gives short display examples for common session responses; it is intentionally not exhaustive.

Status and blocker displays should put the four groups before raw gate detail:

| Display group | Show first | Typical owner refs |
|---|---|---|
| Scope | What may change, what is out of bounds, and whether the intended write fits. | Task, Change Unit, Autonomy Boundary, Write Authorization. |
| Judgment | What the user must decide or approve before progress can continue. | Decision Packet, Approval, Acceptance Decision Packet, Residual Risk. |
| Evidence | What supports the claim, what is missing, and whether support is stale. | Evidence Manifest, Run, artifact refs, Eval input refs. |
| Close Readiness | What remains before close can be attempted or accepted. | Eval, Manual QA, Acceptance, Residual Risk, close blockers. |

These groups are not gate aliases and do not define exact enum values. When exact gate names are useful, show them after the plain group summary and link or cite the owner record.

- `harness.status` means "where are we now?"
- `harness.next` means "what is the next safe action or smallest unblocker?"
- `harness.prepare_write` means "may this exact product write happen now?"
- `harness.record_run` means "what happened, what evidence changed, and what is next?"
- `harness.close_task` means "can this Task finish or cancel now?"

`harness.status`, `harness.next`, compact status cards, and recommendation lines are read-only displays. They can recommend a Decision Packet, `prepare_write`, evidence collection, verification, QA, reconcile, or close attempt, but the recommendation itself does not mutate state, authorize writes, satisfy gates, accept results, accept residual risk, or close the Task.

When `harness.next` returns an `action_kind`, render the plain action before the enum. Use the exact enum only when it helps a power user or explains a boundary:

| `action_kind` | Say to the user |
|---|---|
| `ask_user` | A user-owned answer is needed; show the focused question, recommendation, impact, and refs. |
| `prepare_write` | Check write authority for the exact intended write. |
| `implement` | Continue the scoped implementation path; for product writes, use only current compatible Write Authorization. |
| `launch_verify` | Start or prepare an independent verification path from current evidence refs. |
| `record_eval` | Record the evaluator result; do not claim detached verification until the Eval qualifies. |
| `record_manual_qa` | Record a human QA outcome or valid waiver; do not treat browser artifacts alone as Manual QA. |
| `request_acceptance` | Ask whether the user accepts the result after evidence, verification, QA, and residual-risk visibility are shown. |
| `close_task` | Attempt close through the close path and be ready to show blockers. |
| `reconcile` | Refresh or reconcile stale display, managed-block drift, or proposal/state mismatch. |
| `idle` | No immediate Harness action is needed for this focus. |

The exact enum and API contract are owned by [`harness.next`](../reference/mcp-api-and-schemas.md#harnessnext). This table is display guidance, not a new route or gate.

Every authority claim in status, next, result, acceptance, or close display must be traceable to its source ref or explicit absence. Use a Write Authorization ref for "write allowed," an Approval ref for sensitive-action permission, an Evidence Manifest ref for evidence sufficiency, an Eval ref for detached verification, a Manual QA record or valid waiver ref for Manual QA, an Acceptance Decision Packet ref for result acceptance (Acceptance), Residual Risk refs or `ResidualRiskSummary.status=none` for residual-risk visibility, and artifact refs for logs, diffs, screenshots, traces, or bundles. If the ref is missing, say the claim is not yet supported.

When a response contains errors or blockers, lead with one primary blocker. Use the first `ToolError` chosen by API precedence, or the first `close_task` blocker when close returned blockers. Then show the smallest unblocker in ordinary language. Keep secondary blockers visible only when they will still matter after the primary blocker is resolved.

Every blocker display should also name ownership in user-facing terms:

- User-owned: product direction, material technical direction, sensitive-action Approval, Manual QA judgment or waiver, residual-risk acceptance, result acceptance (Acceptance), or another choice the user must make.
- Agent-resolvable: refresh or reconcile status, retry `prepare_write`, collect missing evidence, run an in-scope check, repair or replace an artifact, or narrow the Change Unit without changing user-owned judgment.
- Surface or system: Core unavailable, surface MCP unavailable, capability insufficient, or another condition that needs reconnection, a different surface, or operator repair.

Do not ask the user to resolve an agent-resolvable blocker. Say what the agent will do next, unless that action would change scope, require Approval, or create new user-owned risk.

Common display examples:

| Raw condition | Say first | Smallest unblocker |
|---|---|---|
| `STATE_CONFLICT` | State changed since this view. | Refresh status and retry with the current state version. |
| `MCP_UNAVAILABLE` with `details.mcp_unavailable_kind=server_unavailable`, or diagnostic `MCP_SERVER_UNAVAILABLE` | Core cannot be reached. | Reconnect or diagnose Core access before claiming state changes. |
| `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with `details.mcp_unavailable_kind=surface_mcp_unavailable`, or diagnostic `SURFACE_MCP_UNAVAILABLE` | This surface cannot use the required Harness tools. | Repair the surface or switch to a capable surface; hold writes by instruction unless the profile has proven pre-tool blocking for the covered operation. |
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

Intake turns an everyday request into a usable task shape without forcing the user to speak Harness. The user may say "add email login and keep reset out of scope"; the agent should translate that into a plain work shape, scope, possible decisions, evidence needs, write checks, and close-readiness handling.

Discovery is the agent's conditional requirements-clarification behavior before implementation planning and before write authority. It is not primarily a user command. Users can trigger the same behavior with plain language such as "clarify the plan before implementation" or "ask what you need before changing code." Use it when clarification is needed because the request is ambiguous, feature-shaped, auth/security-sensitive, UX/copy/workflow-heavy, public-interface or module-boundary-facing, likely to touch policy, or likely to become tracked work; do not add it as ceremony for an obvious small change. It is not approval, sensitive-action Approval, Write Authorization, evidence, verification, QA, acceptance, residual-risk acceptance, close, scope authority, or a new authority path.

Listen for the same task-shape triggers used at session start: product writes, scope drift risk, ambiguous requirements, multi-file or structural work, sensitive or policy-relevant areas, user-owned judgment, and evidence, verification, Manual QA, acceptance, or residual-risk needs. When one appears, translate the ordinary request into a proposed work shape, scope, out-of-bounds area, and next safe action.

The intake route is:

```text
Request -> classify task shape -> clarify requirements when needed -> produce Discovery Brief or equivalent support -> route user-owned decisions -> propose safe next work, a first implementation candidate, or a work split -> prepare_write path when product writes are intended
```

Treat Discovery outputs as support or projection concepts that feed existing owner paths unless an owner reference already records the underlying fact:

- Discovery Brief: compact summary of goal, user value, scope, non-goals, acceptance criteria, facts the agent can inspect from repo/docs/Harness state, judgments only the user can make, product/UX judgment candidates, technical architecture judgment candidates, security/privacy judgment candidates, QA and verification expectations, open assumptions, and a first implementation candidate or work split.
- Question Queue: ordered questions classified as blocking, useful-but-not-blocking, or codebase-answerable.
- Assumption Register: assumptions the agent is using, with source, confidence, owner, and what would change if the assumption fails.
- First Safe Change Unit Candidate: the internal Change Unit-shaped version of the first implementation candidate when product writes are near. It is an advanced/support concept, not the only Discovery output or primary stop condition.

Plain phrases such as "first implementation candidate" and "work split" are proposal/support phrases, not standalone schema fields, canonical record types, gate values, projection kinds, or authority paths.

Route Discovery results into Shared Design, Decision Packet candidates, and Change Unit shaping. Do not treat a Discovery Brief, Question Queue, Assumption Register, or First Safe Change Unit Candidate as scope authority, sensitive-action Approval, Acceptance, residual-risk acceptance, evidence, close readiness, or Write Authorization.

Outside Discovery, ask only questions that change the next safe action. During Discovery, ask targeted questions when they clarify goals, user value, scope, non-goals, acceptance criteria, product/UX behavior, technical architecture, security/privacy posture, QA or verification expectations, first implementation candidates, work splits, user-owned judgment, or hidden assumptions. Group questions by decision area instead of dumping a long questionnaire, and make uncertainty explicit. Park useful-but-not-blocking questions instead of interrupting the user. Prefer the most blocking decision area with a recommendation over a long form.

Before asking, inspect repo, codebase, docs, and Harness state that are available and current for answers the agent can discover safely. Do not ask the user to restate existing file paths, behavior, terminology, or constraints that are already visible from current context. If a source is unavailable or stale, say so rather than relying on it as authority.

One blocking question at a time does not mean one clarification round total. Broad or design-heavy requests may need several short turns until the goal, user value, scope, non-goals, acceptance criteria, affected product areas, user-facing screens or flows, modules, interfaces, sensitive categories, user-owned product or material technical trade-offs, security/privacy choices, verification or Manual QA expectations, and known product, implementation, verification, QA, or follow-up risks are shaped enough to propose safe next work. Discovery may ask multiple targeted questions. It can pause once the agent has separated what it can inspect from what the user must decide and has enough scoped information to propose safe next work, a smaller scope, or a work split.

Classify each open question before asking it. Blocking questions need user judgment before the next safe action. Useful-but-not-blocking questions can be parked in the Discovery Brief, Assumption Register, follow-up work, or later Decision Packet candidate. Codebase-answerable questions should be answered by inspecting current repo, docs, Harness state, or source refs instead of asking the user.

Each user-owned question should name the exact choice, offer realistic options, include the agent's recommendation, state uncertainty, identify affected gates or acceptance criteria when they matter, point to source refs and evidence, risk, or design refs when available or relevant, and say what can continue if the decision is deferred, or why nothing should continue until the decision is made. Record assumptions the agent makes separately from product, technical, security, QA, operational, scope, approval, acceptance, or residual-risk acceptance that belongs to the user.

Examples:

```text
Judgment domain: Product / UX (`product_ux`)
Decision area: failed-login behavior.
Options: inline layer, toast, or modal.
Recommendation: inline layer near the form, pending inspection of existing form patterns.
Uncertainty: existing accessibility patterns may make another option cheaper.
Can inspect first: current login UI and validation components.
```

```text
Judgment domain: Technical architecture (`technical_architecture`)
Decision area: authentication architecture.
Options: session cookie, bearer/JWT, OAuth/OIDC, or social-login provider integration.
Recommendation: inspect the current user/session model before choosing.
Uncertainty: storage and session support may make one option much safer than the others.
Can continue if deferred: read-only inspection and a scoped proposal; not implementation.
```

Good intake:

```text
I can keep this as a small change if it stays inside the settings copy. If it also changes account behavior, it becomes tracked work. Recommendation: start with settings copy only. Is that the intended scope?
```

## Classify the work shape

Lead with the plain work shape. Keep `advisor`, `direct`, and `work` as internal routing labels owned by the kernel contract, not labels the user must learn.

| Plain work shape | Internal mode | Use it for | Escalate when |
|---|---|---|---|
| Read/advice work | `advisor` | Reading, explaining, comparing, reviewing, and decision support without product writes. | Product files may change, a sensitive action is needed, or the user asks to turn advice into implementation. |
| Small change | `direct` | Small, low-risk code or docs changes with narrow scope and lightweight evidence. Tiny typo, one-sentence docs, and obvious rename edits are a subprofile, not a new mode. | Scope is unclear, multiple files or subsystems are involved, product/UX judgment is needed, important architecture judgment is needed, public interface/API impact appears, security/privacy impact appears, a sensitive action appears, QA or verification requirements increase, evidence is insufficient, residual risk is non-trivial, or multi-step delivery is needed. |
| Tracked work | `work` | Feature work, UX workflow, auth-facing behavior, schema, public API/interface, structural change, risky fix, multi-file/multi-step delivery, or work needing meaningful evidence and independent verification. | Keep it tracked; when auth, security, privacy, secrets, infrastructure, or similarly sensitive areas appear, route approvals, Decision Packets, evidence, verification, QA, and residual risk through their owner paths. |

The exact mode/profile contract is owned by [Kernel Reference](../reference/kernel.md#work-modes). These plain work shapes are display guidance; they do not add schema values or change authority rules.

If a small change grows, move the same Task to tracked work and show why in ordinary language.

## Small-change ceremony budget

Small change is a lightweight user experience, not a lower authority path. Keep the visible budget to the smallest useful set:

- state the narrow scope in ordinary language
- name out-of-bounds behavior, files, or decisions when they are relevant
- record or select the internal minimal Change Unit before product writes, but show "narrow scope" or "write authority" to the user only when useful for decision-making and trust
- use compatible `prepare_write` before the exact product-file write attempt when product writes apply
- report changed paths, the self-check or other lightweight evidence, escalation status, and close-relevant risk

For a tiny change, the visible budget may be even smaller: the trivial scope, changed path or no-file result, and self-check. That small display is not an authorization shortcut. The internal tiny profile under `direct` still respects active scope, compatible `prepare_write` when product writes apply, user-owned judgment, sensitive-action Approval, security and privacy boundaries, residual-risk visibility, and close rules.

Do not create a Decision Packet, require Manual QA, request detached verification, or show a full close checklist unless the task shape, policy, changed surface, detected risk, or user request makes that necessary.

Escalate the same Task to tracked work when the target stops being obvious, scope is unclear, the changed paths cross the active Change Unit, the edit affects multiple files, product areas, or subsystems, the change may alter a public API or module contract, product/UX judgment is needed, important technical architecture judgment is needed, security/privacy impact appears, a sensitive action appears, QA or verification requirements increase, evidence is insufficient, residual risk is non-trivial, or multi-step delivery is needed.

## Scope and Change Unit

Before product writes, shape the active scope into a Change Unit. The user-facing explanation should answer:

- included behavior or files
- out-of-bounds behavior or files
- completion conditions
- known sensitive areas
- when the agent must stop and ask

Enough is known to propose safe next work when the agent can state those items without hiding unresolved user judgment and can separate inspectable facts from user-owned decisions. If that cannot be done yet, continue Discovery with the next grouped blocking question, park useful-but-not-blocking questions, answer codebase-answerable questions from current sources, or propose a smaller implementation candidate or work split that avoids the unresolved area. A First Safe Change Unit Candidate may be the internal expression of that proposal when product writes are near, but it is not the only or primary Discovery stop condition.

Autonomy Boundary is not write authority. It only describes what judgment the agent may exercise without asking again. Change Unit scope answers where and what the work may change; Autonomy Boundary answers which choices the agent may make inside that scope. Actual product writes still require a compatible write check.

Use this distinction when explaining stops and permissions:

| Concept | Plain question | Allows | Does not allow |
|---|---|---|---|
| Change Unit scope | What work area is in bounds? | Names the behavior, files, paths, tools, commands, network targets, and sensitive categories the work is scoped around. | Does not decide user-owned product or material technical judgment or create Write Authorization by itself. |
| Autonomy Boundary | What may the agent decide alone inside that scope? | Lets the agent choose covered implementation details without another user decision. | Does not grant paths, tools, commands, network, secrets, sensitive categories, sensitive-action Approval, or write authority. |
| Approval | May this sensitive step proceed? | Allows a named sensitive action within its recorded scope and expiry. | Does not decide user-owned judgment, prove correctness, accept residual risk, or create Write Authorization. |
| Decision Packet | What user-owned judgment is being recorded? | Resolves, defers, rejects, or blocks the named product, material technical, waiver, acceptance, residual-risk, or reconcile choice. | Does not grant sensitive-action Approval unless it is the approval-shaped packet linked to an Approval record. |
| Acceptance | Is the result acceptable when Acceptance is required? | Records the user's result judgment after close-relevant residual risk is visible or confirmed absent. | Does not replace evidence, verification, Manual QA, Approval, Write Authorization, or residual-risk acceptance. |
| Residual-risk acceptance | Is this known remaining risk acceptable for close? | Records acceptance of visible close-relevant risk and supports residual-risk accepted close when other gates allow it. | Does not create detached verification, prove correctness, waive QA, or make the close a normal no-risk close. |
| Write Authorization | May this exact write attempt happen now? | Records that Core allowed one compatible write attempt after the required checks. | Is not reusable and does not expand scope, Autonomy Boundary, or Approval. |

For small changes, the internal active Change Unit may be generated from the user's request and surrounding context. Do not require the user to see "Change Unit" language for every tiny edit; show it only when it explains scope, write authority, or a blocker. Keep examples explanatory, not schema-defining:

- Docs or copy edit: purpose "change this phrase"; non-goals "no behavior or contract change"; scoped paths "the named doc/component and related test if present"; stop if "meaning, localization strategy, or public promise changes."
- Focused test edit: purpose "cover the reported case"; non-goals "no implementation refactor"; scoped paths "the relevant test"; stop if "the fix requires product code."

When a prompt or status uses the word "approved," name the exact authority or recorded decision: sensitive-action Approval, scope confirmation, Decision Packet resolution, residual-risk acceptance, result acceptance (Acceptance), or Write Authorization status. Do not use "approved" as a catch-all label.

Examples:

- Dependency install sensitive-action Approval: Approval to run the install or update dependency files does not decide that the new dependency is the right architecture choice. If that choice affects compatibility, rollback, cost, or maintenance, use a Decision Packet.
- Secret access sensitive-action Approval: Approval to read or use a secret inside the requested scope does not permit exposing secret values in artifacts, projections, exports, logs, screenshots, or summaries.
- Auth/system change sensitive-action Approval: Approval to touch auth files, permissions, or system configuration does not choose the identity-provider or session/storage model, such as local session cookie, bearer token/JWT, OAuth/OIDC sign-in, or social-login provider integration; it also does not decide role model, lockout behavior, or user notice.
- Public API change decision: resolving the API direction decides the contract choice for the Task; it is not deployment authority, merge authority, or a reusable Write Authorization.
- Result acceptance (Acceptance): accepting the result does not authorize more writes, approve new sensitive actions, or retroactively satisfy missing evidence, QA, verification, or Write Authorization.

Use Shared Design to record the shared understanding from Discovery: goal, user value, scope, non-goals, assumptions, domain/module/interface impact, separated user-owned judgments, QA/verification expectations, and safe next work. Do not present Shared Design as sensitive-action Approval, Acceptance, residual-risk acceptance, evidence, close readiness, or Write Authorization. If Shared Design exposes a public API/interface choice, domain-language conflict, module boundary move, architecture direction, security/privacy trade-off, QA/verification waiver, scope expansion, or known-risk waiver that the user owns, route that choice to a Decision Packet.

Inside the Autonomy Boundary, the agent may decide ordinary implementation details: whether to reuse an existing helper, how to split a private function, where to place focused tests, or which conservative internal approach best fits the agreed result. The agent must stop for user judgment before public API or module contract changes, security or privacy trade-offs, UX or product trade-offs, material technical direction such as dependency or migration choices, scope expansion, or residual-risk acceptance.

## Blocking user-owned judgment

When user-owned product, technical, security/privacy, QA/acceptance, residual-risk, or scope/autonomy judgment blocks progress, show or request a Decision Packet. Do not replace it with broad approval or a vague "continue?" prompt.

The word "approved" or a casual "go ahead" is not enough when the underlying choice is a product trade-off, architecture direction, QA waiver, risk from a verification gap, result acceptance (Acceptance), or residual-risk acceptance. The prompt must name the decision route, what the user is deciding, what is not being decided, the evidence or risk refs, what the agent may decide without the user, and the close or write impact.

A user-facing Decision Packet should include:

- decision title
- judgment_domain: `product_ux`, `technical_architecture`, `security_privacy`, `qa_acceptance`, `residual_risk`, `scope_autonomy`, or `mixed`
- friendly judgment label: Product / UX, Technical architecture, Security / privacy, QA / acceptance, Residual risk, Scope / autonomy, or Mixed
- decision_kind
- why the decision is needed now
- what the user is deciding / exact choice
- options
- trade-offs
- recommendation
- uncertainty
- deferral consequence
- residual risk when relevant
- affected gates and affected acceptance criteria
- source refs and evidence, risk, or design refs when available or relevant
- what the agent may decide without the user
- follow-up when relevant

The judgment domain is a schema-owned reader-facing classification that helps users understand what kind of judgment they are making. Use it as the primary display grouping. If a decision is cross-cutting, use `mixed` or show secondary considerations in trade-offs, affected gates, risk, evidence, or follow-up instead of pretending the domain is exclusive. `decision_kind` controls lifecycle, payload branch, gate meaning, and state-transition semantics; `judgment_domain` controls explanation and grouping. It is not a gate, validator input, close aggregation rule, or authority path. The exact public fields are owned by [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision), and canonical authority is owned by [Decision Packet](../reference/kernel.md#decision-packet) and [Decision Gate](../reference/kernel.md#decision-gate). Do not copy the schema body into user prompts; render the decision in ordinary language and keep refs available for drill-down.

Decision-centered prompts use verbs that match the route: choose, defer, reject, waive, accept, or reconcile. Use "approve" only when the route is a sensitive-action Approval. Good prompt shapes:

```text
Decision: Failed-login feedback pattern
Judgment domain: Product / UX (`product_ux`)
Which failed-login UX should I record for this Change Unit: inline layer, toast, or modal? Recommendation: inline layer because it preserves flow and accessibility. If deferred, I can continue backend auth wiring but not claim the final failed-login UX is done.
```

```text
Decision: Mobile Safari QA waiver
Judgment domain: QA / acceptance (`qa_acceptance`)
Should I record acceptance of the remaining mobile Safari wrapping risk for this close, or keep close blocked until Manual QA runs? Recommendation: keep it blocked unless release timing requires the waiver. Affected group: Close Readiness; owner path/gate ref: Manual QA / qa_gate; affected criterion: AC-03 onboarding copy layout.
```

Useful examples:

- Product / UX (`product_ux`): failed-login feedback should compare inline layer, toast, and modal; recommend one based on flow, accessibility, interruption, and copy risk. If deferred, backend auth work may continue, but the final failed-login experience should not be claimed done.
- Product / UX (`product_ux`): failed-login copy should compare generic, specific, and hybrid wording; recommend one based on account enumeration risk, clarity, recovery usefulness, support burden, and product tone. If deferred, validation wiring may continue, but release-ready copy and Manual QA should stay open.
- QA / acceptance (`qa_acceptance`): product taste and Manual QA need should compare a polished interaction that needs human visual review with a simpler conservative behavior that can be checked by tests and browser smoke. Explain the taste trade-off, QA cost, user impact, and what can continue if Manual QA is deferred, or why nothing should continue until the decision is made.
- Technical architecture (`technical_architecture`): auth approach should compare session cookie, bearer token/JWT, OAuth/OIDC, or social-login provider integration. OAuth/OIDC may still produce a local session or token strategy, so separate identity-provider choice from session/storage model when both matter. Explain revocation, CSRF/XSS exposure, client compatibility, operational complexity, and migration cost. If deferred, form scaffolding may continue only if it does not commit to the session model.
- Technical architecture (`technical_architecture`): dependency choice should separate sensitive-action Approval to install or update dependency files from the architecture decision to adopt the dependency. Compare adding the dependency, using existing utilities, or postponing the capability, and explain compatibility, rollback, cost, and maintenance impact.
- Technical architecture (`technical_architecture`): domain-language conflict should compare preserving the current product term, adding a narrow code alias, or migrating to a new term. Explain product meaning, public docs, API/interface naming, caller expectations, module responsibility, migration cost, and what can continue if the decision is deferred.
- Technical architecture (`technical_architecture`): schema/data-model migration should compare additive migration, compatibility shim, and breaking cleanup. Explain migration evidence, data-backfill risk, rollback path, test boundary, and maintenance cost.
- Technical architecture (`technical_architecture`): public API/interface or module boundary should compare preserving the current interface, adding a narrow extension, or moving responsibility across a module boundary. Explain caller impact, compatibility or breaking-change risk, boundary tests, documentation promises, migration path, and future-change cost.
- Scope / autonomy (`scope_autonomy`): scope or Autonomy Boundary expansion should compare keeping the current small scope, adding the requested surface, or splitting a follow-up Change Unit. Explain affected paths, user-facing behavior, what remains out of bounds, write impact, and what the agent can still decide alone.
- Security / privacy (`security_privacy`): sensitive-action Approval to access a secret, change permissions, or export data is only an Approval boundary. Separate product or security judgment may still be needed for roles, fields, redaction, audit logging, retention, rollback, and user notice.
- Security / privacy (`security_privacy`): PII logging policy should compare options such as no PII in logs, redacted or tokenized identifiers, or limited diagnostic fields. Explain privacy exposure, debugging value, retention, redaction, audit trail, and evidence needed to prove the policy is followed.
- QA / acceptance (`qa_acceptance`): QA or verification waiver should use the existing recording required for the Task and cite the owner refs. QA waiver effects are owned by the Manual QA / QA policy path; product/user risk or policy-required judgment uses a QA waiver Decision Packet. Verification waiver effects are owned by the kernel verification-waiver path; when user-owned judgment is needed, use the relevant Decision Packet. Name the skipped check or surface, user-accepted residual risk, residual-risk follow-up, relevant refs, and close impact. Example: ask the user whether to waive mobile Safari Manual QA for a copy-only change, accept the viewport-wrapping residual risk, and keep a browser pass as release follow-up.
- Residual risk (`residual_risk`): residual-risk acceptance before close should show the remaining limitation, the evidence that does exist, why close can still be acceptable, and the follow-up that remains. A residual-risk accepted close is not a detached-verified close.

Ask one blocking question at a time when possible.

## Review lenses and displays

When the user asks for a product, engineering, design, security, QA, or release-handoff perspective, treat `product-review`, `eng-review`, `design-review`, `security-review`, `qa-review`, and `release-handoff` as Role Lens or recommended playbook displays. The label chooses a review posture, not a new mode, Approval, Write Authorization, gate, or close path; the exact Role Lens boundary is owned by [Agent Integration](../reference/agent-integration.md#role-lens-behavior).

Role Lens and status/next recommendations are guidance until an existing Core/MCP path records the underlying action. They may find Decision Packet candidates, evidence gaps, Eval needs, Manual QA needs, residual-risk candidates, Approval needs, Change Unit update recommendations, or close blockers, but they do not by themselves mutate state or satisfy those routes.

For review output, keep the two questions separate:

- Spec Compliance Review: did we build the requested thing under current scope and authority?
- Code Quality / Stewardship Review: is the result maintainable and coherent in the codebase?

Review Stages are managed display/procedure only. They are not canonical records; they are not new `ProjectionKind` values, Approval, evidence, verification, QA, acceptance, residual-risk acceptance, close, or Write Authorization. Same-session review is self-check or stewardship signal unless a qualifying independent Eval or verification record exists. Findings must route through the existing paths before affected writes or close proceed.

When a check, review, Eval, Manual QA result, or Run produces a finding, name the route instead of leaving the finding in chat:

- Evidence gap or support: update Evidence Manifest coverage and cite Run/artifact/Feedback Loop/TDD refs.
- User-owned product, technical, waiver, acceptance, or risk choice: show a Decision Packet candidate or existing Decision Packet ref.
- Scope, completion, or autonomy mismatch: recommend a Change Unit update, smaller Change Unit, or follow-up Change Unit.
- Stewardship or design-quality issue: show the existing design, decision, QA, evidence, residual-risk, close-blocker, or Change Unit recommendation route that carries the impact.
- Known remaining uncertainty or skipped check: show a Residual Risk candidate or ref before acceptance or residual-risk accepted close.
- QA or verification outcome: point to the Manual QA or Eval record and its gate effect.
- Close blocker: show the structured close blocker and smallest unblocker.
- Follow-up work: create or reference the existing follow-up Task, Change Unit, or Journey continuity route rather than burying the note in a summary.

Feedback Loop is the canonical support-record path for selected loops and loop findings. Exact routing boundaries are owned by [Design Quality Policies](../reference/design-quality-policies.md#finding-routing) and [Kernel Reference](../reference/kernel.md#finding-routing); this Use doc only describes the agent display behavior.

## AFK work and public commitments

When the user says to continue while they are away, treat that as permission to use already-recorded latitude, not as new authority. The agent may continue only inside the active Change Unit, the active Autonomy Boundary, granted sensitive-action Approvals, and compatible `prepare_write` / Write Authorization for each product write.

Stop and surface the smallest unblocker before scope expansion, new sensitive action without Approval, Autonomy Boundary breach, residual-risk acceptance, accepting the result, QA or verification waiver, public API or module contract change, domain-language change that affects public meaning, release/support promise, or other public commitment that users or other systems may rely on.

Name the guarantee level when presenting AFK stops. On cooperative or detective surfaces, "stop" means hold by instruction or detect/report after action if the profile supports that validation. Use preventive wording only when the connected profile proves pre-tool blocking for the covered operation. Careful mode may narrow the posture, but it is not a new authority tier.

## Product writes

Before writing product files, the agent must check write authority for the intended operation.

Show a short Write Authority Summary:

```text
Write authority: allowed for src/auth/login.ts and tests/auth/login.test.ts
Scope basis: email login Change Unit
Limitation: cooperative surface; changed-path validation detects violations after the fact
```

For external side effects, separate the before-action claim from the after-action record. Before action, say the intended effect, sensitive category, Approval or Decision Packet need, and guarantee level. After action, say what actually happened, which Run/artifact/evidence refs were recorded, and whether anything was redacted, omitted, blocked, stale, or a violation. Guarantee level is display and risk context; it does not grant Approval, verify the result, record QA, accept residual risk, accept the result, or close the Task. Exact guarantee-level semantics are owned by [Runtime Architecture Reference](../reference/runtime-architecture.md#guarantee-levels).

Do not describe a cooperative or detective hold as if it blocks execution. Say that writes are held by instruction, or that violations can be detected after action when the connected profile supports that validation. Use preventive wording only for proven pre-tool blocking on the covered operation.

If write authority is blocked, unavailable, stale, or incompatible with the intended change, hold product writes and explain the smallest unblocker.

If observed changed paths fall outside the consumed Write Authorization or active Change Unit, do not summarize them as authorized work. Show the mismatch, hold further product writes, and route to repair: revert or isolate the extra change, request a scope decision, or escalate to tracked work (`work`) when the wider change is now intentional.

Documentation-maintenance edits are a separate docs-only workflow. They are governed by
[Authoring Guide](../maintain/authoring-guide.md), not by the product-write flow described here.

## Evidence and checks

After advice, changes, runs, or review, record the result at the right level of detail. User-facing evidence should map to acceptance criteria or the stated task goal.

Display sufficiency as coverage, not volume. The useful question is which acceptance criteria, completion conditions, or close-relevant claims have current supporting refs. A long artifact list does not make a missing criterion supported, and chat text or Markdown report prose should never be the only proof that evidence is sufficient.

Good evidence display:

```text
Evidence:
- AC-01: login form renders with email field, supported by test run RUN-008.
- AC-02: failed login message appears, supported by RUN-009 and ART-TEST-009; final wording still needs Manual QA.
```

When evidence is missing, name the criterion or claim that lacks support. Do not say only "evidence gate failed."

Use refs-first evidence display. Cite Evidence, Run, Eval, Manual QA, artifact, log, screenshot, diff, or trace refs with a short outcome, and embed excerpts only when the user or evaluator needs to inspect the content to decide the next action.

Task shape changes what "enough" looks like. Read/advice work usually cites source refs or a review bundle only when recorded evidence is requested. A tiny docs-only small change can be supported by the changed path, a one-line patch summary or diff ref, and a self-check that says no meaning changed; if Evidence Manifest coverage, artifact refs, link/render proof, or other evidence beyond the tiny result note is needed, escalate to an ordinary small change or tracked work according to scope. Small docs-only changes can be supported by changed path, diff or patch summary, and self-check. Small code changes add a focused check or a recorded reason no automated check applies. Tracked feature work maps each criterion to Run and artifact refs. UI/UX, workflow, copy, accessibility, product-taste, and visual-output work separates visual or Browser QA artifact evidence from Manual QA judgment. Sensitive work keeps Approval, redaction, and omission refs visible without treating Approval as correctness. Verification-required work needs an Eval that names the evidence reviewed.

If evidence becomes stale, say why in ordinary language and name the smallest repair. Common causes are baseline drift, changed files after the supporting Run or Eval, approval drift or expiry, missing or failed-integrity artifacts, and relevant Shared Design, domain term, module map, or interface contract changes.

## Verification, Manual QA, residual risk, acceptance

Keep these separate in the agent response.

| Item | What the user should understand |
|---|---|
| Evidence | What supports the claim that a result or acceptance criterion was met. |
| Verification | What checked correctness, and whether the verifier was independent enough for detached assurance. |
| Manual QA | What a person inspected because human judgment matters. |
| Acceptance | Whether the user accepts the result when that judgment is required. |
| Residual risk | What uncertainty, limitation, unchecked condition, or trade-off remains. |

Verification answers how the work was technically checked. Same-session self-review is useful, but it is not detached verification. Passing tests can be evidence and can support verification, but tests alone do not prove Manual QA happened. A detached candidate becomes detached verified only after a passing Eval with valid independence and current reviewed inputs.

Use these user-facing labels consistently:

| Label | Use when |
|---|---|
| Self-checked | The implementing path checked its own result. |
| Detached candidate | A fresh session, fresh worktree, sandbox, manual bundle, or qualifying subagent path may be independent but has not yet produced detached assurance. |
| Detached verified | The Eval passed with valid independence, no same-session self-review issue, and no stale baseline or bundle input. |
| Waived with user-accepted residual risk | Verification or another close-relevant check was waived and the visible remaining residual risk was accepted by the user for residual-risk accepted close. |

Manual QA answers whether a person inspected qualities that need human judgment, commonly UI/UX, workflow, copy, accessibility interpretation, product taste, or visual output. Do not present a browser smoke run, screenshot capture, Browser QA Capture artifact, or verifier note as Manual QA unless a Manual QA result was actually recorded or validly waived. Browser QA Capture is a v1+ Expansion candidate unless owner docs explicitly promote it; even when available, its artifacts are supporting refs, not result acceptance (Acceptance) or detached verification unless a separate Eval path also satisfies independence. If browser capture is unsupported for the surface, use human Manual QA notes and manually supplied artifacts.

Residual risk is a known remaining limitation, uncertainty, unchecked condition, or trade-off. It must be visible before residual-risk accepted close or result acceptance (Acceptance). Residual-risk acceptance does not upgrade assurance and does not replace verification or QA.

Residual-risk display must distinguish `status=none` from `not_visible`. `status=none` means Core has no known close-relevant residual risk for the current Task and requested action. `not_visible` means known close-relevant risk exists but has not yet been shown with enough context for acceptance or close, so the next action is to surface that risk and refs. Do not summarize `not_visible` as "no risk."

Acceptance is the user's acceptance of the result when the task path requires it. It is not the same as sensitive-action Approval, verification, QA, residual-risk acceptance, or proof of correctness.

Verification waiver and QA waiver do not upgrade assurance. A verification waiver keeps detached verification unsatisfied. When close is otherwise allowed, it can close only through residual-risk acceptance for the waived verification gap. It must not be summarized as verified close. A QA waiver closes only the QA requirement it names and leaves evidence, verification, acceptance, and residual-risk handling unchanged. Waiver prompts and summaries should show the named requirement, user-accepted residual risk, residual-risk owner refs, residual-risk follow-up when needed, and affected owner path or close impact; exact waiver metadata and gate effects are owned by [Design Quality Policies](../reference/design-quality-policies.md#waiver-rules) and [Kernel Reference](../reference/kernel.md#waiver-semantics).

Applied close examples:

- Small change: show changed files, evidence refs, self-check, and whether anything escalated. Do not call it detached verified without a qualifying Eval.
- UI/UX, workflow, copy, accessibility, product-taste, or visual-output work: keep tests, browser smoke, Browser QA artifacts, Manual QA, and acceptance on separate lines. If Manual QA is waived, show the skipped surface, user-accepted residual risk, and residual-risk follow-up.
- Auth or security work: show sensitive-action Approval separately from the security or product decision, then show evidence and verification. Approval to touch a secret or permission does not settle redaction, audit, role, retention, or user-notice choices.
- Public API work: show caller compatibility, migration or documentation impact, evidence, and verification separately. Passing tests does not by itself settle the API contract decision.
- Residual-risk accepted close: show the limitation, existing evidence, missing or waived verification or QA, user-accepted residual risk, and residual-risk follow-up. Do not present the result as detached verified.

## Close

Close only when blockers are clear for the active task path.

For small changes, keep the result low-ceremony: request, scope, changed files or no-file outcome, checks, escalation status, and any close-relevant risk or follow-up.

For tracked work, the close summary must make the close basis visible. Show changed scope, evidence coverage, verification, Manual QA, residual risk, acceptance, and close reason when applicable. If a gate is waived, `not_required`, failed, pending, or blocked, the close display should say so instead of folding it into a generic success line.

Use the close display that matches the task shape: `DIRECT-RESULT` is the compact result display for small changes, `TASK` Close Summary is continuity display for active or recently closed tracked work, and Journey Card close context is compact status/resume display. None of these displays creates state, gates, acceptance, QA, verification, residual-risk acceptance, close, or write authority.

Close displays should lead with the same four groups: Scope for changed scope, Judgment for acceptance or residual-risk decisions, Evidence for support refs, and Close Readiness for verification, Manual QA, residual risk, close blockers, and close reason. Raw gate names can follow when they explain a blocker, but exact gate values and close semantics remain kernel-owned.

Before successful close, show or confirm:

- scope match
- evidence coverage or no evidence requirement
- verification status, including stale evaluator bundle or baseline drift when present, or the kernel-owned verification-waiver path with the relevant Decision Packet and accepted Residual Risk refs when required
- Manual QA status or valid waiver
- close-relevant residual risk shown or no known close-relevant residual risk reported
- Acceptance recorded when required

Do not ask for result acceptance (Acceptance) from a generic "all good?" prompt. The Acceptance prompt must show the close basis: evidence, verification, Manual QA, residual-risk visibility or `none`, what acceptance does not replace, and whether the requested close is normal close or residual-risk accepted close.

If close is blocked, say exactly why and name the smallest next unblocker.

## Good and bad agent response examples

Good start:

```text
This looks like a small change that can stay light because the requested change is one copy string. Scope: settings page label only. Out of bounds: account behavior and API changes. Next safe action: check write authority for that file, then make the edit and self-check.
```

Bad start:

```text
Approved, I will update whatever is needed and close the task.
```

Good decision stop:

```text
Blocked on user-owned product judgment: choose the empty-state behavior for AC-02. Options: invite setup now, or keep the page quiet until data exists. Recommendation: invite setup now, low uncertainty. If you defer, I can finish the data-loading refactor but not claim the empty-state UX is done.
```

Bad decision stop:

```text
Need sensitive-action Approval to continue.
```

Good sensitive-action Approval summary:

```text
Sensitive-action Approval granted for: install zod and update the package lock within CU-03. Not decided: whether validation should become part of the public API. Next safe action: retry prepare_write for the exact dependency-file write.
```

Bad Approval summary:

```text
Approved, so I will finish the auth design, implement it, deploy it, and close.
```

Good close block:

```text
Close is blocked.
Judgment: user-owned Manual QA judgment for the onboarding copy is still needed.
Evidence: AC-02 evidence is still missing, which the agent can collect after Manual QA.
Close Readiness: close cannot proceed until the Manual QA judgment is recorded.
Smallest unblocker: record whether the copy is acceptable.
```

Bad close block:

```text
The state model does not satisfy gates.
```

Bad close claim:

```text
Tests passed, so Manual QA and acceptance are complete.
```

Good small-change result:

```text
Done as a small change.
Scope: one settings label; account behavior stayed out of bounds; no escalation.
Judgment: no user-owned decision was needed.
Evidence: changed `src/settings/Profile.tsx`; checked the related copy test and diff.
Close Readiness: no close-relevant blocker or known residual risk remains for this small change.
```

Power-user or diagnostic displays may include owner refs such as Write Authorization, Evidence Manifest, Run, artifact, or Residual Risk refs when they help explain authority or support.

Good tracked-work close result:

```text
Close summary:
Scope: changed scope stayed inside login form, login API call, and session storage.
Judgment: user accepted the shown mobile Safari residual risk in DEC-022; result acceptance (Acceptance) recorded in DEC-023.
Evidence: AC-01 and AC-02 are covered by Evidence Manifest EM-009, supported by RUN-018 and ART-TEST-018.
Close Readiness: verification is self-checked in RUN-018; no detached Eval was required for this path. Manual QA passed for final copy and layout in MQA-006. Residual Risk RISK-004 has follow-up TASK-144. Close reason: completed with user-accepted residual risk.
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
I cannot reach Harness/Core from this surface, so I am holding product writes and will not claim state changes, gate updates, Approval, result acceptance (Acceptance), residual-risk acceptance, or close. Smallest unblocker: reconnect Core or continue from a capable surface.
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
