# Build: 구현 개요

## 이 문서가 도와주는 일

이 문서는 구현자가 전체 reference 명세에 들어가기 전에 무엇을 먼저 계획해야 하는지 알려 줍니다. 독자 중심 문서가 kernel, runtime, MCP, storage, projection, conformance reference와 어떻게 이어지는지 보여 주는 Build 계층입니다.

이 문서는 구현 계획 문서입니다. 재설계 문서가 승인되기 전에는 runtime/server 구현을 시작하라는 뜻이 아닙니다.

이 문서로 다음을 확인합니다.

- 먼저 필요한 runtime 구성 요소는 무엇인가?
- 첫 실행 가능한 조각은 어떤 증명을 보여야 하는가?
- MVP를 완료했다고 말하려면 어떤 증명이 필요한가?

이 문서는 SQLite DDL, public MCP 스키마, projection 템플릿 본문, 명령 문법을 정의하지 않습니다. 그런 세부 계약은 reference 문서에 둡니다.

## 이런 때 읽기

- 재설계 문서가 승인된 뒤 첫 구현 형태를 계획할 때.
- 제안된 MVP 구현 계획이 올바른 범위를 유지하는지 리뷰할 때.
- 엄밀한 reference 명세를 읽기 전에 짧은 지도가 필요할 때.

## 읽기 전에

Learn 경로에서 Harness의 기본 개념을 먼저 이해해 두는 것이 좋습니다. 정확한 동작은 이 문서 끝에 연결된 reference 문서들을 봅니다. Post-MVP 후보와 승격 기준은 [로드맵](../roadmap.md)을 봅니다.

## 문서 승인 상태

이 항목은 maintainer가 직접 갱신하는 문서 handoff 표시입니다. Reference 계약, conformance 결과, 생성된 운영 record, runtime 구현 승인으로 쓰지 않습니다. 아래 checkpoint에서 acceptance를 자동 추론하지 않습니다. Maintainer가 이 표를 명시적으로 바꿔야 합니다.

| 질문 | 현재 상태 |
|---|---|
| 문서 유지보수가 아직 active인가? | 예. 재설계된 문서는 사람의 검토를 받을 준비가 되었고, 구현 handoff는 아직 accepted로 기록되지 않았습니다. |
| 첫 runtime batch 계획을 위한 문서가 승인되었는가? | 아니오. 아래 checkpoint가 충족된 뒤 maintainer가 이 행을 예로 바꾸기 전까지 첫 runtime batch 계획은 시작할 수 없습니다. |
| runtime/server 구현이 시작되었는가? | 아니오. 이 저장소는 아직 문서만 담고 있으며 Harness runtime/server 구현을 담고 있지 않습니다. |
| 열려 있는 문서 follow-up issue가 있는가? | 예. Acceptance 상태를 바꾸기 전에 아래 maintainer-updated follow-up 목록을 봅니다. |

### 알려진 문서 follow-up issue

Maintainer가 이 목록을 갱신합니다. 이 항목들은 문서 유지보수만 안내하며 Reference 계약, conformance 결과, 생성된 운영 record, runtime/server 승인을 만들지 않습니다.

