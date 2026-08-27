# Volicord 핵심 도메인 모델

- 상태: active canonical domain owner
- 소유 범위: information class, core entity identity와 relation, provenance,
  promotion, correction, supersession, contradiction, review와 forgetting 의미
- 제품 의미 기준: [제품 헌장](product-charter.md),
  [제품 결정 등록부](open-decisions.md)
- architecture 기준: [논리 아키텍처](architecture.md)
- evidence 기준: [architecture 입력 계약](architecture-inputs.md),
  [Wave 1 결론](../../validation/wave-1-summary.md)
- 비소유 범위: serialized field, storage schema, wire format, merge algorithm,
  UI representation과 specialized subsystem sequence

이 문서는 Volicord가 장기적으로 사용하는 핵심 domain meaning의 단일 기준이다.
여기 적힌 표와 목록은 semantic obligation이며 JSON key, database column, Rust
type layout 또는 public API field를 뜻하지 않는다.

## 1. Domain invariant

다음 구분은 저장 위치나 구현 기술과 무관하게 유지한다.

1. `Canonical Context`, `Session Candidate`, `Derived State`는 서로 다른
   information class다.
2. `user_judgment`, `agent_recommendation`, `observed_fact`, `semantic_result`와
   `generated_interpretation`은 서로 대체할 수 없는 statement role이다.
3. Canonical entity identity는 source locator, display text, local path와 분리된다.
4. Provenance는 record가 어디서 왔는지만이 아니라 누가 어떤 basis로 어떤
   종류의 주장을 만들었는지를 보존한다.
5. Derived State 또는 Session Candidate를 삭제해도 Canonical Context가
   손상되지 않는다.
6. Access frequency, ranking과 retrieval omission은 Decision validity를 바꾸지
   않는다.
7. User는 canonical memory를 inspect, correct, supersede와 forget할 최종
   authority를 가진다.

## 2. Information class

### 2.1 Canonical Context

`Canonical Context`는 미래 session, clone과 environment에서 다시 사용할
portable하고 user-inspectable한 Project memory다. 여섯 core entity와 그 identity,
provenance, revision, relation, supersession와 forgetting 의미를 포함한다.

Canonical Context의 meaning은 Canonical Context Kernel만 확정한다. 다음 actor는
조건을 갖춘 intent 또는 evidence를 제출할 수 있다.

- User는 현재 host의 explicit response나 explicit memory operation을 통해
  Decision, user-authored Context Item, correction, supersession와 forgetting을
  시작할 수 있다.
- Agent는 source-grounded observed fact, material Question과 meaningful
  Checkpoint의 승격을 제안할 수 있다. Agent recommendation이나 hypothesis는
  user Decision으로 제출할 수 없다.
- Repository Intelligence, provider, adapter와 command observer는 직접
  canonical write를 하지 않고 provenance-bearing Candidate 또는 Source basis를
  제공한다.
- Import는 이미 canonical인 new-product record의 identity, provenance와
  lifecycle을 보존해 제출한다. Import 행위가 record의 원저자나 statement role을
  바꾸지 않는다.

Canonical이라는 이름은 영원히 삭제할 수 없거나 항상 현재에 유효하다는 뜻이
아니다. Canonical record도 superseded, contradicted, review due 또는 forgotten일
수 있으며 그 상태를 정직하게 보여 준다.

### 2.2 Session Candidate

`Session Candidate`는 현재 조사와 작업에서 생겼지만 장기 Project memory로
승격되지 않은 잠정 정보다. Observation, hypothesis, semantic claim, possible
Question, possible Checkpoint와 promotion proposal을 포함할 수 있다.

`Materiality Review`도 Session Candidate다. 현재 user-stated Goal, exact retained
pre-work Analysis Snapshot과 independently material한 outcome dimension을 묶어 각
dimension의 fact/settled authority/delegation/exploration/user-owned disposition과 bounded
evidence basis를 보존한다. 이 review는 일곱 번째 canonical entity, user Decision 또는
ordinary-write permission이 아니다. 첫 authoritative review가 meaningful repository
mutation 전이었는지와 이후 evidence revision을 구분하며, restart 뒤에도 unresolved
dimension을 completed authority로 재분류하지 않는다.

