# Agent Guide

Use this guide when writing or reviewing agent behavior for a future Harness-connected session. The agent's job is to turn ordinary user requests into careful work: infer the work shape, keep context small, preserve user-owned judgment, check scope before writes, record evidence after meaningful action, and close honestly.

This is Use documentation. It is not a connector contract, schema reference, template catalog, conformance fixture, or proof that this documentation-only repository already contains a Harness Server/runtime implementation. Exact connector behavior lives in [Agent Integration Reference](../reference/agent-integration.md). Exact state, write, run/evidence, close, API, and schema contracts live in the relevant Reference owners linked from the [Reference Index](../reference/README.md).

## 1. Infer Harness Use From Task Shape

Do not require a startup phrase. Users do not need to say "Harness," `Discovery`, `Change Unit`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`.

Infer Harness use from the request and current state. Use the Harness path when the work involves scope risk, product writes, user-owned judgment, sensitive action approval, evidence, verification, QA, final acceptance, residual risk, or close readiness.

Classify the work before choosing procedure weight:

| Work shape | Use when | Behavior |
|---|---|---|
| Read/advice | The user wants explanation, review, search, planning, or inspection without a product write. | Inspect available sources, cite uncertainty, and avoid write/close ceremony. |
| Small change | The edit is narrow, low risk, and does not hide a user-owned decision or sensitive category. | Confirm the narrow scope, edit, run a focused check, and report briefly. |
| Tracked work | The request is ambiguous, multi-file, structural, sensitive, public-interface-facing, policy-relevant, or close-relevant. | Clarify scope, preserve judgment, check writes, record evidence, and report close readiness. |

Escalate from small change to tracked work when you find scope drift, a new public interface, security/privacy impact, destructive risk, dependency or migration choice, QA/verification expectation, final acceptance need, residual risk, or another user-owned judgment.

## 2. Keep Context Small

<a id="8-report-status-for-the-users-next-decision"></a>

Always-on context should fit on one screen and support the next action. Include only:

- current task summary
- work shape
- active scope and non-goals
- relevant allowed paths, tools, commands, or operation class
- pending user judgments
- active blockers
- latest pre-write scope result, if any
- evidence summary and gaps
- close blockers
- residual-risk status
- guarantee level or unavailable/capability condition
- source refs and freshness
- one next safe action

Do not inject full schemas, full DDL, full template bodies, full logs, full artifact contents, paired bilingual docs, unrelated reference sections, future catalog material, or generated projections into every prompt. Pull exact owner sections only when the next action needs them.

Status output should lead with the primary blocker and the smallest unblocker. Name whether the blocker is user-owned, agent-resolvable, or surface/system-owned. Do not ask the user to solve something the agent can safely inspect, refresh, retry, narrow, or record.

## 3. Ask Focused Questions

<a id="4-clarify-without-endless-planning-loops"></a>

Inspect first. Check repository files, docs, tests, current Harness state, accepted judgments, and relevant artifacts before asking the user. If a source is stale or unavailable, say that instead of treating it as authority.

Ask only the question that changes the next safe action or a user-owned judgment. Do not turn agent-resolvable uncertainty into a questionnaire. Do not start broad implementation when the requirement is too ambiguous to be safe.

A focused clarification should show:

- what you verified
- current goal
- proposed scope and non-goals
- success criteria for the next slice
- confirmed facts
- remaining uncertainty
- the one blocking question, if any
- useful non-blocking questions parked for later
- next safe action

In the active MVP, clarification should update the active task summary, proposed or active `Change Unit` when product writes are near, and user-judgment candidates or records. Do not create separate active requirements for a committed `Discovery Brief`, `Question Queue`, `Assumption Register`, `First Safe Change Unit Candidate`, `Shared Design` record, full-format judgment presentation such as `Decision Packet`, or full design artifact.

## 4. Do Not Decide User-Owned Judgments

<a id="5-request-user-judgment-narrowly"></a>

The agent may recommend. The user decides product behavior, material technical direction, scope changes, sensitive-action approval, final acceptance, residual-risk acceptance, and cancellation. Later/reserved QA waiver and verification-risk acceptance routes remain separate if a future owner promotes them.

When using the active owner path, keep these `judgment_kind` values separate: `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `final_acceptance`, `residual_risk_acceptance`, and `cancellation`.

A judgment request should include the exact question, concise options, recommendation, rationale, uncertainty, consequence of deferral, affected scope, and what the answer does not settle. Ask one judgment at a time unless the user explicitly asks to review grouped options and the group still preserves separate answers.

Do not treat "yes," "approved," "looks good," "go ahead," or "continue" as a bundle of every pending judgment. Map a short reply only when one active prompt made the kind, affected object, option, scope, user intent, consequences, and remaining open items unambiguous.

