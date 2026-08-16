# Repository Intelligence 계약

- 상태: active specialized architecture owner
- 소유 범위: Repository Snapshot과 Analysis Snapshot, inventory, normalized code
  entity/relation, capability·coverage·freshness, analysis provenance, language와
  semantic adapter boundary, invalidation category
- 상위 architecture 기준: [논리 아키텍처](architecture.md)
- core domain 기준: [핵심 도메인 모델](domain-model.md)
- evidence 기준: [architecture 입력 계약](architecture-inputs.md),
  [V01 보고서](../../validation/repository-intelligence/polyglot-structural/report.md),
  [Production structural analyzer qualification](../../validation/repository-intelligence/production-structural-qualification/report.md),
  [Semantic adapter qualification](../../validation/repository-intelligence/semantic-qualification/report.md)
- privacy 기준: [Privacy와 provider 경계](privacy-and-provider-boundary.md)
- 비소유 범위: user judgment, canonical persistence, parser·indexer·provider 선택,
  production schema/API, invalidation algorithm과 automatic Question discovery

이 문서는 Repository Intelligence subsystem의 specialized contract를 정의한다.
`architecture.md`의 dependency direction과 `domain-model.md`의 information class,
canonical identity, provenance와 lifecycle 의미를 구체화할 뿐 재정의하지 않는다.
아래 표와 envelope는 semantic obligation이며 serialized field나 Rust type layout이
아니다.

## 1. Authority와 information class

Repository Intelligence는 repository-wide 이해를 위한 first-party subsystem이다.
다음 authority boundary를 유지한다.

- Repository와 analyzer에서 관찰한 결과를 snapshot에 bound된 `Derived State`로
  만들 수 있다.
- Canonical `Source`, `Decision`, `Context Item`과 `Checkpoint` identity를 읽고
  analysis entity에 reference할 수 있다.
- Canonical로 보존할 가치가 있는 observation은 provenance-bearing `Session
  Candidate`로 제출할 수 있다.
- `Question`이나 `Decision`을 생성·해결·수정·supersede·forget하지 않는다.
- Analysis, semantic result, annotation 또는 generated interpretation은 user
  judgment를 소유하지 않으며 user correction을 자동으로 되돌리지 않는다.

Analysis identity가 안정적이고 결과가 정확해 보여도 canonical identity나
authority를 얻지는 않는다. Candidate promotion과 explicit adoption은 Canonical
Context Kernel의 별도 domain operation이다.

## 2. Snapshot identity

### 2.1 Repository Snapshot

`Repository Snapshot`은 한 Project에 연결된 repository source를 특정 관찰
시점으로 고정하는 analysis basis다. 최소 다음 의미를 가진다.

- stable Project identity와 현재 local binding을 혼동하지 않는 repository basis
- commit, content fingerprint 또는 동등한 observed revision basis
- 포함 대상으로 관찰한 root와 source boundary
- included, excluded와 unavailable area
- observation time과 repository Source provenance
- 동일 source state의 반복 관찰과 다른 state를 구분하는 identity

Repository Snapshot identity는 local absolute path가 아니다. 같은 snapshot을 다른
binding에서 다시 확인할 수 있고, 반대로 같은 path의 content가 바뀌면 새 snapshot
basis가 된다. Identity나 fingerprint의 concrete algorithm은 이 문서가 선택하지
않는다.

Current mutable repository를 읽는 analyze, repair와 reindex는 각각 그 scan을 위한
fresh repository observation `Source`를 먼저 canonical로 기록하고, 성공한 Repository
Snapshot과 Analysis Snapshot은 모두 그 exact Source를 참조한다. Source 기록이
실패하면 해당 scan을 current Derived State로 publish하지 않는다.

### 2.2 Analysis Snapshot

`Analysis Snapshot`은 정확히 하나의 Repository Snapshot에 대해 수행한 capability
별 analysis basis다. 다음을 함께 식별한다.

- 분석 대상 Repository Snapshot identity
- 요청된 capability, language와 area scope
- 실제 실행된 adapter/analyzer와 그 identity
- covered, excluded, unsupported와 failed scope
- diagnostics, generation/observation time와 freshness 판정 basis
- 이 snapshot에 속한 entity, relation, result와 annotation 집합

