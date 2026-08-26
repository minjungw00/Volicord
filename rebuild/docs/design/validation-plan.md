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
11. Generated-content language 성공은 requested-language metadata가 아닌 actual
    body realization으로 검증하고, 불가능하면 explicit unavailable/degraded를
    성공 fallback과 구분한다.

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

### 3.1 Maintained final, provider qualification, V11과 documentation handoff lifecycle

Production/test candidate를 봉인하는 exact final, V11 acceptance와 그 결과를 해석하는
documentation conclusion은 서로 다른 책임이다. Maintained lifecycle은 다음 한 방향이다.

```text
implementation and focused checks
→ admission gate
→ exact final once
→ separately authorized production provider qualification once
→ same-session V11 once
→ sanitized evidence archive creation and independent verification
→ sanitized evidence capsule
→ independent documentation-only conclusion
```

`rebuild/scripts/validate admission`은 exact final을 시작하기 전에 독립 실행할 수 있는
machine-readable preflight다. 현재 clean worktree와 candidate HEAD, validation runner와
V11 self-check, architecture contracts, realistic Repository Intelligence, redesigned
Dogfood campaign/harness와 provider qualification self-check, required fixture
identity/integrity, executable, disposable filesystem/runtime
home, repository-owned bounded disk estimate, loopback, Codex executable/authentication,
technical external-network state와 maintained authenticated V11 transmission을 평가한다.
Blocker가 하나라도 있으면 exact final command count와 official V11 command count는 모두
0이다.

Authenticated V11은 installed Codex CLI가 사용하는 OpenAI Codex service를 destination으로
하고, 세 target(`volicord`, `small-python`, `polyglot-medium`)에서 installed
`project_health` MCP tool을 선택하는 세 bounded turn을 purpose/scope로 하는 외부 전송을
필요로 한다. Intended transmitted scope는 bounded V11 prompt, Project identity와 tool
result이며 repository source body 전송은 의도하지 않는다. 이 전송에는 current
invocation의 exact assertion
`v11-openai-codex-project-health-three-targets`가 필요하다. Credential 소유,
`--external-network available`, sandbox escalation, 이전 session/report, 또는 Project
provider opt-in에서 authorization을 추론하지 않는다. Missing assertion은
`authorization_blocked`이며 operator prose나 credential content는 retained evidence에
저장하지 않는다.

`rebuild/scripts/validate gate`는 자체 admission을 다시 평가하는 유일한 exact-final
entry point다. Admission이 통과하면 gate parent process는 admission에서 기록한 HEAD를
다시 확인하고 existing final owner의 ordered command vector를 정확히 한 번 실행한다.
그 호출이 직접 반환한 새 `summary.json`만 읽으며 older ignored artifact를 검색하거나
대체하지 않는다. 모든 exact command와 `failure_count = 0`을 확인한 경우에만 그 같은
HEAD에서 separately authorized production-provider qualification을 정확히 한 번
실행한다. 이 stage가 통과한 경우에만 final path와 HEAD를 existing V11 preflight에
전달한다. Preflight가 통과한 경우에만 official
V11을 정확히 한 번 실행하고 credential-retention audit을 수행한다. Final failure는
provider/V11이나 final retry를 만들지 않고, provider failure는 V11이나 final retry를
만들지 않으며, V11 failure도 final/provider retry를 만들지 않는다. Direct
`rebuild/scripts/validate final` invocation은 이 lifecycle을 우회할 수 없도록 거부한다.

Exact final의 command vector, ordering, invocation count와 orchestration owner는 이 단일
gate/runner에만 남는다. Final clippy command의 intended acceptance는 exit success에
더해 preserved stdout/stderr에 compiler/clippy warning이 없는 warning-clean result다.
문서, 별도 script 또는 later session이 exact final이나 V11을 복제·재실행하여
이 계약을 대신하지 않는다.
어느 stage까지 진행됐든 candidate identity가 있으면 gate는 bounded sanitized evidence
archive를 만들고 independent verifier로 검사한다. Exact final, V11과 credential audit
성공은 필요조건일 뿐이며 archive creation/verification까지 성공한 뒤에만 top-level gate와
handoff가 full readiness를 표시한다. Archive stage 전에는 retained capsule과 gate result가
`evidence_archive_pending`과 `phase_8_ready = false`를 기록하고, creation 또는 verification
failure는 대응하는 blocker를 보존한다.

Exact final은 그 HEAD의 production code와 tests가 통과한 candidate라는 사실을
봉인한다. 이후 V11과 credential audit이 acceptance evidence를 만들며, 나중의
documentation-only conclusion commit까지 exact-final candidate에 포함됐다는 뜻은
아니다. Documentation session은 copied capsule과 tracked maintained input만 해석하고
production/test code를 바꾸거나 final/V11을 다시 실행하지 않는다.

Gate는 numeric legacy version branch가 없는 현재 `validation_handoff_capsule` 하나를
stdout에 전부 출력하고 ignored `capsule.json`에도 쓴다. Capsule은 다음 bounded evidence를
보존한다.

- validated candidate HEAD, sanitized admission check name/status, pre-final candidate check와
  blocking classification
- 실제 Linux OS/release/platform, machine/architecture와 Python runtime identity
- bounded `--version` probe에서 얻은 Python, Git, Cargo, Rust compiler와 installed Codex
  CLI version 또는 probe별 explicit `unavailable`/`error` state
