# 코드베이스 둘러보기

이 문서는 Cargo 멤버, 소스 파일, 심볼, 의존 방향, 테스트를 따라 현재
Rust 워크스페이스를 설명합니다. 학습 가이드이며 계약 담당 문서가
아닙니다. 정확한 API 동작, 저장 효과, 스키마, 보안 보장, 런타임 경계,
Core 권한 의미는 참조 문서에 남습니다.

코드와 테스트 경로는 저장소 루트 기준으로 씁니다. 이 문서의 소스 링크는
바로 열 수 있도록 상대 Markdown 대상으로 둡니다. 워크스페이스 전체 Cargo
의존성 그래프와 런타임 지도는 [구현 아키텍처](architecture.md)를 사용합니다.
영향을 받는 영역을 찾은 뒤 검증 계층을 고를 때는 [테스트 전략](testing-strategy.md)을
사용합니다.

## 첫 번째 읽기 경로

공개 메서드 경로를 배울 때는 아래 순서로 읽습니다.

1. `volicord-types`: 타입 지정 요청, 응답, 값 집합, 식별자, 정규 해시
   형태를 봅니다.
2. `volicord-store`: Runtime Home, 프로젝트 Store, 아티팩트, 마이그레이션,
   커밋 경계를 봅니다.
3. `volicord-core`: 공유 요청 파이프라인, 메서드 계획, 정책, Store 조율을
   봅니다.
4. `volicord-mcp`: stdio 시작, 도구 등록, 타입 지정 인자 디코딩, 호출
   맥락 파생, 디스패치, 응답 래핑을 봅니다.
5. `volicord-test-support`, `tests/integration`, `tests/conformance`: 폐기
   가능한 픽스처와 계층 간 검증 지점을 봅니다.

관리 설정 동작은 `volicord-store` 뒤에 `volicord-cli`를 읽습니다. CLI 경로는
로컬 설정과 등록이며 공개 Core 메서드 동작이 아닙니다.

저장소 문서 검증은 Maintain 정책 뒤에 `xtask`를 읽습니다. 이 패키지는
유지보수 도구이며 공개 메서드 경로의 일부가 아닙니다.

## Cargo 의존 형태

현재 manifest에서 확인되는 일반 내부 Cargo 의존 방향은 아래와 같습니다.

- `volicord-types`는 내부 의존성이 없습니다.
- `volicord-store`는 `volicord-types`에 의존합니다.
- `volicord-core`는 `volicord-store`와 `volicord-types`에 의존합니다.
- `volicord-cli`는 `volicord-core`, `volicord-store`, `volicord-types`에 의존합니다.
- `volicord-mcp`는 `volicord-core`, `volicord-store`, `volicord-types`에 의존합니다.
- `volicord-test-support`는 `volicord-store`와 `volicord-types`에 의존합니다.
- `xtask`는 내부 제품 크레이트에 의존하지 않습니다. 문서 파서 의존성은
  유지보수 패키지 안에 격리됩니다.

테스트 전용 조합은 구현 크레이트에 `volicord-test-support`를 더하고,
`tests/conformance`와 `tests/integration`이 자신이 실행하는 구현 크레이트를
조합하게 합니다. 그래도 Core는 CLI나 MCP 어댑터에 의존하지 않습니다. 이 Cargo
의존성 그래프의 그림 버전은 [구현 아키텍처](architecture.md)에 있습니다.

## `crates/volicord-types`

존재 이유:

`volicord-types`는 공개 API와 도메인 형태 값을 위한 공유 Rust 타입
경계입니다. 어댑터, Core, Store, 테스트가 같은 serde 모델, JsonSchema 생성,
제어 값 타입, 불투명 식별자, 정규 요청 해시를 사용하게 합니다.

구현에서 담당하는 것:

- 지원 메서드의 공개 요청과 결과 Rust 형태.
- `ToolEnvelope`, `ToolResultBase`, `StateRecordRef`, `StateSummary`,
  `WriteCheckSummary`, `EvidenceSummary`, `CloseReadinessBlocker`,
  `ArtifactRef` 같은 공유 스키마 형태 구조체.
