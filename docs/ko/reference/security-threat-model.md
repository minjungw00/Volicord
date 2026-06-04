# 보안 위협 모델 참조

## 이 문서로 할 수 있는 일

Runtime 구현 계획에 들어가기 전에 Harness 보안 asset, trust boundary, threat category, control expectation을 식별할 때 이 참조 문서를 사용합니다.

이 문서는 local authority boundary를 명확하게 유지해야 하는 implementer, operator, connector author, conformance author를 위한 lookup 문서입니다. Architecture, API, storage, kernel, connector, operations owner 문서를 대체하지 않습니다.

이 문서는 향후 Harness 동작을 위한 참조 문서입니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)에 있습니다.

## 이런 때 읽기

- 어떤 file, call, artifact, generated connector output이 security-sensitive인지 정해야 할 때.
- Repo document, projection, generated file, chat transcript, caller claim이 왜 operational authority가 아닌지 설명해야 할 때.
- MCP exposure, artifact handling, connector generation, stale context, approval replay, capability claim을 검토할 때.
- 보안 민감 경로에서 cooperative, detective, preventive, isolated 표현 중 무엇이 정직한지 정해야 할 때.
- Security 또는 threat-model finding을 이름 붙이는 operator diagnostic이나 conformance coverage를 작성할 때.

## 읽기 전에

Runtime space, Core process model, transaction ordering, architecture placement는 [런타임 아키텍처 참조](runtime-architecture.md)를 사용합니다. Connector capability profile, generated manifest, context push/pull, fallback display는 [Agent 통합 참조](agent-integration.md)를 사용합니다. 단계별 `doctor`, `serve mcp`, artifact check, recover, reconcile behavior는 [운영과 Conformance 참조](operations-and-conformance.md)를 사용하고, fixture semantics는 [Conformance Fixtures 참조](conformance-fixtures.md)를 사용합니다.

Public tool envelope와 shared shape는 [API Schema Core](api/schema-core.md)를 사용하고, public error와 replay behavior는 [API Errors](api/errors.md)를 사용합니다. Exact storage layout, artifact row, DDL은 [Storage와 DDL](storage-and-ddl.md)을 사용합니다. State transition, gate, Approval, `prepare_write`, Write Authorization, acceptance, residual risk, close는 [커널 참조](kernel.md)를 사용합니다.

이 문서는 그 exact contract를 복사하지 않고 link합니다.

## 핵심 생각

Harness는 로컬 권한 계층이지 일반적인 운영체제 보안 경계가 아닙니다. 로컬 파일, local process, generated connector output, external command, agent surface가 Harness에 영향을 주려고 할 수 있지만, 가까이에 있다는 이유만으로 authority가 되지는 않습니다.

Canonical operational meaning은 Core가 소유한 state-changing path를 통해서만 흐릅니다. Product repository document, chat text, generated connector file, projection, artifact, external command output, MCP caller claim, remembered context는 관련 owner path가 받아들이기 전까지 input입니다.

Security display는 실제 control과 일치해야 합니다. `cooperative`는 agent나 tool이 문서화된 절차를 따를 때 동작하는 협력형 확인입니다. `detective`는 Harness가 mismatch나 record inconsistency를 사후 확인할 수 있다는 뜻입니다. `preventive`는 입증된 control이 covered action을 실행 전에 차단한다는 뜻입니다. `isolated`는 주장이 정의되고 입증된 isolation boundary를 이름 붙인다는 뜻입니다. Preventive 또는 isolated control이 필요한 high-risk work는 cooperative-only claim에 의존하면 안 됩니다.

초기 로컬 하네스 단계는 OS 권한을 자동으로 제공하거나, 임의 도구를 sandbox 격리하거나, 로컬 파일을 변조 불가능하게 만들거나, 지시 기반 agent behavior를 사전 차단 보안으로 바꾸지 않습니다. 내부 엔지니어링 점검과 MVP-1은 현재 owner record와 맞지 않는 Core state-changing action을 거부하고, state를 기록하고, active Core path에 필요한 최소 artifact/evidence ref를 검증하고, stale 또는 mismatched fact를 보고하고, 정직한 보장 한계를 표시할 수 있습니다. 구조화된 막힘은 Core 또는 연결된 접점이 Harness 기록/확인 경로로 진행할 수 없다고 보고한다는 뜻이며, Harness가 실행 전에 process를 물리적으로 멈췄다는 주장이 아닙니다. 사용자에게 보이는 문구는 "현재 하네스 기록/쓰기 전 범위 확인과 맞지 않음", "지시로 보류됨", "runtime이 실행 전에 물리적으로 막음"을 구분해야 합니다. Preventive control은 owner 문서와 conformance가 exact covered operation을 증명하기 전까지 향후 또는 profile별 범위입니다. Isolated control은 exact separation boundary를 증명하기 전까지 향후 또는 profile별 범위입니다.

