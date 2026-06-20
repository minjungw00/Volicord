# 구현 가이드

이 가이드는 구현자가 구현 변경을 분류하고, 적용되는 계약 담당 문서를 찾고, 구현 경계를 확인하고, 검증을 고를 수 있게 돕습니다. 제품 의미는 기준 참조 담당 문서에 남습니다.

이 문서는 가이드 수준의 읽기 경로입니다. 기준 범위, API 동작, 스키마, 저장 효과, 보안 보장, 런타임 경계, 오류 동작, 닫기 준비 상태 규칙, 커넥터 동작, 적합성 권한, Core 권한 의미를 정의하거나 덮어쓰지 않습니다. 기계가 읽는 정확한 담당 경로는 [`docs/doc-index.yaml`](../../doc-index.yaml)을 사용하고, 사람이 읽는 담당 문서 안내는 [참조 색인](../reference/README.md)을 사용합니다.

하네스는 AI 지원 제품 작업을 위한 로컬 작업 권한 제품이자 시스템입니다. Core는 하네스 상태를 위한 로컬 기준 기록입니다.

## 구현 변경 분류

가장 가까운 행에서 시작하고, 변경이 API, 저장소, 런타임, 보안, Core 권한 경계를 건너면 인접 담당 문서를 추가합니다. 이 표는 경로 안내일 뿐이며 전체 담당 문서 등록부가 아닙니다.

