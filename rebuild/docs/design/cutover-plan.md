# 기존 구현 교체 계획

- 상태: 제품 결정 반영 계획
- 목표: 새 제품이 실제 사용 gate를 통과한 뒤 기존 구현, 문서와 workflow를 제거하고 하나의 `volicord` 제품으로 정리
- 첫 공식 환경: Linux와 Codex
- 데이터 경계: 기존 구현과 데이터는 별도 서비스이며 migration·감지·export·compatibility를 제공하지 않음
- 명명: 최종 산출물에 제품 세대 분기나 영구적인 임시 재구축 명칭을 남기지 않음

## 1. 기본 원칙

- 기존 구현은 교체 전까지 inspectable reference baseline으로만 보존한다.
- 새 구현은 `rebuild/`의 독립 workspace와 별도 Runtime Home·schema에서 진행한다.
- 새 구현을 legacy API wrapper로 만들지 않는다.
- 교체는 파일 이동만이 아니라 product contract, installer, tests, docs와 runtime 경계를 모두 바꾸는 작업이다.
- “구현 완료”가 아니라 acceptance, validation과 dogfood gate 통과를 제거 조건으로 사용한다.
- 교체 전 archive tag 또는 동등한 source 복원 지점을 유지할 수 있으나 이는 제품 rollback 또는 compatibility path가 아니다.
- legacy Runtime Home은 새 제품의 입력이 아니며 clean initialization만 지원한다.

## 2. 단계

### Phase 0 — Reconstruction boundary

상태: 완료 기준선 마련

산출물:

- branch-wide `AGENTS.md`
- root Cargo workspace의 `rebuild` 제외
- 독립 `rebuild/Cargo.toml`
- 제품 헌장, 결정 등록부, acceptance, 자산 분류와 cutover 계획
- 분리된 ignored runtime/build path

통과 조건:

- 새 workspace package가 모두 `rebuild/` 아래 있음
- legacy crate dependency가 없음
- 기존 Runtime Home을 읽거나 수정하지 않음
- legacy implementation은 reference-only로 명시됨

### Phase 1 — Product decision baseline

상태: 이 파일 세트로 완료

산출물:

- Q1–Q13 accepted decision
- 다중 언어 Repository Intelligence capability contract
- Linux/Codex 첫 공식 환경
- Candidate, Checkpoint, Recall와 Decision reuse 정책
- privacy, UI, document와 Guarded policy
- legacy 비호환·clean initialization 결정
- 기술 검증 계획

통과 조건:

- `open-decisions.md`에 미해결 필수 제품 질문이 없음
- 구현 언어, 사용자 자연어와 분석 대상 언어가 구분됨
- Java, Python, JavaScript, TypeScript, C, C++와 Rust structural gate가 명시됨
- legacy migration, detection, backup와 historical export가 active 계획에서 제거됨
- 검증 필요 항목이 `validation-plan.md`에 분리됨

### Phase 2 — Risk validation

필수 검증:

1. polyglot structural analysis
2. 최소 세 ecosystem semantic adapter
3. Canonical Context와 portable bundle
4. divergent bundle merge
5. Question dependency frontier와 session resume
6. source-grounded document generation
7. privacy와 local-only mode
8. Linux install과 Codex integration
9. Recall과 Checkpoint 정확성
10. process/filesystem primitive 재사용 평가

통과 조건:

- Wave 1의 V01, V03와 V05가 architecture 선택에 충분한 결과를 남김
- 각 validation report에 command, input, coverage, failure와 known limit가 있음
- disposable spike code가 production contract로 조용히 승격되지 않음
- accepted product scope를 구현 난이도만으로 축소하지 않음

### Phase 3 — Target architecture

필수 입력:

- `architecture-inputs.md`의 evidence constraint, unsupported conclusion과
  architecture-document ownership plan
- `rebuild/validation/wave-1-summary.md`와 semantic capability path의 V01, V03,
  V05 maintained reports

`architecture-inputs.md`는 target architecture를 대신하지 않으며, 계획된
specialized document는 실제 파일이 생성될 때 active owner가 된다.

산출물:

- Canonical Context Kernel dependency boundary
- Repository Intelligence capability model과 language adapter boundary
- Inquiry, Decision, Candidate, Checkpoint와 Recall sequence
- portable bundle, revision, supersession, deletion과 merge semantics
- privacy/provider boundary
- user surfaces와 document projection boundary
- failure, degraded와 recovery semantics
- schema/format version policy

필수 dependency rule:

```text
Canonical Context Kernel
  does not depend on
  Repository Intelligence / LLM / MCP / CLI / Viewer / Renderer

Repository Intelligence
  may refer to canonical IDs
  but does not own user judgment

Inquiry
  reads canonical and derived information
  but only linked user input becomes a user Decision

Projection and Documents
  read state
  and do not mutate canonical records as a side effect
```

