# 코드베이스 둘러보기

이 문서는 Volicord Rust 워크스페이스를 배우는 유지보수자를 위한 읽기
안내입니다. 코드를 읽는 순서와 각 크레이트의 역할을 설명합니다. 오래 유지되는
진입점 심볼과 흐름도 함께 제시합니다.

이 문서는 소스 담당 지도가 아닙니다. 정확한 소스 경로 책임과 모듈 배치는
[소스 지도](source-map.md)를 사용합니다. 워크스페이스 형태, 의존 경계 개요,
상위 런타임 지도는 [구현 아키텍처](architecture.md), 로컬 관리 실행 흐름은
[CLI 작업 흐름](cli-workflows.md), 대표 MCP에서 Core와 Store로 이어지는 메서드 추적은
[요청 생명주기](request-lifecycle.md), 커밋과 아티팩트 경계는
[저장소와 트랜잭션](storage-and-transactions.md), 테스트 계층 선택은
[테스트 전략](testing-strategy.md), 변경을 시작할 때는
[구현 가이드](change-guide.md)를 사용합니다.

정확한 API 동작, 요청과 응답 스키마, 저장 효과, 보안 표현, 런타임 경계, 오류
의미, Core 권한 의미는 집중 [참조 색인](../reference/README.md) 담당 문서에
남습니다.

아래 코드와 테스트 경로는 저장소 루트 기준입니다.

## 추천 읽기 순서

공개 메서드 실행을 처음 훑을 때는 아래 순서로 코드를 읽습니다.

1. `volicord-types`: 다른 크레이트가 공유하는 형식화된 요청, 응답, 값,
   식별자, 정규화된 해시 형태를 익힙니다.
2. `volicord-mcp`: MCP `tools/call`이 형식화된 요청과 Core 호출로 바뀌는
   흐름을 따라갑니다.
3. `volicord-core`: 공유 사전 점검, 메서드 계획, 분기 선택, 응답 구성을
   따라갑니다.
4. `volicord-store`: 프로젝트 Store 읽기, `CoreStorageMutation` 값, 일반
   커밋, 재실행, 아티팩트 경계를 따라갑니다.
5. `tests/integration`과 `tests/conformance`: MCP/Core/Store를 가로지르는
   경로와 기준 메서드 시나리오가 어떻게 실행되는지 봅니다.

로컬 운영자와 설정 동작은 `volicord-store` 뒤에 `volicord-cli`로 갈라져 읽고,
그다음 [CLI 작업 흐름](cli-workflows.md)을 봅니다. CLI 경로는 로컬 관리
작업을 조율하며 공개 Core 메서드 동작의 다른 구현이 아닙니다.

저장소 질문은 [저장소와 트랜잭션](storage-and-transactions.md)을 옆에 두고
`volicord-store`를 읽습니다. 정확한 기록, DDL, 아티팩트, 저장 효과 의미가
필요하면 [참조 색인](../reference/README.md)의 저장소 참조 담당 문서로
이동합니다.

변경 작업에서는 이 문서를 최종 경로 지정 권한으로 사용하지 않습니다. 이
문서로 방향을 잡고, 정확한 경로는 [소스 지도](source-map.md)에서 찾으며,
검증 계층은 [테스트 전략](testing-strategy.md)에서 고르고, 담당 문서와 완료
점검은 [구현 가이드](change-guide.md)로 확인합니다.

## 워크스페이스 이해 모델

의존 관계를 간단히 정리하면 아래와 같습니다.

- `volicord-types`는 공유 타입 경계에 있습니다.
- `volicord-store`는 공유 타입을 사용해 Runtime Home과 프로젝트 Store
  메커니즘을 관리합니다.
- `volicord-core`는 공유 타입과 Store를 사용해 어댑터 독립 메서드 처리를
  구현합니다.
- `volicord-mcp`와 `volicord-cli`는 Core와 Store 주변의 어댑터이자 로컬
  관리 진입점입니다.
