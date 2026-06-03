# DEC Template

## 사용 시점

Standalone Decision Packet projection이 켜져 있고 특정 사용자 소유 판단인 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, 범위/자율성 판단, 민감 동작 승인, QA 면제 판단, 검증 면제 판단, 작업 수락, 잔여 위험 수용, reconcile decision을 보여줘야 할 때 `DEC`를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: User judgment prompt shape입니다. 사용자 소유 판단이 대기 중일 때 Decision Packet display/card shape로 사용하며, v0.2 MVP projection도 standalone `DEC` `ProjectionKind`도 아닙니다. Standalone persisted `DEC` Markdown projection은 standalone Decision Packet projection 기능이 켜진 경우에만 사용하며, 필요한 prompt는 compact status card, status/next, Decision Packet resource를 통해 나타날 수 있습니다.

## 기준 기록

- `state.sqlite.decision_packets`
- 관련 Task와 Change Unit 참조
- `judgment_category`, `judgment_route`, `display_depth`
- `judgment_category`, `judgment_route`, `display_depth`, 관련 owner record에서 파생되는 표시용 judgment type
- 관련 `decision_gate` 상태와 decision event
- Approval 형태 decision의 Approval 기록
- 필요한 경우 관련 reconcile 기록
- 잔여 위험(residual risk) 참조
- evidence 및 artifact 참조
- 관련 authority context로 표시될 때 Write Authorization, Approval, Evidence Manifest, Eval, 수동 QA, 작업 수락 context, Artifact refs, redaction state, projection freshness
- 영향을 받는 범위 표시 input: product area, screen 또는 flow, module, interface, path, 수용 기준, gate, sensitive category
- 읽기용 보기 최신성(projection freshness) 입력

Approval 형태 표시 항목인 "이 Approval이 포괄하는 것", "이 Approval이 포괄하지 않는 것", "secret 노출 경계"는 연결된 Approval 기록, Approval 범위, 관련 Decision Packet ref, 현재 쓰기 또는 닫기 context에서 파생한 표시 전용 요약입니다. 경계를 설명할 뿐이며 Approval을 부여하거나 별도의 사용자 소유 판단을 확정하지 않습니다. Approval 형태 표시는 민감 동작 승인으로 label해야 하며 작업 수락처럼 보이면 안 됩니다.

해소된 Decision Packet은 Approval 기록에 연결된 Approval 형태 Decision Packet일 때만 sensitive-action Approval입니다. 그 밖의 Decision Packet resolution은 사용자 소유 판단, waiver, 잔여 위험 수용, 작업 수락, reconcile choice를 확정할 수 있지만 sensitive-action Approval을 부여하지 않습니다.

`judgment_category`는 사용자에게 보이는 판단 영역입니다. 자연스러운 label로 렌더링합니다. `judgment_route`는 owner path와 recorded-answer rule을 정하는 route로 표시합니다. 영향을 받는 gate는 category label이 아니라 `affected_gates`와 관련 owner ref에서 렌더링합니다. 별도 owner rule이 없으면 `judgment_category`는 close gate aggregation, sensitive-action Approval, waiver behavior, 작업 수락, 잔여 위험 수용을 직접 바꾸지 않습니다.

`display_depth`는 prompt 깊이입니다. `simple`은 간단한 명시적 판단으로 렌더링합니다. 불완전한 full trade-off packet처럼 보이면 안 됩니다. `tradeoff`, `high-risk`, `close-affecting`은 route와 risk level에 필요한 추가 context를 보여줍니다. `display_depth` 자체는 권한을 바꾸지 않습니다. Approval, 작업 수락, waiver, 잔여 위험 수용, 제품/기술 판단을 하나의 답변으로 합치면 안 됩니다.

## 렌더링 섹션

- Why Now
- Current State
- Judgment Category, Route, And Display Depth
- Approval-Shaped Context, If Applicable
- What User Is Deciding
- What Agent May Decide Without User
- Autonomy Boundary Impact, If Any
- Affected Scope And Boundaries
- Options
- Recommendation
- Consequence Of Deferring
- Minimum Context To Judge
- User Judgment
- Residual-Risk Acceptance, If Applicable
- Follow-Up
- References

