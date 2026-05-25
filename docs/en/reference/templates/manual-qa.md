# MANUAL-QA Template

## Used when

Use `MANUAL-QA` when Manual QA is required, performed, waived, pending, or represented in `qa_gate` and the record needs a readable projection.

## Source records

- `manual_qa_records`
- Task and Change Unit refs
- `qa_gate`
- Manual QA profile, setup, checklist, result, waiver, and findings
- screenshot, browser log, video, or note artifact refs with redaction state
- waiver reason, QA waiver Decision Packet refs when required, and Residual Risk refs related to QA waiver or failure
- design-quality validator results related to `manual_qa`
- projection freshness inputs

## Rendered sections

- Identity
- Setup
- Checklist
- Result
- Waiver And Risk
- Findings
- Evidence Refs
- Redaction And Availability

## Full template

````md
---
doc_type: manual_qa
manual_qa_record_id: null
task_id: TASK-0001
change_unit_id: CU-01
qa_gate: pending
result: null
source_state_version: 45
updated_at: 2026-05-06T10:05:00+09:00
---

# Manual QA

> Projection view: rendered from `source_state_version` at `updated_at`; displays Manual QA records and `qa_gate`. QA results and QA waivers are recorded in `manual_qa_records` and `qa_gate`; QA waivers that involve product/user risk use a linked QA waiver Decision Packet, and residual-risk acceptance is recorded on Residual Risk refs.

## Identity
- manual_qa_record_id: QA-0001 | null
- task_id:
- change_unit_id: CU-01 | null
- qa_profile: ui_quality | workflow | copy | accessibility | browser_smoke | performance_smoke | other
- required: yes | no
- performed by:

## Setup
- build/run command:
- test account/data:
- route or screen:

## Checklist
- [ ] primary workflow works
- [ ] errors are understandable
- [ ] visual layout acceptable
- [ ] accessibility smoke check
- [ ] no obvious regression

## Result
- record result: passed | failed | waived | null when no record exists
- qa_gate: pending | passed | failed | waived | not_required
- qa_gate note: canonical close-relevant gate; this projection is display only
- summary:
- waiver reason:

## Waiver And Risk
- waiver recording:
- skipped check or surface:
- accepted risk:
- follow-up:
- residual risk refs:
- accepted residual-risk summary:
- close impact:

## Findings
| Severity | Finding | Suggested Action | Follow-up CU |
|---|---|---|---|
| minor | | | |

## Evidence Refs
- screenshot:
- browser log:
- video:
- note:

## Redaction And Availability
| Artifact Ref | Redaction State | QA Effect | Note |
|---|---|---|---|
| ART-QA-0001 | secret_omitted | supports observable finding only | |
| ART-QA-0002 | blocked | capture unavailable; QA path unresolved unless replaced or validly waived; `qa_gate` stays pending/failed or `waived` as applicable | |
````

## Notes

This template is a rendered shape, not canonical state. `qa_gate` is the canonical close-relevant gate; this projection only displays it.

Manual QA projections may show safe omission notes, handles, and blocked artifact notices, but must not embed omitted secret/PII values or blocked capture payloads. A `secret_omitted` artifact can support visible workflow, UI, copy, accessibility, or smoke-test observations; a `blocked` capture is unavailable QA input unless a replacement, waiver, Decision Packet outcome, accepted risk, or documented fallback resolves the QA path.
