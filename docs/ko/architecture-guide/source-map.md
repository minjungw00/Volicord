# 소스 지도

이 문서는 현재 Volicord Rust 워크스페이스의 가이드 수준 소스 경로 지도를
담당합니다. 유지보수자가 코드 읽기와 코드 변경 질문을 올바른 모듈로 보낼 수
있도록 소스 경로와 구현 책임을 문서화합니다.

이 문서는 실행 흐름, 요청 생명주기, 저장소 트랜잭션, 공개 API 동작, 스키마
의미, 저장 효과, 보안 보장, Core 권한 의미, 제품 계약을 설명하지 않습니다.
상위 수준 아키텍처와 의존 경계는 [구현 아키텍처](architecture.md), 첫 번째
학습 경로는 [코드베이스 둘러보기](codebase-tour.md), 관리 CLI 작업 흐름 경계는
[CLI 작업 흐름](cli-workflows.md), 대표 메서드 흐름은
[요청 생명주기](request-lifecycle.md), Store 커밋과 아티팩트 경계는
[저장소와 트랜잭션](storage-and-transactions.md), 테스트 계층 선택은
[테스트 전략](testing-strategy.md), 정확한 계약은 [참조 색인](../reference/README.md)을
사용합니다.

모든 소스와 테스트 경로는 저장소 루트 기준입니다.

## 워크스페이스 멤버

| 경로 | Cargo 패키지 | 소스 지도 역할 |
|---|---|---|
| `crates/volicord-types` | `volicord-types` | 공유 Rust 요청, 응답, 스키마 형태, 값 집합, MCP 도구 이름, 식별자, 정규 해시 타입. |
| `crates/volicord-store` | `volicord-store` | SQLite, Runtime Home, 부트스트랩, 프로젝트 Store, 아티팩트 저장소, 검사, guard/session 관찰 저장, local web consent 저장, export snapshot, 저장소 오류 구현. |
| `crates/volicord-core` | `volicord-core` | Core 서비스, 공유 요청 파이프라인, 메서드 계획, 정책 점검, 응답 구성, Store 조율. |
| `crates/volicord-cli` | `volicord-cli` | 로컬 `volicord` 관리 바이너리, 재사용 명령 모듈, Runtime Home 설정, 프로젝트와 Agent Connection 등록, 호스트 어댑터, guard hook, User Channel 명령, 공개 `volicord mcp` 프로세스 디스패치. |
| `crates/volicord-platform-fs` | `volicord-platform-fs` | 로컬 어댑터가 사용하는 플랫폼 고유 파일시스템 이름 공간 연산을 위한 내부 안전 파사드. |
| `crates/volicord-mcp` | `volicord-mcp` | 시작 검증, 도구 목록, `tools/call` 디코딩과 디스패치, stdio 프레이밍, local HTTP 전송, Core 호출을 위한 로컬 MCP 어댑터 라이브러리. |
| `crates/volicord-test-support` | `volicord-test-support` | 구현 테스트가 공유하는 폐기 가능한 Runtime Home, Product Repository, Store, Core, Agent Connection, 픽스처 도우미. |
| `tests/conformance` | `volicord-conformance-tests` | 담당 문서가 정의한 동작을 Core 쪽 API와 공유 픽스처로 실행하는 기준 범위 교차 메서드 시나리오. |
| `tests/integration` | `volicord-integration-tests` | MCP, Core, Store, Agent Connection 바인딩, 작업 범주, 공개 스키마 snapshot을 가로지르는 테스트. |
| `xtask` | `xtask` | 문서 검증을 위한 저장소 유지보수 도구. Volicord 런타임 아키텍처의 일부가 아닙니다. |

## 공유 타입

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-types/src/lib.rs` | 공유 Rust API와 도메인 형태 값의 공개 크레이트 표면. |
| `crates/volicord-types/src/methods.rs` | 타입 지정 공개 요청과 결과 모델, 메서드 요청 스키마 생성, 메서드와 `operation_category` 매핑. |
| `crates/volicord-types/src/schema.rs` | 공유 요청 래퍼, 응답, 상태, 아티팩트, 판단, 표시, 지속 보조 형태. |
| `crates/volicord-types/src/tool_names.rs` | 공개 메서드와 어댑터 유틸리티 도구 집합을 위한 공유 MCP 노출 도구 이름 상수. |
| `crates/volicord-types/src/values.rs` | 문서화된 값 이름을 위한 제어 Rust enum과 상수. |
| `crates/volicord-types/src/ids.rs` | 불투명 식별자 래퍼와 durable ID 생성 도우미. |
| `crates/volicord-types/src/canonical.rs` | 결정적 정규 JSON 직렬화와 요청 해시. |

## 플랫폼 파일시스템 파사드

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-platform-fs/src/lib.rs` | 로컬 어댑터에 필요한 좁은 플랫폼 고유 파일시스템 이름 공간 기본 연산을 위한 안전한 Rust 파사드. 운영체제 고유 연산 실패의 문서화된 이름 공간 효과를 보고하며, 대상 상태 검증, 정리, 복구, 제품 정책 결정은 호출자가 담당합니다. |

