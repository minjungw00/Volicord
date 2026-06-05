# JOURNEY-CARD 템플릿

## 사용 시점

작업의 현재 위치, 범위와 범위 밖, 다음 움직임을 막는 것, 대기 중인 사용자 판단, 자율성 경계(Autonomy Boundary), 쓰기 권한 요약, 수용 기준, 근거와 확인, 잔여 위험, 닫기 맥락, 관문, 읽기용 보기 최신성을 현재 위치 카드로 보여줄 때 `JOURNEY-CARD`를 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. 저장된 이어가기 카드(Journey Card) Markdown과 이어가기 축 스타일 출력은 초기 필수 범위가 아니며, 활성 MVP-1 작은 출력이 초기 현재 위치 맥락을 담당합니다.

## 기준 기록

- 현재 Task 상태와 관문
- 범위와 범위 밖 요약
- 활성 작업 조각(Change Unit)
- 자율성 경계(Autonomy Boundary) 요약
- 현재 수용 기준 스냅샷
- 쓰기 허가 기록(Write Authorization), 민감 동작 승인, 기준선, 보장 수준 참조
- 민감 동작 승인 상태
- 활성 사용자 판단 참조
- 가장 먼저 해소할 막힘, 추가 막힘, 가장 작은 해소 방법 표시 요약
- 막힘 소유자 표시 요약
- 근거 뒷받침 범위, 검증, 수동 QA 요약
- 잔여 위험 요약과 참조
- 최종 수락, 잔여 위험 수락, 닫기 이유 요약
- 최신 근거, Eval(분리 검증 결과), 수동 QA, 보고서 참조
- 읽기용 보기 최신성(projection freshness) 입력
- 상태, 기준선, 근거, MCP, 기능(capability) 최신성/막힘 표시 요약

판단, 쓰기 권한, 닫기 영향, 잔여 위험, 최신성 자리표시자는 위 기록에서 파생한 표시 바인딩입니다. 실제 사용자 판단이 필요하면 이 카드를 사용자 판단 맥락 출처로 취급하지 말고 간결한 판단 요청 또는 선택적 전체 형식 판단 패킷(Decision Packet) 표시를 렌더링합니다.

## 렌더링 섹션

- 현재 위치와 다음 행동
- 범위와 범위 밖
- 수용 기준
- 현재 막는 것
- 판단 맥락
- 자율성 경계(Autonomy Boundary)
- 쓰기 권한 요약
- 근거와 확인
- 잔여 위험
- 닫기 맥락
- 관문(Gates)
- 읽기용 보기 최신성
- 상태/입력 최신성과 기능(capability) 사용 가능 여부

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

자율성 경계(Autonomy Boundary):
- 프로필: {autonomy_profile}
- 에이전트가 할 수 있는 일: {agent_may_do}
- 필요한 사용자 판단: {user_judgment_required}
- AFK 중단 조건: {afk_stop_conditions}

쓰기 권한 요약:
- 활성 작업 조각(Change Unit): {active_change_unit_ref|none}
- 쓰기 허가 기록(Write Authorization): {write_authorization_ref|none}
- 허용 경로: {allowed_paths}
- 허용 도구: {allowed_tools}
- 허용 명령: {allowed_commands}
- 허용 네트워크 대상: {allowed_network_targets}
- 비밀 정보 범위: {secret_scope}
- 민감 범주: {sensitive_categories}
- 민감 동작 승인 상태: {approval_status}
- 기준선: {baseline_ref|none}
- 보장 수준: {guarantee_display}
- 메모: 자율성 경계(Autonomy Boundary)는 판단 재량이지 쓰기 전 범위 확인이나 쓰기 허가 기록이 아니다.

근거와 확인:
- 행동: {next_evidence_action}
- 필요한 이유: {evidence_needed_for}
- 최신 근거: {latest_evidence_ref|none}
- 검증: {verification_summary|none}
- 자체 확인과 분리 검증 경계: {self_check_or_detached_boundary|none}
- 수동 QA: {manual_qa_summary|not_required}
- 생략/차단된 근거 영향: {redaction_availability_summary|none}

잔여 위험:
- 상태: {residual_risk_status}
- 닫기 영향: {residual_risk_close_impact}
- 수락한 잔여 위험 기록 참조: {accepted_residual_risk_record_refs|none}

닫기 맥락:
- 닫기 막힘: {close_blockers|none}
- 최종 수락: {acceptance_summary|not_required}
- 잔여 위험 수락: {accepted_residual_risk_record_refs|none}
- 닫기 이유: {close_reason|none}

관문(Gates):
- 범위(scope): {scope_gate}
- 판단(decision): {decision_gate}
- 민감 동작 승인(approval): {approval_gate}
- 근거(evidence): {evidence_gate}
- 검증(verification): {verification_gate}
- 수동 QA: {qa_gate display: not_required|required|pending|passed|failed|waived}
- 최종 수락(acceptance): {acceptance_gate}

읽기용 보기 최신성(projection freshness): {projection_freshness}; source_state_version={source_state_version|unknown} (읽기용 보기의 최신성, Task 결과 아님)
상태/입력 최신성: {state_baseline_evidence_freshness_summary|current or none}
````

## 메모

이 템플릿은 렌더링 결과일 뿐 기준 상태가 아닙니다. 현재 출처 기록과 참조에서 렌더링되며, 오래된 채팅 기억에서 렌더링하지 않습니다. 저장된 `JOURNEY-CARD` Markdown은 향후/진단용 상태 보기 범위입니다. `status`, `next`, 중요한 이어가기(resume) 흐름의 초기 현재 위치 맥락은 간결한 상태 출력을 사용할 수 있습니다.

이 카드 안이나 주변에 표시되는 `status`/`next` 추천은 읽기 전용 안내입니다. 사용자 판단 요청, 선택적 전체 형식 판단 패킷(Decision Packet) 표시, `prepare_write`, 근거 수집, 검증, QA, 조정(reconcile), 닫기 시도를 가리킬 수는 있지만, 상태를 변경하거나, 쓰기를 허가하거나, 관문을 충족하거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, Task를 닫지 않습니다.

이어가기 카드(Journey Card)의 닫기 맥락(Close context)은 간결한 상태/이어가기 표시입니다. `TASK`는 진행 중이거나 최근 닫힌 `work` Task의 이어가기용 닫기 요약을 담당하고, `DIRECT-RESULT`는 직접 작업의 가벼운 닫기 영향 요약을 담당합니다. 이 표시들은 [projection/report 경계](../../projection-and-templates.md#projection-principles)를 따르며, 닫기와 관문 영향은 여전히 owner 기록에서 옵니다.

막힘(Blocker) 줄은 API와 상태 기록을 사용자에게 보이는 상태로 바꿔 보여줍니다. 가장 먼저 해소할 막힘은 다음 행동이 먼저 해소해야 하는 막힘이어야 하며, 소유자 라벨은 다음 움직임이 사용자 소유인지, 에이전트가 해소 가능한지, 접점/시스템 소유인지 분명히 해야 합니다. 가장 먼저 해소할 막힘이 없으면 소유자는 `none`으로 렌더링하거나 생략할 수 있습니다. 추가 막힘은 후속 경로에 영향을 줄 때만 계속 보여줍니다. 원시 `ErrorCode` 값만으로 설명을 끝내면 안 됩니다.

최신 근거 또는 다음 근거에 `secret_omitted`나 `blocked` 아티팩트 참조가 포함되면 이 카드는 사용 가능성 영향만 표시해야 합니다. 생략된 값 또는 차단된 원본 페이로드(payload) 내용을 포함하면 안 됩니다.
