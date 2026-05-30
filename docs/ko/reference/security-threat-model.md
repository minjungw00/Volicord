# 보안 위협 모델 참조

## 이 문서로 할 수 있는 일

Runtime 구현 계획에 들어가기 전에 Harness security asset, trust boundary, threat category, control expectation을 식별할 때 이 참조 문서를 사용합니다.

이 문서는 local authority boundary를 명확하게 유지해야 하는 implementer, operator, connector author, conformance author를 위한 lookup 문서입니다. Architecture, API, storage, kernel, connector, operations owner 문서를 대체하지 않습니다.

이 문서는 참조 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 실행 목표는 코어 권한 조각(v0.1 Core Authority Slice)이며, 커널 스모크(Kernel Smoke)는 이 조각을 위한 좁은 conformance authoring profile입니다. 첫 제품 MVP 목표는 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)입니다. v0.3과 v0.4는 강화된 로컬 기준 목표(hardened local reference target)를 향해 assurance, stewardship, operations, handoff behavior를 단단하게 만드는 단계이며, v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 roadmap 범위에 둡니다.

## 이런 때 읽기

- 어떤 file, call, artifact, generated connector output이 security-sensitive인지 정해야 할 때.
- Repo document, projection, generated file, chat transcript, caller claim이 왜 operational authority가 아닌지 설명해야 할 때.
- MCP exposure, artifact handling, connector generation, stale context, approval replay, capability claim을 검토할 때.
- Security-sensitive path에서 cooperative, detective, preventive, isolated 표현 중 무엇이 정직한지 정해야 할 때.
- Security 또는 threat-model finding을 이름 붙이는 operator diagnostic이나 conformance coverage를 작성할 때.

## 읽기 전에

Runtime space, Core process model, transaction ordering, guarantee-level definition은 [런타임 아키텍처 참조](runtime-architecture.md)를 사용합니다. Connector capability profile, generated manifest, context push/pull, fallback display는 [Agent 통합 참조](agent-integration.md)를 사용합니다. `doctor`, `serve mcp`, artifact check, recover, reconcile은 [운영과 Conformance 참조](operations-and-conformance.md)를 사용하고, fixture semantics는 [Conformance Fixtures 참조](conformance-fixtures.md)를 사용합니다.

Public tool envelope, error, replay behavior는 [MCP API와 스키마](mcp-api-and-schemas.md)를 사용합니다. Exact storage layout, artifact row, DDL은 [Storage와 DDL](storage-and-ddl.md)을 사용합니다. State transition, gate, Approval, `prepare_write`, Write Authorization, acceptance, residual risk, close는 [커널 참조](kernel.md)를 사용합니다.

이 문서는 그 exact contract를 복사하지 않고 link합니다.

## 핵심 생각

Harness는 local authority layer이지 일반적인 operating-system security boundary가 아닙니다. Local file, local process, generated connector output, external command, agent surface가 Harness에 영향을 주려고 할 수 있지만, 가까이에 있다는 이유만으로 authority가 되지는 않습니다.

Canonical operational meaning은 Core가 소유한 state-changing path를 통해서만 흐릅니다. Product repository document, chat text, generated connector file, projection, artifact, external command output, MCP caller claim, remembered context는 관련 owner path가 받아들이기 전까지 input입니다.

Security display는 실제 control과 일치해야 합니다. Cooperative와 detective 접점은 instruction으로 보류하거나 실행 뒤 감지할 수 있습니다. Preventive 표현에는 covered operation에 대해 fixture로 입증된 pre-tool blocking이 필요하고, isolated 표현에는 실제 separation boundary가 필요합니다. Preventive 또는 isolated control이 필요한 high-risk work는 cooperative-only claim에 의존하면 안 됩니다.

## 담당하는 참조 범위

이 문서는 다음 항목을 담당합니다.

- threat-model concept과 vocabulary
- security asset map
- trust-boundary map
- 필수 threat와 control category
- preventive 또는 isolated control이 필요한 high-risk work가 cooperative-only claim에 의존하면 안 된다는 규칙
- threat-model concept과 exact DDL, API schema, kernel transition 사이의 non-substitution boundary

## 여기서 다루지 않는 것

이 문서는 다음 항목을 담당하지 않습니다.

