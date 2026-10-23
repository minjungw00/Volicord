# Projection과 document 계약

- 상태: active specialized architecture owner
- 소유 범위: first project-scoped Recall, bounded Resume Brief, user/agent read
  projections, Candidate Inspection, Project Understanding Viewer, Decision–Context–Code map, generated-document
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

### Project Understanding read model

`Project Understanding`은 Canonical Context와 Repository Intelligence를 같이 읽어
사람이 Project를 이해하고 작업을 이어 갈 수 있게 구성한 derived/read-side
interpretation이다. Runtime Home, database, opaque record 목록을 기본으로 탐색하는
browser가 아니며 새 canonical truth, Decision 또는 Repository Fact를 만들지 않는다.

기본 Viewer는 다음 user question에 자연어로 답한다.

- 무엇을 달성했고 완료한 work가 무엇인가
- 현재 Project와 work의 상태, 남은 work, blocker와 next meaningful step은 무엇인가
- 어떤 Decision을 왜 내렸고 어디에 적용되며 어떤 code가 영향을 받는가
- code가 어떻게 동작하고 component, boundary, architecture와 flow가 어떻게 연결되는가
- 설명의 evidence basis, coverage, freshness, known gap과 uncertainty가 무엇인가

Record identity, raw envelope, hash, provider request, audit field와 lifecycle detail은 모두
inspectable하게 유지하되 ordinary reading hierarchy 뒤의 explicit detail/audit disclosure에
둔다. 이 hierarchy는 grounding을 삭제하거나 low-level inspection을 막지 않는다.

Production read interface의 `ProjectUnderstanding`은 bounded section으로
`project_purpose`, `current_work`, `completed_work`, `remaining_work`, `work_history`,
`unresolved_work_grouping`, Work Overview의 `next_steps`,
`active_decisions`, `open_questions`, `risks_assumptions_and_limits`, inspectable
architecture `components`/`relationships`, `generated_interpretations`와 `evidence`를
분리해 제공한다. Architecture topology는 Repository Intelligence entity/relation을
복제한 read basis이고, realization이나 Viewer가 node/edge를 추가하는 authority가
아니다. 각 section bound의 exact omitted count를 별도로 제공하며 이 read interface는
canonical, Candidate, analyzer, publication 또는 provider mutation capability를 받지
않는다.

`project_purpose`는 `Project Purpose` 역할의 canonical Context Item과 그 Source basis만
읽는다. Work `Goal`, latest Checkpoint goal, recent Decision text 또는 chronological latest
record를 Project Purpose로 승격하지 않는다. Purpose가 기록되지 않았으면 빈 값과 gap을
정직하게 표시하며 최근 work Goal을 대신 보여 주지 않는다.

Work projection은 Goal Context identity를 `work_item_id`로 사용한다. 같은 identity의
Checkpoint를 하나의 history로 aggregate하고 그 그룹 안의 latest Checkpoint에서 state를
계산하며, explicitly work-scoped Decision과 Checkpoint-applied Decision을 연결한다. Work마다
title, state, Checkpoint/Decision identity, changed path/component, verification, next step과
open Question을 제공한다. Goal만 있고 Checkpoint가 없으면 `open`이다. Work association이
없는 Decision/Checkpoint는 `unresolved_work_grouping`에 record identity와 이유를 표시하며
chronology, equal text 또는 path overlap으로 임의 배치하지 않는다.

Recall/continuation, Project Understanding, 네 generated document와 Viewer Work grouping은
이 canonical identity를 그대로 사용한다. Git commit count, shared commit, HEAD 변경과
repository chronology는 grouping key가 아니며 stale repository evidence도 Work identity를
rewrite하지 않는다. Commit 없이 남긴 Checkpoint와 여러 commit을 거친 같은 Work의
Checkpoint는 같은 history이고, 같은 path 또는 나중의 combined commit을 공유한 distinct
Work는 별도 history다. Git evidence의 freshness/baseline/provenance는 계속 표시한다.

`ProjectProjection.repository_map`은 계속 repository-wide entity/relation inventory에서
고른 일반 Repository Map topology를 소유한다. 별도 `current_work_topology`는 같은 Analysis
Snapshot의 실제 entity/relation 중 latest meaningful Checkpoint path/identity, Goal Context와
active Decision scope에 grounded된 seed 및 bounded one-hop relation을 일반 Repository Map
presentation bound보다 먼저 선택한다. 이는 repository-wide 두 번째 map이나 새 topology
authority가 아니라 current-work 전용 bounded selection evidence다. `current_work_code`는 이
선택 결과의 entity locator와 canonical seed basis를 연결하고, `ProjectUnderstanding.architecture`가
이를 사용해 현재 작업 설명 surface를 소유한다.
Latest Checkpoint가 stable `work_item_id`를 가지면 latest meaningful Checkpoint basis는 같은
Work Item history 전체에서 읽는다. 따라서 later verification/resume Checkpoint에 새 changed
path가 없어도 earlier same-work Checkpoint의 path와 exact Checkpoint identity는 grounding에서
사라지지 않는다. Work association이 없는 latest Checkpoint는 그 record 하나만 사용하며
chronology나 path overlap으로 다른 Checkpoint를 합치지 않는다.
실제 Repository Intelligence relation이 그 seed끼리 연결하거나 seed에서 한 홉 떨어진
endpoint를 설명할 때만 관계와 이웃 component를 포함한다. Current-work seed가 하나도 없으면
generic Repository Map node를 대신 채우지 않고 bounded empty/gap 결과를 제공한다.

Current-work architecture의 각 component는 `selection_basis`에 changed path와 exact
Checkpoint identity, Decision identity, Goal Context identity 또는 grounded one-hop relation
identity/seed identity를 bounded provenance로 보존한다. 이 basis는 공개 scoring formula가
아니며 relevance 이유를 검사하고 Viewer에서 current-work 강조를 설명하기 위한 typed
근거다. Repository entity는 precise Source range가 없는 경우에도 portable area `locator`를
보존하여 path grounding을 잃지 않는다.

Architecture bound는 component를 먼저 자른 뒤 우연히 남은 endpoint 사이의 relation만
보존하지 않는다. 실제 Repository Intelligence relation과 그 양 endpoint를 하나의
deterministic selection으로 선택하고, current bound가 endpoint 둘과 relation 하나를
표현할 수 있으며 qualifying relation이 존재하면 최소 하나의 연결된 topology를
보존한다. Active Decision linkage, source-grounded component kind, connection structure와
stable identity를 relevance/tie-break basis로 사용할 수 있다. 선택된 relation의 양 endpoint는
항상 같은 bounded component set에 있으며 component/relation omission count를 각각 exact하게
보고한다. Target이 resolve되지 않은 실제 relation은 topology edge가 아니라 별도의 bounded
inspectable explanation evidence로 identity와 source endpoint를 보존할 수 있으며 target
component를 발명하지 않는다. 이 bounded subgraph는 repository-wide completeness를 뜻하지
않는다.

Current-work diagram은 선택된 grounded relation endpoint를 우선 보존한다. Component
topology는 relation이 없는 current-work seed를 축약된 node로 보여 줄 수 있지만,
request/data/control-flow diagram은 남는 용량을 disconnected generic component로 채우지
않는다. Qualifying grounded flow가 없으면 node/edge를 발명하는 대신 reduced diagram과
explicit gap을 제공한다.

`ProjectUnderstanding`의 local deterministic explanation은 verified canonical,
Structural Fact와 Semantic Result만 조합한 `deterministic_derived` presentation layer다.
각 item은 사용한 entity, relation, Decision, Source와 snapshot identity를 보존하고,
optional model/agent `generated_interpretation`과 합치지 않는다. 이 path는 background
provider를 요구하거나 새 topology를 만들지 않으며 bundled Viewer의 fixed English/Korean
locale만 실현한다. 그 밖의 requested-language generated body 성공은 host realization
계약을 계속 사용하고 local deterministic explanation으로 임의 충족했다고 표시하지 않는다.

### Work reading and selection

#### Independent question answer specifications

These specifications precede the selection/explanation replacement. The oracle is
the canonical field meaning and self-authored source material, never renderer
output. `fixtures/viewer-reading/answer-cases.json` supplies independent expected
claims. All questions use exact Project/Goal identity; equal titles, paths and Git
commits confer no association. Detail retains original text and revision evidence.

| Question | Required meaning and exact evidence | Time/scope rule | Forbidden claim / absent behavior | Placement |
| --- | --- | --- | --- | --- |
| What was reported? | Latest nonblank `Checkpoint.state_change`, its revision and Sources; purpose/effect from clear source material | Same Work, result observation time then identity; correction uses current revision | Blank/null later records cannot erase earlier result; reported change is not verified achievement; no result means no reported result | Ordinary explanation; quotation in evidence |
| What is the latest state? | `work_state` with exact Checkpoint revision/Sources | Latest same-Work observation; Goal-only derives Open | Cannot derive state from change text, verification, review or acceptance | Ordinary |
| What was verified? | Latest nonempty `verification` observation; individual fact state, Command Source and outcome | Independently selected; subsequent meaningful change makes coverage historical/unknown; no silent inheritance | No record differs from explicit NotRun, failure and historical pass; cannot infer coverage from filename or kind | Full evidence with historical warning and complete history |
| Was it reviewed / accepted? | Latest independent `user_review` / `user_acceptance` facts and their Source, including explicit reset to NotRequested | Latest same-Work observation per dimension | Review is not acceptance; accepted is not passed; absent Checkpoint is no observation | Full evidence; independent current facts and history |
| What next? | Latest Checkpoint `next_step` and exact revision/Sources | Latest observation, no fallback to obsolete earlier direction if blank | No recorded direction means missing next-step information | Full evidence; generated uncertainty stays in narrative |
| What is current/completed/remaining? | Complete canonical Work classification before category bounds; category total/displayed/omitted and completeness | State observation time for current/remaining, result time for outcomes, identity only as tie-break | Catalog page and unrelated completed Works cannot erase classification; unknown completeness is not zero | Shared read model; recent cards link to exact records |
| What does this Work mean? | Purpose, reported change, expected effect, verification limits and unresolved next step, each grounded in exact selected evidence | Current question/evidence revisions and language; interpretation remains derived | No invented feature for generic implementation-changed prose; excerpt/unavailable alone fails sufficient-source explanation | Ordinary answer with deeper grounding |

Null/blank history prefixes and later changes must be observed through Operations
and Viewer. Source-rich multilingual audit prose and independently reordered or
paraphrased cases test reading meaning, rather than lexical match with a fixture
key. A stale realization, generation/authorization failure, selection failure and
genuinely absent information are distinct outcomes.

The current experience requirements are [Reader experience design](reader-experience-design.md),
D-01–D-37 / AC-01–AC-35. One right-aligned navigation includes precisely Overview,
Repository explanation and Work. The brand is left;
read-only status belongs outside navigation and describes the actual surface.
Overview shows PROJECT OVERVIEW, the actual canonical Project name, canonical
Purpose or short absence, Recent Work and All Work. It has no duplicate repository
summary, RECENT WORK eyebrow or Project Decision list. Unassociated Decisions, administrative inspection and protected operations
remain available through existing CLI/MCP/Operations, outside Viewer.

Normal Work cards require a host-authored name and problem/actual-approach/outcome
synopsis, exact lifecycle and separate creation/latest-record event bases. No
renderer clipping or copied Goal supplies generated wording. Missing content is
explicitly incomplete; permitted original Goal/facts remain behind inspection.
A selected document connects the same Work's problem, mechanism, result, Decisions
and limits in reader order. Expected effect is distinct from observed result.
The original Goal, independent verification/review/acceptance and complete history
retain authority in native evidence disclosures. Reading never generates.

Each Source citation opens a claim-specific Source point beside the document on
wide screens and across the viewport on narrow screens. Its exact claim quotation,
Source identity, observed file coordinates and original bytes are preserved, with
truthful bounded/before-state notices and a return to the cited paragraph. Native
fragments provide script-free offline navigation; browsers without relational CSS
selectors retain the inline appendix. Offline publication contains the same page
components, selecting one screen at a time through fragments rather than stacking
all routes into Overview. Evidence remains available without a running Runtime.
Live pages add only a bundled presentation script for disclosure opening, origin
scroll/focus restoration, Escape and narrow-panel keyboard containment. The exact
script digest is the sole HTTP script grant. It performs no requests, persistence,
generation, retention renewal or domain-state writes. Interpretation language and
canonical/source authority are independent of UI locale. Repository explanation
CLI reads pass the selected fixed locale to deterministic facts while selecting
generated prose by its exact requested language. Locale changes neither
publication identity nor Source lifetime, and cannot supply a missing language.
The current route/CLI/
snapshot contract is maintained in the [Viewer README](../../crates/volicord-viewer/README.md).

Shared projection selection has three meanings: `LatestWork` resolves the actual
Goal identity associated with the latest Checkpoint, or the newest Goal when no
Checkpoint exists; `ExactWork(ContextItemId)` validates an existing Goal
in the selected Project; `Repository` reads repository scope without selecting a
Work. Latest selection preserves an unassociated latest Checkpoint as unresolved,
never attaching it by chronology. An exact missing, forgotten, non-Goal or foreign
identity is not-found; malformed identity input is a distinct invalid-selection
error. Neither error falls back to LatestWork. These are read-side selectors, not
canonical entities, HTTP view names, approval states or persistence formats.

Exact selection precedes **every** display bound. The upstream canonical reader
must supply complete Project history; filtering a globally bounded timeline,
Resume Brief or parent Work list is insufficient. Resolve same-Work Checkpoints,
explicit WorkItem Decisions and Checkpoint-applied Decisions, Goal and Source
seeds, and actual snapshot topology before bounded presentation. Keep ProjectWide
and Unresolved scope labels even when a Checkpoint applies such a Decision. Do
not use an unrelated Decision or Goal as a topology seed. An explicitly selected
Work remains available separately from bounded lists. Goal-only Work is open and
readable; no code seeds produce an explicit code gap. Earlier changed paths remain
basis after a same-Work verification-only Checkpoint.

Derived reading records separate original text from display text and exact
record kind/identity/revision/field, Source, Repository and Analysis Snapshot
basis. Whole quotations, labeled excerpts and deterministic structured-fact
explanations are different representations. Excerpts report exact omitted UTF-8
bytes and characters and do not claim semantic comprehension. Missing interpretation produces an explicit unavailable answer and supported structured facts, never a fabricated result/rationale. Hash-like
substrings are preserved; a first sentence is not automatically a result.
User selection and rationale, agent recommendation and rationale, related code,
and implementation evidence remain independently inspectable. Missing user
rationale is not supplied by recommendation rationale. Original record language
is preserved; fixed English/Korean explanatory labels do not attest that quoted
source text was translated or that arbitrary requested-language realization ran.
Document NarrativeRealization is not a Work-summary realizer.

Work, verification, review and acceptance are independently sourced dimensions.
Each history observation retains its Checkpoint revision and Source basis; latest
state does not erase failed/rejected/unverified historical states. Earlier passed
verification is historical evidence and never silently covers later changes.
Ordinary shared Work answers distinguish independently selected current facts
(`VerificationState`, `UserReview`, `UserAcceptance`) from one optional
`HistoricalAdversity` fact. The latter summarizes counts of prior failed-verification
and rejected-acceptance observations, with exact per-dimension Checkpoint/revision
evidence keys. Exclude the selected observation for each dimension independently;
a later empty verification array does not turn the still-selected failure into
history. Counts refer to observations, not unique defects, test executions or
proof of recovery. No adverse prior observation means no historical summary.
Prior history does not assert a current blocker or independently prove resolution.
Current failures, rejections, unverified states, Source gaps and verification
coverage limitations remain ordinary, including compact Work cards.

Exact state arrays, event chronology, Unix-microsecond observation times,
Checkpoint identities/revisions and Source/outcome provenance remain unchanged in
`WorkReading.states` and CLI/MCP `selected_work.evidence.states`. Viewer live and
offline Work details expose every full observation in a closed native
`details.work-state-history[data-reading-role="audit-history"]` disclosure.
English/Korean labels carry equivalent ordinary meaning; audit preserves original
values. Shared ordinary answer text, including document and CLI consumers, contains
the concise history summary rather than repeated `AdverseObservation` event logs.
Classification uses exact structured state and independently selected observation
identity, never display-language prefixes. No new primary timestamp or time-format
contract is needed. Canonical selection, state transitions and evidence retention
are unchanged.
Availability, freshness, supersession, review-due and exact omissions remain
visible independently of prose shortening. All reads are mutation-free and may
use canonical-only remainder when analysis is unavailable. No GET analysis or
provider call, new Viewer store or canonical state is required.

Current shared Recall and four-document default behavior remain supported through
one projection implementation. The Viewer uses the same selector implementation.

### Implemented Work reading interfaces

`volicord-projections` exports `WorkSelector::{LatestWork, ExactWork(ContextItemId),
Repository}`, `WorkSelection { selector, work_item_id, basis }` and
`WorkSelectionBasis`. Exact parsing is `WorkSelector::exact(&str)` and accepts
exactly 32 ASCII hexadecimal digits. `WorkSelectionError::InvalidIdentity`
distinguishes malformed text from `WorkNotFound { project_id, work_item_id }`.
`build_project_projection(ProjectProjectionInputs)` now returns
`Result<ProjectProjection, WorkSelectionError>`; inputs include `selection`.
Latest resolves by `(recorded_at, identity)`, retains an unassociated latest
Checkpoint as `UnassociatedCheckpoint`, and uses `LatestGoal` only when no
Checkpoint exists. No available Goal gives `NoWork`. Exact basis retains Goal
revision; latest-Checkpoint basis retains Checkpoint identity and revision.

