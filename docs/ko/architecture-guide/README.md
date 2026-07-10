# 아키텍처 가이드

아키텍처 가이드는 현재 Rust 워크스페이스를 이해해야 하는 구현자, 검토자,
소스 코드 학습자를 위한 진입점입니다. 워크스페이스 구조, 정확한 소스 경로
책임, 요청 흐름, 저장소와 트랜잭션 경계, 설계 패턴, 테스트 전략, 오래 유지될
결정, 구현 변경 작업 흐름으로 안내합니다.

이 문서들은 구현이 어떻게 배치되어 있고 오래 유지될 경계가 왜 있는지 배우기 위한
자료입니다. 정확한 공개 API 동작, 요청이나 응답 스키마, 저장 효과, 보안 보장,
런타임 경계, Core 권한 의미, 그 밖의 제품 계약은 집중 참조 담당 문서에 있습니다.

Volicord는 AI 지원 제품 작업을 위한 로컬 작업 권한 기록입니다.
Core는 Volicord 상태를 위한 로컬 기준 기록입니다.

## 목적에 맞는 읽기 경로

모든 문서를 순서대로 읽을 필요는 없습니다. 하려는 작업에 맞는 경로에서
시작합니다.

| 목적 | 읽기 경로 | 알 수 있는 내용 |
|---|---|---|
| 워크스페이스 익히기 | [코드베이스 둘러보기](codebase-tour.md) -> [구현 아키텍처](architecture.md) -> [소스 지도](source-map.md) | 어떤 크레이트부터 읽을지, 의존성이 어느 방향인지, 구현 책임이 어느 모듈에 있는지 알 수 있습니다. |
| 관리 작업 흐름 따라가기 | [CLI 작업 흐름](cli-workflows.md) -> [소스 지도](source-map.md) | 설정, 연결, 호스트, 관찰 훅, 진단 경로가 어떻게 조합되고 각 부분이 어디에 있는지 알 수 있습니다. |
| 공개 메서드 호출 따라가기 | [요청 생명주기](request-lifecycle.md) -> [구현 설계 패턴](design-patterns.md) -> [저장소와 트랜잭션](storage-and-transactions.md) | MCP, Core, Store가 어떻게 협력하고 어떤 구조가 반복되며 어디서 지속 저장이 시작되는지 알 수 있습니다. |
| 변경 계획 세우기 | [구현 가이드](change-guide.md) -> [테스트 전략](testing-strategy.md) -> [아키텍처 결정](decisions/README.md) | 어떤 담당 문서와 소스 영역을 확인할지, 어떤 테스트 계층을 사용할지, 구현 경계가 왜 존재하는지 알 수 있습니다. |
| 정확한 동작 확인하기 | [참조 색인](../reference/README.md) -> [API 메서드](../reference/api/methods.md) | API, 스키마, 저장소, 보안, 런타임, 오류, Core 권한 세부사항을 어느 집중 참조 문서가 담당하는지 알 수 있습니다. |

## 소스 읽기 지름길

전체 소스 경로 책임은 [소스 지도](source-map.md)를 사용합니다. 아래 지름길은
자주 묻는 구현 질문에서 흔히 먼저 여는 경로입니다.

공개 메서드 작업에서 가장 짧게 유용한 소스 경로는 아래와 같습니다.

1. [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)
2. [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)
3. [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)
4. [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/)
5. [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)
6. [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)
7. [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)

에이전트 호스트 설정과 운영자 동작을 읽을 때는 실행 흐름 경계를
[CLI 작업 흐름](cli-workflows.md)에서 먼저 확인합니다. 그런 다음
[`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs)에서
시작해
[`crates/volicord-cli/src/connection_command.rs`](../../../crates/volicord-cli/src/connection_command.rs),
[`crates/volicord-cli/src/connection_command/service.rs`](../../../crates/volicord-cli/src/connection_command/service.rs),
[`crates/volicord-cli/src/host_integration/`](../../../crates/volicord-cli/src/host_integration/),
[`crates/volicord-store/src/bootstrap.rs`](../../../crates/volicord-store/src/bootstrap.rs),
[`crates/volicord-store/src/agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs)를
읽습니다. 로컬 User Channel 동작은 이어서
[`crates/volicord-cli/src/user_command.rs`](../../../crates/volicord-cli/src/user_command.rs)와
[`crates/volicord-core/src/methods/judgment.rs`](../../../crates/volicord-core/src/methods/judgment.rs)를
읽습니다.

## 경계 기억하기

- Core 쪽 코드는 CLI와 MCP 어댑터 크레이트에 의존하지 않습니다.
- `volicord-mcp`는 시작과 세션 검증을 위해 Store를 직접 사용할 수 있습니다.
  이 직접 Store 사용은 공개 메서드 의미를 구현하는 다른 경로가 아닙니다.
- `Volicord Runtime Home`과 `Product Repository`는 서로 다른 위치입니다.
- 테스트는 담당 문서가 정의한 사실을 검증하지만, 테스트와 픽스처는 제품
  계약 담당 문서가 아닙니다.
- 학습 문서는 소스 파일과 심볼을 이름으로 가리키며, 불안정한 줄 번호를
  사용하지 않습니다.
