# Build: MVP 계획

## 이 문서가 도와주는 일

이 문서는 MVP scope 내용을 구현 순서로 다시 정리합니다. 구현 단계는 저장소 스키마, DDL, projection 템플릿 본문, 운영자 명령 문법과 분리해서 설명합니다.

이 문서는 구현 계획 문서입니다. 재설계 문서가 승인되기 전에는 runtime/server 구현을 시작하라는 뜻이 아닙니다.

첫 실행 가능한 조각 이후 무엇을 만들지 계획할 때 이 문서를 사용합니다. 정확한 규칙은 reference 문서를 봅니다.

## 이런 때 읽기

- Kernel Smoke 이후 구현을 계획할 때.
- MVP 범위를 stage별로 리뷰해야 할 때.
- 구현 순서를 저장소, 스키마, 템플릿 세부사항과 분리해서 보고 싶을 때.

## 읽기 전에

[구현 개요](implementation-overview.md)와 [첫 실행 가능한 조각](first-runnable-slice.md)을 먼저 읽습니다. 정확한 API 규칙은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)를 보고, Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)을 봅니다. 설계 품질 gate와 validator 동작은 [설계 품질 정책](../reference/design-quality-policies.md)을 봅니다.

## 핵심 생각

MVP 구현은 좁은 Kernel Smoke 경로에서 시작해 Agency-Hardened MVP로 확장됩니다. Later automation은 별도 명세와 증명이 생기기 전까지 경계 밖에 둡니다.

## MVP 범위, 쉬운 말로

MVP는 로컬 커널 권한과 agency conformance를 검증하는 프로젝트입니다. 광범위한 agent 플랫폼이 아닙니다.

MVP는 하나의 로컬 프로젝트와 하나의 기준 agent 접점이 Harness를 통해 다음을 할 수 있게 해야 합니다.

- 기준 Task 상태와 task_events
- 제품 파일 쓰기의 범위를 정하는 Change Unit
- `prepare_write`와 durable Write Authorization
- sensitive category를 위한 approval
- 사용자가 소유한 제품 판단을 위한 Decision Packet
- Run, artifact ref, Evidence Manifest
- verification, Manual QA, 남은 위험 표시, acceptance, 닫기 차단 조건
- Core 위에서 동작하는 MCP resource와 tool
- projection job과 MVP 필수 projection renderer
- human-editable input 또는 managed-block drift를 위한 reconcile
- doctor, recover, export, artifact integrity, conformance smoke 진입점

이 범위는 로컬이고, 살펴볼 수 있으며, fixture로 증명 가능해야 합니다.

## Kernel Smoke

Kernel Smoke는 첫 실행 가능한 conformance 목표입니다. MVP-0부터 early MVP-3까지를 가로지르지만 선택한 권한 경로에만 집중합니다.

다음을 증명해야 합니다.

- 프로젝트와 Task 상태
- scoped Change Unit 하나
- `prepare_write` allow와 block 동작
- durable Write Authorization 생성
- `record_run`의 Write Authorization 사용 기록
- artifact 등록
- 기본 Evidence Manifest
- 최소 required projection 최신성 또는 enqueueing
- 쓰기 권한이 없을 때 차단되는 쓰기 또는 Run
- 근거 또는 decision 요구사항이 없을 때 차단되는 close
- 기본 Core fixture 실행

Kernel Smoke는 나머지 시스템이 완성되기 전에 Harness 쓰기 권한 루프를 증명하기 때문에 유용합니다. 최종 MVP conformance는 아닙니다.

## Agency-Hardened MVP

Agency-Hardened MVP는 최종 reference conformance 목표입니다. 나머지 MVP-3을 완료한 뒤 MVP-4와 MVP-5를 추가합니다.

다음을 증명해야 합니다.

- Decision Packet 품질과 사용자 판단 라우팅
- approval, Decision Packet, Write Authorization의 분리
- acceptance와 close 전에 남은 위험을 표시하는 규칙
- detached verification 독립성
- Manual QA 기록과 차단 조건
- stewardship 및 context-hygiene validators
- feedback-loop와 TDD 확인
- codebase stewardship 적용 범위
- projection과 reconcile 완전성
- recover, export, artifact integrity 동작
- later 경계 확인
- 필수 agency conformance fixture

Agency-Hardened MVP를 통과하면 로컬 reference MVP가 구현에 사용할 만큼 충분히 일관적이라는 뜻입니다. Later automation을 MVP로 승격하지는 않습니다.

