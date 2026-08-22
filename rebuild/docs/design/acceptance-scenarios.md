# 재구축 실사용 Acceptance 시나리오

- 상태: 제품 결정 반영 기준선
- 목적: 기존 절차 수행 여부가 아니라 사용자의 이해·판단·기억·재개와 실제 사용 가능성을 검증
- 첫 공식 환경: Linux와 Codex
- 적용: 설치, Canonical Context, Repository Intelligence, Inquiry, Checkpoint, Recall, UI, 문서와 portable context
- 기술 검증 세부사항: `validation-plan.md`

## 1. 기존 기준과의 관계

기존 SignalBox 시나리오는 Task intake, CLI 전용 UserAction resolution, Change Unit, Write Ticket, Run, Evidence, final acceptance와 close ceremony를 성공 조건으로 삼았다. 이 흐름은 새 제품의 active acceptance contract가 아니다.

다만 다음 실패 방지 원칙은 보존한다.

- 모호한 대화를 사용자 판단으로 위조하지 않는다.
- 기존 dirty change를 현재 작업 결과로 잘못 귀속하지 않는다.
- 실행하지 않은 검증이나 관찰하지 않은 결과를 성공으로 주장하지 않는다.
- cooperative 기록을 OS 수준 보안 강제로 표현하지 않는다.
- 부분 실패, source 부재와 분석 누락을 숨기지 않는다.
- 사용자의 선택과 agent recommendation을 구분한다.

## 2. 공통 통과 원칙

모든 시나리오는 다음을 만족해야 한다.

1. 사용자는 같은 판단을 다른 인터페이스에서 반복하지 않는다.
2. agent recommendation, 사용자 Decision, observed fact와 generated explanation이 구분된다.
3. 일반 파일 수정에 Volicord 사전 허가가 필요하지 않다.
4. Canonical Context는 사용자가 볼 수 있고 수정·supersede·삭제할 수 있다.
5. Derived State를 삭제하거나 재구축해도 Canonical Context가 유지된다.
6. 코드 설명은 source snapshot, capability, coverage, freshness와 known gaps를 표시한다.
7. 새 세션은 결과뿐 아니라 Decision 이유와 열린 Question을 복구한다.
8. 작업 완료, 자동 검증, 사용자 검토와 수락을 독립적으로 표현한다.
9. Context 또는 분석 기능 실패가 실제 repository 작업 결과를 왜곡하지 않는다.
10. 고위험 effect에만 더 강한 confirmation을 요구한다.
11. 지원하지 않는 capability를 제공한 것처럼 표시하지 않는다.
12. legacy Runtime Home, API와 데이터는 입력·호환·마이그레이션 대상으로 취급하지 않는다.

## 3. Capability와 coverage 판정

Repository Intelligence acceptance는 저장소 전체에 하나의 `supported` 값을 사용하지 않는다.

| Capability | Acceptance 의미 |
|---|---|
| `inventory` | 파일, 언어, manifest, config, 문서, Git와 분석 경계를 식별 |
| `agent_assisted` | source-grounded 설명과 질의응답을 제공하며 해석임을 표시 |
| `structural` | parser가 entity, source range와 구문 관계를 재현 가능하게 제공 |
| `semantic` | definition, reference, type와 implementation 관계를 제공 |
| `ecosystem` | build, package, workspace와 toolchain 문맥을 반영 |

각 capability에는 다음 상태가 있어야 한다.

```text
available
partial
unavailable
failed
stale
```

`partial`, `unavailable`, `failed`에는 이유와 영향을 표시한다. 분석 결과는 최소 다음 basis를 가진다.

- Project ID
- repository/source snapshot
- analyzed revision 또는 fingerprint
- covered files와 entities
- excluded, unsupported와 failed areas
- analyzer/provider identity
- generated_at
- freshness

## 4. 첫 공식 fixture matrix

첫 replacement gate는 다음 fixture를 포함한다. 실제 fixture 이름은 구현 중 달라질 수 있지만 언어와 요구사항을 축소해서는 안 된다.

| Fixture | 최소 포함 요소 | 필수 capability |
|---|---|---|
| Java Maven/Gradle | package, class, interface, method, inheritance, implementation, test, multi-module 중 하나 | inventory, agent_assisted, structural, ecosystem |
| Python `pyproject.toml` | package, module, function, class, import, test, dynamic relation 한계 | inventory, agent_assisted, structural, ecosystem |
| JavaScript Node | ES module과 CommonJS 중 하나, function, class, import/export, test | inventory, agent_assisted, structural, ecosystem |
| TypeScript Node/monorepo | type, interface, class, function, import/export, `tsconfig`, workspace package | inventory, agent_assisted, structural, ecosystem |
| C CMake | header/source, function, macro 또는 conditional compilation, test | inventory, agent_assisted, structural, ecosystem |
| C++ CMake/compilation database | namespace, class, method, template, header/source, test | inventory, agent_assisted, structural, ecosystem |
| Rust Cargo workspace | crate, module, struct, enum, trait, impl, function, test, feature 또는 cfg | inventory, agent_assisted, structural, ecosystem |
| Polyglot repository | 최소 세 언어, shared config 또는 inter-process/API boundary, 문서 | 언어별 matrix와 repository-wide explanation |
| Non-gate language repository | 첫 structural 목록 밖의 텍스트 언어 | inventory, 가능한 agent_assisted, unavailable capability 표시 |