- public MCP request/response schema, public error shape, idempotency/replay contract. [MCP API와 스키마](mcp-api-and-schemas.md)를 참고합니다.
- SQLite DDL, storage layout, canonical enum hardening, artifact row shape, exact file layout. [Storage와 DDL](storage-and-ddl.md)을 참고합니다.
- kernel state transition, gate, Approval lifecycle, `prepare_write`, Write Authorization, acceptance, residual-risk acceptance, close. [커널 참조](kernel.md)를 참고합니다.
- operator command semantics, diagnostic severity baseline, recover/reconcile/export behavior. [운영과 Conformance 참조](operations-and-conformance.md)를 참고합니다.
- fixture assertion semantics. [Conformance Fixtures 참조](conformance-fixtures.md)를 참고합니다.
- connector capability-profile field detail, generated-manifest contract, surface recipe. [Agent 통합 참조](agent-integration.md)와 [Surface Cookbook](surface-cookbook.md)을 참고합니다.
- projection template body 또는 managed-block rendering rule. [문서 Projection 참조](document-projection.md)를 참고합니다.
- runtime implementation, generated operational file, executable fixture, runtime data, production deployment

## 기준 전제

코어 권한 조각(v0.1 Core Authority Slice)과 staged-delivery default는 local-first입니다. 기준 배치는 사용자가 관리하는 제품 저장소, local 하네스 서버/설치, 하네스 런타임 홈, 등록된 local connector posture를 통해서만 노출되는 MCP server, 하나 이상의 연결된 agent surface입니다.

Local-first는 모든 local process를 신뢰한다는 뜻이 아닙니다. 다른 process, 오래된 connector configuration, 넓은 file permission, forwarded port, 사람이 편집한 generated file, stale chat context는 여전히 agent가 보고 하는 일에 영향을 줄 수 있습니다. 따라서 Harness는 가까운 surface를 별도 trust zone으로 다루고 owner path를 통해서만 operational meaning을 받아들입니다.

Remote 또는 shared MCP exposure는 owner documentation과 conformance가 특정 connector posture를 승격하고 증명하기 전까지 v0.1 baseline과 staged delivery 밖에 남습니다. 승격된 posture도 access-control contract, secret/PII handling, redaction 또는 omission behavior, 정직한 guarantee display, 계속 적용되는 Core validation을 보여야 합니다.

## 보안 자산

| Asset | Security concern | Boundary |
|---|---|---|
| `state.sqlite` | Core 밖에서 편집되면 canonical current operational record가 위조, replay, corruption될 수 있습니다. | Exact storage layout은 [Storage와 DDL](storage-and-ddl.md)이 담당합니다. State-changing meaning은 [커널 참조](kernel.md)와 Core transaction path를 통해야 합니다. |
| `state.sqlite.task_events` | Direct file edit가 history로 받아들여지면 event history가 위조되거나 rewrite될 수 있습니다. | Event는 state-store history이지 chat log나 report prose가 아닙니다. Recovery는 external edit를 authority로 취급하지 않고 compensating record를 추가합니다. |
| Artifact store | Evidence byte가 secret을 누출하거나, poisoning되거나, 과도하게 크거나, registered metadata와 불일치할 수 있습니다. | Artifact ref, hash, size, content type, redaction state, retention, ownership은 storage와 operations owner path를 통해 검증합니다. |
| Projections | Markdown report는 stale, tamper, prompt-injected 될 수 있고 state로 오해될 수 있습니다. | Projection은 읽기용 view 또는 proposal surface입니다. Freshness, managed block, reconcile behavior는 [문서 Projection 참조](document-projection.md)가 담당합니다. |
| MCP server | Caller가 expected caller가 아니거나, stale, remote, forwarded 상태이거나, Core에 닿지 못하면서도 state change를 주장할 수 있습니다. | Public tool은 Core와 API-owned envelope, state-version, idempotency, error contract를 통해 들어갑니다. |
| Connector-generated files | Generated instruction, manifest, MCP snippet, prompt, adapter file은 drift되거나, 사람이 편집하거나, 악성 context가 될 수 있습니다. | Generated 또는 managed file은 connector manifest와 drift reporting으로 추적합니다. 그 자체로 Task state나 authority를 만들지 않습니다. |
| Local repo | 제품 코드, test, repo docs, AGENTS-style rule, human-editable area에는 prompt injection이나 stale fact가 있을 수 있습니다. | 제품 저장소는 work와 input space이지 operational state store가 아닙니다. 제품 쓰기에는 여전히 기존 scope, Approval, write-authority path가 필요합니다. |
| External commands | Shell command, tool, test, package manager, deploy tool, network call은 file을 바꾸거나 data를 누출하거나 side effect를 만들 수 있습니다. | High-risk command, path, network, secret 사용은 관련 Change Unit, Approval, connector capability, operator control로 bounded되어야 합니다. |
| Secret handles | Handle은 raw value를 노출하지 않고 sensitive material을 가리킬 수 있지만, 오용하면 여전히 access를 넓히거나 누출할 수 있습니다. | Raw secret은 artifact나 projection이 되면 안 됩니다. Owner doc이 허용하는 곳에서는 display-safe handle 또는 omission note를 저장하고, connector manifest에는 raw token이나 secret value를 절대 저장하지 않습니다. |

