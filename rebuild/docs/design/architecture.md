# Volicord 논리 아키텍처

- 상태: active target architecture owner
- 소유 범위: logical subsystem map, cross-subsystem dependency direction,
  subsystem boundary의 read/write/reference authority와 boundary conflict resolution
- 제품 의미 기준: [제품 헌장](product-charter.md),
  [제품 결정 등록부](open-decisions.md)
- domain 의미 기준: [핵심 도메인 모델](domain-model.md)
- evidence 기준: [architecture 입력 계약](architecture-inputs.md),
  [Wave 1 결론](../../validation/wave-1-summary.md)
- specialized 기준: [Repository Intelligence](repository-intelligence.md),
  [Privacy와 provider 경계](privacy-and-provider-boundary.md),
  [Inquiry와 Decision](inquiry-and-decision.md),
  [Projection과 document](projections-and-documents.md),
  [Portable Context](portable-context.md), [Versioning](versioning-policy.md),
  [Failure와 Recovery](failure-and-recovery.md)
- 비소유 범위: crate, process, binary, API, storage, parser, provider와 UI 기술 선택

이 문서는 Volicord의 logical target architecture를 정의한다. 여기서 subsystem은
한 가지 제품 책임과 authority 경계를 뜻한다. Subsystem 이름은 미래 crate,
process, binary, service, API endpoint 또는 database를 미리 정한 이름이 아니다.

## 1. 계약과 ownership

Architecture 문서의 해석 순서는 다음과 같다.

1. `product-charter.md`와 `open-decisions.md`가 제품 목적, accepted Decision과
   revisit trigger를 소유한다.
2. 이 문서가 cross-subsystem dependency direction, integration boundary와
   specialized document 사이의 boundary conflict를 소유한다.
3. `domain-model.md`가 세 information class, canonical entity, identity,
   provenance와 lifecycle 의미를 소유한다. 이 문서와 충돌해 보이는 항목이
   subsystem 배치 문제면 이 문서가, entity 의미 문제면 `domain-model.md`가
   해석 기준이다.
4. Active specialized Phase 3 문서는 `architecture-inputs.md`가 배정한 named
   domain을 구체화한다. 그 문서는 이
   문서의 dependency direction이나 `domain-model.md`의 core meaning을
   재정의할 수 없다.
5. `architecture-inputs.md`와 maintained validation report는 evidence constraint와
   unsupported conclusion을 소유한다. Prototype 구조는 target contract가 아니다.

`acceptance-scenarios.md`는 실사용과 cutover 시나리오를,
`validation-plan.md`는 risk validation과 production promotion gate를,
`cutover-plan.md`는 legacy 제거 조건과 순서를 계속 소유한다. 이 문서는 그
책임을 복제하지 않는다.

현재 아홉 active owner의 정확한 상태와 unique ownership은
`architecture-inputs.md`의 ownership plan이 route한다.

## 2. Dependency 표기와 기본 방향

아래에서 `A → B`는 A가 B의 공개된 logical capability 또는 read model에
의존할 수 있음을 뜻한다. 결과나 event가 반대 방향으로 전달될 수 있다는
사실은 dependency 방향을 뒤집지 않는다.

```text
Host and User Adapters → Local Operations
Host and User Adapters → Inquiry and Decision
Host and User Adapters → Projections and Documents
Local Operations       → Canonical Context Kernel
Local Operations       → Repository Intelligence
Local Operations       → Inquiry and Decision
Local Operations       → Projections and Documents
Inquiry and Decision   → Canonical Context Kernel
Inquiry and Decision   → Repository Intelligence
Projections and Documents → Canonical Context Kernel
Projections and Documents → Repository Intelligence
Repository Intelligence   → Canonical Context Kernel
Repository Intelligence   → optional Semantic Provider Boundary
Canonical Context Kernel  → no other logical subsystem
```

이 방향에는 다음 불변 조건이 적용된다.

- Canonical Context Kernel은 Repository Intelligence, LLM 또는 다른 semantic
  provider, MCP, CLI, viewer, document projection이나 renderer 없이 작동한다.
- Repository Intelligence가 canonical ID를 참조하거나 선택된 canonical context를
  읽는 것은 허용되지만 user judgment의 생성·유효성·변경을 소유하지 않는다.
- Inquiry and Decision은 canonical context와 repository analysis를 읽을 수 있지만,
  명시적으로 연결된 현재 host의 user response만 user Decision 입력이 될 수 있다.
