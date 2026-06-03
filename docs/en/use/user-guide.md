# User Guide

## Start with ordinary requests

Speak normally. Start with the work, not Harness vocabulary:

```text
Help me clarify the plan before implementation.
Show what I need to decide and what you can check yourself.
Tell me if the scope is getting bigger.
Show what still blocks closing this work.
```

Other good requests sound just as ordinary:

```text
I want to add an email login flow. Keep password reset out of scope for now and help me clarify the decisions first.
```

```text
Review this feature idea and ask the questions needed before implementation.
```

```text
Make a small copy change, but tell me if it turns into a broader product decision.
```

Status note: this guide describes the intended Harness-assisted user flow for a future local Harness Server. This repository is documentation-only today; it is not the user's Product Repository or a Harness Runtime Home.

You can describe the work you want, the boundary you already know, and how cautious you want the agent to be. The agent should translate that into work, scope, facts it can inspect, judgments only you can make, evidence, checks or verification, and close status. You should not need to learn internal record names, readiness labels, or tool names before starting.

The primary user-facing model has six plain concepts: work, scope, judgment or thing to decide, evidence, check or verification, and close. Internal Harness labels are optional and advanced. The agent can show exact labels later when they clarify a boundary, source record, or reference contract.

The agent should:

- clarify the work and scope before important writes
- inspect the repository, existing docs, tests, current Harness state, accepted decisions, and current task artifacts before asking questions it can answer itself
- identify decisions that only you can own
- separate product or UX judgment from technical architecture judgment when that distinction matters
- gather or explain the evidence needed to support completion claims
- run or explain the checks and verification that matter for the work
- show what still blocks close and the next safe action

Harness helps preserve scope, user-owned judgment, evidence, verification, QA expectations, work acceptance, and residual-risk status outside fragile chat context. It should make AI-assisted work easier to follow, not turn every task into a management ritual. Small work should stay small. Larger or riskier work should gain structure only when scope, user-owned judgment, evidence, QA, verification, work acceptance, or residual risk actually matter.

You should expect to see plain work facts: what the work is, what is in scope, what only you can decide, what evidence exists, what was checked or verified, and whether close is blocked. QA, work acceptance, and remaining risk still matter, but they should be shown as part of the judgment, check, evidence, or close story instead of as extra vocabulary a new user must memorize.

Harness also does not replace the surrounding engineering process. Source control still records product-file history, tests still check executable behavior, review still reviews changes, and user-owned product or material technical judgment still belongs to the user.

You can be more explicit if you want:

```text
Run this work under the harness.
```

But the agent should infer Harness use from the task shape. You should not have to start with internal labels.

## Three everyday work shapes

Most requests should be explained with plain work shapes:

| Work shape | Use it when | What you should see |
|---|---|---|
| Read/advice work | The agent is reading, explaining, comparing, reviewing, or helping decide without changing product files. | The answer, sources or caveats when useful, and any decision or follow-up that matters. |
| Small change | The requested change is narrow, low risk, and has an obvious result, such as a typo, copy-only edit, or focused fix. | A short scope, changed path or no-file result, what was checked, and whether anything forced escalation. |
| Tracked work | The request has unclear scope, multiple parts, product or technical judgment, security/privacy impact, meaningful evidence needs, QA, verification, work acceptance, or close-relevant risk. | Scope, pending user judgments, evidence, close readiness, next safe action, and the smallest blocker. |

The agent may record more internal detail than it displays. User-facing messages should show the detail that helps you decide, trust, or unblock the work, not a lifecycle checklist for every tiny edit.

## What the agent should answer first

For non-trivial work, the first useful response is not a full plan or a pile of internal state. It should be a short translation of the request into plain working facts.

Example:

```text
Understood scope: add email login only. Password reset, account creation, social login, and global auth redesign stay out of scope.

I can inspect: existing login routes, session handling, auth tests, UI form patterns, validation helpers, and docs for current auth behavior.

Only you can decide: whether email login should use password credentials, one-time codes, magic links, or an external identity provider; what failed-login UX and copy are acceptable; whether any security or UX trade-off is worth the cost.

Next safe action: inspect the current auth flow, then come back with what the codebase answers, what only you can decide, and a scoped next-work proposal.
```