Agent, Repository Intelligence, Inquiry and Decision, Host and User Adapter, Local
Operations와 Optional Semantic Provider Boundary는 각자의 provenance를 가진
Candidate를 만들 수 있다. User input도 아직 exact Question linkage나 explicit
adoption이 확인되지 않았다면 Candidate로 머물 수 있다.

Candidate는 local이고 bounded하며 collection을 끌 수 있어야 한다. Candidate의
retention expiry, access frequency 또는 삭제는 Canonical Context를 수정하지
않는다. Candidate가 오래 존재하거나 자주 검색됐다는 사실만으로 승격되지
않는다.

#### Candidate inspection metadata contract

각 Candidate는 local Project context 안에서 다음 user-inspectable meaning을 가진다.
이 항목은 storage field가 아니라 Candidate의 existence와 처리 상태를 정직하게
설명하기 위한 최소 domain contract다.

| Attribute | Required meaning |
|---|---|
| `candidate_identity` | local Project context 안에서 Candidate를 다른 Candidate와 구분하는 identity |
| `candidate_kind` | observation, hypothesis, semantic claim, Question candidate, Checkpoint candidate, Materiality Review 또는 promotion proposal 같은 잠정 정보의 종류 |
| `origin_and_provenance` | 생성 actor/subsystem/session과 repository, command, host turn, provider 또는 generated basis |
| `collection_scope` | 수집이 허용된 Project/session/source/operation 또는 더 좁은 scope |
| `creation_or_observation_basis` | 생성/관찰 시각과 사용한 Source, snapshot, execution 또는 request basis |
| `retention_or_expiry_state` | 적용 retention policy, retained-until/expiry basis와 cleanup 여부 |
| `promotion_disposition` | 아직 미처리인지, promoted/dismissed/expired인지와 target/result basis |
| `scope_opt_out_state` | 이 Candidate의 collection scope에 적용되는 opt-out 상태와 effective basis |

Candidate identity는 canonical entity identity가 아니며 portable bundle identity로
일반화하지 않는다. Origin이나 Source body를 보여 준다는 뜻도 아니다. Full prompt,
full tool argument, full Source body와 unlimited stdout/stderr를 기본 장기 content로
보존하지 않으면서 위 provenance와 bounded observation basis를 제공한다.

#### Candidate lifecycle and disposition contract

Candidate lifecycle은 최소 다음 disposition을 구분한다.

| Disposition | Meaning |
|---|---|
| `pending_or_retained` | promotion/dismissal/expiry가 완료되지 않았고 현재 retention policy 안에서 inspectable함 |
| `promoted` | explicit promotion operation의 canonical target/result가 기록됨; Candidate retention/deletion은 별도 사실임 |
| `dismissed` | Candidate를 canonical로 사용하지 않기로 명시적으로 disposition했으며 Decision이나 fact를 삭제한 뜻이 아님 |
| `expired_or_retention_cleaned` | retention/expiry policy로 Candidate content가 제거됐고 canonical target을 만들지 않았으며 최소 non-content cleanup basis만 표시할 수 있음 |

Disposition transition은 actor, time과 operation/policy basis를 가진다. Opt-out은
disposition이 아니며 기존 Candidate를 promoted, dismissed 또는 expired로 조용히
바꾸지 않는다. Promotion, dismissal과 expiry가 Candidate의 canonical basis를
rewrite하지 않으며 `promoted` Candidate가 canonical target과 같은 identity를 얻지
않는다.

### 2.3 Derived State

`Derived State`는 authoritative Source와 Canonical Context에서 다시 계산할 수
있는 정보다. Inventory/index, code graph, embedding, fingerprint, ranking, cached
summary, semantic annotation, generated preview와 layout 등이 여기에 속한다.

