# Projection과 document 계약

- 상태: active specialized architecture owner
- 소유 범위: first project-scoped Recall, bounded Resume Brief, user/agent read
  projections, Candidate Inspection, Decision–Context–Code map, generated-document
  grounding, draft/preview, review/correction, explicit adoption과 output format boundary
- 상위 architecture 기준: [논리 아키텍처](architecture.md)
- core domain 기준: [핵심 도메인 모델](domain-model.md)
- Inquiry 기준: [Inquiry와 Decision 계약](inquiry-and-decision.md)
- analysis 기준: [Repository Intelligence 계약](repository-intelligence.md)
- privacy 기준: [Privacy와 provider 경계](privacy-and-provider-boundary.md)
- validation 기준: [기술 검증 계획 V06·V09·V11](validation-plan.md)
- 비소유 범위: canonical mutation semantics, Inquiry transition, UI framework,
  renderer/template technology, portable conflict resolution, storage/API와
  background-provider policy; portable boundary는
  [Portable Context 계약](portable-context.md)이 소유함

이 문서는 Canonical Context, Candidate Inspection에만 top-level architecture가 허용한
bounded Session Candidate metadata와 permitted Derived State를 사람이 이해하고 agent가
다시 사용할 수 있는 read projection으로 만드는 계약이다. Projection은 source
record의 authority나 identity를 복제하지 않으며 generation/render/export를 canonical
write의 숨은 경로로 사용하지 않는다.

## 1. Projection invariant

모든 Recall, map, view와 generated document에 다음 불변 조건을 적용한다.

- Projection input은 Canonical Context, Candidate Inspection에만 허용된 bounded Session
  Candidate metadata와 permitted Derived State로 제한한다. Candidate Inspection 외의
  projection은 Session Candidate read authority를 얻지 않는다.
- canonical record identity와 revision을 새 projection-local identity로 대체하지 않는다.
- Source basis, Repository/Analysis Snapshot, capability와 coverage를 보존한다.
- freshness, uncertainty, contradiction, supersession와 availability를 숨기지 않는다.
- included/omitted state와 omission reason을 inspect할 수 있게 한다.
- user projection과 agent projection이 표현 깊이나 layout이 달라도 같은 canonical
  identity, Source basis와 validity state를 사용한다.
- projection read, render, preview, export 또는 failure가 canonical record를 mutate하지
  않는다.
- Derived cache나 preview를 삭제해도 Canonical Context가 손상되지 않는다.

Projection은 current truth의 별도 authority가 아니다. Projection과 source record가
달라지면 source record와 current analysis를 다시 읽고 projection을 stale/rebuild
대상으로 다룬다.

## 2. First project-scoped automatic Recall

새 agent session의 첫 `project-scoped` 요청에서는 bounded, read-only Recall을
자동으로 수행한다.

- Project identity와 current binding이 확인된 요청만 trigger가 된다.
- 단순 인사, unrelated conversation과 Project를 특정하지 않는 요청에는 실행하지
  않는다.
- 한 session의 first project-scoped trigger를 hidden canonical state transition으로
  기록하지 않는다.
- 사용자는 Recall이 사용됐다는 사실, 핵심 basis와 펼쳐볼 record/source path를 알 수
  있다.
- 매번 전체 Project history를 강제로 출력하지 않는다.

Automatic Recall은 user가 명시적으로 요청하는 later Recall을 막지 않는다. Trigger와
bounded selection의 구체적인 host wire/API는 이 문서가 선택하지 않는다.

## 3. Bounded read-only Resume Brief

`Resume Brief`는 Project를 계속하기 위한 최소한의 source-grounded projection이다.
Bounded는 content budget 안에서 중요한 basis를 선택하고 omission을 보고한다는
뜻이며 record를 truncate해 다른 의미로 바꾼다는 뜻이 아니다.

### Minimum meaning

Resume Brief는 최소 다음을 포함한다.

- **goal and why:** current goal, user value와 관련 Source/Context
- **active Decisions and rationale:** applicability가 맞는 Decision, user rationale,
  alternatives와 supersession state
