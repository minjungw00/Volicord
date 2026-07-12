# Core와 어댑터 의존 경계

## 맥락

Volicord 공개 메서드 동작은 어댑터를 통해 접근할 수 있어야 하지만,
어댑터가 메서드 의미를 정의하면 안 됩니다. Rust 워크스페이스에는 Runtime
Home과 호스트 설정을 준비하는 로컬 관리 CLI도 있지만, 그 명령은 공개
Volicord API 메서드가 아닙니다.

MCP 변경 래핑에는 Core가 반환한 뒤 어댑터가 소유하는 결과 하나도 필요합니다.
이전에는 정상 상세 수준 렌더링, 응답 바이트 상한 복구, 효과 적용 후 복구,
새로 고침 실패 복구가 서로 다른 인접 상태 보기 단계를 만들었습니다. 이 단계들은
같은 메서드 결과와 새 권한 receipt에서 서로 다른 부분을 보존할 수 있었습니다.

## 결정

Core 쪽 동작은 `volicord-core`에 있고 공유 타입과 Store에 의존하지만
`volicord-mcp`나 `volicord-cli`에는 의존하지 않습니다. MCP와 CLI 어댑터는
각자의 책임을 위해 낮은 계층에 의존할 수 있습니다.

- `volicord-mcp`는 stdio와 로컬 HTTP 전송 시작, 세션 바인딩, 도구
  메타데이터, 형식화된 인수 디코딩, 로컬 호출 사실 파생, 응답 래핑을 맡은
  뒤 공개 메서드 실행을 위해 `CoreService`를 호출합니다.
- `volicord-cli`는 공개 Core 메서드가 아니라 Store와 공유 타입을 통해 로컬
  관리 설정, 등록, 설정 계획, 사전 점검 조율, 호스트 설정 생성을
  맡습니다.

이 구조는 포트와 어댑터 의존 방향을 닮았지만, 이 페이지는 저장소에서
보이는 구조만 이름 붙입니다.

### 기준 MCP 변경 결과