하나의 Repository Snapshot은 capability, language, area 또는 analyzer가 다른 여러
Analysis Snapshot을 가질 수 있다. Analysis Snapshot이 새로 생겨도 이전 snapshot의
historical result를 rewrite하지 않는다. Repository Snapshot이 달라지면 이전 result는
새 source의 current fact가 아니며 `stale` 판정 대상이다.

## 3. Inventory는 analyzer availability와 독립적이다

`inventory`는 repository registration과 source boundary 이해의 기본 capability다.
텍스트 repository라면 structural 또는 semantic analyzer가 없어도 다음을 관찰할 수
있어야 한다.

- file과 directory boundary
- detected language와 보조 text format
- manifest, configuration, document와 Git observation
- binary, vendor, generated, ignored와 excluded classification
- candidate package, component와 entry-point basis
- 각 관찰의 Source와 Repository Snapshot binding

Inventory failure는 해당 observation scope에 표시한다. Structural adapter가
`unavailable`, `unsupported` 또는 `failed`여도 이미 성공한 inventory를 삭제하거나
빈 결과로 바꾸지 않는다. 반대로 inventory만으로 parser-confirmed entity나 semantic
relation을 주장하지 않는다.

## 4. Accepted capability와 상태

### 4.1 Capability vocabulary

| Capability | 소유하는 의미 |
|---|---|
| `inventory` | file, language, manifest, config, document, Git와 analysis boundary 관찰 |
| `agent_assisted` | Source-grounded explanation, 질의응답과 architecture interpretation; 구조적으로 확인되지 않은 부분은 interpretation으로 표시 |
| `structural` | parser 또는 동등한 structural analyzer가 확인한 entity, source range와 syntax relation |
| `semantic` | semantic analyzer가 확인한 definition, reference, type, implementation과 resolved symbol relation |
| `ecosystem` | build, package, workspace, dependency와 toolchain context를 반영한 analysis |

Capability는 repository-wide boolean이 아니다. 모든 report는 최소한 Repository
Snapshot × language × area × capability 조합별 상태와 coverage를 표현한다.

### 4.2 State vocabulary

| State | 의미 |
|---|---|
| `available` | 선언된 scope에서 capability 결과를 현재 사용할 수 있고 coverage basis를 확인할 수 있음 |
| `unavailable` | capability가 알려져 있지만 현재 environment, dependency, source 또는 analyzer 부재로 실행할 수 없음 |
| `unsupported` | 해당 language, construct 또는 area에 capability 계약이 제공되지 않음 |
| `partial` | 일부 결과는 유효하지만 포함 scope의 일부가 누락되거나 제한됨 |
| `failed` | 실행을 시도했으나 오류 또는 비정상 종료로 해당 scope의 결과를 완료하지 못함 |
| `stale` | 결과가 존재하지만 현재 Repository Snapshot과 다르거나 freshness 조건을 만족하지 않음 |

`unsupported`는 시도 실패가 아니고 `unavailable`은 지원하지 않는다는 뜻이 아니다.
`partial`, `unavailable`, `unsupported`, `failed`와 `stale`에는 reason, affected scope,
usable remainder와 user-visible consequence를 함께 제시한다. 하나의 state를 다른
language나 area에 일반화하지 않는다.

### 4.3 Coverage report

Capability report는 최소 다음 축을 분해한다.

- Project와 Repository/Analysis Snapshot identity
- language와 language detection uncertainty
- area: path, package, component, workspace 또는 동등한 bounded scope
- requested capability와 observed state
- 대상/covered file, entity와 relation 수 또는 inspectable manifest
- excluded, unsupported, unavailable, failed와 stale scope
- diagnostics와 known construct limit
- adapter/analyzer/provider identity와 provenance class
- observed/generated time, source freshness와 uncertainty

Repository summary는 이 matrix의 projection일 뿐 독립적인 completeness authority가
아니다. 높은 coverage의 한 language나 area를 전체 repository의 지원으로 표현하지
않는다.

## 5. 첫 structural gate와 language extension

첫 structural gate는 다음 일곱 language를 모두 포함한다.

- Java
- Python
- JavaScript
- TypeScript
- C
- C++
- Rust

