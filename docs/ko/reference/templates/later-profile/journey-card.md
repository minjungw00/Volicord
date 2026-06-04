# JOURNEY-CARD 템플릿

## 사용 시점

작업의 현재 위치, 범위와 범위 밖, 다음 움직임을 막는 것, 대기 중인 사용자 판단, 자율성 경계(Autonomy Boundary), 쓰기 권한 요약(Write Authority Summary), 수용 기준, 근거와 확인, 잔여 위험, 닫기 맥락(close context), gate, 읽기용 보기 최신성을 현재 위치 카드로 보여줄 때 `JOURNEY-CARD`를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 projections입니다. Persisted Journey Card Markdown과 Journey Spine-style output은 초기 필수 범위가 아니며, 다섯 가지 작은 MVP 보기가 초기 현재 위치 맥락을 담당합니다.

## 기준 기록

- 현재 Task 상태와 gate
- 범위와 범위 밖 summary
- active Change Unit
- Autonomy Boundary summary(자율성 경계 요약)
- 현재 수용 기준 snapshot
- Write Authorization, 민감 동작 승인, baseline, 보장 수준 참조
- 민감 동작 승인 status
- active user judgment refs
- 가장 먼저 해소할 막힘, 추가 막힘, 가장 작은 해소 방법 표시 summary
- blocker owner 표시 summary
- evidence coverage, verification, 수동 QA summary
- residual-risk summary와 참조
- 작업 수락, 잔여 위험 수용, close-reason summary
- 최신 evidence, Eval, 수동 QA, 보고서 참조
- 읽기용 보기 최신성(projection freshness) 입력
- state, baseline, evidence, MCP, capability freshness/blocker 표시 summary

Judgment, write-authority, close-impact, residual-risk, freshness placeholder는 위 기록에서 파생한 표시 binding입니다. 실제 사용자 judgment가 필요하면 이 card를 사용자 판단 맥락 source로 취급하지 말고 compact 판단 요청 또는 선택적 full-format Decision Packet presentation을 렌더링합니다.

## 렌더링 섹션

- 현재 위치와 다음 행동
- 범위와 범위 밖
- 수용 기준
- 현재 막는 것
- 판단 맥락
- Autonomy Boundary(자율성 경계)
- Write Authority Summary(쓰기 권한 요약)
- 근거와 확인
- 잔여 위험
- 닫기 맥락
- Gates(관문)
- 읽기용 보기 최신성
- state/input 최신성과 capability 사용 가능 여부

## 전체 템플릿

````text
TASK-{id} {title}
표시 전용: 현재 위치를 보여주는 읽기용 보기이며 기준 상태나 쓰기 허가 기록이 아닙니다.
현재 위치: {mode} / {lifecycle_phase} / {current_position}
범위: {scope_summary|none}
범위 밖: {out_of_bounds_summary|none}
수용 기준: {acceptance_criteria_summary|none}
다음 행동: {next_action}

현재 막는 것:
- 가장 먼저 해소할 막힘: {primary_blocker_label|none}
- 소유자: {primary_blocker_owner_label|none}
- 가장 작은 해소 방법: {smallest_unblocker|none}
- 추가로 막는 것: {secondary_blockers_summary|none}

판단 맥락:
- 대기 중인 판단: {user_judgment_ref|none}
- 사용자가 판단할 것: {user_judgment_question|none}
- 에이전트가 판단해도 되는 것: {what_agent_may_decide_without_user}

Autonomy Boundary(자율성 경계):
- profile: {autonomy_profile}
- agent가 할 수 있는 일: {agent_may_do}
- 필요한 사용자 판단: {user_judgment_required}
- AFK 중단 조건: {afk_stop_conditions}

