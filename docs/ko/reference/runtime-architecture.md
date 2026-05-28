# 런타임 아키텍처 참조

## 이 문서가 도와주는 일

이 문서는 Harness가 어디에서 실행되는지, 기준 상태가 어디에 있는지, Core가 상태 전이를 어떻게 기록하는지, artifact와 projection이 어떻게 연결되고 갱신되는지, runtime이 어떤 집행 강도를 정직하게 말할 수 있는지 확인하기 위한 참조 문서입니다.

구현자와 운영자가 찾아보는 참조 문서이며, Learn overview 전체를 다시 설명하지 않습니다.

이 문서는 참조 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 제품 MVP 목표는 v0.1 Kernel MVP이며, Kernel Smoke가 좁은 conformance profile로 이를 실행합니다. v0.2부터 v0.4까지는 Agency-Hardened MVP reference conformance target을 향한 staged pack이고, v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 roadmap 범위에 둡니다.

## 이런 때 읽기

- 제품 저장소 파일과 Harness runtime의 상태 관계를 매핑할 때.
- Core, artifact 수집, projection, reconcile, 검증, 복구, export가 어떻게 동작하는지 구현할 때.
- 실패가 기준 상태, artifact, projection, 표시 영역 중 어디에 영향을 주는지 판단해야 할 때.
- 연결된 접점이 cooperative, detective, preventive, isolated 중 어디에 해당하는지 설명해야 할 때.

## 읽기 전에

정확한 상태 전이는 [커널 참조](kernel.md)를, public tool envelope와 replay 동작은 [MCP API와 스키마](mcp-api-and-schemas.md)를, storage layout과 lock은 [Storage와 DDL](storage-and-ddl.md)을, security asset, trust boundary, threat, control은 [보안 위협 모델 참조](security-threat-model.md)를, operator entrypoint 의미는 [운영과 Conformance 참조](operations-and-conformance.md)를 사용합니다.

## 핵심 생각

Harness는 사용자의 Product Repository 옆에서 실행되는 로컬 권한 계층입니다. Product Repository는 실제 제품 작업이 일어나는 곳이고, Runtime Home은 운영 권한을 저장하며, Harness Server / Installation은 Core, validators, projection, reconcile, 공개 MCP tool을 통해 둘을 연결합니다.

중요한 규칙은 분리입니다. 기준 운영 상태를 변경하는 것은 Core뿐입니다. 제품 소스 파일, 대화 텍스트, 생성된 Markdown, connector 파일, operator output, MCP caller claim은 system에 정보를 줄 수 있지만 기준 운영 상태는 `state.sqlite` 현재 기록과 `state.sqlite.task_events`에 있고, 원본 근거는 artifact store에 있습니다.

## 담당하는 참조 범위

이 문서가 담당합니다.

- 구현 세부 관점의 세 공간
- Product Repository / Harness Server 또는 Installation / Harness Runtime Home 분리
- Core process model
- Core-only canonical mutation authority
- state transaction flow
- artifact store architecture
- security boundary의 architecture placement
- projection과 reconcile architecture
- 보장 수준
- failure와 recovery overview

## 여기서 다루지 않는 것

이 문서는 다음 항목을 담당하지 않습니다.

- public MCP request/response schema. [MCP API와 스키마](mcp-api-and-schemas.md)를 봅니다.
- SQLite DDL. [Storage와 DDL](storage-and-ddl.md)을 봅니다.
- full CLI command 의미. 현재 담당 문서는 [운영과 Conformance](operations-and-conformance.md)입니다.
- conformance fixture 형식. 현재 담당 문서는 [Conformance Fixtures 참조](conformance-fixtures.md)입니다.
- threat-model asset, trust boundary, threat category, control category. [보안 위협 모델 참조](security-threat-model.md)를 봅니다.
- 접점별 connector cookbook. [Surface Cookbook](surface-cookbook.md)을 봅니다.
- connector capability profile. [Agent 통합 참조](agent-integration.md)를 봅니다.
- kernel transition table. 자세한 내용은 [커널 참조](kernel.md)를 봅니다.
- projection template body

## 세 공간, 짧은 요약