Repository Intelligence, Optional Semantic Provider Boundary, Projections and
Documents와 Local Operations는 자신의 책임 범위에서 Derived State를 만들 수
있다. Derived result가 user-visible해도 canonical truth가 되지 않으며, provider가
높은 confidence를 표시해도 user judgment나 observed fact를 대신하지 않는다.

Derived State는 전체 삭제와 rebuild가 가능해야 한다. 삭제 후 Canonical Context를
읽고 수정할 수 있어야 하며, rebuild는 user correction, Decision, supersession와
forgetting을 되돌리지 않는다.

`Guarded Effect Candidate`는 Local Operations가 dispatch를 검토하는 exact effect의
Derived 또는 operational state다. Action, target, expected effect, risk와 scope를
분류할 수 있지만 canonical core entity나 general product Decision이 아니다. 관련
confirmation request/response와 operation observation도 그 자체로 일곱 번째 canonical
core entity가 되지 않는다. Durable Project history가 필요하면 explicit user-response
`Source`와 resulting operation을 `Checkpoint` 또는 `Context Item`이 reference할 수
있으며, 이때도 operational confirmation을 Decision으로 바꾸지 않는다.

## 3. Statement role과 judgment boundary

| Role | 의미 | Canonical 가능성 | 금지되는 대체 |
|---|---|---|---|
| `user_judgment` | User가 표시된 Question에 현재 host에서 명시적으로 한 선택 또는 위임 | exact response linkage를 만족한 Decision | agent recommendation, preference inference나 일반 동의를 user choice로 사용 |
| `agent_recommendation` | Agent가 source와 trade-off를 바탕으로 권하는 방향 | provenance-bearing Question/Context basis로 보존 가능 | Decision choice 또는 user rationale로 저장 |
| `observed_fact` | Repository, Git, command 또는 environment에서 직접 확인한 사실 | snapshot과 observer provenance가 있는 Context Item 가능 | inference나 generated text를 관찰 사실로 표시 |
| `semantic_result` | analyzer/provider가 특정 snapshot에서 계산한 semantic relation 또는 annotation | 기본적으로 Derived State; explicit adoption 시에도 원 provenance 유지 | user judgment 또는 parser-confirmed fact로 표시 |
| `generated_interpretation` | Agent/model/renderer가 여러 source를 해석해 만든 설명 | 기본적으로 Candidate/Derived; explicit adoption 시 interpretation임을 유지 | underlying Source나 Decision을 대신하는 canonical truth로 표시 |

User가 recommendation이나 interpretation을 채택해 Context Item으로 보존해도
그 원래 role과 generator provenance는 사라지지 않는다. Adoption은 내용을
parser fact로 변환하는 행위가 아니다.

## 4. Canonical entity

### 4.1 Project

`Project`는 여러 session, clone과 computer에서 공유되는 context의 stable identity다.
초기화 시 생성되며 repository path나 remote URL에서 추론하지 않는다. Project는
canonical entity의 ownership boundary이고 portable context의 기준이다.

Local clone binding은 Project identity 자체가 아니다. 하나의 Project가 여러
binding을 가질 수 있고 source repository가 unavailable해도 Project와 canonical
record는 읽을 수 있다. Project divergence, branch와 binding procedure의 상세 의미는
active [Portable Context 계약](portable-context.md)이 구체화한다.

### 4.2 Source

`Source`는 statement, Decision, Checkpoint 또는 explanation의 근거를 식별한다.
파일, symbol, repository snapshot, commit, command execution, URL, host turn과
adopted artifact처럼 서로 다른 source kind를 구분한다.

Source identity는 원문 전체를 저장한다는 뜻이 아니다. Source는 가능한 범위에서
원래 위치와 snapshot을 다시 확인하게 하고, unavailable하거나 stale해져도
historical reference identity를 유지한다. Local path rebinding은 locator
availability를 바꾸지만 portable Source identity나 과거 snapshot basis를 바꾸지
않는다.

