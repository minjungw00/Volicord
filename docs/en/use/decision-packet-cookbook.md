# Decision Packet Cookbook

## Ask for one focused decision

Use this after [User Guide](user-guide.md) when work is blocked by a choice the agent should not make alone. You can ask the agent to show the options, recommend one path, name uncertainty, explain what can continue if you defer, and say what still blocks close.

The agent should clarify why the decision is needed now, what the realistic options are, which trade-offs belong to you, what the codebase or current evidence can answer, and what evidence, QA, verification, acceptance, or residual-risk handling may be affected.

Harness helps preserve the user-owned decision separately from broad approval, implementation evidence, final acceptance, and residual-risk acceptance. You should expect a compact decision prompt, not a field list.

Decision prompts can be concise or detailed. A tiny unblocker can show only the question, scope, concise options, and related evidence links; omitted pros/cons, recommendation, uncertainty, or deferral analysis are valid when the selected profile is `minimal_decision` and those details are not material. Complex or high-risk choices should include detailed options, trade-offs, recommendation or explicit no-recommendation reason, uncertainty, deferral consequence, and affected evidence or risk links.

This is advanced usage and example material, not the primary user entrypoint and not the exact contract for Decision Packet behavior.

## When to use it

Use these examples when a task is blocked by product, UX, architecture, security, QA, verification, final acceptance, residual-risk acceptance, or scope/autonomy judgment that the agent should not decide alone.

## Main idea

A Decision Packet should feel like decision support, not a blank permission slip. It names the real user-owned choice, shows options and trade-offs, recommends a path, states uncertainty, explains deferral, and links evidence or residual risk where relevant.

The examples below are prompt examples, not contract definitions. Exact behavior stays with the Reference owners.

## What every example shows

Each cookbook example includes:

- the decision area
- the prompt depth, when useful
- the decision route, when useful
- why the decision is needed now
- realistic options or a chosen outcome
- a recommendation, uncertainty, and deferral consequence when the profile needs them
- related risk, evidence links, recorded runs, saved decisions, or files where applicable

Implementation labels live in the Reference docs. These examples use plain decision prompts first.

## Tiny decision: label wording

Use this when a simple product or technical unblocker needs the user's choice, but a full trade-off packet would be ceremony without extra safety.

```text
Decision title: Settings form button label
Decision area: Product / UX
Prompt depth: concise
Decision route: product trade-off
Why now: the scoped settings copy change needs one label before the agent updates the text and related snapshot.
Options:
- Save.
- Update.
Related support: settings form copy scope and related snapshot or test evidence link if present.
Does not settle: broader settings workflow, localization strategy, final acceptance, residual-risk acceptance, or write authority.
```

Why this works: it records the user-owned choice explicitly without forcing pros/cons, uncertainty, or architecture-style detail where the decision is small and bounded.

## UX decision: inline layer vs toast vs modal

Use this when a user-visible behavior must be chosen before implementation or QA can finish.

```text
Decision title: Failed-login feedback pattern
Decision area: Product / UX
Prompt depth: detailed trade-off
Decision route: product trade-off
Why now: the login flow needs one failure-feedback pattern before final UI wiring, copy tests, and human QA.
Options:
- Inline layer near the form fields.
- Toast after failed submit.
- Modal that interrupts the flow.
Recommendation: choose inline layer.
Uncertainty: confirm existing design-system support for inline errors and screen-reader announcement behavior.
Deferral consequence: API error mapping and state plumbing can continue, but final UI behavior, copy, screenshots, and human QA should wait.
Related risk or evidence: account-enumeration copy risk, accessibility evidence, screenshots or browser-smoke evidence links, and QA notes after implementation.
```

Why this works: it asks for the UX choice instead of asking the user to "approve the login change." It also says what can continue while the user decides.

