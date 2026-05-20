# Agent 통합

## 문서 역할

이 문서는 agent surface를 하네스에 연결할 때 지켜야 하는 공통 integration contract를 설명한다. Capability tier, capability profile, generated manifest expectation, push/pull context 원칙, fallback semantic, reference surface contract, connector conformance 개요를 정의한다.

본문은 product name에 중립적이다. Surface별 recipe는 [Appendix B](appendix/B-surface-cookbook.md)에 둔다.

이 문서는 kernel state transition, MCP request/response schema, SQLite DDL, capability gate, operational fixture detail, surface-specific cookbook을 정의하지 않는다.

## 통합 목표

통합의 목표는 사용자가 agent와 자연스럽게 대화하는 동안, 하네스가 bounded work, state recording, evidence, verification, Manual QA, acceptance, projection, reconcile flow를 뒤에서 안정적으로 제공하는 것이다.

```mermaid
flowchart TB
  Conversation["natural conversation"] --> Bounded["bounded work"]
  Bounded --> Harness["harness-supported flows"]
  Harness --> State["state recording"]
  Harness --> Evidence["evidence"]
  Harness --> Verification["verification"]
  Harness --> ManualQA["Manual QA"]
  Harness --> Acceptance["acceptance"]
  Harness --> Projection["projection"]
  Harness --> Reconcile["reconcile"]
```

Integrated surface는 agent가 다음을 할 수 있게 도와야 한다.

- status 또는 intake로 시작
- `advisor`, `direct`, `work`로 분류
- work를 scoped Change Unit으로 shaping
- 사용자 판단 없이 agent가 진행할 수 있는 일을 Autonomy Boundary로 shaping/update
- design-quality policy가 적용될 때 check
- state change에는 MCP tool call 사용
- product write 전 `prepare_write`와 반환된 Write Authorization 존중
- Write Authority Summary를 Autonomy Boundary와 별도로 표시
- blocking product judgment에는 Decision Packet을 request 또는 display
- run, artifact, evidence, user decision, QA, acceptance 기록
- approval, product decision, QA waiver, verification waiver, residual-risk acceptance, final acceptance 구분
- successful close 전에 알려진 close-relevant residual risk를 visible하게 표시
- detached verification launch 또는 package
- projection refresh 또는 reconcile

## 공통 통합 구조

```text
사용자 대화 surface
  -> 짧은 always-on rules/context
  -> harness skill, command, 또는 playbook
  -> harness MCP server
  -> harness Core
  -> adapter, hook, sidecar, validator, 또는 isolation layer
```

```mermaid
flowchart LR
  Surface["user surface"] --> Rules["rules / skill / playbook"]
  Rules --> MCP["harness MCP"]
  MCP --> Core["harness Core"]
  Core --> Layer["adapter / hook / sidecar / validator / isolation"]
  Layer --> Facts["observable facts 또는 stronger enforcement"]
```

### Always-On Rules

Always-on rule은 짧아야 한다. Agent에게 언제 harness를 쓰는지, status 또는 Journey Card를 어디서 읽는지, product write에는 `prepare_write`가 필요하다는 점을 알려주면 충분하다.

```mermaid
flowchart TD
  Start["agent receives work"] --> Resume{"significant work resumes?"}
  Resume -- yes --> Card["status 또는 Journey Card 표시"]
  Resume -- no --> Classify["advisor / direct / work 분류"]
  Card --> Classify
  Classify --> Judgment{"blocking product judgment?"}
  Judgment -- yes --> Packet["Decision Packet 표시 또는 요청"]
  Judgment -- no --> Write{"product write intended?"}
  Packet --> Resolved{"affected operation에 compatible user judgment resolved?"}
  Resolved -- no --> HoldJudgment["hold 또는 one blocking question"]
  Resolved -- yes --> Write
  Write -- no --> Proceed["current contract 안에서 계속 진행"]
  Write -- yes --> MCPAvailable{"authoritative MCP available?"}
  MCPAvailable -- no --> Hold["product write hold"]
  MCPAvailable -- yes --> Prepare["prepare_write call 및 Write Authority Summary 표시"]
  Prepare --> Allowed{"compatible Write Authorization?"}
  Allowed -- yes --> ProductWrite["authorized scope 안에서 write"]
  Allowed -- no --> Hold
```