통과 조건:

- architecture가 V01, V03와 V05 결과에 근거함
- 첫 structural 언어 전체를 수용하는 extension boundary가 있음
- local-only mode가 first-class임
- legacy Store, Core와 wire contract를 의존하지 않음

### Phase 4 — Canonical Context Kernel

순서:

1. Project와 Source
2. Question
3. Decision
4. Context Item
5. Checkpoint
6. revision, supersession, contradiction와 forget
7. portable bundle와 Project binding
8. deterministic Recall basis

통과 조건:

- LLM과 repository analyzer 없이 기록, restart, export/import, 수정과 삭제가 작동
- Derived State가 없어도 canonical records를 읽을 수 있음
- user, agent, source와 generated annotation provenance가 구분됨
- crash recovery와 schema version test가 통과함
- legacy Runtime Home 또는 schema를 읽지 않음

### Phase 5 — Repository Intelligence

순서:

1. repository snapshot, inventory와 language detection
2. polyglot structural adapter
3. Code Entity, Relation, capability와 coverage
4. fingerprint와 incremental update
5. source-grounded search
6. semantic adapters 최소 세 ecosystem
7. agent-assisted explanation
8. Decision/Context/Checkpoint linkage

통과 조건:

- 모든 텍스트 repository에서 inventory가 작동
- Java, Python, JavaScript, TypeScript, C, C++와 Rust structural acceptance 통과
- 최소 세 ecosystem semantic acceptance 통과
- unsupported/excluded/failed coverage 표시
- semantic provider 없이 local structural mode 사용 가능
- stale analysis가 current fact로 제시되지 않음
- polyglot repository에서 capability별 결과를 정직하게 연결

### Phase 6 — Inquiry, Checkpoint와 Recall

기능:

- material Question Candidate
- fact research before asking
- dependency frontier
- option, recommendation, trade-off와 uncertainty
- current host user response linkage
- delegation, research, prototype와 deferment
- pause/resume와 recommendation batch choice
- source-grounded Checkpoint
- first project-scoped bounded Recall
- Decision applicability와 revisit trigger

통과 조건:

- 긴 설계 세션을 새 대화에서 이어감
- 같은 판단을 다른 interface에서 다시 입력하지 않음
- 이미 답한 Question을 반복하지 않음
- 사용자 Decision과 agent recommendation이 분리됨
- ordinary repository write가 차단되지 않음
- unrelated dirty change가 Checkpoint에 귀속되지 않음
- Recall이 canonical state를 변경하지 않음

### Phase 7 — Viewer, documents와 host integration

기능:

- Project overview와 Resume Brief
- Repository Map과 Decision–Context–Code Map
- inspect/correct/supersede/forget
- Checkpoint timeline
- four required documents
- Korean/English bundled UI locale
- unrestricted requested-language generated content
- requested-language generated body realization 또는 explicit unavailable/degraded behavior
- task-oriented, discoverable, repository-relative CLI with human-readable defaults and no
  routine opaque Project ID handling
- Codex MCP integration
- Guarded effect confirmation
- bounded long-running process reporting

통과 조건:

- Viewer의 기본 Project Understanding에서 raw JSON, database 또는 opaque ID 없이
  completed/current/remaining work, next step, Decision rationale, affected code,
  component/architecture/flow, evidence, gap, freshness와 uncertainty를 이해 가능
- verified structural/semantic fact와 generated interpretation이 구분되고 diagram
  topology가 inspectable repository/Decision relation에서 옴
- generated document가 자동 canonical truth가 되지 않음
- source snapshot, capability coverage와 known gaps가 문서에 포함됨
- external semantic analysis는 Project opt-in 전 실행되지 않음
- ordinary edit는 confirmation을 요구하지 않음
- high-risk effect만 exact action-scoped confirmation을 사용

### Phase 8 — Dogfood와 replacement gate

반복할 전체 journey:

```text
clean Linux install
→ Codex connect
→ Project init and clone binding
→ repository inventory and capability analysis
→ source-grounded understanding
→ evidence-appropriate inquiry behavior, including no Question when correct
→ ordinary work
→ Checkpoint
→ process restart and new-session Recall
→ another-clone bundle import
→ divergent conflict handling
→ document output
→ memory correction and deletion
→ degraded failure recovery
```

최소 대상:

1. Volicord 자체 Rust workspace
2. 여러 source/test/config 파일의 meaningful behavior 변경·검증을 포함한 소규모
   단일 언어 application
3. 문서/config, component boundary와 cross-language flow가 있는 최소 세 언어의
   현실적인 중간 규모 polyglot repository