- `volicord-platform-fs`는 로컬 어댑터에 좁은 범위의 안전한 플랫폼
  파일시스템 기본 연산을 제공하는 내부 의존성 말단입니다. 제품 정책, 검증,
  정리, 복구, 진단은 호출자가 계속 담당합니다.
- `volicord-test-support`, `tests/integration`, `tests/conformance`는 폐기
  가능한 검증을 위해 크레이트를 조합합니다.
- `xtask`는 저장소 유지보수 도구이며 제품 런타임 아키텍처와 분리됩니다.

워크스페이스 형태와 의존 경계 개요는 [구현 아키텍처](architecture.md)를
사용합니다. 정확한 Cargo 의존 관계는 워크스페이스와 각 크레이트의
`Cargo.toml` 매니페스트에서 확인합니다. 정확한 소스 배치는
[소스 지도](source-map.md)를 사용합니다.

## `crates/volicord-types`

어댑터, Core, Store, 테스트가 공유하는 데이터 형태를 이해해야 할 때 여기서
시작합니다. 이 크레이트는 형식화된 요청과 결과, 스키마 형태 구조체, 정해진
Rust 값, 불투명 식별자, MCP 노출 도구 이름, 정규화된 요청 해시의 경계입니다.

먼저 [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs)를
열고, 이어서 아래 앵커를 따라갑니다.

- [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs):
  공개 요청과 결과 구조체, `MethodOperationCategory`, `public_request_schema`.
- [`crates/volicord-types/src/schema.rs`](../../../crates/volicord-types/src/schema.rs):
  `ToolEnvelope`, `StateSummary`, `EvidenceSummary`, `ArtifactRef` 같은 공유
  요청 래퍼, 응답, 상태, 아티팩트, 판단, 표시 형태.
- [`crates/volicord-types/src/values.rs`](../../../crates/volicord-types/src/values.rs):
  `MethodName`, `OperationCategory`, `EffectKind`, `ResponseKind`, `ErrorCode`
  같은 정해진 값.
- [`crates/volicord-types/src/ids.rs`](../../../crates/volicord-types/src/ids.rs)와
  [`crates/volicord-types/src/canonical.rs`](../../../crates/volicord-types/src/canonical.rs):
  불투명 ID, `DurableIdGenerator`, `canonical_request_hash`.

테스트를 읽을 때는 `crates/volicord-types/src/lib.rs`의 타입 형태와 정규화된
해시 테스트에서 시작합니다. 그다음 MCP 인자에서 형식화된 요청이 만들어지는 모습을
보려면 `volicord-mcp`로, 요청이 계획되는 모습을 보려면 `volicord-core`로
이동합니다.

## `crates/volicord-mcp`

로컬 MCP 호스트가 Core에 도달하는 경로를 보려면 `volicord-mcp`를 읽습니다.
이 어댑터는 공개 도구를 등록하고, 시작/세션 맥락을 검증하고, `tools/call`
인자를 디코딩하고, 연결된 로컬 Agent Connection에서 호출 사실을
파생합니다. Core를 호출한 뒤에는 Core JSON을 MCP `content`에 담습니다.

크레이트 표면은 [`crates/volicord-mcp/src/lib.rs`](../../../crates/volicord-mcp/src/lib.rs)에서
보고, 그다음 아래 경로를 따라갑니다.

1. [`crates/volicord-mcp/src/tool_registry.rs`](../../../crates/volicord-mcp/src/tool_registry.rs):
   공개 도구 목록과 `PUBLIC_METHOD_TOOL_NAMES`.
2. [`crates/volicord-mcp/src/routing.rs`](../../../crates/volicord-mcp/src/routing.rs):
   시작 검사, 연결 맥락, 프로젝트 허용 목록, 요청 시점 프로젝트 선택.
3. [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs):
   `McpAdapter::call_tool`, 형식화된 디코딩, 생성된 요청 래퍼 사실,
   `McpDerivedInvocationContext::core_invocation`.
4. [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs):
   JSON-RPC 표준 입출력 디스패치, 사전 점검, 응답 래핑.
