# Build: 구현 개요

## 이 문서가 도와주는 일

이 문서는 구현자가 전체 reference 명세에 들어가기 전에 무엇을 먼저 계획해야 하는지 알려 줍니다. 독자 중심 문서가 kernel, runtime, MCP, storage, projection, conformance reference와 어떻게 이어지는지 보여 주는 Build 계층입니다.

이 문서는 문서 재설계 / 피드백 반영과 handoff review를 위한 구현 계획 문서입니다. maintainer가 문서 세트를 첫 runtime batch 계획에 사용할 수 있다고 명시적으로 승인하기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 제품 MVP 목표는 v0.1 Kernel MVP이며, Kernel Smoke conformance profile이 이를 실행 가능한 방식으로 증명합니다. 즉 모듈을 가진 로컬 프로세스 하나로 권한 루프 하나를 증명합니다. v0.2부터 v0.4까지는 Agency-Hardened MVP reference conformance target을 향한 staged pack입니다. v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 roadmap 범위에 둡니다.

이 문서로 다음을 확인합니다.

- 먼저 필요한 runtime 구성 요소는 무엇인가?
- 첫 실행 가능한 조각은 어떤 증명을 보여야 하는가?
- Agency-Hardened MVP reference conformance target을 완료했다고 말하려면 어떤 staged proof가 필요한가?

이 문서는 SQLite DDL, public MCP 스키마, projection 템플릿 본문, 명령 문법을 정의하지 않습니다. 그런 세부 계약은 reference 문서에 둡니다.

## 이런 때 읽기

- maintainer handoff가 문서를 첫 runtime batch 계획에 사용할 수 있다고 명시적으로 승인한 뒤 첫 구현 형태를 계획할 때.
- 제안된 MVP 구현 계획이 올바른 범위를 유지하는지 리뷰할 때.
- 엄밀한 reference 명세를 읽기 전에 짧은 지도가 필요할 때.

## 읽기 전에

Learn 경로에서 하네스의 기본 개념을 먼저 이해해 두는 것이 좋습니다. 정확한 동작은 이 문서 끝에 연결된 reference 문서들을 봅니다. v1+ Expansion 후보와 승격 기준은 [로드맵](../roadmap.md)을 봅니다.

## 핵심 생각

하네스는 AI 지원 제품 작업을 위한 로컬 작업 장부이자 판단 라우터입니다. 무엇을 바꿀 수 있는지, 누가 판단해야 하는지, 어떤 근거가 있는지, 어떤 위험이 남았는지, 작업을 닫아도 되는지를 기록합니다. 첫 구현 경로는 evidence depth, agency hardening, operations, automation을 더하기 전에 그 로컬 장부와 판단 경로를 가장 작은 Core 권한 루프로 증명해야 합니다.

v0.1 Kernel MVP를 먼저 만듭니다. 즉 가장 작은 로컬 Core 권한 경로를 증명하며, Kernel Smoke는 그 첫 conformance profile입니다. 기준 운영 상태를 변경하는 것은 Core뿐입니다. 그다음 v0.2 Evidence & Projection Pack, v0.3 Agency Pack, v0.4 Operations Pack으로 그 경로를 단단하게 만듭니다.

