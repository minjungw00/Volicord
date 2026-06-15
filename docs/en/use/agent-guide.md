# Agent Guide

<a id="purpose"></a>
## Purpose

Use this guide when writing or reviewing agent behavior for a Harness-connected session.

A good Harness-connected agent turns ordinary user requests into careful work, keeps context small, preserves user-owned judgment, checks before writes, records evidence after meaningful action, reports status for the user's next decision, and closes honestly.

In this guide, Harness names the local work-authority product/system. Core names the local authority record for Harness state. Keep those roles separate when summarizing state, approvals, evidence, and close basis.

This guide is use documentation. It is not a connector contract, API schema, template catalog, conformance fixture, storage contract, or security guarantee.

Owner links:

- Exact connector behavior: [Agent Integration Reference](../reference/agent-integration.md)
- Surface-specific presentation: [Surface Recipes](surface-recipes.md)
- Exact API, schema, storage, security, and close readiness contracts: [Reference Index](../reference/README.md)

<a id="operating-loop"></a>
## Operating loop

Use this loop unless the user has asked only for simple advice:

1. Shape the request into a visible goal, scope, non-goals, and next safe action.
2. Inspect what the agent can safely inspect before asking the user.
3. Ask only for user-owned judgment that changes the next safe action.
4. Refresh scope before writes or sensitive actions.
5. Record meaningful execution and evidence after action.
6. Report the primary blocker, what is known, what is missing, and one next safe action.
7. Before close, separate evidence, final acceptance, residual risk, and remaining blockers.

Keep the loop light for tiny changes. Increase procedure weight when the task becomes ambiguous, multi-file, public-interface-facing, sensitive, close-relevant, or dependent on a user-owned decision.

<a id="infer-use"></a>
## Infer Harness use from task shape

The agent should not require a startup phrase. Users do not need to say "Harness", know internal labels, or name API methods before ordinary work can begin.

Use the Harness path when the work involves:

- scope risk
- product writes
- user-owned judgment
- sensitive-action approval
- evidence gaps
- check limits
- user-visible verification criteria
- final acceptance
- residual risk
- close readiness

Choose procedure weight from the work shape:

- Advice or inspection: inspect available sources, cite uncertainty, and avoid write or close ceremony.
- Small change: confirm narrow scope, edit inside that scope, run a focused check, and report briefly.
- Tracked work: clarify scope, preserve judgment, check writes, record evidence, and report close readiness.

Escalate from small change to tracked work when you find scope drift, a new public interface, security or privacy impact, destructive risk, a dependency or migration choice, user-visible verification criteria, an evidence limit, final acceptance need, residual risk, or another user-owned judgment.

<a id="keep-context-small"></a>
## Keep context small

Always-on context should fit the next action. Carry summaries and refs, then load exact owner sections only when the next action needs them.

Include only what is currently useful:

- verified surface status and capability limits
- current `Task` or work boundary
- current scope, non-goals, and relevant paths or operation class
- pending user-owned judgment
- sensitive-action approval or write-approval summary when relevant
- artifact and evidence summaries when they support a claim
- current blockers and stale-state warnings
- evidence gaps, residual-risk status, and close blockers when relevant
- guarantee level supported by the current surface context and [Security](../reference/security.md)
- source freshness
- one next safe action

Do not inject full schemas, DDL, template bodies, logs, artifact bodies, paired bilingual docs, unrelated contract material, out-of-scope catalogs, or generated readable views into every prompt.

<a id="clarify-focused"></a>
## Clarify with focused questions

Inspect first. Before asking the user, check relevant files, docs, tests, current Harness state, accepted judgments, and artifacts when they are available.

Ask only the question that changes the next safe action or resolves a user-owned judgment. Prefer one blocking question at a time. Save useful but non-blocking curiosity questions until they affect the work.

A focused clarification should show:

- what was verified
- current goal
- candidate or current scope and non-goals
- verification criteria for the next slice
- what the agent may decide on its own
- remaining uncertainty
- required user-owned judgment, if any
- evidence need or evidence gap
- why close is already blocked, if relevant
- next safe action

Unknowns block progress only when they affect the first safe work item or the next safe action. If the blocker is agent-resolvable or surface-owned, name the next action instead of asking the user.

<a id="preserve-user-judgment"></a>
## Preserve user-owned judgment

The agent may identify a bounded option when current facts and accepted scope already support one. It must not decide a user-owned choice silently.

The user decides:

- user-visible product behavior
- user flow, messages, UX, accessibility, or product trade-offs
- scope expansion or non-goal removal
- data retention, privacy, security, or authentication choices
- new dependency or external service introduction
- migration, public interface, or compatibility-breaking direction
- irreversible or costly-to-reverse technical choices
- sensitive-action approval
- final acceptance
- residual-risk acceptance
- cancellation or supersession

Inside accepted scope, the agent may usually decide local implementation details when they stay inside scope, preserve product behavior, and do not change material technical direction. Examples include a local variable name, nearby test placement, behavior-preserving refactor, or code detail already forced by accepted scope.

Escalate back to the user when a detail becomes product-visible, changes accepted direction, introduces a dependency or service, affects security or privacy, breaks compatibility, becomes costly to reverse, or changes scope, verification criteria, sensitive-action approval, final acceptance, or residual risk.

<a id="request-judgment-narrowly"></a>
### Request judgment narrowly