5. [`crates/volicord-mcp/src/local_http.rs`](../../../crates/volicord-mcp/src/local_http.rs)와
   [`crates/volicord-mcp/src/http.rs`](../../../crates/volicord-mcp/src/http.rs):
   로컬 HTTP 수신기, 세션 처리 경로, 공통 HTTP 파싱과 응답 도우미.

경계를 분리해서 읽어야 합니다. MCP 시작과 요청 처리 중 Store를 직접 읽는 것은 공개
메서드 디스패치 전의 검증과 선택 작업입니다. 공개 메서드 의미는 계속
`volicord-core`를 통합니다. 대표 호출 추적은 [요청 생명주기](request-lifecycle.md),
정확한 MCP 전송 동작은 [MCP 전송](../reference/mcp-transport.md)을 사용합니다.

## `crates/volicord-core`

어댑터와 독립적인 메서드 경로를 보려면 `volicord-core`를 읽습니다. Core는 공유
사전 점검, Store 열기, 재실행 점검, 메서드별 계획, 분기 선택, 응답 구성을
조율합니다.

[`crates/volicord-core/src/lib.rs`](../../../crates/volicord-core/src/lib.rs)를
열고, 이어서 [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)를
봅니다. 따라갈 주요 심볼은 `CoreService`, `InvocationContext`,
`MethodPolicy`, `PreparedRequest`, `OwnerPipelineBranch`,
`CoreService::prepare_request`,
`CoreService::execute_prepared_request`입니다.

파이프라인 뒤에는
[`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/)에서
메서드 모듈 하나를 고릅니다.

- `status.rs`는 읽기 전용 분기를 보여 줍니다.
- `intake.rs`는 계획된 커밋 변이 분기를 보여 줍니다.
- `prepare_write.rs`는 정책이 많은 계획과 쓰기 티켓 결정을 보여 줍니다.
- `record_run.rs`, `judgment.rs`, `reconcile_changes.rs`, `close_task.rs`는
  정확한 메서드 계약을 Core 산문으로 옮기지 않으면서 뒤쪽 작업 흐름 사실이
  어떻게 계획되는지 보여 줍니다.

[`crates/volicord-core/src/policy/`](../../../crates/volicord-core/src/policy/)
아래의 재사용 정책 도우미는 메서드 하나를 이해한 뒤 읽으면 좋습니다. 반복
구조는 [구현 설계 패턴](design-patterns.md), 추적 예시는
[요청 생명주기](request-lifecycle.md)를 사용합니다.

테스트는 분기와 사전 점검 경계를 다루는 `crates/volicord-core/src/pipeline.rs`에서
시작한 뒤, 메서드 계획과 Store에 보이는 효과를 다루는
`crates/volicord-core/src/methods/tests/` 아래의 메서드별 파일로 이동합니다.

## `crates/volicord-store`

런타임 데이터와 트랜잭션 메커니즘이 필요할 때 `volicord-store`를 읽습니다.
Store는 Runtime Home 경로 처리, 레지스트리와 프로젝트 데이터베이스 설정, 스키마
검증, 프로젝트 Store 읽기, 일반 Core 변이 커밋, 재실행 행, 아티팩트 스테이징,
검사, 저장소 오류 처리를 관리합니다.

[`crates/volicord-store/src/lib.rs`](../../../crates/volicord-store/src/lib.rs)를
열고, 질문에 맞는 경로를 따라갑니다.

- 설정과 로컬 등록:
  [`runtime_home.rs`](../../../crates/volicord-store/src/runtime_home.rs),
  [`bootstrap.rs`](../../../crates/volicord-store/src/bootstrap.rs),
  [`agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs).
- SQLite 형태와 검증:
  [`sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs),
  [`schema.rs`](../../../crates/volicord-store/src/schema.rs),
  [`schema/`](../../../crates/volicord-store/src/schema/).
- Core 쪽 Store 작업:
  [`core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)에서
  `CoreProjectStore`, `CoreStorageMutation`, `CommitMutationInput`,
  `MutationCommitOutcome`, `CoreProjectStore::commit_mutation`.
- 아티팩트 작업:
  [`artifacts.rs`](../../../crates/volicord-store/src/artifacts.rs)의 스테이징과
  영속 본문 검증 도우미.