- reconstruction `Cargo.lock`, workspace `Cargo.toml`과 maintained fixture manifest의 path,
  SHA-256 및 V11 required fixture identity
- gate의 reproducible `argv`, technical external-network assertion, exact bounded
  authorization assertion ID, maintained destination/purpose/target/source scope
- exact final aggregate status/failure count/summary SHA-256와 command별 actual `argv`,
  outcome, exit/termination/spawn state 및 duration
- final artifact가 같은 gate invocation에서 생성되고 V11 preflight와 official V11에
  전달됐는지를 나타내는 artifact-flow fact
- V11 status/result SHA-256, fixture identity, required-step/status count,
  `phase_8_ready`, credential-audit count/result, target별 authenticated Codex
  classification과 reported active Decision revisit-trigger ID
- production provider qualification status/evidence SHA-256, exact provider/model/source
  scope, usable success/degradation outcome와 raw material non-retention state
- sanitized evidence archive filename/SHA-256/size/member count, independent verification
  status와 archive 이전 prerequisite completion state

Sanitized evidence archive의 current process representation은 payload마다 하나의
`sanitized_argv_policy`를 두고, execution마다 sanitized argv와 projected/redacted
argument의 index·classification·semantic role만 한 번 기록한다. Policy에서 생략된
argument-role entry는 approved structural token이라는 명시적 default이며 independent
verifier의 closed allowlist를 통과해야 한다. Exact raw argv는 ignored local execution
evidence에만 남는다. Builder는 tar를 쓰기 전에 모든 JSON member를 encode하고 manifest가
선언하는 256 KiB uncompressed per-member bound를 검사하며, verifier는 같은 선언과
bound를 독립적으로 다시 검사한다. 기존 512 KiB compressed archive bound도 별도로
유지한다. 다른 process schema, numeric format branch 또는 legacy decoder는 두지 않는다.

Version probe는 fixed non-secret command만 사용하며 environment variable, home content,
username 또는 unrelated host metadata를 수집하지 않는다. Capsule은 Credential/API/session
token, `auth.json` content, credential content나 reusable fingerprint, source body, full
command log, raw provider payload와 private prompt body를 포함하지 않는다. 따라서 raw
`.local` evidence가 session 뒤 삭제돼도 copied capsule은 독립 conclusion handoff로
충분하며, ignored artifact의 cross-session persistence는 maintained contract가 아니다.

`check-validation-report`의 기본 one-report mode는 기존 generic report shape만 검사한다.
V11 documentation conclusion은 `--capsule <copied-capsule.json>`을 함께 전달하는 semantic
mode를 사용한다. 이 mode는 raw `.local` artifact를 읽거나 추론하지 않고 capsule의
structured value를 report section과 비교해 candidate/environment/tool/dependency,
exact command/configuration, final/V11 hash와 count, credential audit, `phase_8_ready` 및
Decision revisit-trigger state가 실제로 기록됐는지 검사한다. Capsule에 value가 있는데도
version이나 command가 unavailable/not projected라고 대체한 report는 통과하지 않는다.

Semantic mode는 success 전용 capsule을 가정하지 않는다. 같은 현재 capsule contract에서
관찰된 stage를 admission status, blocker, final status, official V11 status와 same-session
artifact-flow fact로 판정하고 다음 evidence를 조건부로 요구한다.

- admission 또는 immediate pre-final check가 막히면 blocker를 뒷받침하는 sanitized check를
  요구하고 final/V11/audit evidence는 `not_run`과 truthful false flow로 둔다.
- final이 실패하면 exact final command/process/failure/hash evidence를 요구하고 V11
  preflight와 official V11 evidence를 허용하지 않는다.
- successful final 뒤 V11 preflight가 실패하면 같은 gate final artifact의 production과
  preflight consumption을 요구하고 official V11 evidence는 `not_run`으로 둔다.
- official V11이 시작된 뒤 실패하면 successful final과 same-session ownership, 실제 V11
  result/status/count, attempted target outcome과 credential-audit evidence만 요구한다.
- fully passed이면 exact final, 세 target, official V11, credential audit과 모든 artifact-flow
  fact에 더해 sanitized archive identity와 independent verification이 완전할 때만
  `phase_8_ready = true`를 허용한다.

아직 시작하지 않은 stage의 hash, command, authenticated target 또는 consumption evidence는
요구하지 않는다. 반대로 earlier-stage failure와 later-stage success를 함께 주장하거나,
gate-produced final 없이 V11 consumption을 주장하거나, required target이 빠진 V11 pass처럼
관찰 순서와 모순되는 조합은 거부한다. 별도 failure schema, numeric capsule version 또는
legacy decoder는 두지 않는다.

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

Fixture는 작지만 각 언어의 핵심 차이를 포함해야 한다. 이 deterministic
fixture gate는 adapter contract를 검증하지만 realistic repository generalization
evidence를 대체하지 않는다. Production qualification은 V11의 multi-file
single-language application과 medium polyglot repository에서 practical analysis usefulness,
cross-component grounding과 resource behavior를 별도 검증한다.

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
- requested-language metadata/HTML tag와 generated body language의 분리

### 통과 조건

- 모든 핵심 architecture claim에 source 또는 명시적 inference marker가 있다.
- 문서 metadata에 Project, snapshot, Decisions, capability coverage, gaps, generator와 time이 있다.
- partial analyzer 영역을 complete로 표현하지 않는다.
- generated document는 명시적 adoption 전 canonical record를 변경하지 않는다.
- user-specified path가 없으면 Product Repository에 쓰지 않는다.
- Connected model이 요청 언어를 실현하면 generated body가 실제 그
  언어로 생성되고, 실현할 수 없으면 English body success가 아닌
  explicit `unavailable`/`degraded` result를 남긴다.