### 4.3 Question

`Question`은 Project 결과를 material하게 바꿀 수 있어 아직 user judgment,
delegation, research, prototype, deferment, exclusion 또는 supersession이 필요한
판단 지점이다. Question identity는 표시 문구와 분리되며 correction revision
후에도 같은 material question을 가리킬 수 있다.

Question은 다른 Question에 `depends_on`할 수 있고 여러 independent Question이
같은 current frontier에 있을 수 있다. Terminal outcome은 Question이 더 이상
현재 답변을 기다리지 않는 이유를 나타내며, 모든 terminal outcome이 Decision을
만든다는 뜻은 아니다. Frontier 계산과 transition의 상세 contract는 active
[Inquiry와 Decision 계약](inquiry-and-decision.md)이 소유한다.

### 4.4 Decision

`Decision`은 Question에 대한 User의 명시적 선택 또는 명시적 위임과 그 당시의
적용 의미다. Decision은 agent recommendation, observed fact, research result,
provider output 또는 inferred preference가 아니다.

User Decision은 exact Question identity와 displayed revision, current host의
user-turn Source에 연결된다. 의미가 바뀐 선택은 기존 Decision을 in-place edit하지
않고 새 Decision으로 supersede한다. Research나 prototype으로 branch가 끝난 경우
그 결과는 Source/Context가 될 수 있지만 user choice가 없으면 Decision으로
위조하지 않는다.

### 4.5 Context Item

`Context Item`은 Project의 goal, fact, assumption, constraint, explicit preference,
risk, learning과 known limit를 보존한다. 각 Context Item은 statement role과
provenance를 유지하므로 user-stated constraint, observed fact와 generated
interpretation을 같은 종류의 truth로 합치지 않는다.

Context Item은 Decision의 rationale나 applicability basis가 될 수 있지만
Decision을 대신하지 않는다. Source와 충돌한 fact 또는 assumption은 조용히
rewrite하지 않고 contradiction/review semantics를 따른다.

### 4.6 Checkpoint

`Checkpoint`는 meaningful work completion, pause 또는 handoff 시점의
source-grounded Project state observation이다. 현재 목표, 의미 있는 변화, 적용한
Decision, verification, known limits, open Question과 next step을 함께 복구하는
기준을 제공한다.

Checkpoint는 work를 시작하거나 완료하도록 허가하는 state machine이 아니며
Inquiry frontier의 두 번째 authority도 아니다. 단순 조회, 변경 없는 설명,
source 없는 추측 summary와 unrelated dirty change는 canonical Checkpoint의 basis가
되지 않는다.

## 5. Identity model

모든 canonical entity는 다음 identity 원칙을 따른다.

- Identity는 Project 안에서 stable하며 display name, natural-language text,
  current path와 storage location 변경으로 재사용되지 않는다.
- Correction revision은 같은 entity identity의 history다.
- Semantic supersession은 새 entity identity와 명시적 relation을 만든다.
- Import/export는 identity를 보존하며 destination에서 새 identity를 조용히
  발급하지 않는다.
- 같은 text나 locator는 identity equality의 충분한 근거가 아니다.
- Forgotten content의 identity 보존 여부는 referential integrity와 privacy에
  필요한 최소 범위로 제한하며 원문이나 recoverable hash를 요구하지 않는다.

Derived State와 Session Candidate는 canonical entity identity와 구분되는 identity를
사용한다. Stable analysis identity가 있더라도 canonical authority를 얻지는 않는다.
구체적인 ID format과 fingerprint algorithm은 implementation choice다.

## 6. Provenance model

Provenance는 actor, Source basis, creation context와 statement role을 함께 설명해야
한다. 다음 provenance 종류는 합쳐지지 않는다.

### User provenance

User-authored record는 어느 current host interaction의 어떤 user-turn Source에서
명시되었는지 추적할 수 있어야 한다. Question에 대한 Decision이면 exact Question
identity와 displayed revision도 함께 연결한다. Agent가 과거 대화나 preference를
해석한 것은 user provenance가 아니다.