첫 replacement gate에서는 Java, Python, JavaScript, TypeScript, C, C++와 Rust 전부에 `structural` capability가 필요하다. 최소 세 ecosystem에서 `semantic` capability를 통과해야 한다. 어떤 세 ecosystem을 선택했는지와 근거는 검증 보고서에 기록한다.

## 5. 시나리오 형식

각 시나리오는 다음 항목을 가진다.

- 시작 상태
- 사용자 요청
- 기대 사용자 경험
- Canonical Context 변화
- Derived State 변화
- 금지 행동
- 자동 검증
- 수동 평가

## A. Linux clean install과 Codex 연결

### 시작 상태

- 지원 Linux 환경
- Volicord가 설치되지 않음
- 새 Git 저장소 또는 fixture repository
- 새 제품 Runtime Home 없음
- legacy Runtime Home은 acceptance 입력으로 제공하지 않음

### 사용자 요청

Volicord를 설치하고 현재 저장소를 Project로 초기화한 뒤 Codex와 연결한다.

### 기대 사용자 경험

- 설치 명령, binary 위치와 결과가 명확하다.
- Codex adapter와 MCP health를 확인할 수 있다.
- stable Project ID와 현재 clone binding이 만들어진다.
- Canonical, Candidate와 Derived data의 저장 위치를 확인할 수 있다.
- background semantic analysis가 기본적으로 꺼져 있음을 확인한다.
- health 결과는 success, degraded와 failure를 구분한다.
- 한국어와 영어 고정 UI 또는 CLI 출력이 정상적으로 표시된다.

### Canonical Context 변화

- Project identity
- clone/repository source binding
- portable해야 하는 사용자 설정

### Derived State 변화

- 초기 repository inventory 또는 빈 analysis state
- adapter health cache

### 금지 행동

- 기존 Runtime Home을 찾거나 읽음
- source를 사용자 opt-in 없이 background provider에 전송
- 연결 실패를 성공으로 보고
- Product Repository에 tracked identity marker 자동 생성

### 자동 검증

- clean install smoke test
- process restart 후 Project ID 유지
- Codex connection health
- runtime path와 Product Repository 분리
- reinstall 후 clean project journey

### 수동 평가

- 사용자가 무엇이 저장되고 외부로 전달될 수 있는지 이해할 수 있는가
- 첫 설정이 기존 authority workflow를 요구하지 않는가

## B. 모든 텍스트 저장소의 Inventory와 agent-assisted fallback

### 시작 상태

- Project가 연결됨
- 첫 structural gate 밖의 언어를 포함한 텍스트 repository
- semantic background provider 꺼짐

### 사용자 요청

저장소의 목적, 파일 구조, manifest, 문서와 주요 진입점 후보를 설명한다.

### 기대 사용자 경험

- 언어와 파일 형식 inventory를 제공한다.
- manifest, config, README와 Git source를 탐색한다.
- Codex가 읽을 수 있는 source에 대해 best-effort 설명을 제공한다.
- 구조적으로 검증되지 않은 설명은 `agent_assisted` 해석으로 표시한다.
- structural, semantic 또는 ecosystem capability가 없으면 정확히 표시한다.
- repository 등록, Decision, Checkpoint와 Recall은 계속 사용할 수 있다.

### Canonical Context 변화

- 사용자가 명시적으로 저장하지 않는 한 분석 결과 자체는 canonical이 아님
- 중요한 fact 또는 Question은 Candidate로 제안 가능

### Derived State 변화

- repository inventory
- language detection
- source fingerprints
- 선택적 agent annotations

### 금지 행동

- analyzer가 없다는 이유로 Project 사용 전체를 거부
- agent 해석을 parser fact로 저장
- 파일 일부만 읽고 전체 coverage를 주장

### 자동 검증

- 파일·언어·manifest count
- exclude와 binary/vendor handling
- source citation
- capability matrix schema

### 수동 평가

- 사용자는 어느 설명이 확인된 사실이고 어느 부분이 해석인지 구분할 수 있는가

## C. 다중 언어 Structural analysis

### 시작 상태

- fixture matrix의 각 공식 언어 repository
- 해당 structural analyzer 사용 가능
- semantic provider는 없어도 됨

### 사용자 요청

저장소 구조와 주요 entity·관계를 분석하고 source로 이동한다.

### 기대 사용자 경험

