# Judgment Examples

Use this compact catalog after the [User Guide](user-guide.md) when a task is blocked by a choice the agent should not make alone. These examples show active judgment-request behavior, not runtime records, generated evidence, acceptance records, or conformance outputs from this documentation repository.

The active user path is a focused judgment request through `user_judgment`. Full-format presentation such as `Decision Packet` is later candidate material for complex judgments. Users should not need a special label to answer ordinary prompts.

Each example asks for one judgment, names what the answer settles, and names what it does not settle.

## 1. Product Choice

Kind: `product_decision`

Use when user-visible behavior, copy, flow, UX, or accessibility trade-offs must be chosen before implementation or review can finish.

```text
Judgment needed: choose the Save feedback pattern.

Options:
- Inline message near the saved form.
- Toast that confirms the save without blocking the flow.
- Modal that interrupts the flow.

Recommendation: toast for a non-blocking success confirmation; inline if the message is tied to a field or error.

If deferred: save-state wiring can continue, but final UI behavior, screenshots, and human review remain blocked.

Settles: Save feedback pattern.
Does not settle: broader settings workflow, localization strategy, final acceptance, residual-risk acceptance, or pre-write scope check.
```

## 2. Technical Choice

Kind: `technical_decision`

Use when architecture, dependency, migration, interface, security, privacy, retention, or compatibility choices materially affect the work.

```text
Judgment needed: choose the login session direction.

Options:
- Server-side session cookie for first-party web login.
- Client-handled JWT or bearer token.
- OAuth/OIDC identity provider with a local session or token strategy.

Recommendation: inspect the current auth model first. If this is a first-party web app without external identity-provider requirements, server-side session cookie is likely the conservative default.

Uncertainty: current clients, revocation needs, SSO requirements, deployment constraints, and migration cost.

If deferred: current auth code can be inspected and a narrow slice can be proposed, but storage, token lifetime, middleware behavior, and provider integration should not be committed.

Settles: session architecture direction.
Does not settle: failed-login UX, rate limits, audit logging, final acceptance, or dependency install approval.
```

## 3. Sensitive Action Approval

Kind: `sensitive_approval`

Use when the user must permit one named sensitive action. Keep this separate from the technical decision to adopt the result.

```text
Judgment needed: approve one dependency install/update action for this task.

Covered if approved:
- install the named dependency and version
- update the named dependency manifest and lockfile
- use the approval only within this task and approval window

Options:
- Approve this scoped install/update action.
- Deny and continue with a no-new-dependency path if one exists.
- Ask for a separate technical judgment before any install approval.

Settles: permission for the named install/update action.
Does not settle: whether the dependency is the right architecture, future installs, product writes outside scope, later/reserved QA waiver, later/reserved verification-risk acceptance, final acceptance, or residual-risk acceptance.
```

## 4. QA Waiver

Kind: `qa_waiver`

Reserved path example. `qa_waiver` is not an active current MVP `UserJudgment.judgment_kind` value. It remains a later candidate in [Later Candidate Index](../later/index.md) until a future owner promotes a scoped Manual QA requirement and allows the user to perform it, waive it where allowed, or keep close blocked.

```text
Judgment needed: decide how to handle a promoted Manual QA requirement for the responsive login layout.

Options:
- Perform Manual QA now.
- Waive Manual QA for this close and keep any visible residual risk separate.
- Keep the task open and schedule QA before close.

Recommendation: perform Manual QA for a user-facing login workflow. Waive only if the environment is unavailable and the change is low risk or time-bound.

Uncertainty: small-screen layout, keyboard flow, screen-reader behavior, and visual polish have not been inspected by a person.

If deferred: implementation can remain complete. The current MVP has no Manual QA gate; close is blocked only if a future promoted owner path makes this specific Manual QA requirement close-blocking.

Settles: whether this promoted Manual QA requirement is performed, waived, or left blocking.
Does not settle: evidence sufficiency, verification, final acceptance, or residual-risk acceptance.
```

## 5. Verification Risk Acceptance

Kind: `verification_risk_acceptance`

Reserved path example. `verification_risk_acceptance` is not an active current MVP `UserJudgment.judgment_kind` value. Use it only after a future owner path has promoted a specific verification requirement and that verification is missing, incomplete, stale, or waived through an allowed path. Without that promoted requirement, route the gap as ordinary evidence, residual risk, or a narrowed claim instead of implying a verification gate.

```text
Judgment needed: accept or reject the risk of missing browser verification.

Context: automated unit tests passed, but browser verification is unavailable because the local browser surface is unavailable.

Options:
- Accept the verification risk and keep the limitation visible in close.
- Do not accept; keep close blocked only if the promoted owner path made browser verification close-blocking.
- Narrow the claim to code-level behavior only and leave UI behavior unclosed.

Recommendation: do not accept the risk for a user-facing login flow unless timing or environment constraints are more important than close confidence.

Settles: whether this named verification gap can be accepted as risk.
Does not settle: final acceptance, Manual QA, evidence sufficiency for other claims, or residual-risk acceptance beyond this named risk.
```

## 6. Final Acceptance

Kind: `final_acceptance`

Use when the visible close basis is available and the user needs to accept the finished result.

```text
Judgment needed: accept the completed typo-only documentation edit.

Visible basis:
- Scope: typo fixes only in the named file.
- Evidence: diff review shows no wording, structure, terminology, or example changes.
- Checks: link text and identifiers unchanged.
- Known gaps: no broader editorial review was requested.

Options:
- Accept the result.
- Do not accept; name what should change.
- Keep the task open for broader review.

Settles: final acceptance of the typo-only result.
Does not settle: residual risk, scope expansion, future editorial work, or acceptance of unrelated files.
```

## 7. Residual Risk Acceptance

Kind: `residual_risk_acceptance`

Use when a named residual risk is visible and the active close path requires the user to accept or reject that risk.

```text
Judgment needed: accept the residual risk that small-screen visual review was not performed.

Risk: the login form may have layout or focus-order issues on narrow mobile screens.

Evidence: desktop screenshot and unit tests exist; no mobile visual review result exists.

Options:
- Accept this named residual risk and close with the risk visible.
- Do not accept; keep close blocked until the mobile visual review is performed or the close claim is narrowed.
- Narrow the close claim to non-mobile behavior.

Recommendation: do not accept for a high-traffic login screen unless mobile review is temporarily impossible.

Settles: acceptance of this named residual risk.
Does not settle: final acceptance of the whole result, other residual risks, later/reserved QA waiver, or later/reserved verification-risk acceptance.
```

## 8. Cancellation Or Defer Decision

Kind: `cancellation` when stopping the task; otherwise no successful close judgment yet.

Use when the user decides to stop, pause, or defer instead of choosing an implementation path or accepting close.

```text
Judgment needed: cancel or defer the login task.

Options:
- Cancel the task with no successful result.
- Defer the technical choice and keep the task open.
- Narrow the task to read-only investigation and return later.

Recommendation: defer rather than cancel if the goal still matters but the architecture choice is not ready.

If deferred: the agent may keep notes on inspected facts and blockers, but must not claim implementation completion or final acceptance.

Settles: whether the current task stops, waits, or narrows.
Does not settle: product direction, technical direction, sensitive-action approval, final acceptance, residual-risk acceptance, or close readiness for a completed result.
```