- Open - repo-level `.agents` / `.codex` instruction audit: repo-level `.agents`와 `.codex` placeholder가 의도된 instruction surface인지, generated connector artifact인지, 제거할 placeholder인지 결정합니다. Owner docs: [문서 작성 가이드의 진입점 규칙](../maintain/authoring-guide.md#진입점-규칙)과 [Surface Cookbook: Codex](../reference/surface-cookbook.md#codex). `TODO_DECISION`: 문서 승인 전에 owner와 기대 처리 방식을 정합니다. `TODO_IMPLEMENT`: 결정 뒤 stale repo-level instruction placeholder를 갱신하거나 제거합니다.
- 이번 batch에서 resolved - User Guide opening convention alignment: [사용자 가이드](../use/user-guide.md)가 이미 [문서 작성 가이드의 문서 시작 방식](../maintain/authoring-guide.md#문서-시작-방식)을 따르고, startup phrase 없음 convention을 [Agent 세션 흐름](../use/agent-session-flow.md)과 맞게 유지함을 확인했습니다.

## 구현 handoff checkpoint

이 checkpoint는 maintainer가 문서 승인 상태를 문서 유지보수에서 첫 runtime batch 계획으로 바꾸기 전에 무엇이 참이어야 하는지 판단할 때 사용합니다. 이것은 계획 handoff일 뿐입니다. 그 자체로 runtime/server 구현을 승인하지 않으며, 정확한 schema, DDL, fixture 의미, runtime contract를 정의하지 않습니다.

첫 구현 계획은 아래 조건이 모두 참일 때만 시작할 수 있습니다.

- 최종 docs-maintenance drift pass가 완료되었거나, 남은 알려진 gap이 관련 owner 문서에 `TODO_DECISION` 또는 `TODO_IMPLEMENT`로 기록되어 있다. Docs-maintenance는 읽기 전용 문서 점검으로 남습니다. [문서 작성 가이드](../maintain/authoring-guide.md#docs-maintenance-checks)와 [운영과 Conformance 참조](../reference/operations-and-conformance.md#docs-maintenance-프로필)를 봅니다.
- MVP의 local-only MCP 노출 baseline이 승인되어 있다. Remote, shared, tunneled, non-loopback 노출은 owner 문서가 connector profile을 승격하고 증명하기 전까지 MVP baseline 밖입니다. [런타임 아키텍처](../reference/runtime-architecture.md#로컬-접근-기대사항)와 [MCP API와 스키마](../reference/mcp-api-and-schemas.md#mcp-경계와-호출자-신뢰)를 봅니다.
- Core-only mutation model이 승인되어 있다. 상태 변경 작업은 Core를 거치고, resource, projection, report, diagnostic은 Core 경로가 상태를 commit하지 않는 한 read-only 또는 derived로 남습니다. [Core process model](../reference/runtime-architecture.md#core-process-model)과 [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow)를 봅니다.
- Kernel Smoke fixture queue가 첫 runtime conformance 작성 순서로 확인되어 있다. 정확한 fixture format, assertion, catalog semantics는 [운영과 Conformance 참조](../reference/operations-and-conformance.md#kernel-smoke-authoring-queue)에 둡니다.
- 첫 실행 가능한 조각은 local, single-project, single-reference-surface, fixture-proven 범위를 유지한다. 계획 점검 목록은 [첫 실행 가능한 조각](first-runnable-slice.md)을 사용합니다.
- Post-MVP feature는 [로드맵 승격 규칙](../roadmap.md#승격-규칙)에 따라 owner 문서가 승격하기 전까지 MVP 밖에 남아 있다.

이 handoff는 roadmap 항목, dashboard, Browser QA Capture automation, Context Index, broad connector marketplace, remote MCP exposure, preventive guard expansion, parallel orchestration을 MVP로 승격하지 않습니다. 정확한 계약은 Reference 문서에 두고, 이 섹션은 짧은 readiness checkpoint로만 사용합니다.

## 핵심 생각

가장 작은 로컬 Core 권한 경로를 먼저 증명하고, 그다음 근거, projection, conformance, 운영자 복구 경로를 붙여 단단하게 만듭니다.

기준 상태, `task_events`, artifact refs, Core tool 동작, 그리고 그 경로를 실행해 볼 최소 reference surface와 MCP reachability에서 시작합니다. Projection template 다듬기, dashboard, index, 넓은 connector ecosystem 또는 marketplace, 접점별 connector automation, hook expansion, Browser QA automation, broad automation은 그 권한 루프가 존재한 뒤 그것을 읽거나 감싸는 권한 없는 요소로 다룹니다.

구현 계획이 projection template 다듬기, dashboard, Context Index, connector marketplace, hook expansion, broad automation lane에서 시작한다면 순서가 잘못된 것입니다.

## 증명 경계

| 경계 | 증명하는 것 | 사용자 또는 운영자가 관찰할 수 있는 것 |
|---|---|---|
| Kernel Smoke | 하나의 로컬 Task가 Core 권한 루프를 통과할 수 있음을 증명합니다. 여기에는 scoped write decision, Write Authorization, `record_run`, artifact로 뒷받침되는 근거, status, 최소 projection 최신성, close blocker가 포함됩니다. | Status가 active Task, gate, Change Unit, 근거, blocker, projection 최신성을 보여 줍니다. 범위 밖 작업은 차단되고, compatible scoped work는 권한을 받아 한 번만 사용되며, 근거 또는 필요한 decision이 없으면 close가 거절됩니다. |
| Agency-Hardened MVP | 로컬 reference MVP가 사용자 판단, approval, detached verification, Manual QA, Residual Risk, reconcile, recovery, export, conformance를 정직한 경계 안에서 처리함을 증명합니다. | Fixture와 operator 진입점이 같은 Core record와 error를 통해 work가 계속될 수 있는지, 멈춰야 하는지, verify, accept, export, recover, close할 수 있는지를 보여 줍니다. |
| Post-MVP roadmap | 로컬 kernel과 agency 증명이 안정된 뒤에만 later surface 또는 automation을 검토할 수 있음을 분리합니다. | 선택 capability는 담당자가 [로드맵 승격 규칙](../roadmap.md#승격-규칙)에 따라 exact contract와 fixture로 승격하기 전까지 read-only, display-only, metadata-only, 또는 artifact 후보 제공 전용으로 남습니다. |

## 무엇을 만드는가

Harness MVP는 AI 지원 제품 작업을 위한 로컬 권한 커널입니다. 첫 구현 계획은 명확한 내부 모듈을 가진 하나의 로컬 시스템을 기준으로 하며, 분산 플랫폼으로 시작하지 않습니다.

### Local Server / Process

MCP 경계를 제공하고, Core 전이를 소유하며, runtime home을 읽고 쓰는 로컬 Harness server 또는 프로세스 하나를 계획합니다. 검증기 실행, projection 대기열 추가, reconcile, 복구, export, conformance 진입점은 모두 같은 Core 규칙 위에서 실행되어야 합니다.

MVP는 모듈을 가진 단일 프로세스로 충분합니다. Core, projection, validation, 운영자 도구를 별도 서비스로 나눌 필요는 없습니다.

### Core

Core는 운영 상태의 기준 기록을 변경하는 유일한 경로입니다. Core는 다음을 해야 합니다.

- tool envelope, idempotency key, expected state version을 검증한다
- 필요한 project 또는 task lock을 획득한다
- 현재 기록을 읽는다
- Core check와 validator를 실행한다
- 하나의 transaction에서 현재 기록을 갱신하고 `task_events`에 추가한다
- 상태 변경 뒤 projection 작업을 대기열에 넣는다
- 결과를 설명하는 막힘과 참조를 반환한다

Agent, 운영자 명령, projector, recovery flow는 Core를 통하거나 같은 Core compatibility rule을 보존해야 합니다.

### State Store

State store는 운영 상태의 기준 기록을 보관합니다. 여기에는 project state, Task, gate, Change Unit, Decision Packet, approval, Write Authorization, Run, Evidence Manifest, Eval record, Manual QA record, Residual Risk, projection job, reconcile item, `task_events`가 포함됩니다.

Build 계층에서 이를 새로 설계하지 않습니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)이 담당합니다.

### Artifact Store

Artifact store는 오래 보존해야 하는 근거 파일과 integrity metadata를 보관합니다. Raw artifact는 diff, log, screenshot, bundle, manifest, checkpoint, export component, 그 밖의 근거 파일이 될 수 있습니다.

Artifact store는 느슨한 파일 덤프가 아닙니다. Harness 상태를 뒷받침하는 모든 artifact에는 등록된 artifact ref, hash, size, redaction state, 그리고 이를 사용하는 Task 또는 owner record와의 relation이 필요합니다.

### MCP API

MCP server는 read resource와 public tool을 제공합니다. MCP resource는 read-only입니다. 상태를 변경하는 작업은 public tool과 Core를 거칩니다.

첫 Build 경로에서는 다음을 우선합니다.

- 상태와 active Task read
- intake 또는 Task creation
- next-action guidance
- `prepare_write`
- `record_run`
- 필요한 tool flow를 통한 artifact 등록
- Evidence Manifest 갱신
- `close_task` 차단 조건 동작

Public request와 response 규칙은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)가 담당합니다.

### Projections

Projection은 state record와 artifact ref에서 나온 사람이 읽기 쉬운 보기입니다. 기준 상태가 아닙니다.

Projection output은 그것이 의존하는 Core 원천 기록에서 파생합니다. 예를 들어 Task, gate, Run, artifact, evidence, Eval, QA, 그 밖의 owner record가 존재한 뒤 그 기록에서 나와야 합니다. 최소 `TASK` projection 최신성 또는 대기열 추가 경로는 Kernel Smoke에 포함될 수 있지만, projection template은 권한을 만들거나, 근거를 충족하거나, state를 대체하거나, state model을 정하거나, 첫 증명이 될 수 없습니다.

첫 실행 가능한 조각은 최소 `TASK` projection job을 대기열에 넣거나 최소 `TASK` projection을 렌더링할 수 있으면 됩니다. 최종 MVP는 원천 기록이 있을 때 MVP-required `ProjectionKind`인 `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`를 지원해야 합니다.

Projection failure는 committed Core 상태를 롤백하면 안 됩니다. Projection이 최신인지 또는 job 상태가 어떤지 표시하고, repair나 reconcile은 이후 action에 맡깁니다.

### Operator Commands

Operator 진입점은 Core 동작 위에 놓이는 경로이지 두 번째 상태 모델이 아닙니다. 먼저 command-independent 기능으로 계획합니다.

- project connect 또는 등록
- doctor/readiness 상태 표시
- MCP 경계 제공
- projection 새로고침
- human edit 또는 managed-block drift reconcile
- interrupted 또는 최신이 아닌 운영 상태 복구
- state, projection, artifact ref export
- artifact 무결성 확인
- conformance fixture 실행

정확한 command name과 flag는 나중에 정해도 됩니다. 중요한 것은 operator 동작이 MCP tool과 같은 Core 상태, event, artifact, projection, error를 사용한다는 점입니다.

## 아직 만들지 않는 것

첫 구현 계획은 좁게 유지합니다. 다음은 MVP 선행 조건으로 만들지 않습니다.

- 권한 경로로서의 dashboard 또는 rich hosted UI
- 넓은 connector ecosystem
- 권한 또는 읽기/쓰기 선행 조건으로 쓰이는 Context Index
- 필수 자동화 또는 acceptance 대체물로서의 Browser QA Capture
- 필수 assurance 경로로서의 Cross-Surface Verification
- 기준 agent 접점의 구체적인 capability를 넘어서는 native hook expansion
- 필수 집행 장치로 쓰이는 Advanced Sidecar Watcher
- MVP-critical 상태로 쓰이는 Local Derived Metrics
- team workflow, shared workspace, permission, profile import/export
- parallel orchestration automation
- 기준 agent 접점이 구체적인 pre-tool blocking 경로를 증명하지 않은 preventive guard expansion

MVP는 cooperative 또는 detective guard/freeze 상태를 표시할 수 있고, existing Change Unit, Autonomy Boundary, `prepare_write` 동작을 통해 작업을 보류하거나 범위를 좁힐 수 있습니다. 접점 label만으로 저장된 guarantee level이 올라가지는 않습니다.

유용한 later capability라도 owner 문서가 capability profile, redaction/secret/PII policy, 필요한 경우 retention 또는 test-environment rule, fixture coverage, fallback 동작, projection-as-canonical 의존성 없음을 정의하기 전까지는 읽기 전용 표시, metadata, 기존 owner path를 위한 artifact 후보, fixture candidate로만 나타날 수 있습니다.

## 첫 증명

첫 증명은 Kernel Smoke입니다. Harness가 하나의 권한 결정을 만들고 적용할 수 있음을 보여 주는 가장 작은 실행 가능한 경로입니다.

Kernel Smoke는 권한 루프를 증명하는 단계입니다. 전체 MVP, template 완성도, broad automation을 증명하는 단계가 아닙니다.

다음을 보여야 합니다.

- 등록된 프로젝트 하나와 기준 agent 접점 하나
- 현재 상태와 gate를 가진 Task 하나
- active scoped Change Unit 하나
- `prepare_write`가 권한 없는 쓰기를 차단하고 compatible scoped 쓰기를 허용함
- 허용된 `prepare_write`가 durable Write Authorization을 만듦
- `record_run`이 `direct` 또는 구현 Run에서 그 Write Authorization을 한 번 사용한 것으로 기록함
- artifact를 등록하고 Run 또는 근거에 연결할 수 있음
- 최소 Evidence Manifest가 뒷받침 여부 또는 불충분 상태를 기록함
- 최소 `TASK` projection이 최신이거나 적어도 내구성 있게 대기열에 추가됨
- 근거 또는 decision 요구사항이 없으면 `close_task`가 차단함
- 같은 동작이 basic Core fixture로 실행 가능함

Kernel Smoke는 최종 MVP가 아닙니다. 쓰기 권한 경로가 살아 있음을 증명하는 단계입니다.

## 최종 MVP 증명

최종 증명은 Agency-Hardened MVP입니다. Agent가 정직한 경계 안에서 행동하기 위해 필요한 나머지 conformance를 추가합니다.

- Decision Packet 품질과 사용자 판단 라우팅
- approval, Decision Packet, Write Authorization의 분리
- acceptance와 close 전에 남은 위험을 표시하는 규칙
- detached verification 독립성
- Manual QA 기록과 QA 차단 조건
- feedback-loop, TDD, stewardship, context-hygiene validators
- projection과 reconcile 완전성
- recovery, export, artifact integrity 동작
- broad automation을 MVP 밖에 두는 later 경계 확인
- 필수 agency conformance fixture 적용 범위

Agency-Hardened MVP는 생성된 문장뿐 아니라 Core 상태, events, artifacts, projections, errors로 동작을 증명할 때 완료됩니다.

## Build 읽기 경로

Build 계층은 다음 순서로 읽습니다.

1. [구현 개요](implementation-overview.md): 무엇을 만드는지 확인합니다.
2. [첫 실행 가능한 조각](first-runnable-slice.md): 가장 먼저 계획할 최소 증명을 확인합니다.
3. [MVP 계획](mvp-plan.md): MVP-0부터 MVP-5까지 단계별 구현을 확인합니다.

그다음 정확한 동작은 reference 문서와 현재 담당 문서를 봅니다.

- [커널 참조](../reference/kernel.md): entity, gate, state logic, `prepare_write`, `close_task`.
- [런타임 아키텍처 참조](../reference/runtime-architecture.md): runtime space, Core flow, artifact, projection/reconcile, guarantee level.
- [MCP API와 스키마](../reference/mcp-api-and-schemas.md): public resource, tool, schema, error, artifact ref.
- [Storage와 DDL](../reference/storage-and-ddl.md): runtime layout과 DDL, migration, lock, artifact, baseline, projection job, validator-run storage를 다룹니다.
- [운영과 Conformance 참조](../reference/operations-and-conformance.md): operator semantics와 fixture expectation.