- `MethodName`, `OperationCategory`, `EffectKind`, `ResponseKind`, `ResumePolicy`,
  `PrepareWriteDecision`, `ErrorCode` 같은 제어 값 enum.
- 불투명 식별자 래퍼와 durable ID 생성 도우미.
- 결정적 정규 JSON과 요청 해시.

담당하지 않는 것:

- Core 메서드 동작.
- Store 변이, DDL, 마이그레이션, 저장 효과.
- MCP 또는 CLI 전송 동작.
- 스키마나 값 집합의 제품 계약 의미.

추천 첫 파일:

- [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs)

중요 모듈:

- [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs):
  `MethodOperationCategory`, 메서드 요청 구조체, 메서드 결과 구조체,
  `public_request_schema`.
- [`crates/volicord-types/src/schema.rs`](../../../crates/volicord-types/src/schema.rs):
  공유 요청 래퍼, 응답, 상태, 아티팩트, 판단, 표시 형태.
- [`crates/volicord-types/src/values.rs`](../../../crates/volicord-types/src/values.rs):
  제어 enum과 상수.
- [`crates/volicord-types/src/ids.rs`](../../../crates/volicord-types/src/ids.rs):
  ID 래퍼, `DurableIdKind`, `DurableIdGenerator`,
  `RandomDurableIdGenerator`, `SequenceDurableIdGenerator`.
- [`crates/volicord-types/src/canonical.rs`](../../../crates/volicord-types/src/canonical.rs):
  `canonical_json_string`, `canonical_json_sha256`, `canonical_request_hash`.

중요한 현재 심볼:

- `MethodOperationCategory`, `IntakeRequest`, `StatusRequest`,
  `PrepareWriteRequest`, `RecordRunRequest`, `CloseTaskRequest`
- `ToolEnvelope`, `ToolResponse`, `ToolRejectedResponse`,
  `ToolDryRunResponse`, `ToolError`, `DryRunSummary`
- `MethodName`, `OperationCategory`, `EffectKind`, `ResponseKind`, `ErrorCode`
- `RequiredNullable<T>`, `StateSummary`, `StateRecordRef`,
  `WriteCheckSummary`, `WriteCheckAttemptScope`
- `canonical_request_hash`, `DurableIdGenerator`, `DURABLE_ID_RETRY_LIMIT`

가장 관련 있는 테스트:

- [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs)의
  단위 테스트. 먼저 `typed_requests_derive_documented_operation_categories`,
  `unknown_top_level_fields_are_rejected_on_public_requests`,
  `authority_looking_request_fields_are_rejected`를 봅니다.

다음에 읽을 컴포넌트:

- 타입 지정 요청이 메서드 동작이 되는 과정을 보려면 `volicord-core`를
  읽습니다. MCP 인자가 이 타입 지정 요청으로 바뀌는 과정을 보려면
  `volicord-mcp`를 읽습니다.

## `crates/volicord-store`

존재 이유:

`volicord-store`는 SQLite 기반 Runtime Home과 프로젝트 Store 메커니즘을
담당합니다. 데이터베이스 열기, 스키마 검증, 로컬 기록 부트스트랩,
마이그레이션 적용, 설정 상태 검사, 아티팩트 스테이징, 저장소 실패 분류,
Core 변이 원자 커밋이 여기에 속합니다.

구현에서 담당하는 것:

- Runtime Home 해석과 registry/project 경로 도우미.
- Runtime Home 초기화, 프로젝트 등록, Agent Connection 등록.
- SQLite 열기, 스키마 검증, 마이그레이션, 트랜잭션 도우미.
- `CoreProjectStore` 읽기 도우미와 `CoreStorageMutation` 적용.
- `CoreProjectStore::commit_mutation` 원자 트랜잭션 경계.
- 일시적 아티팩트 스테이징과 영구 아티팩트 본문 검증 도우미.
- 설정과 진단에서 쓰는 읽기 전용 검사 스냅샷.
- `StoreError`, `StoreFailureRoute`, 저장소 실패 분류.