Local Operations exposes `project_projection_selected(project_id, WorkSelector)`
and `project_projection_selected_profiled`, the latter returning the projection
and existing `ProjectProjectionProfile`. Its error offers `work_selection_cause()`.
Existing `project_projection` and its profiled form use `LatestWork` through the
same implementation. Viewer adapters map their typed views to these selectors.

The Store reader supplies complete canonical Checkpoint history without a SQL
display limit. Operations validates the selection before analysis and Candidate
reads. Exact scope retains all same-Work Checkpoints, explicit WorkItem and
Checkpoint-applied Decisions, including superseded history, before Resume/list
bounds. Goal/source/path/component seeds select topology from the complete
selected Analysis Snapshot graph before graph bounds. Operations loads its
existing latest Analysis Snapshot per Project; this is not historical-snapshot
selection or an analysis refresh. Related code remains evidence of scope overlap,
not proof that a Decision was implemented.

`ProjectProjection` and `ProjectUnderstanding` expose `selection`, `selected_work`
and `selected_work_decisions` independently of bounded parent lists.
`ProjectProjection.work_overview` classifies the complete history before each
independent category bound (eight), ordered by relevant observation/result time
with Goal identity only as a tie-break. `WorkSection` retains complete/total/items/
omitted; no catalog page participates. `ProjectUnderstanding.work_overview` applies
its own tighter section bound while preserving totals. `WorkAnswers` selects
nonblank recorded result and its observation time, latest state, latest nonempty
verification, review and acceptance independently with exact Checkpoint revisions.
A subsequent nonblank change, changed path or changed Source is a conservative
verification coverage boundary. Blank state_change authoring is rejected by the
current canonical writer; null history is supported and does not erase a result.
The replaced renderer `changes.last()`/`states.last()` selections and the
Understanding ID-sorted paginated-catalog categorization are removed. History
vectors are evidence only.

`ProjectProjection.work_history` aggregates complete history before its list bound;
the selected Work's reading retains all its observations and quotations. Global
Checkpoint timeline presentation keeps the recent suffix with exact omissions.
Unassociated/unknown-Goal Checkpoints and unresolved Decisions are grouped from
complete canonical input before their own bound. Work code links retain exact
`CurrentWorkPathBasis { checkpoint_id, checkpoint_revision, path }` pairs, avoiding
invented combinations between path and Checkpoint lists.

`UnderstandingWork.reading: WorkReading` retains independently selected evidence
(`answers`), Goal, original changes, next step, state observations and code basis.
`ReadingText` is a quotation/evidence DTO with exact revision, field, Sources,
status, actor/observer and snapshot binding. Its optional 384-character excerpt is
used only for labeled quotation inspection and navigation; it does not supply an
ordinary result or rationale. The semantic-summary flag, redundant status text and
flattened historical verification/change vectors have been removed.

`work_answers` and `decision_answers` produce the sole shared `QuestionAnswers`
ordinary-reading contract: generated question-specific prose, deterministic facts,
availability/diagnostic and exact provenance. State labels use `FixedLocale`;
generated prose selects the exact requested language. Goal-only Open is derived,
no verification record differs from NotRun, failed/rejected observations stay visible,
and historical checks do not establish coverage of later changes. Decision user
rationale and agent recommendation remain separate; ordinary facts explicitly label
user choice, agent recommendation, declared consequences and Decision state; missing user rationale is never
supplied from recommendation. Declared Work/Project scope does not establish wider
applicability. Original text is explicit supporting evidence.

Overview categories, Work/Decision detail, offline snapshot, all four documents,
CLI status/decisions/Recall and MCP Recall/repository_understanding use these answers.
CLI/MCP JSON has `answers` and separate `evidence`; human CLI prints answer paragraphs
and facts. Recall retains bounded typed Checkpoint evidence for host resumption. Its selected Work
uses the common LatestWork selector over complete canonical history: the latest
Checkpoint’s explicit Work association, or a latest Goal when no Checkpoint exists.
An unassociated/unknown latest Checkpoint never falls back to an older Work or prose.
The shared `QuestionAnswers::next_step_answer` selects the `RecordedNextStep` fact
(or the distinct `NextStepAvailability` gap), never generated `NextStep` prose or
`ExplanationAvailability`. Its `recorded_action` contains exact untruncated
`recorded_text`, Work identity, latest same-Work Checkpoint identity/revision/field,
Source identities and availability/freshness/snapshot status. The fact's evidence
key is `checkpoint:ID@REV:next_step`; it exists without generated provenance.
Ordinary reading labels this text as a recorded quotation in its original language;
fixed en/ko labels do not translate it. Recall's top-level `next_step` is the exact
`recorded_next_action().recorded_text`, or null when no action is recorded.
Current generated `NextStep` paragraphs remain separately labeled interpretations
with generated provenance and evidence keys. They never override the canonical
direction. Explanation repair stays in `ExplanationAvailability`.
Viewer next-step section headings explicitly identify host interpretation. Its empty
Overview direction notice states only that no Checkpoint next action is recorded;
this does not establish whether task instructions or further suggestions exist.
An observed task-file instruction retains Source-content authority, not an inferred
author or a fabricated Checkpoint action. No original explanation wording or binding
is rewritten by these display labels.

Blank/missing latest Checkpoint direction produces `NextStepAvailability`, without
inheriting an older action; Goal-only Work identifies the absence of a Checkpoint.
Missing selected Work yields no action. Unavailable/stale Source support remains
explicit historical support status alongside a surviving canonical recorded action,
not current repository proof. Forgotten canonical records are excluded upstream;
no quoted Source body or deleted action is reconstructed from generated content.
Corrupt, unsupported, absent and stale interpretations preserve this recorded fact
and their separate repair diagnostic. Freshness checks remain strict, including
punctuation-only canonical corrections. Whole-field transport omissions must carry
the typed byte-budget reason and enclosing field/identity scope; they cannot stand
in for the minimum Work identity, direction and basis in restart verification.
Code questions continue to use source-grounded entity/relation explanations and
real graph identities, without generated ownership or runtime-flow claims.
Documents tag sections Reading or Evidence; raw Checkpoint/Decision originals use
closed disclosure in Markdown/HTML. Generated answer text is never silently clipped
into a purported complete answer; total-size failure returns no publication artifact.
Snapshots include the union of the catalog and Overview Work targets from one
projection. Request-specific graph materialization and exact omission states remain.

### Shared explanation lifecycle

`prepare_explanation(CanonicalReadBasis, ExplanationSubject, language)` produces
`ExplanationPlan` for `work_outcome`, independently of document NarrativePlan.
`LocalOperations::{prepare_explanation, record_explanation,
delete_explanations}` expose the lifecycle. Public CLI entry points are
`work explain prepare`, `work explain record --input FILE`, and `work explain delete`;
prepare/record require `--work` and accept `--language` (default `en`); delete requires `--work`.
They use the usual explicit Project/runtime or bound-repository resolution.

Preparation contains full selected result prose, Goal, next step, separately selected
state/verification/review/acceptance, historical observations, same-Work limits,
Project Purpose, canonical contradiction/supersession relations and immutable Source
observations. Each evidence key identifies record kind, identity, revision, field and
Sources; Source entries preserve snapshot, availability, freshness, actor and observer.
Current-host user-turn observation metadata contains only kind/host/session and
policy-withheld state; raw turn text is never copied into a payload Debug string.
Only independently selected canonical fields supply explanatory user facts.
Preparation also exposes `question_grounding`: the subject's permitted reader roles, each with exact
`required_any_evidence_keys`. Every authored unit needs nonempty prepared canonical
keys; a role with listed keys must include at least one of them. Roles are optional;
unsupported sections are omitted rather than filled with absence prose. The same selection drives validation:
Work purpose uses `goal`, reported change/effect uses `result` or same-Work `change:*`,
verification uses `verification`, and next step uses `next_step`. Without result or
verification, that question falls back to `goal` only to describe the request and
the explicitly unrecorded canonical state. Decision questions retain their own keys.
Optional limits still requires relevant prepared evidence. These relationships are
structural admission requirements, never automatic semantic support for arbitrary prose.

Canonical Work meaning and independently retrieved code/command evidence are separate.
Source citations supplement canonical keys and cannot replace them. Select further
relevant `course:*`, `change_scope:*`, `history:*` or `limits:*` evidence for the
actual assertion and chronology. Do not mechanically attach `goal` to technical
paragraphs. A Source-only technical paragraph has no faithful standalone representation
under this contract: integrate it with a genuinely supported canonical statement
explaining its Work relevance, or report that limitation. Do not fabricate a report,
verification or canonical key to admit it. Work advice belongs as qualified advice
within `next_step` or `limits`; `recommendation` is a Decision question only.
An unavailable Checkpoint action does not establish absent task instructions or
future work, and a Source path does not establish authorship. Host-level provider
authorization is outside canonical preparation; missing plan evidence establishes
neither authorization nor its absence. Task-file instructions do not supply consent.
The preparation is bounded at 131,072 bytes and fails rather than silently truncating
source material. Its SHA-256 fingerprints these inputs, scope, question, instructions
and exact language. Unrelated Work/catalog insertion does not change this basis.

The current active host interprets this plan under its existing interaction authority.
There is no production deterministic free-prose engine: the audit-heavy source cases
showed that truthful first-384-character quotations do not answer the task, and the
canonical fields do not separately encode feature purpose or expected effect.
Inventing source-specific lexical rules would not generalize to the reordered variant
and independent export case. Deterministic selection and state explanations remain
appropriate for structured facts; explicit host realization supplies Work prose.

`ExplanationRealization` uses exact-current `volicord_explanation` version 6,
plan fingerprint, language, host/session and nullable agent/model provenance, and
question/text/evidence-key/citation units in reader-selected order. A document has
1..32 grounded units; heading and synopsis are optional singletons. Purpose,
reported change, expected effect, verification, next step and limits are optional
roles subject to their prepared grounding rules. Observation, comparison and
reading-point units require exact Source citations.
Recording recomputes the plan under the existing mutation lock, rejects stale/foreign
keys or mismatched language/version, and bounds the response at its prepared capacity,
at most 32,768 bytes. This
validates structure and grounding references, **not** prose entailment, translation
quality, model identity, authorship or implementation success. All generator identity
is `self_reported_not_independently_verified`; unknown model stays null.

The existing Privacy managed `CachedSummary` store retains disposable Derived content
under exact Project/subject/language purpose. `RetainedExplanation` preserves subject,
question, evidence revisions/fields/Sources, snapshot/status, conflicts, fingerprint,
recording time and generator status without duplicating original evidence text.
Canonical links cover every used record and Source. No canonical schema, Viewer
database, provider invocation, background opt-in or adoption authority is introduced.

#### Independent Work progress publications

`WorkName` selects only the exact Goal statement/revision/Sources. `WorkSummary`
selects fields before article/history materialization, within the same Project/Work,
ordered by `(recorded_at, checkpoint_id)`. It never receives article prose, Decisions,
Project Purpose or complete chronological evidence as a substitute for progress.

| Key | Selection and exact field | Meaning / absence |
| --- | --- | --- |
| `goal` | Current Goal `statement` | Request, never achieved outcome |
| `result` | Latest nonblank `state_change` | Reported approach/result; later blanks do not erase it; absent remains unreported |
| `latest_state` | Latest Checkpoint `work_state` | Independent lifecycle, never inferred from checks; Goal-only is Open |
| `verification` | Latest nonempty `verification` | Individual recorded facts/outcomes and their own Sources; no command body needed |
| `verification_coverage` | Latest meaningful change after selected verification: `state_change,changed_paths,changed_source_basis` | Historical verification after change even with blank report; absence is no proof of coverage |
| `review`, `acceptance` | Latest Checkpoint `user_review`, `user_acceptance`, independently | Preserve explicit NotRequested reset and rejection; no inherited approval |
| `limits` | Latest Checkpoint `known_limits,non_goals` | Current qualifications, including empty replacement |
| `result_limits` | Selected report's `known_limits,non_goals`, when nonempty and distinct from latest | Do not drop the earlier report's qualifications when later progress is recorded |
| `next_step` | Latest Checkpoint `next_step` | Current direction; blank never falls back to an older direction |

Each key preserves exact record identity/revision/field and only that field's Source
basis. Verification coverage uses the shared reported-change predicate, including
changed paths or changed Sources; it does not infer execution or assertions from
paths. Selected revisions, content, field Sources and relevant conflicts/status enter
the summary fingerprint, even when a new Checkpoint has no report. A correction,
forgetting, material state/check/review/reset/limit/direction change requires explicit
fresh preparation and admission. Earlier unselected history and unrelated article
Sources do not become dependencies. The single synopsis must reference Goal and all
selected qualification keys. This establishes structural grounding, not prose truth;
authoring must keep material qualifications and the missing report meaningful.
Record-only facts survive disposal of uncited command/file bodies. Actual Source
citations independently retain all currentness, expiry, deletion and forgetting
consequences. Replacement is exact Project/Work/scope/language, with independent
name/summary/article validity. Reads and state changes never initiate generation.

#### Explanation byte contract

All dimensions are UTF-8 bytes, including JSON string escaping. Character count,
raw-file length and compact realization length are different measurements.

| Boundary | Inclusive limit | Measurement / action |
| --- | ---: | --- |
| Preparation | 131,072 | Complete compact `ExplanationPlan`, including its final fingerprint and retention budget; reject unrepresentable evidence before generation |
| Response | 40,960 maximum | Complete compact `ExplanationRealization`, including every Source/claim citation; actual admission uses the plan's `response_byte_capacity` |
| CLI / Dogfood input file | 65,536 | Raw JSON bytes, including formatting; compact-response validation still applies after parsing; remove whitespace for transport excess |
| Retained content / decoder | 147,456 | Complete compact `RetainedExplanation` in managed Derived `content`; same limit at record, Privacy body admission and explanation decoding; inspection preserves accepted bodies |
| Privacy short fields | 16,384 | Purpose, retention basis and unrelated existing short metadata retain their independent bounds |

The retained bound remains 147,456 bytes independently of the response ceiling;
neither Privacy short fields nor Source body bounds expand. Citation metadata can
use available envelope space instead of displacing a short grounded explanation.
Preparation publishes mandatory
`retention_budget` with `response_byte_limit`, `retained_byte_limit`,
`metadata_byte_reserve` and `response_byte_capacity`. The reserve is measured by
serializing the same retained representation used for recording, subtracting the
serialized realization, with the longest possible signed i64 recording timestamp
(20 bytes). It includes the complete subject/question, evidence identity/revision/
field/Source lists, Source status, conflicts, JSON keys/escaping and recorder-assigned
identity limitation. Only original evidence bodies and Source `observation` are
excluded, as already required by retention; grounding is never pruned.

An admitted plan still guarantees the **full** 16,384-byte response. Its exact
capacity is `min(40,960, 147,456 - metadata_byte_reserve)`, so previously supported
metadata-heavy plans retain at least their guaranteed capacity without pruning
grounding. If required metadata plus the guaranteed response cannot fit,
preparation rejects the evidence shape
with measured/reserved and allowed sizes before requesting host generation. Metadata
alone exceeding the envelope is the same pre-generation insufficiency. Seek Product
support for an unrepresentable basis; repeated shorten-and-retry cannot repair it.
Record recomputes the current plan and budget under the mutation lock, validates the
exact fingerprint/language/grounding, then measures the actual complete envelope
before opening/writing storage. Size fit never admits a changed/stale plan.
Individual paragraphs keep the MCP limit of 16,384 characters on both CLI and MCP.
The budget is part of the preparation fingerprint: a resource-policy change makes
older cached interpretations stale without deleting them or changing canonical
records. Re-recording requires an explicit fresh preparation. A caller preserving
an actual host output must retain its original request, prose, Source selections
and failure, and disclose any purely mechanical fingerprint rebinding; that
rebinding is not another model invocation or semantic approval.

The 40 KiB realization policy follows feedback-guided revisions of two independently
reviewed real-repository articles, needing 35,732 and 40,138 compact bytes for missing responsibilities
and claim bindings. Their 32 KiB refusals remain preserved. This is a resource-policy
change, not a larger preparation, retained envelope, Source copy, citation page,
HTTP or MCP transport grant. Source-first quality and cost must still be assessed
on each actual publication; fitting the new ceiling establishes no semantic pass.

Shared answers and Viewer use the same decoder. CLI `status`/`decisions` and local
Operations inspection expose full retained provenance. Existing bounded CLI/MCP Recall
surfaces keep their separate 56 KiB brief / 80 KiB structured / 256 KiB total transport
bounds and typed whole-field/suffix omissions; an omitted field is not complete
explanation evidence. Use full local status/decisions or Viewer evidence disclosure
for that subject/language. `RecordedNextStep` remains independently selected from
canonical facts. Four documents preserve full generated answer text and fail total
publication rather than clip it (Markdown 3 MiB, HTML 8 MiB). Viewer HTTP's 64 KiB
request-body bound applies to mutation requests, not explanation read responses.
Dogfood retains up to 32 MiB process stdout / 1 MiB stderr; private review body bounds
remain 1 MiB per selected stage / 32 MiB per projection with explicit incompleteness.
Those aggregate bounds are not new per-explanation storage/decoder limits.
`repository_understanding` uses full shared answers and section cardinality bounds.
A current Repository article, including the scoped article read, uses the complete
256 KiB MCP result bound without Recall-style field omissions. Over-bound reads
report measured size and exact local CLI/Viewer reading alternatives; preparation
or regeneration is not recovery for a retained document. External host capture
limits remain separately observable and cannot be inferred from local decode success.

