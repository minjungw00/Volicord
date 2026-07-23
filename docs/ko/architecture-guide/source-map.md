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
| `crates/volicord-types/src/diagnostics.rs` | Lifecycle별 occurrence/current finding 타입, opaque `DiagnosticSubjectIdentity`, `CurrentDiagnosticKey` 정규 identity와 고정 digest ID 파생, lifecycle-aware `StoredDiagnosticFinding` 및 `StoredDiagnosticGraph`, 별도의 `DiagnosticLookupReport`, 공유 read-only `DiagnosticFinding` 및 선택한 Connection의 `DiagnosticReport` 타입, 안정적인 네임스페이스 code 검증, 담당 크레이트의 typed fact에 한도와 민감정보 제거를 적용하는 projection, cause graph 검증, 예기치 않은 실패 대체 표현. |
| `crates/volicord-types/src/platform.rs` | 공유 플랫폼 환경과 플랫폼 경로 타입. |
| `crates/volicord-types/src/host_configuration.rs` | 공유 connection intent와 host scope 구성 타입. |
| `crates/volicord-types/src/connection_verification.rs` | 정규 `ConnectionStatus`, `ConnectionActivationState`, `HookActivationState`, check, 고정 action metadata, session-role evidence, 검증 보고서 타입. |
| `crates/volicord-types/src/integration_revision.rs` | Typed Connection/프로젝트 integration revision basis와 파생. |
| `crates/volicord-types/src/guard_manifest.rs` | 정규 Guard manifest, 관리 artifact, hook phase, typed command 계약. |
| `crates/volicord-types/src/tool_names.rs` | 폐쇄형 `AgentToolId` catalog, Core 소유 도구의 `MethodName` 재사용, category 및 mode metadata, 컴파일 시점 verification role 결합, 안정적인 MCP wire 이름 투영. |
| `crates/volicord-types/src/integration_verification.rs` | 공유 폐쇄형 tagged integration-verification workflow 상태, 정규 `AgentToolId`에 결속된 고정 tool-reference 타입, restart reason, begin/probe/get 공개 결과 형태. |

## Host Wire 계약

| 경로 | 책임 |
|---|---|
| `crates/volicord-host-contract/src/lib.rs` | 명시적인 `CodexMcpTurnMetadataV1`/`codex-mcp-2025-06-18-v1` 및 `CodexHooksV1`/`codex-hooks-v1` parsing, 결정적인 profile digest, 한도 있는 값과 error, source별 `CodexMcpCorrelation`, `CodexHookPromptCorrelation`, `CodexHookToolCorrelation`. |
| `crates/volicord-host-contract/tests/host_contracts.rs` | 계약 parsing, source type 분리, 필수 field 및 한도 강제, MCP 일관성, 고정 fixture manifest/checksum/profile 일치. |
| `tests/conformance/codex-host/` | 검토된 오프라인 Codex hook 및 MCP host-wire fixture와 production coverage manifest 및 checksum. |

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
| `crates/volicord-test-process/src/lib.rs` | 저장소 테스트와 스모크 하네스를 위한 안전한 `BoundedCommand`, `ProcessDeadline`, 한도 있는 수집·출력, 분류된 실패, 단일 supervisor stdio 처리, 프로세스 트리 종료, 직접 자식 회수, 한도 있는 정리. |

## Store