- 언어와 ecosystem에 맞는 package, module, type, function, method와 test를 찾는다.
- entity마다 stable identity, source file와 range를 제공한다.
- imports, contains, declares, implements 또는 syntax-level calls 등 지원 관계를 표시한다.
- macro, generated code, dynamic dispatch와 build-context 한계를 별도로 표시한다.
- 같은 snapshot에서 구조 결과가 안정적이다.
- 변경된 파일만 재분석하고 영향을 받은 derived record를 stale 또는 갱신한다.

### Canonical Context 변화

- 없음. 사용자가 특정 발견을 채택하면 Source-linked Context Item 가능

### Derived State 변화

- Analysis Snapshot
- Code Entity
- Structural Relation
- capability coverage
- fingerprint와 stale state

### 금지 행동

- 모든 언어에 동일한 entity·relation을 억지로 적용
- compile 또는 build context가 없는데 semantic completeness 주장
- parser 실패를 빈 성공 결과로 처리
- JavaScript와 TypeScript 또는 C와 C++의 실제 차이를 숨김

### 자동 검증

각 fixture에서 다음을 확인한다.

- known declarations와 source ranges
- package/module/file hierarchy
- imports 또는 includes
- test entity
- syntax error와 partial parse behavior
- deterministic serialization
- incremental update
- coverage와 failed construct report

### 수동 평가

- 구조 graph가 실제 code navigation과 설명에 유용한가
- 언어별 한계가 사용자에게 과도한 내부 구현 세부사항 없이 전달되는가

## D. Semantic capability 최소 세 ecosystem

### 시작 상태

- structural analysis가 완료된 공식 fixture
- semantic adapter 또는 indexer 사용 가능

### 사용자 요청

특정 symbol의 definition, references, implementation과 변경 영향 후보를 설명한다.

### 기대 사용자 경험

- semantic 결과와 structural 결과의 source를 구분한다.
- definition/reference와 type-aware relation을 제공한다.
- build가 불완전하거나 dependency가 없을 때 degradation을 표시한다.
- agent가 semantic 결과를 설명할 수 있지만 관계 자체를 조용히 창작하지 않는다.

### Canonical Context 변화

- 없음. 영향 판단이 material Question을 만들 수 있으나 자동 Decision은 아님

### Derived State 변화

- Semantic Relation
- analyzer diagnostics
- capability coverage와 freshness

### 금지 행동

- semantic adapter 미실행 결과를 semantic fact로 표시
- unresolved symbol을 다른 동일 이름 symbol과 합침
- 영향 후보를 correctness 판정으로 표현

### 자동 검증

- known definition/reference set
- implementation/override 관계가 있는 fixture
- analyzer restart와 cache rebuild
- broken-build degradation
- common model normalization

### 수동 평가

- semantic capability가 사용자의 설계·구현 판단에 실제로 더 나은 근거를 제공하는가

## E. Polyglot repository-wide 이해

### 시작 상태

- 최소 세 공식 언어와 문서·config가 섞인 repository
- 언어별 capability 수준이 서로 다를 수 있음

### 사용자 요청

전체 시스템의 component, 경계, 데이터 또는 요청 흐름과 언어 간 연결을 설명한다.

### 기대 사용자 경험

- 언어별 분석 결과를 repository-wide view로 연결한다.
- process, API, file, config 또는 document source를 통해 cross-language 경계를 설명한다.
- 확인된 구조 관계와 agent의 architecture 해석을 구분한다.
- 한 언어 analyzer 실패가 다른 영역을 사용할 수 없게 만들지 않는다.
- 전체 coverage는 언어별·영역별로 분해해 표시한다.

### Canonical Context 변화

- 사용자가 채택한 architecture fact 또는 open Question만 canonical 후보

### Derived State 변화

- cross-language Context Map
- semantic annotations
- degraded coverage

### 금지 행동

- 일부 언어의 높은 capability를 전체 repository에 일반화
- 문서의 주장과 실제 source를 구분하지 않음
- process boundary를 직접 call relation으로 위조

### 자동 검증

- 언어별 entity count와 coverage
- cross-component source refs
- partial analyzer failure
- snapshot consistency

### 수동 평가

- 사용자가 전체 시스템과 각 언어 component의 관계를 설명할 수 있는가

## F. 단계적 Inquiry와 사용자 Decision

### 시작 상태

- Project Context와 Repository Intelligence 사용 가능
- 여러 material decision이 dependency를 가짐

### 사용자 요청

설계 또는 구현을 시작하기 전에 필요한 판단을 충분히 정리한다.

### 기대 사용자 경험

