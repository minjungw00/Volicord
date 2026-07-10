# Core와 어댑터 의존 경계

## 맥락

Volicord 공개 메서드 동작은 어댑터를 통해 접근할 수 있어야 하지만,
어댑터가 메서드 의미를 정의하면 안 됩니다. Rust 워크스페이스에는 Runtime
Home과 호스트 설정을 준비하는 로컬 관리 CLI도 있지만, 그 명령은 공개
Volicord API 메서드가 아닙니다.

## 결정

Core 쪽 동작은 `volicord-core`에 있고 공유 타입과 Store에 의존하지만
`volicord-mcp`나 `volicord-cli`에는 의존하지 않습니다. MCP와 CLI 어댑터는
각자의 책임을 위해 낮은 계층에 의존할 수 있습니다.

- `volicord-mcp`는 stdio와 로컬 HTTP 전송 시작, 세션 바인딩, 도구
  메타데이터, 타입 지정 인수 디코딩, 로컬 호출 사실 파생, 응답 래핑을 맡은
  뒤 공개 메서드 실행을 위해 `CoreService`를 호출합니다.
- `volicord-cli`는 공개 Core 메서드가 아니라 Store와 공유 타입을 통해 로컬
  관리 설정, 등록, 설정 계획, 사전 점검 오케스트레이션, 호스트 설정 생성을
  맡습니다.

이 구조는 포트와 어댑터 의존 방향을 닮았지만, 이 페이지는 저장소에서
보이는 구조만 이름 붙입니다.

## 결과

- MCP 전송을 시작하지 않고도 `CoreService`를 직접 테스트할 수 있습니다.
- MCP 통합 테스트는 어댑터에서 보이는 동작과 직접 Core 동작을 비교할 수
  있습니다.
- 어댑터 시작 검증은 Store를 직접 사용할 수 있지만, 그 Store 사용은 공개
  메서드 동작의 다른 구현이 아닙니다.
- 공개 메서드 추가나 동작 변경은 어댑터 디스패치만이 아니라 Core와 참조
  담당 문서를 갱신해야 합니다.

## 비목표

- 이 결정은 공개 메서드 목록이나 메서드 동작을 정의하지 않습니다.
- CLI 명령을 공개 API 메서드로 만들지 않습니다.
- MCP 전송 계약이나 보안 보장을 정의하지 않습니다.
- 어댑터가 자체 시작, 바인딩, 설정 검증을 수행하지 못하게 하지 않습니다.

## 관련 구현

- [`crates/volicord-core/src/pipeline.rs`](../../../../crates/volicord-core/src/pipeline.rs):
  `CoreService`, `MethodPolicy`, `OwnerPipelineBranch`, 공통 사전 점검.
- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  `PUBLIC_METHOD_TOOL_NAMES`, `McpToolDefinition`, 공개 도구 메타데이터.
- [`crates/volicord-mcp/src/routing.rs`](../../../../crates/volicord-mcp/src/routing.rs):
  `McpConnectionStartupInspection`, `McpConnectionContext`, 시작/프로젝트 라우팅
  도우미.
- [`crates/volicord-mcp/src/adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs):
  `McpAdapter`, `McpAdapter::call_tool`, 타입 지정 인자 준비, 어댑터가 생성한
  요청 래퍼 필드, 로컬 호출 사실 파생, Core 디스패치.
- [`crates/volicord-mcp/src/stdio.rs`](../../../../crates/volicord-mcp/src/stdio.rs):
  JSON-RPC stdio 디스패치, `tools/call` 결과 래핑, elicitation 처리.
- [`crates/volicord-mcp/src/local_http.rs`](../../../../crates/volicord-mcp/src/local_http.rs):
  로컬 HTTP 수신기 설정, 연결 맥락, 세션 처리, MCP 요청 라우팅.
- [`crates/volicord-cli/src/connection_command/service.rs`](../../../../crates/volicord-cli/src/connection_command/service.rs):
  Core/MCP 어댑터 경로 밖의 관리 호스트 설정과 연결 프로비저닝.
- [`crates/volicord-store/src/bootstrap.rs`](../../../../crates/volicord-store/src/bootstrap.rs)와
  [`crates/volicord-store/src/agent_connections.rs`](../../../../crates/volicord-store/src/agent_connections.rs):
  관리 프로비저닝에서 사용하는 프로젝트 등록, Agent Connection 기록,
  Connection Project 멤버십.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs)와
  [`crates/volicord-core/src/methods/judgment.rs`](../../../../crates/volicord-core/src/methods/judgment.rs):
  로컬 User Channel 오케스트레이션과 Core 판단 기록.
- `volicord-core`, `volicord-mcp`, `volicord-cli` Cargo manifest.

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