| 변경 유형 | 첫 계약 담당 경로 | 아키텍처 또는 코드 경로 | 유용한 검증 위치 |
|---|---|---|---|
| 공개 API 메서드 구현 | [범위](../reference/scope.md), 그다음 [API 메서드](../reference/api/methods.md)와 연결된 메서드 담당 문서. 스키마, 오류, 저장소, 보안 관심사가 있으면 해당 담당 문서를 추가합니다. | [구현 아키텍처](architecture.md)의 Core 파이프라인, Store 경계, 효과 경로, 코드에서 담당 문서로 가는 경로 절. `crates/harness-core/src/methods/`, `crates/harness-core/src/pipeline.rs`, `crates/harness-core/src/policy/`. | `crates/harness-core/src/methods/tests.rs`의 메서드와 Core 테스트, `tests/conformance/baseline.rs`, 어댑터 노출이 영향을 받을 때 `tests/integration/mcp_surface.rs`. |
| 공통 Core 파이프라인 또는 Core 정책 | [Core 모델](../reference/core-model.md), [API 코어 스키마](../reference/api/schema-core.md), [API 오류](../reference/api/errors.md), 지속 효과가 있으면 [저장 효과](../reference/storage-effects.md). 접근이나 보장 표현이 걸리면 [에이전트 통합](../reference/agent-integration.md)이나 [보안](../reference/security.md)을 추가합니다. | 구현 아키텍처의 Core 파이프라인과 Store 경계, 효과와 커밋 경계, 구현 불변식 절. `crates/harness-core/src/pipeline.rs`, `crates/harness-core/src/policy/`. | Core 메서드 테스트, 변경된 담당 문서 사실을 주장하는 적합성 시나리오, MCP 또는 Store 경계가 영향을 받을 때 통합 테스트. |
| 공유 타입, 스키마 표현, 식별자, 값 집합 | API 스키마 담당 문서 묶음: [API 코어 스키마](../reference/api/schema-core.md), [상태 스키마](../reference/api/schema-state.md), [아티팩트 스키마](../reference/api/schema-artifacts.md), [판단 스키마](../reference/api/schema-judgment.md), [값 집합](../reference/api/schema-value-sets.md). 메서드별 요청이나 결과 의미는 메서드 담당 문서를 사용합니다. | [구현 아키텍처](architecture.md)의 워크스페이스 형태와 소스 모듈 지도. `crates/harness-types/src/methods.rs`, `schema.rs`, `values.rs`, `ids.rs`, `canonical.rs`. | 타입과 직렬화 단위 테스트, 해당 형태를 쓰는 메서드 테스트, 담당 문서가 정의한 값 동작의 적합성 범위. |
| 저장 효과, 기록, 트랜잭션, 마이그레이션 | 저장소 담당 문서 묶음: [저장소](../reference/storage.md), [저장 효과](../reference/storage-effects.md), [저장소 기록](../reference/storage-records.md), [저장소 DDL](../reference/storage-ddl.md), [아티팩트 저장소](../reference/storage-artifacts.md), [저장소 버전 관리](../reference/storage-versioning.md). | 구현 아키텍처의 Store 경계, 효과와 커밋 경계, 소스 모듈 지도. `crates/harness-store/src/`, 특히 `core_pipeline.rs`, `migrations.rs`, `sqlite.rs`, `artifacts.rs`. | Store 단위 테스트, 커밋된 효과를 다루는 Core 메서드 테스트, `tests/conformance/baseline.rs`, 계층 간 저장 효과를 다룰 때 `tests/integration/mcp_surface.rs`. |
| MCP 시작, 바인딩, 전송, 도구 디스패치 | [MCP 전송](../reference/mcp-transport.md). 검증된 접점 맥락은 [에이전트 통합](../reference/agent-integration.md)을 추가하고, 도구로 노출되는 지원 공개 메서드 집합은 [API 메서드](../reference/api/methods.md)를 확인합니다. | 구현 아키텍처의 운영 경로와 MCP/Core 실행 흐름. `crates/harness-mcp/src/lib.rs`, `crates/harness-mcp/src/main.rs`. | `crates/harness-mcp/tests/binary_transport.rs`, `tests/integration/mcp_surface.rs`. |
| 관리 CLI 설정, 등록, 호스트 설정 | [관리 CLI](../reference/admin-cli.md). Runtime Home, Product Repository, 프로세스, 호스트 설정 경계에는 [런타임 경계](../reference/runtime-boundaries.md)와 [MCP 전송](../reference/mcp-transport.md)을 추가합니다. | 구현 아키텍처의 관리 CLI 설정 흐름. `crates/harness-cli/src/`, 특히 `local_mcp_command.rs`, `setup.rs`, `wizard.rs`, `host_config.rs`, `registration.rs`. | `crates/harness-cli/tests/binary_admin.rs`, 부트스트랩이나 마이그레이션 동작이 영향을 받을 때 Store 설정 테스트. |
| 테스트, 픽스처, 테스트 지원 기능 | 각 주장 사실의 담당 문서. [적합성](../reference/conformance.md)은 문서 수준 적합성 시나리오 의미와 주장 경로만 담당합니다. | 구현 아키텍처의 테스트 구조. `crates/harness-test-support/`, `tests/conformance/`, `tests/integration/`, 구현 크레이트 안의 테스트. | 변경된 테스트 패키지나 크레이트 테스트, 테스트가 빠진 계약 담당 문서를 드러낼 때 담당 문서 중심 문서 점검. |

## 기본 구현 읽기 순서

변경에 더 좁은 담당 경로가 이미 정해져 있지 않다면 아래 순서를 사용합니다.

1. [범위](../reference/scope.md)에서 지원 범위를 확인합니다.
2. 계약 질문마다 [`docs/doc-index.yaml`](../../doc-index.yaml)에서 기준 담당 문서를 찾습니다.
3. 정확한 의미는 집중 참조 담당 문서에서 읽습니다.
4. [구현 아키텍처](architecture.md)에서 구현 경계와 실행 흐름을 찾습니다.
5. 관련 소스와 테스트를 확인합니다.
6. 코드, 테스트, 문서를 담당 문서가 정의한 계약과 비교합니다.
7. 변경된 계층에 맞는 검증을 실행합니다.

하나의 구현 변경이 둘 이상의 담당 문서를 필요로 할 수 있습니다. 예를 들어 메서드 변경은 메서드 동작, 스키마 형태, 저장 효과, 런타임 경계, 오류 처리 경로, 보안 표현, 적합성 주장을 함께 건드릴 수 있습니다. 각 질문을 집중 담당 문서에 두고 이 가이드를 합쳐진 계약처럼 사용하지 않습니다.

## 코드와 문서의 불일치

구현과 문서가 어긋나 보이면 무엇을 고칠지 결정하기 전에 불일치의 종류를 분류합니다.