담당하지 않는 것:

- 공개 메서드 동작이나 메서드 정책.
- MCP 또는 CLI 어댑터 의미.
- `Product Repository` 제품 파일 쓰기.
- 정확한 저장소 계약, DDL 의미, 저장 효과 계약.

추천 첫 파일:

- [`crates/volicord-store/src/lib.rs`](../../../crates/volicord-store/src/lib.rs)

중요 모듈:

- [`crates/volicord-store/src/runtime_home.rs`](../../../crates/volicord-store/src/runtime_home.rs):
  `resolve_runtime_home`, `RuntimeHomeResolutionError`.
- [`crates/volicord-store/src/bootstrap.rs`](../../../crates/volicord-store/src/bootstrap.rs):
  `initialize_runtime_home`, `register_project`, `ProjectRegistration`,
  `ProjectRecord`.
- [`crates/volicord-store/src/agent_connections.rs`](../../../crates/volicord-store/src/agent_connections.rs):
  `AgentConnectionRecord`, `AgentConnectionRegistration`,
  `ensure_agent_connection`, `add_connection_project`.
- [`crates/volicord-store/src/sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs):
  데이터베이스 경로, 열기, 검증, `begin_immediate_transaction`.
- [`crates/volicord-store/src/migrations.rs`](../../../crates/volicord-store/src/migrations.rs):
  기준 마이그레이션 상수와 마이그레이션 적용.
- [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs):
  Core 쪽 Store 읽기, `CoreStorageMutation`, 커밋 결과.
- [`crates/volicord-store/src/artifacts.rs`](../../../crates/volicord-store/src/artifacts.rs):
  `CoreProjectStore::create_artifact_staging`,
  `verify_persistent_artifact_body`.
- [`crates/volicord-store/src/inspection.rs`](../../../crates/volicord-store/src/inspection.rs):
  읽기 전용 Runtime Home과 프로젝트 상태 검사.
- [`crates/volicord-store/src/error.rs`](../../../crates/volicord-store/src/error.rs):
  `StoreError`와 저장소 실패 처리 경로.

중요한 현재 심볼:

- `CoreProjectStore`, `ProjectStateHeader`, `ProjectEnforcementProfileRecord`
- `ToolInvocationRecord`, `VerifiedReplayContext`, `PendingTaskEvent`
- `CommitMutationInput`, `MutationCommitOutcome`, `CommittedMutationFacts`
- `CoreStorageMutation`, `StorageEffectCounts`, `ProjectMutation`
- `RuntimeHomeRecord`, `ProjectRegistration`, `AgentConnectionRegistration`
- `ArtifactStagingInsert`, `ArtifactStagingRecord`,
  `PersistentArtifactVerification`
- `inspect_runtime_home`, `inspect_registry_database`,
  `inspect_project_state_database`

가장 관련 있는 테스트:

- Store 모듈 안의 단위 테스트.
- Store에 보이는 효과는
  [`crates/volicord-core/src/methods/tests.rs`](../../../crates/volicord-core/src/methods/tests.rs)의
  Core 메서드 테스트에서 확인합니다.
- 계층 간 저장소 확인은
  [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)와
  [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)에 있습니다.

다음에 읽을 컴포넌트:

- 메서드 계획이 Store 읽기와 `CoreStorageMutation` 값을 어떻게 고르는지
  보려면 `volicord-core`를 읽습니다. 로컬 설정이 Store 부트스트랩과 검사를
  직접 사용하는 경로를 보려면 `volicord-cli`를 읽습니다.

## `crates/volicord-core`

존재 이유:

`volicord-core`는 공개 Volicord 메서드 동작을 위한 Core 쪽 서비스를
담당합니다. 어댑터와 독립적인 메서드 동작을 한 크레이트에 두고 Store
읽기, 정책 점검, 메서드 계획, dry-run 미리보기, 커밋된 변이, 공통 응답
구성을 조율합니다.

구현에서 담당하는 것:

- `CoreService`와 그 위의 공개 메서드 진입 함수.
- 요청 래퍼 형태, 어댑터 바인딩, 요청 해시, Store 열기, 프로젝트 상태,
  호출 맥락 검증, 재실행, Task 해석, 상태 버전 최신성, 접근 점검의 공통 사전
  점검.
- `crates/volicord-core/src/methods/`의 메서드별 계획.
- `crates/volicord-core/src/policy/`의 재사용 정책 도우미.
- Core 응답 구성과 읽기 전용, 효과 없음, dry-run, 커밋된 변이 분기 처리.

담당하지 않는 것:

- MCP stdio 프레이밍이나 CLI 설정 동작.
- SQLite DDL, 마이그레이션 정의, 원시 저장소 레이아웃 계약.
- `Product Repository` 제품 파일 쓰기.
- 공개 스키마 계약이나 정확한 값 집합 의미.

추천 첫 파일:

- [`crates/volicord-core/src/lib.rs`](../../../crates/volicord-core/src/lib.rs),
  그다음 [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)

중요 모듈:

- [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs):
  `CoreService`, `InvocationContext`, `MethodPolicy`,
  `OwnerPipelineBranch`, `PreparedRequest`, `PipelineResponse`,
  `CoreService::prepare_request`, `CoreService::execute_prepared_request`.
- [`crates/volicord-core/src/methods/`](../../../crates/volicord-core/src/methods/):
  메서드별 진입 함수와 계획기.
- [`crates/volicord-core/src/methods/status.rs`](../../../crates/volicord-core/src/methods/status.rs):
  `CoreService::status`, `status_task`, `status_result_fields`.
- [`crates/volicord-core/src/methods/intake.rs`](../../../crates/volicord-core/src/methods/intake.rs):
  `CoreService::intake`, `plan_intake`.
- [`crates/volicord-core/src/methods/prepare_write.rs`](../../../crates/volicord-core/src/methods/prepare_write.rs):
  `CoreService::prepare_write`, `prepare_write_policy`, `plan_prepare_write`.
- [`crates/volicord-core/src/policy/`](../../../crates/volicord-core/src/policy/):
  접근, 재실행, 경로, `Write Check`, 증거, 판단 관련성, 닫기 준비 상태
  도우미.

중요한 현재 심볼:

- `CoreService`, `CoreResult`, `CorePipelineError`
- `AdapterSessionBinding`, `InvocationContext`, `VerifiedInvocationContext`,
  `VerifiedActorContext`
- `MethodPolicy`, `TaskRequirement`, `ReplayPolicy`, `FreshnessPolicy`,
  `MethodEffectPolicy`
- `OwnerPipelineBranch`, `PreparedRequest`, `VerifiedRequestContext`,
  `PipelinePreflightOutcome`, `PipelineResponse`
- `prepare_or_response`, `mutation_method_policy`, `validation_rejected`
- `CoreService::status`, `CoreService::intake`,
  `CoreService::prepare_write`, `CoreService::record_run`,
  `CoreService::close_task`

가장 관련 있는 테스트:

- [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)는
  재실행, 최신성, 분기 형태, 효과 없는 동작, Store 실패 처리 경로를 단위
  테스트합니다.
- [`crates/volicord-core/src/methods/tests.rs`](../../../crates/volicord-core/src/methods/tests.rs)는
  메서드 계획과 효과를 실행합니다. 먼저
  `status_is_read_only_including_dry_run`,
  `intake_commits_once_and_replays_without_effect`,
  `prepare_write_allowed_creates_one_write_check_with_post_commit_basis`,
  `prepare_write_dry_run_has_no_write_check_effect`,
  `status_read_only_rejects_corrupt_owner_state_without_effect`를 봅니다.
- 계층 간 확인은
  [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)와
  [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)에 있습니다.

다음에 읽을 컴포넌트:

- 커밋 메커니즘은 `volicord-store`에서, `CoreService`로 들어오는 어댑터
  디스패치는 `volicord-mcp`에서 봅니다.

## `crates/volicord-cli`

존재 이유:

`volicord-cli`는 로컬 `volicord` 관리 실행 파일과 재사용 가능한 명령 모듈을
구현합니다. Setup 프로필 준비 상태, Git 저장소 프로젝트 감지, 프로젝트와
Agent Connection 등록, Agent Connection 설정, 호스트별 MCP 설정, generic MCP
config export, 로컬 `User Channel` 명령, 사전 점검 실행을 처리합니다.

구현에서 담당하는 것:

- `volicord` 바이너리의 프로세스 진입과 관리 명령 디스패치.
- `volicord connect`, `volicord connections`, `volicord connection ...` 파싱,
  저장소 준비, 호스트 계획 구성, 사전 점검 호출, status, verification, mode,
  removal, 출력.
- Setup, doctor, project, export, 로컬 User Channel 명령 파싱과 출력.
- Codex, Claude Code, generic export 호스트 통합 계획.
- 관리 호스트 설정 계획과 안전성 점검.
- Agent Connection, Connection Projects, 호출 출처 메타데이터 생성.

담당하지 않는 것:

- 공개 Volicord API 메서드 동작.
- MCP `tools/call` 의미.
- Core 상태 전이 또는 메서드 정책.
- 정확한 CLI 명령 계약.

추천 첫 파일:

- [`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs)

