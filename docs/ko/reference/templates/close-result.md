# 닫기 결과 템플릿

## 사용 시점

사용자나 에이전트가 닫기 준비 상태, 닫기 막힘, 또는 닫기 결과를 간결하게 봐야 할 때 `close-result`를 사용합니다. 작업 수락, 잔여 위험, 근거, 확인, 막힘을 서로 분리해 보여줍니다.

구현 계층: MVP-1 사용자 작업 루프 보기입니다. 상세 continuity, Journey, direct-result, release-handoff, export 보고서는 later/full-profile 템플릿입니다.

경계: 이 템플릿은 닫기 상태를 표시합니다. Task를 닫거나, 작업 수락을 기록하거나, 잔여 위험을 수용하거나, QA 또는 검증을 면제하거나, 근거를 만들거나, gate 값을 바꾸지 않습니다. 닫기 권한은 Core close path에 남습니다.

## 기준 기록

- 현재 Task 상태와 close attempt 또는 close-readiness result
- 범위와 변경 범위 요약
- 근거 참조와 근거 공백
- 관련 있을 때 확인, 검증, 수동 QA, 면제 상태
- 필요한 경우 작업 수락 user judgment 참조
- 관련 있을 때 잔여 위험 표시와 잔여 위험 수용 참조
- 닫기 막힘과 가장 작은 해소 방법
- source state version, 최신성, capability 상태

## 렌더링 섹션

- 닫기 상태
- 범위
- 근거
- 확인 또는 검증
- 판단과 작업 수락
- 잔여 위험
- 막힘
- 다음 안전한 행동
- 출처와 최신성

## 전체 템플릿

````text
닫기 결과: {ready|blocked|closed|not_requested}
표시 전용: Core close state와 owner ref가 기준입니다.

범위: {scope_summary}
근거: status={evidence_summary.status}; summary={evidence_summary.summary}; 공백={evidence_gaps|none}
확인 또는 검증: {check_verification_manual_qa_summary}
판단과 작업 수락: work_acceptance={work_acceptance_status}; sensitive_action_permission={sensitive_permission_status|not_applicable}
잔여 위험 표시: {residual_risk_visibility}
잔여 위험 수용: {residual_risk_acceptance_status|not_applicable}
막힘: {close_blockers|none}
가장 작은 해소 방법: {smallest_unblocker|none}
닫기 근거 또는 이유: {close_reason|not_applicable}
다음 안전한 행동: {next_safe_action|none}
출처/최신성: state={source_state_version}; refs={source_refs}; rendered={updated_at}; freshness={freshness_state}
````

## 메모

근거, 검증, 수동 QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용, blocker, 읽기용 보기 최신성을 하나의 "완료" 줄로 뭉개지 않습니다. 닫기가 막혔으면 primary blocker를 먼저 말하고, 다음 경로에 영향을 주는 secondary blocker도 보이게 둡니다. 읽기용 close view가 stale 또는 failed이면 이 템플릿의 prose에서 close하지 말고 current Core close result를 가져와야 합니다.