Always-on rule은 user agency도 보존해야 한다.

- 중요한 work를 재개하기 전에 현재 Journey Card를 보여준다.
- Decision Packet이 필요한 상황을 포괄적인 승인 질문으로 뭉개지 않는다.
- 한 번에 하나의 blocking question만 묻고, 가능하면 recommendation과 uncertainty를 함께 제시한다.
- AFK implementation은 active Change Unit scope, Autonomy Boundary latitude, 적용되는 granted sensitive approval, 실제 product write 전 compatible `prepare_write` / Write Authorization이 모두 맞을 때만 허용한다.
- Autonomy Boundary는 judgment latitude이지 write authority가 아니다.
- Work가 write를 시작하려 할 때 Write Authority Summary를 보여준다.
- Authoritative MCP가 unavailable이면 product write를 hold한다.
- Planning direction, product trade-off, QA waiver, verification risk acceptance, final acceptance는 사용자가 쥔다.

Write Authority Summary는 active scoped Change Unit의 scope, `prepare_write`, approval, allowed path/tool/command/network/secret, product-judgment blocker를 제거하는 compatible Decision Packet ref에서 나온 current write boundary display다. Decision Packet은 그 자체로 write를 authorize하지 않는다. Autonomy Boundary는 agent가 추가 user decision 없이 행사할 수 있는 judgment만 설명한다.

Always-on rule에는 full state transition table, MCP schema, full template, 긴 design playbook, 모든 historical project context를 넣지 않는다.

### Skill Or Playbook Layer

Skill/playbook layer는 절차를 가르친다.

- status, intake, next를 언제 call할지
- status/next의 `recommended_playbooks`를 optional stage-router guidance로 어떻게 사용할지
- `advisor`/`direct`/`work`를 어떻게 분류할지
- shaping question을 어떻게 물을지
- Change Unit을 어떻게 form할지
- Autonomy Boundary를 어떻게 shaping/update할지
- blocking product judgment에 Decision Packet을 어떻게 request 또는 display할지
- write 전 Write Authority Summary를 어떻게 보여주고 compatible Write Authorization을 run과 함께 기록할지
- user decision을 어떻게 기록할지
- approval, product decision, QA waiver, verification waiver, residual-risk acceptance, final acceptance를 어떻게 구분할지
- TDD trace, evidence, Manual QA, acceptance를 어떻게 record할지
- two review stages를 어떻게 실행할지: 먼저 Spec Compliance Review, 그 다음 Code Quality / Stewardship Review
- successful close 전에 알려진 close-relevant residual risk를 visible하게 하고, risk-accepted close에는 accepted Residual Risk refs를 요구하며, required acceptance는 close-relevant residual risk가 visible한 뒤에만 record하는 방법
- work verification이 왜 detached되어야 하는지
- stale projection과 reconcile을 어떻게 처리할지

Stage routing은 shared-design, product-review, eng-review, tdd-loop, spec-review, code-quality-review, qa-review, guard-check, release-handoff, browser-qa-candidate 같은 recommended playbooks를 사용할 수 있습니다. 이 recommendations는 skill/playbook layer 안에 있습니다. Display guidance일 뿐이며 state를 mutate하거나, write를 authorize하거나, gate를 satisfy하거나, evidence를 만들거나, work를 verify하거나, QA를 waive하거나, risk를 accept하거나, Task를 close하지 않습니다. Recommended playbook이 product judgment를 제안하면 surface는 existing Decision Packet 또는 normal Decision Packet request path로 route해야 합니다.