운영자 진입점은 그것을 도입한 stage와 connector profile의 guarantee level을 그대로 따릅니다. 나중 단계의 recover, export, reconcile, artifact check, conformance run, release handoff surface도 입증된 cooperative, detective, preventive, isolated capability보다 더 강하게 prevention이나 enforcement를 제공한다고 설명하면 안 됩니다.

격리 주장은 어떤 종류의 분리를 주장하는지 이름 붙여야 합니다. Fresh evaluator bundle, fresh session, separate worktree는 verification independence, stale-context control, blast-radius reduction을 뒷받침할 수 있습니다. Sandbox 격리, 권한 계층, locked-down runner, process boundary, container boundary는 connector profile이 exact mechanism을 이름 붙이고 증명한 경우에만 더 강한 보안 격리를 뒷받침합니다.

## 단계별 guarantee level

아래는 로컬 reference path의 기본 단계별 guarantee입니다. 구체적인 connector, operator path, later profile은 exact covered operation 또는 separation boundary를 이름 붙이고 owner documentation과 conformance proof를 제시할 때만 더 강한 level을 주장할 수 있습니다.

| 단계 | 기본 guarantee posture | 정직한 주장 경계 |
|---|---|---|
| 내부 엔지니어링 점검 | 협력형 확인(cooperative) + 제한된 사후 확인(detective) behavior. | Core는 현재 owner record와 맞지 않는 state-changing call을 거부하고, 구조화된 상태/막힘 출력을 만들고, 호환되는 내부 Write Authorization record 하나를 만들고 consume하며, Run 하나를 기록하고, active path에 필요한 최소 artifact/evidence ref를 검증할 수 있습니다. 이 내부 기록은 OS 권한이 아니며, 별도의 preventive profile이 증명되지 않는 한 local process나 agent가 Harness 밖에서 file을 edit하는 것을 멈추지 않습니다. |
| MVP-1 사용자 작업 루프 | 협력형 확인(cooperative) + 사용자에게 보이는 blocker/status와 제한된 사후 확인(detective) behavior. | 사용자는 missing scope, missing decision, missing evidence, close blocker, MCP availability, 정직한 보장 상태를 볼 수 있습니다. 필요한 Harness 기록/확인에 닿을 수 없거나 확인할 수 없으면 product/runtime/code write는 지시로 보류됩니다. 이것은 여전히 기본 도구 실행 전 차단이나 권한 격리가 아닙니다. |
| 보증 프로필 | Verification, QA, residual risk, 작업 수락, sensitive-action Approval 분리를 더 강하게 보여 주는 cooperative/detective assurance. | Harness는 assurance gap, stale evidence, missing independence, QA blocker, waiver/risk/acceptance boundary, context-hygiene finding을 기록하고 보고할 수 있습니다. 특정 profile이 capability를 증명하지 않는 한 preventive 또는 isolated가 되지 않습니다. |
| 운영 프로필 | Recover, export, readiness, artifact integrity, projection freshness, handoff reporting 주변의 탐지 가능(detective) operational behavior. | Operator surface는 진단, 보고, owner path를 통한 repair, safe bundle export, artifact integrity check를 수행할 수 있습니다. 기본적으로 Runtime Home을 변조 불가능하게 만들거나, projection을 authoritative하게 만들거나, 임의 도구를 격리하지 않습니다. |
| 로드맵 | owner 문서가 승격하고 해당 operation 또는 boundary가 증명된 경우에만 사전 차단(preventive) 또는 격리(isolated) 후보. | 더 강한 주장은 exact contract, covered operation, fixture proof, fallback behavior가 필요하며, 격리의 경우 proven sandbox, permission boundary, locked-down runner, process boundary, container boundary 같은 실제 separation boundary를 이름 붙여야 합니다. |

이 단계 지도는 Core 권한을 낮추지 않습니다. Core는 active owner contract에 따라 invalid state transition을 거부하거나, 내부 Write Authorization record를 만들지 않거나, gate 또는 파생 view를 `stale`/`blocked`로 표시하거나, 구조화된 막힘을 보고할 수 있습니다. 이 지도는 Harness가 action을 실행 전에 물리적으로 멈출 수 있는지, 또는 security boundary 뒤에 격리할 수 있는지에 대한 보안 표현만 제한합니다.

## 내부 엔지니어링 점검 / MVP-1에서 가능한 기본 통제

내부 엔지니어링 점검과 MVP-1 reference path는 사전 차단형(preventive) 또는 격리형(isolated) runtime boundary를 주장하지 않고도 다음 통제를 사용할 수 있습니다.

