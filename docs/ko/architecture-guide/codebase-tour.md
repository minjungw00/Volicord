# 코드베이스 둘러보기

이 가이드는 Rust workspace를 읽기 위한 유지관리자 경로입니다. 정확한 제품 동작은 집중된
Reference 소유자에 있으며, 구현 코드는 그 계약을 보존해야 합니다.

## Workspace 계층

workspace를 바깥쪽에서 안쪽 순서로 읽습니다.

1. `volicord-command-model`은 완전한 Clap 명령 선언, 명령 DTO, 문법 검증,
   가시성, 명령 introspection, typed 정규 invocation builder를 소유합니다.
2. `volicord-user-action-presentation`은 adapter-neutral UserAction fact의 공유 CLI
   지향 projection을 소유합니다.
3. `volicord-cli`는 프로세스 시작, 관리 명령 디스패치, Codex 연결 설정, CLI 받은
   편지함, 렌더링, stdio 프로세스 시작을 소유합니다.
4. `volicord-mcp`는 MCP 생명주기, JSON-RPC stdio 프레이밍, 공개 도구 디코딩, 응답
   projection을 소유합니다.
5. `volicord-core`는 요청 조율, 메서드 계획, replay 결정, 권한 결과, 원자적 commit
   순서 조율을 소유합니다.
6. `volicord-user-action-service`는 재사용 가능한 UserAction 의미 검증, 구성,
   authority, lifecycle, 영속화 매핑, resolution, continuity, adapter-neutral fact를
   소유합니다.
7. `volicord-store`는 Runtime Home 탐색, SQLite 접근, 엄격한 저장 레코드 검증,
   transaction 적용을 소유합니다.
8. `volicord-types`는 공유 폐쇄 값, 식별자, 정규 인코딩을 소유합니다.
9. `volicord-platform-fs`는 좁은 내부 facade 뒤의 플랫폼별 파일시스템 검사를
   소유합니다.

Core-facing 코드는 CLI나 MCP 어댑터 세부사항에 의존하지 않습니다. 어댑터는 서버 소유
맥락을 파생하고 typed 요청을 Core에 제출합니다.

## 하나의 요청에서 시작하기

MCP 호출은 다음 순서로 추적합니다.

- [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs):
  공개 stream facade
- [`crates/volicord-mcp/src/binding.rs`](../../../crates/volicord-mcp/src/binding.rs):
  Runtime Home, repository, Connection, managed session binding
- [`crates/volicord-mcp/src/transport.rs`](../../../crates/volicord-mcp/src/transport.rs)와
  [`json_rpc.rs`](../../../crates/volicord-mcp/src/json_rpc.rs):
  한도가 있는 framing과 JSON-RPC envelope
- [`crates/volicord-mcp-protocol/src/lib.rs`](../../../crates/volicord-mcp-protocol/src/lib.rs):
  지원 revision의 정확한 선택과 어댑터가 소비하는 완전한 의미 기반 capability profile
- [`crates/volicord-mcp/src/lifecycle.rs`](../../../crates/volicord-mcp/src/lifecycle.rs):
  initialize 순서, message 승인, 폐쇄형 session 상태, 종료
- [`crates/volicord-mcp/src/tool_dispatch.rs`](../../../crates/volicord-mcp/src/tool_dispatch.rs):
  tool call 디코딩, adapter dispatch, 공유 result carrier
- [`crates/volicord-mcp/src/mutation_projection.rs`](../../../crates/volicord-mcp/src/mutation_projection.rs),
  [`authority_refresh.rs`](../../../crates/volicord-mcp/src/authority_refresh.rs),
  [`committed_result_recovery.rs`](../../../crates/volicord-mcp/src/committed_result_recovery.rs):
  일반 mutation projection, 현재 authority 재조회, 선택된 profile의 projection 예산을
  초과하는 committed result의 authority 우선 복구
