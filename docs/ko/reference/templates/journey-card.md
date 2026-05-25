# JOURNEY-CARD Template

## 사용 시점

작업의 현재 위치, 대기 중인 판단, Autonomy Boundary, Write Authority Summary, 다음 근거, 잔여 위험, gate, projection 최신성을 현재 위치 카드로 보여줄 때 `JOURNEY-CARD`를 사용합니다.

## 기준 기록

- 현재 Task 상태와 gate
- active Change Unit
- Autonomy Boundary summary
- Write Authorization, approval, baseline, guarantee 참조
- active Decision Packet 참조
- residual-risk summary와 참조
- 최신 evidence, Eval, Manual QA, 보고서 참조
- projection 최신성 입력

Judgment, write-authority, close-impact, residual-risk, freshness placeholder는 위 기록에서 파생한 표시 binding입니다. 실제 사용자 decision이 필요하면 이 card를 judgment-context source로 취급하지 말고 Decision Packet 또는 decision prompt를 렌더링합니다.

## 렌더링 섹션

- 현재 위치와 next action
- 판단 context
- Autonomy boundary
- Write Authority Summary
- 다음 근거
- 잔여 위험
- Gates
- projection 최신성

## 전체 템플릿

````text
TASK-{id} {title}
표시 전용: 현재 위치를 보여주는 읽기용 보기이며 기준 상태나 쓰기 권한이 아닙니다.
현재 위치: {mode} / {lifecycle_phase} / {current_position}
다음 action: {next_action}

판단 context:
- pending decision: {decision_packet_ref|none}
- user deciding: {what_user_is_deciding|none}
- agent may decide: {what_agent_may_decide_without_user}

Autonomy Boundary:
- profile: {autonomy_profile}
- agent may do: {agent_may_do}
- user judgment required: {user_judgment_required}
- AFK stop conditions: {afk_stop_conditions}

Write Authority Summary:
- active Change Unit: {active_change_unit_ref|none}
- write authorization: {write_authorization_ref|none}
- allowed paths: {allowed_paths}
- allowed tools: {allowed_tools}
- allowed commands: {allowed_commands}
- allowed network targets: {allowed_network_targets}
- secret scope: {secret_scope}
- sensitive categories: {sensitive_categories}
- approval status: {approval_status}
- baseline: {baseline_ref|none}
- guarantee: {guarantee_display}
- note: Autonomy Boundary는 판단 재량이지 쓰기 권한이 아니다.

Next evidence:
- action: {next_evidence_action}
- needed for: {evidence_needed_for}
- latest evidence: {latest_evidence_ref|none}
- 생략/차단된 근거 영향: {redaction_availability_summary|none}

Residual risk:
- status: {residual_risk_status}
- 닫기 영향: {residual_risk_close_impact}
- accepted residual-risk record refs: {accepted_residual_risk_record_refs|none}

Gates:
- scope: {scope_gate}
- decision: {decision_gate}
- approval: {approval_gate}
- evidence: {evidence_gate}
- verification: {verification_gate}
- Manual QA: {qa_gate display: pending|passed|failed|waived|not_required}
- acceptance: {acceptance_gate}

Projection freshness: {projection_freshness}; source_state_version={source_state_version|unknown} (읽기용 보기의 최신성, Task result 아님)
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. Persisted `JOURNEY-CARD` Markdown은 선택 사항입니다. `status`, `next`, 중요한 resume flow에서 보여주는 현재 위치 Journey Card output도 읽기/표시용 접점입니다.

Latest 또는 next evidence에 `secret_omitted`나 `blocked` artifact ref가 포함되면 이 card는 사용 가능성 영향만 표시해야 합니다. 생략된 값 또는 차단된 원본 payload 내용을 포함하면 안 됩니다.
