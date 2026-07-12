# 오래 유지되는 작업 결과 조회

## 맥락

Agent Connection 변경이 성공적으로 커밋된 뒤에도 MCP 응답 바이트 상한 때문에
정확한 공개 메서드 결과가 반환 상태 보기에 들어가지 못할 수 있습니다. 간결한
결과는 다음 행동에 필요한 필드를 보존하지만 정확한 과거 응답을 오래 유지하는
대체물은 아닙니다. 효과 anchor는 효과를 연관 지을 뿐 응답 바이트를 식별하거나
그 바이트에 대한 접근을 허용하지 않습니다.

일반 Core 커밋 변경은 이미 정확히 직렬화된 응답을 변경 불가능한
`tool_invocations.response_json` 재실행 행에 저장합니다. 이 행을 재사용하면 두
번째 결과 저장소, 저장소 프로필 변경, 재실행과 조회 결과의 불일치를 피할 수
있습니다.

## 결정

Volicord는 추가되는 안정적 읽기 전용 메서드
`volicord.get_operation_result`를 제공합니다. 조회할 수 있는 커밋 및 재실행
작업 상태 보기는 기존의 변경 불가능한 재실행 행에서 파생한
`OperationResultRef`를 전달합니다. Core는 현재 호출과 참조를 검증하고, Store는
일치하는 재실행 응답을 읽으며, Core는 고정된 크기의 UTF-8 JSON 텍스트 페이지를
반환합니다. 디코드한 페이지 청크를 cursor 순서대로 연결하면 저장된 응답
바이트가 정확히 복원됩니다.

조회 구현은 아래 경계를 지킵니다.

- `OperationResultRef`는 내용에 묶인 locator입니다. 메서드, 멱등 키, 커밋 상태
  버전, 바이트 크기, SHA-256 사실이 선택한 재실행 행과 모두 일치해야 합니다.
- cursor는 불투명하며 전체 결과 참조와 다음 바이트 오프셋에 묶입니다. 페이지
  경계는 UTF-8 코드 포인트를 나누지 않으며 응답 바이트를 하나라도 반환하기
  전에 각 페이지의 무결성을 확인합니다.
- 페이지마다 현재 프로젝트 접근과 원래 행위자 일치를 다시 확인합니다. 참조,
  cursor, 효과 anchor, 복사한 연결 식별자는 bearer credential이 아닙니다.
- 조회한 본문은 과거 기록이며 현재 권한이 아닙니다. 호출자는 이전 메서드
  응답을 현재 상태로 취급하지 않고 `volicord.status`에서 현재 권한을 읽습니다.
- 호스트가 중개한 User Channel 답변으로 호출이 끝나면 MCP는 원래 에이전트
  소유 `volicord.request_user_judgment` 결과 참조를 유지합니다. 이 참조를
  안전한 간결한 선택 결과와 함께 반환하며, 사용자 전용
  `volicord.record_user_judgment` 결과나 자유 형식 note는 Agent Connection에
  노출하지 않습니다.
- `volicord.stage_artifact`는 재실행 행 경로 밖에 계속 둡니다. 스테이징이
  바이트나 핸들을 만들기 전에 전체 결과를 직렬화해 크기 상한을 확인해야 하며,
  간결한 MCP 결과는 행동 가능한 모든 스테이징 필드를 보존합니다.

정확한 공개 메서드 동작, 스키마 필드, 페이지 상한, 오류, 저장 효과, 보안
규칙은 집중 참조 담당 문서에 둡니다. 이 결정은 오래 유지할 구현 방향만
기록합니다.

## 결과

- 정확한 복구와 멱등 재실행은 같은 변경 불가능한 응답 본문을 읽습니다.
- 조회는 새 테이블, 아티팩트 계열, 마이그레이션 절차, 저장소 프로필 버전을
  추가하지 않습니다. 기존 `baseline_sqlite_v3` DDL이면 충분합니다.