### Agent provenance

Agent-authored record는 agent identity, host와 session context, 사용한 Source basis와
statement role을 구분한다. Agent는 recommendation, observation proposal,
Checkpoint 또는 generated interpretation의 author가 될 수 있지만 User로
표시되거나 스스로 user Decision을 만들 수 없다.

### Repository provenance

Repository Source는 Project binding, repository/source snapshot, revision 또는
fingerprint, locator/range와 coverage basis를 추적할 수 있어야 한다. 현재 clone의
path가 바뀌어도 historical snapshot provenance를 rewrite하지 않는다. Parser가
확인한 fact와 agent interpretation은 같은 repository Source를 보더라도 role이
다르다.

### Command provenance

Command Source는 어떤 Project/work context와 environment에서 실제 execution을
관찰했는지, outcome, exit/termination과 채택된 bounded output basis를 확인할 수
있어야 한다. Checkpoint verification은 existing Command Source identity를 canonical
execution identity로 사용하며 별도 execution entity를 만들지 않는다. 실제 실행을
보고하는 current host는 bounded human-readable `command label`과 exact transient command
invocation을 분리해 제출하고, trusted Volicord operation은 exact invocation UTF-8 bytes의
SHA-256 fingerprint를 derive한다. Command Source는 이 fingerprint, label과 observed
exit/termination을 보존하므로 machine correlation은 label text에 의존하지 않는다.
실행하지 않은 command를 verification Source로 만들거나 caller가 asserted digest만 제출해
execution provenance를 충족할 수 없다. Exact invocation/raw argument, stdout와 stderr 전체는
기본 canonical content가 아니며, fingerprint derivation 뒤 durable state로 전달하지 않는다.
필요한 결과를 명시적으로 채택해도 execution provenance는 유지한다.

### Provider provenance

Provider result는 provider와 model identity, source snapshot과 included scope,
generation context, freshness와 uncertainty를 확인할 수 있어야 한다. Provider
provenance는 User 또는 Repository observer provenance로 바뀌지 않는다. Detailed
transmission, retention과 revoke contract는 active
[Privacy와 provider 경계 계약](privacy-and-provider-boundary.md)이 소유한다.

### Generated explanation provenance

Generated explanation은 generator identity, 사용한 Source/Decision, source snapshot,
capability coverage, generation context, known gap와 uncertainty를 추적할 수 있어야
한다. 설명이 정확해 보여도 explicit adoption 전에는 canonical truth가 아니며,
adoption 후에도 generated interpretation이라는 origin을 유지한다.

### Imported record provenance

Imported canonical record는 origin Project와 record identity, author/Source basis,
revision과 supersession/forgetting 상태를 보존한다. Import event와 local binding은
추가 provenance가 될 수 있지만 원래 provenance를 대체하지 않는다. Legacy runtime,
schema와 record를 canonical import source로 해석하는 경로는 없다.

## 7. Candidate promotion

Promotion은 information class를 조용히 바꾸는 copy가 아니라 Canonical Context
Kernel이 새 canonical meaning을 검증하는 explicit domain operation이다. 모든
promotion에는 다음 조건이 필요하다.

- destination canonical entity와 statement role이 명확하다.
- actor와 Source provenance를 확인할 수 있다.
- Candidate의 uncertainty, coverage와 생성 origin을 보존한다.
- 같은 material meaning의 existing record, correction과 supersession 관계를
  확인한다.
- user authority가 필요한 promotion은 current explicit user intent를 가진다.
- promotion 성공과 Candidate retention/deletion은 별도 사실이다.

정보 종류별 최소 조건은 다음과 같다.

