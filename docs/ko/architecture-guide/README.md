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

## 읽는 순서

1. 워크스페이스와 크레이트 책임: [코드베이스 둘러보기](codebase-tour.md)에서
   시작합니다. 모든 Cargo 워크스페이스 멤버, 처음 열 소스 파일, 중요한
   심볼, 관련 테스트, 다음에 읽을 컴포넌트를 확인합니다.
2. 정확한 소스 담당: [소스 지도](source-map.md)에서 정확한 소스 경로, 모듈
   책임, CLI 하위 모듈 경계, 호스트 어댑터 배치, guard integration 배치, MCP
   어댑터 모듈, 테스트 지원 경로를 봅니다.
3. 대표 요청 흐름: [요청 생명주기](request-lifecycle.md)를 읽습니다.
   `volicord.status`, `volicord.intake`, `volicord.prepare_write`가 MCP
   `tools/call`에서 Core와 Store 동작을 거쳐 MCP 응답 래퍼로 돌아오는
   경로를 따라갑니다.
4. 아키텍처와 경계: [구현 아키텍처](architecture.md)에서 오래 유지되는
   워크스페이스 형태, 의존 방향, 실행 흐름 지도, 관리 CLI 설정 흐름,
   코드에서 담당 문서로 가는 경로를 봅니다.
5. 구현 패턴: [구현 설계 패턴](design-patterns.md)에서 `CoreService`,
   `MethodPolicy`, 메서드 계획, `CoreStorageMutation`, 주입된 시간, 불투명
   ID, 제어 enum, 정규 요청 해시, 공유 테스트 픽스처 같은 반복 구조를
   봅니다.
6. 저장소와 트랜잭션 개념: [저장소와 트랜잭션](storage-and-transactions.md)에서
   Runtime Home, registry와 프로젝트 데이터베이스, `CoreProjectStore`,
   메서드 계획, 변이 값, 원자적 커밋, 재실행, 아티팩트 스테이징, 실패
   경계를 읽습니다. 정확한 저장소 질문은 [저장소](../reference/storage.md),
   [저장 효과](../reference/storage-effects.md),
   [저장소 기록](../reference/storage-records.md),
   [저장소 DDL](../reference/storage-ddl.md),
   [아티팩트 저장소](../reference/storage-artifacts.md),
   [저장소 버전 관리](../reference/storage-versioning.md)로 보냅니다.
7. 테스트 전략: [테스트 전략](testing-strategy.md)에서 모듈 단위 테스트,
   Core 메서드 테스트, 바이너리 테스트, MCP 통합 테스트, 적합성 구현 테스트,
   `volicord-test-support` 중 무엇을 사용할지 고릅니다.
8. 오래 유지될 결정: [아키텍처 결정](decisions/README.md)에서 Core/어댑터
   경계, 원자적 변이 커밋 전 계획, `Volicord Runtime Home`과
   `Product Repository` 분리를 집중 설명으로 확인합니다.
9. 변경 작업 흐름: 변경을 분류하고, 담당 문서를 찾고, 구현 경계를 확인하고,
   검증을 고르고, 영향을 받은 아키텍처 가이드 페이지를 갱신할 준비가 되면
   [구현 가이드](change-guide.md)를 사용합니다.
10. 정확한 참조 계약: [참조 색인](../reference/README.md)과
   [API 메서드](../reference/api/methods.md)를 사용합니다. 학습 문서는
   현재 코드 배치를 설명할 수 있지만, 정확한 메서드 동작, 스키마, 저장
   효과, 보안 표현, 런타임 경계, 오류 의미, Core 권한 의미는 참조 문서가
   담당합니다.

## 학습 문서와 담당 문서

| 질문 | 여기서 시작 | 정확한 담당 경로 |
|---|---|---|
| 어떤 크레이트를 먼저 열어야 하나? | [코드베이스 둘러보기](codebase-tour.md) | [구현 아키텍처](architecture.md)가 가이드 수준 구현 구조를 담당합니다. |
| 어떤 경로가 모듈 책임을 담당하나? | [소스 지도](source-map.md) | [구현 아키텍처](architecture.md)는 상위 수준 아키텍처 경계를 담당합니다. 정확한 제품 동작은 계속 참조 담당 문서로 보냅니다. |
| 메서드 호출이 MCP, Core, Store를 지나 응답까지 어떻게 흐르나? | [요청 생명주기](request-lifecycle.md) | 메서드 동작은 [API 메서드](../reference/api/methods.md)와 연결된 메서드 담당 문서가 담당합니다. |
| 왜 Core는 CLI나 MCP에 의존하지 않나? | [구현 아키텍처](architecture.md)와 [Core와 어댑터 의존 경계](decisions/core-adapter-boundary.md) | 의존 경계 안내는 아키텍처 가이드 문서에 남고, 공개 동작은 참조 담당 문서로 돌아갑니다. |
| 왜 계획 함수와 Store 커밋이 분리되나? | [구현 설계 패턴](design-patterns.md)과 [원자적 변이 커밋 전 계획](decisions/plan-and-atomic-commit.md) | 정확한 메서드 동작과 저장 효과는 메서드와 저장소 담당 문서로 보냅니다. |
| 어떤 저장소 변이가 커밋되나? | [요청 생명주기](request-lifecycle.md)와 [저장소와 트랜잭션](storage-and-transactions.md) | 정확한 저장 효과는 [저장 효과](../reference/storage-effects.md)와 인접 저장소 담당 문서로 보냅니다. |
| 어떤 테스트 계층을 써야 하나? | [테스트 전략](testing-strategy.md) | 테스트는 담당 문서가 정의한 사실을 검증하지만 제품 계약을 담당하지 않습니다. |
| 변경할 때 무엇을 고쳐야 하나? | [구현 가이드](change-guide.md) | [참조 색인](../reference/README.md) 또는 `docs/doc-index.yaml`에서 고른 집중 참조 담당 문서입니다. |

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

에이전트 호스트 설정과 운영자 동작을 읽을 때는
[`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs)에서
시작한 뒤
[`crates/volicord-cli/src/connection_command.rs`](../../../crates/volicord-cli/src/connection_command.rs),
[`crates/volicord-cli/src/host_integration/`](../../../crates/volicord-cli/src/host_integration/),
[`crates/volicord-cli/src/registration.rs`](../../../crates/volicord-cli/src/registration.rs)를
읽습니다. 로컬 User Channel 동작은 이어서
[`crates/volicord-cli/src/user_command.rs`](../../../crates/volicord-cli/src/user_command.rs)를
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
