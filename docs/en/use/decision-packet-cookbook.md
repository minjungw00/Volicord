# Judgment Request Cookbook

## Ask For One Focused Judgment

Use this after [User Guide](user-guide.md) when work is blocked by a choice the agent should not make alone. A good judgment request shows the choice, realistic options, consequence, what can still continue, and what remains blocked. It should feel like decision support, not a schema form or a blank permission slip.

Status note: these are documentation examples for planned Harness behavior. They are not `user_judgment` records, acceptance records, evidence manifests, or other runtime outputs from this repository.

The everyday label is "judgment request." The internal record family is `user_judgment`. "Decision Packet" is a full-format or legacy presentation label for complex judgments; users should not need that label to answer ordinary prompts.

Before asking, the agent should check what the repository, docs, tests, current Harness state, accepted decisions, current task artifacts, or available evidence already answer. Do not ask the user to re-answer facts the project can answer. Ask only for judgments the user owns, and keep answerable facts, blocking questions, and useful non-blocking questions separate.

## Use It For

Use a judgment request when the next safe action depends on one of these user-owned choices:

- Product/UX judgment
- Technical judgment
- Sensitive action approval
- Work acceptance
- Residual risk acceptance

Do not merge these into one "approve?" prompt. Permission to install a dependency is not the same as adopting that dependency as the architecture. Work acceptance is not the same as accepting known residual risk. A QA waiver is not evidence that QA passed.

## Good Shape

A good judgment request normally answers:

- what the agent already checked, when that context affects the choice
- what judgment is needed now
- why it blocks the next safe action or close
- which options are realistic
- what the agent recommends, when a recommendation is appropriate
- what is uncertain
- what can continue if the user defers
- what does not get settled by the answer
- what evidence, QA, verification, acceptance, or residual risk may be affected

Small unblockers can stay short. Complex, security-sensitive, close-relevant, or architecture-shaping choices need fuller trade-offs.

A judgment request is not a general requirements questionnaire. If the decision depends on repository or documentation facts, inspect those first or say which source is unavailable, then ask the user for the judgment that remains.

## Tiny Product Judgment

Use this when a small user-visible choice is real but does not need a full trade-off prompt.

User request:

```text
Show a message after clicking Save.
```

Good agent response:

```text
Judgment request: choose the Save feedback pattern.

Options:
- Inline message near the saved form.
- Toast that confirms the save without blocking the flow.
- Modal that interrupts the flow.

Recommendation: use a toast for a non-blocking success message. Use inline text instead if the message is tied to a specific field or error.

If you defer: save-state wiring can continue, but final UI behavior, screenshots, and human QA should wait.

This settles only the Save feedback pattern. It does not settle broader settings workflow, localization strategy, work acceptance, residual-risk acceptance, or write authority.
```

Why this is good: it asks for one bounded product/UX judgment without pretending the answer approves every later step.

Bad agent response:

```text
I will add a modal after Save because it is the safest way to make users notice the message.
```

Why this is bad: it silently chooses a disruptive UX pattern and does not check whether inline or toast behavior fits the product.

## Product/UX Judgment: Inline Message Vs Toast Vs Modal

Use this when user-visible behavior must be chosen before implementation or QA can finish.

```text
Judgment request: choose the failed-login feedback pattern.

Options:
- Inline message near the form fields.
- Toast after failed submit.
- Modal that interrupts the flow.

Recommendation: choose the inline message. It stays visible, fits the form context, and is usually easier to make accessible.

Uncertainty: the agent still needs to confirm existing design-system support for inline errors and screen-reader announcement behavior.

If you defer: API error mapping and state plumbing can continue, but final UI behavior, final copy, screenshots, and human QA should wait.

This does not settle: login architecture, account recovery, work acceptance, residual-risk acceptance, or permission for sensitive steps.
```

Why this works: it asks for the UX choice instead of asking the user to "approve the login change."

## Technical Judgment: Auth Direction

Use this when an authentication direction affects storage, revocation, client behavior, migration, or security posture.

User request:

```text
Build a login feature.
```

Good agent response:

```text
Judgment request: choose the login session architecture.

Options:
- Server-side session cookie for first-party web login.
- JWT or bearer token handled by the client.
- OAuth/OIDC identity provider, with a separate local session or token strategy when needed.
- Social-login provider integration, with provider-specific account linking and support implications.

Recommendation: inspect the current user/session model before choosing. If this is a first-party web app and there is no current third-party identity-provider requirement, server-side session cookie is likely the conservative default.

Uncertainty: current client mix, existing auth middleware, revocation requirements, SSO requirements, deployment constraints, and migration cost.

If you defer: the agent can inspect current auth code and draft a narrow work slice, but should not commit to storage, token lifetime, middleware behavior, or identity-provider integration.

This does not settle: failed-login UX, audit logging, rate limits, work acceptance, or permission to install dependencies.
```

