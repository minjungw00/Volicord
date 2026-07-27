# 소스 맵

이 맵은 유지관리자를 현재 구현 소유자로 안내합니다. 제품 계약이 아니므로 정확한 동작은
집중된 Reference 문서를 사용합니다.

## 공유 타입

| 경로 | 책임 |
|---|---|
| `crates/volicord-types/src/lib.rs` | 공개 담당 모듈 경로. 공유 정의는 각 담당 모듈을 통해 공개됩니다. |
| `crates/volicord-types/src/schema.rs` | 공유 요청, 응답, 저장 레코드 형태. |
| `crates/volicord-types/src/product_path.rs` | 담당 문서가 정의한 typed 상대 제품 경로와 정규화. |
| `crates/volicord-types/src/values.rs` | 폐쇄 제품 값 집합. |
| `crates/volicord-types/src/ids.rs` | 불투명 식별자. |
| `crates/volicord-types/src/canonical.rs` | 정규 직렬화와 해시. |
| `crates/volicord-types/src/diagnostics.rs` | Lifecycle별 occurrence/current finding 타입, opaque `DiagnosticSubjectIdentity`, `CurrentDiagnosticKey` 정규 identity와 고정 digest ID 파생, lifecycle-aware `StoredDiagnosticFinding` 및 `StoredDiagnosticGraph`, 별도의 `DiagnosticLookupReport`, 공유 read-only `DiagnosticFinding` 및 선택한 Connection의 `DiagnosticReport` 타입, 안정적인 네임스페이스 code 검증, 담당 크레이트의 typed fact에 한도와 민감정보 제거를 적용하는 projection, cause graph 검증, 예기치 않은 실패 대체 표현. |
| `crates/volicord-types/src/platform.rs` | 공유 플랫폼 환경과 플랫폼 경로 타입. |
| `crates/volicord-types/src/host_configuration.rs` | 공유 connection intent와 host scope 구성 타입. |
| `crates/volicord-types/src/connection_verification.rs` | 정규 `ConnectionStatus`, `IntegrationActivationState`, `HookActivationState`, check, 단일 계층형 `IntegrationActivationPlan`, 안정적인 actor/channel/step metadata, 위상 검증, nested agent sequence, session-role evidence, 검증 보고서 타입. |
| `crates/volicord-types/src/integration_revision.rs` | Typed Connection/프로젝트 integration revision basis와 파생. |
| `crates/volicord-types/src/guard_manifest.rs` | 정규 Guard manifest, 관리 artifact, hook phase, typed command 계약. |
| `crates/volicord-types/src/tool_names.rs` | 폐쇄형 `AgentToolId` catalog, Core 소유 도구의 `MethodName` 재사용, category 및 mode metadata, 컴파일 시점 verification role 결합, catalog 소유 `IntegrationVerificationToolRole`, 안정적인 MCP wire 이름 투영. |
| `crates/volicord-types/src/integration_verification.rs` | 공유 폐쇄형 tagged integration-verification workflow 상태, 정규 `AgentToolId`에 결속된 고정 tool-reference 타입, typed routed-event relevance, terminal reason mapping을 포함한 Guard probe acquisition stage, restart reason, begin/probe/get 공개 결과 형태. |

## Host Wire 계약

| 경로 | 책임 |
|---|---|
| `crates/volicord-host-contract/src/lib.rs` | Semantic `CodexMcpTurnMetadata`, `CodexCommandHooks`, `CodexMcpCallableNames` 계약, typed host-tool 및 server-namespace/catalog-derived-exact hook routing, MCP 전용 routing 분류, 결정적인 profile digest, 한도 있는 값과 error, source별 상관관계, 명시적인 `McpServerKey`·`McpRawToolName`·`McpToolIdentity`, `HostCallableIdentity`로의 충돌 및 role 일관성 검사 투영, 정확한 `McpToolCatalog` 역방향 조회. |
| `crates/volicord-host-contract/tests/host_contracts.rs` | 계약 parsing, source type 분리, 필수 field 및 한도 강제, typed matcher routing 및 재구성, MCP 일관성, 고정 fixture manifest/checksum/profile 일치. |
| `tests/conformance/codex-host/` | 검토된 오프라인 Codex command-hook, MCP turn-metadata, MCP callable-name fixture와 semantic profile coverage manifest 및 checksum. |

## 플랫폼 파일시스템 경계

