# DIRECT-RESULT Template

## Used when

Use `DIRECT-RESULT` for a compact, low-ceremony result report after small direct work closes or escalates. It should read like a direct outcome, not a full task-level gate report.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Future/diagnostic projections. Use as an optional compact direct-work result display when that profile is active; it is not required for MVP-1 User Work Loop projection or the first kernel proof.

## Source records

- direct run record
- consumed Write Authorization ref when a compatible direct product-write Run consumed it; attempted invalid authorization refs only as violation/audit context
- changed paths
- out-of-bounds or unchanged scope summary
- checks performed
- User Judgment refs, sensitive-action approval user judgment refs, later Approval refs, `evidence_ref` refs and derived evidence summaries, Evidence Manifest when the full evidence profile is active, Eval, Manual QA, work-acceptance user judgment refs, Residual Risk, and Artifact refs when those claims are displayed
- artifact refs with redaction state and availability
- projection freshness inputs
- escalation flag
- close assurance
- evidence, verification, Manual QA, work acceptance, residual-risk visibility, and residual-risk acceptance close summaries when applicable

Close Summary lines are derived display summaries from existing gate and owner-record refs. Direct work does not create additional close fields beyond the records it summarizes.

## Rendered sections

- Request
- Scope
- Outcome
- Changed Scope
- Checks
- Assurance
- Authority Refs
- Close Summary
- Escalation
- Evidence Refs
- Projection Freshness

## Full template

````md
---
doc_type: direct_result
task_id: TASK-0001
run_id: RUN-20260506-093015-LEAD-01
result: passed
assurance_level: self_checked
surface_id: reference
source_state_version: 41
updated_at: 2026-05-06T09:40:00+09:00
---

# DIRECT-RESULT

> Projection view: rendered from `source_state_version` at `updated_at`; displays the direct Run result. Editing it does not change result, assurance, escalation, or close state.

## Request
- user request:

## Scope
- direct run scope:
- limits:
- write authorization ref:
- allowed paths:
- sensitive-action approval user judgment refs (minimum MVP-1, when applicable):
- approval refs (later Approval profile only; otherwise none):

## Outcome
- result summary:
- close reason:

## Changed Scope
- changed files: `path/to/file`
- no-file result:
- out of bounds kept:

## Checks
- self-check:
- tests/build:
- validator outcomes:
- artifact refs and redaction state:
- artifact availability:

## Assurance
- assurance_level:
- meaning:
- detached verify needed:
- self-check refs:
- detached verification Eval ref:
- verification waiver ref:
- QA waiver ref:
- risk-accepted close refs:

## Authority Refs
- write authorization:
- User Judgment:
- Sensitive-action approval user judgment / Approval:
- evidence refs / derived summary:
- Evidence Manifest (full evidence profile only):
- Eval:
- Manual QA:
- Work acceptance user judgment:
- Residual Risk:
- Artifact refs:
- redaction state:
- projection freshness:

## Close Summary
- display state label (plain text, not a schema value):
- evidence:
- verification:
- Manual QA:
- work acceptance:
- residual-risk visibility:
- residual-risk acceptance:
- verification waiver ref:
- QA waiver ref:
- follow-up:

## Escalation
- escalated_to_work: yes | no
- reason:

## Evidence Refs
- logs:
- diff:
- follow-up report:
- omitted or blocked artifact impact:

## Projection Freshness
- freshness:
- source_state_version:
- stale or reconcile impact:
````

## Notes

Direct work may close self-checked by default unless policy or the user requires detached verification or other gates. A consumed Write Authorization ref may be displayed, but the projection does not become the canonical authorization record. An attempted invalid authorization ref must be displayed only as violation/audit or validator-finding context, not as consumed authority or completion evidence.

Direct Result should display self-checked, `detached_verified`, verification-waived, QA-waived, and risk-accepted-close states as separate lines. A waiver line points to the waiver ref or says it is not recorded; it does not become verification or QA. A risk-accepted close points to the residual-risk acceptance user judgment plus related blocker/evidence refs in MVP-1, and accepted Residual Risk refs only when that later profile is active, instead of being rendered as detached verified.

Checks and tests in a Direct Result are evidence or self-check context. They do not become detached verification without a qualifying Eval, do not become Manual QA without a Manual QA result or valid waiver, and do not imply work acceptance. If direct work closes with accepted risk, the Close Summary should point to the residual-risk acceptance user judgment, related blocker/evidence refs, later accepted Residual Risk refs when active, and follow-up instead of presenting the result as detached verified. If no close-relevant risk is known, say that directly rather than adding gate inventory.

Authority claims in a Direct Result should cite source refs or explicit absence: Write Authorization for write permission; a resolved `user_judgment` with `judgment_type=sensitive_action_approval` for minimum MVP-1 sensitive-action permission; an Approval ref only when the later Approval profile is active; `evidence_ref` when present, Run refs, ArtifactRefs, and visible gap summaries for MVP-1 evidence display; Evidence Manifest only when the full evidence profile is active and the result claims full criteria-to-evidence sufficiency; Eval for detached verification when that profile is active; Manual QA record or waiver path for QA when that profile is active; the work-acceptance user judgment path for work acceptance; blocker/user-judgment refs or `ResidualRiskSummary.status=none` for MVP-1 residual-risk visibility; residual-risk acceptance user judgment plus related blocker/evidence refs for MVP-1 residual-risk acceptance; and rich Residual Risk refs only when that later profile is active. Do not render `not_visible` residual risk as "none."

`DIRECT-RESULT` is the low-ceremony close impact display for direct work. `TASK` owns continuity Close Summary display for active or recently closed `work` tasks, and Journey Card close context is compact status/resume display. These displays follow the [projection/report boundary](../../projection-and-templates.md#projection-principles); close and gate effects still come from owner records.

Direct result artifact refs must keep redaction state visible. `secret_omitted` supports only visible nonsecret evidence, and `blocked` means the raw input is unavailable until resolved by a replacement, waiver, user judgment outcome, accepted risk, or documented fallback.
