# API 오류

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 이 문서 저장소에 MCP 서버나 런타임 동작이 구현되어 있다는 뜻이 아닙니다.

## 이 문서가 담당하는 것

| 이 문서가 담당하는 것 | 경계 |
|---|---|
| 공개 `ErrorCode` 식별자 | 공개 코드 집합, 공개 의미, 각 코드를 실을 수 있는 공개 경로입니다. |
| 오류 우선순위 | 응답 분기에 공개 오류가 여러 개 있을 때 `errors[0]`을 고르는 방식입니다. |
| 오류와 차단 사유 경로 | 조건이 `ToolRejectedResponse.errors[]`, 메서드별 차단 결과, dry-run 미리보기 데이터 중 어디에 속하는지입니다. |
| `STATE_VERSION_CONFLICT` | 공개 오래된 상태와 멱등성 충돌 동작입니다. 이 값은 공개 오류 코드이지 차단 사유 코드가 아닙니다. |
| 사용자 표시 라벨 | 공개 오류를 화면에 설명하는 지침입니다. 라벨은 공개 식별자를 대체하지 않습니다. |

## 이 문서가 담당하지 않는 것

| 여기서 담당하지 않는 것 | 담당 문서 |
|---|---|
| 메서드 요청 본문 스키마, 응답 필드 형태, 공통 요청/응답 래퍼 | [API 코어 스키마](schema-core.md), [MVP API](mvp-api.md), 분리된 API 스키마 담당 문서입니다. |
| Core의 게이트 의미, 사용자 판단 경계, 전체 닫기 준비 상태 평가 순서 | [Core 모델](../core-model.md)과 [MVP API](mvp-api.md)입니다. |
| `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, 값 집합 필드 정의 | [API 상태 스키마](schema-state.md), [API 코어 스키마](schema-core.md), [API 값 집합](schema-value-sets.md)입니다. |
| 저장소 행, 재실행 행, DDL, 잠금, 마이그레이션, 저장 효과 | [저장소 기록](../storage-records.md), [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md)입니다. |
| 보안 보장 표현과 접근 경계 주장 | [보안](../security.md)입니다. |

## 오류와 차단 사유

| 개념 | 공개 형태 | 세부사항 |
|---|---|---|
| 거부 응답 | `ToolRejectedResponse.errors[]` | [거부 응답](#error-vs-blocker-rejected-response) |
| 차단 결과 | 메서드별 결과 필드 | [차단 결과](#error-vs-blocker-blocked-result) |
| dry-run 미리보기 | `ToolDryRunResponse` | [dry-run 미리보기](#error-vs-blocker-dry-run-preview) |

<a id="error-vs-blocker-rejected-response"></a>
거부 응답:
- 공개 형태: `ToolRejectedResponse.errors[]`와 `ToolError.code: ErrorCode`.
- 의미: 메서드가 커밋되는 동작으로 진행하지 않았다는 뜻입니다.
- 조건: 공개 전송, 요청, 최신성, 로컬 접근, 역량, 선행조건 거부입니다.
- 상태 효과: 커밋된 동작이 없고 상태 변경도 없습니다.

<a id="error-vs-blocker-blocked-result"></a>
차단 결과:
- 공개 형태: `write_decision_reasons`나 `blockers` 같은 메서드별 결과 필드입니다.
- 의미: 메서드가 동작별 차단 결과를 반환했을 수 있다는 뜻입니다.
- 비주장: 공개 전송 또는 스키마 오류가 아닙니다.
- 상태 효과: 메서드 담당 문서가 허용한 커밋된 차단 결과나 읽기 전용 차단 사유 데이터만 가능합니다.

<a id="error-vs-blocker-dry-run-preview"></a>
dry-run 미리보기:
- 공개 형태: `DryRunSummary.would_errors[]` 또는 `DryRunSummary.would_blockers[]`를 담은 `ToolDryRunResponse`입니다.
- 의미: 유효한 dry-run 요청에서 미리 볼 수 있는 진단입니다.
- 상태 효과: 커밋된 쓰기가 아니며 저장된 차단 사유 상태도 아닙니다.

`ErrorCode` 값은 공개 API 식별자입니다. 차단 사유 코드는 동작별 결과 값입니다. 공개 `ErrorCode`는 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 차단 사유 코드로 재사용하면 안 됩니다.

<a id="error-taxonomy"></a>
## 공개 `ErrorCode` 표

| `ErrorCode` | 사용 위치 | 의미 | 상태 변경 | 차단 사유 코드 가능 여부 |
| --- | --- | --- | --- | --- |
| `VALIDATION_FAILED` | `ToolRejectedResponse.errors[]` | 요청 본문 형태, enum 값, 활성화 규칙, 프로필 검증, 아티팩트 입력 형태가 유효하지 않습니다. | 없음 | 요청 거부에서는 불가 |
| `STATE_VERSION_CONFLICT` | `ToolRejectedResponse.errors[]` | 오래된 `expected_state_version`, 오래된 `WriteAuthorization.basis_state_version`, 또는 멱등 요청 해시 충돌입니다. | 없음 | 불가 |
| `MCP_UNAVAILABLE` | `ToolRejectedResponse.errors[]` | 필요한 Core, MCP, 접점 도달 가능성을 사용할 수 없습니다. | 없음 | 요청 거부에서는 불가 |
| `LOCAL_ACCESS_MISMATCH` | `ToolRejectedResponse.errors[]` | 도달 가능한 로컬 접근이 등록된 전송 경로, 세션, 바인딩, 프로젝트, 접점 인스턴스와 맞지 않거나 접근이 철회되었습니다. | 없음 | 요청 거부에서는 불가 |
| `NO_ACTIVE_TASK` | `ToolRejectedResponse.errors[]` | Task가 필요하지만 활성 Task나 지정된 Task가 없습니다. | 없음 | 기본 불가 |
| `NO_ACTIVE_CHANGE_UNIT` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 쓰기 가능하거나 닫기와 관련된 동작에 활성 범위 지정 Change Unit이 없습니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `BASELINE_STALE` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 동작에 필요한 저장소 상태와 기준 상태가 더 이상 맞지 않습니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `SCOPE_REQUIRED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 요청한 쓰기나 동작 전에 범위 확인이 필요합니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `SCOPE_VIOLATION` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 의도했거나 관찰된 경로 또는 민감 범주가 활성 범위나 저장된 승인 범위를 넘었습니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `WRITE_AUTHORIZATION_REQUIRED` | `ToolRejectedResponse.errors[]` | 쓰기 가능한 Run에 필요한 Write Authorization이 없습니다. | 없음 | 기본 불가 |
| `WRITE_AUTHORIZATION_INVALID` | `ToolRejectedResponse.errors[]` | 제공된 Write Authorization이 만료, 철회, 소비, 또는 버전 외 사유로 비호환입니다. | 없음 | 기본 불가 |
| `APPROVAL_DENIED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 필요한 민감 동작 승인이 거부되었습니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `APPROVAL_EXPIRED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 필요한 민감 동작 승인이 만료되었거나 범위 또는 기준 상태와 달라졌습니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `APPROVAL_REQUIRED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 진행 전에 민감 동작 승인이 필요합니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `DECISION_UNRESOLVED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 관련 사용자 판단이 대기, 적용 범위 없는 보류, 거부, 차단, 오래됨, 대체됨, 비호환 상태입니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `AUTONOMY_BOUNDARY_EXCEEDED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 의도한 동작이 활성 Change Unit Autonomy Boundary를 넘었습니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `DECISION_REQUIRED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 진행 전에 차단 중인 사용자 소유 판단을 요청해야 합니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `CAPABILITY_INSUFFICIENT` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 접점은 인식되었지만 필요한 접근 등급, 관찰, 캡처, 보장 지원, 활성 동작이 없습니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용할 때만 |
| `EVIDENCE_INSUFFICIENT` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 필요한 증거 범위가 없거나, 부분적이거나, 오래되었거나, 막혔습니다. | 거부 밖에서는 담당 문서가 정함 | 닫기 준비 상태 담당 문서가 허용할 수 있음 |
| `RESIDUAL_RISK_NOT_VISIBLE` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 닫기에 영향을 주는 알려진 잔여 위험이 최종 수락이나 닫기 전에 보이지 않았습니다. | 거부 밖에서는 담당 문서가 정함 | 닫기 준비 상태 담당 문서가 허용할 수 있음 |
| `ACCEPTANCE_REQUIRED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 필요한 최종 수락이 대기 중이거나, 거부되었거나, 표시된 결과 근거와 호환되지 않습니다. | 거부 밖에서는 담당 문서가 정함 | 닫기 준비 상태 담당 문서가 허용할 수 있음 |
| `PROJECTION_STALE` | `ToolRejectedResponse.errors[]` | 요청한 읽기용 상태나 보기가 오래되었거나 실패했습니다. | 없음 | 그 자체로는 불가 |
| `ARTIFACT_MISSING` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 참조한 지속 아티팩트가 없거나, 사용할 수 없거나, 닫기 근거로 쓸 수 없거나, 무결성/메타데이터 확인에 실패했습니다. | 거부 밖에서는 담당 문서가 정함 | 닫기 준비 상태 담당 문서가 허용할 수 있음 |
| `VALIDATOR_FAILED` | `ToolRejectedResponse.errors[]`, 담당 문서가 정의한 결과 경로 | 필요한 활성 validator나 차단 사유 확인이 실패했고 더 구체적인 타입 코드가 없을 때 쓰는 대체 코드입니다. | 거부 밖에서는 담당 문서가 정함 | 담당 문서가 허용한 대체 코드 |

`ToolError.details.authorization_reason`은 `missing`, `expired`, `stale`, `revoked`, `consumed`, `incompatible`만 사용합니다. 오래된 `WriteAuthorization.basis_state_version`은 `WRITE_AUTHORIZATION_INVALID`가 아니라 `STATE_VERSION_CONFLICT`를 사용합니다.

`ToolError.details.artifact_input_error.reason`은 아래 세부 보조 값을 사용합니다. 이 값들은 최상위 공개 `ErrorCode` 값이 아닙니다. 스테이징된 아티팩트 핸들 검증 실패는 실제 실패가 요청 수준 로컬 접근이나 역량 확인이 아닌 한 공개 코드 `VALIDATION_FAILED`를 유지합니다.

| `artifact_input_error.reason` | 의미 |
|---|---|
| `staged_handle_expired` | 스테이징된 아티팩트 핸들의 사용 가능 시간이 지났습니다. |
| `staged_handle_consumed` | 스테이징된 아티팩트 핸들이 이미 소비되었습니다. |
| `staged_handle_project_mismatch` | 스테이징된 아티팩트 핸들이 다른 프로젝트에 속합니다. |
| `staged_handle_task_mismatch` | 스테이징된 아티팩트 핸들이 다른 Task에 속합니다. |
| `staged_handle_surface_mismatch` | 스테이징된 아티팩트 핸들의 출처가 확인된 접점과 맞지 않습니다. |
| `staged_handle_checksum_mismatch` | 스테이징된 바이트가 예상 체크섬과 맞지 않습니다. |
| `staged_handle_size_mismatch` | 스테이징된 바이트가 예상 크기와 맞지 않습니다. |
| `staged_handle_not_found` | 스테이징된 아티팩트 핸들을 찾을 수 없습니다. |

<a id="primary-error-code-precedence"></a>

## 오류 우선순위

오류를 담는 분기의 `errors`가 비어 있지 않으면 메서드 담당 문서가 더 좁은 메서드별 순서를 정의하지 않는 한 아래 순서로 `errors[0]` 공개 주 오류를 고릅니다.

| 우선순위 | 주 `ErrorCode` | 적용 대상 |
|---:|---|---|
| 1 | `VALIDATION_FAILED` | 거부된 요청 형태나 검증 실패입니다. |
| 2 | `STATE_VERSION_CONFLICT` | 거부 응답에만 적용됩니다. 커밋된 차단 결과의 주 오류가 될 수 없습니다. |
| 3 | `MCP_UNAVAILABLE` | Core, MCP, 접점 도달 가능성 실패로 거부되었습니다. |
| 4 | `LOCAL_ACCESS_MISMATCH` | 로컬 접근 바인딩 불일치나 철회로 거부되었습니다. |
| 5 | `NO_ACTIVE_TASK` | Task 식별자가 없어 거부되었습니다. |
| 6 | `NO_ACTIVE_CHANGE_UNIT` | 활성 Change Unit이 없습니다. |
| 7 | `BASELINE_STALE` | 기준 상태가 오래되었습니다. |
| 8 | `SCOPE_REQUIRED` | 필요한 범위 확인이 없습니다. |
| 9 | `SCOPE_VIOLATION` | 범위 또는 승인된 시도 범위를 위반했습니다. |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` | 필요한 Write Authorization이 없습니다. |
| 11 | `WRITE_AUTHORIZATION_INVALID` | 버전 외 사유로 Write Authorization을 사용할 수 없습니다. |
| 12 | `APPROVAL_DENIED` | 민감 동작 승인이 거부되었습니다. |
| 13 | `APPROVAL_EXPIRED` | 민감 동작 승인이 만료되었거나 달라졌습니다. |
| 14 | `APPROVAL_REQUIRED` | 민감 동작 승인이 없습니다. |
| 15 | `DECISION_UNRESOLVED` | 기존 사용자 판단을 사용할 수 없습니다. |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` | 자율성 경계를 넘었습니다. |
| 17 | `DECISION_REQUIRED` | 새 사용자 소유 판단이 필요합니다. |
| 18 | `CAPABILITY_INSUFFICIENT` | 접점 역량이 부족합니다. |
| 19 | `EVIDENCE_INSUFFICIENT` | 증거 범위가 충분하지 않습니다. |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` | 닫기 관련 위험이 보이지 않습니다. |
| 21 | `ACCEPTANCE_REQUIRED` | 최종 수락이 필요하거나 호환되지 않습니다. |
| 22 | `PROJECTION_STALE` | 읽기용 보기가 오래되었거나 실패했습니다. |
| 23 | `ARTIFACT_MISSING` | 지속 아티팩트가 없거나, 사용할 수 없거나, 실패했습니다. |
| 24 | `VALIDATOR_FAILED` | 더 구체적인 활성 코드가 없을 때 쓰는 타입 있는 대체 코드입니다. |