- Projections and Documents는 읽기 결과를 만들며 rendering 또는 export의 side
  effect로 canonical record를 수정하지 않는다.
- Host and User Adapters는 host 표현을 use case 입력으로 번역할 뿐 domain
  meaning, provenance 또는 authority를 발명하지 않는다.
- Local Operations는 subsystem을 시작·연결·관찰할 수 있지만 각 subsystem의
  write invariant를 우회하지 않는다.
- Optional Semantic Provider Boundary의 부재나 실패는 Canonical Context Kernel과
  local Repository Intelligence capability를 사용할 수 없게 만들지 않는다.
- 어떤 analyzer, provider, adapter, projection 또는 renderer도 user response,
  agent recommendation, observed fact, semantic result나 generated interpretation을
  서로 대신하게 할 수 없다.

이 graph에는 circular dependency가 없다. 상위 subsystem은 하위 authority의
결과를 사용할 수 있지만, 하위 subsystem은 상위 presentation이나 orchestration을
자신의 correctness 조건으로 삼지 않는다.

## 3. Logical subsystem map

### 3.1 Canonical Context Kernel

Canonical Context Kernel은 portable하고 user-inspectable한 장기 맥락의 유일한
write authority다. `Project`, `Source`, `Question`, `Decision`, `Context Item`과
`Checkpoint` identity, provenance, relation, correction, supersession와 forgetting
invariant를 적용한다.

이 subsystem은 다음 책임을 가진다.

- canonical command의 의미와 허용 여부를 판정한다.
- canonical record와 relation을 authoritative하게 읽는다.
- user, agent, repository, command, provider, generated explanation과 import
  provenance가 섞이지 않게 한다.
- portable canonical representation에 필요한 domain truth를 제공한다.
- Derived State 없이도 canonical record를 inspect, correct, supersede와 forget할
  수 있게 한다.

Kernel은 repository를 분석하거나 질문 문구를 생성하거나 UI를 render하거나
provider를 호출하지 않는다. Local clone binding이나 file I/O를 어떤 기술로
구현할지도 이 경계가 정하지 않는다.

### 3.2 Repository Intelligence

Repository Intelligence는 repository-wide inventory, structural, semantic,
ecosystem과 agent-assisted understanding의 first-party 책임이다. Source snapshot을
기준으로 analysis를 만들고 언어·영역별 capability, coverage, freshness,
unsupported와 failure를 보존한다.

Repository Intelligence는 canonical `Project`, `Source`, `Decision` 등의 ID를
reference할 수 있고 설명에 필요한 selected canonical context를 읽을 수 있다.
하지만 analysis fact, semantic result 또는 interpretation을 user Decision으로
승격하거나 canonical correction을 되돌릴 수 없다. Canonical 승격이 필요한
관찰은 provenance가 있는 Session Candidate로 제출하고 Kernel의 domain
operation을 거친다.

세부 entity/relation vocabulary, language adapter, snapshot invalidation과
capability contract는 active [Repository Intelligence 계약](repository-intelligence.md)이
소유한다. 이 문서는 V01 prototype의 graph나 parser 모양을 채택하지 않는다.

### 3.3 Inquiry and Decision

Inquiry and Decision은 material uncertainty를 단계적으로 해결하는 use case를
소유한다. Canonical Question state와 relevant Decision을 읽고 Repository
Intelligence의 fact를 조사하며, 현재 dependency frontier와 선택에 필요한
설명을 구성한다.

이 subsystem은 Question 또는 Decision의 canonical write authority가 아니다.
새 Question의 승격, 현재 host response의 linkage와 Decision 생성은
`domain-model.md`의 provenance 조건을 갖춘 Kernel operation으로 완료된다.
Agent recommendation, 조사 결과, prototype 필요와 user choice는 별개의 의미로
유지한다.

Question discovery, frontier transition, response 처리, Recall/Checkpoint와의 상세
sequence는 active [Inquiry와 Decision 계약](inquiry-and-decision.md)이 소유한다.

### 3.4 Projections and Documents

Projections and Documents는 Canonical Context와 필요한 Derived State를 읽어 Recall,
viewer view와 generated document를 만든다. User용과 agent용 표현의 깊이는 달라도
record identity, source, freshness, uncertainty, supersession와 omission basis는
같다.