A good clarification response should separate:

- goal
- user value
- non-goals
- acceptance criteria
- facts the agent can inspect from the repo, docs, or current Harness state
- judgments only the user can make
- product or UX judgment candidates
- technical architecture judgment candidates
- security or privacy judgment candidates
- human review and verification expectations
- remaining uncertainty
- safe next-work candidate or work-splitting candidate

Clarification is enough to proceed only when:

- the goal can be summarized in one sentence
- at least one non-goal or boundary is known when a boundary matters
- success criteria, acceptance criteria, or the desired end state are known
- questions answerable from the repository, existing docs, tests, current Harness state, accepted decisions, or current task artifacts have been checked before asking you
- user-only judgments are separated from facts the agent can check
- blocking questions are separated from useful-but-not-blocking questions
- the next safe action is classified as advice/read-only work, a small direct change, or tracked work
- remaining uncertainty is visible instead of hidden inside a confident plan

If those conditions are not met, the agent should either inspect the available sources, ask the smallest blocking question, park useful-but-not-blocking questions, or propose a smaller safe slice that avoids the unresolved area.

When the request needs a user-owned judgment, the agent should show a user judgment request instead of asking for broad approval. Product/UX judgment, technical architecture judgment, security/privacy judgment, scope/autonomy judgment, sensitive-action approval, QA waiver, verification waiver, work acceptance, and residual-risk acceptance are separate judgments. Small judgments can be asked as short, explicit prompts; complex or risky judgments should include fuller trade-offs and evidence. Any internal saved-record label should stay behind the plain question unless it helps explain a real boundary or source link.

Product or UX judgment:

```text
Judgment request: choose the failed-login experience.
Options: inline layer, toast, or modal.
Recommendation: inline layer near the form because it is persistent and easier to make accessible.
What can continue if you defer: API wiring and tests that do not commit to final UI behavior.
What cannot close yet: final UX, copy, and the human check of the login screen.
```

Technical architecture judgment:

```text
Judgment request: choose the login architecture.
Options: session cookie, bearer/JWT, OAuth/OIDC, or social-login provider integration.
Recommendation: inspect existing session and user model first; do not choose until we know what the codebase already supports or what identity-provider requirement exists.
What can continue if you defer: read-only inspection and a scoped implementation proposal.
What cannot close yet: implementation, security evidence, and acceptance criteria for the chosen auth path.
```

Evidence needed:

```text
Evidence needed before "done": changed paths, focused auth tests, failure-path tests, any security-sensitive redaction notes, and a human QA note for the login form copy and error states if the UI changes.
```

Why work cannot close yet:

```text
Cannot close yet: the failed-login UX is not chosen, no implementation evidence exists, and the expected human check for the login screen is not settled.
Smallest unblocker: choose the failed-login pattern, or ask me to propose a smaller slice that avoids final UI behavior for now.
```

<a id="first-read-path"></a>
<a id="phrase-reference"></a>

## Useful things you can ask

Use these as ordinary requests. They are not commands you must memorize.

```text
Help me clarify the plan before implementation.
Show what I need to decide and what you can check yourself.
Ask what you need before changing code.
Start with goals, non-goals, and acceptance criteria.
Show the current status and next safe action.
Resume from the current state, not old chat.
What is blocking this task now?
What one decision or check would unblock it?
Show close readiness in plain language.
Show close-relevant residual risk before I accept.
What evidence is still missing?
Separate product decisions from technical decisions.
Tell me if the scope is getting bigger.
Keep this small unless it turns into a product or technical decision.
After you inspect, show the safe next work or work split.
```

## Planning examples

For product planning, you can say:

```text
We need to improve onboarding for new workspace owners. First, inspect what exists, separate product choices from facts, and ask only the questions you cannot answer from the repo.
```

