# 재구축 기술 검증 계획

- 상태: 실행 준비 완료
- 목적: 확정된 제품 계약을 production architecture와 구현 전에 실험으로 검증
- 제품 결정 기준: `product-charter.md`, `open-decisions.md`
- 실사용 판정 기준: `acceptance-scenarios.md`
- 원칙: 검증은 구현 방식을 선택하기 위한 것이며 accepted product scope를 조용히 축소하는 절차가 아님

## 1. 검증과 제품 결정의 경계

제품 결정과 기술 검증 상태를 분리한다.

- `open-decisions.md`는 사용자가 확정한 제품 의미와 revisit trigger를 소유한다.
- 이 문서는 그 의미를 구현할 수 있는 기술 조합, 위험과 known limit를 검증한다.
- prototype 실패는 자동으로 제품 Decision을 바꾸지 않는다.
- 다른 구현 접근을 평가한 뒤에도 accepted contract가 현실적으로 불가능하다는 근거가 있을 때만 새 제품 Question을 등록한다.
- disposable spike code는 명시적 review 없이 production contract나 dependency로 승격하지 않는다.

## 2. 공통 실험 규칙

모든 검증은 다음을 지킨다.

1. 입력 fixture와 source revision을 고정한다.
2. 실행한 command, 환경, dependency와 결과를 기록한다.
3. 성공 결과뿐 아니라 실패, unsupported, partial과 timeout을 보존한다.
4. 구조적으로 확인된 fact와 agent-generated interpretation을 구분한다.
5. 외부 semantic provider 사용 여부와 전송 source 범위를 기록한다.
6. spike output은 runtime 또는 experiment artifact이며 maintained design truth가 아니다.
7. production에 채택하는 primitive는 새 책임, dependency boundary와 테스트를 다시 정의한다.
8. legacy Runtime Home, schema와 API를 검증 입력 또는 compatibility target으로 사용하지 않는다.
9. Linux에서 실행하며 Codex를 첫 host integration 대상으로 사용한다.
10. 각 실험은 재현 가능한 report를 남긴다.

Maintained Wave 1 asset은 capability 기준 경로인
`rebuild/validation/repository-intelligence/polyglot-structural/`,
`rebuild/validation/canonical-context/portability/`와
`rebuild/validation/inquiry/frontier-resume/`에 둔다. Fixture catalog와 report
template은 각각 `rebuild/validation/shared/fixture-manifest.json`과
`rebuild/validation/shared/report-template.md`가 공동 소유한다. V01, V03와
V05는 path가 아니라 stable validation metadata와 report identifier다.

### Experiment language와 production language ownership

- Python 또는 다른 적합한 external language는 disposable feasibility
  experiment, fixture orchestration, external analyzer invocation,
  cross-language black-box comparison과 end-to-end harness에 사용할 수 있다.
- Rust는 production domain semantics, Canonical Context invariant, durable
  storage behavior, Inquiry transition, production serialization, production
  crash/recovery behavior와 production integration/property test를 소유한다.
- Python experiment를 Rust production behavior의 두 번째 long-lived reference
  implementation으로 유지하지 않는다.
- production 의미를 disposable Python prototype만으로 검증된 상태로 남기지
  않는다.
- V01 orchestration은 language-neutral black-box validation에 이점이 있으면
  external로 유지할 수 있다.
- V03와 V05 semantics는 production promotion 전에 실제 Rust production
  implementation을 대상으로 다시 표현하고 검증한다.

## 3. 검증 보고서 형식

각 실험은 다음 형식의 보고서를 작성한다.

```text
Validation ID and title
Status
Goal
Accepted decisions being validated
Input repositories and revisions
Environment and tool versions
Candidate approaches
Commands and configuration
Observed results
Coverage and failures
Performance and resource observations
Privacy and external transmission
Acceptance results
Known limits
Recommended implementation choice
Rejected alternatives and reasons
Reusable primitive decision
Decision revisit trigger status
Follow-up work
Artifacts
```