Projection은 authoritative record가 아니며 generated document도 explicit adoption
전에는 canonical truth가 아니다. Rendering, preview, 파일 출력 또는 실패가
source record를 바꾸지 않는다. 상세 Recall selection, document grounding,
adoption과 output contract는 active
[Projection과 document 계약](projections-and-documents.md)이 소유한다.

### 3.5 Host and User Adapters

Host and User Adapters는 Codex, MCP, CLI, local viewer와 이후 지원되는 host의
input/output을 logical use case로 번역한다. Adapter는 user turn, agent session,
confirmation과 display context를 손실 없이 전달하고 subsystem 결과를 host에
맞게 표현한다.

현재 host의 explicit Guarded confirmation request/response transport는 Host and User
Adapters가 소유한다. Host가 confirmation을 elicitation할 수 없으면 같은 logical
contract를 local viewer 또는 CLI로 전달하는 fallback을 제공한다. Fallback은 weaker
approval, general consent 또는 다른 operation identity를 만드는 경로가 아니다.

Adapter는 Question을 답한 것으로 추론하거나, recommendation을 user choice로
바꾸거나, low-level transport identity를 domain identity로 만들지 않는다. 특정
host, command 또는 wire representation은 이 문서의 architecture contract가
아니다.

### 3.6 Local Operations

Local Operations는 한 local environment에서 Project binding, subsystem lifecycle,
analysis scheduling, health, rebuild, portable I/O와 사용자 지정 output publication을
조정한다. 이 subsystem은 local resource와 execution concern을 다루며 canonical
meaning이나 analysis meaning을 소유하지 않는다.

Local Operations는 Kernel의 canonical operation, Repository Intelligence의
analysis operation과 Projection의 read operation을 순서대로 호출할 수 있다.
실패를 숨기거나 하나의 subsystem 결과를 다른 subsystem의 성공으로 위조할 수
없다. Process, canonical/projection failure separation, repair/rebuild와 recovery의
상세 contract는 active [Failure와 Recovery 계약](failure-and-recovery.md)이,
durable format evolution은 active [Versioning 정책](versioning-policy.md)이 소유한다.

Local Operations는 Guarded effect dispatch와 exact confirmation validation의 logical
owner다. Transport가 confirmation을 받았다는 사실만으로 effect를 실행하지 않으며,
아래 contract를 통과한 operation만 dispatch한다.

#### Guarded-effect confirmation contract

Accepted Guarded boundary의 initial high-risk categories는 다음과 같다.

- destructive file/data deletion
- irreversible 또는 large-scale migration
- external deployment 또는 public publication
- payment 또는 continuing cost
- secret/credential access 또는 change
- personal data 또는 source code의 external transmission
- external system으로 message, email 또는 issue 전송
- production data change
- permission, authentication 또는 security-setting change

Ordinary code edit, local test, local repository inventory와 local structural analysis는
Guarded boundary 밖이며 confirmation 때문에 block되지 않는다. `Guarded Effect
Candidate`는 `domain-model.md`가 분류하는 Derived/operational state이고 새 canonical
core entity가 아니다.

하나의 logical confirmation request identity는 immutable revision별로 다음 exact
meaning을 가진다.

| Confirmation request field | Required meaning |
|---|---|
| `confirmation_request_identity` | 같은 logical request의 revision history를 묶는 identity |
| `request_revision` | user에게 실제 표시되고 response가 bind되는 exact revision |
| `exact_action` | dispatch하려는 action |
| `target` | effect가 적용되는 exact target |
| `expected_effect` | user가 승인 전에 이해해야 하는 예상 effect |
| `risk` | category와 concrete consequence |
| `scope` | action/target에 허용되는 bounded scope |
| `expiration` | confirmation이 더 이상 유효하지 않은 time/basis |
| `requesting_actor_and_provenance` | request를 만든 actor/session과 Candidate/Source/operation basis |
| `effect_fingerprint` | action, target, expected effect, risk, scope와 revision을 exact-match하는 stable comparison basis |

User confirmation input은 current host에서 explicit하게 제출된 `Source`-linked response다.
Exact confirmation request identity/revision, user-turn Source와 response basis를
연결하며 general product `Decision`, 과거 consent, agent inference 또는 unrelated
approval과 분리한다.