중요 모듈:

- [`crates/volicord-cli/src/main.rs`](../../../crates/volicord-cli/src/main.rs):
  프로세스 디스패치와 `run_cli`.
- [`crates/volicord-cli/src/setup_command.rs`](../../../crates/volicord-cli/src/setup_command.rs)와
  [`crates/volicord-cli/src/doctor_command.rs`](../../../crates/volicord-cli/src/doctor_command.rs):
  setup 프로필 생성, 실행 파일 발견, 진단 점검.
- [`crates/volicord-cli/src/project_context.rs`](../../../crates/volicord-cli/src/project_context.rs):
  Git 저장소 루트 감지와 `volicord project ...` 명령.
- [`crates/volicord-cli/src/agent_command.rs`](../../../crates/volicord-cli/src/agent_command.rs):
  `volicord connect`, `volicord connections`,
  `volicord connection status/verify/mode/remove` 오케스트레이션.
- [`crates/volicord-cli/src/export_command.rs`](../../../crates/volicord-cli/src/export_command.rs):
  `volicord export mcp-config`.
- [`crates/volicord-cli/src/host_integration/`](../../../crates/volicord-cli/src/host_integration/):
  Codex, Claude Code, generic 호스트 통합 어댑터.
- [`crates/volicord-cli/src/registration.rs`](../../../crates/volicord-cli/src/registration.rs):
  Agent Connection, Connection Project, User Channel 레지스트리 도우미.