Two-stage review procedure는 stages를 visible하게 분리해야 합니다.

1. Spec Compliance Review는 requested work가 current Harness authority 안에서 complete한지 확인합니다: acceptance criteria, Change Unit completion conditions, scope/write authority compatibility, Decision Packet compatibility, evidence coverage, residual-risk visibility.
2. Code Quality / Stewardship Review는 implementation이 maintainable한지 확인합니다: domain language, module/interface boundary, vertical slice shape, feedback loop 또는 TDD trace, codebase stewardship, context hygiene, follow-up risk.

두 stage의 findings는 validator results, evidence gaps, Decision Packet candidates, Change Unit update recommendations, residual-risk candidates, close blockers로 route되어야 합니다. Same-session review는 useful self-checking일 수 있지만 detached verification이 아니며 `assurance_level=detached_verified`로 표시하면 안 됩니다. Detached verification에는 여전히 valid independence boundary와 Eval path가 필요합니다.

Core와 validator가 policy를 enforce한다. Skill은 guidance이지 authority가 아니다.

### MCP Layer

MCP는 preferred state boundary다. Public tool name과 schema는 MCP API document가 담당한다. Integration doc은 tool intent를 reference할 수 있지만, connector는 `05-mcp-api-and-schemas.md`의 schema를 사용해야 한다.

### Adapter, Hook, Sidecar, Validator, Isolation

Adapter와 sidecar는 surface behavior를 observable fact 또는 stronger enforcement로 바꾼다.

- artifact capture
- command output capture
- changed-path detection
- generated file drift detection
- projection freshness detection
- approval and scope guard support
- same-session verification guard support
- evaluator read-only 또는 fresh-context support
- Manual QA capture support

이 layer는 guarantee level을 높일 수 있지만 kernel capability gate를 만들지는 않는다.

## Capability Tier

| Tier | Meaning | Typical capability |
|---|---|---|
| `T0 Context` | Surface가 harness principle을 읽을 수 있음 | rules/context file |
| `T1 Skill` | Surface가 harness procedure를 따를 수 있음 | skill, command, prompt, playbook |
| `T2 MCP` | Surface가 harness tool과 resource를 call할 수 있음 | MCP server connection |
| `T3 Capture` | Surface가 diff, log, run output을 reliable하게 반환할 수 있음 | structured output, wrapper, adapter |
| `T4 Guard` | Surface가 out-of-scope file, command, network, secret을 실행 전에 block 또는 interrupt할 수 있음 | hook, permission system, policy engine, sidecar |
| `T5 Isolation` | Surface가 verification 또는 risky work를 별도 boundary에서 run할 수 있음 | worktree, sandbox, fresh process, isolated runner |
| `T6 QA Capture` | Surface가 browser, screenshot, walkthrough, Manual QA artifact를 structure할 수 있음 | browser runner, screenshot capture, QA note capture |

일반 interactive harness use는 `T2` 이상에서 가장 자연스럽다. Reliable detached verification에는 보통 `T3` capture와 실제 independence boundary가 필요하다. High-risk work는 가능하면 `T4` guard 또는 `T5` isolation을 사용해야 한다. `T6`는 UI/UX evidence를 개선하지만, human QA note를 기록할 수 있다면 MVP에 필수는 아니다.

```mermaid
flowchart TB
  T0["T0 Context"] --> T1["T1 Skill"]
  T1 --> T2["T2 MCP"]
  T2 --> T3["T3 Capture"]
  T3 --> T4["T4 Guard"]
  T4 --> T5["T5 Isolation"]
  T5 --> T6["T6 QA Capture"]
```

## Capability Profile

Harness connector는 product 또는 surface name에서 behavior를 가정하지 않고 capability profile을 사용해야 한다.