```text
Product Repository:
  product code, tests, human-readable projections, and human-editable proposal areas

Harness Server / Installation:
  MCP server, Core, validators, connectors, projector, reconcile worker, and operator tools

Harness Runtime Home:
  registry.sqlite, project.yaml, state.sqlite, and the artifact store
```

```mermaid
flowchart LR
  Repo["Product Repository<br/>product code, tests, projections, proposal areas"]
  Server["Harness Server / Installation<br/>MCP server, Core, validators, connectors, projector, reconcile worker"]
  Home["Harness Runtime Home<br/>registry.sqlite, project.yaml, state.sqlite, artifact store"]

  Repo -->|user intent, repo facts, human edits| Server
  Server -->|managed projections and reconcile candidates| Repo
  Server -->|Core 상태 전이와 artifact 등록| Home
  Home -->|현재 기록, events, 원본 근거 refs| Server
```

이 분리는 대화, Markdown 보고서, 생성된 connector 파일, operator output, MCP caller claim, 제품 소스 파일이 우연히 운영 상태가 되는 일을 막습니다. Core 상태 변경 경로만 기준 운영 상태를 commit할 수 있습니다.

## 로컬 위협 모델

Harness는 로컬 권한 계층으로 설계되며, 일반적인 operating-system security boundary를 대신하지 않습니다. 전체 asset map, trust-boundary map, threat category, control category는 [보안 위협 모델 참조](security-threat-model.md)가 담당합니다.

Architecture implication은 단순합니다. 가까이 있는 file과 caller도 별도의 trust zone입니다. Product file, chat text, generated connector file, operator output, projection Markdown, artifact bytes, external command output, MCP caller claim은 Harness에 정보를 줄 수 있지만, canonical operational state를 commit하는 것은 Core뿐입니다.

Architecture는 다음 security boundary를 보이게 유지합니다.

| Boundary | Architecture handling |
|---|---|
| Product Repository와 projections | Input과 readable view입니다. Operational meaning은 Core 또는 reconcile을 통해 흐릅니다. |
| MCP server와 connected surfaces | Public tool은 Core를 통해 들어오며, capability는 실제 profile에 맞게 표시합니다. |
| Runtime Home | `state.sqlite`, `state.sqlite.task_events`, registry/config file, artifact는 local control data로 취급합니다. Direct file edit는 authority가 아닙니다. |
| Artifact store | Evidence bytes는 artifact registration, integrity, redaction/omission, owner-record check가 성공하기 전까지 untrusted입니다. |
| External tools와 network | Side effect가 있는 command는 기존 scope, Approval, write-authority, connector, operator control 안에 머물러야 합니다. |

Local-only MCP exposure, secret/PII handling, high-risk work용 command/path/network allowlist, artifact path validation, stale approval replay, projection tampering, capability overclaiming, stale context poisoning은 threat-model concept입니다. Exact API, storage, kernel, connector, operations contract는 threat model에서 연결한 owner 문서에 남습니다.

### 로컬 접근 기대사항

Architecture 수준에서 v0.1 baseline과 staged-delivery default의 MCP posture는 registered project surface에 대한 local-only입니다. Local-only는 expected local user/profile에 대해 runtime이 local process, local socket, localhost-loopback, in-process/stdio, process-scoped configuration material, per-project token 또는 handle, 이에 준하는 local IPC/control path를 사용한다는 뜻입니다.

