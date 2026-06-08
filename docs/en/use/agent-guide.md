# Agent Guide

Use this guide when writing or reviewing agent behavior for a future Harness-connected session. The agent's job is to turn ordinary user requests into careful work: infer the work shape, keep context small, preserve user-owned judgment, update scope when it changes, check scope before writes, record evidence after meaningful action, and close honestly.

This is Use documentation. It is not a connector contract, schema reference, template catalog, conformance fixture, or proof that this documentation-only repository already contains a Harness Server/runtime implementation. Exact connector behavior lives in [Agent Integration Reference](../reference/agent-integration.md). Exact state, write, run/evidence, close, API, and schema contracts live in the relevant Reference owners linked from the [Reference Index](../reference/README.md).

## 1. Infer Harness Use From Task Shape

Do not require a startup phrase. Users do not need to say "Harness," know internal Harness labels, or name API methods before ordinary work can begin.

Infer Harness use from the request and current state. Use the Harness path when the work involves scope risk, product writes, user-owned judgment, sensitive action approval, evidence gaps, check limits, user-visible inspection expectations, final acceptance, residual risk, or close readiness.

For ordinary-language intake, `requested_mode=auto` means ask Harness to classify the request. The returned `mode` is the resolved task mode; never treat `auto` as the active, stored, or displayed mode. The concrete modes map to the work shapes below: `advisor` for read/advice, `direct` for small change, and `work` for tracked work.

Classify the work before choosing procedure weight:

| Work shape | Use when | Behavior |
|---|---|---|
| Read/advice | The user wants explanation, review, search, planning, or inspection without a product write. | Inspect available sources, cite uncertainty, and avoid write/close ceremony. |
| Small change | The edit is narrow, low risk, and does not hide a user-owned decision or sensitive category. | Confirm the narrow scope, edit, run a focused check, and report briefly. |
| Tracked work | The request is ambiguous, multi-file, structural, sensitive, public-interface-facing, policy-relevant, or close-relevant. | Clarify scope, preserve judgment, check writes, record evidence, and report close readiness. |

Escalate from small change to tracked work when you find scope drift, a new public interface, security/privacy impact, destructive risk, dependency or migration choice, user-visible inspection expectation, evidence/check limit, final acceptance need, residual risk, or another user-owned judgment.

## 2. Keep Context Small

<a id="8-report-status-for-the-users-next-decision"></a>

Always-on context should fit on one screen and support the next action. Include only:

- verified surface status and whether mutation access or artifact body access is currently available
- project-wide `state_version`
- current Task and active Change Unit summary
- work shape
- shaping readiness gaps that affect the next safe action
- active scope and non-goals
- relevant allowed paths, tools, commands, or operation class
- pending user judgments
- active `SensitiveActionScope` summary when a named sensitive action is relevant
- Write Authorization summary when product-file writes are near or an existing authorization may no longer match current state
- staged artifact handle status and persisted `ArtifactRef` status when evidence or artifact promotion/linking is relevant
- active blockers
- latest pre-write scope result, if any
- `EvidenceSummary` status and gaps
- reasons close is blocked
- residual-risk status
- what Harness can verify: `cooperative` by default, `detective` only after the relevant capability verification has passed for the verified scope, or unavailable/capability condition
- source refs and freshness
- one next safe action

Keep those items as summaries and refs. Do not inject full schemas, full DDL, full template bodies, full logs, full artifact contents, paired bilingual docs, unrelated reference sections, future catalog material, or generated readable views into every prompt. Pull exact owner sections only when the next action needs them.

Status output should lead with the primary blocker and the next action that would unblock it. Name whether the blocker is user-owned, agent-resolvable, or surface/system-owned. Do not ask the user to solve something the agent can safely inspect, refresh, retry, narrow, or record.

## 3. Ask Focused Questions

<a id="4-clarify-without-endless-planning-loops"></a>

Inspect first. Check repository files, docs, tests, current Harness state, accepted judgments, and relevant artifacts before asking the user. If a source is stale or unavailable, say that instead of treating it as authority. Do not ask the user to know or translate Harness labels before ordinary work can begin.

