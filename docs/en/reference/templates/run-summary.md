# RUN-SUMMARY Template

## Used when

Use `RUN-SUMMARY` when `record_run` commits an execution run and Harness needs a readable summary of what ran, what changed, what checks or validators reported, and which artifacts hold the raw evidence.

## Source records

- run record
- actor and surface identity
- baseline
- Change Unit
- consumed Write Authorization ref, when present
- changed paths
- command results
- validator results
- Review Stage findings, when recorded
- artifact refs
- evidence updates and follow-ups

## Rendered sections

- Run Identity
- Scope
- Changed Files
- Commands And Checks
- Checks And Validator Outcomes
- Review Stages
- TDD Trace Summary
- Key Changes
- Issues And Follow-Ups
- Journey Spine Updates
- Evidence Refs

## Full template

````md
---
doc_type: run_summary
run_id: RUN-20260506-093015-LEAD-01
task_id: TASK-0001
change_unit_id: CU-01
profile: lead
kind: implementation
surface_id: reference
source_state_version: 43
updated_at: 2026-05-06T09:45:10+09:00
---

# RUN-SUMMARY

## Run Identity
- run_id:
- actor kind:
- surface:
- baseline_ref:
- state_version:
- status:

## Scope
- task_id:
- change_unit_id:
- slice type:
- write authorization:
- allowed paths:
- allowed tools:
- allowed commands:
- allowed network targets:
- secret scope:
- sensitive categories:
- approval refs:

## Changed Files
- `path/to/file`

## Commands And Checks
```bash
npm test -- --runInBand
```

## Checks And Validator Outcomes
### Core Checks And Command Checks
- changed_paths:
- approval_scope:
- lint:
- test:
- build:
- evidence_sufficiency:

### ValidatorResult IDs
- vertical_slice_shape:
- shared_design_alignment:
- decision_quality_check:
- autonomy_boundary_check:
- feedback_loop_check:
- tdd_trace_required:
- domain_language_consistency:
- module_interface_review:
- codebase_stewardship_check:
- residual_risk_visibility_check:
- manual_qa_required:

## Review Stages
- note: run-local review display only; same-session review cannot create `detached_verified` assurance.

### Spec Compliance Review
- acceptance criteria coverage:
- Change Unit completion conditions:
- scope / Write Authority compatibility:
- Decision Packet compatibility:
- evidence coverage:
- residual-risk visibility:
- outcome refs:

### Code Quality / Stewardship Review
- domain language:
- module / interface boundary:
- vertical slice shape:
- feedback loop / TDD:
- codebase stewardship:
- context hygiene:
- follow-up risk:
- outcome refs:

## TDD Trace Summary
- required:
- feedback loop ref:
- RED target / plan:
- RED evidence (actual):
- green evidence:
- refactor notes:
- waiver / alternate loop:
- trace ref:

## Key Changes
-

## Issues And Follow-Ups
-

## Journey Spine Updates
- new facts:
- rejected options:
- domain language update:
- module/interface update:
- watchpoint changes:
- next run should know:

## Evidence Refs
- evidence manifest:
- TDD trace:
- Manual QA:
- diff:
- logs:
- bundle:
- checkpoint:
- omitted or blocked artifact impact:
````

## Notes

Raw logs and diffs stay as artifacts; the report links to them. Same-session review content in a `RUN-SUMMARY` is self-check or stewardship signal only and cannot be rendered as detached verification.

Evidence refs in this report must preserve `redaction_state`. `secret_omitted` refs may support only visible nonsecret evidence, and `blocked` refs are committed metadata-only notices for unavailable input, not raw logs, diffs, screenshots, or bundles.