보고서에는 raw 전체 source나 secret를 복제하지 않는다. 필요한 artifact는 ignored experiment output에 두고 maintained report에는 path, hash와 요약만 기록한다.

## 4. 실행 순서

```text
Wave 1 — Core feasibility
V01 Polyglot structural analysis
V03 Canonical Context and portable bundle
V05 Inquiry frontier and resume

Wave 2 — Quality and integration
V02 Semantic adapters
V04 Divergent bundle merge
V06 Source-grounded documents
V09 Recall and Checkpoint

Wave 3 — Trust and product operation
V07 Privacy and local-only mode
V08 Linux install and Codex integration
V10 Reusable process/filesystem primitives

Wave 4 — Combined acceptance rehearsal
V11 End-to-end multi-repository journey
```

V01, V03와 V05는 서로 독립적으로 시작할 수 있다. 목표 architecture를 확정하기 전에 Wave 1 결과가 필요하다.

## 5. V01 — Polyglot structural analysis

### 목표

Java, Python, JavaScript, TypeScript, C, C++와 Rust를 하나의 Repository Intelligence model로 표현할 수 있는지 검증한다.

### 입력 fixture

- Java Maven 또는 Gradle project
- Python `pyproject.toml` project
- JavaScript Node project
- TypeScript Node 또는 monorepo
- C CMake project
- C++ CMake 또는 `compile_commands.json` project
- Rust Cargo workspace
- 최소 세 언어가 섞인 polyglot repository
- 첫 structural 목록 밖의 텍스트 언어 repository

Fixture는 작지만 각 언어의 핵심 차이를 포함해야 한다.

### 비교할 접근

- 공통 incremental parser framework와 언어별 query/normalizer
- compiler frontend 또는 언어별 parser를 직접 사용하는 접근
- 두 접근의 혼합

특정 library 선택은 이 문서의 계약이 아니다.

### 공통 내부 model 후보

```text
CodeEntity
- repository
- package
- module / namespace
- file
- class / interface / trait
- struct / enum / type
- function / method
- field
- test
- configuration
- document

StructuralRelation
- contains
- declares
- imports / includes / exports
- inherits / implements
- calls_syntactically
- tests
- configures
```

언어에 존재하지 않는 개념을 억지로 채우지 않는다. 언어별 extension이나 capability-specific property를 허용한다.

### 측정 항목

- known declaration recall과 precision
- source range 정확성
- stable entity identity
- package/module/file hierarchy
- import/include/export 관계
- test detection
- syntax error와 partial parse
- macro, generated code와 conditional compilation coverage
- incremental update 범위와 안정성
- polyglot normalization 난이도
- analysis 시간, peak memory와 output size
- parser unavailable 또는 crash degradation

### 통과 조건

- 모든 첫 structural 언어에서 known entity와 range를 재현 가능하게 추출한다.
- 언어별 unsupported construct와 coverage를 표현한다.
- 동일 snapshot을 반복 분석했을 때 안정적인 entity identity와 serialization을 얻는다.
- 변경된 file에 대해 전체 재분석 없이 affected derived state를 갱신할 수 있다.
- 한 언어 analyzer 실패가 다른 언어 inventory와 structural result를 무효화하지 않는다.
- first structural language set을 축소하지 않고 production 후보 architecture를 제시한다.

### 실패 시

- 언어별 adapter를 분리하거나 internal model extension을 추가한다.
- parser-only로 보증할 수 없는 relation을 capability에서 제거하고 semantic 또는 agent interpretation으로 이동한다.
- 모든 합리적 접근이 accepted structural gate를 충족하지 못할 때만 Q2 revisit trigger를 제기한다.

## 6. V02 — Semantic adapter normalization

### 목표

최소 세 ecosystem에서 definition, reference, type와 implementation 관계를 공통 semantic model로 정규화할 수 있는지 검증한다.

### 후보 입력

V01 fixture 중 analyzer 생태계와 설치 가능성이 좋은 최소 세 곳을 선택한다. 선택 이유는 다음을 비교해 기록한다.

