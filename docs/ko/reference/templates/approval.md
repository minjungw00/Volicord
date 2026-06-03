# APR Template

## 사용 시점

Approval 요청이 기록된 뒤, 민감한 행동 요청과 결정을 읽기 쉽게 보여줘야 할 때 `APR`을 사용합니다. `APR`은 민감 동작 승인 범위를 보여줄 뿐이며 사용자 소유의 제품 판단이나 기술 구조 판단, correctness, 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, deployment, merge, Write Authorization을 결정하지 않습니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: Agency assurance reports입니다. `APR`은 committed sensitive-action Approval support가 active일 때만 사용하며 v0.1이나 v0.2 compact-card MVP의 일부가 아닙니다.

## 기준 기록

- Approval 기록
- 관련 Approval 형태 Decision Packet
- 구현이 유지하는 경우 선택적 decision request 라우팅/replay 기록
- Change Unit scope
- 민감 category
- 허용된 path, tool, command, network target, secret
- baseline, expiry, alternative, decision note
- boundary context로 표시될 때 관련 Write Authorization, artifact refs, redaction state, projection freshness

`prepare_write`가 반환한 상태를 변경하지 않는 `approval_request_candidate`는 `APR` source가 아닙니다. 표시가 필요하면 candidate 표시로만 보여줍니다.

Boundary Summary는 Approval 범위, 연결된 Approval 기록, 관련 Decision Packet ref, 현재 쓰기 또는 닫기 context에서 파생한 표시 block입니다. 사용자에게 경계를 상기시키는 용도이며 독립된 권한 출처나 gate가 아닙니다.

## 렌더링 섹션

- Request Summary
- Source Refs
- Boundary Summary
- Related Decision Packet
- Requested Scope
- Expiry And Use
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

# APR-0001 민감 동작 승인 요청

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 Approval 요청과 경계를 표시합니다. Approval은 sensitive-action permission일 뿐입니다. Approval은 여전히 기준 Approval 결정 경로를 거쳐야 하며, write에는 호환되는 `prepare_write`가 필요합니다.

## Request Summary
- proposed action:
- 승인하려는 sensitive action:
- 여기서 'approved'가 의미하는 것:

## Source Refs
- Approval record:
- Decision Packet:
- related Write Authorization:
- artifact refs:
- redaction state:
- projection freshness:

## Boundary Summary
- 이 request가 포괄하는 것:
- 이 request가 결정하지 않는 것:
- 승인되더라도 이후 여전히 필요한 것:
- 작업 수락 경계:
- 잔여 위험 수용 경계:
- 면제 판단 경계:
- secret 노출 경계:

## Related Decision Packet
- Approval 형태 Decision Packet:
- 필요한 경우 사용자 소유의 제품 판단 또는 기술 구조 판단을 위한 별도 Decision Packet:
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

## Expiry And Use
- expires at:
- expires on:
- approval reuse:
- single-use behavior if applicable:
- Write Authorization boundary:

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
- broad approval check: 이 decision은 위의 민감 동작 승인만 기록하며, "go ahead", "proceed", "looks good", "좋아", "진행해" wording이 이를 넓히지 않는다.

## Boundary
- Approval은 사용자 소유의 제품 판단이나 기술 구조 판단을 해소하지 않고, correctness를 증명하지 않고, verification이나 수동 QA를 대체하지 않고, 작업 수락을 암시하지 않으며, 잔여 위험 수용을 대신하지 않는다.
- Approval은 QA 또는 검증을 면제하지 않는다. 면제 판단은 policy가 허용할 때 별도의 scoped waiver path가 필요하다.
- Approval은 Write Authorization이 아니다. 이후 호환되는 `prepare_write` retry가 write를 allow해야 implementation 또는 direct `record_run`이 authorization을 consume할 수 있다.
- dependency install Approval은 그 dependency를 사용하는 architecture 방향을 결정하지 않는다.
- secret access Approval은 secret 값을 artifacts, projections, exports, logs, screenshots, summaries에 노출해도 된다는 뜻이 아니다.
- auth, permission, system-change Approval은 session auth, JWT, social login, role model, lockout behavior, user notice를 결정하지 않는다.
- public API 방향, deployment, merge, 작업 수락, 잔여 위험 수용, 면제 판단, 추가 write attempt에는 필요한 경우 각각 적용되는 기록된 decision 또는 authority가 필요하다.
````

## 메모

이 template은 렌더링 projection일 뿐 Approval 권한이 아닙니다. Approval 기록과 Approval decision path가 계속 기준이며, 이 Markdown은 request, decision, boundary를 표시만 합니다.

Boundary section은 사용자에게 보이는 안내입니다. Decision request 라우팅 기록만으로는 decision 권한이 생기지 않으며, 연결된 compatible Decision Packet을 통하지 않고는 `decision_gate`에 영향을 줄 수 없습니다.

Approval wording은 broad answer를 유도하면 안 됩니다. 사용자가 "go ahead", "proceed", "looks good", "좋아", "진행해"라고 말하더라도 rendered decision은 이름 붙은 sensitive action과 scope만 승인됐음을 계속 보여줘야 합니다. 그 답변이 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, 다른 pending Decision Packet도 가리킬 수 있으면 기록하기 전에 다시 확인합니다.
