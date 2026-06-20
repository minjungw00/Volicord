# 테스트 전략

이 가이드는 Harness Rust 변경에서 어떤 구현 테스트 계층을 사용할지
설명합니다. 테스트는 담당 문서가 정의한 사실을 검증합니다. 테스트가 제품
계약을 정의하거나, 보안을 증명하거나, QA를 완료하거나, 닫기 준비 상태를
확립하거나, 제품 수락을 기록하지 않습니다.

정확한 동작은 [참조 색인](../reference/README.md)을 사용합니다. 워크스페이스
구조는 [구현 아키텍처](architecture.md)를 사용합니다. 변경 작업 흐름은
[구현 가이드](change-guide.md)를 사용합니다.

## 테스트 계층

| 계층 | 실제 패키지 또는 경로 | 사용할 때 | 사용하면 안 되는 것 |
|---|---|---|---|
| 모듈 단위 테스트 | [`crates/harness-types/src/lib.rs`](../../../crates/harness-types/src/lib.rs), [`crates/harness-core/src/pipeline.rs`](../../../crates/harness-core/src/pipeline.rs), [`crates/harness-store/src/core_pipeline.rs`](../../../crates/harness-store/src/core_pipeline.rs), [`crates/harness-store/src/sqlite.rs`](../../../crates/harness-store/src/sqlite.rs), CLI 또는 MCP 모듈 같은 구현 모듈 안의 테스트. | 로컬 도우미 동작, 타입 지정 파싱, 정규 해시, 정책 도우미, Store 트랜잭션 경계, 스키마 검증, 코드 가까이의 작은 분기 점검. | 계층 간 수락 테스트나 제품 계약 출처. |
| Core 메서드 테스트 | `harness-core` 패키지의 [`crates/harness-core/src/methods/tests.rs`](../../../crates/harness-core/src/methods/tests.rs). | 메서드 계획, `CoreService`를 통한 공유 사전 점검, dry-run/효과 없음/커밋 분기, 재실행, 상태 버전 효과, 아티팩트 스테이징 구분, 메서드에 보이는 Store 효과. | MCP 전송 범위나 전체 공개 동작 권위. |
| 관리 CLI 바이너리 테스트 | `harness-cli` 패키지의 `binary_admin` 대상인 [`crates/harness-cli/tests/binary_admin.rs`](../../../crates/harness-cli/tests/binary_admin.rs). | `harness` 바이너리, Runtime Home 설정 명령, 로컬 MCP 설정, dry-run, 호스트 설정 쓰기, 사전 점검 실패 처리, 명령줄 오류 경로. | 공개 API 메서드 동작. |
| MCP 전송 바이너리 테스트 | `harness-mcp` 패키지의 `binary_transport` 대상인 [`crates/harness-mcp/tests/binary_transport.rs`](../../../crates/harness-mcp/tests/binary_transport.rs). | `harness-mcp` 바이너리, help/version, `--check`, stdio 프레이밍, JSON-RPC 동작, 재연결 사례, 응답 래핑. | Core 메서드 의미. |
| MCP 통합 테스트 | `harness-integration-tests` 패키지의 `mcp_surface` 대상인 [`tests/integration/mcp_surface.rs`](../../../tests/integration/mcp_surface.rs). | MCP, Core, Store, 접점 바인딩, 접근 파생, 도구 노출, 재실행 맥락 바인딩, MCP를 통해 보이는 저장소 효과 없음 점검. | 집중 메서드 테스트나 참조 담당 문서의 대체물. |
| 적합성 구현 테스트 | `harness-conformance-tests` 패키지의 `baseline` 대상인 [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs). | Core 쪽 API를 통한 기준 범위 교차 메서드 시나리오. 재실행, 쓰기 권한 부여, 아티팩트, 판단, 닫기 준비 상태, 오류 처리 경로, 손상 처리 등을 포함합니다. | 제품 수락, 보안 증명, 닫기 준비 상태, 또는 제품 규칙의 유일한 출처. |
| 공유 테스트 지원 | `harness-test-support` 패키지의 [`crates/harness-test-support/src/lib.rs`](../../../crates/harness-test-support/src/lib.rs). | 폐기 가능한 Runtime Home 픽스처, 등록된 프로젝트와 접점 설정, 요청 빌더, Store 검사 도우미, 공유 픽스처 구성. | 프로덕션 동작이나 오래 유지될 Runtime Home. |
| 문서 유지보수 도구 테스트 | `xtask` 패키지의 [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs). | 읽기 전용 문서 검증기, 메타데이터 파싱, 한영 대응 범위, 로컬 링크와 앵커 점검, 용어 경로 점검, 폐기 경로 감지, 임시 픽스처 동작. | 의미 번역 검토, 기술 정확성 검토, 제품 계약 출처. |