- 등록된 project surface의 local-only 접근 상태 표시
- 제품 저장소 / 하네스 서버 / 하네스 런타임 홈의 명확한 분리
- raw secret과 token 응답 금지, 표시해도 안전한 handle, redaction, omission, blocked-payload notice 사용
- active owner path가 요구하는 artifact 경로 검증, owner 관계 확인, 기본 fingerprint/hash 확인
- 상태 변경 호출을 위한 `expected_state_version` freshness check와 idempotency key
- `prepare_write`가 반환하고 compatible `record_run`이 consume하는 1회용 내부 Write Authorization record. 이는 하네스 수준의 협력형 확인 기록이며 OS 권한이나 sandboxing이 아닙니다.
- 오래된 민감 동작 permission 또는 later Approval record, projection, baseline, connector profile, evaluator bundle, retrieved context에 대한 stale context blocker 또는 warning
- MCP/Core를 사용할 수 없을 때 authority claim을 fail closed로 처리
- Core가 현재 범위/상태와 맞는지 확인할 수 없는 것 또는 surface가 탐지할 수 있는 것을 보여 주되 물리적 pre-tool enforcement를 암시하지 않는 cooperative/detective blocker display

이 통제들은 Core state change를 거부하거나, authority claim이 만들어지지 않게 하거나, inconsistency를 보이게 할 수 있습니다. 기본 reference path에서는 임의의 로컬 프로세스나 도구가 파일을 쓰는 일을 물리적으로 막지 않습니다.

## 향후 또는 profile 승격 통제

다음 통제는 owner 문서가 mechanism과 covered operation 또는 separation boundary를 정의하고, conformance가 이를 증명하기 전까지 향후 또는 profile별 범위입니다.

- 운영체제 sandboxing
- 임의 도구 격리
- 변조 불가능한 Harness Runtime Home storage
- product/runtime/code write에 대한 사전 차단형(preventive) pre-tool blocking
- 강화된 다중 사용자 권한
- local, remote, shared, cloud, CI, cross-user posture 전반의 broad connector security model
- full secret manager 또는 data-loss-prevention system

그렇게 승격되기 전까지 guard, freeze mode, careful mode, sidecar, hook, wrapper, worktree, bundle, local file에 대한 언급은 exact preventive 또는 isolated boundary가 증명되지 않는 한 cooperative 또는 detective control 설명입니다.

## 단계별 scenario posture

| Scenario | 내부 엔지니어링 점검 | MVP-1 사용자 작업 루프 | 보증 프로필 | 운영 프로필 | 로드맵 |
|---|---|---|---|---|---|
| MCP unavailable | Harness record/check가 필요한 call은 fail 또는 hold됩니다. Chat이나 cached text에서 Core state, Write Authorization record, evidence, 작업 수락, 잔여 위험 수용, close claim을 만들어 내지 않습니다. | 사용자는 availability 막힘/status와 다음 reconnect 또는 diagnosis action을 봅니다. 입증된 더 강한 profile이 해당 operation을 cover하지 않는 한 product/runtime/code write는 지시로 보류됩니다. | Assurance path는 unavailable path를 통해 verification, QA, waiver, risk, acceptance state를 신뢰할 수 없다고 보고합니다. | `serve mcp`, `doctor`, `recover`는 `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`을 구분하고 public `MCP_UNAVAILABLE`/capability error boundary를 보존합니다. | 승격된 guard는 증명된 path에서만 covered write를 실행 전에 멈출 수 있고, 승격된 isolation profile은 실제 boundary를 통해 work를 route할 수 있습니다. |
| Out-of-scope write | `prepare_write`는 Write Authorization record를 만들지 않고 구조화된 막힘을 반환할 수 있습니다. External edit는 active path가 관찰할 때만 탐지됩니다. | 사용자는 무엇이 scope 밖인지 보고, 올바른 decision path를 통해 scope를 줄이거나 의도적으로 넓힐 수 있습니다. | Autonomy, approval, evidence, changed-path check가 run, evidence, verification, close readiness를 stale/blocked/insufficient로 표시할 수 있습니다. | Doctor, recover, reconcile이 changed-path 또는 generated-file drift를 보고하고 repair를 owner path로 route할 수 있습니다. | Preventive profile은 fixture proof가 해당 operation을 cover할 때만 covered path/command/network/secret을 실행 전에 멈출 수 있습니다. |
| Sensitive-action approval | Full Approval semantics는 owner profile이 좁은 case를 승격하지 않는 한 최소 조각 밖입니다. Active scope 밖의 sensitive action은 보류되거나 unsupported로 취급됩니다. | 사용자는 이름 붙은 sensitive step, permission 필요/기록 여부, 그리고 permission이 작업 수락이나 잔여 위험 수용이 아니라는 점을 봅니다. | Approval은 Decision Packet, Write Authorization, QA/verification waiver, 작업 수락, 잔여 위험 수용과 분리됩니다. | Operator diagnostic과 export/handoff report는 외부 approval 또는 deployment authority를 만들지 않고 Approval status를 보여줄 수 있습니다. | Policy wrapper 또는 permission system은 exact covered action에 대한 proof가 있을 때만 preventive가 될 수 있습니다. |
| Stale projection | Persisted projection은 필수가 아닙니다. 오래된 readable text는 Core state가 아닙니다. | Readable summary/card는 freshness warning을 표시할 수 있으며 stale이면 authority로 쓰면 안 됩니다. | Assurance와 context-hygiene check는 verification/QA/close가 view에 의존하기 전에 fresh state, fresh evaluator bundle, reconcile을 요구할 수 있습니다. | Projection refresh, reconcile, doctor, export, recover가 committed state를 유지하면서 freshness를 owner path로 보고하거나 repair할 수 있습니다. | Richer projection/UI system은 owner docs가 mutation path를 정의하고 증명하기 전까지 read-only로 남습니다. |
| Artifact tampering | Active path에서 registered artifact ref와 최소 integrity fact를 확인합니다. Direct file edit는 evidence authority가 아닙니다. | Evidence와 close summary는 missing, stale, mismatched artifact support를 보여줍니다. | Evidence, Eval, Manual QA, waiver, risk, close path는 replacement 또는 owner decision이 gap을 해소할 때까지 stale, insufficient, blocked, unresolved가 될 수 있습니다. | Artifact check, recover, export는 hash, retention, redaction, omitted-secret, blocked-payload metadata를 검증하되 staged file이나 Markdown을 신뢰하지 않습니다. | Storage hardening 또는 locked artifact handling은 실제 boundary와 conformance proof가 있을 때만 더 강해질 수 있습니다. |
| Prompt injection | Repo doc, generated file, old projection, chat은 input입니다. Authority를 만들거나 Core를 우회할 수 없습니다. | User-facing status와 judgment prompt는 broad approval처럼 보이는 prose를 authority로 취급하지 않고 current scope와 decision을 보여줘야 합니다. | Context-hygiene, stewardship, evaluator freshness, User Judgment route는 assurance claim이 stale 또는 malicious context에 의존하기 전에 이를 보이게 합니다. | Doctor와 reconcile은 generated-file drift, stale context, projection tampering, managed-block edit를 보고할 수 있습니다. | Content filter, isolated evaluator, 더 강한 prompt-containment mechanism은 exact boundary가 증명되기 전까지 Expansion 후보입니다. |
| Secret leakage | Raw secret은 artifact, manifest, projection, prompt context가 되면 안 됩니다. 최소 evidence path는 필요할 때 redaction, omission, safe handle을 사용합니다. | 사용자는 raw value 없이 evidence gap, omitted-secret note, safe secret handle을 봅니다. | Evidence, QA, Eval, waiver, residual-risk path는 assurance 또는 close claim이 의존하기 전에 redaction, omission, blocked payload를 반영합니다. | Artifact check와 export/handoff는 omission/block metadata를 보존하고 raw staged, omitted, blocked, secret, PII value를 복사하지 않습니다. | Secret scanner, permission wrapper, data-loss-prevention control은 storage 또는 transmission 전에 covered leakage를 막고 그 path를 증명할 때만 preventive입니다. |

