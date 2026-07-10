# 요청 생명주기

이 가이드는 현재 Rust 구현에서 세 가지 대표 공개 메서드 호출을 따라갑니다.

- 읽기 전용 경로인 `volicord.status`
- 커밋된 상태 변경 경로인 `volicord.intake`
- 정책과 쓰기 티켓에 민감한 경로인 `volicord.prepare_write`

개발자가 코드를 따라갈 수 있도록 소스 파일과 심볼을 이름으로 가리킵니다.
정확한 공개 메서드 동작, 요청이나 응답 스키마, 저장 효과, 보안 보장,
런타임 경계, 오류 의미, Core 권한 의미는 정의하지 않습니다. 정확한 동작은
각 절에 연결된 참조 담당 문서가 담당합니다.

아키텍처 가이드 안에서 이 문서는 어댑터 또는 전송 입력에서 Core 메서드 처리,
Store 상호작용, 응답 또는 오류 형태 구성으로 이어지는 대표 흐름을 담당합니다.
Store 트랜잭션 순서, `dry-run` 저장소 경계, 아티팩트 스테이징, 커밋 실패 경계는
[저장소와 트랜잭션](storage-and-transactions.md)에서 설명합니다.

## MCP에서 Core까지의 공통 형태

이 순서도는 공개 MCP `tools/call`이 Volicord 응답을 반환하기까지의 대표 호출
순서를 보여 줍니다. 화살표는 공유 경로의 구현 순서와 반환 흐름을 나타냅니다.
온보딩 단계, 정확한 공개 메서드 계약, 저장 효과 정의가 아닙니다. 구현 구조는
아래의 `volicord-mcp`, `volicord-core`, 메서드 모듈, `volicord-store` 코드 영역에서
확인합니다. 정확한 제품 동작은 연결된 참조 담당 문서가 정의합니다.

표준 입출력 MCP 경로에서 `volicord mcp --stdio`는 먼저 Runtime Home과 Agent Connection
프로세스 맥락을 해석하고, 시작 검사는 표준 입출력이 시작되기 전에 필요한 사실을
검증합니다. 로컬 HTTP 경로는 `volicord serve --transport local-http`로 시작하며,
묶인 연결 맥락을 해석하고 전송 계층의 프로젝트 허용 목록을 적용한 뒤 HTTP MCP
요청을 같은 어댑터로 보냅니다. 전송 계층이 요청을 넘기면 공개 `tools/call`은 허용된
프로젝트를 선택하고, 형식화된 요청을 디코딩하고, 어댑터가 생성한 요청 사실을
채우고, 로컬 Core 호출 사실을 파생한 뒤 해당 `CoreService` 메서드를 호출합니다.

```mermaid
sequenceDiagram
  participant Host as MCP 호스트
  participant MCP as volicord-mcp
  participant Core as volicord-core
  participant Method as 메서드 모듈
  participant Store as volicord-store

  Host->>MCP: JSON-RPC tools/call
  MCP->>MCP: call_tool_result_with_elicitation이 name과 arguments 추출
  MCP->>MCP: McpAdapter::call_tool이 도구 처리 경로 선택
  MCP->>MCP: prepare_mcp_arguments가 프로젝트 선택
  MCP->>MCP: decode_params가 형식화된 요청 디코딩
  MCP->>MCP: generated_envelope가 어댑터 생성 요청 래퍼 필드 채움
  MCP->>MCP: McpDerivedInvocationContext::core_invocation이 InvocationContext 파생
  MCP->>Core: CoreService method(request, invocation)
  Core->>Core: prepare_or_response -> CoreService::prepare_request
  Core->>Store: CoreProjectStore::open과 공유 읽기
  Core->>Method: 메서드별 계획
  Method-->>Core: OwnerPipelineBranch
  Core->>Store: 커밋된 변경 분기에서만 커밋
  Core-->>MCP: PipelineResponse
  MCP-->>Host: Volicord JSON을 담은 tools/call content 텍스트
```

공유 어댑터 경로는 `volicord-mcp` 모듈들에 나뉘어 있습니다.

