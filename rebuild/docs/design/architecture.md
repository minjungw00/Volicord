# Volicord 논리 아키텍처

- 상태: active target architecture owner
- 소유 범위: logical subsystem map, cross-subsystem dependency direction,
  subsystem boundary의 read/write/reference authority와 boundary conflict resolution
- 제품 의미 기준: [제품 헌장](product-charter.md),
  [제품 결정 등록부](open-decisions.md)
- domain 의미 기준: [핵심 도메인 모델](domain-model.md)
- evidence 기준: [architecture 입력 계약](architecture-inputs.md),
  [Wave 1 결론](../../validation/wave-1-summary.md)
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
4. 이후 생성되는 specialized Phase 3 문서는
   `architecture-inputs.md`가 배정한 named domain을 구체화한다. 그 문서는 이
   문서의 dependency direction이나 `domain-model.md`의 core meaning을
   재정의할 수 없다.
5. `architecture-inputs.md`와 maintained validation report는 evidence constraint와
   unsupported conclusion을 소유한다. Prototype 구조는 target contract가 아니다.

`acceptance-scenarios.md`는 실사용과 cutover 시나리오를,
`validation-plan.md`는 risk validation과 production promotion gate를,
`cutover-plan.md`는 legacy 제거 조건과 순서를 계속 소유한다. 이 문서는 그
책임을 복제하지 않는다.

현재 active specialized owner와 remaining planned owner의 정확한 상태는
`architecture-inputs.md`의 ownership plan이 route한다. 생성되지 않은 문서는 파일이
생성되고 owner routing이 갱신되기 전에는 active contract가 아니다.

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
없다. Process topology, supervisor, transaction, filesystem publication과 recovery의
상세 contract는 이후 `failure-and-recovery.md` 등 해당 owner가 생성된 뒤 정한다.

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
| Host and User Adapters | 노출이 허용된 use-case result | transport/session observation과 explicit user intent 전달 | host turn/session identity를 provenance로 전달 |
| Local Operations | health, local binding, subsystem result와 operation status | local binding, runtime coordination과 rebuildable operational state | Project/Source ID를 local resource와 연결 |
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

### Project initialization과 local clone binding

Host and User Adapter가 explicit init/bind intent를 Local Operations에 전달한다.
Canonical Context Kernel이 path와 독립적인 Project identity와 portable Source
meaning을 만들고, Local Operations가 현재 clone과의 local binding을 소유한다.
Tracked marker 자동 생성이나 path/remote 기반 Project identity 추론은 하지 않는다.

### Repository analysis

Local Operations가 analysis를 요청·관찰하고 Repository Intelligence가 snapshot,
inventory, capability, coverage와 analysis result를 소유한다. Analyzer가 없거나
일부 언어가 실패해도 Project와 Canonical Context는 계속 사용할 수 있다.

### First project-scoped Recall

Host and User Adapter가 첫 project-scoped interaction임을 전달하면 Projections and
Documents가 Kernel의 canonical basis와 허용된 current analysis를 읽어 bounded,
read-only Recall을 구성한다. Recall은 canonical mutation을 만들지 않으며 사용한
record, Source, freshness와 omission을 추적할 수 있게 한다.

### Staged Inquiry와 user Decision

Host and User Adapter가 Inquiry entry 또는 material response를 전달한다. Inquiry
and Decision은 먼저 확인 가능한 fact를 조사하고 current frontier를 제시한다.
User response는 exact Question identity/revision과 current user-turn Source에
연결된 경우에만 Kernel이 user Decision으로 기록한다. Agent recommendation이나
provider result는 이 경로를 대신할 수 없다.

### Ordinary work와 Checkpoint

일반 repository 작업은 Volicord의 사전 admission 대상이 아니다. Adapter와 Local
Operations는 work observation을 Candidate로 전달할 수 있다. 의미 있는 완료,
pause 또는 handoff boundary에서 source, changed basis, verification, known limits와
next step이 확인되면 Kernel이 Checkpoint를 canonical로 기록한다. Work,
verification, user review와 user acceptance는 서로 독립적으로 남는다.

### Portable export/import

Local Operations가 user-requested I/O를 조정하고 Kernel이 portable canonical
meaning과 identity를 제공한다. Import는 canonical provenance를 보존하고 현재
clone binding은 별도로 수행한다. Bundle content, divergence, conflict와 resolution
authority의 상세 contract는 active [Portable Context 계약](portable-context.md)이
소유하며 concrete algorithm이나 serialization technology는 선택하지 않는다. Format
version behavior는 active [Versioning 정책](versioning-policy.md)이 소유한다.

### Document projection

Projections and Documents가 canonical/derived read basis에서 문서를 만들고 Host and
User Adapter가 preview 또는 user-selected destination을 다룬다. 생성 자체는
canonical mutation이 아니다. 편집본이나 생성물을 Source/Context로 보존하려면
별도의 explicit adoption intent와 Kernel operation이 필요하다.

### Background semantic-provider opt-in