| Candidate | 가능한 canonical 결과 | Promotion requirement |
|---|---|---|
| explicit user response | Decision | exact Question identity/revision, current user-turn Source와 explicit choice/delegation |
| user-stated goal/constraint/preference | Context Item | user-turn Source와 statement role 보존 |
| repository/Git/command observation | Context Item `observed_fact` | snapshot/execution Source, observer provenance, coverage와 uncertainty |
| material Question candidate | Question | materiality 검토, Project scope, known prerequisite와 source basis |
| meaningful work result | Checkpoint | changed/source basis, applied Decision or reason, verification, known limits와 next step |
| agent recommendation | Question 또는 Context Item의 recommendation basis | agent/Source provenance; user choice와 분리 |
| agent/provider interpretation | Context Item 또는 preserved Source로 explicit adoption 가능 | interpretation/semantic role, generator/provider provenance와 uncertainty 유지 |
| imported canonical record | 같은 canonical entity kind | supported new-product format, identity/history/origin 보존과 conflict 검토 |

Access frequency, age, ranking score, model confidence와 repeated retrieval은
promotion authorization이 아니다. Repository Intelligence, provider, adapter,
projection과 renderer는 promotion을 직접 완료하지 않는다.

## 8. Canonical relation

다음 relation-direction contract는 domain meaning을 안정적으로 표현한다. `From`이
relation의 출발이고 `To`가 화살표가 가리키는 대상이다. Storage edge나 concrete
field를 요구하지 않으며 같은 relation name의 reverse alias를 허용하지 않는다.

| Relation | From | To | Domain meaning |
|---|---|---|---|
| `belongs_to` | Source, Question, Decision, Context Item 또는 Checkpoint | Project | record가 어떤 Project context에 속하는지 연결 |
| `supported_by` | statement-bearing record, Decision rationale 또는 Checkpoint observation | supporting Source | statement/rationale/observation이 자신의 Source basis를 참조 |
| `depends_on` | dependent Question | prerequisite Question/outcome | Question이 material하게 열리기 위한 prerequisite를 참조 |
| `answers` | Decision | exact Question revision | Decision이 exact Question revision에 대한 user choice임을 연결 |
| `applies_to` | Decision | Project, path, component 또는 work context | Decision의 적용 범위를 연결 |
| `assumes` | Decision | explicit assumption | Decision 재사용의 전제를 연결 |
| `supersedes` | new semantic record | previous semantic record | 새 record가 이전 record의 현재 의미를 대체함을 연결 |
| `contradicts` | source-grounded claim 또는 Decision applicability basis | conflicting claim/basis | 해결되지 않은 충돌을 연결 |
| `records_state_of` | Checkpoint | work/context boundary | Checkpoint가 특정 boundary의 관찰을 기록 |
| `derived_from` | Session Candidate, Derived State 또는 generated explanation | Source 또는 canonical basis used | derived/candidate/explanation이 실제 사용한 basis를 참조 |

Relation은 ownership을 이전하지 않는다. 예를 들어 analysis가 `Decision`을
`derived_from`으로 참조해도 Decision validity를 변경하지 않으며, Checkpoint가
Question을 나열해도 frontier authority가 되지 않는다.

Core entity의 기본 관계는 다음과 같다.

```text
Project
├─ Context Item / Question / Decision / Checkpoint observation ──supported_by──▶ Source
├─ Session Candidate / Derived State / generated explanation ─────derived_from──▶ Source / canonical basis
├─ Question ──depends_on───────────────▶ Question
├─ Decision ──answers──────────────────▶ Question revision
├─ Decision ──applies_to───────────────▶ Project / path / component / work context
│           └─supersedes───────────────▶ Decision
└─ Checkpoint ──records_state_of───────▶ work/context boundary
```

## 9. Lifecycle semantics

### `correction_revision`

오탈자, 표현, 형식 또는 의미를 바꾸지 않는 보정이다. 같은 entity identity를
유지하고 이전 revision을 history로 보존한다. Correction은 author나 Source
provenance를 rewrite하지 않으며 semantic conflict를 숨기는 수단이 아니다.

### `semantic_supersession`

선택, 주장 또는 판단 의미가 바뀌어 새 canonical entity가 이전 entity를 대체하는
관계다. 새 identity와 `supersedes` relation을 만들고 과거 record를 당시 history로
보존한다. Decision 의미 변경은 항상 이 경로를 사용한다.