- [`crates/volicord-mcp/src/user_action_projection.rs`](../../../crates/volicord-mcp/src/user_action_projection.rs):
  공개 UserAction result와 CLI fallback projection
- [`crates/volicord-mcp/src/telemetry.rs`](../../../crates/volicord-mcp/src/telemetry.rs)와
  [`session_metrics.rs`](../../../crates/volicord-mcp/src/session_metrics.rs):
  진단 fact와 runtime-session metric
- [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs):
  context에 결합된 adapter와 Core 호출
- [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs):
  공통 사전 점검, replay, 계획, commit 선택
- [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/):
  메서드별 계획
- [`crates/volicord-store/src/core_pipeline/`](../../../crates/volicord-store/src/core_pipeline/):
  `CoreProjectStore` facade, aggregate별 읽기와 grouped mutation, Store 검증,
  replay, 원자적 commit 조율

전체 순서는 [요청 생명주기](request-lifecycle.md), transaction 경계는
[저장소와 transaction](storage-and-transactions.md)을 봅니다.

## Codex 설정에서 시작하기

관리 연결 작업은 다음 경로를 따릅니다.

- [`crates/volicord-command-model/src/lib.rs`](../../../crates/volicord-command-model/src/lib.rs):
  명령 선언과 parsing된 명령 DTO
- [`crates/volicord-cli/src/connection_command/`](../../../crates/volicord-cli/src/connection_command/):
  명령 조율
- [`crates/volicord-cli/src/host_integration/codex/`](../../../crates/volicord-cli/src/host_integration/codex/):
  Codex 탐색, 구성, 식별, 검증
- [`crates/volicord-store/src/agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs):
  저장 연결 레코드
- [`crates/volicord-mcp/src/binding.rs`](../../../crates/volicord-mcp/src/binding.rs):
  결속된 프로세스 시작 점검과 managed call 상관관계

지원되는 연결 의도는 `personal`과 `shared`이며, 둘 다 Record profile stdio 경계를
시작합니다.

## 사용자 소유 행동에서 시작하기

`volicord.request_user_action`은 pending Core 요청을 생성하거나 재개합니다. 로컬
구현 흐름은
[`crates/volicord-user-action-service/src/`](../../../crates/volicord-user-action-service/src/)의
재사용 가능한 의미 model, validation, 정규 body와 identity, Store 기반 typed fact
취득, materialization, 영속화 매핑, resolution, continuity, projection으로
이어집니다. Core 요청 조율은
[`methods/user_action.rs`](../../../crates/volicord-core/src/methods/user_action.rs)에
있고 User Channel 읽기와 continuity 영속화는 인접한 `user_action_read.rs`,
`user_action_continuity.rs`에 있습니다. Reconciliation은
[`methods/reconcile_changes.rs`](../../../crates/volicord-core/src/methods/reconcile_changes.rs)에서
의미 의도를 제공합니다.

CLI 받은 편지함은 adapter-neutral Core fact를 읽고 공유 presentation과 정규
command-model invocation으로 엄격한 저장 form을 표시한 뒤 별도 user-only
resolution 경로를 호출합니다. MCP는 자체 safe projection을 구성하며 그 form을
표시하거나 제출하지 않습니다. Guard prompt capture는 관찰 원천일 뿐입니다.

정확한 계약은 [User Action Schema](../reference/api/schema-user-action.md),
[Request User Action](../reference/api/method-request-user-action.md),
[Resolve User Action](../reference/api/method-resolve-user-action.md)을 봅니다.

## 테스트 경로

지속 가능한 검사는 불변식을 소유하는 가장 좁은 계층에 둡니다.

- 순수 파싱, 인코딩, 정책, UserAction 서비스 의미는 인접 unit test
- 어댑터와 Store 경계는 crate integration test
- 공개 교차 메서드 동작은 workspace conformance test
- target, 패키지, checksum, workflow invariant는 일반 release-integrity 테스트

[테스트 전략](testing-strategy.md)과 [검증](../maintain/validation.md)을 함께 봅니다.
