# User Guide

## Start with ordinary requests

Speak normally. Describe the work you want, the boundary you already know, and how cautious you want the agent to be. You can ask the agent to clarify the plan before implementation, keep a change small unless it turns into a product or technical decision, separate product judgment from architecture judgment, show what evidence is missing, or explain what still blocks close.

The agent should:

- clarify scope before important writes
- inspect the repository, docs, and current Harness context before asking questions it can answer itself
- identify decisions that only you can own
- separate product or UX judgment from technical architecture judgment
- gather or explain the evidence needed to support completion
- show the next safe action and what still blocks closing the work

Harness helps preserve scope, user-owned judgment, evidence, verification, QA expectations, final acceptance, and residual-risk status outside fragile chat context. It should make AI-assisted work easier to follow, not turn every task into a management ritual. Small work should stay small. Larger or riskier work should gain structure only when scope, user-owned judgment, evidence, QA, verification, final acceptance, or residual risk actually matter.

You should expect to see plain work facts: what is in scope, what is out of scope, what the agent can inspect, what only you can decide, what changed, what was checked, what evidence exists, what risk remains, and whether close is blocked. You should not need to learn internal record names, gate names, or tool names before starting.

Harness also does not replace the surrounding engineering process. Source control still records product-file history, tests still check executable behavior, review still reviews changes, and user-owned product or material technical judgment still belongs to the user.

Good Harness requests sound like normal work requests:

```text
I want to add an email login flow. Keep password reset out of scope for now and help me clarify the decisions first.
```

```text
Review this feature idea and ask the questions needed before implementation.
```

```text
Make a small copy change, but tell me if it turns into a broader product decision.
```

```text
Before changing code, separate the product decisions from the technical decisions.
```

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
| Tracked work | The request has unclear scope, multiple parts, product or technical judgment, security/privacy impact, meaningful evidence needs, QA, verification, final acceptance, or close-relevant risk. | Scope, pending user decisions, evidence, close readiness, next safe action, and the smallest blocker. |

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

When the request needs a user-owned decision, the agent should name the decision type instead of asking for broad approval. Product/UX judgment, technical architecture judgment, security/privacy judgment, scope/autonomy judgment, sensitive-action approval, QA waiver, verification waiver, final acceptance, and residual-risk acceptance are separate decisions. Small decisions can be asked as short, explicit prompts; complex or risky decisions should include fuller trade-offs and evidence.

Product or UX judgment:

```text
Decision needed: failed-login experience.
Options: inline layer, toast, or modal.
Recommendation: inline layer near the form because it is persistent and easier to make accessible.
What can continue if you defer: API wiring and tests that do not commit to final UI behavior.
What cannot close yet: final UX, copy, and the human check of the login screen.
```

Technical architecture judgment:

```text
Decision needed: login architecture.
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
Clarify the plan before implementation.
Ask what you need before changing code.
Start with goals, non-goals, and acceptance criteria.
Show the current status and next safe action.
Resume from the current state, not old chat.
What is blocking this task now?
What one decision or check would unblock it?
Show close readiness in plain language.
Show close-relevant residual risk before I accept.
What evidence is still missing?
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

The light display does not mean the agent bypasses Harness internally. If product files may change, the agent still keeps scope narrow, uses the appropriate internal write checks, records what changed, and reports if the work grows beyond the original request. You should not need to see internal boundary labels for every tiny edit unless a label helps explain a boundary or blocker.

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
- what still blocks acceptance or close

The larger the blast radius, the more important this separation becomes. A security-sensitive feature should not be closed just because tests pass. A UI feature should not treat screenshots or browser smoke as final acceptance. A dependency install approval should not be treated as a decision to adopt that dependency as the architecture direction.

<a id="the-four-display-groups"></a>

## Four everyday display groups

Most status should fit into four plain groups. The agent may save precise records behind the scenes, but the first display should answer these questions in ordinary language.

| Display group | What it answers | What the agent should show |
|---|---|---|
| What are we doing? | What is in scope, what is out of scope, and what happens next? | Included behavior, excluded items, affected areas, and whether the next action still fits the agreed scope. |
| What must you decide? | Which choices belong to the user? | Each pending choice separately, such as a product/UX choice, technical architecture choice, security/privacy choice, permission for a sensitive step, final result acceptance, or acceptance of a named remaining risk. |
| What do we know? | What supports the current claim? | Changed paths, focused tests, command output, logs, screenshots, human QA notes, evidence links, recorded runs, saved decisions, related files or artifacts, and anything missing or stale. |
| Why can or can't we close this? | Is the work ready to call done? | The remaining blocker, the smallest unblocker, whether verification or human QA is still needed, whether the user has accepted the result when required, and any known remaining risk. |

These groups are readable summaries, not a new checklist. The agent should show what helps you decide, trust, or unblock the work. It should not make you read internal record names before you understand the status.

Useful status:

```text
What are we doing?
Login form and login API call. Password reset and account creation remain out of scope.