Write Authority Summary(쓰기 권한 요약):
- active Change Unit: {active_change_unit_ref|none}
- Write Authorization: {write_authorization_ref|none}
- 허용 path: {allowed_paths}
- 허용 tool: {allowed_tools}
- 허용 command: {allowed_commands}
- 허용 network target: {allowed_network_targets}
- secret scope: {secret_scope}
- 민감 category: {sensitive_categories}
- 민감 동작 승인 상태: {approval_status}
- baseline: {baseline_ref|none}
- 보장 수준: {guarantee_display}
- note: Autonomy Boundary는 판단 재량이지 쓰기 전 범위 확인이나 쓰기 허가 기록이 아니다.

근거와 확인:
- 행동: {next_evidence_action}
- 필요한 이유: {evidence_needed_for}
- 최신 근거: {latest_evidence_ref|none}
- 검증: {verification_summary|none}
- self-check vs detached boundary: {self_check_or_detached_boundary|none}
- 수동 QA: {manual_qa_summary|not_required}
- 생략/차단된 근거 영향: {redaction_availability_summary|none}

잔여 위험:
- 상태: {residual_risk_status}
- 닫기 영향: {residual_risk_close_impact}
- 받아들인 residual-risk record refs: {accepted_residual_risk_record_refs|none}

닫기 맥락:
- 닫기 막힘: {close_blockers|none}
- 작업 수락: {acceptance_summary|not_required}
- 잔여 위험 수용: {accepted_residual_risk_record_refs|none}
- close reason: {close_reason|none}

Gates(관문):
- scope: {scope_gate}
- decision: {decision_gate}
- approval: {approval_gate}
- evidence: {evidence_gate}
- verification: {verification_gate}
- 수동 QA: {qa_gate display: not_required|required|pending|passed|failed|waived}
- acceptance: {acceptance_gate}

읽기용 보기 최신성(projection freshness): {projection_freshness}; source_state_version={source_state_version|unknown} (읽기용 보기의 최신성, Task result 아님)
상태/입력 최신성: {state_baseline_evidence_freshness_summary|current or none}
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. Current source record와 ref에서 렌더링되며, 오래된 chat memory에서 렌더링하지 않습니다. 저장된 `JOURNEY-CARD` Markdown은 Future/diagnostic projections 범위입니다. `status`, `next`, 중요한 이어가기(resume) 흐름의 초기 현재 위치 맥락은 간결한 상태 출력을 사용할 수 있습니다.

이 card 안이나 주변에 표시되는 status/next recommendation은 읽기 전용 안내입니다. 사용자 판단 요청, 선택적 full-format Decision Packet presentation, `prepare_write`, evidence collection, verification, QA, reconcile, close attempt를 가리킬 수는 있지만, state를 mutate하거나, write를 허가하거나, gate를 충족하거나, 작업 수락을 기록하거나, 잔여 위험을 받아들이거나, Task를 close하지 않습니다.

Journey Card의 닫기 맥락(Close context)은 간결한 status/resume 표시입니다. `TASK`는 진행 중이거나 최근 닫힌 `work` Task의 이어가기용 닫기 요약(Close Summary)을 담당하고, `DIRECT-RESULT`는 direct 작업의 가벼운 close impact summary를 담당합니다. 이 표시들은 [projection/report 경계](../../projection-and-templates.md#projection-principles)를 따르며, close와 gate effect는 여전히 owner record에서 옵니다.

Blocker 줄은 API와 state record를 사용자에게 보이는 상태로 바꿔 보여줍니다. 가장 먼저 해소할 막힘은 다음 행동이 먼저 해소해야 하는 blocker여야 하며, 소유자 라벨은 다음 움직임이 사용자 소유인지, 에이전트가 해소 가능한지, 접점/시스템 소유인지 분명히 해야 합니다. 가장 먼저 해소할 막힘이 없으면 소유자는 `none`으로 렌더링하거나 생략할 수 있습니다. 추가 막힘은 후속 경로에 영향을 줄 때만 계속 보여줍니다. Raw `ErrorCode` 값만으로 설명을 끝내면 안 됩니다.

최신 근거 또는 다음 근거에 `secret_omitted`나 `blocked` artifact ref가 포함되면 이 card는 사용 가능성 영향만 표시해야 합니다. 생략된 값 또는 차단된 원본 payload 내용을 포함하면 안 됩니다.
