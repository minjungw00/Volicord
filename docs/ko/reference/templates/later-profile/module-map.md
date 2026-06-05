# MODULE-MAP 템플릿

## 사용 시점

모듈 역할, 공개 인터페이스, 내부 복잡도, 의존성, 테스트 경계, 소유자 판단, 주의 지점을 읽기 쉬운 상태 보기(projection)로 확인해야 할 때 `MODULE-MAP`을 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. 모듈 맵(Module Map) 출력은 나중 스튜어드십/참조 보기이며 첫 구현 조각이나 활성 MVP-1 작은 출력 세트에 필요하지 않습니다.

## 기준 기록

- `module_map_items`
- 모듈 맵(module map) 항목에 저장된 모듈 단위 주의 지점
- 모듈 맵(module map) 변경을 제안하는 조정(reconcile) 항목
- 관련 사용자 판단과 설계 참조
- `deep_module_interface` 또는 `codebase_stewardship` 관련 설계 품질 검증기 결과
- 표시되는 경우 모듈 또는 경계 참조에 영향을 주는 라우팅된 스튜어드십 발견 사항
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- 요약
- 모듈
- 깊은 모듈(Deep Module) 후보
- 모듈 주의 지점 모음
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

# 모듈 맵(Module Map)

> 상태 보기(Projection): `module_map_items`와 관련 참조를 `source_state_version` / `updated_at` 기준으로 렌더링한 보기입니다. 관리 섹션은 생성된 표시 영역이며, 조정(reconcile) 입력은 `사용자 메모와 제안`에 적습니다.

<!-- HARNESS:BEGIN managed -->
## 요약
- 아키텍처 상태:
- 최근 검토:
- 오래된 것으로 보는 조건:

## 모듈
| 모듈 | 역할 | 공개 인터페이스 | 내부 복잡도 | 의존성 | 테스트 경계 | 소유자 판단 | 주의 지점 |
|---|---|---|---|---|---|---|---|
| AuthService | 인증을 확인하고 세션을 발급함 | `login`, `logout` | 자격 증명 검증, 세션 발급 | UserRepo, SessionStore | 서비스 인터페이스 테스트 | human_reviewed | 세션 만료 불일치(drift) |

## 깊은 모듈(Deep Module) 후보
| 후보 | 현재 문제 | 제안 경계 | 예상 테스트 경계 | 우선순위 |
|---|---|---|---|---|

## 모듈 주의 지점 모음
- 출처: `module_map_items.watchpoints_json`
- 기준 owner: Module Map Item; 전용 아키텍처 주의 지점 참조는 나중 DDL 배치가 정의한 경우에만 사용한다
- 얕은 모듈 성장:
- 의존성 방향 위험:
- 공개 인터페이스 drift(불일치):
<!-- HARNESS:END managed -->

## 사용자 메모와 제안
<!-- 사람이 편집 가능: 여기의 모듈 제안은 조정(reconcile)/Core를 통해 수락(accepted)되기 전에는 기준 Module Map Item이 아닙니다. -->
-
````

## 메모

이 템플릿은 렌더링 결과일 뿐 기준 상태가 아닙니다. 기준 모듈 참조는 `StateRecordRef.record_kind=module_map_item`을 사용합니다. 검토, 주의 지점, 스튜어드십 모음 문구는 owner 기록 위의 표시이며 민감 동작 승인(Approval), 근거, QA, 검증, 최종 수락, 잔여 위험 수락, 닫기, 쓰기 허가 기록(Write Authorization)을 만들지 않습니다.

제안된 모듈 경계 변경이 제품 약속, 공개 인터페이스, 호출자 의무, 의존성 방향, 아키텍처 방향을 바꾸면 해당 판단은 기존 설계 품질 및 사용자 판단 경로로 라우팅합니다. 제안을 여기에 렌더링하는 것만으로 `design_gate`, `decision_gate`, 닫기 영향이 해소되지는 않습니다.
