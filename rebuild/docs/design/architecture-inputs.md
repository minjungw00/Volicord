# Phase 3 architecture 입력 계약

- 상태: 현재 evidence constraint와 문서 ownership 계획
- 적용 범위: Phase 3 target architecture 작업
- 제품 의미 기준: `product-charter.md`, `open-decisions.md`
- 검증 기준: `validation-plan.md`, `rebuild/validation/`의 maintained report와
  `rebuild/validation/wave-1-summary.md`
- 비소유 범위: target architecture, production API, crate 경계와 구현 기술

이 문서는 Phase 3가 사용할 수 있는 증거와 아직 결론 내릴 수 없는 사항을
구분한다. Wave 1 prototype의 구현 모양을 production 설계로 승격하지 않으며,
아래에 계획된 architecture 문서가 생성되기 전 그 문서의 계약을 대신 쓰지
않는다.

## 1. Phase 3를 제약하는 accepted product decisions

Phase 3는 다음 accepted contract를 유지해야 한다.

- 제품 구현 언어는 Rust다. 사용자 자연어에는 allowlist를 두지 않으며,
  Repository Intelligence는 Java, Python, JavaScript, TypeScript, C, C++와
  Rust의 첫 structural gate를 수용하고 모든 text repository에 inventory를
  제공해야 한다.
- Repository Intelligence는 first-party이지만 Canonical Context Kernel과
  분리된다. analyzer fact, semantic result와 agent interpretation은 provenance와
  capability state를 잃지 않는다.
- Canonical Context는 `Project`, `Source`, `Question`, `Decision`, `Context Item`,
  `Checkpoint`를 보존한다. Canonical, Session Candidate와 Derived의 정보 계층을
  분리하고 Derived 삭제가 Canonical 손실을 일으키지 않게 한다.
- 사용자의 현재 host 답변만 연결된 Question의 사용자 Decision이 될 수 있다.
  agent recommendation, observed fact와 generated explanation은 사용자 선택과
  분리한다.
- Inquiry는 material Question을 dependency frontier로 제시하고 decision,
  delegation, research, prototype, deferment, exclusion 또는 supersession으로
  branch를 끝낸다. 질문 수에는 고정 상한을 두지 않는다.
- stable Project identity와 portable canonical context는 repository path와
  분리된다. 다른 clone은 명시적으로 bind하고, 의미 충돌과 delete/modify는
  조용히 합치지 않는다.
- correction revision, semantic Decision supersession, contradiction/review_due와
  forget은 서로 다른 의미다. 개인정보 삭제는 원문을 보존하는 immutable
  audit보다 우선한다.
- 첫 project-scoped 요청의 Recall은 bounded하고 read-only다. Checkpoint는
  source-grounded meaningful boundary에만 생성하며 작업, 자동 검증, 사용자
  review와 acceptance를 독립 상태로 유지한다.
- generated document는 source-grounded read projection이며 명시적 adoption
  전에는 canonical truth가 아니다.
- canonical record 처리와 structural mode는 local에서 작동한다. background
  external provider 전송은 Project opt-in과 inspectable scope가 필요하고,
  interactive host access와 구분한다.
- 일반 repository 작업은 사전 authority 절차로 차단하지 않는다. 열거된
  high-risk effect에만 action-scoped confirmation을 적용한다.
- 첫 공식 환경은 Linux와 Codex다. 새 제품은 독립된 clean runtime에서
  동작하며 legacy workflow crate, Runtime Home, schema 또는 public surface에
  의존하지 않는다.

이 항목은 새 architecture 선택이 아니라 이미 accepted된 제품 제약의 요약이다.
상세 의미와 revisit trigger는 `open-decisions.md`가 계속 소유한다.

## 2. Wave 1 evidence baseline

Architecture Gate는 `ready`다. 이 상태는 Phase 3를 시작할 만큼 책임 경계의
feasibility evidence가 있다는 뜻이며 production 승인을 뜻하지 않는다.

| Evidence | Architecture input으로 사용할 수 있는 결론 | 유지해야 하는 한계 |
|---|---|---|
| V01 — `rebuild/validation/repository-intelligence/polyglot-structural/report.md` | 작은 source-bound common structural envelope와 language adapter 책임을 분리할 수 있고, inventory는 analyzer availability와 독립적일 수 있다. | fixture-sufficient lexical prototype이며 production parser, 실제 repository accuracy, semantic relation 또는 build-aware invalidation을 확정하지 않는다. |
| V03 — `rebuild/validation/canonical-context/portability/report.md` | canonical persistence, portable representation, local clone binding, derived state, revision, supersession와 managed deletion을 별도 책임으로 둘 수 있다. | final database schema, concurrency, upgrade, repair, encryption, merge 또는 tombstone policy를 확정하지 않는다. |
| V05 — `rebuild/validation/inquiry/frontier-resume/report.md` | durable Question state에서 deterministic frontier를 재계산하고 exact Question revision/user turn에 답을 연결하며 recommendation과 choice를 분리할 수 있다. | automatic Question discovery, materiality quality, concurrent turns, production atomic response transaction 또는 bounded Recall 표현을 검증하지 않는다. |
| `rebuild/validation/wave-1-summary.md` | 세 validation의 결론을 함께 사용해 Canonical, Repository Intelligence, Inquiry와 host concern의 책임을 분리할 수 있다. | later validation을 완료한 것으로 간주하거나 experiment code를 production으로 promote하지 않는다. |