- **current state and recent Checkpoint:** meaningful work state, recent change,
  verification, review/acceptance의 독립 상태
- **open Questions:** canonical identity/revision, current frontier/blocked distinction과
  what each answer unlocks
- **risks, assumptions and known limits:** statement role, Source basis와 review trigger
- **next meaningful step:** Checkpoint/Decision/Question basis가 있는 actionable direction
- **sources, capability, freshness and omissions:** used Sources/snapshots, analysis
  capability/coverage, stale/unavailable/failed scope, omitted count와 reason

Brief는 사용자 판단, agent recommendation, observed fact, semantic result와 generated
interpretation을 구분한다. Source repository가 unavailable해도 goal, Decision과
Checkpoint를 제공하고 current code relation을 unavailable로 표시한다.

### Bounded selection과 omission

Selection은 Project/scope relevance, active applicability, recency of meaningful
Checkpoint, open material Question와 declared risk를 사용할 수 있다. Access frequency는
ordering input이 될 수 있지만 Decision validity나 Question outcome을 바꾸지 않는다.

각 candidate item은 최소 `included` 또는 `omitted`로 판정되고 omission에는
budget, scope, superseded/history, unavailable basis 또는 user filter 같은 reason을
연결한다. Deterministic section bound로 생긴 omission은 omitted identity를
하나씩 projection에 복제하지 않고 bounded scope, exact omitted count와 reason을
하나의 stable report로 표현한다. 사용자와 agent view는 같은 input scope에
대해 같은 inclusion/omission basis와 count를 사용한다. 더 깊은 view는
authoritative input과 scope를 다시 읽어 omitted item을 펼칠 수 있지만 aggregated
report나 hidden memory를 identity authority로 사용하지 않는다.

동일한 input state, scope와 bound에서는 stable tie-breaker로 reproducible selection을
만든다. Ranking/model의 concrete algorithm은 V09 evidence 뒤의 implementation choice다.

## 4. Projection purity와 no-mutation

Projection operation은 다음을 하지 않는다.

- Source, Question, Decision, Context Item 또는 Checkpoint create/correct/supersede/forget
- stale Source를 current로 갱신하거나 Decision `review_due`를 자동 resolve
- Inquiry frontier나 Question terminal outcome 변경
- Semantic Annotation을 canonical fact로 promotion
- access/omission을 user preference나 acceptance로 기록
- generated output을 preserved Source로 자동 채택

Projection은 disposable cache, layout, selection trace와 preview를 Derived State로
만들 수 있다. Operational observation이 필요해도 canonical mutation과 별도이며
projection result의 성공 조건이 아니다. Canonical correction, Checkpoint 생성 또는
adoption은 explicit intent와 Kernel operation을 사용하는 별도 command다.

여러 subsystem read 중 일부가 실패하면 available section과 omitted/failed section을
구분한다. Projection 실패를 repository work나 canonical transaction failure로
바꾸지 않는다.

## 5. User와 agent projection

User projection은 comprehension과 inspectability를, agent projection은 accurate
continuation에 필요한 structured depth를 우선할 수 있다. 차이는 표현과 depth에만
있다.

두 projection은 다음 basis를 공유한다.

- canonical identity, revision과 relation
- Source와 statement role/provenance
- Decision applicability, supersession와 revisit state
- Repository/Analysis Snapshot과 capability/coverage
- freshness, uncertainty, contradiction와 known limit
- inclusion/omission state, reason와 bounded count

Agent-only hidden summary를 user-visible record보다 높은 authority로 사용하지 않는다.
User view의 단순화가 uncertainty나 failed scope를 complete success로 바꾸지 않는다.

### Candidate Inspection projection contract

`Candidate Inspection`은 local Project context의 Session Candidate를 읽는 named,
read-only projection이다. Projections and Documents가 이 read projection을 소유하고
`domain-model.md`의 Candidate meaning/lifecycle과
`privacy-and-provider-boundary.md`의 collection/retention policy를 그대로 사용한다.
Read는 domain owner가 정의한 user-inspectable Candidate metadata와 applicable
collection, retention, deletion 및 privacy policy가 명시적으로 허용한 bounded Candidate
content로 제한된다. Full prompts, full tool arguments, full Source bodies, unlimited
stdout/stderr, provider-private payloads, expired/deleted content와 authorized Project/scope
밖의 content를 읽는 blanket authority가 아니다.

