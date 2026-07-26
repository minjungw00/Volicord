# Agent Connection 라우팅 설계

## 목적

이 설계는 공개 도구 인수에서 권한 좌표를 받지 않으면서 관리 MCP 경로가 실행 중인
어댑터 하나를 현재 Agent Connection, Runtime Home, 명시적으로 승인된 Product
Repository에 결속하는 방식을 설명합니다.

## 설계

숨은 CLI launcher는 현재 관리 구성을 검증하고 Registry에 일회성 launch lease를
만듭니다. `volicord-mcp`는 그 claim을 메모리에서 소비하고 Connection과 repository
binding을 해석하며 runtime 및 project-session 관찰을 기록한 뒤 typed
`McpConnectionContext`로 `McpAdapter`를 구성합니다.

공개 `volicord mcp serve` 경로는 같은 transport 구현을 사용하지만 계속 수동 runtime
source입니다. Connection scope, mode, project membership, integration revision, active
session 검사는 서로 다른 typed 좌표로 남습니다.

## 불변 조건

- 어댑터 프로세스 하나에는 승인된 Runtime Home과 Connection identity 하나가 있습니다.
- Project routing은 현재 Connection Project membership으로만 해석합니다.
- 공개 인수는 connection, actor, session, project 권한을 선택하거나 바꾸지 못합니다.
- Mutation context는 어댑터 routing context와 같은 정규 Runtime Home identity를
  가져야 합니다.
- 저장된 구성과 session 관찰만으로 actor, binary, 운영체제 identity를 증명하지
  않습니다.

## 책임 경계

`volicord-cli`는 관리 구성, launcher 조율, 로컬 운영자 보고를 담당합니다.
`volicord-mcp`는 시작 binding, lifecycle 승인, 요청 시점 routing, 어댑터가 파생한
invocation fact를 담당합니다. `volicord-store`는 Connection, membership, lease,
runtime-session, project-session 지속 저장을 담당합니다. Core는 검증된 typed
invocation context를 받으며 host 구성과 독립적입니다.

## 실행 흐름

1. 숨은 launcher가 관리 entry를 다시 검증하고 일회성 Registry lease를 발급합니다.
2. MCP 시작이 lease를 소비하고 정규 Runtime Home과 Connection을 해석한 뒤 runtime
   session을 기록합니다.
3. 초기화가 현재 protocol profile을 선택하고 lifecycle milestone을 기록합니다.
4. 각 도구 호출이 Connection mode, project membership, runtime 및 project session,
   revision, mutation-context identity를 다시 검증합니다.
5. 어댑터가 `InvocationContext`를 파생하고 공개 메서드 실행을 위해 Core를 호출합니다.

## 실패 동작

라우팅 상태가 없거나, 오래되었거나, 일치하지 않거나, 손상되었거나, 이미 소비되었다면
Core 실행 전에 관리 경로를 중단합니다. Lifecycle과 routing 실패는 typed diagnostic
identity를 유지하며, 어댑터는 이를 빈 project 선택이나 다른 Connection으로 바꾸지
않습니다.

## 범위 제외

이 설계는 host trust, 사용자 identity, OS 권한, 공개 Connection 필드, session 권한
계약, 명령 동작을 정의하지 않습니다. 두 번째 user-action 해결 channel도 만들지
않습니다.

## 구현 경로

- [`crates/volicord-cli/src/host_launch.rs`](../../../../crates/volicord-cli/src/host_launch.rs)와
  [`host_integration/`](../../../../crates/volicord-cli/src/host_integration/):
  관리 entry 검증과 launch 조율.
- [`crates/volicord-mcp/src/binding.rs`](../../../../crates/volicord-mcp/src/binding.rs),
  [`routing.rs`](../../../../crates/volicord-mcp/src/routing.rs),
  [`adapter.rs`](../../../../crates/volicord-mcp/src/adapter.rs): 시작, routing,
  Core invocation context.
- [`crates/volicord-store/src/agent_connections.rs`](../../../../crates/volicord-store/src/agent_connections.rs),
  [`managed_launch_leases.rs`](../../../../crates/volicord-store/src/managed_launch_leases.rs),
  [`operational_sessions.rs`](../../../../crates/volicord-store/src/operational_sessions.rs):
  지속되는 routing과 session 상태.

## 참조 담당 문서

정확한 동작은 [Agent Connection](../../reference/agent-connection.md),
[MCP 전송](../../reference/mcp-transport.md),
[관리 CLI](../../reference/admin-cli.md),
[런타임 경계](../../reference/runtime-boundaries.md),
[보안](../../reference/security.md)에 남습니다.