A useful response should identify the user value, non-goals, inspectable facts, product/UX judgment candidates such as checklist versus setup prompt, QA expectations for the flow, remaining uncertainty, and a safe next-work or work-splitting proposal.

For technical planning, you can say:

```text
I want to replace our login approach, but I do not know whether sessions, magic links, or OAuth/OIDC fit best. Inspect the current auth shape first and show the decisions before implementation.
```

A useful response should inspect existing auth, session, user model, tests, and docs before asking; separate technical architecture choices from security/privacy choices; name verification and human QA expectations; and propose either a safe investigation slice or a work split. It should not treat the first plausible implementation path as enough to stop clarifying if acceptance criteria, major judgment candidates, or remaining uncertainty are still unclear.

## Small work should stay light

Harness should not make a tiny change feel heavy. If the user says:

```text
Fix the typo in the settings toast: "Succesful" -> "Successful."
```

The agent should keep the visible flow compact:

```text
Scope: settings toast typo only.
Out of scope: account behavior, layout, localization strategy, and broader copy cleanup.
I will edit the target component and any directly related copy test if one exists.
```

After the change, the result can be just as small:

```text
Done.
Changed: settings toast typo to "Successful."
Checked: related copy test and diff review.
Write check: allowed for this focused change.
No broader product decision appeared, and no close-relevant residual risk is known for this small change.
```

The light display does not mean the agent bypasses Harness internally. If product files may change, the scope, write, evidence, and close boundary still matters. The agent still keeps scope narrow, uses the appropriate internal write checks, records what changed, preserves the evidence needed for the claim, and reports if the work grows beyond the original request. You should not need to see internal boundary labels for every tiny edit unless a label helps explain a boundary or blocker.

Small work should escalate when it stops being small. Escalate to tracked work when scope is unclear; multiple files, product areas, or subsystems are involved; a product or UX judgment is needed; an important technical architecture judgment is needed; a public interface or API may be affected; security or privacy may be affected; a sensitive action is needed; QA or verification requirements increase; evidence is insufficient; residual risk is non-trivial; or multi-step delivery is needed.

Examples:

- A typo or copy-only change can stay lightweight when it touches one obvious surface and does not change meaning, behavior, localization strategy, security posture, or required QA.
- "Make Enter submit this modal instead of closing it" should escalate if it changes UI behavior, accessibility expectations, or product workflow. That is a product/UX decision, not just a small edit.
- "Change login to magic links" should escalate because it changes authentication architecture and security/privacy behavior. The agent can inspect first, but implementation needs tracked scope, user-owned technical/security judgment, evidence, and likely QA/verification.

## Larger work gets more structure

If the user says:

```text
I want to add an email login flow. Keep password reset out of scope for now and help me clarify the decisions first.
```

The agent should add structure because the work may involve product behavior, security, UI, tests, evidence, QA, and close-relevant risk.

A good early response:

```text
Scope I heard: add email login. Out of scope for now: password reset, account creation, social login, and global auth redesign.

I will inspect: existing auth routes, user/session model, login UI patterns, validation and error handling, current tests, and docs.

Likely user-owned decisions:
- Product / UX: credential flow, failed-login behavior, login copy, and recovery messaging.
- Technical architecture: session model, token/cookie strategy, password storage or identity-provider path, migration impact, and dependency choices.
- Security / privacy: account-enumeration risk, audit logging, rate limiting or lockout behavior, redaction, and secret handling.

Evidence likely needed: focused tests for success and failure paths, changed-path summary, security-sensitive notes, and a human QA note if the login screen changes.

Close cannot happen yet because scope, required user-owned decisions, evidence, human-review expectations, and residual risk are not settled.
```

As the work progresses, the agent should keep the same shape visible:

- what is in scope and what remains out of scope
- what the agent can decide inside the agreed scope
- what only the user can decide
- what was changed and checked
- what evidence supports each completion claim
- whether verification or a human QA check is needed
- what residual risk remains
- what still blocks work acceptance or close

