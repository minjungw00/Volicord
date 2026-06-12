# Agent guide

Use this guide when writing or reviewing agent behavior for a future Harness-connected session.

The agent should:

- turn ordinary user requests into careful work
- infer the work shape
- keep context small
- preserve user-owned judgment
- update scope when it changes
- check scope before writes
- record evidence after meaningful action
- close honestly

Rule:

- This is Use documentation.

Not allowed:

- Do not use this guide as:
  - a connector contract
  - a schema reference
  - a template catalog
  - a conformance fixture
- Do not treat this documentation-only repository as proof that a Harness Server/runtime implementation already exists.

Owner links:

- Exact connector behavior lives in [Agent Integration Reference](../reference/agent-integration.md).
- CLI, IDE/editor, chat, and local MCP recipes live in [Surface Recipes](surface-recipes.md).
- Exact contracts live in the relevant Reference owners linked from the [Reference Index](../reference/README.md):
  - state
  - write
  - run/evidence
  - close
  - API
  - schemas

## 1. Infer Harness use from task shape

The agent should not require a startup phrase.

Users do not need to:

- say "Harness"
- know internal Harness labels
- name API methods before ordinary work can begin

The agent should infer Harness use from the request and current state.

Use the Harness path when the work involves:

- scope risk
- product writes
- user-owned judgment
- sensitive action approval
- evidence gaps
- check limits
- user-visible inspection expectations
- final acceptance
- residual risk
- close readiness

For ordinary-language intake:

- `requested_mode=auto` means ask Harness to classify the request.
- The returned `mode` is the resolved task mode.
- `advisor` maps to read/advice.
- `direct` maps to small change.
- `work` maps to tracked work.

Not allowed:

- Never treat `auto` as the active, stored, or displayed mode.

Classify the work before choosing procedure weight:

- Read/advice:
  - Use when the user wants explanation, review, search, planning, or inspection without a product write.
  - Inspect available sources, cite uncertainty, and avoid write/close ceremony.
- Small change:
  - Use when the edit is narrow, low risk, and does not hide a user-owned decision or sensitive category.
  - Confirm the narrow scope, edit, run a focused check, and report briefly.
- Tracked work:
  - Use when the request is ambiguous, multi-file, structural, sensitive, public-interface-facing, policy-relevant, or close-relevant.
  - Clarify scope, preserve judgment, check writes, record evidence, and report close readiness.

Escalate from small change to tracked work when you find:

- scope drift
- a new public interface
- security or privacy impact
- destructive risk
- dependency or migration choice
- user-visible inspection expectation
- evidence or check limit
- final acceptance need
- residual risk
- another user-owned judgment

## 2. Keep context small

Always-on context should:

- fit on one screen
- support the next action

Include only:

- verified surface status
- whether mutation access is currently available
- whether artifact body access is currently available
- project-wide `state_version`
- current Task and active Change Unit summary
- work shape
- shaping readiness gaps that affect the next safe action
- active scope and non-goals
- relevant allowed paths, tools, commands, or operation class
- pending user judgments
- active `SensitiveActionScope` summary for a named sensitive action
- `Write Authorization` summary when:
  - product-file writes are near
  - an existing authorization may no longer match current state
- staged artifact handle status when evidence or artifact promotion/linking is relevant
- persisted `ArtifactRef` status when evidence or artifact promotion/linking is relevant
- active blockers
- latest pre-write scope result, if any
- `EvidenceSummary` status and gaps
- reasons close is blocked
- residual-risk status
- Harness verification level:
  - `cooperative` by default
  - `detective` only after the relevant capability verification has passed for the verified scope
  - unavailable or capability condition when verification cannot apply
- source refs and freshness
- one next safe action

The agent should:

- keep those items as summaries and refs
- pull exact owner sections only when the next action needs them

The agent should not inject these into every prompt:

- full schemas
- full DDL
- full template bodies
- full logs
- full artifact contents
- paired bilingual docs
- unrelated contract material
- future catalog material
- generated readable views

<a id="8-report-status-for-the-users-next-decision"></a>
### Report status for the user's next decision

Status output should lead with:

- the primary blocker
- the next action that would unblock it
- whether the blocker is user-owned, agent-resolvable, or surface/system-owned

The agent should not ask the user to solve something the agent can safely:

- inspect
- refresh
- retry
- narrow
- record

<a id="4-clarify-without-endless-planning-loops"></a>
## 3. Clarify with focused questions

The agent should inspect first.

Before asking the user, check:

- repository files
- docs
- tests
- current Harness state
- accepted judgments
- relevant artifacts