Remote, shared, tunneled, forwarded, non-loopback, cross-user, cloud/CI relay 노출은 owner docs가 connector posture를 승격하고 증명하기 전까지 v0.1 baseline과 staged delivery 밖에 남습니다. 전체 asset, trust-boundary, threat, control model은 [보안 위협 모델 참조](security-threat-model.md#mcp-local-access와-caller-boundary)가 담당하고, connector profile reporting은 [Agent 통합 참조](agent-integration.md#capability-profiles), API validation은 [MCP API와 스키마](mcp-api-and-schemas.md#mcp-경계와-호출자-신뢰), operator diagnostic은 [운영과 Conformance 참조](operations-and-conformance.md#serve-mcp)가 담당합니다.

MCP reachability는 authorization이 아닙니다. Public tool call은 계속 Core envelope validation, state-version check, idempotency, registered project/task/surface compatibility, 실제 connected surface guarantee level에 의존합니다.

## Product Repository

Product Repository는 사용자의 실제 제품 작업 공간입니다. 제품 소스 코드, tests, repository-level agent rules, 사람이 읽는 Harness projection이 여기에 있습니다.

대표적인 repository-owned paths는 다음과 같습니다.

```text
repo/
  AGENTS.md
  docs/
    tasks/
    approvals/
    reports/
    design/
  .harness/
    agent/generated/
    reconcile/pending/
```


Repository는 생성된 `TASK`, `APR`, `RUN-SUMMARY`, `EVAL`, `DIRECT-RESULT`, `EVIDENCE-MANIFEST`, `TDD-TRACE`, `MANUAL-QA`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, 그 밖의 report projection Markdown 보고서를 담을 수 있습니다. 이 파일들은 사람과 agent가 작업을 읽는 데 도움을 주지만 기준 상태가 아닙니다. 사람이 편집할 수 있는 영역은 입력 접점입니다. 사람이 남긴 edit은 reconcile이 Core 상태 변경 action으로 라우팅할 때만 상태 기록이 됩니다.

## Harness Server / Installation

Harness Server / Installation은 제어 계층입니다. v0.1 Kernel MVP는 여러 service의 fleet 대신 내부 모듈을 가진 하나의 로컬 프로세스로 구현할 수 있습니다.

Core runtime의 책임:

- MCP server를 통해 읽기 resource와 public tool을 제공합니다.
- Core에서 커널 상태 전이를 실행합니다.
- write 전, Run 기록 후, close 전에 validator를 실행합니다.
- artifact와 무결성 metadata를 기록합니다.
- projection job을 대기열에 넣고 렌더링합니다.
- 사람의 편집이나 managed-block drift에서 reconcile candidate를 감지합니다.
- 진단, 복구, export, conformance 진입점을 제공합니다.

MCP server는 shell command를 감싼 얇은 wrapper가 아닙니다. MCP server는 높은 수준의 의도 호출을 제공하고, Core는 이를 상태 전이, validator, artifact 기록, projection job으로 변환합니다.

## Harness Runtime Home

Harness Runtime Home은 로컬 운영 권한을 저장합니다. Reference location은 `~/.harness`이지만 정확한 layout은 [Storage와 DDL](storage-and-ddl.md)이 담당합니다.

Runtime Home에는 다음이 있습니다.

- project registration, 연결된 접점, connector manifest를 위한 `registry.sqlite`
- 정적 프로젝트 설정을 위한 registered project별 `project.yaml`
- 현재 운영 기록과 `state.sqlite.task_events`를 위한 project별 `state.sqlite`
- 지속 보관되는 근거 파일을 위한 artifact directories


Runtime Home은 대화 기록이 사라지거나 Product Repository projection이 최신이 아니어도 운영 상태를 복구할 수 있을 만큼 충분해야 합니다. Product Repository 문서는 상태 기록과 artifact refs에서 다시 생성될 수 있으며, 그 기록을 대체하지 않습니다.

Runtime Home file은 user-private local control data로 취급해야 합니다. 관련 없는 user나 process가 secret/PII를 읽거나 `state.sqlite`, `registry.sqlite`, `project.yaml`, connector config snippet, connector manifest, generated manifest, artifact file, staging file, generated operational file을 수정할 수 있게 하는 file permission 또는 storage location은 local tampering 또는 기밀성 위험입니다. Harness는 operating-system permission을 스스로 enforce한다고 주장하지 않습니다. 이러한 file은 Core, `doctor`, `recover`, artifact-integrity validation path를 통해서만 authoritative하게 취급합니다.

## Core process model

### Runtime layers

```text
사용자 대화 접점
  ↓
Agent 접점
  ↓
Harness 규칙 / skill / local instructions
  ↓
Harness MCP server
  ↓
Harness Core
  ↓
state.sqlite / artifact store / validators / projector / reconcile worker
```


대화 접점은 사용자 의도, decision, approval, QA 판단, acceptance를 모읍니다. Agent 접점은 읽기, 편집, 확인을 수행합니다. Harness rules와 skills는 agent가 현재 상태를 놓치지 않게 합니다. MCP server는 tool 경계를 제공합니다. Core는 상태 모델을 담당합니다. Validator, artifact 수집, projection, reconcile은 근거와 읽기용 출력을 상태 전이에 붙입니다.

Native hooks, sidecars, command wrappers, file watchers, worktree isolation은 capability에 따라 달라지는 집행 계층입니다. 구체적인 capability profile이 fixture로 더 강한 enforcement를 증명하지 않는 한 v0.1 Kernel MVP는 reference 접점에서 cooperative/detective behavior에 의존합니다.


### Core modules

v0.1 Kernel MVP Core는 다음 내부 모듈을 가진 단일 프로세스로 실행할 수 있습니다.

| Module | Runtime responsibility |
|---|---|
| State store | 현재 기록, state version, locks, `state.sqlite.task_events` |
| Task workflow | intake, mode selection, next action, gate 갱신, 닫기 판단 |
| Journey module | Journey Spine reconstruction, Journey Spine Entry support records, Journey Card inputs, continuity refs |
| Decision module | Decision Packet lifecycle, `decision_gate` aggregation, 사용자 판단 연결, residual-risk visibility inputs |
| Approval module | scope-bound Approval 요청, decision, expiry, drift handling |
| Evidence module | run records, artifact refs, evidence manifests, coverage checks |
| Verification module | verification bundles, evaluator runs, Eval records, independence checks |
| Manual QA module | QA records, `qa_gate` aggregation |
| Projection module | projection jobs, managed blocks, freshness, 보고서 paths |
| Reconcile module | human-editable proposals, managed drift, accepted-state routing |
| Validator runner | core, decision, autonomy/boundary, design-quality, artifact, projection, connector checks |
| Autonomy/Boundary validator responsibility | Autonomy Boundary compatibility, agent latitude, user-judgment 요구사항, AFK stop conditions, boundary drift findings |
| Connector adapter | 기준 접점 등록, capability 보고, capture hints |


Core만 기준 운영 상태를 업데이트합니다. Agents, MCP tools, CLI commands, projectors, reconnect/recovery flows는 Core 로직을 거치거나 같은 상태 compatibility rules를 보존하는 recovery code를 사용해야 합니다. 이들은 Core record를 표시, 진단, 복구, 파생할 수 있지만 두 번째 기준 상태 모델을 유지하면 안 됩니다. Operator command name과 flag는 표시/entrypoint 선택입니다. 동작은 Core state record, `state.sqlite.task_events`, artifacts, projection jobs, API-owned errors 또는 문서화된 diagnostics가 정의합니다.

Decision, Journey, Autonomy/Boundary modules는 새로운 권한 tier를 만들지 않습니다. 기준 기록은 `state.sqlite` 현재 기록과 `state.sqlite.task_events`에 있고, 원본 근거는 artifact store에 있으며, Markdown views는 projections 또는 proposal 접점으로 남습니다.


### Validators and adapter placement

Validator는 Core 옆에 위치하고 구조화된 result를 Core에 반환합니다. Core는 그 result가 transition을 차단할지, gate를 `stale`/`partial`/`blocked`로 표시할지, 사용자 판단을 요청할지, 표시에만 영향을 줄지 결정합니다.

Stable MVP ValidatorResult ID set은 API가 소유하며 [MCP API와 스키마](mcp-api-and-schemas.md#validatorresult)에 나열됩니다. 이 runtime reference는 해당 validator가 Core와 adapter 옆에 어디에 놓이는지 담당하며, 두 번째 ID registry를 만들지 않습니다.

`feedback_loop_check`는 Feedback Loop support records와 related execution evidence를 읽습니다. 별도의 kernel gate를 도입하지 않습니다. 그 결과는 다른 설계 품질 check와 같은 validator placement model 안에서 `design_gate`, evidence sufficiency, blockers, display로 전달됩니다.

State/envelope validation, active Task, active Change Unit, changed paths, baseline freshness, Approval 범위, evidence sufficiency, artifact integrity, verification independence, same-session verification guard, evaluator bundle freshness, projection 최신성 같은 Core preconditions와 mechanical checks는 이 validators 전이나 옆에서 실행될 수 있습니다. 이 값들은 이 section, MCP API, [Storage와 DDL](storage-and-ddl.md)이 stable ValidatorResult-emitting set으로 명시적으로 승격하지 않는 한 대체 validator ID가 아닙니다. Surface capability는 `ValidatorResult`로 emit될 때 의도적으로 `surface_capability_check` capability validator로 model됩니다.


Adapters와 sidecars는 접점 capability를 observable facts로 번역합니다. Capability에 대한 kernel gate를 만들지는 않습니다. Capability는 `surface_capability_check` validator, `prepare_write` blocked reasons, 보장 수준 표시를 통해 나타납니다. 구체적인 host/profile의 capability declaration과 refresh trigger는 [Agent 통합 참조](agent-integration.md#capability-profiles)가 담당하고, 접점별 path는 [Surface Cookbook](surface-cookbook.md)이 이름 붙입니다.

## State transaction flow

상태를 변경하는 모든 operation은 현재 기록, event history, projection enqueue row에 대해 하나의 SQLite transaction을 사용합니다.

```text
1. request envelope, idempotency replay state, expected state version을 검증
2. transition에 필요한 project/task lock을 획득
3. 현재 상태 기록을 읽음
4. pre-transition validator를 실행
5. 현재 기록과 affected state/projection version counter를 업데이트
6. state.sqlite.task_events에 하나 이상의 row를 추가
7. 변경된 source record에 대해 projection job을 대기열에 넣음
8. commit
9. commit 이후 Markdown projections를 렌더링
```


이 transaction 안에서 Core는 current-record update의 일부로 affected scope clock을 증가시킵니다. Task-scoped changes는 `tasks.state_version`을 증가시키고, `task_id=null`인 project-scoped changes는 `project_state.state_version`을 증가시킵니다. Event rows는 각 affected scope의 resulting state version을 기록합니다. State conflict와 idempotency replay 동작은 [MCP API와 스키마의 Idempotency](mcp-api-and-schemas.md#idempotency)와 [State Conflict 동작](mcp-api-and-schemas.md#state-conflict-동작)에 드러나는 public API 계약입니다.

Projection 렌더링은 transaction 이후에 일어납니다. Projection failure는 state-isolated입니다. Projection 최신성 또는 job status를 `stale` 또는 `failed`로 표시하고 커밋된 상태는 그대로 둡니다. Projection은 transaction을 roll back하거나, `state.sqlite.task_events`를 rewrite하거나, passed task를 failed task로 바꾸거나, 나중의 reconcile decision 없이 기준 상태를 repair할 수 없습니다.

## Artifact store architecture

Artifact store는 지속 보관되는 근거 파일을 보관하지만, loose file dump가 아닙니다. Raw artifacts에는 diffs, logs, screenshots, traces, checkpoints, bundles, captured manifests, exported bundle components, 기타 integrity metadata와 owner 관계로 등록된 뒤에만 저장되는 evidence file이 포함됩니다.

Artifact는 두 부분으로 이루어집니다.

- artifact store 안의 raw file
- kind, path, hash, size, redaction state, retention class, Task-scoped owner relation을 이름 붙이는 registered artifact ref와 `state.sqlite`의 artifact 상태 기록


Core는 runs, evidence manifests, Eval records, Manual QA records, Decision Packets, 렌더링된 Task-scoped projection refs 같은 기존 Task-scoped owner record에 artifact refs를 기록합니다. MVP에서 렌더링된 projection ref로 향하는 `artifact_links`는 artifact의 `task_id` 안에 머뭅니다. Project-level projection job은 owner docs가 허용하는 곳에서 `projection_jobs` metadata로 track될 수 있지만, current MVP에서는 project-scoped artifact links가 아닙니다. Export snapshots와 components는 valid owners 또는 Task-scoped projections로 다시 link되는 artifact files로 남습니다. Exact relation rules는 MCP API, Storage와 DDL, Document Projection, Operations owner docs가 담당합니다. Large logs, diffs, screenshots, traces, patches는 원본 artifact로 두고, Markdown 보고서는 제한 없는 evidence 본문을 포함하는 대신 artifact refs로 link해야 합니다.

Raw secrets는 artifacts로 저장하면 안 됩니다. Secret-related evidence가 required라면 Core는 redacted artifact, secret handle, relevant validator를 통과한 operator note를 기록합니다.

Large logs, diffs, screenshots, traces 같은 큰 근거는 registered artifact ref로 link해야 합니다. Markdown 보고서와 export는 ref가 무엇을 뒷받침하는지 요약하고 redaction 및 availability state와 safe note를 표시할 수 있지만, 큰 evidence 본문을 붙여 넣거나 생략된 secret value를 다시 만들면 안 됩니다.

Export는 파생 bundle이며 네 번째 권한 공간이 아닙니다. Export는 Core state snapshot, 안전한 state/event version fact, report projection snapshot, artifact ref, 허용된 raw artifact file, artifact integrity result, redaction status, omitted-secret note, retained, expired, unavailable, `secret_omitted`, `blocked` artifact의 retention/availability fact를 포함할 수 있습니다. Durable file이 되는 export component는 valid owner record 또는 Task-scoped projection ref에 연결된 artifact로 남습니다. Export는 recovery artifact, stale projection, Markdown prose, chat text, staging path, operator console output에서 성공을 추론하면 안 됩니다.

### Raw artifacts, 상태 기록, Markdown 보고서

경계는 다음과 같습니다.

| Item | Authority | Examples |
|---|---|---|
| Raw artifact | Durable evidence file in artifact store | diff, log, screenshot, checkpoint, bundle, manifest file |
| 상태 기록 | `state.sqlite`의 기준 structured record | Task, Change Unit, Decision Packet, Journey Spine Entry, Residual Risk, Run, Approval, Eval, Manual QA record, Evidence Manifest, Shared Design, Artifact record |
| Markdown 보고서 | 기록과 artifact refs에서 만든 사람이 읽을 수 있는 projection | TASK, Journey Card/Spine views, Decision Packet views, APR, RUN-SUMMARY, EVAL, DIRECT-RESULT, EVIDENCE-MANIFEST |


이 named 보고서 kind는 기본적으로 상태 기록과 artifact refs에서 생성되는 projections입니다. Artifact store의 evidence file을 참조할 수 있고 export가 snapshots를 포함할 수 있지만, 그렇다고 Markdown 보고서가 기준 근거 파일이나 기준 상태가 되지는 않습니다.

## Projection and reconcile flow

Projection은 outbox-style flow입니다.

```text
상태 전이 commit 완료
→ projection job이 대기열에 들어감
→ 상태 기록과 artifact refs에서 managed block 렌더링
→ projected version과 managed hash 기록
→ human-editable area 보존
```

Projector는 managed area만 쓰고 사람이 편집할 수 있는 영역은 보존합니다. Managed area가 직접 edit되었다면 projector는 그 edit를 state로 조용히 받아들이지 않고 reconcile candidate를 기록합니다. Connector-generated file과 managed instruction block도 같은 safe non-overwrite 경계를 따릅니다. Manifest와 hash로 drift를 감지하고, existing file 또는 block은 그대로 두며, reconcile 또는 explicit reconnect decision이 owner record에서 refresh할지 결정합니다. Human-editable area에 proposal이 있으면 reconcile은 candidate record를 만들고 명시적 decision을 요청합니다. `source_state_version` 같은 front matter와 freshness line은 렌더링된 view에 대한 표시 진단 정보이지 두 번째 state clock이 아닙니다.

Reconcile 권한 경로:

```text
human-editable input
→ state.sqlite.reconcile_items
→ accepted Core state-changing action과 state.sqlite.task_events row, 또는 rejected/deferred/note outcome
```


Reconcile은 merge, reject, note로 convert, decision 생성, design support record 생성 또는 갱신, defer를 할 수 있습니다. Accepted operational changes는 Core를 통해 기록되고 `state.sqlite.task_events`에 추가됩니다.

## 보장 수준

Harness는 집행 강도를 솔직하게 보여주기 위해 보장 수준을 보고합니다.

| 수준 | 의미 |
|---|---|
| `cooperative` | agent 접점이 Harness 지시와 MCP 결정을 따를 것으로 기대됩니다. 보류는 지시에 따른 것이며 Harness가 실행 전 차단을 주장하지는 않습니다 |
| `detective` | Harness가 실행 뒤에 위반을 관찰하고 상태를 `blocked`, `stale`, `partial`, `failed`로 표시할 수 있습니다. 이는 detection이지 prevention이 아닙니다 |
| `preventive` | 구체적인 connector 또는 runtime path가 covered operation에 대해 fixture로 입증된 pre-tool blocking을 갖고 있어 실행 전에 차단할 수 있습니다 |
| `isolated` | risky work가 worktree, sandbox, process 경계, evaluator 경계 또는 동등한 isolation으로 분리됩니다. Isolation은 blast radius를 줄이지만 그 자체로 work를 승인하거나 검증하지 않습니다 |

### 보장 수준 강제 지도

이 diagram은 guarantee label이 어디에서 enforcement strength를 바꾸고, 어디에서는 바꾸지 않는지 보여줍니다. 눈여겨볼 점은 Core가 먼저 authority decision을 내린다는 것입니다. Guarantee level은 authority를 만들지 않습니다. Denied 또는 held operation이 covered operation에 대해 instruction, after-action detection, fixture-proven pre-execution blocking, isolation 중 무엇으로 처리되는지 설명할 뿐입니다.

```mermaid
flowchart TB
  Operation["intended operation"] --> Core["Core prepare_write decision<br/>state, scope, approvals,<br/>decisions, baseline, capability"]
  Core --> Decision{"allowed?"}
  Decision -->|allowed| Authorization["Write Authorization<br/>for one compatible attempt"]
  Authorization --> Attempt["covered execution or attempt<br/>under connected surface"]
  Attempt --> Run["record_run records<br/>what happened"]
  Decision -->|not allowed / hold| Hold["hold work or route blocker"]
  Hold --> Profile{"connected profile<br/>enforcement or reporting strength"}
  Profile --> Cooperative["cooperative<br/>instruction-only hold"]
  Profile --> Detective["detective<br/>detect or report after action<br/>if violation occurs"]
  Profile --> Preventive["preventive<br/>fixture-proven pre-execution block<br/>for covered operation"]
  Profile --> Isolated["isolated<br/>bounded execution 또는 promotion path"]
  Cooperative -. "when an event is recorded" .-> OwnerPaths
  Detective -. "when violation is observed" .-> OwnerPaths
  Preventive -. "when blocked attempt is recorded" .-> OwnerPaths
  Isolated -. "when promotion or escape is recorded" .-> OwnerPaths
  Run --> OwnerPaths["Core owner paths update<br/>state, artifacts, evidence,<br/>and projection jobs when applicable"]
```

Preventive와 isolated label은 연결된 profile이 해당 operation에 대한 fixture-proven coverage를 가질 때만 적용됩니다. 이 label은 work를 approve하거나, Write Authorization을 만들거나, gate를 충족하거나, evidence를 만들거나, verification을 수행하거나, risk를 accept하거나, Task를 close하지 않습니다. 엄격한 `prepare_write`와 `record_run` 동작은 [커널 참조](kernel.md#prepare_write)와 [커널 참조](kernel.md#record_run)가 담당합니다. Public response shape와 error precedence는 [MCP API와 스키마](mcp-api-and-schemas.md)가 담당합니다. 구체적인 profile declaration은 [Agent 통합 참조](agent-integration.md#capability-profiles)가 담당합니다. 이 diagram은 enforcement orientation일 뿐입니다.


보장 수준 표시는 경계의 양쪽을 모두 이름 붙여야 합니다. 연결된 profile이 실행 전에 실제로 막을 수 있는 것과, 실행 뒤에만 감지할 수 있는 것을 나눠 보여줘야 합니다. Surface name, product name, recipe name, friendly mode label만으로는 capability가 증명되지 않습니다. 선언은 실제 host/profile capability profile과 현재 proof basis에서 나와야 합니다. Guard, freeze, careful-mode label은 connected profile이 입증한 capability를 그대로 따르며, cooperative 또는 detective profile을 preventive blocking으로 올려 주지 않고 새 authority tier도 만들지 않습니다.

MVP reference behavior는 연결된 접점이 covered operation에 대해 구체적으로 fixture로 입증된 pre-tool guard나 isolation layer를 갖는 경우가 아니라면 cooperative/detective입니다. Native hook expansion, advanced sidecar watching, broad isolated execution은 MVP 기준 접점을 위해 명시적으로 구현되지 않는 한 later roadmap items입니다. Owner 문서를 통해 승격되기 전까지 이 항목들은 관찰이나 표시를 개선할 수 있을 뿐이며, write를 authorize하거나, gate를 충족하거나, Approval을 부여하거나, verification 또는 QA를 증명하거나, acceptance를 기록하거나, Core 권한을 대체하지 않습니다.

보장 수준은 표시와 risk context입니다. Approval, Write Authorization, verification, QA, acceptance, residual-risk acceptance, close readiness, kernel gate가 아닙니다.

## Failure and recovery overview

Failures는 숨기지 않고 기록합니다.

| Failure | Architecture-level handling |
|---|---|
| Agent crash during write | active Run을 `runs.status=interrupted`로 표시하거나 equivalent interrupted recovery Run을 commit합니다. 가능하면 diff/log snapshots를 캡처하고 successful completion의 증거가 아닌 recovery artifacts로 등록합니다 |
| Baseline drift | fresh baseline 또는 compatible owner path가 생길 때까지 baseline-dependent write, verification, evidence, approval, close-readiness path를 `stale` 또는 blocked로 표시합니다 |
| Approval drift | scope, baseline, sensitive category, expiry, actor context가 더 이상 맞지 않으면 Approval을 만료, 축소, 또는 재요청합니다. 오래된 Approval을 broad authorization으로 바꾸지 않습니다 |
| evaluator가 repo drift 관찰 | verification을 차단하거나 `stale`로 표시합니다. Fresh baseline, evaluator bundle, 또는 Eval path를 요구하며 drifted observation에서 detached verification passed를 설정하지 않습니다 |
| artifact file missing 또는 hash mismatch | artifact와 dependent evidence, projection, export, close-readiness view를 `stale` 또는 blocked로 표시합니다. Recovery를 통해 다시 scan하거나, 등록된 정확한 bytes를 restore하거나, replacement를 등록합니다 |
| Projection job failed | state는 current로 유지하고 projection을 failed로 표시한 뒤 retry 또는 reconcile합니다. Core state를 roll back하거나 Task result를 fail로 만들거나 rendered Markdown에서 state를 만들어내지 않습니다 |
| Managed Markdown edited directly | reconcile item을 만들고 기준 상태를 직접 바꾸지 않습니다 |
| Stale PRD, chat memory, evaluator bundle | stale context는 pull-only input으로 취급합니다. Owner path가 refresh, reconcile, supersede하기 전까지 write authorization, current Task state replacement, gate satisfaction, result acceptance, detached verification 기록, close에 사용할 수 없습니다 |
| MCP unavailable | `MCP_SERVER_UNAVAILABLE`은 tool 호출이 Core에 닿을 수 없어 authoritative Core response가 불가능한 진단 조건이고, `SURFACE_MCP_UNAVAILABLE`은 Core 또는 operator가 연결된 접점에서 사용할 수 있는 MCP가 없거나 MCP configuration이 최신이 아니거나 required tools를 호출할 수 없음을 관찰할 수 있는 진단 조건입니다. `MCP_UNAVAILABLE`은 stable public availability code로 남습니다. Product/runtime/code writes는 cooperative 접점에서는 instruction으로 보류되고, 가능한 detective path에서는 실행 뒤에 감지되며, covered operation에 대해 fixture로 입증된 preventive guard가 있을 때만 실행 전에 차단됩니다 |
| Surface capability mismatch | validator result를 기록하고 보장 수준 표시를 조정하며, required checks를 충족할 수 없으면 Write Authorization을 거부하거나 unsafe writes를 보류합니다. 실행 전 차단은 여전히 connected profile에서 fixture로 입증된 coverage에 달려 있습니다 |


Recovery tools는 projection 최신성 repair, artifact rescan, 최신이 아닌 runs interrupt, drifted approvals expire, reconcile items create를 수행할 수 있습니다. 다만 같은 권한 규칙을 보존해야 합니다. `state.sqlite`는 운영 상태이고, `state.sqlite.task_events`는 그 state store 안의 event 이력이며, 원본 근거는 artifact store에 있고, Markdown 보고서는 projection으로 남습니다. Recovery artifact와 compensating event는 recovery가 관찰하거나 변경한 내용을 설명합니다. 그 자체로 successful implementation을 증명하거나, evidence를 충족하거나, verification 또는 QA를 pass하거나, 결과 또는 잔여 위험을 수락하거나, Task를 close하지 않습니다.
