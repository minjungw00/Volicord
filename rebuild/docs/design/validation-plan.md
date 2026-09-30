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

`rebuild/scripts/validate admission`은 optional diagnostic/preflight다. Candidate
identity/cleanliness, required executables, disposable filesystem/runtime home,
bounded disk estimate, loopback, authentication material, network, model과 두
operator authorization을 검사한다. 성공은 `preflight_passed`와
`preflight_eligible = true`이며 authoritative `eligible`은 false다. Support는
`not_run`/`diagnostic_preflight_only`로 남고 exit 0은 cheap prerequisite 통과만
뜻한다. Gate는 이 artifact를 읽거나 신뢰하지 않으며 gate 직전에 standalone
admission을 실행할 의무는 없다.

Authoritative gate admission은 같은 cheap checks를 자체 실행한다. 하나라도
막히면 모든 support command를 `not_run`/`cheap_preflight_blocked`로 남기고
blocking check IDs를 기록한다. Final/provider/preflight/official V11/audit는
시작하지 않는다. Cheap checks가 통과한 경우 아래 deterministic support를 그
candidate revision에서 정확히 한 번 실행한다. Suite가 시작된 뒤에는 첫 failure로
중단하지 않고 모든 command의 stdout/stderr, numeric exit와 result를 보존한다.
Support failure는 `validation_failed`이며 Final을 시작하지 않는다.

| Local exact-revision owner | Preserved invariant/evidence |
| --- | --- |
| runner `self-test` | argv, streams, exit/signal/spawn truth, non-fail-fast aggregation |
| V11 `self-check` | required leaves/authenticated probe, recovery, credential audit, bounded execution and resource regressions |
| `gate-self-test` | blockers, one-call lifecycle, fresh Final selection, coverage execution and candidate continuity |
| `gate-entrypoint-self-test` | actual CLI in isolated candidates, dirty state, gate-parent Final binding and refusal controls |
| `evidence-archive-self-test` | real collector/sanitizer, bounds, membership/hash/mode/candidate and prohibited-content controls |
| report checker `--self-test` | report shape and capsule stage consistency, including contract-execution failure and skipped support |
| contract coverage + `--self-test` | required entrypoints, real non-ignored declarations and negative controls |
| architecture checker + `--self-test` | owner routing and cross-owner structural drift controls |
| realistic RI assertions | seven-language fixture matrix and absent/available external-corpus integration |
| Dogfood assertions | maintained contract checks, review workflow and one transitive harness self-test |
| Dogfood campaign self-test | collection/activation plus resume, realization, long-lived Project, repository state, task freezing and evidence controls |
| remediation integration | distinct MachineFindingTests: finite authority, attribution and immutable evaluation/lineage |
| provider qualification `--self-test` | provider evidence/degradation and source-boundary controls |
| fixture checker | actual maintained fixture hashes/integrity |

Reconstruction CI also runs the runner/gate/entrypoint/archive/report self-tests;
CI is an additional execution environment, not the local gate's execution owner.
The gate does not reuse a different HEAD's CI or diagnostic result.

No separate Dogfood harness command repeats the harness already invoked by
assertions. Authority regression IDs are checked against that completed harness
result rather than rerunning their suite. Campaign owns current repository-state, task-freezing, evidence-control,
resume, document-realization and long-lived Project regressions previously
selected again by remediation. Assertions/harness own WorkflowTests, FrontierTests,
ContractTests and PolicyTests/FileBoundaryTests; remediation retains the distinct
MachineFindingTests. Gate self-test's former direct V11 required-step/credential
regressions remain owned by V11 self-check.

Final's existing all-targets/all-features workspace test command owns the formerly
repeated remediation Rust selections: operations `multi_work_project` and
`work_authority`, projections `current_work_flow` and `project_documents`, viewer
`viewer` (understanding, disclosure and bounded profile), and host `mcp` (Inquiry,
Decision and learning). These crates declare no package feature variants and the
removed filters/`--exact`/`--nocapture` did not add integration semantics. Final also
owns RI `realistic_qualification` and the injected structural adapter-failure unit
test. The external-corpus Rust run remains: its explicit
`VOLICORD_EXTERNAL_CORPUS_ROOT` supplies different evidence from the ordinary
workspace run. Official V11's Rust fixture-control invocations also remain because
they seed/inspect that journey's actual Runtime Home and Project.

Contract coverage is registration/execution evidence, not a semantic oracle.
The mapping's owner is `exact_candidate_final_workspace_tests`. Static inspection
accepts only direct integration-test declarations with plain `#[test]`, strips
comments/literals, and rejects ignored/conditional/nested/ordinary declarations.
After successful Final, the gate requires each mapped source to be a test target
in Final's actual Cargo metadata and each unique mapped name to have exactly one
`ok` record in Final's preserved workspace test stdout. Omitted/ignored/ambiguous
execution is `contract_coverage_execution_failed` and starts no provider or V11.
The capsule preserves this separate result. Registry/checker success does not prove
the domain adequacy or semantic correctness of the test bodies.

Authenticated V11은 installed Codex CLI가 사용하는 OpenAI Codex service를 destination으로
하고, 세 target(`volicord`, `small-python`, `polyglot-medium`)에서 installed
`project_health` MCP tool을 선택하는 세 bounded turn을 purpose/scope로 하는 외부 전송을
필요로 한다. Gate의 explicit `--provider-model`은 background qualification과 세
인증된 probe에 동일하게 전달하며 host/global default model로 대체하지 않는다.
Official V11 run은 `--model`을 명시하고 missing/empty model이면 전송 전에 거부한다.
Intended transmitted scope는 bounded V11 prompt, Project identity와 tool
result이며 repository source body 전송은 의도하지 않는다. 이 전송에는 current
invocation의 exact assertion
`v11-openai-codex-project-health-three-targets`가 필요하다. Credential 소유,
`--external-network available`, sandbox escalation, 이전 session/report, 또는 Project
provider opt-in에서 authorization을 추론하지 않는다. Missing assertion은
`authorization_blocked`이며 operator prose나 credential content는 retained evidence에
저장하지 않는다.

`rebuild/scripts/validate gate`는 authoritative admission을 자체 평가하는 유일한 exact-final
entry point다. Admission이 통과하면 gate parent process는 admission에서 기록한 HEAD를
다시 확인하고 existing final owner의 ordered command vector를 정확히 한 번 실행한다.
그 호출이 직접 반환한 새 `summary.json`만 읽으며 older ignored artifact를 검색하거나
대체하지 않는다. 모든 exact command와 `failure_count = 0`, mapped-test execution을 확인한 경우에만 그 같은
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

- contract-coverage execution owner/status/mapped-test count from Final metadata and test output
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
유지한다. Verifier는 membership/hash/mode/size/candidate와 prohibited-content
integrity를 검사하며 Final, provider, V11이나 Product semantics를 독립 재실행하지
않는다. Technical qualification은 gate의 실제 execution evidence가 소유한다.
다른 process schema, numeric format branch 또는 legacy decoder는 두지 않는다.

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
- broad feature Goal 아래 서로 독립적인 public API/failure choice
- broad reload Goal 아래 서로 독립적인 persistence/reload semantics
- actual outcome 하나가 반드시 함께 해결하는 legitimate coupled choices

### 측정 항목

