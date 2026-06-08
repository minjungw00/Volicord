# Judgment Examples

Use this compact catalog after the [User Guide](user-guide.md) when a task is blocked by a choice the agent should not make alone. It also includes one counterexample for details the agent may usually decide inside accepted scope. These examples show active judgment-request behavior and judgment-boundary guidance, not runtime records, generated evidence, acceptance records, or conformance outputs from this documentation repository.

The active user path is focused: ask with `harness.request_user_judgment` when a choice blocks work, and record the answer with `harness.record_user_judgment` when the user answers. Users should not need a special label to answer ordinary prompts.

Each judgment example asks for one judgment, names what the answer settles, and names what it does not settle. The implementation-detail example names why no `UserJudgment` is needed.

## 1. Product Choice

Kind: `product_decision`

Use when user-visible behavior, copy, flow, messages, UX, or accessibility trade-offs must be chosen before implementation or review can finish.

```text
Judgment needed: choose the Save feedback pattern.

Options:
- Existing UI message layer near the saved form.
- Toast that confirms the save without blocking the flow.
- Modal that interrupts the flow.

Decision basis: toast keeps success feedback non-blocking; the existing UI message layer fits messages tied to a field or error.

If deferred: save-state wiring can continue, but final UI behavior, screenshots, and user-visible inspection remain blocked.

Settles: Save feedback pattern.
Does not settle: broader settings workflow, localization strategy, final acceptance, residual-risk acceptance, or `harness.prepare_write`.
```

## 2. Technical Choice

Kind: `technical_decision`

Use when architecture, dependency or external service introduction, authentication direction, migration, public interface, security, privacy, retention, or compatibility choices materially affect the work.

```text
Judgment needed: choose the login session direction.

Options:
- Server-side session auth for first-party web login.
- Token auth with a client-handled JWT or bearer token.
- Social login through an OAuth/OIDC provider with a local session or token strategy.

Required basis before choosing: inspect the current auth model first. If this is a first-party web app without external identity-provider requirements, server-side session auth is likely the conservative default.

Uncertainty: current clients, revocation needs, SSO requirements, deployment constraints, and migration cost.

If deferred: current auth code can be inspected and a narrow slice can be proposed, but storage, token lifetime, middleware behavior, and provider integration should not be committed.

Settles: session architecture direction.
Does not settle: failed-login UX, rate limits, audit logging, final acceptance, or dependency install approval.
```

## 3. Scope Choice

Kind: `scope_decision`

Use when the next safe action would expand scope, remove a non-goal, or touch a path the user has excluded.

```text
Judgment needed: decide whether to expand scope beyond `src/auth`.

Observed: the fix appears to require a shared session helper in `src/session`.

Options:
- Keep the original `src/auth` boundary and report the helper change as blocked.
- Expand scope to include the named helper file in `src/session`.
- Narrow this task to read-only investigation and return with a concrete follow-up.

Safe boundary if expanded: include only the named helper file when that change is required for the login fix and no unrelated session behavior changes.

If deferred: inspection can continue, but product-file writes outside `src/auth` stay blocked.

Settles: whether this task may include the named `src/session` helper path.
Does not settle: login architecture, sensitive-action approval, final acceptance, residual-risk acceptance, or `harness.prepare_write`.
```

## 4. Sensitive Action Approval

Kind: `sensitive_approval`

Use when the user must permit one named sensitive action. Keep this separate from the technical decision to adopt the result.

```text
Judgment needed: approve one dependency install/update action for this task.

Covered if approved:
- Action: install or update one named dependency version.
- Command/tool: run the named package-manager command only.
- Intended paths: dependency manifest and lockfile only.
- Hosts: the named package registry host only.
- Dependencies: the named dependency and version only.
- Secret handles: the named registry credential handle, or none.
- Time window: this task and approval window only.
- Scope limit: no future installs, upgrades, or broad package changes.
- Explicitly not authorized: unrelated dependencies, production deploy, secret printing, broad network requests, or product behavior decisions.
- Capability claim: record this as cooperative approval unless the active surface can honestly observe the exact action.

Options:
- Approve this scoped install/update action.
- Deny and continue with a no-new-dependency path if one exists.
- Ask for a separate technical judgment before any install approval.

Settles: permission for the named install/update action.
Does not settle: whether the dependency is the right architecture, future installs, product-file Write Authorization for manifest or lockfile changes, product writes outside scope, evidence sufficiency, final acceptance, or residual-risk acceptance.
```

## 5. Final Acceptance

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
Does not settle: residual risk, missing required evidence, scope expansion, future editorial work, or acceptance of unrelated files. A plain "looks good" counts only if this exact final-acceptance question was pending.
```

## 6. Residual Risk Acceptance

Kind: `residual_risk_acceptance`

Use when a named residual risk is visible and the active close path requires the user to accept or reject that risk.

```text
Judgment needed: accept the residual risk that password reset remains out of scope for this login slice.

Risk: users who forget their password still cannot recover access through this change.

Evidence: scope and non-goals exclude password reset; the close claim covers sign-in for existing users with known credentials.

Options:
- Accept this named residual risk and close with the risk visible.
- Do not accept; keep close blocked until password reset is added or the close claim is narrowed.
- Narrow the close claim to email/password sign-in only.

Required basis for acceptance: accept only if this task was intentionally limited to the login slice and account recovery will be handled separately.

Settles: acceptance of this named residual risk.
Does not settle: final acceptance of the whole result, other residual risks, or missing required evidence.
```

## 7. Cancellation Or Defer Decision

Kind: `cancellation` when stopping the task; otherwise no successful close judgment yet.

Use when the user decides to stop, pause, or defer instead of choosing an implementation path or accepting close.

```text
Judgment needed: cancel or defer the login task.

Options:
- Cancel the task with no successful result.
- Defer the technical choice and keep the task open.
- Narrow the task to read-only investigation and return later.

Safe boundary if deferred: defer rather than cancel only when the goal still matters but the architecture choice is not ready.

If deferred: the agent may keep notes on inspected facts and blockers, but must not claim implementation completion or final acceptance.

Settles: whether the current task stops, waits, or narrows.
Does not settle: product direction, technical direction, sensitive-action approval, final acceptance, residual-risk acceptance, or close readiness for a completed result.
```

## 8. Usually Agent-Owned Implementation Detail

Kind: usually no `UserJudgment` when already inside accepted scope.

Use this distinction when the detail follows accepted scope and acceptance criteria and does not change product behavior, technical direction, scope, sensitive-action need, final acceptance, or residual risk.

```text
No user judgment needed: choose a tiny local variable name while implementing the accepted login slice.

Known scope:
- The user accepted the failed-login feedback pattern.
- The user accepted the login authentication direction.
- The affected file and behavior are already inside the accepted work boundary.

Agent-owned detail:
- Rename `tmp` to `normalizedEmail` because it follows project style and clarifies an internal value.
- Keep the test in the existing nearby test file because that is the local organization pattern.

Escalate instead if:
- the naming or cleanup changes user-visible copy or behavior
- the refactor changes public interfaces, auth direction, privacy/security/retention behavior, or compatibility
- the work needs a new dependency, external service, migration, sensitive action, scope expansion, or costly-to-reverse technical choice

Settles: nothing in `UserJudgment`; this is ordinary agent implementation latitude.
Does not settle: product judgment, technical judgment, scope judgment, sensitive-action approval, final acceptance, or residual-risk acceptance.
```
