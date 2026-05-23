# MODULE-MAP Template

## 사용 시점

모듈 역할, 공개 interface, 내부 복잡도, 의존성, 테스트 경계, 소유자 결정, watchpoint를 읽기 쉬운 projection으로 확인해야 할 때 `MODULE-MAP`을 사용합니다.

## 기준 기록

- `module_map_items`
- module map 항목에 저장된 모듈 단위 watchpoint
- module map 변경을 제안하는 reconcile item
- 관련 Decision Packet과 design ref
- `deep_module_interface` 또는 `codebase_stewardship` 관련 design-quality validator 결과
- projection 최신성 입력

## 렌더링 섹션

- Summary
- Modules
- Deep Module Candidates
- Module Watchpoint Rollup
- User Notes and Proposals

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

# Module Map

<!-- HARNESS:BEGIN managed -->
## Summary
- architecture state:
- latest review:
- stale conditions:

## Modules
| Module | Role | Public Interface | Internal Complexity | Dependencies | Test Boundary | Owner Decision | Watchpoints |
|---|---|---|---|---|---|---|---|
| AuthService | verifies auth and issues sessions | `login`, `logout` | credential validation, session issue | UserRepo, SessionStore | service interface tests | human_reviewed | session expiry drift |

## Deep Module Candidates
| Candidate | Current Pain | Proposed Boundary | Expected Test Boundary | Priority |
|---|---|---|---|---|

## Module Watchpoint Rollup
- source: `module_map_items.watchpoints_json`
- canonical owner: Module Map Item; 전용 architecture watchpoint ref는 later DDL batch가 정의한 경우에만 사용한다
- shallow module growth:
- dependency direction risk:
- public interface drift:
<!-- HARNESS:END managed -->

## User Notes and Proposals
-
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. 기준 module 참조는 `StateRecordRef.record_kind=module_map_item`을 사용합니다.