Confirmation은 action-scoped, target-scoped, scope-scoped, expiring, non-transferable이며
single-use다. Dispatch 전에 Local Operations는 request revision, exact action, target,
expected effect, scope, expiration과 effect fingerprint가 current Candidate와 일치하고
user-response Source가 valid하며 아직 consumption되지 않았는지 검증한다. Action,
target, expected effect, scope 또는 request revision이 바뀌거나 confirmation이 stale/
expired하면 새 confirmation이 필요하다. 한 confirmation은 다른 effect를 authorize하지
않고 consumed confirmation의 reuse는 reject한다.

Valid confirmation 전에는 external 또는 그 밖의 Guarded effect를 dispatch하지 않는다.
Confirmation consumption과 dispatch는 하나의 `operation_identity`로 연결해 다음
outcome을 구분한다.

| Guarded operation outcome | Meaning |
|---|---|
| `not_dispatched` | confirmation validation/consumption 또는 dispatch 전 단계에서 멈춤 |
| `dispatched_and_completed` | exact operation이 dispatch되고 completion outcome을 확인함 |
| `dispatched_and_failed` | dispatch는 일어났고 failure outcome을 확인함 |
| `execution_outcome_indeterminate` | termination/communication loss 뒤 dispatch 또는 effect completion을 확정할 수 없음 |

Consumption과 dispatch의 implementation atomicity/repair mechanism은 아직 선택하지
않지만 결과는 approval reuse나 silent retry를 허용해서는 안 된다. Durable Project
history가 필요하면 user-response Source와 resulting operation을 Checkpoint/Context
Item이 reference할 수 있다. Operational confirmation을 general Decision이나 일곱 번째
canonical core entity로 만들지 않는다.

이 경계는 cooperative confirmation이며 OS sandbox 또는 security enforcement가
아니다. Host/adapter가 표현을 전달하고 Local Operations가 cooperative product
contract를 적용하지만 외부 process나 operating system을 강제로 격리한다는 보증은
하지 않는다.

### 3.7 Optional Semantic Provider Boundary

Optional Semantic Provider Boundary는 background semantic capability가 외부 또는
별도 provider를 사용할 때의 논리적 격리점이다. Repository Intelligence만
snapshot-scoped analysis request를 통해 이 경계를 사용하며, provider output은
provenance를 가진 Derived State 또는 Session Candidate로 돌아온다.

이 boundary는 기본적으로 absent일 수 있다. Project opt-in, source scope와
interactive/background 구분을 만족하지 않은 background transmission을 허용하지
않으며 user judgment나 canonical write authority를 갖지 않는다. Transmission,
retention, revoke와 deletion의 상세 contract는
[Privacy와 provider 경계 계약](privacy-and-provider-boundary.md)이 소유한다.

## 4. Boundary authority

| Subsystem | Read authority | Write authority | Reference authority |
|---|---|---|---|
| Canonical Context Kernel | 모든 canonical record와 relation | Canonical Context의 유일한 authoritative mutation | canonical entity identity와 relation의 기준 |
| Repository Intelligence | repository snapshot, selected canonical context, 자신의 Derived State | analysis Derived State와 provenance-bearing Candidate | canonical ID를 연결점으로 참조하되 의미를 재정의하지 않음 |
| Inquiry and Decision | canonical Question/Decision/Context, relevant analysis와 Candidate | inquiry-local Candidate와 Kernel에 제출할 intent | exact Question revision, user-turn Source와 analysis Source를 참조 |
| Projections and Documents | canonical read model과 허용된 Derived State | disposable projection, preview와 user-requested output | source/Decision/snapshot identity를 그대로 보존 |
| Host and User Adapters | 노출이 허용된 use-case result | transport/session observation, explicit user intent와 current-host confirmation response 전달 | host turn/session/confirmation request identity를 provenance로 전달 |
| Local Operations | health, local binding, subsystem result, confirmation state와 operation status | local binding, runtime coordination, Guarded dispatch와 rebuildable operational state | Project/Source/confirmation/operation identity를 local resource와 연결 |
| Optional Semantic Provider Boundary | opt-in된 snapshot-scoped source 범위 | provider result와 delivery observation | provider/model/source snapshot basis를 보존 |

`write authority`는 저장 매체의 독점이라는 뜻이 아니라 해당 information class의
meaning을 확정할 권한을 뜻한다. 예를 들어 Adapter가 user input을 capture하거나
Repository Intelligence가 Candidate를 기록해도 Canonical Context를 직접 만든
것은 아니다.

모든 subsystem boundary에는 다음 규칙이 공통으로 적용된다.