- indexer 또는 language service의 maturity
- reproducible setup
- build 준비 요구
- offline 가능성
- result stability
- license와 distribution 영향
- Linux packaging 가능성

### 비교할 접근

- language server protocol 기반 query
- language-neutral code index format
- compiler 또는 native analyzer output
- parser structural result와 semantic result의 결합

### semantic model 후보

```text
SemanticRelation
- defines
- references
- resolves_to
- type_of
- implements
- overrides
- instantiated_by
```

### 측정 항목

- known definition/reference accuracy
- overload와 동일 이름 symbol 구분
- implementation/override 관계
- package dependency resolution
- incomplete build 또는 missing dependency degradation
- index 생성과 incremental update 시간
- output normalization complexity
- source snapshot binding
- analyzer diagnostics와 failure visibility

### 통과 조건

- 최소 세 ecosystem에서 fixture의 known semantic relation을 source range와 함께 제공한다.
- analyzer가 실행되지 않았거나 실패한 경우 semantic capability를 `unavailable` 또는 `failed`로 표시한다.
- semantic result와 parser structural fact를 별도 provenance로 저장한다.
- broken-build fixture에서도 가능한 capability와 누락 이유를 정직하게 제공한다.
- chosen adapters가 Linux distribution과 Codex journey에 현실적으로 포함 가능하다.

### 실패 시

- 첫 세 ecosystem 후보를 바꾸거나 adapter approach를 교체한다.
- semantic capability가 없는 언어도 inventory, agent_assisted와 structural mode를 유지한다.
- 최소 세 ecosystem gate가 여러 접근에서 불가능할 때만 Q2 revisit trigger를 제기한다.

## 7. V03 — Canonical Context와 portable bundle

### 목표

Repository Intelligence와 LLM 없이도 Project, Source, Question, Decision, Context Item과 Checkpoint를 durable하고 portable하게 보존할 수 있는지 검증한다.

### 필수 기능

- stable Project ID
- clone/path binding
- canonical record creation과 read
- revision과 supersession
- contradiction/review_due
- deletion과 minimal tombstone
- deterministic export/import
- schema/format version
- process restart
- crash/fault recovery
- derived state 완전 삭제 후 canonical read

### 입력 시나리오

- 새 Project 생성
- user Decision과 rationale
- source-grounded fact
- open Question dependency
- agent-authored Checkpoint
- record correction와 Decision supersession
- sensitive Context deletion
- 다른 path의 clone import

### 측정 항목

- transaction atomicity
- restart consistency
- deterministic serialization
- bundle size와 readability
- source ref portability
- deleted data 잔존 여부
- schema version handling
- derived index independence

### 통과 조건

- hard process termination 후 committed record만 복구된다.
- derived directory를 삭제해도 모든 canonical record를 읽을 수 있다.
- export/import 후 Project identity와 record relation이 유지된다.
- 다른 clone path에서 Source를 rebind할 수 있다.
- deleted sensitive content가 bundle, index와 recoverable log에 남지 않는다.
- legacy schema와 Runtime Home을 읽거나 참조하지 않는다.

## 8. V04 — Divergent bundle merge

### 목표

두 clone에서 독립적으로 바뀐 canonical context를 사용자 Decision을 덮어쓰지 않고 병합할 수 있는지 검증한다.

### 입력 시나리오

```text
common base bundle
├─ clone A: independent Context Item + Decision revision
└─ clone B: independent Checkpoint + same Decision semantic change
```

추가 conflict:

- independent record additions
- same-record non-semantic revision
- same Question state change
- conflicting Decision choice
- delete/modify
- supersede/supersede
- no source repository

### 측정 항목

- common base discovery
- automatic merge safety
- three-way presentation clarity
- conflict result portability
- branch relation
- Recall after merge

### 통과 조건

- independent additions는 안정적으로 자동 병합된다.
- semantic conflict와 delete/modify는 사용자 선택 없이 해결되지 않는다.
- conflict view는 base, A, B와 consequence를 설명한다.
- user resolution이 canonical Source와 함께 기록된다.
- source가 없어도 context conflict를 해결할 수 있다.
- merge 후 export/import와 Recall이 일관된다.