4. 각 first structural language fixture
5. first structural 목록 밖 언어 fallback fixture

통과 조건은 `acceptance-scenarios.md`의 최종 통과 조건과 일치한다.
Automated Dogfood passage alone is not replacement passage: the current
campaign-level human review must also pass, while an absent review leaves
replacement explicitly pending and a human pass cannot override machine
failure.
Dogfood passage는 unique expected Question/Decision/user choice를 가정하지 않고 maintained
behavior vocabulary로 independent classification을 수행한다. Exact campaign behavior profile과
repository distribution은 evaluator/steward-private state에 integrity-bound되고, 모든 six blind
provisional review가 고정된 뒤에만 reveal·validation·comparison에 사용한다. 세 repository
class마다 두 cycle, 모든 cycle의 work/resume을 포함하는 6-cycle/12-fresh-session campaign과
모든 qualifying cycle의 human review와 current production background semantic-provider의
별도로 authorized real success path가 필요하다. Final exact validation은 `rebuild/scripts/validate gate`의
단일 owner/run을 유지하고 clippy result는 warning-clean이어야 한다.

### Phase 9 — Cutover

하나의 집중된 교체 batch로 다음을 수행한다.

1. 기존 `crates/`, `tests/`, `xtask/`와 legacy workspace 제거
2. `rebuild/` 내용을 최종 root layout으로 이동
3. root Cargo manifest와 lockfile 교체
4. final binary와 package 이름을 `volicord`로 정리
5. Linux installer, Codex MCP setup, release packaging과 workflows 교체
6. README와 maintained documentation 전면 교체
7. 임시 `rebuild/` 경계와 work instructions 제거 또는 최종 정책으로 재작성
8. legacy terms, API, command와 workflow method가 active product에 남지 않았는지 검사
9. clean new Runtime Home에서 full acceptance를 final root 기준으로 다시 실행
10. official fixture와 dogfood report를 release gate에 연결

## 3. 제거 gate

다음 항목이 모두 통과하기 전 기존 구현을 삭제하지 않는다.

### 설치와 운영

- [ ] Linux clean install과 uninstall/reinstall
- [ ] Codex connection과 health
- [ ] stable Project ID와 clone binding
- [ ] 새 Runtime Home과 schema의 독립성
- [ ] long-running process termination과 exit result 보존

### Canonical Context

- [ ] Project, Source, Question, Decision, Context Item와 Checkpoint
- [ ] restart와 crash recovery
- [ ] correction, revision, supersession와 deletion
- [ ] portable export/import
- [ ] divergent bundle conflict handling
- [ ] Derived State 삭제 후 canonical read

### Repository Intelligence

- [ ] 모든 text fixture inventory
- [ ] Java structural
- [ ] Python structural
- [ ] JavaScript structural
- [ ] TypeScript structural
- [ ] C structural
- [ ] C++ structural
- [ ] Rust structural
- [ ] 최소 세 ecosystem semantic
- [ ] polyglot capability와 coverage
- [ ] realistic multi-file small-application work의 analysis usefulness
- [ ] realistic medium-polyglot cross-component/flow comprehension
- [ ] unsupported language agent-assisted fallback
- [ ] provider unavailable local-only mode
- [ ] partial parser failure와 degradation
- [ ] derived index corruption과 rebuild

### Inquiry와 작업

- [ ] staged Question frontier와 pause/resume
- [ ] 현재 host의 한 번의 user answer로 Decision 기록
- [ ] already answered Question 반복 방지
- [ ] fact research before asking
- [ ] ordinary work와 source-linked Checkpoint
- [ ] unrelated dirty change 분리
- [ ] verification, user review와 acceptance의 독립 상태
- [ ] Decision reuse와 revisit trigger
- [ ] explicit/hidden user-owned Decision discovery와 no-question/research/delegation/prototype/defer qualification
- [ ] hidden task semantic non-disclosure와 unavoidable-outcome counterfactual independent review
- [ ] blind reviewer/operator preparation의 opaque slot, non-matrix ordering과 private mapping integrity

### Recall, UI와 documents

- [ ] first project-scoped bounded Recall
- [ ] user-visible Recall basis와 omissions
- [ ] local viewer의 Project Understanding default와 deeper inspect/correct/supersede/forget
- [ ] completed/current/remaining work, next step, Decision rationale, affected code와
      component/architecture/flow
- [ ] verified fact/generated interpretation distinction와 relation-grounded diagrams
- [ ] Project & Architecture Guide
- [ ] Decision Report
- [ ] Implementation Plan
- [ ] Handoff / Resume Document
- [ ] Markdown와 self-contained HTML
- [ ] source snapshot, capability coverage와 known gaps metadata
- [ ] Korean/English fixed UI와 unrestricted requested-language content
- [ ] requested-language actual body realization 또는 explicit unavailable/degraded result
- [ ] task-oriented/repository-relative CLI, human-readable default와 ordinary UUID-free use