## 담당하는 참조 범위

이 문서는 다음 항목을 담당합니다.

- threat-model concept과 vocabulary
- security asset map
- trust-boundary map
- 필수 threat와 control category
- guarantee-level 의미와 honest-display rule
- preventive 또는 isolated control이 필요한 high-risk work가 cooperative-only claim에 의존하면 안 된다는 규칙
- threat-model concept과 exact DDL, API schema, kernel transition 사이의 non-substitution boundary

## 여기서 다루지 않는 것

이 문서는 다음 항목을 담당하지 않습니다.

- public MCP request/response schema. [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md)를 참고합니다.
- public error shape, idempotency/replay contract. [API Errors](api/errors.md)를 참고합니다.
- SQLite DDL, storage layout, canonical enum hardening, artifact row shape, exact file layout. [Storage와 DDL](storage-and-ddl.md)을 참고합니다.
- kernel state transition, gate, Approval lifecycle, `prepare_write`, Write Authorization, acceptance, 잔여 위험 수용, close. [커널 참조](kernel.md)를 참고합니다.
- 단계별 operator command semantics, diagnostic severity baseline, recover/reconcile/export behavior. [운영과 Conformance 참조](operations-and-conformance.md)를 참고합니다.
- fixture assertion semantics. [Conformance Fixtures 참조](conformance-fixtures.md)를 참고합니다.
- connector capability-profile field detail, generated-manifest contract, surface recipe. [Agent 통합 참조](agent-integration.md)와 [Surface Cookbook](surface-cookbook.md)을 참고합니다.
- projection template body 또는 managed-block rendering rule. [문서 Projection 참조](document-projection.md)를 참고합니다.
- runtime implementation, generated operational file, executable fixture, runtime data, production deployment

## 기준 전제

내부 엔지니어링 점검과 staged-delivery default는 local-first입니다. 기준 배치는 사용자가 관리하는 제품 저장소, local 하네스 서버/설치, 하네스 런타임 홈, 등록된 local connector posture를 통해서만 노출되는 MCP server, 하나 이상의 연결된 agent surface입니다.

Local-first는 모든 local process를 신뢰한다는 뜻이 아닙니다. 다른 process, 오래된 connector configuration, 넓은 file permission, forwarded port, 사람이 편집한 generated file, stale chat context는 여전히 agent가 보고 하는 일에 영향을 줄 수 있습니다. 따라서 Harness는 가까운 surface를 별도 trust zone으로 다루고 owner path를 통해서만 operational meaning을 받아들입니다.

