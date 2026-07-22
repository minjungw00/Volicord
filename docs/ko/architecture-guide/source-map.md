# 소스 맵

이 맵은 유지관리자를 현재 구현 소유자로 안내합니다. 제품 계약이 아니므로 정확한 동작은
집중된 Reference 문서를 사용합니다.

## 공유 타입

| 경로 | 책임 |
|---|---|
| `crates/volicord-types/src/schema.rs` | 공유 요청, 응답, 저장 레코드 형태. |
| `crates/volicord-types/src/values.rs` | 폐쇄 제품 값 집합. |
| `crates/volicord-types/src/ids.rs` | 불투명 식별자. |
| `crates/volicord-types/src/canonical.rs` | 정규 직렬화와 해시. |
| `crates/volicord-types/src/diagnostics.rs` | 공유 `DiagnosticFinding`과 `DiagnosticReport` 구조, 안정적인 네임스페이스 코드 검증, 담당 크레이트의 타입이 지정된 사실에 한도와 민감정보 제거를 적용하는 투영, 원인 그래프 검증, 예기치 않은 실패 대체 표현. |
| `crates/volicord-types/src/platform.rs` | 공유 플랫폼 환경과 플랫폼 경로 타입. |
| `crates/volicord-types/src/host_configuration.rs` | 공유 connection intent와 host scope 구성 타입. |
| `crates/volicord-types/src/connection_verification.rs` | 정규 connection 상태, check, action, 검증 보고서 타입. |
| `crates/volicord-types/src/integration_revision.rs` | Typed Connection/프로젝트 integration revision basis와 파생. |
| `crates/volicord-types/src/guard_manifest.rs` | 정규 Guard manifest, 관리 artifact, hook phase, typed command 계약. |
| `crates/volicord-types/src/tool_names.rs` | 정규 공개 method 및 adapter utility MCP 도구 이름 집합. |

## 플랫폼 파일시스템 경계

| 경로 | 책임 |
|---|---|
| `crates/volicord-platform-fs/src/lib.rs` | 현재 프로세스 target 및 플랫폼 관찰, kernel을 통한 네이티브 Linux/WSL2 분류, WSL2 `/etc/os-release` 배포판 검증, 경로 파일시스템 관찰, 플랫폼 고유 이름 공간 연산, 정규 읽기 전용 Git layout 탐색. |
| `crates/volicord-cli/src/host_integration/process.rs` | 플랫폼 경계 관찰을 바탕으로 한 프로세스 target 검증과 target 경로 파일시스템 제한 집행. |

## 플랫폼 프로세스 경계

| 경로 | 책임 |
|---|---|
| `crates/volicord-platform-process/src/lib.rs` | 한도가 있는 자식 프로세스 격리, 명령 설정, 연결, 프로세스 트리 종료, 비차단 자식 파이프 폴링을 위한 안전한 API와 안정적으로 분류된 오류 범주. |
| `crates/volicord-platform-process/src/unix.rs` | Unix 프로세스 그룹 격리와 비차단 파이프 primitive. |
| `crates/volicord-platform-process/src/windows.rs` | 비공개 Windows Job Object 소유권과 익명 파이프 준비 상태 primitive. |

## Store

| 경로 | 책임 |
|---|---|
| `crates/volicord-store/src/schema/registry.sql` | Runtime Home registry DDL 정본 소스. |
| `crates/volicord-store/src/schema/project.sql` | project Store DDL 정본 소스. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home과 Store bootstrap. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection 레코드, project allowlist, managed fingerprint, 영속 검증 보고서 경계. |
| `crates/volicord-store/src/diagnostic_findings.rs` | 구조화된 finding 및 cause graph의 transaction 영속화, runtime terminal-finding 연결, 현재 좌표 조회, 한도가 있는 결정적 traversal. |
| `crates/volicord-store/src/operational_sessions.rs` | 관리 runtime session, protocol milestone, revision 범위 project session, 정확한 데이터베이스 간 binding. |
| `crates/volicord-store/src/workflow_records.rs` | workflow 레코드 읽기와 쓰기. |
| `crates/volicord-store/src/core_pipeline/` | Core open, 검증, replay, commit, mutation 적용. |
| `crates/volicord-store/src/guards.rs` | Guard 관찰, 예상 쓰기, suppression 입력. |
| `crates/volicord-store/src/evidence_capture.rs` | Evidence-capture intent와 producer 레코드. |
| `crates/volicord-store/src/artifacts.rs` | 아티팩트 staging과 영속 본문 검증. |
| `crates/volicord-store/src/error.rs` | Store 실패 분류. |