- [`crates/volicord-mcp/src/stdio.rs`](../../../crates/volicord-mcp/src/stdio.rs):
  `run_stdio`가 줄 단위 JSON-RPC를 읽고,
  `handle_json_rpc_request`가 `initialize`, `ping`, `tools/list`,
  `tools/call`을 디스패치하며, `call_tool_result_with_elicitation`이
  `params.name`과 `params.arguments`를 추출하고 `McpAdapter`를 호출한 뒤
  `PipelineResponse.response_json`을 MCP 텍스트 `content`에 담습니다.
- [`crates/volicord-mcp/src/local_http.rs`](../../../crates/volicord-mcp/src/local_http.rs):
  `run_local_http_server`가 연결된 어댑터 맥락을 해석하고 전송 계층의
  프로젝트 허용 목록을 적용한 뒤 로컬 HTTP 세션과 MCP 요청을
  `McpAdapter`로 보냅니다.
- [`crates/volicord-mcp/src/tool_registry.rs`](../../../crates/volicord-mcp/src/tool_registry.rs):
  `PUBLIC_METHOD_TOOL_NAMES`, `McpToolDefinition`, 도구 목록 메타데이터.
- [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs):
  `McpAdapter::call_tool`이 도구 이름에 맞는 분기를 고르고, 메서드별 도우미가
  형식화된 Core 요청을 구성합니다. `prepare_mcp_arguments<T>`는 내부 전용 필드를
  거부하고 허용된 프로젝트를 선택하고 `decode_params<T>`로 인자를
  디코딩합니다. `generated_envelope`는 어댑터가 생성하는 요청 래퍼 필드를
  채우고, `call_core_request`는 `CoreService`를 호출하기 전에 로컬 호출
  사실을 파생합니다.
- [`crates/volicord-mcp/src/routing.rs`](../../../crates/volicord-mcp/src/routing.rs):
  시작 검사, `McpConnectionContext`, 연결 모드 파싱, 프로젝트 허용 목록
  점검, 프로젝트 가용성 도우미.
- 프로젝트를 선택한 뒤 `call_core_request`는 `derive_invocation_context`를
  사용해 선택된 프로젝트, 묶인 Agent Connection의 행위자 출처, 요청
  `operation_category`, 어댑터 바인딩 근거를 담은
  `McpDerivedInvocationContext`를 만듭니다.
- `McpDerivedInvocationContext::core_invocation`은 Core `InvocationContext`를
  만듭니다.

시작과 세션 검증도 `volicord-mcp`에 있으며, 특히
`McpConnectionStartupInspection::resolve`가 핵심입니다. 이 시작 경로는
Runtime Home 초기화, 설치 프로필, Agent Connection 식별자, 활성화 여부,
메타데이터 객체 형태, 모드, Connection Projects 멤버십, 프로젝트 가용성을
검증하기 위해 Store를 직접 읽습니다. 시작 검사는 `actor_source`를 파생하거나
모든 호출에 쓸 프로젝트 하나를 선택하지 않습니다. 요청 시점의 어댑터 코드가
프로젝트 선택 뒤 묶인 Agent Connection에서 `actor_source`를 파생합니다. 시작
검사는 공개 메서드 동작을 구현하는 다른 경로가 아니며, 공개 메서드 실행은
`volicord-core`를 통과합니다.

공유 Core 경로는 주로
[`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)와
[`crates/volicord-core/src/methods/mod.rs`](../../../crates/volicord-core/src/methods/mod.rs)에
있습니다.

- 메서드 파일은 `prepare_or_response`를 호출하고, 이 도우미는
  `CoreService::prepare_request`로 위임합니다.
- `MethodPolicy`는 필요한 `OperationCategory`, `TaskRequirement`,
  `ReplayPolicy`, `FreshnessPolicy`, `MethodEffectPolicy`를 고릅니다.
- `CoreService::prepare_request`는 요청 래퍼를 검증하고, 어댑터 바인딩
  불일치를 거부하고, 커밋 효과 요청 래퍼 요구사항을 검증하고,
  `canonical_request_hash`를 계산하고, `CoreProjectStore`를 열고,
  `project_state`를 읽고, `VerifiedInvocationContext`를 파생하고, 재실행 사전
  점검을 처리하고, Task를 해석하고, 상태 버전 최신성을 점검하고, 메서드
  접근을 점검한 뒤 `PreparedRequest`를 만듭니다.
- `CoreService::execute_prepared_request`는 `OwnerPipelineBranch`를 읽기 전용,
  효과 없음, `dry-run` 미리보기, 커밋된 변경의 응답 구성 경로 중 하나로 보냅니다.

Store 커밋 경로는
[`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)와
[`crates/volicord-store/src/core_pipeline/mutation_apply.rs`](../../../crates/volicord-store/src/core_pipeline/mutation_apply.rs)에
있습니다.