| 경로 | 책임 |
|---|---|
| `crates/volicord-platform-fs/src/lib.rs` | 현재 프로세스 target 및 플랫폼 관찰, kernel을 통한 네이티브 Linux/WSL2 분류, WSL2 `/etc/os-release` 배포판 검증, 경로 파일시스템 관찰, 고유 정규 code와 한도가 있는 세부사항을 가진 폐쇄형 typed 플랫폼 diagnostic kind, 공유 플랫폼 finding projection, 효과를 인식하는 정확한 directory-tree 제거, typed 원자적 기존 대상 비대체 일반 파일 공개와 상위 entry 내구성, 플랫폼 고유 이름 공간 연산, 안전한 Runtime Home 변경 lease와 permit export, 정규 읽기 전용 Git layout 탐색. |
| `crates/volicord-platform-fs/src/mutation_lease.rs` | 정규 Runtime Home identity, domain-separated 전체 digest 기반 외부 coordination 파일 파생, OS lock 영역 하나를 공유하는 shared-writer 및 exclusive-setup mode, 즉시 및 한도 있는 typed 획득, 빌린 변경 permit, Unix/macOS 또는 네이티브 Windows의 handle 수명 기반 해제. |
| `crates/volicord-platform-fs/tests/mutation_lease_process.rs` | 프로세스 간 공유·배타 변경 lease 경합과 프로세스 종료 시 해제 regression. |
| `crates/volicord-cli/src/host_integration/process.rs` | 플랫폼 경계 관찰을 바탕으로 한 프로세스 target 검증, target 경로 파일시스템 제한 집행, 정규 플랫폼 diagnostic 표시 projection. |

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
| `crates/volicord-store/src/mutation.rs` | 복제할 수 없고 permit을 빌리며 정확한 target에 결합된 `RuntimeHomeMutationContext`, 유지되는 `CanonicalRuntimeHomePath`, 승인 뒤 재정규화 없는 typed identity 비교, 공유·배타 mode 검사, 안정적인 setup-in-progress condition projection. |
| `crates/volicord-store/src/sqlite.rs` | 분리된 읽기 전용 open과 정확한 Runtime Home 소유권을 검증하는 crate-private context-gated 쓰기 가능 Registry/project database open. |
| `crates/volicord-store/src/bootstrap.rs` | Context 소유 Runtime Home staging과 project 조회, 불투명 publication provenance, 기존 대상을 교체하지 않는 원자적 publication 결과, typed publication identity, token-backed terminal rollback 상태, composite 확인 실패, Store bootstrap. |
| `crates/volicord-store/src/diagnostics.rs` | 비권한 진단 schema와 manifest, 같은 directory의 staged carrier 공개, 동시 승자 검증, 정확한 read/write, retention. |
| `crates/volicord-store/src/setup_transaction.rs` | Setup이 변경하는 기존 Store 파일의 명시적인 prepare, 입력 검증, mutation checkpoint, commit, guarded rollback 경계. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection 레코드, project allowlist, managed fingerprint, 영속 검증 보고서 경계. |
| `crates/volicord-store/src/diagnostic_findings/mod.rs` | Lifecycle별 진단 영속화 facade와 공개 Store API export. |
| `crates/volicord-store/src/diagnostic_findings/occurrence.rs` | 추가 전용 occurrence 영속화와 원자적 runtime terminal-finding 연결. |
| `crates/volicord-store/src/diagnostic_findings/current_state.rs` | Current snapshot 활성화, 교체, 해소, 재활성화. |
| `crates/volicord-store/src/diagnostic_findings/graph.rs` | Cause graph 검증, 현재 보고서 root 선택, 한도가 있는 결정적 lifecycle-aware 정확한 순회. |
| `crates/volicord-store/src/diagnostic_findings/queries.rs` | Lifecycle-aware 정확한 식별자, 현재 보고서 projection, runtime-session occurrence, 활성 current scope 조회. |
| `crates/volicord-store/src/diagnostic_findings/row.rs` | 내부 finding row 인코딩, 디코딩, lifecycle identity 검증. |
| `crates/volicord-store/src/managed_launch_leases.rs` | 수명이 짧은 일회성 managed MCP launch lease, 현재 Connection 재검증, 결정적인 취소·만료 정리, 원자적 lease 소비와 runtime 생성을 담당합니다. |
| `crates/volicord-store/src/operational_sessions.rs` | Runtime-session source 디코딩, protocol milestone, revision 범위 managed MCP project session, 정확한 데이터베이스 간 binding, lease 소비 밖의 직접 `managed_host` 생성 거절. |
| `crates/volicord-store/src/integration_verification/mod.rs` | 공개 Store facade, 안정적인 integration-verification 입력과 레코드, lifecycle 구현을 위한 한정된 export. |
| `crates/volicord-store/src/integration_verification/begin.rs` | 검증 생성, 정확한 coordinate 재개, coordinate 변경의 terminal 처리, typed retry eligibility, 현재 prompt 선택, 하나의 즉시 Registry transaction 안에서 수행하는 영속 ID 할당. |
| `crates/volicord-store/src/integration_verification/probe.rs` | 최초 쓰기 probe acknowledgement, 정확한 활성 및 terminal replay, 하나의 즉시 Registry transaction 안에서 수행하는 동시 호출 수렴. |
| `crates/volicord-store/src/integration_verification/observation.rs` | Typed hook acquisition, Connection server의 `McpToolCatalog`를 통한 semantic callable filtering, 서로 다른 상관관계 불일치 stage, payload 없는 한도 내 관찰 영속화. |
| `crates/volicord-store/src/integration_verification/correlation.rs` | Prompt와 acquisition을 통과한 pre/post event matching, hook contract 및 tool-use 상관관계, timestamp 순서, 원자적 completion refresh. |
| `crates/volicord-store/src/integration_verification/status.rs` | 유효 lifecycle 상태, 최신 및 정확한 읽기, 공개 결과와 tagged workflow projection, 오래된 owner 처리. |
| `crates/volicord-store/src/integration_verification/coordinate.rs` | Typed caller, current, stored 검증 coordinate와 caller 및 run owner 검증. |
| `crates/volicord-store/src/integration_verification/row.rs` | 비공개 검증 SQL, row decoding, 상태와 timestamp parsing, 데이터베이스 표현 변환, 집중 row decoder 테스트. |
| `crates/volicord-store/src/integration_verification/tests/` | Begin, probe, typed acquisition, correlation, status, 동시 최초 acknowledgement를 위한 lifecycle 담당 테스트와 assertion에서 분리한 공유 fixture 구성. |
| `crates/volicord-store/src/workflow_records.rs` | 프로젝트 workflow policy record 읽기, workflow-policy mutation 입력과 적용, typed policy mutation 효과. |
| `crates/volicord-store/src/core_pipeline/mod.rs` | 공개 Core Store 타입 routing, commit 및 mutation 입력, transaction 수준 Store 테스트. |
| `crates/volicord-store/src/core_pipeline/facade.rs` | `CoreProjectStore` Connection 및 프로젝트 identity, 유지되는 mutation 권한, facade accessor, 공유 읽기 snapshot primitive. |
| `crates/volicord-store/src/core_pipeline/open.rs` | 명시적인 읽기 전용 open과 context의 typed canonical Runtime Home identity를 유지하는 mutation open. |
| `crates/volicord-store/src/core_pipeline/project_state.rs` | 프로젝트 상태 column projection, row decoding, timestamp 검증, facade 읽기. |
| `crates/volicord-store/src/core_pipeline/enforcement_profile.rs` | 프로젝트 enforcement profile projection, 엄격한 JSON decoding, 검증, facade 읽기. |
| `crates/volicord-store/src/core_pipeline/clock.rs` | Store handle clock sample, 프로젝트 UTC floor 읽기, transaction floor 전진. |
| `crates/volicord-store/src/core_pipeline/tasks.rs` | Task와 수락 mutation 입력, 저장 검증과 SQL 적용, Task·수락 기준·증거 주장·Task revision projection, facade 읽기, 집중 테스트. |
| `crates/volicord-store/src/core_pipeline/change_units.rs` | Change Unit mutation 입력, 저장 검증과 SQL 적용, projection, 엄격한 row 및 JSON decoding, facade 읽기, 집중 테스트. |
| `crates/volicord-store/src/core_pipeline/write_tickets.rs` | Write Ticket mutation 입력, 저장 검증과 SQL 적용, projection, 엄격한 row 및 JSON decoding, facade 읽기, 집중 테스트. |
| `crates/volicord-store/src/core_pipeline/runs.rs` | Run mutation 입력, 저장 검증과 SQL 적용, Run 및 observed-change projection, 엄격한 decoding, facade 읽기, 집중 테스트. |
| `crates/volicord-store/src/core_pipeline/evidence.rs` | 증거 mutation 입력, 저장 검증과 SQL 적용, 증거 요약 및 관찰 projection, 엄격한 row decoding, record reference projection, facade 읽기, 집중 테스트. |
| `crates/volicord-store/src/core_pipeline/artifacts.rs` | Artifact mutation 입력, 저장 검증과 SQL 적용, staging 및 영속 artifact projection, 엄격한 decoding, link 읽기, 영속 본문 검증, facade 읽기, 집중 테스트. |
| `crates/volicord-store/src/core_pipeline/user_actions.rs` | User Action mutation 입력, 저장 검증과 SQL 적용, 물리 JSON 및 저장 scalar에서 typed 요청·해결 레코드로의 엄격한 decoding, 유효 상태 파생, facade 읽기, 집중 테스트. |
| `crates/volicord-store/src/core_pipeline/continuity.rs` | Continuity mutation 입력, 저장 검증과 SQL 적용, 프로젝트 continuity projection, 한도 있는 snapshot page, facade 읽기, 집중 테스트. |
| `crates/volicord-store/src/core_pipeline/replay.rs` | Tool invocation projection, SQL, 엄격한 replay context decoding, 변경 불가능한 operation-result projection, facade 읽기. |
| `crates/volicord-store/src/core_pipeline/reconciliation.rs` | 확인된 expected-write 및 unrecorded-change 관찰 후보 projection과, 닫기 준비 상태 사실 취득에 사용하는 현재 handle 기반 미조정 변경 읽기. |
| `crates/volicord-store/src/core_pipeline/blockers.rs` | 활성 blocker reference query와 facade 읽기. |
| `crates/volicord-store/src/core_pipeline/events.rs` | 프로젝트 authority event identity 조회. |
| `crates/volicord-store/src/core_pipeline/agent_sessions.rs` | Guard 소유 엄격한 row reader를 사용하는 프로젝트 로컬 Agent Session facade 진입점. |
| `crates/volicord-store/src/core_pipeline/record_refs.rs` | Aggregate 읽기가 공유하는 저장 record reference 표현. |
| `crates/volicord-store/src/core_pipeline/inspection.rs` | 검증 경로에서 사용하는 무효과 프로젝트 저장소 counter. |
| `crates/volicord-store/src/core_pipeline/mutations.rs` | Grouped `CoreStorageMutation` routing, 정적 aggregate dispatch, transaction 범위 mutation context, typed aggregate 적용 결과. |
| `crates/volicord-store/src/core_pipeline/commit.rs` | Replay 및 최신성 gate, 순서 있는 aggregate 위임, state-version 전진 한 번과 정규 commit timestamp 하나, 원자적 event·replay·response 영속화, rollback, 최종 commit 결과. |
| `crates/volicord-store/src/core_pipeline/validation.rs` | 현재 Store 담당 모듈이 공유하는 저장 값 및 mutation 입력 검증. |
| `crates/volicord-store/src/guards.rs` | Typed host 상관관계 정규화, MCP 전용 project anchor, phase별 Guard 관찰, prompt capture, 예상 쓰기, suppression 입력. |
| `crates/volicord-store/src/evidence_capture.rs` | Evidence-capture intent와 producer 레코드. |
| `crates/volicord-store/src/artifacts.rs` | 아티팩트 staging과 영속 본문 검증. |
| `crates/volicord-store/src/runtime_home.rs` | Runtime Home 선택과 경로 경계 검증, runtime-path failure를 거치는 typed 플랫폼 diagnostic 전파. |
| `crates/volicord-store/src/operational_diagnostics.rs` | Typed Runtime Home 및 Store finding projection, 플랫폼 담당 finding identity와 action 정책의 직접 보존. |
| `crates/volicord-store/src/error.rs` | Store 실패 분류와 typed 플랫폼 diagnostic 보존. |

