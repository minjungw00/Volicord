# EVIDENCE-MANIFEST Template

## Used when

Use `EVIDENCE-MANIFEST` when Harness needs a readable map from acceptance criteria and completion conditions to supporting evidence.

## Source records

- evidence manifest record
- acceptance criteria
- changed file coverage
- design-quality coverage
- approval refs
- artifact refs with redaction state and downstream evidence impact
- related Run, Eval, Feedback Loop, Manual QA, and TDD trace refs

## Rendered sections

- Identity
- Summary
- Acceptance Criteria Coverage
- Changed File Coverage
- Design Quality Coverage
- Approval Refs
- Evidence Refs
- Redaction And Availability
- Stale If

## Full template

````md
---
doc_type: evidence_manifest
evidence_manifest_id: EM-0001
task_id: TASK-0001
change_unit_id: CU-01
status: partial
source_state_version: 44
updated_at: 2026-05-06T09:50:00+09:00
---

# EM-0001 Evidence Manifest

## Identity
- task_id:
- change_unit_id:
- baseline_ref:
- run_summary:
- latest_eval:

## Summary
- evidence state:
- unsupported criteria:
- omitted or blocked evidence impact:
- stale conditions:
- next evidence action:

## Acceptance Criteria Coverage
| AC ID | Statement | Coverage State | Supporting Evidence | Notes |
|---|---|---|---|---|
| AC-01 | | supported | test:, tdd:, log:, diff: | |
| AC-02 | | unsupported | | |

## Changed File Coverage
| Path | Covered Criteria | Evidence Refs |
|---|---|---|
| `src/...` | AC-01 | DIFF-0001, LOG-0001 |

## Design Quality Coverage
| Item | Coverage / Gate Display | Evidence Refs | Notes |
|---|---|---|---|
| vertical_slice_shape | passed | CU-01 | |
| decision_quality_check | passed | DEC-0001 | |
| autonomy_boundary_check | passed | CU-01 | |
| feedback_loop_check | passed | FBL-0001, TDD-0001, LOG-0001 | |
| tdd_trace_required | passed | TDD-0001, RED-LOG-0001, GREEN-LOG-0001 | RED, GREEN, and relevant refactor/check coverage link back to acceptance criteria and changed files. |
| module_interface_review | passed | module_map_item: MMI-0001, interface_contract: IFACE-0001, DEC-0001 | |
| codebase_stewardship_check | passed | domain_term: TERM-0001, module_map_item: MMI-0001, interface_contract: IFACE-0001, feedback_loop: FBL-0001 | |
| residual_risk_visibility_check | pending | RR-0001 | |
| manual_qa_required | pending | qa_gate; no satisfying Manual QA record yet | |

`Coverage / Gate Display` is the evidence coverage or close-relevant gate display state for this manifest. Values such as `pending` in this column are not `ValidatorResult.status` values.

## Approval Refs
- APR-0001:

## Evidence Refs
- run summary:
- feedback loop:
- TDD trace:
- TDD RED target / plan:
- TDD red:
- TDD green:
- TDD refactor/check:
- Manual QA:
- diff:
- logs:
- bundle:
- checkpoint:
- tests:
- build:

## Redaction And Availability
| Artifact Ref | Redaction State | Evidence Effect | Note |
|---|---|---|---|
| ART-0001 | secret_omitted | supports visible nonsecret facts only | |
| ART-0002 | blocked | unavailable input; claim remains insufficient until resolved | |

## Stale If
- baseline head changes
- changed files are modified after eval
- approval scope expires
- relevant config changes
- domain term records change
- interface contract records change
````

## Notes

Where evidence is required, close depends on the canonical `evidence_gate`, not the report text alone.

`secret_omitted` artifacts may support claims whose nonsecret evidence remains visible, but not claims that require omitted values. `blocked` artifacts are committed metadata-only notices, not available raw evidence; dependent criteria remain unsupported, insufficient, or blocked until a replacement, waiver, Decision Packet outcome, accepted risk, or documented fallback resolves the evidence path. This template must not include omitted secret/PII values or blocked payload bytes.