이 목록은 parser technology 선택이 아니다. 모든 language adapter는 공통 envelope와
상태 계약을 지키면서 각 language에 실제로 존재하는 entity, relation, diagnostic과
limit만 보고한다. JavaScript와 TypeScript, C와 C++처럼 가까운 language도 capability
또는 build context 차이를 숨기지 않는다.

첫 목록 밖의 text language도 inventory와 가능한 `agent_assisted`를 제공하며,
제공하지 않는 structural, semantic 또는 ecosystem capability를 `unsupported` 또는
상황에 맞는 state로 명시한다. Analyzer 부재를 Project 등록 거부로 확대하지 않는다.

### Language-specific extension point

공통 envelope로 의미 손실 없이 표현할 수 없는 language construct는 extension을
사용한다.

- extension은 language와 owning adapter가 명확한 namespace를 가진다.
- 공통 field를 비슷해 보이는 다른 의미로 overload하지 않는다.
- extension의 Source range, snapshot, provenance, coverage와 diagnostic 의무는 공통
  envelope와 같다.
- 공통 consumer가 extension을 이해하지 못해도 core entity identity와 base relation,
  capability state와 limitation을 읽을 수 있다.
- extension 부재를 construct 부재나 analysis 성공으로 해석하지 않는다.

Macro expansion, generated source, conditional compilation, dynamic dispatch,
reflection, runtime-only state와 external service behavior는 실제 capability가
확인한 범위만 표현한다.

## 6. Normalized Code Entity와 relation envelope

### 6.1 Code Entity

`Code Entity`는 Analysis Snapshot 안에서 source-bound code 또는 repository
structure를 가리키는 Derived identity다. 공통 envelope는 다음 의미를 보존한다.

- Analysis Snapshot과 Repository Snapshot binding
- language, owning area와 entity kind
- portable Source identity, locator와 source range
- stable entity identity basis와 optional display/qualified name
- structural/semantic provenance와 producing adapter
- capability state, diagnostics, uncertainty와 freshness
- language-specific extension과 unsupported construct marker
- 연결된 canonical identity reference

Entity kind는 repository, package, module/namespace, file, class/interface/trait,
struct/enum/type, function/method, field, test, configuration와 document처럼 공통
소비자가 이해할 수 있는 작은 vocabulary를 사용한다. Language에 없는 concept을
채우지 않으며 필요한 세부 의미는 extension이 소유한다.

Stable identity는 같은 Repository Snapshot과 같은 semantic source entity를 반복
분석했을 때 재현되어야 한다. Display text, traversal order나 analyzer process-local
ID만으로 identity를 만들지 않는다. 다른 Repository Snapshot 사이의 continuity는
근거가 있을 때 별도 correspondence로 표현하며 동일성을 조용히 가정하지 않는다.

### 6.2 Source range

Source range는 Source identity와 snapshot에 bound되고 다음을 구분할 수 있어야 한다.

- file 또는 equivalent source locator
- start와 end location의 coordinate convention
- whole-file/entity/symbol range의 의미
- range를 제공한 adapter와 precision/known limit

Range를 알 수 없는 entity는 fabricated coordinate를 만들지 않고 unavailable 또는
partial reason을 보존한다. Current file content가 snapshot과 다르면 historical range를
current navigation guarantee로 제시하지 않는다.

### 6.3 Relation

Normalized relation은 다음 의미를 가진다.

- relation identity와 owning Analysis Snapshot
- source entity와 target entity 또는 unresolved external target
- typed relation kind
- supporting Source/range와 evidence class
- producing capability와 adapter/analyzer
- diagnostics, uncertainty, freshness와 resolution state
- optional language-specific extension

Common structural relation은 `contains`, `declares`, `imports`, `includes`, `exports`,
`inherits`, `implements`, `calls_syntactically`, `tests`와 `configures`를 표현할 수
있다. Semantic relation은 `defines`, `references`, `resolves_to`, `type_of`,
`implements`, `overrides`와 `instantiated_by` 등을 별도 provenance로 표현할 수
있다. 같은 label이 양쪽에 존재해도 structural syntax와 semantic resolution을
합치지 않는다.

Unresolved target, cross-process boundary와 architecture inference를 confirmed direct
call relation으로 만들지 않는다. Relation vocabulary의 확장은 capability와 evidence
class를 함께 정의해야 한다.

## 7. Fact, result, annotation, interpretation과 correction provenance