## UserAction 서비스

| 경로 | 책임 |
|---|---|
| `crates/volicord-user-action-service/src/lib.rs` | Typed context, intent, fact, 서비스 오류, 책임 담당 함수를 위한 좁은 공개 routing. Core 또는 어댑터 facade는 제공하지 않습니다. |
| `crates/volicord-user-action-service/src/model.rs` | 의미 intent, 검증된 구성 값, 명시적인 구성 및 영속화 context, adapter-neutral pending/current/resolution fact. |
| `crates/volicord-user-action-service/src/validation.rs` | Action kind, 좌표, 권한을 갖는 조합, 연산 대상, 만료 의미의 순수 검증과 정규화. |
| `crates/volicord-user-action-service/src/body.rs` | 검증된 intent와 취득한 fact에서 정규 typed `UserActionRequestBody`와 `UserActionBasis`를 순수하게 구성합니다. |
| `crates/volicord-user-action-service/src/identity.rs` | 안정적인 source identity, 중복 제거 metadata, 집중된 request identity 가용성 검사. |
| `crates/volicord-user-action-service/src/service.rs` | Core 요청 조율 없이 typed 구성, artifact, target, pending, resolved authority fact를 Store에서 취득합니다. |
| `crates/volicord-user-action-service/src/materialization.rs` | Caller가 제공한 연산 identity를 적용하고 정규 공개 request와 불변 resolution을 구성합니다. |
| `crates/volicord-user-action-service/src/persistence.rs` | 정규 request 또는 resolution 값을 Store mutation 입력으로 정확하게 typed 매핑합니다. |
| `crates/volicord-user-action-service/src/authority.rs` | Store가 decoding한 typed 레코드에서 정규화된 authority와 공개 request를 투영합니다. |
| `crates/volicord-user-action-service/src/lifecycle.rs` | 현재 pending authority fact에서 투영된 Task lifecycle을 순수하게 해석합니다. |
| `crates/volicord-user-action-service/src/resolution.rs` | 현재 basis 검증, 정규 typed resolution 구성, replay 입력 비교. |
| `crates/volicord-user-action-service/src/continuity.rs` | 권한을 갖는 수락 resolution에서 continuity draft를 의미적으로 파생합니다. |
| `crates/volicord-user-action-service/src/projection.rs`, `summary.rs` | Adapter-neutral pending, resolution, instruction, safe-summary fact. |
| `crates/volicord-user-action-service/src/tests/` | 책임별 validation, body, identity, authority, lifecycle, materialization, persistence, resolution, continuity, projection 테스트. |

