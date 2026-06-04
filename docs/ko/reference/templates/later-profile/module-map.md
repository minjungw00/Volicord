# MODULE-MAP 템플릿

## 사용 시점

모듈 역할, 공개 interface, 내부 복잡도, 의존성, 테스트 경계, 소유자 판단, watchpoint를 읽기 쉬운 상태 보기(projection)로 확인해야 할 때 `MODULE-MAP`을 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. Module Map output은 later stewardship/reference 보기이며 첫 구현 slice나 MVP-1 다섯 가지 보기 세트에 필요하지 않습니다.

## 기준 기록

- `module_map_items`
- module map 항목에 저장된 모듈 단위 watchpoint
- module map 변경을 제안하는 reconcile 항목
- 관련 사용자 판단과 design ref
- `deep_module_interface` 또는 `codebase_stewardship` 관련 design-quality validator 결과
- 표시되는 경우 module 또는 boundary ref에 영향을 주는 라우팅된 stewardship finding
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- 요약
- 모듈
- Deep Module 후보
- Module watchpoint 모음
- 사용자 메모와 제안

## 전체 템플릿

````md
---
doc_type: module_map
project_id: PRJ-0001
status: active
projection_version: 1
source_state_version: 12
updated_at: 2026-05-06T09:30:15+09:00
---

# Module Map(모듈 맵)

> 상태 보기(Projection): `module_map_items`와 관련 ref를 `source_state_version` / `updated_at` 기준으로 렌더링한 보기입니다. 관리 섹션(Managed section)은 생성된 표시 영역이며, reconcile 입력은 `사용자 메모와 제안`에 적습니다.

<!-- HARNESS:BEGIN managed -->
## 요약
- architecture 상태:
- 최근 review:
- 오래된 것으로 보는 조건:

## 모듈
| 모듈 | 역할 | 공개 interface | 내부 복잡도 | 의존성 | 테스트 경계 | 소유자 판단 | 주의 지점 |
|---|---|---|---|---|---|---|---|
| AuthService | auth를 확인하고 session을 발급함 | `login`, `logout` | credential validation, session issue | UserRepo, SessionStore | service interface tests | human_reviewed | session expiry drift |

## Deep Module 후보
| 후보 | 현재 문제 | 제안 경계 | 예상 테스트 경계 | 우선순위 |
|---|---|---|---|---|

## Module watchpoint 모음
- 출처: `module_map_items.watchpoints_json`
- 기준 owner: Module Map Item; 전용 architecture watchpoint ref는 later DDL batch가 정의한 경우에만 사용한다
- 얕은 module 성장:
- 의존성 방향 위험:
- public interface drift:
<!-- HARNESS:END managed -->

## 사용자 메모와 제안
<!-- Human-editable: 여기의 module 제안은 reconcile/Core를 통해 accepted되기 전에는 기준 Module Map Item이 아닙니다. -->
-
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. 기준 module 참조는 `StateRecordRef.record_kind=module_map_item`을 사용합니다. Review, watchpoint, stewardship rollup text는 owner record 위의 display이며 민감 동작 승인(Approval), evidence, QA, verification, 작업 수락, 잔여 위험 수용, 닫기, 쓰기 허가 기록(Write Authorization)을 만들지 않습니다.

제안된 module boundary change가 product commitment, public interface, caller obligation, dependency direction, architecture direction을 바꾸면 해당 판단은 기존 design-quality 및 사용자 판단 경로로 라우팅합니다. Proposal을 여기에 렌더링하는 것만으로 `design_gate`, `decision_gate`, close impact가 해소되지는 않습니다.