- Project Understanding body와 diagram이 required work/Decision/code/architecture meaning,
  fact/interpretation distinction과 inspectable relation-grounded topology를 유지한다.

## 11. V07 — Privacy와 local-only mode

### 목표

external semantic provider 없이 핵심 기능을 사용할 수 있고, interactive host access와 background transmission을 구분하는지 검증한다.

### 시나리오

- semantic provider 미설정
- Candidate collection 활성/selected-scope opt-out와 pre-existing Candidate
- Candidate retention expiry, explicit deletion과 promoted Candidate
- Project opt-in 전 background analysis 요청
- opt-in 후 explicit scope 전송
- current production background semantic-provider dispatcher/transport의 실제 success request
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
- technical network/credential availability와 exact source-transmission authorization의 분리
- production provider request/result usability, provenance, retention와 transmitted manifest

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
- 첫 replacement qualification은 mock/stub이 아닌 production dispatcher/transport에서
  최소 한 건의 usable background semantic-provider success를 보존한다.
- Project opt-in, network, authentication과 별개인 current-invocation exact
  provider/purpose/source-scope transmission authorization 전에는 dispatch하지 않는다.

### Maintained production-provider qualification

`rebuild/validation/privacy/background-provider-qualification/`은 V07의 production
dispatcher/transport evidence owner다. Maintained entrypoint는 network-free `--self-test`와
explicit `--live` mode를 분리한다. Live mode는 caller가 exact assertion
`openai-codex-background-semantic-bounded-rust-v1`을
`--authorize-source-transmission`으로 전달할 때만 실행한다. 이 assertion은 V11의
`v11-openai-codex-project-health-three-targets`와 다르며 서로 대신할 수 없다.

승인 범위는 authenticated OpenAI Codex service의 `openai-codex` provider, caller가
명시한 exact model, `qualify the bounded background semantic provider fixture` purpose,
`semantic_annotation` capability와 maintained one-file
`fixtures/bounded-rust/src/lib.rs` Source뿐이다. Harness는 승인을 생성하거나 저장하지
않고, missing/다른 assertion이면 fixture read와 live subprocess 전에
`authorization_blocked`로 종료한다. Sanitized evaluation은 request outcome, manifest
locator/byte count, snapshot/provenance completeness와 degradation classification만 보존하며
Source body, provider response body/event stream, credential과 raw provider artifact는
보존하지 않는다. Live success 뒤 missing configured executable을 사용한 독립 요청으로
`provider_unavailable`, `not_transmitted`, Guarded confirmation consumption과 local canonical
continuity를 함께 확인한다. Provider-side deletion은 unsupported로 남긴다.

## 12. V08 — Linux install과 Codex integration

### 목표

clean Linux 환경에서 install, Project init, Codex 연결과 health를 반복 가능하게 검증한다.

### 시나리오

- clean install
- binary path와 permissions
- Runtime Home init
- Project init/bind
- install-only global-registration exclusion과 explicit repository-scoped Codex enable/disable
- trusted-project-owned setup과 `startup|resume|clear|compact` SessionStart activation
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
- task-oriented CLI discovery, repository-relative Project resolution, human-readable default
  output, explicit structured mode와 actionable error/next step
- ordinary command의 opaque Project-ID-free journey
- no legacy dependency or runtime access
- unauthorized second repository에 project-local Volicord config/hook이 없음
- hook matching/nonmatching execution 모두 Runtime Home과 canonical state를 touch하지 않음

### 통과 조건

- documented command로 clean install과 first Project journey가 가능하다.
- install만으로 user-global Volicord MCP가 생기지 않고, explicit enable 뒤 authorized
  trusted repository에서만 required MCP와 SessionStart hook이 발견된다.
- unrelated config/hook 보존, tracked/unowned conflict rejection과 exact disable removal을
  deterministic하게 검증한다.
- Codex가 high-level MCP surface를 발견하고 Recall/Decision/Checkpoint를 호출할 수 있다.
- authenticated activation probe의 plain repository request가 repository inspection 전에
  `project_resolve`와 existing-Project `recall`로 진입한다. Explicit `project_health`
  connection probe는 별도 deterministic subject로 유지한다.
- 연결 실패와 degraded capability를 구분한다.
- Current host가 Guarded response를 받을 수 있고, 받을 수 없으면 local viewer/CLI가
  같은 logical confirmation identity/revision과 Source linkage를 유지한다.
- uninstall/reinstall이 canonical user data를 조용히 삭제하지 않는다.
- active product에 legacy command alias, import 또는 migrate path가 없다.
- Bound repository의 ordinary CLI journey는 Project UUID를 요구하지 않고, ambiguous/
  unbound state는 explicit init/bind/select next action을 제공한다.

## 13. V09 — Recall과 Checkpoint 정확성

### 목표

첫 project-scoped request의 bounded Recall과 meaningful boundary의 Checkpoint가 실제 작업 맥락을 정확히 복구하는지 검증한다.

### 시나리오