각 visible Candidate에 대해 최소 다음 attributes를 노출한다.

| Inspectable attribute | Projection obligation |
|---|---|
| `existence_and_identity` | Candidate가 현재 존재하는지와 local Project 안의 candidate identity |
| `candidate_kind` | observation, hypothesis, semantic claim, Question/Checkpoint candidate 또는 promotion proposal kind |
| `origin_and_provenance` | actor/subsystem/session과 Source/snapshot/command/host/provider basis |
| `collection_scope` | 이 Candidate를 수집한 Project/session/source/operation scope |
| `creation_or_observation_basis` | created/observed time과 bounded evidence/request basis |
| `retention_or_expiry_state` | retention policy, expiry basis와 cleaned state |
| `promotion_disposition` | pending/retained, promoted, dismissed 또는 expired/retention-cleaned 상태와 result basis |
| `scope_opt_out_state` | 해당 collection scope의 current opt-out state와 effective basis |

Candidate Inspection read는 Candidate를 promote, correct, dismiss, expire, delete 또는
reinterpret하지 않는다. Projection access/omission도 retention clock, promotion
authorization 또는 opt-out state를 바꾸지 않는다. Mutation은 각각의 explicit domain,
privacy 또는 lifecycle operation으로 분리한다.

Candidate source/body가 privacy boundary로 unavailable하거나 일부 Candidate read가
실패하면 available metadata와 affected scope를 `partial`/`degraded`로 표시한다.
Inspection failure는 projection degradation일 뿐 Candidate나 canonical record를
promote, delete, rewrite 또는 reinterpret하지 않는다. Direct scoped inspection이나
later retry path를 제공하되 hidden cache를 더 높은 authority로 사용하지 않는다.

## 6. Decision–Context–Code map

`Decision–Context–Code map`은 다음 identity를 연결하는 read-only projection이다.

```text
Decision ──applies_to/assumes───────────────▶ declared scope / assumption Context
Decision rationale ──supported_by───────────▶ Source
    │                                             │
    └──declared path/component scope──────────────┤
                                                  ▼
                          Repository/Analysis Snapshot의 Code Entity / Relation
                                                  │
                                                  ▼
                                  relevant Checkpoint / open Question
```

Map은 최소 다음을 표현한다.

- Decision identity, active/superseded/review_due와 applicability scope
- rationale, assumption, risk, constraint와 known-limit Context identity
- supporting Source와 current availability/freshness
- snapshot-bound Code Entity/Relation과 capability/provenance class
- relevant Checkpoint와 open Question reference
- missing link, unsupported/failed area, uncertainty와 omission

Graph adjacency나 visual proximity는 causal fact가 아니다. Path scope가 Code Entity와
겹친다는 이유만으로 Decision이 구현됐다고 주장하지 않고, Agent Interpretation으로
추론한 architecture link를 Structural Fact처럼 표시하지 않는다. Layout과 graph
storage는 Derived State다.

## 7. Initial generated documents

첫 generated-document contract는 다음 네 유형을 포함한다.

### Project & Architecture Guide

Project goal, component와 boundary, repository structure, key flow, active architecture
Decisions, capability coverage와 known limits를 설명한다. Architecture claim마다 Source,
Structural/Semantic basis 또는 explicit inference marker가 필요하다.

### Decision Report

Question, displayed alternatives, Agent Recommendation, explicit user Decision/rationale,
applicability, assumptions, Source basis, expected consequence, revisit trigger와
supersession trail을 구분한다. Agent recommendation을 user choice로 합치지 않는다.

### Implementation Plan

Current goal과 active Decisions에서 도출한 ordered work, affected path/component,
prerequisite, verification, risk, known limit와 next step을 제시한다. Projection은
계획을 작업 완료나 repository mutation으로 기록하지 않는다.

### Handoff / Resume Document