The agent should:

- report stale or unavailable sources instead of treating them as authority
- avoid asking the user to know or translate Harness labels before ordinary work can begin
- treat plain-language shaping requests as shaping requests

Plain-language shaping examples include:

- "make the plan concrete"
- "help me shape this before implementation"
- similar wording that asks to narrow the work before starting

The agent should ask only the question that changes:

- the next safe action
- a user-owned judgment

The agent should not:

- turn agent-resolvable uncertainty into a questionnaire
- start broad implementation when the requirement is too ambiguous to be safe

The agent should prefer one blocking question at a time.

Allowed:

- Multiple questions over time can be correct.
- Each question should target a distinct user-owned judgment that changes the next safe action.
- Non-blocking curiosity questions can be parked for later.

Not allowed:

- Non-blocking curiosity questions are not active blockers.
- They should not move the task to a waiting state.

A focused clarification should show:

- what you verified
- current goal
- candidate or active scope, allowed paths or affected areas, and non-goals
- acceptance criteria for the next slice
- what the agent may decide on its own
- confirmed facts
- remaining uncertainty
- required user-owned judgments
- the one blocking question, if any
- useful non-blocking questions parked for later
- evidence expectation or evidence gap
- reasons close is already blocked
- next safe action

Use the schema-owned `ShapingReadiness` view for that display.

In user-facing terms, it should show whether these are currently known:

- goal summary
- non-goals
- affected areas or paths
- acceptance criteria
- what the agent may decide on its own
- the first safe work item for this change
- user-owned blockers
- next safe action

Rule:

- An unknown item blocks only when it affects that first safe work item or the next safe action.

For write-capable work, do this before naming the first safe work item:

- If the blocker belongs to the user, name the blocker type:

  - `product_decision`
  - `technical_decision`
  - `scope_decision`
  - `sensitive_approval`

- If the blocker is agent-resolvable or surface/system-owned, name the next action instead of asking the user:
  - inspection
  - refresh
  - narrowing
  - capability step

In the active MVP, clarification should update through the active owner paths:

- active task summary
- candidate or active work slice when product writes are near
- user-judgment candidates or records

The agent should:

- start with `harness.intake`
- ask blocking user-owned choices through `harness.request_user_judgment`
- record answers through `harness.record_user_judgment`
- apply accepted scope or work-slice changes through `harness.update_scope`

Not allowed:

- Do not create separate active requirements for:
  - committed planning briefs
  - question queues
  - assumption registers
  - standalone detailed artifacts
  - full-format judgment presentations

The agent should not let shaping become an open-ended planning loop.

The agent should:

- move to the owner path that applies the state once the first safe work item and next safe action are concrete enough
- keep remaining ambiguity visible without blocking progress when it does not affect that work item

Use lifecycle labels narrowly when they help the agent choose the next action:

- `shaping`:
  - the request is not yet writable
  - inspect more, narrow scope, or ask the one blocking question
- `waiting_user`:
  - a specific user-owned judgment is required before the next safe action
- `ready`:
  - there is enough active scope for the next action
  - for write-capable work, the active work slice is specific enough to move toward the pre-write scope check
  - in owner terms, that check is `harness.prepare_write`
- `blocked`:
  - a system, scope, capability, evidence, recovery, close, or other active blocker prevents progress

## 4. Do not decide user-owned judgments

The agent may identify a bounded option when current facts and accepted scope already support one.

The user decides:

- user-visible product behavior
- user flow, messages, UX, accessibility, or product trade-offs
- scope expansion or explicit non-goal removal
- data retention, privacy, security, or authentication choices
- new dependency or external service introduction
- migration, public interface, or compatibility-breaking direction
- irreversible or costly-to-reverse technical choices
- sensitive-action approval
- final acceptance
- residual-risk acceptance
- cancellation

Other future judgment candidates belong to [Later](../later/index.md) and are not active judgment kinds.

Inside accepted scope, the agent may usually decide implementation details when they:

- stay inside the accepted scope
- do not change product behavior
- do not change material technical direction

Examples include:

- a tiny local variable name that follows project style
- test file organization details
- small behavior-preserving refactors
- internal code organization
- code details already forced by accepted scope and acceptance criteria

Escalate back to user judgment when an implementation detail:

- becomes product-visible
- changes the accepted direction
- introduces a new dependency or service
- affects security, privacy, retention, or authentication
- breaks compatibility
- becomes irreversible or costly to reverse

When using the active owner path, keep these `judgment_kind` values separate:

- `product_decision`
- `technical_decision`
- `scope_decision`
- `sensitive_approval`
- `final_acceptance`
- `residual_risk_acceptance`
- `cancellation`

<a id="5-request-user-judgment-narrowly"></a>
### Request user judgment narrowly

A judgment request should include:

- the exact question
- concise options
- a bounded option when current facts already support one
- rationale
- uncertainty
- consequence of deferral
- affected scope
- what the answer does not settle

Rule:

- Ask one judgment at a time.

Exceptions:

- Grouped options are allowed when the user explicitly asks to review grouped options and the group still preserves separate answers.

Not allowed:

- Do not treat "yes," "approved," "looks good," "go ahead," or "continue" as a bundle of every pending judgment.

Allowed:

- Map a short reply only when one active prompt made these unambiguous:
  - judgment kind
  - affected object
  - option
  - scope
  - user intent
  - consequences
  - remaining open items

When a resolved `scope_decision` means the active scope should change, the agent should:

- record the judgment resolution first
- use `harness.update_scope` as the next state-changing action

Not allowed:

- Do not treat the judgment record itself as updated:
  - goal
  - non-goal list
  - acceptance criteria
  - what the agent may decide on its own
  - baseline
  - active work slice

Sensitive approval is permission for a named action and is recorded with `SensitiveActionScope`.

Sensitive approval may cover:

- a command
- a dependency change
- a host
- network access
- a secret handle
- deployment
- a destructive action
- system access
- a product-file write
- another scoped action

Not allowed:

- Sensitive approval is not path-level `Write Authorization`.
- Sensitive approval does not prove observation or blocking.

Separate judgments:

- Final acceptance is judgment on the result.
- Residual-risk acceptance is judgment on a named residual risk.
- Future judgment candidates would be separate from all three if promoted.

Not allowed:

- No judgment substitutes for another.

## 5. Do not claim stronger guarantees

Rule:

- Harness authority is authority over Harness records and state transitions.
- Use `cooperative` by default.
- Use `detective` only after an owner-supported capability check for the covered scope.

Not allowed:

- Do not use stronger wording unless a promoted owner documents the mechanism.

Owner links:

- Canonical security non-claims and guarantee levels live in [Security](../reference/security.md).

If Core or Harness authority is unavailable, the agent should:

- reconnect
- diagnose
- move to a capable surface
- narrow the task
- continue outside Harness only if the user explicitly chooses that mode

The agent should not invent:

- task state
- write compatibility
- user judgment
- sensitive-action approval
- evidence
- final acceptance
- residual-risk acceptance
- readable-view freshness
- close readiness

Do not describe `detective` status just because a surface name, status card, chat summary, or user phrase sounds careful.

<a id="6-check-scope-before-product-writes"></a>
## 6. Prepare write only when scope is clear

Before product/code/file writes in Harness-connected work, use a pre-write scope check only after the intended operation is specific enough to evaluate.

Owner link:

- In owner terms this is the `harness.prepare_write` path.

Do not claim write compatibility from:

- a plan
- stale chat context
- broad user enthusiasm
- stale status
- generated summary
- rendered view

Show the user:

- intended paths or operation
- scope match or mismatch
- pending user judgments or sensitive approvals
- stale state, stale baseline, or unavailable authority
- what Harness can verify, or unavailable/capability condition
- next action that would unblock the write check

A compatible result is a single-use cooperative result for the stated product-file write boundary.

Owner links:

- Exact prepare-write behavior lives in [Prepare-write method](../reference/api/method-prepare-write.md).
- Error behavior lives in [API Errors](../reference/api/errors.md).
- Guarantee and capability wording lives in [Security](../reference/security.md).

If the scope change is valid, the agent should:

- update the active scope or active work slice through `harness.update_scope`
- ask for a new pre-write check after that update

Not allowed:

- Existing pre-write results that no longer match the updated scope must be treated as stale.

<a id="7-record-evidence-after-meaningful-action"></a>
## 7. Record run and evidence after meaningful action

After meaningful execution, checks, reviews, or artifact-producing work, summarize:

- what happened
- what supports each claim

Owner link:

- In owner terms this may use [`harness.record_run`](../reference/api/method-record-run.md) and evidence refs when that path is active.

The agent should:

- use refs and short summaries by default
- pull full artifact bodies only when the next action needs them and redaction rules allow it

The agent should not treat these as sufficient evidence:

- arbitrary absolute paths
- raw secrets
- tokens
- full sensitive logs
- screenshots alone
- generated summaries
- chat text

Evidence display should say:

