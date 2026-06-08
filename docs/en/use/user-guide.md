# User Guide

Use normal language. Harness should help the agent keep scope, user-owned judgment, evidence, close readiness, and residual risk visible without making you learn internal presentation terms first.

This guide describes intended active user behavior for a future local Harness Server. This repository is still documentation-only; no Harness runtime/server implementation exists here yet. Exact state, API, schema, storage, and template contracts are owned by the [Reference Index](../reference/README.md).

## 1. Start in ordinary language

You should be able to ask for careful work in everyday terms:

```text
Help me make this plan concrete before implementation.
Ask what you need before changing files.
Ask me before deciding product behavior.
Before changing files, tell me what you expect to touch.
Show me what is still blocking the first safe change.
Show what you verified before calling it done.
Record that I approve installing this dependency for this task only.
Close this only if the evidence is sufficient; otherwise tell me what blocks closure.
```

You can set scope the same way:

```text
Add email login, but keep password reset and account creation out of scope.
Fix only typos in this document.
Keep this as a small copy edit unless it becomes a product or technical decision.
```

The agent should translate that into visible working context: the current goal, what is in scope, what is out of scope, what is already known, what only you can decide, what can safely happen next, and what would block close.

Harness may classify ordinary requests as advice/review/planning without product writes, a small direct change, or tracked work. You do not need to choose or say `advisor`, `direct`, `work`, or `auto`; `auto` is only an API input that asks for classification, not a task label shown to you.

You do not need internal Harness labels or API method names before work can begin. If you say "make the plan concrete" or "help me shape this before implementation," the agent should understand that as a request to shape the work before the first implementation step. The agent can explain exact routing only after the natural request is clear.

## 2. Clarify scope before write

Before product files change, the agent should make the working scope plain:

- what you asked for and what the current goal is
- the smallest scope for this change
- what may change
- what is out of bounds
- likely paths, commands, tools, or external actions
- facts the agent already verified
- facts still unknown
- decisions that belong to you
- the next safe action

For anything larger than a tiny obvious edit, the agent should inspect available files, tests, docs, current Harness state, and accepted judgments before asking you. If the answer is discoverable, the agent should discover it. If the work is still too vague to implement safely, the agent should name the ambiguity and ask the one question that changes the next safe action.

You may see a compact readiness view called `ShapingReadiness`. It is not a form you must fill out and not a persistent planning artifact. It is the agent's current view of whether the goal summary, non-goals, affected area or paths, acceptance criteria, what the agent may decide on its own, the first safe work item for this change, user-owned blockers, and next safe action are known enough for the first safe step. Unknowns should remain visible, but they should not hold up that first safe work item when they do not affect it.

In a Harness-connected session, product writes go through a pre-write scope check. In owner terms, the active method is `harness.prepare_write`. A compatible result says the intended write matches current Harness state and active surface capability for the named operation. It is not OS permission, sandboxing, tamper-proof enforcement, arbitrary-tool isolation, or proof that every tool was blocked before action.

If you decide to change the goal, scope boundary, non-goals, acceptance criteria, what the agent may decide on its own, baseline, or active work piece after `harness.intake`, the agent should apply that through `harness.update_scope` before relying on a write check. Existing pre-write checks that no longer match the updated scope become stale.

## 3. Separate facts from user judgment

Facts are things the agent can inspect, verify, or cite. User judgments are choices you own.

| Item | Agent should do | It must not do |
|---|---|---|
| Repository facts | Read files, tests, docs, config, history, or current state when available. | Ask you to restate facts it can safely inspect. |
| Evidence | Show what supports a claim and what is missing. | Treat evidence as your decision or final acceptance. |
| Product judgment | Ask you about user-visible behavior, messages, UX, user flow, accessibility, and product trade-offs. | Pick a material product direction silently. |
| Technical judgment | Ask you about architecture, dependency or external service, authentication, migration, public interface, security, privacy, retention, compatibility, or costly-to-reverse technical choices that matter. | Hide a material technical decision as an implementation detail. |
| Scope judgment | Ask before expanding scope or removing a non-goal, then apply the accepted scope change through `harness.update_scope`. | Treat enthusiasm as permission to broaden the task or as write approval. |
| Close judgment | Ask for final acceptance or residual-risk acceptance only when the close basis is visible. | Turn "looks good" into every pending judgment. |
| Agent-owned implementation detail | Usually decide small refactors, local naming, test file organization, internal cleanup, or details already fixed by accepted scope and acceptance criteria. | Ask for approval on every tiny local variable name, or use "implementation detail" to hide a costly direction change. |