Reads select the latest locally inserted envelope per publication subject/language and recompute
its basis; tied/rollback UTC and random IDs cannot select an older output. Publication
uses the same selection. `ExplanationReading` distinguishes Current, Stale, Missing
(no recording in the requested language), Deleted, Withheld (canonical forgetting
barrier), Unavailable, Unsupported and
Corrupt; only Current carries displayable prose. New/corrected/forgotten evidence,
changed Source status or conflict relations hide old prose. Missing generation is
distinct from missing result evidence and dependency failure. Privacy forgetting
barriers and managed deletion remove linked content; explicit delete removes all
languages/history of this Work and sanitizes local storage. Lost cache can be explicitly
regenerated from available evidence; GET/navigation/snapshot export never generate.
Generation/recording failures remain separate operation failures and preserve the prior
permitted output. An awaiting-host request is neither prose nor a completed generation.

`WorkReading.explanations` and `BriefDecision.explanations` use one store and decoder.
The CLI adds `decision explain prepare/record/delete --decision ID`; Work entry points
retain `--work ID`. Decision plans contain exact choice, separate user and recommendation
rationale, displayed consequences and declared scope/assumptions/triggers/review basis.
They require `user_rationale`, `recommendation`, `consequences` and `applicability`
paragraphs, including an honest missing-rationale answer. Work requires purpose,
reported change, expected effect, verification and next step. There is no old-format
reader, alias or second store; unsupported disposable derived content must be deleted
and regenerated. Recording and Current reads verify each publication's whole retained binding,
not only a response hash.

Document metadata version 10 and Viewer publication carry exact explanation provenance
and a current-build canonical read equality token. Operations verifies canonical basis
and current retained provenance immediately before atomic no-replace publication under
the mutation lock. Correction/forget or explicit cache deletion invalidates prepared
artifacts; newly served document generation also checks current bindings. A previously
exported offline copy remains under the user's control. No GET/export generates prose,
invokes a provider, or acquires new transmission authority.

The fresh fixture/public CLI/browser reproduction is maintained in
`rebuild/crates/volicord-viewer/README.md`. Fake test responses exercise lifecycle only;
fresh active-host Work and Decision recordings with independent claims in two languages
establish this implemented path, not human comprehension or gate qualification.

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
- **behaviorally relevant user context:** authority, Question behavior, learning interruption 또는
  bounded work를 바꿀 수 있는 canonical Learning, Preference와 Constraint의 statement role,
  identity와 Source basis
- **active Decisions and rationale:** applicability가 맞는 Decision, chosen alternative identity,
  recommended alternative identity, user rationale, recommendation rationale, alternative별 expected
  consequence와 supersession state
- **current state and recent Checkpoint:** meaningful work state, recent change,
  verification, review/acceptance의 독립 상태
- **open Questions:** canonical identity/revision, current frontier/blocked distinction과
  what each answer unlocks
- **risks, assumptions and known limits:** statement role, Source basis와 review trigger
- **next meaningful step:** selected Work의 latest same-Work Checkpoint에 기록된 행동과
  identity/revision/Source basis; current generated interpretation과 explanation repair는 별도
- **sources, capability, freshness and omissions:** used Sources/snapshots, analysis
  capability/coverage, stale/unavailable/failed scope, omitted count와 reason

Brief는 사용자 판단, agent recommendation, observed fact, semantic result와 generated
interpretation을 구분한다. Source repository가 unavailable해도 goal, Decision과
Checkpoint를 제공하고 current code relation을 unavailable로 표시한다.

CLI와 interactive-host Recall은 같은 bounded Resume Brief의 minimum meaning을
전달한다. `goals`의 statement 목록에는 `goal_basis`의 identity/role/Source basis를
함께 제공하며, Decision rationale/review basis, Checkpoint의 verification·user review·
acceptance, Question frontier, risk/assumption, snapshot capability/coverage/freshness와
omission reason을 adapter에서 버리지 않는다. `used_sources` identity 목록의
`source_details`는 provenance와 availability/freshness를 제공하지만 raw user turn이나
repository body를 복제하지 않는다.

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

### Host-facing serialized Recall budget

MCP Recall result는 compact UTF-8 JSON의 `content` text와 `structuredContent`를
모두 포함해 **262,144 bytes (256 KiB)** 이하다. 관찰된 약 1 MiB host truncation
경계에 768 KiB headroom을 둔다. Shared CLI/MCP Resume Brief는 56 KiB,
Candidate Inspection의 compact learning resume는 15 KiB, workflow는 8 KiB를
사용하고 health/envelope를 포함한 structured payload는 80 KiB 이하다.
Compact JSON text의 escaping과 중복 structured content 비용도 이 상한에 포함된다.

동일 state에서는 identity/state/authority와 Goal, behavior Context, Decision,
Checkpoint/work/verification/review/acceptance, next action을 history와 repository
metadata보다 먼저 유지한다. 큰 object의 identity/state field를 먼저 보존하고
반복 array는 stable prefix 뒤 `transport_omission` report로 exact suffix count와
가능한 first omitted identity를 제공한다. 큰 field는 전체 field omission과 exact
JSON byte count를 제공한다. 문자열이나 record를 mid-value로 자르지 않는다.
이 report는 semantic item이나 complete authority가 아니며, enclosing Project/record/
Analysis identity와 field scope로 기존 canonical/candidate/repository inspection을
다시 읽는다. 비정상적으로 큰 현재 field도 의미를 추측한 축약문으로 대체하지 않는다.
CLI와 MCP는 같은 shared brief, omission과 expansion basis를 사용한다.

Naturalistic observer는 이 serializer contract의 exact whole-field byte marker,
same-object field-count marker와 stable array suffix marker를 구분한다. Field-count
marker는 같은 current DTO의 absent field와 일치하는 count만 설명하며 다른 ancestor나
sibling의 marker는 generated provenance의 subject omission 근거가 아니다. Present
Project/Work/subject, language, revision 또는 Source contradiction은 marker로 면제되지
않는다. 유효하게 생략된 subject와 결합된 grounding field는 marker 원문, field와 scope를
review evidence에 보존한 bounded indeterminate 관찰이다. 다른 visible grounding 검사는
계속 수행하며 omitted value를 prose나 later response/export에서 추론하지 않는다.
Generated prose adequacy는 계속 qualitative review다.


Learning resume는 Candidate Inspection의 content/forgetting boundary 안에 있는
전용 subprojection이다. Full `CandidateInspection`을 Recall에 복사하지 않는다.
Candidate/revision, Goal/baseline/discovery/review/dimension identity, current learning
state와 selected/delegated/skipped outcome, current response Source와 2,048-byte 이하의
whole current feedback만 노출한다. 더 큰 feedback은 전체 omission으로 표시한다.
Pending learning, newest observation, stable identity 순으로 최대 64개를 선택하고
count/byte omission과 withheld count를 구분한다. Full discovery graph, dimensions,
interaction review, alternative accounting, rounds와 executable artifacts는 기존
`candidate_inspect`/`learning_deliberation` detail 및 explicit Work Candidate Inspection
detail에만 남는다. 이 projection은
canonical Decision, permanent lesson 또는 새 learning authority를 만들지 않는다.

`candidate_inspect`와 `learning_deliberation` detail은 compact resume state와 별도로
`learning_explanation_basis`를 제공한다. 이 read-side basis는 problem, established facts,
actual alternatives와 technical consequences, affected code/design scope, Source/Analysis Snapshot,
selection outcome, latest rationale/feedback/recommendation과 remaining uncertainty를 보존한다.
Availability는 `available`, `degraded`, `unavailable` 중 하나이며 selection completion이나
workflow readiness에서 추론하지 않는다. Content forgetting/withholding은 explicit `unavailable`,
부분 grounding은 `degraded`이고, 어느 상태도 learning selection을 canonical authority 또는
generated interpretation으로 바꾸지 않는다. Host/agent는 이 material로 설명을 만들 수 있지만
projection은 arbitrary natural-language pedagogy의 품질을 보증하거나 score하지 않는다.

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

Command Source는 human-readable label과 exit/termination을 ordinary presentation에 사용하고,
machine correlation이 필요한 audit detail에서는 Volicord-derived invocation fingerprint를
Source identity와 함께 표시할 수 있다. Projection, Viewer와 generated document는 raw exact
command invocation/argv를 읽거나 표시하지 않으며 label을 fingerprint 대신 비교하지 않는다.

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
| `candidate_kind` | observation, hypothesis, semantic claim, Engineering Choice Discovery, Materiality Review, Learning Deliberation, Question/Checkpoint candidate 또는 promotion proposal kind |
| `origin_and_provenance` | actor/subsystem/session과 Source/snapshot/command/host/provider basis |
| `collection_scope` | 이 Candidate를 수집한 Project/session/source/operation scope |
| `creation_or_observation_basis` | created/observed time과 bounded evidence/request basis |
| `retention_or_expiry_state` | retention policy, expiry basis와 cleaned state |
| `promotion_disposition` | pending/retained, promoted, dismissed 또는 expired/retention-cleaned 상태와 result basis |
| `scope_opt_out_state` | 해당 collection scope의 current opt-out state와 effective basis |

Engineering Choice Discovery에는 exact Goal/baseline, choice identity, credible alternatives,
consequences, affected scope와 Source basis를 bounded content로 표시할 수 있다. Materiality
Review에는 learning participation, choice-to-dimension mapping, authority disposition과 독립
learning-value state를 표시한다. Learning Deliberation에는 problem/facts/alternatives,
current ordered state, user response/rationale, 이후 agent feedback/recommendation과 terminal
result를 표시하되 canonical Question/Decision으로 label하지 않는다. 최초 user response 전
projection은 agent recommendation을 포함하지 않는다.

Candidate Inspection read는 Candidate를 promote, correct, dismiss, expire, delete 또는
reinterpret하지 않는다. Projection access/omission도 retention clock, promotion
authorization 또는 opt-out state를 바꾸지 않는다. Mutation은 각각의 explicit domain,
privacy 또는 lifecycle operation으로 분리한다.

Candidate source/body가 privacy boundary로 unavailable하거나 일부 Candidate read가
실패하면 available metadata와 affected scope를 `partial`/`degraded`로 표시한다.
Inspection failure는 projection degradation일 뿐 Candidate나 canonical record를
promote, delete, rewrite 또는 reinterpret하지 않는다. Direct scoped inspection이나
later retry path를 제공하되 hidden cache를 더 높은 authority로 사용하지 않는다.
Candidate dependency read 자체가 실패하면 `unavailable`, `unsupported`, `corrupt`,
`repair_required` 또는 `failed` 원인과 `candidate_inspection` affected scope를 안정적으로
보존하고, 안전한 canonical 및 repository section은 계속 제공한다. Candidate가 없다는
empty success는 current Candidate dependency를 성공적으로 읽은 경우에만 표시한다.
Canonical forgetting cleanup의 read barrier가 남아 있으면 usable Candidate metadata가
있더라도 projection은 `repair_required` degradation과 content omission을 함께 표시한다.

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

### Diagram grounding

Component, dependency, request/data flow, Decision impact처럼 관계가 본질적인 설명은
관계와 흐름을 이해하는 데 필요한 flowchart/UML 도식을 제공해야 한다. 모든 문서에
같은 도식을 강제하지 않으며 채택 여부를 다시 묻지 않는다. Diagram topology의 각 node와 edge는 inspectable
Repository/Analysis Snapshot의 Code Entity/Relation, manifest/config/document relation 또는
canonical Decision/Context relation에 연결된다.

- Structural/Semantic Fact에서 온 topology와 Agent Interpretation인 label,
  grouping, emphasis와 narrative를 구분한다.
- Generated model은 source relation에 없는 component, dependency, call 또는 flow edge를
  설명을 자연스럽게 만들기 위해 발명하지 않는다.
- Inferred grouping이 유용하면 topology edge가 아닌 explicit interpretation layer로
  표시하고 Source, uncertainty와 known gap을 제공한다.
- Unsupported, unavailable, failed 또는 stale scope에서 없는 edge를 채우지 않고
  gap으로 보여 준다.

## 7. Initial generated documents

첫 generated-document contract는 다음 네 유형을 포함한다.

### Project & Architecture Guide

Project goal, component와 boundary, repository structure, key flow, active architecture
Decisions, capability coverage와 known limits를 설명한다. Architecture claim마다 Source,
Structural/Semantic basis 또는 explicit inference marker가 필요하다.

### Decision Report

Question, displayed alternatives, Agent Recommendation, explicit user Decision/rationale,
applicability, assumptions, Source basis, expected consequence, revisit trigger와
supersession trail을 구분한다. Chosen alternative와 recommended alternative identity를 각각
보존하고 expected consequence는 해당 alternative에 연결한다. User rationale가 없으면
recommendation rationale로 채우지 않고 missing으로 표시한다.

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

Requested-language 성공은 title, prose, explanation, option/trade-off, caption과
diagram label 중 번역 가능한 generated body가 요청 언어로 실제 실현된 경우다.
Identifier, path, quoted source와 API name은 원문을 유지할 수 있다. Connected model이
요청 언어를 실현하지 못하거나 실현 여부를 확인할 수 없으면 result와
affected generated section을 `unavailable` 또는 `degraded` actionable outcome으로
표시한다. Requested language 메타데이터, HTML `lang` 또는 fixed-string locale만
맞춘 영어 본문을 requested-language success로 표시하지 않는다.

Active-host realization은 `requested_language`가 plan의 exact request와 같고
`all_generated_prose_realized`가 true라는 host attestation을 요구한다. Projection은 이
attestation만으로 prose 품질이나 언어를 추측하지 않고, 검증된 section/claim text 전체에서
계산한 body fingerprint와 plan fingerprint를 `host_realized` state에 함께 보존한다. 따라서
attestation은 다른 body나 language request에 재사용할 수 없다. False/missing attestation은
heading이나 metadata가 번역돼 있어도 requested-language success가 아니다.

Production host interface는 이 경계를 두 단계로 표현한다. 첫 단계의
`NarrativePlan`은 requested language, exact document/section/claim identity와 순서,
Source/Decision/Analysis grounding, source body와 번역하면 안 되는 code/path term을
하나의 fingerprint에 bind한다. Active host/model은 두 번째 단계에서 그 exact
identity별 realized text와 generator/agent/model identity를 반환한다. Projection owner는
fingerprint, topology, field bound와 protected term을 검증하고 grounding은 host payload가
아니라 plan에서 복사한다. Mismatch나 extra/missing claim은 failed realization이고,
realizer가 없는 caller는 typed `unavailable`을 받으며 requested-language success
artifact를 publish하지 않는다. 이 interaction은 current-host authority이며 background
semantic-provider opt-in이나 transport를 사용하지 않는다.

`NarrativePlan`은 authoritative typed projection의 크기를 transport 상한으로 바꾸지
않는다. 반복 path/component/affected-area material은 안정적인 원래 순서에서 대표 항목을
선택하고 exact omitted item count와 item/per-item bound를 source text에 기록한다. Checkpoint처럼
구조가 있는 source는 goal, work/state change, verification, review/acceptance, remaining work,
next step의 별도 grounded claim으로 먼저 나눈다. 그래도 한 source claim이 크면 가능한 마지막
문장·줄 경계까지의 bounded semantic excerpt를 명시적으로 표시하고, 나머지는
`exact_source_utf8_bytes`, `exact_source_character_count`, full-source digest가 있는 bounded
remainder로 표시한다. 의미가 일부만 전달됐음을 숨기거나 digest-only placeholder로 전체
semantic content를 대체하지 않는다. 따라서 같은 크기의 다른 source claim도 같은 plan으로
오인하지 않는다. Full typed claim과 Source/Decision/Analysis grounding은 그대로 남는다.
Public realization field 4,096-byte contract를 늘리지 않고 host realization을 위한
headroom을 남기기 위해 plan `source_text`는 3,072 UTF-8 bytes 이하다. Protected code/path
term도 deterministic representative set만 plan에 포함하며 term/item bound 때문에 제외된
수는 exact하게 기록한다. 이 bound와 omission metadata는 plan fingerprint에 포함된다.

### Active-host realization provenance in campaign evidence

Work/Decision explanation evidence uses the existing CLI `work explain
prepare/record --work ID` and `decision explain prepare/record --decision ID`.
It is independent of document NarrativeRealization; same-language explanations
may also require the authorized current host. Campaign collection retains the
before-generation shared answer, exact Product plan, submitted response, Product
record receipt and after-generation shared answer as separate immutable artifacts.
Project, subject, language, executable, revisions, Source/snapshot basis, times and
raw call/turn locators remain bound. A later generation never rewrites a measured
absent/stale/current answer. Steward generation is explicitly post-session and
does not establish earlier use, adoption or user experience. Explanation observation order is an append-only, hash-bound predecessor relation
within exact campaign candidate/executable, raw inputs, journey/Project, subject kind/identity,
requested language and evidence purpose. Each attempted Product preparation is declared
before invocation; IDs, file ordering and timestamps do not select publication. Every retained
completed lifecycle is validated against its own original plan, response, receipt and current-at-recording
readback. The unique chain tip is selected for final publication and alone must match a fresh
Product plan and the currently returned realization. Earlier completions remain historical without
rewriting their bytes or original current state. Repeated preparation on an unchanged basis still
creates a new publication obligation. Pending/failed preparation, failed recording and unrecorded
responses block readiness; no abandonment or success-fallback operation exists. A later success
cannot erase an earlier unresolved obligation. Document preparation freezes the selection; final
collection rechecks the same readiness and selection. Complete requested
explanation lifecycles before document realization and final document/Viewer
capture; no per-session generation quota applies. Model/host claims remain
self-reported or unknown; a record receipt proves local retention and structural
linkage, not authorship, translation quality or semantic adequacy.

Naturalistic identity observation distinguishes an empty Project, a new Goal/Work without a
meaningful Checkpoint, a Work with a Checkpoint, and an existing-Work resume. Project
initialization and observation-time record receipts establish these states independently
of the answer. Supported null Work/Checkpoint values are absence, rather than identity
contradictions. Unproved lifecycle/temporal state stays indeterminate; a later export cannot
make a future Work mandatory at an earlier read. Independently expected resume identity,
present wrong Project/Work/Goal/Checkpoint and conflicting visible Source evidence remain
hard findings. These observer rules do not change Product serialization or Work selection.

