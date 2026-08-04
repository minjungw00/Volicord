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

```schema
StateRecordRef:
  record_kind: string
  record_id: string
  project_id: string
  task_id: string | null
  produced_at_state_version: integer | null

UserActionResolutionRef:
  record_kind: user_action_resolution
  record_id: string
  project_id: string
  task_id: string
  produced_at_state_version: integer
```

`UserActionResolutionRef`는 승인 참조 전용 형태입니다. `record_kind`는
`user_action_resolution`으로 고정되며, 필수 `project_id`, `task_id`,
`record_id`가 완전한 resolution identity를 이룹니다.
필수 `produced_at_state_version`은 이 참조를 만든 projection의 구체적인 프로젝트
상태 버전이며 identity에는 참여하지 않습니다.

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

```schema
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

```schema
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
  workflow: WorkflowProjection
  pending_user_action_summaries: AgentSafeUserActionRequestSummary[]
  blocker_refs: StateRecordRef[]
  write_ticket_summary: WriteTicketStateSummary | null
  evidence_summary: EvidenceSummary | null
  evidence_gate: EvidenceGateSummary | null
  close_state: string | null
  close_blockers: CloseReadinessBlocker[]
  guarantee_display: GuaranteeDisplay | null
```

의미:
- `StateSummary`는 상태 참조, 요약, 닫기 준비 상태 필드를 담는 간결한 응답 형태입니다.
- 메서드의 `include` 플래그는 이 형태의 일부만 선택할 수 있습니다. 메서드 담당 문서가 어떤 상태 보기를 선택하지 않는다고 말하면 `evidence_summary`, `evidence_gate`, `close_state`, `close_blockers`, `guarantee_display` 같은 `include` 제어 필드는 `null`이나 빈 값으로 반환하지 않고 생략합니다. 반환된 빈 배열은 그 상태 보기를 계산했고 비어 있음을 뜻합니다.
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

```schema
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
```

의미:

- `TaskLineageSummary`는 predecessor 관계 하나를 기록합니다. `applied`
  carry-forward는 새로 검증된 Task 입력이 되고 `reference_only`는 이전 권한을
  현재 상태로 만들지 않은 채 predecessor 맥락만 보존합니다.
- `TaskFlowItem[]`은 full status가 연결된 predecessor component를 표시하는
  파생 보기이며 새 parent-goal 레코드가 아닙니다.
- `ProjectWorkflowPolicySummary.policy_schema`는 `volicord.workflow_policy`입니다.
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
  설명합니다. `next_actor`는 간결한 행위자 분류이며 현재 메서드 진행은 태그 기반
  `StateSummary.workflow`에 남습니다. receipt 자체는 커밋, 닫기, 수락, 제품 정확성
  증명이 아닙니다.

<a id="unrecorded-change-reconciliation-shapes"></a>
## 미기록 변경 조정 형태

`UnrecordedChangeFinding`은 `volicord.reconcile_changes`가 해결되지 않은 미기록
Product Repository 변경에 대해 반환하는 공개 형태입니다.

`UnrecordedChangeResolutionSummary`는 조정 호출 하나가 해결한 미기록 변경의 공개
요약 형태입니다.

```schema
UnrecordedChangeFinding:
  unrecorded_change_ref: StateRecordRef
  status: string
  summary: string
  observed_paths: string[]
  detected_at: string
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
- 모든 미해결 finding은 완전히 관찰된 비어 있지 않은 불일치 저장소 delta를
  근거로 하며 닫기 차단 사유입니다. 기준선, 결과, delta, 불일치 delta의 의미는
  [저장소 관찰](../repository-observation.md)이 담당합니다.
- `summary`, `capture_basis`, `next_action.label`은 표시 문자열이며 정확성 증명이 아닙니다.
- `observed_paths`는 정확한 관찰의 불일치 delta에서 decode한 비어 있지 않은 정규
  Product Repository 상대 경로 집합을 담습니다. 프롬프트 텍스트, 명령 텍스트,
  셸 인수, 전체 민감 내용을 포함하지 않습니다.
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

```schema
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

ContinuityPageRequest:
  page_size: integer
  cursor: ContinuityCursor | null

ContinuityCursor:
  updated_at: string
  continuity_record_id: string

ProjectContinuityPage:
  items: ProjectContinuitySummary[]
  page_info: ContinuityPageInfo

ContinuityPageInfo:
  total_count: integer
  returned_count: integer
  truncated: boolean
  next_cursor: ContinuityCursor | null
```

