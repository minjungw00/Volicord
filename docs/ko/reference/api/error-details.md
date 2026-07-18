# API 오류 세부사항

이 문서는 Volicord API 오류의 기계 판독용 `ToolError.details` 의미, 세부 필드, 보조 값, 세부사항 제약을 담당합니다.

`ToolError.details` 아래의 진단 키와 보조 값을 확인할 때 이 문서를 사용합니다. 분기 경로, 공개 오류 코드 의미, 스키마 형태, 표시 문구는 이웃 담당 문서를 사용합니다.

## 담당 경계

이 문서가 담당합니다.

- 알려진 `ToolError.details` 필드와 중첩 세부 키의 의미.
- `ToolError.details` 아래에서 쓰는 보조 값.
- 기계 판독용 세부사항을 표시 문구와 민감한 요청 본문에서 분리하는 제약.

이웃 담당 문서:

- `ToolError` 형태: [API 코어 스키마](schema-core.md#shared-support-shapes).
- 공개 `ErrorCode` 값과 의미: [API 오류 코드](error-codes.md).
- 주 코드 우선순위와 충돌 선택: [API 오류 우선순위](error-precedence.md).
- API 응답 분기 경로: [API 오류 처리 경로](error-routing.md).
- 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md).
- 표시 문구만: [템플릿 본문](../template-bodies.md).
- 저장 효과: [저장 효과](../storage-effects.md).
- 제품 전체 실패 범주 의미: [실패 모델](../failure-model.md).

<a id="machine-readable-error-details"></a>

## 기계 판독용 세부사항 제약

`ToolError.details`는 기계 판독용 진단 데이터입니다. 표시 문구가 아니며 공개 `ToolError.code`를 대체하지 않습니다.

세부 키와 보조 값은 정확한 식별자입니다.

조건:
- 세부 키와 보조 값은 담당 메서드나 스키마가 그 정확한 사용을 명시적으로 허용할 때만 차단 사유 코드로 재사용할 수 있습니다.

필수 동작:
- 세부 키와 보조 값을 기계 판독용 식별자로 보존해야 합니다.

허용되지 않는 것:
- 세부 키와 보조 값을 지역화하면 안 됩니다.
- 사용자용 표시 문구로 렌더링하면 안 됩니다.
- 담당 메서드나 스키마의 지원 없이 차단 사유 코드로 재사용하면 안 됩니다.

세부 데이터는 안정적인 진단 사실로 제한해야 합니다. 민감한 요청 본문을 노출하거나, 메서드 요청 본문을 중복하거나, 원본 저장 JSON, 비밀값, SQL 텍스트, 민감한 절대 경로를 노출하거나, 저장 효과를 정의하면 안 됩니다.

<a id="state-conflict-detail-fields"></a>

## 상태 충돌 세부 필드

오래된 `expected_state_version` 세부사항:
- 가능하면 `state_clock: project_state.state_version`, `current_state_version`, `expected_state_version`, `project_id`, `task_id`를 포함합니다.

`WriteTicket.basis_state_version`은 상태 충돌 세부 필드가 아닙니다. 감사 순서
메타데이터이며 그 불일치만으로 오류를 만들지 않습니다.

멱등 요청 해시 충돌 세부사항:
- 민감한 요청 본문을 노출하지 않고 `idempotency_key`와 요청 해시 불일치를 식별합니다.

<a id="owner-state-corruption-detail-fields"></a>

## 담당 상태 손상 세부 필드

타입이 지정된 담당 상태 손상을 `code=PERSISTED_DATA_CORRUPT`,
`category=corrupt`로 보고할 때 세부사항은 아래 항목을 식별할 수 있습니다.

- `owner_state_error.table`
- `owner_state_error.record_ref`
- `owner_state_error.logical_column`
- `owner_state_error.corruption_category`

이 진단은 원본 저장 JSON, 비밀값, SQL 텍스트, 민감한 절대 경로를 포함하면 안 됩니다. 형식이 잘못된 JSON을 부재와 동등하게 만들지 않습니다.

<a id="error-detail-helper-values"></a>