- 읽기 전용 설정과 진단 보기:
  [`inspection.rs`](../../../crates/volicord-store/src/inspection.rs).

커밋 경로를 읽을 때는 [저장소와 트랜잭션](storage-and-transactions.md)을 함께
사용합니다. 그 문서는 계획과 변이의 분리, 원자적 커밋 경계, 재실행, 상태 버전,
아티팩트, 실패 경계를 가이드 수준으로 설명합니다. 정확한 Store 하위 모듈 지도는
[소스 지도](source-map.md)를 사용합니다.

## `crates/volicord-platform-fs`

로컬 어댑터가 플랫폼 고유 파일시스템 이름 공간 경계에 도달할 때만
`volicord-platform-fs`를 읽습니다. 안전한 결과 타입과 좁은 운영체제 고유 연산
파사드는
[`crates/volicord-platform-fs/src/lib.rs`](../../../crates/volicord-platform-fs/src/lib.rs)에서
확인합니다. 대상 스냅샷, 소유권 규칙, 연산 후 검증, 정리, 복구, 진단은 다시
호출 어댑터에서 확인해야 합니다. 현재 `guard` 통합 경로의 호출자는
[`crates/volicord-cli/src/guard_integration/files.rs`](../../../crates/volicord-cli/src/guard_integration/files.rs)입니다.

플랫폼 파사드는 범용 파일시스템 추상화가 아니며 관리 동작을 담당하지 않습니다.
정확한 관리 파일 계약과 전제 조건은 [관리 CLI](../reference/admin-cli.md),
[런타임 경계](../reference/runtime-boundaries.md),
[시스템 요구사항](../reference/system-requirements.md)을 사용합니다.

## `crates/volicord-cli`

설치 프로필 설정, 프로젝트 감지, Agent Connection 등록, 호스트 어댑터 계획,
`guard` 통합, 연결 상태와 검증, 진단, 권한 번들 내보내기, User Channel 명령,
공개 `volicord mcp` 프로세스 모드 인계를
읽어야 할 때 `volicord-cli`를 봅니다.

[`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs)에서
`run_cli`와 프로세스 디스패치를 먼저 봅니다. 그다음 살펴볼 작업 흐름을 고릅니다.

- 설정 작업 흐름: [`setup_command.rs`](../../../crates/volicord-cli/src/setup_command.rs)와
  [`setup_command/`](../../../crates/volicord-cli/src/setup_command/)의
  `run_setup_command`, `run_setup_workflow`.
- Agent Connection 구성과 검증:
  [`connection_command.rs`](../../../crates/volicord-cli/src/connection_command.rs)와
  [`connection_command/`](../../../crates/volicord-cli/src/connection_command/) 아래의
  `run_init_command`, `run_connection_command`, `provision_connection`,
  `select_connection`, `verify_connection`, 렌더링 경로.
- `guard` 훅 생명주기:
  [`guard_command.rs`](../../../crates/volicord-cli/src/guard_command.rs)와
  [`guard_command/`](../../../crates/volicord-cli/src/guard_command/) 아래의
  `run_guard_command`, `guard_envelope`, `tool_observation`,
  `handle_prompt_capture`, `render_guard_output`.
- 호스트와 `guard` 통합:
  [`host_integration/`](../../../crates/volicord-cli/src/host_integration/)와
  [`guard_integration/`](../../../crates/volicord-cli/src/guard_integration/) 아래의
  `HostKind`, `HostAdapter`, `plan_guard_integration`,
  `apply_guard_integration`.
- User Channel 명령:
  [`user_command.rs`](../../../crates/volicord-cli/src/user_command.rs).

흩어진 CLI 모듈만 보고 추론하기 전에 [CLI 작업 흐름](cli-workflows.md)을
읽습니다. 그 문서는 아키텍처 수준 실행 흐름 경계를 담당합니다. 정확한 명령
계약은 [관리 CLI](../reference/admin-cli.md)에 남습니다.

테스트는 바이너리에서 보이는 관리 작업 흐름을 다루는
`crates/volicord-cli/tests/binary_admin.rs`, 훅 생명주기 동작을 다루는
`guard_command.rs`, 프로세스 전송 경로를 다루는 `mcp_transport.rs` 또는
`serve_transport.rs`에서 시작합니다.

## `crates/volicord-test-support`

테스트 설정이 어렵게 느껴질 때 `volicord-test-support`를 읽습니다. 이 크레이트는
폐기 가능한 Runtime Home 픽스처, 등록된 프로젝트와 Agent Connection 설정,
Core 요청 빌더, Store 검사 도우미, 공유 픽스처 유틸리티를 제공합니다.

[`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs)를
열고 `disposable_runtime_home`, `TempRuntimeHome`, `CoreFixture`, 메서드 요청
빌더를 봅니다. 이 도우미는 테스트 조합으로 다루며, 프로덕션 동작이나 제품
계약 담당으로 다루지 않습니다.