- Core는 `commit_input`으로 `CommitMutationInput`을 만듭니다.
- `CoreProjectStore::commit_mutation`은 재실행 조회, 오래된 상태 점검,
  `project_state.state_version` 증가, 메서드가 제공한 `CoreStorageMutation`
  값을 트랜잭션 범위 SQL 도우미로 적용, 권한 이벤트 삽입, 응답 JSON 구성,
  선택적 재실행 행 삽입, 트랜잭션 커밋을 수행합니다.
- `MutationCommitOutcome`은 커밋, 재실행, 재실행 맥락 불일치, 멱등성 충돌,
  오래된 상태 결과를 Core로 돌려보냅니다.

응답과 오류 형태 구성도 같은 계층 분리를 따릅니다. 어댑터나 처리 경로의 오류는
Core 계획 전에 반환될 수 있습니다. Core는 준비된 공개 메서드 호출에 대해 거부,
읽기 전용, 효과 없음, `dry-run` 미리보기, 커밋, 재실행, 충돌 결과를 포함하는
`PipelineResponse`를 반환합니다. MCP는 `PipelineResponse.response_json`을
`tools/call`의 텍스트 `content`에 담습니다. 정확한 공개 오류 우선순위, 응답 스키마,
MCP 전송 래핑 규칙은 [API 오류](../reference/api/errors.md),
[API 코어 스키마](../reference/api/schema-core.md),
[MCP 전송](../reference/mcp-transport.md)이 담당합니다.

## 분기 차이

`OwnerPipelineBranch`는 공통 사전 점검과 메서드별 계획 뒤에 선택되는
Core 쪽 분기입니다. 정확한 저장 효과 계약은
[저장 효과](../reference/storage-effects.md)가 담당합니다. 이 표는 소스를 따라갈
때 쓰는 구현 중심 지도입니다.

| 분기 또는 응답 경로 | 읽을 위치 | 가이드 수준 영속 저장 결과 |
|---|---|---|
| MCP 디코딩 또는 사전 점검의 거부 응답 | `McpAdapter::call_tool`, `CoreService::prepare_request`, `validation_rejected` | Core 커밋 없이 거부 응답 또는 JSON-RPC 오류를 반환합니다. `state_version` 증가, 권한 이벤트, 재실행 행, 아티팩트 효과, 쓰기 티켓 효과를 만들지 않습니다. |
| `OwnerPipelineBranch::ReadOnly` | `CoreService::execute_prepared_request` | 현재 읽기 결과에서 `EffectKind::ReadOnly` 결과를 만들고 `CoreProjectStore::commit_mutation`을 호출하지 않습니다. 응답에 계산된 닫기 차단 사유나 아티팩트 관찰이 있더라도 읽는 시점의 데이터입니다. |
| `OwnerPipelineBranch::NoEffectResult` | `CoreService::execute_prepared_request`; 현재는 `close_task`의 차단된 결과 경로에서 사용 | `EffectKind::NoEffect`인 유효한 결과를 만들고 `CoreProjectStore::commit_mutation`을 호출하지 않습니다. 이 경로의 차단 사유형 결과는 응답 데이터이며 커밋된 차단 사유 행이 아닙니다. |
| `OwnerPipelineBranch::DryRunPreview` | `CoreService::execute_prepared_request` | `ToolDryRunResponse` 미리보기 데이터를 만들지만 생성된 영속 참조, 권한 이벤트, 재실행 행, 스테이징 핸들, 아티팩트, `state_version` 변경은 저장하지 않습니다. |
| `OwnerPipelineBranch::CommitMutation` | `CoreService::execute_prepared_request`, Core `commit_mutation`, Store `CoreProjectStore::commit_mutation` | Store 커밋 트랜잭션을 실행합니다. 이 트랜잭션은 `project_state.state_version`을 증가시키고, 권한 이벤트를 최소 하나 추가하고, 커밋 호출이 멱등이면 재실행 행을 저장하며, 메서드가 제공한 `CoreStorageMutation` 값을 적용합니다. 메서드 담당 문서가 그 분기를 정의한다면 메서드가 `CoreStorageMutation` 값을 하나도 제공하지 않아도 이벤트/재실행/상태 버전 효과를 커밋할 수 있습니다. |
| `volicord.stage_artifact` 스테이징 경로 | `crates/volicord-core/src/methods/stage_artifact.rs`, Store 아티팩트 스테이징 도우미 | `EffectKind::StagingCreated`인 `StageArtifactResult`를 반환하고 저장소 소유 임시 스테이징과 안전한 바이트를 만들 수 있습니다. 일반 Core 커밋 트랜잭션을 사용하지 않고, 권한 이벤트나 재실행 행을 추가하지 않으며, `project_state.state_version`을 증가시키지 않고, 영속 `ArtifactRef`를 만들지 않습니다. [아티팩트 저장소](../reference/storage-artifacts.md)를 봅니다. |

