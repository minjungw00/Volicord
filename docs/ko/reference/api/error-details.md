# API 오류 세부사항

이 문서는 하네스 API 오류의 기계 판독용 `ToolError.details` 의미, 세부 필드, 보조 값, 세부사항 제약을 담당합니다.

`ToolError` 객체 형태, 공개 `ErrorCode` 의미, 우선순위 선택, 닫기 준비 상태 차단 사유 처리 경로, 표시 라벨, 저장 효과는 정의하지 않습니다.

## 담당 경계

이 문서가 담당합니다.

- 알려진 `ToolError.details` 필드와 중첩 세부 키의 의미.
- `ToolError.details` 아래에서 쓰는 보조 값.
- 기계 판독용 세부사항을 표시 라벨과 민감한 요청 본문에서 분리하는 제약.

이 문서는 담당하지 않습니다.

- `ToolError` 형태: [API 코어 스키마](schema-core.md#shared-support-shapes).
- 공개 `ErrorCode` 값과 의미: [API 오류 코드](error-codes.md).
- 주 코드 우선순위와 충돌 선택: [API 오류 우선순위](error-precedence.md).
- API 응답 분기 경로: [API 오류 처리 경로](error-routing.md).
- 닫기 준비 상태 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md).
- 표시 문구로만 쓰는 렌더링 라벨과 메시지 문구: [템플릿 본문](../template-bodies.md).

<a id="machine-readable-error-details"></a>

## 기계 판독용 세부사항 제약

`ToolError.details`는 기계 판독용 진단 데이터입니다. 표시 문구가 아니며 공개 `ToolError.code`를 대체하지 않습니다.

세부 키와 보조 값은 정확한 식별자입니다. 지역화하거나 사용자 표시 라벨로 렌더링하거나, 담당 메서드나 스키마가 명시적으로 허용하지 않는 한 차단 사유 코드로 재사용하면 안 됩니다.

세부 데이터는 안정적인 진단 사실로 제한해야 합니다. 민감한 요청 본문을 노출하거나, 메서드 요청 본문을 중복하거나, 저장 효과를 정의하면 안 됩니다.

<a id="state-conflict-detail-fields"></a>

## 상태 충돌 세부 필드

오래된 `expected_state_version` 세부사항:
- 가능하면 `state_clock: project_state.state_version`, `current_state_version`, `expected_state_version`, `project_id`, `task_id`를 포함합니다.

오래된 Write Authorization 근거 버전 세부사항:
- 오래된 권한 부여 근거와 현재 `project_state.state_version`을 식별합니다.

멱등 요청 해시 충돌 세부사항:
- 민감한 요청 본문을 노출하지 않고 `idempotency_key`와 요청 해시 불일치를 식별합니다.

<a id="error-detail-helper-values"></a>

## 오류 세부사항 보조 값

<a id="authorization-reason"></a>

### `authorization_reason`

`ToolError.details.authorization_reason`은 `missing`, `expired`, `stale`, `revoked`, `consumed`, `incompatible`만 사용합니다. 오래된 `WriteAuthorization.basis_state_version`은 `WRITE_AUTHORIZATION_INVALID`가 아니라 `STATE_VERSION_CONFLICT`를 사용합니다.

<a id="artifact-input-error-reason"></a>

### `artifact_input_error.reason`

`ToolError.details.artifact_input_error.reason`은 아래 세부 보조 값을 사용합니다. 이 값들은 최상위 공개 `ErrorCode` 값이 아닙니다. 스테이징된 아티팩트 핸들 검증 실패는 실제 실패가 요청 수준 로컬 접근이나 역량 확인이 아닌 한 공개 코드 `VALIDATION_FAILED`를 유지합니다.

| `artifact_input_error.reason` | 의미 |
|---|---|
| `staged_handle_expired` | 스테이징된 아티팩트 핸들의 사용 가능 시간이 지났습니다. |
| `staged_handle_consumed` | 스테이징된 아티팩트 핸들이 이미 소비되었습니다. |
| `staged_handle_project_mismatch` | 스테이징된 아티팩트 핸들이 다른 프로젝트에 속합니다. |
| `staged_handle_task_mismatch` | 스테이징된 아티팩트 핸들이 다른 `Task`에 속합니다. |
| `staged_handle_surface_mismatch` | 스테이징된 아티팩트 핸들의 출처가 확인된 접점과 맞지 않습니다. |
| `staged_handle_checksum_mismatch` | 스테이징된 바이트가 예상 체크섬과 맞지 않습니다. |
| `staged_handle_size_mismatch` | 스테이징된 바이트가 예상 크기와 맞지 않습니다. |
| `staged_handle_not_found` | 스테이징된 아티팩트 핸들을 찾을 수 없습니다. |
