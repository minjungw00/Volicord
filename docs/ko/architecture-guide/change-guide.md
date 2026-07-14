# 구현 가이드

이 가이드는 Rust 워크스페이스에서 좁은 구현 변경을 수행하는 실용적인
흐름을 제공합니다. 제품 의미는 집중 참조 담당 문서에 남습니다. 이 문서는
기준 범위, API 동작, 스키마, 저장 효과, 보안 보장, 런타임 경계, 오류
동작, 닫기 준비 상태 규칙, 커넥터 동작, 적합성 권한, Core 권한 의미를
정의하거나 덮어쓰지 않습니다.

소스를 배우는 중이면 [아키텍처 가이드](README.md)를 사용하고, 첫 파일과
심볼은 [코드베이스 둘러보기](codebase-tour.md), 정확한 소스 경로와 모듈
책임은 [소스 지도](source-map.md), 대표 메서드 흐름은
[요청 생명주기](request-lifecycle.md), 반복 구조는
[구현 설계 패턴](design-patterns.md), Store 경계는
[저장소와 트랜잭션](storage-and-transactions.md), 테스트 계층 선택은
[테스트 전략](testing-strategy.md)을 사용합니다. 기계가 읽는 담당 경로는
[`docs/doc-index.yaml`](../../doc-index.yaml)을 사용하고, 사람이 읽는 담당 문서
안내는 [참조 색인](../reference/README.md)을 사용합니다.

Volicord는 AI 지원 제품 작업을 위한 로컬 작업 권한 기록입니다.
Core는 Volicord 상태를 위한 로컬 기준 기록입니다.

## 실용 순서

1. 요청된 변경을 분류합니다.

   변경이 공유 타입, 플랫폼 파일시스템 기본 연산, Store 동작, Core 메서드 동작,
   MCP 어댑터 동작, 설정 작업 흐름, 연결 프로비저닝, guard 훅 생명주기,
   guard 통합 파일, 호스트 어댑터, 테스트 픽스처, 아키텍처 가이드 전용 중
   어디에 닿는지 정합니다. 둘 이상의 경계를 건너면 질문을 나누어 둡니다.

2. 현재 구현 경로를 찾습니다.

   [구현 아키텍처](architecture.md)에서 상위 워크스페이스 경계를 확인하고
   [소스 지도](source-map.md)에서 정확한 소스 경로를 봅니다. 그런 다음 아래
   경로 표에서 가장 가까운 소스와 테스트를 엽니다. 편집 전에 이름 붙인
   심볼이 여전히 존재하는지 확인합니다.

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

6. 영향을 받은 아키텍처 가이드 설명을 갱신합니다.

   오래 유지될 소스 형태, 의존 방향, 실행 흐름, Store 경계가 바뀌면 관련
   아키텍처 가이드 페이지를 두 언어 모두에서 갱신합니다. 테스트 구조나 검증
   책임이 바뀌면 [테스트 전략](testing-strategy.md)을 갱신합니다. 변경 경로,
   담당 경로, 검증 명령 경로가 바뀌면 이 가이드를 갱신합니다. 정확한 제품
   계약은 참조 담당 문서에 둡니다.