- materiality classification
- fact research before ask
- dependency frontier accuracy
- user turn과 Question revision linkage
- recommendation와 user choice 분리
- terminal branch handling
- pause/resume
- answered Question repetition
- Engineering Choice identity completeness와 every-choice-to-one-dimension binding
- independent-choice collapse rejection과 symmetric complete coupling acceptance
- broad Goal이 subordinate observable/durable outcome authority를 대신하지 않는 counterfactual

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
- canonical Work/Git independence: zero commits (including unborn HEAD), multiple commits
  inside one Goal, changing HEAD, distinct Works without an intervening commit, a later
  combined commit, overlapping paths and pre-existing dirty state
- fresh/resumed work에서 Recall 뒤 첫 ordinary repository write 전 Analysis Snapshot baseline
- current Goal/baseline에 bind된 typed Materiality Review, dimension별 disposition과 explicit
  executable path/component/work-context scope 뒤의 `ready_for_work`
- same Goal/baseline의 prior Engineering Choice Discovery, exact choice identity mapping,
  independent/coupled completeness와 effect-category non-authority
- inactive/explicit-active learning participation, independent routine/deliberation-worthy assessment,
  agent-owned Learning Deliberation과 user-owned Question priority
- equivalent successor Review의 unsupported routine/inactive reset rejection, supported prior revision inheritance, typed choice/alternative completion continuity, coupled artifact expansion과 reopened branch blocking
- unsupported deliberation-worthy-to-routine revision rejection, prior learning path preservation과
  Source-backed research/prototype 또는 exact current-user withdrawal/narrowing revision basis
- first authoritative review의 pre-mutation success와 post-mutation backfill rejection
- production/test/document multi-file executable scope의 first-Checkpoint success, parent-root
  descendant coverage, material expansion 뒤 rebind requirement와 changed-path late expansion rejection
- 한 Checkpoint의 multiple uncovered path/component/work-context가 current binding/review basis와
  maintained next action을 포함한 one-response diagnostic으로 aggregate되는지
- disposition, authority anchor, blocking readiness 또는 affected-scope applicability를 늦게 수정한
  dimension의 affected path가 maintained baseline/current evidence로 증명되면 later fact/agent/
  delegation/Decision/research/learning-ready meaning이 earlier work를 certify하지 않는 상태
- bounded work 뒤 처음 만든 Analysis Snapshot을 Checkpoint baseline으로 제출하는 rollout
- completed, paused와 handoff boundary
- verification pass, fail와 not-run
- pending/promoted/dismissed/expired Candidate와 Candidate Inspection degradation

### 측정 항목

Work/Git regression은 existing Production owners에서 disposable real Git repository를
사용한다. `volicord-operations/tests/multi_work_project.rs`는 exact Goal continuation,
work-scoped Decision/rejected cross-work application, Checkpoint history, fresh Recall,
CLI, Project Understanding와 네 document의 Work identity/Decision grounding을 각 Git
boundary 전후에 검증한다. `tests/work_authority.rs`의 grounded regression은 unborn HEAD와
multi-commit A, same-HEAD dirty B, same-path delta와 combined commit에서 actual Checkpoint
publication, baseline dirty evidence와 stable Goal identity를 검증한다.
`volicord-viewer/tests/viewer.rs`는 shared-path A/B를 dirty, combined-commit와 refreshed
HEAD 상태에서 읽어 separate Work card, state, Decision/Checkpoint membership과 no-mutation을
검증한다. 이 focused Product evidence는 Naturalistic Git qualification 또는 authoritative
Final/V11 gate를 실행하거나 그 결과를 대신하지 않는다.

- Recall selection precision/recall
- current-host turn의 behavior-changing Goal/Learning/Preference/Constraint decomposition과 fresh
  Recall recovery precision
- no-mutation property
- omitted count와 reason
- repeated Decision/Question rate
- dirty change attribution
- pre-write baseline/first-write/Checkpoint baseline identity ordering
- Checkpoint false-positive/false-negative
- work/verification/review state separation
- Candidate promotion authorization/disposition과 inspection attribute completeness
- fact/settled/delegated/exploratory/user-owned work-authority behavior와 false-authority rejection
- fact/settled exact-authority coverage, non-empty remaining-alternative rejection과 related
  architecture/convention evidence의 constrain-without-settle behavior
- hidden API/failure와 persistence/reload choice non-collapse, legitimate coupling과 trivial internal
  detail non-discovery
- `end-to-end/multi-repository/materiality_scenarios.py`의 installed-MCP 7-case matrix:
  tuple precedent만 있는 새 public result와 returned-versus-thrown failure는 invalid authority
  rejection → 실제 Question Candidate/promotion → presentation receipt → current-host response →
  Decision → ready → first affected write 순서를 검증한다. Exact accepted contract, applicable
  Decision reuse, exact delegation, unique mechanical fact, private helper는 추가 Question 없이
  진행한다. 이 matrix는 V11 Inquiry 결과의 필수 child evidence이며 isolated Runtime Home을 쓴다.
- 위 matrix와 Dogfood의 sanitized chronological-capture regression은 deterministic qualification이다.
  Authenticated V11의 project-health connection probe와 active model의 semantic ownership 품질을
  혼동하지 않는다. 실제 model의 자연스러운 ownership 판단은 newly gated HEAD의 fresh campaign에서
  별도로 평가하며 private evaluator label이나 정답을 model 입력에 넣지 않는다.
- normal-mode autonomy, active pending blocker/Checkpoint refusal, response-before-feedback ordering,
  select/delegate/skip/prototype/reconsideration terminal/research state와 restart reconstruction
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
- new session이 separately recorded canonical Learning, Preference와 Constraint를 role/Source basis와
  함께 복구하며 Goal-only turn에는 behavior context를 제조하지 않는다.
- Candidate Inspection이 existence, kind, provenance, collection scope, retention/expiry,
  promotion disposition과 opt-out state를 노출하고 read/failure가 Candidate를 mutate하지
  않는다.
- Restart 뒤 Materiality Review revision과 unresolved requirement가 유지되고, agent
  recommendation/library convention/implementation preference/fake delegation이 ready authority로
  바뀌지 않는다.
- Fresh resume에서 no-new-choice guidance가 retained Candidate inspection과 non-empty stable-choice
  re-evaluation 또는 verified-state read-only path를 직접 제공하며 invalid empty Discovery를 요구하지 않는다.
- Restart 뒤 pending Learning Deliberation은 pending이고 completed selection은 exact bounded
  implementation basis로 남는다. Delegate/skip은 Decision을 만들지 않고 prototype request는
  research state이며, user-owned outcome은 learning transition으로 해제되지 않는다.

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

Installed Product의 deterministic integrated journey가 정의된 technical
invariant를 만족하는지 검증한다. Naturalistic work quality, context recovery의
실용성, Question relevance/necessity, interruption cost와 document usefulness는
Phase 8 repeated Dogfood와 qualitative review가 소유한다. Technical V11 성공은
Phase 9 approval이나 replacement passage가 아니다.

Official V11 실행은 3.1의 admission과 exact final을 통과한 같은 gate process/session만
소유한다. 별도 session의 prior final artifact를 preflight input으로 요구하거나
대체해서는 안 된다.

