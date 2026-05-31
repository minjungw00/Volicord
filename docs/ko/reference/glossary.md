# 용어집 참조

## 이 문서로 할 수 있는 일

다른 문서를 읽다가 하네스의 공식 용어, 대소문자, record name, 서로 대체할 수 없는 경계를 확인할 때 이 용어집을 사용합니다.

이 문서는 참조 문서입니다. 문서 수락과 별도의 구현 계획 준비 결정 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 실행 목표는 코어 권한 조각(v0.1 Core Authority Slice)이며, 커널 스모크(Kernel Smoke)는 이 조각을 위한 좁은 conformance authoring profile입니다. 첫 제품 MVP 목표는 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)입니다. 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)은 agency assurance, operations, handoff behavior를 단단하게 만드는 단계이며, v1+ Expansion은 담당 문서가 승격하고 증명하기 전까지 로드맵 범위에 남습니다.

## 이런 때 읽기

하네스 용어를 확인하거나, 권한 경로를 섞지 않도록 점검하거나, 정확한 동작을 담당하는 Reference 문서를 찾을 때 읽습니다.

## 읽기 전에

하네스 개념을 처음 이해하려면 Learn 경로를 사용합니다. 정확한 behavior가 필요하면 아래 owner link나 개별 정의 안의 link를 따라갑니다.

## 핵심 생각

용어집은 찾아보기 도구이자 담당 문서 지도입니다. 공개 용어, 내부 구현 용어, 대소문자, record name, 짧은 non-substitution reminder를 일관되게 유지하지만, 담당 Reference 문서를 대체하지는 않습니다.

## 참조 범위

이 용어집은 공식 용어 표현, 대소문자 안내, record-name 방향 잡기, 담당 문서 연결을 담당합니다. Kernel behavior, public MCP schema, storage DDL, projection rule, template body, connector capability profile, conformance fixture semantics는 담당하지 않습니다.

## 공개 용어

사용자용 문서, 프롬프트, 상태 요약에서는 아래 표현을 먼저 씁니다. 사용자가 record name을 배우지 않아도 하네스를 쓸 수 있도록 일부러 쉬운 말로 둡니다.

| 공개 용어 | 쉬운 뜻 |
|---|---|
| 작업 | 사용자가 끝내거나, 답을 얻거나, 조사하거나, 결정하고 싶은 일. |
| 범위 | 무엇이 바뀔 수 있고, 무엇은 범위 밖이며, 에이전트가 어디에서 멈춰야 하는지. |
| 판단 / 사용자 결정 | 사용자가 소유하는 선택입니다. 사용자에게 보이는 표시는 구체적인 유형을 이름 붙여야 합니다. 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, 범위/자율성 판단, 민감 동작 승인, QA 면제 판단, 검증 면제 판단, 작업 수락, 잔여 위험 수용을 구분합니다. |
| 근거 | 작업에 대한 주장을 뒷받침하는 오래 남는 자료. |
| 닫기 준비 상태 | 작업을 끝내거나 닫기 전에 아직 확인하거나 처리해야 하는 것. |
| 위험 | 계속 보여야 하는 알려진 불확실성, 한계, 생략된 확인, 절충, 가능한 영향. |

사용자용 문서는 쉬운 개념을 먼저 설명해야 합니다. 정확한 하네스 라벨은 경계, 막힘, 출처 참조, Reference 링크를 설명하는 데 도움이 될 때만 괄호로 덧붙입니다.

## 내부 구현 용어

아래 용어는 reference, API, schema, record, 상태 참조에서 쓰는 구현 라벨입니다. 사용자가 프롬프트에서 이 용어를 쓸 필요는 없습니다. 에이전트가 평소 말로 들어온 요청을 알맞은 하네스 절차로 바꿔야 합니다.

| 내부 용어 | 쉬운 설명 |
|---|---|
| Change Unit | 제품 파일 쓰기의 경계를 정하는 작업 범위입니다. 무엇이 바뀔 수 있는지 말하지만 그 자체로 쓰기를 허가하지는 않습니다. |
| Decision Packet | 진행, 쓰기, 면제, 작업 수락, 위험 처리, 닫기를 막는 특정 사용자 소유 결정을 기록하는 경로입니다. |
| Write Authorization | 범위와 필요한 확인 뒤에 특정 제품 파일 쓰기 시도 하나를 지금 진행해도 된다는 하네스 결과입니다. |
| Evidence Manifest | 완료 조건이나 수용 기준이 어떤 근거 참조로 뒷받침되는지 연결하는 기록입니다. |
| Projection | 하네스 상태에서 만든 읽기용 요약입니다. 상태를 보여 주지만 상태를 대체하지 않습니다. |
| Autonomy Boundary | 활성 범위 안에서 에이전트가 다시 묻지 않고 판단해도 되는 선택의 경계입니다. |
| `task_events` | 작업 상태 변화를 남기는 내부 event log table입니다. 사용자용 어휘가 아니라 reference/schema 용어입니다. |

## 담당 문서 지도

| 용어 묶음 | 담당 참조 문서 |
|---|---|
| Task, Change Unit, gate(관문), close, 민감 동작 승인, 작업 수락, verification, QA, 잔여 위험, write authority / 쓰기 허가 기록 | [Kernel Reference](kernel.md) |
| MCP resource, MCP tool, public schema, error, `ValidatorResult`, `ProjectionKind` | [MCP API와 스키마](mcp-api-and-schemas.md) |
| SQLite record, artifact layout, enum hardening, `tree_hash`, `request_hash` storage use | [Storage와 DDL](storage-and-ddl.md) |
| 읽기용 요약 / Projection, managed block, projection freshness, Markdown 보고서, template body | [문서 Projection 참조](document-projection.md); [Template 참조](templates/README.md) |
| Discovery와 Shared Design, design quality, stewardship, Feedback Loop finding routing, context hygiene, severity composition, policy contract | [설계 품질 정책](design-quality-policies.md) |
| Surface capability, guarantee display, connector behavior | [Agent 통합 참조](agent-integration.md) |
| Security asset, trust boundary, threat category, high-risk control expectation | [보안 위협 모델 참조](security-threat-model.md) |
| Operator procedure, conformance run overview, docs-maintenance 보고 | [운영과 Conformance 참조](operations-and-conformance.md) |
| Conformance fixture body shape, assertion semantics, suite catalog, example | [Conformance Fixtures 참조](conformance-fixtures.md) |

## 공식 용어

### Agency Conformance

하네스 behavior, projections, validators, close decisions가 사용자의 Strategic Agency를 얼마나 보존하는지를 나타내는 정도입니다. 작업 여정을 따라갈 수 있는지, 사용자 소유 판단이 명시적인지, Autonomy Boundary가 지켜지는지, 차단하는 사용자 소유 판단에 결정 패킷이 있는지, 작업 수락 전에 잔여 위험이 보이는지 확인합니다.

### Acceptance

한국어 기준 표현: 작업 수락.

Evidence, verification, 수동 QA 상태, 닫기에 영향을 주는 잔여 위험이 보였거나 없다고 확인된 뒤, 작업 결과가 받아들일 만하다는 사용자의 최종 판단입니다. Required 작업 수락는 결정 패킷 user decision, `task_gates.acceptance_gate`, `state.sqlite.task_events`를 포함하는 kernel 작업 수락 경로를 통해 기록됩니다. 작업 수락은 민감 동작 승인, assurance, verification, 수동 QA, evidence sufficiency, 면제 판단, 잔여 위험 수용과 구분됩니다. 추가 write를 허가하거나, 알려진 위험을 그 자체로 수용하거나, 잔여 위험을 지우거나, 빠진 check를 나중에 충족된 것으로 만들지 않습니다.

### Acceptance Gate