Plain-language shaping requests count. If the user says "make the plan concrete", "help me shape this before implementation", or similar wording, route into shaping behavior without requiring Harness terms.

Ask only the question that changes the next safe action or a user-owned judgment. Do not turn agent-resolvable uncertainty into a questionnaire. Do not start broad implementation when the requirement is too ambiguous to be safe.

Prefer one blocking question at a time. Multiple questions over time can be correct when each one targets a distinct user-owned judgment that changes the next safe action. Non-blocking curiosity questions can be parked for later, but they are not active blockers and should not move the task to a waiting state.

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

Use the schema-owned `ShapingReadiness` view for that display. In user-facing terms, it should show whether the goal summary, non-goals, affected areas or paths, acceptance criteria, what the agent may decide on its own, the first safe work item for this change, user-owned blockers, and next safe action are currently known. An unknown item blocks only when it affects that first safe work item or the next safe action.

Before naming the first safe work item for this change in write-capable work, name the blocker type if the blocker belongs to the user: `product_decision`, `technical_decision`, `scope_decision`, or `sensitive_approval`. If the blocker is agent-resolvable or surface/system-owned, name the inspection, refresh, narrowing, or capability step instead of asking the user.

In the active MVP, clarification should update the active task summary, the candidate or active work slice when product writes are near, and user-judgment candidates or records through the active owner paths. Start with `harness.intake`; ask blocking user-owned choices through `harness.request_user_judgment`; record answers through `harness.record_user_judgment`; and apply accepted scope or work-slice changes through `harness.update_scope`. Do not create separate active requirements for committed planning briefs, question queues, assumption registers, standalone detailed artifacts, or full-format judgment presentations.

Do not let shaping become an open-ended planning loop. Once the first safe work item and next safe action are concrete enough, move to the owner path that applies the state. Remaining ambiguity can stay visible without blocking progress when it does not affect that work item.

Use lifecycle labels narrowly when they help the agent choose the next action:

- `shaping`: the request is not yet writable; inspect more, narrow scope, or ask the one blocking question.
- `waiting_user`: a specific user-owned judgment is required before the next safe action.
- `ready`: there is enough active scope for the next action; for write-capable work, the active work slice is specific enough to move toward the pre-write scope check (`harness.prepare_write` in owner terms).
- `blocked`: a system, scope, capability, evidence, recovery, close, or other active blocker prevents progress.

## 4. Do Not Decide User-Owned Judgments

<a id="5-request-user-judgment-narrowly"></a>

The agent may identify a bounded option when current facts and accepted scope already support one. The user decides user-visible product behavior; user flow, messages, UX, accessibility, or product trade-offs; scope expansion or explicit non-goal removal; data retention, privacy, security, or authentication choices; new dependency or external service introduction; migration, public interface, or compatibility-breaking direction; irreversible or costly-to-reverse technical choices; sensitive-action approval; final acceptance; residual-risk acceptance; and cancellation. Other future judgment candidates belong to [Later](../later/index.md) and are not active judgment kinds.

Inside accepted scope, the agent may usually decide implementation details that do not change product behavior, scope, or material technical direction. Examples include a tiny local variable name that follows project style, test file organization details, small behavior-preserving refactors, internal cleanup, and code details already forced by accepted scope and acceptance criteria. Escalate back to user judgment when an implementation detail becomes product-visible, changes the accepted direction, introduces a new dependency or service, affects security/privacy/retention/authentication, breaks compatibility, or becomes irreversible or costly to reverse.