## 9. V05 — Inquiry frontier와 session resume

### 목표

material Question dependency를 단계적으로 제시하고 process/session 종료 후 같은 frontier를 복구하는지 검증한다.

### fixture decision tree

최소 다음 branch를 포함한다.

- repository fact로 해결되는 Question
- 사용자 가치 판단
- agent에 위임 가능한 implementation choice
- prototype이 필요한 UX choice
- 사용자가 모른다고 답하는 Question
- 상위 Decision으로 superseded되는 branch
- 독립 질문 batch

### 측정 항목

- materiality classification
- fact research before ask
- dependency frontier accuracy
- user turn과 Question revision linkage
- recommendation와 user choice 분리
- terminal branch handling
- pause/resume
- answered Question repetition

### 통과 조건

- 현재 prerequisite가 해결된 Question만 표시한다.
- repository에서 확인 가능한 사실을 사용자에게 묻지 않는다.
- 사용자의 `delegate`, `research`, `prototype`, `defer`와 `out_of_scope`를 보존한다.
- process 종료 후 같은 open frontier를 복구한다.
- answered Question을 표현만 바꾸어 반복하지 않는다.
- 질문 수 상한 없이 모든 material branch를 terminal 상태로 만들 수 있다.

## 10. V06 — Source-grounded 문서 생성

### 목표

Canonical Context와 Repository Intelligence에서 네 필수 문서를 생성하고 각 핵심 주장을 source와 capability에 연결하는지 검증한다.

### 문서

- Project & Architecture Guide
- Decision Report
- Implementation Plan
- Handoff / Resume Document

### 입력

- 최소 하나의 단일 언어 fixture
- polyglot fixture
- partial/failed analyzer가 있는 fixture
- active와 superseded Decision
- latest Checkpoint와 open Question

### 측정 항목

- source citation validity
- structural fact와 agent interpretation 구분
- included Decision completeness
- coverage/known-gap visibility
- stale invalidation
- Markdown portability
- self-contained HTML
- Korean, English와 추가 사용자 요청 언어 output

### 통과 조건

- 모든 핵심 architecture claim에 source 또는 명시적 inference marker가 있다.
- 문서 metadata에 Project, snapshot, Decisions, capability coverage, gaps, generator와 time이 있다.
- partial analyzer 영역을 complete로 표현하지 않는다.
- generated document는 명시적 adoption 전 canonical record를 변경하지 않는다.
- user-specified path가 없으면 Product Repository에 쓰지 않는다.

## 11. V07 — Privacy와 local-only mode

### 목표

external semantic provider 없이 핵심 기능을 사용할 수 있고, interactive host access와 background transmission을 구분하는지 검증한다.

### 시나리오

- semantic provider 미설정
- Candidate collection 활성/selected-scope opt-out와 pre-existing Candidate
- Candidate retention expiry, explicit deletion과 promoted Candidate
- Project opt-in 전 background analysis 요청
- opt-in 후 explicit scope 전송
- excluded file과 secret-like fixture
- annotation 삭제
- portable bundle export

### 측정 항목

- network/process observation
- transmitted path/source manifest
- exclude와 secret behavior
- opt-in persistence와 revoke
- annotation provenance
- Candidate retention/expiry와 opt-out state
- Candidate, annotation, canonical forgetting과 related Derived deletion propagation
- deletion completeness

### 통과 조건

- provider 미설정 상태에서 inventory, supported structural analysis, Decision, Checkpoint와 Recall이 작동한다.
- background semantic analysis는 명시적 Project opt-in 전 실행되지 않는다.
- 사용자에게 provider, model, source 범위와 exclusions를 표시한다.
- raw source body가 portable bundle에 포함되지 않는다.
- annotation 삭제 후 cache와 derived output에서 제거된다.
- Candidate opt-out은 selected scope의 새 automatic collection을 중단하고 existing
  Candidate를 explicit deletion/dismissal/promotion/retention expiry까지 inspectable하게
  유지한다.