## MVP-0부터 MVP-5까지

### MVP-0: Runtime Bootstrap

로컬 runtime home을 만들고 프로젝트 하나를 등록하는 단계입니다.

중점:

- 프로젝트 등록
- project state 초기화
- 정적 프로젝트 설정
- artifact store 초기화
- honest cooperative 또는 detective 능력을 가진 기준 agent 접점 등록
- doctor/readiness 상태 표시

여기서 multi-project orchestration이나 connector ecosystem은 추가하지 않습니다.

### MVP-1: Core State, Journey/Decision Skeleton, MCP Facade

Core 상태 전이 기반과 첫 MCP-facing read/tool을 계획합니다.

중점:

- transaction wrapper, lock, state version 확인, idempotency replay
- 현재 기록 갱신과 task_events append 동작
- active Task가 없는 상태
- advisor Task intake, read-only progress, close
- Journey Spine 재구성과 Journey Card input
- Decision Packet 기록과 `decision_gate` aggregation
- `harness.status`, `harness.intake`, `harness.next`
- 권한을 만들지 않는 읽기 전용 추천 playbook과 Role Lens 추천

표시 안내가 gate를 충족시키거나, 쓰기를 허가하거나, 근거를 만들거나, QA 또는 verification을 waive하거나, 위험을 수용하거나, 결과를 accept하거나, Task를 close하거나, assurance를 upgrade하게 만들면 안 됩니다.

### MVP-2: Shaping Kernel, Write Gate, Approval, Baseline, Artifacts

첫 쓰기 가능 권한 경로를 계획합니다.

중점:

- Change Unit 기록과 active scope
- Autonomy Boundary 필드
- baseline capture와 최신성 확인
- `harness.prepare_write`
- durable Write Authorization 기록
- sensitive category를 위한 approval 요청과 결정 흐름
- 최소 changed-path, scope, approval, baseline, decision, autonomy, 능력 확인
- integrity와 redaction metadata를 가진 raw artifact 등록

Approval을 제품 판단으로 취급하지 않습니다. 제품 절충점, 아키텍처 선택, QA waiver, 검증 위험, acceptance, residual-risk acceptance에는 적용될 때 compatible Decision Packet이 여전히 필요합니다.

### MVP-3: Runs, Evidence, Feedback Loop, Projection, Reconcile

쓰기 이후의 기록과 읽을 수 있는 output 경로를 계획합니다.

중점:

- `harness.record_run`
- Run 기록과 Write Authorization 사용 기록
- Evidence Manifest 기록과 evidence gate 갱신
- policy가 요구할 때 Feedback Loop와 TDD support 기록
- codebase stewardship 확인
- verification 전에 원천 기록이 존재하는 MVP 필수 renderer와 projection job
- managed block hash
- managed drift와 human-editable proposal을 위한 reconcile item 생성