`STATE_VERSION_CONFLICT`는 이 표에서 `ToolRejectedResponse.errors[]`에만 나타납니다. `MethodResult.base.errors[0]`, `CloseTaskResult(close_state=blocked).errors[0]`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`로 선택하면 안 됩니다.

<a id="blocked-and-dry-run-behavior"></a>

## 거부 응답 동작

| 조건 | 응답 경로 | 필요한 결과 |
|---|---|---|
| 메서드가 진행되기 전에 요청 형태, 스키마, 프로필, 스테이징된 아티팩트 핸들 검증이 실패합니다. | `ToolRejectedResponse.errors[]` | 커밋된 동작이 없습니다. 메서드별 결과 전용 필드를 넣지 않습니다. |
| 커밋 전에 Core, MCP, 로컬 접근, 접점 역량, 상태 조회, Task 식별자, 필요한 선행조건이 실패합니다. | `ToolRejectedResponse.errors[]` | 기록, 재실행 행, 아티팩트, 이벤트, Write Authorization 소비, 닫기 상태 변경, 상태 버전 증가가 없습니다. |
| `expected_state_version`, `WriteAuthorization.basis_state_version`, 멱등 요청 해시가 오래되었거나 충돌합니다. | `STATE_VERSION_CONFLICT`를 담은 `ToolRejectedResponse.errors[]` | 커밋된 동작이 없습니다. 이 충돌은 차단 사유가 아닙니다. |
| `dry_run=true` 요청이 읽기 결과나 dry-run 미리보기를 만들기 전에 실패합니다. | `dry_run=true`인 `ToolRejectedResponse` | 이 거부는 `DryRunSummary.would_errors[]`도 아니고 `PlannedBlocker`도 아닙니다. |

거부 응답은 메서드가 커밋되는 동작으로 진행하지 않았다는 뜻입니다. 거부 응답은 차단 결과가 아니며, 요청에 없던 권한, 증거, 수락, 닫기 상태를 만들지 않습니다.

## 차단 결과 동작

| 차단 경로 | 결과 데이터 | 오류 코드 규칙 |
|---|---|---|
| `decision=blocked`, `decision=approval_required`, `decision=decision_required`인 `PrepareWriteResult` | `write_decision_reasons: WriteDecisionReason[]` | 메서드 담당 판단 사유를 사용합니다. `CloseReadinessBlocker`를 반환하지 않습니다. |
| 유효한 닫기 준비 상태 평가 뒤의 `CloseTaskResult(close_state=blocked)` | `blockers: CloseReadinessBlocker[]` | 닫기 차단 사유 매핑을 사용합니다. `STATE_VERSION_CONFLICT`를 쓰면 안 됩니다. |
| `StatusResult.close_blockers`와 `harness.close_task intent=check` | 읽기 전용 `CloseReadinessBlocker` 관찰 데이터 | 읽기 때문에 저장된 차단 사유나 상태 버전 증가가 생기지 않습니다. |

차단 결과는 메서드가 동작별 차단 결과를 반환했을 수 있다는 뜻입니다. 공개 전송 또는 스키마 오류가 아닙니다. 커밋된 차단 결과와 상태 효과는 [MVP API](mvp-api.md)와 [저장 효과](../storage-effects.md)가 허용해야 합니다.

## Dry-run 동작

| 요청 | 응답 | 규칙 |
|---|---|---|
| `dry_run=true`인 유효한 읽기 전용 호출 | `base.dry_run=true`, `base.effect_kind=read_only`인 메서드별 결과 | `dry_run=true`는 `ToolDryRunResponse`의 동의어가 아닙니다. |
| `dry_run=true`인 유효한 상태 효과 또는 저장소 소유 스테이징 동작 | `DryRunSummary`를 담은 `ToolDryRunResponse` | Dry-run 미리보기는 커밋된 쓰기가 아닙니다. |
| 예상 차단 사유가 있는 유효한 dry-run 미리보기 | `DryRunSummary.would_blockers: PlannedBlocker[]` | 미리보기 차단 사유는 저장된 `CloseReadinessBlocker` 객체가 아닙니다. |
| `dry_run=true`의 커밋 전 실패 | `ToolRejectedResponse` | 실패는 미리보기가 아니라 거부입니다. |

`PlannedBlocker.code`는 `STATE_VERSION_CONFLICT`가 될 수 없습니다. 오래된 상태는 미리보기 전에 거부됩니다.

<a id="idempotency"></a>
<a id="state-conflict-behavior"></a>
## 상태 버전 충돌

| 충돌 조건 | 공개 코드 | 응답 경로 | 차단 사유 사용 |
|---|---|---|---|
| `ToolEnvelope.expected_state_version`이 `project_state.state_version`보다 오래되었습니다. | `STATE_VERSION_CONFLICT` | `ToolRejectedResponse.errors[]` | 금지 |
| 소비 전 `WriteAuthorization.basis_state_version`이 오래되었습니다. | `STATE_VERSION_CONFLICT` | `ToolRejectedResponse.errors[]` | 금지 |
| 같은 `idempotency_key`가 다른 요청 해시와 함께 재사용되었습니다. | `STATE_VERSION_CONFLICT` | `ToolRejectedResponse.errors[]` | 금지 |

`STATE_VERSION_CONFLICT`의 현재 MVP 의미는 하나뿐입니다. 프로젝트 전체의 커밋 전 최신성 또는 멱등성 충돌입니다. 메서드별 결과도 아니고, dry-run 미리보기 데이터도 아니며, `MethodResult.decision` 값, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`도 아닙니다.