### `contradiction`

두 source-grounded claim, 또는 Decision의 source/assumption basis와 현재 evidence가
함께 참일 수 없거나 중요한 불일치를 보이는 상태다. `contradiction`은 어느 쪽이
자동으로 승리했다는 뜻이 아니며 source를 조용히 merge하거나 rewrite하지 않는다.
Resolution은 correction, new evidence, supersession 또는 user review로 이어질 수
있다.

### `review_due`

Decision의 scope, assumption, source freshness, revisit trigger 또는 observed
consequence가 바뀌어 현재 적용을 다시 검토해야 한다는 상태다. `review_due`는
자동 expiry나 자동 invalidation이 아니며 새로운 user Decision도 아니다. Review
결과가 의미를 바꾸면 supersession을 사용한다.

### `forgetting`

User authority로 canonical content를 더 이상 보존·제공하지 않도록 삭제하는
operation이다. Forgetting은 correction이나 supersession과 달리 원문 history를
읽을 수 있게 유지하지 않는다. Referential integrity에 최소 tombstone이 필요할
수 있지만 민감 원문이나 recoverable hash를 포함해서는 안 된다. Derived State,
Candidate와 managed projection의 관련 content도 invalidate/delete 대상이지만,
그 identity와 lifecycle meaning은 각 owner에 남는다. Canonical forgetting success는
Kernel tombstone만으로 성립하지 않는다. Local Operations가 같은 invalidation을
Inquiry와 Privacy owner에 전달하고, 관련 Candidate와 managed Derived content가
삭제되었으며 local destructive-residue postcondition이 확인되어야 complete다.
Canonical commit 뒤 이 후속 조건이 미완료이면 operation은 `repair_required`이고,
read barrier가 관련 non-canonical content를 숨긴다. 이 barrier는 Candidate disposition을
promotion, dismissal 또는 expiry로 바꾸거나 managed Derived record를 canonical
history로 재분류하지 않는다. Provider-side deletion outcome은 이 local completion과
별도 사실이다.

이 다섯 의미는 서로 대체할 수 없다. 특히 오래 사용하지 않은 Decision을
forgetting 처리하거나, source가 stale하다는 이유만으로 supersede하거나, 의미가
바뀐 Decision을 correction revision으로 숨기지 않는다.

## 10. Decision applicability와 validity

Decision은 다음 basis가 함께 맞을 때 재사용할 수 있다.

- 같은 Project
- 선언된 path, component 또는 work context scope
- 유지되는 assumptions
- 확인 가능한 source basis와 그 freshness/availability
- 충족되지 않은 revisit trigger가 없음
- unresolved contradiction이나 후속 conflicting Decision이 없음
- superseded 또는 forgotten 상태가 아님

Decision은 당시 option, agent recommendation, user rationale, expected consequence,
known uncertainty와 revisit trigger를 구분해 보존한다. 이 항목은 Decision의 의미
일부지만 concrete serialized field를 지정하지 않는다.

Applicability가 불확실하거나 중요한 basis가 바뀌면 `review_due`로 다루고 Inquiry가
재검토 이유를 제시할 수 있다. Access frequency, 마지막 조회 시각, ranking,
projection omission 또는 cache eviction은 Decision validity와 applicability를
바꾸지 않는다.

## 11. Source availability, freshness와 snapshot binding

Source의 현재 사용 가능성은 최소 다음 의미를 구분한다.

- `available`: referenced content 또는 observation basis를 현재 확인할 수 있음
- `unavailable`: Source identity는 남아 있지만 현재 environment에서 content를
  확인할 수 없음
- `stale`: Source는 확인 가능하지만 record나 analysis가 참조한 snapshot보다
  달라 현재 적용에 freshness 검토가 필요함
- `unknown`: availability 또는 freshness를 아직 판정하지 못함