The larger the blast radius, the more important this separation becomes. A security-sensitive feature should not be closed just because tests pass. A UI feature should not treat screenshots or browser smoke as work acceptance. A dependency install approval should not be treated as a decision to adopt that dependency as the architecture direction.

<a id="the-four-display-groups"></a>

## Six everyday status concepts

Most status should fit into six plain concepts. The agent may save precise records behind the scenes, but the first display should answer these questions in ordinary language.

| Concept | What it answers | What the agent should show |
|---|---|---|
| Work | What are we trying to do? | The requested result, current work shape, affected area, and next safe action. |
| Scope | What may change, and what stays out? | Included behavior, excluded items, affected paths or areas, and whether the next action still fits the agreed scope. |
| Judgment | What must you decide? | Each pending choice separately, such as a product/UX choice, technical architecture choice, security/privacy choice, permission for a sensitive step, work acceptance, or acceptance of a named remaining risk. |
| Evidence | What supports the current claim? | Changed paths, command output, logs, screenshots, human QA notes, evidence links, recorded runs, related files or artifacts, and anything missing or stale. |
| Check or verification | What was checked, and from what review boundary? | Focused tests, diff review, inspection, source lookup, verification result, or required human QA expectation. |
| Close | Can this honestly finish? | The remaining blocker, the smallest unblocker, whether work acceptance is required, and any known remaining risk or residual-risk acceptance need. |

These concepts are readable summaries, not a new checklist. The agent should show what helps you decide, trust, or unblock the work. It should not make you read internal record names before you understand the status.

Useful status:

```text
Work:
Add email login support.

Scope:
Login form and login API call. Password reset and account creation remain out of scope.

Judgment:
Choose the failed-login feedback pattern.

Evidence:
Repository inspection notes are saved. No implementation evidence exists yet.

Check or verification:
No implementation check has run yet. Human QA expectations for the login screen are still unsettled.

Close:
Blocked until the UX choice, implementation evidence, human QA expectation, and remaining-risk review are handled.

Next safe action:
Choose the failed-login pattern, or ask me to propose a smaller slice.
```

When several decisions are pending, the agent should split them instead of asking for broad approval:

```text
What must you decide?
- Product/UX: choose the failed-login feedback pattern.
- Sensitive step: allow or deny installing the auth helper package.
- Remaining risk: accept or reject the named mobile Safari wrapping risk.
```

## Plain links to saved work

You may see links or IDs in a status summary. They should be described by what they help you inspect:

- evidence link: support for a claim, such as a diff, command output, test result, screenshot, or QA note
- recorded run: the saved result of a command, test, check, or verifier
- saved decision: the specific user-owned choice that was recorded
- related file or artifact: a changed file, generated support file, screenshot, report, or other object relevant to the work

The exact record type matters to implementers, but ordinary status should start with the plain meaning. "Evidence link from the focused auth test" is more useful than making a raw record ID the center of the sentence.

<a id="judgment"></a>

## What the agent can decide

Once scope is clear, the agent can usually decide routine implementation details without asking every small question. Examples include reusing an existing helper, splitting a private function, adding a focused test, following local naming conventions, or choosing the conservative internal approach that best fits the agreed result.

The agent should stop and ask when a choice changes what users, callers, or future work can rely on:

- product behavior or UX
- public API or module contracts
- security, privacy, audit, retention, or redaction choices
- material dependency, migration, or architecture direction
- scope expansion
- QA or verification waivers
- accepting known residual risk
- work acceptance when that judgment is required

Useful phrases:

```text
Help me clarify the plan before implementation.
Show what I need to decide and what you can check yourself.
If scope needs to grow, show me the options and impact first.
Separate product decisions from technical decisions.
Tell me what evidence would be enough before you claim this is done.
Tell me if the scope is getting bigger.
Show what still blocks closing this work.
```

## Security guarantee level, briefly

Harness makes security-sensitive AI work easier to see and route, but early local Harness is not a sandbox. It does not automatically change OS permissions, sandbox arbitrary tools, make local files tamper-proof, or turn an instructed agent into preventive security.