- Candidate retention/deletion은 canonical target을 silent rewrite/delete하지 않고,
  canonical forgetting은 관련 managed Candidate/Derived content로 전파된다.

## 12. V08 — Linux install과 Codex integration

### 목표

clean Linux 환경에서 install, Project init, Codex 연결과 health를 반복 가능하게 검증한다.

### 시나리오

- clean install
- binary path와 permissions
- Runtime Home init
- Project init/bind
- Codex MCP setup
- Guarded confirmation의 current-host transport와 elicitation-unavailable fallback
- host restart
- health degraded/failure
- uninstall/reinstall

### 측정 항목

- required dependencies
- install artifacts
- startup time와 failure message
- adapter lifecycle
- exact confirmation request/revision과 user-response Source transport fidelity
- local viewer/CLI fallback equivalence
- process cleanup
- locale rendering
- no legacy dependency or runtime access

### 통과 조건

- documented command로 clean install과 first Project journey가 가능하다.
- Codex가 high-level MCP surface를 발견하고 Recall/Decision/Checkpoint를 호출할 수 있다.
- 연결 실패와 degraded capability를 구분한다.
- Current host가 Guarded response를 받을 수 있고, 받을 수 없으면 local viewer/CLI가
  같은 logical confirmation identity/revision과 Source linkage를 유지한다.
- uninstall/reinstall이 canonical user data를 조용히 삭제하지 않는다.
- active product에 legacy command alias, import 또는 migrate path가 없다.

## 13. V09 — Recall과 Checkpoint 정확성

### 목표

첫 project-scoped request의 bounded Recall과 meaningful boundary의 Checkpoint가 실제 작업 맥락을 정확히 복구하는지 검증한다.

### 시나리오

- prior Decisions와 Checkpoint가 있는 fresh agent session
- unrelated greeting
- large context with truncation
- stale Source와 superseded Decision
- ordinary work with unrelated dirty changes
- completed, paused와 handoff boundary
- verification pass, fail와 not-run
- pending/promoted/dismissed/expired Candidate와 Candidate Inspection degradation

### 측정 항목

- Recall selection precision/recall
- no-mutation property
- omitted count와 reason
- repeated Decision/Question rate
- dirty change attribution
- Checkpoint false-positive/false-negative
- work/verification/review state separation
- Candidate promotion authorization/disposition과 inspection attribute completeness
- Candidate Inspection no-mutation과 failure isolation

### 통과 조건

- unrelated greeting에는 project Recall을 수행하지 않는다.
- first project-scoped request에서 bounded brief를 제공한다.
- active, stale, superseded와 unavailable context를 구분한다.
- existing dirty changes를 current Checkpoint 변경으로 포함하지 않는다.
- 단순 조회나 변경 없는 설명에 canonical Checkpoint를 만들지 않는다.
- new session이 goal, rationale, current state, open Questions와 next step을 복구한다.
- Candidate Inspection이 existence, kind, provenance, collection scope, retention/expiry,
  promotion disposition과 opt-out state를 노출하고 read/failure가 Candidate를 mutate하지
  않는다.

## 14. V10 — 기존 process/filesystem primitive 재사용 평가

### 목표

기존 implementation에서 domain-independent primitive를 추출할 가치가 있는지 검증한다. crate 또는 API compatibility는 목표가 아니다.

### Process 평가

- child process containment
- timeout와 termination
- stdout/stderr bounded capture
- exit-status preservation
- child tree cleanup
- Linux behavior

### Filesystem/Git 평가

- path normalization
- symlink handling
- repository/worktree/clone identity
- dirty change observation
- source fingerprint
- atomic publication

### Storage pattern 평가

- transaction boundary
- crash/fault injection
- schema versioning
- repair behavior

### 통과 조건

- 필요한 primitive를 legacy workflow type 없이 정의할 수 있다.
- 새 workspace에 legacy crate dependency를 추가하지 않는다.
- moved/reimplemented code에 새 responsibility test가 있다.
- UserAction, Task, Write Ticket, Evidence, Guard admission 또는 legacy Runtime Home 의미가 유입되지 않는다.

