# 코드베이스 둘러보기

이 가이드는 Rust workspace를 읽기 위한 유지관리자 경로입니다. 정확한 제품 동작은 집중된
Reference 소유자에 있으며, 구현 코드는 그 계약을 보존해야 합니다.

## Workspace 계층

workspace를 바깥쪽에서 안쪽 순서로 읽습니다.

1. `volicord-cli`는 로컬 관리 명령, Codex 연결 설정, CLI 받은 편지함, stdio 프로세스
   시작을 소유합니다.
2. `volicord-mcp`는 MCP 생명주기, JSON-RPC stdio 프레이밍, 공개 도구 디코딩, 응답
   projection을 소유합니다.
3. `volicord-core`는 메서드 계획, 정책, replay 결정, 권한 결과, 원자적 commit 조율을
   소유합니다.
4. `volicord-store`는 Runtime Home 탐색, SQLite 접근, 엄격한 저장 레코드 검증,
   transaction 적용을 소유합니다.
5. `volicord-types`는 공유 폐쇄 값, 식별자, 정규 인코딩을 소유합니다.
6. `volicord-platform-fs`는 좁은 내부 facade 뒤의 플랫폼별 파일시스템 검사를
   소유합니다.

Core-facing 코드는 CLI나 MCP 어댑터 세부사항에 의존하지 않습니다. 어댑터는 서버 소유
맥락을 파생하고 typed 요청을 Core에 제출합니다.

## 하나의 요청에서 시작하기

MCP 호출은 다음 순서로 추적합니다.

- [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs):
  프로세스 생명주기와 프레이밍
- [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs):
  공개 인수 디코딩과 Core 디스패치
- [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs):
  공통 사전 점검, replay, 계획, commit 선택
- [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/):
  메서드별 계획
- [`crates/volicord-store/src/core_pipeline/`](../../../crates/volicord-store/src/core_pipeline/):
  Store 검증과 원자적 효과

전체 순서는 [요청 생명주기](request-lifecycle.md), transaction 경계는
[저장소와 transaction](storage-and-transactions.md)을 봅니다.

## Codex 설정에서 시작하기

관리 연결 작업은 다음 경로를 따릅니다.

- [`crates/volicord-cli/src/connection_command/`](../../../crates/volicord-cli/src/connection_command/):
  명령 조율
- [`crates/volicord-cli/src/host_integration/codex/`](../../../crates/volicord-cli/src/host_integration/codex/):
  Codex 탐색, 구성, 식별, 검증
- [`crates/volicord-store/src/agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs):
  저장 연결 레코드
- [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs):
  결속된 프로세스 시작 점검

지원되는 연결 의도는 `personal`과 `shared`이며, 둘 다 Record profile stdio 경계를
시작합니다.

## 사용자 소유 행동에서 시작하기

`volicord.request_user_action`은 pending Core 요청을 생성하거나 재개합니다. 로컬 CLI
받은 편지함이 엄격한 저장 form을 표시하고 별도 user-only resolution 경로를 호출합니다.
MCP는 그 form을 표시하거나 제출하지 않습니다. Guard prompt capture는 관찰 원천일
뿐입니다.

정확한 계약은 [User Action Schema](../reference/api/schema-user-action.md),
[Request User Action](../reference/api/method-request-user-action.md),
[Resolve User Action](../reference/api/method-resolve-user-action.md)을 봅니다.

## 테스트 경로

지속 가능한 검사는 불변식을 소유하는 가장 좁은 계층에 둡니다.

- 순수 파싱, 인코딩, 정책은 인접 unit test
- 어댑터와 Store 경계는 crate integration test
- 공개 교차 메서드 동작은 workspace conformance test
- 정확한 최종 아티팩트 지원 주장은 독립된 Codex target/environment release-validation 셀 여섯 개

[테스트 전략](testing-strategy.md)과 [검증](../maintain/validation.md)을 함께 봅니다.