You may see four guarantee levels. `cooperative` means the agent is instructed to follow the rules. `detective` means a mismatch can be detected or recorded after action. `preventive` means a proven control blocks the operation before it happens. `isolated` means work or verification runs behind a documented separation boundary; a worktree or fresh evaluator bundle is not automatically an OS sandbox, permission boundary, or tamper-proof security. For early local use, expect cooperative/detective wording unless the agent can name the exact blocking control or exact proven separation boundary in use.

When a status says something is blocked, read it as "Harness cannot honestly proceed or close under the current record" unless the agent also names a proven preventive control. Early local Harness may reveal, record, or hold by instruction; it should say plainly when it can stop something before it happens and when it can only report it.

## When blocked

A blocker should be concrete. It should say who owns the next move and what the smallest unblocker is.

```text
Blocked.
What are we doing? The requested copy edit appears to affect account behavior outside the original label.
What must you decide? Whether to keep this label-only or expand the product behavior.
What do we know? I can inspect call sites and show the affected screens.
Why can't we close this? Close is blocked until scope is either narrowed back to the label or expanded deliberately.
Smallest unblocker: choose whether to keep this as a label-only change or include account behavior.
```

Do not let the agent turn an agent-resolvable issue into a user burden. If the agent can inspect code, refresh status, rerun a test, collect missing evidence, or narrow the work without changing your judgment, it should say what it will do next.

Close blockers should be readable without knowing internal readiness labels. A close blocker display should show:

- what user judgment remains, if any
- what evidence is missing, stale, or insufficient
- whether verification or human QA is required by the active profile
- whether work acceptance is required
- what residual risk is visible, not yet visible, accepted, or still unresolved
- the next smallest resolving action

## Before close

Before close, ask:

```text
Show what changed, what was checked, what risks remain, and what still blocks close.
```

For larger work, also ask:

```text
Show close-relevant residual risk before I accept.
```

The agent should keep these separate:

| Plain item | Job | Not a substitute for |
|---|---|---|
| Evidence | Supports the claim that a result or criterion was met. | The agent saying "done" or the user accepting the result. |
| Verification | Checks correctness from the appropriate review boundary. | Human QA or broad confidence. |
| Human QA | Captures a person's inspection where judgment matters. | Automated tests or screenshots alone. |
| Work acceptance | Captures the user's result judgment when required. | Evidence, verification, QA, permission for a sensitive step, waiver, or risk acceptance. |
| Remaining risk | Names known uncertainty, limitation, unchecked condition, or trade-off. | Evidence, verification, QA, work acceptance, or sensitive-step permission. |
| Accepting remaining risk | Captures that the user accepts an identified known remaining risk. | Work acceptance, verification, QA, sensitive-step permission, or generic consent. |
| Permission for a sensitive step | Allows a named sensitive step to proceed. | Product judgment, correctness, work acceptance, risk acceptance, or waiver. |

That separation is why work can still be blocked after tests pass. Tests can support evidence or verification, but close may still need human QA for the real experience, your work acceptance, or your explicit acceptance of a known remaining risk.

Residual-risk wording should be precise. "No known close-relevant residual risk" means the system has no known close-relevant risk for this requested action. "Risk not visible yet" means known risk exists but has not been shown clearly enough for acceptance or close.

A casual "go ahead," "proceed," "looks good," "진행해," or "좋아" is only usable when the agent has already named the exact thing you are deciding. It must not automatically resolve every pending judgment. If the response could mean approval for a sensitive step, work acceptance, residual-risk acceptance, or simple continuation, the agent should route or clarify before recording it. It is not enough for product trade-offs, architecture choices, QA or verification waivers, accepting the result, or accepting residual risk unless the prompt shows the choice, consequences, relevant evidence links or saved decisions, and what remains outside that decision. If the phrase could apply to more than one pending judgment, the agent should ask which one you mean.

## Advanced: exact close labels

You may see stricter labels when a tool, report, or reference page needs precision.