V01, V03와 V05는 stable `validation_id`와 report identifier다. 경로의 capability
이름은 탐색과 ownership을 표현하며 제품 generation이나 production branch를
뜻하지 않는다.

## 3. Evidence-to-architecture matrix

| Major area | Evidence가 지지하는 boundary | Evidence가 지지하지 않는 결론 | 남은 불확실성 owner |
|---|---|---|---|
| Repository Intelligence | V01은 common source-bound entity/relation envelope, per-language adapter, capability degradation, failure isolation과 inventory independence를 지지한다. | production parser library, real-repository precision, semantic source, build context, macro/generated-code completeness와 dependency-aware invalidation은 선택할 수 없다. | V02가 semantic adapter normalization을, V11이 combined multi-repository journey를 검증한다. Production parser integration은 실제 Rust tests와 production promotion gate를 통과해야 한다. |
| Canonical Context | V03은 analyzer/provider/host와 독립된 canonical responsibility, derived separation, portable representation, local binding, revision, supersession와 managed deletion을 지지한다. | final database schema, API, multi-project layout, concurrency, encryption, corruption repair, schema upgrade와 final tombstone policy는 정해지지 않았다. | V04가 divergent state와 conflict를, V07이 privacy/deletion boundary를, V10이 storage/process primitive를, V11이 combined recovery를 검증한다. |
| Inquiry and Decision | V05는 durable Question identity/revision/status, deterministic frontier recomputation, exact user-turn linkage, terminal outcome vocabulary와 recommendation/choice separation을 지지한다. | automatic discovery나 materiality quality, general paraphrase recognition, production prerequisite rules, concurrent host turns, authorization와 atomic response API는 입증되지 않았다. | V09가 Recall/Checkpoint와 반복·오귀속을, V11이 실제 combined inquiry journey를 검증한다. Production Inquiry semantics는 Rust integration/property tests로 다시 검증한다. |
| Portable context and conflict handling | V03은 stable Project identity, deterministic export/import, source-independent read와 explicit another-clone binding을 지지한다. | merge algorithm, common-base discovery, automatic merge safety, delete propagation와 conflict presentation은 지지하지 않는다. | V04가 divergent bundle merge와 사용자 conflict resolution을 소유한다. V11이 combined another-clone journey를 검증한다. |
| Projections and generated documents | Accepted contract와 Wave 1 boundaries는 projection이 canonical/derived input을 읽고 source identity와 coverage를 보존하며 side effect로 canonical을 바꾸지 않아야 함을 제약한다. | Wave 1은 claim grounding quality, document completeness, Markdown/HTML rendering, stale invalidation 또는 adoption behavior를 실행 검증하지 않았다. | V06이 source-grounded documents를 검증한다. V11이 combined document journey를 검증한다. |
| Privacy and provider boundaries | Accepted contract와 V03의 provider-independent/local experiment는 Canonical Context가 provider에 의존하지 않고 raw source 없이 portable할 수 있는 책임 분리를 지지한다. | opt-in enforcement, actual network scope, secret/exclude quality, semantic-provider implementation과 annotation deletion completeness는 지지하지 않는다. | V07이 local-only mode, provider opt-in, transmission scope와 deletion을 검증한다. V11이 combined degraded behavior를 검증한다. |
| Recall and Checkpoint | V03은 Checkpoint를 다른 canonical records와 함께 durable하게 보존할 수 있음을, V05는 pause snapshot이 frontier authority가 아니어도 resume할 수 있음을 지지한다. | bounded Recall selection quality, no-mutation projection, omission reporting, dirty-change attribution, meaningful Checkpoint detection과 user comprehension은 검증되지 않았다. | V09가 Recall/Checkpoint 정확성을 검증하고 V11이 fresh-session resume journey를 검증한다. |
| Failure and recovery | V03의 committed-state recovery와 V05의 killed reader resume는 canonical storage와 derived/frontier recomputation을 분리할 수 있음을 좁은 fixture 범위에서 지지한다. | production crash atomicity, process supervision, corruption repair, aggregate degradation, resource limits와 end-to-end recovery policy는 정할 수 없다. | V08이 Linux/Codex operation을, V10이 process/filesystem primitive를, V11이 combined failure recovery를 검증한다. |

Later validation 결과가 accepted decision의 revisit trigger를 충족하면
`open-decisions.md` 절차를 따른다. Architecture 문서가 evidence gap을 자체
가정으로 닫아서는 안 된다.

