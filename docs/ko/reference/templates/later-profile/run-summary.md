# RUN-SUMMARY 템플릿

## 사용 시점

`record_run`으로 execution Run이 기록된 뒤, 무엇을 실행했고 무엇이 바뀌었는지, 확인 또는 validator가 무엇을 보고했는지, 원본 근거(evidence)가 어떤 artifact에 남았는지 요약해야 할 때 `RUN-SUMMARY`를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 projections입니다. Later profile을 위한 상세 Run view로 유지하며 초기 필수 범위가 아닙니다.

## 기준 기록

- run 기록
- actor/surface identity
- baseline
- Change Unit
- 있는 경우 consumed Write Authorization 참조
- changed path
- command result
- validator 결과
- 기록된 경우 기존 owner ref로 연결된 Review Stage 표시 finding
- artifact 참조
- evidence update와 follow-up

## 렌더링 섹션

- Run 식별 정보
- 범위
- 변경된 파일
- 명령과 확인
- 확인과 Validator 결과
- 검토 단계
- TDD trace 요약
- 주요 변경
- 이슈와 후속 조치
- Journey Spine 업데이트
- 근거 참조

## 전체 템플릿

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

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 committed Run과 artifact ref를 표시합니다. 이 문서를 편집해도 Run, evidence, gate, `state.sqlite.task_events`는 바뀌지 않습니다.

## Run 식별 정보
- run_id:
- actor kind:
- surface:
- baseline_ref:
- state_version:
- status:

## 범위
- task_id:
- change_unit_id:
- slice type:
- write authorization:
- 허용 path:
- 허용 tool:
- 허용 command:
- 허용 network target:
- secret scope:
- 민감 category:
- approval refs(later Approval profile only; 그 외에는 none):

## 변경된 파일
- `path/to/file`

## 명령과 확인
```bash
npm test -- --runInBand
```

## 확인과 Validator 결과
### Core 확인과 명령 확인
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

## 검토 단계
- note: run-local review display only. Record, `ProjectionKind` value, Approval, evidence, verification, QA, 작업 수락, 잔여 위험 수용, close, Write Authorization을 만들지 않습니다. Review-stage 경계는 [Design Quality Policies](../../design-quality-policies.md#two-stage-review-display)가 담당합니다. 발견 사항은 기존 ref, gate, blocker로 연결합니다.

### 명세 준수 검토
- acceptance criteria coverage:
- Change Unit completion conditions:
- 범위 / Write Authority compatibility:
- User judgment compatibility:
- evidence coverage:
- 잔여 위험 표시:
- outcome refs(existing path/ref only):

### 코드 품질 / Stewardship 검토
- domain language:
- module / interface boundary:
- vertical slice shape:
- feedback loop / TDD:
- codebase stewardship:
- context hygiene:
- follow-up risk:
- outcome refs(existing path/ref only):

## TDD trace 요약
- required:
- feedback loop ref:
- RED target / plan:
- RED evidence (actual):
- green evidence:
- refactor notes:
- waiver / alternate loop:
- trace ref:

## 주요 변경
-

## 이슈와 후속 조치
-

## Journey Spine 업데이트
- 새 facts:
- 거절된 options:
- domain language 업데이트:
- module/interface 업데이트:
- watchpoint 변경:
- 다음 run이 알아야 할 것:

## 근거 참조
- evidence manifest:
- TDD trace:
- 수동 QA:
- diff:
- logs:
- bundle:
- checkpoint:
- omitted or blocked artifact impact:
````

## 메모

Raw log와 diff는 artifact로 남기고, 보고서에는 link만 둡니다. `RUN-SUMMARY`에 담긴 같은 세션 검토(review) 내용은 자체 확인(self-check) 또는 stewardship signal로만 취급하며 [review-stage 경계](../../design-quality-policies.md#two-stage-review-display)를 따릅니다. 발견 사항은 기존 gate, user judgment, evidence, Eval, 수동 QA, Residual Risk, Approval, Change Unit 업데이트, close-blocker ref로 연결하며, report 자체가 그 record나 authority를 만들지는 않습니다.

이 report의 evidence ref는 `redaction_state`를 보존해야 합니다. `secret_omitted` ref는 보이는 nonsecret evidence만 뒷받침할 수 있고, `blocked` ref는 원본 log, diff, screenshot, bundle이 아니라 사용할 수 없는 입력을 표시하는 committed metadata-only notice입니다.