## Core

| 경로 | 책임 |
|---|---|
| `crates/volicord-core/src/pipeline.rs` | 분리된 읽기 전용 경로 및 승인 context 기반 `CoreService` 구성, typed Core/Store Runtime Home 권한 검사, 공통 사전 점검, replay, plan 선택, 응답, commit 조율, 정규 플랫폼 diagnostic code를 포함한 Store 오류 세부사항 projection. |
| `crates/volicord-core/src/methods/` | 메서드별 구조 검증과 계획. 프로덕션 메서드 모듈은 공유 helper, pipeline과 정책 함수, Store 서비스, 공유 타입을 각 담당 모듈에서 명시적으로 가져오며 상위 모듈을 import prelude로 사용하지 않습니다. |
| `crates/volicord-core/src/methods/evidence_facts.rs` | 증거 정책 분류를 담당하지 않으면서 저장된 증거와 투영된 증거를 위한 typed 사실을 취득하는 공유 Store 조회와 엄격한 디코딩. |
| `crates/volicord-core/src/methods/close_readiness/mod.rs` | 메서드 plan이 사용하는 닫기 준비 상태 서비스, projection, 차단 사유 helper의 좁은 패키지 표면. |
| `crates/volicord-core/src/methods/close_readiness/facts.rs` | 하나의 수락 기준 snapshot, 하나의 workflow policy snapshot, 현재 handle 기반 미조정 변경 읽기를 포함한 typed 현재 사실 취득 및 투영 사실 조립. 준비 상태 판단은 담당하지 않습니다. |
| `crates/volicord-core/src/methods/close_readiness/change_control.rs` | Task, Change Unit, 닫기 근거, baseline, 복구, 미조정 변경, Write Ticket 조건 평가. |
| `crates/volicord-core/src/methods/close_readiness/evidence.rs` | 집중 증거 사실 및 순수 정책 담당 모듈을 통한 닫기 증거와 아티팩트 가용성 평가. |
| `crates/volicord-core/src/methods/close_readiness/acceptance.rs` | 대기 중인 닫기 권한, 취소, 민감 작업 승인, 최종 수락, 잔여 위험 수락 평가. |
| `crates/volicord-core/src/methods/close_readiness/policy.rs` | Store와 독립적인 유효 control 해석 및 typed 준비 상태 평가를 닫기 상태로 만드는 순수한 순서 결합. |
| `crates/volicord-core/src/methods/close_readiness/blockers.rs` | 정규 typed 닫기 차단 사유 구성, Write Ticket 차단 사유 projection, 여러 차단 사유에 걸친 action 정규화. |
| `crates/volicord-core/src/methods/close_readiness/guidance.rs` | Typed 담당 메서드와 연산 범주를 포함하는 adapter-neutral 의미 기반 후속 행위 선택. CLI 문법, 캡처 경로, Markdown, rendering, credential은 담당하지 않습니다. |
| `crates/volicord-core/src/methods/close_readiness/summary.rs` | 전체 닫기 연산 평가와 의도적으로 더 작은 메서드 중립 준비 상태 projection. |
| `crates/volicord-core/src/methods/close_readiness/service.rs` | 사실 취득, 책임별 평가, 순수 정책 결합, 전체 닫기 평가, 메서드 중립 요약 projection의 좁은 조율. |
| `crates/volicord-core/src/methods/close_readiness/tests/` | 책임별 사실, 변경 제어, 증거, 수락, 정책, 차단 사유, guidance 테스트와 닫기 준비 상태 서비스 통합 coverage. |
| `crates/volicord-core/src/methods/prepare_evidence_capture.rs` | 증거 캡처 요청 검증과 계획. 수락 기준 및 보충 주장 일치에는 대상 정책을 사용합니다. |
| `crates/volicord-core/src/methods/record_run.rs` | 실행 및 증거 갱신 검증과 계획. 출처, 관련성, 대상, 결속, 닫기 준비 상태 증거 정책을 사용합니다. |
| `crates/volicord-core/src/methods/close_task.rs` | 요청별 닫기 조율. 요청 검증, 닫기 준비 상태 서비스 호출, 종료 변경 계획, typed 결과 구성을 담당합니다. |
| `crates/volicord-core/src/methods/update_scope.rs` | 닫기 준비 상태 증거 정책 담당 모듈을 통한 범위 갱신 계획과 투영 증거 요약 완성. |
| `crates/volicord-core/src/methods/status.rs` | 공유 Core 투영 경로를 통해 닫기 준비 상태 증거 정책을 사용하는 읽기 전용 상태 투영. |
| `crates/volicord-core/src/methods/user_action.rs` | 직접 request와 resolution 메서드 조율. 공유 typed UserAction 서비스를 사용하고 결과를 메서드 plan과 response로 매핑합니다. |
| `crates/volicord-core/src/methods/user_action_read.rs` | User Channel 권한 검사, 일관된 Store snapshot, 원래 결과 replay, 공개 메서드 결과 projection. |
| `crates/volicord-core/src/methods/user_action_continuity.rs` | Store fact 취득, Core 소유 continuity 식별자와 timestamp, 서비스 draft 사용, 영속화 순서 조율. |
| `crates/volicord-core/src/methods/reconcile_changes.rs` | Reconciliation별 계획. 해결되지 않은 변경에 typed pending action이 필요할 때 UserAction 서비스를 직접 사용합니다. |
| `crates/volicord-core/src/policy/` | 책임별 재사용 정책. 메서드 구현은 형제 메서드 모듈에서 공유 정책을 얻지 않고 이 담당 모듈을 직접 사용합니다. |
| `crates/volicord-core/src/policy/evidence_provenance.rs` | Typed 사실에 대한 순수 증거 출처 및 보증 수준 분류. |
| `crates/volicord-core/src/policy/evidence_relevance.rs` | 순수 증거 관련성 및 뒷받침 여부 분류. |
| `crates/volicord-core/src/policy/evidence_target.rs` | 증거 대상, 관찰 근거, `CurrentCloseBasis` 일치 정책. |
| `crates/volicord-core/src/policy/evidence_binding.rs` | 생산자 참조, 생산자 출력, 정확한 아티팩트 결속 정책. |
| `crates/volicord-core/src/policy/close_readiness_evidence.rs` | 닫기 준비 상태 증거 해석, 필수 수락 기준 요약 완성, 증거 게이트 평가. |
| `crates/volicord-core/src/agent_session.rs` | 현재 Connection, project membership, mode, 관리 runtime/project session 검증. |
| `crates/volicord-core/src/authority_status.rs` | typed status와 authority receipt 대응. |