- 에이전트가 먼저 repository와 환경에서 사실을 조사한다.
- 현재 frontier의 질문만 배경, 선택지, 권장안, trade-off와 uncertainty와 함께 제시한다.
- 질문에는 제품, architecture와 implementation 관점이 필요한 만큼 포함된다.
- 사용자는 선택, 수정안, 위임, 조사, prototype 또는 보류로 답할 수 있다.
- 답변에 따라 다음 질문이 열리거나 닫힌다.
- 세션을 중단하고 새 세션에서 이어갈 수 있다.
- 같은 질문을 CLI에서 다시 입력하지 않는다.

### Canonical Context 변화

- 열린 Question과 dependency
- 명시적 사용자 Decision 또는 delegated/deferred 상태
- 사용자 rationale가 있으면 별도 필드로 저장
- 당시 agent recommendation과 Source

### Derived State 변화

- Question Candidate와 materiality ranking
- 설명용 option 비교와 semantic summary

### 금지 행동

- 코드에서 확인할 수 있는 사실을 사용자에게 질문
- 모호한 과거의 “좋아요”를 Decision으로 적용
- agent recommendation을 사용자 choice로 저장
- 답을 모른다는 사용자에게 추측을 강요
- 고정 질문 수에 맞추기 위해 material branch를 생략

### 자동 검증

- Question frontier 계산
- dependency별 후속 질문
- user turn과 Question revision 연결
- restart 후 open frontier 복구
- answered Question 반복 방지
- terminal branch 상태

### 수동 평가

- 질문이 제품·설계·구현 결과에 실제로 중요한가
- 사용자가 선택의 의미와 영향을 설명할 수 있는가
- 질문 횟수가 아니라 relevance와 shared understanding으로 종료되는가

## G. Candidate 수집과 승격

### 시작 상태

- Candidate 수집 활성
- 일반 코드 탐색과 명령 실행이 진행됨

### 사용자 요청

작업을 조사하고 필요한 경우 중요한 발견을 기억한다.

### 기대 사용자 경험

- Candidate Inspection에서 existence/identity, kind, origin/provenance, collection scope,
  creation/observation basis, retention/expiry, promotion disposition과 scope opt-out
  state를 확인할 수 있다.
- raw prompt, 전체 tool arguments와 source body가 자동 장기 저장되지 않는다.
- observed fact, agent hypothesis와 Question Candidate가 구분된다.
- 사용자 Decision만 명시적 user turn에서 승격된다.
- 사용자는 selected scope의 Candidate 수집을 끌 수 있고 이후 새 automatic collection이
  중단된다.
- Opt-out 전에 존재한 Candidate는 explicit deletion, dismissal, promotion 또는 retention
  expiry까지 inspectable하며 opt-out이 이를 조용히 rewrite/promote하지 않는다.
- Candidate Inspection failure는 projection degradation으로 표시되고 Candidate를
  promote, delete 또는 reinterpret하지 않는다.

### Canonical Context 변화

- 승격 조건을 충족한 fact, Question 또는 Decision만 추가

### Derived State 변화

- session-local Candidate와 bounded observation

### 금지 행동

- hypothesis를 canonical fact로 자동 승격
- 모든 stdout를 무제한 보존
- Candidate decay를 Decision 삭제로 적용
- inspection read를 Candidate promotion/dismissal/deletion side effect로 사용
- opt-out을 existing Candidate의 silent promotion, rewrite 또는 hidden deletion으로 적용

### 자동 검증

- V07: scoped opt-out, existing Candidate visibility, retention/expiry와 deletion boundary
- V09: candidate type/provenance, promotion authorization/disposition과 read-only inspection
- V11: collection → inspection → promotion/dismissal/expiry integrated journey와 projection
  failure isolation

### 수동 평가

- 사용자가 무엇이 잠정 정보이고 장기 기억인지 이해할 수 있는가

## H. 일반 작업과 source-grounded Checkpoint

### 시작 상태

- 목표와 필요한 Decision이 충분히 확정됨
- working tree에 기존 unrelated dirty change가 있을 수 있음

### 사용자 요청

확정된 범위의 실제 코드 또는 문서 작업과 검증을 수행한다.

### 기대 사용자 경험

- 에이전트는 일반 도구로 작업하며 ordinary write permission을 요청하지 않는다.
- 변경된 path와 적용 Decision을 구분한다.
- existing dirty change를 현재 작업 결과로 잘못 귀속하지 않는다.
- 의미 있는 작업 완료, pause 또는 handoff에서 하나의 Checkpoint를 만든다.
- Checkpoint는 변경, 이유, 검증, 한계, non-goals와 다음 단계를 설명한다.
- 사용자 review가 없어도 work와 verification 상태를 정직하게 표현한다.

### Canonical Context 변화

- Checkpoint
- applied Decision refs
- changed Source refs 또는 observed paths
- verification result Source
- 새 Context Item, risk, known limit와 open Question

### Derived State 변화

- changed source의 증분 분석
- impact Candidate

### 금지 행동

