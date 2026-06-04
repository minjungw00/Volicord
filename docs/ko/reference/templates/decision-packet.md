# DEC Template

## 사용 시점

특정 `user_judgment` record에 대해 standalone full-format Decision Packet presentation이 켜져 있을 때만 `DEC`를 사용합니다. 일반 MVP-1 경로는 status, next-action, user-judgment resource를 통한 compact 판단 요청입니다. 작은 unblocker는 한 화면에 들어가야 하며, 사용자가 drill-down을 요청하지 않는 한 이 full template을 노출하지 않습니다.

지원하는 user-facing display label은 다음 다섯 가지뿐입니다.

- 제품/UX 판단
- 기술 판단
- 민감 동작 승인
- 작업 수락
- 잔여 위험 수용

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: optional full-format judgment presentation입니다. Standalone persisted `DEC` Markdown projection은 standalone Decision Packet projection 기능이 켜진 경우에만 optional로 사용합니다. "Decision Packet"은 presentation label이고, `user_judgment`가 canonical record family입니다.

## 기준 기록

- `state.sqlite.user_judgments`
- 관련 Task와 Change Unit refs
- `judgment_type`, `presentation`, `display_label`
- 관련 `decision_gate` state와 user-judgment events
- `judgment_type=sensitive_action_approval`의 `approval_scope`, 그리고 later Approval profile이 active일 때만 Approval records
- later profile이 active일 때 관련 reconcile records
- residual risk refs
- minimum MVP-1의 evidence summaries, Run refs, ArtifactRefs, visible evidence gaps
- related authority context로 표시될 때 Write Authorization, sensitive-action permission, Eval, Manual QA, work-acceptance context, residual-risk refs, ArtifactRefs, redaction state, projection freshness
- affected scope 표시 input: product areas, screens/flows, modules, interfaces, paths, acceptance criteria, gates, sensitive categories

`decision_packet_id`, `judgment_category`, `judgment_route`, `display_depth` 같은 legacy 이름은 migration note 또는 compatibility drill-down에서만 나타날 수 있습니다. 새 template, example, fixture는 `user_judgment_id`, `judgment_type`, `presentation`, `display_label`, `record_kind=user_judgment`를 사용해야 합니다.

민감 동작 승인 display의 "포괄하는 것", "포괄하지 않는 것", "secret 노출 경계"는 `judgment_payload.approval_scope`, 관련 `user_judgment` ref, later profile이 active일 때만의 linked Approval record, 현재 write/close context에서 파생한 표시용 요약입니다. 경계만 설명하며 별도 사용자 판단을 확정하거나 Write Authorization을 만들거나 minimum MVP-1에서 committed Approval record를 암시하지 않습니다.

Resolved user judgment가 민감 동작 permission을 부여하는 경우는 `judgment_type=sensitive_action_approval`이고 compatible `approval_scope`를 가진 경우뿐입니다. 다른 user judgment resolution은 제품/UX 판단, 기술 판단, 작업 수락, 잔여 위험 수용, later-profile waiver/reconcile choice를 확정할 수 있지만 민감 동작 permission을 부여하지 않습니다.

`presentation=short`는 simple unblocker와 compact prompt의 기본값입니다. `presentation=full`은 복잡하거나 high-risk이거나 close-affecting이거나 reconcile/later-profile 판단을 위한 full-format Decision Packet-style presentation입니다. Presentation은 렌더링되는 context 양만 바꾸며 authority를 바꾸지 않습니다.

## 렌더링 섹션

- Why Now
- Current State
- Judgment Type And Presentation
- Sensitive Action Approval Context, If Applicable
- What User Is Judging
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