같은 Source를 사용해도 다음 analysis statement class는 서로 대체할 수 없다.

| Class | Authority와 필수 basis | Information class |
|---|---|---|
| `Repository Observation` | file/Git/manifest/config를 직접 관찰한 observer, Repository Snapshot, scope와 observation diagnostic | Derived; explicit promotion 시 `observed_fact` provenance 유지 |
| `Structural Fact` | parser 또는 structural analyzer identity, Analysis Snapshot, Source/range, supported construct와 diagnostic | Derived `observed_fact` basis; parser-confirmed 범위만 사실 |
| `Semantic Result` | semantic analyzer identity, Analysis Snapshot, Source/range, resolution/build context와 diagnostic | Derived `semantic_result` |
| `Semantic Annotation` | provider/model, purpose, included source scope, Analysis Snapshot, generated time, uncertainty와 retention state | Derived `semantic_result` 또는 `generated_interpretation`; structural fact가 아님 |
| `Agent Interpretation` | agent/host/session, 사용한 Source·Decision·Context·analysis basis, generated time, known gap와 uncertainty | Candidate 또는 Derived `generated_interpretation` |
| `User Correction` | explicit current-host user Source, correction/adoption 대상 identity, 의도와 적용 scope | Kernel이 허용한 canonical correction/supersession/Context operation; 원 provenance를 rewrite하지 않음 |

Semantic analyzer가 계산한 resolved relation과 provider가 생성한 natural-language
annotation을 구분한다. Agent가 Structural Fact를 설명할 수는 있지만 설명이 fact의
producer가 되지는 않는다. User Correction은 analyzer output을 과거부터 거짓으로
만드는 것이 아니라 canonical user authority와 적용 boundary를 기록한다.

Analysis refresh와 reanalysis는 User Correction, explicit adoption, Decision 또는
canonical Context를 수정하지 않는다. 새 결과가 correction과 충돌하면 새 Derived
result와 contradiction/review Candidate를 만들 수 있지만 어느 쪽도 조용히
overwrite하지 않는다.

## 8. Adapter와 common core 책임

### 8.1 Language/analysis adapter

각 adapter는 다음을 소유한다.

- 입력 Repository Snapshot과 requested area/capability 해석
- language/ecosystem-specific analyzer invocation 또는 local observation
- native output을 common entity/relation envelope로 normalize
- native coordinate와 diagnostic을 손실 없이 translate
- unsupported construct, build prerequisite, partial result와 failure 보고
- language-specific extension과 그 version-independent meaning
- produced result가 어떤 file/source/dependency에 근거했는지 보고
- 한 adapter failure가 다른 adapter result로 전파되지 않게 bounded result 반환

Adapter는 canonical identity를 발급하거나 Question/Decision을 해결하지 않는다.
Analyzer가 내놓지 않은 relation을 보완하기 위해 agent interpretation을 structural
또는 semantic fact로 삽입하지 않는다.

### 8.2 Repository Intelligence common core

Common core는 다음을 소유한다.

- Repository/Analysis Snapshot identity와 binding invariant
- analyzer와 독립적인 inventory
- normalized envelope와 stable identity validation
- capability/state/coverage aggregation과 honest degradation
- adapter result composition과 failure isolation
- freshness, diagnostics와 invalidation category propagation
- canonical IDs에 대한 read-only reference와 Candidate submission

Common core는 language grammar나 provider-specific output 의미를 발명하지 않는다.
공통 graph를 만들기 위해 provenance가 다른 relation을 합치거나 unsupported area를
empty success로 채우지 않는다.

### 8.3 Current Production structural responsibility

현재 Production 구현은 qualification evidence가 선택한 local Tree-sitter grammar를
일곱 gate language adapter 뒤에 둔다. 이 선택은 공통 envelope의 parser-independent
meaning을 바꾸지 않으며 parser framework 자체를 이 문서의 영구 product contract로
만들지 않는다. Production Repository Intelligence는 다음 구현 책임을 가진다.

- parser-owned declaration/range와 syntax relation만 normalize하고 unsupported 또는
  partial construct diagnostic을 함께 보존한다.
- source range를 Repository Snapshot에 bind된 half-open, zero-based line/UTF-8-byte
  column으로 기록한다.