- prior Decisions와 Checkpoint가 있는 fresh agent session
- unrelated greeting
- large context with truncation
- stale Source와 superseded Decision
- ordinary work with unrelated dirty changes
- fresh/resumed work에서 Recall 뒤 첫 ordinary repository write 전 Analysis Snapshot baseline
- bounded work 뒤 처음 만든 Analysis Snapshot을 Checkpoint baseline으로 제출하는 rollout
- completed, paused와 handoff boundary
- verification pass, fail와 not-run
- pending/promoted/dismissed/expired Candidate와 Candidate Inspection degradation

### 측정 항목

- Recall selection precision/recall
- no-mutation property
- omitted count와 reason
- repeated Decision/Question rate
- dirty change attribution
- pre-write baseline/first-write/Checkpoint baseline identity ordering
- Checkpoint false-positive/false-negative
- work/verification/review state separation
- Candidate promotion authorization/disposition과 inspection attribute completeness
- Candidate Inspection no-mutation과 failure isolation

### 통과 조건

- unrelated greeting에는 project Recall을 수행하지 않는다.
- first project-scoped request에서 bounded brief를 제공한다.
- active, stale, superseded와 unavailable context를 구분한다.
- existing dirty changes를 current Checkpoint 변경으로 포함하지 않는다.
- fresh/resumed meaningful work는 first ordinary repository write 전에 만든 exact Analysis
  Snapshot을 Checkpoint baseline으로 사용하며 first post-work analysis는 이를 대신하지 않는다.
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

Official V11 실행은 3.1의 admission과 exact final을 통과한 같은 gate process/session만
소유한다. 별도 session의 prior final artifact를 preflight input으로 요구하거나
대체해서는 안 된다.

### 대상

1. Volicord 자체 Rust workspace
2. 여러 production-like source/test/config 파일을 이어 이해하고 behavior를
   변경·검증하는 소규모 단일 언어 application; trivial arithmetic/example
   edit는 qualification work가 아님
3. 최소 세 언어, 문서/config, component boundary와 cross-language request/data
   flow가 있는 현실적인 중간 규모 polyglot repository

### journey

```text
clean Linux install
→ test-owned project trust와 repository-scoped Codex enable
→ Codex connection
→ Project init/bind
→ inventory and capability analysis
→ source-grounded explanation
→ Candidate collection, inspection and bounded promotion/disposition
→ evidence-appropriate inquiry behavior, including no Question when correct
→ ordinary work
→ exact Guarded confirmation and effect outcome where applicable
→ source-grounded Checkpoint
→ process restart and new-session Recall
→ bundle export/import to another clone
→ divergent conflict handling
→ correction, supersession and deletion
→ four document outputs
→ requested-language body realization or explicit unavailable/degraded result
→ one authorized production background semantic-provider success path
→ provider/parser/index failure recovery
```

### 통과 조건

`acceptance-scenarios.md`의 최종 통과 조건을 모두 만족한다. 하나의 repository에서만 통과한 결과로 cutover gate를 열지 않는다.

특히 Candidate collection/inspection/promotion/retention journey와 Guarded effect의 exact
action/target/effect/scope/revision/expiration match, user-response Source, single-use/reuse
rejection, no-dispatch-before-valid-confirmation, ordinary-action non-blocking 및
indeterminate no-silent-retry behavior를 같은 integrated run에서 검증한다.

### Phase 8 naturalistic Dogfood qualification

Phase 8 Dogfood full passage는 V11 scripted conformance와 별개의 real-session qualification
이다. 하나의 current candidate에 대해 `volicord`, `small-python`, `polyglot-medium` 세
actual repository class에서 정확히 두 cycle씩 실행하고, 각 cycle은 globally distinct한 fresh
VS Code Codex work session과 fresh resume session을 사용한다. 따라서 automated
qualification에는 `3 repositories × 2 cycles × 2 sessions = 12`개의 distinct real
session이 필요하다. Current result schema는 `automated_qualification`, `human_review`,
`replacement_qualification`을 분리한다. 모든 machine requirement가 통과하면 human review가
`not_provided`여도 automated command는 성공하지만 replacement는
`pending_human_review`이며 `replacement_pass_candidate`와 `phase_9_ready`는 false다.

각 cycle descriptor는 unique Question, alternatives, recommendation, terminal outcome,
Decision 또는 prescribed user selection을 evaluator 정답으로 두지 않는다. 대신 pinned
repository revision의 actual owner contract, repository facts, delegated boundaries,
non-exhaustive material concerns와 consequences, user에게 물어서는 안 되는 facts를 담은 bounded
evaluation basis를 보존한다. Independent control review는 해당 cycle의 inquiry behavior class와
판정 근거를 보존한다. `explicit_user_owned_decision`과 `hidden_user_owned_decision`은 material
concern의 존재만으로 봉인할 수 없다.
Independent reviewer는 exact frozen task를 user Question 없이 완수하는 counterfactual을
repository facts, accepted Decision/contract, delegated authority와 frozen request를 완전히
만족하는 narrower implementation 관점에서 시도한다. Complete하고 defensible한 no-question
path가 claimed user-owned outcome을 선택하지 않고 frozen task를 만족하면 descriptor를
material user-owned class로 봉인하지 않는다. Review provenance는 scope,
safe relative path, SHA-256와 repository revision을 보존하는 typed reference를 사용한다.
Path/hash 검증은 reviewer의 semantic materiality 판단을 대신하지 않는다.

Accepted explicit/hidden user-owned review는 다음을 bounded evaluator material로 보존한다.