Naturalistic observation-time `LatestWork` follows the Product selector: the latest
applicable, independently ordered Checkpoint selects its associated Work; only established
Project-wide Checkpoint absence permits the newest Goal fallback. A new Goal without its
own Checkpoint does not prove Project-wide absence or override another Work's Checkpoint.
Missing request/publication chronology or prior-state coverage leaves selector expectation
unknown. Retained initialization, Goal and Checkpoint receipt coordinates establish the
basis; later final exports only corroborate exact immutable records. The basis retains
`latest_checkpoint`, `latest_goal`, `empty_project` or `unknown` separately from the
independently bound resume Work. Resume contradictions and known Checkpoint existence
remain checkable even when the latest selector is unknown.

Naturalistic Recall evidence checks Goal grounding at each observed read, using prior successful
creation/correction receipts and supported capture relationships rather than the first creation or
latest export for every read. Successful CAS correction advances the same identity's revision while
preserving original Goal Sources; its user authorization Source is separate. Failed/replayed/later
operations cannot manufacture a new historical basis. Missing or overlapping temporal evidence is
scoped indeterminate evidence; independent identity/action/typed-grounding contradictions remain
hard findings. Request/completion coordinates and completed handoff/fresh resolution/capture bounds
support temporal conclusions without certifying Source freshness, semantic equivalence or prose truth.
The global Goal list is bounded separately from selected Work. An exact `context_goal`/`bound`
omission for that Goal permits checking its selected-Work Goal reading basis against independently
witnessed identity, revision and supporting Sources. Missing selected basis stays indeterminate;
generic/foreign omission reports, duplicates and visible scope or grounding contradictions cannot
excuse an identity failure.

Phase 8 recorder는 preparation의 candidate/local MCP executable binding과 realizer의
identity claim을 구분한다. Verified preparation은 exact plan/route의 local evidence일 뿐
active-host authorship 또는 exact model identity의 attestation이 아니다. 현재 control path가
독립 검증하지 못하는 host/agent/model/session/runtime은 `self_reported` 또는 `unknown`으로 immutable하게
기록하고, 기존 Product generator metadata에도 unverified 상태를 명시한다. Arbitrary
non-empty model string이나 환경의 session ID만으로 verified model provenance를 만들지 않는다.
Unknown exact identity는 language failure가 아니다. Fingerprint, topology, protected-term,
field/language attestation 검사는 유지하며 provenance 검증이 prose quality classifier가 되지 않는다.
이 local MCP route hash는 campaign preparation의 닫힌 candidate-artifact binding에 있는
`volicord-mcp` hash와 동일해야 한다. Plan preparation, immutable realization record와 final
generation은 MCP 사용 전후에 그 binding을 다시 검증하며 같은 path의 replacement를 realized
document evidence로 publish하지 않는다.
Codex의 maintained rollout이 `session_meta` source/originator/session과 일관된
`turn_context.model`을 제공하면 mutable draft는 exact rollout SHA-256에 bound된
`runtime_observed` host/agent/model/session/runtime claim으로 갱신할 수 있다. Session은 rollout의
session identity를, runtime은 host-recorded Codex CLI version을 보존하며 generic model string과
합쳐 쓰지 않는다. Recorder는 preflight와 record
때 같은 rollout bytes를 다시 확인한다. 이 state는 host-recorded runtime observation이며
authorship attestation이 아니므로 Product metadata도 그 한계를 표시한다. Runtime observation과
충돌하는 self-report는 거부하고, exact metadata가 없으면 `unknown` 또는 명시적
`self_reported`를 유지한다. Model value에는 이름 allowlist를 적용하지 않는다.
Preparation은 plan별 bounded progress와 final publication 완료를 stderr의 machine-readable
event로 보고한다. 별도 read-only state inspection은 `not_prepared`, published preparation,
partially recorded와 fully recorded를 구분한다. Caller가 final publication 뒤 중단됐으면
inspection 결과에서 기존 draft의 validate/record로 계속하며 immutable preparation을 다시
publish하지 않는다. Publication 도중 불완전한 artifact 조합은 `repair_required`로 fail closed하고
완료된 publication으로 추정하지 않는다.

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

사람이 읽는 기본 경로는 문서 목적에 맞는 Goal, current Decision과 consequence,
work/verification state, open material Question 또는 blocker, next meaningful step과 material
gap을 먼저 제시한다. Opaque identity, hash, snapshot identity, 전체 capability inventory와
claim별 direct basis는 machine-readable `GeneratedDocument.body` grounding sidecar에 유지한다.
Markdown의 trailing audit appendix는 bounded metadata와 aggregate grounding count만 보여 주고,
HTML은 상세 claim basis를 기본 closed audit disclosure에 둔다. 본문 단순화는 typed grounding을 삭제하거나 failed,
unavailable, partial, stale, known-limit와 omission을 숨기는 근거가 아니다.

네 initial document는 stable Work identity에 연결된 bounded work summary를 공통으로
제공한다. Current/open/paused Work를 completed Work보다 먼저 두고 state, latest change,
next step, affected code와 verification을 사람이 읽을 수 있는 문장으로 설명한다.
Project & Architecture Guide의 current-work architecture 본문은 arbitrary repository inventory를
나열하지 않고 `ProjectUnderstanding`의 source-grounded deterministic explanation을 사용하며,
optional generated interpretation은 별도 claim class로 유지한다. Capability gap 본문은 coverage
요약과 우선순위가 높은 gap/issue 및 exact omission을 bounded하게 보여 주고, 전체 capability별
상태와 provenance는 typed grounding sidecar에 남긴다.

Current Decision으로 originating Question이 terminally answered/delegated된 경우, 그
Question을 열어 두었던 choice ambiguity는 current unresolved uncertainty로 표시하지 않는다.
그 ambiguity는 Decision의 historical displayed basis로 audit에서 inspect 가능하게 유지하고,
current assumption, known limit, revisit trigger, applicability review와 genuinely unresolved
Question은 계속 본문에 표시한다.

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
- Generated-content language request는 allowlist 없는 사용자 지시로 그대로 보존하고,
  HTML `lang` syntax metadata와 분리한다. `lang`은 보수적으로 검증·정규화한 language
  tag만 사용하며 요청을 tag로 표현할 수 없으면 bundled fixed locale의 안전한 `en` 또는
  `ko` fallback을 쓴다. Fallback은 generated-content language request를 변경하지 않는다.
- Syntax metadata fallback은 generated body 언어 fallback 허가가 아니다. Body
  realization이 불가능하면 명시적 `unavailable`/`degraded` outcome을 내고
  requested-language success artifact를 생성하지 않는다.
- Markdown/HTML renderer는 claim, diagnostic, name과 metadata의 각 동적 field에
  deterministic UTF-8 byte policy를 적용한다. Ordinary answer text는 complete하게 렌더링하고 total byte limit을 넘으면
  publication artifact 생성 전체를 실패시킨다. Audit-only metadata/diagnostic field는 전체 text 대신 exact omission marker를
  사용할 수 있다.
- Section별 claim 수, rendered metadata item 수와 per-field bound를 함께 적용해 output
  format별 deterministic total byte contract를 만든다. 이 contract는 authoritative typed
  projection이나 repository 전체 크기의 상한이 아니며, 더 깊은 inspection은 source
  projection을 다시 읽는다.
- Markdown 본문에는 opaque claim/source/Decision/analysis identity를 claim마다 interleave하지
  않는다. Trailing appendix는 compact grounding summary를 제공하고 상세 direct basis는 typed
  `GeneratedDocument.body` sidecar에 보존한다. HTML은 같은 body와 grounding을 사용하되 상세
  audit appendix를 closed `<details>`로 제공한다.
- PDF와 DOCX는 initial required output이 아니다.

Local Viewer는 필요한 current Project explanation projection과 bounded admitted Sources의 human-first
hierarchy를 하나의 self-contained read-only HTML snapshot으로 user-specified local
destination에 publish할 수 있다. 이 Viewer snapshot은 네 generated document를 대체하거나
그 metadata/adoption lifecycle을 공유하는 artifact가 아니다. Snapshot은 live listener,
mutation/Guarded/document-export form, authenticity token, live endpoint, JavaScript 또는
external runtime asset을 포함하지 않으며 생성 뒤 Runtime 없이 읽을 수 있다. Project,
canonical revision, Repository/Analysis Snapshot, generation time, degradation과 omission
basis는 ordinary reading path를 방해하지 않는 closed audit disclosure에서 inspect할 수 있다.
Snapshot 생성과 local publication은 background provider opt-in이나 external sharing/upload가
아니며 자동 network transmission을 수행하지 않는다.

Production Viewer의 first reading section은 `ProjectUnderstanding`의 Project Purpose,
completed/current/remaining work, next step, active Decision rationale와 affected code,
open material Question, risk/limit, architecture, interpretation과 evidence를 이 순서의
human explanation으로 구성한다. Verified canonical/structural/semantic layer와 generated
interpretation layer는 `data-statement-role`과 서로 다른 visual treatment로 구분한다.
Component/dependency와 flow figure는 self-contained accessible inline SVG이며 각 node는
Repository Intelligence entity, 각 edge는 resolved relation identity와 endpoint를
`data-entity-id`/`data-relation-id`로 보존한다. Narrative realization은 이 topology를
추가·삭제할 수 없다. Raw canonical rows, opaque identity와 exhaustive relation audit은
CLI/MCP/Operations 또는 맥락에 필요한 closed evidence disclosure에 남는다.

이 계약은 Markdown dialect, HTML renderer, template engine, CSS, sanitizer, viewer
framework 또는 conversion library를 선택하지 않는다. Output format은 canonical
portable-context bundle format이나 storage schema가 아니다.

## 12. Freshness, failure와 omission

Projection은 input별 current state를 보존한다.

- stale Source/analysis를 current evidence로 표시하지 않는다.
- unavailable repository에서는 canonical-only section을 계속 제공하고 code section의
  unavailable basis를 표시한다.
- 분석 cache가 corrupt, unsupported 또는 unreadable이면 safe canonical-only projection을
  유지하고 `derived_analysis` failure/omission과 실제 supported repair action을 표시한다.
  Read는 cache를 삭제하거나 재생성하지 않으며, 이전 cache를 current로 대신 사용하지 않는다.
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
- requested-language generated body의 실제 실현 또는 explicit
  `unavailable`/`degraded` outcome; metadata-only language match는 성공으로 취급하지 않음
- Project Understanding의 completed/current/remaining work, next step, Decision rationale,
  affected code, architecture/flow, evidence, gap, freshness와 uncertainty가 human-first
  body에 있고 audit detail은 더 깊은 inspection으로 남음
- Diagram node/edge topology가 inspectable repository/Decision relation에서 오고
  generated interpretation이 relation을 발명하지 않는 성질

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
- Viewer가 record inspection이 아닌 Project Understanding을 기본으로 제공하고,
  verified fact와 generated interpretation 및 source-grounded diagram을 이해할 수 있음
- 요청 언어별 generated body realization 성공 또는 explicit unavailable/degraded behavior
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

### Projection materialization resource boundary

Repository map의 topology 선택은 borrowed entity/relation identity, kind와 endpoint를
사용한다. Canonical-linked importance, connected endpoint retention, deterministic ordering과
omission accounting을 먼저 확정하고, 선택된 항목의 Source/range/freshness/uncertainty/
diagnostic payload만 MapEntity/MapRelation으로 복제한다. 출력 bound는 직렬화 크기뿐
아니라 이러한 rich projection payload의 생성 수에도 적용한다. 선택 전 lightweight
lookup/degree/ranking index의 메모리는 입력 graph 크기에 비례할 수 있다.

Recall의 AnalysisMetadata read는 snapshot identity, Source, inventory 기반 current
observation, capability/coverage/freshness만 decode한다. Graph payload의 schema 검증이나
graph가 usable하다는 주장은 하지 않는다. 유효한 metadata를 읽을 수 있으면 Canonical
Recall과 snapshot 근거는 유지할 수 있으며, graph consumer/health의 full payload 검증은
별도로 실패를 보고한다. Full-snapshot Recall과 metadata Recall은 같은 metadata와
freshness observation에 대해 동일한 ResumeBrief를 생성해야 한다. Metadata reader도
공통 Analysis Snapshot kind/current-version 및 typed identity 계약을 검사한다.


Work Overview는 complete canonical history의 Goal/Checkpoint 참조를 분류하고 state,
result chronology와 next-step eligibility 및 전체 category count를 먼저 확정한다.
Catalog page, 각 Overview section, exact selection과 Recall에 필요한 identity의 합집합만
상세 원문·state history·coverage evidence로 materialize한다. 선택된 Work의 전체 history는
생략하지 않는다. Canonical store read와 lightweight index는 전체 기록 크기에 비례하며,
이 비용을 표시 항목 bound로 숨기지 않는다. `WorkReadCost`는 projection에서 분류한 Work,
indexed/materialized Checkpoint, materialized Work와 evidence input byte 수를 구분한다.
Operations는 retained subject/language마다 한 번 수행하는 explanation freshness preparation
횟수를 별도로 보고한다. 이는 allocation/RSS 또는 model generation latency 측정이 아니다.

### Viewer detail selection

`ProjectionDetail` selects an optional Decision or code entity from the complete
Project canonical/analysis basis before list bounds. `ProjectProjection` carries
`selected_decision`, `selected_entity`, its bounded incoming/outgoing relations,
real neighbor entities and exact omitted relation count separately from parent
lists. Missing selected details on an available basis remain absent, letting the adapter
return not-found without falling back or revealing another Project. If the graph
basis is unavailable, requested entity identity is explicitly unverifiable; canonical
remainder remains readable without claiming entity existence or absence. Reads retain canonical identity,
revision, Source/range, evidence class, freshness and unresolved-target meaning.
Snapshot navigation allows only existing unambiguous internal fragments and native
`details`; omitted detail produces an explanation rather than a broken link.

### Requested-section materialization

`ProjectionReadRequirements { code, inspection }` separates read materialization
from identity selection. Both default to requested for full existing consumers.
`ProjectReadSections` reports `NotRequested`, `Available` or `Unavailable` for
code and inspection. Candidate dependency also has explicit `NotRequested`.
A thin read preserves canonical meanings and stored AnalysisMetadata coverage,
identity and freshness, but makes no graph-integrity or missing-code conclusion.
Selected Work code availability and Decision code-link gaps distinguish sections
not requested from requested data that could not be read. Requested missing or
failed graph input leaves canonical remainder readable with an affected-scope gap.

Viewer explanations and exact citation readers do not require a generic graph catalog.
Ordinary reading and snapshots do not generate documents, load privacy inventories
or run bulk health/diagnostics. Work-linked Candidate inspection remains explicitly
scoped to actual Work context; it does not restore a Project inspector. Full
document generation, narrative planning and realization reject any projection
whose code or inspection section is `NotRequested`. An `Unavailable` requested
section remains a legitimate degraded complete attempt and reports its gaps.
Recall's existing metadata read and ordinary full document defaults are preserved.

Local Operations owns selective reads and profiles. CLI doctor retains store/recovery
health and full integrity checking. A Viewer snapshot shares one canonical/analysis
metadata projection among bounded explanation sections, without document previews
or a bulk Code/diagnostic catalog. Per-Work presentation never reopens analysis.
These requirements add no persisted format, cache, analyzer invocation, provider
call, canonical mutation or weakened correction/forgetting invalidation.

Exact entity detail also focuses `ProjectUnderstanding.architecture` and its
relationship evidence on `selected_entity`, actual bounded neighbors and incident
relations. This read-side presentation selection precedes parent map bounds and
retains the selected entity through the downstream architecture bound. It leaves
canonical Work selection, `current_work_topology` seed semantics and default
Recall/document inputs unchanged. No unrelated repository components replace this
neighborhood. Resolved endpoints, unresolved evidence and exact omissions remain
separate, and an explicit entity selection is not a canonical Work-seed assertion.

Local Operations의 coordinated document read는 기존 inspection coordination lock을
canonical read 이전부터 네 preview의 grounding·output validation과 generation 완료까지
유지한다. 하나의 complete projection과 freshly matched retained explanation basis를 사용하며,
이 request 안에서 같은 canonical history를 재조회하거나 freshness plan을 재계산하지 않는다.
Lock, canonical/analysis read, projection과 document generation 비용은 각각 기존 profile
stage에 포함한다. 이미 만들어진 외부 projection의 document generation은 generation 후
별도 live-basis 검사를 유지한다. Publication도 계속 explicit Local Operations mutation
boundary에서 current basis를 별도로 검사한다. Request가 끝나면 lock과 근거는 재사용하지
않으며, persistent cache나 adapter-owned validity authority를 추가하지 않는다. Candidate
저장소 장애는 unrelated store의 private-path 준비 실패로 승격하지 않는다.
삭제·수정 이후 새 read/export의 stale text 차단과 네 complete preview는 유지한다.

Canonical read equality token은 full `CanonicalReadBasis`의 derived field-wise `Hash` 입력을
SHA-256으로 계산한다. 모든 Eq field, enum/sequence/field boundary, Source observation,
revision·relation·forgetting·merge history를 포함하며 Debug formatting/escaping을 수행하지
않는다. 이는 native/current-build 내부 equality binding이며 portable digest·authenticity·schema
version이 아니다. Token 값 변경은 canonical identity나 explanation plan fingerprint를 변경하지
않는다. 다른 build에서 생성된 준비 artifact는 다시 render한 뒤 current publication 검사를
통과해야 한다.