Harness preserves this boundary so the agent can keep moving on ordinary implementation details already inside the accepted scope without replacing your judgment.

## 4. Ask one narrow judgment at a time

A useful judgment request asks for one decision and fits on one screen when possible. It should include:

- the exact decision
- realistic options
- a bounded option when the current facts already support one
- why that option fits the current scope and evidence
- uncertainty
- what can continue if you defer
- what remains blocked
- what the answer does not settle

Example:

```text
Judgment needed: choose failed-login feedback.

Options:
- Existing UI message layer near the form.
- Toast after failed submit.
- Modal that interrupts the flow.

Current bounded option: existing UI message layer. It stays visible and fits the form context.

If deferred: backend error mapping can continue, but final UI behavior, copy, screenshots, and user-visible inspection remain unresolved.

Does not settle: login architecture, account recovery, final acceptance, or residual-risk acceptance.
```

A broad reply such as "go ahead," "approved," or "looks good" applies only to the one active, clearly named judgment. It does not automatically grant sensitive-action approval, accept the final result, change scope, cancel the task, accept residual risk, or settle another future judgment candidate.

The compact active path asks through `harness.request_user_judgment` and records the answer through `harness.record_user_judgment` when a judgment is needed. Ordinary decisions do not require a special presentation format.

## 5. Treat sensitive action approval separately

Sensitive action approval is permission for a named action, not approval of the whole plan. In schema terms, it is recorded as `SensitiveActionScope`; path-level product-file Write Authorization is a separate check.

Examples include installing or updating dependencies, running commands with destructive potential, touching secrets, accessing restricted systems, deploying, sending network requests outside the current scope, or making a change with privacy/security impact.

A good sensitive-action prompt should name:

- the exact action
- the reason it is needed
- the command or tool, if any
- intended paths, if any
- hosts or network destinations, if any
- dependencies and versions, if any
- secret handles, never raw secret values
- the time window
- the scope limit, such as "this task only"
- actions that are explicitly not authorized
- the honest capability claim, such as whether Harness can only record the approval cooperatively, whether the surface can observe the action, or whether the action is not observable
- any safer bounded action already known, if one exists
- what the approval does not settle

Approving a dependency install does not mean the dependency is the right architecture. Approving a deployment command does not mean the product result is accepted. Approving secret access does not accept residual risk. Each of those needs its own judgment when relevant.

Approving a command, dependency change, host, network access, secret handle, deployment, destructive action, or system access also does not mean Harness can observe or block that action. Current MVP wording should say only what the active surface can honestly confirm.

## 6. Record evidence after meaningful action

After meaningful work, the agent should summarize what happened and what supports the claim. In owner terms, the active path may use `harness.record_run` and evidence references when that path is available.

Useful evidence can include changed paths, diffs, command output, test results, screenshots, logs, source links, and inspection notes. If a future owner promotes a separate user-visible inspection path, its notes remain separate later-path material. The summary should say:

- what ran or changed
- what claim each item supports
- what passed, failed, was skipped, or was not applicable
- what evidence is missing, stale, redacted, omitted, blocked, or insufficient
- what was not verified

Evidence does not replace your judgment or final acceptance. Tests do not replace user-visible inspection or any future promoted review path. A screenshot does not prove accessibility. A generated summary does not become operational truth. Raw secrets, tokens, and full sensitive logs should be redacted, omitted, blocked, or represented by safe handles.

Large diffs, logs, screenshots, and similar support may be staged first and promoted into persistent artifact refs later. At the user level, this means a temporary staged attachment is not persistent evidence yet. It becomes persistent evidence support only when a compatible run recording consumes it and links the persisted artifact ref to a claim. A raw file path, copied local path, or "the log is over there" claim is not evidence authority by itself.

For tracked work, the evidence summary should distinguish required close evidence from optional support. In current MVP terms, evidence is sufficient only when every required coverage item is supported or not applicable. Unsupported, partial, stale, blocked, or missing required evidence remains a reason the work cannot be closed yet. A usable artifact can support a claim, but artifact availability by itself is not evidence sufficiency.

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
- the next action that would unblock close

