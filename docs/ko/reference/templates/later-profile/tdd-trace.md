# TDD-TRACE 템플릿

## 사용 시점

Change Unit에서 TDD가 필요하거나 선택 또는 기록된 상태이고 RED, GREEN, refactor/check, waiver, 근거 참조를 읽기 쉬운 상태 보기(projection)로 볼 때 `TDD-TRACE`를 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. TDD Trace output은 later policy 또는 diagnostic profile용이며 첫 구현 범위를 키우면 안 됩니다.

## 기준 기록

- `tdd_traces`
- selected `feedback_loops`
- Task와 Change Unit 참조
- RED, GREEN, refactor/check artifact 참조
- 근거 목록(Evidence Manifest) coverage 참조
- waiver 또는 non-TDD justification 참조
- 해당되는 경우 근거 목록(Evidence Manifest), 사용자 판단(User Judgment), Change Unit, 잔여 위험(Residual Risk), 수동 QA, Eval(분리 검증 결과), close blocker, follow-up ref를 통한 finding route
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

> 상태 보기(Projection): `source_state_version`와 `updated_at` 기준으로 렌더링되며 TDD record와 ref를 표시합니다. 계획 문구는 기록된 artifact 또는 result ref가 뒷받침하기 전까지 RED 근거가 아닙니다.

## 식별 정보
- task_id:
- change_unit_id:
- trace 상태: required | recorded | waived | not_required
- 요구/출처:
- feedback loop 참조:
- 근거 목록(Evidence Manifest) coverage 참조:

## Red(실패 단계)
- 대상 / 계획:
- 실패 테스트 ref:
- 명령:
- 결과: failed_as_expected | failed_unexpectedly | missing
- 로그 참조:
- non-test 구현 전 기록 여부: yes | no | waived
- 대상 / 계획은 근거 목록(Evidence Manifest) coverage로 계산됨: no

## Green(통과 단계)
- 명령:
- 결과: passed | failed | missing
- 로그 참조:

## Refactor(정리 단계)
- 수행 여부: yes | no
- 메모:
- verification 명령:
- 로그 참조:

## Non-TDD 근거
- 이유:
- feedback loop 참조:
- 대체 feedback loop:
- non-test 구현 전 waiver 기록 여부: yes | no

## 근거 참조
- test:
- RED 로그:
- GREEN 로그:
- refactor/check 로그:
- 근거 목록(Evidence Manifest):
- diff:

## Finding 라우팅
- 근거 공백 또는 뒷받침:
- 사용자 판단 후보 또는 참조:
- Change Unit 업데이트 또는 후속 조치:
- 잔여 위험 후보 또는 참조:
- 수동 QA 또는 Eval(분리 검증 결과) 참조:
- 닫기 막힘:
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. RED target 또는 plan은 계획 맥락이며, 실제 RED 근거는 여전히 기록된 artifact 또는 result ref에서 나와야 합니다.

TDD가 advisory일 뿐 required 또는 selected가 아니라면 TDD waiver는 필요하지 않습니다. Required, selected, recorded, waived TDD는 owner record에서만 렌더링하고, finding은 template-only state를 추가하지 말고 기존 owner ref로 라우팅합니다.