Exact Decision Packet behavior is owned by [Decision Packet](../reference/kernel.md#decision-packet) and [Decision Gate](../reference/kernel.md#decision-gate). Manual QA behavior is owned by [QA Gate](../reference/kernel.md#qa-gate).

## Auth decision: session cookie vs bearer/JWT vs OAuth/OIDC vs social login

Use this when an authentication direction affects storage, revocation, client behavior, or security posture.

```text
Decision title: Login session architecture
Decision area: Technical architecture
Prompt depth: detailed architecture trade-off
Decision route: architecture choice
Why now: the implementation must choose the session model before storage, middleware, tests, and threat review can be scoped.
Options:
- Server-side session cookie for first-party web login.
- JWT or bearer token handled by the client.
- OAuth/OIDC identity provider, with a separate local session or token strategy when needed.
- Social-login provider integration, with provider-specific account linking and support implications.
Recommendation: choose server-side session cookie for a first-party web app unless the product requires third-party identity provider sign-in, social-login conversion, or non-browser clients now.
Uncertainty: current client mix, existing auth middleware, revocation requirements, SSO requirements, and deployment constraints.
Deferral consequence: the agent can inspect current auth code and draft a narrow work slice, but implementation should not commit to storage, token lifetime, or middleware behavior.
Related risk or evidence: CSRF/XSS exposure, revocation evidence, session-lifetime tests, migration notes, and security review evidence links.
```

Why this works: it uses the full architecture profile because this choice affects storage, revocation, client behavior, security posture, migration, tests, and review. It also separates identity-provider choice from session/storage choice. OAuth/OIDC may still need a local session or token strategy, so the packet does not pretend those are interchangeable.

Exact sensitive-action approval and user-owned architecture judgment boundaries are owned by [Approval](../reference/kernel.md#approval), [Decision Packet](../reference/kernel.md#decision-packet), and [Sensitive Categories](../reference/mcp-api-and-schemas.md#sensitive-categories).

## Security decision: PII logging

Use this when a feature, debug path, run, export, or artifact might expose personal data.

```text
Decision title: PII logging policy for login diagnostics
Decision area: Security / privacy
Decision route: design choice
Why now: the agent needs to know what may be written to logs and evidence artifacts before adding diagnostics or tests.
Options:
- Do not log PII; use request IDs and nonidentifying error codes.
- Log redacted or tokenized identifiers.
- Log limited raw fields for a short retention window with audit controls.
Recommendation: do not log raw PII; use request IDs plus redacted or tokenized identifiers only if debugging needs them.
Uncertainty: support/debugging requirements, retention policy, compliance obligations, and whether existing log redaction is proven.
Deferral consequence: implementation can continue without PII logging, but diagnostics that require user identifiers should wait.
Related risk or evidence: privacy exposure, artifact redaction notes, log sample evidence, retention/audit evidence links, and any residual risk if debugging value is reduced.
```

Why this works: it treats privacy as a product/security judgment, not as a hidden implementation detail. If a sensitive action is also required, that approval is separate from the policy decision.

Exact security concepts live in [Security Threat Model Reference](../reference/security-threat-model.md). Exact approval and evidence authority live in [Approval](../reference/kernel.md#approval) and [Evidence Gate](../reference/kernel.md#evidence-gate).

## QA waiver

Use this when required human QA cannot be completed, and the user must decide how to handle close without treating the waiver as proof or risk acceptance.

```text
Decision title: Waive Manual QA for responsive login layout
Decision area: QA / acceptance
Decision route: QA waiver
Why now: close is blocked because required Manual QA has not passed for the responsive login flow.
Options:
- Perform Manual QA now.
- Record a Manual QA waiver for this close; if close-relevant residual risk remains, also route or record residual-risk acceptance through the owner path.
- Keep the task open and schedule QA before close.
Recommendation: perform Manual QA for a user-facing login workflow; waive only if the environment is unavailable and the change is low risk or time-bound.
Uncertainty: small-screen layout, keyboard flow, screen-reader interpretation, and visual polish have not been inspected by a human.
Deferral consequence: implementation can remain complete, but close should stay blocked until Manual QA passes or a valid QA waiver and any required residual-risk acceptance path are recorded.
Related risk or evidence: existing test logs, screenshots if available, skipped viewport list, the Manual QA requirement, and residual-risk follow-up.
```

Why this works: it names the skipped inspection. A QA waiver does not prove QA passed and does not by itself accept residual risk unless the required residual-risk acceptance path is also recorded.

Exact QA behavior is owned by [QA Gate](../reference/kernel.md#qa-gate), [`harness.record_manual_qa`](../reference/mcp-api-and-schemas.md#harnessrecord_manual_qa), and [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision).

## Verification waiver

Use this when detached verification is required or expected, but the user wants to proceed without it.

```text
Decision title: Waive detached verification for invoice export fix
Decision area: QA / acceptance
Decision route: verification waiver
Why now: close as verified is blocked because no compatible independent verification exists, and the user is asking to close today.
Options:
- Run detached verification from a fresh bundle or fresh worktree.
- Keep the task open until independent verification is available.
- Waive verification and close only through a risk-accepted path if the remaining risk is visible and accepted.
Recommendation: run detached verification for billing/export behavior; waive only if the change is low blast-radius and existing self-check evidence is strong.
Uncertainty: same-session bias, unreviewed export edge cases, stale bundle risk, and whether the self-check covered the affected formats.
Deferral consequence: the task cannot close as detached verified; close either waits or uses the documented risk-accepted path when allowed.
Related risk or evidence: self-check recorded run, missing independent-verification link, affected export formats, residual-risk link, and follow-up verification plan.
```

Why this works: it keeps assurance language honest. A verification waiver may unblock a risk-accepted close path, but it does not create detached verification.

Exact verification and close behavior is owned by [Verification Gate](../reference/kernel.md#verification-gate), [Verification Independence Profiles](../reference/kernel.md#verification-independence-profiles), [Residual Risk](../reference/kernel.md#residual-risk), and [`close_task`](../reference/kernel.md#close_task).

## Residual risk acceptance

Use this when known close-relevant risk remains after implementation and evidence, and the user must decide whether that risk is acceptable for this close.

```text
Decision title: Accept legacy CSV encoding limitation
Decision area: Residual risk
Decision route: residual-risk acceptance
Why now: the export fix works for current UTF-8 files, but legacy encodings remain unsupported and close needs a risk decision.
Options:
- Fix legacy encoding support before close.
- Accept the bounded risk for this close and create a follow-up.
- Cancel or supersede the task because the remaining limitation changes the requested outcome.
Recommendation: accept only if legacy encoding is rare, documented, and has an owner-visible follow-up; otherwise fix before close.
Uncertainty: real customer frequency, support impact, and whether existing imports include legacy files.
Deferral consequence: final acceptance or close may remain blocked until the risk is resolved, made non-close-relevant, or accepted through the owner path.
Related risk or evidence: passing UTF-8 export tests, missing legacy-encoding test coverage, known limitation note, follow-up link, and visible residual-risk link.
```

Why this works: it makes the remaining limitation visible before acceptance. The user is not just accepting the result; they are also deciding whether a named remaining risk is acceptable for this close.

Exact residual-risk behavior is owned by [Residual Risk](../reference/kernel.md#residual-risk), [Acceptance Gate](../reference/kernel.md#acceptance-gate), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision), and [`close_task`](../reference/kernel.md#close_task).

## Owner links

Use these Reference owners when the cookbook examples need exact behavior:

| Need | Owner |
|---|---|
| Decision Packet meaning and gate aggregation | [Decision Packet](../reference/kernel.md#decision-packet), [Decision Gate](../reference/kernel.md#decision-gate) |
| Public request and answer shapes | [`harness.request_user_decision`](../reference/mcp-api-and-schemas.md#harnessrequest_user_decision), [`harness.record_user_decision`](../reference/mcp-api-and-schemas.md#harnessrecord_user_decision) |
| Sensitive-action Approval | [Approval](../reference/kernel.md#approval) |
| Evidence sufficiency | [Evidence Gate](../reference/kernel.md#evidence-gate) |
| Verification and verification waiver impact | [Verification Gate](../reference/kernel.md#verification-gate) |
| Manual QA and QA waiver impact | [QA Gate](../reference/kernel.md#qa-gate) |
| Final acceptance and residual-risk visibility | [Acceptance Gate](../reference/kernel.md#acceptance-gate), [Residual Risk](../reference/kernel.md#residual-risk) |
| Close blockers and close reasons | [`close_task`](../reference/kernel.md#close_task) |

## Good answer pattern

When you answer a Decision Packet, choose the option in ordinary language and add any boundary you care about:

```text
Choose inline failed-login feedback. Keep the message generic, do not add a modal, and keep account recovery out of scope for this task.
```

That kind of answer is useful because it resolves the named choice without pretending to grant every other authority. The agent still needs the normal owner paths for Write Authorization, evidence, QA, verification, final acceptance, residual-risk acceptance, and close.