- Read 결과는 identity, provenance, information class와 freshness를 잃지 않는다.
- Write 요청은 목적 subsystem의 invariant validation을 거치며 caller 권한만으로
  의미가 확정되지 않는다.
- Reference는 대상 record의 ownership을 이전하지 않는다.
- Derived 또는 Candidate identity는 canonical identity처럼 제시되지 않는다.
- Partial, unavailable, failed와 stale 상태를 empty success로 바꾸지 않는다.

## 5. 주요 journey의 entry와 ownership

### 1. Project initialization과 local clone binding

Host and User Adapter가 explicit init/bind intent를 Local Operations에 전달한다.
Canonical Context Kernel이 path와 독립적인 Project identity와 portable Source
meaning을 만들고, Local Operations가 현재 clone과의 local binding을 소유한다.
Canonical mutation이 commit된 뒤 binding result를 별도로 확정하며 한쪽 실패를 다른
쪽 success로 위조하지 않는다. Tracked marker 자동 생성이나 path/remote 기반 Project
identity 추론은 하지 않는다.

### 2. Repository inventory와 degraded analysis

Local Operations가 analysis를 요청·관찰하고 Repository Intelligence가 snapshot,
analyzer-independent inventory, language/area별 capability, coverage와 analysis result를
소유한다. 각 adapter result는 bounded outcome으로 aggregate하며 한 language가
`failed`/`partial`이어도 다른 language, inventory와 historical result를 보존한다.
Analyzer/provider/index state와 usable remainder를 표시하고 Project와 Canonical Context는
계속 사용할 수 있다.

### 3. First project-scoped Recall

Host and User Adapter가 첫 project-scoped interaction임을 전달하면 Projections and
Documents가 Kernel의 canonical basis와 허용된 current analysis를 읽어 bounded,
read-only Recall을 구성한다. Recall은 canonical mutation을 만들지 않으며 사용한
record, Source, freshness와 omission을 추적할 수 있게 한다.

### 4. Staged Inquiry와 user Decision

Host and User Adapter가 Inquiry entry 또는 material response를 전달한다. Inquiry
and Decision은 먼저 확인 가능한 fact를 조사하고 current frontier를 제시한다.
User response는 exact Question identity/revision과 current user-turn Source에
연결된 경우에만 Kernel이 user Decision으로 기록한다. Agent recommendation이나
provider result는 이 경로를 대신할 수 없다.

### 5. Ordinary work와 source-grounded Checkpoint

일반 repository 작업은 Volicord의 사전 admission 대상이 아니다. Adapter와 Local
Operations는 work observation을 Candidate로 전달할 수 있다. 의미 있는 완료,
pause 또는 handoff boundary에서 source, changed basis, verification, known limits와
next step이 확인되면 Kernel이 Checkpoint를 canonical로 기록한다. Work,
verification, user review와 user acceptance는 서로 독립적으로 남는다.

### 6. Portable export/import와 conflict resolution

Local Operations가 user-requested I/O를 조정하고 Kernel이 portable canonical
meaning과 identity를 제공한다. Import는 canonical provenance를 보존하고 현재
clone binding은 별도로 수행한다. Bundle content, divergence, conflict와 resolution
authority의 상세 contract는 active [Portable Context 계약](portable-context.md)이
소유하며 concrete algorithm이나 serialization technology는 선택하지 않는다. Format
version behavior는 active [Versioning 정책](versioning-policy.md)이 소유한다.

Import 전 bundle/version/integrity와 common-base basis를 검증하고 local binding은
portable record와 분리한다. Independent addition 외의 semantic Decision/Question,
delete/modify와 unavailable-base conflict는 user-owned resolution 또는 branch 전까지
unresolved로 남긴다. Resolution은 common-base와 input provenance를 보존한다.

### 7. Generated document preview와 explicit adoption

Projections and Documents가 canonical/derived read basis에서 문서를 만들고 Host and
User Adapter가 read-only draft/preview와 user-selected destination을 다룬다. Grounding,
coverage, omission과 generated-document metadata version을 검증하며 render/export
failure는 canonical state를 바꾸지 않는다. 편집본이나 생성물을 Source/Context로
보존하려면 exact artifact/revision, current-host user Source와 origin grounding을 가진
별도의 explicit adoption intent를 Kernel operation에 제출한다. Publication success와
adoption success는 독립 결과다.

### 8. Background semantic-provider opt-in

