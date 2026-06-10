# Later: Workflow and Collaboration

## What this document owns

This document owns inactive later candidates about richer task shaping, next-action flow, verification and evaluation workflow, user-judgment branches, risk review, release handoff, reconciliation, team orchestration, and collaboration lifecycle. It keeps workflow and collaboration candidates grouped so [Later Candidate Index](index.md) can remain a router and short summary.

Every candidate here is future-facing. The candidate details are documentation source material only and do not create current workflow, API, storage, UI, or collaboration requirements.

## What this document does not own

This document does not define current MVP API behavior, task lifecycle, close-readiness behavior, acceptance behavior, residual-risk acceptance, team permissions, release automation, storage effects, security guarantees, conformance behavior, or implementation readiness.

It also does not make any later judgment branch a substitute for user-owned judgment. Sensitive-action approval, final acceptance, residual-risk acceptance, waiver, reconciliation, and verification-risk acceptance remain distinct unless a promotion batch explicitly changes the relevant current owners.

## Category boundary

This category is for candidates whose main question is "how might Harness coordinate people, agents, judgments, and lifecycle steps later?" It includes shaping records, next actions, richer verification and evaluation flows, `Decision Packet` presentation, risk review, reconcile, release handoff, team workflows, and derived workflow context.

It does not own connector mechanics, artifact body storage, security guarantees, or validator catalogs. If a future workflow depends on those areas, this document records only the workflow-and-collaboration framing before promotion.

## Candidate summary

| Candidate | Summary |
|---|---|
| Discovery brief, question queue, and assumption register | Future shaping records for open questions, assumptions, and task context. |
| Verification-risk acceptance `verification_risk_acceptance` | Future user-judgment route for accepting verification risk. |
| Eval and detached verification workflows | Future evaluation and detached verification workflows. |
| Full `Decision Packet` and `presentation=full` | Future full-format decision presentation. |
| Rich risk review and residual-risk lifecycle | Future richer risk review records, residual-risk lifecycle, and expiry behavior. |
| Release handoff | Future release handoff workflow without production authority. |
| Recovery and reconcile | Future recovery, reconcile, and state-repair workflow. |
| Persistent projection jobs | Future projection job lifecycle and job storage. |
| Projection reconcile and editable projection areas | Future projection reconcile, managed-block repair, and editable projection areas. |
| `harness.next` | Future next-action API method. |
| `harness.launch_verify` | Future verification-launch API method. |
| `harness.record_eval` | Future evaluation-recording API method. |
| `harness.record_manual_qa` | Future Manual QA recording API method. |
| Later `harness.record_run` branches | Future `harness.record_run` branches for verification input, feedback-loop updates, or TDD trace updates. |
| Later user-judgment branches | Future `qa_waiver`, `verification_risk_acceptance`, waiver, reconcile, residual-risk, and richer acceptance branches. |
| Later next-action values | Future next-action values such as `launch_verify`, `record_eval`, `record_manual_qa`, and `reconcile`. |
| Waiver, reconcile, and residual-risk branches | Future waiver, reconcile, and residual-risk branches. |
| Verification result cards and richer verification workflows | Future verification cards and richer verification workflow without substituting for QA. |
| Context index and derived metrics | Future context indexing and derived metrics that support workflow review without becoming authority by themselves. |
| Team workflows and orchestration | Future team permissions, shared capability sets, orchestration, and parallel-lane behavior. |
| Advanced release and deployment automation | Future deployment, canary, rollback, merge, and production-monitoring automation. |

## Candidate details

<a id="discovery-brief-question-queue-and-assumption-register"></a>
### Discovery brief, question queue, and assumption register

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active shaping records or persistence requirements.
- Promotion focus: core owners, schema owners, storage effects, and conformance checks for any promoted shaping record.

<a id="verification-risk-acceptance-verification-risk-acceptance"></a>
### Verification-risk acceptance `verification_risk_acceptance`

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create an active verification-risk acceptance route.
- Promotion focus: user-judgment owners, schema owners, API behavior, and conformance checks for any promoted verification-risk route.

<a id="eval-and-detached-verification-workflows"></a>
### Eval and detached verification workflows

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active evaluator authority, detached verification, or evaluation workflow requirements.
- Promotion focus: Eval owners, schema owners, API behavior, and conformance checks for any promoted detached verification workflow.

<a id="full-decision-packet-and-presentation-full"></a>
### Full `Decision Packet` and `presentation=full`

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create an active full-format presentation path.
- Promotion focus: user-judgment owners, template owners, API behavior, and conformance checks for any promoted full presentation.

<a id="rich-risk-review-and-residual-risk-lifecycle"></a>
### Rich risk review and residual-risk lifecycle

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active rich risk records, review workflow, or expiry behavior.
- Promotion focus: core owners, user-judgment owners, schema owners, and conformance checks for any promoted risk lifecycle.

<a id="release-handoff"></a>
### Release handoff

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active deployment, merge, rollback, or production authority.
- Promotion focus: handoff owners, security wording, API behavior, and conformance checks for any promoted release handoff.