| 경로 | 책임 |
|---|---|
| `crates/volicord-store/src/schema/registry.sql` | Runtime Home registry DDL 정본 소스. |
| `crates/volicord-store/src/schema/project.sql` | project Store DDL 정본 소스. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home과 Store bootstrap. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection 레코드, project allowlist, managed fingerprint, 영속 검증 보고서 경계. |
| `crates/volicord-store/src/diagnostic_findings/mod.rs` | Lifecycle별 진단 영속화 facade와 공개 Store API export. |
| `crates/volicord-store/src/diagnostic_findings/occurrence.rs` | 추가 전용 occurrence 영속화와 원자적 runtime terminal-finding 연결. |
| `crates/volicord-store/src/diagnostic_findings/current_state.rs` | Current snapshot 활성화, 교체, 해소, 재활성화. |
| `crates/volicord-store/src/diagnostic_findings/graph.rs` | Cause graph 검증, 현재 보고서 root 선택, 한도가 있는 결정적 lifecycle-aware 정확한 순회. |
| `crates/volicord-store/src/diagnostic_findings/queries.rs` | Lifecycle-aware 정확한 식별자, 현재 보고서 projection, runtime-session occurrence, 활성 current scope 조회. |
| `crates/volicord-store/src/diagnostic_findings/row.rs` | 내부 finding row 인코딩, 디코딩, lifecycle identity 검증. |
| `crates/volicord-store/src/managed_launch_leases.rs` | 수명이 짧은 일회성 managed MCP launch lease, 현재 Connection 재검증, 결정적인 취소·만료 정리, 원자적 lease 소비와 runtime 생성을 담당합니다. |
| `crates/volicord-store/src/operational_sessions.rs` | Runtime-session source 디코딩, protocol milestone, revision 범위 managed MCP project session, 정확한 데이터베이스 간 binding, lease 소비 밖의 직접 `managed_host` 생성 거절. |
| `crates/volicord-store/src/integration_verification.rs` | 한도가 있는 Guard integration-verification run lifecycle, 정확한 managed runtime/session/turn 검증, probe acknowledgement, prompt/pre/post event 상관관계, 만료, 오래된 owner 처리, begin/probe/get이 함께 쓰는 record-to-public-workflow 단일 domain projection. |
| `crates/volicord-store/src/workflow_records.rs` | workflow 레코드 읽기와 쓰기. |
| `crates/volicord-store/src/core_pipeline/` | Core open, 검증, replay, commit, mutation 적용. |
| `crates/volicord-store/src/guards.rs` | Typed host 상관관계 정규화, MCP 전용 project anchor, phase별 Guard 관찰, prompt capture, 예상 쓰기, suppression 입력. |
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
| `crates/volicord-cli/src/host_launch.rs` | 숨은 동일 프로세스 host launcher, 현재 Codex entry의 정확한 재검증, launch-lease 발급·정리, managed stdio로의 메모리 내 전환. |
| `crates/volicord-cli/src/connection_command/` | connection add, list, status, verify, mode, remove 조율. |
| `crates/volicord-cli/src/connection_command/verification/mod.rs` | Connection 검증 조율, 공유 step/report 타입, 한도가 있는 패키지 export. |
| `crates/volicord-cli/src/connection_command/verification/host_checks.rs` | Managed configuration, host executable, project trust, managed-host session check. |
| `crates/volicord-cli/src/connection_command/verification/mcp_checks.rs` | MCP preflight/handshake check 투영과 MCP finding ID 입력. |
| `crates/volicord-cli/src/connection_command/verification/guard_checks.rs` | Guard 파일, hook execution, observation check 평가. |
| `crates/volicord-cli/src/connection_command/verification/dependency_graph.rs` | Cause 부착, `Blocked` 전파, graph 확정, action 선택, 정규 check 구성. |
| `crates/volicord-cli/src/connection_command/verification/finding_projection.rs` | Process, host, peer version, Guard 관찰을 lifecycle별 finding으로 투영. |
| `crates/volicord-cli/src/connection_command/verification/report_inputs.rs` | 능동 검증과 current-status 보고서 입력 조립. |
| `crates/volicord-cli/src/operational_diagnostics/mod.rs` | Typed 운영 diagnostic module facade와 한도가 있는 내부 export. |
| `crates/volicord-cli/src/operational_diagnostics/definitions.rs` | 불변 CLI 운영 diagnostic definition과 전체 폐쇄형 diagnostic 값 매핑. |
| `crates/volicord-cli/src/operational_diagnostics/subjects.rs` | 폐쇄형 typed 운영 subject, subject family별 정규 encoding과 opaque identity 파생, scope 소유권, 별도의 안전한 표시 projection. |
| `crates/volicord-cli/src/operational_diagnostics/facts.rs` | 한도가 있는 typed 운영 fact projection. |
| `crates/volicord-cli/src/operational_diagnostics/actions.rs` | Diagnostic definition, typed facts, typed check state에 따른 권장 action 선택. |
| `crates/volicord-cli/src/operational_diagnostics/projection.rs` | Current 및 occurrence finding 구성과 명시적인 active-current 보고서 projection. |
| `crates/volicord-cli/src/operational_diagnostics/persistence.rs` | Store lifecycle API를 통한 담당자 범위 활성화와 명시적 해소. |
| `crates/volicord-cli/src/connection_command/mcp_process/` | 관리 시작 구체화, 한도가 있는 자식 프로세스 감독 정책과 기한, 사전 점검 해석, stdio JSON-RPC 프레이밍과 점검 순서, 교환 진행 상태, 타입이 지정된 생명주기 또는 프로토콜 진단. 저수준 격리와 파이프 준비 상태는 `volicord-platform-process`를 통합니다. |
| `crates/volicord-cli/src/connection_command/mcp_process/host_compatibility.rs` | 프로덕션 프로토콜 레지스트리에서 파생하지 않고 독립적으로 고정한 host profile fixture와 Codex 요청/도구 호출 형태. |
| `crates/volicord-cli/src/connection_command/mcp_process/pinned_schema.rs` | 고정된 오프라인 schema를 사용한 revision별 initialize, `tools/list`, `tools/call` probe message 검증. |
| `crates/volicord-cli/src/connection_command/output/` | 선택한 Connection의 정규 진단 보고서 구성, 집계 상태와 root, 같은 보고서의 concise·verbose·lossless JSON 표시. |
| `crates/volicord-cli/src/diagnostics_command.rs` | Finding ID 및 runtime-session 세부 명령, 한도가 있는 lifecycle-aware cause traversal, lookup별 JSON 및 사람용 projection, finding severity와 독립적인 lookup-status 종료 결과. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex 구성 parsing 및 직렬화, 정규 관리 entry 검증, 허용된 도구 승인 overlay 보존, 관리 구성 변경, 진단용 실행 파일 관찰, 연결 검증. |
| `crates/volicord-cli/src/guard_integration/manifest.rs` | Guard manifest, 정확한 host-contract profile/digest, 정규 관리 artifact 기대값 생성. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | 현재 Guard 소유자, artifact, command, marker, executable 동작 audit. |
| `crates/volicord-cli/src/guard_command/` | 명시적인 `codex-hooks-v1` event decoding과 한도 있는 source별 observation. |
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
| `crates/volicord-mcp/src/managed_launch.rs` | 정규 typed 개인/공유 숨은 launcher 명령과 인자, Runtime Home 환경 binding, 엄격한 시작 형태 검증, 공개 수동 probe 구체화, projection, fingerprint 입력. |
| `crates/volicord-mcp/src/stdio.rs` | 공개 수동 stdio와 메모리 내 lease에 결속된 managed stdio 진입 경로, 권위 있는 runtime source 선택, 생명주기와 프레이밍, typed initialization profile 선택, 명시적인 `codex-mcp-2025-06-18-v1` turn-metadata parsing, revision-aware message 처리, 프로세스 사전 점검. |
| `crates/volicord-mcp/src/adapter.rs` | 공개 인수 디코딩, 서버 소유 맥락, Core 디스패치와 wrapping, Store 소유 workflow projection을 adapter-local 상태 파생 없이 직렬화하는 Core 밖의 managed in-chat begin/probe/get integration-verification 조율. |
| `crates/volicord-mcp/src/tool_registry.rs` | `AgentToolId`로 식별한 schema, annotation, 효과 설명, metadata를 세 Connection-integration 도구를 포함한 정규 도구 정의/결과로 조립하고 선택한 protocol profile을 통해 revision별 wire 이름을 투영하는 구현. |
| `crates/volicord-mcp/src/schema_validation.rs` | 공개 schema 검증. |
| `crates/volicord-mcp/src/routing.rs` | 결속된 Product Repository 탐색과 현재 Connection/project routing. |

