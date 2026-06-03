# DIRECT-RESULT Template

## Used when

Use `DIRECT-RESULT` for a compact, low-ceremony result report after small direct work closes or escalates. It should read like a direct outcome, not a full task-level gate report.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Future/diagnostic projections. Use as an optional compact direct-work result display when that profile is active; it is not required for v0.2 MVP projection or the first kernel proof.

## Source records

- direct run record
- consumed Write Authorization ref, when present for direct product writes
- changed paths
- out-of-bounds or unchanged scope summary
- checks performed
- Decision Packet, Approval, Evidence Manifest, Eval, Manual QA, Acceptance Decision Packet, Residual Risk, and Artifact refs when those claims are displayed
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
- approval refs:

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
- Decision Packet:
- approval:
- Evidence Manifest:
- Eval:
- Manual QA:
- Acceptance Decision Packet:
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

Direct work may close self-checked by default unless policy or the user requires detached verification or other gates. A consumed Write Authorization ref may be displayed, but the projection does not become the canonical authorization record.

Direct Result should display self-checked, `detached_verified`, verification-waived, QA-waived, and risk-accepted-close states as separate lines. A waiver line points to the waiver ref or says it is not recorded; it does not become verification or QA. A risk-accepted close points to accepted Residual Risk refs and any required Decision Packet instead of being rendered as detached verified.

Checks and tests in a Direct Result are evidence or self-check context. They do not become detached verification without a qualifying Eval, do not become Manual QA without a Manual QA result or valid waiver, and do not imply work acceptance. If direct work closes with accepted risk, the Close Summary should point to accepted Residual Risk refs, the Decision Packet that recorded the risk acceptance when one was required, and follow-up instead of presenting the result as detached verified. If no close-relevant risk is known, say that directly rather than adding gate inventory.

Authority claims in a Direct Result should cite source refs or explicit absence: Write Authorization for write permission, Approval for sensitive-action permission, Evidence Manifest for evidence sufficiency, Eval for detached verification, Manual QA record or waiver path for QA, Acceptance Decision Packet for work acceptance, Residual Risk refs or `ResidualRiskSummary.status=none` for residual-risk visibility, and accepted Residual Risk refs for residual-risk acceptance. Do not render `not_visible` residual risk as "none."

`DIRECT-RESULT` is the low-ceremony close impact display for direct work. `TASK` owns continuity Close Summary display for active or recently closed `work` tasks, and Journey Card close context is compact status/resume display. These displays follow the [projection/report boundary](../document-projection.md#projection-principles); close and gate effects still come from owner records.

Direct result artifact refs must keep redaction state visible. `secret_omitted` supports only visible nonsecret evidence, and `blocked` means the raw input is unavailable until resolved by a replacement, waiver, Decision Packet outcome, accepted risk, or documented fallback.