이 Build 경로의 모든 구현 동사는 maintainer handoff가 문서를 첫 runtime batch planning에 사용할 수 있다고 명시적으로 승인한 뒤의 future runtime-batch planning을 설명합니다. [문서 승인 상태](#문서-승인-상태)가 첫 runtime batch 계획을 승인하지 않는 동안에는 이 문서를 scope와 handoff readiness를 리뷰하는 용도로만 사용합니다.

로컬 커널은 조율과 권한의 기록이지 제품 저장소, 소스 관리, 테스트, 코드 리뷰, 대화, 사용자 소유 제품 판단과 중요한 기술 판단을 대체하지 않습니다. 첫 경로는 상태와 닫기 출력이 무엇이 바뀌었는지, 무엇을 확인했는지, 어떤 위험이 남았는지, 어떤 결정이 필요한지 설명하도록 계획합니다.

첫 권한 루프는 좁게 유지합니다. `prepare_write`는 제품 파일 쓰기에 대한 유일한 권한 판단 지점이고, 반환된 쓰기 허가 기록은 durable하고 single-use이며, `record_run`은 관찰된 변경과 artifact를 기록하면서 하나의 compatible direct Run 또는 implementation Run에 대해 이를 consume하고, `close_task`는 유일한 완료 판단 지점입니다. 정확한 상태 로직은 [커널 참조](../reference/kernel.md#prepare_write)에, public request/response detail은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md#public-tools)에 둡니다.

기준 상태, `task_events`, 아티팩트 참조, Core tool 동작, 그리고 그 경로를 실행해 볼 최소 reference surface와 MCP reachability에서 시작합니다. 초기 구현 가정은 분산 platform이 아니라 모듈을 가진 로컬 프로세스 하나입니다. Projection template 다듬기, dashboard 또는 hosted workflow UI, index, 넓은 connector ecosystem 또는 marketplace, team workflow, 접점별 connector automation, hook expansion, Browser QA automation, derived metrics, parallel orchestration, broad automation은 그 권한 루프가 존재한 뒤 그것을 읽거나 감싸는 권한 없는 요소로 다룹니다.

구현 계획이 Agency-Hardened MVP 전체를 첫 batch로 삼거나, projection template 다듬기, dashboard 또는 hosted workflow UI, Context Index, connector marketplace, hook expansion, metrics, parallel orchestration, broad automation lane에서 시작한다면 순서가 잘못된 것입니다.

## 문서 승인 상태

이 항목은 maintainer가 직접 갱신하는 문서 handoff 표시입니다. Reference 계약, conformance 결과, 생성된 운영 record, runtime 구현 승인으로 쓰지 않습니다. 아래 checkpoint에서 acceptance를 자동 추론하지 않습니다. maintainer가 이 표를 명시적으로 바꿔야 합니다.

현재 revision status: 문서 재설계 / 피드백 반영이 진행 중입니다. 이 status marker는 runtime/server 구현, runtime conformance, 구현 준비 상태가 아닙니다.

| 질문 | 현재 상태 |
|---|---|
| 문서 재설계 / 피드백 반영이 아직 active인가? | 예. 문서 전용 재설계 작업이 진행 중이며, 구현 handoff는 여전히 maintainer의 명시적 갱신이 필요합니다. |
| 첫 runtime batch 계획을 위한 문서가 승인되었는가? | 아니오. 아래 checkpoint가 충족된 뒤 maintainer가 이 행을 예로 바꾸기 전까지 첫 runtime batch 계획은 시작할 수 없습니다. |
| runtime/server 구현이 시작되었는가? | 아니오. 이 저장소는 아직 문서만 담고 있으며 하네스 runtime/server 구현을 담고 있지 않습니다. |
| 열려 있는 문서 follow-up issue가 있는가? | 계획된 문서 재설계 후속 작업은 남아 있습니다. 지금까지 완료된 문서 전용 재설계 변경에서 알려진 차단 docs-maintenance drift는 없습니다. 첫 runtime batch 계획이 시작되려면 maintainer가 docs-accepted 행을 여전히 명시적으로 예로 바꿔야 하며, 이 상태는 runtime conformance나 구현 준비 상태를 뜻하지 않습니다. |

Build 독자는 이 표를 진입 gate로 보아야 합니다. maintainer handoff가 두 번째 행을 예로 바꾸기 전까지 v0.1 Kernel MVP도 이 저장소에서는 planning-only이며 runtime/server 구현을 시작하면 안 됩니다.

## 구현 handoff checkpoint

이 checkpoint는 maintainer가 문서 승인 상태를 문서 유지보수에서 첫 runtime batch 계획으로 바꾸기 전에 무엇이 참이어야 하는지 판단할 때 사용합니다. 이것은 계획 handoff일 뿐입니다. 그 자체로 runtime/server 구현을 승인하지 않으며, 정확한 schema, DDL, fixture 의미, runtime contract를 정의하지 않습니다.

첫 구현 계획은 Agency-Hardened MVP나 roadmap automation이 아니라 v0.1 Kernel MVP 계획부터 시작한다는 뜻입니다. 아래 조건이 모두 참일 때만 시작할 수 있습니다.

- 최종 docs-maintenance drift pass가 완료되었거나, 남은 알려진 gap이 관련 owner 문서에 `TODO_DECISION` 또는 `TODO_IMPLEMENT`로 기록되어 있다. Docs-maintenance는 읽기 전용 문서 점검으로 남습니다. [문서 작성 가이드](../maintain/authoring-guide.md#docs-maintenance-checks)와 [운영과 Conformance 참조](../reference/operations-and-conformance.md#docs-maintenance-프로필)를 봅니다.
- v0.1 Kernel MVP의 local-only MCP 노출 baseline이 승인되어 있다. Remote, shared, tunneled, non-loopback 노출은 owner 문서가 connector profile을 승격하고 증명하기 전까지 v0.1 baseline 밖입니다. [런타임 아키텍처](../reference/runtime-architecture.md#로컬-접근-기대사항), [보안 위협 모델 참조](../reference/security-threat-model.md#mcp-local-access와-caller-boundary), [MCP API와 스키마](../reference/mcp-api-and-schemas.md#mcp-경계와-호출자-신뢰)를 봅니다.
- Reference surface capability profile이 실제 사용하는 host/profile/configuration에 대한 구체적인 declaration으로 승인되어 있다. Version, MCP config, hook, permission, workspace policy, generated file, conformance result, capture method, QA capture method, redaction policy, artifact retention behavior가 바뀌면 refresh되어야 합니다. 정확한 connector profile과 surface recipe detail은 [Agent 통합 참조](../reference/agent-integration.md#capability-profiles)와 [Surface Cookbook](../reference/surface-cookbook.md)에 둡니다.
- Core-only mutation model이 승인되어 있다. 기준 운영 상태를 변경하는 것은 Core뿐이며, resource, projection, report, diagnostic, MCP caller, operator entrypoint는 Core의 상태 변경 경로에 들어가지 않는 한 read-only 또는 derived로 남습니다. [Core process model](../reference/runtime-architecture.md#core-process-model), [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow), MCP [Idempotency](../reference/mcp-api-and-schemas.md#idempotency)와 [State Conflict 동작](../reference/mcp-api-and-schemas.md#state-conflict-동작)을 봅니다.
- Kernel Smoke fixture queue가 v0.1 Kernel MVP conformance 작성 순서로 확인되어 있다. 정확한 fixture format, assertion, catalog semantics는 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)에 둡니다.
- 첫 실행 가능한 조각은 local, single-project, single-reference-surface, fixture-proven 범위를 유지한다. 계획 점검 목록은 [첫 실행 가능한 조각](first-runnable-slice.md)을 사용합니다.
- v1+ Expansion 기능은 [로드맵 승격 규칙](../roadmap.md#승격-규칙)에 따라 owner 문서가 승격하기 전까지 v0.1 Kernel MVP, v0.2부터 v0.4까지의 staged pack, Agency-Hardened MVP 밖에 남아 있다.

이 handoff는 roadmap 항목, dashboard 또는 hosted workflow UI, Browser QA Capture automation, Context Index, broad connector ecosystem 또는 marketplace, team workflow, remote MCP exposure, preventive guard expansion, Local Derived Metrics 또는 long-term metrics, parallel orchestration을 v0.1 Kernel MVP, v0.2부터 v0.4까지의 staged pack, Agency-Hardened MVP로 승격하지 않습니다. 정확한 계약은 Reference 문서에 두고, 이 섹션은 짧은 readiness checkpoint로만 사용합니다.

## 증명 경계

| 경계 | 증명하는 것 | 사용자 또는 운영자가 관찰할 수 있는 것 |
|---|---|---|
| v0.1 Kernel MVP | 하나의 로컬 Task가 첫 Core 권한 루프를 통과할 수 있음을 증명합니다. 여기에는 project registration, Task, direct/work/advisor mode basics, Change Unit, basic 결정 패킷 lifecycle, `prepare_write`, single-use 쓰기 허가 기록, `record_run`, minimal `ArtifactRef`, minimal Evidence Manifest, status/next, minimal `TASK` projection 또는 enqueue, structured close blocker가 포함됩니다. | Status와 next가 active Task, gate, Change Unit, 결정 패킷 refs, 근거, blocker, projection 최신성을 보여 줍니다. `prepare_write`가 범위 밖 쓰기 권한을 거절하고, compatible scoped work는 권한을 받아 한 번만 사용되며, 근거 또는 필요한 decision이 없으면 `close_task`가 structured blocker와 함께 거절합니다. |
| v0.2 Evidence & Projection Pack | 첫 loop가 존재한 뒤 evidence와 projection behavior를 넓히되 projection은 파생된 보기로 남습니다. | Evidence state, artifact-backed support, projection freshness, projection failure isolation, reconcile item이 owner record에서 보입니다. |
| v0.3 Agency Pack | 로컬 reference path가 user judgment, sensitive-action Approval separation, 분리 검증, 수동 QA, residual-risk visibility와 accepted-close semantics, stewardship, TDD, feedback-loop policy를 정직한 경계 안에서 처리함을 증명합니다. | Fixture가 같은 Core record와 error를 통해 work가 진행, verify, QA 요구, accept, close될 수 있는지 보여 줍니다. |
| v0.4 Operations Pack | Operator readiness, recover/export, artifact integrity, release handoff, large fixture suite coverage, later-boundary checks가 Agency-Hardened MVP reference conformance target을 완성합니다. | Operator 진입점이 두 번째 authority model을 만들지 않고 같은 Core state 위에서 diagnose, recover, export, artifact check, conformance run을 수행합니다. |
| Roadmap 경계: v1+ Expansion | 로컬 kernel과 agency 증명이 안정된 뒤에만 later surface 또는 automation을 검토할 수 있음을 분리합니다. | 선택 capability는 담당자가 [로드맵 승격 규칙](../roadmap.md#승격-규칙)에 따라 exact contract와 fixture로 승격하기 전까지 read-only, display-only, metadata-only, 또는 artifact 후보 제공 전용으로 남습니다. |

## 무엇을 만드는가

maintainer handoff가 문서를 첫 runtime batch 계획에 사용할 수 있다고 명시적으로 승인한 뒤, 하네스 구현은 v0.1 Kernel MVP에서 시작합니다. 이는 AI 지원 제품 작업을 위한 로컬 작업 장부이자 판단 라우터입니다. 작업 흐름 주변에 지속 로컬 상태(durable local state), 아티팩트 참조, 읽기용 투영 문서(projection)를 유지하되, 제품 이력, 실행 가능한 확인, 리뷰, 사용자 판단은 기존 엔지니어링 절차에 남겨 둡니다. 사용자 판단권을 보존하는 로컬 권한 커널 원칙은 구현의 중심에 남습니다. Core가 기준 로컬 상태를 소유하고, 사용자 소유 판단은 사용자에게 남습니다. 초기 구현 가정은 명확한 내부 모듈을 가진 하나의 로컬 시스템이며, 분산 플랫폼으로 시작하지 않습니다.

아래 section은 그 runtime batch의 future responsibility를 설명합니다. 현재 documentation-acceptance phase의 work order가 아닙니다.

### Local Server / Process

MCP 경계를 제공하고, Core 전이를 소유하며, Harness Runtime Home을 읽고 쓰는 로컬 Harness Server / Installation 프로세스 하나를 계획합니다. 검증기 실행, projection 대기열 추가, reconcile, 복구, export, conformance 진입점은 모두 같은 Core 규칙 위에서 실행되어야 합니다.

v0.1 Kernel MVP는 모듈을 가진 단일 프로세스로 충분합니다. Core, projection, validation, 운영자 도구를 별도 서비스로 나눌 필요는 없습니다.

### Core

Core는 운영 상태의 기준 기록을 변경하는 유일한 경로입니다. [런타임 아키텍처](../reference/runtime-architecture.md#state-transaction-flow)가 담당하는 transaction order를 구현합니다. 순서는 envelope와 state-version validation, lock 획득, current-state read, validators, record update, `task_events` append, projection job enqueue, commit입니다. Build 계층에서 요약하면 Core는 다음을 해야 합니다.

- 새 mutation 전에 tool envelope, idempotency key, expected state version을 검증한다
- 필요한 project 또는 task lock을 획득한다
- 현재 기록을 읽는다
- Core check와 validator를 실행한다
- Core transaction 안에서 현재 기록을 갱신하고, `task_events`에 추가하고, projection 작업을 대기열에 넣는다
- 결과를 설명하는 막힘과 참조를 반환한다

Agent, MCP tool, 운영자 명령, projector, recovery flow는 Core를 통하거나 같은 Core compatibility rule을 보존해야 합니다. 어느 것도 두 번째 기준 상태 모델을 유지하면 안 됩니다.

### State Store

State store는 운영 상태의 기준 기록을 보관합니다. 여기에는 project state, Task, gate, Change Unit, 결정 패킷, Approval record, 쓰기 허가 기록, Run, Evidence Manifest, Eval record, 수동 QA record, 잔여 위험, projection job, reconcile item, `task_events`가 포함됩니다.

Build 계층에서 이를 새로 설계하지 않습니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)이 담당합니다.

### Artifact Store

Artifact store는 오래 보존해야 하는 근거 파일과 integrity metadata를 보관합니다. Raw artifact는 diff, log, screenshot, bundle, manifest, checkpoint, export component, 그 밖의 근거 파일이 될 수 있습니다.

Artifact store는 느슨한 파일 덤프가 아닙니다. 하네스 상태를 뒷받침하는 모든 artifact에는 등록된 아티팩트 참조, hash, size, redaction state, 그리고 이를 사용하는 Task 또는 owner record와의 relation이 필요합니다.

### MCP API

MCP server는 read resource와 public tool을 제공합니다. MCP resource는 read-only입니다. 상태를 변경하는 작업은 public tool과 Core를 거칩니다.

MCP server에 닿을 수 없으면 해당 call path에서 authoritative Core response가 없습니다. 첫 구현은 이를 MCP unavailable로 보고하고, reference surface의 실제 guarantee level에 따라 write-capable work를 보류하며, cached projection, generated file, chat text에서 상태를 만들어 내지 않아야 합니다.

첫 Build 경로에서는 다음을 우선합니다.

- 상태와 active Task read
- intake 또는 Task creation
- next-action guidance
- direct, work, advisor mode basics
- basic 결정 패킷 lifecycle과 blocker visibility
- 제품 파일 쓰기에 대한 유일한 권한 판단 지점으로서의 `prepare_write`
- 하나의 implementation 또는 direct 제품 파일 쓰기 Run을 위한 compatible 쓰기 허가 기록 하나의 `record_run` consumption
- 필요한 tool flow를 통한 artifact 등록
- Evidence Manifest 갱신
- 유일한 완료 판단 지점으로서의 `close_task` 차단 조건 동작

Public request와 response 규칙은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)가 담당합니다.

State conflict와 idempotency replay 동작도 그 public tool 계약의 일부입니다. Build code는 [Idempotency](../reference/mcp-api-and-schemas.md#idempotency)와 [State Conflict 동작](../reference/mcp-api-and-schemas.md#state-conflict-동작) owner section을 사용하고, durable storage detail은 [Storage와 DDL](../reference/storage-and-ddl.md)에 맡깁니다.

### Projections

읽기용 투영 문서(projection)는 state record와 아티팩트 참조에서 나온 사람이 읽기 쉬운 보기입니다. `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, 그 밖의 report projection은 기준 상태가 아닙니다.

Projection output은 그것이 의존하는 Core 원천 기록에서 파생합니다. 예를 들어 Task, gate, Run, artifact, evidence, Eval, QA, 그 밖의 owner record가 존재한 뒤 그 기록에서 나와야 합니다. 최소 `TASK` projection 최신성 또는 대기열 추가 경로는 v0.1 Kernel MVP에 포함될 수 있지만, projection template은 권한을 만들거나, 근거를 충족하거나, state를 대체하거나, state model을 정하거나, 첫 증명이 될 수 없습니다.

v0.1 조각은 최소 `TASK` projection job을 대기열에 넣거나 최소 `TASK` projection을 렌더링할 수 있으면 됩니다. Later pack은 원천 기록이 존재하거나 변경될 때 staged-delivery required `ProjectionKind` value를 지원해야 합니다. `ProjectionKind` value와 API-owned tiering은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md#shared-schemas)가 담당합니다. [문서 Projection 참조](../reference/document-projection.md#template-tiers)는 projection authority boundary, source-record rule, freshness rule, template tier presentation을 담당하고, [Template 참조](../reference/templates/README.md)는 rendered template body와 display card를 담당합니다.

Projection failure는 committed Core 상태를 롤백하면 안 됩니다. Projection이 최신인지 또는 job 상태가 어떤지 표시하고, repair나 reconcile은 이후 action에 맡깁니다. `source_state_version`과 freshness는 display/readiness fact입니다. Close/readiness output은 readable view가 오래되었거나 failed임을 보여줘야 하지만, stale Markdown이 work를 authorize하거나 close를 충족하거나 current Core state, 소스 관리, 테스트, 리뷰를 대체할 수는 없습니다.

사람이 편집할 수 있는 projection section은 proposal surface입니다. 구현 경로는 proposal -> reconcile item -> accepted Core state-changing action과 `task_events` row, 또는 reject, defer, note로 라우팅해야 합니다. Managed block direct edit는 drift이지 state change가 아닙니다.

### Operator Commands

Operator 진입점은 Core 동작 위에 놓이는 경로이지 두 번째 상태 모델이 아닙니다. 먼저 command-independent 기능으로 계획합니다.

- project connect 또는 등록
- Harness Runtime Home, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, agency/stewardship/context에 대한 doctor/readiness 상태 표시
- MCP 경계 제공
- projection 새로고침
- human edit, generated-file drift, managed-block drift를 조용히 덮어쓰거나 state로 취급하지 않고 reconcile
- baseline drift, approval drift, evaluator repo drift, artifact missing 또는 hash mismatch, projection failure, managed Markdown direct edit, MCP unavailable, surface capability mismatch를 포함한 interrupted 또는 최신이 아닌 운영 상태 복구. Recovery artifact를 successful completion proof로 취급하지 않습니다
- state snapshot, report projection snapshot, 아티팩트 참조, redaction status, omitted-secret note, retained/expired/unavailable artifact status export
- artifact 무결성 확인
- conformance fixture 실행

정확한 command name과 flag는 나중에 정해도 됩니다. 중요한 것은 command-independent behavior contract입니다. Operator 동작은 MCP tool과 같은 Core state, `task_events`, artifacts, projections, 기존 error 또는 diagnostics를 사용합니다. 상태를 변경하는 operator outcome은 Core 또는 Core ordering을 보존하는 문서화된 recovery path에 들어가야 하며, operator output이 별도 state truth가 되면 안 됩니다.

## 아직 만들지 않는 것

첫 구현 계획은 좁게 유지합니다. 아래 항목은 owner 문서가 승격하기 전까지 v0.1 Kernel MVP, v0.2부터 v0.4까지의 staged pack, Agency-Hardened MVP, 또는 authority path의 선행 조건으로 만들지 않습니다.

- 권한 경로 또는 close-readiness source로 쓰이는 dashboard, hosted workflow UI, rich UI
- 기준 agent 접점 하나를 넘어서는 넓은 connector ecosystem 또는 marketplace
- 권한 또는 읽기/쓰기 선행 조건으로 쓰이는 Context Index
- 필수 자동화 또는 acceptance 대체물로서의 Browser QA Capture
- 필수 assurance 경로로서의 Cross-Surface Verification
- 기준 agent 접점의 구체적인 capability를 넘어서는 native hook expansion
- 필수 집행 장치로 쓰이는 Advanced Sidecar Watcher
- staged-delivery-critical 상태, 권한, readiness로 쓰이는 Local Derived Metrics 또는 long-term metrics
- team workflow, shared workspace, permission, profile import/export
- parallel orchestration automation, concurrent lane scheduling, multi-agent scheduling
- 기준 agent 접점이 해당 operation에 대해 구체적인 pre-tool blocking 경로를 증명하지 않은 preventive guard expansion

v0.1 Kernel MVP는 협력형(cooperative) 또는 탐지형(detective) guard/freeze 상태를 표시할 수 있고, existing Change Unit, Autonomy Boundary, `prepare_write` 동작을 통해 작업을 보류하거나 범위를 좁힐 수 있습니다. 접점 label만으로 저장된 guarantee level이 올라가지는 않습니다.

유용한 later capability라도 owner 문서가 capability profile, redaction/secret/PII policy, 필요한 경우 retention 또는 test-environment rule, fixture coverage, fallback 동작, projection-as-canonical 의존성 없음을 정의하기 전까지는 읽기 전용 표시, metadata, 기존 owner path를 위한 artifact 후보, fixture candidate로만 나타날 수 있습니다. v0.1 Kernel MVP를 실행하거나 staged-delivery close readiness를 주장하기 위한 전제 조건이 되어서는 안 됩니다.

## 첫 증명

첫 제품 MVP 목표는 v0.1 Kernel MVP입니다. 하네스가 하나의 권한 결정을 만들고 적용할 수 있음을 보여 주는 가장 작은 실행 가능한 경로입니다. Kernel Smoke는 이 목표의 conformance profile입니다.

v0.1은 권한 루프를 증명하는 단계입니다. Agency-Hardened MVP, template 완성도, broad automation을 증명하는 단계가 아닙니다.

다음을 보여야 합니다.

- 등록된 프로젝트 하나와 기준 agent 접점 하나
- 현재 상태와 gate를 가진 Task 하나
- direct, work, advisor mode basics
- active scoped Change Unit 하나
- basic 결정 패킷 lifecycle과 blocker visibility
- `prepare_write`가 권한 없는 쓰기 권한 부여를 거절하고 compatible scoped 쓰기를 허용함
- 허용된 `prepare_write`가 durable single-use 쓰기 허가 기록을 만듦
- `record_run`이 direct Run 또는 implementation Run에서 그 쓰기 허가 기록을 한 번 사용한 것으로 기록하고 observed changes와 artifact를 기록함
- artifact를 등록하고 Run 또는 근거에 연결할 수 있음
- 최소 Evidence Manifest가 뒷받침 여부 또는 불충분 상태를 기록함
- status와 next read가 mutation을 만들지 않음
- 최소 `TASK` projection이 최신이거나 적어도 내구성 있게 대기열에 추가됨
- 근거 또는 decision 요구사항이 없으면 `close_task`가 structured blocker와 함께 차단함
- 같은 동작이 basic Core fixture로 실행 가능함

v0.1 Kernel MVP는 Agency-Hardened MVP가 아닙니다. 쓰기 권한 경로가 살아 있음을 증명하는 단계입니다. 문서 수준 승인 점검은 [첫 실행 가능한 조각](first-runnable-slice.md#문서-수준-승인-점검)을 사용하고, 정확한 fixture 의미는 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#conformance-fixture-format)를 사용합니다.

## Agency-hardened 증명

Later reference conformance target은 Agency-Hardened MVP입니다. v0.1 Kernel MVP 이후 v0.2, v0.3, v0.4 pack을 통해 도달하는 목표이지 첫 구현 batch가 아닙니다. Agent가 정직한 경계 안에서 행동하기 위해 필요한 나머지 conformance를 추가합니다.

- 결정 패킷 품질과 사용자 판단 라우팅
- sensitive-action Approval, 결정 패킷, 쓰기 허가 기록의 분리
- acceptance와 close 전에 잔여 위험을 표시하는 규칙
- 분리 검증 독립성
- 수동 QA 기록과 QA 차단 조건
- feedback-loop, TDD, stewardship, context-hygiene validators
- projection과 reconcile 완전성
- recovery, export, artifact integrity 동작
- owner 문서가 정의하는 release handoff report/export behavior
- broad automation을 v1+ Expansion에 두는 later 경계 확인
- 필수 agency conformance fixture 적용 범위

Agency-Hardened MVP는 생성된 문장뿐 아니라 Core 상태, events, artifacts, projections, errors로 동작을 증명할 때 완료됩니다.

## Build 읽기 경로

Build 계층은 다음 순서로 읽습니다.

1. [구현 개요](implementation-overview.md): 무엇을 만드는지 확인합니다.
2. [첫 실행 가능한 조각](first-runnable-slice.md): 가장 먼저 계획할 최소 증명을 확인합니다.
3. [MVP 계획](mvp-plan.md): v0.1부터 v0.4까지의 단계별 전달과 v1+ Expansion 경계를 확인합니다.

v1+ Expansion 후보와 승격 규칙은 [로드맵](../roadmap.md)을 사용합니다.

그다음 정확한 동작은 reference 문서와 현재 담당 문서를 봅니다.

- [커널 참조](../reference/kernel.md): entity, gate, state logic, `prepare_write`, `close_task`.
- [런타임 아키텍처 참조](../reference/runtime-architecture.md): runtime space, Core flow, artifact, projection/reconcile, guarantee level.
- [MCP API와 스키마](../reference/mcp-api-and-schemas.md): public resource, tool, schema, error, 아티팩트 참조, idempotency, state conflict behavior.
- [Storage와 DDL](../reference/storage-and-ddl.md): runtime layout과 DDL, migration, lock, artifact, baseline, projection job, validator-run storage를 다룹니다.
- [운영과 Conformance 참조](../reference/operations-and-conformance.md): operator semantics와 conformance run overview.
- [Conformance Fixtures 참조](../reference/conformance-fixtures.md): fixture body shape, assertion semantics, suite catalog, example.