충분한 rendered Decision Packet은 이 section들로 하나의 사용자 소유 판단에 답하며, 넓은 permission을 요청하지 않습니다. 정확한 public request/response field는 [`harness.request_user_judgment`](../mcp-api-and-schemas.md#harnessrequest_user_judgment)가 소유하고, 기준 authority rule은 [Decision Packet](../kernel.md#decision-packet)과 [Decision Gate](../kernel.md#decision-gate)가 소유합니다. 이 template은 `judgment_category`, `judgment_route`, `display_depth` 같은 existing field를 요약해 보여줄 수 있지만 additional schema field, gate, alternate authority를 추가하면 안 됩니다.

Route별 rendering은 선택된 MCP `judgment_payload`와 `judgment_route`를 따릅니다. 공통 field는 계속 보여주고, 선택한 route와 display depth가 요구하지 않는 section은 생략할 수 있습니다. 최종으로 기록되는 답변은 별도입니다. `judgment_route`가 user-judgment route와 `RecordUserJudgmentPayload` value rule을 고릅니다. `display_depth=simple` card는 question, route, category, scope, 간결한 options 또는 selected outcome, related refs, 답변이 확정하지 않는 것을 보여줘야 하지만, 중요하지 않다면 full pros/cons, recommendation, uncertainty, deferral analysis를 요구하지 않습니다. Higher-depth prompt는 사용자가 risk, trade-off, approval scope, waiver impact, acceptance basis, residual-risk consequence, reconcile target을 판단하는 데 필요한 detailed section을 렌더링해야 합니다.

사용자가 보는 질문은 decision을 직접 물어야 합니다. Option을 선택할지, stated consequence와 함께 defer할지, path를 reject할지, 이름 붙은 check를 waive할지, 이름 붙은 risk를 accept할지, result를 accept할지, 이름 붙은 drift를 reconcile할지 묻습니다. "approve" 또는 "승인"은 Approval에 연결된 Approval 형태 context에서만 사용합니다. 다른 packet kind에서는 어떤 choice를 기록할지와 그 choice 밖에 남는 것이 무엇인지 물어야 합니다. 여러 decision이 대기 중이면 별도 prompt 또는 별도 줄로 렌더링하며 승인, 작업 수락, 잔여 위험 수용을 하나의 답변으로 합치면 안 됩니다.

**예시 내용 단서:**

다음과 같은 Decision Packet에도 같은 렌더링 섹션을 사용합니다. 이 단서는 추가 template section이 아닙니다.

- Tiny unblocker(`display_depth=simple`, `judgment_route=choose`): 이미 scoped된 settings copy change 안에서 button label을 "Save"로 할지 "Update"로 할지 고르는 경우입니다. 간결한 choice, scope, refs, non-effects는 What User Is Deciding과 References에 둡니다. Full architecture-tradeoff layout을 강제하지 않습니다.
- Product/UX trade-off(`judgment_category=product_ux`, `display_depth=tradeoff`): 로그인 실패 피드백을 인라인 메시지, 토스트, 모달 중에서 고르는 경우입니다. 흐름, 방해 정도, 접근성, 문구, 제품 위험의 차이는 Options와 Recommendation에 둡니다.
- Product/copy trade-off: 로그인 실패 문구를 일반적인 문구, 더 구체적인 문구, hybrid 문구 중에서 고르는 경우입니다. 계정 열거(account-enumeration) 위험, 복구 도움 정도, 지원 부담, 명확성, 제품 톤은 Options와 Minimum Context To Judge에 둡니다.
- 기술 구조 판단(`judgment_category=technical_architecture`, `display_depth=tradeoff`): session cookie, bearer/JWT token, OAuth/OIDC provider, social-login provider integration 중에서 고르는 경우입니다. 폐기 가능성, CSRF/XSS 노출, client 호환성, 구현 비용, identity-provider 경계, migration 영향은 Options와 Minimum Context To Judge에 둡니다.
- Dependency 민감 동작 승인과 dependency decision 구분: 사용자가 install command나 dependency-file edit을 승인하는 경우 그 sensitive-action 경계는 Approval-Shaped Context에 둡니다. 그 dependency가 올바른 architecture 방향인지 선택하는 경우에는 technical choice를 What User Is Deciding과 Options에 둡니다.
- Schema/data-model 결정: additive migration, compatibility shim, breaking cleanup, data backfill, migration 근거, rollback risk, test boundary는 Options와 Minimum Context To Judge에 둡니다.
- Scope 또는 Autonomy Boundary 확장: proposed additional surface, current scope 또는 latitude가 부족한 이유, 계속 범위 밖에 남는 것, 더 작은 Change Unit으로 계속할 수 있는지 여부는 Consequence Of Deferring에 둡니다.
- 보안/개인정보 판단(`judgment_category=security_privacy`): PII logging, exported fields, redaction, audit logging, retention, rollback, user notice, role exposure는 privacy exposure, debugging value, 필요한 proof, follow-up을 비교합니다. Sensitive action도 필요하다면 그 Approval 경계는 Approval-Shaped Context에 두고, Approval packet이 security/privacy 판단을 해결한다고 취급하지 않습니다.
- Public API/interface decision: 호출자 호환성, migration path, documentation promise, rollback risk는 Options와 Minimum Context To Judge에 둡니다. Resolved API decision을 merge 권한, deployment 권한, Write Authorization처럼 다루면 안 됩니다.
- QA/verification waiver(`judgment_category=qa_verification`, `judgment_route=waive`): 생략하는 확인, 사용자·제품·기술 측면에서 받아들이는 위험, 관련 refs, 닫기 영향, 가장 작은 신뢰 가능한 follow-up을 User Judgment, 필요한 경우 Residual-Risk Acceptance, Follow-Up에 둡니다.
- 작업 수락(`judgment_category=work_acceptance`, `judgment_route=accept-result`): 최종 결과, evidence 상태, 수동 QA와 검증 상태, close-relevant 잔여 위험 표시는 Current State와 Minimum Context To Judge에 둡니다. 작업 수락을 새 sensitive action, 추가 write, deployment, merge approval처럼 다루면 안 됩니다.
- 닫기 전 잔여 위험 수용(`judgment_category=residual_risk`, `judgment_route=accept-risk`): 사용자에게 보인 한계, 기존 근거, 사용자가 받아들일지 판단해야 하는 risk ref, 남은 follow-up은 Current State, Minimum Context To Judge, Residual-Risk Acceptance, Follow-Up에 둡니다.
- 넓은 "go ahead" 답변: packet이 왜 이 specific route와 option을 묻는지 보여줍니다. 막연한 동의 phrase는 이 packet이 정확히 그 judgment를 기록하지 않는 한 product trade-off, architecture choice, QA 면제 판단, 검증 위험, 작업 수락, 잔여 위험 수용을 해결하지 않습니다.

**Rendered example: minimal decision**

```text
결정: 설정 라벨 문구
표시 깊이: simple
경로/영역: choose, 제품/UX(`product_ux`)
질문: 이 범위의 설정 라벨을 "Save"로 둘까요, "Update"로 둘까요?
범위/ref: CU-04의 설정 폼 문구, source ref TASK-012/CU-04, 민감 동작이나 닫기 위험 ref 없음.
기록할 선택: Save | Update
확정하지 않는 것: 더 넓은 설정 흐름, 현지화 전략, 작업 수락, 잔여 위험 수용, 쓰기 권한.
```

**Rendered example: approval-shaped decision**

```text
결정: 의존성 설치 승인
표시 깊이: high-risk
경로/영역: approve-sensitive-action, 보안/개인정보(`security_privacy`)
질문: 이 작업에서 이름 붙은 의존성 설치 또는 업데이트 동작을 승인할까요?
승인 범위: 이름 붙은 설치 명령 또는 의존성 파일 업데이트, 이름 붙은 manifest/lockfile 경로, 현재 작업과 승인 창 안에서만 유효.
포괄하는 것: 범위가 정해진 민감 동작.
포괄하지 않는 것: 그 의존성이 올바른 기술 방향인지에 대한 판단, 이후 설치, 관련 없는 제품 쓰기, QA 또는 검증 면제, 작업 수락, 잔여 위험 수용.
별도 판단 필요: 의존성 선택 자체가 사용자 소유 기술 판단이면 `judgment_category=technical_architecture`, `display_depth=tradeoff` packet을 사용합니다.
Refs: 승인 범위 ref, 쓰기 준비 후보 ref, 의존성 비교 ref, 영향을 받는 파일 ref가 있으면 그 ref.
```

**Rendered example: full architecture trade-off**

```text
결정: 로그인 세션 구조
표시 깊이: tradeoff
경로/영역: choose, 기술 구조(`technical_architecture`)
질문: 이 로그인 작업은 어떤 세션 모델을 사용해야 하나요?
선택지: 서버 측 세션 쿠키, 클라이언트 보관 Bearer/JWT, 로컬 세션 전략을 포함한 OAuth/OIDC 제공자, 소셜 로그인 제공자 연동.
추천: 자사 웹 앱이면 현재 요구사항이 외부 ID, 브라우저 밖 클라이언트, 소셜 로그인을 지금 요구하지 않는 한 서버 측 세션 쿠키를 선택합니다.
불확실성: 기존 세션 미들웨어, 폐기 요구사항, SSO 요구사항, CSRF 자세, 이전 제약.
미룰 때의 결과: 읽기 전용 조사와 UI 뼈대 작업은 저장 방식, 토큰 수명, 제공자, 미들웨어 동작을 확정하지 않는 범위에서만 계속할 수 있습니다.
Refs: 인증 모델 ref, 영향을 받는 수용 기준, 사용할 수 있는 보안 근거 ref, 잔여 위험 또는 이전 ref.
```

## 전체 템플릿

````md
---
doc_type: decision_packet
projection_kind: DEC
projection_id: DEC-PROJ-0001
decision_packet_id: DEC-0001
task_id: TASK-0001
change_unit_id: CU-01
judgment_category: product_ux
judgment_route: choose
display_depth: tradeoff
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
- source refs: decision={decision_packet_id}; write={write_authorization_ref|none}; approval={approval_refs|none}; evidence={evidence_manifest_ref|none}; eval={eval_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={acceptance_context_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}

## Judgment Category, Route, And Display Depth
- judgment_category: product_ux | technical_architecture | security_privacy | scope_autonomy | qa_verification | work_acceptance | residual_risk | mixed
- judgment_route: choose | defer | approve-sensitive-action | waive | accept-result | accept-risk | reconcile
- display_depth: simple | tradeoff | high-risk | close-affecting
- display type: 제품/UX 판단 | 기술 구조 판단 | 보안/개인정보 판단 | 범위/자율성 판단 | 민감 동작 승인 | QA/verification waiver | 작업 수락 | 잔여 위험 수용 | reconcile
- route-specific detail: 공통 field와 `judgment_route`로 validate되는 `judgment_payload` branch; simple prompt는 중요하지 않은 detailed field를 생략하거나 schema가 null을 허용하는 곳에서 null로 렌더링할 수 있음
- final recorded answer: `judgment_route`가 user-judgment route와 `RecordUserJudgmentPayload` value rule을 고름
- display label:
- route verb: choose | defer | reject | approve | waive | accept result | accept named risk | reconcile
- 이 route가 기록할 수 있는 것:
- 이 route가 기록할 수 없는 것:
- generic consent handling:

## Approval-Shaped Context, If Applicable
- card label: 민감 동작 승인
- `judgment_route=approve-sensitive-action` 범위:
- linked approval record:
- sensitive categories:
- 이 Approval이 포괄하는 것:
- 이 Approval이 포괄하지 않는 것:
- 이렇게 렌더링하면 안 되는 것: 작업 수락 또는 잔여 위험 수용
- separate Decision Packet이 필요한 사용자 소유 판단:
- Approval 경계:
- Write Authorization 경계:
- secret 노출 경계:

## What User Is Judging
- judgment type:
- judgment_category:
- display label:
- judgment_route:
- display_depth:
- user-facing question:
- decision:
- 이 decision이 확정하는 것:
- 이 decision이 확정하지 않는 것:
- broad approval이 충분하지 않은 이유:

## What Agent May Decide Without User
- implementation detail:
- code organization inside granted scope:
- 근거 수집:
- follow-up proposal:

## Autonomy Boundary Impact, If Any
- current boundary impact:
- proposed boundary update:
- user judgment required:

## Affected Scope And Boundaries
- 범위 안:
- 범위 밖:
- 영향을 받는 product area:
- 영향을 받는 screen 또는 flow:
- 영향을 받는 module/interface/path:
- 영향을 받는 수용 기준:
- 영향을 받는 gate:
- sensitive categories:

## Options
### Option A
- choice:
- trade-offs:
- benefits:
- costs:
- risks:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high
- evidence refs:

### Option B
- choice:
- trade-offs:
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
- 결정을 미뤄도 계속할 수 있는 일:
- 판단 전에는 멈춰야 하는 일:
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

## User Judgment
- status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
- selected option:
- user judgment:
- decision note:
- broad approval handling:
- decided by:
- decided at:

## Residual-Risk Acceptance, If Applicable
- 수용하는 named risk:
- residual-risk visibility status:
- accepted residual-risk summary:
- accepted residual-risk refs:
- accepted consequence:
- 잔여 위험 수용이 대체하지 않는 것:

## Follow-Up
- [ ]

## References
- TASK:
- Change Unit:
- Write Authorization:
- DESIGN:
- APR:
- EVIDENCE-MANIFEST:
- EVAL:
- MANUAL-QA:
- Acceptance context:
- Residual Risk:
- artifacts:
- redaction state:
- projection freshness:
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. Active stage/profile이 요구하는 Decision Packet visibility는 compact status card, status/next response, judgment-context resource, decision-packet resource, 또는 dedicated prompt를 통해 제공될 수 있습니다. Later continuity profile이 active이면 `TASK`도 이를 보여줄 수 있습니다. Standalone `DEC` projection은 optional입니다.

Decision Packet projection은 authority context ref를 간결하고 명시적으로 유지해야 합니다. 이 template에 Write Authorization, Approval, Evidence Manifest, Eval, 수동 QA, 작업 수락, 잔여 위험, artifact, redaction, freshness ref를 표시하더라도 packet prose가 그 기록의 authority가 되지는 않습니다.

Decision Packet card는 한 번에 하나의 decision type을 표시해야 합니다. Approval card는 민감 동작 승인 언어를 쓰고, 작업 수락 prompt는 작업 수락 언어를 쓰며, 잔여 위험 수용 prompt는 수용하는 구체적 위험을 이름 붙입니다.

Option subsection은 필요한 만큼 반복할 수 있습니다. 어떤 제품 선택은 현실적인 선택지가 두 개보다 많습니다.