Official preflight and run independently require current `HEAD` to equal the
validated candidate exactly, with a clean tracked and untracked worktree. No
ancestor equivalence or validation-only dirty override qualifies. The current gate
parent issues a private Final binding (invocation, parent process, exact candidate,
artifact path and digest), inherited only by its preflight/run children. Both
commands validate the actual Final artifact against the runner-owned ordered
command vector and preserved command results. Missing, fabricated, changed, or
prior-gate evidence is rejected before V11 dispatch.

V11 repeats candidate and bound-Final validation at run completion and publication;
its result records both boundaries and the current gate identity. Gate consumption
requires exact result candidate, Final path/digest and invocation agreement. The
gate also checks candidate continuity after Final, before V11, after V11/audit and
before archive/publication. A changed HEAD or dirty worktree blocks readiness and
archive promotion; artifact hash verification alone cannot establish qualification.

### 대상

1. Volicord 자체 Rust workspace
2. maintained 소규모 단일 언어 application fixture; technical V11은 scripted
   adapter/invariant rehearsal이며 실제 source/test/config behavior work는 Phase 8가 검증
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
→ Volicord target: continue exact Work A, author distinct Work B/C with new Goal,
  fresh Analysis Snapshot, resolved Materiality Review and grounded Checkpoint
→ reject Work A-scoped Decision on Work B without canonical mutation; record a valid
  B Checkpoint after rejection and preserve Project Purpose
