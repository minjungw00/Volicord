# Build: MVP 계획

## 이 문서가 도와주는 일

이 문서는 MVP 범위를 구현 순서로 다시 정리합니다. 구현 단계는 저장소 스키마, DDL, projection 템플릿 본문, 운영자 명령 문법과 분리해서 설명합니다.

이 문서는 구현 계획 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 구현/증명 대상은 Kernel Smoke입니다. 즉 모듈을 가진 로컬 프로세스 하나로 권한 루프 하나를 증명합니다. Agency-Hardened MVP는 Kernel Smoke 이후의 later hardening과 conformance target이며, roadmap automation은 owner 문서가 승격하고 증명하기 전까지 MVP 밖에 둡니다.

첫 실행 가능한 조각 이후 무엇을 만들지 계획할 때 이 문서를 사용합니다. 정확한 규칙은 reference 문서를 봅니다.

## 이런 때 읽기

- Kernel Smoke 이후 구현을 계획할 때.
- MVP 범위를 stage별로 리뷰해야 할 때.
- 구현 순서를 저장소, 스키마, 템플릿 세부사항과 분리해서 보고 싶을 때.

## 읽기 전에

[구현 개요](implementation-overview.md)를 먼저 읽고 [문서 승인 상태](implementation-overview.md#문서-승인-상태)를 확인한 뒤 [첫 실행 가능한 조각](first-runnable-slice.md)을 봅니다. 정확한 API 규칙은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)를 보고, Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)을 봅니다. 설계 품질 gate와 validator 동작은 [설계 품질 정책](../reference/design-quality-policies.md)을 봅니다. Post-MVP 후보와 승격 기준은 [로드맵](../roadmap.md)을 봅니다.

## 핵심 생각

MVP 구현은 좁은 Kernel Smoke 경로에서 시작하고, 그 뒤 Agency-Hardened MVP를 향해 단단해집니다. Later automation은 future owner가 [로드맵 승격 규칙](../roadmap.md#승격-규칙)을 통해 승격하고 별도로 증명하기 전까지 경계 밖에 둡니다.

이 계획의 중심은 Core state, `task_events`, artifact refs, 근거, blocker, 그리고 그 권한 경로를 실행해 볼 최소 reference surface와 MCP reachability입니다. 초기 구현 가정은 모듈을 가진 로컬 프로세스 하나로 남습니다. Projection template 다듬기, dashboard 또는 hosted workflow UI, index, hook expansion, 넓은 connector ecosystem 또는 marketplace, team workflow, 접점별 connector automation, metrics, parallel orchestration, broad automation은 그 경로가 존재한 뒤에 유용해집니다. 첫 구축 대상이 아닙니다.

## MVP 범위, 쉬운 말로

MVP는 로컬 커널 권한과 agency conformance를 검증하는 프로젝트입니다. 광범위한 agent 플랫폼이 아닙니다.

MVP는 하나의 로컬 프로젝트와 하나의 기준 agent 접점이 Harness를 통해 다음을 할 수 있게 해야 합니다.

- 기준 Task 상태와 `task_events`
- 제품 파일 쓰기의 범위를 정하는 Change Unit
- `prepare_write`와 durable Write Authorization
- sensitive category를 위한 approval
- 사용자가 소유한 제품 판단 또는 중요한 기술 판단을 위한 Decision Packet
- Run, artifact ref, Evidence Manifest
- verification, Manual QA, 남은 위험 표시, acceptance, 닫기 차단 조건
- Core 위에서 동작하는 MCP resource와 tool
- projection job과 MVP 필수 projection 렌더러
- human-editable input 또는 managed-block drift를 위한 reconcile
- Core state, `task_events`, artifacts, projections, 기존 error 또는 diagnostics를 통해 보고하는 doctor/readiness, recover, export, artifact integrity, conformance smoke 진입점

이 범위는 로컬이고, 살펴볼 수 있으며, fixture로 증명 가능해야 합니다.

실제로는 표현을 다듬기보다 state와 권한 경로를 먼저 만듭니다. Projection이나 UI는 status를 읽기 쉽게 만들 수 있지만 반드시 Core record에서 파생되어야 합니다.

## Kernel Smoke

Kernel Smoke는 첫 실행 가능한 conformance 목표이자 첫 구현/증명 대상입니다. MVP-0부터 early MVP-3까지를 가로지르지만 선택한 권한 경로에만 집중합니다.

다음을 증명해야 합니다.

- 프로젝트와 Task 상태
- scoped Change Unit 하나
- `prepare_write` allow와 block 동작
- durable Write Authorization 생성
- `record_run`의 Write Authorization 사용 기록
- artifact 등록
- 기본 Evidence Manifest
- 최소 required projection 최신성 또는 대기열 추가
- 쓰기 권한이 없을 때 차단되는 쓰기 또는 Run
- 근거 또는 decision 요구사항이 없을 때 차단되는 close
- 기본 Core fixture 실행

Kernel Smoke는 나머지 시스템이 완성되기 전에 Harness 쓰기 권한 루프를 증명하기 때문에 유용합니다. 최종 MVP conformance는 아닙니다.

이 시점에서 사용자 또는 운영자는 작지만 완결된 루프를 볼 수 있어야 합니다. 현재 Task 상태, scoped write block/allow, durable Write Authorization 생성과 사용, artifact와 Evidence Manifest 연결, projection 최신성 또는 대기열 추가, structured close blocker가 그 관찰 대상입니다.

실제 fixture 작성 순서는 [Kernel Smoke Authoring Queue](../reference/operations-and-conformance.md#kernel-smoke-authoring-queue)를 사용합니다. 이 queue는 첫 runtime fixture candidate를 이 stage에 매핑하되 exact fixture body shape를 바꾸지 않습니다.

Kernel Smoke pass/fail은 runtime fixture가 Core 또는 operator action을 실행하고 captured state, `task_events`, artifacts, projections, primary errors를 비교해서 결정됩니다. Status prose, Journey Card text, close prose, scenario description은 관찰 가능한 context일 뿐입니다. Exact fixture body와 assertion rule은 [운영과 Conformance](../reference/operations-and-conformance.md#conformance-fixture-format)가 담당합니다.

## Agency-Hardened MVP

Agency-Hardened MVP는 later hardening이자 최종 reference conformance 목표이며, 첫 구현 batch가 아닙니다. 나머지 MVP-3을 완료한 뒤 MVP-4와 MVP-5를 추가합니다.

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
- recovery artifact가 successful completion을 증명하지 않는다는 규칙을 포함한 recover, export, artifact integrity 동작
- later 경계 확인
- 필수 agency conformance fixture

Agency-Hardened MVP를 통과하면 로컬 reference MVP가 구현에 사용할 만큼 충분히 일관적이라는 뜻입니다. Later automation을 MVP로 승격하지는 않습니다.

이 시점에서 사용자 또는 운영자는 쓰기가 통제된다는 사실뿐 아니라 work가 계속될 수 있는지, 멈춰야 하는지, verify, Manual QA 요구, Residual Risk 표시, accept, recover, export, close가 가능한 이유를 볼 수 있어야 합니다.

## MVP-0부터 MVP-5까지

아래 stage description은 문서 승인 이후를 계획하기 위해 구현 동사를 사용합니다. 현재 documentation-acceptance phase에서 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture, runtime data를 시작하라는 허가가 아닙니다.

### MVP-0: Runtime Bootstrap

로컬 runtime home을 만들고 프로젝트 하나를 등록하는 단계입니다.

중점:

- 프로젝트 등록
- project state 초기화
- 정적 프로젝트 설정
- artifact store 초기화
- honest cooperative 또는 detective 능력을 가진 기준 agent 접점 등록
- state를 만들지 않고 runtime home, project state, artifact store, reference surface, MCP availability 상태를 표시하는 doctor/readiness

여기서 multi-project orchestration, team workflow, hosted workflow UI, metrics, connector ecosystem은 추가하지 않습니다.

### MVP-1: Core State, Journey/Decision Skeleton, MCP Facade

Core 상태 전이 기반과 첫 MCP-facing read/tool을 계획합니다.

중점:

- transaction wrapper, lock, state version 확인, idempotency replay
- 현재 기록 갱신과 `task_events` 추가 동작
- active Task가 없는 상태
- advisor Task intake, read-only progress, close
- Journey Spine 재구성과 Journey Card input
- Decision Packet 기록과 `decision_gate` aggregation
- `harness.status`, `harness.intake`, `harness.next`
- 권한을 만들지 않는 읽기 전용 추천 playbook과 Role Lens 추천

표시 안내는 읽기 전용 routing과 status context로 남습니다. 정확한 Role Lens/playbook 경계는 [Agent Integration](../reference/agent-integration.md#role-lens-동작)에 있고, projection/report 경계는 [Document Projection Reference](../reference/document-projection.md#projection-principles)에 있습니다.

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

Approval을 사용자 소유 판단으로 취급하지 않습니다. 사용자 소유 제품 절충점, 아키텍처 선택, 중요한 기술 선택, 해결되지 않은 security 또는 product-security 판단, QA 면제, 검증 위험, acceptance, Residual Risk 수용에는 적용될 때 compatible Decision Packet이 여전히 필요합니다.

### MVP-3: Runs, Evidence, Feedback Loop, Projection, Reconcile

쓰기 이후의 기록과 읽을 수 있는 출력 경로를 계획합니다.

중점:

- `harness.record_run`
- Run 기록과 Write Authorization 사용 기록
- Evidence Manifest 기록과 evidence gate 갱신
- policy가 요구할 때 Feedback Loop와 TDD 뒷받침 기록
- codebase stewardship 확인
- verification 전에 원천 기록이 존재하는 MVP 필수 렌더러와 projection job
- managed block hash
- managed drift와 human-editable proposal을 위한 reconcile item 생성

이미 존재하는 record에서 MVP-required renderer를 만듭니다. Projection template, template polish, 추가 renderer-first 작업이 Task, Run, evidence, verification 설계를 이끌게 하지 않습니다.

이 단계에서는 원천 기록이 존재할 때 `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `DIRECT-RESULT`를 만들 수 있습니다. `EVAL`은 MVP-required로 남지만 Eval 원천 기록이 생기는 MVP-4에서 실행 가능한 렌더링 경로가 완료됩니다.

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
- evaluator bundle 최신성
- Manual QA aggregation과 QA 차단 조건
- acceptance와 close 전에 남은 위험을 표시하는 규칙
- acceptance와 위험 수용 후 닫기 규칙
- Approval, Manual QA, verification waiver, acceptance, residual-risk acceptance 판단의 분리
- Decision Packet 닫기 확인
- 닫기 차단 조건 표시

MVP에서 자동 Browser QA Capture나 hosted workflow automation을 요구하지 않습니다. screenshot, console log, network trace, accessibility snapshot, workflow recording은 기존 Manual QA/artifact path를 통해 등록되고 연결될 때만 QA evidence를 보강할 수 있지만, MVP 요구사항은 Manual QA 기록과 artifact ref입니다. 캡처 자료는 Manual QA judgment, final acceptance, detached verification을 대체하지 않습니다. Detached verification은 별도 Eval 경로가 independence를 충족해야 합니다. 지원하지 않는 접점은 사람이 작성한 Manual QA notes와 수동 제공 artifacts로 fallback합니다.

### MVP-5: Operator Smoke, Agency Conformance, Later-Boundary Checks

운영자와 conformance 증명 계층을 계획합니다.

중점:

- runtime home, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, agency/stewardship/context를 다루는 doctor/readiness category
- baseline drift, approval drift, evaluator repo drift, artifact missing 또는 hash mismatch, projection failure, managed Markdown direct edit, MCP unavailable, surface capability mismatch를 구분하는 recover handling
- reconcile
- state snapshot, report projection snapshot, artifact refs, redaction status, omitted-secret note, retained/expired/unavailable artifact status를 포함하는 export
- artifact 무결성 확인
- state, events, artifacts, projections, errors를 대상으로 하며 suite catalog metadata는 fixture body 밖에 두는 fixture 기반 conformance smoke
- core, connector, connector guard/freeze, agency, stewardship, context-hygiene, design-quality path의 coverage-map conformance
- Journey 표시, 사용자 판단, Autonomy Boundary 준수, 서로 다른 사용자 판단, 남은 위험 표시를 대상으로 하는 agency conformance
- MCP unavailable hold, surface capability mismatch, generated-file drift, stale projection write guard, stale PRD/chat-memory pull-only 동작, evaluator bundle 최신성, artifact integrity effect를 대상으로 하는 connector와 context conformance
- Dashboard, hosted workflow UI, Browser QA Capture, Cross-Surface Verification, Context Index, parallel orchestration, team workflow, broad connector automation, native hook 또는 sidecar expansion, derived metrics, preventive guard expansion을 별도 증명과 승격 없이는 MVP 밖에 두는 later 경계 확인

운영자 명령을 위해 두 번째 상태 모델을 만들지 않습니다. Operator는 같은 Core 상태 모델 위에서 진단, 복구, export, fixture 실행을 수행합니다. 정확한 command name과 flag는 달라질 수 있습니다. 계약은 Core state, `task_events`, artifacts, projections, 기존 error 또는 diagnostics 위의 command-independent behavior입니다.

Docs-maintenance는 별도의 읽기 전용 문서 profile로 남습니다. Documentation drift를 보고할 수 있지만 Kernel Smoke도, Agency-Hardened runtime conformance도, 구현 준비 상태 신호도 아닙니다.

## Stage별 완료 기준

다음은 문서 승인 이후의 future runtime planning을 위해 구현자가 읽기 쉬운 checklist입니다. Stage별 완료 기준을 다시 쓴 것이며, schema, fixture, DDL, runtime requirement를 새로 추가하지 않고, [문서 승인 상태](implementation-overview.md#문서-승인-상태)가 첫 runtime batch 계획을 막고 있는 동안 implementation을 승인하지 않습니다.

Stage exit는 두 층으로 읽습니다.

| Stage | Kernel Smoke로 읽는 범위 | Agency-Hardened MVP로 읽는 범위 |
|---|---|---|
| MVP-0 | 첫 local project, runtime home, artifact store, reference surface, idle readiness를 위한 필수 기반입니다. | 같은 기반이 유지되고, 이후 더 넓은 doctor/readiness category를 뒷받침합니다. |
| MVP-1 | 첫 권한 루프에 필요한 Task state, state-version, `task_events`, 최소 status/intake, decision blocker visibility만 필요합니다. | 최종 local MVP에 필요한 Journey/Decision skeleton과 read-only guidance boundary를 완성합니다. |
| MVP-2 | active Change Unit 하나, `prepare_write` allow/block, durable Write Authorization 생성, artifact registration basics가 필요합니다. | Hardened conformance에 필요한 더 넓은 approval, baseline, autonomy, sensitive-category, drift handling을 추가합니다. |
| MVP-3 | compatible `record_run` 하나, Write Authorization consumption, artifact-backed Evidence Manifest basics, minimal `TASK` projection freshness 또는 enqueue가 필요합니다. | Local reference MVP를 위한 feedback-loop, TDD, stewardship, projection, reconcile coverage를 완성합니다. |
| MVP-4 | Kernel Smoke 통과에는 필요하지 않습니다. MVP-4 동작이 없다는 것은 첫 조각이 아직 증명하지 않았다는 뜻일 뿐입니다. | Verification, Manual QA, residual-risk visibility, acceptance, close-readiness hardening에 필요합니다. |
| MVP-5 | Kernel Smoke 통과에는 필요하지 않습니다. MVP-5 동작이 없다는 것은 첫 조각이 아직 증명하지 않았다는 뜻일 뿐입니다. | Operator smoke, agency conformance, recover/export/artifact-integrity proof, later-boundary check에 필요합니다. |

Kernel Smoke는 위의 선택된 MVP-0부터 early MVP-3 subset만으로 통과할 수 있습니다. Agency-Hardened MVP는 남은 stage criteria와 [운영과 Conformance](../reference/operations-and-conformance.md#hardened-mvp-fixture-coverage)가 담당하는 fixture coverage를 요구합니다.

### MVP-0 완료 checklist

- 프로젝트 하나가 등록되어 있다.
- Expected state version을 사용하는 상태 변경 전에 project state가 존재한다.
- 기준 agent 접점이 등록되어 있다.
- Runtime 파일과 artifact 저장소가 있다.
- Doctor/readiness가 state를 만들지 않고 runtime home, project state, artifact store, reference surface, MCP availability 상태를 표시할 수 있다.

### MVP-1 완료 checklist

- No-active-Task 상태 표시가 동작한다.
- Advisor Task가 Core를 통해 intake와 close를 할 수 있다.
- Task 상태가 Journey/Decision 상태를 보여 준다.
- 읽기 안내는 권한을 만들지 않는다.
- 진행을 막는 사용자 판단은 Decision Packet을 만들거나 연결할 수 있다.
- 모든 상태 변경은 현재 기록과 `task_events`를 하나의 transaction에서 갱신한다.

### MVP-2 완료 checklist

- Active scoped Change Unit이 없는 제품 파일 쓰기는 차단된다.
- Sensitive change는 approval을 요구한다.
- Autonomy Boundary violation은 차단되거나 Decision Packet으로 라우팅된다.
- 해소되지 않은 blocking Decision Packet은 영향받는 쓰기를 차단한다.
- 허용된 `prepare_write`는 durable Write Authorization ref를 만든다.
- Idempotent replay가 동작한다.
- Approval drift는 approval을 차단하거나 만료시킬 수 있다.
- Shaping은 필요한 경계를 기록한다.
- Raw artifact는 integrity/redaction metadata를 저장한다.

### MVP-3 완료 checklist

- `direct` 및 구현 Run은 artifact를 등록하고 근거를 갱신한다.
- Run은 compatible Write Authorization을 한 번 사용한 것으로 기록한다.
- Authorization 밖의 observed change는 감지된다.
- 발견된 사항은 상태, 근거, Decision Packet, Change Unit, 차단 조건 중 적절한 경로로 연결된다.
- Stewardship issue가 보인다.
- 검증 전 MVP 필수 projection은 대기열에 넣거나 렌더링할 수 있다.
- Projection failure는 상태와 분리되어 처리된다.
- Managed Markdown edit는 reconcile item을 만든다.

### MVP-4 완료 checklist

- Work는 같은 세션의 self-review만으로 detached verified 상태로 닫힐 수 없다.
- 최신이 아닌 evaluator bundle은 detached verification을 passed로 기록할 수 없다.
- Verification waiver는 accepted risk로만 닫을 수 있다.
- Required Manual QA와 acceptance는 독립적으로 차단한다.
- Close-relevant residual risk는 successful close 전에 보인다.
- Risk-accepted close에는 accepted Residual Risk refs가 필요하다.
- Acceptance는 risk visibility 뒤에 온다.
- Approval, Manual QA, verification waiver, acceptance, residual-risk acceptance는 서로 분리되어 남는다.
- Blocking Decision Packet은 close를 차단한다.
- Policy 또는 사용자가 detached verification을 요구하지 않는 한 direct work는 self-checked로 닫힐 수 있다.

### MVP-5 완료 checklist

- Conformance smoke는 core, connector, connector guard/freeze, agency, stewardship, context-hygiene, 설계 품질 경로를 포괄한다.
- Catalog scenario coverage는 artifact integrity, MCP unavailable, surface capability mismatch, generated-file drift, stale projection write guard, stale PRD/chat-memory context, evaluator bundle freshness, residual-risk visibility, distinct user judgments를 포함한다.
- Suite catalog metadata는 exact-shape fixture를 suite, stage, tag로 group하되 Core에 전달되지 않는다.
- Agency 점검은 Journey 표시, 해소되지 않은 결정, agent latitude, 서로 다른 사용자 판단, 남은 위험 표시를 증명한다.
- Dependency DAG 지원은 metadata만 남는다.
- Export는 state snapshots, report projection snapshots, artifact refs, redaction status, omitted-secret note, retained/expired/unavailable artifact status를 포함한다.
- Browser QA Capture 항목은 owner 문서를 통해 승격되기 전까지 future candidate로 남는다.

## Stage별 관찰 가능성

| Stage | 사용자 또는 운영자가 관찰할 수 있는 것 |
|---|---|
| MVP-0 | Doctor/readiness가 runtime home, project state, artifact store, reference surface, MCP availability, idle state를 보여 줄 수 있습니다. |
| MVP-1 | Status, intake, next-action read가 state를 변경하지 않고 active Task, Journey/Decision state, 권한 없는 guidance를 보여 줄 수 있습니다. |
| MVP-2 | `prepare_write`가 missing scope, out-of-scope path, sensitive approval 필요성, stale baseline, unresolved decision, compatible Write Authorization 생성을 설명할 수 있습니다. |
| MVP-3 | `record_run`이 권한을 사용하고, Run이 artifact를 참조하며, 근거가 갱신되고, projection 최신성이 보이며, managed drift에는 reconcile item이 생깁니다. |
| MVP-4 | Verification, Manual QA, Residual Risk 표시, acceptance, `close_task` blocker가 Task를 닫을 수 있는지 설명합니다. |
| MVP-5 | Doctor, recover, reconcile, export, artifact integrity, conformance fixture가 같은 Core state를 증명하고 later automation을 MVP 경계 밖에 둡니다. |

## Later 경계

다음은 향후 계획이 owner 문서, capability profile, 정확한 계약, redaction/secret/PII policy, runtime 접점을 capture할 때의 artifact retention과 test-environment rule, fixture 또는 conformance target, fallback 동작, projection-as-canonical 의존성 없음으로 승격하기 전까지 MVP 밖에 둡니다.

- 권한, implementation-readiness, close-readiness 기준으로 쓰이는 dashboard, hosted workflow UI, local metric
- 기준 접점 하나를 넘어서는 broad connector marketplace 또는 접점 ecosystem
- 필수 자동화 또는 acceptance 대체물로서의 Browser QA Capture
- 필수 assurance 경로로서의 Cross-Surface Verification
- 증명된 pre-tool blocking 경로가 없는 preventive `T4` guard expansion
- 기준 agent 접점의 구체적인 capability를 넘어서는 native hook expansion 또는 Advanced Sidecar Watcher
- 권한 또는 읽기/쓰기 선행 조건으로 쓰이는 Context Index
- deployment, canary, rollback, production monitoring automation
- parallel orchestration과 concurrent lane scheduling
- team workflow, permissions, team profile export/import
- MVP-critical 상태로서의 Local Derived Metrics 또는 long-term operational metrics

구현 중 later feature가 유용해 보이더라도 owner 문서가 권한 경로를 정의하고 증명하기 전까지는 읽기 전용 표시, metadata, 기존 owner path를 위한 artifact 후보, fixture candidate로 유지합니다. Kernel Smoke 또는 Agency-Hardened MVP의 전제 조건이 되어서는 안 됩니다.
