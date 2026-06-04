# TDD-TRACE 템플릿

## 사용 시점

Change Unit에서 TDD가 필요하거나 선택 또는 기록된 상태이고 RED, GREEN, refactor/check, waiver, evidence ref를 읽기 쉬운 projection으로 볼 때 `TDD-TRACE`를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 projections입니다. TDD Trace output은 later policy 또는 diagnostic profile용이며 첫 구현 범위를 키우면 안 됩니다.

## 기준 기록

- `tdd_traces`
- selected `feedback_loops`
- Task와 Change Unit 참조
- RED, GREEN, refactor/check artifact 참조
- Evidence Manifest coverage 참조
- waiver 또는 non-TDD justification 참조
- 해당되는 경우 Evidence Manifest, 사용자 판단(User Judgment), Change Unit, Residual Risk, 수동 QA, Eval, close blocker, follow-up ref를 통한 finding route
- `tdd_trace` 관련 design-quality validator 결과
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- 식별 정보
- Red(실패 단계)
- Green(통과 단계)
- Refactor(정리 단계)
- Non-TDD 근거
- 근거 참조
- Finding 라우팅

## 전체 템플릿

````md
---
doc_type: tdd_trace
tdd_trace_id: TDD-0001
task_id: TASK-0001
change_unit_id: CU-01
status: recorded
source_state_version: 43
updated_at: 2026-05-06T09:40:00+09:00
---

# TDD-0001 Trace 제목

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 TDD record와 ref를 표시합니다. Plan text는 기록된 artifact 또는 result ref가 뒷받침하기 전까지 RED evidence가 아닙니다.

## 식별 정보
- task_id:
- change_unit_id:
- trace 상태: required | recorded | waived | not_required
- 요구/출처:
- feedback loop ref:
- evidence manifest coverage ref:

## Red(실패 단계)
- target / plan:
- failing test ref:
- command:
- result: failed_as_expected | failed_unexpectedly | missing
- log ref:
- recorded before non-test implementation: yes | no | waived
- target / plan counts as Evidence Manifest coverage: no

## Green(통과 단계)
- command:
- result: passed | failed | missing
- log ref:

## Refactor(정리 단계)
- performed: yes | no
- notes:
- verification command:
- log ref:

## Non-TDD 근거
- 이유:
- feedback loop ref:
- alternate feedback loop:
- waiver recorded before non-test implementation: yes | no

## 근거 참조
- test:
- red log:
- green log:
- refactor/check log:
- Evidence Manifest:
- diff:

## Finding 라우팅
- evidence gaps or support:
- User judgment 후보 또는 refs:
- Change Unit update 또는 follow-up:
- residual-risk 후보 또는 refs:
- 수동 QA 또는 Eval refs:
- close blockers:
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. RED target 또는 plan은 계획 context이며, 실제 RED evidence는 여전히 기록된 artifact 또는 result ref에서 나와야 합니다.

TDD가 advisory일 뿐 required 또는 selected가 아니라면 TDD waiver는 필요하지 않습니다. Required, selected, recorded, waived TDD는 owner record에서만 렌더링하고, finding은 template-only state를 추가하지 말고 기존 owner ref로 라우팅합니다.