Remote 또는 shared MCP exposure는 owner documentation과 conformance가 특정 connector posture를 승격하고 증명하기 전까지 내부 엔지니어링 점검 baseline과 staged delivery 밖에 남습니다. 승격된 posture도 access-control contract, secret/PII handling, redaction 또는 omission behavior, 정직한 guarantee display, 계속 적용되는 Core validation을 보여야 합니다.

## 보안 자산

| Asset | Security concern | Boundary |
|---|---|---|
| `state.sqlite` | Core 밖에서 편집되면 canonical current operational record가 위조, replay, corruption될 수 있습니다. | Exact storage layout은 [Storage와 DDL](storage-and-ddl.md)이 담당합니다. State-changing meaning은 [커널 참조](kernel.md)와 Core transaction path를 통해야 합니다. |
| `state.sqlite.task_events` | Direct file edit가 history로 받아들여지면 event history가 위조되거나 rewrite될 수 있습니다. | Event는 state-store history이지 chat log나 report prose가 아닙니다. Recovery는 external edit를 authority로 취급하지 않고 compensating record를 추가합니다. |
| Artifact store | Evidence byte가 secret을 누출하거나, poisoning되거나, 과도하게 크거나, registered metadata와 불일치할 수 있습니다. | Artifact ref, hash, size, content type, redaction state, retention, ownership은 storage와 operations owner path를 통해 검증합니다. |
| Projections | Markdown report는 stale, tamper, prompt-injected 될 수 있고 state로 오해될 수 있습니다. | Projection은 읽기용 요약 또는 proposal surface입니다. Freshness, managed block, reconcile behavior는 [문서 Projection 참조](document-projection.md)가 담당합니다. |
| MCP server | Caller가 expected caller가 아니거나, stale, remote, forwarded 상태이거나, Core에 닿지 못하면서도 state change를 주장할 수 있습니다. | Public tool은 Core와 API-owned envelope, state-version, idempotency, error contract를 통해 들어갑니다. |
| Connector-generated files | Generated instruction, manifest, MCP snippet, prompt, adapter file은 drift되거나, 사람이 편집하거나, 악성 context가 될 수 있습니다. | Generated 또는 managed file은 connector manifest와 drift reporting으로 추적합니다. 그 자체로 Task state나 authority를 만들지 않습니다. |
| Local repo | 제품 코드, test, repo docs, AGENTS-style rule, human-editable area에는 prompt injection이나 stale fact가 있을 수 있습니다. | 제품 저장소는 work와 input space이지 operational state store가 아닙니다. 제품 쓰기에는 여전히 현재 scope, 필요한 민감 동작 permission, 쓰기 전 범위 확인 / 내부 Write Authorization path가 필요합니다. |
| External commands | Shell command, tool, test, package manager, deploy tool, network call은 file을 바꾸거나 data를 누출하거나 side effect를 만들 수 있습니다. | High-risk command, path, network, secret 사용은 관련 Change Unit, Approval, connector capability, operator control로 bounded되어야 합니다. |
| Secret handles | Handle은 raw value를 노출하지 않고 sensitive material을 가리킬 수 있지만, 오용하면 여전히 access를 넓히거나 누출할 수 있습니다. | Raw secret은 artifact나 projection이 되면 안 됩니다. Owner doc이 허용하는 곳에서는 display-safe handle 또는 omission note를 저장하고, connector manifest에는 raw token이나 secret value를 절대 저장하지 않습니다. |

## 신뢰 경계