Resume Brief의 minimum meaning을 다른 agent/session/environment가 독립적으로 읽을 수
있는 형태로 제공한다. Recent Checkpoint, open Question/frontier basis, unfinished work,
verification state, omissions와 source availability를 포함한다.

문서 유형 이름은 template/renderer 선택이 아니며 사용자가 요청한 natural language를
인위적인 allowlist로 제한하지 않는다. Code identifier, path와 API name은 원문을
유지한다.

## 8. Grounding metadata

각 generated draft, preview와 export는 최소 다음 grounding을 가진다.

- Project identity
- generation time과 generator/agent/model identity
- canonical read revision 또는 동등한 generation basis
- Repository Snapshot과 사용한 Analysis Snapshot
- included Decision identities/revisions와 applicability state
- used Source identities와 availability/freshness
- capability별 language/area coverage
- excluded, unsupported, unavailable, partial, failed와 stale scope
- known gaps, uncertainty와 explicit inference marker
- bounded scope별 exact omitted record/source count, reason와 user-specified scope
- output document type, language와 requested destination basis

Metadata는 claim마다 필요한 direct Source reference를 대체하지 않는다. Core claim은
어떤 Source/Decision/analysis basis에서 왔는지 추적할 수 있어야 한다. Snapshot이나
Decision이 바뀌면 existing document를 current로 가장하지 않고 stale/review
projection으로 표시한다.

## 9. Draft, preview, correction과 review

Generated output의 기본 lifecycle은 다음과 같다.

```text
source-grounded generated draft
→ read-only preview/export candidate
→ user/agent correction or review annotation
→ optional regenerated/reviewed draft
→ explicit adoption request
```

- `generated draft`와 preview는 Derived State다.
- Preview/render failure는 draft Source basis나 canonical records를 변경하지 않는다.
- User correction은 generated wording/selection에 대한 review input이며 underlying
  Decision/Context/Source를 자동 correction하지 않는다.
- Underlying canonical 오류를 발견하면 별도의 explicit canonical correction,
  supersession 또는 forgetting operation으로 route한다.
- Agent review/recommendation은 user review/acceptance와 구분한다.
- Regeneration은 prior user correction과 review basis를 조용히 버리거나 underlying
  provenance를 rewrite하지 않는다.

Generated draft가 file로 export됐다는 사실만으로 preserved Source가 되지 않는다.
Product Repository write는 user가 exact destination을 지정한 경우에만 수행하고,
publication 결과와 document adoption은 별도 사실로 남긴다.

## 10. Explicit adoption과 preserved Source boundary

Generated 또는 user-edited document를 장기 basis로 보존하려면 explicit adoption
intent와 Canonical Context Kernel operation이 필요하다.

Adoption은 최소 다음을 확인한다.

- adopted artifact/document identity와 exact revision/content basis
- adopting current-host user Source와 intent
- origin generated draft와 grounding metadata
- user/agent edits, review status와 editor provenance
- document가 support하거나 제안하는 canonical target/scope
- known stale source, gaps, uncertainty와 exclusions

성공한 adoption은 artifact를 preserved `Source`로 만들거나 별도의 explicit Context
promotion basis로 사용할 수 있다. Adoption은 다음을 하지 않는다.

- generated claims를 observed fact로 변환
- included Decision을 rewrite/supersede
- original repository Source를 document로 대체
- user-edited text를 original agent/model output으로 표시
- document acceptance를 implementation completion이나 user Decision으로 일반화

Adopted document와 underlying Sources는 각각 identity/provenance를 유지한다. Generated
explanation/draft는 사용한 Source 또는 canonical basis를 향해 `derived_from`하고,
adopted statement-bearing Source나 Context는 supporting Source를 향해 `supported_by`한다.
Document 수정이 semantic meaning을 바꾸면 adopted Source의 새 revision/adoption 의미를
명시하며 원래 Source basis를 조용히 rewrite하지 않는다.

## 11. Portable output format boundary

- Markdown은 네 initial document의 portable default다.
- Self-contained HTML은 preview 또는 공유/export 형식이다.
- HTML은 필요한 presentation asset을 자체 포함하고 external runtime dependency를
  강제하지 않는다.