<a id="recovery-and-reconcile"></a>
### Recovery and reconcile

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active recovery, reconcile, or state-repair behavior.
- Promotion focus: operations owners, storage effects, API behavior, and conformance checks for any promoted recovery or reconcile workflow.

<a id="persistent-projection-jobs"></a>
### Persistent projection jobs

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active projection job lifecycle or job storage.
- Promotion focus: projection owners, storage effects, API behavior, and conformance checks for any promoted projection job lifecycle.

<a id="projection-reconcile-and-editable-projection-areas"></a>
### Projection reconcile and editable projection areas

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active editable projection areas, reconcile queues, managed-block repair, or projection state authority.
- Promotion focus: projection owners, core owners, storage effects, and conformance checks for any promoted projection reconcile behavior.

<a id="harness-next"></a>
### `harness.next`

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create an active API method.
- Promotion focus: API behavior, schema owners, storage effects, and conformance checks for any promoted next-action method.

<a id="harness-launch-verify"></a>
### `harness.launch_verify`

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create an active verification-launch API method.
- Promotion focus: API behavior, verification owners, security wording, and conformance checks for any promoted verification-launch method.

<a id="harness-record-eval"></a>
### `harness.record_eval`

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create an active evaluation-recording API method.
- Promotion focus: API behavior, Eval owners, schema owners, and conformance checks for any promoted evaluation-recording method.

<a id="harness-record-manual-qa"></a>
### `harness.record_manual_qa`

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create an active Manual QA recording API method.
- Promotion focus: API behavior, Manual QA owners, schema owners, and conformance checks for any promoted Manual QA recording method.

<a id="later-harness-record-run-branches"></a>
### Later `harness.record_run` branches

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active `harness.record_run` branches for verification input, feedback-loop updates, or TDD trace updates.
- Promotion focus: API behavior, schema owners, storage effects, and conformance checks for any promoted `harness.record_run` branch.

<a id="later-user-judgment-branches"></a>
### Later user-judgment branches

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not activate `qa_waiver`, `verification_risk_acceptance`, waiver, reconcile, residual-risk, or richer acceptance branches.
- Promotion focus: user-judgment owners, schema owners, API behavior, and conformance checks for any promoted judgment branch.

<a id="later-next-action-values"></a>
### Later next-action values

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not activate `launch_verify`, `record_eval`, `record_manual_qa`, or `reconcile`.
- Promotion focus: schema owners, API behavior, user-facing guidance, and conformance checks for any promoted next-action value.

<a id="waiver-reconcile-and-residual-risk-branches"></a>
### Waiver, reconcile, and residual-risk branches

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active waiver, reconcile, or residual-risk branches.
- Promotion focus: user-judgment owners, core owners, schema owners, and conformance checks for any promoted waiver, reconcile, or residual-risk branch.

<a id="verification-result-cards-and-richer-verification-workflows"></a>
### Verification result cards and richer verification workflows

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active verification cards, richer verification workflows, or QA substitution.
- Promotion focus: verification owners, template owners, API behavior, and conformance checks for any promoted verification card or workflow.

<a id="context-index-and-derived-metrics"></a>
### Context index and derived metrics

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active authority, close effect, long-term metric storage, or retrieval requirements.
- Promotion focus: retrieval owners, storage effects, API behavior, and conformance checks for any promoted context index or metric.

<a id="team-workflows-and-orchestration"></a>
### Team workflows and orchestration

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active team permissions, shared capability sets, orchestration, or parallel-lane behavior.
- Promotion focus: scope owners, permission owners, user-judgment owners, and conformance checks for any promoted team workflow.

<a id="advanced-release-and-deployment-automation"></a>
### Advanced release and deployment automation

- Status: Later candidate; currently inactive.
- Current MVP non-effect: Not part of the current MVP. Does not create active release, deployment, canary, rollback, merge, or production-monitoring automation.
- Promotion focus: release owners, security owners, API behavior, and conformance checks for any promoted deployment automation.

## Promotion rule

Promotion is not a local edit to this file. A candidate becomes active only when current active scope and the relevant current owner documents are updated in the same documentation-only batch.

If no current owner exists for the promoted behavior, the promotion batch must create or designate that owner before defining active API, storage, security, UI, or conformance requirements.

## Active-scope non-effect

This document has no effect on the current MVP. Mentioning a candidate here does not activate a workflow branch, API method, state transition, judgment route, permission model, release automation, team behavior, UI, storage effect, or close-readiness requirement.

## Related owners

- [Later Candidate Index](index.md)
- [Active MVP Scope](../reference/active-mvp-scope.md)
- [Reference Index](../reference/README.md)
- [Core Model](../reference/core-model.md)
- [MVP API](../reference/api/mvp-api.md)
- [API Judgment Schemas](../reference/api/schema-judgment.md)
- [Projection Authority Reference](../reference/projection-and-templates.md)
- [Template Bodies](../reference/template-bodies.md)