Tests can pass while close is still blocked. A UI change can still have a visible inspection gap. A security-sensitive change can need a risk decision. Missing required evidence remains a blocker until it is gathered, recorded as not applicable with a clear reason, or honestly reported as unresolved.

In owner terms, `harness.close_task` returns blockers or a close result. A close check is read-only: it can show whether close would be blocked, but it should not change Task state. In user terms, the agent should not claim completed close while required scope, evidence, user judgment, final acceptance, residual-risk handling, or reasons the work cannot be closed yet remain unresolved. Separate quality routes are later candidates unless a future owner promotes them.

Close can end as completed, cancelled, or superseded, but those are different outcomes. Completed close requires the required evidence first, then required final acceptance, and then residual-risk acceptance only when a visible close-affecting risk requires it. Cancelled and superseded are honest terminal outcomes, not successful completion, and they do not require evidence sufficiency, final acceptance, or residual-risk acceptance. A failed command, failed derived view, missing artifact, evidence gap, or unresolved blocker should stay visible as the specific problem; it is not a generic failed Task result.

## 8. Accept final result separately from residual risk

Final acceptance means you accept the result you can see. Residual-risk acceptance means you accept a named residual risk that is still visible. They are separate judgments.

Neither judgment substitutes for missing required evidence. If required evidence is unsupported, partial, stale, blocked, or missing, the task still needs an evidence unblocker even if you accept the visible result or accept a named residual risk.

The agent should ask for final acceptance only after the close basis is visible: scope, result, evidence, checks, known gaps, user-visible inspection status when relevant, and blockers. The prompt should name exactly what result you are accepting.

The agent should ask for residual-risk acceptance only when a known residual risk is visible and the active close path requires that judgment. The prompt should name the risk, affected area, consequence, evidence gap or uncertainty, and any safer bounded action already known.

"Looks good" may be final acceptance only when the agent has clearly asked for final acceptance of a named result. It is not residual-risk acceptance unless the risk was named and the prompt asked for that judgment.

## 9. Read what current MVP can verify honestly

Early Harness behavior is cooperative by default. Limited detective reporting applies only to supported observable facts after the relevant capability check has passed, unless a specific owner-documented mechanism proves more.

| What Harness can verify | What it means | What it does not mean |
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
- name the first safe login work item before writing.

Visible shaped result:
- Goal: add a basic email/password login path.
- Scope: login form, submit handling, session creation, and focused tests.
- Out of scope: account creation, password reset, social login, and production deployment.
- Acceptance: existing users can sign in, failed login shows a chosen error treatment, and tests cover the touched path.
- Readiness: goal, non-goals, affected areas, acceptance, and next action are known; failed-login feedback is the named product blocker before the first safe work item for this change starts.
- Blocking question: should failed login feedback be inline text or a toast?
- Next safe action: after that answer, update the visible scope and check the intended login-form write.

Reference alignment after the request is clear:
- the visible scope is shaped before product files change;
- the failed-login feedback choice is a user-owned product judgment;
- the intended login-form, session, and test writes need a compatible pre-write scope check;
- changed paths, checks, and missing support are recorded after meaningful action;
- close is not complete until required evidence, final acceptance, and any required residual-risk decision are handled separately.
```

### Make a plan concrete

```text
User: Help me make this plan concrete before implementation.

Good agent behavior:
- read the plan and related docs before asking anything;
- identify whether the goal, non-goals, affected areas or paths, acceptance criteria, what the agent may decide on its own, the first safe work item for this change, user-owned blockers, and next safe action are known;
- ask only the decision that changes the first safe work item or next safe action.

Visible shaped result:
- Goal: turn the plan into one implementable first change.
- Scope: clarify the current objective, affected areas, acceptance criteria, and evidence expectation.
- Out of scope: new standalone artifacts, broad roadmap rewrite, and implementation.
- Readiness: affected docs are known, but the first safe work item for this change is not known until the user outcome is chosen; no persistent planning artifact is created.
- Evidence gap: no repository files or tests have been checked yet.
- Blocking question: which user outcome should the first slice prove?
- Next safe action: inspect the named owner docs or files, then update the visible scope for the first implementable slice.