- 일반 파일 쓰기를 Write Ticket이나 동등한 ceremony로 차단
- 실행하지 않은 테스트를 성공으로 기록
- 기존 dirty file을 현재 Checkpoint 변경으로 자동 포함
- user review가 없다는 이유만으로 실제 완료 상태 왜곡
- 단순 조회나 변경 없는 설명에 canonical Checkpoint 생성

### 자동 검증

- Checkpoint serialization과 restart
- changed path basis
- work, verification, review와 acceptance 상태 독립성
- known limits와 non-goals 보존
- session-end Candidate와 canonical promotion 차이

### 수동 평가

- Checkpoint만 읽고 무엇을 왜 했는지 이해할 수 있는가

## I. 완전히 새로운 세션의 자동 Recall

### 시작 상태

- 하나 이상의 Decision과 Checkpoint 존재
- 이전 대화 context 없음

### 사용자 요청

현재 Project의 작업을 이어간다.

### 기대 사용자 경험

- 첫 project-scoped 요청에서 bounded read-only Recall을 자동 수행한다.
- 단순 인사나 unrelated request에는 Recall하지 않는다.
- 목표, why, active Decisions와 rationale, current state, open Questions, risks와 next step을 복구한다.
- 어떤 records와 Sources를 사용했는지 사용자에게 표시하거나 펼쳐볼 수 있게 한다.
- stale, unavailable, contradicted, superseded와 omitted 상태를 표시한다.
- 이미 해결된 Question을 다시 묻지 않는다.

### Canonical Context 변화

- Recall 자체는 없음

### Derived State 변화

- bounded selection 또는 cache 가능

### 금지 행동

- Recall을 숨은 authority mutation으로 사용
- user-visible record와 다른 비밀 memory를 근거로 행동
- unrelated history를 반복 주입
- omission을 숨김

### 자동 검증

- fresh session trigger
- no-mutation property
- deterministic tie-breaking
- budget/truncation metadata
- stale/superseded filtering

### 수동 평가

- 새 에이전트가 결과뿐 아니라 결정 이유와 남은 불확실성을 정확히 복구하는가

## J. 다른 clone의 portable bundle과 divergent merge

### 시작 상태

- 공통 base bundle을 가져온 clone A와 clone B
- 각 clone에서 독립적인 canonical 변경 발생

### 사용자 요청

두 환경의 context를 합치고 다른 clone에서 작업을 계속한다.

### 기대 사용자 경험

- stable Project ID와 clone binding을 유지한다.
- 독립적인 새 record는 안전한 경우 자동 병합한다.
- 같은 Question·Decision·Context의 의미 충돌을 three-way 비교로 보여 준다.
- 삭제와 수정 또는 상충 Decision을 조용히 선택하지 않는다.
- 사용자는 선택, 병합 또는 context branch를 만들 수 있다.
- source repository가 없으면 canonical context를 읽고 code relation을 unavailable로 표시한다.

### Canonical Context 변화

- merged records
- user-resolved conflict Decision 또는 branch relation
- bundle revision/base metadata

### Derived State 변화

- import 후 index rebuild
- source rebind와 freshness check

### 금지 행동

- last writer wins로 사용자 Decision 덮어쓰기
- path 또는 remote URL만으로 다른 Project를 자동 동일시
- source copy와 raw session trace를 bundle에 기본 포함

### 자동 검증

- export/import determinism
- independent additions merge
- same-record conflict
- delete/modify conflict
- another-path clone binding
- derived index rebuild

### 수동 평가

- 사용자가 충돌의 의미와 결과를 이해하고 안전하게 선택할 수 있는가

## K. 기억 수정, supersession과 삭제

### 시작 상태

- Decision, Context Item, Checkpoint와 semantic annotation 존재

### 사용자 요청

오탈자를 고치고, 과거 Decision을 바꾸며, 민감한 기록을 삭제한다.

### 기대 사용자 경험

- 비의미적 보정과 의미 변경의 차이를 설명한다.
- Decision 변경은 supersession으로 history와 현재 적용 상태를 구분한다.
- 민감 원문을 삭제하고 derived index와 annotation에서도 제거한다.
- 필요한 최소 tombstone은 민감 원문이나 복구 가능한 hash를 포함하지 않는다.
- 자동 재분석이 사용자 correction을 덮어쓰지 않는다.

### Canonical Context 변화

- revision
- superseding Decision
- deletion 또는 minimal tombstone

### Derived State 변화

- affected indexes와 documents invalidate/rebuild

### 금지 행동

- 삭제 원문을 immutable audit log에 보존
- 의미 변경을 과거 record rewrite로 숨김
- stale generated document를 current로 유지

### 자동 검증

- revision history
- active/superseded selection
- V07 privacy/managed deletion propagation
- export/import 후 forget semantics

### 수동 평가

- 사용자는 현재 유효한 판단과 과거 판단을 혼동하지 않는가

## L. Decision 재사용과 재검토

### 시작 상태