## 명령 모델

| 경로 | 책임 |
|---|---|
| `crates/volicord-command-model/src/lib.rs` | `volicord` 바이너리의 완전한 Clap 명령 선언, root parser, 공개 및 숨은 하위 명령 tree, 명령과 인수 DTO, 명령 표면의 value enum과 문법 validator, root `clap::Command` 구성, 실제 모델 기반 가시성 분류, 명령 경로 순회, 정규 synopsis 렌더링, 공개 invocation 검증, parsing 가능한 정규 공개 invocation 생성, 같은 선언에서 경로와 option spelling을 도출하고 결과를 parse-check하는 typed inbox-resolution invocation builder를 담당합니다. |

## UserAction Presentation

| 경로 | 책임 |
|---|---|
| `crates/volicord-user-action-presentation/src/lib.rs` | Adapter-neutral UserAction fact에서 `CliUserActionInboxResponse`, `CliUserActionInboxItem`, 폐쇄형 channel/capture-path 상태, CLI JSON Schema, recovery instruction을 만드는 typed CLI projection. Command syntax는 typed `volicord-command-model` invocation에서만 얻으며 Core 정책, Store read, command 실행, terminal rendering, MCP envelope는 담당하지 않습니다. |

## CLI와 Codex 어댑터

| 경로 | 책임 |
|---|---|
| `crates/volicord-cli/src/main.rs` | 프로세스 진입, `volicord-command-model`을 통한 parsing, 관리 명령 디스패치. |
| `crates/volicord-cli/src/mutation_admission.rs` | 정확한 Runtime Home 해석, 연산별 `SharedWriter` 획득, Store mutation context 구성, 안정적인 typed busy mapping, 변경 CLI 및 Guard 연산 전체의 lease 유지. |
| `crates/volicord-cli/src/host_launch.rs` | 숨은 동일 프로세스 host launcher, 현재 Codex entry의 정확한 재검증, launch-lease 발급·정리, managed stdio로의 메모리 내 전환. |
| `crates/volicord-cli/src/connection_command/` | connection add, list, status, verify, mode, remove 조율. |
| `crates/volicord-cli/src/connection_command/service.rs` | Init과 Connection add에서 planning 전에 `ExclusiveSetup` 변경 승인을 획득하고 정규 Runtime Home lease를 dry-run 보고, commit, 정리 또는 rollback까지 유지하는 잠긴 setup service 경계. |
| `crates/volicord-cli/src/connection_command/setup_transaction.rs` | `volicord init`의 typed `SetupPlan`, 명시적인 Runtime Home publication 소유권·제거 효과 상태, 같은 directory의 원자적 파일 mutation, freshness 검증, 결정적인 commit, 효과를 인식하는 Project Home 정리, guard로 제한한 rollback. |
| `crates/volicord-cli/src/connection_command/verification/mod.rs` | Connection 검증 조율, 공유 step/report 타입, 한도가 있는 패키지 export. |
| `crates/volicord-cli/src/connection_command/verification/host_checks.rs` | Managed configuration, host executable, project trust, managed-host session check. |
| `crates/volicord-cli/src/connection_command/verification/mcp_checks.rs` | MCP preflight/handshake check 투영과 MCP finding ID 입력. |
| `crates/volicord-cli/src/connection_command/verification/guard_checks.rs` | Guard 파일, hook execution, observation check 평가. |
| `crates/volicord-cli/src/connection_command/verification/dependency_graph.rs` | Cause 부착, `Blocked` 전파, graph 확정, 현재 activation-plan suffix와 typed repair 선택, 정규 check 구성. |
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
| `crates/volicord-cli/src/connection_command/output/` | 선택한 Connection의 정규 진단 보고서 구성, 집계 상태와 root, typed Runtime Home rollback 효과·내구성 출력, 두 번째 renderer 소유 step 목록 없이 같은 필수·선택 activation plan을 concise·verbose·lossless JSON으로 표시. |
| `crates/volicord-cli/tests/init_record_regression.rs` | Init plan/read-only, replay, 정확한 소유자 record, rename 뒤 및 모든 단계 setup fault injection, 두 invocation 순서의 결정적인 배타 변경 승인 성공·rollback 경합, busy 및 dry-run 비변경, 예상하지 않은 외부 publication 중단, 동시 파일 변경, 전체 rollback, partial-rollback 보고 regression. |
| `crates/volicord-cli/src/diagnostics_command.rs` | Finding ID 및 runtime-session 세부 명령, 한도가 있는 lifecycle-aware cause traversal, lookup별 JSON 및 사람용 projection, finding severity와 독립적인 lookup-status 종료 결과. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex 구성 parsing 및 직렬화, 정규 관리 entry 검증, 허용된 도구 승인 overlay 보존, 관리 구성 변경, 진단용 실행 파일 관찰, 연결 검증. |
| `crates/volicord-cli/src/host_integration/contracts.rs` | 명시적인 semantic Codex host-contract 선택, typed Guard routing strategy 투영, 등록된 `McpServerKey`로부터의 엄격한 구성 재구성. |
| `crates/volicord-cli/src/guard_integration/manifest.rs` | Guard manifest, 정확한 host-contract profile/digest, 정규 관리 artifact 기대값 생성. |
| `crates/volicord-cli/src/guard_integration/audit.rs` | 현재 Guard 소유자, artifact, command, marker, executable 동작 audit. |
| `crates/volicord-cli/src/guard_integration/plan.rs` 및 `hosts/codex.rs` | Nested integration-verification sequence, stop 규칙, diagnostic 경계를 포함한 managed AGENTS 및 Codex rule 안내 source template. |
| `crates/volicord-cli/src/guard_command/` | 명시적인 `codex-command-hooks` event decoding, semantic Guard probe filtering, routing된 MCP payload를 보관하지 않는 한도 있는 source별 observation. |
| `crates/volicord-cli/src/user_command.rs` | CLI 받은 편지함과 local-user resolution. 승인 전에는 구문과 repository target만 처리하고, 같은 mutation context를 유지한 채 승인 뒤 Registry/project 선택, neutral Core fact 사용, 공유 UserAction presentation, 단일 snapshot 후보 계획, 진단, Core 효과, terminal 응답 표시를 수행합니다. |
| `crates/volicord-cli/src/doctor_command.rs` | 진단 사실 수집과 표시. |