Sensitive approval is permission for a named action. Final acceptance is judgment on the result. Residual-risk acceptance is judgment on a named residual risk. A future QA waiver or verification-risk acceptance route would be separate from all three. None substitutes for another.

## 5. Do Not Claim Stronger Guarantees

Harness authority is authority over Harness records and state transitions. It is not OS permission control, arbitrary-tool sandboxing, tamper-proof storage, universal pre-tool blocking, or security isolation unless the exact mechanism and covered operation are documented and proven.

Use guarantee wording carefully:

- cooperative: the agent is instructed to hold, ask, refresh, or proceed through the record path
- detective: Harness or a surface can report mismatch after observation
- preventive: a specific proven mechanism blocks a covered action before it happens
- isolated: a documented separation boundary exists

If Core or Harness authority is unavailable, do not invent task state, write compatibility, user judgment, sensitive-action approval, evidence, final acceptance, residual-risk acceptance, projection freshness, or close readiness. Reconnect, diagnose, move to a capable surface, narrow the task, or continue outside Harness only if the user explicitly chooses that mode.

## 6. Prepare Write Only When Scope Is Clear

<a id="6-check-scope-before-product-writes"></a>

Before product/code/file writes in Harness-connected work, use a pre-write scope check only after the intended operation is specific enough to evaluate. In owner terms this is the `prepare_write` / Write Authorization path.

Do not claim write compatibility from a plan, stale chat context, broad user enthusiasm, stale status, generated summary, or rendered view. Show the user:

- intended paths or operation
- scope match or mismatch
- pending user judgments or sensitive approvals
- stale state, stale baseline, or unavailable authority
- current guarantee level or unavailable/capability condition
- smallest unblocker

A compatible result means the intended write matches current Harness state and active surface capability. It is a single-use cooperative record for the stated boundary. If paths, commands, tools, network targets, secret scope, sensitive category, baseline, task, Change Unit, state, surface, related judgments, or guarantee level change, refresh the check or treat the claim as unverified/blocked.

## 7. Record Run And Evidence After Meaningful Action

<a id="7-record-evidence-after-meaningful-action"></a>

After meaningful execution, checks, reviews, or artifact-producing work, summarize what happened and what supports each claim. In owner terms this may use `record_run` and evidence refs when that path is active.

Use refs and short summaries by default. Pull full artifact bodies only when the next action needs them and redaction rules allow it. Do not treat arbitrary absolute paths, raw secrets, tokens, full sensitive logs, screenshots alone, generated summaries, or chat text as sufficient evidence.

Evidence display should say what ran or changed, which claim it supports, which refs or artifacts support it, what passed or failed, and what is missing, stale, redacted, omitted, blocked, or insufficient.

Evidence does not automatically satisfy final acceptance, residual-risk acceptance, close, or any future promoted verification or Manual QA path.

## 8. Do Not Close When Blockers Remain

<a id="10-close-work-honestly"></a>

Close only when the active path can support the close claim. In owner terms, `close_task` should return blockers or a close result.

For small work, a close-like result can be brief: request, scope, changed files or no-file outcome, checks, and known residual risk.

For tracked work, show the close basis before asking for final acceptance or attempting close:

- scope match
- evidence coverage or gap
- checks run and known verification limits
- sensitive-action approval status when relevant
- final acceptance status when required
- residual-risk visibility and acceptance status when relevant
- close blockers and smallest unblocker

The current MVP has no active `verification_gate`, `qa_gate`, Manual QA gate, `qa_waiver`, or `verification_risk_acceptance` close requirement. If a future owner promotes one, route it as later material with its own active contract.

Do not close from prose, tests alone, broad acceptance-like language, a generated projection, or a stale status summary. If blockers remain, lead with them and name the next safe action.

## 9. Respect The Active/Later Boundary

Active MVP behavior should stay compact. Later candidate presentation formats may be named for contrast or routing, but they must not look like active requirements.

Do not make full-format judgment presentation such as `Decision Packet`, standalone `DEC` projections, full Evidence Manifest display, detached verification, broad Manual QA catalogs, future conformance runners, operations hardening, or later candidates appear required for ordinary active MVP work.

Use compact user-facing shapes first: status, focused judgment request, run/evidence summary, and close result. Reference exact contracts only when needed for a visible blocker, source ref, write check, evidence gap, close result, connector behavior, or implementation owner link.

## 10. Load One Language Version Per doc_id

For ordinary Harness session context, do not load both English and Korean paired docs for the same `doc_id` into one prompt. Choose the language needed for the current user or task, and cite the paired doc path only when parity matters.

Bilingual documentation maintenance is different: use the authoring and translation guides, compare paired files deliberately, and keep semantic parity. Do not turn that maintenance workflow into ordinary always-on agent context.

When the task is Korean-facing, preserve exact identifiers such as API names, schema fields, enum values, file paths, error codes, table names, and validator IDs. Write natural Korean in user-facing output instead of English nouns with Korean particles.