→ read current/historical A/B/C Work identities through CLI/MCP and portable clone
→ bundle export/import to another clone
→ divergent conflict handling
→ correction, supersession and deletion
→ four document outputs
→ requested-language body realization or explicit unavailable/degraded result
→ one authorized production background semantic-provider success path
→ provider/parser/index failure recovery
```

Controlled derived-index recovery는 변경하지 않은 repository에서 repair 전후의
Canonical Recall 필드(Goal basis, Decision rationale, Checkpoint의 verification/review/
acceptance, 열린 질문, risk/assumption, next step 포함)를 그대로 비교한다. Source 목록과
상세 정보의 identity 일치 및 중복 부재, 기존 non-Repository Source와 유지된 Source의
provenance 보존을 별도로 확인한다. Repair가 새로 만든 Repository Source는 available/
current이며 새로운 observation basis를 가져야 한다. 복구 후 Recall의 Analysis Snapshot은
repair 결과와 일치하는 새 식별자와 current repository freshness를 가져야 하며, 같은
repository를 관찰한 capability의 state, coverage, diagnostics, uncertainty는 보존해야
한다. 관찰 시각과 snapshot 식별자 갱신 자체를 Canonical 변경으로 판정하지 않는다.
이 구분은 정상 갱신의 통과뿐 아니라 Canonical 내용, Source provenance, freshness,
capability/coverage 손실 및 불완전한 복구 증거의 거부를 self-check로 검증한다.
Recall의 명시적 transport omission은 capability 손실 자체가 아니다. V11은 corruption
직전 및 repair 직후의 같은 Project/Analysis Snapshot에 묶인 로컬 snapshot metadata에서
전체 capability 근거를 읽고, Recall의 표시된 값과 정확한 생략 수를 대조한 뒤 위의 보존
검사를 수행한다. 큰 분석 graph 본문은 이 metadata 검사에 로드하지 않는다. 확장 근거가
없거나 identity가 다르면 통과하지 않는다. Restart learning 검사는 현재 compact Recall
항목의 candidate identity, completed state와 `canonical_decision = false`를 확인한다.

### 통과 조건

아래 세 target의 required technical steps를 모두 만족해야 Phase 8 technical
entry가 가능하다. `acceptance-scenarios.md`의 naturalistic/qualitative 최종 조건은
Phase 8가 별도로 만족해야 하며 technical V11만으로 cutover gate를 열지 않는다.

`ordinary_work`는 ready-for-work materiality/learning result 뒤 disposable
`v11-ordinary-work.txt`를 일반 filesystem write로 만들고 `guarded.sqlite3` hash가
전후 동일한지 확인한다. 이는 ordinary write가 Guarded ceremony를 요구하지 않는
technical invariant다. 실제 source/test/config behavior 변경·검증, engineering
판단의 품질이나 의미 있는 ordinary work의 관측을 대신하지 않는다.

특히 Candidate collection/inspection/promotion/retention journey와 Guarded effect의 exact
action/target/effect/scope/revision/expiration match, user-response Source, single-use/reuse
rejection, no-dispatch-before-valid-confirmation, ordinary-action non-blocking 및
indeterminate no-silent-retry behavior를 같은 integrated run에서 검증한다.

The current internal V11 result uses schema 2, technical contract
`v11-work-continuity-1`, and workload identity
`three-target-installed-journey-volicord-three-work-1`. Historical schema 1
results remain historical; they do not acquire lifecycle evidence through
conversion. The V11-local required-steps-for-target contract owns 18 common
categories for each target and `multi_work_continuity` only for Volicord.
Thus current successful coverage derives as 19 + 18 + 18. Unknown, duplicate,
missing, or unexpected target/leaf combinations fail. Counts must equal actual
leaf statuses; aggregate `passed` and `phase_8_ready = true` require every
target-specific leaf to pass, maintained performance evidence, a completed
no-trigger Decision assessment, and authenticated evidence for each exact
Project. The Volicord leaf records bounded expected/observed restart, Work,
authority, canonical retention, CLI/MCP, and portable relations. The maintained
result validator rechecks these against raw public-operation observations;
capsule/report/archive verification rechecks the sanitized relation proof,
technical identity, and derived target coverage. A passed status or count alone
does not qualify. The gate consumes the maintained V11 validator rather than a
separate top-level success predicate. Final success likewise requires each ordered
exact argv once, numeric exit code 0, succeeded outcome, and no spawn error or
termination; command/failure counts and aggregate outcome must agree with leaves.

The bounded authenticated probe requires one completed successful `volicord` /
`project_health` call with exact Project arguments in structured Codex CLI events,
a completed turn, and a connected, healthy result with canonical/repository
availability. Started-only, failed, text-only, conflicting result representations,
and additional tool, shell, file-change, or search activity cannot qualify.
The maintained positive fixture follows captured `codex exec --json` events;
mutation controls exercise completion, identity, health, and action restrictions.

V11's local MCP transport bounds request writes and complete newline-framed response
reads with a monotonic per-RPC deadline (the maintained 90-second call ceiling).
Matching integer request ID, JSON-RPC 2.0 and exclusive valid result/error framing
are required; explicit RPC errors cannot be tool success. The measured call duration
remains independent budget evidence. Stderr is captured to a file to avoid pipe
backpressure. Linux command/MCP sessions use bounded process-group TERM/KILL cleanup,
including descendants after leader exit. Recorder evidence separates timeout or
interruption cause, final numeric exit/signal and cleanup completion; cleanup failure
cannot become successful execution. Parent SIGINT/SIGTERM unwinds this local boundary.
Short fake-server/process regressions run within the maintained harness self-check.

The raw V11 credential audit compares whole auth material and individual known
credential-bearing values, including their JSON string spellings. Each authenticated
probe captures the actual staged auth before execution and refreshed auth before
cleanup in memory, audits retained artifacts, and records only safe counts/categories.
Streaming artifact reads catch values across chunk boundaries; retained symlinks are
scan failures and are never followed outside the artifact boundary. Invalid/unreadable
auth or retained artifacts cannot certify a clean audit. Temporary auth deletion and
sanitized archive verification remain distinct checks; no credential value or reusable
fingerprint is diagnostic evidence. Synthetic controls cover access/refresh/ID/API
credentials, compact/reformatted JSON, clean artifacts and refreshed staging values.

### V11 resource regression qualification

Official V11은 `performance-budgets.json`의 maintained Linux regression ceiling을 함께
검사한다. 이 예산은 workload identity
`three-target-installed-journey-volicord-three-work-1`에 묶인다: MCP process high-water
RSS 4 GiB, manifest 파일 하나 2 GiB, analysis storage logical 2 GiB/physical 2.125 GiB,
Project별 첫 완성 snapshot 관측 이후 누적 증가 512 MiB, inventory/entity/relation item당
logical storage 2,048 bytes, V11 journey 15분, 개별 MCP RPC 90초. Absolute ceiling은
안정화되었지만 과대한 footprint를, normalized ceiling은
repository scale 대비 과대한 표현을, post-warmup ceiling은 full-copy 누적 회귀를 서로
독립적으로 거부한다. 이는 현재 three-target journey의 회귀 상한이며 일반 제품의
모든 repository에 대한 latency SLA는 아니다. 상한 초과 또는 측정 누락/오류는 functional
target별 필수 leaf가 모두 통과해도 aggregate readiness를 막는다. 변경 시 실제 원인과 근거를 검토하며
실패 실행을 통과시키기 위해 관측값에 맞춰 상한을 올리지 않는다. 이전 54-leaf 여정도
15분/512 MiB 예산을 사용했으나, 현재 55-leaf 여정은 별도의 workload identity와
독립적인 qualification contract를 가진다. 현재 여정은 Volicord의 실제 B/C Work
연속성 검사와 B/C의 완성 Analysis Snapshot 8개를 포함한다. 첫 clean-HEAD 실행은
모든 leaf를 통과하면서 1,026,151.145 ms 및 post-warmup 증가 763,064,961 bytes를
측정해 현재 상한을 초과했다. Analysis storage 최적화 후 같은 55-leaf workload의
clean-HEAD focused rehearsal은 836,349.979 ms 및 525,260,967 bytes를 측정했다.
이는 최종 gate qualification을 대체하지 않는다. 다른 여섯 상한은 그대로다. Workload identity가 예산과
일치하지 않으면 V11 실행과 독립 검증은 실패한다.

각 RPC는 monotonic duration과 Linux `/proc` VmHWM을 50ms 간격으로 관측하고 호출
종료 시 한 번 더 읽는다. VmHWM은 그 프로세스의 누적 high-water 값이므로 call-local
allocation delta로 해석하지 않는다. Snapshot publication 이후 manifest 수, graph scale,
전체 content-addressed tree의 logical bytes와 Linux allocated blocks를 검사한다. Project의
첫 완성 snapshot 관측을 warmup baseline으로 삼아 이후 최대 누적 delta를 별도로 기록한다.
빈 Project 디렉터리, 잘못되거나 incomplete manifest, referenced blob이 없는 publication은
baseline을 만들지 않는다. Current manifest의 identity/Project/metadata/count와 referenced
blob 존재를 확인하며 graph body의 content integrity는 production reader가 소유한다.
Normalized storage는 각 관측에서 Project의 전체 logical bytes를 그 Project의 완성
manifest들이 나타내는 inventory/entity/relation item 합계로 나눈 뒤 최대값을 보존한다.
다른 시점 또는 Project의 bytes/items 최대값을 서로 나누지 않으며 fractional ratio를
내림하여 ceiling 위반을 숨기지 않는다. Corrupt/incomplete bytes도 absolute storage와 이미
설정된 baseline 이후 growth에는 포함하며 완성 snapshot/item evidence를 대신하지 않는다.
Raw per-call 수치는 ignored evidence에, bounded aggregate/ceiling/verdict는 gate capsule과
sanitized archive에 보존한다. RPC argument/response나 Source body는 성능 기록에 넣지
않는다. Self-check는 초과값, NaN, 측정 누락/오류의 거부와 실제 local process 관측을
검사한다. Storage fixture의 pre-fix 두 snapshot 1,656,908 bytes/unchanged delta 828,454
bytes와 post-fix 691,245 bytes/delta 332,517 bytes, 그리고 prior Volicord campaign의 최대
25,179,035,785-byte cycle을 threshold basis로 보존한다. Functional coverage, provenance,
freshness를 줄여서 이 상한을 만족시키지 않는다.

### Phase 8 naturalistic Dogfood qualification

Phase 8 is a fresh real-session campaign for one exact clean Product candidate.
The verified technical gate capsule and archive remain separate prerequisites for
replacement qualification. The campaign has three actual pinned repository journeys
(`volicord`, `small-python`, `polyglot-medium`), five ordinary Works
(Volicord A/B/C and A in each other repository), three same-Work fresh-session
resume pairs, and eight globally distinct fresh sessions. Volicord A → Resume A
→ B → C uses one workspace, Runtime Home, and Project with three distinct Work
identities and retained history. The other journeys have independent workspaces,
Runtime Homes, and Projects. No prior campaign's sessions or evidence are reused.

`dogfood-campaign prepare --campaign-root <new-private-root> --campaign-id <id>
--candidate-head <clean-HEAD> --repositories <repository-input.json> --tasks
<task-manifest.json>` reads five Work mappings before mutation. The task manifest
has exactly one `tasks` object keyed by maintained Work slot ID. Each Work maps
`start` to a UTF-8 file path; each Work A also maps `resume` to a UTF-8 file
path. The helper freezes the exact bytes in eight create-only operator task
artifacts, binds their lengths and SHA-256 hashes, pins repository identities and
revisions, creates journey workspaces and Runtime Homes, records session slots,
and writes the eight-entry run sheet. Task bytes must uniquely identify each
role within one journey. The five Work states are `frozen` before activation.
Preparation assigns no expected semantic behavior, evaluator-private profile,
materiality quota, Learning obligation, provisional review, or semantic seal.

`activate-all` verifies complete frozen preparation, exact candidate artifacts,
inventory, repository identity and task binding before enabling candidate
integration in all three repositories. Repository and hook trust remain explicit
user actions. The helper never starts Codex chats or answers Product Questions.
The operator sends each exact task from the generated raw `.txt` artifact in
its own fresh session and preserves eight raw rollouts. All task text is fixed
before the first session.

`collect-batch` maps all eight rollouts before publication. It checks host
provenance, exact frozen first-turn transport identity, workspace and descendant
revision, distinct session IDs, actual SessionStart activation, same-Work
start/resume Project and Work identities (a resume may use its structured
canonical Recall checkpoint Work ID without writing a new Checkpoint; any new
Checkpoint must agree), Volicord one-Project/three-Work
continuity, cross-journey isolation, candidate artifacts, raw hashes and
destination collisions. A descendant commit alone does not prove a Work boundary.
Every meaningful observed repository-relative path must occur in the net committed
tree delta from that session's HEAD to its completed boundary HEAD (`git diff
--name-only -z --no-renames --no-ext-diff --no-textconv <base> <boundary> --`);
additions/deletions and both observed rename leaves are exact paths, never pathspecs.
The completed capture must also retain successful structured execution of exactly
`git --no-optional-locks -c core.fsmonitor=false status --porcelain=v1 --untracked-files=all && git rev-parse HEAD`
at the repository root. Its only output is that same boundary HEAD and a newline,
proving empty tracked/nonignored-untracked status. The check must start after the
last meaningful mutation and every other command's completion; a later or overlapping
command invalidates it. This prevents partial staging or dirty same-file carryover
from passing merely because that file occurs in a commit.
Known `git add`/`git commit` operator housekeeping is classified as repository
maintenance, never as Product verification; the terminal check is inspection.
Neither supplies a requested test's successful numeric execution.

An incomplete start (`paused`/`in_progress` structured Checkpoint state) may carry
dirty changes into its paired same-Work resume: its path proof defers to the Work's
distinct-Work/final boundary and its cleanliness proof to the terminal resume.
A completed start proves both at the paired resume's HEAD; a checkpoint-free,
no-write `verified_state_continuation` adds no commit or new Checkpoint requirement.
Other completed mutations must be committed and checked before a distinct Work
starts, or before collection for the terminal Work. Each journey must be clean at
collection. A genuinely no-change session needs no empty commit.

The run sheet asks for an atomic Work-only commit, excluding unrelated pre-existing
changes, and this terminal check in the same naturalistic chat; frozen first-turn
bytes remain unchanged. Retained lineage records session/base and boundary revisions,
observed and proven paths, boundary kind, same-Work dirty-continuation permission,
and the cleanliness command's raw-rollout SHA-256, session/execution identity,
sequence/completion, output hash and numeric success. Both proofs are necessary.
Path observations establish path coverage, not semantic hunk ownership. They cannot
distinguish intentionally fully reverted paths from discarded work; any observed
path absent from the net tree delta conservatively fails, including a committed
change later reverted by its boundary. Shell-only changes without meaningful
FileChange/path evidence do not gain invented path ownership. Ignored content and
out-of-session mutations after the recorded boundary check are outside raw-session
cleanliness proof; live final attestation independently checks collection state.
Collection observes and retains the
final Git state without changing it; ignored content remains outside attestation. Journey-final canonical
bundle, documents, Viewer snapshot, Runtime and activation summaries, and
repository-state evidence are inventory-bound. Publication is immutable and
atomic. A failed collection cannot be converted to qualified evidence by
qualitative prose or retrying inside the same campaign.

`evaluate` reads the immutable evidence set and appends a machine run. The
machine policy makes deterministic candidate, inventory, raw/session/task,
repository, Project/Work, resume, privacy, credential, canonical provenance,
projection identity and attributable numeric execution violations hard.
Question, Decision applicability, Learning quality, invocation order, document
content, prompt naturalness, interruption and source-grounding probes remain
inspectable observations for post-hoc review. Parser uncertainty is
`indeterminate`, not a confirmed Product violation. Raw underlying
observations and numeric execution records remain available to reviewers.
Learning-specific machine findings use actual runtime participation:
inactive/absent participation is `not_observed`; active participation requires
post-hoc assessment of meaningful fork, alternatives and trade-offs, feedback
chronology, implementation fidelity and proportionate interruption. An
interesting task alone does not activate Learning.

Recorded agent and human qualitative reviews use one evidence-bound rubric.
Each criterion is `satisfied`, `violated`, `insufficient_evidence`,
`not_observed`, `not_applicable`, or `not_reviewed`. `not_observed`
means the optional opportunity did not occur; it is neither a pass nor a
violation. `insufficient_evidence` means an opportunity may matter but the
available evidence cannot establish a judgment. `not_applicable` requires a
permitted structural reason. Optional hidden-materiality and Learning
opportunities may be `not_observed` without invalidating the naturalistic
campaign. Direct human Viewer/browser and applicable Decision-comprehension
observations remain required and cannot be supplied by agent prose or static
markup. Review runs preserve identity, inspected evidence, reasoning,
uncertainty, counterevidence and machine relationships; later runs are
append-only. Substantive disagreement is review evidence and high-impact
authority/context-recovery disagreement may require targeted human resolution.

`qualify` combines exact-candidate technical verification, evidence integrity,
deterministic topology, hard machine findings, recorded qualitative states and
required direct-human observations. A hard technical or integrity failure is
non-overridable. Confirmed qualitative violations block; unresolved required
judgments remain incomplete. Review-support machine findings do not become
a separate semantic quota. The result retains `phase_9_ready = false`.
Only an explicit `approve-phase-9` operator action over a qualified run grants
Phase 9 readiness. Result-lineage publication preserves the exact evaluation,
review, qualification and optional approval bytes outside the immutable campaign.
A changed candidate requires a new campaign and independent qualification.

Naturalistic MCP memory remains `unsupported_current_architecture` until a
candidate-bound process/lifecycle observer exists; harness-tree RSS cannot be
relabeled as VS Code's MCP memory. The exact technical gate retains its separate
resource measurements. Synthetic campaign, resume, document, repository-state,
machine, review and qualification fixtures are regression support, never a
substitute for the eight naturalistic sessions.

### Deterministic behavior owners outside Naturalistic Dogfood

These tests exercise typed Product transitions or evidence chronology. They are
technical regressions, not assigned behavior opportunities in a naturalistic Work.

| Durable boundary | Existing executing owner | Naturalistic review limit |
| --- | --- | --- |
| User authority before affected commitment | `volicord-operations/tests/work_authority.rs`: `late_user_authority_correction_preserves_prospective_only_work_state`, `user_owned_and_hidden_material_signals_require_question_lifecycle`; `dogfood/frontier_self_test.py`: `test_late_revision_cannot_authorize_earlier_write` | Whether an unanticipated outcome is material or user-owned remains evidence-bound review. |
| Repository facts and settled outcomes do not create unnecessary Questions | `work_authority.rs`: `settled_contract_and_repository_fact_are_ready_without_question_and_survive_restart`, `settled_choice_does_not_manufacture_a_second_user_question`; `frontier_self_test.py`: `test_source_evidence_settles_initial_user_uncertainty_without_question` | The adequacy of actual research is judged after execution. |
| Delegated implementation choices retain their scope | `work_authority.rs`: `current_goal_explicit_delegation_is_ready_and_checkpoints_without_a_decision`, `discovery_evidence_precedes_delegated_or_agent_owned_implementation_authority`; `frontier_self_test.py`: `test_delegated_behavior_does_not_hide_independent_user_owned_policy` | Whether a real change exceeds delegation requires review of its effect. |
| Inquiry provenance and repeated-question avoidance | `frontier_self_test.py`: `test_discovery_binds_current_sources_exact_goal_baseline_and_chronology`, `test_answered_question_repeat_is_distinct_from_lifecycle`; `work_authority.rs`: `independent_user_owned_outcomes_cannot_share_question_authority` | Relevance and necessity of actual wording remain qualitative. |
| Learning participation and non-canonical choice | `work_authority.rs`: `provenance_representation_learning_selection_never_becomes_product_authority`, `active_learning_keeps_routine_and_user_owned_choices_on_their_existing_paths`, `current_user_can_withdraw_learning_without_creating_a_decision`; `resume_self_test.py`: `test_selected_state_chronology_and_non_decision_invariants` | Meaningful fork value, alternatives, trade-offs, fidelity and proportional interruption are reviewed only when participation is active. |
| Routine detail does not force an authority Question | `work_authority.rs`: `materially_atomic_private_details_terminate_without_question`; `frontier_self_test.py`: `test_non_user_authority_is_not_a_behavior_label` | Whether a specific interruption was useful remains qualitative. |
| Same-Work Recall and continuation | `dogfood/campaign_self_test.py`: `assert_current_campaign_contract`; `dogfood/resume_self_test.py`: `test_recall_transport_and_identity_are_evidence_failures` | Comprehension and usefulness of recovered context remain qualitative. |
| Decision applicability and scope | `work_authority.rs`: `relevant_evidence_cannot_claim_exact_authority_while_credible_alternatives_remain`, `settling_dispositions_require_explicit_exact_authority_sufficiency`; `frontier_self_test.py`: `test_settlement_rejects_stale_response_and_inapplicable_decision` | A reviewer assesses disputed meaning and high-impact disagreement. |

No row establishes a semantic pass for any of the five ordinary Works. The
post-hoc rubric records `not_observed` when an optional opportunity did not occur.

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

### Focused host transport regression

Maintained Host/Operations tests scale a validated learning lifecycle to 80 historical
Candidates with 16 rounds each and large polyglot capability coverage. They characterize
the previous full-inspection Recall shape above 1 MiB, assert current Goal/behavior
Context/Decision/Checkpoint/authority continuation, deterministic semantic omissions,
CLI agreement, and parse both MCP content representations under the 256 KiB result
budget. Escaped Unicode and oversized whole fields are tested independently. Raw
campaign completion-size measurements under ignored local state are diagnostics only;
malformed or truncated captures never qualify a replacement campaign.

Materiality's focused Host regression records 32 real independent choices, constructs
record and revise from the compact draft and existing tools/list schema, then binds
inspect from the returned skeleton without test-helper argument supplementation or
intentional malformed calls. It checks all current identities, closed variants,
full-wire byte size and both JSON representations against the maintained budget,
and compares size with the previous duplicated schema shape. Existing authority,
learning, delegation, interaction and artifact semantic rejection tests remain active.

### Dogfood prospective authority frontier

Work intake와 full qualification은 first affected write 직전 current Project/Goal/baseline의
latest completed Engineering Choice Discovery를 먼저 선택하고 그 Discovery의 latest created
Review와 write 전에 완료된 monotonic revisions만 평가한다. Older Discovery의 Review가 더 늦게
revised되어도 current authority를 되찾지 못하고, D2 without R2는 blocked다. Successful serialized
creation calls는 creation order의 observable evidence이며 inspect/revise는 creation이 아니다.
Executable scope는 각 observed first-path write 전에 current review와 exact closure를 요구한다.

Behavior class는 최종 disposition equality가 아니다. Evidence가 exploration을 resolve하면
settled/agent-owned authority로 끝날 수 있고 settled product outcome과 delegated internal detail은
공존한다. Hidden outcome도 pre-write reassessment에서 발견하고 current authority로 해결할 수 있다.
Already researched hidden Candidate는 ready-to-ask로 제출할 수 있으며 meaningful pre-Discovery
investigation과 independent outcome별 prospective resolution은 계속 필요하다. Learning-only
authority는 canonical Decision으로 승격할 수 없다. Temporal commitment는 production처럼 current
temporal/lifetime outcome/result와 primary binding의 일치를 검사한다. Prose에서 숨은 outcome을
추론하지 않으며 completeness와 cited authority의 semantic sufficiency는 independent evaluator와
human authority-obligation review가 계속 소유한다. Sanitized frontier regressions는
`rebuild/validation/dogfood/frontier_self_test.py`이며 maintained harness self-test에 포함된다.

### Resume evidence and verification chronology

Maintained resume reasons are `recall_transport_incomplete`, `recall_identity_or_project_invalid`,
`recall_operation_failed`, `pre_recall_repository_access_or_order_violation`, `baseline_invalid`,
`scope_or_authority_missing`, `post_change_validation_missing`, `terminal_validation_failed`,
`terminal_validation_indeterminate`, and `validator_invariant_failure`. Intake, failed_checks and
transactional batch summaries preserve the exact finite basis and remediation domain. Malformed or
truncated Recall is evidence failure; an observed failed product Recall is product_integration.
No text-message classification or blanket behavior attribution is used.

Verification is evaluated after the last observed material mutation, including repeat writes to the
same path. Later demonstrably read-only inspection/report commands do not replace numeric verification;
static shell wrappers and Git -C inspection are supported, while write flags/compound mutations cannot
be ignored as inspection. Echoed success claims never qualify. A later material mutation requires new
verification. The selected command retains execution identity, aggregate/decomposed group index, numeric
exit and termination; actual nonzero verification fails and incomplete execution remains indeterminate.
Session 1's 256 KiB MCP Recall budget remains unchanged. Sanitized bounded Recall and chronology
regressions live in `resume_self_test.py` and run in the maintained campaign self-test.

Hidden pre-Discovery investigation has explicit `complete`, `indeterminate` and `missing`
evidence states. Successful numeric execution of a supported inspection or successful structured
`repository_understanding` is complete. When numeric execution evidence is unavailable, a closed
source-read command may prove observation only: explicit in-repository paths, one `cat`, bounded
`sed -n`, `nl -ba` or `rg -n` reader, optionally followed by bounded head/sed, with at least two
non-empty output lines totalling 64 non-whitespace-edge characters. Ripgrep output must carry path
and numeric line references within the requested scopes. External/runtime paths, arbitrary producers,
compound shell output, malformed completion and uncorrelated prose cannot qualify this path. Observed
inspection without usable timely completion or bounded content is indeterminate; no qualifying
inspection is missing. Numeric exit and termination fields are never synthesized.

Hidden investigation belongs to the originating material choice. A later serialized re-materialization
may retain it only when both exact Project/Goal-correlated `repository_analyze` calls return equal,
valid `repository_observation_basis` values under the Repository Intelligence owner contract. Fresh
repository Source, Repository Snapshot and Analysis Snapshot identities neither establish nor refute
observation equivalence. Each Discovery must be grounded in its own analysis's repository Source.
Only that proven Source reference in typed `source_ids`/`source_basis` links is normalized before exact
comparison of the choice graph, stable choice/alternative identities, dimensions, consequences,
relationships, material boundary and interaction meaning. Unrelated Source identities remain exact;
prose similarity never establishes equivalence. Evidence readiness may advance independently.
No intervening material mutation (including the originating observation/investigation interval),
malformed evidence or ambiguous discovery/baseline correlation is permitted. A missing, malformed or
different basis, new choice/dimension/alternative/consequence or unrelated Source substitution requires
timely investigation on the new basis. Absence of recorded writes alone cannot prove equivalence.
The earliest equivalent discovery still fails if it preceded investigation, even when later discovery
was preceded by redundant research. Maintained frontier fixtures use fresh Sources and Source-bound
snapshots with equal production-format observation bases, and cover these negative controls.
Historical rollout evidence without this field remains fail-closed and must never be supplemented
with a synthesized basis. Production integration tests and sanitized fresh-Source regressions own
this proof; old campaign replay is diagnostic only. This evidence rule does not change canonical
Inquiry or Decision applicability.

Terminal verification uses explicit inspection/report/validation/unknown roles. Supported validation
entry points and bounded shell/env/focused wrappers are selected; arbitrary commands are not validation.
Later `nl -ba ... | sed -n 'range;range'` listings (including newline-separated listings) remain
inspection. Unsafe/ambiguous compounds cannot certify success or recover a prior failure. A later
unknown command blocks a success claim without replacing the last genuine validation outcome.
Read-only reporting includes the closed `find PATH -maxdepth N -type f -print -exec sed -n
'RANGEp' {} \;` form. A bounded Python assertion heredoc is validation only when its complete shell
transport and parsed assertion/loop structure match the supported subset; prints, constant assertions,
empty loops, swallowed assertions, dynamic execution and shell tails do not qualify. This classification
never infers the numeric outcome from script text or stdout.
Only a later genuine numeric successful validation can recover an earlier failure. Unknown numeric
outcome remains indeterminate even when stdout or an assistant claims that tests passed.

### Current-host Decision transport provenance

Decision caller text is compared directionally to the actual captured response with the existing
unique-alignment Markdown escape and CRLF/terminal-line-ending primitive. In this Decision-only
comparison, additional terminal ASCII space/tab/CR/LF in the host response may be removed; no leading,
interior, Unicode-whitespace, case, punctuation, paraphrase or fuzzy normalization is permitted.
Frozen task and Context transport contracts are unchanged. Canonical response Source content must
remain the exact caller-supplied text, with the existing Source identity, Project, host/session,
Question revision and presentation linkage. The canonical session belongs to HostAdapter and need
not equal the raw Codex thread ID. Cross-layer correlation uses the observed call/result, exact
Source/response/Decision/witness, maintained presentation receipt and captured current user turn.
Context and user-withdrawal checks likewise retain their exact text/linkage contracts without
comparing the two session domains; learning's canonical Decision audit uses internal sessions from
Sources returned by observed calls. No unavailable authenticated host turn-ID API is invented.

Bounded comparison evidence preserves the raw rollout SHA-256, captured turn identity, raw response
SHA-256, caller and canonical Source SHA-256, canonical Source ID, normalized comparison hash and
reported escape/suffix normalization. Original source bytes are never rewritten. The sanitized
`decision_transport_self_test.py` exercises parser-to-canonical provenance, terminal whitespace and
semantic-difference rejection and is included in the maintained harness self-test.

Current-frontier evaluation also preserves legitimate historical Question lifecycles: each obsolete
branch must prove its own dimension-linked pre-write resolution, while the current settled Review
must independently pass authority/closure validation. Historical Decisions do not turn agent-owned
learning into canonical product authority. Scenario labels do not suppress an independent user-owned
outcome outside an internal delegation. Serialized creation order is required when no candidate
creation timestamp/identity ordering is exposed; overlapping creations remain indeterminate instead
of guessing which completion represents production's latest candidate.

A recorded user-owned concern may be settled by subsequent exact source evidence before commitment;
only dimensions that still require user authority demand a canonical Decision. The initial disposition
is historical, while reported current dimensions and disposition come from the last pre-write revision.
Meaningful text manifests such as requirements.txt and CMakeLists.txt are repository mutations; only
known synthetic markers/generated state are excluded, rather than an entire text-file suffix family.
A later Goal invalidates the earlier authority frontier even when the older Goal's Source linkage is
still valid historical evidence.


### Common evidence-bound qualitative review

`qualitative-review.md`와 `evaluation.json.qualitative_review_contract`가 공통 rubric,
reviewer metadata와 reviewer-safe operation 계약을 정의한다. Agent와 human은 같은 criterion을
사용하지만 immutable `reviewer.kind`를 공유하거나 prose로 추론하지 않는다. Criterion assessment는
maintained `inspect-agent-review`가 먼저 제시하는 exact evidence identity, hash, path와 locator를
reviewer가 실제로 검사한 뒤 작성한다. 각 assessment는 run-wide union과 별도로
`inspected_evidence`를 보존한다. 이 operation은 criterion/sample/repository class 또는 machine
status에서 verdict를 생성하거나 추천하지 않으며, structural preflight는 semantic judgment의
진실을 검증했다고 주장하지 않는다. Reviewer는 explicit relationship으로 machine finding과
불일치할 수 있지만 immutable machine disposition을 변경하지 않는다.
`satisfied`, `violated`, `insufficient_evidence`, `not_applicable`, `not_reviewed`를 구분한다.
Missing observation은 inapplicability가 아니며 incomplete review는 만족으로 집계하지 않는다.
Review artifact의 구조·hash·locator validation은 semantic judgment의 proof가 아니다.
각 citation은 exact criterion과 그 locator의 relevance를 보존한다. Viewer topology,
concrete code behavior와 actual diagram usefulness는 서로의 group-level 판정을 상속하지
않는다. Document usefulness는 primary semantic content와 placeholder/audit-only 여부를,
fidelity는 user choice, recommended alternative, 각 rationale와 alternative-specific
consequence attribution을 각각 검사한다. 이 구조는 phrase별 verdict를 계산하지 않으며
근거가 부족하면 `insufficient_evidence`를 유지한다.
Technical result의 `qualitative_review = not_recorded`는 review publication과 독립이며 기존
qualification은 `qualification_policy.py`가 결정한다. Volicord one-Project/three-Work continuity는
main-campaign journey의 immutable structural finding이 소유한다. Live accessibility, browser input/paint
responsiveness, 그 Viewer organization의 human comprehension과 실제 사용자 Decision comprehension은
human observation을 요구하고 나머지 semantic criteria는 evidence-backed agent
review로 해결할 수 있다. Conflict 또는 high-impact authority/context-recovery insufficiency는
해당 criterion만 human에게 escalate한다. Human은 `resolves_review_runs`로 충돌한 review ID를
명시하며 무관한 criterion을 재검토할 필요가 없다. 어떤 hard violation도 override하지 못한다.
Human conversational capture는 multi-line observation과 limit을 한 response로 보존하고,
`skip`, `already_covered`, `same_as_prior`, `same_as_other_locale`, `cannot_assess`,
`not_applicable`을 prose observation이 아닌 typed `human_controls`로 기록한다. Locale/criterion
reference는 compatible prior assessment를 가리키며 literal answer는 provenance trace에만 남는다.
`insufficient_evidence`는 fabricated locator를 요구하지 않고 inspected set과 missing-evidence
설명을 보존한다. Partial review의 `not_reviewed`/`insufficient_evidence`는 semantic failure나
qualification success로 재분류하지 않는다.
Work-specific criterion은 모든 5개 Work에 남고, journey-final projection criterion은 represented
Work identity를 보존한 세 projection에 적용된다. CLI group만 `volicord`, `small-python`,
`polyglot-medium` repository class별 일곱 criterion으로 생성된다. Dedicated observation이 없는
class는 일곱 bounded unresolved gap을 남기며 `not_applicable`로 숨기지 않는다.

### Append-only evaluation identity

`dogfood-campaign evaluate --campaign-root <input> --output <new-run>` reads an
immutable evidence set without changing Campaign metadata, inventory or artifacts.
The output must be outside the Campaign and is published once with a receipt.
Omitting output selects a new sibling run directory. The same operation performs
re-evaluation; `--previous-evaluation <file>` retains the original run ID and byte
hash for comparison without interpreting historical policy as current authority.
Missing historical evidence is never synthesized or upgraded.

Machine schema 3 separates Product `candidate_head`, evidence-set SHA-256,
`evaluator_revision` and implementation file hashes, policy revision/hash, random
run nonce/content-derived run ID, and consumed qualitative review IDs (empty for
machine-only evaluation). A later evaluator HEAD may inspect an older candidate;
only fresh evidence can establish the behavior of a different Product candidate.
It retains five Work observations and three repository-journey observations, with
the exact `3 journey / 5 Work / 3 resume-pair / 8 session` coverage projection.
Review packaging consumes the external immutable evaluation receipt rather than
requiring registration by mutating the Campaign. One evaluator serves collection
and re-evaluation; historical run bytes remain addressable, never rewritten.

### Maintained replacement qualification and operator authorization

`qualification_policy.py.contract()` owns the finite criterion authority and escalation
policy. `machine_findings` owns machine certainty/disposition and semantic jurisdiction.
`dogfood-campaign qualify` verifies immutable evaluation/review identities and publishes
`qualification.json` with exact input references. `blocked`, `unresolved`, `qualified`
are distinct. Missing evidence never becomes inapplicability or satisfaction. A schema-valid
review is a recorded judgment, not verified semantic truth.

`approve-phase-9` is a separate explicit operator action with the exact
`approve-phase-9` authorization assertion. It rechecks the qualification's complete
input set and current policy and refuses missing technical evidence, hard violations,
unfinished reviews or unresolved escalations. Its immutable approval embeds the complete
qualification state and binds the original qualification bytes/hash. Operator identity
is declared, not authenticated; the cooperative filesystem does not provide signatures.
The engineering agent must not exercise this operator action without explicit authorization.

Completed result discovery uses `publish-result-lineage`. Its default create-only location is
`<campaign-parent>/results/<qualification-run-id>`; an explicit durable `--output` may be used
instead. `index.json` and its receipt bind the exact Product candidate, `evidence-set.json` bytes,
evaluation run/evaluator revision/policy, every recorded qualitative review, qualification run and
optional approval. The package copies the immutable artifacts with relative paths and excludes the
qualification input file's absolute staging paths. `verify-result-lineage` performs independent
hash/identity/policy/receipt verification using only copied contents. Therefore `/tmp` is allowed
for transient preparation but is never the sole authoritative discovery path. A later evaluation,
review, qualification or approval publishes a new lineage; it cannot mutate Campaign evidence or
claim that a later run belonged to the original candidate execution.

The naturalistic Dogfood definition declares only
`technical_evidence_dependency = verified_current_contract_gate_capsule_archive_for_exact_candidate`.
It does not enumerate V11 leaf names or counts. A V11-only technical leaf may evolve without
changing Dogfood task or topology definitions; the gate and its current report checker own
technical coverage. Review preparation and qualification do not execute Final, provider, or
V11. Missing, failed, wrong-candidate, stale-contract, or tampered evidence cannot qualify.

The maintained candidate technical gate remains authoritative and unchanged. Qualification
requires a capsule matching the independently verified archive's completion transition for
the exact Product candidate, followed by exactly one successful `archive_publication`
continuity record when completion restores readiness. The prior continuity prefix and all
other fields must remain unchanged; the publication observation must bind the same HEAD
and a clean worktree through the gate-owned candidate check. Failed or blocked completion
cannot be promoted by publication. Before fresh Dogfood preparation, the real
`qualification_policy.verify_technical()` must consume that candidate's newly produced
gate capsule/archive successfully, with its output and numeric exit preserved.
Policy changes never trigger expensive technical execution.
The engineering final HEAD requires its own gate-owned authoritative admission and maintained gate; standalone diagnostic admission is optional.

### Machine authority audit

The sole finite disposition contract is
`rebuild/validation/dogfood/machine-policy.json`. It names every maintained integrity,
behavior, execution and procedural rule, its owner, rationale, uncertainty authority
and permitted semantic review groups. `machine_findings.py` validates complete coverage.

- Exact `project_resolve`/Recall counts and total operation counts are retained as
  advisory efficiency observations. Composite recovery/ordering predicates require
  qualitative review; observed conflicting Project/session identities remain hard.
- Lexical task/coaching tests, investigation sufficiency, discovery order, no-change
  deliberation, terminal Checkpoint selection, path-count and operation-count proxies
  cannot terminally reject naturalistic work. Their original check values remain intact.
- Question necessity, authority applicability, learning proportionality and source-grounded
  document/interpretation usefulness use the common rubric. Structural HTML checks are
  observations and never direct human-observed accessibility or usability evidence.
- Canonical Decision response/Source/Question/receipt/witness integrity, measured Project
  and session provenance, raw mutation, credentials/privacy and candidate binding remain
  hard. An evidence-attributable numeric failed required validation after the last material
  mutation also remains hard. Evaluation preserves each verification execution instead of
  selecting the last validation-looking command: task-required/diagnostic role, focused or
  broader aggregate scope, numeric outcome, baseline/setup-environment/candidate attribution
  and evidenced supersession/recovery are independent fields. An equivalent pre-mutation
  success followed by failure is candidate-attributable; an identical failed pre-mutation
  execution/output can establish known baseline failure. A later numeric success recovers a
  failed validation only when its same-validator semantic profile covers the failed scope in
  the same working directory. Exact reruns retain an equivalence relationship; changed
  environment and inspectable broader-scope reruns retain distinct invocations and a covering
  relationship. A narrower rerun, scratch/prototype success, unknown command or different
  validator does not erase a failure. Exit 126/127, missing module and permission/setup
  failures remain preserved setup-environment executions: correlated covering success marks
  recovery, while no such success leaves incomplete review-required evidence rather than a
  candidate-attributable hard failure.
- Raw execution and canonical Checkpoint verification reconcile only through the persisted
  invocation fingerprint plus numeric exit/termination and ordered Source identity. Labels,
  outcome prose and textual similarity are not correlation keys. Canonical outcome comparison
  reconstructs only the Product's typed behavior-preserving compatibility-note enrichment;
  matching declared/canonical passed evidence therefore reconciles without discarding the
  enriched value. Any other exact-identity outcome disagreement remains an explicit conflict.
  Missing/ambiguous command classification or attribution is reviewable, never success; an
  echoed success cannot establish a numeric outcome. A later broader diagnostic/aggregate
  failure does not replace a successful required functional verification unless inspectable
  baseline or correlated same-validator evidence attributes that failure to the current
  candidate.
- Hard facts are extracted independently of broad procedural checks, so a redundant
  Recall cannot erase an identity conflict, and a positive review cannot waive a failed
  required validation. Review-required findings need explicit evidence-backed relationships
  in their permitted semantic group; they never automatically pass.

Historical campaigns without an immutable collection receipt may use `dogfood-campaign
diagnose --campaign-root <historical> --output <new-run>`. Superseded schema 1–3 campaigns receive
identity-and-inventory-only inspection; they are not upgraded or routed through the current Work
evaluation engine. The helper hashes historical inventory, metadata and raw rollouts before/after.
It does not add missing evidence links, create a collection receipt, rewrite rejection results or
qualify any candidate. An intact current collected campaign instead
uses the ordinary `evaluate` append-only operation. Sanitized fresh fixtures prove the full
re-evaluation contract; historical incomplete diagnostics prove only what was observable.

Current Naturalistic preparation freezes task bytes and deterministic identities only.
No pre-execution semantic reviewer dimension, profile reveal, reconciliation,
or Work seal is an admission condition. Post-hoc additional material outcomes
are recorded in qualitative review against actual Work evidence. Semantic
disagreement remains review evidence; it cannot alter immutable task,
session, Project, candidate, or raw hash integrity.