## MCP 프로토콜 프로필

| 경로 | 책임 |
|---|---|
| `crates/volicord-mcp-protocol/src/lib.rs` | 폐쇄형 MCP 리비전 타입 파싱, 정확한 프로덕션 프로필 조회, 유일한 revision-to-semantic-capability map, 결정론적인 지원 리비전 순회, 추적 중인 사전 릴리스 분류, 알 수 없거나 지원하지 않는 값의 명시적 거절. |
| `crates/volicord-mcp-protocol/tests/protocol_registry.rs` | 고정 매니페스트 일치, 완전한 semantic/schema capability 일치, registry 유일성, 결정론적 순회, 정확한 파싱과 선택, 선호 리비전 포함, 사전 릴리스 배제 검증. |

## MCP 어댑터

| 경로 | 책임 |
|---|---|
| `crates/volicord-mcp/src/lib.rs` | 어댑터 소유 공개 진입점과 경계 타입. 공유 타입과 도구 identity는 각 `volicord-types` 담당 모듈 경로에 남습니다. |
| `crates/volicord-mcp/src/managed_launch.rs` | 정규 typed 개인/공유 숨은 launcher 명령과 인자, Runtime Home 환경 binding, 엄격한 시작 형태 검증, 공개 수동 probe 구체화, projection, fingerprint 입력. |
| `crates/volicord-mcp/src/mutation_admission.rs` | Message 및 tool별 `SharedWriter` 획득, Store context 구성, typed setup-busy 전파, 전체 MCP 효과 동안의 한정된 lease 수명. |
| `crates/volicord-mcp/src/stdio.rs` | 공개 수동 stdio와 메모리 내 lease에 결속된 managed stdio facade. 진입 경로 binding을 선택하고 연결된 stream을 위임하며 protocol, lifecycle, tool dispatch 구현을 보유하지 않습니다. |
| `crates/volicord-mcp/src/transport.rs` | 한도가 있는 줄바꿈 구분 stdio 읽기·쓰기, UTF-8 및 frame 한도 집행, transport loop 종료, 디코딩한 JSON 값을 lifecycle 처리로 위임하는 경계. |
| `crates/volicord-mcp/src/json_rpc.rs` | JSON 구문 디코딩, JSON-RPC envelope 분류, 문자열·정수 request ID 검증, object parameter 검증, Core 접근 없는 성공·오류 응답 구성. |
| `crates/volicord-mcp/src/lifecycle.rs` | 정확한 initialize profile 선택, initialized notification 승인, capability 기반 batch와 method별 lifecycle 유효성, runtime session 시작·종료, 폐쇄형 `SessionState` variant인 `AwaitingInitialization`, `AwaitingInitializedNotification`, `InitializedAndReady`, `Closed`. Initialization 선택 정보는 initialized variant에만 있고 종료 정보는 `Closed`에만 있습니다. |
| `crates/volicord-mcp/src/binding.rs` | Runtime Home 해석, repository 탐색, Connection/project 사전 점검과 binding, managed Codex session/thread/turn 상관관계. |
| `crates/volicord-mcp/src/tool_dispatch.rs` | `tools/list`와 `tools/call` parameter 디코딩, 정규 도구 선택, adapter/Core 호출, 공유 정규 tool-result carrier 조립. Transport message framing이나 mutation, recovery, UserAction, metric projection은 담당하지 않습니다. |
| `crates/volicord-mcp/src/mutation_projection.rs` | Mutation detail 선택, effect anchor 구성, 간결한 method-result projection, 새 authority 구성, capability 기반 정상 결과 예산 집행. |
| `crates/volicord-mcp/src/authority_refresh.rs` | Mutation 뒤 Agent Session binding, 현재 authority 다시 읽기, 좌표 검증, 새 authority receipt와 next action 추출. |
| `crates/volicord-mcp/src/committed_result_recovery.rs` | Mutation을 다시 시도하지 않으면서 committed mutation projection, refresh, post-effect failure 뒤 capability가 선택하는 authority 우선 bounded recovery. |
| `crates/volicord-mcp/src/user_action_projection.rs` | Committed UserAction 좌표 추출, neutral current fact 다시 읽기, adapter 소유 safe MCP 결과 구성, neutral failure mapping, 공유 CLI inbox fallback 부착. |
| `crates/volicord-mcp/src/telemetry.rs` | Runtime session finding과 diagnostic event 영속화, 계약이 허용하는 diagnostic carrier failure의 한정된 best-effort 처리. |
| `crates/volicord-mcp/src/session_metrics.rs` | Diagnostic session 생성과 session 범위 tools-list, method-call, status-reread workflow metric. |
| `crates/volicord-mcp/src/diagnostics.rs` | 폐쇄형 MCP diagnostic mapping, 공유 finding 구성, bootstrap 및 영속 terminal projection에서 플랫폼 담당 diagnostic code와 action class 보존. |
| `crates/volicord-mcp/src/adapter.rs` | 유지되는 연산 전 routing identity, 활성 mutation-context 상관관계, context에 결합된 Core 호출 API, Store 소유 workflow projection을 adapter-local 상태 파생 없이 직렬화하는 Core 밖의 managed in-chat begin/probe/get integration-verification 조율. |
| `crates/volicord-mcp/src/constants.rs` | 사용자 수준 verification 요청, nested workflow-directed sequence, stop 규칙, unavailable 경계, 선택적 active diagnostics를 설명하는 MCP initialize instruction. |
| `crates/volicord-mcp/src/tool_registry.rs` | `AgentToolId`로 식별한 schema, annotation, 효과 설명, metadata, method lookup을 세 Connection-integration 도구를 포함한 정규 도구 정의/결과로 조립하고 semantic capability만으로 wire projection을 수행하며 명시적 server를 사용하는 충돌 검사 Codex callable catalog를 구성하는 구현. |
| `crates/volicord-mcp/src/schema_validation.rs` | 공개 schema 검증. |
| `crates/volicord-mcp/src/routing.rs` | 결속된 Product Repository 탐색, 현재 Connection/project routing, 정규 catalog에서 server/raw/callable identity를 가져오는 preflight diagnostic 투영. |

