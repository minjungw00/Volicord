# API 상태 스키마

이 문서는 기준 범위의 상태 형태 API 스키마를 담당합니다. 공통 상태 참조,
현재 위치 요약, 관찰 상태, 프로젝트 연속성, 쓰기 티켓, 증거, 닫기 준비 상태
데이터를 다룹니다.

## 담당 경계

이 문서는 상태 형태 API 필드, 중첩 구조, 참조, 요약, 스냅샷, 표시 형태, 그리고 필드 존재와 응답 효과의 경계를 담당합니다. 인접 계약은 아래 담당 문서로 연결합니다.

| 인접 계약 | 담당 문서 |
|---|---|
| 공통 요청 래퍼와 응답 분기 | [API 코어 스키마](schema-core.md) |
| 지원되는 enum 형태 값 | [API 값 집합](schema-value-sets.md) |
| 메서드 동작 | [API 메서드](methods.md)와 메서드 담당 문서 |
| 공개 오류 의미 | [API 오류 코드](error-codes.md), [API 오류 처리 경로](error-routing.md) |
| Core 생명주기와 닫기 준비 상태의 제품 의미 | [Core 모델](../core-model.md) |
| 저장소 기록과 지속 효과 | [저장소 기록](../storage-records.md), [저장 효과](../storage-effects.md) |

## 스키마 찾기