7. 완료된 공개 변경 묶음마다 릴리스 버전을 한 번 평가합니다.

   서로 관련된 한 묶음의 변경이 지원되는 공개 계약이나 배포 동작을 바꾸면, 완료된
   변경 묶음 전체의 SemVer 영향을 한 번 평가하고 루트 `Cargo.toml`의
   `[workspace.package].version`을 그 묶음에 대해 한 번 갱신합니다. 묶음 안의 커밋마다
   버전을 올리지 않습니다. 모든 워크스페이스 패키지는 이 하나의 버전을 계속
   상속합니다. 태그를 만들기 전에는
   `cargo run --locked -p xtask -- release-version-check --tag vX.Y.Z`를 실행합니다.
   태그 없이 워크스페이스 상속만 점검할 때는
   `cargo run --locked -p xtask -- release-version-check`를 사용합니다. 기존
   `volicord --version`과 MCP initialize의 `serverInfo.version`은 상속된 패키지 버전에서
   값을 가져옵니다. 별도 운영 `build_id`는 소스와 컴파일 차원을 기록하며 알 수 없거나
   근사인 값은 명시적으로 표시합니다. 이 값은 두 번째 SemVer, 태그 출처, 바이너리
   다이제스트, dirty 작업 트리의 정확한 지문, 암호학적 증명이 아니며 빌드 시각을
   포함하지 않습니다. 릴리스 빌드는 체크인된 작업 흐름을 통해 clean 소스 커밋과 정확한
   프로필을 제공하고 검증합니다.

   호스트 역량 릴리스 증거에서 이 빌드 설명자는 일치 대조 좌표로만 사용합니다. 증거는
   최종 실행 파일이 생긴 뒤에 관찰되므로 예상 증거 다이제스트는 실행 파일 밖의 별도로
   검증된 정확한 최종 아티팩트 manifest 또는 receipt에 두고 역량, 호스트·클라이언트,
   어댑터, 빌드, source, target, 최종 실행 파일 다이제스트에 결속해야 합니다. 완료 뒤
   다이제스트를 내장하려고 다시 빌드하면 실행 파일 다이제스트가 바뀌어 재귀 결속이
   생기므로 그렇게 하지 않습니다. 현재 어댑터에는 이 manifest를 신뢰해 획득하는 경로가
   없습니다. 운영 local-web 자격을 주장하기 전에 구체적인 스키마와 획득 작업을 집중 참조
   담당 문서로 먼저 경로 지정합니다.

8. 검증을 실행합니다.

   Rust 구현을 편집했으면 기본적으로 `cargo fmt`,
   `cargo clippy --all-targets --all-features`,
   `cargo test --all-targets --all-features`를 실행합니다. 문서를 편집했으면
   구조, 링크/색인, 언어 일치, 용어에 맞는 Maintain 점검을 실행합니다.
   실행하지 않은 명령은 이유를 보고합니다.

9. 동작을 새로 만들지 말고 담당 문서 공백을 보고합니다.

   구현에 필요한 동작을 어떤 담당 문서도 정의하지 않는다면 제품 의미
   변경을 멈추고 담당 문서 공백을 보고하거나 적절한 참조 담당 문서를 먼저
   갱신합니다. README, 가이드, 테스트, 픽스처, 어댑터, 생성된 출력,
   구현 주석으로 그 공백을 메우지 않습니다.

## 변경 유형 경로