| Boundary | Trust risk | Required posture |
|---|---|---|
| User conversation surface | Chat에는 intent, approval처럼 보이는 말, stale memory, 악성 pasted content가 들어갈 수 있습니다. | Conversation은 input으로 취급합니다. Authority에 영향을 주는 user-owned judgment는 관련 Decision Packet, Approval, 작업 수락과 잔여 위험 path로 기록해야 합니다. |
| Agent surface | Surface가 MCP를 건너뛰거나, capability를 과장하거나, stale context에서 계속하거나, scope 밖 action을 수행할 수 있습니다. | Capability는 실제 host/profile에 대해 선언하고 정직하게 표시해야 합니다. 필요한 Harness record/check를 확인할 수 없으면 product/runtime/code write를 지시로 보류합니다. |
| Harness Server / Installation | Local control-plane process, connector adapter, projector, reconciler, operator entrypoint가 stale, misconfigured이거나 Core를 우회하는 input을 신뢰하라는 요청을 받을 수 있습니다. | Installation은 control plane이지 일반 OS sandbox가 아닙니다. State-changing effect는 Core owner path를 거치며, adapter와 tool은 authority를 만들지 않고 capability, diagnostic, proposal을 보고합니다. |
| Local process | Shell, editor, test runner, package manager, sidecar, 다른 local process가 의도한 profile 밖에서 file을 바꾸거나 secret을 읽거나 local endpoint를 호출할 수 있습니다. | Local execution 자체가 trust는 아닙니다. Scope, Approval, connector capability, least-privilege tool choice로 process behavior를 제한하고, cooperative/detective posture가 충분하지 않으면 더 강한 control을 요구합니다. |
| Local socket 또는 API surface | Local endpoint가 잘못된 caller, stale configuration, forwarded port, 약한 socket/config permission, off-profile access material로 도달될 수 있습니다. | Local process, local socket, localhost-loopback, in-process/stdio, 또는 문서화된 access control이 있는 promoted connector posture를 사용합니다. Public envelope는 Core를 통해 검증하며 reachability를 OS permission이나 유효한 Harness record/check로 취급하지 않습니다. |
| Core | Core는 canonical mutation을 위한 authority boundary입니다. | Core만 operational state change와 owner-record effect를 commit합니다. Report, projection, generated file, caller claim은 Core를 우회하지 못합니다. |
| Harness Runtime Home | 로컬 파일이 unrelated user, shared container, off-profile automation에 의해 읽히거나 쓰일 수 있습니다. | Broad read/write access는 tampering 또는 confidentiality risk로 취급합니다. Direct edit는 Core, recovery, artifact-integrity path가 effect를 검증하기 전까지 invalid입니다. 로컬이라는 이유만으로 이 파일이 변조 불가능하다고 주장하지 않습니다. |
| Product Repository | Human-editable docs, generated Markdown, product files, repo rule은 agent behavior에 영향을 줄 수 있습니다. | Repo file은 input, product work, projection입니다. Repo에 있다는 이유로 canonical operational state가 되지는 않습니다. |
| Artifact store | Staged 또는 committed evidence가 secret을 포함하거나, 교체되거나, integrity check에 실패할 수 있습니다. | Bytes에 의존하기 전에 path, task/run ownership, hash, size, content type, redaction/omission/block state, retention을 검증합니다. |
| Generated projections | Managed Markdown, compact status card, report, generated connector view는 stale, edited, prompt-injected 될 수 있고 authority로 오해될 수 있습니다. | Projection은 읽기용 요약 또는 proposal surface로 취급합니다. Freshness, managed-block hash, reconcile은 state에 영향을 주기 전에 owner path를 거칩니다. |
| External tools/network | Command와 network call은 Harness 밖 시스템에 영향을 주고 되돌리기 어려운 side effect를 만들 수 있습니다. | High-risk work에는 least-privilege tool과 explicit command/path/network/secret boundary를 사용합니다. Cooperative hold로 충분하지 않으면 stronger control이 필요합니다. |

## Threat와 control 지도

| Threat | Typical path | Required controls |
|---|---|---|
| Repo docs의 prompt injection | Repo document, old projection, generated instruction이 agent에게 Harness를 무시하거나 authority를 spoof하라고 지시합니다. | Context는 refs-first로 유지하고, repo docs는 input으로 취급하며, authority는 Core로 route합니다. Old prose 대신 current status/Journey/projection freshness를 사용합니다. |
| Projection tampering | Managed Markdown report를 편집해 Task가 approved, verified, closed된 것처럼 보이게 합니다. | Managed-block hash, `source_state_version`, projection freshness, reconcile을 사용합니다. Owner path 없이 Markdown edit를 state로 받아들이지 않습니다. |
| Stale approval replay | Scope, baseline, sensitive category, expiry, actor context가 바뀐 뒤 old approval text 또는 stale Approval record를 재사용합니다. | 쓰기 전 범위 확인이 compatible record를 만들기 전에 Kernel과 MCP owner path를 통해 scope, baseline/state version, expiry, sensitive category, actor compatibility를 확인합니다. |
| Out-of-scope write | Agent가 active Change Unit 또는 Approval 밖의 path를 쓰거나 command를 실행하거나 network target에 닿거나 secret에 접근합니다. | Active scope, `prepare_write`, 내부 Write Authorization record, changed-path validation, high-risk work용 command/path/network/secret allowlist를 사용합니다. 이 확인은 preventive profile이 해당 operation을 증명하지 않는 한 OS 권한, sandboxing, 물리적 도구 실행 전 차단을 뜻하지 않습니다. |
| MCP unavailable인데 agent가 state update를 주장 | Core가 unreachable이거나 surface가 required MCP tool을 호출할 수 없는데 agent가 상태가 바뀌었다고 말합니다. | Authority는 fail closed합니다. `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`을 구분하고, MCP가 reconnect 또는 diagnosis될 때까지 product/runtime/code write를 보류합니다. |
| Evidence artifact를 통한 secret leakage | Log, screenshot, trace, export, run summary에 token, credential, PII, private customer data가 들어갑니다. | Durable storage 전에 redact 또는 omit하고, secret handle 또는 safe note를 사용하며, forbidden bytes를 저장하지 않고 redaction/omission/block metadata를 기록합니다. |
| Artifact hash mismatch | Registered artifact metadata와 stored bytes가 맞지 않거나 staged file이 바뀝니다. | Recovery 또는 replacement가 새 artifact ref를 검증하기 전까지 artifact와 의존하는 evidence, projection, export, close-readiness view를 stale 또는 blocked로 취급합니다. |
| 악성 generated connector file | Generated instruction, MCP config snippet, manifest, adapter file이 control을 약화하거나 data exfiltration을 유도하도록 편집됩니다. | Generated/managed path를 connector manifest로 추적하고, drift를 감지하며, silent overwrite를 피하고, reconnect 또는 reconcile로 replacement를 route합니다. |
| Capability overclaiming | Surface가 실제 profile로 입증할 수 없는데 blocking, capture, isolation, MCP reachability를 주장합니다. | Current capability profile, `surface_capability_check` 또는 equivalent blocked reason, 정직한 cooperative/detective/preventive/isolated display를 요구합니다. |
| Stale context poisoning | Old chat, cached status, stale projection, stale PRD, old evaluator bundle이 agent를 unsafe 또는 outdated action으로 이끕니다. | Stale context는 pull-only input으로 취급하고, freshness를 표시하며, baseline/state version을 확인하고, authority가 의존하기 전에 refresh 또는 reconcile하며, 분리 검증에는 fresh evaluator bundle을 사용합니다. |

