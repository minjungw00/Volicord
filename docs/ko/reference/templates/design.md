# DESIGN Template

## 사용 시점

Shared design, domain language 영향, module/interface 계획, 대안, 추천안, verification consideration을 독립적으로 읽을 수 있는 projection으로 볼 때 `DESIGN`을 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: Future/diagnostic projections입니다. Standalone design projection은 later-profile scope이며 early 사용자 판단 맥락은 사용자 판단 요청 display에 나타날 수 있습니다.

## 기준 기록

- shared design 기록과 event
- Task와 Change Unit 참조
- 관련 Decision Packet과 approval
- `domain_terms`
- `module_map_items`
- `interface_contracts`
- feedback loop, TDD, 수동 QA, evidence 참조
- 표시되는 경우 기존 owner path로 라우팅된 design-quality 또는 stewardship finding
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- Problem
- Goals
- Non-Goals
- Constraints
- Shared Design Summary
- Domain Language Impact
- Module And Interface Plan
- Proposed Shape
- Alternatives
- Recommendation
- Verification Considerations
- References

## 전체 템플릿

````md
---
doc_type: design
design_id: DESIGN-0001
task_id: TASK-0001
status: draft
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# DESIGN-0001 Design Title

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 owner record와 proposal을 요약합니다. 이 문서를 편집해도 Domain Language, Module Map, Interface Contract, Decision Packet, Task state를 대체하지 않습니다.

## Problem
- design problem:

## Goals
- goal:

## Non-Goals
- non-goal:

## Constraints
- technical:
- operational:
- compatibility:
- security/privacy:

## Shared Design Summary
- resolved questions:
- remaining assumptions:
- rejected options:

## Domain Language Impact
| Term | Impact | Action |
|---|---|---|

## Module And Interface Plan
| Module | Current Role | Proposed Change | Public Interface | Test Boundary | Risk |
|---|---|---|---|---|---|

## Proposed Shape
- components:
- boundaries and responsibilities:
- data flow:
- dependency direction:

## Alternatives
### Alternative A
- benefits:
- drawbacks:

### Alternative B
- benefits:
- drawbacks:

## Recommendation
- recommendation:
- remaining trade-off:

## Verification Considerations
- success criteria:
- regression watchpoint:
- selected feedback loop:
- required TDD trace:
- required 수동 QA:
- required evidence:

## References
- TASK:
- DEC:
- APR:
- design-support owner refs:
  - domain term refs:
  - module map item refs:
  - interface contract refs:
- rendered projection refs, if shown:
  - DOMAIN-LANGUAGE:
  - MODULE-MAP:
  - INTERFACE-CONTRACT:
- EVIDENCE-MANIFEST:
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. Design-support owner ref와 라우팅된 stewardship finding을 요약할 수 있지만 owner 기록이나 Review Stages가 가리키는 owner path를 대체하지 않습니다. Close를 충족하거나 차단하지 않고, Approval을 부여하지 않으며, 근거 생성, QA 또는 검증 기록, 작업 수락, 잔여 위험 수용, Write Authorization 생성을 하지 않습니다.
