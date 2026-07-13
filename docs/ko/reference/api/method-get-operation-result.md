<a id="volicordget_operation_result"></a>

# `volicord.get_operation_result` 참조

## 담당하는 것

이 문서는 `volicord.get_operation_result`의 기준 동작을 담당합니다.

- 과거 Core 변경 응답 원문의 읽기 전용 조회
- 고정 크기 page와 불투명 cursor 동작
- 메서드별 접근, 결과 없음, 무효과 규칙
- 최상위 요청과 결과 필드

공유 `OperationResultRef`와 page 좌표 필드 형태는
[API 코어 스키마](schema-core.md#operation-result-retrieval)가 담당합니다.
정확한 replay 행 저장은
[저장 버전 관리](../storage-versioning.md#exact-operation-result-retrieval)가 담당합니다.

## 목적

MCP 변경 상태 보기는 변경이 커밋된 뒤 응답 예산을 지키기 위해 정확한 결과를
생략할 수 있습니다. 따라서 조회할 수 있는 모든 agent-workflow Core 커밋과 정확한
replay는 idempotent replay를 위해 이미 저장한 변경 불가능한 응답을 가리키는
`operation_result_ref`를 노출합니다.

`volicord.get_operation_result`는 그 과거 JSON text를 크기가 제한된 page로
읽습니다. Cursor 순서대로 `chunk_utf8` 값을 이어 붙이면 저장된 응답 JSON과 byte
단위로 정확히 같아야 합니다. 조회는 변경을 replay하거나 응답을 다시 계산하지 않으며,
과거 상태가 현재 상태라고 주장하지 않습니다.

## 요청

```yaml
GetOperationResultRequest:
  envelope: ToolEnvelope
  operation_result_ref: OperationResultRef
  cursor: string | null
```

규칙:

- `envelope.project_id`는 `operation_result_ref.project_id`와 같아야 합니다.
- 확인된 호출은 `operation_category=read`, `dry_run=false`,
  `idempotency_key=null`, `expected_state_version=null`을 사용합니다.
- `cursor=null`은 첫 page를 선택합니다. null이 아닌 cursor는 바로 앞 page가 반환한
  불투명 값이며 해석하지 않고 그대로 다시 보내야 합니다.
- Cursor는 전체 ref, 응답 checksum, 다음 byte offset에 묶입니다. 잘못된 형식,
  변조, 다른 결과의 cursor는 응답 조각을 반환하지 않고 거절합니다.

## 접근 요구사항

메서드는 page마다 현재 활성 Agent Connection과 Connection Projects membership을
다시 확인합니다. 선택 프로젝트와 현재 확인된 `actor_source`는 저장된
agent-workflow 호출과 일치해야 합니다. Ref나 cursor를 가지고 있다는 사실만으로는
접근 권한이 생기지 않습니다.

정확한 `volicord.resolve_user_action` 본문, 사용자의 자유 형식 note, Evidence 관찰
summary를 포함한 user-only 결과는 Agent Connection에 노출하지 않습니다. Host가
중개한 사용자 행동 흐름은 최초 agent 소유 `volicord.request_user_action` ref를 유지합니다. 따라서
정확한 결과 조회는 최초 대기 응답을 복원합니다. 별도 담당 MCP 결과 상태 보기는 선택
결과의 안전한 선택 식별자와 파생 ref를 보고할 수 있지만 user-only ref로 바꾸거나
사용자 note, Evidence 관찰 summary, 정확한 user-only 응답 본문을 노출하지 않습니다.

## 결과

```yaml
GetOperationResultResult:
  base: ToolResultBase
  operation_result_ref: OperationResultRef
  start_offset_bytes: integer
  end_offset_bytes: integer
  chunk_utf8: string
  next_cursor: string | null
  complete: boolean
  historical: true
  current_authority_refresh_required: true
```

성공 page는 `base.response_kind=result`, `base.effect_kind=read_only`를 사용하고
`chunk_utf8`에 원본 UTF-8 byte를 최대 16,384개 담습니다. Page 경계는 UTF-8
code point를 나누지 않습니다. 첫 page는 `start_offset_bytes=0`이며 반환한
`end_offset_bytes`는 다음 page의 시작점입니다. `next_cursor=null`이고 page가
`operation_result_ref.response_size_bytes`에 도달했을 때만 `complete=true`입니다.

메서드는 어떤 page도 반환하기 전에 저장 byte 길이와 SHA-256을 ref와 비교합니다.
결과 없음, 무결성 불일치, ref에 묶이지 않은 cursor, 사용할 수 없는 행은
`OPERATION_RESULT_UNAVAILABLE`을 반환합니다. Actor 또는 프로젝트 맥락 불일치는
`INVOCATION_CONTEXT_MISMATCH`, 잘못된 요청이나 cursor 문법은
`VALIDATION_FAILED`를 반환합니다. Store 접근 불가나 손상된 담당 상태는 일반
`MCP_UNAVAILABLE` 경계를 따릅니다. 실패 응답은 과거 응답 일부를 노출하지 않습니다.

## 상태와 권한 효과

조회는 읽기 전용입니다. 이벤트, replay 행, Task나 Change Unit 변경, artifact 효과,
쓰기 티켓 효과, `project_state.state_version` 증가를 만들지 않습니다. 이후 상태가
바뀌어도 변경 불가능한 과거 byte나 올바른 cursor는 달라지지 않지만, 현재 권한은
반드시 `volicord.status`로 별도 조회해야 합니다.

`OperationResultRef`는 조회 locator일 뿐 변경 재시도 credential,
`StateRecordRef`, `ArtifactRef`, `AuthorityReceipt`, 쓰기 티켓, Evidence, 권한
token이 아닙니다.

## Core 밖 staging 경계

`volicord.stage_artifact`는 일반 Core commit/replay transaction 밖에서 임시
staging을 만들므로 `OperationResultRef`가 없습니다. Core는 staging 상태를 만들기
전에 전체 직렬화 결과가 담당 문서의 staging 결과 상한 안에 들어가는지 증명해야
합니다. 간결한 MCP 결과는 handle과 만료 시각을 포함한 다음 행동 필드를 모두
보존합니다. 결과가 효과 전 상한을 넘으면 staged handle이나 byte를 만들기 전에
거절합니다.

## 관련 담당 문서

- [MCP 전송](../mcp-transport.md#mutation-authority-receipt-projection)
- [보안](../security.md#historical-operation-result-access)
- [Agent Connection](../agent-connection.md#operation-result-retrieval)
- [저장 기록](../storage-records.md)
- [저장 효과](../storage-effects.md)
- [저장 버전 관리](../storage-versioning.md#exact-operation-result-retrieval)