- [`crates/volicord-cli/src/user_command.rs`](../../../crates/volicord-cli/src/user_command.rs):
  로컬 User Channel 상태와 판단 명령.

중요한 현재 심볼:

- `run_cli`, `CliError`
- `run_setup_command`, `run_doctor_command`
- `run_project_command`, `resolve_repository_root`
- `run_connect_command`, `run_connections_command`, `run_connection_command`,
  `connect_usage`, `connections_usage`, `connection_usage`
- `run_export_command`, `run_user_command`
- `AgentCommandError`, `AgentProcessOutput`
- `HostKind`, `HostScope`, `HostPlan`, `HostAdapter`, `Verification`
- `AgentConnectionRegistration`, `ConnectionProjectRegistration`,
  `AgentConnectionRecord`
- `actor_source`, `operation_category`, `connection_id`,
  `verification_basis`

가장 관련 있는 테스트:

- [`crates/volicord-cli/tests/binary_admin.rs`](../../../crates/volicord-cli/tests/binary_admin.rs)는
  `volicord` 바이너리의 setup, doctor, 프로젝트 감지, dry-run 동작,
  `volicord connect`, connection status/verification/mode/removal, generic export,
  User Channel 명령, 사전 점검 처리, 설정 파일 안전성을 실행합니다.