- 적용 범위와 revisit trigger가 있는 active Decision
- 동일 범위와 변경된 범위의 후속 작업

### 사용자 요청

기존 방향을 바탕으로 다음 구현을 진행한다.

### 기대 사용자 경험

- 동일한 Project, 적용 범위와 전제에서는 Decision을 재사용한다.
- 이미 선택된 방향의 직접적인 구현 세부사항을 다시 묻지 않는다.
- source·전제·scope 변경 또는 revisit trigger가 발생하면 이유와 함께 재검토를 요청한다.
- preference를 새로운 user Decision으로 위조하지 않는다.

### Canonical Context 변화

- 재검토가 필요한 경우 새 Question
- 사용자가 변경한 경우 superseding Decision

### Derived State 변화

- applicability match와 review Candidate

### 금지 행동

- 모든 새 작업에서 같은 질문 반복
- 과거 Decision을 unrelated Project에 일반화
- source change만으로 Decision 자동 폐기

### 자동 검증

- applicability match
- trigger evaluation
- conflict handling
- repeated-question prevention

### 수동 평가

- 질문 반복을 줄이면서도 잘못된 과거 판단을 적용하지 않는가

## M. Guarded effect confirmation

### 시작 상태

- 일반 작업과 하나 이상의 고위험 effect 후보

### 사용자 요청

로컬 코드 수정 후 외부 배포 또는 민감한 action을 수행한다.

### 기대 사용자 경험

- 일반 수정과 테스트는 사전 confirmation 없이 진행한다.
- 고위험 effect 직전에 confirmation request identity/revision, exact action, target,
  expected effect, risk, scope, expiration, requesting actor/provenance와 exact-match
  fingerprint basis를 제시한다.
- 현재 host에서 답할 수 있고, 지원하지 않으면 viewer 또는 CLI fallback을 제공한다.
- Explicit current-host response는 exact request/revision에 연결된 Source이며 general
  product Decision과 분리된다.
- Confirmation은 action/target/scope-scoped, expiring, single-use와 non-transferable이고
  action, target, expected effect, scope 또는 revision이 바뀌면 새 confirmation이 필요하다.
- Guarded effect는 valid exact-match confirmation을 검증하기 전에 dispatch되지 않는다.
- 하나의 operation identity로 not-dispatched, completed, failed와 indeterminate outcome을
  구분하며 indeterminate outcome은 silent retry나 success로 처리하지 않는다.

### Canonical Context 변화

- explicit user response Source
- 필요한 durable history만 resulting operation과 함께 Checkpoint/Context Item에서 참조
- operational confirmation은 general Decision이나 seventh canonical core entity가 아님

### Derived State 변화

- Guarded Effect Candidate와 confirmation/operation operational state

### 금지 행동

- 모든 file write를 Guarded로 분류
- 한 번의 승인으로 다른 target·effect까지 포괄
- cooperative confirmation을 sandbox로 표현
- 승인 전 외부 effect 수행
- stale/expired/mismatched/reused confirmation으로 dispatch 또는 silent retry
- denied/missing confirmation을 general consent나 inferred approval로 대체

### 자동 검증

- V08: current-host response transport와 viewer/CLI logical fallback identity/linkage
- V11: effect category mapping, exact action/target/effect/scope/revision/expiration match,
  Source linkage, confirmation reuse rejection, no-dispatch-before-validation,
  ordinary-action non-blocking과 indeterminate recovery

### 수동 평가

- 사용자는 무엇이 왜 위험하고 무엇을 승인하는지 이해할 수 있는가

## N. Local viewer, 학습과 문서 출력

### 시작 상태

- Repository Intelligence, Decisions와 Checkpoints 존재

### 사용자 요청

프로젝트를 이해하고 architecture, Decision, implementation과 handoff 문서를 생성한다.

### 기대 사용자 경험

- viewer에서 Project overview, Repository Map, Decision trail와 Checkpoint timeline을 탐색한다.
- `overview`, `working`, `deep` 설명 수준을 선택한다.
- code entity와 Decision에 연결된 개념 설명을 본다.
- 네 필수 문서를 Markdown으로 export하고 self-contained HTML로 preview한다.
- 현재 Viewer projection을 명시한 local path에 하나의 self-contained read-only HTML
  snapshot으로 export하고, 생성 뒤 Runtime이나 listener 없이 읽는다.
- 문서마다 source snapshot, Decisions, capability coverage, known gaps와 generator identity가 표시된다.
- 저장소 write는 사용자 지정 경로가 있을 때만 수행한다.
- 사용자가 편집한 문서는 review/import 후에만 canonical input이 된다.

### Canonical Context 변화

- 문서 생성 자체는 없음
- explicit adoption 시 Source 또는 Context Item 가능

### Derived State 변화

- document projection과 preview
- graph layout

### 금지 행동