충분한 rendered Decision Packet은 하나의 사용자 소유 판단에 답합니다. 넓은 permission을 묻지 않습니다. 정확한 public request/response field는 [`harness.request_user_judgment`](../api/mvp-api.md#harnessrequest_user_judgment)가 소유하고, canonical authority rule은 [User Judgment](../kernel.md#user-judgment)와 [Decision Gate](../kernel.md#decision-gate)가 소유합니다. 이 template은 existing user judgment fields를 요약할 수 있지만 schema field, gate, alternate authority를 추가하면 안 됩니다.

사용자에게 보이는 질문은 판단을 직접 물어야 합니다. Option 선택, 정해진 결과가 있는 defer, path reject, 민감 동작 승인 grant/deny, 결과 accept/reject, 이름 붙은 잔여 위험 accept/reject, later-profile waiver/reconcile outcome처럼 기록될 값을 분명히 말합니다. "approve"나 "승인"은 민감 동작 승인 또는 later Approval record에만 씁니다. 여러 판단이 pending이면 별도 prompt 또는 별도 줄로 렌더링합니다. 민감 동작 승인, 작업 수락, 잔여 위험 수용을 하나의 답변으로 합치면 안 됩니다.

## 예시 단서

- Tiny unblocker(`judgment_type=product_choice`, `presentation=short`): 이미 scoped된 settings copy change 안에서 button label을 "Save"로 할지 "Update"로 할지 고릅니다. 간결한 choice, scope, refs, non-effects를 보여주고 full architecture layout을 강제하지 않습니다.
- 제품/UX 판단(`judgment_type=product_choice`): failed-login feedback을 inline, toast, modal 중에서 고르거나 wording tone을 정합니다.
- 기술 판단(`judgment_type=technical_choice`): session cookie, bearer/JWT token, OAuth/OIDC provider, social-login integration, dependency adoption, schema migration, public API direction, privacy/logging policy, QA/verification expectation, waiver, scope/autonomy expansion을 다룹니다.
- 민감 동작 승인(`judgment_type=sensitive_action_approval`): dependency install, secret access, network write, destructive write 같은 scoped sensitive step입니다. Approval boundary를 보여주되 제품/UX 판단이나 기술 판단을 해소한 것으로 취급하지 않습니다.
- 작업 수락(`judgment_type=work_acceptance`): final result, evidence status, Manual QA와 verification status, close-relevant residual-risk visibility를 보여줍니다. 새 sensitive action, 추가 write, deployment, merge, 잔여 위험 수용으로 취급하지 않습니다.
- 잔여 위험 수용(`judgment_type=residual_risk_acceptance`): visible limitation, evidence, risk refs, remaining follow-up을 보여줍니다.
- Broad "go ahead" answers: prompt가 이 특정 judgment type과 option을 묻는 이유를 보여줍니다. Generic consent phrase는 이 prompt가 그 정확한 judgment를 기록하는 경우가 아니면 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용을 해소하지 않습니다.

**Rendered example: minimal judgment**

```text
판단 요청: Settings label wording
Record: user_judgment_id=UJ-0001
Judgment type: product_choice
Presentation: short
Display label: 제품/UX 판단
Question: 이 scoped settings label을 "Save"로 할까요, "Update"로 할까요?
Scope/refs: settings form copy in CU-04; source ref TASK-012/CU-04; 민감 동작 또는 close-risk ref 없음.
Choice to record: Save | Update
Does not settle: broader settings flow behavior, localization strategy, 작업 수락, 잔여 위험 수용, write authority.
```

**Rendered example: sensitive action approval**

```text
판단 요청: Dependency install approval
Record: user_judgment_id=UJ-0002
Judgment type: sensitive_action_approval
Presentation: short
Display label: 민감 동작 승인
Question: 이 Task에 대해 이름 붙은 dependency install/update 동작을 허가하시겠습니까?
Approval scope: named install command 또는 dependency-file update; named manifest/lockfile paths; current task and approval window only.
Covers: scoped sensitive action.
Does not cover: dependency가 올바른 architecture direction인지, future installs, unrelated product writes, QA/verification waiver, 작업 수락, 잔여 위험 수용.
Separate judgments required: dependency choice itself가 사용자 소유 기술 판단이면 `judgment_type=technical_choice`를 사용합니다.
```

**Rendered example: full technical trade-off**

```text
판단 요청: Login session architecture
Record: user_judgment_id=UJ-0003
Judgment type: technical_choice
Presentation: full
Display label: 기술 판단
Question: 이 login 작업은 어떤 session model을 써야 합니까?
Options: server-side session cookie; client-held bearer/JWT; OAuth/OIDC provider plus local session strategy; social-login provider integration.
Recommendation: first-party web app이면 current requirements가 third-party identity, non-browser clients, social sign-in을 요구하지 않는 한 server-side session cookie.
Uncertainty: existing session middleware, revocation requirements, SSO requirement, CSRF posture, migration constraints.
Deferral consequence: storage, token lifetime, provider, middleware behavior를 확정하지 않는 read-only inspection과 UI scaffolding만 계속할 수 있습니다.
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

# UJ-0001 Judgment Request Title

> Projection view: `source_state_version`와 `updated_at` 기준으로 렌더링되며 state의 `user_judgment_id`와 관련 refs를 표시합니다. 이 Markdown을 편집해도 judgment는 해소되지 않습니다. 답변은 `harness.record_user_judgment`를 통해 기록합니다.

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
- source refs: judgment={user_judgment_id}; write={write_authorization_ref|none}; sensitive_action_permission={user_judgment_ref|approval_ref_when_profile_active|none}; evidence={evidence_summary_ref|evidence_manifest_ref_when_profile_active|none}; eval={eval_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={work_acceptance_user_judgment_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}

## Judgment Type And Presentation
- judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
- presentation: short | full
- display_label: 제품/UX 판단 | 기술 판단 | 민감 동작 승인 | 작업 수락 | 잔여 위험 수용
- final recorded answer:
- what this judgment can record:
- what this judgment cannot record:
- generic consent handling:

## Sensitive Action Approval Context, If Applicable
- card label: 민감 동작 승인
- judgment_type=sensitive_action_approval scope:
- linked approval record (later profile only):
- sensitive categories:
- what this approval covers:
- what this approval does not cover:
- must not be rendered as: 작업 수락 또는 잔여 위험 수용
- separate user-owned judgment still required:
- approval boundary:
- write authorization boundary:
- secret exposure boundary:

## What User Is Judging
- judgment type:
- display label:
- user-facing question:
- decision:
- what this decision settles:
- what this decision does not settle:
- why broad approval is insufficient:

## User Judgment
- selected option:
- value: selected | rejected | deferred | granted | denied | expired | accepted
- note:
- decided by:
- decided at:
- broad consent check: "proceed", "go ahead", "looks good", "좋아", "진행해"는 자동으로 민감 동작 승인, 작업 수락, 잔여 위험 수용이 되지 않습니다.

## References
- task:
- change unit:
- user judgment:
- write authority:
- evidence:
- verification:
- Manual QA:
- residual risk:
- artifacts:
- projection freshness:
````

## 메모

이 template은 rendered shape이지 canonical state가 아닙니다. Active stage/profile이 요구하는 user judgment visibility는 compact status card, status/next response, judgment-context resource, user-judgment resource, dedicated prompt를 통해 제공될 수 있습니다. Standalone `DEC` projection은 optional입니다.

Decision Packet projection은 authority context refs를 간결하고 명시적으로 유지해야 합니다. 이 template에 Write Authorization, 민감 동작 permission ref, evidence summary, 해당 profile이 active일 때의 Evidence Manifest, Eval, Manual QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용, artifact, redaction, freshness ref를 표시하더라도 prose가 그 record의 authority가 되지는 않습니다.

Decision Packet card는 한 번에 하나의 judgment type만 표시해야 합니다. 민감 동작 승인 prompt는 승인 언어를 쓰고, 작업 수락 prompt는 작업 수락 언어를 쓰며, 잔여 위험 수용 prompt는 수용하는 구체적 위험을 이름 붙입니다.