| Reference label | Plain meaning |
|---|---|
| Evidence summary / Evidence Manifest when active | Minimum MVP-1 can show known evidence summaries, Run refs, ArtifactRefs, and visible gaps. A full Evidence Manifest appears only on the later/profile owner path. |
| Verification or Eval | A check from the required review boundary; an Eval is one implementation form of that check. |
| Manual QA record | The saved human QA result or waiver context. |
| Acceptance | The saved user judgment that the completed result is good enough, when required. |
| Residual Risk or ResidualRiskSummary | The saved statement of known remaining risk. |
| Approval | Permission for a named sensitive step; not broad agreement. |

For exact contracts, use the Reference docs: [Kernel Reference](../reference/kernel.md), [MCP API And Schemas](../reference/mcp-api-and-schemas.md), and [Operations and Conformance Reference](../reference/operations-and-conformance.md).

## Advanced: Harness labels you may see

You can skip this section until an agent shows one of these labels. They are useful for precision, but they should not be the first way a normal task is explained.

| Harness label | Plain meaning |
|---|---|
| Discovery | The internal name for the agent's requirements-clarification behavior before implementation planning. Users can ask for this as "help me clarify the plan before implementation." |
| Change Unit | The bounded work area that may change for this task. |
| Autonomy Boundary | The decisions the agent may make alone inside that scope. |
| Decision Packet | The internal record/template label behind a user judgment request. It records a user-owned product/UX, technical architecture, security/privacy, scope/autonomy, waiver, work acceptance, residual-risk acceptance, or reconcile judgment. It can be concise for a small unblocker or detailed for complex/risky choices. |
| `judgment_category` | Schema field for the judgment grouping, such as Product / UX, Technical architecture, Security / privacy, QA / verification, Work acceptance, Residual risk, Scope / autonomy, or Mixed. Ordinary prompts should show the friendly judgment type first. |
| `judgment_route` | Schema field for the answer route, such as choose, waive, accept result, accept risk, approve a sensitive step, or reconcile. Ordinary prompts should use the matching verb. |
| `display_depth` | Schema field for prompt depth. Ordinary users should see the practical result: a short question, a trade-off question, a high-risk question, or a close-affecting question. |
| Approval | Permission for a named sensitive action; not generic agreement or work acceptance. |
| Write Authorization | A one-attempt check that the intended product write fits the current task, scope, user judgments, and sensitive-action permissions. |
| Evidence Manifest | The later/profile record that maps completion claims to supporting evidence. Minimum MVP-1 can use evidence summaries, Run refs, ArtifactRefs, and visible gaps without a full Evidence Manifest. |
| Gate | An internal readiness or compatibility condition. User-facing status should show the blocker or check first. |
| Projection | A readable summary derived from owner records and related files or artifacts. Early use may be compact status text or a card, not a full Markdown report; it helps orientation and is not authority by itself. |
| Journey Card / Journey Spine | Later continuity display. It can help orientation when enabled and fresh, but it is not authority by itself. |
| ProjectionJob | The internal job that creates or refreshes a readable projection. |
| `task_events` | Low-level event history for implementers and diagnostics. |

These labels do not collapse into each other. Approval is not work acceptance. Work acceptance does not erase residual risk. A decision is not write authority. A readable summary is not state. Passing tests does not mean Manual QA happened. Accepting residual risk does not make the risk disappear.

For exact contracts, use the Reference docs only when needed: [Kernel Reference](../reference/kernel.md), [MCP API And Schemas](../reference/mcp-api-and-schemas.md), and [Agent Integration Reference](../reference/agent-integration.md).

## Where to go next

Read [Concepts](../learn/concepts.md) for the vocabulary behind the user-facing words.

Use [Judgment Request Cookbook](decision-packet-cookbook.md) when a product, UX, architecture, security, QA, verification, work acceptance, risk, or scope judgment needs a focused prompt.

Agent integrators should read [Agent Session Flow](agent-session-flow.md). Ordinary users do not need it for the primary path.