Required 작업 수락을 위한 kernel gate입니다. Value set과 compatibility meaning은 [Acceptance Gate](kernel.md#acceptance-gate)가 담당합니다. 작업 수락은 QA나 verification을 대신할 수 없습니다.

Current reference model에서 required 작업 수락은 결정 패킷 user decision, `task_gates.acceptance_gate`, `state.sqlite.task_events`를 통해 기록됩니다. 별도의 acceptance record 또는 table은 없습니다.

### Approval

한국어 기준 표현: 민감 동작 승인.

정의된 scope 안에서 특정 sensitive action 또는 경계가 정해진 민감 동작을 진행할 수 있도록 허용하는 제한된 사전 user authorization입니다. 민감 동작 승인은 paths, tools, commands 또는 command classes, network targets, secret scope, baseline, sensitive categories, expiry conditions에 묶입니다. 민감 동작 승인이 요청되면 Core는 approval-shaped 결정 패킷과 linked Approval record를 통해 user judgment를 capture합니다. Granted 민감 동작 승인이 있어도 쓰기 허가 기록이 생기려면 이후 compatible `prepare_write` result가 필요합니다. Approval은 민감 동작 승인일 뿐입니다. 막연한 동의, 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, correctness proof, 사용자 소유 제품/UX 판단이나 기술 구조 판단의 대체물이 아닙니다.

### Approval Gate

민감 동작 승인을 위한 kernel gate입니다. Sensitive categories가 있을 때만 required입니다. Granted 민감 동작 승인은 correctness를 증명하지 않고, 작업 수락을 뜻하지 않으며, 잔여 위험을 수용하거나 QA/검증 면제 판단을 기록하거나 사용자 소유 판단을 해소하거나 쓰기 허가 기록을 만들지 않습니다.

### Assumption Register

구현 계획 전에 agent가 사용하는 assumptions를 정리한 Discovery 또는 Shared Design support/projection 목록입니다. Source, confidence, owner, assumption이 틀릴 때 바뀌는 일을 이름 붙여야 합니다. 이는 권장 display/support 내용이지 standalone schema나 canonical record field list가 아닙니다. Assumption Register는 Discovery Brief, 안전한 다음 작업 후보, 작업 분할 제안, 또는 First Safe Change Unit Candidate를 구체화하는 데 도움을 주지만 사용자 동의, 민감 동작 승인, 작업 수락, 잔여 위험 수용, evidence, 닫기 준비 상태, scope authority, 쓰기 허가 기록은 아닙니다.

### Artifact

Evidence, recovery, audit에 사용하는 recorded output입니다. 기준 evidence-file 경계는 Raw Artifact를 참고합니다.

### Artifact Reference

한국어 기준 표현: 아티팩트 참조.

Artifact store에 등록된 raw artifact file을 가리키는 구조화된 포인터입니다. identity, kind, URI 또는 path, hash, size, content type, redaction state, task/run 관계를 포함합니다. `ArtifactRef`는 이 pointer shape의 정확한 schema name입니다. [Storage와 DDL](storage-and-ddl.md)에서 아티팩트 참조와 `artifact_links`는 Task-scoped입니다. `bundle`, `manifest`, `export_component` 같은 artifact kind는 file을 설명합니다. Owner link는 여전히 기존 상태 또는 Task-scoped projection record를 가리킵니다.

### Autonomy Boundary

추가 user judgment 없이 agent가 진행할 수 있는 사용자 소유 판단 경계를 기록하는 Change Unit semantics입니다. 쉽게 말해 active Change Unit 안에서 agent가 무엇을 혼자 판단해도 되는지 말합니다. 일상적인 구현 세부사항은 경계 안에 있을 수 있지만, public API 또는 module contract 변경, security 또는 privacy trade-off, UX 또는 제품 동작 trade-off, 중요한 dependency 또는 migration 방향, scope expansion, 잔여 위험 수용은 명시적인 user judgment가 필요하며 넓은 자율성에서 추론하면 안 됩니다.

이는 scope grant나 write authority가 아니며 active Change Unit 밖의 paths, tools, commands, network targets, secret access, sensitive categories를 허가하지 않습니다. 결정 패킷이 Autonomy Boundary update나 Change Unit update proposal을 허가할 수는 있지만, resulting write에는 여전히 compatible Change Unit scope와 sensitive categories에 필요한 민감 동작 승인이 필요합니다. 정확한 kernel behavior는 [Autonomy Boundary](kernel.md#autonomy-boundary)가 담당하고, policy placement는 [설계 품질 정책](design-quality-policies.md#autonomy-boundary-autonomy_boundary)이 담당합니다.

### Assurance

Recorded checks와 verification independence가 뒷받침하는 technical confidence level입니다.

```text
none | self_checked | detached_verified
```

Eval verdict만으로 assurance가 올라가지 않습니다. `detached_verified`에는 valid independence가 있는 passed verification과 same-session self-review violation 없음이 필요합니다.

### Baseline

Scope, approval drift, evidence freshness, verification validity를 판단하는 데 사용하는 captured repository state입니다.

### Blocker

한국어 기준 표현: 막힘.

진행, 쓰기, 닫기 또는 요청된 다음 단계를 해결하거나 유효하게 미루기 전까지 막는 구체적인 조건입니다. 사용자용 prose에서는 `막힘`을 쓸 수 있고, API/reference 문맥에서는 `blocker`를 유지하거나 `차단 조건(blocker)`으로 설명합니다. 정확한 field name, template key, enum-like value, schema name은 번역하지 않습니다. 유용한 막힘 표시는 무엇이 막혔는지, 다음 움직임을 누가 소유하는지, 가장 작은 해소 방법이 무엇인지, 관련 소유자 ref가 무엇인지 보여줍니다. 막힘은 일반 note, evidence 자체, 작업 수락, 잔여 위험 수용, 민감 동작 승인이 아닙니다.

### `tree_hash`

Ignored paths를 제외한 뒤 sorted NFC-normalized relative POSIX paths, file bytes, size, executable bit, symlink target handling을 사용해 계산하는 baseline file snapshot의 deterministic hash입니다. 세부 규칙은 [Storage와 DDL](storage-and-ddl.md)이 정의합니다.

### Capability Profile

연결된 agent 접점의 실제 capabilities를 declared and verified description으로 기록한 것입니다. target profile, support tier, guarantee level, supported features, risks, fallbacks, last verification time을 기록합니다. 하네스는 product name만으로 capability를 infer하지 않습니다.

### Capability Tier

연결된 접점에 대한 coarse integration level입니다.

```text
T0 Context | T1 Skill | T2 MCP | T3 Capture |
T4 Guard | T5 Isolation | T6 QA Capture
```

Capability tiers는 available integration support를 설명할 뿐 kernel gates가 아닙니다.

### Change Unit

Product writes의 범위를 정하는 scoped implementation unit입니다. Product write에는 intended paths, tools, commands, network targets, sensitive categories를 cover하는 active Change Unit이 필요하지만, Change Unit 자체가 write 권한을 부여하지는 않습니다. Core가 `prepare_write`와 applicable gates를 통해 write 허용 여부를 판단합니다.

### Close Reason

Task가 terminal close state에 도달한 기준 reason입니다.

```text
none | completed_verified | completed_self_checked |
completed_with_risk_accepted | cancelled | superseded
```

### Codebase Stewardship

제품 코드베이스를 durable asset으로 지키는 책임입니다. Domain language, module 경계, interface contracts, dependency direction, testability, maintainability, future-change risk를 살피는 일을 포함합니다.

### Common Tool Envelope

Public MCP tool calls가 공통으로 갖는 fields입니다. `request_id`, `idempotency_key`, `expected_state_version`, `project_id`, optional `task_id`, `surface_id`, optional `run_id`, `actor_kind`, `dry_run`을 포함합니다.

### Core-owned State

한국어 기준 표현: Core가 소유한 상태.

Harness Core가 커밋된 소유자 기록과 `state.sqlite.task_events`를 통해 소유하는 운영 상태입니다. Core가 소유한 상태는 gate, decision, 쓰기 허가 기록, 근거 상태, QA, verification, 작업 수락, 잔여 위험, 닫기의 기준입니다. Chat, 생성된 Markdown 읽기용 요약, connector 파일, 제품 저장소 문서는 소유자 경로를 통해 Core에 정보를 줄 수 있지만 Core가 소유한 상태를 대체하지 않습니다.

### Cooperative Guarantee

연결된 agent 접점에서 하네스 instructions와 MCP decisions를 따르는 협력형(cooperative) integration을 기대하는 guarantee level입니다. 하네스는 behavior를 guide할 수 있지만 hard pre-execution enforcement가 제공되지 않을 수 있습니다.

### Connector Manifest

Connector가 생성하거나 관리하는 path, MCP config snippet, managed block hash, capability/profile 최신성, capture/guard/isolation 설명 또는 mechanism, 수동 fallback 설명, drift 또는 stale status를 기록하는 generated manifest입니다. 생성되거나 관리되는 접점 file이 조용히 overwrite되지 않게 합니다. 전체 manifest contract는 [Agent 통합 참조](agent-integration.md#generated-manifest-기대사항)가 담당합니다.

### Context Hygiene

항상 주입되는 맥락을 짧고 최신으로 유지하는 policy입니다. Compact rule set은 10개 이하로 유지하고, current status 또는 현재 위치 맥락을 먼저 읽으며, Journey Card는 해당 projection/profile이 활성화되어 있고 최신일 때만 사용하고, 현재 context profile을 push하고, 더 큰 record는 pull-on-demand로 둡니다. Profile-relevant할 때 compact status card, 활성화된 최신 Journey Card ref, active 결정 패킷, Autonomy Boundary, Write Authority Summary, active scoped Change Unit, 수용 기준, approval status, evidence refs, residual-risk summary, gate summary, 읽기용 요약 최신성을 push할 수 있습니다. 오래된 PRD, design, log, module map, old projection, closed issue, Reference contract, oversized raw artifact는 현재 세션 시작, 요구사항 구체화/Discovery, 사용자 결정 요청, 쓰기 준비, 실행/근거, 닫기 준비 상태, 오류/복구 또는 verification bundle이 필요로 할 때만 pull합니다. Indexed, retrieved, remembered, summarized context는 ref나 source에 연결된 excerpt로 여기에 포함될 수 있습니다. 무엇을 살펴볼지 정하는 데 도움을 줄 뿐, 무엇이 허가되었는지, 검증되었는지, 결과가 수락되었는지, 요구사항이 면제되었는지, risk-accepted 되었는지, Task가 닫혔는지를 결정하지는 않습니다.

오래된 chat memory는 pull-only context입니다. 담당 소유자 경로가 관련 변화를 기록하지 않는 한 write를 허가하거나, gate를 충족하거나, Task를 close하거나, 결과를 수락하거나, QA 또는 verification을 면제하거나, 잔여 위험을 받아들이거나, 현재 상태를 대체하거나, stale projection을 고칠 수 없습니다.

### Context Index

Relevant projection, 아티팩트 참조, repo file, doc, note를 보여줄 수 있는 later read-only context provider입니다. 담당 문서로 승격되기 전까지는 v1+ Expansion 후보이자 권한 없는 retrieval only입니다. 승격 이후에도 해당 담당 문서가 명시적으로 바뀌지 않는 한 기존 권한 경로를 대체할 수 없습니다. Retrieved context는 살펴볼 source를 가리킬 수 있지만 write를 허가하거나, decision을 해소하거나, Approval을 부여하거나, evidence를 만들거나, verification을 수행하거나, 위험을 받아들이거나, gate를 충족하거나, Task를 close하면 안 됩니다. 정확한 future-feature 경계는 [Roadmap: Context Index](../roadmap.md#context-index)가 담당하고, connector 처리는 [Agent Integration](agent-integration.md#context-pushpull-principles)이 담당합니다.

### Decision Gate

진행, write, close 전에 필요한 차단하는 사용자 소유 판단을 나타내는 Task-level aggregate gate입니다. 기준 field는 `decision_gate`이며 value set과 recompute rule은 [Decision Gate](kernel.md#decision-gate)가 담당합니다. 관련 blocking 결정 패킷과 감지된 blockers에서 다시 계산되며 민감 동작 승인, verification, 수동 QA, 작업 수락을 대신하지 않습니다.

### Decision Kind

한국어 기준 표현: 결정 경로.

Decision Packet의 schema field인 `decision_kind`입니다. Lifecycle, payload branch, gate 의미, state-transition semantics를 제어합니다. 사용자에게 보이는 묶음인 `judgment_domain`과 구분됩니다. 표시는 쉬운 말로 결정 경로를 설명할 수 있지만 schema/API 문맥에서는 field name과 enum value를 정확히 유지합니다.

### Decision Profile

한국어 기준 표현: 결정 profile.

Decision Packet의 schema field인 `decision_profile`입니다. `minimal_decision`, `product_ux_tradeoff`, `architecture_tradeoff`, `approval_shaped`, `waiver`, `acceptance`, `residual_risk_acceptance`, `reconcile`, `mixed`처럼 decision record가 요구하는 깊이를 제어합니다. `decision_kind`, `judgment_domain`과 구분됩니다. Route는 lifecycle semantics를 제어하고, profile은 필요한 detail의 양을 제어하며, domain은 독자를 위해 judgment를 묶습니다.

### Decision Type Display

한국어 기준 표현: 결정 유형 표시.

대기 중인 사용자 소유 결정의 구체적인 종류를 보여주는 사용자용 label입니다. 결정 패킷의 route, `decision_profile`, `judgment_domain`, 관련 owner record에서 파생되는 표시이며 별도 schema field, gate, authority path가 아닙니다.

하나의 승인 checklist로 만들지 말고 다음 label을 구분해 사용합니다.

- 제품/UX 판단
- 기술 구조 판단
- 보안/개인정보 판단
- 범위/자율성 판단
- 민감 동작 승인
- QA 면제 판단
- 검증 면제 판단
- 작업 수락
- 잔여 위험 수용

민감 동작 승인은 이름 붙은 민감한 단계만 허용합니다. 작업 수락은 사용자의 결과 판단을 기록하며 알려진 잔여 위험을 그 자체로 수용하지 않습니다. 잔여 위험 수용은 받아들이는 위험을 이름 붙여야 하며 검증이나 QA가 통과했다는 뜻이 아닙니다.

### Decision Packet

한국어 기준 표현: 결정 패킷.

차단하는 사용자 소유 결정을 위한 기준 kernel state record입니다. 필요한 결정, `decision_kind`, `decision_profile`, `judgment_domain`, pending options 또는 chosen outcome, 영향받는 scope, supporting refs, owner, status, next action을 명시합니다. Full profile은 필요할 때 recommendation, uncertainty, detailed trade-offs, evidence, 잔여 위험, approval scope, waiver context, acceptance context, reconcile target도 보여줍니다. 결정 패킷 record ID는 `DEC-*`를 사용합니다. Record-level status는 [Decision Gate Aggregate Recompute](kernel.md#decision-gate-aggregate-recompute)와 public `DecisionPacket` schema가 담당하며, 관련 statuses가 Task-level `decision_gate`에 반영됩니다. Required Decision Packet visibility는 Task/status/next/judgment-context 및 decision-packet 접점을 통해 제공되며, standalone `DEC` Markdown 렌더링 결과는 기능이 켜져 있을 때만 optional projection 또는 제안용 접점입니다. Public API/interface 선택, architecture direction, domain-language conflict, module boundary change, 면제 판단, 작업 수락, 잔여 위험 선택은 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, 범위/자율성 판단, QA 면제 판단, 검증 면제 판단, 작업 수락, 잔여 위험 수용이 진행, write, close를 막거나 public commitment를 만들 때 이 경로를 사용합니다. 넓은 approval text는 특정 recorded route와 option에 답하지 않는 한 결정 패킷을 충족하지 않습니다.

`judgment_domain`은 결정 패킷에서 schema가 소유하는 판단 영역입니다. 값은 `product_ux`, `technical_architecture`, `security_privacy`, `qa_acceptance`, `residual_risk`, `scope_autonomy`, `mixed`이며, 표시는 Product / UX 또는 Security / privacy 같은 자연스러운 label로 바꿔 보여줄 수 있습니다. 영향을 받는 gate나 막힌 행동은 `affected_gates`와 관련 owner record로 별도 기록합니다. `judgment_domain`과 `decision_profile`은 사용자가 어떤 종류와 깊이의 판단을 요구받는지 이해하도록 돕지만 status, gate, owner record, validator input, close aggregation rule, authority path가 아닙니다. 여러 영역에 걸친 결정은 domain을 배타적으로 다루지 말고 부차적인 고려사항을 trade-offs, 영향받는 gates, risk, evidence, follow-up에 보여줘야 합니다. 표시는 결정 유형, 결정 제목, profile에 맞는 choice context, 사용자가 정확히 결정하는 것, 왜 지금 필요한지, 해당하는 경우 잔여 위험을 보이게 하되 `decision_kind`, 민감 동작 승인, 작업 수락, QA, 잔여 위험 수용, close, 쓰기 허가 기록의 owner contract를 바꾸면 안 됩니다.

### Decision Request

기준 결정 패킷을 가리킬 수 있는 optional routing, interaction, idempotency replay, compatibility handoff metadata입니다. Minimal 코어 권한 조각(v0.1 Core Authority Slice) 구현은 이를 생략할 수 있습니다. Decision Request는 decision authority가 아니며 그 자체로 `decision_gate`, approval, 작업 수락, 면제 판단, 잔여 위험 수용, close를 절대 충족하지 않습니다. Gate aggregation에는 linked compatible `decision_packet_id`를 통해서만 relevant합니다.

### Design Gate

Shared design, domain language, TDD trace, module/interface review 또는 기타 policy-pack requirements 같은 required design-quality preconditions를 위한 kernel gate입니다.

### Design-Quality Policy Pack

Design-quality policy contracts와 severity composition의 담당 문서입니다. Shared design, decision quality, autonomy 경계, domain language, vertical slice, feedback loop, TDD trace, module/interface review, Codebase Stewardship, 수동 QA, context hygiene를 다룹니다. Gates, validators, evidence, write blockers, close blockers에 영향을 주지만 kernel state machine을 재정의하지 않습니다.

### Detached Verification

한국어 기준 표현: 분리 검증.

Fresh session, fresh worktree, sandbox, manual evaluator bundle처럼 의미 있는 독립성 경계를 가로질러 수행되는 분리 검증입니다. 이는 verification independence와 stale-context control을 뒷받침하지만, 자동으로 OS 수준 보안 격리를 뜻하지는 않습니다. Same-session self-review는 분리 검증이 아니며, subagent context도 기본적으로 detached가 아닙니다.

### Discovery

구현 계획과 쓰기 권한 전에 에이전트가 요구사항을 구체화하는 workflow posture의 내부 이름입니다. Goal, user value, non-goals, 수용 기준, repo/docs/Harness state에서 에이전트가 확인할 수 있는 사실, assumptions, 사용자만 결정할 수 있는 판단, 제품/UX 판단 후보, 기술 구조 판단 후보, security/privacy 판단 후보, QA와 verification 기대 수준, 남은 불확실성, 안전한 다음 작업 후보 또는 작업 분할 제안을 분리합니다. Codebase와 현재 하네스 context가 답할 수 없는 결정만 사용자에게 묻고, decision area별로 여러 targeted question을 물을 수 있으며, 확인 가능한 사실과 사용자 소유 결정이 분리되고, 목표/비목표/수용 기준과 중요한 판단 후보가 충분히 분명하며, 해소되지 않은 판단을 숨기지 않고 안전한 다음 작업 또는 작업 분할을 제안할 수 있고, 남은 불확실성이 명시되면 잠시 멈추거나 진행할 수 있습니다. 요구사항 구체화 출력은 Shared Design, 결정 패킷 candidate, Change Unit shaping으로 라우팅합니다. `안전한 다음 작업 후보`와 `작업 분할 제안` 같은 표현은 proposal/support phrase이며 standalone schema field, canonical record type, gate value, projection kind, authority path가 아닙니다. 이 posture는 일반 승인, 민감 동작 승인, 쓰기 허가 기록, evidence, verification, QA, 작업 수락, 잔여 위험 수용, close, scope authority, 새 authority path가 아닙니다.

### Discovery Brief

구체화된 goal, user value, non-goals, 수용 기준, 확인 가능한 사실, Question Queue, Assumption Register, 분리된 사용자 소유 판단, 제품/UX, 기술 구조, security/privacy, QA와 verification 기대 수준, 남은 불확실성, 안전한 다음 작업 후보 또는 작업 분할 제안을 담은 compact Discovery 또는 Shared Design support/projection summary입니다. 제품 쓰기가 가까워졌다면 First Safe Change Unit Candidate를 포함할 수 있습니다. 이는 권장 display/support 내용이지 standalone schema나 canonical record field list가 아닙니다. Discovery Brief는 Shared Design, 결정 패킷 candidate, Change Unit shaping에 정보를 줄 수 있지만, 그 자체로 canonical scope를 만들거나, 결정을 해소하거나, write를 authorize하거나, evidence를 증명하거나, 잔여 위험 수용을 기록하거나, result를 작업 수락하거나, task를 close하지 않습니다.

### Detective Guarantee

하네스가 observation 후 violations를 감지하고 상태를 `blocked`, `stale`, `partial`, `failed`로 표시할 수 있는 탐지형(detective) guarantee level입니다.

### Direct

Scope와 result가 명확한 작고 low-risk인 changes를 위한 work mode입니다. Direct product writes에도 active scoped Change Unit이 필요합니다. Direct에는 trivial typo, single-sentence docs, obvious rename work를 위한 tiny direct profile이 포함됩니다. Tiny는 top-level mode가 아니며 사용자 소유 판단, 민감 동작 승인, security boundary, evidence, scope, 쓰기 허가 기록, 잔여 위험 표시, close rule을 우회하지 않습니다.

### Docs-Maintenance Conformance

Bilingual parity, links, owner 경계, stable catalogs, glossary terms, 기준 기록 표현, TODO usage, non-owner duplicate contracts의 drift를 감지하는 read-only documentation maintenance check profile입니다. Rule bodies는 [문서 작성 가이드](../maintain/authoring-guide.md#docs-maintenance-checks)가 담당하고, operator 보고와 entrypoint expectation은 [운영과 Conformance 참조](operations-and-conformance.md#docs-maintenance-프로필)가 담당합니다. Runtime conformance나 Task state 권한이 아닌 docs-only profile입니다.

### Domain Language

Product의 기준 vocabulary와 meanings입니다. 기준 기록은 `domain_terms`이며 Markdown domain-language documents는 projections이자 proposal 접점입니다. Term conflict는 policy validation을 통해 `design_gate`에 영향을 줄 수 있고, meaning 선택이 사용자 소유 제품/UX 판단이나 기술 구조 판단이면 결정 패킷으로 라우팅합니다.

### Domain Term

Product term, meaning, code representation, related terms, source, status, `"not this"` 같은 경계를 저장하는 `domain_terms`의 기준 structured record입니다. Public state refs는 `record_kind=domain_term`을 사용합니다.

### Evidence

Work에 대한 주장을 뒷받침하는 recorded support입니다. diffs, logs, tests, run summaries, screenshots, Eval records, 수동 QA records, 등록된 아티팩트 참조 등이 여기에 해당합니다. Evidence는 Evidence Manifest와 ArtifactRef 같은 owner records를 통해 특정 수용 기준, completion condition, 또는 close-relevant claim을 뒷받침합니다. 에이전트가 작업이 끝났다고 말하는 것 자체나 Markdown report prose만으로는 Evidence가 sufficient해지지 않습니다.

### Evidence Gate

Required evidence coverage를 위한 kernel gate입니다. Value set과 close meaning은 [Evidence Gate](kernel.md#evidence-gate)가 담당합니다.

### Evidence Manifest

Acceptance criteria 또는 completion conditions를 이를 뒷받침하는 evidence references에 매핑하는 state record입니다. Sufficiency는 artifact 개수나 report prose가 아니라, 그 criteria와 conditions가 current owner records와 `ArtifactRef` refs로 covered되는지에 달려 있습니다.

### Evidence Profile

Task shape에 충분한 evidence가 무엇인지 validators에 알려주는 named evidence sufficiency profile입니다. 예: `advisor`, `direct docs-only`, `direct code`, `work feature`, `UI/UX/copy work`, `sensitive work`, `verification-required work`. Tiny direct docs-only work는 Direct evidence expectation 아래에서 가장 작은 changed-path, patch-summary 또는 diff-ref, self-check support로 처리되며, 별도 authorization path가 아닙니다.

### Evidence Sufficiency

필수 수용 기준 또는 completion conditions가 Evidence Manifest와 관련 state records 및 아티팩트 참조로 뒷받침되는지에 대한 close-relevant judgment입니다. Criteria-based 판단이므로 각 required row에는 compatible current support가 필요합니다. Chat text나 Markdown 보고서 prose만으로 판단하지 않으며, baseline drift, changed files, approval drift, missing artifacts, relevant design record changes로 stale이 될 수 있습니다.

### Eval

Verification result record입니다. verdict, checks performed, evidence reviewed, independence qualifier, blockers, 아티팩트 참조를 포함합니다.

### Feedback Loop

Checks와 findings가 state, scope, design, evidence, follow-up work, close status로 되돌아가는 기준 support record이자 recorded path입니다. Inputs에는 tests, typecheck, lint, build, browser smoke, TDD red/green/refactor traces, 수동 QA, Eval findings, user decisions, operational findings, residual-risk decisions가 포함될 수 있습니다. Public refs는 `StateRecordRef.record_kind=feedback_loop`를 사용하며, public mutation은 `record_run`의 `FeedbackLoopUpdate` 또는 수동 QA execution link를 사용합니다. Feedback loops는 findings가 chat 속에서 사라지지 않게 하며, applicable한 경우 Evidence Manifest coverage, 결정 패킷, Change Unit update, 잔여 위험 record, 수동 QA 또는 Eval record, close blocker, follow-up Task/Change Unit record 같은 기존 소유자 경로로 연결합니다.

### Finding

Run, Eval, 수동 QA record, validator, review display, operator diagnostic, conformance check에서 나온 관찰된 issue, gap, risk, blocker, noteworthy result입니다. Finding은 standalone authority path가 아니며 chat이나 report prose에 남아 있는 것만으로 gate 또는 close에 영향을 주지 않습니다. Existing owner record 또는 structured result를 통해 라우팅될 때만 state-relevant해집니다. 예: Evidence Manifest gap, 결정 패킷 candidate 또는 record, Change Unit update, Feedback Loop 또는 TDD Trace update, 수동 QA 또는 Eval record, 잔여 위험 record, reconcile item, close blocker, follow-up Task/Change Unit record. 라우팅 계약은 [설계 품질 정책](design-quality-policies.md#finding-라우팅)과 [커널 참조](kernel.md#finding-라우팅)가 담당합니다.

### First Safe Change Unit Candidate

제품 쓰기가 가까워졌을 때 안전한 다음 작업 후보를 내부 Change Unit 모양으로 표현한 것입니다. 해소되지 않은 사용자 소유 판단을 숨기지 않고 포함되는 동작, 범위 밖 동작, 완료 조건, 알려진 sensitive area, stop condition을 이름 붙여야 합니다. Discovery 또는 Shared Design은 에이전트가 확인할 수 있는 사실과 사용자 소유 결정을 분리하고 안전한 다음 작업이 충분히 분명해진 뒤 이것을 만들 수 있지만, Discovery가 오직 이 candidate를 찾기 위해 존재하는 것은 아닙니다. 이는 권장 display/support 내용이지 standalone schema나 canonical record field list가 아닙니다. 이것은 candidate일 뿐입니다. 제품 쓰기 전에는 여전히 active Change Unit, compatible scope gate state, 이후 `prepare_write`가 필요합니다.

### Fixture Assertion Semantics

`expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, `expected_error`를 captured Core results와 어떻게 match하는지 정하는 conformance comparison rules입니다. [Conformance Fixtures 참조](conformance-fixtures.md#fixture-assertion-semantics)가 담당하며 fixture body 밖에 있고, prose-only matching으로 fixture를 pass시키는 것을 허용하지 않습니다.

### Fresh Session

Evaluator가 lead chat context를 이어받지 않고 task/evidence bundle에서 시작해 Evidence Manifest와 changed files를 검토하고 Eval을 기록하는 verification independence profile입니다.

### Fresh Worktree

Evaluator가 별도 worktree 또는 동등하게 independent repository state에서 baseline, changed paths, artifacts, Evidence Manifest를 확인하는 verification independence profile입니다. Fresh worktree는 scope, freshness, drift detection을 뒷받침할 수 있지만, 자동으로 OS sandbox 격리, 권한 경계, 변조 불가능한 보안 경계가 되지는 않습니다.

### Freeze

Current work 주변의 보류 또는 narrower posture를 request하는 user-facing safety control입니다. Freeze는 product write를 보류하거나, next action을 더 strict하게 만들거나, existing scope가 incompatible할 때 `prepare_write`가 block 또는 보류하게 만들 수 있습니다. Change Unit scope, allowed paths, Autonomy Boundary, AFK stop conditions, related owner records를 직접 변경하지 않습니다. Persistent owner-record change는 existing Core state-changing path, 결정 패킷 route, owner-record update path를 사용합니다. Freeze는 쓰기 허가 기록, approval, evidence, verification, QA, 작업 수락, 잔여 위험 수용, close, new authority tier를 만들지 않습니다.

### Gate

Task가 write, proceed, close할 수 있는지 control하는 기준 kernel field입니다. Gates는 state이며 display text가 아닙니다.

### Generated File

Connector, projector, operator tool이 produced한 repository file 또는 managed block입니다. Generated files가 기준 상태에서 drift될 수 있으면 manifest 또는 projection job으로 track해야 합니다.

### Guarantee Display

Status 또는 write decision에 대한 actual guarantee level을 user-facing 및 connector-facing으로 보여주는 display입니다. Enforcement가 cooperative 또는 detective인 경우 limitation notes를 포함합니다.

### Guarantee Level

연결된 접점 또는 runtime path에서 available한 honest enforcement strength입니다.

```text
cooperative | detective | preventive | isolated
```

Capability는 validator results, blocked reasons, display에 영향을 주지만 Approval, 쓰기 허가 기록, verification, QA, 작업 수락, 잔여 위험 수용, 닫기 준비 상태, kernel gate는 아닙니다. 정확한 level meanings는 [런타임 아키텍처](runtime-architecture.md#보장-수준)가 담당합니다.

### Guard

Connected profile의 actual enforcement 또는 detection layer를 적용하는 user-facing safety control입니다. Guard는 협력형(cooperative), 탐지형(detective), 예방형(preventive), 격리형(isolated)일 수 있습니다. Proven `T4` path가 operation을 cover하지 않는 한 이름만으로 실행 전 차단을 imply하지 않습니다.

### 강화된 로컬 기준 목표

영어 label: hardened local reference target.

사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) 이후 담당 문서가 정의한 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)을 완료해 도달하는 로컬 기준 동작 전체입니다. 별도 delivery stage도, 첫 구현 batch도, fixture profile이나 suite name도 아닙니다.

강화된 로컬 기준 목표는 코어 권한 조각(v0.1 Core Authority Slice), 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP), v1+ Expansion 경계를 대체하지 않습니다. Conformance는 Core Authority Slice fixtures, User-Facing Harness MVP fixtures, Agency Assurance Pack fixtures, Operations & Handoff Pack 또는 promoted-expansion fixtures라는 이름의 fixture profile로 증명합니다.

### Harness Core

한국어 기준 표현: 하네스 Core.

상태 전이, gate updates, validator interpretation, artifact registration, projection job 대기열 추가, close decisions를 담당하는 runtime component입니다.

### Harness Server

한국어 기준 표현: 하네스 서버.

에이전트 요청을 받고, Core를 통해 상태 변경을 검증하거나 기록하며, validator를 실행하고, 읽기용 요약을 만드는 로컬 하네스 프로그램과 도구 접점입니다. 이 문서 저장소의 향후 역할은 하네스 서버 소스 저장소입니다. 제품 저장소나 하네스 런타임 홈은 아닙니다. 여기서 서버/런타임 구현을 시작하려면 문서 수락과 별도의 구현 계획 준비 결정이 모두 필요합니다.

### Harness Runtime Home

한국어 표현: 하네스 런타임 홈.

`registry.sqlite`, per-project `project.yaml`, per-project `state.sqlite`, artifact directories를 포함하는 local runtime storage area입니다.

### Human-editable Area

사람이 notes, proposals, questions, corrections를 쓸 수 있는 Markdown area입니다. Input 접점이지 기준 상태가 아닙니다. Authority path는 `human-editable input -> reconcile_items -> accepted state event/record`입니다.

### Implementation Micro-Plan

작은 execution step 또는 slice, purpose, active Change Unit scope alignment 또는 likely paths, relevant한 경우 selected feedback loop 또는 TDD status, expected evidence, stop condition을 보여주는 managed `TASK` projection section입니다. Execution aid이지 기준 상태, `ProjectionKind`, scope authority, approval, 쓰기 허가 기록이 아닙니다. 이 text를 edit해도 accepted reconcile outcome 또는 Core state-changing action을 통하지 않으면 상태를 변경하지 않습니다.

### Isolated Guarantee

Work 또는 verification이 문서화된 separation boundary 뒤에서 실행되는 격리형(isolated) guarantee level입니다. Worktree 또는 fresh evaluator bundle은 scope, freshness, blast-radius 분리를 제공할 수 있지만, profile이 exact isolation mechanism을 증명하지 않는 한 자동으로 OS sandbox 격리, 권한 경계, 변조 불가능한 보안 경계가 되지는 않습니다. Isolation만으로 approval, verification, 작업 수락, 잔여 위험 수용, close, assurance upgrade가 생기지 않습니다.

### Journey Card

현재 Task 위치를 간결하게 보여주는 human-readable projection입니다. state, next action, scope, active scoped Change Unit, Autonomy Boundary, blockers, active 결정 패킷, Write Authority Summary, 수용 기준, 민감 동작 승인 status, evidence, verification, QA, 작업 수락, 잔여 위험, 읽기용 요약 최신성을 포함합니다. Journey Card는 display이며 기준 상태가 아니고, 오래된 chat memory가 아니라 current owner record에서 렌더링됩니다.

### Judgment Domain

한국어 기준 표현: 판단 영역.

Decision Packet의 schema field인 `judgment_domain`입니다. `product_ux`, `technical_architecture`, `security_privacy`, `qa_acceptance`, `residual_risk`, `scope_autonomy`, `mixed`처럼 사용자에게 보이는 판단 영역을 묶습니다. 사용자가 어떤 판단을 요청받는지 이해하도록 돕지만 전체 결정 유형은 아니며 gate, status, authority path, validator input, close aggregation rule, affected-gate relation, `decision_kind`의 대체물이 아닙니다. 표시에서는 해당 route가 대기 중이면 민감 동작 승인, QA 면제 판단, 검증 면제 판단, 작업 수락, 잔여 위험 수용을 계속 구분해야 합니다.

### Journey Spine

Task의 ordered work journey를 state에서 파생해 이어 주는 continuity model입니다. Chat memory가 아니라 Task, Change Unit, Run, 결정 패킷, Approval, Evidence Manifest, Eval, 수동 QA, 잔여 위험, `task_gates.acceptance_gate`, 작업 수락 결정 패킷 state, close events, 아티팩트 참조, `state.sqlite.task_events`에서 재구성됩니다. Journey Card와 Journey Spine Markdown views는 projections입니다.

### Journey Spine Entry

Existing state events나 owner records만으로 완전히 재구성하기 어려운 durable continuity annotations를 위한 기준 support record입니다. Journey Spine Entry records는 Journey Spine을 보완하지만 Task, Change Unit, Run, 결정 패킷, 잔여 위험, evidence, verification, QA, 작업 수락 gate/decision state, close state/events, artifact, event authority를 대체하지 않습니다.

### Interface Contract

Module 또는 external 경계의 public interface, inputs, outputs, errors, compatibility impact, callers, 경계 tests에 대한 기준 record입니다. 기준 기록은 `interface_contracts`입니다. Public state refs는 `record_kind=interface_contract`를 사용합니다. 이 record는 interface understanding을 문서화할 뿐이며 민감 동작 승인, 작업 수락, 잔여 위험 수용, 쓰기 허가 기록이 아닙니다. Public interface 또는 compatibility 선택에 사용자 소유 판단이 필요하면 기존 design-quality 및 결정 패킷 경로로 라우팅합니다.

### JSON `TEXT` Field

저장된 값이 JSON인 SQLite `TEXT` column입니다. `TEXT` type은 reference storage flexibility일 뿐입니다. Core는 commit 전에 API-owned 또는 storage-owned shape에 맞게 값을 검증해야 하며, malformed JSON 또는 schema-incompatible JSON은 invalid state입니다.

### Local Derived Metrics

`state.sqlite.task_events`, runs, validator results, projection jobs, reconcile items 같은 local record에서 파생되는 later diagnostic-only metric입니다. Owner 문서로 승격되기 전까지 metric 표시는 rate, count, duration, guard-trigger summary를 읽기 전용 진단으로만 보여줄 수 있습니다. 정확한 권한 없음 경계는 [Roadmap: Local Derived Metrics](../roadmap.md#local-derived-metrics)가 담당합니다.

### 수동 QA

한국어 기준 표현: 수동 QA.

수동 QA는 UX, workflow, copy, visual output, accessibility, product fit 같은 experiential product quality에 대한 human inspection입니다. Required인 경우 수동 QA는 수동 QA record 또는 valid QA 면제 판단 path로 기록됩니다. Browser smoke, screenshots, Browser QA artifacts, tests, verifier notes는 context를 뒷받침할 수 있지만 그 자체로 수동 QA judgment가 아닙니다. 정확한 gate behavior는 [QA Gate](kernel.md#qa-gate)가 담당하고, policy requirements는 [설계 품질 정책](design-quality-policies.md#수동-qa-manual_qa)이 담당합니다.

### Manual Bundle

Human 또는 separate evaluator에게 verification을 handoff하는 package입니다. task summary, 수용 기준, Change Unit scope, approval scope, diff/log/test artifacts, Evidence Manifest, known risks, Eval verdict를 기록하기에 충분한 context를 포함합니다.

### 수동 QA Record

한국어 기준 표현: 수동 QA 기록.

Record-level 수동 QA result입니다. performer, profile, result, artifact, finding, applicable한 경우 waiver reason, next action을 포함합니다. Result value set은 [QA Gate](kernel.md#qa-gate)와 [`harness.record_manual_qa`](mcp-api-and-schemas.md#harnessrecord_manual_qa)가 담당합니다. Pending required QA는 `qa_gate=pending`으로 표현하며 수동 QA record result가 아닙니다.

### `managed_hash`

`HARNESS:BEGIN`과 `HARNESS:END` marker lines를 제외한 projector-owned managed block body의 drift-detection hash입니다. 기준 상태가 아니며 Markdown 읽기용 요약을 authoritative하게 만들지 않습니다.

### Managed Block

하네스 markers로 delimit되고 projector가 state records와 아티팩트 참조에서 regenerate하는 Markdown block입니다. Managed block에 대한 direct edits는 drift 또는 reconcile candidates를 만들며 그 자체로 state가 되지 않습니다.

### MCP Resource

Current project, task, design, policy, status, bundle information을 위한 read-only MCP 접점입니다. Resources는 상태를 변경하지 않습니다.

### MCP Server Unavailable

`MCP_SERVER_UNAVAILABLE`은 tool call이 Core에 닿을 수 없는 diagnostic condition입니다. Authoritative Core response가 불가능하며, caller는 상태 변경을 주장하기 전에 diagnose 또는 reconnect해야 합니다. Stable public error code는 계속 `MCP_UNAVAILABLE`입니다.

### Surface MCP Unavailable

`SURFACE_MCP_UNAVAILABLE`은 Core 또는 operator가 연결된 접점에서 사용할 수 있는 MCP가 없거나, MCP configuration이 최신이 아니거나, required MCP tools를 호출할 수 없음을 관찰할 수 있는 diagnostic condition입니다. Product writes는 cooperative 접점에서는 instruction으로 보류되고 available한 stronger guard에서는 차단됩니다. Core responses는 `details.mcp_unavailable_kind`와 함께 `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT`를 사용할 수 있으며, 이 diagnostic label은 public `ErrorCode` value가 아닙니다.

### MCP Tool

Core에 상태를 검증, 기록, 전이, 닫기 처리하도록 요청하는 public MCP operation입니다. 상태 변경은 resource reads가 아니라 tools 또는 reconcile actions를 통해야 합니다.

### Markdown Report

State records와 아티팩트 참조에서 generated된 human-readable document입니다. Markdown 보고서는 기본적으로 raw artifact가 아니며 기준 상태가 되지 않습니다.

### Natural-Language Consent

한국어 기준 표현: 자연어 동의.

"go ahead", "proceed", "looks good", "좋아", "진행해" 같은 사용자 발화는 active prompt가 정확한 decision route, option, scope, affected gates, consequence, 그리고 아직 승인되지 않은 항목을 모호하지 않게 보여줄 때만 대기 중인 질문에 대한 답으로 사용할 수 있습니다. 자연어 동의는 독립 authority path가 아닙니다. 모호한 동의는 민감 동작 승인, 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, 쓰기 허가 기록으로 넓혀 해석하지 말고 다시 확인해야 합니다.

### Module Map

Product의 modules, responsibilities, public interfaces, dependency direction, internal complexity, test 경계, owner 결정, watchpoints를 정리한 map입니다. 기준 기록은 `module_map_items`입니다. Module boundary update는 공유된 technical understanding을 기록할 뿐이며 write를 승인하거나 risk를 받아들이지 않습니다. Boundary change가 product commitment, caller obligation, architecture direction을 바꾸고 사용자 소유 판단이 필요하면 design-quality policy와 결정 패킷 path로 라우팅합니다.

### Module Map Item

Module role, public interface, dependencies, internal complexity, test 경계, owner 결정, watchpoints를 저장하는 `module_map_items`의 기준 structured record입니다. Public state refs는 `record_kind=module_map_item`을 사용합니다.

### Policy Contract

Design-quality policies가 사용하는 standard form입니다. `name`, `applies_when`, `default_requirement`, `allowed_waiver`, `required_record`, `validator`, `evidence`, `close_impact`를 포함합니다.

### Preventive Guarantee

하네스 또는 connector가 violating action을 execution 전에 block할 수 있는 예방형(preventive) guarantee level입니다.

### Product Repository

한국어 기준 표현: 제품 저장소.

사용자의 실제 제품 작업 공간입니다. 소스 코드, 테스트, 제품 문서, 제품 저장소에 쓰이는 읽기용 하네스 보고서가 여기에 속합니다. 제품 저장소는 제품 내용의 기준 위치로 남습니다. 하네스 런타임 홈이 아니며, 제품 파일은 기존 Core, artifact-registration, reconcile, owner-record path가 관련 Harness fact를 기록할 때만 하네스 운영 사실이 됩니다.

### Projection

한국어 기준 표현: 읽기용 요약.

Projection은 Core 상태 record와 아티팩트 참조에서 생성된 읽기용 요약입니다. 읽고 판단하는 데 유용하지만 기준 상태를 덮어쓰거나 대체할 수 없습니다.

### ProjectionKind

Projection job과 template kind를 나타내는 API enum입니다. Support class, values, extension rules는 [Shared schemas](mcp-api-and-schemas.md#shared-schemas)가 담당합니다. Support class label은 코어 권한 조각(v0.1 Core Authority Slice) run obligation이 아닙니다. v0.1에는 소유자 경로가 이미 만든 freshness/read fact를 보존하는 것 외의 projection rendering exit requirement가 없습니다. 어떤 ProjectionKind도 Projection을 기준 상태로 만들지 않습니다.

### Projection Freshness

Projection과 source record, managed hash, 아티팩트 참조, projection job state 사이의 관계입니다. Value set은 [MCP API와 스키마](mcp-api-and-schemas.md)와 [문서 Projection 참조](document-projection.md)가 담당합니다.

### Projection Job

Committed state records와 아티팩트 참조에서 Markdown 읽기용 요약을 렌더링하도록 projector에 요청하는 지속 outbox record입니다. `record_kind=projection` identity는 `projection_jobs.projection_job_id`입니다. Project-level projection jobs는 현재 Task-scoped artifact DDL에서 그 자체로 project-scoped artifact links를 만들지 않습니다.

### Question Queue

Open questions를 blocking, useful-but-not-blocking, codebase-answerable로 분류한 Discovery 또는 Shared Design support/projection 목록입니다. 이는 권장 display/support 내용이지 standalone schema나 canonical record field list가 아닙니다. Blocking question은 사용자 소유 판단이 필요할 때 결정 패킷 candidate로 라우팅될 수 있습니다. Useful-but-not-blocking question은 남겨 두거나, defer하거나, follow-up work로 바꿀 수 있습니다. Codebase-answerable question은 사용자에게 묻지 말고 current repo, docs, 하네스 state, 출처 참조에서 답해야 합니다. Queue는 결정 패킷, gate, 민감 동작 승인, evidence, 작업 수락, close, 쓰기 허가 기록이 아닙니다.

### QA Gate

Required 수동 QA를 위한 기준 kernel gate입니다. `manual_qa_record.result`는 record-level이고, `qa_gate`는 close-relevant aggregate state입니다.

`qa_gate=pending`은 required QA가 충족 기록을 아직 만들지 못했거나 latest relevant record가 policy를 충족하지 못한다는 뜻입니다.

### Raw Artifact

Diff, log, bundle, screenshot, checkpoint, manifest file처럼 artifact store에 있는 durable evidence file입니다. Raw artifacts는 state records와 Markdown 보고서와 구분됩니다.

### Reconcile

Human-editable input 또는 projection drift를 accepted state change, rejected proposal, note, decision, deferred item으로 바꾸는 process입니다.

### Reconcile Item

Reconcile decision이 accept, reject, convert, defer하기 전에 human-editable input 또는 projection drift에서 생성되는 기준 candidate record입니다.

### Reference Surface

코어 권한 조각(v0.1 Core Authority Slice)이 target하는 단일 agent 접점입니다. Kernel과 커넥터 계약을 demonstrate하기 위한 범위이며 broad connector-surface support를 뜻하지 않습니다.

### Recommended Playbook

Current state와 policy/playbook context에서 계산되는 non-authoritative status/next display guidance입니다. Current stage에 맞는 procedure를 제안하며 review, TDD, QA, guard check, release handoff, browser-QA candidacy 같은 항목을 제안할 수 있습니다. `playbook_id`는 stable display/routing string identifier이지 Core-owned closed enum이나 DDL-backed value set이 아닙니다. 기준 kernel record가 아니고 자체 DDL table, task event, projection job이 없으며 write 권한을 만들거나, gate를 충족하거나, 결과를 수락하거나, 잔여 위험을 받아들이거나, Task를 close하지 않습니다. 사용자 소유 판단이나 필요한 동작은 결정 패킷 path 또는 다른 기존 Core/MCP mutation path로 라우팅합니다.

### Release Handoff

External PR, review, deployment, rollback, monitoring process를 위한 release readiness를 요약하는 optional 보고서/export profile입니다. Close readiness, blocker, evidence ref, verification ref, 수동 QA ref, residual-risk ref, changed file, 읽기용 요약 최신성, redaction note, suggested checklist item을 포함합니다. 정확한 보고서/export 권한 경계는 [Operations And Conformance](operations-and-conformance.md#release-handoff-export-profile)가 담당합니다.

### Role Lens

사용자가 product, engineering, design, security, QA, release-handoff 검토 관점을 요청할 수 있게 하는 non-authoritative skill 또는 playbook 접점입니다. Role Lens output은 `RecommendedPlaybook`, `DecisionPacketCandidate`, validator/check route, evidence, Eval 또는 verification, 수동 QA, Approval, residual-risk, Change Unit update, release handoff route 같은 existing route를 재사용합니다. 기존 Core/MCP path가 underlying action을 기록하기 전까지 read-only guidance이므로, 그 자체로 state를 mutate하거나, write를 허가하거나, gate를 충족하거나, 결과를 수락하거나, 잔여 위험을 받아들이거나, Task를 close하거나, assurance를 올리지 않습니다. 정확한 권한 없음 경계는 [Agent Integration](agent-integration.md#role-lens-동작)이 담당합니다.

### Report Projection

Task 보고서, approval 보고서, run summary, evidence manifest 보고서, Eval 보고서, direct-result 보고서처럼 state records와 아티팩트 참조에서 생성되는 Markdown 보고서입니다.

이름 있는 보고서 projection kind 값은 state records와 아티팩트 참조에서 생성되는 projections입니다. State authority는 Core records에 남고, evidence-file authority는 registered artifact files에 남습니다. 정확한 projection rule은 [문서 Projection 참조](document-projection.md)가 담당하며, 전체 rendered body는 [Template 참조](templates/README.md)가 담당합니다.

### Review Stages

Spec Compliance Review와 Code Quality / Stewardship Review를 분리하는 managed display/procedure split입니다. Spec Compliance Review는 현재 하네스 권한 안에서 requested work가 complete한지 묻습니다. Code Quality / Stewardship Review는 implementation이 codebase 안에서 유지보수하기 좋은지 묻습니다. Review Stages는 findings를 validator results, evidence gaps, 결정 패킷 candidates, Eval 또는 verification 필요, 수동 QA 필요, 민감 동작 승인 필요, 잔여 위험 candidates, Change Unit update recommendations, close blockers로 라우팅할 수 있습니다. 기준 기록, `ProjectionKind` value, 민감 동작 승인, evidence, verification, QA, 작업 수락, 잔여 위험 수용, close, 쓰기 허가 기록은 아닙니다. 정확한 표시 전용 경계는 [Design Quality Policies](design-quality-policies.md#two-stage-review-display)가 담당합니다. Same-session Review Stages는 `assurance_level=detached_verified`를 만들지 않습니다.

### `request_hash`

`tool_name`, schema-normalized request body, `request_id`와 `idempotency_key`를 제외한 envelope fields를 포함하는 정규화된 UTF-8 JSON에서 계산하는 tool request idempotency hash입니다.

### Residual Risk

한국어 기준 표현: 잔여 위험.

잔여 위험은 Evidence, verification, QA, 작업 수락 이후에도 남는 알려진 불확실성, trade-off, limitation, unchecked condition을 위한 기준 close-relevant support record입니다. 출처 참조, 영향받는 scope, 해당하는 경우 관련 결정 패킷, 표시 상태, 받아들인 위험, 후속 작업 필요 여부, 닫기 영향을 기록합니다. 닫기에 영향을 주는 것으로 알려진 잔여 위험은 성공적인 작업 수락 또는 close 전에 보여야 하며, known close-relevant risk가 없으면 `ResidualRiskSummary.status=none`으로 확인되어야 합니다. 잔여 위험 수용은 사용자가 이름 붙은 알려진 잔여 위험을 명시적으로 받아들이는 판단입니다. 이는 결과가 달리 검증, 작업 수락, 민감 동작 승인, 면제 판단되었다는 뜻이 아닙니다. 현재 Reference 모델에서 받아들인 위험은 잔여 위험 record 위의 메타데이터/상태이며 별도의 `accepted_risk` state record가 아닙니다.

### Risk Accepted Close

사용자가 visible close-relevant 잔여 위험을 받아들이는 successful close입니다. Verification risk가 waived된 경우도 포함합니다. `close_reason=completed_with_risk_accepted`를 사용하고, accepted 잔여 위험 refs가 필요하며, `assurance_level=detached_verified`로 표시하면 안 됩니다. User-facing summary는 이를 일반 `completed_verified` 또는 `completed_self_checked` close와 구분해야 합니다.

### Run

Agent, evaluator, operator, 기타 actor가 Task와 optional Change Unit에 대해 수행하는 execution attempt입니다. Run은 baseline, 접점, observed changes, commands, artifacts, summary를 기록합니다. Rejected pre-commit `record_run` request는 Run이 아니며 fabricated Run ID를 받으면 안 됩니다. Audit 또는 violation attempt는 Core가 deliberate하게 commit할 때만 Run이 됩니다.

### Scope Gate

Product writes가 active scoped Change Unit으로 covered되어야 함을 요구하는 kernel gate입니다. Approval이 required가 아니어도 write-capable `direct`와 `work` modes에는 scope가 required입니다. Scope Gate는 민감 동작 승인을 부여하거나, 사용자 소유 판단을 해소하거나, 쓰기 허가 기록을 만들지 않습니다. Exact values와 compatibility는 [Scope Gate](kernel.md#scope-gate)가 담당합니다.

### Severity Composition

여러 applicable task-shape default, policy contract, validator finding을 merge하는 policy-owned rule입니다. Same concern은 전체 Task나 단순히 같은 validator ID가 아니라 같은 policy-relevant target입니다. 이 rule은 모든 finding을 보이게 유지하고, 서로 다른 affected gate 또는 blocker target의 impact를 보존하며, 같은 concern에서 경쟁하는 impacts에만 가장 강한 applicable impact를 사용합니다. Validators, gates, write blockers, close blockers, waivers, 결정 패킷 needs에 영향을 주지만 public primary `ToolError` 선택은 API가 소유합니다. 정확한 policy behavior는 [Severity composition rule](design-quality-policies.md#severity-composition-rule)이 담당합니다.

### Shared Design

Implementation이 plan으로 굳어지기 전에 Task에 대해 최소한으로 기록한 shared understanding입니다. goal, user value, scope, non-goals, 수용 기준, 확인 가능한 사실, assumptions, decisions, rejected options, 남은 불확실성, domain/module/interface impact, QA와 verification 기대 수준, 안전한 다음 작업을 포함합니다. Discovery Brief, Question Queue, Assumption Register, 안전한 다음 작업 후보 또는 작업 분할 제안, First Safe Change Unit Candidate가 Shared Design에 입력될 수 있습니다. Shared Design은 shaping과 `design_gate` readiness를 도울 수 있지만 민감 동작 승인, 작업 수락, 잔여 위험 수용, QA 판단, evidence, 닫기 준비 상태, 쓰기 허가 기록은 아닙니다. Shared Design의 Markdown 렌더링 결과는 projections이자 제안용 접점입니다. 정확한 policy requirements는 [설계 품질 정책](design-quality-policies.md#shared-design-shared_design)이 담당합니다.

### Source-of-truth

어떤 fact에 대한 기준 정보입니다. 하네스에서 운영 상태의 기준 기록은 `state.sqlite` current records와 `state.sqlite.task_events`이고, raw evidence files의 기준 위치는 artifact store이며, Markdown documents는 projections입니다. Product repository files는 product content의 source로 남습니다. Existing Core, reconcile, artifact-registration, owner-record path가 관련 하네스 fact를 기록하기 전까지는 하네스 operational state가 되지 않습니다.

### `state.sqlite.task_events`

`state.sqlite` 안의 추가 전용 event history table입니다. Reference event storage는 별도의 event store를 사용하지 않습니다. Deterministic order는 timestamp나 event ID가 아니라 `task_events.event_seq`입니다.

### Stable Event Catalog

Staged/reference conformance fixtures가 `expected_events`에서 검증할 수 있는 `task_events.event_type` names에 대한 kernel-owned compact list입니다. Stable event names를 prose examples, fixture shorthand, non-stable implementation-local detail 또는 audit events, validator IDs, Core check names, projection status shorthands, future extension events와 구분합니다.

### State Record

Kernel state 안의 기준 structured record입니다. Task, Change Unit, 결정 패킷, Journey Spine Entry, 잔여 위험, Run, Approval, 쓰기 허가 기록, Evidence Manifest, Eval, 수동 QA record, Artifact record, Shared Design record, Domain Term, Module Map Item, Interface Contract, Feedback Loop, TDD Trace, Reconcile Item 등이 있습니다.

### State Version

Core-resolved state scope를 위한 optimistic-concurrency clock입니다. 적용되는 경우 Core는 envelope, tool-specific input, 또는 active Task에서 primary Task를 찾습니다. `expected_state_version`, `ToolResponseBase.state_version`, `EventRef.state_version`, `task_events.state_version`은 하나의 global event-store sequence가 아니라 해당 영향받는 scope에 따라 해석됩니다.

### Project State Version

`project_state.state_version`에 저장되는 project-scoped state clock입니다. Core-resolved primary Task가 없는 project-scoped mutations는 `expected_state_version`을 이 값과 비교하고 resulting value를 primary response `state_version`으로 반환합니다.

### Task State Version

`tasks.state_version`에 저장되는 task-scoped state clock입니다. Task-scoped mutations는 `expected_state_version`을 Core-resolved primary Task의 값과 비교하고 resulting value를 primary response `state_version`으로 반환합니다.

### Strategic Agency

사용자가 작업 여정을 이해하고 goals, scope, design, trade-offs, Codebase Stewardship, QA, 작업 수락, 잔여 위험에 대해 판단하거나 보류할 수 있는 지속적인 권한입니다. 하네스는 chat 밖에서 state, decisions, evidence, blockers, remaining risk를 명시해 Strategic Agency를 보존합니다.

### Secret Handle

Credential, token, certificate, key, 기타 secret value 같은 민감한 material을 raw value 없이 가리키는 display-safe reference입니다. Secret handle은 raw secret을 artifact, connector manifest, projection, export, screenshot, log, summary, prompt context에 저장하지 않고 evidence 또는 approval scope를 뒷받침할 수 있습니다. Exact storage와 API behavior는 storage와 MCP/API owner에 남습니다.

### Security Threat Model

하네스 security asset, trust boundary, threat category, control expectation을 담당하는 reference owner입니다. Repo docs의 prompt injection, projection tampering, stale approval replay, out-of-scope write, MCP unavailable 상태에서의 state claim, evidence artifact를 통한 secret leakage, artifact hash mismatch, 악성 generated connector 파일, capability overclaiming, stale context poisoning 같은 risk를 설명합니다. Exact DDL, public API schema, kernel transition은 담당하지 않습니다.

### Surface Capability Check

연결된 agent 접점의 required 하네스 behavior 충족 가능성을 보고하는 validator입니다. Blocked reasons와 guarantee display에 영향을 주지만 kernel gate는 아닙니다.

### Surface Cookbook

접점별 connector notes, generated file details, profile examples를 담은 reference 문서입니다. Common integration rules는 cookbook이 아니라 agent integration document에 둡니다.

### Subagent Context

Subagent 또는 helper가 일부 inherited implementation context를 가지고 work를 review하는 verification independence profile입니다. 기본적으로 detached가 아니며, stricter profile metadata가 실제 독립성 경계를 입증할 때만 qualify될 수 있습니다.

### Task

Kernel이 track하는 user value unit입니다. mode, lifecycle phase, gates, result, close reason, assurance, current summary, decisions, evidence, projection status를 가집니다.

### Task Level

Task shape를 표시하고 routing하기 위한 label입니다. Tiny, Direct, Work, High-risk Work가 있습니다. Tiny는 `direct` 아래 profile입니다. Direct는 작고 low-risk인 code 또는 docs work입니다. Work는 feature, UX workflow, auth-facing behavior, schema, public API/interface, multi-file 또는 multi-step delivery를 다룹니다. High-risk Work는 auth, security, privacy, secrets, infra, 비슷하게 민감한 category를 다룹니다. Task Level은 새 kernel `mode` enum, gate, schema field, approval, 쓰기 허가 기록 source가 아닙니다.

### TDD Trace

Change Unit 또는 behavior slice에 대한 red, green, refactor evidence record 또는 policy가 허용하는 recorded non-TDD justification입니다. RED target 또는 plan은 intended failing check를 설명하고, RED evidence는 actual failing test artifact/log/result 또는 policy가 명시적으로 인정한 failing-check evidence를 뜻합니다. Required인 경우 normal path는 non-test implementation write 전에 RED evidence를 기록하고, implementation 후 GREEN evidence를 기록하며, relevant한 경우 refactor/check evidence를 기록한 뒤 trace를 Evidence Manifest coverage에 link합니다. TDD Trace는 Feedback Loop의 execution evidence가 될 수 있지만 기준 selected-loop record는 아닙니다. Waiver는 behavior를 증명할 alternate Feedback Loop로 돌아가는 ref를 가져야 합니다.

### Tiny Direct Profile

Typo, 문서 한 문장, obvious rename처럼 scope, result, 사용자 판단이 필요 없다는 경계가 즉시 분명한 Direct 하위 profile입니다. Interaction을 최소화하지만 scope가 넓어져도 여전히 low-risk이고 좁거나, Evidence Manifest coverage, 아티팩트 참조, link/render proof, 또는 tiny result note를 넘는 다른 evidence가 필요하면 일반 Direct로 상향해야 합니다. Product judgment, 기술 구조 판단, architecture choice, public interface/API impact, UX workflow, schema, sensitive category, multi-step delivery가 나타나면 Work로 라우팅해야 합니다.

### Trust Boundary

하네스 surface, file, caller, runtime space 사이의 분리입니다. 한쪽의 input은 소유자 경로 없이 다른 쪽의 authority로 취급하면 안 됩니다. 예를 들어 chat text, 제품 저장소 문서, projection, generated connector 파일, artifact bytes, MCP caller claim은 하네스에 정보를 줄 수 있지만, Core 또는 문서화된 다른 소유자 경로가 그 의미를 받아들이기 전까지 canonical operational state가 되지 않습니다. Trust-boundary map은 [보안 위협 모델 참조](security-threat-model.md)가 담당합니다.

### Verification

Result가 relevant criteria를 충족하는지 check하는 process입니다. Verification은 valid Eval path와 independence profile을 통해 기록될 때 assurance를 뒷받침할 수 있지만, same-session self-check는 분리 검증이 아닙니다. Verification은 approval, 수동 QA, 작업 수락, 잔여 위험 수용과 구분됩니다. 정확한 gate와 independence behavior는 [Verification Gate](kernel.md#verification-gate)와 [`harness.record_eval`](mcp-api-and-schemas.md#harnessrecord_eval)이 담당합니다.

### Verification Gate

Required verification을 위한 kernel gate입니다. User waiver는 `verification_gate=waived_by_user`를 set하지만 `detached_verified` assurance를 만들지 않습니다.

### Verification Independence Profile

Eval independence context의 named minimum qualification입니다. 예: `same_session`, `subagent_context`, `fresh_session`, `fresh_worktree`, `sandbox`, `manual_bundle`. Passed Eval은 `detached_verified`를 뒷받침하기 전에 valid profile을 만족해야 하며, 보안 격리 주장은 profile이 별도로 이름 붙이고 증명해야 합니다.

### Validator Result

Validator의 structured result입니다. status, guarantee level, target, findings, blocked reasons, suggested next action을 포함합니다.

### Vertical Slice

Trigger/input에서 domain logic, persistence 또는 state, caller/API 경계, observable output, tests, optional 수동 QA까지 얇은 경로를 연결하는 Change Unit shape입니다.

### Waiver

한국어 기준 표현: 면제 판단.

Policy가 허용하는 gate 또는 policy requirement에 대한 명시적으로 기록된 예외입니다. 면제 판단은 policy 또는 gate, Task와 Change Unit, 생략하는 check 또는 surface, reason, actor, 필요할 때 expiry 또는 follow-up, 영향받는 gate 또는 닫기 영향, 그리고 필요할 때 잔여 위험 경로로 보여주거나 수용해야 하는 close-relevant 잔여 위험을 이름 붙입니다. 검증 면제 판단, design waiver, QA 면제 판단은 정의된 rules 아래 명시적이고 범위가 정해진 경우에만 허용됩니다. Successful completion을 위해 product-write scope, 민감 동작 승인, required evidence coverage, required 작업 수락은 waived되지 않습니다. 검증 면제 판단과 QA 면제 판단은 assurance를 높이거나, 작업 수락을 암시하거나, unrelated 잔여 위험을 수용하거나, 생략된 check가 passed된 것처럼 만들지 않습니다.

### Write Authorization

한국어 기준 표현: 쓰기 허가 기록.

쓰기 허가 기록은 `prepare_write`가 특정 allowed write attempt에 대해 create하는 durable state record입니다. Replay, 최신성 감지, audit를 위한 compatibility basis로 사용된 affected-scope state version인 `basis_state_version`을 기록합니다. Distinct compatible `prepare_write` requests는 distinct authorization을 create하며, idempotent replay는 committed response를 반환할 수 있습니다. Committed implementation 또는 direct Run에 single-use이며, Change Unit scope, 민감 동작 승인, 결정 패킷 compatibility, evidence, verification, 수동 QA, 작업 수락, 잔여 위험 표시를 대체하지 않습니다.

### Write Authorization Lifecycle Events

한국어 기준 표현: 쓰기 허가 기록 lifecycle events.

쓰기 허가 기록 creation, return, consumption, expiry, staling, revocation, violation detection을 위한 stable event-name set입니다. Exact vocabulary와 `scope_violation_detected`와의 관계는 [Kernel Stable Event Catalog](kernel.md#stable-event-catalog)가 담당합니다.

### Write Authority Summary

Intended operation의 current write authority를 보여주는 user-facing display summary입니다. Active Change Unit scope, `prepare_write`, approval, baseline, guarantee, 결정 패킷 refs, 쓰기 허가 기록 ref에서 파생됩니다. 별도 authority record가 아닌 display이며 그 자체로 work 권한을 부여하지 않습니다.
