# DEC Template

## 사용 시점

Standalone Decision Packet projection이 켜져 있고 사용자 소유의 제품 판단 또는 중요한 기술 판단, Approval 형태의 판단, waiver, acceptance, residual-risk acceptance, reconcile decision을 보여줘야 할 때 `DEC`를 사용합니다.

## 기준 기록

- `state.sqlite.decision_packets`
- 관련 Task와 Change Unit 참조
- 관련 `decision_gate` 상태와 decision event
- Approval 형태 decision의 Approval 기록
- 필요한 경우 관련 reconcile 기록
- residual risk 참조
- evidence 및 artifact 참조
- projection 최신성 입력

Approval 형태 표시 항목인 "이 Approval이 포괄하는 것", "이 Approval이 포괄하지 않는 것", "secret 노출 경계"는 연결된 Approval 기록, Approval 범위, 관련 Decision Packet ref, 현재 쓰기 또는 닫기 context에서 파생한 표시 전용 요약입니다. 경계를 설명할 뿐이며 Approval을 부여하거나 별도의 사용자 소유 판단을 확정하지 않습니다.

해소된 Decision Packet은 Approval 기록에 연결된 Approval 형태 Decision Packet일 때만 sensitive-action Approval입니다. 그 밖의 Decision Packet resolution은 사용자 소유 판단, waiver, residual-risk acceptance, final acceptance, reconcile choice를 확정할 수 있지만 sensitive-action Approval을 부여하지 않습니다.

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
- Dependency Approval과 dependency decision 구분: 사용자가 install command나 dependency-file edit을 승인하는 경우 그 sensitive-action 경계는 Approval-Shaped Context에 둡니다. 그 dependency가 올바른 architecture 방향인지 선택하는 경우에는 technical choice를 What User Is Deciding과 Options에 둡니다.
- 보안 민감 Approval: Approval 경계는 Approval-Shaped Context에 둡니다. 역할, exported fields, redaction, audit logging, retention, rollback, user notice가 아직 결정되지 않았다면 해결되지 않은 제품/보안 판단으로 표시하고 별도의 compatible Decision Packet으로 보냅니다. Approval packet 하나가 그 판단까지 해결한 것처럼 쓰면 안 됩니다.
- Public API/interface decision: 호출자 호환성, migration path, documentation promise, rollback risk는 Options와 Minimum Context To Judge에 둡니다. Resolved API decision을 merge 권한, deployment 권한, Write Authorization처럼 다루면 안 됩니다.
- QA 또는 verification waiver: 생략하는 확인이나 대상 접점, 수용하는 사용자·제품·기술 위험, 관련 refs, 닫기 영향, 가장 작은 신뢰 가능한 follow-up은 User Decision And Accepted Risk와 Follow-Up에 둡니다.
- Close 전 residual-risk acceptance: 사용자에게 보인 한계, 기존 근거, 사용자가 수용할지 판단해야 하는 risk ref, 남은 follow-up은 Current State, Minimum Context To Judge, User Decision And Accepted Risk, Follow-Up에 둡니다.
- Final acceptance: 최종 결과, evidence 상태, Manual QA와 verification 상태, close-relevant residual-risk visibility는 Current State와 Minimum Context To Judge에 둡니다. Final acceptance를 새 sensitive action, 추가 write, deployment, merge approval처럼 다루면 안 됩니다.

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

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며, state의 `decision_packet_id`와 관련 ref를 표시합니다. 이 Markdown을 편집해도 Decision Packet은 해결되지 않으며, decision은 decision path를 통해 기록됩니다.

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
- `decision_kind=approval` 범위:
- linked approval record:
- sensitive categories:
- 이 Approval이 포괄하는 것:
- 이 Approval이 포괄하지 않는 것:
- separate Decision Packet이 필요한 사용자 소유 판단:
- Approval 경계:
- Write Authorization 경계:
- secret 노출 경계:

## What User Is Deciding
- decision category:
- decision:
- 이 decision이 확정하는 것:
- 이 decision이 확정하지 않는 것:
- affected scope:
- affected acceptance criteria:
- affected gates:

## What Agent May Decide Without User
- implementation detail:
- code organization inside granted scope:
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
- 닫기 영향:
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
