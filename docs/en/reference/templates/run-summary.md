# RUN-SUMMARY Template

## Used when

Use `RUN-SUMMARY` when `record_run` commits an execution run and Harness needs a readable summary of what ran, what changed, what checks or validators reported, and which artifacts hold the raw evidence.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Future/diagnostic projections. Keep this as a detailed Run view for later profiles; it is not mandatory early scope.

## Source records

- run record
- actor and surface identity
- baseline
- Change Unit
- consumed Write Authorization ref, when present
- changed paths
- command results
- validator results
- Review Stage display findings routed through existing owner refs, when recorded
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

> Projection view: rendered from `source_state_version` at `updated_at`; displays a committed Run and artifact refs. Editing it does not change the Run, evidence, gates, or `state.sqlite.task_events`.

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
- note: run-local review display only. It does not create records, `ProjectionKind` values, Approval, evidence, verification, QA, work acceptance, residual-risk acceptance, close, or Write Authorization. The review-stage boundary is owned by [Design Quality Policies](../design-quality-policies.md#two-stage-review-display); route findings to existing refs, gates, or blockers.

### Spec Compliance Review
- acceptance criteria coverage:
- Change Unit completion conditions:
- scope / Write Authority compatibility:
- Decision Packet compatibility:
- evidence coverage:
- residual-risk visibility:
- outcome refs (existing path/ref only):

### Code Quality / Stewardship Review
- domain language:
- module / interface boundary:
- vertical slice shape:
- feedback loop / TDD:
- codebase stewardship:
- context hygiene:
- follow-up risk:
- outcome refs (existing path/ref only):

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

Raw logs and diffs stay as artifacts; the report links to them. Same-session review content in a `RUN-SUMMARY` is self-check or stewardship signal only and follows the [review-stage boundary](../design-quality-policies.md#two-stage-review-display). Findings route to existing gate, Decision Packet, evidence, Eval, Manual QA, Residual Risk, Approval, Change Unit update, or close-blocker refs; the report does not create those records or authorities by itself.

Evidence refs in this report must preserve `redaction_state`. `secret_omitted` refs may support only visible nonsecret evidence, and `blocked` refs are committed metadata-only notices for unavailable input, not raw logs, diffs, screenshots, or bundles.