CLI `status`, `recall`, `decisions`의 ordinary text는 공통 answer prose와 독립적으로 선택된
기록 fact/state를 별도 label로 출력한다. Project Purpose와 active Question prompt는 계속
명시적인 canonical quotation/prompt consumer다. Root binding/revision, Source catalog,
snapshot envelope와 generator audit는 `--json` inspection에서 정확히 제공하며 기본 text에
함께 펼치지 않는다. Work grouping gap과 omitted counts는 기본 text에서 계속 보인다.
이 presentation 구분은 result/state selection, source meaning 또는 checksum 내용을 다시
해석하거나 삭제하는 renderer heuristic이 아니다.

Supporting Source availability와 freshness gap은 shared answer의 독립 fact로 표시한다.
Work는 Goal/Checkpoint/verification/review/acceptance의 전체 근거 Source를, Decision은
user와 recommendation의 근거 Source를 사용한다. `Current` freshness도 unavailable
Source를 usable로 만들지 않는다. Source identity/status의 상세값은 evidence inspection에
보존하며, ordinary gap은 기록된 보고를 현재 저장소 동작으로 승격하지 않는다.

### Live Viewer observation context

Live HTML supplies read-side context schema 1 in an escaped, non-executable meta
entry. It binds a fresh render ID to actual Linux executable/process start identity,
opaque Runtime, Project, parsed view/selected subject, fixed locale and requested
language, canonical read fingerprint, bounded Source-state hash, Analysis identities
and materialized explanation states. Current realizations additionally retain plan
fingerprint, recording time and realization hash; stale/unavailable content remains
withheld. No canonical record, provider request or host trust follows from this data.
Unavailable observation binding preserves normal reading with a fixed unavailable
marker and cannot establish captured evidence. The bounded subject list includes
materialized Overview inputs, not a claim that
all those subjects are visibly displayed. Snapshot export keeps its existing
self-contained deterministic contract and has no live process/render context.

The maintained [browser display capture](../../validation/dogfood/viewer-observation.md)
combines this renderer basis with actual stable DOM, URL, geometry and screenshot
hashes. It preserves separate before/after generation or mutation artifacts. A
browser result is supporting display evidence; direct human inspection and its
limits remain separately declared under the qualitative-review owner. Constructor
executable hashing, snapshot export, server/render profiling, browser PaintTiming
and human responsiveness are distinct costs and claims.

### Contextual answer limitations

`ProjectProjection.answer_capability_gaps` is selected before display bounds from
actual Analysis Snapshot inventory, selected Work paths/Source locators/code links,
selected Decision applicability or selected code entity. Work limitation selection
uses the same code-seed relevance as Work topology: active Decisions associated
with the selected Work contribute applicability paths, component matches and
explicit Decision-to-CodeEntity references before display bounds. Declared paths
remain relevant when failed analysis produces no entity. A Decision associated
only with another Work contributes neither code seeds nor contextual limitations.
A capability language name alone proves no relevance. A failed scope outside the
selected paths does not limit
that answer even when the language is the same. Repository-scoped reading considers
actual observed files rather than capabilities for absent languages. Scoped failures
retain state, affected areas, reason, usable remainder and user-visible consequence.
Each contextual gap preserves `freshness` independently of capability `state`; an
unavailable capability with unknown comparison evidence cannot imply currentness.
`answer_issues` separates selected Source and requested graph failures from exhaustive
`issues`/`repository_map.gaps`. Ordinary reads with code `NotRequested` have no missing
code-body warning; this is materialization intent, not an analyzer failure. Canonical
limits remain distinct. Viewer consumers keep contextual reliability visible ordinarily.
Code uses a scope-aware issue/gap summary beside its explanation, with capability
state and freshness counted independently of the supported meaning. Source or analysis
blockers remain visible without opening diagnostics. The primary supported Work/code
reading precedes a closed native full-diagnostic disclosure; absence of supported
meaning remains explicit. Expanded diagnostics preserve the selected producer order,
every identity, original scope, state, freshness, reason, consequence and usable remainder.
Root `.` coverage gets a readable repository-coverage label without changing identity
or relevance; retained affected-area metadata still describes the original report.
Summary counts match retained detail, and exact-entity, Work and repository bases stay
distinct. Live and offline consumers use the same components and limitation states.
Overview, Work and Decision warnings keep their existing ordinary presentation.
Repository/runtime diagnostics remain inspectable in evidence disclosures. Blocking canonical
runtime failure remains ordinary; unrelated auxiliary health is audit evidence.

`ProjectProjection.repository_analysis` and `ProjectUnderstanding.repository_analysis`
expose the bounded shared status to Viewer consumers; `volicord status --json`
exposes the same model. Viewer consumers render `state`, independently preserved
`freshness`, `coverage` plus `omitted_coverage_count`, `latest_attempt`,
`latest_attempt_error`, `retained_prior_result`, `diagnostic` and `refresh_command`.
Snapshot identities/generation time belong to evidence details. The command is text
guidance, not a supported Viewer analysis POST. Missing historical attempt receipts
cannot establish a latest-attempt success. No automatic analyzer execution or automatic
explanation regeneration is added by status reads or explicit analysis refresh.

### Explanation meaning and directed relationship evidence

Work preparation now carries complete same-Work `change:<Checkpoint>` report prose
and `change_scope:<Checkpoint>` paths/changed Sources in chronological evidence, in
addition to independently selected latest result/state/verification/review/acceptance.
A later verification/resume report therefore cannot erase the earlier change meaning
from interpretation input. The active host explains goal/problem, changed or investigated
behavior, supported before/after effect, actual verification and next meaningful limit;
paths, generic change prose and a completed state alone prove no runtime improvement.
Decision interpretation distinguishes actual user choice/delegation, alternative expected
consequences/trade-offs, agent recommendation basis, user reason only when present, and
scope/assumptions/revisit conditions. Recording still validates grounding structure,
not prose entailment; source-rich and absent-evidence controls remain independent.

Critical effect authoring retains Source-first conditions, distinct state objects,
implementations and necessary callee/callback effects, bounded non-effects and
static/reported/observed scope before comparing prose. Exact caller citations alone
do not establish completion, deletion or cancellation. Check shared critical claims
in every applicable language/revision against that Source basis, never another
language's wording. Changed Source or scope requires reassessment. The maintained
[reader assessment procedure](../../validation/end-to-end/multi-repository/repository-reader-assessment.md)
seals inputs and preserves attributed comparisons separately from mechanical admission
and human acceptance. This adds no durable Product schema or semantic classifier.
Repository introductions explain purpose and core record relationships before storage
and authority detail; Work introductions explain actual investigation/actions, reported
results, current state and limits before evaluation/field-authoring notes. Neither
ordering rule imposes section quotas or fabricates missing records.

Shared Repository Snapshot Source provenance binds observation identity but does not
seed every code entity into a Work. Work architecture seeds use bounded File/Symbol
locators, changed paths, explicit canonical links or declared component identity;
only actual incident relationships supply neighbors. Unrelated repository entities
remain inspectable in repository scope. `CodeRelationshipRole` separates containment,
dependency, syntactic call, symbol reference, type relationship and other evidence.
Dependencies, declaration/containment, symbol resolution and implementation are not
execution/data/control flow. Only exact `CallsSyntactically` relations supply the
current static call-flow diagram; unresolved targets retain their own evidence and
never become invented endpoints. `UnderstandingArchitecture.flow_evidence` reports
NotRequested, AnalysisUnavailable, NoResolvedCalls or SyntacticCalls, exact retained
relation identities and missing evidence. Static calls still cannot prove runtime
execution, dynamic dispatch, data/control flow or cross-process behavior. Missing
resolved calls produce an explicit scoped gap, not a disconnected flow node list.
Separately, grounded repository components without resolved relationships may
appear as disconnected architecture nodes, with exact identity/list grounding and
an explicit no-relationship gap. This depicts available structure and never
supplies call, execution or data-flow evidence.
An unrequested code section has no material missing-flow warning.

Agent-assisted availability belongs to analysis status/audit rather than contextual
analyzer limits: retained Work/Decision explanations use the canonical active-host
plan and do not consume a repository agent-assisted adapter. Underlying capability
reports remain inspectable and retain their actual state.

Ordinary `ExplanationProvenance.evidence` carries the exact identity, revision,
field and Sources of paragraph-cited preparation evidence. `uncited_evidence_count`
accounts for offered preparation references not used by that realization; the
retained explanation still preserves every offered reference and its fingerprint.
Original prose remains canonical evidence rather than duplicated provenance content.
The bounded MCP read prioritizes subject, fingerprint, self-reported generator status
and record grounding. Selected Work answers receive a larger section allowance within
the unchanged total response byte budget; excessive grounding still uses exact counted
transport omissions. These changes do not attest generated prose entailment.

### Complete publication reading

`explanation_read` exposes a read-time compact publication without changing
retained explanation v6. Exact Project/subject/language select a publication;
its managed identity, preparation and realization participate in the page
fingerprint. Ordered units retain text, question, evidence keys and exact claim
coordinates/hashes. Equal Source/range bindings share a table entry. Provenance
citations are the ordered concatenation of unit citations, not a duplicated list.
No authored section or evidence reference is dropped to fit a response.

Pages contain up to 8,192 UTF-8 bytes of compact JSON; units may span pages.
Consumers join chunks in `next_offset` order, check the same fingerprint and
decode only after explicit `complete`. Overall supported compact content is
1,048,576 bytes; excess is an explicit unsupported-size failure, never truncated
success. Existing retained/admission and 256 KiB MCP envelope limits are unchanged.
Every request revalidates all publication dependencies under the inspection lock.
Replacement (even equal wording), expiry, deletion, withholding, changed evidence
or a foreign selector reject continuation. Exact citation requests retain the
existing independently bounded Source reader and authority. This is nonmutating
transport completion, not semantic approval or an enduring currentness promise.

Within one Source validation call, Operations may share its current canonical
basis, privacy inspection and verified repository Analysis metadata across exact
Source reads. Each Source still passes its own association, availability, revision,
retention/forgetting, format, content and citation checks. These borrowed inputs
and decoded bodies are request-owned and dropped on return or failure; there is
no cross-request entry, eviction policy, durable cache or additional content owner.
Every page and citation request starts fresh authority/lifetime checks and the
existing full current repository observation, including uncited dependencies.
Previously returned bytes, canonical revision equality, timestamps or a TTL never
replace that observation. Sharing does not change compact bytes, continuation
identity, check-time meaning, Source retention or any transport/excerpt limit.

### Complete embedded code windows

The 32,768-byte shared excerpt budget selects whole cited windows. Each authored
unit offers its complete set of unique ranges as a bundle, visited in original
authored reading order. A fitting complete mechanism is not displaced just because
later supporting windows are smaller. Shared exact ranges consume one allocation;
an oversized bundle is skipped without reserving or clipping its windows.
Additional whole supporting windows may use remaining capacity in size/Source
identity order. No prose role (including Narrative versus ReadingPoint) grants
priority and no range is divided
into an equal prefix to claim coverage. Live exact citation pages keep their
separate 4,096-byte limit and can reach any remaining permitted range.

This is a read-time aggregate, with the existing 16,384-byte maximum per embedded
window. It does not increase Source observation, preparation, realization, retained
metadata, HTTP or MCP transport limits. Two actual multi-operation publications
require 32,435 and 23,051 distinct cited-window bytes; even perfect overlap sharing
would require 31,593 and 20,943 bytes. At 16,384 bytes their complete authored scope
lost 9 and 5 windows, including browser command delivery and publication validation.
Exact shared ranges were already deduplicated; clipping windows or removing essential
claims would not meet that reading obligation. The scoped aggregate now fits both
without reauthoring or changing their admitted identities. Above-bound content still
has explicit omissions; no automatic full-source embedding is promised. Measurements
and alternatives are recorded in the validation report linked by Validation.

`embedded_source_scope` names original unit indices and included/omitted citation
indices. A complete code-reading claim requires all its ranges in full. Reader
and static export expose this declared scope; offline omissions cannot promise
live retrieval or current comparison. Source identity and exact UTF-8/line/hash
coordinates come from the Source owner. Inclusion establishes available code,
not importance, entailment or complete repository understanding. Export creation
continues to revalidate every used publication and Source dependency.

### Typed explanation applicability

Shared reading data carries the explained Repository/Analysis/Source snapshots,
known generation time, actual read comparison time, repository comparison state
and a typed limitation supplied by Operations/Source lifecycle owners. Verified
comparison is scoped to that read; record-only publications establish no current
repository comparison. Missing, changed, expired, deleted, withheld, unavailable
comparison, corrupt and unsupported content remain distinct. Unknown detail is
left unknown, never inferred from a diagnostic substring. A failed dependency
withholds the affected article while independent name/summary and canonical facts
keep their own authority.

A confirmed repository identity mismatch during preparation is `stale` with
comparison `changed`, just as a mismatch during later Source validation is. It
must not be reported as an unavailable comparison. A missing repository remains
`unavailable` with `comparison_unavailable`; unknown checks remain `not_checked`.
The reason is carried by the dependency owner, not parsed from diagnostics.

Dynamic comparison times live outside compact publication fingerprints, so a
continuation preserves stable publication identity while every request rechecks
authority. Live citation failures return typed reasons without Source bytes.
Static rendering describes the comparison used at export creation and exposes
the known export basis; it promises neither live retrieval nor continuing checks.
Generation, comparison and export times are distinct. Raw diagnostics remain
secondary inspection. Source actions display path/range and keep original
claim/citation coordinates; closing inspection restores the exact activating
control where it remains present, with the claim as the deep-link fallback.

### Revision-bound retained evidence detail

Bounded MCP omissions do not grant raw-history or filesystem read authority.
The existing `canonical_inspect` accepts exact `record_kind`, `record_id`,
`revision` and an allowlisted `field`; optional `work_item_id` must match the
record's explicit Work association. `candidate_inspect` accepts `candidate_id`,
`revision` and a retained Candidate section. Operations performs both reads under
its inspection coordination lock. No analysis refresh or provider call occurs.

Canonical manifest paging starts with `canonical_inspect { project_id, offset: 0 }`.
It enumerates all retained canonical identities/revisions and `detail_fields`,
including records omitted from projection lists. Subsequent manifest pages require
its returned `fingerprint` as `expected_fingerprint`. Changed manifests reject
continuation rather than shifting an offset to another record.

Detail output contains at most 2,048 UTF-8 bytes of compact JSON in `chunk`.
Consumers concatenate chunks and then decode JSON. Each continuation sends the
same Project/record/revision/field, returned `next_offset` and
`expected_fingerprint`. The fingerprint binds that identity, exact field value
and source/privacy metadata; changes reject continuation. `total_utf8_bytes` and
`remaining_utf8_bytes` describe this retained serialized field only. Neither
transport completion nor a successful detail read changes the original Recall
omission count or establishes complete/current Source evidence.

The current reader accepts the retained read model's exact revision and reports
`revision_unavailable` for older or changed references, without substituting a
current body. Forgotten/foreign/missing records return `record_unavailable`;
unrecorded optional fields return `field_not_recorded`. Candidate sections retain
their typed retained representation, independently of the compact inspection
summary's presentation. Source `observation` exposes bounded retained execution
and availability metadata; `body` cannot dereference a locator or recover an
unretained historical output. Structured 80 KiB and overall 256 KiB MCP budgets
remain unchanged and apply to both inspection tools.

Detail `metadata.retained_state` and per-Source availability/freshness remain
independent of chunk `state`. Forgotten Checkpoint Source witnesses mark surviving
facts as partially retained; they do not erase an authored outcome or reconstruct
a Source body. `source_status` is itself an allowlisted detail field when metadata
is bounded. Candidate content omission names match the existing inspection's
lowercase states, with read-time retention expiry and cleanup preserved. Original
excerpt `representation`, omitted characters/bytes, semantic omitted record counts
and transport suffix counts keep their own meanings across bounded serialization
and repeated reads; completing a detail field never resets any earlier omission.

### Work course evidence in explanations

Verification count labels use **Checkpoint verification reports across the selected
Work's history** as their unit. Each exact Checkpoint/revision/vector position is
one report, even when several reports cite the same Command Source or repeat the
same free-text test count. Shared answers and Viewer use the same report summary.
The public verification read DTO exposes `report_count_unit`, `report_count_scope`,
computed `report_counts` and `reported_test_count_basis: original_report_text_only`;
these are read-time labels/counts over `checks`, never new stored facts.
Executions remain separately identified Source observations. Explicitly reported
test counts stay attributed to the original report and its scope; the current
canonical facts contain no structured test-count field. Do not parse outcome or
assertion text into a canonical total, sum repetitions into unique tests, or infer
new observed runs, coverage, completion or acceptance. Unknown test counts remain
unknown. This display clarification adds no schema/version or verification event.

Work explanation preparation reads the complete exact-Goal Checkpoint history
before presentation bounds. `course:*` carries each recorded goal, boundary kind,
next action and open Question references; `limits:*` retains limits and non-goals; `change:*` preserves
full reported investigation/change prose independently of the latest result. A
later verification-only or resume report cannot erase earlier same-Work changes.
Reported-change/effect paragraphs may cite an earlier exact `change:*` rather than
a latest verification-only result. Preparation fingerprints include this selected
course and explicit missing Source state. Source locators do not recover historical
bodies. Missing investigation, rationale and unrecorded effects stay explicit.
Ordinary summaries show generated problem and outcome in the requested language;
recorded verification outcomes, including earlier passed/failed observations, stay
separate from current work state and user acceptance. These are recorded reports
and structural grounding checks, not deterministic prose entailment validation.

### Work-linked Learning inspection and Decision context

Decision readings retain the originating Question identity/revision separately
from user choice, user rationale, displayed recommendation, alternative consequences
and applicability. Explanation preparation includes the retained exact Question's
problem, established facts, trade-offs and uncertainty. A missing or different
Question revision is explicitly unavailable; a newer prompt cannot supply missing
historical context or user rationale. Relevant Work explanation evidence includes
only its explicitly scoped or Checkpoint-applied Decisions and their actual contexts.
Shared Work/Decision answers preserve these identities in Overview, detail, Recall
and document consumers without introducing a second retained explanation decoder.