의미:
- 프로젝트 연속성 기록은 원천 `Task`가 닫힌 뒤에도 유지해야 하는 결정, 의무, 알려진 한계, 수락된 잔여 위험, 제약 같은 프로젝트 수준 맥락을 보존합니다.
- `source_task_id`와 `source_change_unit_id`는 기록이 어디에서 비롯되었는지를 식별합니다. 원천 `Task`나 Change Unit을 다시 현재 상태로 만들지는 않습니다.
- `applies_to_paths`, `applies_to_refs`, `source_refs`, `artifact_refs`, `supersedes_refs`, `review_triggers`는 이후 검토를 위한 제한된 맥락입니다. 빈 배열은 그 필드에 항목이 없다는 뜻입니다.
- `ProjectContinuitySummary`는 메서드 담당 문서가 선택하는 읽기 보기이며, 전체 지속 기록이 아닙니다.
- `ContinuityPageRequest.page_size`는 1 이상 64 이하의 정수입니다.
  `ContinuityCursor`는 닫힌 정렬 객체이며 기록 조회나 권한 참조가 아닙니다. 두
  멤버는 모두 필수이고 정확한 정규 저장 값을 사용하며 `updated_at DESC,
  continuity_record_id DESC` 순서의 배타적 위치를 식별합니다.
- `ProjectContinuityPage.items`는 요청 page size 이하의 항목을 담습니다.
  `total_count`는 cursor 적용 전 선택 프로젝트의 전체 활성 기록 수이고,
  `returned_count`는 `items.len`과 같으며, `truncated`는 뒤에 항목이 더 있을 때만
  true입니다. `next_cursor`는 `truncated=true`일 때만 null이 아니며 마지막 항목의
  저장된 `updated_at`과 `continuity_record_id`를 복사합니다.

의미하지 않는 것:
- 프로젝트 연속성 기록은 현재 `Task` 권한, 증거, 쓰기 티켓, 최종 수락, 닫기 준비 상태, 미래 닫기 근거의 잔여 위험 수락, 차단 사유 면제가 아닙니다.
- `status=active`는 그 연속성 기록이 살아 있는 프로젝트 맥락이라는 뜻입니다. 모든 `Task`에 현재 적용된다거나 원천 결정이 새 권한 확인에 충분하다는 뜻은 아닙니다.