Reference alignment after the request is clear:
- planning can stay read-only until the first implementable slice is specific;
- user-owned choices are asked one at a time;
- no temporary planning document is required just to shape the work;
- inspections or checks count as evidence only after they actually happen;
- the plan should not be called done if the first slice, evidence expectation, or reason closure is blocked remains unresolved.
```

### Only change files inside this scope

```text
User: Ask what you need before changing files, and only change files inside `src/auth` and its tests.

Good agent behavior:
- treat the listed paths as the allowed boundary;
- inspect enough to know whether the requested fix fits that boundary;
- ask before touching any path outside it.

Visible result:
- Scope: `src/auth` and its tests only.
- Out of scope: shared session helpers, routing files, migrations, and package changes unless the user expands scope.
- Blocking question if needed: the fix appears to need `src/session/sessionStore.ts`; should this task include that one file, or should I return a follow-up?
- Next safe action: inspect the named files, then check the intended write against the accepted path boundary.

Reference alignment after the request is clear:
- scope expansion is a separate user-owned judgment;
- accepted scope changes must be applied before a write check relies on them;
- missing out-of-scope work or missing checks remain visible blockers.
```

### Split dependency choice from install approval

```text
User: Record that I approve installing `@example/charts@2.4.1` for this task only.

Good agent behavior:
- confirm whether the dependency direction has already been accepted or still needs a technical judgment;
- record only the named install permission if that is the pending question;
- still check manifest and lockfile writes separately before changing product files.

Visible approval scope:
- Action: install one named dependency version for this task.
- Command/tool: the named package-manager command the agent intends to run.
- Intended paths: dependency manifest and lockfile only.
- Hosts: the package registry host needed for the install.
- Dependencies: `@example/charts@2.4.1` only.
- Secret handles: none, unless a named registry credential handle is required.
- Time window: this task and approval window only.
- Scope limit: no future installs or upgrades.
- Explicitly not authorized: unrelated packages, production deploy, secret printing, broad network requests, or product behavior decisions.
- Capability claim: approval is recorded cooperatively unless the active surface can honestly observe the exact action.

Reference alignment after the request is clear:
- dependency direction, sensitive-action approval, path-level write compatibility, final acceptance, and residual-risk acceptance are separate;
- installing the package does not prove the dependency was the right design;
- install output can support evidence only after the approved action is recorded.
```

### Attach evidence without making paths authority

```text
User: Add the test output and screenshot as evidence for this task.

Good agent behavior:
- avoid treating a raw local path as evidence authority;
- stage safe artifact bytes or a safe notice when the artifact path is available;
- record the run so staged data can become a persistent artifact ref linked to the claim;
- show what each artifact supports and what still is not verified.

Visible evidence summary:
- Changed paths: `src/auth/login.ts`, `src/auth/login.test.ts`.
- Checks: login tests passed; accessibility check not run.
- Supporting artifacts: persisted test log and screenshot refs, with redaction and availability shown.
- What this supports: existing users can sign in; failed login path is covered by tests.
- Still missing: no user-visible accessibility inspection yet.
- Next safe evidence action: run or record the missing inspection, or keep it visible as a blocker if required for close.

Reference alignment after the request is clear:
- staged artifact data is temporary until run recording consumes it;
- a persisted artifact ref can support evidence only when linked to the relevant claim;
- artifact availability and evidence sufficiency remain separate.
```

### Close honestly

```text
User: Close this only if the evidence is sufficient; otherwise tell me what blocks closure.

Good agent behavior:
- show scope, evidence, checks, user-visible inspection status when relevant, final acceptance need, and residual risk;
- name blockers before close;
- ask final acceptance and residual-risk acceptance separately when both are relevant.

Possible close display:
- Evidence: partial. Required screenshot is registered, but the required test result is missing.
- Final acceptance: not requested yet because required evidence is still missing.
- Residual risk: password reset remains out of scope; risk acceptance may be needed only after the close basis is visible.
- Reason this cannot be closed yet: required test evidence is missing.
- Next close-unblocking action: record the test result or record why that required item does not apply.

Reference alignment after the request is clear:
- a read-only close check can report blockers without closing the task;
- completed close requires required evidence before final acceptance and any required residual-risk acceptance;
- residual-risk acceptance does not fix missing evidence;
- "looks good" counts as final acceptance only when the agent clearly asked for final acceptance of a named result.
```

For compact judgment prompt patterns, see [Judgment Examples](judgment-examples.md).