Overview and Work detail request the existing Candidate Inspection dependency.
The ordinary Work link carries only an identity reference to that Work's permitted
Learning Deliberation; its explicit **Candidate Inspection** subsection shows the
retained problem, facts, alternatives/consequences, response, feedback, recommendation
and uncertainty with Session Candidate authority. Exact Work association is the
permitted Deliberation's `goal_context_id`, selected before general Candidate bounds;
equal title, path and chronology supply no association. CLI status and MCP Project
Understanding expose the same identity-only references inside `candidate_inspection`,
including revision and the existing `candidate_inspect` detail reference. Candidate
prose is not Work explanation evidence, a document claim or a canonical Decision.

Policy-withheld, expired, cleaned and forgetting-withheld bodies supply no Work
association. Their existing inspection omissions/dependency state remain explicit;
absence of a permitted association is not proof that no learning interaction occurred.
Read-time expiry also applies to the existing bounded Learning continuation section.
Pending/terminal ordering, opt-out semantics and retention are unchanged; no read
generates explanations, collects Candidates or changes canonical authority.

Learning association ranking borrows permitted Goal identity through the same content-admission rule as Candidate Inspection; full rounds/choices materialize only after selection bounds. Identity reference byte omissions retain their exact transport count independently of Candidate list omissions.

Shared Work answers also carry the latest Checkpoint's recorded limits and non-goals with exact Checkpoint/revision keys, independently of optional generated limits prose. Earlier limit observations remain history evidence; omission of a new statement does not assert resolution.

### Source-bound code behavior reading

`MapEntity.behavior` carries bounded body-expression claims and their exact Source
ranges, explanation state, omissions and limitations. Component explanations use
these observations to describe declared inputs, conditions, processing expressions,
assignments and returns, preserving source-authored responsibility text when
available. Work applicability comes from the existing changed-path, canonical-link
and grounded-neighbor selection basis, never from a function name or a shared
Repository Snapshot Source. No unobserved business purpose or callee effect is inferred.

`Current`, `Partial`, `Stale`, `Unsupported` and `Unavailable` describe explanation
reliability separately from underlying structural/semantic capability. Missing body
observations do not make a structural declaration a behavior explanation. Partial
coverage preserves usable expressions and exact omissions; historical evidence cannot
establish current behavior. Viewer consumers expose state ordinarily and exact
expression ranges, Source/Repository/Analysis identity, static supporting relations
and limitations in evidence disclosure. There is no read-side source access, analyzer
execution, provider invocation or new navigation/styling contract.

Retained RI `AgentInterpretation` consumption resolves its `analysis_basis` to exact
entities/relations and carries source ranges, producer, generated time, freshness and
uncertainty. Only interpretations grounded in the displayed entity scope are consumed;
a repository-wide Source alone is insufficient. Generated prose remains a separate
interpretation role and cannot add graph edges or execution observations.

Explanation validity is checked before presenting bounded entity payloads as current.
The consumer checks canonical Source availability/basis, unique exact entity identity,
entity/provenance/range/Analysis/Repository binding, included inventory content and
file analyzer basis. Missing/removed/foreign/ambiguous evidence is unavailable;
failed or unsupported coverage cannot be overridden by an older positive fact. File
partial coverage and bounded omissions remain partial with usable expressions.
Stale body expressions remain historical evidence and supply no current narrative.

Generated interpretations require all named analysis bases and supporting Sources.
Structural and semantic relation bases remain distinct; semantic file/capability
failures constrain semantic interpretation without erasing independent structural
body reading. Invalid generated prose is withheld while applicable state and reasons
remain inspectable. Current state means source-bound generated interpretation,
never independent prose verification. A component with source-grounded return/body
observations remains readable without resolved calls and acquires no flow edge.

Fixed English/Korean behavior prose explains the retained value operation and its
result: a source call result, calculation, binding/update or return. Supported
conditional returns and their continuation are described only through RI's explicit
control references. Unspecified relationships remain limitations; no expression
ordering, name, business-purpose guess or callee effect supplies missing meaning.
Source-authored documentation remains attributed wording. Control/value references
are validated inside the exact callable before explanation, alongside existing
Source/range/snapshot/capability checks. Viewer body reading renders this same fixed
explanation before raw disclosure, including exact-entity reads and components whose
topology explanation was omitted by the separate explanation bound. Each retained
claim still carries its exact Source range and owning entity/snapshots; generated
interpretations and arbitrary requested-language realizations remain separate.

Ordinary bounded code selection gives one eligible retained behavior entity priority
inside the existing topology selector before Repository Map and Understanding bounds.
Eligibility checks the same Source, snapshot, inventory, parser and capability basis
as explanation validity before body syntax can rank. Repository selection can use an
independent repository entity; Work selection only considers already grounded Work
seeds, and Understanding preserves their existing selection provenance. Tiny topology
bounds below three entities retain the established connected-topology selection.
Relations still require retained real endpoints, exact-entity details still resolve
from the complete basis, and entity/relation/explanation omissions keep exact counts.
The separate explanation bound ranks supported body meaning ahead of generic structural
component descriptions. No bound, resource ceiling, Work authority, graph store or
read-side provider/source operation is added or widened. With no eligible body, the
existing entity detail links expose structural evidence and the unavailable/unsupported
body limitation instead of synthesizing behavior.

### Explicit Work Source copy inspection

The Goal `context_item` exact-field allowlist also exposes `source_observations`,
a complete metadata manifest of explicitly recorded Source copies for that exact
Work. Existing 2,048-byte UTF-8 chunks and continuation fingerprints bound this
field independently of projection/Recall list limits. The manifest is not a
semantic relevance selection and does not collect evidence.

CLI/MCP preparation returns no inline Work observation inventory. Name preparation
is Goal-only and never enumerates managed Source observations. Summary/article
preparation advertises `source_inventory.discovery`, an exact Project/Work/Goal
revision `canonical_inspect` selector for this existing field. Optional investigation
reads only permitted metadata; selected body reads retain their independent generation
request/Source authority. CLI `work source list --work WORK` delegates to that same
reader, returning at most 2,048 UTF-8 chunk bytes. Continue with `--offset NEXT` and
`--expected-fingerprint HASH`, concatenate in order and decode after the final chunk.
The complete serialized manifest preserves every explicit association, including
expired/deleted metadata and exact body selectors, without body contents. Fingerprints
bind Project, Work, Goal revision, list content and lifecycle metadata; missing tokens,
foreign continuations, additions, deletion/expiry or Goal correction reject continuation.
Restart explicitly at offset zero after change; no silent offset shifting or partial
list is presented as complete. Unrelated Work insertion does not change this list.
The supported inspector recomputes full metadata for consistency on each page; this
contract bounds delivery and preparation, not constant-time inventory traversal.

A Source `body` read with exact Project, Source revision and `work_item_id` may
return its permitted `volicord_source_observation` envelope. It carries Work and
Source revisions, selected Repository/Analysis Snapshot and repository Source,
actual observation time and explicitly bounded file/command representation.
Without the Work selector a retained copy is unavailable as
`source_body_requires_work_selector`; foreign associations are denied. Existing
policy-withheld user turns and genuinely non-retained bodies keep their meanings.
No locator is opened and no current-file or generated-answer fallback exists.
Historical copy access asserts no current repository state. Wrong revisions,
changed fingerprints and corrupt/unsupported/hash-incompatible copies cannot
provide content. Work correction, canonical forgetting, invalidation, expiry,
explicit copy deletion and unavailable before-state withhold the affected copy.
A prior continuation can only return an unavailable state or reject changed
content; it never returns a deleted suffix. Metadata/lifecycle state remains
separate from transport completion and original representation/omitted counts.

The Work/Decision plan remains canonical-only. Explicit `work_explanation`
preparation also offers the exact Work Source metadata manifest, separately from
canonical facts and missing historical bodies. `generate` requests the current
already authorized interactive host's interpretation; its initial status is
`awaiting_host_interpretation`, never completed generated prose. The host reads
relevant permitted Sources (including separately observed callers/helpers/tests)
and authors `ExplanationRealization` without asking the user to assemble JSON.
There is no server-side model dispatch, automatic retry or mandatory provider.

`read_source` returns a permitted existing whole-file, bounded file excerpt or command envelope,
its SHA-256 retrieval fingerprint and normalized `citation_basis`. It does not
capture, renew retention or substitute current files for missing evidence.
`record` checks a live same-server request, exact Project/Work/plan and that each
cited Source fingerprint was retrieved within that request. Requests are ephemeral,
expire after 1,800 seconds and are single-use on successful recording; failure
preserves the prior record and permits a corrected submission. Restart drops them.
At most 32 requests and 128 distinct Source retrievals per request are retained.
Preparation and retrieval do not create canonical or Managed Derived records.
MCP returns complete structured grounding with a compact text pointer, within the
existing 256 KiB total-result bound; excess rejects, never truncates the plan.

Each citation binds Source identity/revision, retrieval envelope fingerprint,
Analysis/Repository identities and repository Source, locator, exact half-open
UTF-8 byte range, one-based line range and range SHA-256 to an exact half-open
paragraph claim range and its SHA-256. Uses are `current_code`, `before_code` or
`observed_execution`. New wording (including editorial changes) needs renewed
claim hashes/ranges. No separate editorial transformation is promoted without
independent reading-quality evidence; explicit regeneration retains a new output
alongside prior permitted identities through the same managed lifecycle.

Operations rechecks scoped body authority/expiry/hash, Work revision, snapshots,
locator/range and selected current repository inventory before recording and
again at reading/publication. An explicit before citation needs its observed
same-Work/path later file pair; changed/equal comparison is recomputed from both
bodies and chronology. Missing before-state cannot be reconstructed from HEAD.
Observed execution copies preserve actual exit/termination and bounded stream
omissions and establish no execution-time repository equality. Prose uses readable
locator/line references; Viewer links those references to local provenance details,
and documents retain them alongside exact citation metadata (metadata version 10).

Recording adds all citation Sources/repository Sources and explicit before Sources
to existing managed canonical dependencies and limits explanation retention to the
earliest cited body expiry. Deletion, denied read, correction, expiry, corruption
or repository change withhold current prose; read-only canonical facts remain
available. Publication rechecks Source dependencies in addition to canonical and
retained-output identity. The retained recorder-owned `semantic_approval_status`
is always `not_established`: valid ranges, readable prose or generation do not
certify causal meaning, translation, execution success or independent user approval.
Self-reported author/model strings remain self-reported; server session correlation
is operational correlation only. Conditional user instructions, recommendations
and generated explanation never create Decisions or Learning participation.

### Public grounded Work reading

