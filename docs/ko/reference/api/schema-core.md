# API Schema Core

## 이 문서로 할 수 있는 일

현재 MVP에서 쓰는 활성 메서드 이름 집합, 공용 API 형태, 닫힌 스키마 값 집합을 확인할 때 이 참조를 사용합니다. `ToolEnvelope`, 공통 응답, `ArtifactRef`, `StateRecordRef`, `ShapingReadiness`, `UserJudgment`, Write Authorization 요약, `CompletionPolicy`, 증거 요약, 실행 요약, 닫기 차단 사유, 다음 행동 요약, 현재 MVP enum 값을 다룹니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 문서 저장소에 MCP 서버가 구현되어 있다는 뜻이 아닙니다. 향후 스키마 후보는 [이후 후보 색인](../../later/index.md#later-schema-candidates)에 남습니다.

## 계약 위치 지도

| 필요한 것 | 담당 문서 |
|---|---|
| 정확한 활성 메서드 이름 값 집합과 공용 스키마 값 집합 | 이 문서 |
| `ToolEnvelope.surface_id`, `LocalSurfaceRegistration`, `VerifiedSurfaceContext`, 로컬 접점 접근 값 집합, 보장 표시에 쓰이는 `capability_profile` 값 집합 | 이 문서 |
| 활성 메서드별 요청/응답 동작 | [MVP API](mvp-api.md) |
| 공개 오류, 우선순위, 멱등성, 차단 응답, 오래된 상태 동작 | [API Errors](errors.md) |
| Core 상태 의미, 구체화 준비 상태 의미, lifecycle 의미 | [Core Model 참조](../core-model.md) |
| 저장소 테이블, JSON `TEXT`, enum hardening, artifact persistence | [Storage](../storage.md) |
| 보안 보장 의미 | [보안 참조](../security.md) |
| 향후 API/스키마 후보 | [이후 후보 색인](../../later/index.md#later-schema-candidates) |

## 스키마 표기 규칙

이 문서의 YAML 형식 표기는 예시라고 표시하지 않는 한 규범 스키마 표기입니다.

- `field: Type`은 필드가 필수이고 non-null이라는 뜻입니다.
- `field: Type | null`은 필드가 필수이고 JSON `null`을 허용한다는 뜻입니다.
- `Type[]`은 필드가 존재하고 배열을 담는다는 뜻입니다. 빈 배열은 `[]`로 씁니다.
- `a | b | c`는 해당 필드의 닫힌 활성 enum입니다.
- later, reserved, profile-gated 이름은 활성 enum 표기나 활성 값 표에 넣지 않습니다. 담당 문서가 승격하기 전까지 [이후 후보 색인](../../later/index.md)에 남깁니다.
- 명시되지 않은 필드는 명시적인 확장 컨테이너 밖에서 거부됩니다.

저장소 검증은 별도 담당 문서 경계입니다. API 페이로드와 API 형태로 저장되는 JSON은 먼저 이 API 참조로 검증합니다. DDL, 저장소 전용 JSON, 기본값, 잠금, 마이그레이션은 [Storage](../storage.md)가 담당합니다.

[현재 MVP 값 집합](#current-mvp-value-sets)은 정확한 활성 메서드 이름 집합과 이 문서가 선언하는 활성 스키마 enum 값을 담당합니다. 메서드별 동작은 [MVP API](mvp-api.md)가 담당하고, 공개 `ErrorCode` 분류는 [API Errors](errors.md)가 담당합니다.

<a id="tool-envelope"></a>

## ToolEnvelope 봉투

모든 공개 도구 요청은 `ToolEnvelope`를 가집니다. 커밋되는 non-dry-run 상태 변경 도구는 non-null `idempotency_key`와 현재 프로젝트 전체 `project_state.state_version`에 맞는 `expected_state_version`을 요구합니다. `harness.stage_artifact`, `harness.status`, `harness.close_task intent=check`, `dry_run` 호출은 `idempotency_key`와 `expected_state_version`을 `null`로 둘 수 있습니다. `harness.stage_artifact`는 임시 스테이징 핸들만 만들며 Core 상태 전이가 아닙니다. 읽기 전용 호출은 멱등 키를 요구하거나 예약하지 않습니다. 메서드별 상태 효과는 [현재 MVP API](mvp-api.md#active-mvp-method-behavior)가 담당합니다.

```yaml
ToolEnvelope:
  request_id: string
  idempotency_key: string | null
  expected_state_version: integer | null
  project_id: string
  task_id: string | null
  surface_id: string
  actor_kind: user | lead_agent
  dry_run: boolean
```

Envelope 필드는 호출 경로를 정하고 감사 추적에 쓰입니다. `ToolEnvelope.surface_id`는 필수이지만 선택자일 뿐입니다. API 담당 문서가 그 접점에 의존하려면 서버가 확인한 로컬 접점 맥락과 맞아야 합니다. `surface_id`만으로 호출자 권한은 증명되지 않으며, 역량, 쓰기 권한, 로컬 접근, 사용자 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 아티팩트 접근, 닫기를 부여하지 않습니다.

<a id="local-surface-access-values"></a>

## 로컬 접점 접근 값

로컬 접점 접근 값은 하네스 API 호환성을 설명하는 값입니다. OS 권한, 샌드박스 경계, 변조 방지 보장, 보편적 도구 실행 전 차단, 격리를 뜻하지 않습니다.

`LocalSurfaceRegistration`은 한 프로젝트 안의 로컬 접점 등록 사실을 나타내는 개념 스키마입니다. 저장소는 이를 등록 데이터로 보관하지만, 도구 요청이 이 값을 보냈다는 이유만으로 권한으로 받아들이지 않습니다. Product Repository 파일, Projection, 생성된 Markdown, 대화 텍스트, 에이전트 기억은 접점 등록을 만들거나, 바꾸거나, 새로 고칠 수 없습니다.

```yaml
LocalSurfaceRegistration:
  project_id: string
  surface_id: string
  surface_instance_id: string
  transport_kind: local_mcp_stdio | local_http
  transport_binding_fingerprint: string
  access_secret_hash: string | null
  capability_profile_hash: string
  status: active | disabled | stale | revoked
  local_access_posture: registered_local | unavailable | mismatch | revoked
  registered_at: string
  last_verified_at: string | null
```

`VerifiedSurfaceContext`는 서버가 구체적인 요청과 접근 분류에 대해 파생하는 맥락입니다. 요청 본문이 아니고, Markdown 주장도 아니며, 에이전트 기억의 사실도 아닙니다. 서버는 로컬 transport/session/binding과 저장된 `LocalSurfaceRegistration`에서 이 값을 도출합니다.

```yaml
VerifiedSurfaceContext:
  project_id: string
  surface_id: string
  surface_instance_id: string
  access_class: read_status | core_mutation | write_authorization | run_recording | artifact_registration | artifact_read
  verified: boolean
  failure_reason: unavailable | mismatch | revoked | insufficient_capability | null
```

`registered_local`은 성공한 로컬 등록과 확인의 결과로 생기는 태세입니다. 자유 입력 라벨, 호출자 주장, 생성 파일 표식, 권한 우회 값이 아닙니다. `surface_id`는 같은 프로젝트의 등록을 선택해야 하며, 상태 변경 API 접근이나 아티팩트 본문 읽기가 진행되려면 확인된 맥락이 그 등록과 맞아야 합니다.

`LocalSurfaceRegistration.local_access_posture`는 닫힌 현재 MVP 값 집합입니다.

| 값 | 의미 |
|---|---|
| `registered_local` | 최근 서버 확인에서 로컬 transport/session/binding이 이 프로젝트의 등록된 로컬 접점과 맞아, API 담당 문서가 접근 분류를 평가할 수 있습니다. |
| `unavailable` | 필요한 MCP/Core 또는 접점 도달 가능성을 현재 확인할 수 없습니다. |
| `mismatch` | 도달 가능한 로컬 transport/session/binding이 프로젝트에 등록된 로컬 접점 binding과 맞지 않습니다. |
| `revoked` | 등록된 접점의 로컬 접근이 명시적으로 철회되었으며, 새로 유효한 등록이 대체하기 전까지 쓰면 안 됩니다. |

`LocalSurfaceRegistration.status`는 닫힌 현재 MVP 값 집합입니다.

| 값 | 의미 |
|---|---|
| `active` | 등록된 접점을 현재 API 접근 확인에 사용할 수 있습니다. |
| `disabled` | 접점 기록은 남아 있지만 현재 API 접근에 쓰면 안 됩니다. |
| `stale` | 현재 API 접근에서 이 접점에 의존하기 전에 접점 등록 또는 역량 태세를 새로 고쳐야 합니다. |
| `revoked` | 접점 등록이 현재 API 접근에 더 이상 유효하지 않습니다. |

활성 로컬 API 접근 분류 라벨은 `read_status`, `core_mutation`, `write_authorization`, `run_recording`, `artifact_registration`, `artifact_read`입니다. `artifact_registration`은 `harness.stage_artifact`와 `harness.record_run`이 소비할 수 있는 `ArtifactInput[]` 값을 포함합니다. 이 분류의 메서드별 조건은 [현재 MVP API](mvp-api.md#shared-request-rules)가 담당하고, 공개 오류 선택은 [API Errors](errors.md)가 담당합니다. `VerifiedSurfaceContext.failure_reason=unavailable`, `mismatch` 또는 `revoked`, `insufficient_capability`는 각각 `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`로 구분되어야 합니다.

<a id="capability-profile-value-sets"></a>

## Capability Profile 값 집합

Agent 통합 참조는 `capability_profile` 필드 의미, 갱신 규칙, 커넥터 대체 동작, 접점별 메모를 담당합니다. Schema Core는 그 프로필과 `GuaranteeDisplay`가 쓰는 활성 값 집합을 담당합니다.

```yaml
capability_profile:
  surface_id: reference-local-mcp
  surface_status: active
  local_access_posture: registered_local
  cooperative_prepare_write_supported: true
  changed_path_detection_supported: true
  changed_path_detection_verification: not_run | passed | failed | stale
  manual_artifact_attachment_supported: true
  native_artifact_capture_supported: false
  guarantee_level_default: cooperative
  guarantee_level_max_when_verified: detective
```

`changed_path_detection_verification=passed`만 `detective` 표시를 뒷받침할 수 있으며, 그 경우에도 검증된 변경 경로 탐지 범위 안으로 제한됩니다. `not_run`, 예전 `planned_not_run` 문구, `failed`, `stale`은 통과 상태가 아닙니다. `native_artifact_capture_supported=false`는 활성 아티팩트 경로가 `harness.stage_artifact` 스테이징과 담당 경로 등록으로 제한된다는 뜻입니다. `captured_artifact`나 접점 자체 캡처 권한을 추가하지 않습니다.

<a id="common-response"></a>

## 공통 응답

```yaml
ToolResponseBase:
  request_id: string
  idempotency_key: string | null
  project_id: string
  task_id: string | null
  state_version: integer
  dry_run: boolean
  errors: ToolError[]
  validator_results: ValidatorResult[]
  events: EventRef[]

ToolError:
  code: ErrorCode
  message: string
  retryable: boolean
  details: object

EventRef:
  event_id: string
  event_seq: integer
  event_type: string
  task_id: string | null
  state_version: integer
```

`ToolResponseBase.state_version`은 항상 프로젝트 전체 버전입니다. 커밋된 상태 변경에서는 커밋 뒤의 `project_state.state_version`이고, 읽기 전용과 `dry_run` 응답에서는 그 응답이 관찰한 현재 프로젝트 전체 버전입니다. 읽기 전용 응답은 계산된 차단 사유나 닫기 차단 사유를 저장하지 않고 포함할 수 있습니다. `dry_run=true`는 현재 기록, 이벤트, 아티팩트, 증거 요약, Write Authorization, 닫기 상태, `tool_invocations` 재실행 행, 상태 버전 증가를 만들지 않습니다.

<a id="state-summary"></a>

## StateSummary

```yaml
StateSummary:
  mode: advisor | direct | work
  lifecycle_phase: shaping | ready | executing | waiting_user | blocked | completed | cancelled | superseded
  result: none | advice_only | completed | cancelled | superseded
  close_reason: none | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked
  shaping_readiness: ShapingReadiness
  gates:
    scope_gate: not_required | required | pending | passed | failed | blocked
    decision_gate: not_required | required | pending | resolved | deferred | blocked
    approval_gate: not_required | required | pending | granted | denied | expired
    evidence_gate: not_required | none | partial | sufficient | stale | blocked
    acceptance_gate: not_required | required | pending | accepted | rejected

GuaranteeDisplay:
  level: cooperative | detective
  notes: string[]

ShapingReadiness:
  goal_summary_known: boolean
  non_goals_known: boolean
  affected_area_or_paths_known: boolean
  acceptance_criteria_known: boolean
  autonomy_boundary_known: boolean
  first_change_unit_known: boolean
  user_owned_blockers_named: boolean
  next_safe_action_known: boolean
```

`StateSummary.mode`는 지속 저장되는 `tasks.mode`를 그대로 보여 주며 항상 구체적 Task `mode`입니다. `auto`는 저장되는 `mode`, 표시되는 Task `mode`, 상태 요약의 `mode`가 아닙니다. `StateSummary.lifecycle_phase`는 지속 저장되는 `Task.lifecycle_phase`를 그대로 보여줍니다. `intake`는 API 메서드이자 시작 처리 단계이지 생명주기 값이 아닙니다. 종료 `lifecycle_phase` 값은 `completed`, `cancelled`, `superseded`입니다. 특히 `superseded`는 Task가 다른 Task나 경로로 대체되어 다시 활성 작업으로 돌아가지 않는다는 뜻입니다. `StateSummary.close_reason`은 지속 저장되는 `Task.close_reason`을 그대로 보여줍니다. `StateSummary.result`는 큰 단위의 `Task.result`를 보여줍니다. 실패한 Run, `violation`, 차단된 닫기, 증거 공백, 차단 사유는 `RunSummary.status`, `CloseBlocker`, 증거 상태, 차단 사유, 현재 Task 상태에 남고 Task의 종료 결과가 되지 않습니다. 이 문서의 `passed`와 `failed`는 `StateSummary.gates.*` 또는 `ValidatorResult.status` 값일 때만 활성이며 `Task.result` 값이 아닙니다.

`StateSummary.shaping_readiness`는 활성 상태에서 파생한 보기입니다. 현재 Task 상태, 활성 또는 제안된 Change Unit 상태, 대기 중인 `UserJudgment` 후보나 기록, 차단 사유, 증거 요약, 다음 행동 상태에서 계산합니다. 지속 저장되는 Task 필드가 아니고, 별도 `StateRecordRef.record_kind`도 아니며, 커밋된 `Discovery Brief`, `Question Queue`, `Assumption Register` 같은 계획 아티팩트도 아닙니다. `false` 필드는 계속 보이지만, 알 수 없거나 오래된 항목이 첫 안전한 Change Unit 또는 다음 안전한 행동에 영향을 줄 때만 막는 조건입니다.

쓰기 가능한 작업에서 첫 Change Unit을 만들기 전 `user_owned_blockers_named=true`는 막고 있는 사용자 소유 문제가 `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval` 중 하나로 식별되었거나, 다음 안전한 행동에 사용자 소유 blocker가 현재 필요하지 않다는 뜻입니다. `next_safe_action_known=true`는 응답이 확인, `harness.request_user_judgment`, `harness.update_scope`, `harness.prepare_write` 같은 다음 담당 경로 행동을 이름 붙일 수 있다는 뜻입니다.

`Task.close_reason` 값은 서로 바꿔 쓸 수 있는 라벨이 아닙니다. `completed_self_checked`는 필수 증거가 충분하고, 필요한 `final_acceptance`가 해결되었고, 닫기에 영향을 주는 `residual_risk_acceptance`가 필요하지 않다는 뜻입니다. `completed_with_risk_accepted`는 필수 증거가 충분하고, 필요한 `final_acceptance`가 해결되었고, 닫기에 영향을 주는 보이는 잔여 위험에 대해 호환되는 `residual_risk_acceptance`가 있다는 뜻입니다. `cancelled`와 `superseded`는 종료 상태이지만 성공 완료가 아니며 `CompletionPolicy`의 증거, 최종 수락, 잔여 위험 수락 요구를 만족시키지 않습니다.

Task `mode` 값은 독자에게 다음 뜻으로 설명됩니다.

- `advisor`: 제품 쓰기 없는 조언, 검토, 계획.
- `direct`: 작은 직접 변경.
- `work`: 추적되는 작업.

`IntakeRequest.requested_mode=auto`는 `harness.intake` 입력에서만 쓰는 분류 요청입니다. 서버는 `tasks.mode`를 저장하거나 `StateSummary.mode`를 만들거나 `harness.intake`/`harness.status` 요약을 반환하기 전에 이를 `advisor`, `direct`, `work` 중 정확히 하나로 확정해야 합니다.

화면에 표시되는 라벨은 기준 스키마 값이 아닙니다. `GuaranteeDisplay.level`은 문서화된 접점 역량과 증명 수준을 보여 주는 표시 주장입니다. 권한이나 상태 권한을 부여하지 않습니다. 현재 MVP의 활성 `GuaranteeDisplay.level` 값은 `cooperative`와 `detective`뿐입니다. 기본값은 `cooperative`입니다. `detective`는 관련 활성 역량 확인이 통과했을 때만 표시할 수 있습니다. 기준 `reference-local-mcp` 프로필에서는 `changed_path_detection_verification=passed`이고 검증된 변경 경로 탐지 범위 안일 때만 가능합니다. 더 강한 표시 이름은 이후 후보이며 현재 MVP 스키마 값이 아닙니다.

<a id="staterecordref"></a>

## StateRecordRef

```yaml
StateRecordRef:
  record_kind: project | task | change_unit | run | write_authorization | user_judgment | evidence_summary | blocker
  record_id: string
```

지속 보관되는 증거 바이트는 `StateRecordRef`가 아니라 `ArtifactRef`를 사용합니다. 현재 MVP의 활성 사용자 소유 판단은 맞는 `UserJudgment.judgment_kind`를 가진 `record_kind=user_judgment`로 표현합니다.

<a id="artifactref"></a>

## ArtifactRef

`ArtifactRef`는 하네스 저장소에 등록된 영속 증거 파일을 가리킵니다. 호출자가 임의로 준 경로가 아닙니다.

```yaml
ArtifactRef:
  artifact_id: string
  kind: diff | log | screenshot | checkpoint | other
  uri: string
  sha256: string
  size_bytes: integer
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  task_id: string
  run_id: string | null
  relation_owner: ArtifactRelationOwner
  created_at: string
  produced_by: lead_agent | harness
  retention_class: task | project | temporary

ArtifactRelationOwner:
  task_id: string
  run_id: string | null
  record_kind: task | change_unit | run | user_judgment | evidence_summary | blocker
  record_id: string
  relation: string
```

`uri`는 하네스 저장소를 통해 해석되며 보통 `harness-artifact://{project_id}/{artifact_id}`입니다. 원시 비밀값, 토큰, 민감한 전체 로그를 증거로 저장하면 안 됩니다. 본문이 `redacted`, `omitted`, `blocked` 상태라면 `sha256`와 `size_bytes`는 숨겨진 원본이 아니라 커밋된 안전한 바이트를 설명합니다.

<a id="artifactinput"></a>

## ArtifactInput

`ArtifactInput`은 `harness.record_run`에서 활성 `harness.stage_artifact` 유틸리티가 만든 문서화된 `StagedArtifactHandle`이나 이미 등록된 `ArtifactRef`로만 받습니다. 임의 파일 읽기 권한을 부여하지 않습니다. `harness.stage_artifact`는 새 아티팩트 바이트를 위한 현재 MVP 스테이징 유틸리티이지 접점 자체 아티팩트 캡처나 일반 파일시스템 읽기 API가 아닙니다.

```yaml
ArtifactInput:
  artifact_input_id: string
  source_kind: staged_artifact | existing_artifact
  relation: string
  staged_artifact_handle: StagedArtifactHandle | null
  existing_artifact_ref: ArtifactRef | null
  display_name: string | null
  content_type: string
  expected_sha256: string | null
  expected_size_bytes: integer | null

StageArtifactRequest:
  envelope: ToolEnvelope
  task_id: string
  display_name: string
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  safe_bytes_or_notice: bytes | string
  expected_sha256: string | null
  expected_size_bytes: integer | null
  relation_hint: string | null

StageArtifactResponse:
  request_id: string
  project_id: string
  task_id: string
  staged_artifact_handle: StagedArtifactHandle
  expires_at: string
  errors: ToolError[]

StagedArtifactHandle:
  handle_id: string
  project_id: string
  task_id: string
  sha256: string
  size_bytes: integer
  content_type: string
  redaction_state: none | redacted | secret_omitted | blocked
  expires_at: string
```

`source_kind`에 맞는 출처 필드 하나만 있어야 합니다. `staged_artifact`에는 `staged_artifact_handle`, `existing_artifact`에는 `existing_artifact_ref`가 필요합니다. 스테이징 핸들은 같은 `project_id`와 `task_id` 범위에 있어야 하고 `content_type`, `sha256`, `size_bytes`, `redaction_state`, `expires_at`을 가져야 하며, `harness.record_run`이 사용할 때 만료되지 않았고 아직 소비되지 않았어야 합니다. 만료된 핸들, 범위가 맞지 않는 핸들, 이미 소비된 핸들, 다른 Task의 핸들은 변경 전에 거부됩니다.

`harness.stage_artifact`는 임시 `StagedArtifactHandle`을 만들 수 있지만 그 자체로 Core 상태 전이가 아닙니다. 증거를 만들지 않고, gate를 만족하지 않고, 증거 요약을 갱신하지 않으며, `harness.close_task`가 통과하게 만들 수도 없습니다. 유효한 스테이징 핸들을 소비해 지속 `ArtifactRef`로 승격할 수 있는 활성 경로는 `harness.record_run`뿐입니다.

원시 파일 경로, 원시 로그, 임의 로컬 경로 문자열, `captured_artifact`, 캡처 핸들, 접점 자체 아티팩트 캡처, 원시 캡처 어댑터 출력, 원시 비밀값, 토큰, 민감한 전체 로그는 현재 MVP 밖이며 변경 전에 아티팩트 권한으로 거부됩니다. 새 아티팩트 바이트는 `harness.stage_artifact`를 통해서만 현재 MVP에 들어오고, 기존 바이트는 호환되는 `existing_artifact_ref`를 통해서만 재사용합니다.

<a id="evidence-and-pre-write-scope-schemas"></a>

## 증거와 쓰기 전 범위 스키마

```yaml
CompletionPolicy:
  evidence_required: boolean
  final_acceptance_required: boolean
  residual_risk_acceptance_required_when_visible: boolean
  product_write_completion: boolean
  user_visible_result: boolean

EvidenceCoverageItem:
  claim: string
  required_for_close: boolean
  coverage_state: supported | unsupported | partial | not_applicable | stale | blocked
  supporting_state_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_blocker_refs: StateRecordRef[]
  note: string | null

EvidenceSummary:
  evidence_summary_ref: StateRecordRef | null
  task_id: string
  change_unit_id: string | null
  completion_policy: CompletionPolicy
  status: not_required | none | partial | sufficient | stale | blocked
  coverage_items: EvidenceCoverageItem[]
  supporting_run_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_blocker_refs: StateRecordRef[]
  summary: string
  updated_at: string

AuthorizedAttemptScope:
  task_id: string
  change_unit_id: string
  basis_state_version: integer
  surface_id: string
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: SensitiveCategory[]
  baseline_ref: string | null
  related_user_judgment_refs: StateRecordRef[]
  guarantee_level: cooperative | detective

SensitiveActionScope:
  sensitive_action_id: string
  action_kind: product_file_write | dependency_change | destructive_command | network_access | secret_access | deployment | system_access | other
  named_action: string
  command_or_tool: string | null
  intended_paths: string[]
  hosts: string[]
  dependencies: string[]
  secret_handles: string[]
  time_window: string | null
  scope_limit: string
  not_authorized: string[]
  capability_claim: cooperative_only | observed_by_surface | not_observable

WriteAuthorizationSummary:
  write_authorization_id: string
  attempt_scope: AuthorizedAttemptScope
  status: active | consumed | expired | stale | revoked
  consumed_by_run_id: string | null
  created_at: string
  consumed_at: string | null

WriteAuthoritySummary:
  active_change_unit_ref: StateRecordRef | null
  write_authorization_ref: StateRecordRef | null
  active_authorized_attempt_scope: AuthorizedAttemptScope | null
  approval_status: not_required | required | pending | granted | denied | expired | unknown
  guarantee_display: GuaranteeDisplay
```

`CompletionPolicy`는 Task 또는 Change Unit에서 `close_task intent=complete`에 쓰는 활성 닫기 정책을 간결하게 담습니다. 해당 완료 경로에서 증거, 최종 수락, 보이는 잔여 위험의 수락, 제품 쓰기 완료, 사용자에게 보이는 결과가 필요한지 이름 붙입니다. `intent=cancel`과 `intent=supersede`는 성공 완료가 아니며 이 정책을 만족시키지 않습니다. `CompletionPolicy`는 QA gate, verification gate, 전체 Evidence Manifest, 별도 보증 흐름을 추가하는 허가가 아닙니다.

`EvidenceSummary`는 그 `CompletionPolicy`에 묶인 현재 MVP의 간결한 증거 기록입니다. 증거 충분성은 막연한 산문 판단이 아닙니다. `completion_policy.evidence_required=false`이면 증거 상태는 `not_required`여야 합니다. `EvidenceSummary.status=sufficient`는 `required_for_close=true`인 모든 `EvidenceCoverageItem`이 존재하고 `coverage_state=supported` 또는 `not_applicable`일 때만 허용됩니다. 필수 `EvidenceCoverageItem` 중 하나라도 `unsupported`, `partial`, `stale`, `blocked`이면 `harness.close_task`는 닫기 차단 사유를 보고해야 합니다. 필수 증거가 통째로 빠졌다면 그 필수 항목을 `unsupported` 또는 `blocked` `EvidenceCoverageItem`이나 `gap_blocker_refs`로 표현해야 하며, 항목을 생략해서 숨기면 안 됩니다.

선택 증거 항목은 `required_for_close=false`로 명시할 수 있습니다. 선택 공백은 보이게 남아도 `EvidenceSummary.status=sufficient`를 막지 않을 수 있지만, 현재 MVP 요약이 작더라도 필수/선택 구분은 명시해야 합니다.

아티팩트 가용성과 증거 충분성은 관련되어 있지만 별개의 조건입니다. 등록되어 사용할 수 있는 `ArtifactRef`가 있어도 `EvidenceCoverageItem`이 그 아티팩트를 주장에 연결하지 않으면 증거가 충분해지지 않습니다. 필수 `EvidenceCoverageItem`이 빠졌거나, 필수 항목이 없거나 사용할 수 없거나 무결성에 실패했거나 닫기 근거로 쓸 수 없는 아티팩트에 의존하면 충분할 수 없으며, `close_task`는 `CloseBlocker.category=artifact_availability`도 보고할 수 있습니다. 최종 수락과 잔여 위험 수락은 빠진 필수 증거를 대신할 수 없고, 증거도 최종 수락이나 잔여 위험 수락을 만들 수 없습니다.

`AuthorizedAttemptScope`는 `write_authorizations.attempt_scope_json`에 저장되고 나중에 `harness.record_run`에서 비교하는 정확한 범위입니다. `AuthorizedAttemptScope.basis_state_version`은 `prepare_write`가 권한을 준비할 때 사용한 프로젝트 전체 `project_state.state_version`입니다. `WriteAuthorizationSummary.status`는 오래 남는 Write Authorization 생명주기입니다. `blocked`는 Write Authorization의 `status`가 아닙니다. 차단된 쓰기는 소비 가능한 Write Authorization 없이 차단 사유를 반환합니다.

현재 MVP의 `AuthorizedAttemptScope`는 제품 파일 쓰기 시도에만 쓰입니다. 의도한 제품 경로, Change Unit, 프로젝트 전체 기준 상태 버전, baseline, 관련 사용자 판단 참조, 제품 쓰기 민감 범주, 경로 수준 쓰기 호환성 확인의 정직한 보장 수준을 기록합니다. 명령 실행, 의존성 설치, 네트워크 효과, 비밀값 접근, 배포, 파괴적 동작, 시스템 접근, 도구 관찰, 네이티브 아티팩트 캡처, 도구 실행 전 차단, 격리는 `AuthorizedAttemptScope` 필드가 아닙니다. 이런 관찰할 수 없는 보장을 요구하는 요청은 검증 오류나 역량 부족으로 거절하거나 차단해야 하며, 검증된 쓰기 범위처럼 기록하면 안 됩니다.

`SensitiveActionScope`는 `judgment_kind=sensitive_approval`에 대해 별도로 기록되는 민감 동작 승인 범위입니다. 의도한 명령, 의존성 변경, 네트워크 접근, 비밀값 접근, 배포, 파괴적 동작, 시스템 접근, 제품 파일 쓰기, 그 밖의 이름 붙은 민감 동작을 설명할 수 있습니다. `capability_claim`은 활성 접점이 그 동작에 대해 정직하게 주장할 수 있는 수준만 기록합니다. 값은 `cooperative_only`, `observed_by_surface`, `not_observable`입니다. 민감 동작 승인은 그 정확한 동작에 대한 검증된 역량이 따로 있지 않은 한 하네스가 동작을 관찰, 차단, 강제, 샌드박스 처리, 격리할 수 있다는 뜻이 아닙니다.

`WriteAuthoritySummary.approval_status`는 필요한 별도 민감 동작 승인 상태를 보여줍니다. `WriteAuthorizationSummary.status` 생명주기가 아니며, `SensitiveActionScope`를 `AuthorizedAttemptScope`로 바꾸지도 않습니다.

<a id="record-run-payloads"></a>

## record_run 페이로드

```yaml
ObservedChanges:
  product_write: boolean
  changed_paths: string[]
  no_product_changes: boolean
  summary: string

RunSummary:
  run_ref: StateRecordRef
  kind: shaping_update | implementation | direct
  status: completed | interrupted | blocked | violation
  product_write: boolean
  write_authorization_ref: StateRecordRef | null
  evidence_summary_ref: StateRecordRef | null
  artifact_refs: ArtifactRef[]
  summary: string
  started_at: string | null
  completed_at: string
```

`status=completed`만 정상 담당 참조를 통해 증거를 뒷받침할 수 있습니다. `interrupted`, `blocked`, `violation`은 감사/복구 사실이며 증거, 최종 수락, 잔여 위험 수락, 닫기를 스스로 충족하지 않습니다.

<a id="userjudgment"></a>

## UserJudgment

```yaml
UserJudgment:
  user_judgment_id: string
  task_id: string
  change_unit_id: string | null
  status: proposed | pending_user | resolved | deferred | rejected | blocked | superseded
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: next_action | write | run | close | acceptance | risk
  resolution: UserJudgmentResolution | null
  expires_at: string | null
  created_at: string
  updated_at: string
  resolved_at: string | null

UserJudgmentOption:
  option_id: string
  label: string
  meaning: approve | reject | defer | choose | cancel
  consequence: string

UserJudgmentContext:
  why_now: string
  source_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  what_user_is_judging: string
  why_agent_cannot_decide: string
  no_decision_consequence: string

UserJudgmentResolution:
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  note: string | null
```

`judgment_kind`는 기준 판단 종류 필드입니다. 렌더링된 라벨과 지역화된 라벨은 스키마 값이 아닙니다. `presentation=short`가 현재 MVP의 활성 `presentation` 값입니다. 확장 표시 본문은 활성 API 스키마가 아닙니다.

`UserJudgmentResolution.selected_option_id`와 `UserJudgmentResolution.note`는 기준 요청 필드인 `RecordUserJudgmentRequest.selected_option_id`와 `RecordUserJudgmentRequest.note`에서 저장된 복사본입니다. `RecordUserJudgmentPayload`는 판단 종류별 답변 세부정보만 담으며 선택지 식별자나 요청 메모를 반복하면 안 됩니다.

<a id="userjudgmentcandidate"></a>

## UserJudgmentCandidate

```yaml
UserJudgmentCandidate:
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: next_action | write | run | close | acceptance | risk
```

후보는 커밋된 `user_judgment` 행이 아닙니다. `StateRecordRef`가 없고 gate를 충족하지 않으며 민감 동작 승인, 최종 수락, 잔여 위험 수락, 증거, Write Authorization, 닫기 상태를 만들지 않습니다.

```yaml
RecordUserJudgmentPayload:
  sensitive_action_scope: SensitiveActionScope | null
  accepted_result_refs: StateRecordRef[]
  cancellation_reason: string | null
```

`judgment_kind=sensitive_approval`에서는 `sensitive_action_scope`가 대기 중인 판단과 맞아야 합니다. 민감 동작 승인은 `AuthorizedAttemptScope`를 승인 범위로 직접 저장하면 안 됩니다. 제품 파일 Write Authorization은 별도의 `prepare_write`/`record_run` 계약으로 남습니다. `final_acceptance`에서는 `accepted_result_refs`가 보이는 근거를 이름 붙입니다. `cancellation`에서는 `cancellation_reason`이 필요합니다.

<a id="acceptedriskinput"></a>

## AcceptedRiskInput

```yaml
AcceptedRiskInput:
  visible_risk_ref: StateRecordRef
  accepted: boolean
  user_note: string | null
```

`AcceptedRiskInput`은 `judgment_kind=residual_risk_acceptance`에서만 유효합니다. `visible_risk_ref`는 같은 Task의 보이는 닫기 관련 `blocker`를 가리켜야 합니다. 독립적인 잔여 위험 기록을 만들지 않습니다.

<a id="current-position-display-schemas"></a>

## 현재 위치 표시 스키마

```yaml
CloseBlocker:
  category: task | open_run | scope | user_judgment | sensitive_approval | write_compatibility | baseline | surface_capability | evidence | artifact_availability | final_acceptance | residual_risk_visibility | residual_risk_acceptance | cancellation | supersession | recovery
  code: ErrorCode
  message: string
  related_refs: StateRecordRef[]
  required_judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation | null
  next_action: string

NextActionSummary:
  action_kind: ask_user | update_scope | prepare_write | implement | request_acceptance | close_task | idle
  summary: string
  required_tool: harness.intake | harness.status | harness.update_scope | harness.prepare_write | harness.stage_artifact | harness.record_run | harness.request_user_judgment | harness.record_user_judgment | harness.close_task | null
  related_refs: StateRecordRef[]
  blocker_code: ErrorCode | null
```

`CloseBlocker`는 구조화된 차단 결과입니다. 산문으로만 된 상태 텍스트, 보고서, 렌더링된 보기는 차단 결과가 아닙니다. `harness.close_task intent=complete`에서는 [Core Model](../core-model.md#close_task)이 담당하는 결정적 순서로 차단 범주를 계산합니다. `cancellation`과 `supersession` 범주는 해당 종료 intent와의 충돌을 설명합니다. 성공 완료 증거가 아니며 `completed_self_checked` 또는 `completed_with_risk_accepted`와 섞으면 안 됩니다.

<a id="nextactionsummary"></a>

## NextActionSummary

`NextActionSummary`는 [현재 위치 표시 스키마](#current-position-display-schemas)에 정의되어 있습니다. 활성 `action_kind` 값은 정확히 다음과 같습니다.

```text
ask_user | update_scope | prepare_write | implement | request_acceptance | close_task | idle
```

<a id="validatorresult"></a>

## ValidatorResult

```yaml
ValidatorResult:
  validator_id: surface_capability_check
  validator_kind: capability
  status: passed | warning | failed | blocked | skipped
  guarantee_level: cooperative | detective
  checked_at: string
  target:
    task_id: string | null
    change_unit_id: string | null
    run_id: string | null
    artifact_id: string | null
  summary: string
  findings:
    - code: string
      severity: info | warning | error | blocker
      message: string
      path: string | null
      artifact_ref: ArtifactRef | null
  blocked_reasons: string[]
  suggested_next_action: string | null
```

활성 안정 validator ID는 `surface_capability_check`입니다. `ValidatorResult` 출력은 결과가 이름 붙인 활성 담당 경로를 통해서만 차단 사유, 대체 동작, 보장 표시에 영향을 줄 수 있습니다. 예를 들어 역량이 실제 문제일 때 `CloseBlocker.category=surface_capability`로 이어질 수 있습니다. `status=blocked` 결과나 `findings.severity=blocker`는 설계 정책 차단 사유가 아니며, `design_gate`나 `design_policy`를 활성화하지 않고, 심각도만으로 닫기를 차단하지 않습니다. Write Authorization, 사용자 판단, 증거, 최종 수락, 잔여 위험 수락, 닫기를 만들지 않습니다.

`ValidatorResult.status=passed`만 `detective` 표시에 쓰이는 검증된 역량 상태를 뒷받침할 수 있습니다. `skipped`, `warning`, `failed`, `blocked`는 더 강한 라벨의 근거가 아닙니다. 변경 경로 탐지에서는 프로필 수준의 `changed_path_detection_verification` 값이 반드시 `passed`여야 합니다. `not_run`, 예전 `planned_not_run` 문구, `failed`, `stale`이면 메서드에 따라 표시를 `cooperative`로 유지하거나 `CAPABILITY_INSUFFICIENT`를 반환해야 합니다.

<a id="sensitive-categories"></a>

## 민감 범주

민감 범주는 제품 파일 쓰기에 왜 민감 동작 승인이 필요할 수 있는지 설명합니다. `AuthorizedAttemptScope` 안의 제품 쓰기 분류일 뿐이며, 명령, 호스트, 의존성, 비밀값 핸들, 배포, 파괴적 동작, 시스템 접근의 승인 범위가 아닙니다. 제품 판단, 기술 판단, 범위 판단, QA, 검증, 수락, 잔여 위험, 정책 질문을 결정하지 않습니다. 또한 하네스가 명령, 네트워크 효과, 비밀값 접근을 관찰했다는 뜻도 아닙니다. 활성 `SensitiveCategory` enum은 다음과 같습니다.

```text
auth_change
permission_model_change
schema_change
dependency_change
public_api_change
destructive_write
production_config_change
ci_cd_change
infra_or_deployment_change
privacy_or_pii_change
data_export
telemetry_or_logging_change
license_or_compliance_change
billing_or_cost_change
model_or_prompt_policy_change
policy_override
```

<a id="current-mvp-value-sets"></a>

## 현재 MVP 값 집합

아래 값은 현재 MVP의 활성 스키마 값입니다. 메서드별 역량과 접근 분류 확인은 구체적인 요청에서 어떤 값을 거부할 수 있습니다. 여기에 없는 값은 현재 MVP의 활성 값이 아닙니다. 이 표는 첫 validator 구현이 참조할 수 있는 현재 MVP 값 집합입니다. 화면에 표시되는 라벨은 기준 스키마 값이 아닙니다. 공개 `ErrorCode` 값은 이 표가 아니라 [API Errors](errors.md)가 담당합니다.

| 필드 | 현재 MVP 값 |
|---|---|
| 활성 메서드 집합 | `harness.intake`, `harness.status`, `harness.update_scope`, `harness.prepare_write`, `harness.stage_artifact`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.close_task` |
| `ToolEnvelope.actor_kind` | `user`, `lead_agent` |
| 로컬 API 접근 분류 | `read_status`, `core_mutation`, `write_authorization`, `run_recording`, `artifact_registration`, `artifact_read` |
| `LocalSurfaceRegistration.transport_kind` | `local_mcp_stdio`, `local_http` |
| `LocalSurfaceRegistration.local_access_posture` | `registered_local`, `unavailable`, `mismatch`, `revoked` |
| `LocalSurfaceRegistration.status` | `active`, `disabled`, `stale`, `revoked` |
| `VerifiedSurfaceContext.failure_reason` | `unavailable`, `mismatch`, `revoked`, `insufficient_capability`, `null` |
| `capability_profile.surface_id` | `reference-local-mcp` |
| `capability_profile.surface_status` | `LocalSurfaceRegistration.status`와 같은 값 |
| `capability_profile.local_access_posture` | `LocalSurfaceRegistration.local_access_posture`와 같은 값 |
| `capability_profile.changed_path_detection_verification` | `not_run`, `passed`, `failed`, `stale` |
| `capability_profile.guarantee_level_default` | `cooperative` |
| `capability_profile.guarantee_level_max_when_verified` | `detective` |
| `IntakeRequest.requested_mode` | `advisor`, `direct`, `work`, `auto` |
| `StateSummary.mode`와 지속 저장되는 `tasks.mode` | `advisor`, `direct`, `work` |
| `Task.lifecycle_phase`와 `StateSummary.lifecycle_phase` | `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded` |
| `Task.result`와 `StateSummary.result` | `none`, `advice_only`, `completed`, `cancelled`, `superseded` |
| `Task.close_reason`과 `StateSummary.close_reason` | `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` |
| `StatusResponse.close_state` | `none`, `ready`, `blocked`, `closed`, `cancelled`, `superseded` |
| `CloseTaskResponse.close_state` | `ready`, `blocked`, `closed`, `cancelled`, `superseded` |
| `CloseTaskRequest.intent` | `check`, `complete`, `cancel`, `supersede` |
| `CloseTaskRequest.close_reason` | `Task.close_reason`과 같은 값, 그리고 `null`. 메서드 동작이 각 `intent`에서 유효한 값을 정합니다. |
| `StateSummary.assurance_level` | `none`, `self_checked` |
| `StateSummary.gates.scope_gate` | `not_required`, `required`, `pending`, `passed`, `failed`, `blocked` |
| `StateSummary.gates.decision_gate` | `not_required`, `required`, `pending`, `resolved`, `deferred`, `blocked` |
| `StateSummary.gates.approval_gate` | `not_required`, `required`, `pending`, `granted`, `denied`, `expired` |
| `StateSummary.gates.evidence_gate` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` |
| `StateSummary.gates.acceptance_gate` | `not_required`, `required`, `pending`, `accepted`, `rejected` |
| `StateRecordRef.record_kind` | `project`, `task`, `change_unit`, `run`, `write_authorization`, `user_judgment`, `evidence_summary`, `blocker` |
| `ArtifactRef.kind` | `diff`, `log`, `screenshot`, `checkpoint`, `other` |
| `ArtifactRef.produced_by` | `lead_agent`, `harness` |
| `ArtifactRef.retention_class` | `task`, `project`, `temporary` |
| `ArtifactRelationOwner.record_kind` | `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker` |
| `ArtifactInput.source_kind` | `staged_artifact`, `existing_artifact` |
| `EvidenceCoverageItem.coverage_state` | `supported`, `unsupported`, `partial`, `not_applicable`, `stale`, `blocked` |
| `EvidenceSummary.status` | `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked` |
| `AuthorizedAttemptScope.guarantee_level` | `cooperative`, `detective` |
| `SensitiveActionScope.action_kind` | `product_file_write`, `dependency_change`, `destructive_command`, `network_access`, `secret_access`, `deployment`, `system_access`, `other` |
| `SensitiveActionScope.capability_claim` | `cooperative_only`, `observed_by_surface`, `not_observable` |
| `WriteAuthorizationSummary.status` | `active`, `consumed`, `expired`, `stale`, `revoked` |
| `WriteAuthoritySummary.approval_status` | `not_required`, `required`, `pending`, `granted`, `denied`, `expired`, `unknown` |
| `RunSummary.kind` | `shaping_update`, `implementation`, `direct` |
| `RunSummary.status` | `completed`, `interrupted`, `blocked`, `violation` |
| `UserJudgment.status` | `proposed`, `pending_user`, `resolved`, `deferred`, `rejected`, `blocked`, `superseded` |
| `UserJudgment.judgment_kind` | `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `final_acceptance`, `residual_risk_acceptance`, `cancellation` |
| `UserJudgment.presentation` | `short` |
| `UserJudgment.required_for` | `next_action`, `write`, `run`, `close`, `acceptance`, `risk` |
| `UserJudgmentCandidate.judgment_kind` | `UserJudgment.judgment_kind`와 같은 값 |
| `UserJudgmentCandidate.presentation` | `short` |
| `UserJudgmentCandidate.required_for` | `UserJudgment.required_for`와 같은 값 |
| `UserJudgmentOption.meaning` | `approve`, `reject`, `defer`, `choose`, `cancel` |
| `ArtifactRef.redaction_state` | `none`, `redacted`, `secret_omitted`, `blocked` |
| `CloseBlocker.category` | `task`, `open_run`, `scope`, `user_judgment`, `sensitive_approval`, `write_compatibility`, `baseline`, `surface_capability`, `evidence`, `artifact_availability`, `final_acceptance`, `residual_risk_visibility`, `residual_risk_acceptance`, `cancellation`, `supersession`, `recovery` |
| `CloseBlocker.required_judgment_kind` | `UserJudgment.judgment_kind`와 같은 값, 그리고 `null` |
| `NextActionSummary.action_kind` | `ask_user`, `update_scope`, `prepare_write`, `implement`, `request_acceptance`, `close_task`, `idle` |
| `NextActionSummary.required_tool` | 활성 메서드 집합 값, 그리고 `null` |
| `GuaranteeDisplay.level` | `cooperative`, `detective` |
| `ValidatorResult.validator_id` | `surface_capability_check` |
| `ValidatorResult.validator_kind` | `capability` |
| `ValidatorResult.status` | `passed`, `warning`, `failed`, `blocked`, `skipped` |
| `ValidatorResult.guarantee_level` | `cooperative`, `detective` |
| `ValidatorResult.findings.severity` | `info`, `warning`, `error`, `blocker` |
| `SensitiveCategory` | `auth_change`, `permission_model_change`, `schema_change`, `dependency_change`, `public_api_change`, `destructive_write`, `production_config_change`, `ci_cd_change`, `infra_or_deployment_change`, `privacy_or_pii_change`, `data_export`, `telemetry_or_logging_change`, `license_or_compliance_change`, `billing_or_cost_change`, `model_or_prompt_policy_change`, `policy_override` |

`GuaranteeDisplay.level`에서 `cooperative`는 현재 MVP의 기본값입니다. `detective`도 현재 MVP 값이지만, 활성 접점이 관련 사실을 정직하게 관찰할 수 있고 관련 역량 확인이 실제로 통과한 곳에서만 사용할 수 있습니다. 기준 프로필에서는 `changed_path_detection_verification=passed`가 필요하며 검증된 변경 경로 탐지 범위로 제한됩니다. 두 값 모두 OS 권한, 임의 도구 샌드박스, 변조 방지 저장소, 도구 실행 전 차단, 격리를 뜻하지 않습니다.

Schema Core는 활성 표 안에 비활성 enum 값을 예약하지 않습니다. 이 섹션에 없는 사용자 판단 종류, gate 필드, validator ID, `captured_artifact` 같은 actor/source 값, 더 강한 보장 라벨, 여기에 없는 명령/네트워크/비밀값 관찰 또는 차단 필드, API 메서드는 담당 문서가 승격하고 관련 활성 담당 계약에 추가하기 전까지 비활성입니다.

<a id="later-candidate-value-names"></a>

## 이후 후보 값 이름

이후 후보 값 이름은 승격된 담당 문서가 정확한 활성 필드, 값 집합, validator, 대체 동작, 증명 기대치를 이 문서나 다른 활성 담당 문서에 추가하기 전까지 [이후 후보 색인](../../later/index.md#later-schema-candidates)에만 남는 목록 전용 이름입니다. 이 활성 API 참조는 이후 후보 스키마 본문을 일부러 정의하지 않습니다.