## Store

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-store/src/lib.rs` | 저장소 기록, 스키마 초기화, 아티팩트 배관, Store 도우미의 공개 크레이트 표면. |
| `crates/volicord-store/src/runtime_home.rs` | Runtime Home 경로 해석과 Runtime Home/Product Repository 위치 검증 도우미. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home 메타데이터 초기화, 프로젝트 등록, 현재 프로젝트 도우미, User Channel 등록. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection 행, natural key, Connection Projects 멤버십, mode/status 값, Agent Connection 조회와 갱신 도우미. |
| `crates/volicord-store/src/schema.rs`와 `crates/volicord-store/src/schema/` | canonical registry와 project SQL 원본, 스키마 초기화와 검증 연결. |
| `crates/volicord-store/src/sqlite.rs` | registry/project SQLite 경로 도우미, 열기와 검증 도우미, 트랜잭션 도우미. |
| `crates/volicord-store/src/core_pipeline.rs` | Store 쪽 Core 기록, 읽기 도우미, 변이 타입, 커밋 입력/출력 타입, 재실행 도우미, Core를 위한 공개 Store 경계. |
| `crates/volicord-store/src/core_pipeline/open.rs` | 프로젝트 로컬 Store 핸들 열기와 실행 맥락 검증. |
| `crates/volicord-store/src/core_pipeline/replay.rs` | 재실행 행 조회와 재실행 맥락 일치. |
| `crates/volicord-store/src/core_pipeline/commit.rs` | 원자적 Core 변이 커밋 트랜잭션, 상태 버전 전진, 권한 이벤트 추가, 재실행 행 삽입. |
| `crates/volicord-store/src/core_pipeline/mutation_apply.rs` | `CoreStorageMutation` 값의 트랜잭션 범위 SQL 적용. |
| `crates/volicord-store/src/core_pipeline/validation.rs` | 공유 지속 값 검증과 디코딩 도우미. |
| `crates/volicord-store/src/artifacts.rs` | 일시적 아티팩트 스테이징과 영구 아티팩트 본문 검증 도우미. |
| `crates/volicord-store/src/guards.rs` | Guard installation 기록, guard event 기록, prompt capture 기록, expected-write 기록, unrecorded-change 관찰 저장 도우미. |
| `crates/volicord-store/src/session_watch.rs` | Session 수준 Product Repository watch snapshot, observation, unresolved-change 도우미 저장. |
| `crates/volicord-store/src/local_consent.rs` | Local web consent token 생성, 검증, 완료 저장 도우미. |
| `crates/volicord-store/src/inspection.rs` | 읽기 전용 Runtime Home, registry, project, Agent Connection, setup-state 검사 snapshot. |
| `crates/volicord-store/src/export.rs` | 프로젝트 기록과 아티팩트 메타데이터를 위한 읽기 전용 권한 번들 snapshot 조립. |
| `crates/volicord-store/src/error.rs` | Store 오류 타입과 저장소 실패 처리 경로. |

## Core

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-core/src/lib.rs` | Core 쪽 서비스와 어댑터 독립 메서드 진입점의 공개 크레이트 표면. |
| `crates/volicord-core/src/pipeline.rs` | `CoreService`, 호출 맥락, 공통 사전 점검, 요청 해시, Store 열기, 재실행 처리, 효과 경로 선택, 응답 구성, Core 커밋 조율. |
| `crates/volicord-core/src/methods/` | 메서드별 검증, 계획, 저장소 변이 목록, 이벤트 페이로드, dry-run 요약, 결과 필드. |
| `crates/volicord-core/src/methods/status.rs` | `volicord.status` 계획과 읽기 전용 결과 구성. |
| `crates/volicord-core/src/methods/intake.rs` | `volicord.intake` 계획과 task/change-unit 변이 준비. |
| `crates/volicord-core/src/methods/update_scope.rs` | `volicord.update_scope` 계획과 scope 변이 준비. |
| `crates/volicord-core/src/methods/prepare_write.rs` | `volicord.prepare_write` 계획, 호환성 점검, 쓰기 티켓 변이 준비. |
| `crates/volicord-core/src/methods/record_run.rs` | 실행과 증거 관련 변이를 위한 `volicord.record_run` 계획. |
| `crates/volicord-core/src/methods/reconcile_changes.rs` | 해결되지 않은 Product Repository 관찰을 위한 `volicord.reconcile_changes` 계획. |
| `crates/volicord-core/src/methods/judgment.rs` | 사용자 판단 요청과 기록 메서드 계획. |
| `crates/volicord-core/src/methods/close_task.rs` | `volicord.close_task` 계획과 닫기 준비 상태 결과 처리. |
| `crates/volicord-core/src/methods/session_watch.rs` | Session-watch 메서드 계획과 관찰 조율. |
| `crates/volicord-core/src/methods/stage_artifact.rs` | 일시적 아티팩트 스테이징 메서드 처리. |
| `crates/volicord-core/src/policy/` | 접근 점검, 재실행 맥락, Product Repository 경로 정규화, 쓰기 티켓 호환성, 증거 상태, 판단 관련성, continuity, rationale, effect contract, 닫기 준비 상태 계산을 위한 재사용 Core 정책 도우미. |
| `crates/volicord-core/src/methods/tests/` | 메서드 계획기 가까이에 있는 Core 메서드와 파이프라인 테스트. |