담당 문서 링크:
- `kind`와 `status` 값: [프로젝트 연속성 값](schema-value-sets.md#project-continuity-values)
- 저장소 계열과 JSON 배치: [저장소 기록](../storage-records.md)
- 메서드별 생성 효과: [저장 효과](../storage-effects.md)

## `ChangeUnitEffectContract`

`ChangeUnitEffectContract`는 Change Unit에 기록되는 선택적 효과 경계 객체입니다.

```schema
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

```schema
TaskLifecycleState:
  lifecycle_phase: string
  close_reason: string
  result: string
  closed_at: string | null
```

담당 문서 링크:
- `lifecycle_phase`, `close_reason`, `result`의 지원 값: [`Task` 생명주기 값](schema-value-sets.md#task-lifecycle-values)
- 생명주기 영역의 제품 의미: [Core 모델의 `Task` 생명주기](../core-model.md#6-task-lifecycle)

## `WorkflowProjection`과 shaping checkpoint

`StateSummary.workflow`는 단일 태그형 진행 권한입니다. `kind` 값은 `no_active_task`, `shaping_required`, `awaiting_user_action`, `decision_recovery_required`, `ready_to_apply_decisions`, `ready_for_change_unit`, `ready_to_finalize_advice`, `ready_for_implementation`, `implementation`, `close_review`, `terminal` 중 하나입니다.

```schema
WorkflowProjection:
  kind: string
  next_actor: string
  required_action: string | null
  allowed_actions: string[]
  required_refs: StateRecordRef[]
  expected_state_version: integer
  blocking_reason: string | null
  checkpoint: ShapingCheckpointSummary | null
  action_catalog: WorkflowActionCatalog

WorkflowActionCatalog:
  required_method: string | null
  actions: WorkflowActionIntent[]

WorkflowActionIntent:
  method: string
  role: required | allowed
  expected_state_version: integer
  fixed_authority_coordinates: WorkflowActionAuthorityCoordinates
  required_refs: StateRecordRef[]

ShapingUserActionDraft:
  action: UserActionDraft
  expires_at: string | null

ShapingGapInput:
  gap_kind: string
  summary: string
  affected_refs: StateRecordRef[]
  user_action: ShapingUserActionDraft | null

ShapingCheckpointOperation:
  # initial variant
  operation: create_initial

  # replacement variant
  operation: replace_current
  expected_current_checkpoint_id: string
  retired_non_authorizing_request_refs: StateRecordRef[]
  carry_forward_application_refs: StateRecordRef[]
  stale_authority_actions: StaleShapingAuthorityAction[]

StaleShapingAuthorityAction:
  # retirement variant
  action: retire
  stale_application_ref: StateRecordRef

  # reauthorization variant
  action: reauthorize
  stale_application_ref: StateRecordRef
  successor_gap: ShapingGapInput

ShapingCheckpoint:
  shaping_checkpoint_id: string
  predecessor_checkpoint_id: string | null
  project_id: string
  task_id: string
  scope_revision: integer
  baseline_ref: string | null
  summary: string
  implementation_boundary: string | null
  readiness: string
  source_refs: SourceRef[]
  evidence_refs: StateRecordRef[]
  created_at: string
  superseded_at: string | null

ShapingCheckpointSummary:
  checkpoint_ref: StateRecordRef
  predecessor_checkpoint_ref: StateRecordRef | null
  readiness: string
  scope_revision: integer
  baseline_ref: string | null
  implementation_boundary: string | null
  current_application_refs: StateRecordRef[]
  gaps: ShapingCheckpointGap[]
  pending_decision_refs: StateRecordRef[]
  unresolved_application_owners: string[]
  decision_recovery_requirements: ShapingDecisionRecoveryRequirement[]

ShapingCheckpointGap:
  shaping_gap_id: string
  gap_kind: string
  application_owner: string | null
  summary: string
  affected_refs: StateRecordRef[]
  status: string
  decision_authority_state: string | null
  user_action_request_ref: StateRecordRef | null
  user_action_resolution_ref: StateRecordRef | null
  reauthorizes_application_ref: StateRecordRef | null

ShapingDecisionRecoveryRequirement:
  shaping_gap_id: string
  user_action_request_ref: StateRecordRef
  user_action_resolution_ref: StateRecordRef | null
  disposition: string
  reason: string

ShapingDecisionApplication:
  shaping_decision_application_id: string
  project_id: string
  task_id: string
  source_checkpoint_id: string
  source_gap_id: string
  user_action_request_id: string
  user_action_resolution_id: string
  judgment_kind: string
  application_owner: string
  applied_scope_revision: integer
  applied_baseline_ref: string
  applied_change_unit_id: string | null
  applied_at: string
  authority_status: string
  stale_at: string | null
  superseded_at: string | null

ShapingAuthorityReauthorization:
  shaping_authority_reauthorization_id: string
  project_id: string
  task_id: string
  stale_application_id: string
  stale_user_action_request_id: string
  successor_checkpoint_id: string
  successor_gap_id: string | null
  successor_user_action_request_id: string | null
  outcome: string
  created_at: string
```

`ShapingCheckpoint`는 `volicord.record_shaping_checkpoint`이 반환하는 일급 영속 기록입니다.
workflow는 현재 checkpoint의 간결한 요약과 gap projection을 포함합니다. 교체는 영속
record에 정확한 predecessor identity를 담고 엄격한 checkpoint-application lineage로
완전하고 명시적인 `current_application_refs` 집합을 전달합니다. `applied` gap만이 아니라
`ShapingDecisionApplication`이 영속 권한 record입니다. 변경 불가능한 source와 적용 좌표는
감사 이력을 보존하고 `authority_status`가 current, stale, superseded 무효화를 명시적으로
소유합니다.
`ShapingGapInput.user_action`은 사용자 소유 gap에만 null이 아니며, Core가 원자적으로
materialize하고 연결하는 호환 typed draft를 담습니다. Readiness, gap kind, gap status,
workflow kind, blocking reason은 [API 값 집합](schema-value-sets.md)의 폐쇄형 집합을
사용합니다.
`ShapingAuthorityReauthorization`은 변경 불가능한 감사 lineage입니다. `retired`
outcome의 successor gap/request identity는 null이고, `reissued` outcome은 둘 다 가지며
항상 새 unresolved 요청을 가리킵니다.
`ShapingCheckpointOperation`은 하나의 폐쇄형 tagged union입니다. 교체에는 현재 호환
carry-forward 집합 전체와 stale application action 집합 전체가 필요합니다. `retire`는
successor 요청 없이 stale 권한 경로 하나를 끝냅니다. `reauthorize`는
`reauthorizes_application_ref`로 stale application을 지정하는 새 successor gap과
unresolved 요청을 만들며, 이전 accepted resolution을 새 요청으로 넘기지 않습니다.

Checkpoint readiness는 구조적이며 decision application과 독립적입니다.
`application_owner`는 사용자 소유 gap일 때만 null이 아닙니다.
`unresolved_application_owners`는 수락되었지만 아직 적용되지 않은 결정 owner의 고유하고
안정적인 집합입니다. `readiness=ready`여도 비어 있지 않을 수 있습니다. 이 집합에
`volicord.update_scope`가 있을 때만 `ready_to_apply_decisions`를 선택합니다.
`decision_recovery_requirements`는 정확한 각 거부·보류·만료 요청, 존재하는 변경 불가능한
resolution, authority disposition, 타입이 정해진 reason을 식별합니다. 이 값이 있으면 구조적
readiness가 `ready`여도 `next_actor=agent`, `required_action=volicord.record_shaping_checkpoint`인
`decision_recovery_required`를 선택합니다. Work의
advance owner 결정은 Change Unit 또는 `ready_for_implementation` 방향으로 진행합니다.
Advisor finalization owner 결정은 비쓰기 Change Unit과 `ready_to_finalize_advice` 방향으로
진행하며 현재 checkpoint 기반 close basis가 있어야만 `close_review`를 선택합니다.

workflow projection은 현재 진행 상태에서 최대 하나의 필수 메서드를 선택합니다. 진행 권한은 태그가 있는 `required_action`이며 최상위 action 또는 blocker 배열 항목의 위치가 아닙니다. 닫기 차단 사유는 자체 해결 행동을 유지하지만 이 필수 행동을 선택하지 않습니다. 사용자 소유 current gap은 정확한 현재 UserAction 요청 참조를 항상 포함하며 대화 presentation은 그 요청을 해결하지 않습니다. 진행은 `advance_task`, `finalize_advice`, shaping 소유 scope update, write preparation, Run 기록, 닫기 준비 상태, mutation 거부에 Store 소유 현재 유효 shaping 권한 graph를 사용합니다. Ancestor에서 호환되게 carry-forward된 application은 source gap을 복사하지 않아도 `applied`로 유지됩니다. Stale application은 권한을 부여하지 않고 현재 복구 의무로만 나타납니다. `advisor|work` shaping에서는 `shaping_required`, `next_actor=agent`, `required_action=volicord.record_shaping_checkpoint`, `blocking_reason=application_authority_stale`, 정확한 복구 ref를 선택합니다. 이 상태를 만들 implementation 단계 update는 mutation 전에 거부되고 Task를 shaping으로 돌리는 대신 close/supersede 복구로 `volicord.close_task`를 지정합니다. 현재 graph 내부의 모순은 `inconsistent_authority_state`를 사용합니다. Superseded 요청, resolution, application, checkpoint ref는 변경 불가능한 감사 이력으로 남으며 존재한다는 이유만으로 현재 `required_refs`나 진행에 들어가지 않습니다.

`action_catalog`에는 `allowed_actions`의 Task 상태 결속 메서드마다 중립 action intent가
정확히 하나씩 들어가며 그 밖의 메서드 intent는 들어가지 않습니다. Entry는 정규 메서드
이름 순으로 정렬됩니다. `required_method`는 Task 상태 결속 `required_action`이며, 필수
행동이 읽기 전용이거나 User Channel 소유이면 null입니다. 필수 entry는
`role=required`, 나머지는 `role=allowed`입니다. 메서드 중복, 필수 entry 누락, 메서드와
좌표 종류 불일치, 비정규 순서는 유효하지 않습니다. 모든 entry는 동일한 현재 Task
snapshot에서 나온 workflow 상태 버전과 Core 소유 고정 권한 좌표를 사용합니다. 첫
checkpoint 좌표는 실제 null 기준선을 보존하고, 교체 좌표는 정확한 현재·선행 checkpoint
참조, retirement 참조, 호환 application 참조, stale application 참조를 담습니다. 다른
좌표는 해당 메서드에 적용되는 정확한 현재 Task, checkpoint, Change Unit, 범위 리비전,
기준선, resolution, 메서드 내부 권한 사실에 결속됩니다. MCP는 각 중립 intent를 실행
가능한 메서드별 action form으로 투영할 수 있지만 MCP form과 입력 slot은 Core 상태가
아닙니다.

MCP checkpoint 제출의 정규 compare-and-swap 좌표는
`checkpoint_operation.expected_current_checkpoint_id`입니다. 이 workflow projection의
현재·선행 checkpoint 참조는 문맥 계보를 설명하며 최상위 mutation 인자로 중복되지
않습니다.

Workflow mutation 거부 상세는 수신 payload에서 progression을 재구성하지 않고 동일한 완전한
tagged `WorkflowProjection`을 포함합니다. `allowed_actions`, blocker ref, 정확한 Task
mode/work phase, 단일 recovery owner는 현재 authority에서 읽습니다. 거부된 요청의 내장
`expected_state_version`은 커밋된 replay 결과가 아닙니다. 나중 replay는 그 시점의 현재
authority에 대해 다시 평가되고 그 시점의 current workflow를 반환합니다.

담당 문서 링크:
- 메서드 동작과 지속 효과: [API 메서드](methods.md)가 안내하는 메서드 담당 문서와 [저장 효과](../storage-effects.md)

<a id="current-position-display-shapes"></a>
## 현재 위치 표시 형태

```schema
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
  write_authority_fingerprint: string
  approval_basis_refs: UserActionResolutionRef[]

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
  guarantee_display: GuaranteeDisplay | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

의미:
- `SummaryCard`는 주요 사용자 대상 상태 보기에 쓰는 안정적인 간결 요약 형태입니다. `Task`, `Recording`, `Profile`, `Write Ticket`, `Evidence`, `User Judgment`, `Changes`, `Close Status`, `Transport`, 다음 행동 하나, 짧은 `Guarantee` 줄에 공개 표시 문자열을 사용합니다.
- 반환된 `SummaryCard`의 표시 문자열과 반환된
  `NextActionSummary.label`, `NextActionSummary.blocking_question` field는 해당
  응답의 규범적 공개 API 값입니다. CLI 또는 MCP adapter는 이 값을 감쌀 수 있지만,
  명령 문법, transport framing, terminal 또는 Markdown styling, adapter 전용 설명
  문구는 adapter presentation이며 Core가 반환하는 표시 문자열이 아닙니다.
- 증거 또는 닫기 상태 보기를 선택하면 `SummaryCard.evidence`는 [API 값 집합](schema-value-sets.md#evidence-gate-values)이 담당하는 `EvidenceGateSummary.state` 값을 그대로 사용합니다. 스테이징 입력이나 `EvidenceSummary.evidence_state`에서 별도 상태를 추론하지 않습니다.
- `SummaryCard.next`는 표시 힌트일 뿐입니다. `SummaryCard.next_action`은 대응하는 구조화된 `NextActionSummary`를 담을 수 있으며 구조화된 행동이 적용되지 않으면 생략될 수 있습니다. 두 필드 모두 워크플로 권한이 아니며 닫기 차단 사유는 서로 독립된 fact입니다.
- `SummaryCard`는 담당 문서가 선택한 다른 상태 필드의 요약이지 두 번째 권한 기록이 아닙니다. 표시된 다음 행동에 식별자가 필요하지 않은 한 내부 식별자를 추가하면 안 됩니다.
- 이미 존재하는 대기 사용자 행동에는 `SummaryCard.user_action`, `SummaryCard.next`, 메서드
  `status_summary`, blocker message, 그 밖의 모든 display/template string이 일반 문구만
  사용합니다. 사용자 행동이 대기 중이고 다음 actor가 User Channel임을 말할 수 있지만
  요청 질문, 선택지, 맥락, form, 경로, 명령, URL, credential을 다시 만들면 안 됩니다.
- `SummaryCard.guarantee`는 요약된 보기에 대한 짧은 표시 문구입니다. 다른 담당 문서가 명시적으로 그런 보장을 제공하지 않는 한 정확성 증명, 테스트 충분성 증명, 검토 완료, OS 수준 집행을 주장하면 안 됩니다.
- `NextActionSummary`는 차단 사유 로컬 또는 미리보기 해결 행동 형태입니다. 유효한 필드는 `action_kind`, `owner_method`, `allowed_operation_categories`, `label`, `blocking_question`, `expected_state_version`, `required_refs`입니다.
- 성공 메서드 결과는 태그 기반 `workflow` 진행 상태를 노출합니다. `CloseReadinessBlocker.next_actions`는 해당 차단 사유에만 속하며 차단 사유 사이의 선택 역할을 갖지 않습니다.
- `allowed_operation_categories`는 행동에 대해 담당 문서가 지원하는 호출 범주를 이름 붙입니다. 현재 연결이 행동을 실행할 수 있음을 증명하거나 사용자 권한을 부여하지 않으며, 지원되는 API 메서드 호출이 식별되지 않으면 비어 있습니다.
- `expected_state_version`은 항상 존재하는 null 허용 필드입니다. 낙관적 동시성을 사용하는 API 변경 행동에는 그 행동을 만든 상태 보기의 현재 `project_state.state_version`을 담으며, 해당 호출의 `ToolEnvelope.expected_state_version`으로 직접 매핑됩니다. 읽기 행동, `user_only` 행동, 단일 담당 메서드가 없는 행동, 낙관적 동시성을 사용하지 않는 담당 메서드 행동에는 `null`을 사용합니다.
- `expected_state_version`은 재시도 가능한 동시성 입력이며 신원이나 권한이 아닙니다. 다른 변경이 커밋되면 오래될 수 있으므로 호출자는 `STATE_VERSION_CONFLICT` 뒤 현재 상태를 새로 읽습니다. `required_refs`와 참조의 `produced_at_state_version`은 이 토큰을 제공하거나 덮어쓰지 않습니다.
- 오래된 `action` 또는 `reason` 필드를 쓰는 차단 사유 로컬 또는 미리보기 행동은 유효한 `NextActionSummary`가 아닙니다.
- 이미 존재하는 대기 사용자 행동에는 `NextActionSummary.label`과 소유 blocker message가
  일반 User Channel 안내만 사용하고 `blocking_question=null`이며 `required_refs`에
  `user_action_request` ref가 없습니다. Request ID와 pending/next-actor 사실은
  `AgentSafeUserActionRequestSummary`에서만 가져옵니다. 다음 행동 text는 질문, 선택지, 맥락,
  form, 캡처 경로, 명령, URL, credential을 다시 만들면 안 됩니다. 별도의 요청 전
  `missing_final_acceptance` 행동은 Agent가 요청을 만드는 데 필요한 질문과 Task/현재 근거
  ref를 담을 수 있습니다. 요청을 만든 뒤에는 대기 규칙을 적용합니다.
- `WriteTicketStateSummary.status`는 제어 값 문자열입니다.
- 저장 상태가 active인 티켓에 정책 권한 결속이 없거나 현재 결속과 다르면 현재
  projection은 이를 사실상 `status=invalidated,invalidation_reason=explicit_revoke`로
  취급합니다. 이 닫힌 실패 projection은 티켓을 활성 후보로 만들지 않으며 과거에
  소비된 티켓을 다시 쓰지 않습니다.
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
- `WriteTicketValidityBasis.approval_basis_refs`에는
  `UserActionResolutionRef` 값만 들어갑니다. 각 값은 티켓이 소유한 완전한
  프로젝트, `Task`, UserAction resolution identity 하나를 나타내며 중복
  identity는 유효하지 않습니다. 현재성 비교는 이 완전한 identity를 사용하며
  범위가 없는 resolution ID만 비교하지 않습니다. 각 참조에는 구체적인
  `produced_at_state_version`이 있으며, 이 값은 identity가 아니라 metadata입니다.
- `WriteTicketValidityBasis.write_authority_fingerprint`는 정확한 정규화 객체
  `{schema:"volicord.write_authority",default_direct_control,default_work_control,light:{enabled,max_intended_paths,allowed_path_patterns,denied_path_patterns,final_acceptance},write_ticket:{idle_timeout_minutes}}`를
  canonical JSON으로 만든 뒤 계산한 `sha256:` 접두사 SHA-256입니다. 각 값은 대응하는
  `workflow` 정책 필드에서 가져오고 두 패턴 배열은 canonicalization 전에 정렬하고 중복을
  제거합니다. 이 객체에 나열되지 않은 모든 정책 필드를 제외하며 여기에는 host,
  host, connection, MCP, integration binding, 바깥쪽 정책 메타데이터 필드가 포함됩니다.
  이 digest는 전체 canonical 정책 `policy_fingerprint`보다 좁으며 서로 바꾸어 쓸 수
  없습니다. 따라서 패턴 순서와 중복 항목은 digest를 바꾸지 않고, canonical 정책이
  달라도 정규화된 쓰기 권한 객체가 같으면 티켓 호환성을 유지합니다. 프로젝트 정책이
  없으면 정규화 입력은 `default_direct_control=tracked`,
  `default_work_control=tracked`, `light.enabled=false`,
  `light.max_intended_paths=3`, 빈 allowed/denied 패턴 배열,
  `light.final_acceptance=policy_dependent`,
  `write_ticket.idle_timeout_minutes=null`을 사용합니다. 모든 정규 티켓은 현재 digest를
  담습니다. 누락되거나 null인 결속은 호환 형식이 아니라 저장 데이터 손상입니다.
- `WriteTicket.observed_paths`는 기준 범위에서 비어 있습니다. Codex Record Guard
  관찰은 티켓에 다시 쓰지 않고 저장소 관찰 및 미기록 변경 기록으로 남깁니다.
- `WriteTicket.guarantee_display`는 현재 보장 문구를 공개합니다. OS 수준 파일시스템
  집행을 주장하지 않습니다.
- `WriteDecisionReason`은 `PrepareWriteResult.write_decision_reasons`에서 사용합니다.

`NextActionSummary` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
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
| `observed_paths` | 정규화된 `Product Repository` 경로 문자열. | 담당 문서가 정의한 Guard 경로가 관찰을 티켓에 연결했을 때만 관찰된 경로를 나열합니다. 연결된 관찰이 없으면 `[]`를 사용합니다. |
| `basis_state_version` | 상태 시계 값. | 발급 또는 재사용 때 포착한 감사 순서이며 티켓 유효성 좌표가 아닙니다. |
| `validity_basis` | `WriteTicketValidityBasis`. | 상태 결합 재사용과 무효화에 사용하는 정확한 Task, Change Unit, 범위, 기준선, workspace, 프로젝트 쓰기 권한, 승인 좌표입니다. |
| `invalidation_reason` | 제어되는 무효화 사유 또는 `null`. | 티켓이 무효화될 때 기록하는 안정된 사유입니다. |
| `idle_expires_at` | UTC 타임스탬프 또는 `null`. | 선택적 프로젝트 정책 idle 경계입니다. `null`은 idle timeout이 없다는 뜻이며 고정 기본 수명은 없습니다. |
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

```schema
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

EvidenceCaptureSpec:
  capture_kind: verified_command_execution | verified_tool_invocation
  command_sha256: string                       # verified_command_execution에서만 사용
  command_label: string                        # verified_command_execution에서만 사용; 정규화된 1..256 UTF-8 bytes
  expected_exit_code: integer | null           # verified_command_execution에서만 사용
  tool_name: string                            # verified_tool_invocation에서만 사용; 앞뒤 공백 제거, 1..256 UTF-8 bytes
  tool_input_sha256: string                    # verified_tool_invocation에서만 사용
  expected_success: boolean | null             # verified_tool_invocation에서만 사용

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
  host_invocation_id: string | null
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
  host_invocation_id: string | null
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
- `EvidenceCaptureSpec`은 엄격한 태그 합집합입니다. 호출자가 제공하는 소문자
  64자 digest 필드는 정확한 command 또는 tool input을 결합합니다. Typed shape의
  expected-outcome 필드는 nullable이며 MCP에서 생략하면 메서드 담당 기본값을
  사용합니다.
- `EvidenceCaptureIntent`는 만료되는 불변 current-basis 요청입니다.
  `requested_by_actor_source`와 `workspace_context`는 Core가 파생한 근거 필드이며
  호출자가 선택하는 attribution이 아닙니다. 공개 ref는
  `record_kind=evidence_capture_intent`를 사용합니다.
- `EvidenceCaptureReceipt`는 불변 영속 source-fulfillment fact 레코드입니다. 연결된
  staging handle과 staged receipt bytes만 transient입니다. 등록된 connection과 host
  invocation identity, outcome, 완전성, 한계, redaction 상태, observer, 시각은
  source fact입니다. receipt는
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
  ref로 소비하면 authority-owned verified command 또는 verified tool producer를
  설정할 수 있습니다. 해당 앵커가 없는 직접
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

```schema
CurrentCloseBasis:
  close_basis_revision: integer
  scope_revision: integer
  task_id: string
  change_unit_id: string
  baseline_ref: string | null
  result_summary: string
  result_refs: StateRecordRef[]
  evidence_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  residual_risks: ResidualRisk[]
  sensitive_categories: string[]
  sensitive_action_requirements: SensitiveActionRequirement[]
  recovery_constraints: string[]
  source_run_ref: StateRecordRef | null
  shaping_checkpoint_ref: StateRecordRef | null
  shaping_decision_application_refs: StateRecordRef[]
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
- `result_refs`, `evidence_refs`, `source_run_ref`, `shaping_checkpoint_ref`,
  `shaping_decision_application_refs`, `source_refs`, `evidence_summary_ref`,
  `accepted_by_user_action_resolution_refs`는 `StateRecordRef`를 사용합니다.
- `sensitive_categories`는 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 범주 분류 문자열입니다.
- `sensitive_action_requirements`는 커밋된 실행 기록과 소비된 쓰기 티켓에서 Core가 파생한 닫기 요구사항입니다. 범주만 담은 호출자 입력은 이 요구사항을 만들거나 지울 수 없습니다.
- `recovery_constraints`와 `RiskAcceptanceCoverage.missing_reason`은 표시 문자열입니다. 현재 닫기 준비 상태 결과는 필요한 수락이 없으면 `acceptance_required`를 사용하고, 현재 잔여 위험 `risk_id` 값을 덮지 못하는 오래된 잔여 위험 수락이 있으면 `stale_acceptance`를 사용할 수 있습니다.
- `RiskAcceptanceCoverage`는 현재 잔여 위험 요구사항이 호환되는 사용자 작업 resolution으로 덮였는지를 보고합니다. 증거 충분성이나 최종 수락을 보고하지 않습니다.
- `CloseReadinessBlocker`는 닫기 차단 사유를 표현하는 데이터 형태입니다.
- `CloseReadinessBlocker.category`는 제어 값 문자열입니다.
- `CloseReadinessBlocker.code`는 담당 문서가 정의하는 차단 사유 코드입니다. 차단 사유 또는 메서드 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 빠짐없는 전역 공개 enum이 아닙니다.
- `CloseReadinessBlocker.message`, `ValidatorResult.message`, `GuaranteeDisplay.basis`는 자유 형식 표시 문자열입니다.
- `ValidatorResult.validator_id`는 값 집합 담당 문서가 지원되는 안정 값을 공개하기 전까지 보고용 라벨입니다.
- `ValidatorResult.status`, `ValidatorResult.severity`, `GuaranteeDisplay.level`은 제어 값 문자열입니다.
- `GuaranteeDisclosure`는 독자가 결과를 과대 해석할 수 있는 공개 결과 base와 진단 출력에 반환되는 결과 해석 공개입니다.
- `GuaranteeDisclosure.guarantee_class`와 `GuaranteeDisclosure.non_guarantees`는 제어 값 문자열입니다. `GuaranteeDisclosure.guarantees`는 짧은 표시 문장입니다.
- `GuaranteeDisplay`는 상태 또는 호환성 보기의 현재 기능 표시를 설명합니다. `GuaranteeDisclosure`를 대체하지 않습니다.

이 형태들은 닫기 준비 상태 의미, 응답 처리 경로, 지속 동작을 정의하지 않습니다.

닫기 근거 참조 규칙:
- Direct/work basis는 null이 아닌 정확한 호환 `source_run_ref`, null인
  `shaping_checkpoint_ref`, 빈 shaping decision application ref를 가집니다. Advisor basis는
  null인 `source_run_ref`, 정확한 현재 shaping checkpoint, 현재 checkpoint application
  ref의 정확한 집합을 가집니다.
- `CurrentCloseBasis.result_refs`나 `ResidualRisk.source_refs`로 받아들일 수 있는 direct/work 호출자 제공 닫기 평가 참조는 담당 문서가 다른 종류를 명시적으로 추가하지 않는 한 결과/증거 기록 종류인 `run`, `artifact`, `evidence_summary`, `change_unit`으로 제한됩니다. Advisor finalization은 현재 같은 Task의 `change_unit`, `artifact`, `evidence_summary` result ref와 `artifact` 또는 `evidence_summary` evidence ref를 허용하며 Run은 허용하지 않습니다.
- 담당 문서가 명시적으로 추가하지 않는 한 `project_state`, `write_ticket`, `user_action_request`, `user_action_resolution`, `blocker`, `task_event`, `task`는 호출자 제공 결과 참조가 아닙니다.
- 받아들인 모든 참조는 존재해야 하고 같은 프로젝트와 `Task`에 속해야 하며 Core가 정규화해야 합니다. Core는 호출자가 보낸 `produced_at_state_version` 메타데이터를 권한이나 동시성 입력으로 취급하지 않습니다.
- 닫기 증거에 쓰이는 아티팩트 참조는 `Task`에 연결되어 있고 `integrity_status=verified`여야 하며 [아티팩트 저장소](../storage-artifacts.md)에 따라 사용 시점의 현재 바이트 검증을 통과해야 합니다.
- 증거 참조는 현재 `Task` 증거 요약을 식별해야 합니다. 현재 닫기 근거 결과 참조로 쓰이는 실행 기록 참조는 현재 `Task`, 현재 적용 Change Unit, 현재 범위 리비전, 호환되는 기준선, 기록된 상태와 호환되는 기록된 현재 실행 기록을 식별해야 합니다. 이력 실행 기록은 현재 실행 기록이 그 `verified` 아티팩트나 증거를 명시적으로 재사용하고 그 재사용을 기록하지 않는 한 감사 기록입니다.
- Core는 기준 닫기 근거를 구성하면서 현재 실행 기록, 현재 Change Unit, 현재 EvidenceSummary 참조를 추가할 수 있습니다.

보장 표시 규칙:
- `GuaranteeDisplay`는 프로젝트 강제 프로필, 확인된 호출 맥락, 활성화된 강제 메커니즘, 지원되는 기준 범위에서 파생됩니다.
- `capability_refs`는 표시를 정당화하는 참조를 담는 구현 필드 이름입니다. 기준 연결 아키텍처에서는 사용할 수 있으면 호출 바인딩, Agent Connection, 관찰 사실을 인용해야 합니다.
- 별도 지원 기록이 그 표시를 정당화하지 않는 한 협력형 `agent_report` `Run`이나
  관찰을 외부 관찰로 표시하지 않습니다.

담당 문서 링크:
- 닫기 준비 상태 의미와 대체 금지 규칙: [Core 모델의 닫기 준비 상태](../core-model.md#close_task)
- 현재 닫기 근거 생성: direct/work는 [`volicord.record_run`](method-record-run.md),
  advisor는 [`volicord.finalize_advice`](method-finalize-advice.md)
- 판단 호환성과 수락된 위험 입력: [API 판단 스키마](schema-judgment.md)
- 응답 분기 동작, 닫기 준비 상태 평가 순서, 응답 전용 차단 결과: [`volicord.check_close`와 `volicord.close_task`](method-close-task.md)
- 닫기 차단 사유와 API 응답 분기 사이의 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md)
- 차단 사유 범주 값(`CloseReadinessBlocker.category`), 지원되는 `ValidatorResult.status`, `ValidatorResult.severity`, `GuaranteeDisplay.level` 값: [API 값 집합](schema-value-sets.md#state-and-blocker-values)
- 보안 보장 의미: [보안](../security.md)

## 관련 담당 문서

- [API 코어 스키마](schema-core.md): `ToolEnvelope`, 효과별 결과 메타데이터,
  메서드별 결과 base, `ToolRejectedBase`, `ToolDryRunBase`,
  `ToolRejectedResponse`, `ToolDryRunResponse`.
- [API 값 집합](schema-value-sets.md#state-and-blocker-values): 차단 사유 범주 값(`CloseReadinessBlocker.category`)과 인접 상태 값.
- [API 메서드](methods.md)와 메서드 담당 문서: 이 스키마를 반환하는 메서드.
- [API 아티팩트 스키마](schema-artifacts.md): `ArtifactRef`.
- [API 사용자 행동 스키마](schema-user-action.md): 영속 행동 요청과 adapter-neutral resolution form.
- [저장 효과](../storage-effects.md): 지속 저장과 상태 효과.