## Core

| 경로 | 책임 |
|---|---|
| `crates/volicord-core/src/pipeline.rs` | 공통 사전 점검, replay, plan 선택, 응답, commit 조율. |
| `crates/volicord-core/src/methods/` | 메서드별 구조 검증과 계획. |
| `crates/volicord-core/src/policy/` | 재사용 접근, workflow, evidence, continuity, write-ticket, close-readiness 정책. |
| `crates/volicord-core/src/agent_session.rs` | 현재 Connection, project membership, mode, 관리 runtime/project session 검증. |
| `crates/volicord-core/src/authority_status.rs` | typed status와 authority receipt 대응. |

## CLI와 Codex 어댑터

| 경로 | 책임 |
|---|---|
| `crates/volicord-cli/src/main.rs` | 프로세스 진입과 관리 명령 디스패치. |
| `crates/volicord-cli/src/connection_command/` | connection add, list, status, verify, mode, remove 조율. |
| `crates/volicord-cli/src/connection_command/verification.rs` | Dependency-aware 검증 check, `Blocked` 전파, managed-host 관찰 정책, cause 부착, 결정론적 root 선택. |
| `crates/volicord-cli/src/operational_diagnostics/mod.rs` | Typed 운영 diagnostic module facade와 한도가 있는 내부 export. |
| `crates/volicord-cli/src/operational_diagnostics/definitions.rs` | 불변 CLI 운영 diagnostic definition과 전체 폐쇄형 diagnostic 값 매핑. |
| `crates/volicord-cli/src/operational_diagnostics/subjects.rs` | 폐쇄형 typed 운영 subject, 정규 identity byte, scope 소유권, 안전한 표시 projection. |
| `crates/volicord-cli/src/operational_diagnostics/facts.rs` | 한도가 있는 typed 운영 fact projection. |
| `crates/volicord-cli/src/operational_diagnostics/actions.rs` | Diagnostic definition, typed facts, typed check state에 따른 권장 action 선택. |
| `crates/volicord-cli/src/operational_diagnostics/projection.rs` | Current 및 occurrence finding 구성과 명시적인 active-current 보고서 projection. |
| `crates/volicord-cli/src/operational_diagnostics/persistence.rs` | Store lifecycle API를 통한 담당자 범위 활성화와 명시적 해소. |
| `crates/volicord-cli/src/connection_command/mcp_process/` | 관리 시작 구체화, 한도가 있는 자식 프로세스 감독 정책과 기한, 사전 점검 해석, stdio JSON-RPC 프레이밍과 점검 순서, 교환 진행 상태, 타입이 지정된 생명주기 또는 프로토콜 진단. 저수준 격리와 파이프 준비 상태는 `volicord-platform-process`를 통합니다. |
| `crates/volicord-cli/src/connection_command/mcp_process/host_compatibility.rs` | 프로덕션 프로토콜 레지스트리에서 파생하지 않고 독립적으로 고정한 host profile fixture와 Codex 요청/도구 호출 형태. |
| `crates/volicord-cli/src/connection_command/mcp_process/pinned_schema.rs` | 고정된 오프라인 schema를 사용한 revision별 initialize, `tools/list`, `tools/call` probe message 검증. |
| `crates/volicord-cli/src/connection_command/output/` | 선택한 Connection의 정규 진단 보고서 구성, 집계 상태와 root, 같은 보고서의 concise·verbose·lossless JSON 표시. |
| `crates/volicord-cli/src/diagnostics_command.rs` | Finding ID 및 runtime-session 세부 명령, 한도가 있는 cause traversal, 보고서 projection. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex 구성 parsing 및 직렬화, 정규 관리 entry 검증, 허용된 도구 승인 overlay 보존, 관리 구성 변경, 진단용 실행 파일 관찰, 연결 검증. |
| `crates/volicord-cli/src/guard_integration/manifest.rs` | Guard manifest와 정규 관리 artifact 기대값 생성. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | 현재 Guard 소유자, artifact, command, marker, executable 동작 audit. |
| `crates/volicord-cli/src/guard_command/` | Guard 이벤트 디코딩과 bounded 관찰. |
| `crates/volicord-cli/src/user_command.rs` | CLI 받은 편지함과 local-user resolution. |
| `crates/volicord-cli/src/doctor_command.rs` | 진단 사실 수집과 표시. |