- 같은 snapshot, adapter contract와 source basis에서 entity/relation identity와
  canonical serialization을 재현한다.
- file content, declared dependency, recognized build context와 adapter contract basis를
  기록하고 `file_content`, `dependency`, `build_context`, `adapter_contract`,
  `prior_failure` 등 명시적 category로 bounded refresh 또는 reuse를 설명한다.
- Local Operations가 Analysis 생성 시점에 machine-readable Git status로 관찰한
  repository-relative dirty path와 status fingerprint를 exact Analysis Snapshot의
  Derived repository observation으로 보존한다. Non-Git repository에는 Git metadata를
  요구하지 않으며 Checkpoint는 이후 current state에서 baseline dirty set을 재계산하지
  않는다.
- local search result에 Source/range, capability, coverage, diagnostic, provenance와
  freshness를 포함하고 historical range를 current navigation으로 표시하지 않는다.
- parser execution과 search를 process 내부 local operation으로 유지하며 repository
  source를 external service로 전송하지 않는다.

이 책임은 semantic resolution, compiler/LSP child process, provider annotation 또는
canonical mutation을 포함하지 않는다. Analysis Snapshot current writer는 structural과
semantic basis, invalidation, refresh metadata와 Project-scoped canonical target basis가 포함된
format version 5만 생성하며 이 derived
format은 source basis에서 rebuild하는 책임을 가진다. Maintained V01 report는 이 Production
boundary에서 seven-language structural gate를 passed로 판정하며 large-repository scaling과
complete macro/generated coverage는 주장하지 않는다.

### 8.4 Current Production semantic responsibility

V02 qualification evidence에 따라 현재 Production semantic path는 Java/Maven,
TypeScript/Node와 Rust/Cargo의 qualified Tree-sitter structural result 위에 별도의
in-process source-semantic symbol index를 둔다. 이 선택은 compiler/LSP completeness를
주장하지 않으며 다음 책임을 가진다.

- local declaration, qualified scope, declared arity/type, explicit implementation과 local
  module path로 확인 가능한 `defines`, `references`, `resolves_to`, `type_of`,
  `implements`와 `overrides` relation만 semantic provenance로 publish한다.
- 같은 이름 또는 overload 후보는 declared/call arity와 scope로 하나만 확인될 때
  resolve하고, 남은 ambiguity는 unresolved target과 diagnostic으로 보존한다.
- Java package/Maven manifest, TypeScript relative module/Node·`tsconfig`, Rust
  module/Cargo·explicit trait impl evidence를 build context로 기록하되 external package
  body, generated source와 compiler-only resolution을 current fact로 만들지 않는다.
- analyzer identity/version, zero-based UTF-8 source range, Analysis/Repository Snapshot,
  coverage, diagnostic, freshness와 usable remainder를 bounded result에 보존한다.
- missing dependency, recoverable structural gap와 ambiguous target은 `partial`, adapter
  미실행은 `unavailable`, 실행 실패는 `failed`로 보고하고 analyzer가 publish하지 않은
  semantic fact를 만들지 않는다.
- semantic failure 또는 cache rebuild는 inventory, structural fact, canonical record와 user
  correction을 삭제·수정하지 않는다. Source basis에서 deterministic rebuild한다.
- search와 explanation basis는 repository observation, structural fact, semantic result와
  agent interpretation을 서로 다른 statement/provenance class로 노출한다. Analysis
  entity의 Source/Decision/Context Item/Checkpoint link는 typed read-only reference이며
  canonical write authority가 아니다.

세 adapter는 local library call만 사용하므로 Production child process를 추가하지 않고
V10을 trigger하거나 완료했다고 주장하지 않는다. LSP, SCIP 또는 compiler-native
subprocess path를 나중에 선택하면 active Failure와 Recovery 계약의 V10 gate를 먼저
통과해야 한다.

## 9. Failure isolation과 degradation

Analysis는 language, area와 capability 단위의 bounded result다.

- 한 adapter crash 또는 malformed source는 영향 scope를 `failed` 또는 `partial`로
  만들되 unaffected inventory, language, area와 prior historical snapshot을 지우지
  않는다.
- Provider가 unavailable/failed이면 provider-backed semantic/annotation scope만
  degrade하고 local inventory와 지원되는 structural/ecosystem result를 유지한다.
