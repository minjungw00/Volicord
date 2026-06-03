# MODULE-MAP Template

## 사용 시점

모듈 역할, 공개 interface, 내부 복잡도, 의존성, 테스트 경계, 소유자 결정, watchpoint를 읽기 쉬운 projection으로 확인해야 할 때 `MODULE-MAP`을 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: Future/diagnostic projections입니다. Module Map output은 later stewardship/reference view이며 first runnable slice나 v0.2 compact-card MVP에 필요하지 않습니다.

## 기준 기록

- `module_map_items`
- module map 항목에 저장된 모듈 단위 watchpoint
- module map 변경을 제안하는 reconcile item
- 관련 Decision Packet과 design ref
- `deep_module_interface` 또는 `codebase_stewardship` 관련 design-quality validator 결과
- 표시되는 경우 module 또는 boundary ref에 영향을 주는 routed stewardship finding
- 읽기용 보기 최신성(projection freshness) 입력

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

> Projection 보기: `module_map_items`와 관련 ref를 `source_state_version` / `updated_at` 기준으로 렌더링한 보기입니다. Managed section은 생성된 표시 영역이며, reconcile 입력은 `User Notes and Proposals`에 적습니다.

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
<!-- Human-editable: 여기의 module proposal은 reconcile/Core를 통해 accepted되기 전에는 기준 Module Map Item이 아닙니다. -->
-
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. 기준 module 참조는 `StateRecordRef.record_kind=module_map_item`을 사용합니다. Review, watchpoint, stewardship rollup text는 owner record 위의 display이며 Approval, evidence, QA, verification, 작업 수락, 잔여 위험 수용, close, Write Authorization을 만들지 않습니다.

제안된 module boundary change가 product commitment, public interface, caller obligation, dependency direction, architecture direction을 바꾸면 해당 판단은 기존 design-quality 및 Decision Packet 경로로 라우팅합니다. Proposal을 여기에 렌더링하는 것만으로 `design_gate`, `decision_gate`, close impact가 해소되지는 않습니다.
