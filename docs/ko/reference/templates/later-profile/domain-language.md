# DOMAIN-LANGUAGE 템플릿

## 사용 시점

현재 domain term의 의미, 코드 표현, 대기 중인 용어 판단, 폐기된 용어, 사람이 제안한 변경 사항을 읽기 쉬운 상태 보기(projection)로 볼 때 `DOMAIN-LANGUAGE`를 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. Domain-language map은 stewardship 프로필용으로 유지하며 초기 필수 상태 보기가 아닙니다.

## 기준 기록

- `domain_terms`
- domain term 변경을 제안하는 reconcile 항목
- term을 도입하거나 reconcile로 조정한 Task 참조
- domain-language conflict에 사용자 소유 판단이 필요할 때 관련 사용자 판단
- `domain_language` 관련 design-quality validator 결과
- 표시되는 경우 domain-language ref에 영향을 주는 라우팅된 stewardship finding
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- 요약
- 용어
- 대기 중인 용어 판단
- 폐기된 용어
- 사용자 메모와 제안

## 전체 템플릿

````md
---
doc_type: domain_language
project_id: PRJ-0001
status: active
projection_version: 1
source_state_version: 12
updated_at: 2026-05-06T09:30:15+09:00
---

# Domain Language(도메인 언어)

> 상태 보기(Projection): `domain_terms`와 관련 ref를 `source_state_version` / `updated_at` 기준으로 렌더링한 보기입니다. 관리 섹션(Managed section)은 생성된 표시 영역이며, reconcile 입력은 `사용자 메모와 제안`에 적습니다.

<!-- HARNESS:BEGIN managed -->
## 요약
- 현재 상태:
- 최근 reconcile된 task:
- 오래된 것으로 보는 조건:

## 용어
| 용어 | 의미 | 코드 표현 | 이것이 아님 | 관련 용어 | 출처 | 상태 |
|---|---|---|---|---|---|---|
| Account | login할 수 있는 사용자 identity | `src/auth/account.ts` | Profile | User, Session | TASK-0001 | active |

## 대기 중인 용어 판단
| 용어 | 질문 | 선택지 | 추천 | 소유자 |
|---|---|---|---|---|

## 폐기된 용어
| 용어 | 대체 용어 | 이유 | 이후 적용 |
|---|---|---|---|
<!-- HARNESS:END managed -->

## 사용자 메모와 제안
<!-- Human-editable: 여기의 용어 제안은 reconcile/Core를 통해 accepted되기 전에는 기준 domain term이 아닙니다. -->
-
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. 기준 domain term 참조는 `StateRecordRef.record_kind=domain_term`을 사용합니다. 대기 중인 용어 판단, 최신 검토 문구, 사람이 제안한 내용은 표시 또는 reconcile input입니다. 그 자체로 gate를 충족하거나, write를 승인하거나, evidence를 만들거나, risk를 받아들이거나, work를 close하지 않습니다.

Term conflict가 제품 의미, 공개 동작, API/interface naming, 문서 약속, 수용 기준, module responsibility를 바꾸면 해당 판단은 기존 design-quality 및 사용자 판단 경로로 라우팅합니다. Conflict를 여기에 렌더링하는 것만으로 `design_gate`, `decision_gate`, close impact가 해소되지는 않습니다.