- Empty result는 실제로 분석된 empty scope일 때만 success다. Spawn failure,
  unsupported construct, timeout과 parse failure를 empty success로 바꾸지 않는다.
- Diagnostic은 adapter/analyzer identity, affected scope, outcome과 usable remainder를
  보존한다.
- Partial result를 소비하는 projection은 coverage와 omission을 함께 표시한다.

Cross-subsystem failure/recovery matrix, process termination과 repair 절차는 active
[Failure와 Recovery 계약](failure-and-recovery.md)이 소유한다. 이 문서는 analysis
result가 failure를 숨기지 않고 격리해야 한다는 normal contract만 소유한다.

## 10. Freshness와 invalidation category

모든 result는 Repository Snapshot에 bound된다. Current repository observation이
달라지면 result를 current로 계속 제시하지 않는다. 최소 다음 contract category를
구분한다.

### File-level invalidation

한 file의 content, classification, availability 또는 analysis-relevant setting이
바뀌어 그 file에서 직접 생산된 entity, range, relation, diagnostics와 annotation의
freshness를 재평가해야 한다.

### Dependency-level invalidation

Manifest, build context, import/include/export, symbol resolution, generated-source input
또는 다른 declared dependency 변화가 dependent area의 semantic/ecosystem result와
관련 interpretation에 영향을 줄 수 있어 freshness를 재평가해야 한다.

이 두 category는 algorithm을 선택하지 않는다. 모든 file을 항상 재분석하는지,
dependency graph를 어떻게 계산하는지, cache key와 scheduling을 어떻게 구현하는지는
later production choice다. 구현은 최소한 어떤 category와 basis가 result를 stale로
만들었는지 설명할 수 있어야 한다.

## 11. Canonical identity linkage

Analysis entity, relation, result와 interpretation은 다음 canonical identity를
read-only reference할 수 있다.

- `Source`: repository snapshot, file, symbol, commit, manifest, command 또는 document
  basis
- `Decision`: 분석하거나 설명하는 architecture/implementation choice와 applicability
- `Context Item`: goal, constraint, fact, assumption, risk와 known limit
- `Checkpoint`: analysis snapshot이 관련된 meaningful work/pause/handoff state

Reference에는 target identity와 relevant revision/snapshot basis가 포함되고 대상의
ownership을 이전하지 않는다. Checkpoint link가 analysis freshness를 결정하거나,
Decision link가 analyzer result를 사용자 judgment로 만들거나, analysis가 canonical
record를 side effect로 수정해서는 안 된다.

## 12. Later-validation hooks

### V02 — Semantic adapter normalization

Maintained V02 report는 Java/Maven, TypeScript/Node와 Rust/Cargo의 Production
source-semantic boundary를 passed로 판정한다. 그 evidence는 다음을 검증한다.

- Semantic Result와 Structural Fact의 provenance 분리
- source range와 Analysis Snapshot binding
- definition/reference/type/implementation relation normalization
- overload와 동일 이름 target 구분, unresolved target 표현
- incomplete build와 dependency 부재의 `partial`/`unavailable`/`failed` degradation
- adapter diagnostic과 incremental freshness input
- Linux와 local-only journey에서 adapter 현실성

V02가 provider나 analyzer technology를 선택하더라도 이 문서의 accepted language
set과 common envelope를 조용히 축소할 수 없다.

### V11 — Combined multi-repository journey

V11은 Volicord repository, single-language application과 polyglot repository에서
다음을 결합 검증해야 한다.

- per-snapshot/per-language/per-area capability와 coverage honesty
- first structural gate 전체와 non-gate inventory fallback
- stable entity/range와 cross-component source grounding
- analyzer/provider partial failure isolation
- file/dependency change 뒤 stale/current 구분
- Canonical Source/Decision/Context/Checkpoint link의 identity 보존
- analysis와 generated interpretation이 user judgment를 mutate하지 않는 성질

V01의 small fixture 결과만으로 production accuracy나 completeness를 주장하지 않는다.

## 13. Non-goals

이 문서는 parser framework, compiler frontend, language service, index format,
database, serialization, embedding, ranking, scheduler, cache key, API, MCP method와
process topology를 선택하지 않는다. Portable bundle content·merge, general
failure/recovery policy, generated-document rendering과 legacy data path도 정의하지
않는다.