## 테스트

| 경로 | 책임 |
|---|---|
| `crates/*/tests/`와 module-local `tests` | crate 경계와 unit test. |
| `crates/volicord-mcp/src/tests/lifecycle.rs` | Initialization 순서, 거절, 종료, EOF 계약. |
| `crates/volicord-mcp/src/tests/batching.rs` | JSON-RPC batch 순서, notification, 응답 계약. |
| `crates/volicord-mcp/src/tests/protocol_projection.rs` | Registry/profile wire projection과 schema 호환성 계약. |
| `crates/volicord-mcp/src/tests/tool_calls.rs` | Tool dispatch, 결과, 오류, 저장소 capability 계약. |
| `crates/volicord-mcp/src/tests/managed_host_observation.rs` | Lease 결속 managed launch, process 환경의 비권위성, runtime source routing, session binding, host 관찰 계약. |
| `crates/volicord-mcp/src/tests/diagnostics.rs` | 진단 영속화와 workflow metric 계약. |
| `crates/volicord-mcp/src/tests/conformance.rs` | 모듈 수준 registry 기반 protocol conformance assertion. |
| `crates/volicord-mcp/src/tests/support.rs` | 공유 MCP 테스트 fixture와 protocol message 구성만 담당. |
| `crates/volicord-mcp/tests/protocol_conformance.rs` | 모든 프로덕션 profile에 적용하는 하나의 registry 기반 wire 적합성 case. 고정 schema 검증, 필수 도구, 지정 왕복, profile별 projection 및 batching, lifecycle 거절, EOF를 다룹니다. |
| `crates/volicord-cli/tests/binary_admin.rs` | 실제 바이너리 관리 CLI 계약과 `CARGO_BIN_EXE_volicord`에 대한 정규 릴리스 바이너리 스모크 하네스 호출. 스모크 protocol 및 도구 검증은 `xtask`에 남습니다. |
| `crates/volicord-cli/tests/operational_host_e2e.rs` | 적용된 setup부터 lease-bound MCP와 정확한 Guard prompt/pre/post 검증을 거쳐 complete 읽기 전용 status에 이르는 전체 managed Codex activation journey 및 운영 실패·정리 regression. |
| `tests/conformance/` | 교차 메서드 conformance scenario. |
| `tests/conformance/mcp-spec/` | 오프라인 적합성 입력으로 쓰는 버전별 공식 MCP schema, release 및 handshake-family metadata, 검토된 `production_supported`와 `pre_release_only` 사실, 변경 불가능한 upstream pin, 라이선스 저작자 표시, checksum. |
| `tests/release-integrity/` | 일반 target 다섯 개, 버전, 기준 바이트, 패키지, checksum, 릴리스 workflow 무결성 테스트. |
| `crates/volicord-test-support/` | 재사용 fixture만 담당합니다. 일회용 Runtime Home, repository, Store 쪽 설정 및 검사, 의도적인 손상·비정상 저장소 설정, 요청 도우미를 제공합니다. 제품 동작 assertion은 owner별 test에 남고 구현 테스트 모듈은 저장소 SQL을 직접 포함하지 않습니다. |
| `crates/volicord-test-process/tests/` | 플랫폼 공통 한도 자식 실행, stdin, 실패, 시간 초과, truncation, 동시 stream, 자손이 유지하는 pipe, 정리, 프로세스 격리, 경로, 인자, 환경 coverage. 네이티브 Unix와 Windows case는 `volicord-platform-process`가 선택한 플랫폼 격리를 실행합니다. |