### 결과 분류

```text
adopt_as_new_primitive
reimplement_from_behavior
reference_only
reject
```

## 15. V11 — End-to-end multi-repository rehearsal

### 목표

개별 spike가 결합됐을 때 실제 사용 가능한 하나의 Volicord journey를 제공하는지 검증한다.

### 대상

1. Volicord 자체 Rust workspace
2. 소규모 단일 언어 application
3. 최소 세 언어와 문서가 섞인 중간 규모 polyglot repository

### journey

```text
clean Linux install
→ Codex connection
→ Project init/bind
→ inventory and capability analysis
→ source-grounded explanation
→ Candidate collection, inspection and bounded promotion/disposition
→ staged Inquiry and user Decision
→ ordinary work
→ exact Guarded confirmation and effect outcome where applicable
→ source-grounded Checkpoint
→ process restart and new-session Recall
→ bundle export/import to another clone
→ divergent conflict handling
→ correction, supersession and deletion
→ four document outputs
→ provider/parser/index failure recovery
```

### 통과 조건

`acceptance-scenarios.md`의 최종 통과 조건을 모두 만족한다. 하나의 repository에서만 통과한 결과로 cutover gate를 열지 않는다.

특히 Candidate collection/inspection/promotion/retention journey와 Guarded effect의 exact
action/target/effect/scope/revision/expiration match, user-response Source, single-use/reuse
rejection, no-dispatch-before-valid-confirmation, ordinary-action non-blocking 및
indeterminate no-silent-retry behavior를 같은 integrated run에서 검증한다.

## 16. Architecture 확정 gate

다음이 완료되면 production architecture 문서를 확정할 수 있다.

Phase 3 evidence constraint와 architecture document ownership은
`architecture-inputs.md`가 소유한다. 이 입력 계약은 target architecture를
대신하지 않는다.

- V01 결과로 polyglot structural model과 language adapter boundary를 선택함
- V03 결과로 canonical storage, bundle과 revision model을 선택함
- V05 결과로 Question state와 frontier contract를 선택함
- 각 결과에 known limit와 rejected alternative가 있음
- Canonical Context Kernel이 analyzer와 host에 의존하지 않는 dependency graph가 가능함
- accepted decisions를 변경해야 하는 unresolved revisit trigger가 없음

V02, V04, V06와 V09는 architecture의 extension point와 acceptance를 구체화한다. 이 결과가 필요한 영역을 placeholder로 숨기지 않는다.

## 17. Production 코드 승격 gate

spike 코드 또는 legacy primitive를 production에 넣으려면 다음을 만족한다.

- 새 product responsibility가 문서화됨
- public 또는 internal contract가 accepted decision과 일치함
- disposable experiment assumption이 제거됨
- error, degraded와 recovery semantics가 정의됨
- fixture와 property tests가 있음
- source/license/dependency 검토가 완료됨
- legacy crate와 Runtime Home dependency가 없음
- benchmark나 coverage 수치가 재현됨

## 18. 검증 완료 판정

2단계 제품 결정 문서는 검증 시작 전에 완료된 것으로 본다. 기술 검증 단계는 다음이 모두 참일 때 완료된다.

- V01–V10 report가 존재함
- V01, V03와 V05가 통과함
- V02가 최소 세 ecosystem semantic 후보를 확정함
- V04가 conflict policy를 구현 가능하게 검증함
- V06가 네 필수 문서의 source-grounding을 검증함
- V07이 local-only와 opt-in boundary를 검증함
- V08이 Linux/Codex clean journey를 검증함
- V09가 Recall/Checkpoint 오귀속과 반복 질문을 검증함
- V10이 각 reuse candidate의 최종 분류를 남김
- V11 실행 계획에 필요한 architecture와 implementation backlog가 구체적임
- accepted product decision을 변경해야 하는 새로운 Question이 없거나 명시적으로 사용자에게 제출됨