## 테스트

| 경로 | 책임 |
|---|---|
| `crates/*/tests/`와 module-local `tests` | crate 경계와 unit test. |
| `crates/volicord-command-model/src/lib.rs` module test | Clap 구조 assertion, 완전한 공개 순회, 숨은 하위 tree 배제, 정규 invocation 자체 parsing, typed inbox-resolution invocation round trip, 현재 필수 인수·충돌·값 집합 동작. |
| `crates/volicord-mcp/src/transport.rs`, `json_rpc.rs`, `binding.rs` module test | 각 구현 담당 모듈에서 frame 한도와 drain, delimiter와 UTF-8 동작, request ID와 notification 분류, 정확한 managed call metadata, Runtime Home binding failure를 검증합니다. |
| `crates/volicord-mcp/src/tests/lifecycle.rs` | Initialization 순서, 거절, 종료, EOF 계약. |
| `crates/volicord-mcp/src/tests/batching.rs` | JSON-RPC batch 순서, notification, 응답 계약. |
| `crates/volicord-mcp/src/tests/protocol_projection.rs` | Registry/profile wire projection과 schema 호환성 계약. |
| `crates/volicord-mcp/tests/protocol_conformance.rs` | 공통 initialize, lifecycle, schema, discovery, result carrier, rejection, batching, shutdown 시나리오를 위한 단일 실행 가능 프로덕션 profile harness. |
| `crates/volicord-mcp/src/tests/tool_calls.rs` | Tool dispatch, 결과, 오류, 저장소 capability 계약. |
| `crates/volicord-mcp/src/tests/managed_host_observation.rs` | Lease 결속 managed launch, process 환경의 비권위성, runtime source routing, session binding, host 관찰 계약. |
| `crates/volicord-mcp/src/tests/diagnostics.rs` | 진단 영속화와 workflow metric 계약. |
| `crates/volicord-mcp/src/tests/conformance.rs` | 모듈 수준 registry 기반 protocol conformance assertion. |
| `crates/volicord-mcp/src/tests/support.rs` | 공유 MCP 테스트 fixture와 protocol message 구성만 담당. |
| `crates/volicord-mcp/tests/protocol_conformance.rs` | 모든 프로덕션 profile에 적용하는 하나의 registry 기반 wire 적합성 case. 고정 schema 검증, 필수 도구, 지정 왕복, profile별 projection 및 batching, lifecycle 거절, EOF를 다룹니다. |
| `crates/volicord-cli/tests/binary_admin.rs` | 실제 바이너리 관리 CLI parser, help, output, exit 계약. |
| `crates/volicord-cli/tests/operational_host_e2e.rs` | 적용된 setup부터 lease-bound MCP와 정확한 Guard prompt/pre/post 검증을 거쳐 complete 읽기 전용 status에 이르는 전체 managed Codex activation journey 및 운영 실패·정리 regression. |
| `tests/conformance/` | 교차 메서드 conformance scenario. |
| `tests/conformance/mcp-spec/` | 오프라인 적합성 입력으로 쓰는 버전별 공식 MCP schema, release 및 handshake-family metadata, 검토된 `production_supported`와 `pre_release_only` 사실, 변경 불가능한 upstream pin, 라이선스 저작자 표시, checksum. |
| `tests/release-integrity/` | 일반 target 다섯 개, 버전, 기준 바이트, 패키지, checksum, CI 및 릴리스 workflow 의미 테스트. Build/smoke/staging 순서, matrix binary input, 정확히 한 번인 action 사용, 경로 filter, 의존 방향을 포함합니다. |
| `tests/release-smoke/Cargo.toml` | 게시하지 않는 전용 스모크 패키지 경계. Protocol, 정규 tool type, 공유 한도 테스트 프로세스에는 의존하지만 CLI library, MCP 구현, Core, Store, `xtask`에는 의존하지 않습니다. |
| `tests/release-smoke/src/lib.rs` | 전달받은 실제 바이너리 orchestration, 폐기 가능한 Git Product Repository 및 Runtime Home fixture, 선호 리비전 initialize와 `tools/list` transcript 검증, 정규 대표 도구 assertion, 릴리스 전용 프로세스 한도, 스모크 결과 보고, 집중 transcript 실패 테스트. |
| `tests/release-smoke/src/main.rs` | 패키지 명령 진입점과 복사된 `codex` 또는 `codex.exe` identity로 선택되는 비공개 안정적 Codex fixture 동작. |
| `tests/release-smoke/tests/` | 전달받은 바이너리의 성공 흐름, 안정적인 Codex fixture 및 미지원 호출, 누락 및 실행 불가능 바이너리, 테스트 소유 Volicord 프로세스 동작, 한도 프로세스 timeout 및 정리 coverage. |
| `.github/actions/volicord-release-smoke/action.yml` | binary path input 하나를 받는 재사용 workflow 수준 실제 바이너리 스모크 호출. |
| `crates/volicord-test-support/` | 재사용 fixture만 담당합니다. 일회용 Runtime Home, repository, Store 쪽 설정 및 검사, 의도적인 손상·비정상 저장소 설정, 요청 도우미를 제공합니다. 제품 동작 assertion은 owner별 test에 남고 구현 테스트 모듈은 저장소 SQL을 직접 포함하지 않습니다. |
| `crates/volicord-test-process/tests/` | 플랫폼 공통 한도 자식 실행, stdin, 실패, 시간 초과, truncation, 동시 stream, 자손이 유지하는 pipe, 정리, 프로세스 격리, 경로, 인자, 환경 coverage. 네이티브 Unix와 Windows case는 `volicord-platform-process`가 선택한 플랫폼 격리를 실행합니다. |