- generated document를 자동 canonical truth로 채택
- 사용자 요청 없이 Product Repository에 파일 생성
- unsupported/failed 영역을 문서에서 생략
- Viewer snapshot에 mutation/Guarded/document-export form, authenticity token, live endpoint,
  JavaScript 또는 external runtime asset 포함
- Viewer snapshot 생성 중 자동 upload 또는 external network transmission
- 사용자 숙련도를 조용히 영구 추론

### 자동 검증

- Markdown export
- self-contained HTML
- required metadata
- source refs validity
- stale invalidation
- Korean/English fixed UI rendering
- requested user-language generated content를 allowlist로 거부하지 않음
- Viewer snapshot의 explicit-destination atomic publication, no-listener exit, read-only surface,
  self-contained asset, basis/freshness/degradation visibility와 Runtime-independent read

### 수동 평가

- 사용자가 raw JSON 없이 구조, 판단과 다음 단계를 이해할 수 있는가
- 문서가 다른 agent의 실제 handoff에 충분한가

## O. Degraded analysis, provider failure와 crash recovery

### 시작 상태

- 일부 parser 또는 semantic provider 실패 가능
- derived index 또는 process가 중간에 손상될 수 있음

### 사용자 요청

분석을 수행하고 실패 후 Project 작업을 계속한다.

### 기대 사용자 경험

- 실패한 언어·영역과 유지되는 capability를 구분한다.
- semantic provider가 없어도 local structural mode와 Canonical Context가 작동한다.
- derived index를 삭제하고 재구축할 수 있다.
- canonical transaction 실패는 성공으로 보고하지 않는다.
- 장기 process의 stdout, stderr, exit와 termination 결과를 잃지 않는다.

### Canonical Context 변화

- 분석 실패 자체는 canonical mutation이 아님
- 사용자가 보존한 known limit만 Context Item이 될 수 있음

### Derived State 변화

- failed/degraded coverage
- rebuilt indexes

### 금지 행동

- partial result를 complete로 표시
- document generation 실패를 repository 작업 실패로 바꿈
- index 손상을 canonical loss로 확산
- timeout 후 실제 process 상태를 숨김

### 자동 검증

- parser failure injection
- semantic provider unavailable
- index corruption/rebuild
- transaction crash/fault injection
- bounded process timeout와 child cleanup

### 수동 평가

- 사용자는 무엇을 계속 신뢰할 수 있고 무엇이 누락됐는지 이해하는가

## P. Fresh-service 경계와 legacy 비호환

### 시작 상태

- 새 implementation root
- clean new Runtime Home
- repository history에는 기존 구현이 존재할 수 있음

### 사용자 요청

새 제품을 초기화하고 사용한다.

### 기대 사용자 경험

- 새 Runtime Home과 schema로 clean initialization한다.
- legacy migration, import, detection, backup와 historical export 명령이 없다.
- active docs와 CLI는 legacy data path를 안내하지 않는다.
- 기존 source는 Git history일 뿐 제품 기능이 아니다.

### Canonical Context 변화

- 새 Project와 records만 생성

### Derived State 변화

- 새 product indexes만 생성

### 금지 행동

- legacy Runtime Home 검색·읽기·변환
- 기존 ID나 schema를 compatibility shortcut으로 재사용
- 두 runtime 또는 API를 동시 제공
- legacy data가 없다는 전제에서 migration warning을 요구

### 자동 검증

- active source의 legacy import/migrate command 부재
- new runtime schema isolation
- reconstruction crate의 legacy crate dependency 부재
- final package에 old binary/method 없음

### 수동 평가

- 사용자가 하나의 현재 Volicord 제품만 인식하는가

## Q. LLM 개인정보와 background semantic opt-in

### 시작 상태

- Project와 source repository가 연결됨
- background semantic provider는 설정되지 않았거나 opt-in되지 않음
- exclude와 secret-like fixture가 존재함

### 사용자 요청

interactive 코드 설명을 요청한 뒤, 선택적으로 background semantic analysis를 구성한다.

### 기대 사용자 경험

- 현재 Codex 대화의 interactive explanation과 별도 background 전송을 구분한다.
- background analysis는 Project 단위 opt-in 전 실행되지 않는다.
- opt-in 화면에서 provider, model, source 범위, exclude와 secret 처리 정책을 확인한다.
- semantic analysis를 끄거나 revoke해도 inventory, structural analysis, Decision, Checkpoint와 Recall을 계속 사용한다.
- 생성된 annotation과 cache를 삭제할 수 있다.

### Canonical Context 변화

- portable해야 하는 privacy/provider preference만 명시적으로 저장 가능
- semantic annotation 자체는 Derived State

### Derived State 변화

- opt-in 후에만 semantic annotation 생성
- provider/model/source snapshot provenance

### 금지 행동

- opt-in 전 background source 전송
- interactive host access를 background consent로 일반화
- raw source body를 portable bundle에 기본 포함
- excluded 또는 secret file을 전송 범위에 조용히 포함
- semantic provider 실패를 canonical 기능 실패로 표현