Why this is good: it separates identity-provider choice from session/storage strategy. OAuth/OIDC may still need a local session or token strategy.

Bad agent response:

```text
I will use JWTs and add social login now; those are the modern defaults.
```

Why this is bad: it makes a material technical decision without checking the current app, deployment, revocation needs, or user-owned scope.

## Technical Judgment: PII Logging

Use this when a feature, debug path, run, export, or artifact might expose personal data.

```text
Judgment request: choose the login diagnostics logging policy.

Options:
- Do not log PII; use request IDs and nonidentifying error codes.
- Log redacted or tokenized identifiers.
- Log limited raw fields for a short retention window with audit controls.

Recommendation: do not log raw PII. Use request IDs, plus redacted or tokenized identifiers only if debugging truly needs them.

Uncertainty: support requirements, retention policy, compliance obligations, and whether existing log redaction is proven.

If you defer: implementation can continue without PII logging, but diagnostics that require user identifiers should wait.

This does not settle: permission for any sensitive command, artifact redaction evidence, work acceptance, or residual-risk acceptance.
```

Why this works: it treats privacy as a user-owned technical judgment, not a hidden implementation detail.

## Security/Privacy Judgment: API Key Handling

Use this when the user asks where a secret should live or proposes putting a key in source code.

User request:

```text
Can I put the API key in the code?
```

Good agent response:

```text
No. Do not put a raw API key in source code.

Safe next action:
- inspect the repo's config, deployment docs, and ignored local files;
- recommend the existing secret path if one exists, such as environment variables, a platform secret, a local ignored secret file, or a secret manager;
- flag any current committed key as a security issue that needs rotation guidance.

This does not settle: permission to change deployment settings, proof that a key has not leaked, residual-risk acceptance, or any claim that Harness provides OS-level secret isolation or tamper-proof storage.
```

Why this is good: it gives clear security guidance while keeping the local mechanism and remaining risk visible.

Bad agent response:

```text
Yes. Hard-code the key and rely on Harness to keep it safe.
```

Why this is bad: it creates a secret leak risk and overclaims Harness security guarantees.

## Sensitive Action Approval: Dependency Install

Use this when the user must permit a named sensitive action without treating that permission as the architecture judgment.

```text
Judgment request: allow one dependency install/update action for this task.

Allowed if you approve:
- install the named dependency and version for this task
- update the named dependency manifest and lockfile
- use the approval only within this task and approval window

Options:
- Allow this scoped install/update action.
- Deny it and continue with a no-new-dependency path if one exists.
- Ask for a separate architecture judgment before any install approval.

This does not settle: whether the dependency is the right architecture direction, future installs, product writes outside scope, QA or verification waiver, work acceptance, or residual-risk acceptance.
```

Why this works: sensitive action approval is separate from the technical judgment to adopt a dependency.

## Technical Judgment: QA Expectation Or Waiver

Use this when human QA is expected or required, but the user must decide whether to perform it, waive it, or keep close blocked.

```text
Judgment request: decide how to handle Manual QA for the responsive login layout.

Options:
- Perform Manual QA now.
- Waive Manual QA for this close and separately handle any visible residual risk.
- Keep the task open and schedule QA before close.

Recommendation: perform Manual QA for a user-facing login workflow. Waive only if the environment is unavailable and the change is low risk or time-bound.

Uncertainty: small-screen layout, keyboard flow, screen-reader interpretation, and visual polish have not been inspected by a human.

If you defer: implementation can remain complete, but close should stay blocked until Manual QA passes or a valid waiver and any required residual-risk acceptance are recorded.

This does not settle: evidence sufficiency, verification, work acceptance, or residual-risk acceptance.
```

Why this works: it names the skipped human inspection. A QA waiver does not prove QA passed.

### Deferred User Testing

Use this when the user says they will test later.

User request:

```text
I will test it later.
```

Good agent response:

```text
I will show Manual QA as deferred, not passed. The implementation can remain ready for your later check, but close stays blocked unless the active path allows a QA waiver and any visible residual risk is handled separately.

This does not settle: work acceptance, Manual QA pass, verification, or residual-risk acceptance.
```

Why this is good: it preserves the evidence gap instead of turning future testing into current evidence.

Bad agent response:

```text
Great. I will mark Manual QA complete and close the work.
```

Why this is bad: later testing is not evidence that QA passed.

## Technical Judgment: Verification Expectation Or Waiver

Use this when independent verification is required or expected, but the user wants to proceed without it.