- Markdown과 HTML은 같은 grounding metadata, identity, omission과 uncertainty basis를
  보존한다.
- PDF와 DOCX는 initial required output이 아니다.

이 계약은 Markdown dialect, HTML renderer, template engine, CSS, sanitizer, viewer
framework 또는 conversion library를 선택하지 않는다. Output format은 canonical
portable-context bundle format이나 storage schema가 아니다.

## 12. Freshness, failure와 omission

Projection은 input별 current state를 보존한다.

- stale Source/analysis를 current evidence로 표시하지 않는다.
- unavailable repository에서는 canonical-only section을 계속 제공하고 code section의
  unavailable basis를 표시한다.
- partial/failed analyzer area는 coverage와 omitted claim scope를 함께 표시한다.
- superseded Decision은 current recommendation에 섞지 않되 history omission 또는
  explicit trail로 inspect 가능하게 한다.
- provider 부재/실패는 provider-backed annotation을 degrade할 뿐 local/canonical
  projection 전체를 막지 않는다.
- rendering/export 실패는 generated draft나 canonical source mutation으로 보고하지
  않는다.

Projection이 source gap을 발견하면 Question/Context/Checkpoint Candidate를 제안할 수
있지만 read operation 안에서 promotion하거나 frontier를 변경하지 않는다.

## 13. Later-validation hooks

### V06 — Source-grounded documents

V06은 single-language, polyglot와 partial/failed analyzer fixture에서 다음을 검증해야
한다.

- 네 initial document의 required meaning과 grounding metadata
- architecture claim의 Source 또는 explicit inference marker
- Structural Fact, Semantic Result와 Agent Interpretation 분리
- included active/superseded Decision completeness와 applicability
- coverage/known gap/uncertainty/omission visibility
- generated draft/preview의 canonical no-mutation property
- explicit adoption 전후 Source identity와 provenance boundary
- Markdown portability와 self-contained HTML equivalence
- user-specified destination이 없을 때 Product Repository write 부재

### V09 — Recall과 Checkpoint 정확성

V09는 다음을 검증해야 한다.

- unrelated greeting과 first project-scoped trigger 구분
- bounded Resume Brief의 goal/rationale/state/open Question/risk/next-step recovery
- deterministic selection, scope별 exact omitted count/reason과 authoritative expansion basis
- user/agent projection의 identity/source/freshness/uncertainty/supersession/omission 일치
- Recall no-mutation property와 Checkpoint non-frontier authority
- stale Source, superseded Decision, unrelated dirty change와 verification state 표현
- Candidate promotion authorization/disposition과 Candidate Inspection의 existence, kind,
  provenance, collection scope, retention/expiry, opt-out 및 no-mutation behavior

### V11 — Combined journey

V11은 Volicord, single-language와 polyglot repository에서 다음을 결합 검증해야 한다.

- fresh session automatic Recall로 실제 작업 재개
- Decision–Context–Code map의 source/capability honesty
- four documents가 다른 agent의 handoff와 user comprehension에 충분함
- provider/analyzer/source unavailable 상태의 useful degraded projection
- correction/review/regeneration/adoption 뒤 provenance와 canonical purity
- projection/cache deletion과 render/export failure가 canonical loss로 전파되지 않음
- Candidate collection부터 read-only inspection, promotion/dismissal/expiry까지의
  integrated identity와 projection-degradation isolation

V06/V09가 selection 또는 grounding quality 한계를 드러내면 evidence와 omission을
보존하고 accepted Q5/Q9 revisit 절차를 따른다. 문서가 스스로 canonical contract를
확장하지 않는다.

## 14. Non-goals

이 문서는 viewer framework, graph layout, renderer, template, ranking/embedding,
database, API, MCP method, output publication mechanism과 host wire format을 선택하지
않는다. Portable bundle content/merge, Inquiry transition과 legacy document
compatibility도 정의하지 않는다. Production failure matrix는
[Failure와 Recovery 계약](failure-and-recovery.md), versioning은
[Versioning 정책](versioning-policy.md)이 소유한다.