| 필요한 데이터 | 시작할 절 |
|---|---|
| 상태 참조, 현재 `Task` 위치, 생명주기, 구체화 준비 상태 | [상태 참조](#state-references) |
| 관리 호스트 기능 지원 진단 | [호스트 기능 지원 진단](#host-feature-support-diagnostics) |
| 호스트 훅 관찰과 세션 감시 범위 | [`GuardHealthSummary`와 관찰 범위](#guard-health-summary) |
| 미기록 변경과 프로젝트 연속성 | [미기록 변경 조정 형태](#unrecorded-change-reconciliation-shapes) |
| 상태 카드, 다음 행동, 쓰기 티켓 | [현재 위치 표시 형태](#current-position-display-shapes) |
| 증거, 관찰, `Run` 요약 | [증거와 실행 기록 스냅샷 형태](#evidence-and-run-snapshot-shapes) |
| 닫기 근거, 잔여 위험, 차단 사유, 검증기, 보장 | [닫기 준비 상태와 검증 형태](#close-readiness-and-validation-shapes) |

## 경계

상태 스키마는 API 데이터 형태만 설명합니다. 상태처럼 보이는 필드가 있다고 해서 응답 분기가 선택되거나 지속 저장, Core 전이, 재실행 행, `authority_events`, 아티팩트 효과, 쓰기 티켓 효과, `state_version` 증가가 생기지는 않습니다.

상태 보기는 계산된 상태를 정직하게 드러내야 합니다.
- `null` 또는 생략된 필드는 메서드가 값을 선택하지 않았거나, 값을 사용할 수 없거나, 담당 스키마가 부재를 명시적으로 허용한다는 뜻입니다. "계산했고 없음"을 암시하는 빈 값으로 바꾸면 안 됩니다.
- `close_blockers: []`나 `risk_acceptance_coverage: []` 같은 빈 배열은 관련 계산을 실행했고 항목이 없었다는 뜻입니다.
- 변경 결과와 `volicord.status` 상태 보기는 겹치는 스키마 영역에서 같은 현재 상태를 설명해야 합니다.
- 계산된 차단 사유는 공유 닫기 준비 상태 엔진과 같은 계산을 사용합니다. 메서드 담당 문서는 분기가 효과를 지속하는지만 결정합니다.

담당 문서 링크:
- 응답 분기 선택: [공통 응답 분기](schema-core.md#common-response)
- 메서드 동작과 효과: [API 메서드](methods.md)와 메서드 담당 문서

<a id="state-references"></a>
## 상태 참조

의미:
- `StateRecordRef`는 API 응답에 나타나는 Core가 소유하는 기록의 공통 공개 참조 형태입니다.
- `record_kind`는 제어 값 문자열입니다.
- `record_id`, `project_id`, `task_id`는 불투명 식별자입니다.
- 기록의 동일성은 정확히 (`project_id`, `record_kind`, `record_id`) 튜플로 결정됩니다. `task_id`는 null 허용 `Task` 맥락이며 기록의 동일성을 결정하는 값이 아닙니다.
- `produced_at_state_version`은 이 참조를 만든 상태 보기에서 관찰한 null 허용 `project_state.state_version` 값입니다. 상태 보기 최신성의 단서일 뿐 기록의 동일성, 기록 자체의 개정값, 닫기 근거 개정값, 권한, 낙관적 동시성 토큰이 아닙니다. 따라서 같은 논리 기록이 서로 다른 `produced_at_state_version`으로 나타나도 다른 기록이 되지 않습니다.
- 메서드 응답이 현재 상태 보기에서 참조를 내보낼 때 null이 아닌 `produced_at_state_version`은 참조된 기록이 더 일찍 바뀌었더라도 그 응답 상태 보기에 사용한 `project_state.state_version`과 같습니다. 그대로 재실행한 응답은 처음 저장된 응답과 그 응답의 상태 보기 최신성 값을 유지합니다.
- `StateRecordRef`는 동시성 입력을 제공하지 않습니다. 호출자가 변경 메서드를 실행할 때 메서드 담당 문서가 요구하면 현재 프로젝트 시계를 `ToolEnvelope.expected_state_version`에 사용합니다.

이는 공개 참조 형태이며 저장소 행을 그대로 넣은 것이 아닙니다.

```yaml
StateRecordRef:
  record_kind: string
  record_id: string
  project_id: string
  task_id: string | null
  produced_at_state_version: integer | null
```

담당 문서 링크:
- `record_kind` 값: [기록과 참조 값](schema-value-sets.md#record-and-reference-values)
- 요청 수준 낙관적 동시성: [`ToolEnvelope`](schema-core.md#tool-envelope)
- 프로젝트 상태 시계: [저장소 버전 관리](../storage-versioning.md)
- 저장소 기록 계열과 값: [저장소 기록](../storage-records.md)
- 저장소 테이블 이름과 DDL: [저장소 DDL](../storage-ddl.md)

<a id="non-authoritative-source-references"></a>

### 권한 효력이 없는 출처 참조

`SourceRef`는 Core 소유 상태 기록 참조가 아닌 호출자 제공 맥락 또는 출처를
기록합니다. `source_kind` 태그는 아래 `source` 본문 중 정확히 하나를
선택합니다.

```yaml
SourceRef:
  source_kind: repository_file | git_commit | git_diff | command | external_uri | user_context
  source: RepositoryFileSource | GitCommitSource | GitDiffSource | CommandSource | ExternalUriSource | UserContextSource

RepositoryFileSource:
  repository_path: string
  baseline_commit_sha: string
  content_sha256: string
  line_range: SourceLineRange | null

SourceLineRange:
  start_line: integer
  end_line: integer

GitCommitSource:
  commit_sha: string

GitDiffSource:
  base_commit_sha: string
  head_commit_sha: string
  diff_artifact_ref: ArtifactRef | null

CommandSource:
  invocation_id: string
  command_summary: string
  exit_code: integer
  output_artifact_ref: ArtifactRef | null

ExternalUriSource:
  uri: string
  retrieved_at: string
  content_sha256: string

UserContextSource:
  context_id: string
```

검증과 권한 경계:
- `repository_path`는 Product Repository 상대 출처 위치입니다. Core는 절대 경로, Windows 드라이브 접두사, 역슬래시, 어휘적으로 저장소 밖으로 벗어나는 `..` 세그먼트를 거부하고 파일시스템이나 심볼릭 링크를 해석하지 않은 채 `.`과 저장소 밖으로 벗어나지 않는 `..` 세그먼트를 제거합니다. 줄 범위는 1부터 시작하는 양끝 포함 범위이며 `1` 이상에서 시작하고 끝이 시작보다 앞설 수 없습니다.
- Git 객체 ID는 정확히 `40`자 또는 `64`자인 소문자 16진수 전체 SHA-1 또는 SHA-256 ID입니다. 콘텐츠 해시는 소문자 16진수 SHA-256 `64`자 문자열입니다.
- `command_summary`는 비어 있지 않은 삭제 처리된 표시 요약이며 실행 가능한 입력이 아닙니다. 아티팩트 참조가 있으면 메서드 담당 문서가 선택한 같은 프로젝트와 같은 Task의 기준 참조입니다.
- `external_uri`는 사용자 정보가 없는 절대 `http` 또는 `https` URI이며 `retrieved_at`은 RFC 3339입니다. `user_context.context_id`는 비어 있지 않은 불투명 상관 ID이며 메시지 본문, 행위자 신원, User Channel 출처가 아닙니다.
- `SourceRef`는 맥락 또는 출처일 뿐입니다. 기록 identity, 범위, 기준선 선택, 사용자 소유 판단, 승인, 쓰기 티켓, 증거 충분성, 최종 수락, 잔여 위험 수락, 닫기 준비 상태, 보장, 동시성 토큰이 아닙니다.
- Core는 제출된 형태를 검증하고 저장합니다. 참조 파일을 읽거나 해시하지 않고, Git 객체를 해석하지 않고, 명령을 실행하지 않고, URI를 가져오지 않고, 메시지 본문을 해석하지 않습니다. 제출된 해시, 객체 ID, 타임스탬프, 종료 코드, 요약, 맥락 ID는 보고된 사실로 남습니다. 검증된 본문은 계속 `ArtifactRef`와 아티팩트 저장소를 따릅니다.

## `StateSummary`

`StateSummary`는 지원되는 메서드가 현재 `Task` 경로를 보여 줘야 할 때 반환하는 간결한 현재 위치 상태입니다.

```yaml
StateSummary:
  project_id: string
  state_version: integer
  task_ref: StateRecordRef | null
  mode: string | null
  requested_control_level: string | null
  effective_control_level: string | null
  control_level_reason: string | null
  project_policy: ProjectWorkflowPolicySummary | null
  work_phase: string | null
  acceptance_policy: string | null
  acceptance_policy_reason: string | null
  lineage: TaskLineageSummary | null
  lifecycle: TaskLifecycleState | null
  scope_revision: integer
  goal_summary: string | null
  scope_summary: string | null
  non_goals: string[]
  acceptance_criteria: AcceptanceCriterion[]
  autonomy_boundary: string | null
  active_change_unit_ref: StateRecordRef | null
  effect_contract: ChangeUnitEffectContract | null
  baseline_ref: string | null
  workspace_context: WorkspaceContext | null
  shaping_readiness: ShapingReadiness | null
  pending_user_action_summaries: AgentSafeUserActionRequestSummary[]
  blocker_refs: StateRecordRef[]
  write_ticket_summary: WriteTicketStateSummary | null
  evidence_summary: EvidenceSummary | null
  evidence_gate: EvidenceGateSummary | null
  close_state: string | null
  close_blockers: CloseReadinessBlocker[]
  guard_health: GuardHealthSummary | null
  guarantee_display: GuaranteeDisplay | null
```

의미:
- `StateSummary`는 상태 참조, 요약, 닫기 준비 상태 필드를 담는 간결한 응답 형태입니다.
- 메서드의 `include` 플래그는 이 형태의 일부만 선택할 수 있습니다. 메서드 담당 문서가 어떤 상태 보기를 선택하지 않는다고 말하면 `evidence_summary`, `evidence_gate`, `close_state`, `close_blockers`, `guard_health`, `guarantee_display` 같은 `include` 제어 필드는 `null`이나 빈 값으로 반환하지 않고 생략합니다. 반환된 빈 배열은 그 상태 보기를 계산했고 비어 있음을 뜻합니다.
- `mode`, `work_phase`, `acceptance_policy`, `close_state`는 값이 있을 때 제어
  값 문자열입니다. `acceptance_policy_reason`은 Core가 Task 소유의 최종 수락
  정책을 선택한 이유이며 승인이나 면제가 아닙니다.
- `requested_control_level`은 `auto` 또는 호출자의 명시적 요청을 보존합니다.
  `effective_control_level`은 Core가 위쪽으로만 조정해 결정한 값입니다. 자유 형식
  `control_level_reason`은 권한이 되지 않으면서 결정 이유를 설명합니다.
  `project_policy`는 사용한 정확한 권위 정책 복사본을 식별합니다.
- `lineage`는 Task의 정규 predecessor edge 하나와 carry-forward 감사
  기록입니다. `scope_revision`은 현재 Task 범위 리비전입니다.
- `goal_summary`, `scope_summary`, `non_goals`, `autonomy_boundary`는 자유 형식
  표시 문자열입니다. `acceptance_criteria`는 `Task`의 현재 기준 기록을
  정규 형태로 담으며 폐기된 기준을 현재 기준으로 표시하지 않습니다.
- `effect_contract`는 현재 적용 Change Unit의 선택적 추가 효과 계약입니다. `null`은 추가 Change Unit 효과 계약이 기록되어 있지 않다는 뜻입니다. 넓은 안전성이나 제한 없는 실행처럼 설명하면 안 됩니다.
- `baseline_ref`는 불투명 기준선 식별자입니다.
- `workspace_context`는 현재 Change Unit 기준선에 결합한 선택적 검증 Git
  좌표입니다. 그 경로와 해시는 로컬 권한 사실이며 portable repository
  identity나 보안 보장이 아닙니다.
- `pending_user_action_summaries`는 응답 보기에 관련된 현재 대기 사용자 행동을 request ID,
  `status=pending`, `next_actor=user`만으로 나열합니다. Core는 요청이 작업을 차단하는지
  결정하기 위해 required-for 대상, 행동 종류, Task, Change Unit, 영향받는 ref, 현재 근거를
  내부에서 계속 평가하지만 `StateSummary`는 그 요청 상세를 노출하지 않습니다.
- Agent 대상 결과의 기존 대기 행동에는 `StateSummary.blocker_refs`,
  `CloseReadinessBlocker.related_refs`, `NextActionSummary.required_refs`, summary-card ref
  collection에서 `record_kind=user_action_request`를 제외합니다. 다른 blocker 및 authority
  record kind는 각 owner 규칙을 따릅니다. 요청 identity는
  `AgentSafeUserActionRequestSummary`만 제공합니다.

의미하지 않는 것:
- `StateSummary` 필드가 있다는 사실만으로 메서드 커밋 여부가 정의되지 않습니다.

담당 문서 링크:
- Task, lineage, workspace, 닫기 값: [`Task` 생명주기 값](schema-value-sets.md#task-lifecycle-values)
- 커밋 결정 분기: [공통 응답 분기](schema-core.md#common-response)
- 메서드별 커밋 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서

<a id="task-lineage-workspace-and-authority-receipt"></a>
### Task lineage, workspace, authority receipt

```yaml
TaskLineageSummary:
  predecessor_task_ref: StateRecordRef
  relation: string
  creation_reason: string
  carry_forward: CarryForwardDisposition[]

CarryForwardDisposition:
  kind: string
  status: string
  source_refs: StateRecordRef[]

TaskFlowItem:
  task_ref: StateRecordRef
  predecessor_task_ref: StateRecordRef | null
  relation: string | null
  mode: string
  work_phase: string
  lifecycle_phase: string

WorkspaceContext:
  vcs: string
  git_common_dir: string
  worktree_id: string
  branch_ref: string | null
  head_sha: string | null
  workspace_fingerprint: string

ProjectWorkflowPolicySummary:
  policy_schema: string
  policy_version: integer
  policy_fingerprint: string
  source: string

AuthorityReceipt:
  project_id: string
  state_version: integer
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  scope_revision: integer
  latest_run_ref: StateRecordRef | null
  product_file_write_observed: boolean
  evidence_gate: EvidenceGateSummary | null
  close_state: string
  close_blockers: CloseReadinessBlocker[]
  completion_claim_allowed: boolean
  next_actor: string
  next_action: NextActionSummary | null
```

의미:

- `TaskLineageSummary`는 predecessor 관계 하나를 기록합니다. `applied`
  carry-forward는 새로 검증된 Task 입력이 되고 `reference_only`는 이전 권한을
  현재 상태로 만들지 않은 채 predecessor 맥락만 보존합니다.
- `TaskFlowItem[]`은 full status가 연결된 predecessor component를 표시하는
  파생 보기이며 새 parent-goal 레코드가 아닙니다.
- `ProjectWorkflowPolicySummary.policy_schema`는 `volicord-policy-v2`입니다.
  `policy_version`은 프로젝트별 단조 증가 값이고 `policy_fingerprint`는 기준 정책
  JSON의 SHA-256입니다. `source`는 권위 있는 데이터베이스 복사본의 출처이며 파일
  로딩 계약이 아닙니다.
- `AuthorityReceipt.completion_claim_allowed`는 호출자가 제공하지 않고 도출합니다.
  유효한 완료 근거에 blocker가 없을 때만 참이며, 활성 Task가 없거나 권한 refresh가
  실패하면 거짓입니다.
- `WorkspaceContext`는 local integration과 Core 쓰기 검사가 함께 사용하는 정규
  Git common-directory 및 linked-worktree identity입니다. branch가 null이면
  detached HEAD이고 Non-Git repository에서는 context가 null입니다.
- `AuthorityReceipt`는 한 번 새로 읽은 project state version에서 Core가
  생성합니다. 선택적 status projection을 생략해도 blocker 목록은 전체입니다.
  `product_file_write_observed`는 모든 과거 Run이 아니라 최신 기록 Run을
  설명합니다. receipt 자체는 커밋, 닫기, 수락, 제품 정확성 증명이 아닙니다.

<a id="host-feature-support-diagnostics"></a>
## 호스트 기능 지원 진단

모든 관리 지원 평가는 먼저 정확히 같은 여섯 키 map을 출력합니다.

```yaml
HostFeatureSupportMap:
  native_user_action: HostFeatureSupportStatus
  local_web_user_channel: HostFeatureSupportStatus
  verified_tool_producer: HostFeatureSupportStatus
  registered_connection_observation: HostFeatureSupportStatus
  record_final_output: HostFeatureSupportStatus
  detective_final_output: HostFeatureSupportStatus
```

여섯 키는 모두 필수이며 다른 기능 키는 허용하지 않습니다. 최종 출력 진단은 이 map을
대체하지 않고 옆에 놓이는 프로필별 세부정보입니다.

```yaml
FinalOutputAuthorityDisclosureDiagnostic:
  support_status: HostFeatureSupportStatus
  configured: boolean
  configuration_verified: boolean
  required_subcapabilities: string[]
  subcapabilities: object<string, HostFeatureSupportStatus>

DoctorHostFeatureSupportRow:
  connection_id: string
  host_kind: string
  selected_profile: string | null
  host_feature_support: HostFeatureSupportMap
  final_output_authority_disclosure: FinalOutputAuthorityDisclosureDiagnostic | null
```

Record는 `required_subcapabilities`와 `subcapabilities`에
`authority_display`, `authenticated_exact_replay`만 정확히 출력합니다. Detective는 두
항목과 `block_finalization`을 출력합니다. 적용되지 않는 키는 출력하지 않습니다. 집계
`support_status`는 [Agent Connection](../agent-connection.md#host-feature-support-state)이
담당하는 우선순위를 사용합니다. `configured`와 `configuration_verified`는 별도 설정
사실이며 `verified`를 뜻하지 않습니다.

기계 판독 projection은 다음과 같이 정확히 배치합니다.

- 연결 상태는 `HostFeatureSupportMap`을 항상 `states.host_feature_support`에 둡니다.
  설치 프로필이 정확히 `record` 또는 `detective`일 때만 프로필 세부정보를
  `states.final_output_authority_disclosure`에 둡니다. 그 밖의 경우에는
  `states.selected_profile` 원래 값을 보존하고
  `states.control_surface.selected_profile`에도 같은 값을 두며, Record를 기본값으로
  만들지 않고 세부정보를 null로 출력합니다. 설정·감사 객체인 `host_hook`에는 두 typed
  필드를 중복하지 않습니다.
- `connection add --dry-run --json`에는 정확한 설치 프로필이 없으므로 계획 상태는
  `selected_profile=not_configured`를 보존하고 완전한 map을 유지하며 프로필 세부정보를
  null로 출력하고 두 typed 필드를 `host_hook`에 중복하지 않습니다.
- Doctor는 읽을 수 있는 저장 Agent Connection마다
  `DoctorHostFeatureSupportRow` 하나를
  `states.host_feature_support_by_connection`에 두고 `connection_id` 순으로
  정렬합니다. 각 행에는 위 다섯 필드만 정확히 들어갑니다.
- 통과, 미완료, 실패, 사용 불가 결과와 관계없이 모든 terminal 릴리스 기능 매트릭스
  셀은 완전한 `HostFeatureSupportMap`을 `host_feature_support`에 둡니다. 정확한 선택
  프로필이 있으면 그 세부정보를 `final_output_authority_disclosure`에 두고, 없으면 null로
  둡니다. init 이후 셀은 제품이 만든 init projection을 복사하고, 사전 점검에서 사용
  불가인 셀은 설정 사실이 false인 중앙 기본 projection을 사용하며 정적 지원 상태를
  지우거나 재분류하지 않습니다. create-new 기록기는 terminal 셀 산출물만 출력하며
  임시 `result=running` 형태를 저장하지 않습니다. terminal
  `result=failed_before_completion` 산출물은 기록기가 가진 정확한
  프로필 힌트와 기본 projection을 사용하고, 정확한 프로필이 없으면 세부정보를 null로
  두며 Record를 기본값으로 만들지 않습니다.

Registry에 읽을 수 있는 연결 행이 없으면 Doctor는 빈
`host_feature_support_by_connection` 배열을 출력하며, 읽을 수 없는 연결에서 기능 map을
합성하지 않습니다. 연결의 정확한 프로필을 선택할 수 없으면 `selected_profile`과 프로필
세부정보는 모두 null입니다. 이 관리 진단 스키마를 여기서 정의했다는 이유만으로 Core
메서드 결과에 추가되지는 않습니다.

<a id="guard-health-summary"></a>
## `GuardHealthSummary`와 관찰 범위

`GuardHealthSummary`는 메서드 담당 문서가 선택했을 때 닫기 준비 상태와 상태
조회 보기가 반환하는 간결한 `detective` 프로필 호스트 훅 및 관찰 상태 보기입니다.
`guard_*` 필드 이름은 내부 호스트 관찰 기록과 훅 관련 구현 상태를 위한 스키마
식별자입니다. 공개 보안 모드나 보안 경계가 아닙니다.

```yaml
ControlSurfaceSummary:
  selected_profile: string
  host_hooks_active: boolean
  session_watcher_active: boolean
  cooperative_pre_tool_warning_available: boolean
  cooperative_pre_tool_denial_available: boolean
  unrecorded_changes_detectable: boolean
  actor_identity_provable: boolean
  os_enforced: boolean

GuardHealthSummary:
  selected_profile: string
  control_surface: ControlSurfaceSummary
  guard_installation_id: string | null
  guard_installation_status: string
  guard_configuration_status: string
  guard_observation_status: string
  effective_guard_status: string
  generated_config_verified: boolean
  native_host_output_adapter_config_verified: boolean
  hook_path_safety: string
  hook_commands_cwd_independent: boolean
  hook_commands_subdirectory_safe: boolean
  cooperative_pre_tool_warning_available: boolean
  cooperative_pre_tool_denial_available: boolean
  post_tool_correlation_available: boolean
  bash_shell_mutation_coverage: boolean
  direct_file_write_matcher_coverage: boolean
  bypass_detection_active: boolean
  guard_hook_observed: boolean
  last_guard_observed_at: string | null
  last_guard_event_at: string | null
  host_kind: string | null
  observed_hook_phase: string | null
  observed_host_kind: string | null
  expected_policy_hash: string | null
  observed_policy_hash: string | null
  observed_binary_version: string | null
  required_hook_phases: string[]
  missing_required_hook_phases: string[]
  prompt_capture_status: string
  prompt_capture_available: boolean
  local_web_consent_available: boolean
  mcp_connection_healthy: boolean
  mcp_connection_status: string | null
  session_watch_status: string
  last_session_watch_checked_at: string | null
  session_watch_baseline_created_at: string | null
  session_watch_coverage_start_at: string | null
  session_watch_coverage_basis: string | null
  session_watch_partial_coverage_warning: string | null
  session_watch_detail: string | null
  session_watch_scan_summary: SessionWatchScanSummary | null
  unresolved_unrecorded_change_count: integer
  missing_or_stale_write_ticket: boolean
  write_ticket_path_scope_violation: boolean

SessionWatchScanSummary:
  files_scanned: integer
  files_skipped: integer
  unreadable_paths_count: integer
  degraded_reasons: string[]
  degraded_reason_counts: object
  skipped_paths_sample: string[]
  skipped_paths_truncated: boolean
  default_excluded_paths: string[]
  max_file_size_bytes: integer
  max_file_count: integer
  follows_symlinks: boolean
  not_full_filesystem_monitoring: boolean

CoverageSummary:
  active_profile: string
  host_hook_state: string
  session_watcher_state: string
  coverage_started_at: string | null
  last_snapshot_at: string | null
  watcher_scan_summary: SessionWatchScanSummary | null
  unresolved_unrecorded_change_count: integer
  non_guarantees: NonGuarantee[]
```

의미:
- `selected_profile`과 `guard_installation_status`는 제어 값 문자열입니다.
- `control_surface`는 Volicord가 현재 관찰하거나 결정할 수 있는 것을 보여 주는 공개 관찰 요약입니다. 선택된 프로필, 호스트 훅과 세션 감시기의 활성 여부, 협력형 도구 실행 전 경고 또는 거부의 가용성, 미기록 변경 탐지 가능 여부, 행위자 신원 증명 가능 여부, OS 강제 제공 여부를 보고합니다.
- `guard_installation_id`가 `null`이 아니면 불투명 내부 호스트 훅 설치 식별자입니다.
- `guard_configuration_status`, `guard_observation_status`, `effective_guard_status`는 파일과 설정 상태, 런타임 훅 관찰, 닫기 준비 상태에서 쓰는 유효 `detective` 프로필 상태를 구분합니다. 저장 설치 행의 `configured`와 `active`는 모두 설정 상태 `configured`로 projection됩니다. `detective`에서는 이 설정 상태와 현재 일치하는 관찰이 함께 있으면 유효 상태가 `active`입니다. 따라서 동일 신원 설정을 새로 고친 뒤 생명주기 행이 `configured`여도 보존된 일치 관찰이 있으면 유효 상태는 계속 `active`입니다. `reload_required`, `degraded`, `stale`, `broken`은 계속 비활성입니다.
- `generated_config_verified`, `native_host_output_adapter_config_verified`, `hook_path_safety`, `hook_commands_cwd_independent`, `hook_commands_subdirectory_safe`, `cooperative_pre_tool_warning_available`, `cooperative_pre_tool_denial_available`, `post_tool_correlation_available`, `bash_shell_mutation_coverage`, `direct_file_write_matcher_coverage`, `bypass_detection_active`, `prompt_capture_available`, `local_web_consent_available`는 선택된 프로필의 기능 또는 설정 정보를 노출합니다. `native_host_output_adapter_config_verified`는 설정에만 쓰는 닫기 gate이며 호스트 기능 지원이나 실제 전달을 주장하지 않습니다. `detective` 호스트 훅에는 검증된 생성 설정과 호스트 기본 출력 설정, `hook_path_safety=ok`, 현재 작업 디렉터리와 무관하고 하위 디렉터리에서도 안전한 필수 훅 명령, 필수 생명주기 단계, Bash/셸 및 직접 파일 쓰기 매처 적용 범위, 일치하는 정책 해시, 현재 일치하는 호스트 훅 관찰이 필요합니다. 미기록 변경 탐지에는 활성 세션 감시가 필요합니다. 부분 관찰 범위 경고는 `session_watch_partial_coverage_warning`에 계속 표시됩니다. 런타임 전용 기능을 관찰할 수 없는 설정 진단은 그 기능을 `false`로 보고합니다.
- `guard_hook_observed`는 선택된 내부 호스트 훅 설치 기록에 현재 일치하는 호스트 훅 관찰이 기록되어 있는지를 보고합니다. 현재 일치하는 관찰에는 파싱 가능한 시각, 현재 설치와 정확한 `volicord-host-hook-capability-v2` 역량에 일치하는 관찰 호스트 및 정책 해시, 그리고 그 역량의 현재 생명주기 명령에 설정된 알려진 관찰 단계가 있어야 합니다. 사실이 누락되거나, 잘못됐거나, 알려지지 않았거나, 일치하지 않으면 닫힌 상태에서 실패합니다. 하나의 Agent Connection을 여러 Connection Projects에 걸쳐 집계하는 연결 단위 projection에서는 적용되는 모든 Detective 설치에 현재 일치하는 관찰이 있을 때만 이 필드가 `true`입니다.
- `last_guard_observed_at`은 가장 최근 저장된 내부 호스트 훅 설치 관찰 시각입니다. 관찰이 기록되어 있지 않으면 `null`입니다. 관찰이 더 이상 현재 상태가 아니어도 가장 최근에 저장된 시각을 보고하며, 관찰의 현재 일치 여부는 `guard_hook_observed`와 유효 guard 상태로 나타냅니다.
- `last_guard_event_at`은 상태 보기에 사용할 수 있는 최신 호스트 훅 이벤트 시각입니다. 사용할 수 있는 호스트 훅 이벤트가 없으면 `null`입니다.
- `host_kind`, `observed_hook_phase`, `observed_host_kind`, `expected_policy_hash`, `observed_policy_hash`, `observed_binary_version`은 사용할 수 있을 때 선택된 설치와 최신 저장 관찰 메타데이터를 보고합니다.
- `required_hook_phases`와 `missing_required_hook_phases`는 필수 호스트 훅 설정이 완전한지를 보고합니다. 이 공개 projection은 의도적으로 닫힌 상태에서 실패합니다. 필요한 단계가 `required_hook_phases`에 없거나 `missing_required_hook_phases`에 나열되어 있으면 누락된 것으로 취급합니다. 반면 유효한 저장 `volicord-host-hook-capability-v2` Detective 기록은 항상 정규 필수 단계 다섯 개를 선언하고, 중복 없는 부분집합을 `missing_required_hooks`에 나열하는 방식으로만 저하 상태를 표현합니다. 이 저장 계약은 [저장소 기록](../storage-records.md)이 담당합니다. 손상되었거나 독립적으로 구성된 입력이 단계를 생략해 완전한 상태가 될 수 없도록 projection에는 “부재 또는 명시적 나열” 규칙을 유지합니다. 필요한 단계가 누락되면 유효한 훅 이벤트가 관찰되었더라도 유효 `detective` 상태는 `active`가 되지 않습니다.
- `prompt_capture_status`는 선택된 연결에서 프롬프트 캡처를 사용할 수 있는지를 기계가 읽을 수 있는 값으로 보고합니다. `prompt_capture_available=true`는 그 상태가 검증 코드 채팅 명령을 허용할 때만 사용합니다. 원문 프롬프트 텍스트가 포함된다는 뜻은 아닙니다.
- `prompt_capture_available`은 선택된 연결에서 프롬프트 캡처용 검증 코드 채팅 명령을 표시하거나 기록할 수 있는지 보고합니다. 프롬프트 텍스트는 포함하지 않습니다.
- `local_web_consent_available=true`는 현재 어댑터 호출의 중앙 평가기가 관리되는
  generic이 아닌 stdio 호스트, 준비된 loopback listener,
  `params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`의
  정확한 boolean `true` 선언, 만료되지 않은 `outcome=passed` 결과를 가진 현재의 정확히
  일치하는 영속 호스트 역량 상태를 모두 관찰할 때만 사용합니다. 구간은
  `observed_at <= created_at < expires_at <= observed_at + 86,400 seconds`를 만족해야 하고
  행의 `evidence_artifact_sha256`은 같은 역량, 호스트·클라이언트, 어댑터, 빌드, source,
  target, 실행 파일 다이제스트에 결속된, 실행 파일 밖의 별도로 검증한 정확한 최종
  아티팩트 릴리스 증거 manifest 또는 receipt의 예상 다이제스트와도 정확히 일치해야 하며
  현재 평가는
  `observed_at <= now < expires_at`을 사용합니다. 24시간은 기본 수명이나 attestation
  기간이 아니라 최대 최신성 구간입니다. 선언이 생략됐거나
  false이거나, 타입·형태·namespace가 잘못됐거나, 검증이 없거나 통과하지 않았거나
  만료·취소·손상·불일치 상태이면 `false`입니다. 이 값은 token 발급, form 표시, 사용자
  식별, 모델 격리 증명을 뜻하지 않습니다. Status와 check-close는 같은 평가기를 사용하고
  가용성 보고만을 위해 token을 발급하지 않습니다. 모든 런타임·영속 입력을 관찰할 수
  없는 setup 진단도 `false`를 보고합니다.
  Manifest가 없거나, 알 수 없거나, 잘못됐거나, 검증되지 않았거나, 일치하지 않아도
  `false`입니다. 행과 빌드 메타데이터가 예상 다이제스트를 자기 선언할 수 없습니다. 현재
  어댑터에는 신뢰된 manifest 획득 경로가 없으므로 운영 projection은 이 역량을 사용할 수
  없다고 보고합니다.
- `mcp_connection_healthy`와 `mcp_connection_status`는 추적되는 Agent Connection 확인 상태가 있을 때 그 상태를 요약합니다.
- `session_watch_status`는 선택된 연결 또는 세션의 Product Repository 세션 감시기가 `disabled`, `active`, `degraded`, `unavailable`, `pending_project_selection` 중 어떤 상태인지 보고합니다.
- `last_session_watch_checked_at`은 가장 최근 세션 감시 기준선 상태 갱신 시각입니다. 사용할 수 있는 기준선이 없으면 `null`입니다.
- `session_watch_baseline_created_at`은 저장된 세션 감시 기준선 생성 시각입니다. 사용할 수 있는 기준선이 없으면 `null`입니다.
- `session_watch_coverage_start_at`은 선택된 세션에서 감시 기준선의 관찰 범위가 시작되는 시각입니다. 사용할 수 있는 시작 시각이 없으면 `null`입니다.
- `session_watch_coverage_basis`는 `mcp_start`, `first_project_selection`, `method_boundary`, 또는 `null`입니다.
- `session_watch_partial_coverage_warning`은 기록된 관찰 시작 전의 Product Repository 변경이 세션 감시 범위 밖에 있을 때 사람이 읽을 수 있는 경고입니다.
- `session_watch_detail`은 선택된 세션 감시 상태의 짧은 진단 세부정보입니다. 사용할 수 있는 세부정보가 없으면 `null`입니다.
- `session_watch_scan_summary`는 사용할 수 있을 때 선택된 세션 감시기의 스캔 범위를 보고합니다. 스캔한 파일 수, 건너뛴 파일 수, 읽을 수 없는 경로 수, 저하 사유별 수, 건너뛴 경로 샘플, 기본 정책 제외 경로, 파일 크기와 파일 수 제한, `follows_symlinks=false`, `not_full_filesystem_monitoring=true`를 포함합니다.
- `unresolved_unrecorded_change_count`는 해결되지 않은 미기록 Product Repository 변경 수입니다. 프롬프트 텍스트, 명령 텍스트, 경로 목록은 노출하지 않습니다.
- `missing_or_stale_write_ticket`는 호스트 훅 이벤트가 누락되었거나, 확인할 수 없거나, 모호하거나, 오래된 쓰기 티켓 준비 상태를 감지했는지 보고합니다.
- `write_ticket_path_scope_violation`은 호스트 훅 이벤트가 상태가 `active`인 쓰기 티켓 범위 밖의 Product Repository 경로를 관찰했는지 보고합니다.
- `CoverageSummary`는 상태 조회와 닫기 준비 상태 결과가 선택하는 간결한 파생 관찰 범위 보기입니다. `active_profile`은 현재 `record` 또는 `detective` 프로필입니다. `host_hook_state`는 `observed`, `not_observed`, `unsupported`, `degraded` 중 하나이고, `session_watcher_state`는 `active`, `inactive`, `unsupported`, `degraded` 중 하나입니다.
- `coverage_started_at`은 런타임이 추적하는 세션 감시 범위 시작 시각이며, 사용할 수 없으면 `null`입니다. `last_snapshot_at`은 추적 중인 최신 감시 기준선 또는 스냅샷 상태 시각이며, 사용할 수 없으면 `null`입니다.
- `CoverageSummary.watcher_scan_summary`는 관찰 범위가 선택될 때 `GuardHealthSummary.session_watch_scan_summary`와 같습니다.
- `CoverageSummary.unresolved_unrecorded_change_count`는 닫기 준비 상태가 사용하는 해결되지 않은 미기록 Product Repository 변경 수와 같습니다.
- 관찰 범위가 보고될 때 `CoverageSummary.non_guarantees`는 `NotActorAttributionProof`, `NotFullFilesystemMonitoring`, `NotFullWritePrevention`을 포함해야 합니다.

의미하지 않는 것:
- `control_surface`는 정확성, 검토 완료, 테스트 충분성, OS 수준 강제, 쓰기 차단을 증명하지 않습니다.
- `GuardHealthSummary`는 제품 정확성, 테스트 충분성, OS 강제, 샌드박싱, 보안 격리, 최종 수락의 증거가 아닙니다.
- active인 `effective_guard_status`는 증거, 아티팩트 무결성, 사용자 소유 판단, 쓰기 티켓, 최종 수락, 잔여 위험 수락 요구사항을 대체하지 않습니다.
- 세션 감시 상태와 관찰 범위 메타데이터는 Volicord가 쓰기를 막았거나, 전체 파일시스템을 감시했거나, 파일을 바꾼 행위자를 식별했거나, 파일 내용을 저장했거나, OS 수준 강제를 제공했다는 뜻이 아닙니다.
- `session_watch_partial_coverage_warning`이 `null`이 아니면 `session_watch_coverage_start_at` 전의 Product Repository 변경은 세션 감시 범위 밖에 남습니다.
- `record`는 협력형으로 남습니다. `detective` 관찰 상태가 해결되지 않은 미기록 변경을 보고하면 그 변경은 닫기를 막습니다.
- `detective`는 모든 쓰기를 막거나, 전체 파일시스템을 감시하거나, 파일을 바꾼 행위자를 식별하거나, 네트워크를 격리하거나, 샌드박스를 제공하지 않습니다.

담당 문서 링크:
- `selected_profile`, `host_hook_state`, `session_watcher_state`, `hook_path_safety`, `guard_installation_status`, `guard_configuration_status`, `guard_observation_status`, `effective_guard_status`, `prompt_capture_status`, `session_watch_status`, `session_watch_coverage_basis` 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values)
- 닫기 준비 상태 `guard_*` 차단 사유와 메서드 로컬 코드: [`volicord.check_close`와 `volicord.close_task`](method-close-task.md)
- Agent Connection 의미: [Agent Connection](../agent-connection.md)

<a id="unrecorded-change-reconciliation-shapes"></a>
## 미기록 변경 조정 형태

`UnrecordedChangeFinding`은 `volicord.reconcile_changes`가 해결되지 않은 미기록
Product Repository 변경에 대해 반환하는 공개 형태입니다.

`UnrecordedChangeResolutionSummary`는 조정 호출 하나가 해결한 미기록 변경의 공개
요약 형태입니다.

```yaml
UnrecordedChangeFinding:
  unrecorded_change_ref: StateRecordRef
  status: string
  confidence: string
  summary: string
  observed_paths: string[]
  detected_at: string
  can_resolve_in_chat: boolean
  next_action: NextActionSummary

UnrecordedChangeResolutionSummary:
  unrecorded_change_ref: StateRecordRef
  resolution_basis: string
  resolved_by_actor_source: string
  capture_basis: string
  user_action_resolution_ref: StateRecordRef | null
  resolved_at: string
```

의미:

- `unrecorded_change_ref`는 `record_kind=unrecorded_change`인 `StateRecordRef`를 사용합니다.
- `status`는 제어 값 문자열입니다.
- `confidence`는 `confirmed` 또는 `suspected`입니다. 미해결 `confirmed` finding만
  닫기 차단 사유이며 `suspected` finding은 검증 대상으로 계속 보입니다.
- `summary`, `capture_basis`, `next_action.label`은 표시 문자열이며 정확성 증명이 아닙니다.
- `observed_paths`는 Core가 안전하게 디코딩할 수 있을 때 Product Repository 상대 경로를 담습니다. 프롬프트 텍스트, 명령 텍스트, 셸 인수, 전체 민감 내용을 포함하지 않습니다.
- `can_resolve_in_chat`은 메서드 담당 문서가 선택한 채팅 매개 사용자 경로로 진행할 수 있는지를 나타냅니다.
- `resolution_basis`는 미기록 변경이 해결된 이유를 분류합니다.
- `resolved_by_actor_source=system`은 Core가 결정적 basis를 검증했다는 뜻입니다. `resolved_by_actor_source=local_user`는 호환 User Channel 판단이 권한을 제공했다는 뜻입니다.
- `user_action_resolution_ref`는 사용자 소유 수락 해결일 때만 null이 아닙니다.

이 형태들은 제품 정확성, 테스트 충분성, 리뷰 완료, 최종 수락, 잔여 위험 수락, 보안을 증명하지 않습니다. 해결 동작과 호출자 제한은 [`volicord.reconcile_changes`](method-reconcile-changes.md)가 담당합니다.

담당 문서 링크:

- 해결 동작: [`volicord.reconcile_changes`](method-reconcile-changes.md).
- 해결 근거와 상태 값: [API 값 집합](schema-value-sets.md#unrecorded-change-resolution-basis-values).
- 저장 기록 보존: [저장소 기록](../storage-records.md).

<a id="project-continuity-shapes"></a>
## 프로젝트 연속성 형태

`ProjectContinuityRecord`는 오래 유지하는 프로젝트 수준 연속성 기록 하나의 전체 API 상태 형태입니다. `ProjectContinuitySummary`는 상태 조회 보기에 쓰는 간결한 형태입니다.

```yaml
ProjectContinuityRecord:
  continuity_record_id: string
  project_id: string
  source_task_id: string
  source_change_unit_id: string | null
  kind: string
  title: string
  summary: string
  rationale: string | null
  applies_to_paths: string[]
  applies_to_refs: StateRecordRef[]
  source_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
  status: string
  supersedes_refs: StateRecordRef[]
  review_triggers: string[]
  created_at: string
  updated_at: string

ProjectContinuitySummary:
  continuity_record_ref: StateRecordRef
  kind: string
  status: string
  title: string
  summary: string
  source_task_ref: StateRecordRef
  source_change_unit_ref: StateRecordRef | null
  review_triggers: string[]
```

의미:
- 프로젝트 연속성 기록은 원천 `Task`가 닫힌 뒤에도 유지해야 하는 결정, 의무, 알려진 한계, 수락된 잔여 위험, 제약 같은 프로젝트 수준 맥락을 보존합니다.
- `source_task_id`와 `source_change_unit_id`는 기록이 어디에서 비롯되었는지를 식별합니다. 원천 `Task`나 Change Unit을 다시 현재 상태로 만들지는 않습니다.
- `applies_to_paths`, `applies_to_refs`, `source_refs`, `artifact_refs`, `supersedes_refs`, `review_triggers`는 이후 검토를 위한 제한된 맥락입니다. 빈 배열은 그 필드에 항목이 없다는 뜻입니다.
- `ProjectContinuitySummary`는 메서드 담당 문서가 선택하는 읽기 보기이며, 전체 지속 기록이 아닙니다.

의미하지 않는 것:
- 프로젝트 연속성 기록은 현재 `Task` 권한, 증거, 쓰기 티켓, 최종 수락, 닫기 준비 상태, 미래 닫기 근거의 잔여 위험 수락, 차단 사유 면제가 아닙니다.
- `status=active`는 그 연속성 기록이 살아 있는 프로젝트 맥락이라는 뜻입니다. 모든 `Task`에 현재 적용된다거나 원천 결정이 새 권한 확인에 충분하다는 뜻은 아닙니다.

담당 문서 링크:
- `kind`와 `status` 값: [프로젝트 연속성 값](schema-value-sets.md#project-continuity-values)
- 저장소 계열과 JSON 배치: [저장소 기록](../storage-records.md)
- 메서드별 생성 효과: [저장 효과](../storage-effects.md)

## `ChangeUnitEffectContract`

`ChangeUnitEffectContract`는 Change Unit에 기록되는 선택적 효과 경계 객체입니다.

```yaml
ChangeUnitEffectContract:
  allowed_effects: string[]
  forbidden_effects: string[]
  allowed_paths: string[]
  expected_outputs: string[]
  invariants: string[]
  evidence_expectations: string[]
  sensitive_action_expectations: string[]
```

의미:
- `allowed_effects`와 `forbidden_effects`는 현재 Change Unit이 Core 상태로 허용하거나 금지하는 효과를 분류합니다.
- `allowed_paths`는 값이 있을 때 제품 파일 쓰기를 더 좁히는 Product Repository 상대 경로 목록입니다.
- `expected_outputs`, `invariants`, `evidence_expectations`, `sensitive_action_expectations`는 구조화된 기대 문자열입니다. 워크플로 엔진을 만들지 않으면서 의도한 출력과 증거 경계를 사용자와 에이전트가 이해하도록 돕습니다.
- 빈 배열은 그 계약 부분이 추가 제한이나 기대를 더하지 않는다는 뜻입니다.

의미하지 않는 것:
- `ChangeUnitEffectContract`는 런타임 샌드박스, 명령 가로채기 장치, 네트워크 차단 장치, 운영체제 권한 체계, 개발 방법론 상태 기계가 아닙니다.
- 사용자 소유 판단, 민감 동작 승인, 증거, 쓰기 티켓, 최종 수락, 닫기 준비 상태, 잔여 위험 수락을 대신하지 않습니다.

담당 문서 링크:
- 효과 값 문자열: [메서드 내부 값](schema-value-sets.md#method-local-values)
- Product Repository 경로 정규화: [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization)
- 계약을 기록하는 메서드 동작: [`volicord.update_scope`](method-update-scope.md)
- 제품 파일 쓰기 경계를 적용하는 메서드 동작: [`volicord.prepare_write`](method-prepare-write.md)

## `Task` 생명주기 상태

`TaskLifecycleState`는 `StateSummary`나 닫기 결과 안에 나타날 수 있는 `Task` 생명주기 필드의 API 형태입니다.

```yaml
TaskLifecycleState:
  lifecycle_phase: string
  close_reason: string
  result: string
  closed_at: string | null
```

담당 문서 링크:
- `lifecycle_phase`, `close_reason`, `result`의 지원 값: [`Task` 생명주기 값](schema-value-sets.md#task-lifecycle-values)
- 생명주기 영역의 제품 의미: [Core 모델의 `Task` 생명주기](../core-model.md#6-task-lifecycle)

## `ShapingReadiness`

의미:
- `ShapingReadiness`는 `Task`, Change Unit, 대기 중인 사용자 행동, 증거 요약, 차단 사유, 다음 행동 필드를 포괄하는 API 보기 형태입니다.
- boolean 필드와 `gaps` 배열은 현재 상태의 준비 상태 형태 데이터를 드러냅니다.

```yaml
ShapingReadiness:
  goal_summary_known: boolean
  scope_boundary_known: boolean
  non_goals_known: boolean
  affected_area_or_paths_known: boolean
  acceptance_criteria_known: boolean
  autonomy_boundary_known: boolean
  first_change_unit_known: boolean
  user_owned_blocker_kind: string | null
  next_safe_action: NextActionSummary | null
  gaps: ShapingGap[]

ShapingGap:
  gap_kind: string
  message: string
  blocker_ref: StateRecordRef | null
  user_action_request_candidate_ref: StateRecordRef | null
```

의미:
- `ShapingGap`은 형태에 따라 blocker 또는 담당자가 제안한 사용자 행동 요청 후보를
  참조할 수 있습니다. 후보 ref 자체는 resolution이 아닙니다.
- `user_owned_blocker_kind`와 `ShapingGap.gap_kind`는 불투명 준비 상태 분류 문자열입니다. 영향받는 담당 문서가 더 좁은 값을 공개하지 않는 한 빠짐없는 공개 값 집합이 아닙니다.
- `ShapingGap.message`는 자유 형식 표시 문자열입니다.

담당 문서 링크:
- 메서드 동작과 지속 효과: [API 메서드](methods.md)가 안내하는 메서드 담당 문서와 [저장 효과](../storage-effects.md)

<a id="current-position-display-shapes"></a>
## 현재 위치 표시 형태

```yaml
SummaryCard:
  task: string
  recording: string
  profile: string
  write_ticket: string
  evidence: string
  user_action: string
  changes: string
  close_status: string
  transport: string
  next: string
  next_action: NextActionSummary | null
  guarantee: string

NextActionSummary:
  presentation_role: string
  action_kind: string
  owner_method: string | null
  allowed_operation_categories: string[]
  label: string
  blocking_question: string | null
  expected_state_version: integer | null
  required_refs: StateRecordRef[]

WriteTicketStateSummary:
  status: string
  write_ticket_ref: StateRecordRef | null
  basis_state_version: integer | null
  validity_basis: WriteTicketValidityBasis | null
  invalidation_reason: string | null
  idle_expires_at: string | null
  intended_paths: string[]
  consumed_by_run_ref: StateRecordRef | null
  observation_refs: StateRecordRef[]
  guarantee_display: GuaranteeDisplay | null

WriteTicketAttemptScope:
  task_id: string
  change_unit_id: string
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string | null

WriteTicketPathPatterns:
  allowed: string[]
  denied: string[]

WriteTicketValidityBasis:
  task_id: string
  change_unit_id: string
  scope_revision: integer
  baseline_ref: string | null
  workspace_context_sha256: string | null
  write_authority_fingerprint: string | null
  approval_basis_refs: StateRecordRef[]

WriteTicketScope:
  task_id: string
  change_unit_id: string
  intended_operation: string
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string | null

WriteTicket:
  write_ticket_id: string
  write_ticket_ref: StateRecordRef
  state: string
  scope: WriteTicketScope
  path_patterns: WriteTicketPathPatterns
  observed_paths: string[]
  basis_state_version: integer
  validity_basis: WriteTicketValidityBasis
  invalidation_reason: string | null
  idle_expires_at: string | null
  control_surface: ControlSurfaceSummary | null
  guarantee_display: GuaranteeDisplay | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

의미:
- `SummaryCard`는 주요 사용자 대상 상태 보기에 쓰는 안정적인 간결 요약 형태입니다. `Task`, `Recording`, `Profile`, `Write Ticket`, `Evidence`, `User Judgment`, `Changes`, `Close Status`, `Transport`, 다음 행동 하나, 짧은 `Guarantee` 줄에 공개 표시 문자열을 사용합니다.
- 증거 또는 닫기 상태 보기를 선택하면 `SummaryCard.evidence`는 [API 값 집합](schema-value-sets.md#evidence-gate-values)이 담당하는 `EvidenceGateSummary.state` 값을 그대로 사용합니다. 스테이징 입력이나 `EvidenceSummary.evidence_state`에서 별도 상태를 추론하지 않습니다.
- `SummaryCard.next`는 요약을 위해 선택된 단일 표시 다음 행동입니다. 담당 문서가 선택한 보기에서 알 수 있는 다음 행동이 없을 때만 `none`을 사용합니다. `SummaryCard.next_action`은 대응하는 구조화된 `NextActionSummary`를 담을 수 있으며 구조화된 행동이 적용되지 않으면 생략될 수 있습니다. 구조화된 행동이 적용되면 요약은 `presentation_role=primary`인 행동을 선택하며 배열 위치는 선택 계약이 아닙니다.
- `SummaryCard`는 담당 문서가 선택한 다른 상태 필드의 요약이지 두 번째 권한 기록이 아닙니다. 표시된 다음 행동에 식별자가 필요하지 않은 한 내부 식별자를 추가하면 안 됩니다.
- 이미 존재하는 대기 사용자 행동에는 `SummaryCard.user_action`, `SummaryCard.next`, 메서드
  `status_summary`, blocker message, 그 밖의 모든 display/template string이 일반 문구만
  사용합니다. 사용자 행동이 대기 중이고 다음 actor가 User Channel임을 말할 수 있지만
  요청 질문, 선택지, 맥락, form, 경로, 명령, URL, credential을 다시 만들면 안 됩니다.
- `SummaryCard.guarantee`는 요약된 보기에 대한 짧은 표시 문구입니다. 다른 담당 문서가 명시적으로 그런 보장을 제공하지 않는 한 정확성 증명, 테스트 충분성 증명, 검토 완료, OS 수준 집행을 주장하면 안 됩니다.
- `NextActionSummary`는 기준 다음 행동 표시 형태입니다. 유효한 필드는 `presentation_role`, `action_kind`, `owner_method`, `allowed_operation_categories`, `label`, `blocking_question`, `expected_state_version`, `required_refs`입니다.
- 비어 있지 않은 각 최상위 `next_actions` 모음에는 `presentation_role=primary`인 항목이 정확히 하나 있습니다. 나머지 항목은 `additional`을 사용합니다. 닫기 준비 상태는 명시적인 중첩 예외입니다. 닫기 준비 상태 결과 하나의 `blockers[*].next_actions`를 평탄화한 전체가 primary 하나를 갖는 투영 단위이므로 뒤쪽의 개별 차단 사유 목록에는 additional 행동만 있을 수 있습니다. 단일 `next_action`은 `primary`를 사용합니다.
- `additional`은 표시 역할이지 선택 사항이라는 뜻이 아닙니다. 다른 차단 사유를 해소하려면 보조 행동도 필수일 수 있습니다.
- `allowed_operation_categories`는 행동에 대해 담당 문서가 지원하는 호출 범주를 이름 붙입니다. 현재 연결이 행동을 실행할 수 있음을 증명하거나 사용자 권한을 부여하지 않으며, 지원되는 API 메서드 호출이 식별되지 않으면 비어 있습니다.
- `expected_state_version`은 항상 존재하는 null 허용 필드입니다. 낙관적 동시성을 사용하는 API 변경 행동에는 그 행동을 만든 상태 보기의 현재 `project_state.state_version`을 담으며, 해당 호출의 `ToolEnvelope.expected_state_version`으로 직접 매핑됩니다. 읽기 행동, `user_only` 행동, 단일 담당 메서드가 없는 행동, 낙관적 동시성을 사용하지 않는 담당 메서드 행동에는 `null`을 사용합니다.
- `expected_state_version`은 재시도 가능한 동시성 입력이며 신원이나 권한이 아닙니다. 다른 변경이 커밋되면 오래될 수 있으므로 호출자는 `STATE_VERSION_CONFLICT` 뒤 현재 상태를 새로 읽습니다. `required_refs`와 참조의 `produced_at_state_version`은 이 토큰을 제공하거나 덮어쓰지 않습니다.
- 오래된 `action` 또는 `reason` 필드를 쓰는 `next_actions` 항목은 유효한 `NextActionSummary`가 아닙니다.
- 이미 존재하는 대기 사용자 행동에는 `NextActionSummary.label`과 소유 blocker message가
  일반 User Channel 안내만 사용하고 `blocking_question=null`이며 `required_refs`에
  `user_action_request` ref가 없습니다. Request ID와 pending/next-actor 사실은
  `AgentSafeUserActionRequestSummary`에서만 가져옵니다. 다음 행동 text는 질문, 선택지, 맥락,
  form, 캡처 경로, 명령, URL, credential을 다시 만들면 안 됩니다. 별도의 요청 전
  `missing_final_acceptance` 행동은 Agent가 요청을 만드는 데 필요한 질문과 Task/현재 근거
  ref를 담을 수 있습니다. 요청을 만든 뒤에는 대기 규칙을 적용합니다.
- `WriteTicketStateSummary.status`는 제어 값 문자열입니다.
- `WriteTicketStateSummary.consumed_by_run_ref`는 요약된 쓰기 티켓이 기록된 Run에 의해 소비되었을 때만 `null`이 아닙니다.
- `WriteTicketStateSummary.observation_refs`는 사용할 수 있을 때 그 소비 Run이 만든 증거 관찰 참조를 나열합니다. 쓰기 티켓이 소비되지 않았거나 소비 Run이 관찰을 만들지 않았다면 비어 있습니다.
- `WriteTicketAttemptScope`는 쓰기 티켓이 포착하는 한 번의 시도 경계입니다.
- `WriteTicketAttemptScope`는 일반 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 포괄적 사용자 승인이 아닙니다.
- `WriteTicket`은 커밋된 허용 결정이 호환되는 티켓을 발급하거나 재사용할 때 `volicord.prepare_write`가 반환하는 티켓 우선 권한 기록입니다.
- `WriteTicket.state`는 제어되는 값 문자열입니다.
- `WriteTicket.path_patterns.allowed`와 `WriteTicket.path_patterns.denied`는 티켓 결정이 포착한 정규화된 repository-relative 경로 prefix입니다. Prefix는 정확한 경로나 하위 경로와 일치하며 wildcard와 glob 문법은 지원하지 않습니다. 절대 경로, 빈 값, `..` 포함 값, 모호한 값은 유효하지 않고 denied prefix가 우선하며 allowed가 비어 있으면 product-file 쓰기를 하나도 허용하지 않습니다.
- `WriteTicket.validity_basis`, 소비 상태, 선택적 idle timeout, 무효화 사유가
  유효성을 결정합니다. `basis_state_version`은 감사 순서만 기록하며 관련 없는 상태
  버전 증가는 티켓을 무효화하지 않습니다.
- `WriteTicketValidityBasis.write_authority_fingerprint`는 정확한 정규화 객체
  `{schema:"volicord-write-authority-v1",default_direct_control,default_work_control,light:{enabled,max_intended_paths,allowed_path_patterns,denied_path_patterns,final_acceptance},write_ticket:{idle_timeout_minutes}}`를
  canonical JSON으로 만든 뒤 계산한 `sha256:` 접두사 SHA-256입니다. 각 값은 대응하는
  `workflow` 정책 필드에서 가져오고 두 패턴 배열은 canonicalization 전에 정렬하고 중복을
  제거합니다. 쓰기 권한 판단에서 참조하지 않는 detective, host, connection,
  integration binding 필드는 제외합니다. 따라서 패턴 순서와 중복 항목은 digest를 바꾸지
  않습니다. 새로 발급되는 활성 티켓은 항상 현재 digest를 담습니다. `null`은 과거 기록
  디코딩만 지원하며 활성 선택이나 소비에 유효한
  결속이 아닙니다. 과거에 소비된 티켓은 계속 조회할 수 있습니다.
- `WriteTicket.observed_paths`는 기준 범위에서 비어 있습니다. `detective` 호스트 훅과 세션 감시기 관찰은 티켓에 다시 쓰지 않고 호스트 관찰 및 미기록 변경 기록으로 남깁니다.
- `WriteTicket.control_surface`와 `WriteTicket.guarantee_display`는 현재 Volicord 관찰 요약과 보장 문구를 공개합니다. OS 수준 파일시스템 집행을 주장하지 않습니다.
- `WriteDecisionReason`은 `PrepareWriteResult.write_decision_reasons`에서 사용합니다.

`NextActionSummary` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
| `presentation_role` | 제어되는 표시 역할 값. | [다음 행동 값](schema-value-sets.md#next-action-values)의 `primary` 또는 `additional`을 사용합니다. 선택 사항 여부를 나타내지 않습니다. |
| `action_kind` | 제어되는 행동 범주 값. | [다음 행동 값](schema-value-sets.md#next-action-values)의 값 집합을 사용합니다. 메서드 이름 값이 아닙니다. |
| `owner_method` | 담당 메서드 이름 또는 `null`. | 지원되는 공개 메서드 하나가 다음 행동을 담당할 때 그 API 메서드를 이름 붙입니다. 단일 담당 메서드가 없으면 `null`을 사용합니다. |
| `allowed_operation_categories` | 제어되는 작업 범주 값. | 이 행동에 대해 담당 문서가 지원하는 호출 범주를 나열합니다. `owner_method=null`이거나 지원되는 API 호출 경로가 식별되지 않으면 `[]`를 사용합니다. |
| `label` | 자유 형식 표시 문자열. | 사람과 에이전트가 읽는 표시 문자열이며 기준 값이 아닙니다. 기존 대기 사용자 행동에는 요청 상세가 없는 일반 User Channel 안내를 사용합니다. |
| `blocking_question` | 자유 형식 표시 문자열 또는 `null`. | 행동을 진행하기 전에 풀어야 하는 질문입니다. 기존 대기 사용자 행동에는 항상 `null`이며 요청 전 생성 예외는 위 규칙을 따릅니다. |
| `expected_state_version` | 프로젝트 상태 시계 값 또는 `null`. | 낙관적 동시성을 사용하는 변경 행동에서는 `ToolEnvelope.expected_state_version`으로 매핑합니다. 읽기, `user_only`, 동시성을 사용하지 않는 행동에는 `null`을 사용합니다. |
| `required_refs` | `StateRecordRef[]`. | 다음 행동에 필요한 기록입니다. 필요한 참조가 없으면 `[]`를 사용합니다. 참조는 기록과 맥락을 식별할 뿐 동시성 토큰을 제공하지 않습니다. 기존 대기 사용자 행동 항목은 요청 ref를 제외합니다. |

`WriteTicketAttemptScope` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
| `task_id` | 불투명 식별자. | 포착된 시도 경계의 `Task`를 식별합니다. |
| `change_unit_id` | 불투명 식별자. | 포착된 시도 경계의 Change Unit을 식별합니다. |
| `intended_operation` | 자유 형식의 정확한 동작 좌표. | 대소문자와 내부 텍스트를 보존하고 바깥쪽 공백만 제거한 prepare-write 값을 저장합니다. `performed_operation`을 비교하는 메서드는 정확한 일치를 사용하며, 이 좌표는 외부 동작이 실행됐다는 증명이 아닙니다. |
| `intended_paths` | 정규화된 Product Repository 경로 문자열. | API 수준 경로 정규화 뒤의 Product Repository 상대 경로입니다. |
| `product_file_write_intended` | 불리언. | 포착된 시도가 제품 파일 쓰기를 의도했는지 나타냅니다. |
| `sensitive_categories` | 불투명 민감 범주 분류 문자열. | 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 빠짐없는 공개 enum이 아닙니다. |
| `baseline_ref` | 불투명 기준선 식별자 또는 `null`. | 값이 있을 때 시도 경계에 포착된 기준선 식별자입니다. |

`WriteTicket` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
| `write_ticket_id` | 불투명 식별자. | 쓰기 티켓 권한 기록을 식별합니다. |
| `write_ticket_ref` | `StateRecordRef`. | 같은 쓰기 티켓을 `record_kind=write_ticket`으로 참조합니다. |
| `state` | 제어되는 상태 값. | `WriteTicket.state`에 대해 [메서드 로컬 값](schema-value-sets.md#method-local-values)이 담당하는 값을 사용합니다. |
| `scope` | `WriteTicketScope`. | 티켓 발급에 사용된 Task, Change Unit, 작업, 민감 범주, 제품 쓰기 플래그, 기준선을 포착합니다. |
| `path_patterns` | `WriteTicketPathPatterns`. | 티켓 결정에 대한 허용·거부 정규화 `Product Repository` 경로 패턴을 포착합니다. |
| `observed_paths` | 정규화된 `Product Repository` 경로 문자열. | 담당 문서가 정의한 `detective` 경로가 관찰을 티켓에 연결했을 때만 관찰된 경로를 나열합니다. 연결된 관찰이 없으면 `[]`를 사용합니다. |
| `basis_state_version` | 상태 시계 값. | 발급 또는 재사용 때 포착한 감사 순서이며 티켓 유효성 좌표가 아닙니다. |
| `validity_basis` | `WriteTicketValidityBasis`. | 상태 결합 재사용과 무효화에 사용하는 정확한 Task, Change Unit, 범위, 기준선, workspace, 프로젝트 쓰기 권한, 승인 좌표입니다. |
| `invalidation_reason` | 제어되는 무효화 사유 또는 `null`. | 티켓이 무효화될 때 기록하는 안정된 사유입니다. |
| `idle_expires_at` | UTC 타임스탬프 또는 `null`. | 선택적 프로젝트 정책 idle 경계입니다. `null`은 idle timeout이 없다는 뜻이며 고정 기본 수명은 없습니다. |
| `control_surface` | `ControlSurfaceSummary | null`. | 현재 Volicord 제어 표면 공개입니다. |
| `guarantee_display` | `GuaranteeDisplay | null`. | [보안](../security.md)이 범위를 정하는 사람이 읽는 보장 문구입니다. |

`WriteDecisionReason` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
| `category` | 제어되는 범주 값. | [API 값 집합](schema-value-sets.md#state-and-blocker-values)이 담당하는 `WriteDecisionReason.category` 값 집합을 사용합니다. |
| `code` | 메서드 범위의 불투명 사유 코드. | 전역의 빠짐없는 enum이 아닙니다. 메서드 담당 문서가 로컬 코드를 정의할 수 있지만, 예시 코드는 전역 값이 되지 않습니다. |
| `message` | 자유 형식 표시 문자열. | 사람과 에이전트가 읽는 표시 문자열이며 기준 값이 아닙니다. |
| `related_refs` | `StateRecordRef[]`. | 결정 사유와 관련된 기록입니다. 관련 참조가 없으면 `[]`를 사용합니다. |

`WriteDecisionReason`은 `CloseReadinessBlocker`와 다른 형태입니다.

담당 문서 링크:
- `action_kind` 값: [다음 행동 값](schema-value-sets.md#next-action-values)
- `owner_method` 값: [메서드 이름 값](schema-value-sets.md#method-name-values)
- `WriteTicket.state`와 `WriteTicketStateSummary.status` 값: [메서드 내부 값](schema-value-sets.md#method-local-values)
- `WriteDecisionReason.category` 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values)
- `WriteDecisionReason.code` 값 집합 경계: [불투명 문자열과 메서드 범위 문자열 필드](schema-value-sets.md#opaque-and-method-scoped-string-fields)
- `WriteDecisionReason.code` 생성과 로컬 의미: [`volicord.prepare_write`](method-prepare-write.md)를 포함한 메서드 담당 문서
- 쓰기 티켓 발급 동작: [`volicord.prepare_write`](method-prepare-write.md)
- 쓰기 티켓의 제품 의미와 승인 경계: [Core 모델](../core-model.md)
- 공개 `ErrorCode` 값은 별도입니다: [API 오류 코드](error-codes.md)

<a id="evidence-and-run-snapshot-shapes"></a>
## 증거와 실행 기록 스냅샷 형태

```yaml
AcceptanceCriterionInput:
  statement: string
  evidence_requirement: string

AcceptanceCriterionReplacement:
  acceptance_criterion_id: string | null
  statement: string
  evidence_requirement: string

AcceptanceCriterion:
  acceptance_criterion_id: string
  statement: string
  evidence_requirement: string

EvidenceTarget:
  target_kind: acceptance_criterion | supplemental_claim
  acceptance_criterion_id: string  # acceptance_criterion에서만 사용
  evidence_claim_id: string        # supplemental_claim에서만 사용
  statement: string                # supplemental_claim에서만 사용

ConnectionObservationSourceSelector:
  source_kind: guard_event | session_watcher
  event_kind: pre_tool | post_tool | prompt_capture | stop  # guard_event에서만 사용

EvidenceCaptureSpec:
  capture_kind: verified_command_execution | verified_tool_invocation | registered_connection_observation
  command_sha256: string                       # verified_command_execution에서만 사용
  command_label: string                        # verified_command_execution에서만 사용; 정규화된 1..256 UTF-8 bytes
  expected_exit_code: integer | null           # verified_command_execution에서만 사용
  tool_name: string                            # verified_tool_invocation에서만 사용; 앞뒤 공백 제거, 1..256 UTF-8 bytes
  tool_input_sha256: string                    # verified_tool_invocation에서만 사용
  expected_success: boolean | null             # verified_tool_invocation에서만 사용
  source_selector: ConnectionObservationSourceSelector  # registered_connection_observation에서만 사용
  expected_complete: boolean | null            # registered_connection_observation에서만 사용

EvidenceCaptureIntent:
  capture_intent_id: string
  project_id: string
  task_id: string
  change_unit_id: string
  scope_revision: integer
  baseline_ref: string
  target: EvidenceTarget
  capture: EvidenceCaptureSpec
  input_sha256: string
  expected_outcome: object
  requested_by_actor_source: string
  workspace_context: object
  created_at: string
  expires_at: string

EvidenceCaptureReceipt:
  capture_receipt_id: string
  capture_intent_id: string
  capture_intent_ref: StateRecordRef
  producer_kind: string
  project_id: string
  task_id: string
  change_unit_id: string
  scope_revision: integer
  baseline_ref: string
  target: EvidenceTarget
  input_sha256: string
  result_sha256: string
  expected_outcome: object
  observed_outcome: object
  source_refs: StateRecordRef[]
  connection_id: string
  session_id: string | null
  guard_installation_id: string | null
  guard_event_ids: string[]
  watch_observation_refs: string[]
  staged_receipt_handle: StagedArtifactHandle
  complete: boolean
  limitations: string[]
  redaction_state: string
  observed_by_actor_source: string
  observed_at: string
  recorded_at: string

EvidenceProducer:
  evidence_producer_id: string
  capture_receipt_id: string
  capture_intent_id: string
  capture_intent_ref: StateRecordRef
  producer_kind: string
  project_id: string
  task_id: string
  change_unit_id: string
  scope_revision: integer
  baseline_ref: string
  target: EvidenceTarget
  input_sha256: string
  result_sha256: string
  expected_outcome: object
  observed_outcome: object
  source_refs: StateRecordRef[]
  connection_id: string
  session_id: string | null
  guard_installation_id: string | null
  guard_event_ids: string[]
  watch_observation_refs: string[]
  receipt_artifact_refs: ArtifactRef[]
  complete: boolean
  limitations: string[]
  redaction_state: string
  observed_by_actor_source: string
  observed_at: string
  finalized_at: string
  run_ref: StateRecordRef
  observation_ref: StateRecordRef

EvidenceSummary:
  evidence_state: string
  status: string
  coverage_items: EvidenceCoverageItem[]
  artifact_refs: ArtifactRef[]
  observation_refs: StateRecordRef[]
  updated_by_run_ref: StateRecordRef | null

EvidenceGateSummary:
  state: string

EvidenceCoverageItem:
  target: EvidenceTarget
  coverage_state: string
  supporting_run_refs: StateRecordRef[]
  observation_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_refs: StateRecordRef[]

EvidenceCoverageUpdate:
  target: EvidenceTarget
  coverage_state: string
  provenance: EvidenceUpdateProvenance | null
  supporting_run_refs: StateRecordRef[]
  observation_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_refs: StateRecordRef[]

EvidenceUpdateProvenance:
  source_kind: string
  assurance_level: string
  observed_at: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  source_refs: SourceRef[]
  limitations: string[]

EvidenceObservation:
  observation_id: string
  project_id: string
  task_id: string
  change_unit_id: string | null
  run_ref: StateRecordRef | null
  target: EvidenceTarget
  source_kind: string
  assurance_level: string
  producer_anchor: EvidenceProducerAnchor
  relevance_assessment: EvidenceRelevanceAssessment
  observed_by_actor_source: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  input_refs: StateRecordRef[]
  source_refs: SourceRef[]
  output_artifact_refs: ArtifactRef[]
  limitations: string[]
  observed_at: string
  recorded_at: string

EvidenceProducerAnchor:
  producer_kind: string
  producer_ref: StateRecordRef | null
  output_artifact_refs: ArtifactRef[]
  verification_basis: string | null

EvidenceRelevanceAssessment:
  status: string
  assessment_ref: StateRecordRef | null
  assessed_by_actor_source: string | null

EvidenceObservationInput:
  target: EvidenceTarget
  source_kind: string
  assurance_level: string
  observed_by_actor_source: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  input_refs: StateRecordRef[]
  source_refs: SourceRef[]
  output_artifact_refs: ArtifactRef[]
  limitations: string[]
  observed_at: string

RunSummary:
  run_ref: StateRecordRef
  kind: string
  summary: string
  observed_changes: ObservedChanges
  artifact_refs: ArtifactRef[]

ObservedChanges:
  changed_paths: string[]
  product_file_write_observed: boolean
  sensitive_categories: string[]
  baseline_ref: string | null
```

의미:
- `AcceptanceCriterionInput`은 intake에서 사용하며 ID를 받지 않습니다.
  `AcceptanceCriterionReplacement`는 null이 아닌 update-scope 교체 집합에서만
  사용합니다. 현재 같은 `Task` ID는 identity를 유지하고, `null`은 Core가 새
  ID를 만들도록 요청하며, 교체 집합에서 빠진 이전 현재 기준은 폐기됩니다.
  알 수 없거나, 폐기되었거나, 다른 `Task`에 속하거나, 중복된 ID는 유효하지
  않습니다.
- `AcceptanceCriterion.acceptance_criterion_id`는 Core가 생성한 불투명
  식별자입니다. `statement`는 표시 문구이고 `evidence_requirement`는
  `required`, `optional`, `not_required` 중 하나를 선택합니다.
- `EvidenceTarget`은 엄격한 태그 합집합입니다. `acceptance_criterion` 변형에는
  `acceptance_criterion_id`만 있고, `supplemental_claim` 변형에는 호출자가
  부여한 `Task` 범위 `evidence_claim_id`와 비어 있지 않은 불변 `statement`가
  있습니다. 변형별 필드를 섞을 수 없습니다.
- `ConnectionObservationSourceSelector`는 엄격한 태그 합집합입니다. Guard
  branch는 폐쇄형 `event_kind` 하나를 요구하고, session-watcher branch는 그
  필드를 거부하며 추가 호출자 소유 좌표가 없습니다. 알 수 없는 필드,
  불필요한 필드, branch를 섞은 형태는 유효하지 않습니다. `session_start`는 정확한
  intent-bound session이 intent 전에 반드시 시작했으므로 intent 이후 observation을
  제공할 수 없어 제외합니다.
- `EvidenceCaptureSpec`은 엄격한 태그 합집합입니다. 호출자가 제공하는 소문자
  64자 digest 필드는 정확한 command 또는 tool input을 결합합니다. 등록
  connection 관찰에서는 Core가 canonical `source_selector` JSON에서
  `input_sha256`를 파생합니다. 미래 event/observation identity와 시각, raw-event
  digest, snapshot digest는 intent 필드가 아닙니다. Typed shape의 expected-outcome
  필드는 nullable이며 MCP에서 생략하면 메서드 담당 기본값을 사용합니다.
- `EvidenceCaptureIntent`는 만료되는 불변 current-basis 요청입니다.
  `requested_by_actor_source`와 `workspace_context`는 Core가 파생한 근거 필드이며
  호출자가 선택하는 attribution이 아닙니다. 공개 ref는
  `record_kind=evidence_capture_intent`를 사용합니다.
- `EvidenceCaptureReceipt`는 불변 영속 source-fulfillment fact 레코드입니다. 연결된
  staging handle과 staged receipt bytes만 transient입니다. 등록된
  connection/session/guard/watch 좌표, 정확한 source identity, observation 시각,
  raw-event 또는 snapshot/selection digest, outcome, 완전성, 한계,
  redaction 상태, observer, 시각은 source fact입니다. receipt는
  `StateRecordRef`가 아니며 Core 상태를 진행시키지 않습니다.
- `EvidenceProducer`는 소비 Run 관찰과 일대일로 생성되는 불변 Core-finalized 권한
  레코드입니다. receipt artifact, Run ref, observation ref는 source 바이트,
  producer, relevance를 결합합니다. 공개 ref는
  `record_kind=evidence_producer`를 사용합니다.
- `EvidenceSummary.evidence_state`는 값이 있으면 증거 표시 상태입니다. 아직 첨부된 증거나 현재 닫기 근거 증거 참조가 없는 범위 공백 요약에서는 생략됩니다.
- `EvidenceGateSummary`는 현재 활성 기준과 닫기 평가 근거에 대한 기준 파생 증거 gate 상태 보기입니다. Core는 기준 요구 수준과 범위, 현재 증거 출처, 최신성, 아티팩트 가용성, 증거 관련 닫기 차단 사유를 사용해 한 번 계산합니다. `StateSummary`, status와 close 결과, `SummaryCard.evidence`는 그 결과를 복사하며 독립적으로 다시 계산하지 않습니다. 저장되는 권한 기록이나 `AuthorityReceipt`가 아닙니다.
- `EvidenceSummary.status`, `EvidenceCoverageItem.coverage_state`,
  `EvidenceCoverageUpdate.coverage_state`, `EvidenceUpdateProvenance.source_kind`,
  `EvidenceUpdateProvenance.assurance_level`, `EvidenceObservation.source_kind`,
  `EvidenceObservation.assurance_level`, `EvidenceObservationInput.source_kind`,
  `EvidenceObservationInput.assurance_level`, `RunSummary.kind`는 제어 값
  문자열입니다.
- `RunSummary.summary`, 수락 기준 문장, 보충 주장 문장은 자유 형식 표시
  문자열이며 증거 identity가 아닙니다.
- `EvidenceCoverageUpdate.provenance`는 요청 입력에서 선택적으로 사용할 수
  있으며, Core가 대상이 일치하는 `EvidenceObservation`을 만들거나 연결한 뒤
  커밋된 `EvidenceCoverageItem`에서는 생략됩니다. `supported` 갱신에는 대상이
  일치하는 관찰 입력, 사용할 수 있는 대상 일치 관찰 참조, 또는 Core가
  관찰을 만들 수 있게 하는 이 출처 객체가 필요합니다.
- `supporting_run_refs`는 같은 `Task`의 Run 참조를 받습니다.
  `observation_refs`, `supporting_artifact_refs`, `gap_refs`는 대상별 관찰,
  아티팩트, 공백 관계를 보존합니다.
- `EvidenceSummary.observation_refs`와 `EvidenceCoverageItem.observation_refs`는 Core가 요약이나 대상과 관련지은 커밋된 증거 관찰에 대한 `StateRecordRef` 값을 나열합니다.
- `EvidenceObservation`은 하나의 증거 대상에 대한 영속 출처 기록입니다.
  `producer_anchor`는 Core가 검증한 producer 레코드와 정확한 출력을 별도로
  식별하고, `relevance_assessment`는 권한 출처가 해당 출력과 대상의 관련성을
  평가했는지 별도로 식별합니다. 바이트 무결성, producer provenance, 근거
  freshness, 대상 identity, claim relevance는 서로 다른 검사입니다.
- `evidence_observation` `UserActionResolution`은 User Channel 소유의 대상 및 근거
  결합 relevance 레코드입니다. 닫힌 중첩 관찰 본문이 정확한 정규 아티팩트 ref를
  결합합니다. 저장된 정확한 relevance가 `supported` 또는 `contradicted`이면
  user-observed producer provenance를 설정하면서 그 상태를 분리된 relevance 평가에
  그대로 보존합니다. 커밋된 관찰은 호출자의 `EvidenceObservationInput.observed_at`이
  아니라 바깥 resolution의 `resolved_at`을 `observed_at`으로 사용합니다.
  `contradicted` 관찰은 부정적 relevance이며 supported coverage, 증거 충분성,
  `supported`를 세우는 검증된 재사용을 만족할 수 없습니다. 판단 resolution이나 최종
  수락은 아닙니다. 공개 형태는 [API 사용자 행동 스키마](schema-user-action.md)가
  담당합니다.
- `source_refs`는 `SourceRef`를 사용합니다. `input_refs`는 별도 `StateRecordRef[]`로 유지되며 출처 참조는 Core 상태 참조나 닫기 근거 결과 참조가 되지 않습니다.
- `EvidenceObservationInput`은 `volicord.record_run`이 받는 요청 측 형태입니다. Core는 커밋할 때 `observation_id`, 프로젝트와 `Task` 좌표, `run_ref`, `recorded_at`, 관찰자 행위자 출처를 채웁니다. 요청 측 출처와 보장 수준 값은 출처 주장이지 호출자가 부여하는 보장이 아닙니다.
- `evidence_requirement=required`인 현재 기준의 범위만 닫기 권한에 참여합니다.
  필요한 기준은 `coverage_state=not_applicable`을 거부하며, `optional`,
  `not_required`, 보충 대상, 폐기된 대상은 닫기에 권한 효력이 없습니다.
- 제출된 `observed_by_actor_source`는 커밋할 행위자를 선택하지 않습니다. Core는
  검증된 producer 레코드가 있으면 그 레코드에서, 그렇지 않으면 확인된 호출에서
  값을 파생합니다. 제출 값으로 신뢰를 높이거나 다른 actor를 가장할 수 없습니다.
- Core는 확인된 앵커에서 커밋할 `source_kind`와 `assurance_level`을 파생합니다. 앵커가 없는 직접 `connection_observation`, `user_observation`, `external_tool`, 호출자 선언 `reused_evidence` 입력은 `agent_report` / `cooperative_report`로 커밋됩니다. 이 필드 자체는 제품 정확성을 증명하거나, 사용자 권한을 부여하거나, 최종 수락이나 잔여 위험 수락을 만족하거나, `GuaranteeDisplay.level`을 높이지 않습니다.
- 현재의 완전한 capture receipt를 정확히 하나의 `evidence_capture_intent` input
  ref로 소비하면 authority-owned verified command, verified tool, registered
  connection-observation producer를 설정할 수 있습니다. 해당 앵커가 없는 직접
  `external_tool` 또는 `connection_observation` 입력은 아티팩트 바이트가
  검증되어도 협력적으로 남습니다.
- `user_observation`은 현재 `evidence_observation` `UserActionResolution`, 정확한 출력
  일치, 저장된 정확한 `relevance_status`인 `supported` 또는 `contradicted`, 검증된
  로컬 사용자 provenance, 일치하는 Task, Change Unit, scope, baseline, 대상을
  요구합니다. Core는 그 정확한 상태를 `relevance_assessment`에 보존하고 resolution의
  `resolved_at`에서 `observed_at`을 파생합니다. `supported`만 coverage나 충분성을
  만족하거나 `supported`를 세우는 검증된 재사용 자격을 얻을 수 있습니다.
- `reused_evidence`는 각 재귀 관찰의 엄격한 저장 producer/relevance
  메타데이터, 정확한 출력, 대상, 현재 근거, 출처 Run, 승계 보장을 다시 검증한
  뒤에만 Core가 파생합니다.
- `unverified_claim`과 `unverified`는 확인된 관찰 없는 주장을 보존하며 그 자체로 충분한 증거가 아닙니다.
- `tool_metadata`는 설명용 메타데이터이며 권한, 승인, 저장 효과로 취급하면 안 됩니다.
- `ObservedChanges.changed_paths`는 경로 문자열입니다.
- `ObservedChanges.sensitive_categories`는 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 범주 분류 문자열입니다.
- `ObservedChanges.baseline_ref`는 불투명 기준선 식별자입니다.

담당 문서 링크:
- `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md)
- 증거, `coverage_state`, 증거 관찰, 실행 종류 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values), [증거 관찰 값](schema-value-sets.md#evidence-observation-values), [메서드 내부 값](schema-value-sets.md#method-local-values)
- 증거 관찰 행위자 값: [행위자 값](schema-value-sets.md#actor-values)
- 증거 충분성의 의미: [Core 모델의 실행 기록과 증거의 권한](../core-model.md#9-evidence-and-run-authority)
- 메서드 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서

<a id="close-readiness-and-validation-shapes"></a>
## 닫기 준비 상태와 검증 형태

```yaml
CurrentCloseBasis:
  close_basis_revision: integer
  scope_revision: integer
  task_id: string
  change_unit_id: string
  baseline_ref: string | null
  result_summary: string
  result_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  residual_risks: ResidualRisk[]
  sensitive_categories: string[]
  sensitive_action_requirements: SensitiveActionRequirement[]
  recovery_constraints: string[]
  source_run_ref: StateRecordRef
  updated_at: string

SensitiveActionRequirement:
  action_kind: string
  normalized_paths: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  change_unit_id: string
  source_run_ref: StateRecordRef
  source_write_ticket_ref: StateRecordRef

ResidualRisk:
  risk_id: string
  summary: string
  consequence: string
  acceptance_required: boolean
  source_refs: StateRecordRef[]

RiskAcceptanceCoverage:
  risk_id: string
  accepted: boolean
  accepted_by_user_action_resolution_refs: StateRecordRef[]
  missing_reason: string | null

CloseReadinessBlocker:
  category: string
  code: string
  message: string
  control_surface: ControlSurfaceSummary | null
  can_resolve_in_chat: boolean
  outside_chat_action_required: boolean
  related_refs: StateRecordRef[]
  next_actions: NextActionSummary[]

ValidatorResult:
  validator_id: string
  status: string
  severity: string | null
  message: string
  related_refs: StateRecordRef[]

GuaranteeDisplay:
  level: string
  basis: string
  capability_refs: StateRecordRef[]

GuaranteeDisclosure:
  guarantee_class: string
  guarantees: string[]
  non_guarantees: string[]
```

의미:
- `CurrentCloseBasis`는 닫기 준비 상태 응답이 사용하는 현재 결과와 잔여 위험 상태입니다. 종료 닫기 요약이 아닙니다.
- `close_basis_revision`과 `scope_revision`은 호환성 확인을 위해 드러나는 내부 현재 상태 좌표입니다. 호출자가 선택하는 권한이 아닙니다.
- `ResidualRisk.risk_id`는 Core가 생성한 불투명 식별자입니다. `ResidualRisk.summary`와 `ResidualRisk.consequence`는 표시 문자열이며 텍스트 일치를 권한으로 만들지 않습니다.
- `result_refs`, `source_run_ref`, `source_refs`, `evidence_summary_ref`, `accepted_by_user_action_resolution_refs`는 `StateRecordRef`를 사용합니다.
- `sensitive_categories`는 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 범주 분류 문자열입니다.
- `sensitive_action_requirements`는 커밋된 실행 기록과 소비된 쓰기 티켓에서 Core가 파생한 닫기 요구사항입니다. 범주만 담은 호출자 입력은 이 요구사항을 만들거나 지울 수 없습니다.
- `recovery_constraints`와 `RiskAcceptanceCoverage.missing_reason`은 표시 문자열입니다. 현재 닫기 준비 상태 결과는 필요한 수락이 없으면 `acceptance_required`를 사용하고, 현재 잔여 위험 `risk_id` 값을 덮지 못하는 오래된 잔여 위험 수락이 있으면 `stale_acceptance`를 사용할 수 있습니다.
- `RiskAcceptanceCoverage`는 현재 잔여 위험 요구사항이 호환되는 사용자 작업 resolution으로 덮였는지를 보고합니다. 증거 충분성이나 최종 수락을 보고하지 않습니다.
- `CloseReadinessBlocker`는 닫기 차단 사유를 표현하는 데이터 형태입니다.
- `CloseReadinessBlocker.category`는 제어 값 문자열입니다.
- `CloseReadinessBlocker.code`는 담당 문서가 정의하는 차단 사유 코드입니다. 차단 사유 또는 메서드 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 빠짐없는 전역 공개 enum이 아닙니다.
- `CloseReadinessBlocker.control_surface`는 `guard_*` 연결 역량 차단 사유에 있을 수 있으며, 차단 사유를 계산한 시점의 관찰 요약을 보고합니다. `GuardHealthSummary`의 훅 상태에서 도출하지 않은 차단 사유에서는 생략됩니다.
- `can_resolve_in_chat`은 메서드 담당 문서가 그 경로를 알고 있을 때 차단 사유를 채팅으로 매개되는 사용자 경로에서 해소할 수 있는지를 보고합니다.
- `outside_chat_action_required`는 다음 행동에 채팅 밖의 터미널, 호스트, 파일시스템, 설정 작업이 필요하다는 사실을 담당 문서가 알고 있는지를 보고합니다.
- `can_resolve_in_chat`과 `outside_chat_action_required`는 서로 독립된 공개 정보이며 논리적 보수가 아닙니다. 둘 다 `false`이면 어느 경로 주장도 확정되지 않았다는 뜻이며 행동이 필요 없다는 뜻이 아닙니다.
- `CloseReadinessBlocker.message`, `ValidatorResult.message`, `GuaranteeDisplay.basis`는 자유 형식 표시 문자열입니다.
- `ValidatorResult.validator_id`는 값 집합 담당 문서가 지원되는 안정 값을 공개하기 전까지 보고용 라벨입니다.
- `ValidatorResult.status`, `ValidatorResult.severity`, `GuaranteeDisplay.level`은 제어 값 문자열입니다.
- `GuaranteeDisclosure`는 독자가 결과를 과대 해석할 수 있는 공개 결과 base와 진단 출력에 반환되는 결과 해석 공개입니다.
- `GuaranteeDisclosure.guarantee_class`와 `GuaranteeDisclosure.non_guarantees`는 제어 값 문자열입니다. `GuaranteeDisclosure.guarantees`는 짧은 표시 문장입니다.
- `GuaranteeDisplay`는 상태 또는 호환성 보기의 현재 기능 표시를 설명합니다. `GuaranteeDisclosure`를 대체하지 않습니다.

이 형태들은 닫기 준비 상태 의미, 응답 처리 경로, 지속 동작을 정의하지 않습니다.

닫기 근거 참조 규칙:
- `CurrentCloseBasis.result_refs`나 `ResidualRisk.source_refs`로 받아들일 수 있는 호출자 제공 닫기 평가 참조는 담당 문서가 다른 종류를 명시적으로 추가하지 않는 한 결과/증거 기록 종류인 `run`, `artifact`, `evidence_summary`, `change_unit`으로 제한됩니다.
- 담당 문서가 명시적으로 추가하지 않는 한 `project_state`, `write_ticket`, `user_action_request`, `user_action_resolution`, `blocker`, `task_event`, `task`는 호출자 제공 결과 참조가 아닙니다.
- 받아들인 모든 참조는 존재해야 하고 같은 프로젝트와 `Task`에 속해야 하며 Core가 정규화해야 합니다. Core는 호출자가 보낸 `produced_at_state_version` 메타데이터를 권한이나 동시성 입력으로 취급하지 않습니다.
- 닫기 증거에 쓰이는 아티팩트 참조는 `Task`에 연결되어 있고 `integrity_status=verified`여야 하며 [아티팩트 저장소](../storage-artifacts.md)에 따라 사용 시점의 현재 바이트 검증을 통과해야 합니다.
- 증거 참조는 현재 `Task` 증거 요약을 식별해야 합니다. 현재 닫기 근거 결과 참조로 쓰이는 실행 기록 참조는 현재 `Task`, 현재 적용 Change Unit, 현재 범위 리비전, 호환되는 기준선, 기록된 상태와 호환되는 기록된 현재 실행 기록을 식별해야 합니다. 이력 실행 기록은 현재 실행 기록이 그 `verified` 아티팩트나 증거를 명시적으로 재사용하고 그 재사용을 기록하지 않는 한 감사 기록입니다.
- Core는 기준 닫기 근거를 구성하면서 현재 실행 기록, 현재 Change Unit, 현재 EvidenceSummary 참조를 추가할 수 있습니다.

보장 표시 규칙:
- `GuaranteeDisplay`는 프로젝트 강제 프로필, 확인된 호출 맥락, 활성화된 강제 메커니즘, 지원되는 기준 범위에서 파생됩니다.
- `capability_refs`는 표시를 정당화하는 참조를 담는 구현 필드 이름입니다. 기준 연결 아키텍처에서는 사용할 수 있으면 호출 바인딩, Agent Connection, 관찰 사실을 인용해야 합니다.
- 협력형 전용 배포는 `detective`를 주장하면 안 됩니다.
- `detective`는 관찰 범위에 대한 지원되는 강제 또는 관찰 사실을 요구하며, 호스트 지침, 연결 모드, 생성된 텍스트만으로는 부족합니다.
- 별도 지원 관찰이 그 표시를 정당화하지 않는 한 협력형 `agent_report` `Run`이나 관찰을 `detective` 또는 외부 관찰로 표시하지 않습니다.

담당 문서 링크:
- 닫기 준비 상태 의미와 대체 금지 규칙: [Core 모델의 닫기 준비 상태](../core-model.md#close_task)
- 현재 닫기 근거 생성: [`volicord.record_run`](method-record-run.md)
- 판단 호환성과 수락된 위험 입력: [API 판단 스키마](schema-judgment.md)
- 응답 분기 동작, 닫기 준비 상태 평가 순서, 응답 전용 차단 결과: [`volicord.check_close`와 `volicord.close_task`](method-close-task.md)
- 닫기 차단 사유와 API 응답 분기 사이의 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md)
- 차단 사유 범주 값(`CloseReadinessBlocker.category`), 지원되는 `ValidatorResult.status`, `ValidatorResult.severity`, `GuaranteeDisplay.level` 값: [API 값 집합](schema-value-sets.md#state-and-blocker-values)
- 보안 보장 의미: [보안](../security.md)

## 관련 담당 문서

- [API 코어 스키마](schema-core.md): `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`.
- [API 값 집합](schema-value-sets.md#state-and-blocker-values): 차단 사유 범주 값(`CloseReadinessBlocker.category`)과 인접 상태 값.
- [API 메서드](methods.md)와 메서드 담당 문서: 이 스키마를 반환하는 메서드.
- [API 아티팩트 스키마](schema-artifacts.md): `ArtifactRef`.
- [API 사용자 행동 스키마](schema-user-action.md): 영속 행동 요청과 캡처 양식.
- [저장 효과](../storage-effects.md): 지속 저장과 상태 효과.