## MCP 프로토콜 프로필

| 경로 | 책임 |
|---|---|
| `crates/volicord-mcp-protocol/src/lib.rs` | 폐쇄형 MCP 리비전 타입 파싱, 프로덕션 프로필 조회, 메시지·도구·스키마 기능 선언, 결정론적인 지원 리비전 순서, 추적 중인 사전 릴리스 분류, 별도로 선택하는 서버 선호 리비전. |
| `crates/volicord-mcp-protocol/tests/protocol_registry.rs` | 고정 매니페스트 일치, 정확한 스키마 기능 일치, 순서, 중복 배제, 정확한 파싱, 선호 리비전 포함, 사전 릴리스 배제 검증. |

## MCP 어댑터

| 경로 | 책임 |
|---|---|
| `crates/volicord-mcp/src/managed_launch.rs` | 정규 typed 개인/공유 관리형 MCP 명령, 인자, 정적 및 전달 환경 binding, 엄격한 시작 형태 검증, projection, fingerprint 입력. |
| `crates/volicord-mcp/src/conformance.rs` | 어댑터 테스트, CLI revision 매트릭스, 명세 checker가 공유하는 결정론적 저장소 소유 오프라인 런타임 적합성 revision 선언. |
| `crates/volicord-mcp/src/stdio.rs` | stdio 생명주기와 프레이밍, typed initialization profile 선택, revision-aware message 처리, 프로세스 사전 점검. |
| `crates/volicord-mcp/src/adapter.rs` | 공개 인수 디코딩, 서버 소유 맥락, Core 디스패치, wrapping. |
| `crates/volicord-mcp/src/tool_registry.rs` | 담당자가 제공하는 도구 이름과 schema를 정규 도구 정의/결과로 조립하고 선택한 protocol profile을 통해 revision별 wire projection을 수행하는 구현. |
| `crates/volicord-mcp/src/schema_validation.rs` | 공개 schema 검증. |
| `crates/volicord-mcp/src/routing.rs` | 결속된 Product Repository 탐색과 현재 Connection/project routing. |

## 테스트

| 경로 | 책임 |
|---|---|
| `crates/*/tests/`와 module-local `tests` | crate 경계와 unit test. |
| `tests/conformance/` | 교차 메서드 conformance scenario. |
| `tests/conformance/mcp-spec/` | 오프라인 적합성 입력으로 쓰는 버전별 공식 MCP schema, release 및 handshake-family metadata, 검토된 프로덕션 지원 및 `volicord_conformance_covered` 값, 변경 불가능한 upstream pin, 라이선스 저작자 표시, checksum. |
| `tests/release-integrity/` | 일반 target 다섯 개, 버전, 기준 바이트, 패키지, checksum, 릴리스 workflow 무결성 테스트. |
| `crates/volicord-test-support/` | 일회용 Runtime Home, repository, Store, 요청 도우미. |

## 저장소 유지보수 도구

| 경로 | 책임 |
|---|---|
| `xtask/src/mcp_spec.rs` | 고정 명세의 오프라인 검증, manifest/profile/harness의 정확한 집합 일치, 결정론적 개수 보고, 검토된 metadata를 보존하면서 검증된 임시 후보를 거치는 명시적 네트워크 동기화. |
| `xtask/tests/mcp_spec.rs` | 엄격한 manifest parsing, 분류, 집합 불일치, 변경 불가능한 pin, checksum, 필수 artifact, ordering, 보고, 오프라인 성공 coverage. |

지속되는 책임이 이동하면 이 맵을 갱신합니다. 삭제된 경로, 생성 경로, 개인 scratch 경로를
나열하지 않습니다.
