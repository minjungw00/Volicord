# 용어집 참조

이 용어집은 하네스 용어, 대소문자, 정확한 식별자, 담당 문서 경로를 확인하는 짧은 찾아보기 문서입니다. 이 문서는 계획된 하네스 동작을 설명하는 원천 문서입니다. 이 저장소는 아직 문서 전용이며, 현재 저장소 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

이 문서는 Core 동작, API schema, Storage DDL, 보안 보장, projection template, connector 계약, conformance fixture, later profile 계약을 대신 정의하지 않습니다. 정확한 동작은 담당 문서 링크를 따라갑니다.

## 공개 용어

사용자용 문서, 프롬프트, 상태 요약에서는 아래 용어를 먼저 씁니다. 정확한 하네스 식별자는 경계, 차단 사유, 출처 참조, 담당 문서 링크를 설명할 때만 덧붙입니다.

| 공개 용어 | 뜻 | 담당 경로 |
|---|---|---|
| 작업 | 사용자가 끝내거나, 답을 얻거나, 조사하거나, 결정하고 싶은 일입니다. 내부 기록을 이름 붙일 때만 `Task`를 씁니다. | [Core Model](core-model.md#entity-model) |
| 범위 | 무엇이 바뀔 수 있고, 무엇이 범위 밖이며, 에이전트가 어디에서 멈춰야 하는지입니다. | [Core Model](core-model.md#entity-model) |
| 범위 밖 | 현재 범위에서 제외된 파일, 동작, 판단, 주장, 행동입니다. | [Core Model](core-model.md#prepare_write) |
| 요구사항 구체화 | 구현 계획이나 쓰기 가능한 작업 전에 요구사항을 분명히 하는 과정입니다. 내부 참조에서는 `Discovery`라고 부를 수 있습니다. | [Core Model](core-model.md#entity-model) |
| 작업 조각 | 작게 나눈 범위입니다. 내부 참조에서는 쓰기 가능한 범위 단위를 `Change Unit`이라고 부를 수 있습니다. | [Core Model](core-model.md#entity-model) |
| 사용자가 소유하는 판단 | 에이전트 추론, 증거, projection에 표시된 문구, 넓은 동의에서 추론하지 않고 사용자의 선택으로 보존해야 하는 판단입니다. | [Core Model](core-model.md#judgment-route-boundaries) |
| 판단 요청 | 사용자가 소유하는 판단 하나를 묻는 집중된 질문입니다. API 참조에서는 `UserJudgment`를 씁니다. | [MVP API](api/mvp-api.md#harnessrequest_user_judgment) |
| 제품 판단 | 제품 동작, 문구, 흐름, UX, 사용자 가치에 대한 사용자가 소유하는 판단입니다. | [Core Model](core-model.md#judgment-route-boundaries) |
| 기술 판단 | 아키텍처, 의존성, migration, interface, security/privacy, 중요한 기술 방향에 대한 사용자가 소유하는 판단입니다. | [Core Model](core-model.md#judgment-route-boundaries) |
| 범위 판단 | 범위 확장, non-goal 제거, Change Unit 경계, Autonomy Boundary 변경에 대한 사용자가 소유하는 판단입니다. | [Core Model](core-model.md#judgment-route-boundaries) |
| 민감 동작 승인 | 경계가 정해진 범위 안에서 이름 붙은 민감한 단계 하나를 진행해도 된다는 사용자 권한 부여입니다. 최종 수락이나 넓은 동의가 아닙니다. | [Core Model](core-model.md#judgment-route-boundaries) |
| 증거 | 작업에 대한 주장을 뒷받침하는 오래 남는 자료입니다. 변경 경로, 변경 차이, 로그, 스크린샷, 검사 메모, 아티팩트 참조가 될 수 있습니다. | [API Schema Core](api/schema-core.md#evidence-and-pre-write-scope-schemas), [Storage](storage.md#6-artifact-references) |
| 확인 | 테스트, 변경 차이 검토, 검사, 출처 확인 같은 일반 확인입니다. 공식 기록 경로를 말할 때만 `Verification`을 씁니다. | [Core Model](core-model.md#evidence-verification-qa-final-acceptance-and-risk) |
| 검증 | 담당 경로가 요구할 때 기록되는 정확성 확인입니다. 최종 수락, QA, 증거, 잔여 위험 수락을 대신하지 않습니다. | [Core Model](core-model.md#evidence-verification-qa-final-acceptance-and-risk) |
| 수동 QA | 자동 확인이나 증거만으로는 부족하고 사람이 직접 판단해야 하는 품질 확인입니다. | [Core Model](core-model.md#evidence-verification-qa-final-acceptance-and-risk), [Later](../later/index.md#assurance-candidates) |
| 최종 수락 | 작업 경로가 수락을 요구할 때 사용자가 결과를 받아들이는 판단입니다. 그 자체로 민감 동작을 승인하거나 잔여 위험을 수락하지 않습니다. | [Core Model](core-model.md#judgment-route-boundaries) |
| 닫기 가능 여부 | 작업을 지금 정직하게 닫을 수 있는지와 닫기 전에 남은 일을 보여주는 상태입니다. | [Core Model](core-model.md#close_task) |
| 닫기 차단 사유 | 진행, 쓰기, 닫기를 정직하게 계속할 수 없게 하는 구체적인 이유입니다. 해결하거나 유효하게 미뤄야 합니다. | [Core Model](core-model.md#invalid-state-combinations) |
| 잔여 위험 | 닫기에 영향을 주는 알려진 남은 불확실성, 확인하지 못한 조건, 한계, 절충입니다. | [Core Model](core-model.md#13-residual-risk) |
| 다음 안전한 행동 | 해결되지 않은 범위, 판단, 증거, QA, 검증, 수락, 위험을 숨기지 않고 진행할 수 있는 다음 행동입니다. | [API Schema Core](api/schema-core.md#nextactionsummary) |
| 권한 경계 | 무엇이 하네스 권한을 만들고 무엇이 정보로만 쓰이는지를 나누는 선입니다. Chat, projection, report는 권한이 아닙니다. | [Runtime Boundaries](runtime-boundaries.md) |
| 파생 표시 | Status card나 projection처럼 담당 기록에서 렌더링된 사용자 표시입니다. Core가 소유한 상태를 대체하지 않습니다. | [Projection과 Template](projection-and-templates.md) |
| 현재 MVP | 활성 계획 기준 MVP 참조 범위입니다. 런타임/서버 구현이 존재한다는 증거가 아닙니다. | [MVP 계획](../build/mvp-plan.md#mvp-1-included) |
| later 후보 | 담당 문서가 scope, fallback behavior, proof expectation과 함께 승격하기 전까지 active MVP 밖에 남는 future/profile material입니다. | [Later 후보 색인](../later/index.md#boundary) |

## Core 용어

아래 용어는 Core 권한을 이해하기 위한 길잡이입니다. 정확한 lifecycle, gate, close 의미는 [Core Model Reference](core-model.md)가 담당합니다.

| Core 용어 | 짧은 설명 | 담당 경로 |
|---|---|---|
| Core가 소유한 상태 | 하네스 운영 권한이 되는 커밋된 owner record와 `state.sqlite.task_events`입니다. | [Core Model](core-model.md#kernel-invariants), [Storage](storage.md) |
| Task | 사용자의 작업, 상태, 차단 사유, 증거 상태, 닫기 가능 여부, 결과를 담는 내부 단위입니다. | [Core Model](core-model.md#entity-model) |
| Change Unit | 쓰기 가능한 작업의 활성 범위 경계입니다. 그 자체로 쓰기를 승인하지 않습니다. | [Core Model](core-model.md#entity-model) |
| Autonomy Boundary | 활성 Change Unit 안에서 에이전트가 다시 묻지 않고 결정할 수 있는 선택의 경계입니다. Scope grant, approval, write authority가 아닙니다. | [Core Model](core-model.md#autonomy-boundary) |
| `user_judgment` | 사용자가 소유하는 판단을 위한 canonical record family입니다. | [Core Model](core-model.md#judgment-route-boundaries), [API Schema Core](api/schema-core.md#userjudgment) |
| Gate | 진행, 쓰기, run 기록, 닫기에 대한 Core 호환성 축입니다. 필요한지는 활성 담당 경로가 정합니다. | [Core Model](core-model.md#gates) |
| Blocker | 진행, 쓰기, 닫기 또는 요청된 다음 단계를 정직하게 계속할 수 없는 구조화된 이유입니다. | [Core Model](core-model.md#invalid-state-combinations) |
| Write Authorization | 호환되는 non-dry-run `prepare_write`만 만드는 1회용 협력형 Core record입니다. OS permission이나 isolation이 아닙니다. | [Core Model](core-model.md#write-authorization) |
| Run | 실행 또는 관찰을 남기는 커밋된 record입니다. Product-write Run은 호환되는 active Write Authorization을 소비해야 합니다. | [Core Model](core-model.md#record_run) |
| `prepare_write` | 제품 파일 쓰기를 위한 Core의 사전 쓰기 호환성 판단 지점입니다. 공개 API method는 `harness.prepare_write`입니다. | [Core Model](core-model.md#prepare_write), [MVP API](api/mvp-api.md#harnessprepare_write) |
| `record_run` | 실행 또는 관찰을 기록하고 필요한 경우 호환되는 Write Authorization을 소비하는 Core 경로입니다. 공개 API method는 `harness.record_run`입니다. | [Core Model](core-model.md#record_run), [MVP API](api/mvp-api.md#harnessrecord_run) |
| `close_task` | Core의 완료 판단 지점입니다. 공개 API method는 `harness.close_task`입니다. | [Core Model](core-model.md#close_task), [MVP API](api/mvp-api.md#harnessclose_task) |

## API/schema 식별자

Schema, API 문서, record, example, file path, diagnostic output, code-like prose에서는 아래 이름을 정확히 유지합니다. 의미와 value set은 [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)가 담당합니다.

| 식별자 | 짧은 설명 |
|---|---|
| Active MCP methods | `harness.intake`, `harness.status`, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.close_task`. |
| Request/response base shapes | `ToolEnvelope`, `ToolResponseBase`, `ToolError`, `EventRef`는 공통 호출 식별자, state version, error, validator result, event ref를 담습니다. |
| State/display summary shapes | `StateSummary`, `StateRecordRef`, `NextActionSummary`, `GuaranteeDisplay`는 현재 상태, ref, 다음 행동, 보장 표시를 전달합니다. |
| `ArtifactRef` / `ArtifactInput` | 공개 artifact pointer와 `record_run`이 받을 수 있는 artifact input shape입니다. |
| `EvidenceSummary` / `EvidenceCoverageItem` | Compact evidence status와 claim별 coverage shape입니다. |
| `AuthorizedAttemptScope` | 허용된 write attempt 하나의 정확한 stored scope입니다. |
| `WriteAuthorizationSummary` / `WriteAuthoritySummary` | Write Authorization 하나와 현재 write-authority position을 보여주는 public summary입니다. |
| `RunSummary` / `ObservedChanges` | Run result와 관찰된 변경 요약 shape입니다. |
| Judgment shapes | `UserJudgment`, `UserJudgmentCandidate`, `RecordUserJudgmentPayload`, `AcceptedRiskInput`은 판단 요청, 후보, resolution, 잔여 위험 수락 input을 나타냅니다. |
| `judgment_kind` | Canonical user judgment kind field입니다. Active value는 `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, `cancellation`입니다. |
| `presentation` | Active MVP prompt/detail field입니다. `short`가 active이고, `full` Decision Packet presentation은 later/profile material입니다. |
| `CloseBlocker` | Structured close/progress blocker result입니다. Prose-only report text는 blocker result가 아닙니다. |
| `ValidatorResult` | Structured validator output입니다. Active stable validator ID는 `surface_capability_check`입니다. |
| Sensitive categories | `auth_change`, `destructive_write`, `secret_access`, `privacy_or_pii_change`, `policy_override` 같은 exact sensitive-category value는 API Schema Core가 담당합니다. |
| Public error codes | `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `PROJECTION_STALE` 같은 stable public error는 API Errors가 담당합니다. |

## Storage 용어

Storage 용어는 향후 하네스 record가 어디에 사는지 알려줍니다. 정확한 table role, JSON `TEXT` 규칙, state clock, lock, migration, artifact handling은 [Storage](storage.md)가 담당합니다.

| Storage 용어 | 짧은 설명 | 담당 경로 |
|---|---|---|
| 제품 저장소 | 사용자의 제품 작업 공간입니다. 제품 파일은 가까이 있다는 이유만으로 하네스 운영 권한이 되지 않습니다. | [Runtime Boundaries](runtime-boundaries.md#1-product-repository) |
| 하네스 서버 / 설치 | 향후 local Harness control-plane program입니다. 일반 OS sandbox나 permission system이 아닙니다. | [Runtime Boundaries](runtime-boundaries.md#2-harness-server--installation) |
| 하네스 런타임 홈 | Registry, project state, artifact를 담는 사용자별/설치별 운영 데이터 홈입니다. | [Runtime Boundaries](runtime-boundaries.md#3-harness-runtime-home), [Storage](storage.md#2-runtime-home-identity) |
| Runtime identity files | `registry.sqlite`는 Runtime Home identity와 최소 project registration을 저장하고, `project.yaml`은 static project configuration을 저장하며, `state.sqlite`는 project-local Core state를 저장합니다. | [Storage](storage.md#2-runtime-home-identity) |
| Active storage records | `project_state`, `surfaces`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, `tool_invocations`. | [Storage](storage.md#4-tables) |
| JSON `TEXT` columns | Core/API/storage validation 이후 owner-shaped JSON을 저장하는 SQLite `TEXT` columns입니다. 임의 JSON container가 아닙니다. | [Storage](storage.md#5-json-text-columns) |
| Artifact storage links | `artifacts`와 `artifact_links`는 증거 바이트나 안전한 metadata를 등록하고 owner record와 연결합니다. Link 자체가 gate를 만족하지는 않습니다. | [Storage](storage.md#6-artifact-references) |
| Event and replay storage | `task_events`는 committed mutation audit trail이고, `tool_invocations`는 committed idempotency replay row입니다. | [Storage](storage.md) |
| State clocks and hashes | `state_version`, `project_state.state_version`, `tasks.state_version`, `tree_hash`, `request_hash`는 stale-state, baseline, idempotency 확인을 지원합니다. | [Storage](storage.md), [API Errors](api/errors.md) |

## 보안 보장 용어

보안 표현은 담당 문서가 정의하고 증명한 control 수준과 맞아야 합니다. 정확한 guarantee 의미와 non-claim은 [보안 참조](security.md)가 담당합니다.

| 보안 용어 | 뜻 |
|---|---|
| `cooperative` | 연결된 surface가 절차를 따를 때 하네스가 Harness state-changing path를 안내, 기록, 비교, 거부할 수 있다는 뜻입니다. 물리적 차단이 아닙니다. |
| `detective` | 행동 이후나 관찰 가능해진 시점에 지원되는 fact를 감지, 기록, 보고할 수 있다는 뜻입니다. 사전 차단이 아닙니다. |
| `preventive` | 이름 붙은 mechanism이 covered operation을 실행 전에 막을 수 있다는 claim입니다. 현재 MVP에는 default preventive claim이 없습니다. |
| `isolated` | 이름 붙고 증명된 separation boundary가 covered operation에서 어떤 것을 다른 것에서 격리한다는 claim입니다. 현재 MVP에는 default isolation claim이 없습니다. |
| honest guarantee display / capability overclaim | 사용자에게 보이는 guarantee text는 `capability_profile` fact와 owner proof level에 맞아야 합니다. 지원되지 않는 강한 claim은 표시를 낮추거나 blocker/error를 반환하거나 instruction으로 hold해야 합니다. |
| explicit non-claims / trust boundary | 현재 MVP는 OS-level permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, security isolation을 제공하지 않습니다. |

## 에이전트/context 용어

에이전트와 connector 용어는 surface가 owner record를 낮은 context 비용으로 쓰는 방법을 설명합니다. 정확한 connector 동작은 [Agent 통합 참조](agent-integration.md)가 담당합니다.

| 에이전트/context 용어 | 짧은 설명 |
|---|---|
| agent surface / `surface_id` | Agent가 context를 읽고 MCP를 호출하고 작업을 수행하는 연결 환경과 API caller identifier입니다. Surface name이나 `surface_id`만으로 capability나 권한이 생기지 않습니다. |
| `capability_profile` | Surface가 실제로 할 수 있는 일을 선언하고 refresh한 fact입니다. MCP posture, observation, capture, guard, isolation support를 포함합니다. |
| connector manifest | Connector-managed path, snippet, managed block hash, profile freshness, drift, fallback behavior를 위한 generated manifest입니다. |
| always-on context | 한 화면 이하의 현재 context입니다. Task summary, scope, pending judgment, blocker, next safe action, evidence gap, close blocker, residual risk, guarantee level, fresh ref만 둡니다. |
| phase-relevant context / push-pull | Compact current context를 push하고, planning, write preparation, evidence review, close readiness, judgment request, recovery에 필요한 담당 section만 pull하는 방식입니다. |
| Role Lens | Read-only posture guidance입니다. Lens recommendation은 owner path가 action을 기록하기 전까지 권한이 없습니다. |
| reference local MCP surface | Active reference integration profile인 `reference-local-mcp`입니다. 지원되는 범위에서만 cooperative behavior와 limited detective behavior를 표시합니다. |
| fallback behavior | Core, MCP, projection, local access, capability가 unavailable 또는 insufficient일 때의 connector response입니다. |

## later 용어

Later 용어는 후보 또는 전달 label입니다. 담당 문서가 승격하기 전에는 active API/schema/storage contract, fixture body, runtime behavior, generated artifact, MVP-1 requirement가 아닙니다.

| later 용어 | 현재 상태 | 담당 경로 |
|---|---|---|
| 내부 엔지니어링 점검(Engineering Checkpoint) | 첫 향후 internal authority-loop smoke입니다. Product MVP가 아닙니다. | [MVP 계획](../build/mvp-plan.md#first-internal-smoke-target) |
| `Kernel Smoke` | 내부 엔지니어링 점검 아래의 좁은 future smoke-check 작성 label입니다. Stage name이 아닙니다. | [MVP 계획](../build/mvp-plan.md#first-internal-smoke-target) |
| MVP-1 사용자 작업 루프(MVP-1 User Work Loop) | 내부 smoke target 이후의 첫 좁은 사용자 가치 milestone입니다. | [MVP 계획](../build/mvp-plan.md#user-work-loop) |
| 보증 프로필(Assurance Profile) | Assurance behavior를 나중에 단단하게 만드는 범위입니다. | [Later](../later/index.md#assurance-candidates) |
| 운영 프로필(Operations Profile) | Operations와 handoff behavior를 나중에 단단하게 만드는 범위입니다. | [Later](../later/index.md#operations-candidates) |
| 로드맵(Roadmap) | Owner 문서가 승격하고 증명하기 전까지 future scope입니다. | [Later](../later/index.md#roadmap-candidates) |
| 강화된 로컬 기준 목표(hardened local reference target) | MVP-1 이후 owner-defined Assurance Profile과 Operations Profile work를 완료해 도달하는 상위 목표입니다. 추가 stage나 suite가 아닙니다. | [번역 가이드](../maintain/translation-guide.md) |
| Context Index | 나중의 read-only retrieval support입니다. Write 승인, gate 충족, risk 수락, close를 대신하지 않습니다. | [Later](../later/index.md#roadmap-candidates) |
| Journey Card / Journey Spine | 나중의 continuity display입니다. 활성화되고 최신이면 방향 잡기에 도움을 주지만 Core가 소유한 상태는 아닙니다. | [Later](../later/index.md#later-template-candidates) |
| Browser QA Capture | Roadmap capture support 후보입니다. 그 자체로 수동 QA, 최종 수락, detached verification이 아닙니다. | [Later](../later/index.md#roadmap-candidates) |

## 폐기 / 호환성 용어

아래 용어는 호환성 payload나 label과의 혼동을 막을 때만 남깁니다. 새 active 문서의 primary concept로 쓰지 않습니다.

| 용어 | 호환성 메모 | 현재 경로 |
|---|---|---|
| Decision Packet | Full-format user-judgment presentation이자 compatibility label입니다. Active MVP는 compact `presentation=short`를 쓰며, `presentation=full`은 later/profile material입니다. | [API Schema Core](api/schema-core.md#userjudgment), [Later](../later/index.md#assurance-candidates) |
| `request_user_decision` / `record_user_decision` | `request_user_judgment` / `record_user_judgment`의 compatibility alias입니다. | [API Schema Core](api/schema-core.md#stage-specific-active-value-sets) |
| `judgment_type`, `judgment_domain`, `decision_kind`, `decision_profile` | Compatibility alias입니다. `judgment_kind`, route-specific payload validation, `presentation`을 우선합니다. | [API Schema Core](api/schema-core.md#userjudgment) |
| `display_label` | Surface가 이 이름을 노출할 때의 compatibility 또는 response-only display label입니다. Active canonical schema/storage field가 아니며, label은 `judgment_kind`와 locale에서 렌더링합니다. | [API Schema Core](api/schema-core.md#userjudgment), [Storage](storage.md#4-tables) |
| `MCP_SERVER_UNAVAILABLE` / `SURFACE_MCP_UNAVAILABLE` | Diagnostic condition입니다. Stable public availability code는 `MCP_UNAVAILABLE`입니다. | [Agent Integration](agent-integration.md#8-fallback-behavior), [API Errors](api/errors.md) |