When using the active owner path, keep these `judgment_kind` values separate: `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `final_acceptance`, `residual_risk_acceptance`, and `cancellation`.

A judgment request should include the exact question, concise options, a bounded option when current facts already support one, rationale, uncertainty, consequence of deferral, affected scope, and what the answer does not settle. Ask one judgment at a time unless the user explicitly asks to review grouped options and the group still preserves separate answers.

Do not treat "yes," "approved," "looks good," "go ahead," or "continue" as a bundle of every pending judgment. Map a short reply only when one active prompt made the kind, affected object, option, scope, user intent, consequences, and remaining open items unambiguous.

When a resolved `scope_decision` means the active scope should change, record the judgment resolution first, then use `harness.update_scope` as the next state-changing action. Do not treat the judgment record itself as an updated goal, non-goal list, acceptance criteria, what the agent may decide on its own, baseline, or active work slice.

Sensitive approval is permission for a named action and is recorded with `SensitiveActionScope`. It may cover a command, dependency change, host, network access, secret handle, deployment, destructive action, system access, product-file write, or other scoped action, but it is not path-level Write Authorization and does not prove observation or blocking. Final acceptance is judgment on the result. Residual-risk acceptance is judgment on a named residual risk. Future judgment candidates would be separate from all three if promoted. None substitutes for another.

## 5. Do Not Claim Stronger Guarantees

Harness authority is authority over Harness records and state transitions. It is not OS permission control, arbitrary-tool sandboxing, tamper-proof storage, universal pre-tool blocking, or security isolation unless the exact mechanism and covered operation are documented and proven.

Use guarantee wording carefully:

- cooperative: the agent is instructed to hold, ask, refresh, or proceed through the record path
- detective: Harness or a surface can report supported observable mismatch after the relevant capability check has passed
- preventive: a specific proven mechanism blocks a covered action before it happens
- isolated: a documented separation boundary exists

If Core or Harness authority is unavailable, do not invent task state, write compatibility, user judgment, sensitive-action approval, evidence, final acceptance, residual-risk acceptance, readable-view freshness, or close readiness. Reconnect, diagnose, move to a capable surface, narrow the task, or continue outside Harness only if the user explicitly chooses that mode.

Do not describe `detective` status just because a surface name, status card, chat summary, or user phrase sounds careful. Use it only after the relevant capability verification has passed and only for the covered observable fact. Otherwise describe the behavior as cooperative or unavailable/capability-limited.

## 6. Prepare Write Only When Scope Is Clear

<a id="6-check-scope-before-product-writes"></a>

Before product/code/file writes in Harness-connected work, use a pre-write scope check only after the intended operation is specific enough to evaluate. In owner terms this is the `harness.prepare_write` path.

Do not claim write compatibility from a plan, stale chat context, broad user enthusiasm, stale status, generated summary, or rendered view. Show the user:

- intended paths or operation
- scope match or mismatch
- pending user judgments or sensitive approvals
- stale state, stale baseline, or unavailable authority
- what Harness can verify, or unavailable/capability condition
- next action that would unblock the write check

A compatible result means the intended product-file write matches current Harness state and active surface capability. It is a single-use cooperative result for the stated path-level boundary. If intended product-file paths, product-write sensitive category, baseline, task, work slice, state, surface, related judgments, or the set of things Harness can verify changes, refresh the check or treat the claim as unverified/blocked. Command, dependency, host, network, secret-access, deployment, destructive-action, and system-access facts remain separate `SensitiveActionScope` or capability issues unless a future owner promotes observation support.

If the scope change is valid, update the active scope or active work slice through `harness.update_scope` before asking for a new pre-write check. Existing pre-write results that no longer match the updated scope must be treated as stale.

## 7. Record Run And Evidence After Meaningful Action

<a id="7-record-evidence-after-meaningful-action"></a>

After meaningful execution, checks, reviews, or artifact-producing work, summarize what happened and what supports each claim. In owner terms this may use `harness.record_run` and evidence refs when that path is active.

Use refs and short summaries by default. Pull full artifact bodies only when the next action needs them and redaction rules allow it. Do not treat arbitrary absolute paths, raw secrets, tokens, full sensitive logs, screenshots alone, generated summaries, or chat text as sufficient evidence.

Evidence display should say what ran or changed, which claim it supports, which refs or artifacts support it, what passed or failed, and what is missing, stale, redacted, omitted, blocked, or insufficient.

When new artifact bytes matter, use the active `harness.stage_artifact` path to create a temporary staged handle and let the owner `harness.record_run` path consume it before treating it as a persisted `ArtifactRef`. Do not use `captured_artifact`, native artifact capture, raw local paths, raw logs, or capture-adapter output as active artifact authority.

For tracked work, derive the evidence summary from the active `CompletionPolicy`. Mark each close-relevant claim as required or optional. Do not mark evidence sufficient while any required item is unsupported, partial, stale, blocked, or missing; return or surface an evidence blocker instead. Keep artifact availability separate from sufficiency: an available `ArtifactRef` supports a claim only when the evidence coverage links it to that claim.

Evidence does not automatically satisfy final acceptance, residual-risk acceptance, close, or any future promoted quality path.

## 8. Do Not Close When Blockers Remain

<a id="10-close-work-honestly"></a>

Close only when the active path can support the close claim. In owner terms, `harness.close_task` should return blockers or a close result.

For small work, a close-like result can be brief: request, scope, changed files or no-file outcome, checks, and known residual risk.

For tracked work, show the close basis before asking for final acceptance or attempting close:

- scope match
- completion policy and required evidence coverage
- evidence coverage or gap
- close-relevant artifact availability
- checks run and known check limits
- sensitive-action approval status when relevant
- final acceptance status when required
- residual-risk visibility and acceptance status when relevant
- reasons the work cannot be closed yet and the next close-unblocking action

Use `harness.close_task intent=check` for a read-only close check. Use `intent=complete` only after the complete blocker order has passed: Task validity, Run state, scope and `completion_policy`, unresolved judgments and approvals, write and baseline compatibility, surface capability, required evidence, artifact availability, final acceptance when required, residual-risk visibility, residual-risk acceptance when required, and recovery constraints. Evidence comes before final acceptance and residual-risk acceptance; those judgments cannot fill an evidence gap.

Use `intent=cancel` or `intent=supersede` only when the user is ending or replacing the Task rather than completing it. These paths still need valid Task identity, lifecycle, local access, recovery compatibility, and a valid superseding Task when applicable, but they do not require evidence sufficiency, final acceptance, or residual-risk acceptance.

The current MVP has no extra active close requirement for separate quality review or broad quality-risk acceptance. Those separate quality routes stay in [Later](../later/index.md) until an owner promotes a separate active contract.

Do not close from prose, tests alone, broad acceptance-like language, residual-risk acceptance, a generated readable view, or a stale status summary. Final acceptance and residual-risk acceptance cannot override missing required evidence. If blockers remain, lead with them and name the next safe action.

## 9. Respect The Active/Later Boundary

Active MVP behavior should stay compact. Later candidate presentation formats may be named for contrast or routing, but they must not look like active requirements.

Do not make full-format judgment presentations, standalone derived views, full evidence displays, detached later-path checks, broad review catalogs, future conformance runners, operations hardening, or later candidates appear required for ordinary active MVP work.

Quality concerns are not standalone current MVP requirements or reasons the work cannot be closed yet. Route them through active judgment kinds, evidence gaps, residual-risk visibility, surface capability, scope, or another already-active reason the work cannot be closed yet only when that owner path truly applies.

Use compact user-facing shapes first: status, focused judgment request, what was checked, and close result. Reference exact contracts only when needed for a visible blocker, source ref, write check, evidence gap, close result, connector behavior, or implementation owner link.

## 10. Load One Language Version Per doc_id

For ordinary Harness session context, do not load both English and Korean paired docs for the same `doc_id` into one prompt. Choose the language needed for the current user or task, and cite the paired doc path only when parity matters.

Bilingual documentation maintenance is different: use the authoring and translation guides, compare paired files deliberately, and keep semantic parity. Do not turn that maintenance workflow into ordinary always-on agent context.

When the task is Korean-facing, preserve exact identifiers such as API names, schema fields, enum values, file paths, error codes, table names, and validator IDs. Write natural Korean in user-facing output instead of English nouns with Korean particles.
