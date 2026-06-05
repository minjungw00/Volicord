# 에이전트 맥락 패킷 템플릿

## 사용 시점

다음 안전한 행동에 필요한 현재 맥락을 에이전트가 작고 정확하게 받아야 할 때 `agent-context-packet`을 사용합니다. 이 보기는 사용자용 문장이나 전체 보고서가 아니라 최신성, Core 기반 참조, 허용된 행동 경계, 막힘, 다음 행동에 최적화됩니다.

구현 계층: MVP-1 지원 보기입니다. Structured payload나 prompt 크기의 text로 반환할 수 있습니다. Persisted Markdown projection이 필수는 아닙니다.

경계: 에이전트 맥락 패킷은 행동을 돕는 맥락일 뿐입니다. 쓰기를 허가하거나, gate를 충족하거나, 근거를 만들거나, 민감 동작 승인을 부여하거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 닫기 준비 상태를 만들거나, Task를 닫을 수 없습니다.

## 기준 기록

- Task id, Task 요약, 작업 모양, lifecycle, state version
- active Change Unit 참조, 범위 요약, 하지 않을 일, allowed paths/tools/commands
- 활성 막힘과 blocker 참조
- 활성 사용자 판단, 대기 중인 판단 참조, 판단 요청 참조
- 쓰기 권한 요약. 있을 때 Write Authorization 참조
- 근거 요약, 근거 참조, Run 참조, ArtifactRefs, `redaction_state`, 근거 공백
- 닫기 막힘, 잔여 위험 상태, 최종 수락 필요 여부/상태, 관련 owner 참조
- 보장 수준, MCP/Core availability, source clock, 최신성 상태
- 정확히 하나의 작은 다음 행동과 그 행동에 필요한 owner 문서 또는 owner section pointer

## 렌더링 섹션

- 현재 Task
- active Change Unit과 허용된 행동 경계
- 활성 사용자 판단
- 막힘
- 쓰기 권한
- 근거 상태
- 닫기와 잔여 위험 상태
- 다음 안전한 행동
- 최신성과 출처 참조
- 필요할 때 불러올 pointer

## 전체 템플릿

````text
agent_context_packet:
  display_only: true
  authority: none; authority는 current Core state를 사용
  task_id: {task_id}
  task_summary: {task_summary}
  state_version: {source_state_version}
  work_shape: {work_shape}
  active_change_unit: {change_unit_ref|none}
  scope: {scope_summary}
  non_goals: {non_goals|none}
  allowed_paths: {allowed_paths|none}
  allowed_tools: {allowed_tools|none}
  allowed_commands: {allowed_commands|none}
  active_blockers: {active_blockers|none}
  active_user_judgments: {active_user_judgment_refs|none}
  pending_judgments: {pending_user_judgment_refs|none}
  write_authority: {write_authority_summary|none}
  evidence_summary: {evidence_summary}
  evidence_refs: {evidence_refs_and_gaps}
  design_quality: {design_quality_routed_action|none}
  close: {close_blockers_and_acceptance_state}
  residual_risk_status: {residual_risk_status}
  next_safe_action: {next_safe_action}
  guarantee_level: {guarantee_level_or_unavailable}
  sources:
    refs: {source_refs}
    freshness: {freshness_state}
    rendered_at: {updated_at}
  pull_if_needed: {owner_section_refs_for_next_action|none}
````

## 메모

에이전트 맥락 패킷은 한 화면 안팎으로 유지합니다. 전체 schema, 전체 Reference 문서, 전체 historical event log, 등록된 아티팩트 파일 본문, 전체 report body, 전체 template, 관련 없는 template, full design-quality catalog, future catalog material을 기본으로 넣지 않습니다.

`guarantee_level` 필드는 필수 맥락입니다. Core/MCP가 unavailable이면 unavailable/capability condition을 넣고, refresh 전까지 하네스에 의존하는 state, write, evidence, 최종 수락, 잔여 위험, close claim을 unavailable로 다룹니다.
