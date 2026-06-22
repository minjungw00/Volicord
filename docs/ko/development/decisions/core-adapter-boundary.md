# Core와 어댑터 의존 경계

## 맥락

Harness 공개 메서드 동작은 어댑터를 통해 접근할 수 있어야 하지만,
어댑터가 메서드 의미를 정의하면 안 됩니다. Rust 워크스페이스에는 Runtime
Home과 호스트 설정을 준비하는 로컬 관리 CLI도 있지만, 그 명령은 공개
Harness API 메서드가 아닙니다.

## 결정

Core 쪽 동작은 `harness-core`에 있고 공유 타입과 Store에 의존하지만
`harness-mcp`나 `harness-cli`에는 의존하지 않습니다. MCP와 CLI 어댑터는
각자의 책임을 위해 낮은 계층에 의존할 수 있습니다.

- `harness-mcp`는 stdio 시작, 세션 바인딩, 도구 메타데이터, 타입 지정
  인수 디코딩, 호출 맥락 파생, 응답 래핑을 맡은 뒤 공개 메서드 실행을
  위해 `CoreService`를 호출합니다.
- `harness-cli`는 공개 Core 메서드가 아니라 Store와 공유 타입을 통해 로컬
  관리 설정, 등록, 설정 계획, 사전 점검 오케스트레이션, 호스트 설정 생성을
  맡습니다.

이 구조는 포트와 어댑터 의존 방향을 닮았지만, 이 페이지는 저장소에서
보이는 구조만 이름 붙입니다.

## 결과

- MCP stdio를 시작하지 않고도 `CoreService`를 직접 테스트할 수 있습니다.
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

- [`crates/harness-core/src/pipeline.rs`](../../../../crates/harness-core/src/pipeline.rs):
  `CoreService`, `MethodPolicy`, `OwnerPipelineBranch`, 공통 사전 점검.
- [`crates/harness-mcp/src/lib.rs`](../../../../crates/harness-mcp/src/lib.rs):
  `PUBLIC_METHOD_TOOL_NAMES`, `McpIntegrationStartupInspection`,
  `McpIntegrationContext`, `McpAdapter`, `McpAdapter::call_tool`,
  `prepare_integration_arguments`.
- [`crates/harness-cli/src/agent_command.rs`](../../../../crates/harness-cli/src/agent_command.rs):
  Core/MCP 어댑터 경로 밖의 관리 호스트 설정 오케스트레이션.
- [`crates/harness-cli/src/registration.rs`](../../../../crates/harness-cli/src/registration.rs):
  등록된 접점의 역량과 로컬 접근 메타데이터 도우미.
- `harness-core`, `harness-mcp`, `harness-cli` Cargo manifest.

## 관련 테스트와 참조 담당 문서

- [`crates/harness-mcp/src/lib.rs`](../../../../crates/harness-mcp/src/lib.rs)의
  `adapter_and_direct_core_status_have_equivalent_response_meaning`.
- [`tests/integration/mcp_surface.rs`](../../../../tests/integration/mcp_surface.rs)의
  `mcp_session_derives_access_per_method_call`,
  `mcp_exposes_exactly_the_documented_public_methods`.
- [API 메서드](../../reference/api/methods.md), [MCP 전송](../../reference/mcp-transport.md),
  [관리 CLI](../../reference/admin-cli.md), [에이전트 통합](../../reference/agent-integration.md).