- CLI 모듈 안의 단위 테스트는 파싱, 계획, 렌더링, 등록 메타데이터,
  호스트 설정 동작을 다룹니다.

다음에 읽을 컴포넌트:

- 부트스트랩, 검사, registry 저장소 호출은 `volicord-store`에서 봅니다.
  에이전트 설정이 검증하는 `volicord-mcp --check --connection` 사전 점검
  경로는 `volicord-mcp`에서 봅니다.

## `crates/volicord-mcp`

존재 이유:

`volicord-mcp`는 로컬 MCP stdio 어댑터입니다. 공개 Volicord 메서드 도구를
등록하고, 시작/세션 바인딩을 검증하며, `tools/call` 인자를 타입 지정
요청으로 디코딩하고, 로컬 세션에서 신뢰된 호출 맥락을 파생하고, Core를
호출한 뒤 Core의 JSON 응답을 MCP `tools/call` 결과로 래핑합니다.

구현에서 담당하는 것:

- `volicord-mcp` 바이너리 명령 모드: stdio, `--check`, help, version.
- MCP 시작을 위한 Runtime Home과 세션 바인딩 검증.
- `tools/list`가 반환하는 도구 메타데이터.
- `tools/call` 디스패치, 타입 지정 인자 디코딩, 호출 맥락 파생.
- JSON-RPC stdio 프레이밍과 MCP 응답 래핑.

담당하지 않는 것:

- Core 호출 뒤의 공개 메서드 동작.
- Store 변이 정책.
- 관리 CLI 설정 동작.
- `Product Repository` 제품 파일 쓰기.

추천 첫 파일:

- [`crates/volicord-mcp/src/lib.rs`](../../../crates/volicord-mcp/src/lib.rs)

중요 모듈:

- [`crates/volicord-mcp/src/lib.rs`](../../../crates/volicord-mcp/src/lib.rs):
  `PUBLIC_METHOD_TOOL_NAMES`, `McpConnectionStartupInspection`,
  `McpConnectionContext`, `McpAdapter`, `McpAdapter::call_tool`,
  `prepare_connection_arguments`, `public_method_tools`, `run_stdio_from_env`,
  `handle_json_rpc_request`, `call_tool_result`.
- [`crates/volicord-mcp/src/main.rs`](../../../crates/volicord-mcp/src/main.rs):
  `dispatch_args`를 통한 프로세스 모드 디스패치.

중요한 현재 심볼:

- `PUBLIC_METHOD_TOOL_NAMES`, `McpToolDefinition`, `public_method_tools`
- `McpConnectionStartupInspection`, `McpConnectionContext`,
  `McpDerivedInvocationContext`
- `McpAdapter`, `McpAdapter::derive_invocation_context`,
  `McpAdapter::call_tool`
- `prepare_typed_request`, `prepare_connection_arguments`, `decode_params`
- `run_stdio_from_env`, `run_preflight_check_from_env`,
  `preflight_check`
- `McpAdapterError`, `call_tool_result`, `json_rpc_error_for_adapter`

가장 관련 있는 테스트:

- [`crates/volicord-mcp/src/lib.rs`](../../../crates/volicord-mcp/src/lib.rs)의
  단위 테스트. 먼저 `tool_sets_follow_connection_mode_and_exclude_user_only_recording`,
  `connection_context_resolves_and_preflight_reports_allowed_project`,
  `adapter_auto_selects_single_project_and_injects_connection_invocation`,
  `read_only_mode_rejects_agent_workflow_calls_before_core`,
  `mcp_visible_schemas_make_project_selector_optional`을 봅니다.
- [`crates/volicord-mcp/tests/binary_transport.rs`](../../../crates/volicord-mcp/tests/binary_transport.rs)는
  바이너리, `--check`, stdio 프레이밍, 재연결 동작, MCP 응답 래핑을
  실행합니다.
- [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)는
  MCP/Core/Store 계층 간 동작을 실행합니다.

다음에 읽을 컴포넌트:

- 각 `McpAdapter` 분기 뒤의 메서드 의미는 `volicord-core`에서 봅니다.
  시작 검증과 세션 바인딩 읽기는 `volicord-store`에서 봅니다.

## `crates/volicord-test-support`

존재 이유:

`volicord-test-support`는 구현, 통합, 적합성 테스트가 공유하는 폐기 가능한
픽스처 기반을 제공합니다. Runtime Home, Product Repository, 프로젝트 등록,
Agent Connection 등록, 요청 빌더, 직접 Store 검사 도우미를 프로덕션 크레이트 밖에
둡니다.

구현에서 담당하는 것:

- 시스템 임시 디렉터리 아래의 임시 Runtime Home 도우미.
- 등록된 프로젝트와 Agent Connection 하나를 가진 공유 `CoreFixture` 설정.
- 공개 메서드 테스트용 요청 빌더.
- 테스트가 사용하는 픽스처 전용 Store 검사와 변이 도우미.
- 앞으로의 픽스처와 golden-output 도우미를 위한 작은 marker 모듈.

담당하지 않는 것:

- 제품 계약.
- 공개 API 동작.
- 오래 유지되는 Runtime Home 데이터.
- 생성된 보고서나 런타임 출력.

추천 첫 파일:

- [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs)

중요 모듈:

- `fixtures`와 `golden` marker 모듈.
- `CoreFixture`, 요청 빌더, 픽스처 유틸리티가 있는 `core_fixtures`.

중요한 현재 심볼:

- `disposable_runtime_home`, `TempRuntimeHome`
- `CoreFixture`, `CoreFixture::new`, `CoreFixture::store`,
  `CoreFixture::counts`, `CoreFixture::conn`
- `intake_request`, `status_request`, `prepare_write_request`,
  `update_scope_request`, `record_run_request`,
  `request_user_judgment_request`, `record_user_judgment_request`,
  `close_task_request` 같은 픽스처 요청 빌더
- `UpdateScopeFixture`, `RecordJudgmentFixture`, `CloseTaskFixture`,
  `UserJudgmentFixture`
- `supported_evidence_update`, `unsupported_evidence_update`,
  `artifact_input_for_handle`

가장 관련 있는 테스트:

- 이 크레이트는 주로
  [`crates/volicord-core/src/methods/tests.rs`](../../../crates/volicord-core/src/methods/tests.rs),
  [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs),
  [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)를
  통해 실행됩니다.

다음에 읽을 컴포넌트:

- 픽스처를 사용하는 테스트 패키지를 읽습니다. 어댑터 동작은
  `tests/integration`, 교차 메서드 기준 시나리오는 `tests/conformance`에서
  시작합니다.

## `tests/conformance`

존재 이유:

`tests/conformance`는 `volicord-conformance-tests` 패키지와 `baseline` 테스트
대상을 담은 Cargo 워크스페이스 멤버입니다. 공유 픽스처와 Core 쪽 API를
통해 기준 범위 교차 메서드 시나리오를 실행합니다.

구현에서 담당하는 것:

- Core 쪽 공개 메서드를 조합하는 기준 시나리오 범위.
- 효과 분기, idempotency, `Write Check`, 아티팩트 생명주기, 판단 경계,
  닫기 준비 상태, 오류 처리 경로, 손상 처리의 교차 메서드 확인.

담당하지 않는 것:

- 제품 계약 의미나 적합성 권한.
- 공개 API 스키마.
- Store DDL 또는 저장 효과 정의.
- 어댑터 전송 동작.

추천 첫 파일:

- [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)

중요한 현재 심볼:

- `no_effect_branches_state_version_and_idempotency_are_stable`
- `idempotency_replay_rejects_actor_source_mismatch`
- `idempotency_replay_rejects_operation_category_mismatch`
- `committed_non_allow_prepare_write_audit_and_replay_are_exact`
- `prepare_write_allocates_write_check_only_on_committed_allowed_effect`
- `status_projection_matches_public_close_check_and_stays_read_only`
- `core`, `invocation`, `create_task_with_change_unit`,
  `prepare_write_check` 같은 공유 도우미

