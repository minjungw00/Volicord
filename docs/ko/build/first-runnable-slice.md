# Build: 첫 실행 가능한 조각

## 이 문서가 도와주는 일

이 문서는 Build 개요를 구현자가 가장 먼저 계획해야 하는 작은 실행 가능한 증명으로 바꿔 줍니다.

이 문서는 구현 계획 문서입니다. 재설계 문서가 승인되기 전에는 runtime/server 구현을 시작하라는 뜻이 아닙니다.

## 이런 때 읽기

- Kernel Smoke를 계획할 때.
- 처음부터 끝까지 이어지는 권한 경로를 위한 점검 목록이 필요할 때.
- 제안된 첫 조각이 충분한 증명을 만들면서도 전체 MVP로 커지지 않았는지 리뷰할 때.

## 읽기 전에

[구현 개요](implementation-overview.md)를 먼저 읽습니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)을 봅니다.

## 핵심 생각

더 넓은 MVP를 만들기 전에, Task 하나가 Core state, `task_events`, artifact 경로 위에서 범위가 정해진 쓰기 권한, Run 기록, artifact로 뒷받침되는 근거, 상태, 최소 projection 최신성, 닫기 차단 조건을 통과하는지 증명합니다.

## 목표

하나의 로컬 Task에 대해 Harness가 권한을 행사할 수 있음을 증명하는 가장 작은 조각을 계획합니다. 이 조각은 프로젝트 하나, Task 하나, active Change Unit 하나, 허용된 쓰기 결정 하나, 기록된 Run 하나, 등록된 artifact 하나, 최소 Evidence Manifest 하나, 닫기 차단 조건 하나를 만들어야 합니다.

이 문서는 특정 command에 묶이지 않는 구현 안내서입니다. CLI 문법이 아니라 기능과 관찰 가능한 동작을 설명합니다.

여기에 전체 DDL을 포함하거나 반복하지 않습니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)이 담당합니다.

첫 조각은 projection template을 다듬는 단계도, dashboard를 만드는 단계도, 넓은 connector ecosystem이나 marketplace를 만드는 단계도, multi-surface connector expansion도, Context Index, Browser QA Capture system, hook expansion, parallel automation path도 아닙니다. Kernel Smoke에 필요한 기준 agent 접점 하나와 최소 MCP reachability는 여전히 포함합니다. 제외된 항목은 Core record와 transition이 실제로 존재한 뒤 권한 루프를 감싸거나 확장할 수 있을 뿐입니다.

## 성공 이야기

구현자는 임시 제품 저장소에 대해 로컬 Harness 프로세스를 실행한다고 가정했을 때 다음 흐름을 관찰할 수 있어야 합니다.

1. Harness가 프로젝트와 기준 agent 접점을 등록한다.
2. Task가 현재 상태와 초기 gate를 가진다.
3. Change Unit이 의도한 제품 파일 쓰기의 범위를 정한다.
4. 범위 밖의 쓰기는 차단된다.
5. 범위 안의 쓰기는 `prepare_write`에서 durable Write Authorization을 받는다.
6. `direct` 또는 구현 Run이 쓰기를 기록하고 Write Authorization을 한 번 사용한 것으로 기록한다.
7. diff나 log artifact가 등록되고 Run에 연결된다.
8. 최소 Evidence Manifest가 Run과 artifact를 참조한다.
9. 상태 조회는 상태를 변경하지 않고 현재 Task, gate, 쓰기 권한, 근거 상태, 차단 조건, projection 최신성을 보여 준다.
10. `TASK` projection이 최신이거나 렌더링을 위해 durable queue에 들어간다.
11. 근거 또는 결정 요구사항이 아직 충족되지 않았으면 close가 차단된다.

이 흐름을 통과하면 커널 권한 경로가 동작한다는 뜻입니다. Agency-Hardened MVP가 완료되었다는 뜻은 아닙니다.

관찰 결과는 단순해도 됩니다. 사용자 또는 운영자는 현재 Task, 쓰기가 차단되거나 허용된 이유, 어떤 Write Authorization이 사용되었는지, 어떤 artifact가 Run을 뒷받침하는지, Evidence Manifest가 충분한지, `TASK` projection이 최신이거나 대기열에 있는지, close가 왜 아직 막히는지를 볼 수 있어야 합니다.

## Build 순서

### 1. Runtime Home Bootstrap

대화 기록이나 생성된 Markdown 밖에 로컬 Harness 권한을 만들 수 있을 만큼의 runtime home 지원을 계획합니다.

점검 목록:

- 구성 가능한 runtime home을 만들거나 선택한다.
- Registry store, project runtime area 하나, project state store 하나, artifact store를 초기화한다.
- 프로젝트 범위 상태 변경이 의존할 project-level state version을 먼저 기록한다.
- honest cooperative 또는 detective guarantee level을 가진 기준 agent 접점 하나를 등록한다.
- Runtime home, project state, artifact store가 있는지 알려 주는 readiness read를 제공한다.

완료 기준:

- 새 환경을 반복 초기화해도 중복 권한 기록이 생기지 않는다.
- 읽기 전용 상태 조회가 Core 상태에서 "active Task 없음"을 표시할 수 있다.

### 2. Project Registration

multi-project 문제를 다루기 전에 로컬 제품 저장소 하나만 등록합니다.

점검 목록:

- project id, display name, repo root, runtime path, 정적 프로젝트 설정을 저장한다.
- 프로젝트를 기준 agent 접점에 연결한다.
- 정적 프로젝트 설정과 현재 Task 상태를 분리한다.
- 같은 project identity에 대해 등록이 idempotent하게 동작하게 한다.

완료 기준:

- Core가 이후 모든 Task-scoped action에 대해 현재 프로젝트를 찾을 수 있다.
- doctor/readiness가 unregistered repo와 registered but idle repo를 구분할 수 있다.

### 3. One Task Record

Core 또는 같은 검증 규칙을 사용하는 fixture seed 경로를 통해 첫 Task를 만듭니다.

점검 목록:

- mode, lifecycle phase, result, close reason, assurance level, state version, current summary, gate 상태를 저장한다.
- 선택된 mode에 대해 gate를 보수적으로 초기화한다.
- Task가 생성되거나 변경될 때 `task_events`에 추가한다.
- 상태 조회에서 active Task 조회를 제공한다.

완료 기준:

- 시스템이 active Task 하나와 그 state version을 보여 줄 수 있다.
- 오래된 expected state version을 가진 상태 변경 request가 reject되거나 state conflict를 반환한다.

### 4. One Change Unit

제품 파일 쓰기의 범위를 정할 active Change Unit 하나를 추가합니다.

점검 목록:

- intended operation, allowed paths, allowed tools 또는 command classes, sensitive categories, completion conditions, 근거 expectation을 기록한다.
- 최소 Autonomy Boundary를 기록한다. Agent가 무엇을 할 수 있는지, 무엇이 사용자 판단을 요구하는지, stop condition이 무엇인지 포함한다.
- Change Unit을 active Task에 연결하고 active 쓰기 범위로 만든다.
- dependency metadata는 첫 조각에서 ordering, visibility, 닫기 차단 조건에 필요할 때만 둔다.

완료 기준:

- 상태 조회가 무엇을 바꿀 수 있고 무엇이 여전히 사용자 판단을 요구하는지 설명할 수 있다.
- Active compatible Change Unit 없는 제품 파일 쓰기는 쓰기 권한을 받을 수 없다.

### 5. `prepare_write` Allow/Block

첫 의미 있는 gate를 계획합니다.

점검 목록:

- request envelope, idempotency key, project id, Task id, expected state version을 검증한다.
- active Task와 active Change Unit을 찾는다.
- intended paths, tools, commands, network targets, secrets, sensitive categories를 Change Unit과 비교한다.
- 첫 조각에 필요한 수준으로 baseline freshness를 확인한다.
- missing authority를 차단할 만큼 approval과 Decision Packet 요구사항을 확인한다.
- 접점 능력을 정직하게 확인하고 cooperative 또는 detective limit을 표시한다.
- scope, state version, approval, decision, baseline, 능력이 맞지 않으면 차단 조건을 반환한다.
- 허용되면 durable single-use Write Authorization을 만든다.
- 같은 committed request의 idempotent replay에서는 두 번째 authorization을 만들지 않고 committed response를 반환한다.

완료 기준:

- active Change Unit이 없으면 차단된다.
- 범위 밖의 intended path는 차단된다.
- Compatible scoped 쓰기가 Write Authorization ref를 반환한다.
- 그 ref 없이는 구현 Run이나 `direct` Run으로 제품 파일 쓰기를 기록할 수 없다.

### 6. `record_run`

`direct` 또는 구현 Run 하나를 기록하고 Write Authorization을 한 번 사용한 것으로 기록합니다.

점검 목록:

- 제품 파일 쓰기를 기록하는 `direct` 또는 구현 Run에는 compatible, unexpired, unconsumed Write Authorization을 요구한다.
- successful commit에서 Write Authorization을 정확히 한 번 consumed로 표시한다.
- actor, 접점, kind, intended operation, observed changes, command results, artifact refs, summary, Run 상태를 기록한다.
- authorization 밖의 observed change를 감지하고 violation, 차단 조건, 최신이 아닌 근거, 또는 Decision Packet 경로로 연결한다.
- 현재 기록 갱신과 같은 transaction에서 `task_events`에 추가한다.

완료 기준:

- 쓰기 권한 없는 `record_run`이 차단된다.
- Compatible authority가 있는 `record_run`이 한 번 성공한다.
- 같은 committed Run request replay가 idempotent하다.
- 두 번째 distinct Run은 consumed Write Authorization을 재사용할 수 없다.

### 7. Artifact Registration

첫 durable 근거 파일을 등록합니다.

점검 목록:

- approved staged file 또는 existing committed artifact ref를 받는다.
- 제공된 hash와 size를 검증한다.
- 최종 저장 전에 redaction 또는 secret omission을 적용한다.
- Artifact bytes를 artifact store에 저장한다.
- Artifact metadata와 Task, Run, Evidence Manifest, 또는 다른 owner record와의 relation을 저장한다.
- API docs의 public shape를 따르는 `ArtifactRef`를 반환한다.

완료 기준:

- Run이 등록된 artifact를 참조할 수 있다.
- State와 stored bytes로 artifact integrity를 확인할 수 있다.
- Raw secret은 근거로 저장하지 않고 omitted 또는 blocked 처리된다.

### 8. Minimal Evidence Manifest

record와 artifact ref에서 첫 근거 summary를 만듭니다.

점검 목록:

- 하나 이상의 completion condition 또는 acceptance criterion을 Run ref와 artifact ref에 매핑한다.
- 닫기 차단 조건에 필요한 수준에서 supported, unsupported, not applicable, partial, sufficient, stale, blocked 근거를 구분한다.
- 대화 텍스트나 projection prose를 근거로 취급하지 않는다.
- Manifest와 related record에서 evidence gate를 갱신한다.

완료 기준:

- Completed Run이 partial 또는 sufficient 근거 상태를 만들 수 있다.
- Required evidence가 없으면 close가 차단된다.

### 9. Minimal Status Resource

현재 작업 상태를 변경 없이 제공합니다.

점검 목록:

- 프로젝트, active Task, 현재 gate, active Change Unit, 쓰기 권한 요약, active Decision Packet refs, 근거 상태, 닫기 차단 조건, projection 최신성을 읽는다.
- 사용자나 agent가 resume할 수 있을 만큼의 Journey Card-style context를 포함한다.
- Read 동작에서 event를 추가하거나, projection을 대기열에 넣거나, artifact를 만들거나, gate를 충족하거나, 쓰기를 허가하거나, Task를 닫지 않는다.

완료 기준:

- 다른 action이 상태를 바꾸지 않는 한 반복 상태 조회가 같은 state version을 반환한다.
- Stale projection 또는 missing evidence는 조용히 repair되지 않고 상태에 표시된다.

### 10. Minimal `TASK` Projection Or Projection Enqueue

State와 readable output이 분리되어 있음을 증명하는 가장 작은 projection 동작을 계획합니다.

이 단계는 Task, gate, Run, artifact, evidence record가 존재한 뒤에 진행합니다. Projection template이 state model을 정하게 만들지 않고, 첫 조각을 완성된 것처럼 보이게 하려고 template polish나 추가 renderer-first 작업을 넣지 않습니다.

점검 목록:

- Task state가 바뀌면 `TASK` projection job을 대기열에 넣거나, commit 뒤 최소 managed `TASK` projection을 렌더링한다.
- source state version과 projection 최신성을 추적한다.
- Projection 렌더링 실패를 Core 상태 failure가 아니라 projection failure로 취급한다.
- Markdown projection은 기준 기록이 아니라 파생 보기라는 rule을 유지한다.

완료 기준:

- Task-changing action이 projection 최신성을 반환하거나 기록한다.
- Projection failure를 Task 변경 rollback 없이 표현할 수 있다.

### 11. Close Blocker Smoke

Required authority 또는 근거가 없을 때 close가 work를 끝내지 못하게 합니다.

점검 목록:

- gate, evidence, Decision Packet, approval state, residual-risk visibility, QA, verification, acceptance를 최소 수준에서 점검할 만큼의 `close_task` state logic을 계획한다.
- prose만이 아니라 structured 차단 조건을 반환한다.
- 최소한 evidence-insufficient와 decision-required 닫기 차단 조건을 증명한다.
- `direct` 경로에 충분한 state가 있고 required 차단 조건이 남아 있지 않을 때만 clean self-checked direct close를 허용한다.