- Core는 접근과 무결성 판단을, Store는 범위가 확인된 재실행 행 읽기를, MCP는
  결과 참조의 상태 보기와 보존을 담당합니다.
- 조회는 읽기 전용입니다. 변경을 재실행하거나, 권한 이벤트나 재실행 행을
  추가하거나, `project_state.state_version`을 올리지 않습니다.
- 손상, 누락, 잘못된 형식, 다른 결과에 묶인 cursor, 호환되지 않는 접근은
  일부 본문도 반환하지 않고 안전하게 실패합니다.
- 안정적 공개 메서드와 MCP 도구를 추가하는 것은 공개 표면의 minor 변경이므로
  권장 릴리스 버전은 `0.6.0`입니다.

## 비목표

- 이 결정은 과거 응답을 현재 Core 권한으로 만들지 않습니다.
- `OperationResultRef`나 cursor를 인가 토큰으로 만들지 않습니다.
- 사용자 전용 판단 note를 Agent Connection에 노출하지 않습니다.
- 임시 아티팩트 스테이징을 일반 Core 커밋이나 재실행 행으로 바꾸지 않습니다.
- 일반 아티팩트 본문, 이벤트 본문, Runtime Home 파일 다운로드 API를 정의하지
  않습니다.

## 거부한 대안

- 모든 간결한 변경 결과에 정확한 응답 세부사항을 전부 넣는 방식은 간결한 상태
  보기가 다른 목적과 바이트 상한을 가지며 임의의 정확한 과거 데이터를 보존할
  수 없으므로 거부했습니다.
- `effect_anchor`를 조회 credential로 재사용하는 방식은 효과 연관 값이 정확한
  응답 바이트를 식별하지도 접근을 허용하지도 않으므로 거부했습니다.
- 결과를 다시 만들기 위해 변경을 재실행하는 방식은 현재 상태가 달라질 수 있고
  읽기가 효과를 반복해서는 안 되므로 거부했습니다.
- 원본 응답을 페이지 구분 없이 한 본문으로 반환하는 방식은 응답 바이트 상한
  문제를 되살리고 제한된 전송 동작을 약하게 하므로 거부했습니다.
- 별도 작업 결과 테이블을 추가하는 방식은 `tool_invocations.response_json`이 이미
  정확한 변경 불가능 재실행 원천이므로 거부했습니다.
- 응답을 아티팩트로 저장하는 지름길은 아티팩트의 담당 범위, 생명주기, 보존,
  권한 의미가 다르므로 거부했습니다.

## 관련 구현

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  공통 호출 검증과 공개 메서드 디스패치 경계.
- [`crates/volicord-store/src/core_pipeline.rs`](../../../../crates/volicord-store/src/core_pipeline.rs):
  재실행 행 지속 저장과 정확한 응답 조회 경계.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  기준 변경 결과 상태 보기, 크기가 제한된 복구, MCP 응답 래핑.
- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  공개 MCP 도구 메타데이터와 탐색.

## 관련 테스트와 참조 담당 문서

테스트는 여러 페이지의 정확한 재조립, UTF-8 경계, 안정적인 재실행 참조, 누락 및
손상 행, 다른 결과의 cursor, 행위자와 연결 격리, 실패 시 일부 본문 미반환,
호스트가 중개한 판단 개인정보 보호, 스테이징 효과 적용 전 크기 상한을 다뤄야
합니다.

계약 담당 문서는 [`volicord.get_operation_result`](../../reference/api/method-get-operation-result.md),
[API 코어 스키마](../../reference/api/schema-core.md),
[MCP 전송](../../reference/mcp-transport.md),
[Agent Connection](../../reference/agent-connection.md), [보안](../../reference/security.md),
[저장소 기록](../../reference/storage-records.md), [저장 효과](../../reference/storage-effects.md),
[저장소 버전 관리](../../reference/storage-versioning.md)입니다.