What must you decide?
Choose the failed-login feedback pattern.

What do we know?
Repository inspection is done. No implementation evidence exists yet.

Why can't we close this?
Close is blocked until the UX choice, implementation evidence, human QA expectation, and remaining-risk review are handled.

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
- accepting the final result when that judgment is required

Useful phrases:

```text
Start with the scope and questions.
Only ask me what the codebase cannot answer.
If scope needs to grow, show me the options and impact first.
Separate product decisions from technical decisions.
Tell me what evidence would be enough before you claim this is done.
Show what still blocks closing.
```

## Security guarantee level, briefly

Harness makes security-sensitive AI work easier to see and route, but early local Harness is not a sandbox. It does not automatically change OS permissions, sandbox arbitrary tools, make local files tamper-proof, or turn an instructed agent into preventive security.

You may see four guarantee levels. `cooperative` means the agent is instructed to follow the rules. `detective` means a mismatch can be detected or recorded after action. `preventive` means a proven control blocks the operation before it happens. `isolated` means work or verification runs behind a documented separation boundary; a worktree or fresh evaluator bundle is not automatically an OS sandbox, permission boundary, or tamper-proof security. For early local use, expect cooperative/detective wording unless the agent can name the exact blocking control or exact proven separation boundary in use.

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
| Final acceptance | Captures the user's result judgment when required. | Evidence, verification, QA, permission for a sensitive step, waiver, or risk acceptance. |
| Remaining risk | Names known uncertainty, limitation, unchecked condition, or trade-off. | Evidence, verification, QA, final acceptance, or sensitive-step permission. |
| Accepting remaining risk | Captures that the user accepts an identified known remaining risk. | Final acceptance, verification, QA, sensitive-step permission, or generic approval. |
| Permission for a sensitive step | Allows a named sensitive step to proceed. | Product judgment, correctness, final acceptance, risk acceptance, or waiver. |

That separation is why work can still be blocked after tests pass. Tests can support evidence or verification, but close may still need human QA for the real experience, your final acceptance of the result, or your explicit acceptance of a known remaining risk.

Residual-risk wording should be precise. "No known close-relevant residual risk" means the system has no known close-relevant risk for this requested action. "Risk not visible yet" means known risk exists but has not been shown clearly enough for acceptance or close.

A casual "go ahead," "proceed," or "looks good" is only usable when the agent has already named the exact thing you are deciding. It is not enough for product trade-offs, architecture choices, QA or verification waivers, accepting the result, or accepting residual risk unless the prompt shows the choice, consequences, relevant evidence links or saved decisions, and what remains outside that decision. If the phrase could apply to more than one pending decision, the agent should ask which one you mean.

## Advanced: exact close labels

You may see stricter labels when a tool, report, or reference page needs precision.

| Reference label | Plain meaning |
|---|---|
| Evidence record or Evidence Manifest | The saved support for completion claims. |
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
| Discovery | The internal name for the agent's requirements-clarification behavior before implementation planning. Users can ask for this as "clarify the plan before implementation." |
| Change Unit | The bounded work area that may change for this task. |
| Autonomy Boundary | The decisions the agent may make alone inside that scope. |
| Decision Packet | The recorded path for a user-owned product/UX, technical architecture, security/privacy, scope/autonomy, waiver, final acceptance, residual-risk acceptance, or reconcile decision. It can be concise for a small unblocker or detailed for complex/risky choices. |
| Judgment domain | The user-facing grouping on a Decision Packet, such as Product / UX, Technical architecture, Security / privacy, QA / acceptance, Residual risk, Scope / autonomy, or Mixed. |
| Approval | Permission for a named sensitive action; not generic agreement or final acceptance. |
| Write Authorization | A one-attempt check that the intended product write fits the current task, scope, decisions, and approvals. |
| Evidence Manifest | The record that maps completion claims to supporting evidence. |
| Projection | A readable summary derived from owner records and related files or artifacts. Early use may be compact status text or a card, not a full Markdown report; it helps orientation and is not authority by itself. |
| ProjectionJob | The internal job that creates or refreshes a readable projection. |
| task_events | Low-level event history for implementers and diagnostics. |

These labels do not collapse into each other. Approval is not acceptance. Final acceptance does not erase residual risk. A decision is not write authority. A readable summary is not state. Passing tests does not mean Manual QA happened. Accepting residual risk does not make the risk disappear.

For exact contracts, use the Reference docs only when needed: [Kernel Reference](../reference/kernel.md), [MCP API And Schemas](../reference/mcp-api-and-schemas.md), and [Agent Integration Reference](../reference/agent-integration.md).

## Where to go next

Read [Concepts](../learn/concepts.md) for the vocabulary behind the user-facing words.

Use [Decision Packet Cookbook](decision-packet-cookbook.md) when a product, UX, architecture, security, QA, verification, acceptance, risk, or scope decision needs a focused prompt.

Agent integrators should read [Agent Session Flow](agent-session-flow.md). Ordinary users do not need it for the primary path.