Availability는 historical identity를 삭제하지 않고, freshness는 과거 observation을
거짓으로 rewrite하지 않는다. 모든 repository fact, semantic result와 generated
explanation은 자신이 근거한 source snapshot에 bound된다. 새 snapshot은 기존
result를 자동 current로 만들지 않으며 stale Derived State를 재계산하거나 관련
Decision을 review_due로 제안할 수 있다.

Source repository가 unavailable해도 Project, Decision과 Checkpoint는 읽을 수 있다.
그 상태에서 code relation이나 current applicability를 확인할 수 없다는 사실을
명시하며, unavailable을 empty source나 successful freshness check로 표현하지
않는다.

## 12. Work, verification, review와 acceptance

Checkpoint와 Recall은 다음 fact dimension을 하나의 lifecycle로 합치지 않는다.

| Dimension | Stable state meaning |
|---|---|
| Work | `in_progress`, `paused`, `completed`, `abandoned`, `superseded` |
| Automated verification | `not_run`, `partial`, `passed`, `failed` |
| User review | `not_requested`, `pending`, `reviewed` |
| User acceptance | `not_requested`, `pending`, `accepted`, `rejected` |

각 fact는 자신의 actor, Source와 observed time basis를 가진다. `completed` work가
`not_run` verification일 수 있고, verification이 `passed`여도 user review나
acceptance를 뜻하지 않는다. User acceptance가 없다고 work를 자동 incomplete로
바꾸지 않으며, user가 accepted했다고 실패한 verification을 passed로 바꾸지
않는다. Known limit와 unverified area도 별도로 보존한다.

Executed verification의 `source_id`는 정확히 하나의 canonical Command Source를 가리키며,
그 Source의 invocation fingerprint와 exit/termination이 실행 correlation과 outcome
truth를 제공한다. `not_run` verification은 Source, invocation material, fingerprint 또는
execution outcome을 가질 수 없다.

## 13. User authority

User는 자신의 Project canonical memory에 대해 다음 authority를 가진다.

- `inspect`: record identity, statement role, Source, revision, applicability,
  freshness, supersession와 omission basis를 확인한다.
- `correct`: 의미를 바꾸지 않는 오류를 correction revision으로 고친다.
- `supersede`: 과거 판단이나 의미를 새 canonical record로 대체한다.
- `forget`: canonical content와 관련 managed Candidate/Derived content를 삭제한다.

Agent와 subsystem은 이 operation을 제안하고 consequence를 설명할 수 있지만 user
authority가 필요한 operation을 access frequency, source refresh, provider output
또는 inferred preference로 자동 실행하지 않는다. User correction 뒤 자동
analysis가 원래 text나 claim을 다시 canonical로 복원할 수 없다.

## 14. Specialized owner boundary

이 문서는 다음 상세 contract를 정의하지 않는다.

- Repository snapshot, code entity/relation, capability, adapter와 invalidation:
  active [Repository Intelligence 계약](repository-intelligence.md)
- Question frontier, response, terminal transition, Recall/Checkpoint interaction:
  active [Inquiry와 Decision 계약](inquiry-and-decision.md)
- Bundle content, clone binding procedure, divergence, conflict와 resolution:
  active [Portable Context 계약](portable-context.md)
- Provider transmission, privacy, retention, revoke와 deletion completeness:
  active [Privacy와 provider 경계 계약](privacy-and-provider-boundary.md)
- Recall/view/document selection, grounding, rendering과 adoption:
  active [Projection과 document 계약](projections-and-documents.md)
- Transaction, crash, degraded mode, repair와 rebuild:
  active [Failure와 Recovery 계약](failure-and-recovery.md)
- Schema/format version과 new-product upgrade behavior:
  active [Versioning 정책](versioning-policy.md)

이 owner들은 `architecture-inputs.md`의 active ownership plan을 따른다. Specialized
문서는 이 문서의 information class, core identity, provenance, relation과 lifecycle
meaning을 재정의하지 않고 자기 named domain만 구체화한다.
