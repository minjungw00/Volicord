# 구현 가이드

이 가이드는 Rust 워크스페이스에서 좁은 구현 변경을 수행하는 실용적인
흐름을 제공합니다. 제품 의미는 집중 참조 담당 문서에 남습니다. 이 문서는
기준 범위, API 동작, 스키마, 저장 효과, 보안 보장, 런타임 경계, 오류
동작, 닫기 준비 상태 규칙, 커넥터 동작, 적합성 권한, Core 권한 의미를
정의하거나 덮어쓰지 않습니다.

소스를 배우는 중이면 [개발자 문서](README.md)를 사용하고, 첫 파일과
심볼은 [코드베이스 둘러보기](codebase-tour.md), 대표 메서드 흐름은
[요청 생명주기](request-lifecycle.md), 반복 구조는 [구현 설계 패턴](design-patterns.md),
Store 경계는 [저장소와 트랜잭션](storage-and-transactions.md), 테스트 계층
선택은 [테스트 전략](testing-strategy.md)을 사용합니다. 기계가 읽는 담당
경로는 [`docs/doc-index.yaml`](../../doc-index.yaml)을 사용하고, 사람이 읽는
담당 문서 안내는 [참조 색인](../reference/README.md)을 사용합니다.

Volicord는 AI 지원 제품 작업을 위한 로컬 작업 권한 기록입니다.
Core는 Volicord 상태를 위한 로컬 기준 기록입니다.

## 실용 순서

1. 요청된 변경을 분류합니다.

   변경이 공유 타입, Store 동작, Core 메서드 동작, MCP 어댑터 동작, 관리
   설정, 테스트 픽스처, 개발자 문서 전용 중 어디에 닿는지 정합니다. 둘
   이상의 경계를 건너면 질문을 나누어 둡니다.

2. 현재 구현 경로를 찾습니다.

   [구현 아키텍처](architecture.md)에서 워크스페이스와 모듈 지도를 확인한
   뒤 아래 경로 표에서 가장 가까운 소스와 테스트를 엽니다. 편집 전에
   이름 붙인 심볼이 여전히 존재하는지 확인합니다.

3. 정확한 참조 담당 문서를 식별합니다.

   [참조 색인](../reference/README.md) 또는
   [`docs/doc-index.yaml`](../../doc-index.yaml)을 사용합니다. 메서드 동작은
   [API 메서드](../reference/api/methods.md)에서 시작하고, 저장소 질문은
   [저장소](../reference/storage.md)에서 시작하며, 런타임 위치 질문은
   [런타임 경계](../reference/runtime-boundaries.md)에서 시작합니다.

4. 좁은 변경을 구현합니다.

   구현 책임을 가진 크레이트나 모듈을 바꿉니다. Core 쪽 코드는 CLI와 MCP
   어댑터 크레이트에서 독립적으로 유지합니다. 새 API 동작, 스키마 의미,
   저장 효과, 보안 보장, Core 권한 의미를 코드, 테스트, 픽스처, 예시,
   생성된 출력, 주석에만 넣지 않습니다.

5. 알맞은 테스트 계층을 고릅니다.

   [테스트 전략](testing-strategy.md)을 사용해 변경된 동작을 보호하는 가장
   작은 계층을 고르고, 변경이 계층을 건널 때만 더 넓은 테스트를 추가합니다.

6. 영향을 받은 개발자 설명을 갱신합니다.

   오래 유지될 소스 형태, 의존 방향, 실행 흐름, Store 경계, 테스트 구조,
   변경 작업 흐름이 바뀌면 관련 개발자 페이지를 두 언어 모두에서
   갱신합니다. 정확한 제품 계약은 참조 담당 문서에 둡니다.

7. 검증을 실행합니다.

   Rust 구현을 편집했으면 기본적으로 `cargo fmt`,
   `cargo clippy --all-targets --all-features`,
   `cargo test --all-targets --all-features`를 실행합니다. 문서를 편집했으면
   구조, 링크/색인, 언어 일치, 용어에 맞는 Maintain 점검을 실행합니다.
   실행하지 않은 명령은 이유를 보고합니다.

8. 동작을 새로 만들지 말고 담당 문서 공백을 보고합니다.

   구현에 필요한 동작을 어떤 담당 문서도 정의하지 않는다면 제품 의미
   변경을 멈추고 담당 문서 공백을 보고하거나 적절한 참조 담당 문서를 먼저
   갱신합니다. README, 가이드, 테스트, 픽스처, 어댑터, 생성된 출력,
   구현 주석으로 그 공백을 메우지 않습니다.

## 변경 유형 경로

