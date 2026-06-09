# API 오류

## 이 문서로 할 수 있는 일

현재 MVP의 공개 오류 코드, 주 오류 우선순위, 차단 응답과 `dry_run` 동작, 멱등 재실행, 상태 버전 충돌 처리, 문서 스모크 목표의 오류 범위, 닫기 차단 사유 동작, 사용자 표시 라벨 지침을 확인할 때 이 참조를 사용합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 문서 저장소에 MCP 서버가 구현되어 있다는 뜻이 아닙니다.

## 현재 MVP 보장 표시와 profile-gated 주장 경계

`guarantee_display.level`은 승격된 프로필이 profile-gated 표시 값을 명시적으로 지원하지 않는 한 현재 MVP 값인 `cooperative`와 `detective`를 사용합니다. 보안 의미는 [보안 참조: 정직한 보장 표시](../security.md#정직한-guarantee-display)가 담당하고, 정확한 값 집합 경계는 [API Schema Core](schema-core.md#current-mvp-value-sets)가 담당합니다.

프로필 지원 없이 profile-gated 보장 표시 값을 요청하거나 표시하는 것은 보장 주장이 뒷받침된다는 증거가 아니라, 보장 주장 경계 오류입니다. 명령, 네트워크, 비밀값 접근 관찰을 포함해 필요한 차단, 격리, 관찰, 증명 경로 지원이 접점에 없으면 `CAPABILITY_INSUFFICIENT`를 사용합니다. 요청한 값이 활성 프로필이나 요청 형태에서 유효하지 않으면 `VALIDATION_FAILED`를 사용합니다. 어떤 오류도 문서 전용인 현재 저장소에 런타임 강제가 있다는 뜻은 아닙니다.

| 수준 또는 이름 | 오류/상태 의미 |
|---|---|
| `cooperative` | 에이전트나 도구가 문서화된 경로를 따를 때 하네스가 확인하고 기록할 수 있습니다. OS 권한, 샌드박스, 변조 방지 저장소, 실행 전 차단이 아닙니다. |
| `detective` | 관련 역량 확인이 통과한 뒤 하네스 또는 연결된 접점이 관찰 가능한 지원 사실의 불일치를 동작 중이나 이후에 감지, 기록, 보고할 수 있습니다. 예방이 아닙니다. |
| `preventive` | profile-gated 표시 값 이름입니다. 대상 동작에 대한 승격된 도구 실행 전 차단 지원이 없으면 역량 부족 또는 검증 오류를 반환하고 표시되는 `guarantee_display.level` 값을 낮춥니다. |
| `isolated` | profile-gated 표시 값 이름입니다. 이름 붙은 경계에 대한 승격된 격리 지원이 없으면 역량 부족 또는 검증 오류를 반환하고 표시되는 `guarantee_display.level` 값을 낮춥니다. |

활성 MVP 동작은 기본적으로 협력형 확인입니다. 연결된 접점이 사실을 정직하게 관찰할 수 있고 관련 역량 확인이 통과했을 때만 제한된 탐지형 보고를 함께 표시합니다. 이런 보안 비주장은 경계 설명이며 런타임 오류나 강제되는 역량이 아닙니다. 닫기 차단 사유는 사용자 판단, 증거, 잔여 위험 가시성, 잔여 위험 수락 상태를 다루는 구조화된 작업 준비 상태 결과입니다. `preventive` 수준의 도구 실행 전 차단, `isolated` 수준의 격리, 샌드박스, 변조 방지 저장소의 런타임 증명이 아닙니다.

| 조건 | 공개 경로 | 에이전트 규칙 |
|---|---|---|
| `core_or_surface_unavailable` | `MCP_UNAVAILABLE` | 하네스 상태를 만들어 내지 않습니다. Core와 필요한 접점 경로에 다시 닿거나 사용자가 하네스 밖 진행을 명시적으로 선택하기 전까지 하네스에 의존하는 쓰기, 아티팩트 본문 읽기, 닫기를 보류합니다. `VerifiedSurfaceContext.failure_reason=unavailable`에 해당합니다. |
| `local_access_mismatch` | `LOCAL_ACCESS_MISMATCH` | 로컬 파일이나 명령 사실을 추측하지 않고 복사된 `surface_id`를 신뢰하지 않습니다. 등록된 로컬 transport/session/binding을 쓰거나, 담당 경로로 로컬 접근 등록을 고치거나, 입력을 미검증으로 표시합니다. `failure_reason=mismatch` 또는 `revoked`에 해당합니다. |
| `missing_capability` | `CAPABILITY_INSUFFICIENT` | 역량이 맞는 접점을 쓰거나, 동작을 줄이거나, 빠진 관찰, 캡처, 로컬 접근 분류, 차단/격리 주장, 활성 동작이 필요 없는 경로를 선택합니다. 기준 `reference-local-mcp`에서 명령, 네트워크, 비밀값 접근, 접점 자체 아티팩트 캡처, 도구 실행 전 차단, 격리 보장을 요구하는 요청은 요청 형태가 잘못된 경우가 아니라면 이 경로에 속합니다. `failure_reason=insufficient_capability`에 해당합니다. |
| `stale_state` | `STATE_VERSION_CONFLICT`, `BASELINE_STALE`, `PROJECTION_STALE` | 의존하기 전에 현재 상태, baseline, 읽기용 상태 보기, 범위 갱신 결과, 쓰기 전 확인을 새로 확인합니다. 프로젝트 전체 근거 버전이 오래된 Write Authorization은 `STATE_VERSION_CONFLICT`를 사용합니다. |
| `unsupported_surface` | `CAPABILITY_INSUFFICIENT` 또는 `VALIDATION_FAILED` | 요청을 줄이거나, 역량이 맞는 접점으로 옮기거나, 차단 사유를 반환합니다. 지원하지 않는 권한을 설명 문구로 흉내 내지 않습니다. |
| `out_of_scope` | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `NO_ACTIVE_CHANGE_UNIT`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | 영향을 받는 행동을 보류하고, 불일치를 보여 주며, 현재 범위로 줄이거나 구체적인 사용자 소유 범위 판단을 요청하거나, 해결된 범위 변경을 `harness.update_scope`로 적용합니다. |
| `missing_judgment` | `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED` | 집중된 활성 `UserJudgment`를 묻거나 해결합니다. 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 취소 판단, later/reserved QA 면제 판단과 검증 위험 수락 경로를 넓은 승인 하나로 합치지 않습니다. |
| `missing_evidence` | `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING` | 영향을 받는 주장, 참조, 증거 상태, 아티팩트 가용성, 차단 해소에 필요한 최소 조치를 보여 줍니다. 테스트 결과, 아티팩트 무결성, 증거 충분성을 만들어 내지 않습니다. |
| `close_blocked` | `CloseTaskResponse.close_state=blocked`와 주 `ErrorCode` | 유효한 닫기 차단 사유 행렬 평가 뒤 구조화된 차단 사유와 다음 행동을 반환합니다. 행렬 전 거절은 `ToolRejectedResponse`를 반환하며, Task를 종료 상태로 표시하지 않습니다. |
| `residual_risk_present` | `RESIDUAL_RISK_NOT_VISIBLE`, `DECISION_REQUIRED`, 또는 `DECISION_UNRESOLVED` | 잔여 위험을 보여 주고, 활성 닫기 또는 수락 경로가 요구할 때만 `judgment_kind=residual_risk_acceptance`를 묻습니다. |

<a id="error-taxonomy"></a>

## 오류 분류

| 코드 | 의미 |
|---|---|
| `VALIDATION_FAILED` | 요청 본문 형태, enum 값, 활성화 규칙, 프로필별 검증, 또는 `record_run` `ArtifactInput` 검증이 변경 전에 실패했습니다. |
| `STATE_VERSION_CONFLICT` | 프로젝트 전체 최신성 또는 멱등성에 대한 커밋 전 실패입니다. `ToolEnvelope.expected_state_version`이 오래됐거나, 소비 전 `WriteAuthorization.basis_state_version`이 오래됐거나, 같은 `idempotency_key`가 다른 `request_hash`와 함께 재사용될 때만 `effect_kind=no_effect`인 `ToolRejectedResponse`로 반환합니다. |
| `NO_ACTIVE_TASK` | Task가 필요하지만 활성 Task나 지정된 Task가 없습니다. |
| `NO_ACTIVE_CHANGE_UNIT` | 쓰기를 할 수 있거나 닫기와 관련된 동작에 활성 범위 지정 Change Unit이 없습니다. |
| `SCOPE_REQUIRED` | 요청한 쓰기나 동작 전에 범위 확인이 필요합니다. |
| `SCOPE_VIOLATION` | 의도했거나 관찰된 제품 파일 경로나 민감 범주가 활성 범위 또는 저장된 `AuthorizedAttemptScope`를 넘었습니다. |
| `WRITE_AUTHORIZATION_REQUIRED` | 쓰기 가능한 Run에 `harness.prepare_write`에서 요구하는 Write Authorization이 없습니다. |
| `WRITE_AUTHORIZATION_INVALID` | 제공된 Write Authorization이 존재하지만 만료되었거나, 철회되었거나, 재실행 밖에서 이미 소비되었거나, 버전 불일치가 아닌 이유로 호환되지 않습니다. |
| `DECISION_REQUIRED` | 동작 전에 차단 중인 사용자 소유 판단을 요청해야 합니다. |
| `DECISION_UNRESOLVED` | 관련 사용자 판단이 `pending`, 적용 범위 없는 `deferred`, `rejected`, `blocked`, `stale`, `superseded`, 또는 `incompatible` 상태입니다. |
| `AUTONOMY_BOUNDARY_EXCEEDED` | 의도한 동작이 활성 Change Unit Autonomy Boundary를 넘었습니다. |
| `APPROVAL_REQUIRED` | 진행 전에 민감 동작 승인이 필요합니다. |
| `APPROVAL_DENIED` | 관련 민감 동작 승인이 거부되었습니다. |
| `APPROVAL_EXPIRED` | 관련 민감 동작 승인이 만료되었거나 범위/baseline에서 달라졌습니다. |
| `CAPABILITY_INSUFFICIENT` | 접점은 인식되었지만 필요한 접근 분류, 관찰, 캡처, 차단/격리 조건, 보장 주장, 활성 동작을 충족할 수 없습니다. |
| `MCP_UNAVAILABLE` | 필요한 MCP/Core 또는 접점 도달 가능성 자체를 사용할 수 없거나 닿을 수 없어 서버가 쓸 수 있는 로컬 접점 맥락을 파생할 수 없습니다. |
| `LOCAL_ACCESS_MISMATCH` | 등록된 로컬 접근 기대가 도달 가능한 transport/session/binding, `surface_id`/project/surface-instance 짝과 맞지 않거나 로컬 접근이 철회되었습니다. |
| `EVIDENCE_INSUFFICIENT` | 필요한 증거 범위가 없거나, 부분적이거나, 오래되었거나, 막혔습니다. |
| `ACCEPTANCE_REQUIRED` | 필요한 최종 수락이 대기 중이거나, 거부되었거나, 표시된 결과 근거와 호환되지 않습니다. |
| `PROJECTION_STALE` | 요청한 읽기용 상태/보기가 오래되었거나 실패했습니다. Core 상태가 아니며 그 자체로 닫기 차단 사유가 아닙니다. |
| `RESIDUAL_RISK_NOT_VISIBLE` | 닫기에 영향을 주는 알려진 잔여 위험이 최종 수락 또는 닫기 전에 보이지 않았습니다. |
| `ARTIFACT_MISSING` | 참조한 지속 아티팩트가 없거나, 사용할 수 없거나, 닫기 근거로 쓸 수 없거나, 무결성/메타데이터 확인에 실패했습니다. |
| `BASELINE_STALE` | 동작에 필요한 저장소 상태와 baseline이 더 이상 맞지 않습니다. |
| `VALIDATOR_FAILED` | 필수 활성 validator 또는 차단 사유 확인이 실패했고, 더 구체적인 타입 코드가 없을 때 쓰는 대체 코드입니다. 현재 MVP에서 설계 정책 오류가 아닙니다. 설계 품질 우려는 활성 판단, 차단 사유, 증거, 역량, 잔여 위험 경로로 라우팅되거나 조언으로 남아야 합니다. |

`ToolError.details.authorization_reason`은 정확히 다음 값만 사용합니다.

```text
missing | expired | stale | revoked | consumed | incompatible
```

필요한 권한이 제공되지 않았으면 `authorization_reason=missing`과 함께 `WRITE_AUTHORIZATION_REQUIRED`를 사용합니다. 기존 권한을 버전 불일치가 아닌 이유로 소비할 수 없으면 `authorization_reason=expired`, `revoked`, `consumed`, `incompatible` 중 맞는 값과 함께 `WRITE_AUTHORIZATION_INVALID`를 사용합니다.
제공된 Write Authorization이 프로젝트 전체 `basis_state_version`과 현재 `project_state.state_version`의 불일치 때문에 오래된 경우에는 `authorization_reason=stale`과 함께 `STATE_VERSION_CONFLICT`를 사용합니다. 이 응답은 `effect_kind=no_effect`인 `ToolRejectedResponse`입니다. 커밋된 `write_compatibility` `CloseBlocker`가 아니며 Write Authorization 소비 없음이 적용됩니다.

`ArtifactInput.source_kind`와 출처 필드가 스키마 형태에 맞지 않으면 `VALIDATION_FAILED`를 사용합니다. `harness.record_run` 중 유효하지 않은 스테이징된 핸들 검증은 커밋 전 실패이며 `ToolRejectedResponse`를 반환합니다. `ArtifactInput.source_kind=staged_artifact`의 스테이징된 핸들 검증 실패도 공개 `VALIDATION_FAILED`를 사용하고, `ToolError.details.artifact_input_error`에 구조화된 세부정보를 둡니다. 스테이징된 핸들 검증 실패마다 새 top-level 공개 오류 코드를 만들지 않습니다.

`ToolError.details.artifact_input_error`는 입력 id와 구체적인 사유를 담아야 합니다. 활성 세부 사유 집합에는 아래 값이 포함됩니다.

```yaml
artifact_input_error:
  artifact_input_id: string
  reason:
    - staged_handle_expired
    - staged_handle_consumed
    - staged_handle_project_mismatch
    - staged_handle_task_mismatch
    - staged_handle_surface_mismatch
    - staged_handle_checksum_mismatch
    - staged_handle_size_mismatch
    - staged_handle_not_found
```

스테이징된 핸들 검증은 저장된 `project_id`, `task_id`, `created_by_surface_id`, `created_by_surface_instance_id`, 만료 여부, 소비 상태, `sha256`, `size_bytes`, `redaction_state`를 다룹니다. `redaction_state`가 맞지 않는 경우에는 메시지나 추가 detail 필드에서 그 필드를 이름 붙이되 공개 코드는 `VALIDATION_FAILED`로 유지합니다. 스테이징된 핸들의 출처나 범위가 맞지 않는 것은 검증 오류이지 요청 수준 로컬 접근 실패가 아닙니다. 스테이징된 핸들 출처 불일치에 `LOCAL_ACCESS_MISMATCH`를 쓰지 않습니다. `LOCAL_ACCESS_MISMATCH`는 요청 접점 검증 실패에만 씁니다. 스테이징된 핸들 범위나 출처 불일치에 `CAPABILITY_INSUFFICIENT`도 쓰지 않습니다. `CAPABILITY_INSUFFICIENT`는 확인된 접점 역량이 없거나 부족할 때만 씁니다. `ARTIFACT_MISSING`은 참조된 지속 아티팩트와 닫기 관련 아티팩트 가용성에 남겨 두며 스테이징된 핸들 검증에는 쓰지 않습니다.

로컬 접근 관련 코드는 좁게 쓰고 서로 구분합니다. `MCP_UNAVAILABLE`은 MCP/Core 또는 접점 도달 가능성 자체를 사용할 수 없을 때 쓰며, `VerifiedSurfaceContext.failure_reason=unavailable`을 포함합니다. 이 거절 전에 Core 상태를 읽을 수 없으면 `ToolRejectedResponse.state_version`은 `null`일 수 있습니다. 상태를 읽었다면 관찰한 프로젝트 전체 `project_state.state_version`을 담아야 합니다. `LOCAL_ACCESS_MISMATCH`는 도달 가능한 로컬 transport/session/binding이 등록된 프로젝트 접점과 맞지 않거나 로컬 접근이 철회되었을 때 쓰며, `failure_reason=mismatch` 또는 `revoked`를 포함합니다. `CAPABILITY_INSUFFICIENT`는 인식된 활성 접점이 요청한 접근 분류나 보장 주장에 필요한 역량을 갖추지 못했을 때 쓰며, `failure_reason=insufficient_capability`을 포함합니다. `surface_id`만으로는 이 오류 중 어느 것도 해결되지 않습니다. 이 공개 경로 대신 접점별 `UNAUTHORIZED` code를 만들지 않습니다.

<a id="primary-error-code-precedence"></a>

## 주 오류 코드 우선순위

오류를 담는 응답 분기의 `errors`가 비어 있지 않으면 메서드 섹션이 더 좁은 순서를 정의하지 않는 한 `errors[0]`이 아래 순서로 선택된 주 공개 코드입니다. `ToolRejectedResponse`에서는 `ToolRejectedResponse.errors[0]`이 주 거절 코드입니다. 커밋된 차단 결과나 진단을 담은 결과에서는 `MethodResult.base.errors[0]`이 주 공개 코드입니다. 보조 차단 사유는 메서드별 필드와 `ToolError.details`에 남을 수 있습니다. 유효한 `ToolDryRunResponse`는 `errors=[]`를 유지하며, 미리 볼 수 있는 예상 실패는 `DryRunSummary.would_errors`에 둡니다.

`STATE_VERSION_CONFLICT`는 이 우선순위 표에서 `ToolRejectedResponse` 분기에만 나타납니다. `MethodResult.base.errors[0]`, `CloseTaskResult(close_state=blocked).errors[0]`, 또는 커밋된 닫기 차단 결과의 주 오류로 선택하면 안 됩니다.

| 우선순위 | 주 `ErrorCode` |
|---:|---|
| 1 | `VALIDATION_FAILED` |
| 2 | `STATE_VERSION_CONFLICT` |
| 3 | `MCP_UNAVAILABLE` |
| 4 | `LOCAL_ACCESS_MISMATCH` |
| 5 | `NO_ACTIVE_TASK` |
| 6 | `NO_ACTIVE_CHANGE_UNIT` |
| 7 | `BASELINE_STALE` |
| 8 | `SCOPE_REQUIRED` |
| 9 | `SCOPE_VIOLATION` |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` |
| 11 | `WRITE_AUTHORIZATION_INVALID` |
| 12 | `APPROVAL_DENIED` |
| 13 | `APPROVAL_EXPIRED` |
| 14 | `APPROVAL_REQUIRED` |
| 15 | `DECISION_UNRESOLVED` |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` |
| 17 | `DECISION_REQUIRED` |
| 18 | `CAPABILITY_INSUFFICIENT` |
| 19 | `EVIDENCE_INSUFFICIENT` |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` |
| 21 | `ACCEPTANCE_REQUIRED` |
| 22 | `PROJECTION_STALE` |
| 23 | `ARTIFACT_MISSING` |
| 24 | `VALIDATOR_FAILED` |

<a id="blocked-and-dry-run-behavior"></a>

## 차단 응답과 `dry_run` 동작

모든 공개 응답은 정확히 하나의 응답 분기입니다. 분기는 `ToolRejectedResponse`, `ToolResultBase`를 바탕으로 한 메서드별 `MethodResult`, 또는 `ToolDryRunResponse` 중 하나입니다. 어떤 분기를 쓰는지는 표시 방식이 아니라 계약입니다.

응답 분기 선택은 아래 우선순위를 따릅니다.

1. 커밋 전 실패는 `dry_run` 값과 무관하게 `ToolRejectedResponse`를 반환합니다. 오래된 `expected_state_version`, 소비 전 오래된 `WriteAuthorization.basis_state_version`, 요청 검증 실패, 로컬 접근 실패, 역량 부족, 상태 조회 실패, 스테이징된 핸들 검증 실패가 여기에 속합니다.
2. 유효한 읽기 전용 선택 동작은 `dry_run=true`여도 메서드별 `MethodResult`를 반환합니다. 이 결과는 `base.dry_run=true`, `base.effect_kind=read_only`를 사용합니다.
3. Core 커밋이나 저장소 소유 스테이징 부작용을 만들 수 있는 유효한 선택 동작은 `dry_run=true`이고 Core가 미리보기를 만들 수 있을 때 `ToolDryRunResponse`를 반환합니다.
4. 성공한 `dry_run=false` 커밋 또는 스테이징 동작은 메서드별 `MethodResult`를 반환합니다.

`dry_run=true`는 `ToolDryRunResponse`의 동의어가 아닙니다. `dry_run=true`는 주 오류를 가리지 않고, 유효한 읽기 전용 메서드 결과를 `ToolDryRunResponse` 분기로 바꾸지도 않습니다.

`ToolRejectedResponse`는 `STATE_VERSION_CONFLICT`, 요청 검증 실패, 스테이징된 핸들 검증 실패 같은 커밋 전 실패에 쓰는 거절 응답입니다. `response_kind=rejected`, `effect_kind=no_effect`를 가지며 메서드별 결과 객체가 없습니다. `decision`, `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, `close_state` 같은 결과 전용 필드를 포함하면 안 됩니다. 거절 응답은 현재 기록, `task_events`, 재실행 행, 아티팩트, 스테이징된 핸들 소비, 증거 요약, Write Authorization 생성 또는 소비, 닫기 상태 변경, `state_version` 증가를 만들지 않습니다.

커밋된 차단 응답은 `ToolRejectedResponse`가 아닙니다. [현재 MVP API](mvp-api.md#active-mvp-method-behavior)의 메서드별 상태 효과 계약이 커밋된 차단 결과를 허용할 때만 `PrepareWriteResult`나 `CloseTaskResult` 같은 메서드별 결과 스키마 안에서 반환합니다. 커밋된 차단 결과는 `base.response_kind=result`를 가지며, 그 메서드가 허용한 차단 사유나 상태 효과만 커밋할 수 있습니다. `blockers`, 이벤트, 프로젝트 전체 `state_version`, `tool_invocations` 재실행 행을 업데이트할 수 있지만, 차단 사유가 없거나 부족하다고 지적한 권한을 만들면 안 됩니다.

`harness.status`와 `harness.close_task intent=check`를 포함한 읽기 전용 호출은 차단 사유나 닫기 차단 사유를 계산해 반환할 수 있습니다. 그 차단 사유는 응답 필드일 뿐입니다. Core는 읽기를 이유로 차단 사유를 저장하거나, 이벤트를 추가하거나, `tool_invocations` 재실행 행을 만들거나, 상태 버전을 올리면 안 됩니다. 요청이 그 외에는 유효하면 이런 호출은 `dry_run=true`여도 메서드별 결과 분기를 반환합니다.

`ToolDryRunResponse`는 상태 효과가 있는 선택 동작이나 저장소 소유 스테이징 동작의 유효한 `dry_run` 미리보기에만 쓰는 응답 분기입니다. `dry_run=true`는 권한 근거가 아닙니다. 요청 형태, 로컬 접근 확인, 역량 확인, 조회 가능한 상태와 선행조건을 미리보기로 만들 만큼 평가할 수 있는 유효한 `dry_run` 호출은 `ToolDryRunResponse`와 `DryRunSummary`를 반환합니다. 상태 효과 없음: 진단, 후보 차단 사유, `DryRunSummary.would_errors`, `DryRunSummary.next_actions`, 설명용 `PlannedEffect` 예상 효과를 반환할 수 있지만 현재 기록, 이벤트, 아티팩트, 증거 요약, Write Authorization 생성 또는 소비, 닫기 상태, 커밋된 `tool_invocations` 재실행 행, 스테이징된 핸들 생성 또는 소비, 상태 버전 증가를 만들거나 업데이트하면 안 됩니다. 또한 `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref` 같은 메서드별 결과 전용 필드나 실제 생성 참조를 포함하면 안 됩니다. `PlannedEffect`는 미리보기 정보일 뿐이며 아직 존재하지 않는 기록의 가짜 생성 ref를 담으면 안 됩니다.

예시는 다음과 같습니다.

| 요청 조건 | 응답 분기 |
|---|---|
| `harness.status`에 `dry_run=true`를 보낸 유효한 읽기 | `base.dry_run=true`, `base.effect_kind=read_only`인 `StatusResult` |
| `harness.close_task intent=check`에 `dry_run=true`를 보낸 유효한 읽기 | `base.dry_run=true`, `base.effect_kind=read_only`인 `CloseTaskResult` |
| `harness.close_task intent=complete`에 `dry_run=true`를 보냈고 그 외에는 유효하며 미리보기 가능한 경우 | `effect_kind=no_effect`인 `ToolDryRunResponse` |
| 오래된 `expected_state_version`을 `dry_run=true`와 함께 보낸 경우 | 주 오류가 `STATE_VERSION_CONFLICT`이고 `dry_run=true`, `effect_kind=no_effect`인 `ToolRejectedResponse` |

`dry_run=true` 요청 자체가 읽기 전용 결과나 미리보기 생성 전에 요청 검증, 로컬 접근 확인, 역량 확인, 상태 조회, 오래된 상태 확인에서 실패하면 응답은 `dry_run=true`, `effect_kind=no_effect`인 `ToolRejectedResponse`입니다. 이후 비 `dry_run` 호출은 현재 상태를 기준으로 다시 검증해야 합니다.

<a id="idempotency"></a>

## 멱등성

커밋되는 상태 변경 메서드는 모두 `idempotency_key`를 요구합니다. 읽기 전용 호출은 재실행 행을 만들지 않고 키를 예약하지도 않습니다. 키는 `(project_id, tool_name, idempotency_key)` 범위를 가집니다.

`request_hash`는 도구 이름, 스키마 정규화된 요청 본문, 그리고 `request_id`와 `idempotency_key`를 제외한 모든 `ToolEnvelope` 필드에 대한 정규 JSON에서 계산합니다.

재실행 행을 만드는 상태 효과의 커밋된 `dry_run=false` `MethodResult` 응답만 재실행 행에 저장합니다. 같은 키와 같은 `request_hash`를 가진 커밋된 재실행 행이 있으면 Core는 최신성 확인을 다시 실행하거나 이벤트 추가, 아티팩트 승격/연결, Write Authorization 소비, 차단 사유 업데이트, 재실행 행 변경을 하지 않고 원래 커밋된 응답을 반환합니다. 같은 `idempotency_key`를 다른 `request_hash`로 재사용하면 Core는 기존 재실행 행을 보존하고 `STATE_VERSION_CONFLICT`와 `effect_kind=no_effect`를 담은 `ToolRejectedResponse`를 반환합니다. 새 재실행 행은 만들거나 예약하지 않습니다.

`ToolRejectedResponse`와 `ToolDryRunResponse`는 재실행 행을 만들거나 예약하지 않습니다.

<a id="state-conflict-behavior"></a>

## 상태 버전 충돌 처리

`STATE_VERSION_CONFLICT`의 현재 MVP 의미는 하나뿐입니다. 프로젝트 전체 최신성 또는 멱등성에 대한 커밋 전 실패입니다. 응답 분기는 항상 `effect_kind=no_effect`인 `ToolRejectedResponse`입니다. 현재 MVP에서 이 코드는 아래 경우에만 씁니다.

- `ToolEnvelope.expected_state_version`이 현재 `project_state.state_version`보다 오래된 경우.
- 소비 전 `WriteAuthorization.basis_state_version`이 현재 `project_state.state_version`보다 오래된 경우.
- 같은 `idempotency_key`가 다른 `request_hash`와 함께 재사용된 경우.

커밋된 재실행 행이 없는 새 상태 변경 시도에서 Core는 담당 기록을 고르기 위해 최신성 확인 전에 기본 Task를 찾을 수 있습니다. 해석 순서는 도구별 `task_id`, `ToolEnvelope.task_id`, 활성 Task입니다. 이 해석은 별도 상태 시계를 고르지 않습니다. 새 `dry_run=false` 상태 변경은 모두 커밋 전에 `ToolEnvelope.expected_state_version`을 현재 프로젝트 전체 `project_state.state_version`과 비교합니다. `dry_run=true` 요청이 오래된 `expected_state_version`을 제공해도 읽기 전용 결과나 `ToolDryRunResponse` 미리보기 전에 같은 커밋 전 거절이 적용됩니다.

`STATE_VERSION_CONFLICT`는 메서드별 결과가 아닙니다. `MethodResult.decision` 값이 아니고, `CloseTaskResult.close_state`가 아니며, `CloseBlocker.code`도 아닙니다. 또한 `CloseTaskResult(close_state=blocked).errors[0]`이나 커밋된 닫기 차단 결과의 주 오류로 쓰면 안 됩니다. 커밋된 `CloseTaskResult(close_state=blocked)`에서는 `errors[0]`과 `blockers[*].code`가 닫기 차단 사유 행렬 실행 뒤 발견한 의미 차단 사유만 설명할 수 있습니다. 커밋 전 실패 코드는 그 자리에 넣지 않습니다.

이 거절 응답은 현재 기록, `task_events`, 재실행 행, 아티팩트, 스테이징된 핸들 소비, 증거 요약, Write Authorization 생성 또는 소비, 닫기 상태 변경, `state_version` 증가를 만들지 않습니다. 상태 효과 없음이 적용됩니다. `tasks.state_version`은 활성 충돌 기준이나 동시성 기준이 아닙니다.

같은 `idempotency_key`가 다른 `request_hash`와 함께 재사용된 충돌에서는 기존 재실행 행을 보존합니다. 거절된 요청을 위해 재실행 행을 만들거나, 예약하거나, 갈라 놓거나, 덮어쓰지 않습니다.

`WriteAuthorization.basis_state_version`이 오래된 충돌에서는 소비 전에 거절 응답을 반환합니다. 이 조건은 `WRITE_AUTHORIZATION_INVALID`가 아니고, 커밋된 `write_compatibility` `CloseBlocker`가 아니며, Write Authorization 소비 없음이 적용됩니다.

프로젝트 전체 상태 버전 불일치와 멱등성 `request_hash` 충돌에 쓰는 현재 MVP의 유일한 공개 `ErrorCode`는 `STATE_VERSION_CONFLICT`입니다. 이 불일치에 대해 다른 공개 코드, 별칭, 폐기된 표기, 저장소 계층의 다른 공개 오류 이름, 내부 예외 이름을 노출하지 않습니다.

오래된 `ToolEnvelope.expected_state_version`의 `STATE_VERSION_CONFLICT.details`에는 다음 값을 담아야 합니다.

```yaml
state_clock: project_state.state_version
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

멱등성 충돌의 `STATE_VERSION_CONFLICT.details`는 민감한 요청 본문을 노출하지 않고 `idempotency_key`와 `request_hash` 불일치를 식별해야 합니다. 오래된 `WriteAuthorization.basis_state_version`의 세부정보는 오래된 권한 근거 버전과 현재 `project_state.state_version`을 식별해야 합니다.

<a id="documentation-smoke-error-coverage"></a>

## 문서 스모크 오류 범위

[MVP 계획](../../build/mvp-plan.md#첫-내부-스모크-목표)의 첫 내부 문서 스모크 목표는 활성 공개 오류와 활성 `CloseBlocker.category` 값만 사용해야 합니다. 스모크 전용 코드를 만들지 않고, 완전한 적합성 테스트 모음이나 구현 계획을 정의하지 않습니다.

- 등록된 접점 검증은 Core가 등록된 접점에 맞는 `VerifiedSurfaceContext`를 파생할 때만 오류 없이 성공합니다. 실패는 `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`를 사용합니다. 복사된 `surface_id`는 접근이나 역량의 증거가 아닙니다.
- 프로젝트 전체 상태 버전 충돌은 `ToolEnvelope.expected_state_version`이 `project_state.state_version`보다 오래되었을 때 `STATE_VERSION_CONFLICT`를 담은 `ToolRejectedResponse`를 반환합니다. 실패한 시도는 기록, 이벤트, 아티팩트, 증거, Write Authorization 생성 또는 소비, 닫기 상태, 재실행 행, 스테이징된 핸들 소비, 상태 버전 증가를 만들면 안 됩니다.
- `ShapingReadiness` 공백은 담당 경로에 따라 `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, 또는 구조화된 차단 사유로 드러날 수 있습니다. 읽기 전용 상태나 준비 상태 읽기는 상태를 바꾸지 않습니다.
- `prepare_write decision=allowed`는 담당 범위의 1회용 Write Authorization을 만듭니다. `decision=blocked`와 `decision=approval_required`는 메서드별 상태 효과 표가 차단 커밋을 허용할 때만 커밋된 `PrepareWriteResult` 값이며, 소비 가능한 Write Authorization을 만들면 안 됩니다. `STATE_VERSION_CONFLICT`와 요청 검증 실패는 `ToolRejectedResponse` 분기이고 절대 `PrepareWriteResult.decision` 값이 아닙니다.
- `SensitiveActionScope`는 `judgment_kind=sensitive_approval`에 속합니다. 민감 동작 승인 오류는 `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`를 사용합니다. 이 승인은 Write Authorization, 최종 수락, 잔여 위험 수락, 증거, 아티팩트 권한을 대신하지 않습니다.
- `harness.stage_artifact` 성공은 임시 핸들만 만들고 Core 변경을 만들지 않습니다. 유효한 스테이징 핸들을 지속 `ArtifactRef`로 승격할 수 있는 활성 경로는 `harness.record_run`입니다. 출처 필드 형태가 잘못되었거나 스테이징된 핸들 검증이 실패하면, 실제 실패가 요청 수준 로컬 접근이나 역량 확인이 아닌 한 `artifact_input_error` detail을 담은 `VALIDATION_FAILED`의 `ToolRejectedResponse`를 반환합니다. 이런 조건을 증거 충분성, 로컬 접근 불일치, 역량 부족으로 숨기면 안 됩니다.
- `harness.record_run`은 호환되는 Write Authorization을 정확히 한 번 소비합니다. Write Authorization이 없으면 `WRITE_AUTHORIZATION_REQUIRED`를 사용합니다. 프로젝트 전체 근거 버전이 오래된 Write Authorization은 `STATE_VERSION_CONFLICT`를 사용합니다. 만료, 철회, 이미 소비됨, 버전 불일치가 아닌 비호환 상태이면 `WRITE_AUTHORIZATION_INVALID`를 사용합니다. 승인 범위 밖 관찰 시도는 적용되는 범위 또는 Write Authorization 관련 코드를 사용합니다.
- `close_task intent=check`는 차단 사유를 반환하더라도 읽기 전용입니다. `close_task intent=complete`는 구조화된 차단 사유와 함께 `CloseTaskResponse.close_state=blocked`를 반환하거나, 담당 문서가 정의한 complete 차단 사유가 없을 때만 `close_state=closed`를 반환합니다.
- 닫기 스모크 범위는 증거 차단 사유의 `EVIDENCE_INSUFFICIENT`, 아티팩트 사용 불가 또는 누락 차단 사유의 `ARTIFACT_MISSING`, 최종 수락 차단 사유의 `ACCEPTANCE_REQUIRED`, 보이지만 수락되지 않은 잔여 위험에 대한 `category=residual_risk_acceptance`와 `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`를 포함해야 합니다. `RESIDUAL_RISK_NOT_VISIBLE`은 아직 보이지 않은 위험에만 둡니다.
- `close_task intent=supersede`가 유효하지 않으면 supersession, 생명주기, 로컬 접근, 복구 차단 사유를 사용합니다. 오래된 `expected_state_version`, 오래된 `WriteAuthorization.basis_state_version`, `idempotency_key`의 `request_hash` 충돌은 `ToolRejectedResponse` 경우이며 커밋된 차단 사유가 아닙니다. 증거 충분성, 최종 수락, 잔여 위험 수락을 요구하면 안 됩니다. 유효한 supersede가 생명주기와 `project_state.active_task_id`를 함께 바꾸는 경우에도 하나의 프로젝트 전체 상태 변경입니다.

<a id="harnessclose_task-close-blockers"></a>

## `harness.close_task` 닫기 차단 사유

`CloseTaskResponse.blockers`는 [API Schema Core](schema-core.md#current-position-display-schemas)의 구조화된 `CloseBlocker` 객체를 사용해야 합니다. 설명 문구만 있는 상태 텍스트, 보고서 텍스트, 렌더링된 보기, 에이전트 요약은 닫기 차단 사유 결과가 아닙니다.

`harness.close_task`에는 닫기 차단 사유 행렬 실행 전의 커밋 전 거절 경계가 있습니다. 아래 조건은 반드시 `ToolRejectedResponse`를 반환해야 하며 `CloseTaskResult(close_state=blocked)`를 반환하면 안 됩니다.

- `expected_state_version`이 현재 `project_state.state_version`과 맞지 않는 경우.
- 같은 `idempotency_key`를 다른 `request_hash`로 재사용한 경우.
- `WriteAuthorization.basis_state_version`이 오래된 경우.
- 닫기 차단 사유 행렬 실행 전 요청 형태 검증이 실패한 경우.
- 닫기 차단 사유 행렬 실행 전 로컬 접근 또는 역량 확인이 실패한 경우.
- 닫기 차단 사유 행렬 실행 전 Core 상태를 읽을 수 없는 경우.
- 닫기 차단 사유 행렬 실행 전 Project 또는 Task 식별자를 확정할 수 없는 경우.

닫기 행렬 전 거절 응답은 `effect_kind=no_effect`입니다. `CloseBlocker` 없음, `task_event` 또는 `task_events` 추가 없음, 재실행 행 없음, `tool_invocations.response_json` 없음, `close_state` 변경 없음, Write Authorization 생성 없음, Write Authorization 소비 없음, 스테이징된 핸들 소비 없음, 아티팩트 승격 또는 연결 없음, 증거 요약 갱신 없음, `project_state.state_version` 증가 없음이 적용됩니다. `STATE_VERSION_CONFLICT`는 커밋 전 거절 오류일 뿐이며 절대 `CloseBlocker.code`가 될 수 없습니다.

유효한 닫기 차단 사유 행렬 평가에서 발견한 의미 차단 사유만 `CloseTaskResult(close_state=blocked)`와 커밋된 닫기 차단 결과로 반환할 수 있습니다. 이런 커밋된 닫기 차단 결과의 `CloseTaskResult(close_state=blocked).errors[0]`과 `blockers[*].code`는 행렬 실행 뒤 발견한 의미 차단 사유를 설명합니다. 커밋 전 실패 코드를 그 자리에 넣으면 안 되며, `STATE_VERSION_CONFLICT`는 커밋된 닫기 차단 결과 밖에 있습니다. 상태 효과가 있는 `intent=complete`, `intent=cancel`, `intent=supersede`의 유효한 `dry_run` 미리보기는 계속 `ToolDryRunResponse`를 반환합니다. 커밋 전 실패는 `dry_run=true`여도 `ToolRejectedResponse`입니다.

`harness.close_task intent=complete`의 닫기 차단 사유는 [Core Model](../core-model.md#close_task)의 결정적 행렬 순서로 정렬합니다. 공개 오류 우선순위는 메서드가 주 `ErrorCode` 하나를 골라야 할 때 여전히 쓰이지만, complete 차단 행렬의 순서를 바꾸거나 앞선 차단 사유를 뒤의 수락/위험 확인 아래 숨기면 안 됩니다. 증거 차단 사유는 보통 `EVIDENCE_INSUFFICIENT`를 사용합니다. 사용할 수 없거나 누락된 닫기 관련 아티팩트를 포함한 아티팩트 가용성 차단 사유는 `ARTIFACT_MISSING`을 사용합니다. 해결되지 않은 사용자 판단 차단 사유는 `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`를 사용합니다. 민감 동작 승인 차단 사유는 `APPROVAL_*` 코드를 사용합니다. 범위 차단 사유는 범위와 baseline 코드를 사용합니다.

`intent=cancel`과 `intent=supersede`는 성공 완료가 아닙니다. 이 intent의 차단 응답은 Task 식별자나 생명주기, 로컬 접근, 복구 제약, cancellation 충돌, supersession 유효성처럼 해당 종료 전이를 무효로 만드는 조건으로 제한합니다. 증거 충분성, 최종 수락, 잔여 위험 수락을 요구하면 안 되며, 그런 누락 조건을 cancellation이나 supersession의 차단 사유로 쓰면 안 됩니다.

닫기에 영향을 주는 알려진 잔여 위험이 아직 보이지 않으면 `RESIDUAL_RISK_NOT_VISIBLE`를 사용합니다. 보이지만 수락되지 않은 닫기 관련 잔여 위험은 이 코드 아래 숨기지 않습니다. 잔여 위험 수락이 필요하면 닫기 차단 사유는 `category=residual_risk_acceptance`와 `required_judgment_kind=residual_risk_acceptance`를 사용하고, `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`를 반환합니다.

`PROJECTION_STALE`은 읽기용 보기 최신성 오류입니다. 그 자체로 활성 닫기 차단 사유 `category`가 아닙니다.

Run 실패, violation, Projection 실패, 아티팩트 무결성 실패, validator 실패, 증거 공백, 차단 사유를 `Task.result=failed` 같은 종료 결과로 바꾸면 안 됩니다. 무엇이 막혔거나 무엇을 복구해야 하는지는 해당 status, 오류, 증거, 아티팩트, 차단 사유 기록에 남깁니다.

## 사용자 표시 라벨 지침

아래 라벨은 표시 지침이지 새 공개 오류 코드가 아닙니다.

| API 조건 | 사용자 표시 라벨 | 차단 해소에 필요한 최소 조치 |
|---|---|---|
| `VALIDATION_FAILED` | 잘못된 요청 | 다시 시도하기 전에 요청 본문, enum 값, 활성화 규칙, 필드 집합을 고칩니다. |
| `STATE_VERSION_CONFLICT` | 상태 버전 충돌 | 현재 상태를 새로 고치고 현재 상태 버전으로 다시 시도하거나 원래 멱등 요청을 재실행합니다. |
| `MCP_UNAVAILABLE` | Core 또는 접점 사용 불가 | 상태 변경, gate 업데이트, 쓰기 호환성, 아티팩트 본문 접근, 닫기를 주장하기 전에 MCP/Core와 접점 도달 가능성을 다시 연결하거나 진단합니다. |
| `LOCAL_ACCESS_MISMATCH` | 로컬 접근 불일치 | 하네스 상태에 의존하기 전에 등록된 로컬 transport/session/binding을 사용하거나 담당 경로로 로컬 접근 등록을 고칩니다. |
| `CAPABILITY_INSUFFICIENT` | 지원되지 않거나 부족한 접점 | 역량이 있는 접점을 사용하거나, 동작을 줄이거나, 누락된 역량이 필요 없는 경로를 선택합니다. |
| `NO_ACTIVE_TASK` | 활성 Task 없음 | Task 범위 동작 전에 Task를 선택하거나 생성합니다. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | 범위, 경계, baseline 문제 | 범위를 확인하거나 좁히고, 범위 변경이 유효하면 `harness.update_scope`로 Change Unit이나 baseline을 갱신하거나, 필요한 사용자 판단을 요청합니다. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | 쓰기 전 범위 확인 없음 또는 사용할 수 없음 | 정확한 동작, 현재 범위, 현재 상태로 `harness.prepare_write`를 호출하거나 다시 시도합니다. 프로젝트 전체 상태 버전 차이는 `STATE_VERSION_CONFLICT`로 표시합니다. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | 판단 필요 | 종류, 참조, 선택지, 결과와 함께 집중된 `UserJudgment`를 보여 주거나 해결합니다. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | 민감 동작 승인 필요 또는 사용 불가 | `judgment_kind=sensitive_approval` 사용자 판단을 요청, 해결, 갱신합니다. |
| `EVIDENCE_INSUFFICIENT` | 증거 필요 | 누락된 확인을 기록하거나 다시 실행하고, 증거 공백과 차단 해소에 필요한 최소 조치를 보여 줍니다. |
| `ACCEPTANCE_REQUIRED` | 최종 수락 필요 | 표시된 결과 근거에 대해 `judgment_kind=final_acceptance`를 요청하거나 해결합니다. |
| `RESIDUAL_RISK_NOT_VISIBLE` | 잔여 위험이 보이지 않음 | 최종 수락 또는 닫기 전에 닫기 관련 잔여 위험을 보여 줍니다. |
| `PROJECTION_STALE` | 읽기용 보기 오래됨 | 그 보기를 새로 고친 뒤 의존합니다. 기준 닫기 상태로 취급하지 않습니다. |
| `ARTIFACT_MISSING` | 아티팩트 문제 | 누락되었거나 사용할 수 없거나 닫기 근거로 쓸 수 없거나 실패한 아티팩트를 다시 첨부, 다시 생성, 가용성 복구, 교체한 뒤 의존합니다. |
| `VALIDATOR_FAILED` | 확인 또는 차단 사유 실패 | 특정 validator 또는 차단 사유를 보여 줍니다. 타입 있는 차단 사유가 없을 때만 이 대체 코드를 사용합니다. 설계 정책 차단 사유로 사용하면 안 됩니다. |