## 계층 선택

| 변경 범주 | 여기서 시작 | 추가할 때 |
|---|---|---|
| 공유 요청, 응답, 값, 식별자, 정규 해시 타입 | `harness-types` 단위 테스트. | 형태 변경이 메서드 계획이나 어댑터 노출을 바꾸면 Core 메서드 또는 통합 테스트를 추가합니다. |
| Store 읽기 도우미, 변이 적용, 트랜잭션, 마이그레이션, 아티팩트 저장소 동작 | 변경된 코드 가까이의 Store 모듈 테스트. | 공개 메서드 효과가 바뀌면 Core 메서드 테스트를, 계층 간 동작이 영향을 받으면 적합성 또는 MCP 통합 테스트를 추가합니다. |
| Core 메서드 동작 | `crates/harness-core/src/methods/tests.rs`. | 교차 메서드 기준 범위 시나리오는 `tests/conformance/baseline.rs`를, MCP 노출이나 접근 파생이 중요하면 `tests/integration/mcp_surface.rs`를 추가합니다. |
| 공통 Core 사전 점검, 분기 처리, 재실행, 최신성, 접근 정책 | `crates/harness-core/src/pipeline.rs` 단위 테스트와 메서드 테스트. | 어댑터가 파생한 호출 맥락이나 세션 바인딩이 관련되면 MCP 통합 테스트를 추가합니다. |
| MCP 어댑터 시작, 도구 스키마, `tools/call`, stdio 전송 | `crates/harness-mcp/src/lib.rs` 테스트와 `binary_transport`. | MCP를 통과한 Core/Store 계층 간 동작은 `tests/integration/mcp_surface.rs`를 추가합니다. |
| 관리 설정 동작 | `binary_admin`과 CLI 모듈 테스트. | 부트스트랩, 검사, 마이그레이션 동작이 바뀌면 Store 테스트를 추가합니다. |
| 테스트 픽스처 동작 | `harness-test-support` 테스트 또는 소비 패키지의 테스트. | 픽스처가 빠진 계약 담당 문서를 드러내면 담당 문서 중심 문서 점검을 추가합니다. |
| 문서 검증기 동작 | `xtask` 테스트와 `cargo run -p xtask -- docs-check`. | 새 결정적 구조 규칙을 도입하면 픽스처 사례를 추가합니다. |
| 개발자 문서만 바뀐 경우 | `cargo run -p xtask -- docs-check`와 사람이 하는 의미 일치, 담당 경로, 용어 검토. | 사용자가 요청했거나 문서 변경이 새 소스 검증에 의존하면 Cargo 테스트를 실행합니다. |

## 경계를 보여 주는 테스트

일부 테스트는 아키텍처 경계를 이해하는 데 특히 유용합니다.

- `mcp_exposes_exactly_the_documented_public_methods`와
  `stdio_tools_list_exposes_exactly_the_public_method_set`은 공개 메서드
  집합의 MCP 노출을 보여 줍니다.
- `adapter_and_direct_core_status_have_equivalent_response_meaning`와
  `mcp_and_direct_status_omit_same_excluded_projection_fields`는 어댑터를
  통과한 동작과 직접 Core 동작을 비교합니다.
- `rejected_branch_has_no_storage_effect`, `dry_run_branch_has_no_storage_effect`,
  `read_only_branch_has_no_storage_effect`는 커밋 없는 분기를 보호합니다.
- `committed_mutation_increments_state_version_once`와 Store 트랜잭션 재실행
  테스트는 원자적 커밋 경계를 보호합니다.
- `stage_artifact_creates_transient_handle_without_core_commit`는 스테이징
  경로가 정상 Core 변이 커밋과 혼동되지 않도록 보호합니다.
- `no_effect_branches_state_version_and_idempotency_are_stable`은 Core 쪽 API를
  통해 교차 메서드 효과 없음과 재실행 안정성을 보여 줍니다.

이 테스트들은 구현 점검입니다. Harness 런타임 적합성 주장, 제품 수락
기록, QA 완료, 보안 증명, 닫기 준비 상태 결과, 잔여 위험 수락이 아닙니다.

## 검증 기본값

Rust 구현을 편집했을 때 저장소 기본값은 아래와 같습니다.

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

문서만 편집했다면 적용되는 문서 점검을 사용합니다. 문서 작업이 소스
검증을 요구하면 `cargo metadata --no-deps --format-version 1`, 저장소 검색,
요청된 테스트 명령이 적절한 구현 점검입니다.

유지 문서 구조 점검은 아래 명령으로 실행합니다.

```sh
cargo run -p xtask -- docs-check
```

그다음 바뀐 문서에 맞는 한영 의미 검토, 계약 담당 문서 검토, 기술 정확성
검토를 사람이 완료합니다.