## 오류 세부사항 보조 값

<a id="reason"></a>

### `reason`

`ToolError.details.reason`은 정확한 도메인별 식별자입니다. 아래 코드와 도메인 조합은
표시된 값을 사용해야 합니다.

| 공개 코드와 도메인 | `ToolError.category` | `details.reason` | 의미 담당 문서 |
|---|---|---|---|
| `volicord.prepare_write`의 `NO_ACTIVE_CHANGE_UNIT` | `rejected` | `current_change_unit_required` | [`volicord.prepare_write`](method-prepare-write.md) |
| 알 수 없는 외부 설명자 또는 경계 계약의 `UNSUPPORTED_CONTRACT` | `unsupported_contract` | `unsupported_external_contract` | [외부 계약](../external-contracts.md) |

사유는 도메인 원인을 좁힐 뿐 필수 실패 범주나 공개 코드를 대신하거나 바꾸지 않습니다.
이 값들은 표시 텍스트, fallback 선택자, alias, 다른 계약을 decode할 권한이 아닙니다.
다른 세부사항 계열은 이 필드에 의미를 겹쳐 넣지 않고 `write_ticket_reason`,
`artifact_input_error.reason`처럼 이름 붙은 중첩 필드를 사용합니다.

<a id="authorization-reason"></a>

### `write_ticket_reason`

`ToolError.details.write_ticket_reason`은 아래 값을 사용합니다.

<a id="write-ticket-reason"></a>

```text
missing
revoked
consumed
incompatible
task_mismatch
change_unit_mismatch
scope_revision_changed
change_unit_changed
baseline_changed
workspace_changed
approval_basis_changed
idle_timeout
task_closed
explicit_revoke
product_write_flag_mismatch
baseline_mismatch
operation_mismatch
workspace_mismatch
approval_basis_mismatch
policy_authority_mismatch
sensitive_category_mismatch
path_mismatch
```

`*_changed`, `idle_timeout`, `task_closed`, `explicit_revoke`는 안정되게 기록되는
무효화 사유입니다. `*_mismatch`, `consumed`, `revoked`, `incompatible`는 시도 시점
비호환을 식별합니다. 공개 코드는 `WRITE_TICKET_INVALID`를 유지합니다. 전역
`basis_state_version` 불일치는 무효가 아니므로 보조 값이 없습니다.
`policy_authority_mismatch`는 활성 티켓 결속이 없거나 현재 정규화된 프로젝트 쓰기
권한 fingerprint와 일치하지 않는다는 뜻입니다.

<a id="artifact-input-error-reason"></a>

### `artifact_input_error.reason`

`ToolError.details.artifact_input_error.reason`은 아래 세부 보조 값을 사용합니다. 이
값들은 최상위 공개 `ErrorCode` 값이 아닙니다. 스테이징된 아티팩트 핸들 검증 실패는
실제 실패가 요청 수준 호출 맥락, `actor_source`, 또는 Product Repository 경로 경계
불일치가 아닌 한 공개 코드 `VALIDATION_FAILED`를 유지합니다.

| `artifact_input_error.reason` | 의미 |
|---|---|
| `staged_handle_expired` | 스테이징된 아티팩트 핸들의 사용 가능 시간이 지났습니다. |
| `staged_handle_consumed` | 스테이징된 아티팩트 핸들이 이미 소비되었습니다. |
| `staged_handle_project_mismatch` | 스테이징된 아티팩트 핸들이 다른 프로젝트에 속합니다. |
| `staged_handle_task_mismatch` | 스테이징된 아티팩트 핸들이 다른 `Task`에 속합니다. |
| `staged_handle_actor_source_mismatch` | 스테이징된 아티팩트 핸들의 출처가 확인된 actor source와 맞지 않습니다. |
| `staged_handle_checksum_mismatch` | 스테이징된 바이트가 예상 체크섬과 맞지 않습니다. |
| `staged_handle_size_mismatch` | 스테이징된 바이트가 예상 크기와 맞지 않습니다. |
| `staged_handle_not_found` | 스테이징된 아티팩트 핸들을 찾을 수 없습니다. |