가장 관련 있는 테스트:

- 이 패키지는
  [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)의
  `baseline` 테스트 대상을 노출합니다.

다음에 읽을 컴포넌트:

- 더 작은 집중 사례는 `volicord-core` 메서드 테스트에서 본 뒤, 정확한 동작
  질문은 참조 담당 문서로 돌아갑니다.

## `tests/integration`

존재 이유:

`tests/integration`은 `volicord-integration-tests` 패키지와 `mcp_connection`
테스트 대상을 담은 Cargo 워크스페이스 멤버입니다. MCP, Core, Store,
Agent Connection 바인딩, 호출 경로 조합을 계층 간으로 검증합니다.

구현에서 담당하는 것:

- MCP를 통한 도구 노출과 스키마 노출.
- MCP 세션 바인딩, 호출 맥락 파생, operation category 라우팅.
- 대표 요청의 MCP/Core 응답 일치.
- 계층 간 저장 효과와 효과 없음 확인.
- Store 상태를 바꾸면 안 되는 stdio 프로토콜 오류 처리.

담당하지 않는 것:

- 공개 메서드 계약.
- MCP 전송 계약.
- Store 계약.
- Core 권한 의미.

추천 첫 파일:

- [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)

중요한 현재 심볼:

- `workflow_tools_include_agent_workflow_and_read_tools_but_exclude_user_only`
- `read_only_tools_expose_only_read_operations_and_project_discovery`
- `connection_invocation_is_injected_and_single_project_is_auto_selected`
- `read_only_mode_rejects_agent_workflow_methods_before_core`
- `multiple_allowed_projects_require_explicit_project_selector`
- `explicit_project_outside_allowlist_is_rejected_before_core`
- `explicit_allowed_project_routes_to_that_project`
- `adapter`, `invocation`, `set_connection_mode`, `add_project` 같은 도우미

가장 관련 있는 테스트:

- 이 패키지는
  [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)의
  `mcp_connection` 테스트 대상을 노출합니다.

다음에 읽을 컴포넌트:

- 테스트 대상 어댑터 경로는 `volicord-mcp`에서 본 뒤, 성공 호출 뒤의 동작은
  `volicord-core`와 `volicord-store`에서 봅니다.

## `xtask`

존재 이유:

`xtask`는 결정적 문서 검증을 위한 저장소 유지보수 패키지입니다.
`cargo run -p xtask -- docs-check`를 제공하며, 문서 도구 의존성이 제품
크레이트나 테스트 지원 크레이트에 들어가지 않게 합니다.

구현에서 담당하는 것:

- 버전 3 `docs/doc-index.yaml` 구조 검증. 여기에는 담당 영역, 날짜, 적용
  가능성 메타데이터가 포함됩니다.
- `docs/en/`과 `docs/ko/`의 유지 Markdown 대응 범위 점검.
- 숨김 앵커를 포함한 로컬 Markdown 링크와 조각 검증.
- `docs/terminology-map.yaml`의 저장소 문서 경로 검증.
- 유지 Markdown과 YAML 경로 메타데이터의 폐기된 문서 경로 감지.

담당하지 않는 것:

- Volicord 런타임 동작.
- 공개 API, 스키마, 저장소, 보안, Core 권한 계약.
- 의미 번역 검토나 계약 담당 문서의 기술 검토.
- 자동 파일 재작성.

추천 첫 파일:

- [`xtask/src/lib.rs`](../../../xtask/src/lib.rs), 그다음
  [`xtask/src/main.rs`](../../../xtask/src/main.rs)

가장 관련 있는 테스트:

- [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs)는 작은 임시
  픽스처 트리로 메타데이터, 대응, 링크, 조각, 폐기 경로, 용어 경로 사례를
  점검합니다.

다음에 읽을 컴포넌트:

- 명령을 이름 붙이고 자동 구조 점검과 사람이 하는 검토를 구분하는 유지보수
  정책은 [검증](../maintain/validation.md)에서 봅니다.