차단된 것처럼 보이는 모든 결과를 같은 구현 경로로 다루면 안 됩니다. 예를
들어 `volicord.prepare_write`는 커밋 전 거부되어 효과가 없을 수 있고,
`dry-run` 미리보기로 효과가 없을 수 있고, 쓰기 티켓을 발급하지
않는 비허용 결정 이벤트를 커밋할 수 있으며, 허용 결정에서는
쓰기 티켓 호환성 행을 삽입할 수 있습니다. `volicord.check_close`는 읽기 전용
확인에서 닫기 차단 사유를 반환할 수 있고, `volicord.close_task`는 기준 효과 없음
차단 경로에서 닫기 차단 사유를 반환할 수 있습니다.
API 오류는 거부 응답으로 남으며 닫기 차단 사유가 아닙니다. 차단
사유와 API 사이의 정확한 경계는 [API 차단 사유 처리 경로](../reference/api/blocker-routing.md)가
담당합니다.

## `volicord.status`: 읽기 전용 경로

참조 담당 문서:

- [상태 메서드 담당 문서](../reference/api/method-status.md)

주요 소스 경로:

1. [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)는
   `StatusRequest`, `StatusInclude`, `StatusResult`, 그리고
   `OperationCategory::Read`를 반환하는 `MethodOperationCategory` 구현을 정의합니다.
2. [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)는
   `McpAdapter::call_tool`에서 `"volicord.status"` 처리 경로를 선택하고,
   형식화된 `status` 인자를 준비합니다. 어댑터 생성 요청 래퍼를 만들고
   로컬 호출 사실과
   `InvocationContext`를 파생한 뒤 `CoreService::status`를 호출합니다.
3. [`crates/volicord-core/src/methods/status.rs`](../../../crates/volicord-core/src/methods/status.rs)는
   `CoreService::status`, `status_task`, `status_result_fields`를 구현합니다.
4. [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)는
   공통 사전 점검과 `OwnerPipelineBranch::ReadOnly` 응답 경로를 실행합니다.
5. [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)는
   `project_state`, Task 읽기, Change Unit 읽기, 쓰기 권한 읽기, 증거 읽기,
   닫기 준비 상태 입력 읽기, 프로젝트 연속성 읽기 같은 `CoreProjectStore` 읽기를 제공합니다.

생명주기:

1. MCP 호스트가 `name="volicord.status"`로 `tools/call`을 보냅니다.
2. `call_tool_result_with_elicitation`이 도구 이름과 인자를 추출합니다.
3. `McpAdapter::call_tool`이 호출을 `status` 분기로 보냅니다.
4. `prepare_mcp_arguments`는 `McpConnectionContext`에서 허용된 프로젝트를
   선택하고 형식화된 `status` 인자를 디코딩합니다. `generated_envelope`는
   `status`의 `operation_category`에 맞는 어댑터 생성 요청 래퍼 필드를 채우며,
   `call_core_request`는 로컬 호출 사실에서 Core `InvocationContext`를 만듭니다.