### Risk와 신뢰

- [ ] background semantic provider Project opt-in
- [ ] source transmission scope visibility
- [ ] 별도 source-transmission authorization으로 production provider real success
- [ ] Guarded high-risk effect confirmation
- [ ] ordinary action non-blocking
- [ ] cooperative guarantee의 정직한 표현
- [ ] no hidden user Decision inference

### 실제 사용

- [ ] Volicord dogfood journey
- [ ] small single-language application journey
- [ ] medium polyglot repository journey
- [ ] failure recovery rehearsal
- [ ] 사용자가 raw protocol 없이 Project를 이해하고 판단·재개할 수 있음
- [ ] 모든 qualifying cycle의 fact/interpretation, analysis/polyglot, CLI, Viewer,
      documents, Question necessity·Decision comprehension와 interruption-cost human review
- [ ] final gate의 warning-clean clippy

## 4. 기존 Runtime Home과 데이터

기존 Runtime Home, schema와 record는 새 제품에서 전혀 고려하지 않는다.

제공하지 않는 기능:

- legacy path detection
- legacy data read
- migration 또는 importer
- historical export
- backup recommendation
- compatibility alias
- dual runtime
- old identifier preservation
- legacy rollback product path

개발과 acceptance는 clean new Runtime Home에서 시작한다. 기존 Runtime Home이 개발 machine에 남아 있다면 새 구현과 물리적으로 분리하고 수동으로 관리한다. 새 binary는 이를 찾거나 경고하거나 변환하지 않는다.

## 5. Cutover 시 제거 대상

활성 제품에서 다음을 제거한다.

- Task phase와 shaping/implementation progression
- Change Unit
- ordinary-write Write Ticket과 Guard admission
- CLI 전용 UserAction resolution과 별도 application transition
- Run/Evidence/final-acceptance/close ceremony
- legacy Core, Store, types와 MCP method schema
- legacy conformance와 SignalBox success criteria
- legacy Runtime Home installer assumptions
- legacy migration 또는 historical export 관련 코드와 문서
- old README와 bilingual authority contract tree
- Windows 공식 지원 주장과 기존 installer path

고위험 confirmation, provenance, source observation와 process reliability는 새 계약으로 다시 구현된 경우에만 남긴다.

## 6. Final naming과 versioning

- 최종 root, package, binary와 command는 하나의 `volicord` 제품을 나타낸다.
- public API에 영구적인 제품 세대 namespace나 기존 구현 호환 namespace를 두지 않는다.
- database, portable bundle, analysis snapshot과 generated document에는 독립적인 schema 또는 format version을 둔다.
- format migration은 새 제품 내부 데이터 해석 계약이며 legacy Volicord migration과 무관하다.

## 7. 역사 보존과 실패 처리

- 교체 직전 source commit에 archive tag를 둘 수 있다.
- tag는 개발 이력이며 사용자-facing rollback, compatibility 또는 migration support가 아니다.
- final cutover가 실패하면 branch source를 되돌릴 수 있지만 legacy와 replacement를 runtime에서 동시에 활성화하지 않는다.
- legacy source와 문서는 Git history/tag로만 보존하며 active tree에 archive copy를 중복 저장하지 않는다.
- cutover 후 발견한 문제를 이유로 legacy workflow compatibility layer를 복원하지 않는다. 필요한 사용자 가치와 최소 수정안을 새 Core 기준으로 설계한다.

## 8. 정적 제거 검사

final cutover에서는 최소 다음 범주의 active reference를 검사한다.

```text
Write Ticket
Change Unit
check_close
close_task
final acceptance
UserAction application
legacy Runtime Home
migration
historical export
compatibility alias
```

역사 설명이나 commit message가 아니라 active code, command, user documentation과 tests에 남아 있지 않아야 한다.

reconstruction workspace package가 legacy crate를 의존하지 않는지도 Cargo metadata로 확인한다.

## 9. Cutover 완료 판정

다음이 모두 참일 때 교체가 완료된다.

1. root에서 새 workspace와 final `volicord` binary가 build, test와 package된다.
2. Linux와 Codex의 full journey가 final root에서 통과한다.
3. active docs가 새 제품 목적과 실제 동작만 설명한다.
4. legacy workflow code와 public surface가 제거되었다.
5. first structural language와 semantic gate가 통과했다.
6. clean new Runtime Home에서 Project를 초기화한다.
7. legacy migration, data detection, export와 compatibility 기능이 없다.
8. 임시 `rebuild/` 이름과 product-generation labels가 active artifact에 남지 않는다.