## 4. Validation-language ownership policy

- Python 또는 다른 적합한 external language는 disposable feasibility
  experiment, fixture orchestration, external analyzer invocation,
  cross-language black-box comparison과 end-to-end harness에 사용할 수 있다.
- Rust는 production domain semantics, Canonical Context invariant, durable
  storage behavior, Inquiry transition, production serialization, production
  crash/recovery behavior와 production integration/property test를 소유한다.
- Python experiment는 Rust production behavior의 두 번째 장기 reference
  implementation이 될 수 없다.
- production 의미가 disposable Python prototype만으로 검증된 상태로 남아서는
  안 된다.
- V01 orchestration은 language-neutral black-box validation의 이점이 있는 동안
  external로 남을 수 있다.
- V03와 V05 semantics는 production promotion 전에 실제 Rust production
  implementation을 대상으로 다시 표현하고 검증해야 한다.
- Experiment source의 import 관계는 internal test support일 뿐 production
  dependency 방향이나 module boundary의 근거가 아니다.

## 5. Phase 3 document ownership plan

아래 파일은 계획된 owner다. 파일이 실제로 생성되기 전에는 active contract가
아니며, 이 문서나 이름만으로 내용을 추론하지 않는다.

| Planned document | 생성 후 소유하는 contract | 소유하지 않는 사항 |
|---|---|---|
| `architecture.md` | subsystem map, cross-subsystem dependency direction, integration boundary와 cross-document boundary conflict resolution | specialized domain의 상세 invariant와 Wave 1 evidence 자체 |
| `domain-model.md` | Canonical/Candidate/Derived 분류, core identity, provenance, record relation, revision/supersession/forget의 domain meaning | storage schema, bundle merge procedure와 UI representation |
| `repository-intelligence.md` | snapshot, inventory, entity/relation, capability/coverage/freshness와 language/semantic adapter contract | 사용자 judgment, final parser/provider technology와 canonical persistence |
| `inquiry-and-decision.md` | Question/Decision/Candidate behavior, dependency frontier, response linkage, terminal transition, Decision applicability, Checkpoint/Recall interaction sequence | Canonical storage schema, projection rendering과 host-specific wire format |
| `portable-context.md` | portable bundle boundary, Project/clone binding, source availability, divergence, conflict class와 resolution contract | canonical domain meaning의 재정의와 evidence 없는 merge algorithm 선택 |
| `privacy-and-provider-boundary.md` | local processing, interactive/background distinction, Project opt-in, transmission scope, secret/exclude, annotation retention/deletion contract | provider implementation 선택과 general authorization architecture |
| `projections-and-documents.md` | Recall/view/document read projections, grounding metadata, adoption boundary, Markdown/HTML output contract | canonical mutation semantics, UI framework와 inquiry transition |
| `failure-and-recovery.md` | subsystem failure/degraded states, transaction/crash boundary, repair/rebuild responsibility와 process recovery contract | final storage/process technology와 normal domain meaning의 재정의 |
| `versioning-policy.md` | production schema/format version boundaries, new-product format evolution, upgrade/test responsibility와 unsupported-version behavior | legacy data handling과 concrete schema field design |

## 6. Ownership precedence and activation

1. `product-charter.md`와 `open-decisions.md`의 accepted product meaning이 모든
   architecture 문서를 제약한다.
2. `architecture.md`는 cross-subsystem dependency direction을 소유하고
   specialized 문서 사이의 boundary conflict를 해결한다.
3. 각 specialized document는 위 표의 named domain contract를 소유한다.
4. `architecture-inputs.md`는 evidence constraint, unsupported conclusion과 이
   ownership plan만 소유한다. Target architecture 자체를 소유하지 않는다.
5. Planned document는 파일이 생성되고 governing instructions가 해당 파일을
   route한 시점부터 active owner가 된다. 생성 전에는 active contract로
   표시하거나 없는 내용을 전제하지 않는다.
6. Evidence와 target contract가 충돌해 보이면 evidence limit을 먼저 확인하고,
   accepted Decision을 바꿔야 할 때만 revisit 절차를 사용한다.

## 7. Phase 3가 추론해서는 안 되는 사항

현재 evidence는 다음을 결정하지 않는다.

- production parser library
- final database schema
- merge algorithm
- semantic-provider implementation
- automatic Question-discovery quality
- UI framework
- Wave 1 prototype의 production approval

또한 Wave 1 status나 report recommendation을 crate/API taxonomy, wire schema,
storage field 또는 provider selection으로 번역해서는 안 된다. 이런 선택은
해당 owner 문서와 later validation evidence가 존재할 때만 다룬다.

## 8. Phase 3 시작 조건

Phase 3 작업자는 이 문서와 root/rebuild `AGENTS.md`의 required read를 완료하고,
Wave 1 summary와 세 maintained report의 current path를 확인해야 한다. 이 준비
session의 focused validation과 exact final aggregate가 통과한 commit 위에서만
target architecture 작업을 시작한다.