```mermaid
flowchart TB
  Profile["Capability Profile"]
  Profile --> Identity["surface_id, surface_kind, target_profile"]
  Profile --> Support["support_tier"]
  Profile --> Guarantee["guarantee_level"]
  Profile --> Caps["capabilities"]
  Profile --> Risks["risks"]
  Profile --> Fallbacks["fallbacks"]
  Profile --> Verified["last_verified_at"]
  Caps --> CapabilityCheck["surface_capability_check"]
  Guarantee --> Display["guarantee display"]
```

```yaml
surface_id: SURF-0001
surface_kind: generic_agent
target_profile: local_cli
detected_version: optional string
capability_profile_version: 1
last_verified_at: 2026-05-06T10:05:00+09:00
support_tier: T2
guarantee_level: cooperative
capabilities:
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: false
  artifact_capture: manual
  hooks: false
  pre_tool_guard: false
  explicit_permissions: false
  changed_path_detection: validator
  fresh_verify: manual_bundle
  worktree_isolation: false
  local_sidecar: false
  browser_qa_capture: false
  screenshot_capture: false
risks:
  - no pre-tool guard
fallbacks:
  - cooperative prepare_write discipline
  - changed_paths validator
  - manual verification bundle
```

Target profile value 예시:

- `local_cli`
- `ide_chat`
- `ide_agent`
- `cloud_agent`
- `extension`
- `custom_agent`
- `manual_bundle`

Capability profile은 version, MCP config, hook, permission, workspace policy, generated file, conformance result, capture method, QA capture method가 바뀌면 refresh해야 한다.

## Guarantee Levels