| 변경 유형 | 첫 구현 경로 | 첫 참조 담당 경로 | 유용한 테스트 계층 | 확인할 아키텍처 가이드 설명 |
|---|---|---|---|---|
| 공유 요청 또는 값 타입 | `crates/volicord-types/src/methods.rs`, `schema.rs`, `values.rs`, `ids.rs`, `canonical.rs` | API 스키마 담당 문서와 [값 집합](../reference/api/schema-value-sets.md), 메서드별 의미는 메서드 담당 문서 | `volicord-types` 단위 테스트. 형태가 메서드 계획이나 어댑터 노출에 영향을 주면 Core 또는 MCP 테스트 | [코드베이스 둘러보기](codebase-tour.md), [구현 설계 패턴](design-patterns.md), [테스트 전략](testing-strategy.md) |
| 플랫폼 파일시스템 기본 연산 또는 어댑터 관리 조건부 파일 교체 | 안전한 플랫폼 파사드는 `crates/volicord-platform-fs/src/lib.rs`, 계획, 대상 검증, 정리, 복구, 진단은 `crates/volicord-cli/src/guard_integration/files.rs` 같은 호출 어댑터 | 정확한 명령 동작은 [관리 CLI](../reference/admin-cli.md), Product Repository 파일 배치는 [런타임 경계](../reference/runtime-boundaries.md), 환경 전제 조건은 [시스템 요구사항](../reference/system-requirements.md) | `volicord-platform-fs` 단위 테스트와 호출 모듈 테스트. 바이너리에 보이면 `binary_admin`, 운영체제 고유 경로가 바뀌면 대상별 컴파일 또는 테스트 | [구현 아키텍처](architecture.md), [소스 지도](source-map.md), [CLI 작업 흐름](cli-workflows.md), [테스트 전략](testing-strategy.md) |
| Store 동작 | `crates/volicord-store/src/core_pipeline.rs`, `core_pipeline/mutation_apply.rs`, `schema.rs`, `schema/*.sql`, `sqlite.rs`, `bootstrap.rs`, `artifacts.rs` | [저장소](../reference/storage.md), [저장 효과](../reference/storage-effects.md), [저장소 기록](../reference/storage-records.md), [저장소 DDL](../reference/storage-ddl.md), [아티팩트 저장소](../reference/storage-artifacts.md), [저장소 버전 관리](../reference/storage-versioning.md) | Store 단위 테스트. 공개 효과는 Core 메서드 테스트, 계층 간 동작은 적합성 또는 MCP 통합 테스트 | [저장소와 트랜잭션](storage-and-transactions.md), [구현 아키텍처](architecture.md), 결정 기록 |
| Core 메서드 동작 | `crates/volicord-core/src/methods/`, `pipeline.rs`, `policy/` | [API 메서드](../reference/api/methods.md)에서 연결된 메서드 담당 문서. 닿은 영역에 따라 스키마, 오류, 저장소, Core 모델, 보안 담당 문서 추가 | `crates/volicord-core/src/methods/tests/` 아래의 해당 파일, 파이프라인 테스트, 교차 메서드 기준 범위 시나리오는 적합성 테스트 | [요청 생명주기](request-lifecycle.md), [구현 설계 패턴](design-patterns.md), [저장소와 트랜잭션](storage-and-transactions.md) |
| MCP 어댑터 동작 | `crates/volicord-mcp/src/lib.rs`, `adapter.rs`, `routing.rs`, `tool_registry.rs`, `stdio.rs`, `local_http.rs`, `local_web_consent.rs`, `crates/volicord-cli/src/main.rs`의 `volicord mcp` 또는 `volicord serve` 디스패치 | [MCP 전송](../reference/mcp-transport.md), 검증된 연결 맥락은 [Agent Connection](../reference/agent-connection.md), 공개 도구 집합은 [API 메서드](../reference/api/methods.md) | `crates/volicord-mcp/src/tests.rs`, `mcp_transport`, `serve_transport`, `tests/integration/mcp_connection.rs`. 생성 API 또는 MCP 도구 스키마가 바뀌면 `public_contract_snapshots` | [요청 생명주기](request-lifecycle.md), [아키텍처 결정](decisions/README.md), [테스트 전략](testing-strategy.md) |
| Runtime Home 또는 Product Repository 경계 동작 | `crates/volicord-store/src/runtime_home.rs`, `crates/volicord-store/src/bootstrap.rs`, `crates/volicord-cli/src/project_context.rs`, `crates/volicord-core/src/policy/`의 경로 관련 도우미 | [런타임 경계](../reference/runtime-boundaries.md), 인접한 지속성, 명령, 비보장 경계는 [저장소](../reference/storage.md), [관리 CLI](../reference/admin-cli.md), [보안](../reference/security.md) | Store와 CLI 모듈 테스트. 바이너리에 보이는 경계는 `binary_admin`, 담당 문서가 정의한 공개 메서드 동작이 바뀌면 Core 메서드 또는 적합성 테스트 | [구현 아키텍처](architecture.md), [소스 지도](source-map.md), [Runtime Home과 Product Repository 분리](decisions/runtime-home-and-product-repository.md) |
| 설정 작업 흐름과 출력 | `crates/volicord-cli/src/setup_command.rs`, `setup_command/workflow.rs`, `setup_command/discovery.rs`, `setup_command/linking.rs`, `setup_command/shell_startup.rs`, `setup_command/interactive.rs`, `setup_command/output.rs`. 설정 정보를 공유하는 진단은 `doctor_command.rs` | [관리 CLI](../reference/admin-cli.md), 인접한 프로세스, 위치, 비보장 경계는 [런타임 경계](../reference/runtime-boundaries.md), [MCP 전송](../reference/mcp-transport.md), [보안](../reference/security.md) | 설정 모듈 테스트와 `binary_admin`. 부트스트랩, 레지스트리, 검사, 스키마 초기화 동작이 바뀌면 Store 설정 테스트 | [구현 아키텍처](architecture.md), [코드베이스 둘러보기](codebase-tour.md), [테스트 전략](testing-strategy.md) |
| 연결 프로비저닝, 상태, 출력 | `crates/volicord-cli/src/connection_command.rs`, `connection_command/service.rs`, `selection.rs`, `verification.rs`, `mcp_process.rs`, `connection_command/output/`, `crates/volicord-store/src/bootstrap.rs`, `agent_connections.rs` | [관리 CLI](../reference/admin-cli.md), 인접 관심사는 [Agent Connection](../reference/agent-connection.md), [런타임 경계](../reference/runtime-boundaries.md), [MCP 전송](../reference/mcp-transport.md) | CLI 모듈 테스트와 `binary_admin`. 사전 점검 또는 MCP에 보이는 동작을 관찰해야 하면 `mcp_transport`나 `mcp_connection` | [구현 아키텍처](architecture.md), [Runtime Home과 Product Repository 분리](decisions/runtime-home-and-product-repository.md) |
| Guard 통합 파일, 역량 기록, 감사 정보 | `crates/volicord-cli/src/guard_integration/`. 적용과 기록은 `connection_command/service.rs`, 상태 렌더링은 `connection_command/output/`, 진단 소비는 `doctor_command.rs` | [관리 CLI](../reference/admin-cli.md#guard-hook-commands), 인접한 진단과 비보장 경계는 [런타임 경계](../reference/runtime-boundaries.md), [저장소 기록](../reference/storage-records.md), [MCP 전송](../reference/mcp-transport.md), [보안](../reference/security.md) | `binary_admin`의 guard 적용 초기화/상태 테스트, guard 통합 모듈 테스트, doctor 테스트. 감사 출력을 보안 증명이나 승인 기록으로 취급하지 않습니다 | [구현 아키텍처](architecture.md), [코드베이스 둘러보기](codebase-tour.md), [테스트 전략](testing-strategy.md) |
| Guard 훅 생명주기 동작과 호스트 고유 렌더링 | `crates/volicord-cli/src/guard_command.rs`, `guard_command/envelope.rs`, `tool_observation.rs`, `mutation.rs`, `prompt_command.rs`, `prompt_capture.rs`, `write_ticket.rs`, `render.rs`, `guard_command/phase/` | [관리 CLI](../reference/admin-cli.md#guard-hook-commands), 인접 정보는 [저장소 기록](../reference/storage-records.md), [Core 모델](../reference/core-model.md), [런타임 경계](../reference/runtime-boundaries.md), [보안](../reference/security.md) | `guard_command` 테스트. 훅 경로가 담당 문서가 정의한 Core 동작에 의존하면 Core 메서드 또는 적합성 테스트 | [구현 아키텍처](architecture.md), [코드베이스 둘러보기](codebase-tour.md), [테스트 전략](testing-strategy.md) |
| 호스트 어댑터와 호스트 기능 지원 상태 | 정적 호스트·버전 구현 사실은 `crates/volicord-types/src/host_feature_support.rs`, 동적 진단 집계는 `crates/volicord-cli/src/host_integration/`와 특히 `capability_status.rs`, MCP 전달 자격은 `crates/volicord-mcp/src/adapter.rs`, 정확한 아티팩트 주장은 `tests/release-validation`에 둡니다. 해당 사실이 바뀌면 CLI 호스트별·설정·검증·guard 경로도 포함합니다. | 평가는 [Agent Connection](../reference/agent-connection.md), 정확한 typed 값과 진단 형태는 [API 값 집합](../reference/api/schema-value-sets.md)과 [API 상태 스키마](../reference/api/schema-state.md), 명령 projection은 [관리 CLI](../reference/admin-cli.md), 어댑터 전달은 [MCP 전송](../reference/mcp-transport.md), 전제 조건은 [시스템 요구사항](../reference/system-requirements.md), 저장 capability JSON은 [저장소 기록](../reference/storage-records.md), 릴리스 주장은 [호스트 릴리스 증거](../reference/host-release-evidence.md) | 표 기반 공유 정적 평가기 테스트와 CLI 집계 테스트, 정확한 Codex 음성 MCP 테스트, 연결 상태와 Doctor projection은 `binary_admin`, 릴리스 증거는 네 실제 최종 출력 셀 모두. 관련 경로가 바뀌면 `guard_command`도 실행합니다. 설정 픽스처를 정확한 호스트 증거로 대체하면 안 됩니다. | [구현 아키텍처](architecture.md), [소스 지도](source-map.md), [테스트 전략](testing-strategy.md), [호스트 기능 지원 상태 평가](decisions/host-feature-support-state-evaluation.md) |
| 정확한 호스트 릴리스 증거 게이트 | `tests/release-validation`. 운영 crate에 넣으면 안 됩니다. | [호스트 릴리스 증거](../reference/host-release-evidence.md) | 패키지 테스트, 고정 12개 셀 게이트, 정확한 담당 명령을 쓰는 별도 프로세스 audit | [외부 호스트 릴리스 증거 게이트](decisions/host-release-evidence-gate.md), [테스트 전략](testing-strategy.md), [검증](../maintain/validation.md) |
| 테스트 픽스처 동작 | `crates/volicord-test-support/src/lib.rs`, `tests/conformance/`, `tests/integration/`, `crates/volicord-cli/tests/support/`, 구현 모듈 안의 테스트 도우미 | 각 주장 사실의 담당 문서. [적합성](../reference/conformance.md)은 적합성 시나리오 의미와 주장 경로만 담당 | 소비 패키지의 테스트와 집중 픽스처 테스트 | [테스트 전략](testing-strategy.md), [코드베이스 둘러보기](codebase-tour.md) |
| CLI 통합 테스트 지원 | `crates/volicord-cli/tests/support/assertions.rs`, `binary_fixture.rs`, `fake_hosts.rs`, `fake_mcp.rs`, `guard_fixture.rs`, `json.rs` | 설정, 연결, guard, MCP, 호스트 어댑터의 각 주장 사실을 담당하는 문서 | 도우미를 사용하는 `binary_admin`, `guard_command`, `mcp_transport` 또는 소비 CLI 테스트 대상 | [테스트 전략](testing-strategy.md), [코드베이스 둘러보기](codebase-tour.md) |
| 아키텍처 가이드만 바뀐 경우 | `docs/en/architecture-guide/`, `docs/ko/architecture-guide/`, 경로 메타데이터 | 아키텍처 가이드 페이지의 `doc-index.yaml` 담당 범위. 정확한 동작이 바뀔 때만 참조 담당 문서 | 문서 점검. Cargo 명령은 요청되었거나 소스 검증이 필요할 때만 | 대응 페이지, [아키텍처 가이드](README.md), `docs/doc-index.yaml` |

## 검증 명령 경로

영향을 받는 변경 영역을 고른 뒤에는 아래 명령을 기본 경로로 사용합니다. 이 표는
가능성이 높은 첫 검증 명령을 이름 붙이는 것이며, 작은 편집마다 인접 명령을 모두
실행해야 한다는 규칙이 아닙니다. Rust 구현을 편집했을 때의 워크스페이스 기본값은
계속 `cargo fmt`, `cargo clippy --all-targets --all-features`,
`cargo test --all-targets --all-features`입니다.

| 변경 영역 | 첫 명령 경로 | 추가할 때 |
|---|---|---|
| 아키텍처 가이드, 문서 경로, 링크, 메타데이터 | `cargo run -p xtask -- docs-check` | docs-check 동작이 바뀌면 `cargo test -p xtask`. |
| 릴리스 버전 또는 태그 릴리스 작업 흐름 | `cargo run --locked -p xtask -- release-version-check`. 제안된 릴리스 태그에는 `--tag vX.Y.Z`를 추가합니다. | 점검기가 바뀌면 `cargo test -p xtask --test release_version_check`를 추가하고, 태그 게이트나 작업 의존성이 바뀌면 `.github/workflows/release.yml`을 검토합니다. |
| 외부 정확한 최종 아티팩트 호스트 릴리스 검증 | `cargo test -p volicord-release-validation-tests`, 이어서 [호스트 릴리스 증거](../reference/host-release-evidence.md)의 정확한 `host-release-gate` 및 별도 `host-release-audit` 명령 | 바이너리, 셀 증거, 외부 호스트, 요청한 주장 중 빠진 항목은 조용히 생략하지 말고 사용할 수 없음 또는 실패로 보고합니다. |
| 공유 타입, 공개 스키마, 값 집합, 식별자, 요청 해시, 생성 공개 API/MCP 스키마 | `cargo test -p volicord-types`. 생성 스키마나 스냅샷이 영향을 받으면 `cargo test -p volicord-integration-tests --test public_contract_snapshots` | 메서드 계획이 바뀌면 Core 메서드 테스트, 어댑터에 보이는 동작이 바뀌면 MCP 통합 테스트, 유지 문서가 바뀌면 docs-check. |
| 플랫폼 파일시스템 기본 연산 또는 어댑터 관리 조건부 파일 교체 | `cargo test -p volicord-platform-fs`. `cargo test -p volicord-cli --lib guard_integration` 같은 호출 모듈 테스트 | 운영체제 고유 코드가 바뀌면 대상별 `cargo check` 또는 테스트, 관리 결과가 바이너리에 보이면 `binary_admin`, 담당 문서나 아키텍처 가이드가 바뀌면 `docs-check`. |
| Core 메서드 또는 공유 파이프라인 동작 | `cargo test -p volicord-core` | 교차 메서드 기준 범위 시나리오는 `cargo test -p volicord-conformance-tests --test baseline`, MCP에 보이는 맥락은 `cargo test -p volicord-integration-tests --test mcp_connection`. |
| Store, Storage DDL, 트랜잭션, Runtime Home, 아티팩트 저장소 동작 | `cargo test -p volicord-store`. DDL 또는 기준 SQL 변경에는 `cargo test -p volicord-store --test storage_ddl_contract` | 공개 메서드에서 보이는 저장 효과가 바뀌면 Core, 적합성, MCP 통합 테스트. |
| MCP 표준 입출력 또는 로컬 HTTP 전송, 도구 목록, 시작, 프로젝트 처리 경로 | `cargo test -p volicord-mcp`. 영향을 받는 프로세스 경로에 따라 `cargo test -p volicord-cli --test mcp_transport` 또는 `cargo test -p volicord-cli --test serve_transport` | MCP를 통해 Core/Store 동작을 관찰해야 하면 `cargo test -p volicord-integration-tests --test mcp_connection`, 생성 도구 스키마 변경은 `cargo test -p volicord-integration-tests --test public_contract_snapshots`. |
| 설정 작업 흐름, 연결 프로비저닝, 상태, 검증, 호스트 어댑터, 관리 CLI 출력 | `cargo test -p volicord-cli`. 바이너리에 보이는 동작이 바뀌면 `cargo test -p volicord-cli --test binary_admin` | 부트스트랩 또는 레지스트리 변경에는 Store 테스트, 시작 또는 사전 점검 변경에는 MCP 전송 테스트. |
| Guard 통합 파일, 역량 기록, 감사 정보, guard 훅 생명주기, 호스트 고유 guard 렌더링 | `cargo test -p volicord-cli --test binary_admin`, `cargo test -p volicord-cli --test guard_command` | 훅 경로가 CLI 밖의 담당 문서 정의 동작에 의존하면 Core, Store, 적합성, MCP 테스트. |
| 적합성, 계층 간 통합, 공유 픽스처 동작 | `cargo test -p volicord-test-support`, `cargo test -p volicord-conformance-tests --test baseline` 또는 `cargo test -p volicord-integration-tests --test mcp_connection` 같은 소비 패키지 테스트 대상 | 픽스처 동작이 다른 계층의 관찰 결과를 바꾸면 추가 패키지 테스트. |

## 불일치 처리

구현과 문서가 어긋나 보이면 편집하기 전에 불일치의 종류를 분류합니다.

- 가이드 수준 소스 구조 설명이 안정적인 코드와 다르면 그 설명을 담당하는
  아키텍처 가이드 페이지를 고칩니다.
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
- 완료된 공개 계약 또는 배포 변경 묶음에 대해 SemVer 영향을 한 번 평가했고, 필요하면
  공유 워크스페이스 버전을 한 번 갱신했습니다.
- 오래 유지될 소스 구조, 실행 흐름, 저장소 경계, 테스트 전략, 변경 작업
  흐름이 바뀌었을 때 관련 아키텍처 가이드 담당 문서를 갱신했습니다.
- 유지되는 문서가 바뀌었을 때 영어와 한국어 문서가 의미상 맞게 남았습니다.
- 스크래치 메모, 생성된 보고서, 런타임 홈, SQLite 파일, 픽스처 출력, 로그,
  그 밖의 부수 파일이 유지 문서에 남아 있지 않습니다.
