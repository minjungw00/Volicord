# DEC Template

## 사용 시점

Standalone Decision Packet projection이 켜져 있고 제품 판단, approval 형태의 judgment, waiver, acceptance, residual-risk acceptance, reconcile decision을 보여줘야 할 때 `DEC`를 사용합니다.

## 기준 기록

- `state.sqlite.decision_packets`
- 관련 Task와 Change Unit 참조
- 관련 `decision_gate` 상태와 decision event
- approval-shaped decision의 approval 기록
- 필요한 경우 관련 reconcile 기록
- residual risk 참조
- evidence 및 artifact 참조
- projection 최신성 입력

## 렌더링 섹션

- Why Now
- Current State
- Approval-Shaped Context, If Applicable
- What User Is Deciding
- What Agent May Decide Without User
- Autonomy Boundary Impact, If Any
- Options
- Recommendation
- Consequence Of Deferring
- Minimum Context To Judge
- User Decision And Accepted Risk
- Follow-Up
- References

## 예시 내용 단서

다음과 같은 Decision Packet에도 같은 렌더링 섹션을 사용합니다. 이 단서는 추가 template section이 아닙니다.

- Product/UX trade-off: 로그인 실패 피드백을 inline message, toast, modal/layer 중에서 고르는 경우입니다. 흐름, 방해 정도, 접근성, 문구, 제품 위험의 차이는 Options와 Recommendation에 둡니다.
- 기술 선택: session cookie, JWT, social login 중에서 고르는 경우입니다. 폐기 가능성, CSRF/XSS 노출, client 호환성, 구현 비용, migration 영향은 Options와 Minimum Context To Judge에 둡니다.
- 보안 민감 approval: approval boundary는 Approval-Shaped Context에 둡니다. 역할, exported fields, redaction, audit logging, retention, rollback, user notice가 아직 결정되지 않았다면 해결되지 않은 제품/보안 판단으로 표시하고 별도의 compatible Decision Packet으로 보냅니다. approval packet 하나가 그 판단까지 해결한 것처럼 쓰면 안 됩니다.
- QA 또는 verification waiver: 생략하는 확인, 수용하는 사용자·제품·기술 위험, 가장 작은 신뢰 가능한 follow-up은 User Decision And Accepted Risk와 Follow-Up에 둡니다.
- Close 전 residual-risk acceptance: 사용자에게 보인 한계, 기존 근거, 사용자가 수용할지 판단해야 하는 risk ref, 남은 follow-up은 Current State, Minimum Context To Judge, User Decision And Accepted Risk, Follow-Up에 둡니다.

## 전체 템플릿

````md
---
doc_type: decision_packet
projection_kind: DEC
projection_id: DEC-PROJ-0001
decision_packet_id: DEC-0001
task_id: TASK-0001
change_unit_id: CU-01
decision_kind: product_tradeoff
status: pending_user
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# DEC-0001 Decision Packet Title

## Why Now
- trigger:
- blocker:
- affected operation:
- why this cannot proceed under current state:

## Current State
- task state:
- active change unit:
- current gates:
- latest evidence:
- residual risk:
- source refs:

## Approval-Shaped Context, If Applicable
- decision_kind=approval scope:
- linked approval record:
- sensitive categories:
- separate Decision Packet이 필요한 product judgment:
- approval boundary:
- write authorization boundary:

## What User Is Deciding
- decision:
- affected scope:
- affected acceptance criteria:
- affected gates:

## What Agent May Decide Without User
- implementation detail:
- code organization inside approved scope:
- evidence collection:
- follow-up proposal:

## Autonomy Boundary Impact, If Any
- current boundary impact:
- proposed boundary update:
- user judgment required:

## Options
### Option A
- choice:
- benefits:
- costs:
- risks:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high
- evidence refs:

### Option B
- choice:
- benefits:
- costs:
- risks:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high
- evidence refs:

## Recommendation
- recommendation:
- reason:
- uncertainty:

## Consequence Of Deferring
- consequence:
- operation impact:
- close impact:
- residual risk or follow-up visibility:

## Minimum Context To Judge
- relevant facts:
- assumptions:
- constraints:
- evidence refs:
- residual risk refs:
- related decisions:

## User Decision And Accepted Risk
- status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
- selected option:
- user decision:
- decision note:
- accepted residual-risk summary:
- accepted residual-risk refs:
- accepted consequence:
- decided by:
- decided at:

## Follow-Up
- [ ]

## References
- TASK:
- Change Unit:
- DESIGN:
- APR:
- EVIDENCE-MANIFEST:
- EVAL:
- MANUAL-QA:
- Residual Risk:
- artifacts:
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. Standalone `DEC` projection이 켜져 있지 않다면 MVP Decision Packet visibility는 여전히 `TASK` projection, status/next response, judgment-context resource, decision-packet resource를 통해 제공됩니다.

Option subsection은 필요한 만큼 반복할 수 있습니다. 어떤 제품 선택은 현실적인 선택지가 두 개보다 많습니다.