## 신뢰 경계

| Boundary | Trust risk | Required posture |
|---|---|---|
| User conversation surface | Chat에는 intent, approval처럼 보이는 말, stale memory, 악성 pasted content가 들어갈 수 있습니다. | Conversation은 input으로 취급합니다. Authority에 영향을 주는 user-owned judgment는 관련 Decision Packet, Approval, acceptance, residual-risk path로 기록해야 합니다. |
| Agent surface | Surface가 MCP를 건너뛰거나, capability를 과장하거나, stale context에서 계속하거나, scope 밖 action을 수행할 수 있습니다. | Capability는 실제 host/profile에 대해 선언하고 정직하게 표시해야 합니다. 필요한 authority를 확인할 수 없으면 product/runtime/code write를 보류합니다. |
| MCP server | Local endpoint가 잘못된 caller, stale configuration, forwarded port, 약한 socket/config permission으로 도달될 수 있습니다. | Local process, local socket, localhost-loopback, 또는 문서화된 access control이 있는 promoted connector posture를 사용합니다. Public envelope는 Core를 통해 검증합니다. |
| Core | Core는 canonical mutation을 위한 authority boundary입니다. | Core만 operational state change와 owner-record effect를 commit합니다. Report, projection, generated file, caller claim은 Core를 우회하지 못합니다. |
| Runtime Home | Local file이 unrelated user, shared container, off-profile automation에 의해 읽히거나 쓰일 수 있습니다. | Broad read/write access는 tampering 또는 confidentiality risk로 취급합니다. Direct edit는 Core, recovery, artifact-integrity path가 effect를 검증하기 전까지 invalid입니다. |
| Product Repository | Human-editable docs, generated Markdown, product files, repo rule은 agent behavior에 영향을 줄 수 있습니다. | Repo file은 input, product work, projection입니다. Repo에 있다는 이유로 canonical operational state가 되지는 않습니다. |
| Artifact store | Staged 또는 committed evidence가 secret을 포함하거나, 교체되거나, integrity check에 실패할 수 있습니다. | Bytes에 의존하기 전에 path, task/run ownership, hash, size, content type, redaction/omission/block state, retention을 검증합니다. |
| External tools/network | Command와 network call은 Harness 밖 시스템에 영향을 주고 되돌리기 어려운 side effect를 만들 수 있습니다. | High-risk work에는 least-privilege tool과 explicit command/path/network/secret boundary를 사용합니다. Cooperative hold로 충분하지 않으면 stronger control이 필요합니다. |

## Threat와 control 지도