## CLI

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-cli/src/main.rs` | `volicord` 프로세스 진입, 관리 명령 디스패치, `volicord mcp`와 local HTTP 프로세스 모드 인계, setup gate, 바이너리 종료 동작. |
| `crates/volicord-cli/src/lib.rs` | 재사용 명령 모듈을 위한 공유 관리 CLI 크레이트 표면. |
| `crates/volicord-cli/src/setup_command.rs`와 `crates/volicord-cli/src/setup_command/` | Setup 명령 진입, setup workflow 실행, 실행 파일 발견, command-link 계획, shell startup 계획, interactive 선택, setup 출력 렌더링. |
| `crates/volicord-cli/src/connection_command.rs`와 `crates/volicord-cli/src/connection_command/` | `volicord init`, `volicord connection add`, `volicord connection list`, `volicord connection status/verify/mode/remove` 파싱, 프로비저닝, 선택, 검증, MCP process 점검, 출력 렌더링. |
| `crates/volicord-cli/src/guard_command.rs`와 `crates/volicord-cli/src/guard_command/` | Guard hook 명령 디스패치, 인수 파싱, host event 정규화, tool observation 추출, mutation 분류, phase 처리, prompt capture, prompt 안의 judgment 명령, 쓰기 티켓 점검, hook output 렌더링. |
| `crates/volicord-cli/src/guard_integration/` | Guard integration 계획, 생성 guard 파일 적용, capability metadata, policy helper, 호스트별 guard hook 계획, connection status와 doctor diagnostics가 쓰는 사실 기반 audit helper. |
| `crates/volicord-cli/src/guard_integration/plan.rs` | 호스트 capability, profile, project, runtime fact를 가로지르는 guard integration plan 조립. |
| `crates/volicord-cli/src/guard_integration/files.rs` | 생성 guard 파일과 관리 policy 파일 계획, 고정된 Product Repository 경로 순회, 대상 스냅샷, 조건부 동일 디렉터리 교체, 연산 후 복구 검사. |
| `crates/volicord-cli/src/guard_integration/apply.rs` | 계획된 생성 guard 파일과 관리 projection을 위한 렌더링과 적용 디스패치. |
| `crates/volicord-cli/src/guard_integration/capability.rs` | Capability metadata와 기록된 guard installation metadata 도우미. |
| `crates/volicord-cli/src/guard_integration/policy.rs` | Guard policy helper 값과 lifecycle phase 도우미. |
| `crates/volicord-cli/src/guard_integration/hooks.rs`와 `crates/volicord-cli/src/guard_integration/hosts/` | Host hook command 계획과 호스트별 생성 파일 계획. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | 기록된 capability metadata, 생성 파일, wrapper script, hook command path, 관리 projection에 대한 사실 점검. 이 사실은 진단 관찰이지 보안 보장, 사용자 승인 기록, 정확성 증명이 아닙니다. |
| `crates/volicord-cli/src/doctor_command.rs` | 진단 보고를 위한 installation, connection, host, guard fact 수집. |
| `crates/volicord-cli/src/user_command.rs` | 로컬 User Channel 상태와 `volicord inbox` 명령 파싱 및 오케스트레이션. |
| `crates/volicord-cli/src/host_integration/` | 공유 host kind, scope, capability, lifecycle phase, config editing, integration contract, generic-host guidance, diagnostic status type. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex 어댑터 내부 구현. config 계획, 실행 파일 점검, managed identity, trust fact, 검증. |
| `crates/volicord-cli/src/host_integration/claude_code/` | Claude Code 어댑터 내부 구현. CLI command 구성, config 계획, managed identity 점검, host-native output 파싱, 검증. |
| `crates/volicord-cli/src/registration.rs` | Agent Connection, Connection Project, User Channel metadata 구성. |
| `crates/volicord-cli/src/project_context.rs` | Product Repository root 감지와 `volicord project ...` 명령 오케스트레이션. |
| `crates/volicord-cli/src/export_command.rs` | 권한 번들 export 명령 파싱과 렌더링. |
| `crates/volicord-cli/src/changes_command.rs` | 로컬 change-reconciliation 명령 파싱과 오케스트레이션. |
| `crates/volicord-cli/src/serve_command.rs` | Local HTTP service 명령 파싱과 server configuration 인계. |
| `crates/volicord-cli/src/disclosure.rs`, `managed_block.rs`, `shell_path.rs`, `setup_report.rs`, `summary_card.rs` | Disclosure text, managed file block, shell path 처리, setup report data, compact status summary를 위한 공유 CLI 도우미. |

## MCP 어댑터

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-mcp/src/lib.rs` | CLI 바이너리가 사용하는 공개 크레이트 표면과 다시 내보낸 adapter entry point. |
| `crates/volicord-mcp/src/tool_registry.rs` | Agent Connection mode별 MCP 노출 도구 metadata와 tool set. |
| `crates/volicord-mcp/src/routing.rs` | Agent Connection 시작 검사, project availability, project allowlist 점검, 요청 시점 project selection 도우미. |
| `crates/volicord-mcp/src/adapter.rs` | 타입 지정 공개 `tools/call` 디코딩, adapter utility call, `operation_category`와 `actor_source` 파생, Core 호출, 응답 래핑 도우미. |
| `crates/volicord-mcp/src/stdio.rs` | JSON-RPC stdio 프레이밍, 초기화, 응답 래핑, elicitation 처리, `volicord mcp`가 쓰는 stdio/preflight runner. |
| `crates/volicord-mcp/src/local_http.rs` | Local loopback HTTP server 설정, endpoint routing, token handling, local HTTP MCP serving. |
| `crates/volicord-mcp/src/local_web_consent.rs` | User Channel 답변을 위한 local web consent 요청과 완료 처리. |
| `crates/volicord-mcp/src/http.rs` | 공유 HTTP parsing과 response helper. |
| `crates/volicord-mcp/src/constants.rs` | MCP 모듈이 공유하는 adapter constant. |
| `crates/volicord-mcp/src/errors.rs` | MCP adapter와 local HTTP error type. |
| `crates/volicord-mcp/src/prelude.rs`와 `crates/volicord-mcp/src/util.rs` | 내부 공유 import와 작은 adapter utility helper. |
| `crates/volicord-mcp/src/tests.rs` | 크레이트 로컬 MCP adapter와 transport 테스트. |