- 가이드 수준의 소스 구조 설명이 현재 안정적인 코드와 다르면 [구현 아키텍처](architecture.md)를 고쳐 구현 구조와 맞춥니다.
- 코드가 API, 스키마, 저장소, 보안, 오류, 범위, Core 권한 담당 문서와 다르면 코드를 새 계약으로 취급하지 않습니다.
- 제품 의미의 차이는 적용되는 담당 문서와 구현을 통해 해결합니다. 경로 문서, README, 사용 문서, 이 가이드에서 해결하지 않습니다.
- 테스트, 픽스처, 예시, 적합성 시나리오 산문만 동작을 표현한다면 계약 담당 문서 공백으로 다룹니다.
- 담당 문서를 식별할 수 없으면 계약을 이 가이드에 넣지 말고 담당 문서 공백을 보고합니다.

불일치 자체에서 제품 결정을 추론하지 않습니다. 담당 경로는 그 결정이 어디에 속하는지 알려 줍니다.

## 계약 담당 문서가 아닌 입력

아래 입력은 구현할 때 유용하지만 제품 계약을 정의하지 않습니다.

| 입력 | 정당한 사용 | 담당 문서 경계 |
|---|---|---|
| [사용자 가이드](../use/user-guide.md), [에이전트 가이드](../use/agent-guide.md), [판단 예시](../use/judgment-examples.md), [접점별 사용 레시피](../use/surface-recipes.md) 같은 사용 문서 | 워크플로 의도, 독자 판단, 커넥터 맥락, 접점 기대를 이해합니다. | API 페이로드, 저장 효과, 접근 경계, 보안 보장, 닫기 준비 상태 규칙, 오류 동작은 참조 담당 문서로 돌아갑니다. |
| 예시 | 대표 분기, 간결한 형태, 시나리오를 이해합니다. | 예시는 완전한 스키마, 값 집합 정의, 저장 효과 정의, 구현 지름길이 아닙니다. |
| 적합성 시나리오 | 점검 범위 입력과 주장 경로를 확인합니다. | 시나리오 산문과 시나리오 ID는 주장되는 제품 사실을 담당하지 않습니다. 그 사실은 범위 문서나 집중 참조 담당 문서로 갑니다. |
| 테스트, 픽스처, 테스트 지원 도우미 | 담당 문서가 정의한 동작을 검증하고, 폐기 가능한 Runtime Home 상태를 구성하며, 계층 간 경로를 실행합니다. | 테스트 단언, 픽스처 형태, 도우미 API가 제품 계약의 유일한 출처가 되면 안 됩니다. |
| 생성된 출력, 로그, 렌더링된 보고서, 현재 구현 동작 | 동작을 진단하고 관찰된 구현을 담당 문서와 비교합니다. | 런타임 출력과 관찰된 코드 동작은 API, 저장소, 보안, Core 권한, 적합성 계약이 되지 않으며, 생성물이나 부수 파일은 유지 문서에 두지 않습니다. |

## 구현 완료 점검

이 목록은 구현과 문서 유지보수 점검입니다. 제품 수락, 런타임 적합성, 닫기 준비 상태, QA 완료, 보안 증명, 잔여 위험 수락이 아닙니다.

- 변경된 각 동작에 대해 범위와 기준 담당 문서 또는 담당 문서 묶음을 식별했습니다.
- [구현 아키텍처](architecture.md)를 통해 아키텍처 경계와 코드 영역을 식별했습니다.
- 코드, 테스트, 문서가 담당 문서가 정의한 계약과 맞거나, 담당 문서 공백을 보고했습니다.
- 제품 의미가 바뀌었을 때 대응 영어와 한국어 문서를 함께 갱신했습니다.
- 계층에 맞는 테스트나 문서 점검을 실행했거나, 건너뛴 이유를 남겼습니다.
- 코드, 테스트, 픽스처, 예시, 생성된 출력, 이 가이드에만 정의된 동작이 없습니다.
- 스크래치 메모, 생성된 보고서, 런타임 홈, SQLite 파일, 픽스처 출력, 로그, 그 밖의 부수 파일을 유지 문서에 남기지 않았습니다.