| Threat | Typical path | Required controls |
|---|---|---|
| Repo docs의 prompt injection | Repo document, old projection, generated instruction이 agent에게 Harness를 무시하거나 authority를 spoof하라고 지시합니다. | Context는 refs-first로 유지하고, repo docs는 input으로 취급하며, authority는 Core로 route합니다. Old prose 대신 current status/Journey/projection freshness를 사용합니다. |
| Projection tampering | Managed Markdown report를 편집해 Task가 approved, verified, closed된 것처럼 보이게 합니다. | Managed-block hash, `source_state_version`, projection freshness, reconcile을 사용합니다. Owner path 없이 Markdown edit를 state로 받아들이지 않습니다. |
| Stale approval replay | Scope, baseline, sensitive category, expiry, actor context가 바뀐 뒤 old approval text 또는 stale Approval record를 재사용합니다. | Write authority가 생기기 전에 Kernel과 MCP owner path를 통해 scope, baseline/state version, expiry, sensitive category, actor compatibility를 확인합니다. |
| Out-of-scope write | Agent가 active Change Unit 또는 Approval 밖의 path를 쓰거나 command를 실행하거나 network target에 닿거나 secret에 접근합니다. | Active scope, `prepare_write`, Write Authorization, changed-path validation, high-risk work용 command/path/network/secret allowlist를 사용합니다. |
| MCP unavailable인데 agent가 state update를 주장 | Core가 unreachable이거나 surface가 required MCP tool을 호출할 수 없는데 agent가 state가 바뀌었다고 말합니다. | Authority는 fail closed합니다. `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`을 구분하고, MCP가 reconnect 또는 diagnosis될 때까지 product/runtime/code write를 보류합니다. |
| Evidence artifact를 통한 secret leakage | Log, screenshot, trace, export, run summary에 token, credential, PII, private customer data가 들어갑니다. | Durable storage 전에 redact 또는 omit하고, secret handle 또는 safe note를 사용하며, forbidden bytes를 저장하지 않고 redaction/omission/block metadata를 기록합니다. |
| Artifact hash mismatch | Registered artifact metadata와 stored bytes가 맞지 않거나 staged file이 바뀝니다. | Recovery 또는 replacement가 새 artifact ref를 검증하기 전까지 artifact와 의존하는 evidence, projection, export, close-readiness view를 stale 또는 blocked로 취급합니다. |
| 악성 generated connector file | Generated instruction, MCP config snippet, manifest, adapter file이 control을 약화하거나 data exfiltration을 유도하도록 편집됩니다. | Generated/managed path를 connector manifest로 추적하고, drift를 감지하며, silent overwrite를 피하고, reconnect 또는 reconcile로 replacement를 route합니다. |
| Capability overclaiming | Surface가 실제 profile로 입증할 수 없는데 blocking, capture, isolation, MCP reachability를 주장합니다. | Current capability profile, `surface_capability_check` 또는 equivalent blocked reason, 정직한 cooperative/detective/preventive/isolated display를 요구합니다. |
| Stale context poisoning | Old chat, cached status, stale projection, stale PRD, old evaluator bundle이 agent를 unsafe 또는 outdated action으로 이끕니다. | Stale context는 pull-only input으로 취급하고, freshness를 표시하며, baseline/state version을 확인하고, authority가 의존하기 전에 refresh 또는 reconcile하며, detached verification에는 fresh evaluator bundle을 사용합니다. |

## Control 계열

### MCP local access와 caller boundary

v0.1 baseline과 staged-delivery default의 MCP posture는 registered project surface에 대한 local-only입니다. Local-only는 expected local user/profile에 대해 local process, local socket, localhost-loopback, in-process/stdio, process-scoped configuration material, per-project token 또는 handle, 이에 준하는 local IPC/control path를 뜻합니다.

Transport에 origin, caller identity, authentication token, socket path, filesystem permission, bind address가 있다면 connector profile과 operations display는 raw secret을 출력하지 않고 access-control class를 보여야 합니다. Non-loopback binding, forwarded 또는 tunneled endpoint, shared socket, cloud/CI relay, cross-user path, remote caller, stale access material은 connector owner가 해당 posture를 승격하고 증명하기 전까지 off-profile입니다.

MCP reachability는 authorization이 아닙니다. Public tool call은 계속 Core envelope validation, `project_id`, `task_id`, `surface_id`, `run_id`, `actor_kind` compatibility, idempotency, expected state version, API-owned error handling에 의존합니다.

### Least privilege와 high-risk allowlist

High-risk work는 active Change Unit을 만족할 수 있는 가장 작은 tool, command, path, network target, secret scope를 사용해야 합니다. Destructive write, network write, external service write, data export, infrastructure 또는 deployment change, production configuration change, CI/CD change, billing 또는 cost change, telemetry 또는 logging change, auth change, permission model change, secret access, privacy/PII change, license/compliance change, model 또는 prompt policy change, policy override 같은 sensitive category는 local execution이라는 이유로 안전해지지 않습니다.

Command/path/network allowlist는 여기서는 control concept이지 새 schema가 아닙니다. Exact authority는 기존 owner path에서 나옵니다. 즉 Change Unit scope, sensitive-action Approval, `prepare_write`, Write Authorization, connector capability profile, operator diagnostic입니다. Risk가 preventive blocking 또는 isolation을 요구하면 cooperative-only instruction은 충분하지 않습니다. Work는 범위를 줄이거나, 기다리거나, fixture로 입증된 preventive path를 사용하거나, 실제 isolation boundary를 사용해야 합니다.

### Storage 전 redaction

Evidence capture는 bytes가 durable artifact, projection, export, long-lived summary가 되기 전에 secret과 PII를 고려해야 합니다. Redaction, omission, blocked-payload notice는 보기 좋은 formatting이 아니라 evidence-handling control입니다.

