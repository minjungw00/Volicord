# DEC 템플릿

## 사용 시점

특정 `user_judgment` record에 대해 standalone full-format Decision Packet presentation이 켜져 있을 때만 `DEC`를 사용합니다. 일반 MVP-1 경로는 status, next-action, user-judgment resource를 통한 간결한 판단 요청입니다. 작은 unblocker는 한 화면에 들어가야 하며, 사용자가 drill-down을 요청하지 않는 한 이 full template을 노출하지 않습니다.

지원하는 user-facing display label은 다음 다섯 가지뿐입니다.

- 제품/UX 판단
- 기술 판단
- 민감 동작 승인
- 작업 수락
- 잔여 위험 수용

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 선택적 full-format 판단 presentation입니다. Standalone persisted `DEC` Markdown projection은 standalone Decision Packet projection 기능이 켜진 경우에만 선택적으로 사용합니다. "Decision Packet"은 presentation label이고, `user_judgment`가 canonical record family입니다.

## 기준 기록

- `state.sqlite.user_judgments`
- 관련 Task와 Change Unit refs
- `judgment_type`, `presentation`, `display_label`
- 관련 `decision_gate` state와 user-judgment events
- `judgment_type=sensitive_action_approval`의 `approval_scope`, 그리고 later Approval profile이 active일 때만 Approval records
- later profile이 active일 때 관련 reconcile records
- residual risk refs
- minimum MVP-1의 evidence summaries, Run refs, ArtifactRefs, visible evidence gaps; full Evidence Manifest profile이 active일 때만 Evidence Manifest refs
- 관련 권한 맥락으로 표시될 때 쓰기 허가 기록(Write Authorization), 민감 동작 permission, Eval, Manual QA, 작업 수락 context, residual-risk refs, ArtifactRefs, redaction state, 읽기용 보기 최신성(projection freshness)
- 영향받는 범위 표시 input: product areas, screens/flows, modules, interfaces, paths, acceptance criteria, gates, sensitive categories
- 읽기용 보기 최신성(projection freshness) inputs

`decision_packet_id`, `judgment_category`, `judgment_route`, `display_depth` 같은 legacy 이름은 migration note 또는 compatibility drill-down에서만 나타날 수 있습니다. 새 template, example, fixture는 `user_judgment_id`, `judgment_type`, `presentation`, `display_label`, `record_kind=user_judgment`를 사용해야 합니다.

민감 동작 승인 display의 "포괄하는 것", "포괄하지 않는 것", "secret 노출 경계"는 `judgment_payload.approval_scope`, 관련 `user_judgment` ref, later profile이 active일 때만의 linked Approval record, 현재 write/close context에서 파생한 표시용 요약입니다. 경계만 설명하며 별도 사용자 판단을 확정하거나 Write Authorization을 만들거나 minimum MVP-1에서 committed Approval record를 암시하지 않습니다. 민감 동작 승인 display는 작업 수락이나 잔여 위험 수용처럼 보여서는 안 됩니다.

Resolved user judgment가 민감 동작 permission을 부여하는 경우는 `judgment_type=sensitive_action_approval`이고 compatible `approval_scope`를 가진 경우뿐입니다. 다른 user judgment resolution은 제품/UX 판단, 기술 판단, 작업 수락, 잔여 위험 수용, later-profile waiver/reconcile choice를 확정할 수 있지만 민감 동작 permission을 부여하지 않습니다.

`presentation=short`는 simple unblocker와 compact prompt의 기본값입니다. `presentation=full`은 복잡하거나 high-risk이거나 close-affecting이거나 reconcile/later-profile 판단을 위한 full-format Decision Packet-style presentation입니다. Presentation은 렌더링되는 context 양만 바꾸며 authority를 바꾸지 않습니다.

## 렌더링 섹션

- 지금 필요한 이유
- 현재 상태
- 판단 유형과 표시 형식
- 해당되는 경우 민감 동작 승인 맥락
- 사용자가 판단하는 것
- 에이전트가 사용자 없이 결정해도 되는 것
- 해당되는 경우 자율성 경계 영향
- 영향받는 범위와 경계
- 선택지
- 추천
- 판단을 미룰 때의 영향
- 판단에 필요한 최소 맥락
- 사용자 판단
- 해당되는 경우 잔여 위험 수용
- 후속 조치
- 참조