Host and User Adapter가 Project-scoped opt-in intent와 inspectable source scope를
Local Operations에 전달한다. Repository Intelligence만 Optional Semantic Provider
Boundary를 통해 background request를 수행하며 결과를 Derived State 또는
Candidate로 분류한다. Opt-in이 없거나 provider가 unavailable이면 이 path만
비활성화되고 local core journey는 유지된다.

### 9. Guarded effect confirmation과 dispatch

Host and User Adapter가 Guarded Effect Candidate의 exact request identity/revision과
current-host response Source를 운반한다. Host가 response를 elicitation할 수 없으면 local
viewer 또는 CLI가 같은 request를 표시하고 같은 Source-linkage contract로 응답을
전달한다. Local Operations는 expiration, exact action/target/expected effect/scope,
fingerprint와 unused state를 확인한 뒤에만 하나의 operation identity로 consume/dispatch를
연결한다. Missing/denied/stale/expired/mismatched/reused response는 dispatch하지 않고,
indeterminate execution은 [Failure와 Recovery 계약](failure-and-recovery.md)에 따라
silent retry나 success claim 없이 scoped recovery로 보낸다.

### 10. Analyzer, provider, index, source와 process failure recovery

Local Operations는 bounded subsystem outcome을 모두 관찰하고 active
[Failure와 Recovery 계약](failure-and-recovery.md)의 root cause, scope, usable remainder,
retry owner와 repair/rebuild consequence를 보존한다. Analyzer/provider failure는 affected
analysis만 degrade하고, Derived Index corruption은 quarantine/rebuild하며, unavailable
source에서도 canonical read를 유지한다. Forced termination은 complete stdout/stderr,
exit/termination, duration, cancellation/timeout과 child cleanup을 보고한다. Canonical
transaction failure는 projection degradation으로 축소하지 않고 commit state를 Kernel이
확인한다. Repair/rebuild 뒤에는 source snapshot, provenance, coverage와 user correction이
유지됐는지 다시 검증한다.

## 6. Logical boundary와 implementation boundary의 구분

이 문서의 일곱 subsystem은 responsibility separation이다. 미래 구현은 검증된
필요에 따라 다음 중 어느 형태로든 배치할 수 있다.

- 같은 crate의 module 또는 여러 crate
- 같은 process 또는 격리된 process
- 하나 또는 여러 binary
- in-process call, local protocol 또는 다른 API
- 하나 또는 여러 storage mechanism

반대로 code가 같은 module이나 transaction에 있다는 이유로 두 subsystem의
authority가 합쳐지지 않는다. Process 분리가 있다는 이유만으로 올바른 domain
boundary가 생기는 것도 아니다. 구체적 배치는 production promotion과 관련
specialized validation을 통과한 뒤 선택한다.

## 7. Phase 4 responsibility handoff

Phase 4는 다음 dependency-respecting responsibility chain을 순서대로 입증한다.

```text
Project and Source
→ Question
→ Decision
→ Context Item
→ Checkpoint
→ revision, supersession, contradiction, and forgetting
→ portable bundle and local binding
→ deterministic Recall basis
```

| Responsibility boundary | Phase 4 handoff result |
|---|---|
| Project and Source | path-independent Project identity, actor/repository/command Source provenance와 local binding 분리를 provider/analyzer 없이 durable하게 읽고 검증 |
| Question | material identity, revision, Source basis와 dependency를 Decision 없이도 보존하고 current state를 읽음 |
| Decision | exact current-host Question revision/turn linkage와 user choice/delegation을 recommendation과 분리해 atomic canonical result로 보존 |
| Context Item | goal/fact/assumption/constraint/preference/risk/limit과 statement role/provenance를 구분해 보존 |
| Checkpoint | meaningful work/pause/handoff의 changed basis, applied Decision, verification, limits, open Question과 next step을 독립 fact로 보존 |
| revision, supersession, contradiction, and forgetting | non-semantic correction, semantic replacement, unresolved evidence conflict와 content removal을 서로 바꾸지 않고 inspectable하게 적용 |
| portable bundle and local binding | deterministic canonical export/import, Project identity preservation, local path exclusion, explicit bind와 supported format check를 제공 |
| deterministic Recall basis | Derived State 없이도 stable canonical read order와 active/history/applicability basis를 제공하며 projection 자체는 후속 read owner에 남김 |

