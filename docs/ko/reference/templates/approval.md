# APR Template

## 사용 시점

Approval 요청이 기록된 뒤, 민감한 변경 요청과 결정을 읽기 쉽게 보여줘야 할 때 `APR`을 사용합니다.

## 기준 기록

- Approval 기록
- 관련 Approval 형태 Decision Packet
- 구현이 유지하는 경우 선택적 decision request 라우팅/replay 기록
- Change Unit scope
- 민감 category
- 허용된 path, tool, command, network target, secret
- baseline, expiry, alternative, decision note

`prepare_write`가 반환한 상태를 변경하지 않는 `approval_request_candidate`는 `APR` source가 아닙니다. 표시가 필요하면 candidate 표시로만 보여줍니다.

Boundary Summary field는 Approval 범위, 연결된 Approval 기록, 관련 Decision Packet ref, 현재 쓰기 또는 닫기 context에서 파생한 표시 전용 요약입니다. 기준 schema field, DDL column, 상태 기록, 권한 입력, 독립 gate가 아닙니다.

## 렌더링 섹션

- Request Summary
- Boundary Summary
- Related Decision Packet
- Requested Scope
- Why This Is Needed
- Impact
- Risks
- Alternatives
- Recommendation
- Decision
- Boundary

## 전체 템플릿

````md
---
doc_type: approval
approval_id: APR-0001
task_id: TASK-0001
category: dependency_change
status: pending
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# APR-0001 Approval 요청

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 Approval 요청과 경계를 표시합니다. Approval은 여전히 기준 Approval 결정 경로를 거쳐야 하며, write에는 호환되는 `prepare_write`가 필요합니다.

## Request Summary
- proposed action:
- 승인하려는 sensitive action:
- 여기서 'approved'가 의미하는 것:

## Boundary Summary
- 이 request가 포괄하는 것:
- 이 request가 결정하지 않는 것:
- 승인되더라도 이후 여전히 필요한 것:
- secret 노출 경계:

## Related Decision Packet
- Approval 형태 Decision Packet:
- 필요한 경우 제품 또는 중요한 기술 판단을 위한 별도 Decision Packet:
- decision gate impact:
- approval gate impact:

## Requested Scope
- sensitive categories:
- allowed paths:
- allowed tools:
- allowed commands:
- allowed network targets:
- required secrets:
- baseline ref:
- expected diff envelope:
- expires on scope drift:

## Why This Is Needed
- purpose:
- relation to current task:

## Impact
- code/docs:
- user/operations:
- security/privacy:
- cost/deployment:
- domain language:
- module/interface:

## Risks
- main risk:
- failure impact:
- scope drift condition:

## Alternatives
### Alternative A
- description:
- benefit:
- cost/risk:

### Alternative B
- description:
- benefit:
- cost/risk:

## Recommendation
- recommendation:
- reason:

## Decision
- status: pending | granted | denied | expired
- decision note:
- decided by:
- decided at:

## Boundary
- Approval은 사용자 소유의 제품 판단이나 중요한 기술 판단을 해소하지 않고, correctness를 증명하지 않고, verification이나 Manual QA를 대체하지 않고, acceptance를 암시하지 않으며, residual risk를 수용하지 않는다.
- Approval은 Write Authorization이 아니다. 이후 호환되는 `prepare_write` retry가 write를 allow해야 implementation 또는 direct `record_run`이 authorization을 consume할 수 있다.
- dependency install Approval은 그 dependency를 사용하는 architecture 방향을 결정하지 않는다.
- secret access Approval은 secret 값을 artifacts, projections, exports, logs, screenshots, summaries에 노출해도 된다는 뜻이 아니다.
- auth, permission, system-change Approval은 session auth, JWT, social login, role model, lockout behavior, user notice를 결정하지 않는다.
- public API 방향, deployment, merge, final acceptance, 추가 write attempt에는 필요한 경우 각각 호환되는 권한 경로가 필요하다.
````

## 메모

Boundary section은 사용자에게 보이는 안내입니다. Decision request 라우팅 기록만으로는 decision 권한이 생기지 않으며, 연결된 compatible Decision Packet을 통하지 않고는 `decision_gate`에 영향을 줄 수 없습니다.