| 세부 경우 | 필요한 세부정보 지침 |
|---|---|
| 오래된 `expected_state_version` | 가능하면 `state_clock: project_state.state_version`, `current_state_version`, `expected_state_version`, `project_id`, `task_id`를 포함합니다. |
| 멱등 요청 해시 충돌 | 민감한 요청 본문을 노출하지 않고 `idempotency_key`와 요청 해시 불일치를 식별합니다. |
| 오래된 Write Authorization 근거 버전 | 오래된 권한 근거와 현재 `project_state.state_version`을 식별하고, 권한을 소비하지 않습니다. |

## 금지된 blocker-code 규칙

| 금지된 사용 | 대신 사용할 것 |
|---|---|
| `STATE_VERSION_CONFLICT`를 `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`, `MethodResult.decision`, 또는 커밋된 차단 결과의 주 오류로 사용합니다. | `effect_kind=no_effect`인 `ToolRejectedResponse.errors[]`를 사용합니다. |
| 커밋 전 공개 오류를 차단 사유 배열로 복사합니다. | `ToolRejectedResponse.errors[]`를 반환합니다. |
| 담당 문서의 명시적 허용 없이 공개 `ErrorCode`를 차단 사유 코드로 재사용합니다. | 메서드/스키마 담당 문서의 차단 사유 코드나 결과 사유를 사용합니다. |
| 사용자 표시 라벨을 API 식별자로 사용합니다. | 공개 `ErrorCode`는 그대로 두고 표시 문구만 지역화합니다. |
| dry-run 미리보기의 오래된 상태 충돌을 `DryRunSummary.would_errors[]`나 `DryRunSummary.would_blockers[]`로 표현합니다. | `STATE_VERSION_CONFLICT`로 요청을 거부합니다. |