## Control 계열

### MCP local access와 caller boundary

내부 엔지니어링 점검 baseline과 staged-delivery default의 MCP posture는 registered project surface에 대한 local-only입니다. Local-only는 expected local user/profile에 대해 local process, local socket, localhost-loopback, in-process/stdio, process-scoped configuration material, per-project token 또는 handle, 이에 준하는 local IPC/control path를 뜻합니다.

Transport에 origin, caller identity, authentication token, socket path, filesystem permission, bind address가 있다면 connector profile과 operations display는 raw secret을 출력하지 않고 access-control class를 보여야 합니다. Non-loopback binding, forwarded 또는 tunneled endpoint, shared socket, cloud/CI relay, cross-user path, remote caller, stale access material은 connector owner가 해당 posture를 승격하고 증명하기 전까지 off-profile입니다.

MCP reachability는 OS permission이나 유효한 Harness record/check가 아닙니다. Public tool call은 계속 Core envelope validation, `project_id`, `task_id`, `surface_id`, `run_id`, `actor_kind` compatibility, idempotency, expected state version, API-owned error handling에 의존합니다.

Core에 닿을 수 없으면 authoritative Core response는 존재하지 않으며 API-visible path는 `MCP_UNAVAILABLE` 또는 `MCP_SERVER_UNAVAILABLE` 같은 operations diagnostic입니다. Core 또는 operator가 reachable local caller나 access path가 registered local profile 밖이라고 분류할 수 있으면 API-visible path는 display-safe detail을 포함한 `LOCAL_ACCESS_MISMATCH`입니다. Caller가 recognized profile 위에 있지만 required capability가 없으면 `CAPABILITY_INSUFFICIENT`를 사용합니다.

### Least privilege와 high-risk allowlist

High-risk work는 active Change Unit을 만족할 수 있는 가장 작은 tool, command, path, network target, secret scope를 사용해야 합니다. Destructive write, network write, external service write, data export, infrastructure 또는 deployment change, production configuration change, CI/CD change, billing 또는 cost change, telemetry 또는 logging change, auth change, permission model change, secret access, privacy/PII change, license/compliance change, model 또는 prompt policy change, policy override 같은 sensitive category는 local execution이라는 이유로 안전해지지 않습니다.

Command/path/network allowlist는 여기서는 control concept이지 새 schema가 아닙니다. Exact Harness record/check path는 기존 owner path에서 나옵니다. 즉 Change Unit scope, sensitive-action Approval, `prepare_write`, 내부 Write Authorization record, connector capability profile, operator diagnostic입니다. Risk가 preventive blocking 또는 isolation을 요구하면 cooperative-only instruction은 충분하지 않습니다. Work는 범위를 줄이거나, 기다리거나, fixture로 입증된 preventive path를 사용하거나, connector profile이 주장하고 문서화와 증명을 마친 separation boundary를 사용해야 합니다.

### Storage 전 redaction

Evidence capture는 bytes가 durable artifact, projection, export, long-lived summary가 되기 전에 secret과 PII를 고려해야 합니다. Redaction, omission, blocked-payload notice는 보기 좋은 formatting이 아니라 evidence-handling control입니다.

Raw secret은 artifact, connector manifest field, projection, exported bundle text, prompt context로 저장하면 안 됩니다. Secret-related evidence가 필요하면 관련 owner path가 허용하는 display-safe secret handle, redacted artifact, omission note, operator note를 사용합니다.

### Artifact path와 integrity validation

Artifact input은 registration이 path boundary, task/run ownership, artifact kind, size, hash, content type, redaction 또는 omission state, retention/availability fact를 검증하기 전까지 untrusted입니다. Path validation은 staged path, traversal, symlink surprise, off-profile location이 실수로 trusted evidence가 되지 않게 해야 합니다.