- 남아 있는 specific externally meaningful outcome
- exact frozen task가 그 outcome을 반드시 만나는 이유
- repository/environment research가 이를 해결하지 못하는 이유
- accepted Decision/contract가 이를 해결하지 못하는 이유
- delegated boundary가 아닌 이유
- viable alternatives 사이의 materially different consequences
- 검토한 no-question approach와 각 approach가 frozen task를 실패하거나 같은 user-owned
  outcome을 implicit하게 선택하는 이유
- `unavoidable_user_owned_outcome` conclusion

이 evidence는 clear source-grounded externally meaningful user-owned outcome을 식별해야 한다.
Repository facts, accepted contract/Decision 또는 delegation이 outcome을 정하거나 viable outcomes의
material consequence 차이가 사라지면 user-owned positive control이 아니다. Independent review 뒤에도
hidden case가 ordinary delegated 또는 conventional implementation detail로 plausibly 설명되면
`hidden_user_owned_decision`으로 봉인하지 않는다.

`explicit_user_owned_decision`의 frozen work task는 externally meaningful outcome이 unresolved임을
진실하게 disclosure할 수 있다. `hidden_user_owned_decision`의 frozen task는 ordinary realistic
repository request이며 unresolved policy, evaluator alternatives, user-choice requirement,
materiality concern, Volicord/Inquiry/Question Candidate/Decision/Checkpoint/Recall 또는 behavior
class를 드러내지 않는다. Static leak check는 conservative supplement일 뿐이며 semantic
non-disclosure는 independent review가 소유한다.

Hidden review는 full evaluator basis를 보기 전에 campaign-generated random
`review_slot_id`, candidate와 pinned revision, exact frozen tasks, work scope,
owner-document location과 `reviewer/workspaces/<review_slot_id>/repository`의 별도 pinned
source-inspection clone만 담은 reviewer-preparation artifact로 repository/owner를 조사하고
provisional classification/materiality conclusion을 같은 opaque ID와 preparation SHA-256에
고정한다. Reviewer-visible preparation content, filename, workspace path와 index는 repository
class, logical cycle, behavior class와 fixed matrix position을 포함하지 않으며 opaque-ID
순서로 표시한다. 그 뒤에만 evaluator concern, alternatives, recommendation과
counterfactual conclusion을 비교한다. Hidden descriptor는 provisional/final review가 모두
`material_outcome_unavoidable = true`와
`operator_prompt_does_not_disclose_material_outcome = true`를 기록해야 봉인된다. 이 경계는
confirmation bias를 줄이는 workflow isolation이며 OS secrecy 주장이 아니다.

Question wording, exact alternatives, recommendation이나 expected user selection은 이
review의 정답이 아니다. `research_or_no_question`, `delegated_implementation_choice`와
`exploratory_uncertainty`에는 counterfactual이 `not_required_for_behavior_class`이며 user
Decision ceremony를 추가하지 않는다. Evaluator와 independent reviewer의 repository fact
또는 authority conclusion이 다르면 `unresolved_conflict` 상태로 봉인을 차단한다. Conflict는
typed source/active-owner provenance를 인용해 `resolved_from_evidence`가 되거나 conclusions가
`agreed`가 된 뒤에만 accepted review가 될 수 있다.

Reviewer-safe contract는 다음 behavior-class vocabulary와 각 class의 의미를 제공한다.
이 목록은 현재 campaign의 multiplicity, duplicated class, coverage requirement 또는
repository placement를 공개하지 않는다.

- `explicit_user_owned_decision`: ordinary task가 unresolved material outcome을 disclosure하는
  positive control이며 Question과 explicit current-host Decision이 필요함
- `hidden_user_owned_decision`: ordinary task는 outcome을 disclosure하지 않지만 complete work가
  user-owned material outcome을 반드시 만나며 repository investigation 뒤 agent가 이를 발견해
  Question과 explicit current-host Decision을 기록해야 함
- `research_or_no_question`: research, accepted contract 또는 repository fact로 user interruption이
  불필요하며 no-question outcome이 맞음
- `delegated_implementation_choice`: user가 이미 위임한 implementation boundary 안에서 agent가
  선택하고 Question을 만들지 않음
- `exploratory_uncertainty`: prototype, 추가 research 또는 evidence-backed defer가 immediate user
  choice보다 적절함

Campaign preparation은 exact realized assignment와 release-qualification profile을
evaluator/steward-private integrity-bound state에만 보존한다. Profile의 histogram, duplicate,
coverage와 repository distribution은 모든 six provisional review가 immutable하게 고정된 뒤에만
reveal하고 검증한다. Logical cycle number, opaque slot ID, workspace path, reviewer filename,
operator label과 presentation order는 behavior class를 encode하지 않는다. Question wording이나
하나의 answer를 고정하지 않는다.
Simple repository는 Decision count를 채우기 위해 user Decision을 제조하지 않는다. 한
small-Python cycle은 work와 resume 사이에 user interruption이 전혀 없어도 통과할 수 있다.
Work/resume prompt는 Volicord operation order, 물어야 할 Question, outcome, Checkpoint content,
Recall 또는 prescribed user selection을 지시하지
않는다.