각 boundary는 이전 responsibility의 identity, provenance와 failure invariant를 유지한
상태에서 observable behavior와 production Rust test로 닫는다. 이 handoff는 crate/module
plan, public/internal API catalog, database/schema design, implementation schedule 또는
빈 future entity shell을 정하지 않는다. Repository Intelligence, provider, host와
renderer는 이 chain의 correctness dependency가 아니다.

## 8. Accepted Decision traceability contract

제품 의미와 revisit trigger의 전체 wording은 `open-decisions.md`가 소유한다. 아래
표의 architecture owner는 해당 accepted constraint가 target architecture에서 들어가는
유일한 주 interface owner다. 다른 문서는 reference할 수 있지만 다시 정의하지 않는다.

| Decision | Authoritative architecture owner | Owned interface |
|---|---|---|
| D1 | `architecture.md` | 하나의 clean logical product graph, legacy dependency 부재와 cutover boundary |
| D2 | `domain-model.md` | Project-scoped user/agent/session/Source identity와 provenance separation |
| D3 | `portable-context.md` | user-owned portable canonical context와 local clone binding 분리 |
| D4 | `domain-model.md` | Project, Source, Question, Decision, Context Item, Checkpoint의 canonical meaning |
| D5 | `domain-model.md` | Canonical Context, Session Candidate와 Derived State classification/promotion |
| D6 | `inquiry-and-decision.md` | material Question의 unbounded staged dependency frontier |
| D7 | `inquiry-and-decision.md` | exact current-host Question revision/turn response와 Decision atomic boundary |
| D8 | `architecture.md` | ordinary work non-blocking과 exact, single-use Guarded confirmation/dispatch isolation |
| D9 | `domain-model.md` | work, verification, review와 acceptance fact dimension separation |
| D10 | `projections-and-documents.md` | user/agent가 공유하는 canonical/source/freshness Recall basis |
| D11 | `architecture.md` | first-party Repository Intelligence와 document projection을 Kernel에서 분리한 subsystem map |
| D12 | `architecture.md` | end-to-end journey와 Phase 4 responsibility handoff를 향한 implementation boundary |
| Q1 | `inquiry-and-decision.md` | Inquiry entry, frontier, round, terminal outcome와 pause/resume |
| Q2 | `repository-intelligence.md` | polyglot capability, snapshot/envelope, coverage와 adapter extension boundary |
| Q3 | `privacy-and-provider-boundary.md` | local/interactive/background authority, opt-in, transmission와 deletion |
| Q4 | `architecture.md` | Host/User Adapter, Local Operations, Inquiry와 Projection surface responsibility |
| Q5 | `projections-and-documents.md` | four grounded documents, Markdown/HTML, preview/publication/adoption boundary |
| Q6 | `portable-context.md` | Project/binding, bundle, common base, conflict class와 resolution authority |
| Q7 | `domain-model.md` | correction, supersession, contradiction/review와 forgetting meaning |
| Q8-A | `architecture.md` | Linux/Codex Host/Operations entry와 logical core separation |
| Q8-B | `architecture.md` | legacy runtime/API/data path가 없는 clean product graph |
| Q9 | `projections-and-documents.md` | first project-scoped bounded read-only Recall과 visible omission basis |
| Q10 | `domain-model.md` | Candidate identity/metadata/lifecycle와 promotion meaning; inspection은 Projection, collection/retention은 Privacy owner로 route |
| Q11 | `domain-model.md` | source-grounded meaningful Checkpoint와 independent status dimensions |
| Q12 | `architecture.md` | Guarded Candidate, Source-linked confirmation, Host/fallback transport와 exact pre-dispatch validation/consumption |
| Q13 | `inquiry-and-decision.md` | Decision applicability, reuse와 evidence-driven re-questioning |

## 9. Acceptance scenario traceability contract

Acceptance 시나리오의 complete user experience는 `acceptance-scenarios.md`, phase
sequence/cutover는 `cutover-plan.md`, validation execution은 `validation-plan.md`가
소유한다. 이 표는 primary architecture interface, first implementing phase와 owning
later validation을 연결한다.