Raw secret은 artifact, connector manifest field, projection, exported bundle text, prompt context로 저장하면 안 됩니다. Secret-related evidence가 필요하면 관련 owner path가 허용하는 display-safe secret handle, redacted artifact, omission note, operator note를 사용합니다.

### Artifact path와 integrity validation

Artifact input은 registration이 path boundary, task/run ownership, artifact kind, size, hash, content type, redaction 또는 omission state, retention/availability fact를 검증하기 전까지 untrusted입니다. Path validation은 staged path, traversal, symlink surprise, off-profile location이 실수로 trusted evidence가 되지 않게 해야 합니다.

Artifact hash mismatch는 security 및 evidence-integrity finding입니다. Markdown을 편집하거나 byte를 직접 복사해 repair하지 않습니다. Recovery 또는 replacement는 documented artifact registration과 recovery path를 통해야 합니다.

### Freshness, replay, stale context

Baseline과 state-version check는 replay와 stale context를 막습니다. Old approval, old status text, old projection, old evaluator bundle, chat memory는 current write나 current work close를 authorize할 수 없습니다. Authority가 이에 의존한다면 owner path를 통해 refresh, reconcile, supersede, replace해야 합니다.

Expected state version, idempotency, baseline compatibility, approval expiry, projection freshness, connector profile freshness는 서로 다른 control입니다. 이 문서는 그 threat-model 이유를 이름 붙입니다. Exact field와 behavior는 API, kernel, storage, projection, connector, operations owner에 남습니다.

### Authority가 unavailable이면 fail closed

State-changing, write-capable, sensitive, verification, QA, acceptance, residual-risk, close-relevant action에 필요한 authority path를 사용할 수 없으면 chat, stale projection text, generated file, cached context, operator prose에서 계속하지 말고 fail, hold, capability insufficiency report로 처리해야 합니다.

MCP unavailability에 대해서 operations와 connector는 기존 diagnostic distinction인 `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`을 사용하고, API-visible failure는 해당하는 경우 API-owned `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT` path를 사용합니다.

### 정직한 guarantee display

Security wording은 입증된 control과 일치해야 합니다.

| Guarantee | Honest security meaning |
|---|---|
| `cooperative` | Surface가 Core decision을 따르거나 보류하라고 지시받습니다. 실행 전 차단이 아닙니다. |
| `detective` | Harness가 실행 뒤 violation을 관찰하고 보고할 수 있습니다. Prevention이 아니라 detection입니다. |
| `preventive` | Concrete hook, wrapper, permission layer, policy engine, sidecar 또는 equivalent가 covered operation에 대해 fixture로 입증된 pre-tool blocking을 제공합니다. |
| `isolated` | Work 또는 verification이 별도 worktree, sandbox, process, evaluator bundle, 또는 equivalent boundary를 가로질러 실행됩니다. Isolation은 blast radius를 줄이지만 그 자체로 approval, verification, acceptance, close를 만들지 않습니다. |

Guard, freeze, careful-mode, recipe name, product name, surface name, friendly mode label은 guarantee를 올려 주지 않습니다. High-risk work는 실제 사용하는 control을 보여야 하며, preventive 또는 isolated control이 필요한 경우 cooperative-only claim에 의존하면 안 됩니다.

## Exact contract owner 지도

| Threat-model concept | Exact contract owner |
|---|---|
| MCP tool envelope, `ToolError`, public error, idempotency, replay, expected state version | [MCP API와 스키마](mcp-api-and-schemas.md) |
| Kernel state transition, gate, Approval, `prepare_write`, Write Authorization, acceptance, residual risk, close | [커널 참조](kernel.md) |
| `state.sqlite`, `task_events`, artifact storage row, DDL, enum hardening, hash, storage layout | [Storage와 DDL](storage-and-ddl.md) |
| Runtime space, Core transaction ordering, artifact architecture, guarantee level definition | [런타임 아키텍처 참조](runtime-architecture.md) |
| Connector capability profile, generated manifest, context push/pull, fallback display | [Agent 통합 참조](agent-integration.md) |
| Operator diagnostic, severity baseline, `doctor`, `serve mcp`, artifact check, recover, reconcile | [운영과 Conformance 참조](operations-and-conformance.md) |
| Conformance fixture body shape, assertion semantics, suite catalog, example | [Conformance Fixtures 참조](conformance-fixtures.md) |
| Projection freshness, managed block, reconcile behavior, template ownership | [문서 Projection 참조](document-projection.md)와 [Template 참조](templates/README.md) |