Qualifying work session은 exact first work task를 current-host Goal Context의 Source로
보존하고, repository analysis로 baseline을 만든 뒤 ordinary work를 시작한다. Agent는
research, active Decision/delegation와 materiality를 판단해 적절한 behavior class를 선택하고
evidence를 남긴다. Material Question이 필요하면 Question Candidate/research/promotion
경계와 current-host response linkage를 모두 적용한다. Question이 필요하지 않은 경우
Question/Decision absence와 그 evidence를 정답으로 허용한다. User Decision은 displayed
Question revision에 대한 explicit current-host user response에서만 기록하며 agent
recommendation이나 implementation preference를 response로 사용하지 않는다. Meaningful
completion/pause는 Goal, baseline, applicable Decision 또는 no-Decision behavior basis, actual
changed basis와 numeric-exit verification을 연결한 source-grounded Checkpoint를 요구한다.
Work session은 pause/handoff history를 포함해 하나 이상의 successful Checkpoint를 가질 수
있다. Qualification은 마지막 meaningful repository change 뒤의 latest Checkpoint candidate를
terminal state로 결정하며 malformed final candidate에서 earlier valid Checkpoint로 fallback하지
않는다. 선택된 terminal Checkpoint만 ordinary-work qualification의 Goal/baseline,
applicable Decision 또는 evidence-backed no-Decision behavior basis과 correlated numeric-exit
verification을 충족해야 한다.

Fresh resume session의 exact first task에는 Project ID가 포함되지 않는다. Repository
inspection 또는 continued work 전에 current repository path로 `project_resolve`가
`found`를 성공적으로 반환하고, 그 result의 Project identity가 cycle canonical bundle의
Project와 같으며 current binding identity/revision을 포함해야 한다. 같은 session의
Recall은 이 successful resolution 뒤, repository inspection/continuation 전에 발생한다.
Resume session은 identity를 얻기 위해 `project_initialize`로 replacement Project를
만들 수 없다. Work/resume session의 global distinctness 조건은 그대로 유지한다.

Fresh resume session은 successful Recall 뒤 local `repository_analyze` baseline을 만들고
그 Analysis Snapshot identity를 첫 ordinary repository write 전에 보존한다. Change
continuation의 eventual grounded Checkpoint는 이 exact pre-write identity를 사용하며 first
post-edit analysis는 baseline으로 qualification되지 않는다. Current provenance가 edit ordering을
deterministically 증명하지 못하므로 timestamp나 dirty-state heuristic으로 이를 대체하지 않고
rollout operation order와 exact identity linkage로 검증한다.
Work와 resume session에 추가 successful `repository_analyze`가 first write 전, write 뒤 또는
validation 뒤 존재할 수 있다. Qualification은 analysis call count나 first-call heuristic을 쓰지
않고 각 applicable Checkpoint의 `baseline_analysis_snapshot_id`와 일치하는 same-Project
successful analysis evidence를 선택해 required Goal/Recall boundary 뒤와 first meaningful write
전 completion을 검증한다. Unknown identity, wrong-Project analysis, pre-Recall resume analysis와
post-write baseline substitution은 실패한다.

Resume continuation은 두 mode를 허용한다. `change_continuation`은 Recall과 inspection 뒤
relevant repository change와 그 뒤의 별도 numeric-exit validation을 요구한다.
`verified_state_continuation`은 recalled terminal Checkpoint가 `completed`이고 inspection이
그 state가 current임을 확인하며 post-inspection numeric-exit validation이 있고 final behavior가
completed state와 충돌하지 않을 때 source mutation 없이 통과할 수 있다. Paused/in-progress
Checkpoint나 meaningful unfinished next step이 있는 state는 no-change mode를 사용할 수 없고,
Recall 뒤 inspection/validation 없이 끝난 session도 통과하지 않는다.

Work-capture intake는 product inquiry behavior보다 먼저 repository-scoped SessionStart activation
evidence를 확인한다. Activation이 없으면 operator/environment setup failure로 분류하고 그
campaign path를 중단하며 Question/Decision 부재를 product failure로 귀속하지 않는다.

Internal harness는 completed real work capture 뒤 machine-observable terminal failure를
보존하기 위한 failure-only command를 제공한다.

```text
python3 rebuild/validation/dogfood/harness.py qualify-work-blocker \
  --candidate-head <current-candidate-head> \
  --descriptor <one-cycle-descriptor.json> \
  --repository <exact-pinned-cycle-repository> \
  --work-capture <completed-work-rollout.jsonl> \
  --output <blocker-result.json>
```

이 path는 current candidate, valid descriptor와 completed capture의 repository class,
cycle, revision, `source=vscode`, `originator=codex_vscode`, fresh thread와 exact first
`work_user_task`를 먼저 검증한다. Completed work capture에 required high-level Project,
Goal, baseline, evidence-backed behavior classification 또는 grounded Checkpoint operation이
없으면 later resume이 그 operation을 work session에 retroactively 추가할 수 없으므로
terminal blocker다. 두 user-owned class는 material Question Candidate/promotion과
current-host user Decision이 없으면 blocker이지만 다른 class에 이 operation을 요구하지
않는다. 반대로 capture만으로 required semantic fact를
증명할 수 없으면 blocker를 추측하지 않고 full qualification을 요구한다. 모든 required
work-session condition을 충족한 positive capture는 early-stop failure로 변환할 수 없다.

Early-stop output은 `kind = phase8_dogfood_blocker_result`이며 항상
`campaign_complete = false`, `replacement_pass_candidate = false`,
`phase_9_ready = false`다. Candidate, repository class, cycle, revision, failed check,
completed capture SHA-256와 later required session/check의 `not_run` 상태만 보존한다. Plain
task text, evaluator behavior-review reasoning, source body, credential과 raw provider content는
보존하지 않는다.
이 result는 full passage, Phase 8 completion 또는 Phase 9 readiness의 evidence가 아니다.