5. `CoreService::status`는 형식화된 요청을 요청 JSON으로 직렬화하고,
   `MethodPolicy::exact`, `TaskRequirement::Optional`, `ReplayPolicy::None`,
   `FreshnessPolicy::None`, `MethodEffectPolicy::ReadOnly`로
   `prepare_or_response`를 호출합니다.
6. `CoreService::prepare_request`가 공통 사전 점검을 실행합니다. 사전 점검이
   응답을 반환하면 메서드는 메서드별 결과 구성 없이 그 응답을 반환합니다.
7. `status_task`는 요청 래퍼에 Task가 있으면 그 Task를, 없으면 현재 적용
   Task를 선택합니다.
8. `status_result_fields`는 Store 읽기와 요청된 `StatusInclude` 플래그에서
   결과 필드를 만듭니다. `include.close=true`이면 `CloseIntent::Check`와 함께
   `close_task::plan_close_task`를 재사용해 읽기 전용 닫기 보기를 계산합니다.
   `include.continuity=true`이면 저장소를 변경하지 않고 현재 프로젝트
   연속성 요약을 읽습니다.
9. `CoreService::execute_prepared_request`는 `OwnerPipelineBranch::ReadOnly`를
   받아 `EffectKind::ReadOnly` 결과를 만들고 `PipelineResponse`를 반환합니다.
10. `call_tool_result_with_elicitation`은 `PipelineResponse.response_json`을 MCP
    `content[0].text`에 담습니다.

일어나지 않는 일:

- `CoreProjectStore::commit_mutation` 호출 없음.
- 상태 버전 증가 없음.
- 권한 이벤트 없음.
- 재실행 행 없음.
- 쓰기 티켓 변경 없음.
- 프로젝트 연속성 기록 생성 없음.

대표 테스트:

- [`crates/volicord-core/src/methods/tests/status.rs`](../../../crates/volicord-core/src/methods/tests/status.rs)의
  `status_is_read_only_including_dry_run`,
  `status_include_false_omits_optional_sections_without_effect`
- [`crates/volicord-mcp/src/tests.rs`](../../../crates/volicord-mcp/src/tests.rs)의
  `mcp_status_succeeds_with_readonly_storage`,
  `mcp_status_does_not_advance_state_version`
- [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)의
  `status_projection_matches_public_close_check_and_stays_read_only`

정확한 동작 질문:

- 메서드 동작: [상태 메서드 담당 문서](../reference/api/method-status.md)
- 공통 응답 형태: [API 코어 스키마](../reference/api/schema-core.md)
- 상태와 닫기 준비 상태 표시 형태:
  [상태 스키마](../reference/api/schema-state.md)
- 저장 효과: [저장 효과](../reference/storage-effects.md)

## `volicord.intake`: 커밋된 변경 경로

참조 담당 문서:

- [접수 메서드 담당 문서](../reference/api/method-intake.md)

주요 소스 경로:

1. [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)는
   `IntakeRequest`, `InitialScope`, `IntakeResult`, 그리고
   `OperationCategory::AgentWorkflow`을 반환하는 `MethodOperationCategory` 구현을 정의합니다.
2. [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)는
   `McpAdapter::call_tool`에서 `"volicord.intake"` 처리 경로를 선택하고,
   형식화된 `intake` 인자를 준비합니다. 어댑터 생성 요청 래퍼를 만들고
   로컬 호출 사실과
   `InvocationContext`를 파생한 뒤 `CoreService::intake`를 호출합니다.
3. [`crates/volicord-core/src/methods/intake.rs`](../../../crates/volicord-core/src/methods/intake.rs)는
   `CoreService::intake`와 `plan_intake`를 구현합니다.
4. [`crates/volicord-core/src/methods/mod.rs`](../../../crates/volicord-core/src/methods/mod.rs)는
   `mutation_method_policy`, `prepare_or_response`, 공통 메서드 계획 도우미,
   응답 도우미를 제공합니다.
5. [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs)는
   `OwnerPipelineBranch::DryRunPreview` 또는
   `OwnerPipelineBranch::CommitMutation`을 실행합니다.