완료 기준:

- Required evidence가 없는 Task는 successful close 상태가 될 수 없다.
- 해소되지 않은 blocking Decision Packet이 있는 Task는 successful close 상태가 될 수 없다.
- Close result가 생성된 보고서가 아니라 기준 기록에 근거한다.

## 이것이 증명하는 것

첫 실행 가능한 조각은 다음을 증명합니다.

- Core가 상태 전이를 소유할 수 있다.
- State store와 `task_events`를 사용할 수 있다.
- 제품 파일 쓰기에는 scoped Change Unit이 필요하다.
- `prepare_write`가 쓰기 시도의 권한 결정이다.
- Write Authorization은 durable하고 single-use다.
- `record_run`이 쓰기 권한을 한 번 사용한 것으로 기록하고 observed work를 기록한다.
- Artifact와 근거가 chat에 의존하지 않고 등록될 수 있다.
- 상태 조회는 read-only다.
- Projection은 파생된 결과이며 failure-isolated하다.
- Required evidence 또는 decision이 없으면 닫기가 차단될 수 있다.

## 아직 증명하지 않는 것

이 조각은 다음을 아직 증명하지 않습니다.

- 전체 Decision Packet 품질
- 전체 approval lifecycle과 approval drift handling
- detached verification 독립성
- Manual QA policy 적용 범위
- acceptance와 close 전에 남은 위험을 표시하는 규칙
- feedback-loop와 TDD conformance
- codebase stewardship과 context-hygiene 적용 범위
- 전체 projection과 reconcile 동작
- projection template 완성도
- recover, export, artifact integrity, broad operator smoke
- dashboard, Context Index, connector marketplace, Browser QA Capture 동작
- preventive guard 동작
- parallel orchestration 또는 team workflow

이 내용은 [MVP 계획](mvp-plan.md)의 Agency-Hardened MVP 경로에 속합니다.

## 작성할 Fixture

Core 동작을 실행하고 state, events, artifacts, projections, errors를 검증하는 fixture를 작성합니다. Rendered prose matching만으로 success를 검증하지 않습니다.

첫 조각의 최소 fixture:

- no-active-task 상태 조회가 `idle` 상태를 반환하고 event를 추가하지 않음
- 프로젝트 bootstrap이 project state와 기준 agent 접점을 만듦
- intake 또는 seeded Task가 active Task 하나와 초기 gate를 만듦
- active Change Unit이 intended path 하나의 범위를 정함
- `prepare_write`가 active Change Unit 없을 때 차단함
- `prepare_write`가 out-of-scope 경로를 차단함
- `prepare_write`가 compatible scoped 쓰기를 허용하고 Write Authorization 하나를 만듦
- idempotent `prepare_write` replay가 committed authorization response를 반환함
- `record_run`이 쓰기 권한 없을 때 차단함
- `record_run`이 compatible Write Authorization을 한 번 사용한 것으로 기록하고 observed changes를 기록함
- 두 번째 distinct `record_run`이 consumed Write Authorization을 재사용할 수 없음
- artifact 등록이 hash, redaction state, owner relation을 저장함
- Evidence Manifest가 partial과 sufficient 근거 상태를 기록함
- 상태 조회가 상태 변경 없이 gate, evidence, 쓰기 권한, projection 최신성을 표시함
- Task 변경이 `TASK` projection을 대기열에 넣거나 렌더링함
- projection failure가 커밋된 상태를 롤백하지 않음
- `close_task`가 evidence-insufficient close를 차단함
- `close_task`가 해소되지 않은 decision close를 차단함

Fixture shape와 비교 규칙은 [운영과 Conformance 참조](../reference/operations-and-conformance.md#conformance-fixture-format)를 따릅니다.

## 참고할 Reference 문서

- [커널 참조](../reference/kernel.md): Task, Change Unit, Decision Packet, gate, `prepare_write`, Write Authorization, `record_run` semantics, `close_task`.
- [런타임 아키텍처 참조](../reference/runtime-architecture.md): 세 공간, Core process model, transaction flow, artifact store, projection/reconcile, guarantee level, failure handling.
- [MCP API와 스키마](../reference/mcp-api-and-schemas.md): public resource, tool envelope, request/response schema, error taxonomy, artifact ref, `ProjectionKind`.
- [Storage와 DDL](../reference/storage-and-ddl.md): runtime layout과 DDL, migration, lock, artifact, baseline, projection job, validator-run storage를 다룹니다.
- [운영과 Conformance 참조](../reference/operations-and-conformance.md): operator 의미, conformance 단계화, fixture 형식, 실행, assertion rule.