Current retained Work answers carry a read-time, citation-only `source_excerpts`
basis from Operations. Each excerpt retains its exact Source/Work revision,
observation time, citation/range/claim hash and observed comparison. The shared
bound is 32,768 UTF-8 text bytes, with complete windows at most 16,384 bytes each.
The [complete-window contract](#complete-embedded-code-windows) supersedes the earlier
prefix allocation and ReadingPoint priority: complete unit bundles precede whole
supporting ranges, independent of prose role. Exact repeated ranges consume one
share; unavailable or over-bound windows retain counted navigation placeholders.
Live citation chunks remain independently bounded to 4,096 bytes. Selection changes
neither claim spans nor semantic importance or entailment. No uncited Source, current
file fallback or new Source observation is materialized. Only Current permitted
interpretations supply excerpts. Source bytes remain disposable managed content;
they are never copied into the retained explanation envelope.

CLI/MCP, live Viewer and offline snapshot read the same answers and source basis.
Existing MCP transport omissions remain separate from shared excerpt bounds.
The public Work `reading_title` uses only the independently admitted name Heading.
`QuestionAnswers.work_name` and `.work_summary` expose their own availability,
prose, citations/excerpts and provenance; the enclosing prose/state/provenance
continues to represent the article. `card_name`/`card_summary` select these scopes;
`reading_title`/`synopsis` on an individual answer set select that publication's
Heading/Synopsis. No card falls back to article text. Synopsis selection uses its
own role, never Purpose or a first paragraph as result. Each card scope carries its own exact realization fingerprint.
No unit can survive failure of its own dependency or current-language mismatch. Canonical
Goal, Source and Work identities remain independent of reader wording.

Citation navigation leads with the bound unit's explanatory role, evidence use and
exact locator. Each link selects its own keyboard-focusable reading point, quoting
that exact claim immediately beside the shared Source range. Each point links back
to its own original heading, synopsis or paragraph. Shared ranges never substitute
the first claim for another. Native fragments and browser history retain exact
Project/subject/realization/unit/citation identity. The script-free document uses
only embedded ranges; live selected inspection follows the exact read boundary below.
Neither navigation generates nor grants new Source access. Concise material limits and an explicit limits shortcut appear before narrative;
full authored/canonical limits retain their own section. The original unit indices
remain stable for grounding and return navigation. A code panel's closed disclosure
preserves all exact claims, ranges, hashes and revisions. Matching bytes establish
binding, not entailment, semantic approval or authenticated authorship.

Viewer prose links to unique keyboard-focusable Source ranges. Actual code/output
is ordinary reading after recorded verification/direction facts; full revision and
generator grounding stays in closed native disclosures. Dependency diagnostics,
including expired/denied/changed Sources, have readable ordinary notices in Overview, Work
and Work-scoped Code; exact raw diagnostics remain in a separate disclosure. Current unavailability cannot look like a missing initial
generation merely because its canonical Goal remains readable. Code scroll regions
are keyboard reachable, preserve indentation and do not widen ordinary prose.
Explicit paired observations offer exact-fragment navigation to the retained before
range when cited; an uncited or absent before body remains unavailable. An explicit changed/equal pair describes observed file bytes and is
not a reconstructed diff. Missing before-state stays absent; an uncited earlier
dependency is identified without presenting invented before code. Snapshots retain
the same admitted bytes and identities without Runtime, listener, scripts or assets.
No read generates, transmits, adopts Learning, renews retention or changes records.

For file Sources, citation byte and line coordinates are always absolute in the
observed repository file, including when only a bounded excerpt was retained.
Operations translates absolute byte intervals into retained-body offsets and checks
measured absolute line bases; model-supplied relative offsets/lines cannot redefine
the locator. Command field coordinates remain field-relative. Citation ranges must
be nonempty UTF-8 ranges wholly inside the observed region; range hashes and claims
keep their existing checks. Current-code recording, reading and publication verify
the full current file hash through the same bounded no-follow streaming reader,
plus the selected repository snapshot. A change outside the excerpt also makes
current interpretation stale; historical Source reading still returns only exact
previously retained bytes.

Public `source_excerpts` carries `source_representation` and `observation_region`
from the verified observation, separately from the smaller cited/displayed range.
CLI and Viewer visibly label bounded file observations and missing surrounding
context. Displayed ends retain absolute repository coordinates even when the shared
4,096-byte per-range display bound clips a citation. Neither completed transport
nor an unclipped citation labels the observation a complete source file.

### Grounded reader documents

The current retained explanation is one managed reader document, not a checklist of
mandatory answers. The same `ExplanationParagraph` envelope represents authored
`heading`/`synopsis`, grounded narrative (`purpose`, `reported_change`,
`expected_effect`), `observation`, `comparison`, `limits`, `reading_point`, and
qualified `next_step`. Unsupported sections are omitted rather than filled with
absence prose. Canonical state, result, verification, review, acceptance, original
Goal and recorded direction remain separate facts with their actual scope/chronology.
A no-change investigation may explain conditions and observations without pretending
there was a patch or a before-state.

`generated_units()` and `citations()` are the common exhaustive traversal for admission,
MCP retrieval checks, Operations Source revalidation, dependencies, expiry, public
provenance and read-time excerpts. Every unit requires exact prepared canonical keys;
technical claims also require scoped Source bindings. Observation/comparison/reading
point units require citations. Heading (240 characters) and synopsis (2,000) are
optional singleton authored interpretation, never clipped Goal text, a first body
paragraph or canonical title authority. Each publication binds exact language, plan and realization
identity and cannot survive deletion or invalidation of its own dependencies. Original
quotations retain their own language. All unit JSON participates in the existing byte
budgets; no limit is widened and no read generates or writes.

The format disposition is Unsupported for non-current content, with canonical facts
readable; explicit current preparation/recording is recovery. Historical assets and
feedback are preserved controls, never refreshed retention, new generation or acceptance.

Scoped execution and Checkpoint verification remain independent. Permitted cited
command observations supply `SourceExecutionObservation` facts with exact Source
revision, label, exit/termination and the explicit limit that exit status establishes
no test assertions, Checkpoint verification or acceptance. Multiple stream citations
of one execution produce one observation fact. No Checkpoint verification is labeled
as such; it cannot imply no execution occurred. Deleting/withholding the supporting
copy removes its observation-derived reading while preserving canonical remainder.
Existing independent result/state/check/review/acceptance/next-action chronology and
complete category selection remain the common rule; no new completion inference is
introduced.

File citation labels describe an observed file in the selected snapshot, including
preserved archival reports; they do not claim the file is current Product code or
that a report is a new execution. Original historical identity/time remains in the
control provenance. An absent authored synopsis may show a separately labeled
problem paragraph or original Goal, never relabel that paragraph as a synopsis.
Recorded next actions and recorded limits stay ordinary in Work rows; absent
next-action metadata remains separate from original instructions and suggestions.

### Viewer reading and tool navigation

The single shell/Overview contract above and D-04–D-13 replace the former
two-item masthead plus separate Project row, duplicate Overview repository summary,
Project Decision list, neutral-title-as-normal-output and observation-time-only rules.
Work-scoped and applied Decisions remain within their related Work; unassociated
Decisions remain inspectable through CLI/MCP/Operations outside Viewer. Live and script-free snapshots share the same components,
exact identities and meaningful normal-versus-unavailable distinction. Snapshot
publication grants no Runtime mutation capability.

### Repository reader document

`ExplanationSubject::Repository(AnalysisSnapshotId)` is independently scoped;
Operations prepares exact snapshot/binding evidence rather than choosing recent Work.
A repository reader document explains system purpose, material responsibilities,
their cooperation and distinct significant operations proportionate to complexity,
distinguishes recorded rationale from inferred reasons and names material unverified
coverage. A complex repository needs at least two materially different complete
operations; a genuinely single-purpose tiny repository need not invent another.
Inventory alone and a fully explained single subsystem are insufficient breadth.
The existing reader-unit envelope and exhaustive traversal remain shared with Work
and Decision. Repository roles are optional heading/synopsis/purpose/observation/
reading_point/limits. Every unit needs exact prepared evidence keys, and every
non-limits unit needs current-code Source citations from that exact Analysis Snapshot.
Repository operations may use the existing `flow_step` units, with exact
`repository_basis` and current-code citations. Authored order is retained through
composition, storage, bounded CLI/MCP reading and live/script-free ordered lists.
These are static source interpretations unless execution is separately established;
step order is not an execution observation or a new diagram topology. This corrects
a Repository-only admission omission without changing the v6 envelope or the
Work/Decision/name/summary grounding rules. Repository preparation fingerprints
change and require fresh explicit authorship; short Work publications do not.
Scope-only limits can use repository basis without a technical Source assertion.
Product composition derives Source revision/locator, absolute UTF-8 ranges/lines and
range/claim hashes from explicit host-selected spans; the host authors prose, not
mechanical identities. A valid hash/range is neither entailment nor semantic approval.
Repository snapshot/coverage/scope metadata remains retained and inspectable; raw
Source bytes stay in separately managed disposable copies. Latest permitted reading
uses committed insertion order; failed writes preserve prior records, while a later
deleted/invalid record cannot revive an older document. Ordinary public projection
exposes repository explanations independently of selected Work. Original Project
Purpose stays separate canonical context, including when absent.

Repository preparation preserves inventory classification counts and a commitment
to the complete inventory, plus exact capability state, reason, usable remainder
and producer identity. Repository boundaries and each capability's six coverage
sets carry area/file counts, common file parent (a lexical scope, not topology)
and a SHA-256 commitment to the ordered typed areas; an empty count denotes an
exact empty set. Full paths are explicitly excluded from preparation and Viewer,
including closed disclosures and offline export. Inventory scope, investigated
Sources and authored topic coverage remain distinct. The plan binds exact Project,
Analysis/Repository Snapshot, Source and local binding; citations still require
separately permitted Source investigation. Full immutable metadata/diagnostics are available through
`repository explain evidence --analysis ID`. This is an inspectable projection,
not silent truncation or complete semantic coverage. The preparation/retention
bounds remain unchanged and reject an unrepresentable basis before authoring.
Diagnostic/uncertainty lists likewise retain exact counts/hashes rather than bulk
debug strings. Reads never regenerate or replace a formerly prepared document.
Repository preparation, observation and citation admission/publication validate
the verified analysis metadata/inventory and fresh repository identity, not a
decoded whole analysis graph. This permits bounded useful Source interpretation
without claiming graph integrity or complete semantic understanding. Exact graph,
diagnostic and historical worktree inspection remains an explicit evidence read.
Work analysis consumers retain their own analysis validation boundary.

MCP `repository_understanding` accepts optional `reading_focus` (`overview` or
`repository_explanation`). The latter returns the exact retained reader document
without unrelated topology/Work catalogs, using existing complete explanation
transport bounds and read-only lifecycle. This is selection, not a new retained
format, Source grant, summary store or generation operation.

Repository reading summaries distinguish the exact cited Source file area from
included inventory files. Their common directory is a presentation of the retained
paths, never a claim to explain every file below it. Root/directory entries do not
count as included files. Excluded and unavailable/unexamined boundaries, limited
and failed capability reports, and non-current freshness remain separate counts;
complete semantic understanding is not inferred. Exact included/excluded/unavailable
paths and original capability coverage remain in the inspectable retained basis.
The same deterministic scope facts reach CLI/MCP, live Viewer and offline documents.
Unavailable/stale/withheld explanations cannot supply a current scope preview.

Ordinary Work reading shows current failed verification and rejection separately
from prior adverse observations, Source gaps and verification coverage limits near
the opening. Complete independent state observations stay in full evidence. When
host interpretation is missing, a nonblank canonical result remains a labeled
original-language quotation; it is not generated explanation, a current code claim
or a new execution. Decision reading likewise preserves exact Question context and
recorded user rationale without substituting a recommendation. Unrecorded unknown
states remain inspectable without dominating every first screen.

### Exact selected citation inspection

Live Viewer may read one selected citation from the latest Current permitted
Work/repository realization. Operations takes exact Project, subject, language,
realization fingerprint, original unit/citation indices and a UTF-8 offset relative
to that citation. The retained citation supplies Source/revision, envelope hash,
Analysis/Repository Snapshot, repository Source, field, locator, range and claim
hashes; no caller path, Source-ID-only request or new observation can replace it.
The existing inspection lock, latest-record ordering, current preparation and full
Source dependency revalidation apply before bytes leave Operations. Changed
realizations reject old selections rather than reinterpreting their indices.

Each selected response contains at most 4,096 original UTF-8 text bytes and 32,768
serialized JSON bytes. It preserves the complete authored claim span and original
citation plus actual displayed byte/line starts/ends, omitted before/after bytes,
next citation-relative offset, observation scope and execution/comparison facts.
Pages replace the displayed chunk; they do not accumulate a hidden full-file copy.
A complete citation page does not imply a whole file was observed or that the claim
is entailed. Before/after pairing, absent before-state, non-retained surrounding
context and observed command exit/termination retain their existing meanings.

`GET /citation` is bound to the live listener's exact Project and Host. Present
Origin and Fetch Metadata must be same-origin; no CORS grant or redirect exists.
Its only selectors are subject/language/realization/unit/citation/offset. Responses
are no-store JSON under existing HTTP bounds and CSP; only same-origin connections
are enabled. Opening a citation checks its current permitted bytes in the existing
desktop evidence pane/narrow focused view. Failure removes displayed code from
that reading point and names current unavailability with its original return target.
Reading, retry and pagination acquire no Source, renew no retention, perform no
canonical/Derived write and invoke no host/provider. Current filesystem identity
checks verify retained dependencies using existing bounded readers; they never
observe/persist new Sources or substitute current bytes for missing retained bodies.

Offline export includes only the shared bounded initial selection, with precise
clipping/omission and identity. It includes no selected-read controls, endpoint or
script and cannot retrieve omitted evidence. Initial unique text bytes, per-range
limits and additional selected-read byte limits are visible in their own context;
HTML transfer size includes markup, repeated claim panels and escaped text separately.

### Admitted Source syntax presentation

Viewer decorates only the exact initial excerpt or selected `/citation` chunk
after Operations has validated its authority, lifetime and complete grounding.
`source_highlight.rs` is an in-tree, bounded lexical reading aid, with no new
dependency, external grammar, I/O, retained state or semantic parsing authority.
Current excerpt metadata has no explicit programming-language field: admitted
file `body` plus `whole_file`/`bounded_excerpt` representation and recorded locator
suffix select JavaScript/TypeScript, Rust, Python, Shell or JSON decoration.
Execution/stdout/stderr and unrecognized formats remain plain text regardless of
their locator. This display hint neither opens a path nor establishes capability.

The same lexer produces escaped static markup (including script-free offline
HTML) and bounded UTF-16 token ranges for live DOM text nodes/spans. Identical
admitted chunks receive equivalent decoration. Each chunk starts without outside
lexical context: incomplete strings/comments, regex/JSX/template interpolation,
Shell heredocs and identifiers classified as types/functions remain approximate.
No surrounding Source may be obtained to improve colors. Input is at most 4,096
UTF-8 bytes; more than 256 colored runs falls back to the full plain chunk.
Live decoration must fit the existing 32,768-byte response limit or is omitted.
Invalid client token hints also fall back to the complete text. No token metadata
enters canonical/portable/retained explanation formats or changes their versions.

Original newlines, indentation, Unicode and half-open byte coordinates remain
Source-owned. Static CR uses an HTML character reference to avoid parser CRLF
normalization. Line numbers stay outside `code`; spans add no copied text. The
HTML parser cannot represent U+0000 in script-free DOM text: such an excerpt
explicitly discloses the replacement-character limitation, while the permitted
live JSON/text-node path retains NUL. Rendering never changes the retained bytes
or silently declares that exceptional offline text a byte-faithful DOM copy.
The range label and left rule identify the observed cited extent without inferring
more precise claim-to-line entailment or a patch. Colors are decorative, with
keyword weight/comment style, light/dark palettes and forced-color fallback;
horizontal scrolling preserves original line geometry at narrow widths.

### Explicit Work content composition

#### Independently admitted Work publications

Exact-current format 6 uses three publication subjects for the same Goal-backed
Work: `work_name`, `work_summary`, and `work` (article). These are Derived purposes,
not canonical entities or alternate Work identities. They use the same producer,
retained envelope, CachedSummary store, decoder, read barriers and Source validator.
CLI `work explain prepare/compose/record/delete --scope name|summary|article`
(default article) and MCP `work_explanation scope=name|summary|article` explicitly
select one scope. MCP request identity binds that exact subject; a request/retrieval
from another scope cannot authorize recording. No read or canonical write generates.

Name preparation contains only the current Goal identity/revision/field, original
Source metadata and relevant conflict relations. It admits exactly one Heading
grounded in `goal`, describing the requested action/problem, with no technical
citations or result assertion. Summary preparation follows the field selection in
[Independent Work progress publications](#independent-work-progress-publications):
Goal, latest nonblank report when present, current state, applicable recorded checks
and coverage, independent review/acceptance resets, material limits and direction.
It admits one Synopsis grounded in every selected field, preserving missing reports
and partial/failed/paused/no-change meaning. A report is no current-code verification;
technical assertions beyond it need their own permitted Source citations. Article
preparation retains the full course and technical dependencies. Every scope fingerprints
its exact Project/subject/language, current preparation instructions/budget, evidence
revisions, Source metadata and conflicts. Cited copies additionally bind Work revision,
Repository/Analysis identities, retrieval fingerprint, range/claim hashes and original
expiry. Uncited technical bodies are not dependencies of record-only publications.

Each successful explicit record creates one complete independently admitted envelope
under its subject/language purpose. Latest local insertion selects replacement only
within that purpose. Retention is bounded by that scope's earliest cited body expiry;
record-only scopes need no retained code copy. Deleting a publication removes its
languages/history without deleting sibling publications. Canonical correction or
forgetting invalidates/deletes all genuinely linked publications through existing
managed links/barriers. Expired or invalid article text is never partitioned, stripped
of dependencies or reused as a current card: recovery requires fresh preparation,
host authorship and admission. Failed admission leaves prior permitted output intact.

Counterexamples determine this boundary:

| Input/change | Name | Summary | Article |
| --- | --- | --- | --- |
| Goal only, no code | Meaningful requested-action name | Request and unreported outcome | Missing/partial detail remains explicit |
| New selected progress, even with blank report | Unchanged Goal name remains current | Stale; explicitly author supported current progress and qualifications | Full preparation becomes stale |
| Uncited technical copy expires | Record-only name remains current | Record-only report summary remains current | Citing article becomes stale |
| Summary actually cites deleted Source | Goal name remains current | Withheld/stale; never drop citation after admission | Only affected publications become non-current |
| Goal correction/forgetting | Non-current/deleted | Non-current/deleted | Non-current/deleted |
| Different requested language | No language fallback | No language fallback | No language fallback |
| Replacement name/summary/article | Only that scope selects the new realization | No implicit prose merging or historical fallback | Exact publication provenance remains separate |

A single whole-document lifecycle unnecessarily couples a Goal name to unrelated
technical expiry. Splitting a stale document on read would change its admitted basis.
Per-paragraph article storage would create unnecessary editorial/replacement complexity.
The three scopes are the smallest meaningful publication boundary for these cases.
Readers expose separate name/summary/article availability and provenance, preserve
exact canonical state, Goal, Decisions and both event bases, and never label a safe
missing screen as successful normal content. Export/publication rechecks every used
scope independently. Offline copies retain the admitted identities and explicit
limits of their export instant; they never renew retention or access a Source.

CLI status/Recall, MCP read projections, Overview and Work catalog use those same
card scopes. Work detail labels the independent summary separately from article
prose; an unavailable article may use the current name as its heading with the
name's own realization identity. Source inspection links for a cited summary use
the exact `work_summary:WORK` subject and its own original unit/citation indices.
Documents label each scope rather than attributing combined wording to the article.
Document metadata, Viewer publication barriers and live observation context carry
every used publication independently, including record-only card content when the
article is unavailable. GET/render/export preserve Canonical and managed records.

`LocalOperations::compose_work_explanation` and Work CLI/MCP compose consume a
new host-authored heading, synopsis and reader-ordered narrative. Product derives
Source revisions, coordinates and Source/claim hashes from exact permitted reads;
MCP additionally requires retrieval in the same ephemeral generation request.
Compose persists nothing and invokes no model. Work record/read retain the single
v6 publication-scoped lifecycle; a fresh record-only document is allowed, while
technical claims keep Source dependencies. Canonical-only failures or missing
bodies are not reasons to extract stale title text. See the current reader ledger
for direct tests and outstanding semantic/human conditions.

### Technical article, verification and inspection contract

[Reader experience D-19–D-27](reader-experience-design.md) supersedes any earlier
fixed question-enum-to-screen-section, raw-dump-first or duplicated navigation
requirement. Evidence questions/roles remain grounding coordinates, not mandatory
headings. Work prose explains supported background, mechanism/cause (when observed
or explicitly inferred), actual attempts/change/comparison and result in suitable
reader order. No-change, error-free, no-Decision and no-retry tasks omit fictional
events. Direct technical prose can name mechanisms without repetitive report
attribution; material uncertainty remains beside the claim and in a short basis.

Article, structured verification, material warnings/limits, original instructions,
evidence index and administrative inspection have distinct reading responsibilities.
Verification lists available named checks, exact Command Source/run, selected target
snapshot (including whether execution state was rechecked), times, outcome/termination,
coverage, skipped/unstructured records and historical/current applicability. Similar
labels or invocation fingerprints alone do not establish a retest or assertion
coverage. Unrelated later passes cannot heal an earlier failure. Canonical reports,
local process observations and host interpretation retain their different provenance.

One persistent accessible document navigation replaces duplicate shortcut controls;
fragment/offline/keyboard/pane behavior and original protected actions survive.
Warnings state type, severity, relevance and safe action; current conclusion-changing
failures are prominent while historical adversity is qualified. The evidence index
connects claims by meaning/purpose to exact permitted records; full raw structs are
available through explicit inspection. Scoped search/filter/paging/relations/counts
must use Operations projections and Privacy barriers. No SQL or database-file bypass
is granted. These are target outcomes; previous v4 roles/rendering were only a partial
implementation, not proof that flexible articles, typed verification UI or a secure
inspector have been delivered.

The reader-question ledger in repository-intelligence.md maps existing producers
and known missing inputs. Explicit Source manifest/read and preparation/composition
are separate steps: a canonical plan does not automatically incorporate every body,
and a missing Checkpoint cannot manufacture attempts, verification or Decision.

### Flexible article production

Preparation now gives the authorized host a source-first editorial pass: identify
missing explanatory steps, organize actual topics, author useful section headings
and read each causal bridge against original evidence. Short single-topic answers
can stay unheaded; neither section counts nor enum order establish adequacy.
The shared `article_sections` contract groups exact original unit indices into
article, verification and limits layers. A heading introduces the next content
unit; layer changes end a group. Empty/consecutive headings and headings followed
by a title/synopsis are rejected before retention. All labels remain ordinary
grounded units, with their existing complete dependency traversal. The ephemeral
public `sections` read metadata contains only indices/layers, never new prose.
It neither synthesizes missing headings nor makes unstructured prose semantically
approved. General admission still allows truthful short/partial content.

The exact-current v6 reader stream adds `section_heading` and `narrative`, paired
`comparison_condition` / `comparison_result`, and cited `flow_step` fragments.
Each fragment uses the existing question/text/evidence_keys/citations envelope,
common exhaustive traversal and validity within its admitted publication scope. No nested generated
strings, extra retention store or format-dependent decoder exists. Conditions
precede their adjacent result; all other sections follow the author's order.
Section headings are bounded to 240 characters; existing aggregate budgets remain.
Work and Decision compose require a fresh authored heading and synopsis. General
record admission still permits partial documents; unsupported detail is omitted.

Work CLI prepare includes its explicit Source inventory; the host selects and reads
permitted caller, callee, configuration, test and before/current material. Missing
Source, unobserved execution or absent before-state cannot be supplied by prose.
Work prose summarizes command outcomes with precise citations and only necessary
short diagnostic quotes. Complete JSON reports and execution logs remain in Source
inspection instead of being repeated in the article. This is authoring guidance,
not structural proof of readability or semantic approval.
Repository CLI prepare identifies its exact Analysis Snapshot and separate repository
observation, Source read and analysis-evidence entry points. Preparation grants no
body authority, observes no files and never substitutes the Decision-only boundary.
Repository preparation has its own source-first investigation guidance, independent
of Work correction/attempt authoring. Before prose, the host discovers responsibilities
and questions from real entry points/configuration and follows permitted implementation,
caller/callee, persistence, transport and failure paths across the system. Inventory,
analyzed topology, investigated Sources, explained topics and unknown/omitted areas
are separate scopes. Counts/hashes do not supply material questions or finished meaning.
The overview integrates boundaries before selective detail; sections and their count
follow the actual repository. Missing causal bridges remain explicit. This guidance
changes preparation fingerprints, not v6 representation or numeric budgets. Generic
headings retain grounding because a self-declared role cannot prove nonassertiveness;
the Viewer keeps their actions quiet without removing any dependency. Source-first
semantic review remains necessary even for correctly cited headings and body claims.
Decision CLI compose and MCP `decision_explanation` use the exact recorded Question,
alternatives, recommendation/rationale, user choice/optional reason and scope. They
grant no Work Source authority. Preparation, compose and record never dispatch a
model; the authorized active host supplies semantics. All public readers carry the
same admitted units. Viewer uses actual section names and typed comparisons/flows
in live/offline output, without fixed enum-titled sections or additional generation.

### Independent verification and evidence reads

`WorkReading.verification_report` is a read model over original Checkpoint facts,
Command Sources and permitted same-Work SourceObservation metadata. It is independent
of article presence, language, generation and excerpt selection. Each report row
retains Checkpoint/revision/original vector position (not a test ID), observation
time, reported state, exact original quotation, exact Command Source link, later
recorded changes and typed Unknown/Historical applicability. Free text is not
parsed into authoritative intended checks, assertion details or repair causality.
NotRun remains distinct from absence. Historical quotation-only producers retain no typed comparable relation, so their
comparison stays Unresolved. The explicit scoped-check producer records exact run,
artifact and prior Checkpoint basis; readers compare only that recorded relation. A subsequent
unrelated pass never repairs a failure or grants current coverage.

Canonical execution metadata survives unavailable disposable bodies as separately
qualified historical evidence. Permitted command copies add exact operation/window,
Work revision, selected Repository/Analysis Snapshot, target-not-rechecked limit,
expiry, stream representation/count/completeness, timeout/cancellation/signal.
Historical environment and assertion detail remain NotObserved; exit zero alone supplies none.
New scoped facts expose declared coverage/environment and stream-bound operator
assertion reports separately from measured process metadata.
Unavailable, deleted, expired and changed copies remove observation-derived fields.
Every enriched execution uses the existing exact Work/source read barrier.

CLI `work verification --work ID`, MCP `work_explanation action=verification`, and
ordinary public Work reads expose this same typed report without a generation
request. Viewer live/offline and CLI display at most 64 reports and 32 executions,
show totals/displayed counts and retain full original inspection. Review/acceptance
remain independent original dimensions. `QuestionAnswers.evidence_index` maps exact
unit indices to admitted keys/citations; typed `cautions` expose verification risks
without a packed status string. The index quotes admitted wording behind disclosure,
never authors new claims. Canonical originals and full audit remain inspectable.

Viewer publication carries exact Work/source observation fingerprints, including
retention metadata, separately from article provenance. Mutation-locked publication
rechecks those dependencies even when no article exists. Expiry/deletion/change
rejects the old render; it never silently drops fields to publish that generation.


### Reader presentation of independent reports

Viewer joins original verification rows only by exact Command Source identity.
Recorded labels are presentation, not parsed check names. Numeric exit/termination,
window, selected target/not-rechecked limitation, body state, environment/assertion
availability, timeout/signal/cancellation and stream capture each retain separate
attribution. Record-only outcomes remain honest quotations. All report originals,
including rows beyond display bounds, remain in an explicitly disclosed JSON read
model and independent canonical audit. No assertion or comparable retest is inferred from labels, quotations or exits.

The shared live/offline article renderer uses one compact native TOC disclosure.
Both the closed control and expanded outline remain sticky after deep scrolling,
with a bounded independently scrollable list and target offsets that keep the
selected text visible. Basic links work without JavaScript. Material warning labels distinguish latest reported failure, prior adverse
observations, coverage uncertainty, Source unavailability and rejected acceptance;
missing prose alone is not critical. Claim-purpose evidence disclosures point to
exact admitted Source selections and original records, with code before optional
observation metadata in the focused pane. These presentation changes alter no
retained format, authority, public route, protected action or generation policy.
Technical browser results remain separate from semantic and human acceptance.

Authored primary section headings now populate that same navigator in their
original order, only when their section has substantive content. Short unheaded
documents need no synthesized heading or primary introduction link. Supplemental
verification, limits and original-record destinations form one secondary group. Verification and limits
consume their separately grouped units, preserving original indices and citations.
Their authored subheadings are nested within their reading layer. The independent
card summary is disclosed in inspection when an article has its own lead, and
remains visible when the article is unavailable. Evidence index entries use the
shared section/layer metadata; they cannot borrow the preceding article heading
for a limitation that has moved into its own layer. Generic navigation shortcuts
and repeated equivalent per-entry cautions are consolidated, without removing
material warnings, lifecycle diagnostics or exact supporting record references.

### Purpose-led local inspection

D-24 / AC-22 uses a bounded Operations query over already authorized Project read
models, with class, role, lifecycle, literal visible-summary search and stable
identity pagination. Exact revision/field reads reuse existing evidence readers;
relations use explicit canonical IDs and independently admitted endpoints. Full
raw envelopes belong behind intentional inspection. Analysis health/recovery and
Documents preview/export, memory and general evidence inspection remain
CLI/MCP/Operations capabilities. D-30 removes their Viewer routes; contextual
Work/Repository evidence indices retain exact citation and return paths. See the [inspection threat model](privacy-and-provider-boundary.md#bounded-project-data-inspection-threat-model).
No durable schema or format changes are needed for this request-local projection.

Scoped verification rows retain the original canonical `ScopedCheckRun` separately
from disposable execution observation. Expired/deleted stdout cannot erase that
reported assertion, sampled artifacts or recorded comparison basis, and cannot
make output available. `CapturedArtifactsAtExecution` is limited to original sampled
paths; current whole-tree applicability remains unestablished. An explicit retest
selects an exact prior same-Work Checkpoint revision/vector position/Source. The
reader returns Comparable only for equal check key/purpose/assertion, known equal
operator environment and declared coverage, same invocation/path scope, observed
assertions, stable samples and measured same/changed-artifacts relation. Otherwise
the linked run is NotComparable with a typed reason. Unlinked passes stay Unresolved.
Original failures never change state, even when a later scoped rerun passes.

CLI text/JSON, MCP verification, selected Work detail, live Viewer and script-free
offline publication use the same independent typed report. Viewer discloses original
run window, artifact hashes, declared scope/environment, assertion provenance and
prior identity/rationale. No article, generation or citation selection is required.
Existing bounded display and publication revalidation remain authoritative.

#### Inspection query resource contract

Exact permitted counts and continuation fingerprints require scanning the requested
class and hashing every matching admitted row, including its complete admitted
relationships. They are not constant-cost operations or limited to the displayed
1–20 rows. Operations builds a request-local borrowed-record/identity/relationship
index, resolves endpoints by class and identity, and streams rows in the existing
stable identity order into the existing fingerprint encoding. Only one transient
rich row and at most the requested page are retained; other record classes do not
materialize quotation/field/metadata payloads. Relationship continuation uses that
same read/index pass rather than rebuilding all rows a second time.

For N admitted records, E explicit admitted relationships and B owner-read input
bytes, index memory is O(N + E), separate from the existing O(B) owned store read
bases. Index construction uses ordered lookups, O((N + E) log N), plus per-node
relationship ordering; query work includes requested-class rows and serialized
permitted matching relationship bytes. One high-degree row can still require O(E)
transient relationship storage. No persisted cache, hidden body hash, arbitrary
record cap, approximate/zero unavailable count or new source reader is introduced.
Candidate bulk admission shares the existing omission precedence and indexes
pending-forgetting identities without cloning complete Candidate content graphs.
`InspectionReadCost` distinguishes canonical/auxiliary reads, index construction,
visited/matched/retained rows, admitted edges and fingerprint byte work. Its counters
are not allocator or RSS measurements. Existing store decoding and invalidation
costs remain distinct; no constant-time side-channel guarantee or absolute Project
size/latency SLA is established by this projection contract.

### Publication-scoped reader fragments

Viewer anchor namespaces derive from a structured Project, subject, publication
scope, requested language, current/unavailable state and realization identity.
Missing Work name/summary/article publications never borrow identity from equal
prose or empty facts. Stable unit/citation indices retain the original Source
selection authority; DOM identity grants no additional read permission. Whole
script-free snapshots must have one target for every internal link, including
multiple Works and related Decisions. This presentation metadata changes no
retained explanation format or decoder.

### Progressive supporting reads

Verification uses a short typed report digest in normal flow and one closed native
disclosure for exact command labels, run/environment details and check histories.
Unresolved failures remain visible with links to their exact report, including
when a different later check passes. Historical applicability comes from recorded
changes or an explicitly comparable passing rerun, never from being older than
another report. All original facts and declared coverage limits remain inspectable.
The evidence index is one closed disclosure. Default adjacent Source actions
prioritize technical mechanism/cause/result claims. Authored names, article/section
headings and lead provenance are deliberately available in that index rather than
repeated prominent inline actions (Reader D-34 / AC-32). All original units,
evidence keys and citations remain admitted, retained and revalidated unchanged;
role-based presentation cannot prove semantic sufficiency or relax source authority. Missing summary and article availability have distinct
messages without repeating the complete unavailable-publication fallback.

### Persistent outline and repository reading hierarchy

[Reader D-35–37 / AC-33–35](reader-experience-design.md) require a compact persistent
outline whose expanded trigger/list remain in the viewport after deep scrolling.
The list has bounded independent scrolling; selection reveals the exact target
without overlay and restores focus. Native no-script disclosure/fragment navigation
remains functional. Page-unique publication namespaces and original unit/citation
indices remain unchanged, including unavailable publications and independent cards.

Repository reading keeps a short truthful scope and result-changing unresolved
failures near the opening. Routine Source counts, inventory/capability/coverage
counts and unsupported/investigated/excluded detail live in existing optional
inspection. Exact facts still reach CLI/MCP and offline supporting disclosures.
This reorders presentation only: repository-wide semantic purpose is system roles,
cooperation and representative flows, never one Work topic relabeled as the system.
Existing per-unit current-code admission (including Repository headings/synopsis)
remains in force. A later evidenced authoring/admission change owns any repair to
that rule or budget/publication constraints; hiding a link removes no dependency.

### Grounded concept and sequence publications

The current explanation envelope is v8. It adds `diagrams` to the exact admitted
publication, supporting concept relationships, components, guarded flowcharts, owned states and UML implementation sequences.
The earlier flat `flow_step` admission and v6 no-diagram disposition are historical;
ordered prose alone does not satisfy the required visual understanding outcome.

A concept has shared typed nodes and directed typed relations; a sequence has typed
participants and vector-ordered groups of messages with sequential or parallel
meaning and optional conditions. Labels, title and conditions are exact UTF-8
selections of ordinary Source-grounded units. The full caption unit identifies scope,
static interpretation, uncertainty and non-effects. `record`, `context_role`,
`reading_group`, `component` and `participant` are explanation classes; they do not
create canonical entities. In particular Goal/Purpose are ContextItem roles, and
Work groups the Goal identity and explicitly linked Checkpoints/Decisions.

Compose seals each figure's complete meaning plus preparation, requested language
and all referenced units/citations. Endpoints, relationship/message type, conditions,
ordering/parallelism and captions affect that semantic fingerprint; valid hashes
still establish identity rather than entailment. Exact Source assessment remains
independent. Every authored string lives in `generated_units()`; `labels()` exhausts
all semantic selections. Ordinary citation/dependency traversal therefore covers
nested labels, guards and captions, without a second Source authority. Figure and
element IDs are bounded tokens, never display strings or Source locators.

Whole-publication limits remain unchanged: 32 units, four figures, 12 nodes and
20 relations/messages per figure, 12 sequence groups and 96 printable characters per
label. Empty/missing/duplicate elements, unknown shapes/fields, unbound selections,
foreign preparation/language, edited seals and non-current envelopes fail admission.
Name/summary and Decision publications cannot carry diagrams. There is no conversion
or partial salvage. Retention/currentness/publication atomicity use the same managed
record and all original dependencies; ordinary reads generate nothing.

Public CLI/MCP ordinary answers and complete publication pages include the exact
semantics and seal. Human text includes endpoints, relation types, conditions and
explicit parallel groups. Live and self-contained offline Viewer render the same
meaning through in-tree `volicord-static-svg/2` plus a faithful linear alternative.
Layout coordinates and SVG are presentation only. Concept edges use numbered keys
matched to their text claims, avoiding long labels across shared-node lines. Sequence
lifelines, self messages, guard frames and return dashes do not assert observation.
Parallel messages occupy separate dashed UML `par` operands; their row labels are
identifiers, not an ordering claim. Sequential groups have no operand separators.
The local renderer has no dependency, markup interpreter, remote asset or process;
all authored text is escaped. SVG is bounded to 256 KiB per figure. Failure is explicit,
leaves the admitted text where available and never counts as visual completion.
Native scroll/anchors and existing exact-citation/Source return remain usable without
script; no browser zoom interception or whole-page scaling is introduced.

The existing architecture SVG primitive describes analyzed topology, not authored
publication claims, so it cannot provide this semantic path by itself. A general
Mermaid/PlantUML interpreter would add grammar, packaging and execution dependencies
for two bounded shapes. The in-tree renderer avoids that cost and introduces no
third-party renderer license or bundled assets. It is purpose-built, not a general
UML editor. This checkout declares no separate renderer package license; this is
not a claim about wider project licensing. Rendering/first display and complete
public reading costs must be measured separately; no budget is raised for diagrams.

### Diagram selection and exact return

Each node, relation, condition and message has a publication/language-scoped SVG
origin and its own concise claim panel; a faithful text list reaches the same panel.
Claims keep original unit/citation indices and show purposeful implementation
buttons with inspectable file/range labels. Several elements may share a unit and
Source payload without sharing their return positions or inventing a new binding.
The default caption appears once; quiet caption provenance stays in Evidence.

Live Source fragments may carry `!<diagram-claim-id>` after the exact Source-point
ID. The client accepts this only when the current DOM claim contains that exact
Source action. Reload/history therefore restore a bound origin without granting
another read selector or changing the HTTP citation protocol. Source return focuses
the selected claim; element return restores original SVG/text trigger, page and
figure scrolling. Foreign/stale origins produce an unavailable fragment notice.
The native no-script href remains the exact Source anchor; browser Back and the
Source's named diagram-return disclosure reach each original claim/element.

Local controls expose 75–200% figure zoom, reset and bounded reading-area expansion.
Native keyboard panning, page scrolling and browser zoom remain available. These
controls change presentation only and are absent without script. SVG/text identity,
claim selection, code bytes and native return anchors survive in offline HTML.
A known changed/expired/deleted/withheld/comparison-unavailable live citation failure
removes that realization's cached SVG, text alternative and claim panels, retaining
only opaque focus/return IDs and a safe unavailable notice. A distributed static
export cannot promise revocation or a new live comparison.

### Complete flow and state vocabulary

Version 8 shares nodes/relations for concept, component, flowchart and state figures.
Components distinguish dependency, production, data flow and control flow; arrow direction
is the authored relationship, never a dependency inferred from event direction. Flowcharts
use process, guarded branch, initial and final nodes and control-flow edges. Each branch
requires at least two guarded outcomes. State figures require a separately grounded owner,
state/initial/final nodes and event-labelled transitions with optional guards. Initial nodes
have no incoming and final nodes no outgoing edges. Every node participates in a relation.
Self transitions are supported; concept/component self relations are refused. Unsupported
node/edge classes fail the complete figure rather than degrade to generic boxes. State
ownership participates in labels, seal, Source lifecycle, public text and selectable origins.
Stored enum values preserve meaning; visible words, conditions and labels remain grounded
reader content. This bounded vocabulary asserts no UML composition, inheritance or
multiplicity. Conditions and unordered parallel operands remain explicit in SVG and text.

### Claim-led evidence and local applicability

Visible prose actions derive a bounded printable name from the exact admitted citation
claim (72 characters, with an explicit ellipsis when longer); they retain full use/path/range
in accessible detail and a separate wrapping range label. No new authored string bypasses
grounding and no truncation changes the bound claim or Source. Diagram action names use
validated printable grounded label selections. Purpose names are navigation, not new
claims or inferred file authority. Authors split broad effects into qualified claim units.
Concept/component/flow/state figures expose a visible numbered relationship key with
endpoints, wording, type and guard; it reaches the same publication-scoped claim as SVG.
A selection shows the selected wording and its complete material qualification; Source
continues to carry the exact bound claim with full article context available by return.
Workless Overview offers repository reading independently of canonical Purpose and Work.
Applicability explicitly says local comparison and no remote freshness check; snapshots
state that neither live nor remote checks occur in the distributed file.

### Responsive figure and history restoration

The persistent outline includes exact publication-scoped figure destinations, with
focusable native anchors. Controls retain native keyboard panning and browser/page
scrolling. Expansion exposes its pressed state and controlled viewport; Escape closes
it and restores the activation control, page position and local pan without a focus
trap. Caption, claim, path and controls wrap independently; only the figure scrolls
in two dimensions. Native fragment navigation updates CSS targets as well as history.

Each SVG/text activation and evidence activation owns a bounded ephemeral return
record. History entries retain their original activation rather than a latest value
keyed by label or element alone. These records carry presentation focus/offset only;
publication, language, realization and exact citation remain authority. Reload without
an ephemeral trigger falls back to the exact bound claim/element, never a similar name.
Repeated apply/resize callbacks do not steal focus from current figure controls.
Sources retain desktop article context and narrow focused code with explicit return.
No-JS uses the same figures, relationship key, text and exact embedded bytes through
native fragments; distributed exports promise neither live checks nor revocation.
