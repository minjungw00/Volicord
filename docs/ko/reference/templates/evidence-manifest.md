# EVIDENCE-MANIFEST Template

## 사용 시점

acceptance criteria와 completion condition이 어떤 supporting evidence로 뒷받침되는지 보여줘야 할 때 `EVIDENCE-MANIFEST`를 사용합니다.

## 기준 기록

- evidence manifest 기록
- acceptance criteria
- changed file coverage
- design-quality coverage
- approval 참조
- artifact 참조와 redaction state, downstream evidence impact
- 관련 Run, Eval, Feedback Loop, Manual QA, TDD trace 참조

## 렌더링 섹션

- Identity
- Summary
- Acceptance Criteria Coverage
- Changed File Coverage
- Design Quality Coverage
- Approval Refs
- Evidence Refs
- Redaction And Availability
- Stale If

## 전체 템플릿

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
| AC ID | Statement | Coverage 상태 | Supporting Evidence | Notes |
|---|---|---|---|---|
| AC-01 | | supported | test:, tdd:, log:, diff: | |
| AC-02 | | unsupported | | |

## Changed File Coverage
| Path | Covered Criteria | Evidence Refs |
|---|---|---|
| `src/...` | AC-01 | DIFF-0001, LOG-0001 |

## Design Quality Coverage
| Item | Coverage / gate 표시 상태 | Evidence Refs | Notes |
|---|---|---|---|
| vertical_slice_shape | passed | CU-01 | |
| decision_quality_check | passed | DEC-0001 | |
| autonomy_boundary_check | passed | CU-01 | |
| feedback_loop_check | passed | FBL-0001, TDD-0001, LOG-0001 | |
| tdd_trace_required | passed | TDD-0001, RED-LOG-0001, GREEN-LOG-0001 | RED, GREEN, relevant refactor/check coverage가 acceptance criteria 및 changed files로 link된다. |
| module_interface_review | passed | module_map_item: MMI-0001, interface_contract: IFACE-0001, DEC-0001 | |
| codebase_stewardship_check | passed | domain_term: TERM-0001, module_map_item: MMI-0001, interface_contract: IFACE-0001, feedback_loop: FBL-0001 | |
| residual_risk_visibility_check | pending | RR-0001 | |
| manual_qa_required | pending | qa_gate; no satisfying Manual QA record yet | |

`Coverage / gate 표시 상태`는 이 manifest의 evidence coverage 또는 close와 관련된 gate 표시 상태입니다. 이 column의 `pending` 같은 값은 `ValidatorResult.status` 값이 아닙니다.

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
| ART-0001 | secret_omitted | visible nonsecret fact만 지원 | |
| ART-0002 | blocked | unavailable input; claim은 해소 전까지 insufficient | |

## Stale If
- baseline head changes
- changed files are modified after eval
- approval scope expires
- relevant config changes
- domain term records change
- interface contract records change
````

## 메모

Evidence가 필요한 경우 close 판단은 보고서 문장만이 아니라 기준 `evidence_gate`를 따릅니다.

`secret_omitted` artifact는 secret이 아닌 evidence가 visible한 주장만 뒷받침할 수 있으며, omitted value가 필요한 주장은 뒷받침하지 못합니다. `blocked` artifact는 committed metadata-only notice이지 사용 가능한 원본 근거가 아닙니다. Dependent criteria는 replacement, waiver, Decision Packet outcome, accepted risk, documented fallback이 evidence path를 해소할 때까지 unsupported, insufficient, blocked 중 적절한 상태로 남습니다. 이 template은 omitted secret/PII value 또는 blocked payload bytes를 포함하면 안 됩니다.