### 자동 검증

- opt-in 전 network/provider invocation 부재
- transmitted source manifest
- exclude와 secret fixture 처리
- revoke 후 background invocation 차단
- annotation/cache deletion
- local-only end-to-end core journey

### 수동 평가

- 사용자는 어떤 코드가 어떤 provider로 언제 전송되는지 이해할 수 있는가
- privacy setting이 기능을 사용하기 위해 강제되는 동의처럼 보이지 않는가

## 6. Replacement gate repository set

최소 다음 대상에서 전체 journey를 반복한다.

1. Volicord 자체의 Rust workspace
2. 공식 structural 언어 중 하나를 사용하는 소규모 단일 언어 application
3. 문서와 최소 세 언어가 섞인 중간 규모 polyglot repository
4. 각 공식 structural 언어 fixture
5. 첫 structural 목록 밖의 언어 fallback fixture

첫 세 실제 repository에서는 다음 end-to-end journey가 필요하다.

```text
clean install
→ Project initialization and binding
→ inventory and capability analysis
→ source-grounded understanding
→ staged Inquiry and user Decision
→ ordinary repository work
→ source-grounded Checkpoint
→ process restart and new-session Recall
→ another-clone bundle import
→ divergent conflict handling
→ memory correction and deletion
→ document output
→ degraded failure recovery
```

## 7. 교체 전 정량·정성 지표

| 지표 | 판정 질문 |
|---|---|
| Context recovery accuracy | 새 세션이 goal, Decision, rationale, state와 open Question을 정확히 복구하는가 |
| Decision repeat rate | 이미 해결된 판단을 불필요하게 다시 묻지 않는가 |
| Question relevance | 사용자 가치와 결과를 바꾸지 않는 세부사항을 넘기지 않는가 |
| Decision comprehension | 사용자가 option, trade-off와 consequence를 설명할 수 있는가 |
| Source grounding | 핵심 코드 설명과 문서가 source로 추적되는가 |
| Capability honesty | unavailable·partial·failed capability를 정확히 표시하는가 |
| Coverage | 언어·영역별 분석 범위와 누락이 측정되는가 |
| Memory correctability | record를 쉽게 수정·supersede·삭제할 수 있는가 |
| Interruption cost | 사용자의 관심이 작업보다 Volicord 절차에 쏠리지 않는가 |
| Document fidelity | generated document가 현재 source와 Decision을 반영하는가 |
| Recovery | crash, provider failure와 index 손상 후 안전하게 복구되는가 |
| Portability | 다른 clone에서 같은 Project 맥락을 정확히 복구하는가 |

속도와 token은 product success metric이 아니라 usability guardrail로 측정한다. 분석 시간이 길거나 output이 커서 사용자가 실제 기능을 포기하게 되면 실패다.

## 8. 최종 통과 조건

다음이 모두 참이어야 기존 구현 제거 gate로 이동할 수 있다.

- Linux clean install과 Codex connection이 반복 가능하다.
- 모든 텍스트 fixture에서 inventory가 제공된다.
- Java, Python, JavaScript, TypeScript, C, C++와 Rust structural acceptance가 통과한다.
- 최소 세 ecosystem semantic acceptance가 통과한다.
- polyglot repository에서 언어별 capability를 정직하게 연결한다.
- local-only mode에서 canonical, structural, Inquiry, Checkpoint와 Recall이 작동한다.
- 사용자는 한 번의 host 답변으로 Decision을 기록한다.
- Question session을 중단하고 새 세션에서 이어갈 수 있다.
- ordinary repository write는 Volicord ceremony로 차단되지 않는다.
- Checkpoint가 dirty change와 verification을 정확히 구분한다.
- 첫 project-scoped 요청에서 bounded Recall이 작동한다.
- bundle export/import, divergence와 conflict resolution이 작동한다.
- correction, supersession과 deletion이 portable state에 반영된다.
- 네 필수 문서가 source-grounded metadata와 함께 생성된다.
- Naturalistic Dogfood의 machine-observable qualification은 human review 부재와 구분되어
  독립적으로 통과할 수 있다.
- Human review가 없으면 replacement는 명시적으로 `pending_human_review`이며 pass로
  표현되지 않는다.
- Replacement usability review는 campaign-level deterministic representative sample로
  Question/Decision/interruption, simple/complex document readability, static Viewer
  readability와 `en`/`ko` live Viewer accessibility를 포함한다.
- Human review는 deterministic machine failure를 override하지 않으며, immutable automated
  result에 나중에 결합할 때 naturalistic session을 다시 실행하지 않는다.
- Guarded effect만 action-scoped confirmation을 요구한다.
- partial analyzer, provider와 derived-index 실패가 canonical state를 손상시키지 않는다.
- active product에 legacy migration, data detection, compatibility와 workflow surface가 없다.