```text
Judgment request: decide whether to waive detached verification for the invoice export fix.

Options:
- Run detached verification from a fresh bundle or fresh worktree.
- Keep the task open until independent verification is available.
- Waive verification and close only through a risk-accepted path if the remaining risk is visible and accepted.

Recommendation: run detached verification for billing/export behavior. Waive only if blast radius is low and self-check evidence is strong.

Uncertainty: same-session bias, unreviewed export edge cases, stale bundle risk, and whether self-checks covered affected formats.

If you defer: the task cannot close as detached verified. Close either waits or uses the documented risk-accepted path when allowed.

This does not settle: work acceptance, Manual QA, or acceptance of any named residual risk.
```

Why this works: it keeps assurance language honest. A waiver does not create verification.

## Work Acceptance

Use this when the task path requires the user to accept the result after the close basis is visible.

```text
Judgment request: decide whether you accept the completed result for this task.

Shown before you answer:
- scope that was completed
- evidence that supports each completion claim
- verification status
- Manual QA status or valid waiver
- close-relevant residual risk, or an explicit "no known close-relevant residual risk" report

Options:
- Accept the result for this task.
- Do not accept it; name what must change.
- Defer acceptance until one listed blocker is resolved.

This does not settle: new writes, new sensitive-step permission, missing evidence, verification or QA waiver, or acceptance of named residual risk.
```

Why this works: work acceptance is a result judgment after the close basis is visible. It is not proof, permission, or risk acceptance.

## Residual Risk Acceptance

Use this when known close-relevant risk remains after implementation and evidence.

```text
Judgment request: decide whether to accept the legacy CSV encoding limitation for this close.

Visible risk: the export fix works for current UTF-8 files, but legacy encodings remain unsupported.

Options:
- Fix legacy encoding support before close.
- Accept the bounded risk for this close and create a follow-up.
- Cancel or supersede the task because the remaining limitation changes the requested outcome.

Recommendation: accept only if legacy encoding is rare, documented, and has an owner-visible follow-up. Otherwise fix before close.

Uncertainty: real customer frequency, support impact, and whether existing imports include legacy files.

If you defer: work acceptance or close may remain blocked until the risk is resolved, made non-close-relevant, or accepted through the owner path.

This does not settle: work acceptance, verification, QA, or proof that the risk is harmless.
```

Why this works: it makes the remaining limitation visible before acceptance and asks about the named risk, not a vague "looks good."

## Answering A Judgment Request

Answer in ordinary language and add the boundary you care about:

```text
Choose inline failed-login feedback. Keep the message generic, do not add a modal, and keep account recovery out of scope for this task.
```

That kind of answer resolves the named judgment without granting every other authority. The agent still needs the normal owner paths for write authority, evidence, QA, verification, work acceptance, residual-risk acceptance, and close.

If you answer "proceed," "go ahead," "looks good," "진행해," or "좋아," the agent must not automatically treat that phrase as sensitive action approval, work acceptance, or residual risk acceptance. If more than one judgment is pending, or if the phrase could mean permission, acceptance, risk acceptance, waiver, scope confirmation, or simple continuation, the agent should ask which judgment you mean before recording it.

User request:

```text
Looks good, go ahead.
```

Good agent response:

```text
I will apply that to the one active judgment: use inline failed-login feedback. This does not grant sensitive-action approval, work acceptance, QA waiver, or residual-risk acceptance.
```

Why this is good: it maps the user's short reply only to the named pending judgment.

Bad agent response:

```text
Invalid: I would collapse that into implementation approval, result acceptance, QA waiver, and residual-risk acceptance.
```

Why this is bad: one broad phrase cannot collapse separate judgment routes.

## Exact Owners

Use these Reference owners when exact behavior is needed:

| Need | Owner |
|---|---|
| Internal user judgment behavior and gate aggregation | [User Judgment](../reference/core-model.md#user-judgment), [Decision Gate](../reference/core-model.md#decision-gate) |
| Public request and answer shapes | [`harness.request_user_judgment`](../reference/api/mvp-api.md#harnessrequest_user_judgment), [`harness.record_user_judgment`](../reference/api/mvp-api.md#harnessrecord_user_judgment) |
| Sensitive-action Approval | [Approval](../reference/core-model.md#approval) |
| Evidence sufficiency | [Evidence Gate](../reference/core-model.md#evidence-gate) |
| Verification and verification waiver impact | [Verification Gate](../reference/core-model.md#verification-gate) |
| Manual QA and QA waiver impact | [QA Gate](../reference/core-model.md#qa-gate) |
| Work acceptance and residual-risk visibility | [Acceptance Gate](../reference/core-model.md#acceptance-gate), [Residual Risk](../reference/core-model.md#residual-risk) |
| Close blockers and close reasons | [`close_task`](../reference/core-model.md#close_task) |