Artifact hash mismatch는 security 및 evidence-integrity finding입니다. Markdown을 편집하거나 byte를 직접 복사해 repair하지 않습니다. Recovery 또는 replacement는 documented artifact registration과 recovery path를 통해야 합니다.

### Freshness, replay, stale context

Baseline과 state-version check는 Harness record/check가 이에 의존하기 전에 replay와 stale context를 드러내는 데 도움을 줍니다. Old 민감 동작 permission 또는 later Approval record, old status text, old projection, old evaluator bundle, chat memory는 현재 쓰기 확인이나 현재 work close를 support할 수 없습니다. Current path가 이에 의존한다면 owner path를 통해 refresh, reconcile, supersede, replace해야 합니다.

Expected state version, idempotency, baseline compatibility, approval expiry, projection freshness, connector profile freshness는 서로 다른 control입니다. 이 문서는 그 threat-model 이유를 이름 붙입니다. Exact field와 behavior는 API, kernel, storage, projection, connector, operations owner에 남습니다.

### Authority가 unavailable이면 fail closed

State-changing, write-capable, sensitive, verification, QA, 작업 수락과 잔여 위험, close-relevant action에 필요한 Harness record/check path를 사용할 수 없으면 chat, stale projection text, generated file, cached context, operator prose에서 계속하지 말고 fail, hold, capability insufficiency report로 처리해야 합니다.

MCP unavailability에 대해서 operations와 connector는 기존 diagnostic distinction인 `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`을 사용하고, API-visible failure는 해당하는 경우 API-owned `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT` path를 사용합니다.

### 정직한 guarantee display

Security wording은 입증된 control과 일치해야 합니다.

| Guarantee | Honest security meaning |
|---|---|
| `cooperative` | Agent나 tool이 문서화된 절차를 따를 때 동작하는 협력형 확인입니다. 강한 보안 경계나 실행 전 차단이 아니라 instruction-following behavior입니다. |
| `detective` | Harness가 action 뒤 또는 관찰 가능해진 뒤 mismatch나 record inconsistency를 감지, 기록, 보고할 수 있는 사후 확인입니다. 이는 탐지와 보고이지 예방이 아닙니다. |
| `preventive` | Concrete hook, wrapper, permission layer, policy engine, sidecar 또는 equivalent가 covered operation을 실행 전에 사전 차단하며, exact path에 대한 fixture 증명이 있습니다. |
| `isolated` | 주장하는 내용에 맞는 실제 문서화된 separation boundary 뒤에서 work 또는 verification이 실행되는 격리 수준입니다. Worktree 또는 fresh evaluator bundle은 scope, freshness, blast-radius 분리를 제공할 수 있지만, profile이 exact isolation mechanism을 증명하지 않는 한 자동으로 OS sandbox 격리, 권한 경계, 변조 불가능한 보안 경계가 되지는 않습니다. Isolation만으로 민감 동작 승인, verification, 작업 수락, 잔여 위험 수용, close, assurance upgrade가 생기지 않습니다. |

Guard, freeze, careful-mode, recipe name, product name, surface name, friendly mode label은 guarantee를 올려 주지 않습니다. High-risk work는 실제 사용하는 control을 보여야 하며, preventive 또는 isolated control이 필요한 경우 cooperative-only claim에 의존하면 안 됩니다.

## Exact contract owner 지도

| Threat-model concept | Exact contract owner |
|---|---|
| MCP tool envelope와 `ToolError` shape | [API Schema Core](api/schema-core.md#common-response) |
| Public error, idempotency, replay, expected state version | [API Errors](api/errors.md) |
| Kernel state transition, gate, Approval, `prepare_write`, Write Authorization, acceptance, residual risk, close | [커널 참조](kernel.md) |
| `state.sqlite`, `task_events`, artifact storage row, DDL, enum hardening, hash, storage layout | [Storage와 DDL](storage-and-ddl.md) |
| Guarantee-level 의미와 honest display rule | 이 문서의 [정직한 guarantee display](#정직한-guarantee-display) |
| Runtime space, Core transaction ordering, artifact/projection architecture placement | [런타임 아키텍처 참조](runtime-architecture.md) |
| Connector capability profile, generated manifest, context push/pull, fallback display | [Agent 통합 참조](agent-integration.md) |
| 단계별 operator diagnostic, severity baseline, `doctor`, `serve mcp`, artifact check, recover, reconcile | [운영과 Conformance 참조](operations-and-conformance.md) |
| Core fixture mechanics: 정확한 fixture body, runner 동작, assertion semantics, fixture profile, suite metadata boundary, 축소된 Kernel Smoke 작성 순서 | [Conformance Fixtures 참조](conformance-fixtures.md) |
| 향후 상세 scenario 후보, 향후 fixture example, 단계별 fixture coverage map, fixture suite family summary, catalog-only future candidate | [향후 Fixture Catalog](future-fixture-catalog.md) |
| Projection freshness, managed block, reconcile behavior, template ownership | [문서 Projection 참조](document-projection.md)와 [Template 참조](templates/README.md) |
