# 상태 카드 템플릿

## 사용 시점

MVP-1에서 사용자가 현재 상태를 짧게 읽어야 할 때 `status-card`를 사용합니다. 상태 카드는 지금 무엇을 하는지, 무엇이 범위 안인지, 사용자가 무엇을 판단해야 하는지, 어떤 근거가 있거나 빠졌는지, 닫기를 무엇이 막는지, 다음 안전한 행동이 무엇인지를 보여줍니다.

구현 계층: MVP-1 사용자 작업 루프 보기입니다. 내부 엔지니어링 점검은 이 카드 대신 plain structured status/blocker output을 반환해도 됩니다.

경계: 이 템플릿은 렌더링된 표시일 뿐입니다. Core 상태, 근거, 민감 동작 승인, 작업 수락, 잔여 위험 수용, Write Authorization, 닫기 준비 상태가 아닙니다. 오래된 대화가 아니라 현재 Core 소유 상태와 참조에서 렌더링해야 합니다.

## 기준 기록

- 현재 Task 상태, 작업 모양, lifecycle, 다음 안전한 행동
- 관련 있을 때 범위, 하지 않을 일, active Change Unit 요약, 멈춤 조건
- 대기 중인 `user_judgment` 참조와 간결한 판단 요약
- Run 참조, `evidence_ref` 참조, ArtifactRefs, `redaction_state`, 근거 공백
- 닫기 막힘, 작업 수락 필요 여부/상태, 잔여 위험 표시, 필요한 경우 잔여 위험 수용 참조
- 보장 수준과 capability/fallback 상태
- `source_state_version`, 렌더링 시각, 최신성 상태

## 렌더링 섹션

- 작업
- 범위
- 판단
- 근거
- 확인 또는 검증
- 닫기
- 다음 안전한 행동
- 출처와 최신성

## 전체 템플릿

````text
{task_id} {title}
표시 전용: Core 상태와 ref에서 파생된 보기이며 Core 상태나 쓰기 허가 기록이 아닙니다.

작업: {work_shape}. {current_task_summary}
범위: {scope_summary}
범위 밖: {non_goals|none}
판단: {pending_user_judgments|none}
근거: status={evidence_summary.status}; summary={known_evidence_summary|none}
근거 공백: {evidence_gaps|none}
확인 또는 검증: {check_or_verification_summary|none}
닫기: {close_readiness_summary}; 막힘={close_blockers|none}
잔여 위험: {residual_risk_visibility|none}
다음 안전한 행동: {next_safe_action}
보장 수준: {guarantee_level}; {guarantee_note}
출처/최신성: state={source_state_version}; refs={source_refs}; rendered={updated_at}; freshness={freshness_state}
````

## 메모

상태 카드는 읽기 쉬워야 합니다. Schema, DDL, event log, 전체 artifact, 전체 report body, 전체 template, future catalog, 상세 Evidence Manifest 본문, 상세 Eval 본문, 전체 수동 QA record를 쏟아내지 않습니다.

기준 기록이 없으면 상태를 만들어내지 말고 `none`, `unknown`, `not_required`, 또는 명시적인 막힘으로 렌더링합니다.