Core가 변경 결과를 반환하면 MCP 어댑터는 정확한 메서드 결과, 그 결과에서
파생한 간결한 결과, 효과 사실, 사용할 수 있을 때 검증된 새 receipt, 현재 다음
행동을 담은 내부 기준 결과 하나를 구성합니다. 정상 상세 수준 렌더링과 크기가
제한된 모든 복구 경로는 이 결과를 사용합니다. 정확한 wire 필드, 보존 우선순위,
바이트 상한, 재실행할 수 없는 복구 동작은 계속
[MCP 전송](../../reference/mcp-transport.md#mutation-authority-receipt-projection)이
담당합니다.

이 결과는 어댑터 상태 보기 객체입니다. 두 번째 Core 결과, Store 기록, 권한
출처, 정확한 공개 메서드 응답의 대체물이 아닙니다. 따라서 Core와 Store는 MCP
바이트 상한과 호스트 응답 형태 정책에 의존하지 않습니다.

## 결과

- MCP 전송을 시작하지 않고도 `CoreService`를 직접 테스트할 수 있습니다.
- MCP 통합 테스트는 어댑터에서 보이는 동작과 직접 Core 동작을 비교할 수
  있습니다.
- 어댑터 시작 검증은 Store를 직접 사용할 수 있지만, 그 Store 사용은 공개
  메서드 동작의 다른 구현이 아닙니다.
- 공개 메서드 추가나 동작 변경은 어댑터 디스패치만이 아니라 Core와 참조
  담당 문서를 갱신해야 합니다.
- 인접한 MCP 응답 분기는 서로 다른 간결한 결과 파생 방식이나 보존 순서를
  선택할 수 없습니다.
- null일 수 있는 기존 공개 필드로 이미 표현할 수 있는 새 복구 조합은 호환되는
  동작 수정입니다. 저장소 마이그레이션은 필요하지 않으며, 릴리스 버전 영향은
  관련 공개 계약 변경 묶음과 함께 평가합니다.

## 비목표

- 이 결정은 공개 메서드 목록이나 메서드 동작을 정의하지 않습니다.
- CLI 명령을 공개 API 메서드로 만들지 않습니다.
- MCP 전송 계약이나 보안 보장을 정의하지 않습니다.
- 어댑터가 자체 시작, 바인딩, 설정 검증을 수행하지 못하게 하지 않습니다.
- MCP 상태 보기 결과를 저장하거나 효과 연관 anchor를 정확한 결과 조회
  credential로 만들지 않습니다.

## 거부한 대안

- 분기마다 별도 보존 단계를 유지하는 방식은 각 분기의 테스트가 통과해도 보존
  우선순위가 서로 달라질 수 있으므로 거부했습니다.
- 응답 바이트 상한이나 간결한 결과 선택을 Core로 옮기는 방식은 MCP 전송
  관심사로 어댑터 의존 경계를 거꾸로 만들기 때문에 거부했습니다.
- receipt나 메서드 결과를 잘라 내는 방식은 일부만 남은 권한 객체나 다음 행동
  결과의 의미를 바꾸므로 거부했습니다.

## 관련 구현

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `CoreService`, `MethodPolicy`, `OwnerPipelineBranch`, 공통 사전 점검.
- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  `PUBLIC_METHOD_TOOL_NAMES`, `McpToolDefinition`, 공개 도구 메타데이터.
- [`crates/volicord-mcp/src/routing.rs`](../../../../crates/volicord-mcp/src/routing.rs):
  `McpConnectionStartupInspection`, `McpConnectionContext`, 시작과 프로젝트 처리 경로
  도우미.
- [`crates/volicord-mcp/src/adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs):
  `McpAdapter`, `McpAdapter::call_tool`, 형식화된 인자 준비, 어댑터가 생성한
  요청 래퍼 필드, 로컬 호출 사실 파생, Core 디스패치.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  JSON-RPC 표준 입출력 디스패치, 기준 변경 결과 상태 보기, `tools/call` 결과
  래핑, 사용자 입력 요청 처리.
- [`crates/volicord-mcp/src/local_http.rs`](../../../../crates/volicord-mcp/src/local_http.rs):
  로컬 HTTP 수신기 설정, 연결 맥락, 세션 처리, MCP 요청 처리 경로.
- [`crates/volicord-cli/src/connection_command/service.rs`](../../../../crates/volicord-cli/src/connection_command/service.rs):
  Core/MCP 어댑터 경로 밖의 관리 호스트 설정과 연결 구성.
- [`crates/volicord-store/src/bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs)와
  [`crates/volicord-store/src/agent_connections.rs`](../../../../crates/volicord-store/src/agent_connections.rs):
  관리 구성에서 사용하는 프로젝트 등록, Agent Connection 기록,
  Connection Project 멤버십.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs)와
  [`crates/volicord-core/src/methods/judgment.rs`](../../../../crates/volicord-core/src/methods/judgment.rs):
  로컬 User Channel 조율과 Core 판단 기록.
- `volicord-core`, `volicord-mcp`, `volicord-cli` Cargo 매니페스트.

## 관련 테스트와 참조 담당 문서

- [`crates/volicord-core/src/methods/tests/status.rs`](../../../../crates/volicord-core/src/methods/tests/status.rs)의
  `status_is_read_only_including_dry_run`,
  [`crates/volicord-mcp/src/tests.rs`](../../../../crates/volicord-mcp/src/tests.rs)의
  `mcp_status_succeeds_with_readonly_storage`,
  `mcp_status_does_not_advance_state_version`은 전체 응답 동등성이 아니라 Core와
  MCP에서 보이는 읽기 전용 속성을 각각 확인합니다.
- [`tests/integration/mcp_connection.rs`](../../../../tests/integration/mcp_connection.rs)의
  `connection_invocation_is_injected_and_single_project_is_auto_selected`,
  `read_only_mode_rejects_agent_workflow_methods_before_core`.
- [API 메서드](../../reference/api/methods.md), [MCP 전송](../../reference/mcp-transport.md),
  [관리 CLI](../../reference/admin-cli.md), [Agent Connection](../../reference/agent-connection.md).