픽스처 변경에 Core, CLI, 통합, 적합성 패키지의 소비자 테스트가
필요한지는 [테스트 전략](testing-strategy.md)으로 판단합니다.

## `tests/integration`

계층을 가로지르는 MCP 관점이 필요할 때 `tests/integration`을 읽습니다. 주요
시작점은
[`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)입니다.
이 파일은 대표 호출을 통해 MCP, Core, Store, Agent Connection 바인딩,
프로젝트 선택, `operation_category` 처리 경로, 응답 일치, 효과 없음 점검을
조합합니다.

이 테스트는 계층이 어떻게 조합되는지 이해할 때 사용합니다. 공개 메서드 계약,
MCP 전송 계약, Store 계약, Core 권한 의미의 담당 문서로 다루지 않습니다.

## `tests/conformance`

Core 쪽 API를 통한 기준 교차 메서드 시나리오를 보려면 `tests/conformance`를
읽습니다. Core 메서드 테스트 하나를 읽은 뒤
[`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)에서
시작합니다.

이 패키지는 재실행, 쓰기 티켓, 아티팩트 생명주기, 판단 경로, 닫기 준비 상태
점검, 오류 처리 경로, 손상 처리를 메서드 사이에서 볼 때 유용합니다. 제품 의미는
계속 집중 참조 담당 문서로 보냅니다.

## `xtask`

문서 검증을 유지보수할 때만 `xtask`를 읽습니다. `cargo run -p xtask -- docs-check`
같은 읽기 전용 문서 점검을 위한 저장소 유지보수 패키지입니다.

[`xtask/src/lib.rs`](../../../xtask/src/lib.rs)를 열고, 이어서
[`xtask/src/main.rs`](../../../xtask/src/main.rs)를 봅니다.
[`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs)의 테스트는
메타데이터, 한영 경로 포함 여부, 링크, 앵커, 명령 예시, 용어 역할, 용어 경로,
공개 산문 표현을 점검하기 위해 작은 픽스처 트리를 사용합니다.

자동 구조 점검과 사람이 하는 의미 검토를 구분하고 명령 경계를 정의하는
유지보수 정책은 [검증](../maintain/validation.md)을 사용합니다.

## 경계 기억하기

- 이 문서는 읽기 순서와 구체적인 첫 진입 앵커를 제공합니다. 정확한 소스 경로
  책임은 담당하지 않습니다. 그 목적에는 [소스 지도](source-map.md)를 사용합니다.
- Core 쪽 코드는 CLI와 MCP 어댑터 크레이트에 의존하지 않습니다.
- MCP 시작과 요청 처리는 Core 디스패치 전에 Store를 읽을 수 있습니다. 이것은 공개
  메서드 의미의 다른 구현이 아닙니다.
- `Volicord Runtime Home`과 `Product Repository`는 서로 다른 경계입니다.
- 테스트는 담당 문서가 정의한 사실을 검증하지만, 테스트와 픽스처는 제품 계약
  담당 문서가 아닙니다.
- 학습 문서는 불안정한 줄 번호가 아니라 오래 유지될 파일, 심볼, 흐름을
  이름으로 가리켜야 합니다.