Campaign 준비와 routine evidence collection은 maintained internal helper인
`rebuild/scripts/dogfood-campaign`을 사용한다. 사용자는 repository/hook trust를 직접 승인하고,
12개의 genuinely naturalistic VS Code Codex chat을 실행하며 agent가 genuine material
Question을 제시한 경우에만 실제로 답하고 raw rollout을 한 번에 제공한다. Helper는
rollout intake, activation validation,
blocker gating, Project identity extraction, canonical bundle export와 hash, bounded Runtime
summary, descriptor evidence completion, manifest assembly와 review packaging을 담당한다.
Ordinary review에 full Runtime Home을 추출하거나 package하지 않는다.
`activate-all`은 각 repository-scoped enable 뒤 production-owned ownership manifest, MCP entry,
SessionStart hook과 exact candidate-local executable/Runtime binding을 다시 읽어 검증하며
불일치하면 naturalistic execution 전에 실패한다. 이 static postcondition은 repository/hook
trust를 자동 승인하지 않고 VS Code가 SessionStart를 실제 실행했다는 증거도 아니다. Trust
또는 activation setup이 불확실하면 operator는 frozen task를 보내기 전에 이를 직접 검사해야
하며 각 raw work/resume capture의 real SessionStart evidence는 계속 필수다. Missing runtime
activation diagnostic은 capture/hash/session, opaque slot, work/resume role과 관찰 가능한
Volicord MCP call 존재 여부를 보존하고 product inquiry failure와 분리한다.
`prepare`는 모든 campaign mutation 전에 evaluator/steward-private qualification profile과
assignment를 만들고, 6개의 unique cryptographic-random opaque slot과 qualifying
repository/Runtime Home을 `slots/<review_slot_id>/...` 아래에 준비한다.
Evaluator/steward-private mapping만 opaque slot을 repository class, logical cycle, expected
behavior class와 authoritative descriptor에 연결하며 mapping은 campaign SHA-256와 evidence
inventory에 묶인다. Numeric old/new layout branch나 prior campaign migration은 없다.
Independent evaluator/control은 actual repository와 pinned revision을 조사하고 prescribed
selection이 아닌 behavior-class review를 준비·독립 검토한 뒤 maintained helper로 cycle
descriptor를 봉인한다.
Evaluator material은 operator instruction, example 또는 review index에 넣지 않으며 operator에게
descriptor를 직접 열거나 수정하라고 요구하지 않는다. Naturalistic operator는 intended
repository를 검사하고 trust하며 SessionStart hook을 명시적으로 승인하고, required fresh VS
Code Codex session을 열어 generated run sheet의 frozen work/resume task만 보낸다. Genuine
material Question이 실제로 제시된 경우에는 본인의 답을 제공하고 12개 raw
rollout을 session 사이의 control 접촉 없이 보존한다.
`prepare`는 evaluator input과 operator material을 분리한다. `prepare-review`는 opaque
`review_slot_id`, exact candidate/pinned revision, frozen work/resume tasks, work scope,
owner-document location과 opaque reviewer workspace만 reviewer plane에 동결하고 evaluator
repository class/logical cycle mapping, concerns, alternatives, recommendation,
user-owned outcome과 counterfactual conclusion은 제외한다. Phase A reviewer는 repository
source와 listed owner document를 조사하되 qualification-control implementation이나
evaluator/steward state가 아닌 prepared reviewer plane만 campaign evidence로 사용한다. 이는
workflow isolation이며 OS secrecy 주장이 아니다. Independent control agent가 provisional
review를 작성하면 `record-provisional-review`가 exact candidate,
opaque `review_slot_id`, preparation identity와 strict reviewer-visible schema를 검증하고 immutable
private artifact와 hash/inventory binding을 고정하면서 `review_prepared`에서
`provisional_recorded`로 성공 전환한다. 이 operation은 evaluator descriptor를 읽지 않고
reviewer가 기록한 classification에서만 materiality와 disclosure self-consistency를 계산한다.
Maintained vocabulary에 속하고 internally consistent한 provisional conclusion은 private evaluator
class와 일치하지 않아도 성공하며, 결과는 repository class, logical cycle, evaluator behavior
class 또는 match/mismatch를 노출하지 않는다. 모든 six provisional artifact와 hash가 고정되기
전에는 qualification-profile reveal과 `seal-cycle` evaluator reveal을 모두 거부한다.
`reveal-qualification-profile`은 `provisional_count = 6`과 모든 immutable hash를 확인한 뒤 private
profile을 검증하고 reveal state를 고정한다. 그 뒤 `seal-cycle`은 이미 고정된 provisional
artifact를 byte/hash 그대로 유지한 채 full evaluator basis와 비교한다. Structured
`classification_comparison`은 provisional/evaluator classification과 classification, materiality,
unavoidability, prompt-disclosure 차이를 기계적으로 열거한다. 일치한 비교는 `agreed`, 불일치는
inspectable provenance와 bounded basis를 가진 `resolved_from_evidence`일 때만 descriptor를
봉인할 수 있고 `unresolved_conflict` 또는 불일치를 `agreed`로 표시한 입력은 거부된다. 이 두
단계가 완료되기 전에는 activation과 rollout collection이
거부된다. Sealing은 authoritative hidden descriptor를 evaluator plane에 두고 semantic hash를
동결하며 exact work/resume task와 opaque authoritative workspace/Runtime path만으로 operator
run sheet를 다시 만든다. Run sheet는 repository grouping 안에서도 opaque-ID 순서를 쓰며
logical cycle label이나 behavior class를 표시하지 않는다.
Operator-facing artifact의 leak check는 prescribed Question, evaluator concern, recommendation,
prescribed selection, counterfactual approach, fact/authority agreement,
behavior-review reasoning과 evaluator-only sentinel을 거부한다. 이 분리는 workflow/evidence
isolation이며 evaluator file을 의도적으로 여는 user에 대한 OS security boundary 주장이 아니다.
6개 descriptor가 모두 봉인되면 steward는 session 시작 전에 `activate-all`을 실행할 수 있지만
repository/hook trust는 계속 user-controlled다. Default operator flow는 frozen task로 12개 fresh
chat을 모두 실행하고 raw rollout을 한 번에 `collect-batch`에 제공한다. Batch operation은 12개
explicit path 또는 정확히 12개 file만 있는 directory를 받고, state mutation 전에 frozen first
task, exact workspace/revision, work/resume role, `source=vscode`, `originator=codex_vscode`, fresh
session identity와 SessionStart activation으로 unordered input을 전역 mapping한다. Ambiguity,
duplicate, missing capture, identity mismatch와 session reuse는 전체 mapping을 거부한다.