6. [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs)는
   커밋 트랜잭션을 열고 이벤트와 재실행 행을 커밋하며,
   [`crates/volicord-store/src/core_pipeline/mutation_apply.rs`](../../../crates/volicord-store/src/core_pipeline/mutation_apply.rs)는
   그 트랜잭션 안에서 `CoreStorageMutation` 값을 적용합니다.

생명주기:

1. MCP 호스트가 `name="volicord.intake"`로 `tools/call`을 보냅니다.
2. `McpAdapter::call_tool`이 형식화된 `intake` 인자를 준비하고, 어댑터 생성
   요청 래퍼를 만들고, 로컬 호출 사실과 `InvocationContext`를 파생한 뒤
   `CoreService::intake`를 호출합니다.
3. `CoreService::intake`는 `TaskRequirement::None`과 함께
   `mutation_method_policy`를 고릅니다. `dry-run`이면 정책은
   `MethodEffectPolicy::DryRunPreview`와 `ReplayPolicy::None`을 사용합니다.
   커밋 호출이면 `MethodEffectPolicy::CoreMutation`과
   `ReplayPolicy::Committed`를 사용합니다.
4. `prepare_or_response`는 공통 사전 점검을 위해
   `CoreService::prepare_request`로 위임합니다. 커밋 호출은 공유 커밋 효과
   요청 래퍼 점검, 재실행 사전 점검, 최신성 정책, 접근 점검을 사용합니다.
5. 현재 프로젝트 상태에 현재 적용 Task가 있는데
   `ResumePolicy::RejectIfActive`이면 메서드는 거부합니다.
6. `plan_intake`는 새 Task를 만들지, 현재 적용 Task를 재개할지, 현재 적용
   Task를 대체할지 해석합니다. 생성된 `TaskId`를 할당할 수 있고,
   `TaskRecord`를 만들고, 재개된 Task의 현재 적용 Change Unit을 선택하고,
   예상 `StateSummary`를 계산하고, `CoreStorageMutation` 값을 만듭니다.
7. `request.envelope.dry_run`이 `true`이면 Core는
   `OwnerPipelineBranch::DryRunPreview`를 실행하고 Store 커밋 없는 `dry-run`
   응답을 반환합니다.
8. 그렇지 않으면 Core는 `event_kind="task_intake"`, 메서드 결과 필드, 선택된
   `task_id`, 계획된 저장소 변이를 담은 `OwnerPipelineBranch::CommitMutation`을
   실행합니다.
9. Core 내부 `commit_mutation` 도우미는 정규화된 요청 해시, 재실행 맥락,
   예상 상태 버전, `PendingTaskEvent`를 담은 `CommitMutationInput`을 만듭니다.
10. `CoreProjectStore::commit_mutation`은 하나의 즉시 트랜잭션을 열고,
    재실행과 최신성을 다시 점검하고, `project_state.state_version`을 증가시키고,
    `CoreStorageMutation` 값을 적용하고, 권한 이벤트를 삽입하고, 응답 JSON을
    만들고 검증하고, 멱등성 키가 있는 커밋 호출의 재실행 행을 삽입한 뒤
    커밋합니다.
11. 커밋된 응답은 `PipelineResponse`로 돌아오고 MCP는 이를 `tools/call`의
    텍스트 `content`에 담습니다.

분기별 차이:

- `dry-run` 접수는 `OwnerPipelineBranch::DryRunPreview`를 사용합니다. Task,
  이벤트, 재실행 행, 상태 버전 증가는 만들어지지 않습니다.
- 사전 점검 또는 검증 거부는 Core 커밋 없이 거부 응답을 반환합니다.
- 커밋된 접수는 `OwnerPipelineBranch::CommitMutation`을 사용합니다. 상태
  버전을 증가시키고, `task_intake` 이벤트를 추가하고, 멱등성 키가
  있으면 재실행 행을 저장하고, 메서드가 계획한 변이를 적용합니다.

대표 테스트:

- [`crates/volicord-core/src/methods/tests/intake.rs`](../../../crates/volicord-core/src/methods/tests/intake.rs)의
  `intake_commits_once_and_replays_without_effect`,
  `intake_dry_run_has_no_storage_effect`
- [`crates/volicord-mcp/src/tests.rs`](../../../crates/volicord-mcp/src/tests.rs)의
  `adapter_auto_selects_single_project_and_injects_connection_invocation`