| Scenario | Owning architecture document | Future implementation phase | Owning later validation |
|---|---|---|---|
| A | `architecture.md` — init/bind와 Host/Operations | Phase 7 | V08 |
| B | `repository-intelligence.md` — inventory/agent-assisted fallback | Phase 5 | V11 |
| C | `repository-intelligence.md` — structural adapter/envelope | Phase 5 | V11 |
| D | `repository-intelligence.md` — semantic normalization | Phase 5 | V02 |
| E | `repository-intelligence.md` — per-area polyglot composition | Phase 5 | V11 |
| F | `inquiry-and-decision.md` — staged frontier/response | Phase 6 | V11 |
| G | `domain-model.md` — Candidate lifecycle/promotion; Projection inspection과 Privacy collection/retention route | Phase 6 | V07, V09, V11 |
| H | `domain-model.md` — Checkpoint/source/status meaning | Phase 6 | V09 |
| I | `projections-and-documents.md` — automatic bounded Recall | Phase 6 | V09 |
| J | `portable-context.md` — another-clone divergence/conflict | Phase 4 | V04 |
| K | `domain-model.md` — revision/supersession/forgetting과 privacy/deletion propagation | Phase 4 | V04, V07 |
| L | `inquiry-and-decision.md` — Decision reuse/re-questioning | Phase 6 | V09 |
| M | `architecture.md` — Guarded confirmation transport, fallback, exact validation/dispatch와 non-reuse | Phase 7 | V08, V11 |
| N | `projections-and-documents.md` — viewer/map/documents/adoption | Phase 7 | V06 |
| O | `failure-and-recovery.md` — degradation/process/recovery | Phase 7 | V10, V11 |
| P | `architecture.md` — fresh-service/legacy exclusion | Phase 9 | V08, V11 |
| Q | `privacy-and-provider-boundary.md` — background opt-in/local-only | Phase 7 | V07 |

## 10. Later-validation interface traceability

| Validation | Owning Phase 3 interface | Precise contract validated later |
|---|---|---|
| V02 | `repository-intelligence.md`, `versioning-policy.md` | 최소 세 ecosystem의 Semantic Result normalization, structural/semantic provenance, snapshot version, range, diagnostics와 incomplete-build degradation |
| V04 | `portable-context.md`, `versioning-policy.md` | common base, six conflict classes, automatic/user resolution limit, deletion propagation, merge provenance와 bundle-version non-mutation |
| V06 | `projections-and-documents.md`, `versioning-policy.md` | four-document grounding/omission, canonical purity, preview/adoption, generated metadata와 Markdown/HTML equivalence |
| V07 | `privacy-and-provider-boundary.md` | three provider boundaries, Candidate collection opt-out/retention, privacy/deletion propagation, revoke와 local-only journey |
| V08 | `architecture.md`, `failure-and-recovery.md` | Linux/Codex init/bind/health, Guarded current-host transport와 viewer/CLI fallback, process cleanup와 clean-runtime exclusion |
| V09 | `inquiry-and-decision.md`, `projections-and-documents.md` | frontier/resume, Decision reuse, Checkpoint/Recall 및 Candidate promotion/inspection/no-mutation |
| V10 | `failure-and-recovery.md`, `versioning-policy.md` | complete streams/exit/termination, timeout/cancellation/progress, child cleanup, atomic publication, corruption, repair/rebuild와 upgrade failure |
| V11 | `architecture.md`와 모든 specialized owner | integrated Candidate journey와 Guarded exact-match/non-reuse/ordinary-action behavior를 포함한 multi-repository product boundaries |

Validation이 accepted Decision revisit trigger를 충족하면 `open-decisions.md` 절차를
따른다. Interface owner가 validation 결과를 product scope 축소나 새로운 implementation
technology 선택으로 조용히 번역하지 않는다.

## 11. Non-goals와 열린 implementation choice

이 architecture는 다음을 선택하거나 보증하지 않는다.

- crate/module taxonomy, public/internal API와 MCP method
- process topology, daemon, supervisor와 binary 수
- database, table, transaction, serialization library와 concrete field
- portable bundle format, version number, merge algorithm과 conflict UI
- parser framework, language grammar, indexer, LSP와 semantic protocol
- provider, model, transmission mechanism과 retention implementation
- Question discovery algorithm, ranking, general paraphrase recognition과 atomic host API
- viewer framework, graph layout, renderer와 document template
- repair/encryption implementation, schema upgrade engine와 production crash technology
- Wave 1 prototype code 또는 experimental schema의 promotion

이 문서는 first structural language set, user-owned canonical context, local-only
operation과 source-grounded explanation 같은 accepted product contract를 구현
편의 때문에 좁히지 않는다. Evidence gap은 해당 active owner와 validation이
해결하며, accepted Decision 변경이 필요하면 제품 결정 revisit 절차를
따른다.