Integration은 [04-runtime-architecture.md](04-runtime-architecture.md#guarantee-levels)에 정의된 guarantee level을 사용하고, 이를 connected surface profile, current enforcement path, fallback choice에 적용한다.

이 문서는 connector profile이 그 level을 report하고 display하는 방식을 담당한다. Surface name에서 더 강한 level을 추론하면 안 되며, guarantee level을 approval, verification, QA, acceptance, kernel gate로 취급하면 안 된다.

## Guarantee Display Requirements

Surface behavior에 의존하는 모든 status 또는 `prepare_write` result는 실제 guarantee level을 보여야 한다. Level은 surface name에서 추론한 약속이 아니라 connected profile과 current enforcement path의 property로 표시한다.

```mermaid
flowchart TD
  Result["status 또는 prepare_write result"] --> Profile["connected capability profile"]
  Result --> Enforcement["current enforcement path"]
  Profile --> Level{"actual guarantee_level"}
  Enforcement --> Level
  Level --> Cooperative["cooperative: instruction-following limitation 표시"]
  Level --> Detective["detective: after-action detection limitation 표시"]
  Level --> Preventive["preventive: blocking path 표시"]
  Level --> Isolated["isolated: separation boundary 표시"]
  Level --> Rule["approval, verification, QA, acceptance, kernel gate가 아님"]
  SurfaceName["surface name"] --> SurfaceRule["name에서 guarantee_level을 infer하면 안 됨"]
```

User-visible examples:

| Level | Example display text |
|---|---|
| `cooperative` | "이 surface는 Harness decision을 따를 것으로 기대되지만, out-of-scope write를 실행 전에 물리적으로 막지 못할 수 있습니다. Changed-path validation으로 사후 위반을 감지할 수 있습니다." |
| `detective` | "Harness는 action 후 changed path나 artifact를 관찰해 scope/evidence/projection을 stale 또는 blocked로 표시할 수 있습니다." |
| `preventive` | "Hook, wrapper, permission layer, sidecar가 위반 write를 실행 전에 막을 수 있습니다." |
| `isolated` | "Risky work 또는 verification이 별도 worktree, sandbox, process, 또는 동등한 boundary에서 실행됩니다." |

Rules:

- Cooperative가 preventive라는 뜻으로 보이면 안 된다.
- Surface name이 level을 보장한다는 뜻으로 보이면 안 된다.
- Guarantee level은 approval, verification, QA, acceptance, kernel gate가 아니다.

## Generated Manifest Concept

Connector는 rule, skill, MCP config snippet, prompt, local adapter file을 generate할 수 있다. 모든 generated 또는 managed path는 connector manifest에 기록해야 한다.

Manifest responsibility:

- generated path naming
- managed block hash 기록
- generated 시 사용한 capability profile 기록
- surface target profile 기록
- creation/update time 기록
- human edit를 overwrite하기 전에 drift detect
- 필요할 때 drift를 reconcile로 route

Manifest concept는 common하다. Surface-specific generated filename은 Appendix B에 둔다.

```mermaid
flowchart LR
  Generated["generated 또는 managed paths"] --> Manifest["connector manifest"]
  Manifest --> Hashes["managed block hashes"]
  Manifest --> Profile["capability profile과 target profile"]
  Manifest --> Drift["overwrite 전 drift detection"]
  Drift --> Reconcile["필요할 때 reconcile item"]
```

## Push And Pull Context

Implementation agent에게는 작은 current context를 push하고, 큰 reference는 필요할 때만 pull하게 해야 한다.

```mermaid
flowchart LR
  Current["current state와 evidence"] --> Push["usually pushed"]
  LongRefs["longer references"] --> Pull["usually pulled"]
  Push --> Agent["implementation agent"]
  Pull --> Agent
  Agent --> Bundle["tighter evaluator bundle"]
  Bundle --> Evaluator["detached evaluator"]
```

보통 push:

- Journey Card
- active Decision Packet summary
- Autonomy Boundary summary
- Write Authority Summary
- active scoped Change Unit
- acceptance criteria snapshot
- approval status
- latest evidence manifest/run ref
- close 또는 acceptance가 가까울 때 residual risk summary

보통 pull:

- old PRD
- old design
- closed issue
- long log
- module map
- interface contract
- domain language
- coding standard
- TDD guidance

Evaluator는 더 tight한 verification bundle을 받아야 한다.

- acceptance criteria
- changed file
- approval scope
- resolved, pending, close-relevant 항목을 포함한 관련 Decision Packet
- residual risk summary
- Autonomy Boundary
- deferred decision과 follow-up constraint
- relevant domain/module/interface record를 포함한 codebase stewardship ref
- evidence manifest
- required인 경우 TDD trace
- Manual QA requirement
- artifact ref
- forbidden pattern

이 context model은 Context Hygiene policy를 지원한다. Current state와 evidence는 stale chat이나 old doc보다 우선된다.

## Direct Fast Path

작은 direct work에서는 agent가 Harness를 대부분 보이지 않게 유지해야 한다. 좁은 active scope를 정하고, `prepare_write`를 call하고, 변경하고, changed path, self-check evidence를 기록한 뒤 blocker가 없으면 close한다.

Scope, risk, uncertainty, file spread가 커지면 direct mode를 broad autonomy로 늘리지 말고 같은 Task를 `work`로 escalate한다.

## Fallback Semantics

Fallback은 surface name이 아니라 guarantee level과 risk로 설명한다.

```mermaid
flowchart TD
  Need["fallback needed"] --> PreBlock{"pre-execution block available?"}
  PreBlock -- yes --> Preventive["preventive fallback"]
  PreBlock -- no --> Observe{"reliable after-action observation?"}
  Observe -- yes --> Detective["detective fallback"]
  Observe -- no --> Follow{"surface can follow instructions?"}
  Follow -- yes --> Cooperative["cooperative fallback"]
  Follow -- no --> Insufficient["capability insufficient"]
  Need --> Separation{"risk requires separation?"}
  Separation -- yes --> Isolated["isolated fallback"]
```

### Cooperative Fallback

Surface가 instruction을 따를 수 있지만 enforce할 수 없을 때 사용한다. Connector는 agent에게 `prepare_write`를 call하고, blocked decision에서는 hold하고, run을 record하라고 알려준다. Authoritative MCP가 unavailable이거나 write scope를 check할 수 없으면 product write를 pause해야 한다.

### Detective Fallback

Harness가 action 후 changed file, log, projection drift, artifact gap을 observe할 수 있을 때 사용한다. Validator는 state를 stale, partial, blocked, failed로 mark하고 repair, reconcile, fresh verification을 요구할 수 있다.

### Preventive Fallback

Hook, permission layer, wrapper, policy engine, sidecar가 violating edit, command, network call, secret access를 발생 전에 block할 수 있을 때 사용한다.

### Isolated Fallback

Risk에 separation이 필요할 때 사용한다. Connector는 별도 worktree, sandbox, process, manual evaluator bundle에서 work 또는 verification을 launch한다. Same-session review가 qualify하지 않는 detached verification에는 이것이 preferred fallback이다.

### MCP Unavailable

MCP가 unavailable이면 connector는 authoritative state update를 claim하면 안 된다. `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`은 diagnostic conditions이며, 추가 public `ErrorCode` values가 아니다. 이 conditions를 `ToolError`로 surface할 때는 API-owned error selection과 details shape를 사용한다. `MCP_UNAVAILABLE`은 stable public availability code로 남고, surface-side availability 또는 capability cases는 문맥에 따라 `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT`와 `details.mcp_unavailable_kind`로 표현될 수 있다. `MCP_SERVER_UNAVAILABLE`은 tool call이 Core에 닿을 수 없어 authoritative Core response가 불가능하다는 뜻이다. Caller는 state change를 claim하기 전에 reconnect 또는 diagnose해야 한다. `SURFACE_MCP_UNAVAILABLE`은 Core 또는 operator가 connected surface에 usable MCP가 없거나, MCP configuration이 stale이거나, required MCP tools를 call할 수 없음을 observe할 수 있다는 뜻이다. Product/runtime/code write의 safe behavior는 write를 hold하고 user/operator에게 MCP reconnect 또는 diagnose를 안내하는 것이다. Stronger profile은 preventive block도 enforce할 수 있다.

Pre-MVP Harness documentation-authoring batch는 exact path allowlist가 있는 명시적 `DOCS_AUTHORING_OVERRIDE` 아래에서만 진행할 수 있다. Connector는 이를 documentation-maintainer override로 label해야 하며, Core authorization, Write Authorization, evidence, verification, QA, acceptance, residual-risk acceptance, close, canonical state transition으로 label하면 안 된다. Authoritative MCP가 unavailable이면 product/runtime/code write는 계속 hold한다.

```mermaid
flowchart TD
  Problem["MCP unavailable condition"] --> ReachCore{"tool call이 Core에 닿는가?"}
  ReachCore -- no --> Server["diagnostic<br/>MCP_SERVER_UNAVAILABLE"]
  ReachCore -- yes --> Surface["diagnostic<br/>SURFACE_MCP_UNAVAILABLE"]
  Server --> NoState["authoritative Core response 없음"]
  Surface --> Diagnosed["Core 또는 operator가 unusable/stale surface MCP를 observe"]
  NoState --> Hold["product/runtime/code write hold"]
  Diagnosed --> Hold
  Hold --> DocsOverride{"exact allowlist가 있는 DOCS_AUTHORING_OVERRIDE?"}
  DocsOverride -- yes --> DocsOnly["listed docs only documentation-maintainer override"]
  DocsOverride -- no --> Reconnect["MCP reconnect 또는 diagnose"]
```

### Weak Guard

MCP는 동작하지만 pre-tool guard가 weak하면 low-risk direct work는 cooperative `prepare_write`와 detective changed-path validation으로 진행할 수 있다. Medium/high-risk work에는 stricter validation, sidecar guard, explicit approval, detached verification, isolation이 필요해야 한다.

### Projection Stale

Projection staleness는 state와 별도로 report된다. Connector가 canonical state를 직접 read할 수 있다면 계속 진행할 수 있지만, Markdown projection에 의존하는 action은 먼저 refresh 또는 reconcile해야 한다.

### Capability Insufficient

Connector는 product name이 아니라 missing capability를 말해야 한다. 예:

```text
Connected profile에 pre-tool guard가 없습니다. 이 작업에는 sidecar guard, 다른 profile, 또는 더 작은 active scoped Change Unit이 필요합니다.
```

## Reference Surface Contract

MVP는 하나의 reference surface를 target한다. Reference surface는 broad ecosystem support가 아니라 kernel을 demonstrate해야 한다.

Minimum reference expectations:

- public tool과 resource를 위한 `T2 MCP` available
- product write 전 cooperative `prepare_write`
- run 후 detective changed-path와 artifact validation
- evidence manifest에 충분한 run summary와 artifact capture
- manual verification bundle 또는 fresh evaluator instruction
- Manual QA note artifact support
- generated file을 위한 connector manifest
- common state와 fallback path를 cover하는 conformance smoke

Reference surface behavior detail과 product-specific setup은 concrete surface를 name할 때만 Appendix B에 둔다.

## Connector Conformance Overview

Connector conformance는 profile이 declared capability tier에서 common contract를 지킬 수 있음을 prove해야 한다.

Overview scenarios:

```mermaid
flowchart TB
  Conformance["connector conformance"]
  Conformance --> Status["status, intake, Journey Card"]
  Conformance --> Shaping["shared design, scope, one blocking question"]
  Conformance --> Decisions["Decision Packets and Autonomy Boundary"]
  Conformance --> Writes["prepare_write, Write Authority Summary, record_run authorization"]
  Conformance --> Evidence["artifacts, evidence, verification, same-session guard"]
  Conformance --> QA["Manual QA, acceptance, residual-risk visibility"]
  Conformance --> Drift["projection, generated file drift, reconcile"]
  Conformance --> Fallbacks["capability fallback and MCP unavailable hold"]
```

- active Task 유무에 따른 status
- significant work resume 전에 현재 Journey Card를 반드시 표시하는지
- advisor/direct/work로 intake classification
- shared design과 decision을 포함한 work shaping
- Change Unit scope와 vertical/horizontal exception handling
- 가능할 때 recommendation과 uncertainty가 있는 one blocking question
- blocking product judgment에 포괄적인 승인 대신 Decision Packet 표시
- Autonomy Boundary breach가 stop되거나 Decision Packet으로 route되는지
- AFK work가 active Change Unit scope, Autonomy Boundary latitude, 적용되는 granted sensitive approval, 실제 product write 전 compatible `prepare_write` / Write Authorization으로 모두 cover되는지
- `prepare_write` allowed 및 blocked path
- allowed write에 Write Authorization이 생성되고 Write Authority Summary로 노출되는지
- write-capable `record_run`이 compatible Write Authorization을 consume하는지
- sensitive approval request, granted, denied, expired path
- artifact와 evidence update가 있는 `record_run`
- direct result projection
- verification launch 또는 manual verification bundle
- same-session verification guard
- Manual QA required, passed, failed, waived
- product/user risk가 있는 QA waiver가 Decision Packet으로 route되는지
- acceptance required 및 recorded
- acceptance focus에 acceptance 요청 전 residual risk visibility가 포함되는지
- Known close-relevant residual risk가 successful close 전에 반드시 visible한지
- Risk-accepted close에 추가로 accepted Residual Risk refs가 필요한지
- Acceptance가 required인 경우 close-relevant residual risk가 visible한 뒤에만 record되는지
- stale projection과 reconcile flow
- generated file drift detection
- required tier가 missing일 때 capability fallback
- MCP unavailable product-write hold

정확한 fixture format과 operational command는 operations/conformance doc이 담당한다.