| 변경 유형 | 첫 구현 경로 | 첫 참조 담당 경로 | 유용한 테스트 계층 | 확인할 개발자 설명 |
|---|---|---|---|---|
| 공유 요청 또는 값 타입 | `crates/volicord-types/src/methods.rs`, `schema.rs`, `values.rs`, `ids.rs`, `canonical.rs` | API 스키마 담당 문서와 [값 집합](../reference/api/schema-value-sets.md), 메서드별 의미는 메서드 담당 문서 | `volicord-types` 단위 테스트. 형태가 메서드 계획이나 어댑터 노출에 영향을 주면 Core 또는 MCP 테스트 | [코드베이스 둘러보기](codebase-tour.md), [구현 설계 패턴](design-patterns.md), [테스트 전략](testing-strategy.md) |
| Store 동작 | `crates/volicord-store/src/core_pipeline.rs`, `core_pipeline/mutation_apply.rs`, `sqlite.rs`, `migrations.rs`, `bootstrap.rs`, `artifacts.rs` | [저장소](../reference/storage.md), [저장 효과](../reference/storage-effects.md), [저장소 기록](../reference/storage-records.md), [저장소 DDL](../reference/storage-ddl.md), [아티팩트 저장소](../reference/storage-artifacts.md), [저장소 버전 관리](../reference/storage-versioning.md) | Store 단위 테스트. 공개 효과는 Core 메서드 테스트, 계층 간 동작은 적합성 또는 MCP 통합 테스트 | [저장소와 트랜잭션](storage-and-transactions.md), [구현 아키텍처](architecture.md), 결정 기록 |
| Core 메서드 동작 | `crates/volicord-core/src/methods/`, `pipeline.rs`, `policy/` | [API 메서드](../reference/api/methods.md)에서 연결된 메서드 담당 문서. 닿은 영역에 따라 스키마, 오류, 저장소, Core 모델, 보안 담당 문서 추가 | `crates/volicord-core/src/methods/tests.rs`, 파이프라인 테스트, 교차 메서드 기준 범위 시나리오는 적합성 테스트 | [요청 생명주기](request-lifecycle.md), [구현 설계 패턴](design-patterns.md), [저장소와 트랜잭션](storage-and-transactions.md) |
| MCP 어댑터 동작 | `crates/volicord-mcp/src/lib.rs`, `adapter.rs`, `routing.rs`, `tool_registry.rs`, `stdio.rs`, `local_http.rs`, `local_web_consent.rs`, `crates/volicord-cli/src/main.rs`의 `volicord mcp` 디스패치 | [MCP 전송](../reference/mcp-transport.md), 검증된 연결 맥락은 [Agent Connection](../reference/agent-connection.md), 공개 도구 집합은 [API 메서드](../reference/api/methods.md) | `crates/volicord-mcp/src/tests.rs`, `mcp_transport`, `tests/integration/mcp_connection.rs` | [요청 생명주기](request-lifecycle.md), [아키텍처 결정](decisions/README.md), [테스트 전략](testing-strategy.md) |
| 관리 에이전트 설정 동작 | `crates/volicord-cli/src/connection_command.rs`, `host_integration/`, `registration.rs` | [관리 CLI](../reference/admin-cli.md), 인접 관심사는 [Agent Connection](../reference/agent-connection.md), [런타임 경계](../reference/runtime-boundaries.md), [MCP 전송](../reference/mcp-transport.md) | `binary_admin`, 부트스트랩/검사/registry/마이그레이션 동작은 Store 설정 테스트 | [구현 아키텍처](architecture.md), [Runtime Home과 Product Repository 분리](decisions/runtime-home-and-product-repository.md) |
| 테스트 픽스처 동작 | `crates/volicord-test-support/src/lib.rs`, `tests/conformance/`, `tests/integration/`, 구현 모듈 안의 테스트 도우미 | 각 주장 사실의 담당 문서. [적합성](../reference/conformance.md)은 적합성 시나리오 의미와 주장 경로만 담당 | 소비 패키지의 테스트와 집중 픽스처 테스트 | [테스트 전략](testing-strategy.md), [코드베이스 둘러보기](codebase-tour.md) |
| 개발자 문서만 바뀐 경우 | `docs/en/development/`, `docs/ko/development/`, 경로 메타데이터 | 개발자 페이지의 `doc-index.yaml` 담당 범위. 정확한 동작이 바뀔 때만 참조 담당 문서 | 문서 점검. Cargo 명령은 요청되었거나 소스 검증이 필요할 때만 | 대응 페이지, [개발자 문서](README.md), `docs/doc-index.yaml` |

## 불일치 처리

구현과 문서가 어긋나 보이면 편집하기 전에 불일치의 종류를 분류합니다.

- 가이드 수준 소스 구조 설명이 안정적인 코드와 다르면 그 설명을 담당하는
  개발자 학습 페이지를 고칩니다.
- 코드가 API, 스키마, 저장소, 보안, 오류, 범위, 런타임, Core 권한 담당
  문서와 다르면 코드를 새 계약으로 취급하지 않습니다.
- 테스트, 픽스처, 예시, 적합성 시나리오 산문만 동작을 표현한다면 담당 문서
  공백으로 다룹니다.
- 담당 문서를 식별할 수 없으면 제품 규칙을 이 가이드에 넣지 말고 담당
  문서 공백을 보고합니다.

불일치 자체에서 제품 결정을 추론하지 않습니다. 담당 경로가 결정이 어디에
속하는지 알려 줍니다.

## 완료 점검

이 목록은 구현과 문서 유지보수 점검입니다. 제품 수락, 런타임 적합성,
닫기 준비 상태, QA 완료, 보안 증명, 잔여 위험 수락이 아닙니다.

- 변경된 각 동작에 집중 담당 문서가 있거나 담당 문서 공백 보고가 있습니다.
- 편집 전에 구현 경로와 경계를 식별했습니다.
- 변경된 계층에 맞는 테스트를 골랐습니다.
- 오래 유지될 소스 구조, 실행 흐름, 저장소 경계, 테스트 전략이 바뀌었을 때
  개발자 학습 문서를 갱신했습니다.
- 유지되는 문서가 바뀌었을 때 영어와 한국어 문서가 의미상 맞게 남았습니다.
- 스크래치 메모, 생성된 보고서, 런타임 홈, SQLite 파일, 픽스처 출력, 로그,
  그 밖의 부수 파일이 유지 문서에 남아 있지 않습니다.
