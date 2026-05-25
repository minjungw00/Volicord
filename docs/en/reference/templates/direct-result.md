# DIRECT-RESULT Template

## Used when

Use `DIRECT-RESULT` for a compact result report after small direct work closes or escalates.

## Source records

- direct run record
- consumed Write Authorization ref, when present for direct product writes
- changed paths
- checks performed
- artifact refs
- escalation flag
- close assurance
- evidence, verification, Manual QA, acceptance, and residual-risk close summaries when applicable

## Rendered sections

- Request
- Scope
- Changed Files
- Checks And Validator Outcomes
- Outcome
- Assurance
- Close Summary
- Escalation
- Evidence Refs

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
- write authorization:
- allowed paths:
- allowed tools:
- allowed commands:
- approval refs:

## Changed Files
- `path/to/file`

## Checks And Validator Outcomes
### Core Checks And Command Checks
- changed_paths:
- approval_scope:
- test:
- build:

### ValidatorResult IDs
- context_hygiene_check:
- surface_capability_check:

## Outcome
- result summary:

## Assurance
- assurance_level:
- meaning:
- detached verify needed:

## Close Summary
- evidence:
- verification:
- Manual QA:
- acceptance:
- residual risk:
- follow-up:

## Escalation
- escalated_to_work: yes | no
- reason:

## Evidence Refs
- logs:
- diff:
- follow-up report:
- omitted or blocked artifact impact:
````

## Notes

Direct work may close self-checked by default unless policy or the user requires detached verification or other gates. A consumed Write Authorization ref may be displayed, but the projection does not become the canonical authorization record.

Checks and tests in a Direct Result are evidence or self-check context. They do not become detached verification without a qualifying Eval, do not become Manual QA without a Manual QA result or valid waiver, and do not imply final acceptance. If direct work closes with accepted risk, the close summary should point to accepted Residual Risk refs, the Decision Packet that recorded the risk acceptance when one was required, and follow-up instead of presenting the result as detached verified.

Direct result artifact refs must keep redaction state visible. `secret_omitted` supports only visible nonsecret evidence, and `blocked` means the raw input is unavailable until resolved by a replacement, waiver, Decision Packet outcome, accepted risk, or documented fallback.