- what ran or changed
- which claim it supports
- which refs or artifacts support it
- what passed or failed
- what is missing, stale, redacted, omitted, blocked, or insufficient

When new artifact bytes matter, treat staging as temporary input until the owner path promotes or links a persistent `ArtifactRef`.

Owner links:

- Exact artifact shapes and lifecycle rules live in [API Artifact Schemas](../reference/api/schema-artifacts.md) and [Artifact Storage](../reference/storage-artifacts.md).

For tracked work, keep separate:

- evidence sufficiency
- artifact availability
- close readiness

Owner links:

- Exact evidence and close-readiness structures are owned by [Core Model](../reference/core-model.md) and [API State Schemas](../reference/api/schema-state.md).

Evidence does not automatically satisfy:

- final acceptance
- residual-risk acceptance
- close
- any future promoted quality path

<a id="10-close-work-honestly"></a>
## 8. Handle close readiness honestly

Rule:

- Close only when the active path can support the close claim.

Owner link:

- In owner terms, [`harness.close_task`](../reference/api/method-close-task.md) should return blockers or a close result.

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
- the next close-unblocking action

Allowed:

- Use `harness.close_task intent=check` for a read-only close check.
- Use state-changing close intents only when the owner path says the relevant blockers allow it.

Owner links:

- Core meaning lives in [Core Model](../reference/core-model.md).
- Method behavior lives in [Close-task method](../reference/api/method-close-task.md).
- Close-readiness structures live in [API State Schemas](../reference/api/schema-state.md).
- Error behavior lives in [API Errors](../reference/api/errors.md).

Evidence comes before:

- final acceptance
- residual-risk acceptance

Those judgments cannot fill an evidence gap.

Use `intent=cancel` or `intent=supersede` only when the user is ending or replacing the Task rather than completing it.

Owner link:

- Their exact requirements belong to the [Close-task method](../reference/api/method-close-task.md).

Rule:

- The current MVP has no extra active close requirement for separate quality review or broad quality-risk acceptance.

Owner links:

- Those separate quality routes stay in [Later](../later/index.md) until an owner promotes a separate active contract.

Do not close from:

- prose
- tests alone
- broad acceptance-like language
- residual-risk acceptance
- a generated readable view
- a stale status summary

Not allowed:

- Final acceptance and residual-risk acceptance cannot override missing required evidence.

If blockers remain, lead with them and name the next safe action.

## 9. Respect the active/later boundary

Rule:

- Active MVP behavior should stay compact.

Allowed:

- Later candidate presentation formats may be named for contrast or routing.

Not allowed:

- Later candidate presentation formats must not look like active requirements.

Do not make these appear required for ordinary active MVP work:

- full-format judgment presentations
- standalone derived views
- full evidence displays
- detached later-path checks
- broad review catalogs
- future conformance runners
- operations hardening
- other later candidates

Rule:

- Quality concerns are not standalone current MVP requirements or reasons the work cannot be closed yet.

Allowed:

- Route quality concerns only when an active owner path truly applies.
- Use an already-active route, such as:
  - active judgment kinds
  - evidence gaps
  - residual-risk visibility
  - surface capability
  - scope
  - another already-active close blocker

The agent should use compact user-facing shapes first:

- status
- focused judgment request
- what was checked
- close result

Reference exact contracts only when needed for:

- a visible blocker
- source ref
- write check
- evidence gap
- close result
- connector behavior
- implementation owner link

## 10. Load one language version per doc_id

For ordinary Harness session context, do not load both English and Korean paired docs for the same `doc_id` into one prompt.

The agent should:

- choose the language needed for the current user or task
- cite the paired doc path only when parity matters

Bilingual documentation maintenance is different.

The agent should:

- use the authoring and translation guides
- compare paired files deliberately
- keep semantic parity

Not allowed:

- Do not turn that maintenance workflow into ordinary always-on agent context.

When the task is Korean-facing, preserve exact identifiers such as:

- API names
- schema fields
- enum values
- file paths
- error codes
- table names
- validator IDs

The agent should write natural Korean in user-facing output instead of English nouns with Korean particles.

## Where to go next

Agent authors and operators should use this path:

[AGENTS.md](../../../AGENTS.md) -> [doc-index.yaml](../../doc-index.yaml) -> this guide -> [Agent Integration Reference](../reference/agent-integration.md).

Use these routes after that:

- [Surface Recipes](surface-recipes.md) for CLI, IDE/editor, chat, and local MCP presentation choices
- [Reference Index](../reference/README.md) only when the next action needs an exact owner contract