A judgment request should include:

- the exact question
- concise options
- a bounded recommendation when facts support one
- rationale and uncertainty
- consequence of deferral
- affected scope
- what the answer does not settle

Do not treat "yes", "approved", "looks good", "go ahead", or "continue" as a bundle of every pending judgment. Map a short reply only when one current prompt made the judgment kind, object, option, scope, user intent, consequences, and remaining open items unambiguous.

Keep product judgment, technical judgment, scope judgment, sensitive-action approval, final acceptance, residual-risk acceptance, and cancellation separate. No judgment substitutes for another.

<a id="check-before-writes"></a>
## Check before writes

Before product, code, or file writes in Harness-connected work, use the owner write path only after the intended operation is specific enough to evaluate. Exact prepare-write behavior belongs to [Prepare-write Method](../reference/api/method-prepare-write.md).

Do not claim write compatibility from a plan, stale chat context, broad enthusiasm, stale status, generated summary, or rendered view.

Show the user:

- intended paths or operation
- scope match or mismatch
- pending user judgments or sensitive approvals
- stale state or unavailable authority
- what Harness can verify, or the capability limit
- next action that would unblock the write check

If scope changes, update the current scope before asking for a new write check. Treat any old write result that no longer matches the updated scope as stale.

<a id="record-evidence"></a>
## Record evidence after action

After meaningful execution, checks, reviews, or artifact-producing work, summarize what happened and what supports each claim. Exact run/evidence behavior belongs to [Record-run Method](../reference/api/method-record-run.md), with artifact details owned by [API Artifact Schemas](../reference/api/schema-artifacts.md) and [Artifact Storage](../reference/storage-artifacts.md).

Evidence display should say:

- what ran or changed
- which claim it supports
- which refs or artifacts support it
- what passed or failed
- what is missing, stale, redacted, omitted, blocked, or insufficient

Do not treat arbitrary absolute paths, raw secrets, tokens, full sensitive logs, screenshots alone, generated summaries, or chat text as sufficient evidence by themselves.

Keep evidence sufficiency, artifact availability, close readiness, final acceptance, and residual-risk acceptance separate.

<a id="report-status"></a>
## Report status for the next decision

Status output should lead with:

- the primary blocker
- the next action that would unblock it
- whether the blocker is user-owned, agent-resolvable, or surface/system-owned

The agent should not ask the user to solve something it can safely inspect, refresh, retry, narrow, or record.

A compact status summary should include the current `Task` or work boundary, current scope, freshest relevant facts, pending judgment or approval, evidence gap when relevant, close blocker when relevant, and one next safe action.

<a id="handle-close"></a>
## Handle close honestly

Close only when the applicable path can support the close claim. In user-facing terms, close readiness asks whether the task can honestly finish now. Exact close meaning belongs to [Core Model](../reference/core-model.md); method behavior belongs to [Close-task Method](../reference/api/method-close-task.md); state shapes belong to [API State Schemas](../reference/api/schema-state.md).

For small work, a close-like result can be brief:

- request
- scope
- changed files or no-file outcome
- checks
- known residual risk

For tracked work, show the close basis before asking for final acceptance or attempting close:

- scope
- evidence
- checks
- required judgments
- residual risk
- blockers
- next close-unblocking action

Use a read-only close review when the user only asks whether close would be blocked. Use state-changing close only when the close-task method and close readiness contracts show no relevant blockers.

Do not close from prose, tests alone, broad acceptance-like language, residual-risk acceptance, generated readable views, or stale status summaries. Final acceptance and residual-risk acceptance cannot override missing required evidence.

<a id="respect-boundaries"></a>
## Respect owner and scope boundaries

Baseline behavior should stay compact. Do not make out-of-scope capability presentation formats look like supported requirements.

Do not make these appear required for ordinary baseline work:

- full-format judgment presentations
- standalone derived views
- full evidence displays
- detached checks
- broad review catalogs
- out-of-scope conformance runners
- operations control programs
- other out-of-scope capabilities

Quality concerns should route to the applicable owner when one applies, such as scope, user-owned judgment, evidence, residual-risk visibility, surface capability, or another applicable blocker. Do not invent a separate quality gate or waiver path in the use guide.

Use compact user-facing shapes first: status, focused judgment request, what was checked, and close result. Reference exact contracts only when the next action depends on the owner.

<a id="language-context"></a>
## Choose language context deliberately

For ordinary Harness session context, load the language needed for the current user or task. Do not load both English and Korean paired docs for the same `doc_id` unless translation parity is the work.

Bilingual documentation maintenance is different: use the authoring and translation guides, compare paired files deliberately, and keep semantic parity.

When the task is Korean-facing, preserve exact identifiers such as API names, schema fields, enum values, file paths, error codes, table names, and validator IDs. Write natural Korean for ordinary concepts instead of English nouns with Korean particles.

<a id="where-next"></a>
## Where to go next

Agent authors and operators should use this path:

[AGENTS.md](../../../AGENTS.md) -> [doc-index.yaml](../../doc-index.yaml) -> this guide -> [Agent Integration Reference](../reference/agent-integration.md)

Then use:

- [Surface Recipes](surface-recipes.md) for CLI, IDE/editor, chat, and local MCP presentation choices
- [Reference Index](../reference/README.md) only when the next action needs an exact owner contract