Mapping 뒤 raw byte와 SHA-256를 보존하고, terminal work blocker가 있어도 resume evidence로 이를
복구하지 않는다. Missing activation은 operator/environment invalid로 유지한다. 다른 capture는
bounded diagnostic과 안전하게 식별 가능한 evidence extraction을 위해 계속 parse한다. Extraction은
Project identity, canonical bundle, bounded Runtime/activation summary, descriptor evidence reference,
supported product document-export path로 네 initial kind 각각의 Markdown과
self-contained HTML을 deterministic private evidence path에 생성한다. Summary는 모든
kind/format의 status, bounded failure basis 또는 relative path, bytes와 SHA-256를 보존하고,
operator document-review index는 produced path만 노출한다. 한 kind라도 usable evidence가
없으면 automated document evidence가 실패한다. Public static Viewer snapshot
capability도 cycle마다 campaign fixed locale/language로 self-contained read-only HTML을 만들고
path, bytes, SHA-256와 Project/candidate basis를 summary에 기록한다. Review package는 이
summary/index, produced documents와 Viewer snapshots를 포함하지만 Runtime Home, SQLite/sidecar, raw Derived
Analysis, credential, prompt, provider payload와 source copy는 계속 제외한다.
각 cycle의 document/Viewer evidence는 requested-language generated body를 실제 검사하고,
Project Understanding required meaning, verified-fact/generated-interpretation distinction과
diagram relation grounding을 machine-inspectable basis와 human review surface에 보존한다.
Campaign 전체에서 current production background semantic-provider dispatcher/transport의
별도로 authorized successful request/result를 최소 하나 요구하고 network/credential
availability, Project opt-in과 source-transmission authorization을 독립 evidence로 남긴다.
Ordinary independent review의 handoff는 byte-exact raw rollout archive와 bounded review
package 두 artifact를 함께 요구한다. Raw rollout은 bounded package의 default member가 아니며
별도 private archive로 전달한다. Full Runtime Home은 이 handoff의 일부가 아니다.

Automated run은 repository/candidate identity, 12-session semantics, post-reveal private
qualification profile, bundle/provenance, document와 static snapshot 생성,
requested-language realization, production
provider success authorization, machine accessibility, resource, regression, Decision revisit와
candidate cleanliness를 독립 판정한다. Human review는 immutable automated result 뒤에
한 번만 생성하며 replacement qualification에 필수다. Lowest-numbered 또는 lowest
automated-passed cycle 하나를 repository class 대표로 삼지 않고 모든 qualifying cycle을
검토한다. 각 cycle에서 source-vs-interpretation comprehension, repository-analysis
usefulness, CLI usability, Viewer Project Understanding, four-document usefulness, Question
necessity/Decision comprehension과 interruption cost를 평가한다. Interaction review는 explicit
material handling quality, hidden material discovery quality와 다른 세 class의 unnecessary
interruption을 구분한다. 두 user-owned quality criterion은 Question 존재만으로 통과하지
않는다. Affected work 전에 필요한 independently material user-owned dimension을 모두
식별하고, 각 dimension을 독립적으로 제시하거나 coupled choice의 모든 material consequence를
진실하게 disclose하며, recommendation·preferred API shape·implementation이 별도 material
dimension을 조용히 선택하지 않았음을 human reviewer가 확인해야 한다. Exact evaluator
wording, alternative label, expected answer 또는 하나의 decomposition은 요구하지 않고,
agent에 위임된 trivial implementation detail은 별도 Question 누락으로 판정하지 않는다.
Evaluator-private concern과 counterfactual evidence는 naturalistic execution 뒤 bounded review
grounding으로만 사용하며 frozen operator task와 work/resume session에는 노출하지 않는다.
Polyglot cycle은 언어·component
경계와 flow comprehension을 추가하고, static Viewer readability와 Volicord live Viewer의
`en`/`ko` keyboard/focus/color/zoom accessibility도 campaign에서 검토한다. Human fail은
automated pass를 훼손하지 않지만 replacement를 fail하며, human pass도 machine failure를 override할
수 없다. `prepare-human-review`와 `qualify-review`는 session 또는 machine Dogfood를 rerun하지 않는다.

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