<a id="harnessclose_task-close-blockers"></a>

## `close_task` 차단 사유 매핑

| `close_task` 상황 | 세부사항 |
|---|---|
| 닫기 준비 상태 평가 전 사전 확인 실패 | [사전 확인 실패](#close-task-preflight-failure) |
| 유효한 읽기인 `intent=check` | [`intent=check`](#close-task-intent-check) |
| 닫기 차단 사유를 찾은 `intent=complete` | [차단된 `intent=complete`](#close-task-intent-complete-blocked) |
| 닫기 차단 사유가 없는 `intent=complete` | [닫힌 `intent=complete`](#close-task-intent-complete-closed) |
| 유효하지 않은 `intent=cancel` 또는 `intent=supersede` 종료 전이 | [유효하지 않은 종료 전이](#close-task-invalid-terminal-transition) |

<a id="close-task-preflight-failure"></a>
사전 확인 실패:
- 조건: 닫기 준비 상태 평가 전에 오래된 상태, 오래된 Write Authorization 근거, 멱등성 충돌, 검증 실패, 로컬 접근 실패, 역량 실패, Core 상태 읽기 실패, Project/Task 식별 실패가 발생합니다.
- 응답 경로: `ToolRejectedResponse.errors[]`.
- 공개 코드 규칙: `STATE_VERSION_CONFLICT`와 다른 커밋 전 오류는 거부 응답에 남습니다.
- 허용되지 않는 것: `CloseReadinessBlocker` 항목을 반환하지 않습니다.

<a id="close-task-intent-check"></a>
`intent=check`:
- 조건: 요청이 유효한 읽기입니다.
- 응답 경로: 읽기 전용 `CloseTaskResult`.
- 허용되는 것: `CloseReadinessBlocker` 관찰 데이터를 반환할 수 있습니다.
- 허용되지 않는 것: 저장된 차단 사유와 상태 버전 증가가 없습니다.

<a id="close-task-intent-complete-blocked"></a>
차단된 `intent=complete`:
- 조건: 유효한 평가에서 닫기 차단 사유를 찾습니다.
- 응답 경로: `CloseTaskResult(close_state=blocked)`.
- 허용되는 것: `CloseReadinessBlocker[]`를 반환할 수 있습니다.
- 허용되지 않는 것: `STATE_VERSION_CONFLICT`는 금지됩니다.

<a id="close-task-intent-complete-closed"></a>
닫힌 `intent=complete`:
- 조건: 담당 문서가 정의한 닫기 차단 사유가 더 없습니다.
- 응답 경로: `CloseTaskResult(close_state=closed)`.
- 공개 코드 규칙: 닫기 차단 사유가 없습니다.

<a id="close-task-invalid-terminal-transition"></a>
유효하지 않은 종료 전이:
- 조건: `intent=cancel` 또는 `intent=supersede`의 종료 전이가 유효하지 않습니다.
- 응답 경로: 메서드 담당 결과 또는 거부 경로.
- 공개 코드 규칙: 차단 사유는 전이 유효성으로 제한합니다.
- 허용되지 않는 것: 취소나 대체에 증거 충분성, 최종 수락, 잔여 위험 수락을 요구하지 않습니다.

| 닫기 준비 상태 발견 사항 | 공개 코드 매핑 |
|---|---|
| 증거 공백 | `EVIDENCE_INSUFFICIENT` |
| 닫기에 영향을 주는 지속 아티팩트가 없거나, 사용할 수 없거나, 닫기 근거로 쓸 수 없거나, 실패했습니다. | `ARTIFACT_MISSING` |
| 필요한 최종 수락이 없거나 호환되지 않습니다. | `ACCEPTANCE_REQUIRED` |
| 닫기에 영향을 주는 알려진 잔여 위험이 보이지 않습니다. | `RESIDUAL_RISK_NOT_VISIBLE` |
| 잔여 위험은 보였지만 수락되지 않았습니다. | `category=residual_risk_acceptance`와 함께 `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED` |
| 사용자 소유 판단이 해결되지 않았습니다. | `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED` |
| 민감 동작 승인이 없거나, 거부되었거나, 만료되었거나, 달라졌습니다. | `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` |
| 유효한 평가 뒤 범위, 자율성 경계, 기준 상태 차단 사유가 있습니다. | 담당 문서가 허용할 때 `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` |
| 읽기용 보기 최신성 문제입니다. | `PROJECTION_STALE`; 그 자체로 닫기 차단 사유가 아닙니다. |
| 프로젝트 전체 상태나 Write Authorization 근거 버전이 오래되었습니다. | `ToolRejectedResponse.errors[]`의 `STATE_VERSION_CONFLICT`; 절대 닫기 차단 사유가 아닙니다. |

전체 닫기 준비 상태 평가 순서는 [Core 모델의 닫기 준비 상태](../core-model.md#close_task)가 담당합니다. 메서드 동작은 [`harness.close_task`](mvp-api.md#harnessclose_task)가 담당합니다. `CloseReadinessBlocker` 형태와 범주는 [API 상태 스키마](schema-state.md)와 [API 값 집합](schema-value-sets.md)이 담당합니다.

## 사용자 표시 라벨

사용자 표시 라벨은 공개 오류 식별자와 다를 수 있습니다. 라벨은 표시 문구일 뿐 새 공개 코드가 아닙니다.

| 공개 조건 | 권장 라벨 | 차단 해소에 필요한 최소 조치 |
|---|---|---|
| `VALIDATION_FAILED` | 잘못된 요청 | 다시 시도하기 전에 요청 본문, enum 값, 활성화 규칙, 프로필 값, 필드 집합을 고칩니다. |
| `STATE_VERSION_CONFLICT` | 상태 버전 충돌 | 현재 상태를 새로 고치고 현재 `project_state.state_version`으로 다시 시도하거나 원래 멱등 요청을 재실행합니다. |
| `MCP_UNAVAILABLE` | Core 또는 접점 사용 불가 | Core, MCP, 접점 도달 가능성을 다시 연결하거나 진단합니다. |
| `LOCAL_ACCESS_MISMATCH` | 로컬 접근 불일치 | 등록된 로컬 전송 경로/세션/바인딩을 사용하거나 로컬 접근 등록을 고칩니다. |
| `CAPABILITY_INSUFFICIENT` | 접점 역량 부족 | 역량이 있는 접점을 사용하거나, 동작을 줄이거나, 빠진 역량이 필요 없는 경로를 선택합니다. |
| `NO_ACTIVE_TASK` | 활성 Task 없음 | Task 범위 동작 전에 Task를 선택하거나 생성합니다. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | 범위, 경계, 기준 상태 문제 | 범위를 확인하거나 좁히고, 유효한 범위 또는 기준 상태 변경을 담당 경로로 갱신하거나, 필요한 사용자 판단을 요청합니다. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | 쓰기 전 확인 없음 또는 사용할 수 없음 | 정확한 동작, 현재 범위, 현재 상태로 `harness.prepare_write`를 호출하거나 다시 시도합니다. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | 판단 필요 | 집중된 `UserJudgment`를 요청하거나 해결합니다. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | 민감 동작 승인 필요 또는 사용 불가 | `judgment_kind=sensitive_approval`을 요청, 해결, 갱신합니다. |
| `EVIDENCE_INSUFFICIENT` | 증거 필요 | 누락된 증거를 기록, 재실행하거나 증거 공백과 최소 차단 해소 조치를 보여 줍니다. |
| `ACCEPTANCE_REQUIRED` | 최종 수락 필요 | 표시된 결과 근거에 대해 `judgment_kind=final_acceptance`를 요청하거나 해결합니다. |
| `RESIDUAL_RISK_NOT_VISIBLE` | 잔여 위험이 보이지 않음 | 최종 수락이나 닫기 전에 닫기 관련 잔여 위험을 보여 줍니다. |
| `PROJECTION_STALE` | 읽기용 보기 오래됨 | 그 보기에 의존하기 전에 새로 고칩니다. |
| `ARTIFACT_MISSING` | 아티팩트 문제 | 없거나 사용할 수 없는 아티팩트를 복구, 재생성, 교체, 다시 연결합니다. |
| `VALIDATOR_FAILED` | 확인 실패 | 가능하면 특정 validator나 차단 사유를 보여 줍니다. 타입 있는 코드가 없을 때만 이 대체 코드를 사용합니다. |

<a id="documentation-smoke-error-coverage"></a>

## 담당 문서 링크

| 질문 | 담당 문서 |
|---|---|
| 공개 `ErrorCode` 값, 의미, 우선순위 | 이 문서입니다. |
| `ToolRejectedResponse`, `ToolDryRunResponse`, `ToolError`, `ToolResultBase`, `DryRunSummary`, 응답 분기 형태 | [API 코어 스키마](schema-core.md)입니다. |
| 메서드 동작, 분기 선택, 메서드별 요청 본문 | [MVP API](mvp-api.md)입니다. |
| `WriteDecisionReason`, `CloseReadinessBlocker`, 상태 요약, 닫기 준비 상태 데이터 형태 | [API 상태 스키마](schema-state.md)입니다. |
| `response_kind`, `effect_kind`, `PlannedBlocker.source_kind`, 차단 사유 범주, enum 형태 API 값 | [API 값 집합](schema-value-sets.md)입니다. |
| `ArtifactInput`, `ArtifactRef`, `StagedArtifactHandle`, 아티팩트 입력 형태 | [API 아티팩트 스키마](schema-artifacts.md)입니다. |
| 스테이징된 아티팩트 핸들 저장소 검증과 아티팩트 승격 생명주기 | [아티팩트 저장소](../storage-artifacts.md)입니다. |
| 사용자 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락 형태 | [API 판단 스키마](schema-judgment.md)와 [Core 모델](../core-model.md)입니다. |
| 전체 닫기 준비 상태 평가 순서와 비대체 규칙 | [Core 모델의 닫기 준비 상태](../core-model.md#close_task)입니다. |
| 저장 효과, 재실행 행, 상태 시계, DDL | [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md), [저장소 기록](../storage-records.md)입니다. |
| 보안 보장 표현과 접근 경계 주장 | [보안](../security.md)입니다. |
