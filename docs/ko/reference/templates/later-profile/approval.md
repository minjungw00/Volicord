# APR 템플릿

## 사용 시점

나중의 Approval 프로필이 Approval request record를 commit한 뒤, 민감한 행동 요청과 결정을 읽기 쉽게 보여줘야 할 때 `APR`을 사용합니다. `APR`은 민감 동작 승인 범위를 보여줄 뿐이며 사용자 소유의 제품/UX 판단이나 기술 판단, 정확성, 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, 배포, merge, Write Authorization을 결정하지 않습니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 보증 프로필 보고서입니다. `APR`은 commit된 민감 동작 Approval 지원이 active일 때만 사용하며 내부 엔지니어링 점검이나 MVP-1 다섯 가지 보기 세트의 일부가 아닙니다.

## 기준 기록

- committed Approval 기록
- 관련 민감 동작 승인 `user_judgment`
- 구현이 유지하는 경우 선택적 user-judgment request 라우팅/replay 기록
- Change Unit 범위
- 민감 category
- 허용된 path, tool, command, network target, secret
- baseline, 만료 조건, 대안, decision note
- 경계 맥락으로 표시될 때 관련 쓰기 허가 기록(Write Authorization), 아티팩트 참조, redaction state, 읽기용 보기 최신성(projection freshness)

`prepare_write`가 반환한 상태를 변경하지 않는 `approval_request_candidate`는 `APR` source가 아닙니다. 표시가 필요하면 candidate 표시로만 보여줍니다.

경계 요약은 Approval 범위, 연결된 Approval 기록, 관련 user judgment ref, 현재 쓰기 또는 닫기 맥락에서 파생한 표시 block입니다. 나중의 Approval 프로필에서 사용자에게 경계를 다시 알려주는 표시이며, 독립된 권한 출처나 gate가 아닙니다.

## 렌더링 섹션

- 요청 요약
- 출처 참조
- 경계 요약
- 관련 사용자 판단
- 요청된 범위
- 만료와 사용
- 필요한 이유
- 영향
- 위험
- 대안
- 추천
- 결정
- 경계

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

## 요청 요약
- 제안된 동작:
- 승인하려는 sensitive action:
- 여기서 'approved'가 의미하는 것:

## 출처 참조
- Approval 기록:
- 관련 user judgment:
- 관련 Write Authorization:
- 아티팩트 참조:
- redaction state:
- 보기 최신성:

## 경계 요약
- 이 request가 포괄하는 것:
- 이 request가 결정하지 않는 것:
- 승인되더라도 이후 여전히 필요한 것:
- 작업 수락 경계:
- 잔여 위험 수용 경계:
- 면제 판단 경계:
- secret 노출 경계:

## 관련 사용자 판단
- 민감 동작 승인 user judgment:
- 필요한 경우 사용자 소유의 제품/UX 판단 또는 기술 판단을 위한 별도 user judgment:
- decision_gate 영향:
- approval_gate 영향:

## 요청된 범위
- 민감 category:
- 허용 path:
- 허용 tool:
- 허용 command:
- 허용 network target:
- 필요한 secret:
- baseline ref:
- 예상 diff envelope:
- 범위가 drift되면 만료:

## 만료와 사용
- 만료 시각:
- 만료 조건:
- Approval 재사용:
- 해당되는 경우 1회 사용 동작:
- Write Authorization 경계:

## 필요한 이유
- 목적:
- 현재 Task와의 관계:

## 영향
- code/docs:
- user/operations:
- security/privacy:
- cost/deployment:
- domain language:
- module/interface:

## 위험
- 주요 위험:
- 실패 영향:
- 범위 drift 조건:

## 대안
### 대안 A
- 설명:
- 이점:
- 비용/위험:

### 대안 B
- 설명:
- 이점:
- 비용/위험:

## 추천
- 추천:
- 이유:

## 결정
- status: pending | granted | denied | expired
- decision note:
- 결정한 사람:
- 결정 시각:
- 넓은 approval 확인: 이 decision은 위의 민감 동작 승인만 기록하며, "go ahead", "proceed", "looks good", "좋아", "진행해" 같은 표현이 이를 넓히지 않는다.

## 경계
- Approval은 사용자 소유의 제품/UX 판단이나 기술 판단을 해소하지 않고, correctness를 증명하지 않고, verification이나 수동 QA를 대체하지 않고, 작업 수락을 암시하지 않으며, 잔여 위험 수용을 대신하지 않는다.
- Approval은 QA 또는 검증을 면제하지 않는다. 면제 판단은 policy가 허용할 때 별도의 scoped waiver path가 필요하다.
- Approval은 Write Authorization이 아니다. 이후 호환되는 `prepare_write` retry가 write를 allow해야 implementation 또는 direct `record_run`이 authorization을 consume할 수 있다.
- dependency install Approval은 그 dependency를 사용하는 architecture 방향을 결정하지 않는다.
- secret access Approval은 secret 값을 artifacts, projections, exports, logs, screenshots, summaries에 노출해도 된다는 뜻이 아니다.
- auth, permission, system-change Approval은 session auth, JWT, social login, role model, lockout behavior, user notice를 결정하지 않는다.
- public API 방향, deployment, merge, 작업 수락, 잔여 위험 수용, 면제 판단, 추가 write attempt에는 필요한 경우 각각 적용되는 기록된 decision 또는 authority가 필요하다.
````

## 메모

이 template은 렌더링 projection일 뿐 Approval 권한이 아닙니다. Approval 기록과 Approval decision path가 계속 기준이며, 이 Markdown은 request, decision, boundary를 표시만 합니다.

경계 section은 사용자에게 보이는 안내입니다. User-judgment request 라우팅 기록만으로는 판단 권한이 생기지 않으며, 연결된 compatible `user_judgment`를 통하지 않고는 `decision_gate`에 영향을 줄 수 없습니다.

Approval wording은 broad answer를 유도하면 안 됩니다. 사용자가 "go ahead", "proceed", "looks good", "좋아", "진행해"라고 말하더라도 rendered decision은 이름 붙은 sensitive action과 scope만 승인됐음을 계속 보여줘야 합니다. 그 답변이 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, 다른 pending user judgment도 가리킬 수 있으면 기록하기 전에 다시 확인합니다.