## 저장소 유지보수 도구

| 경로 | 책임 |
|---|---|
| `xtask/Cargo.toml` | 가벼운 유지보수 의존 경계. `volicord-command-model`에서 문서 예시용 공개 명령 grammar를 받고, `volicord-types`에서 런타임 담당 계약 식별자를 받으며, `volicord-mcp-protocol`에서 고정 명세 일치 검사용 프로덕션 profile을 받습니다. `volicord-mcp`, Core, Store, CLI, platform, test-process 크레이트는 끌어오지 않습니다. |
| `xtask/src/lib.rs` | 간결한 저장소 점검 조합과 공개 보고 타입 재노출. |
| `xtask/src/diagnostics.rs` | 경로, 범주, 선택적 줄 번호, 메시지를 담는 공통 검증 이슈 표현. |
| `xtask/src/doc_index.rs` | 현재 문서 색인 스키마, 적용 가능성과 정확한 의미 기반 계약 경로, 담당 경로, 색인 경로, 유지 문서 coverage. |
| `xtask/src/markdown.rs` | 공유 Markdown event parsing, 제목 의미 단위, 지원되는 계약 리터럴 구성. |
| `xtask/src/links.rs` | 로컬 Markdown 대상 해석, 링크, fragment, anchor. |
| `xtask/src/parity.rs` | 한영 제목 구조 일치. |
| `xtask/src/terminology.rs` | 용어 지도 경로와 신원 민감 역할 검증. |
| `xtask/src/cli_docs.rs` | `docs-sync` 조합, 생성 관리 CLI 영역, `volicord-command-model`을 통한 문서 invocation 검증. 셸 tokenization은 두 번째 명령 grammar가 아닙니다. |
| `xtask/src/document_structure.rs` | 현재 아키텍처 설계 절과 표면 안정성 구조. |
| `xtask/src/contract_identifiers.rs` | 현재 공개 스키마, 명령 모델, typed 진단, 프로토콜 레지스트리 식별자 도출, 대응 의미 단위 검증, 작업 범주 표 일치. |
| `xtask/src/workspace_manifests.rs` | 공유 workspace manifest parsing과 현재 package 및 Rust 적용 가능성 값. |
| `xtask/src/architecture.rs` | Cargo metadata에서 가져온 package manifest, target source root, 의존 edge, 패키지 수준 아키텍처 검증, 한영 생성 책임 및 의존 영역, 생성 영역 drift 검사, 정보 제공용 유지보수성 보고. |
| `xtask/src/release_metadata.rs` | workspace release version 상속과 release tag 검증. |
| `xtask/src/storage.rs` | 기준 Storage DDL 문서 검증. |
| `xtask/src/artifact_hygiene.rs` | `.gitignore`가 담당하는 저장소 아티팩트 제외 규칙과 Git 색인의 일치 검증. |
| `xtask/src/repository.rs` | 집중 validator가 사용하는 공유 repository 경로 정규화. |
| `xtask/src/mcp_spec/mod.rs` | MCP 명세 유지보수 facade와 명령 진입점. |
| `xtask/src/mcp_spec/manifest.rs` | 엄격한 고정 manifest 모델, parsing, 결정론적 rendering. |
| `xtask/src/mcp_spec/validation.rs` | 오프라인 metadata, 변경 불가능한 pin, checksum, artifact, schema, ordering, registry 일치 검증. |
| `xtask/src/mcp_spec/report.rs` | 결정론적 검사와 동기화 보고 타입. |
| `xtask/src/mcp_spec/sync.rs` | 교체 전에 검증된 임시 후보를 사용하는 유일한 네트워크 MCP 명세 경로. |
| `xtask/tests/docs_check.rs` | 중립적인 공유 fixture 구성과 현재 문서 점검 테스트 조합. |
| `xtask/tests/docs_check/*.rs` | 현재 스키마, 링크, 구조, 계약 식별자, 용어, 아티팩트, CLI, 아키텍처 집중 테스트를 담당 validator별로 묶은 모듈. |
| `xtask/tests/mcp_spec.rs` | 엄격한 manifest parsing, 분류, 집합 불일치, 변경 불가능한 pin, checksum, 필수 artifact, ordering, 보고, 오프라인 성공 coverage. |

지속되는 책임이 이동하면 이 맵을 갱신합니다. 삭제된 경로, 생성 경로, 개인 scratch 경로를
나열하지 않습니다.