이 단계에서는 원천 기록이 존재할 때 `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `DIRECT-RESULT`를 만들 수 있습니다. `EVAL`은 MVP-required로 남지만 Eval 원천 기록이 생기는 MVP-4에서 실행 가능한 render 경로가 완료됩니다.

Projection failure는 Core 상태 failure와 분리됩니다.

### MVP-4: Verification, Manual QA, Residual Risk, Close

닫기 준비 상태와 assurance 경로를 계획합니다.

중점:

- `harness.launch_verify`
- `harness.record_eval`
- `harness.record_manual_qa`
- `harness.close_task`
- 검증 독립성 확인
- 같은 세션 검증 방지 규칙
- Manual QA aggregation과 QA 차단 조건
- acceptance와 close 전에 남은 위험을 표시하는 규칙
- acceptance와 위험 수용 후 닫기 규칙
- Decision Packet 닫기 확인
- 닫기 차단 조건 표시

MVP에서 자동 Browser QA Capture를 요구하지 않습니다. screenshot, console log, network trace, accessibility snapshot, workflow recording은 제공될 때 연결할 수 있지만, MVP 요구사항은 Manual QA 기록과 artifact ref입니다.

### MVP-5: Operator Smoke, Agency Conformance, Later-Boundary Checks

운영자와 conformance 증명 계층을 계획합니다.

중점:

- doctor
- recover
- reconcile
- export
- artifact 무결성 확인
- fixture 기반 conformance smoke
- Journey 표시, 사용자 판단, Autonomy Boundary 준수, 남은 위험 표시를 대상으로 하는 agency conformance
- parallel orchestration, broad connector automation, preventive guard expansion을 별도 증명 없이는 MVP 밖에 두는 later 경계 확인

운영자 명령을 위해 두 번째 상태 모델을 만들지 않습니다. Operator는 같은 Core 상태 모델 위에서 진단, 복구, export, fixture 실행을 수행합니다.

## Stage별 완료 기준

| Stage | 완료 기준 |
|---|---|
| MVP-0 | 프로젝트 하나가 등록되고, expected state version을 사용하는 상태 변경 전에 project state가 존재해야 한다. 기준 agent 접점이 등록되어 있고, runtime 파일과 artifact 저장소가 있으며, doctor/readiness가 프로젝트와 runtime 상태를 표시할 수 있어야 한다. |
| MVP-1 | No-active-Task 상태 표시가 동작해야 한다. advisor Task는 Core를 통해 intake와 close를 할 수 있어야 하며, Task 상태는 Journey/Decision state를 보여 줘야 한다. 읽기 안내는 권한을 만들지 않아야 하고, 진행을 막는 사용자 판단은 Decision Packet을 만들거나 연결할 수 있어야 한다. 모든 상태 변경은 현재 기록과 task_events를 하나의 transaction에서 갱신해야 한다. |
| MVP-2 | Active scoped Change Unit이 없는 제품 파일 쓰기는 차단되어야 한다. Sensitive change는 approval을 요구해야 하며, Autonomy Boundary violation은 차단되거나 Decision Packet으로 라우팅되어야 한다. Unresolved blocking Decision Packet은 영향받는 쓰기를 차단해야 한다. 허용된 `prepare_write`는 durable Write Authorization ref를 만들고, idempotent replay가 동작해야 한다. Approval drift는 approval을 차단하거나 expire할 수 있어야 하며, shaping은 필요한 boundary를 기록해야 한다. Raw artifact는 integrity/redaction metadata를 저장해야 한다. |
| MVP-3 | `direct` 및 구현 Run은 artifact를 등록하고 근거를 갱신해야 한다. Run은 compatible Write Authorization을 한 번 사용한 것으로 기록해야 하며, authorization 밖의 observed change는 감지되어야 한다. 발견된 사항은 상태, 근거, Decision Packet, Change Unit, 차단 조건 중 적절한 경로로 연결되어야 한다. Stewardship issue가 보여야 하고, 검증 전 MVP 필수 projection은 enqueue 또는 render될 수 있어야 한다. Projection failure는 상태와 분리되어 처리되어야 하며, managed Markdown edit는 reconcile item을 만들어야 한다. |
| MVP-4 | work는 같은 세션의 self-review만으로 detached verified 상태로 닫힐 수 없어야 한다. Verification waiver는 accepted risk로만 close되어야 하며, required Manual QA와 acceptance는 독립적으로 차단해야 한다. Close-relevant residual risk는 successful close 전에 보여야 한다. Risk-accepted close에는 accepted Residual Risk refs가 필요하고, acceptance는 risk visibility 뒤에 와야 한다. Blocking Decision Packet은 close를 차단해야 하며, policy 또는 사용자가 detached verification을 요구하지 않는 한 direct work는 self-checked로 close될 수 있어야 한다. |
| MVP-5 | Conformance smoke는 core, connector, agency, stewardship, context-hygiene, design-quality 경로를 포괄해야 한다. Agency 점검은 Journey 표시, unresolved decisions, agent latitude, 남은 위험 표시를 증명해야 한다. Dependency DAG support는 metadata만 남아야 하며, export는 state snapshots, report projections, artifact refs, redaction status를 포함해야 한다. |

## Later boundary

다음은 향후 계획이 정확한 규칙과 fixture로 승격하기 전까지 MVP 밖에 둡니다.

- dashboard 또는 hosted workflow UI
- broad connector marketplace 또는 접점 ecosystem
- 필수 자동화로서의 Browser QA Capture
- 증명된 pre-tool blocking 경로가 없는 preventive `T4` guard expansion
- Context Index와 derived analytics
- deployment, canary, rollback, production monitoring automation
- parallel orchestration과 concurrent lane scheduling
- team workflow, permissions, team profile export/import
- MVP-critical 상태로서의 long-term operational metrics

구현 중 later feature가 유용해 보이더라도 owner docs가 권한 경로를 정의하기 전까지는 표시, metadata, 선택적 첨부, fixture candidate로 유지합니다.
