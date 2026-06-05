# 닫기 결과 템플릿

## 사용 시점

사용자가 닫기 준비 상태, 닫기 차단 사유, 또는 닫기 결과를 간결하게 봐야 할 때 `close-result`를 사용합니다. 최종 수락, 잔여 위험, 증거, 아티팩트 가용성, 자체 확인 근거, 차단 사유를 서로 분리해 보여줍니다.

구현 계층: MVP-1 사용자 작업 루프 보기입니다. 상세 continuity, Journey, direct-result, release-handoff, export 보고서는 later/full-profile 템플릿입니다.

경계: 이 템플릿은 닫기 상태를 표시합니다. Task를 닫거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, verification 또는 Manual QA를 기록하거나, 증거를 만들거나, gate 값을 바꾸지 않습니다. 닫기 권한은 Core close path에 남습니다.

## 기준 기록

- 현재 Task 상태와 close attempt 또는 close-readiness result
- 범위와 변경 범위 요약
- 증거 참조와 증거 공백
- active evidence summary에 포함된 자체 확인 요약
- close-relevant evidence ref에 대한 아티팩트 가용성
- 필요한 경우 최종 수락 user judgment 참조
- 관련 있을 때 잔여 위험 표시와 잔여 위험 수락 참조
- close에 영향을 줄 때 design-quality routed action. Later profile이 active가 아니면 활성 MVP 차단 집합으로 제한합니다.
- 닫기 가능 여부, 닫기 차단 사유, 가장 작은 해소 방법
- source state version, 최신성, capability 상태

## 렌더링 섹션

- 닫기 상태
- 범위
- 증거
- 아티팩트 가용성과 자체 확인 근거
- 판단과 최종 수락
- 잔여 위험
- 차단 사유
- 다음 안전한 행동
- 출처와 최신성

## 전체 템플릿

````text
닫기 가능 여부: {ready|blocked|closed|not_requested}
표시 전용: Core close state와 owner ref가 기준입니다.

범위: {scope_summary}
증거: status={evidence_summary.status}; summary={evidence_summary.summary}; 공백={evidence_gaps|none}
Artifacts: {artifact_availability_summary}
Self-check basis: {self_check_summary|none}
판단과 최종 수락: final_acceptance={final_acceptance_status}; sensitive_action_permission={sensitive_permission_status|not_applicable}
설계 품질: {design_quality_close_action|none}
잔여 위험 표시: {residual_risk_visibility}
잔여 위험 수락: {residual_risk_acceptance_status|not_applicable}
닫기 불가 이유: {close_blockers|none}
가장 작은 해소 방법: {smallest_unblocker|none}
닫기 증거 또는 이유: {close_reason|not_applicable}
에이전트가 안전하게 할 수 있는 다음 행동: {next_safe_action|none}
출처/최신성: state={source_state_version}; refs={source_refs}; rendered={updated_at}; freshness={freshness_state}
````

## 메모

증거 요약, 아티팩트 가용성, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, blocker, design-quality routed action, 읽기용 보기 최신성을 하나의 "완료" 줄로 뭉개지 않습니다. MVP-1 `close-result` 출력은 detached verification 또는 Manual QA row를 표시하지 않습니다. Later/profile 템플릿은 owner profile이 active일 때 그 row를 추가할 수 있습니다. 닫기가 막혔으면 primary blocker와 다음 행동 하나를 말하고, 다음 경로에 영향을 주는 secondary blocker만 보이게 둡니다. 읽기용 close view가 stale 또는 failed이면 이 템플릿의 prose에서 close하지 말고 current Core close result를 가져와야 합니다.