충분한 렌더링 Decision Packet은 하나의 사용자 소유 판단에 답합니다. 넓은 permission을 묻지 않습니다. 정확한 public request/response field는 [`harness.request_user_judgment`](../../api/mvp-api.md#harnessrequest_user_judgment)가 소유하고, canonical authority rule은 [사용자 판단(User Judgment)](../../core-model.md#user-judgment)와 [Decision Gate](../../core-model.md#decision-gate)가 소유합니다. 이 template은 existing user judgment fields를 요약할 수 있지만 schema field, gate, alternate authority를 추가하면 안 됩니다.

사용자에게 보이는 질문은 판단을 직접 물어야 합니다. 선택지를 고를지, 명시된 결과를 감수하고 defer할지, 해당 path를 reject할지, 민감 동작 승인을 grant/deny할지, 결과를 accept/reject할지, 이름 붙은 잔여 위험을 accept/reject할지, later-profile waiver/reconcile outcome을 기록할지처럼 기록될 값을 분명히 말합니다. "approve"나 "승인"은 민감 동작 승인 또는 later Approval record에만 씁니다. 여러 판단이 pending이면 별도 prompt 또는 별도 줄로 렌더링합니다. 민감 동작 승인, 작업 수락, 잔여 위험 수용을 하나의 답변으로 합치면 안 됩니다.

**예시 단서:**

아래의 일반적인 full-format user judgment 형태에는 같은 rendered section을 사용합니다. 이 단서들은 추가 template section이 아닙니다.

- Tiny unblocker(`judgment_type=product_choice`, `presentation=short`): 이미 범위가 정해진 settings copy change 안에서 button label을 "Save"로 할지 "Update"로 할지 고릅니다. 간결한 선택, 범위, refs, non-effects를 `사용자가 판단하는 것`과 `참조`에 둡니다. Full architecture-tradeoff layout을 강제하지 않습니다.
- 제품/UX 판단(`judgment_type=product_choice`): failed-login feedback을 inline layer, toast, modal 중에서 고르거나 failed-login wording을 generic, specific, hybrid 중에서 정합니다. Flow, interruption, accessibility, copy, product tone, user-risk 차이는 `선택지`와 `추천`에 둡니다.
- 기술 판단(`judgment_type=technical_choice`): session cookie, bearer/JWT token, OAuth/OIDC provider, social-login provider integration 중에서 session model을 고릅니다. Revocation, CSRF/XSS exposure, client compatibility, implementation cost, identity-provider boundary, migration impact는 `선택지`와 `판단에 필요한 최소 맥락`에 둡니다.
- 기술 판단(`judgment_type=technical_choice`): dependency adoption, schema/data-model migration, public API/interface direction, module boundary change, privacy/logging policy, QA expectation, verification expectation, waiver, scope/autonomy expansion, later profile이 active일 때의 reconcile choice를 다룹니다.
- 민감 동작 승인(`judgment_type=sensitive_action_approval`): dependency install, secret access, network write, destructive write 또는 다른 scoped sensitive step입니다. Approval boundary는 `민감 동작 승인 맥락`에 두고, 제품/UX 판단이나 기술 판단을 해소한 것으로 취급하지 않습니다.
- 작업 수락(`judgment_type=work_acceptance`): final result, evidence status, Manual QA와 verification status, close-relevant residual-risk visibility를 `현재 상태`와 `판단에 필요한 최소 맥락`에 둡니다. 작업 수락을 새 sensitive action, 추가 write, deployment, merge를 허가하거나 잔여 위험 수용을 대신하는 판단처럼 취급하지 않습니다.
- 잔여 위험 수용(`judgment_type=residual_risk_acceptance`): visible limitation, existing evidence, 사용자에게 수용 여부를 묻는 risk refs, remaining follow-up을 `현재 상태`, `판단에 필요한 최소 맥락`, `잔여 위험 수용`, `후속 조치`에 둡니다.
- Broad "go ahead" answers: prompt가 이 특정 judgment type과 option을 묻는 이유를 보여줍니다. Generic consent phrase는 이 prompt가 그 정확한 judgment를 기록하는 경우가 아니면 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용을 해소하지 않습니다.

**렌더링 예시: 최소 판단**

```text
판단 요청: Settings 라벨 문구
기록: user_judgment_id=UJ-0001
판단 유형: product_choice
표시 형식: short
표시 라벨: 제품/UX 판단
질문: 이 scoped settings label을 "Save"로 할까요, "Update"로 할까요?
범위/참조: settings form copy in CU-04; source ref TASK-012/CU-04; 민감 동작 또는 close-risk ref 없음.
기록할 선택: Save | Update
결정하지 않는 것: 더 넓은 settings 흐름 동작, localization 전략, 작업 수락, 잔여 위험 수용, 쓰기 전 범위 확인 / Write Authorization.
```

**렌더링 예시: 민감 동작 승인**

```text
판단 요청: dependency install 승인
기록: user_judgment_id=UJ-0002
판단 유형: sensitive_action_approval
표시 형식: short
표시 라벨: 민감 동작 승인
질문: 이 Task에 대해 이름 붙은 dependency install/update 동작을 허가하시겠습니까?
Approval 범위: 이름 붙은 install command 또는 dependency-file update, 이름 붙은 manifest/lockfile path, 현재 Task와 approval window만 포함.
포괄하는 것: scoped sensitive action.
포괄하지 않는 것: dependency가 올바른 architecture 방향인지, 향후 install, 관련 없는 제품 파일 쓰기, QA/verification waiver, 작업 수락, 잔여 위험 수용.
별도 판단 필요: dependency choice 자체가 사용자 소유 기술 판단이면 `judgment_type=technical_choice`를 사용합니다.
참조: approval scope refs, prepare-write candidate refs, dependency 비교 refs, 사용 가능한 경우 영향받는 file refs.
```

**렌더링 예시: 전체 기술 장단점 비교**

```text
판단 요청: Login session architecture
기록: user_judgment_id=UJ-0003
판단 유형: technical_choice
표시 형식: full
표시 라벨: 기술 판단
질문: 이 login 작업은 어떤 session model을 써야 합니까?
선택지: server-side session cookie, client-held bearer/JWT, OAuth/OIDC provider와 local session strategy, social-login provider integration.
추천: first-party web app이면 현재 요구사항이 third-party identity, non-browser client, social sign-in을 요구하지 않는 한 server-side session cookie.
불확실성: existing session middleware, revocation 요구사항, SSO 요구사항, CSRF posture, migration 제약.
미룰 때의 영향: storage, token lifetime, provider, middleware behavior를 확정하지 않는 read-only inspection과 UI scaffolding만 계속할 수 있습니다.
참조: auth model refs, 영향받는 수용 기준, 사용 가능한 경우 security evidence refs, residual-risk 또는 migration refs.
```

## 전체 템플릿

````md
---
doc_type: user_judgment_decision_packet
projection_kind: DEC
projection_id: DEC-PROJ-0001
user_judgment_id: UJ-0001
task_id: TASK-0001
change_unit_id: CU-01
judgment_type: product_choice
presentation: full
display_label: 제품/UX 판단
status: pending_user
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# UJ-0001 판단 요청 제목

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링되며 state의 `user_judgment_id`와 관련 refs를 표시합니다. 이 Markdown을 편집해도 judgment는 해소되지 않습니다. 답변은 `harness.record_user_judgment`를 통해 기록합니다.

## 지금 필요한 이유
- trigger:
- blocker:
- 영향받는 작업:
- 현재 상태에서 진행할 수 없는 이유:

## 현재 상태
- Task 상태:
- active Change Unit:
- 현재 gate:
- 최신 근거:
- residual risk:
- 출처 참조: judgment={user_judgment_id}; write={write_authorization_ref|none}; sensitive_action_permission={user_judgment_ref|approval_ref_when_profile_active|none}; evidence={evidence_ref|evidence_manifest_ref_when_profile_active|none}; eval={eval_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={work_acceptance_user_judgment_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}

## 판단 유형과 표시 형식
- judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
- presentation: short | full
- display_label: 제품/UX 판단 | 기술 판단 | 민감 동작 승인 | 작업 수락 | 잔여 위험 수용
- 최종 기록 답변:
- 이 판단이 기록할 수 있는 것:
- 이 판단이 기록할 수 없는 것:
- 일반 동의 표현 처리:

## 해당되는 경우 민감 동작 승인 맥락
- 카드 라벨: 민감 동작 승인
- judgment_type=sensitive_action_approval scope:
- 연결된 approval record(later profile only):
- 민감 category:
- 이 approval이 포괄하는 것:
- 이 approval이 포괄하지 않는 것:
- 렌더링하면 안 되는 형태: 작업 수락 또는 잔여 위험 수용
- 여전히 필요한 별도 사용자 소유 판단:
- approval 경계:
- write authorization 경계:
- secret 노출 경계:

## 사용자가 판단하는 것
- judgment type:
- display label:
- 사용자에게 보이는 질문:
- decision:
- 이 decision이 확정하는 것:
- 이 decision이 확정하지 않는 것:
- 넓은 approval이 부족한 이유:

## 에이전트가 사용자 없이 결정해도 되는 것
- 구현 세부사항:
- 허용된 범위 안의 code organization:
- 근거 수집:
- 후속 제안:

## 해당되는 경우 자율성 경계 영향
- 현재 boundary 영향:
- 제안된 boundary update:
- 필요한 user judgment:

## 영향받는 범위와 경계
- 범위 안:
- 범위 밖:
- 영향받는 product area:
- 영향받는 screen 또는 flow:
- 영향받는 module/interface/path:
- 영향받는 수용 기준:
- 영향받는 gate:
- 민감 category:

## 선택지
### 선택지 A
- 선택:
- 장단점:
- 이점:
- 비용:
- 위험:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high

### 선택지 B
- 선택:
- 장단점:
- 이점:
- 비용:
- 위험:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high

## 추천
- 추천 선택지:
- 근거:
- confidence:
- 추천이 바뀌는 조건:

## 판단을 미룰 때의 영향
- 계속할 수 있는 것:
- 계속 막히는 것:
- close 영향:

## 판단에 필요한 최소 맥락
- 보이는 evidence:
- 모르는 것:
- QA/verification 상태:
- 잔여 위험 표시 상태:
- close 또는 write 영향:

## 사용자 판단
- 선택한 option:
- value: selected | rejected | deferred | granted | denied | expired | accepted
- note:
- 결정한 사람:
- 결정 시각:
- 넓은 동의 표현 확인: "proceed", "go ahead", "looks good", "좋아", "진행해"는 자동으로 민감 동작 승인, 작업 수락, 잔여 위험 수용이 되지 않습니다.

## 해당되는 경우 잔여 위험 수용
- 이름 붙은 risk:
- 보이는 risk refs:
- 수용 범위:
- 받아들일 때의 영향:
- follow-up:

## 후속 조치
- write 전에 필요한 것:
- close 전에 필요한 것:
- 제안된 follow-up:

## 참조
- task:
- change unit:
- user judgment:
- write authority:
- evidence:
- verification:
- Manual QA:
- residual risk:
- artifacts:
- 보기 최신성:
````

## 메모

이 template은 rendered shape이지 canonical state가 아닙니다. Active stage/profile이 요구하는 user judgment visibility는 상태 카드, 판단 요청, status/next response, judgment-context resource, user-judgment resource를 통해 제공될 수 있습니다. Standalone `DEC` projection은 optional입니다.

Decision Packet projection은 authority context refs를 간결하고 명시적으로 유지해야 합니다. 이 template에 Write Authorization, 민감 동작 permission ref, evidence summary, 해당 profile이 active일 때의 Evidence Manifest, Eval, Manual QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용, artifact, redaction, freshness ref를 표시하더라도 prose가 그 record의 authority가 되지는 않습니다.

Decision Packet card는 한 번에 하나의 judgment type만 표시해야 합니다. 민감 동작 승인 prompt는 승인 언어를 쓰고, 작업 수락 prompt는 작업 수락 언어를 쓰며, 잔여 위험 수용 prompt는 수용하는 구체적 위험을 이름 붙입니다.