## 테스트와 유지보수 지원

| 소스 경로 | 책임 |
|---|---|
| `crates/volicord-test-support/src/lib.rs` | 폐기 가능한 Runtime Home 도우미, Core fixture, 요청 builder, fixture 전용 Store 도우미, 구현 테스트용 공유 assertion. |
| `crates/volicord-cli/tests/support/` | CLI 통합 테스트용 binary fixture, fake host, fake MCP process, JSON helper, assertion, guard lifecycle fixture. |
| `crates/volicord-cli/tests/binary_admin.rs` | setup, project, connection, status, inbox, preflight, host configuration 동작의 binary 수준 관리 CLI coverage. |
| `crates/volicord-cli/tests/guard_command.rs` | Guard hook lifecycle, prompt capture, observed mutation, expected-write, write-ticket matching, guarded init/status coverage. |
| `crates/volicord-cli/tests/mcp_transport.rs` | `volicord mcp` 하위 명령, `--check`, stdio 프레이밍, 재연결, MCP 응답 래핑 coverage. |
| `crates/volicord-cli/tests/serve_transport.rs` | Local HTTP service 명령과 transport coverage. |
| `crates/volicord-cli/tests/live_host_smoke.rs` | 테스트 환경 가용성에 의해 보호되는 host smoke-test coverage. |
| `tests/conformance/baseline.rs` | Core 쪽 API를 통한 교차 메서드 기준 시나리오. |
| `tests/integration/mcp_connection.rs` | MCP/Core/Store와 Agent Connection 동작을 가로지르는 coverage. |
| `tests/integration/public_contract_snapshots.rs`와 `tests/integration/snapshots/` | 공개 schema와 MCP tool snapshot contract coverage. |
| `xtask/src/main.rs`와 `xtask/src/lib.rs` | 문서 검증을 포함한 읽기 전용 저장소 유지보수 명령. |

이 소스 설명은 구현 배치 지침입니다. 이 지도와 집중 참조 담당 문서가 제품
동작에 대해 어긋나 보이면, 소스 배치에서 제품 계약을 추론하지 말고 담당 경로
공백이나 구현 공백으로 다룹니다.