Host and User Adapter가 Project-scoped opt-in intent와 inspectable source scope를
Local Operations에 전달한다. Repository Intelligence만 Optional Semantic Provider
Boundary를 통해 background request를 수행하며 결과를 Derived State 또는
Candidate로 분류한다. Opt-in이 없거나 provider가 unavailable이면 이 path만
비활성화되고 local core journey는 유지된다.

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

## 7. Phase 4 initial implementation boundary

Phase 4의 첫 구현 경계는 **Canonical Context Kernel의 Project–Source identity와
provenance 책임**이다. 이 boundary는 crate나 API 목록이 아니라 다음 observable
responsibility로 정의한다.

- path와 독립적인 Project를 initialize하고 inspect한다.
- user, agent, repository와 command basis를 구분한 Source를 연결한다.
- canonical identity와 local clone binding을 분리한다.
- Repository Intelligence, provider와 host adapter 없이 같은 canonical meaning을
  읽고 invariant를 검증한다.
- Derived State를 만들지 않아도 이 책임을 수행한다.
- legacy runtime, schema, identifier 또는 workflow를 입력으로 사용하지 않는다.

Question, Decision, Context Item과 Checkpoint는 이 foundation 위에서 accepted
순서대로 추가한다. 첫 boundary는 demo용 public slice나 final storage 선택이
아니며, 이후 entity를 미리 빈 shell로 만들 이유가 되지 않는다.

## 8. Accepted Decision traceability

아래 표는 accepted constraint가 architecture의 어디에 들어오는지만 보여 준다.
제품 의미와 revisit trigger의 전체 wording은 `open-decisions.md`가 소유한다.

| Decision | Architecture entry |
|---|---|
| D1 | 별도 logical architecture, clean runtime, legacy dependency 부재와 cutover non-goal |
| D2 | Project-scoped Kernel identity와 Adapter가 보존하는 user/agent/session provenance |
| D3 | Kernel의 portable canonical authority와 Local Operations의 clone binding 분리 |
| D4 | Kernel이 소유하는 여섯 canonical entity와 `domain-model.md`의 단일 정의 |
| D5 | boundary마다 Canonical Context, Session Candidate와 Derived State write authority 분리 |
| D6 | Inquiry and Decision의 material staged frontier entry |
| D7 | exact current host response가 Adapter를 거쳐 Kernel Decision operation에 연결되는 경로 |
| D8 | ordinary work 비차단과 Host/Local Operations의 별도 high-risk confirmation entry |
| D9 | Checkpoint와 projection에서 work, verification, review와 acceptance 독립성 |
| D10 | Projections가 같은 canonical identity/source basis로 역할별 Recall을 만드는 경로 |
| D11 | first-party Repository Intelligence와 Projections를 Kernel에서 분리한 subsystem map |
| D12 | Phase 4 responsibility slice와 replacement journey를 향한 incremental ownership |
| Q1 | Inquiry and Decision 경계와 durable Question basis; 상세 transition은 후속 owner |
| Q2 | polyglot Repository Intelligence extension boundary와 per-area degradation 보존 |
| Q3 | provider-optional graph, local core independence와 explicit background boundary |
| Q4 | Host and User Adapters, Local Operations와 Projections의 surface responsibility 분리 |
| Q5 | read-only document projection, user-selected publication과 explicit adoption 경계 |
| Q6 | portable Project identity와 local binding 분리; bundle/conflict 상세는 `portable-context.md`가 소유 |
| Q7 | Kernel lifecycle authority와 Derived invalidation 경계 |
| Q8-A | Linux/Codex가 첫 Host/Operations acceptance entry이며 logical core와 분리됨 |
| Q8-B | legacy runtime/API/data path가 graph와 import entry에 존재하지 않음 |
| Q9 | 첫 project-scoped bounded read-only Recall을 Projection entry로 배치 |
| Q10 | analyzer/adapter observation을 Candidate로 제한하고 Kernel promotion을 요구 |
| Q11 | source-grounded meaningful boundary만 Kernel Checkpoint operation으로 진입 |
| Q12 | high-risk effect confirmation을 Adapter/Operations concern으로 격리 |
| Q13 | Decision applicability와 revisit 의미를 Kernel/domain이 소유하고 Inquiry가 사용 |

## 9. Non-goals와 열린 implementation choice

이 architecture는 다음을 선택하거나 보증하지 않는다.

- crate/module taxonomy, public/internal API와 MCP method
- process topology, daemon, supervisor와 binary 수
- database, table, transaction, serialization library와 concrete field
- portable bundle format, version number, merge algorithm과 conflict UI
- parser framework, language grammar, indexer, LSP와 semantic protocol
- provider, model, transmission mechanism과 retention implementation
- Question discovery algorithm, ranking, general paraphrase recognition과 atomic host API
- viewer framework, graph layout, renderer와 document template
- repair, encryption, schema upgrade와 production crash strategy
- Wave 1 prototype code 또는 experimental schema의 promotion

이 문서는 first structural language set, user-owned canonical context, local-only
operation과 source-grounded explanation 같은 accepted product contract를 구현
편의 때문에 좁히지 않는다. Evidence gap은 해당 validation 또는 future active
owner가 해결하며, accepted Decision 변경이 필요하면 제품 결정 revisit 절차를
따른다.