- [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)의
  `connection_invocation_is_injected_and_single_project_is_auto_selected`
- [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)의
  `no_effect_branches_state_version_and_idempotency_are_stable`

정확한 동작 질문:

- 메서드 동작: [접수 메서드 담당 문서](../reference/api/method-intake.md)
- 공통 요청 래퍼와 응답 분기:
  [API 코어 스키마](../reference/api/schema-core.md)
- Task와 상태 형태: [상태 스키마](../reference/api/schema-state.md)
- 저장 효과: [저장 효과](../reference/storage-effects.md)
- 재실행과 오류 동작: [API 오류](../reference/api/errors.md)와 메서드 담당 문서

## `volicord.prepare_write`: 정책과 쓰기 티켓 경로

참조 담당 문서:

- [쓰기 준비 메서드 담당 문서](../reference/api/method-prepare-write.md)

주요 소스 경로:

1. [`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)는
   `PrepareWriteRequest`, `PrepareWriteResult`, 그리고
   `OperationCategory::AgentWorkflow`을 반환하는 `MethodOperationCategory` 구현을
   정의합니다.
2. [`crates/volicord-mcp/src/adapter.rs`](../../../crates/volicord-mcp/src/adapter.rs)는
   `McpAdapter::call_tool`에서 `"volicord.prepare_write"` 처리 경로를 선택하고,
   형식화된 쓰기 준비 인자를 준비합니다. 어댑터 생성 요청 래퍼를 만들고 로컬
   호출 사실과 `InvocationContext`를 파생한 뒤 `CoreService::prepare_write`를
   호출합니다.
3. [`crates/volicord-core/src/methods/prepare_write.rs`](../../../crates/volicord-core/src/methods/prepare_write.rs)는
   `CoreService::prepare_write`, `prepare_write_policy`,
   `plan_prepare_write`를 구현합니다.
4. [`crates/volicord-core/src/policy/write_ticket.rs`](../../../crates/volicord-core/src/policy/write_ticket.rs)는
   `prepare_write_decision`, `prepare_write_dry_run_summary`,
   쓰기 티켓 호환성 도우미, `write_decision_reason`을 제공합니다.
5. [`crates/volicord-core/src/policy/path.rs`](../../../crates/volicord-core/src/policy/path.rs)는
   `Product Repository` 경로 정규화 도우미를 제공합니다.
6. [`crates/volicord-core/src/policy/judgment_relevance.rs`](../../../crates/volicord-core/src/policy/judgment_relevance.rs)는
   계획기가 사용하는 판단 관련성 점검을 제공합니다.
7. [`crates/volicord-store/src/core_pipeline/mutation_apply.rs`](../../../crates/volicord-store/src/core_pipeline/mutation_apply.rs)는
   커밋된 허용 분기가 쓰기 티켓을 발급할 때 Store 커밋 트랜잭션 안에서
   `CoreStorageMutation::InsertWriteTicket`을 적용합니다.

생명주기:

1. MCP 호스트가 `name="volicord.prepare_write"`로 `tools/call`을 보냅니다.
2. `McpAdapter::call_tool`이 형식화된 쓰기 준비 인자를 준비하고, 어댑터
   생성 요청 래퍼를 만들고, 로컬 호출 사실과 `InvocationContext`를 파생한 뒤
   `CoreService::prepare_write`를 호출합니다.
3. `CoreService::prepare_write`는 먼저 `envelope.task_id`가 있을 때
   `PrepareWriteRequest.task_id`와 일치하는지 확인합니다.
4. `prepare_write_policy`는 요청 또는 요청 래퍼가 Task ID를 제공하면
   `TaskRequirement::Exact`를, 그렇지 않으면 `TaskRequirement::Required`를
   고릅니다. `dry-run`은 `MethodEffectPolicy::DryRunPreview`와
   `ReplayPolicy::None`을 사용하고, 커밋 호출은
   `MethodEffectPolicy::CoreMutation`과 `ReplayPolicy::Committed`를 사용합니다.
5. `prepare_or_response`는 공통 사전 점검으로 위임합니다. 접근 불일치,
   오래된 상태, 누락된 커밋 효과 요청 래퍼 필드, 재실행 불일치, Store 사용
   불가가 메서드별 계획 전에 응답을 반환할 수 있습니다.
6. `plan_prepare_write`는 `intended_operation`, `sensitive_categories`,
   `Product Repository` 경로를 정규화합니다. 그런 뒤 Task와 현재 Change
   Unit을 해석합니다. 제품 파일 쓰기 의도, 기준 범위, 경로 범위, 대기 중인
   사용자 소유 판단, 민감 동작 승인, 검증된 `operation_category`, 연결 역량을 비교합니다.
7. `prepare_write_decision`은 모인 `WriteDecisionReason` 값을 분류합니다.
   이유가 없으면 허용 계획이고, 있으면 비허용 결정입니다.
8. 요청이 `dry-run`이면 `CoreService::execute_prepared_request`는
   `prepare_write_dry_run_summary`가 담긴 `OwnerPipelineBranch::DryRunPreview`를
   받습니다. 쓰기 티켓 ID는 할당되지 않고 Store 커밋은 실행되지
   않습니다.
9. 커밋된 허용 계획이면 `OwnerPipelineBranch::CommitMutation`은
   `CoreStorageMutation::InsertWriteTicket`,
   `event_kind="write_ticket_issued"`, 새 `write_ticket_ref`를
   담은 결과 필드를 운반합니다.
10. 커밋된 비허용 계획이면 `OwnerPipelineBranch::CommitMutation`은
    `event_kind="write_decision_recorded"`를 운반하고
    `InsertWriteTicket` 변이는 없습니다. 그래도 Store 트랜잭션은 결정
    이벤트를 기록하고, 상태 버전을 전진시키며, 커밋 호출이 멱등이면
    재실행 데이터를 저장합니다.
11. `CoreProjectStore::commit_mutation`은 트랜잭션을 실행하고
    `MutationCommitOutcome`을 반환합니다. Core는 그 결과를
    `PipelineResponse`로 만들고, MCP는 응답 JSON을 `tools/call`의 텍스트
    `content`에 담습니다.

분기별 차이:

- 사전 점검 또는 초기 검증 거부는 Core 커밋이 없고 쓰기 티켓을
  발급하지 않습니다.
- `dry-run`은 `ToolDryRunResponse`를 반환하고, Core 커밋이 없으며, 영속
  쓰기 티켓 ID를 할당하지 않습니다.
- 커밋된 비허용 결정은 결정 이벤트를 커밋하지만 소비 가능한
  쓰기 티켓을 만들지 않습니다.
- 커밋된 허용 결정은 이벤트와
  `CoreStorageMutation::InsertWriteTicket`을 커밋합니다.
- 멱등 재실행은 다른 쓰기 티켓을 만들지 않고 재실행 처리에서 저장된
  원래 응답을 반환합니다.

대표 테스트:

- [`crates/volicord-core/src/methods/tests/prepare_write.rs`](../../../crates/volicord-core/src/methods/tests/prepare_write.rs)의
  `prepare_write_allowed_issues_one_write_ticket_with_post_commit_basis`,
  `prepare_write_blocked_path_issues_no_write_ticket`,
  `prepare_write_dry_run_has_no_write_ticket_effect`,
  `prepare_write_user_only_category_is_invocation_context_rejection`
- [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs)의
  `read_only_mode_rejects_agent_workflow_methods_before_core`
- [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs)의
  `committed_non_allow_prepare_write_audit_and_replay_are_exact` 및
  `prepare_write_issues_write_ticket_only_on_committed_allowed_effect`

정확한 동작 질문:

- 메서드 동작과 결정 분기:
  [쓰기 준비 메서드 담당 문서](../reference/api/method-prepare-write.md)
- 쓰기 티켓, 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험
  수락 같은 Core 권한 용어: [Core 모델](../reference/core-model.md)
- `Product Repository` 경로 정규화:
  [런타임 경계](../reference/runtime-boundaries.md)
- 공통 응답 분기: [API 코어 스키마](../reference/api/schema-core.md)
- 판단 형태: [판단 스키마](../reference/api/schema-judgment.md)
- 저장 효과: [저장 효과](../reference/storage-effects.md)
- 보안 보장 의미: [보안](../reference/security.md)