## 저장소 유지보수 도구

| 경로 | 책임 |
|---|---|
| `xtask/Cargo.toml` | 가벼운 유지보수 의존 경계. `volicord-mcp-protocol`에서 프로덕션 profile과 선호 서버 리비전을 받고 `volicord-test-process`에서 한도 있는 자식 실행을 받으며 `volicord-mcp`, Core, Store, CLI, host integration 크레이트를 끌어오지 않습니다. |
| `xtask/src/release_binary_smoke.rs` | 하나뿐인 플랫폼 공통 릴리스 바이너리 스모크 orchestration 담당 소스. 폐기 가능한 Git Product Repository, Runtime Home, 가짜 `codex`, 공개 `init` JSON 역직렬화, 공개 `mcp serve` JSON-RPC lifecycle, 선호 리비전 선택, 대표 도구 검증, `volicord-test-process`에 전달하는 릴리스별 프로세스 한도를 담당합니다. |
| `xtask/src/mcp_spec/mod.rs` | MCP 명세 유지보수 facade와 명령 진입점. |
| `xtask/src/mcp_spec/manifest.rs` | 엄격한 고정 manifest 모델, parsing, 결정론적 rendering. |
| `xtask/src/mcp_spec/validation.rs` | 오프라인 metadata, 변경 불가능한 pin, checksum, artifact, schema, ordering, registry 일치 검증. |
| `xtask/src/mcp_spec/report.rs` | 결정론적 검사와 동기화 보고 타입. |
| `xtask/src/mcp_spec/sync.rs` | 교체 전에 검증된 임시 후보를 사용하는 유일한 네트워크 MCP 명세 경로. |
| `xtask/tests/mcp_spec.rs` | 엄격한 manifest parsing, 분류, 집합 불일치, 변경 불가능한 pin, checksum, 필수 artifact, ordering, 보고, 오프라인 성공 coverage. |

지속되는 책임이 이동하면 이 맵을 갱신합니다. 삭제된 경로, 생성 경로, 개인 scratch 경로를
나열하지 않습니다.
