# Inquiry와 Decision 계약

- 상태: active specialized architecture owner
- 소유 범위: Question Candidate와 Question behavior, materiality, dependency frontier,
  recommendation/alternative presentation, exact current-host response linkage, terminal
  transition, pause/resume, Decision applicability·reuse·re-questioning과 Checkpoint interaction
- 상위 architecture 기준: [논리 아키텍처](architecture.md)
- core domain 기준: [핵심 도메인 모델](domain-model.md)
- repository fact 기준: [Repository Intelligence 계약](repository-intelligence.md)
- evidence 기준: [architecture 입력 계약](architecture-inputs.md),
  [V05 보고서](../../validation/inquiry/frontier-resume/report.md)
- projection 기준: [Projection과 document 계약](projections-and-documents.md)
- 비소유 범위: canonical storage schema, host wire format, automatic Question-discovery
  algorithm/quality, UI rendering, portable merge와 production transaction technology

이 문서는 material uncertainty를 단계적으로 해결하는 Inquiry and Decision
subsystem의 specialized contract다. `domain-model.md`가 소유하는 Question, Decision,
Source, Checkpoint identity와 provenance meaning을 사용하며 새 canonical entity를
추가하거나 Kernel의 write authority를 대신하지 않는다.

## 1. Question Candidate와 canonical Question

### 1.1 Question Candidate

`Question Candidate`는 작업 결과를 바꿀 수 있는 uncertainty라는 관찰 또는
가설이며 `Session Candidate`다. Candidate는 user에게 답을 요구하는 authority가
아니고 Decision을 만들지 않는다. 최소 다음 basis를 가진다.

- Project와 candidate identity
- 발견 actor/session과 Source basis
- 현재 알려진 fact, assumption과 uncertainty
- 영향을 받을 path, component, work context 또는 product concern
- possible prerequisite/dependency
- materiality assessment와 그 근거
- repository/environment research 가능성
- candidate freshness와 duplicate/supersession basis

Candidate의 text가 설득력 있거나 여러 번 발견되었다는 이유만으로 canonical
Question으로 승격하지 않는다. Automatic discovery와 materiality quality는 아직
실행 검증되지 않았으며 이 문서는 generation/ranking algorithm을 선택하지 않는다.

### 1.2 Engineering Choice Discovery

`Engineering Choice Discovery`는 Question/authority classification 전에 current Goal과 exact
pre-work Analysis Snapshot에서 meaningful technical fork를 표현하는 Session Candidate다.
Choice마다 다음 bounded meaning을 보존한다.

- stable choice identity, Project/Goal/baseline binding과 summary
- affected scope, 실제 viable alternatives와 technical consequences
- repository/source/research basis와 additional research/prototype need
- `public API shape/semantics`, `compatibility`, `failure/error semantics`,
  `persistence/lifetime`, `privacy/disclosure`, `security`, `user-visible behavior/default`,
  `performance/resource behavior`, `concurrency/operability`, `maintenance/support`,
  `implementation-internal` effect category
- independent 또는 actual outcome 하나가 반드시 함께 해결하는 symmetric coupled peer와 rationale

Effect category는 completeness/discovery metadata이며 authority를 자동 결정하지 않는다.
두 credible approach 또는 meaningful consequence를 가진 unresolved fork가 없으면 discovery-worthy가
아니다. Syntax, local naming, private helper split와 mechanically equivalent refactor를 inventory하지
않는다.

Independent choice는 별도 identity로 유지한다. "result retry behavior", "custom parser reload"
같은 broad Goal label은 API, failure, persistence, network, instrumentation 또는 compatibility
semantics를 coupled로 만드는 evidence가 아니다. 하나의 authority dimension은 independent choice를
collapse할 수 없고, multiple choice를 참조할 때는 complete symmetric coupled set을 사용한다.

### 1.3 Materiality status

Materiality assessment는 최소 다음 상태를 구분한다.

| Status | 의미 |
|---|---|
| `unassessed` | 결과 영향과 Source basis를 아직 검토하지 않음 |
| `needs_evidence` | repository/environment research 또는 prototype evidence가 있어야 materiality를 판정할 수 있음 |
| `material` | user choice, delegation 또는 명시적 branch disposition이 결과를 실질적으로 바꿈 |
| `not_material` | 현재 scope에서 결과를 실질적으로 바꾸지 않거나 이미 다른 canonical meaning에 포함됨 |

Canonical Question으로의 promotion에는 current revision 기준 `material` assessment,
Project scope, Source basis와 known prerequisite가 필요하다. `not_material` Candidate는
질문 수를 채우기 위해 승격하지 않는다. Materiality basis가 나중에 바뀌면 기존
Question을 조용히 삭제하지 않고 적절한 terminal outcome 또는 새 revision/source
basis로 처리한다.

### 1.4 Canonical Question

Canonical `Question`은 다음 meaning을 보존한다.

- stable Question identity와 Project ownership
- displayed wording을 포함한 correction revision
- materiality basis와 why it matters now
- established fact, assumption, known limit와 Source basis
- dependencies, prerequisite outcome requirement와 dependent branch
- displayed alternatives, recommendation와 explanation basis
- applicability scope와 what the answer unlocks
- current lifecycle outcome와 supersession relation

Question identity는 wording과 다르다. 같은 material meaning을 바꿔 표현했다고 새
Question identity를 만들지 않으며, meaning이 달라지면 correction revision으로
숨기지 않는다. Display된 revision은 response linkage의 일부다.

## 2. Dependency와 prerequisite semantics

`depends_on`은 dependent Question이 material하게 열리기 전에 필요한 다른 Question
outcome을 가리킨다. Dependency는 단순한 ordering hint가 아니며 다음을 명시한다.

- prerequisite Question identity
- dependent Question identity
- 어떤 prerequisite outcome 또는 resulting basis가 dependency를 만족하는지
- 어떤 outcome이 dependent branch를 supersede, exclude 또는 계속 block하는지
- dependency assessment의 Source와 revision basis

모든 terminal outcome이 모든 dependency를 만족하지 않는다. 예를 들어 특정
`deferred`나 `out_of_scope` outcome이 dependent implementation Question을 자동으로
열지 않는다. Dependency cycle이나 missing prerequisite는 empty frontier 성공으로
숨기지 않고 diagnostic/review 대상으로 표시한다.

## 3. Current dependency frontier

`current dependency frontier`는 canonical Question state에서 매번 재계산하는
read model이다. 다음 조건을 모두 만족하는 Question만 포함한다.

- current Project와 Inquiry scope에 속함
- current revision의 materiality가 `material`임
- terminal outcome이 없음
- 모든 prerequisite가 그 dependency가 요구한 positive basis를 만족함
- active Decision이나 upstream outcome으로 superseded되지 않음
- repository/environment research로 먼저 해결해야 할 fact가 남아 있지 않음

미해결 prerequisite가 있는 Question은 canonical하게 사라지지 않지만 frontier에는
나오지 않는다. 모든 material branch가 terminal이고 unresolved prerequisite나
dependency diagnostic이 없을 때 frontier는 비어 있고 Inquiry를 종료할 수 있다.

### Deterministic ordering

같은 canonical state와 scope에서는 모든 host/session이 같은 frontier order를
만든다. Ordering basis는 Question promotion 또는 dependency review에서 보존한 explicit
presentation order를 먼저 사용하고 stable Question identity를 final tie-breaker로
사용한다. Independent Questions는 이 순서로 같은 round에 묶을 수 있다.

Runtime discovery order, map iteration, access frequency, generated wording, model score와
Checkpoint listing order는 authority가 아니다. Ordering basis를 바꾸는 operation은
inspectable revision/basis를 남기며 이미 terminal인 Question을 다시 열지 않는다.

## 4. Research before asking

Repository, source, environment 또는 bounded experiment로 결정적으로 확인할 수 있는
fact는 user preference Question처럼 묻지 않고 먼저 조사한다.

- Repository Intelligence의 capability/coverage/freshness를 확인한다.
- 필요한 local file, Git, manifest, command와 environment observation을 Source로
  남긴다.
- 확인 범위와 unavailable/unsupported/failed uncertainty를 표시한다.
- 충분한 evidence가 있으면 Question을 `resolved_by_research`로 transition한다.
- evidence가 부족하고 user value가 필요하지 않으면 `requires_prototype`,
  `deferred` 또는 계속 research 대상으로 둔다.

Research result는 user Decision이 아니다. Agent가 사실을 찾지 못했다는 이유만으로
사용자에게 추측을 요구하지 않으며, repository/environment fact와 user preference가
혼합된 Question이면 established fact와 실제 선택 지점을 분리한다.

### Behavior choice is evidence-driven

Inquiry의 성공은 Question 또는 Decision의 존재 자체가 아니라 current evidence와
materiality에 맞는 behavior를 선택한 결과다.

- Repository/environment research로 작업의 불확실성이 해결되면 user Question
  없이 진행하거나 `resolved_by_research`로 닫는다.
- Accepted Decision 또는 user가 이미 명시적으로 위임한 implementation choice가
  현재 scope에 적용되면 재질문하지 않고 재사용한다.
- 대화로 판단할 수 없는 경우 `requires_prototype`, 추가 research 또는
  `deferred`를 선택할 수 있고 user choice를 위조하지 않는다.
- 실제로 사용자 가치·선호·용인 가능한 trade-off가 결과를 바꾸는 경우만
  material Question을 제시하고 explicit user-owned Decision을 받는다.
- 간단한 repository와 자명한 bounded task에서 qualification을 위해 Question이나
  Decision을 제조하지 않는다.

Materiality screen은 구현 방법의 개수가 아니라 outcome의 consequence와 ownership을
판정한다. 관련 current owner, applicable Decision/contract와 repository/environment
fact를 먼저 확인한 뒤 다음 순서로 분류한다.

- inspectable authority가 이미 outcome을 정하면 적용한다. Repository/environment
  fact는 조사하며 user Question으로 바꾸지 않고, accepted contract 또는 applicable
  Decision은 재질문 없이 적용한다.
- 명시적으로 위임된 implementation choice는 위임 범위 안에서 agent가 질문 없이
  선택한다. 이 authority에는 두 경로만 있다. 현재 user-stated Goal 자체의 위임은
  typed explicit-delegation evidence가 exact Goal identity와 그 Goal을 만든 exact
  current-host user-turn Source를 함께 가리키고, bounded verbatim statement가 Goal
  statement와 exact user turn에 실제 포함되며, evidence가 dimension identity, discovered
  choice identity, affected scope, material consequence와 effect category를 명시적으로 bind하고
  current Goal/work scope를 포함할 때 그대로 재사용한다. Source identity만 있거나 delegation
  statement/scope가 없는 basis는 authority가 아니다. 이 경우 위임을 되풀이하는
  Question이나 Decision을 만들지 않는다.
  Inquiry 중 새로 받은 위임은 기존 Question/current revision/current-host response에서
  생성된 applicable delegation Decision이 exact dimension scope를 포함할 때만 사용한다.
- exploratory uncertainty는 필요에 따라 research, bounded prototype, deferment 또는
  inspectable revisit basis로 전환할 수 있으며 user Question이나 Decision을 강제하지
  않는다.
- material하게 다른 consequence가 남은 unresolved user-owned outcome은 선택하거나
  구현하기 전에 explicit user authority를 얻는다.

Ask 전 unresolved user-owned outcome의 independently material한 dimension을 모두
식별한다. Recommendation이나 preferred implementation은 user authority가 아니며 다른
material dimension을 조용히 결정할 수 없다. Consequence와 authority가 독립적인
dimension은 각각 explicit authority를 얻는다.

Dimension이 실제로 coupled되어 한 user choice가 함께 해결한다면 하나의 Question으로
제시할 수 있지만, alternatives와 trade-off가 coupled dimension 각각의 material
consequence를 드러내야 한다. 이 completeness rule은 trivial implementation detail을 별도
Question으로 승격하지 않으며 material consequence와 ownership 기준을 그대로 적용한다.
Branch가 authority, delegation, research, prototype evidence, deferment 또는 user Decision으로
해결된 뒤 ordinary edit에는 별도 approval ceremony를 추가하지 않는다.

### Typed Materiality Review와 work readiness

Pre-work screening은 prose-only instruction이 아니라 `Materiality Review` Session Candidate로
기록한다. Review는 current Project, latest user-stated Goal Context, exact retained pre-work
Analysis Snapshot과 exact Engineering Choice Discovery Candidate에 bind된다. 모든 discovered
choice identity는 정확히 한 dimension에서 분류되며 independent choice를 broad Goal authority로
collapse하지 않는다. 각 dimension은 bounded Source, accepted contract, applicable Decision,
explicit delegation 또는 research/prototype/defer/revisit basis와 함께 다음 disposition 중
하나를 가진다.

Host의 Materiality draft는 이 판단 지점에 exact current Goal Context identity, 그 Goal을 만든
current-host user-turn Source identity와 Goal statement를 함께 제공한다. 또한 agent가 각 discovered
choice마다 먼저 다음 counterfactual을 명시적으로 수행하도록 machine-visible checklist를 제공한다.
Credible alternative가 externally observable contract, durable effect, compatibility/support
commitment, privacy/security posture, user-visible default, observable failure policy 또는 다른
material product outcome을 바꾸는가? 바꾼다면 그 exact dimension을 정하는 current repository/
environment fact, accepted contract, applicable Decision 또는 explicit delegation을 식별해야 한다.
Overall feature request, implementation preference, agent recommendation과 convention은 이 exact
authority를 대신하지 않는다.

Public host contract는 discovery-owned fact를 caller가 다시 전송하게 하지 않는다. `draft`가 반환한
단일 `record_request`에 Project와 Engineering Choice Discovery identity를 그대로 사용하고, caller는
각 discovered choice마다 `choice_id`, disposition, bounded basis summary, learning value와 그
disposition에만 허용된 semantic evidence를 `judgments`로 한 번 제공한다. Goal/baseline identity,
dimension/choice linkage, summary, affected scope, technical consequence, observable signal,
discovery Source와 source-operation provenance는 bound Engineering Choice Discovery와 canonical read
basis에서 server가 파생한다. `revise`도 review identity와 새 judgments만 받아 같은 discovery-owned
meaning을 다시 파생한다. 따라서 old full-dimension echo와 simplified judgment path를 동시에
지원하지 않는다.

각 disposition schema는 closed variant이며 required/forbidden field를 machine-readable하게
노출한다. Repository fact와 agent-owned choice는 다른 authority field를 받지 않는다. Settled
authority는 accepted contract basis, applicable Decision identity 또는 둘 다를 요구한다. Current-task
delegation은 bounded verbatim delegation statement와 delegated scope를 caller에게 요구하고,
Goal/Source identity와 exact dimension/choice/consequence/effect boundary는 현재 bound Goal과
discovery에서 server가 정확히 파생한다. Inquiry-time delegation은 Decision identity만
요구한다. Exploratory uncertainty는 exact exploratory disposition과 bounded research/prototype/revisit
basis를 요구한다. Unresolved user-owned outcome은 open 상태에서 다른 settling authority를 금지하고,
해결 뒤에는 exact resolution Decision 하나만 받는다. Validation failure는 exact field path,
invalid value, allowed values, bound Goal/baseline identities와 다음 supported `draft` action을 함께
반환해 같은 invalid payload의 반복을 피하게 한다.

`draft`는 validator와 같은 closed schema variant owner에서 각 legal judgment contract를
기계적으로 투영한다. 각 contract는 stable variant identity, exact required/allowed/forbidden
field, singleton enum value, caller가 제공할 semantic field와 실제 record/revise input schema를
포함한다. 별도 hand-maintained disposition table은 public contract가 아니다. 각 discovered
choice template은 discovery-owned summary/scope/alternative/consequence/effect/Source/evidence와
exact `choice_id`를 제공하고 모든 legal variant identity를 위 contract에 연결한다.
`learning_value`의 `routine`/`deliberation_worthy`와 learning participation의
`inactive`/current-host `active`도 실제 validator schema에서 같은 형태로 투영한다.
Current-task delegation을 검토할 때 draft는 각 choice별 exact Goal identity/text, current-host Goal
Source identity, dimension/choice identity, affected scope, material consequence와 effect category를
하나의 reusable evidence candidate로도 제공한다. 이는 delegation을 semantic하게 선언하지 않으며
caller가 exact verbatim excerpt와 dimension coverage를 판단할 책임을 유지한다.

반환된 `record_request`는 아직 review가 없으면 exact discovery identity를 가진 `record`, 같은
discovery의 current review가 있으면 exact review identity를 가진 `revise`를 prefill하고 실제
request schema와 choice order를 함께 제공한다. Caller는 variant를 semantic하게 선택하고,
prefilled identity와 그 variant의 fixed enum을 합친 뒤 요구된 semantic field만 채워 각 choice당
정확히 하나의 judgment를 조립한다. 따라서 one-pass caller는 validation failure를 schema discovery로
사용하지 않으며 placeholder semantic truth를 제출할 필요도 없다.

Current Goal 자체가 어떤 outcome을 user control로 남기거나 user가 choice를 retain한다고 밝히면,
older contract나 repository convention이 존재한다는 이유로 그 exact dimension을 agent-owned
implementation preference로 낮추지 않는다. Goal/source identity와 checklist는 deterministic하게
노출하지만 production은 prose keyword, regex, provider classifier 또는 hidden semantic inference로
ownership을 자동 판정하지 않는다. Semantic assessment의 품질은 active agent와 naturalistic
evaluation에 남고 production은 typed provenance, scope와 lifecycle invariant만 검증한다.

- repository/environment fact
- already settled authority
- agent-owned implementation choice
- explicitly delegated implementation choice
- exploratory uncertainty
- unresolved material user-owned outcome

Public API semantics, CLI compatibility/exit behavior, observable failure policy,
privacy/external disclosure, security posture, user-visible default와 maintenance/support policy는
강한 discovery signal이다. Signal 자체가 user ownership을 정하지는 않지만 settled/delegated
disposition은 inspectable authority가 필요하다. Agent recommendation, implementation
preference와 library/convention은 user authority basis가 될 수 없다. Agent-owned implementation
discretion은 `implementation preference`로 명시하되 contract, Decision, delegation 또는
recommendation authority로 가장하지 않는다.

Bounded counterfactual로 다른 credible implementation이 externally observable contract,
durable effect, compatibility/support commitment, privacy/security posture 또는 다른 material
product outcome을 바꾸는지 확인한다. 바뀐다면 exact discovered choice에 적용되는 repository
fact, accepted contract, applicable Decision 또는 explicit delegation을 식별해야 한다. Overall
feature request 자체나 implementation preference는 subordinate difference를 settled로 만들지
않는다. Multiple implementations라는 사실만으로 user ownership을 만들지도 않는다.

첫 authoritative review는 exact baseline과 fresh review observation 사이 meaningful repository
delta가 없어야 한다. 이 transition은 typed Local Operations path만 만들 수 있으며 generic
Candidate submission으로 timing을 주장할 수 없다. Timely first review 뒤 exploratory research나
prototype evidence가 alternatives를 바꾸면 같은 Candidate revision으로 재검토할 수 있다.
Maintained baseline/current fingerprint evidence가 dimension의 affected path 변경 뒤 disposition,
authority anchor, blocking readiness 또는 affected-scope applicability 변경을 증명하면 Review는 late
work-authority revision과 exact changed path를 보존한다. Delegated에서 repository fact/agent-owned로
바꾸거나 evidence-required exploratory/learning blocker를 ready로 바꾼 경우도 같은 prospective
경계다. Current truthful state와 필요한 Question/current-host Decision은 계속 기록할 수 있지만
earlier affected work를 certify하지 않으며 그 Review는 그 work의 ready-for-work나 Checkpoint authority로
돌아가지 않는다. Rationale, summary 또는 authority/readiness/applicability를 바꾸지 않는 metadata
revision은 late marker가 아니다.
Path/scope chronology를 deterministically correlate할 수 없으면 durable violation을 만들지 않고 host
guidance가 같은 prospective limit를 명시하며 naturalistic rollout validation이 operation order를 판정한다.
Restart는 current Candidate revision과 canonical Question/Decision state에서 workflow를 다시
평가하고 unresolved state를 ready로 바꾸지 않는다.

Shared work-readiness result는 current stage, overall disposition, next required Volicord action,
blocking 여부, reason/basis, satisfied requirements와 unresolved requirements를 제공한다.
`ready_for_work`는 current Goal/baseline review가 timely하고 모든 dimension의 evidence가 current이며
unresolved user-owned, evidence-required exploratory dimension 또는 active required Learning
Deliberation이 없고 current review에 bounded executable work scope가 bind되었을 때만 가능하다.
Descriptive `affected_scope`는 material outcome을 설명하고 executable scope를 암묵적으로 만들지
않는다. Existing `materiality_review.inspect(paths, components, work_contexts)` transition이 exact
current dimension set에 typed scope를 보존한다. Repository-relative parent path는 descendant를
cover하지만 component와 work context는 exact match다. Material dimension/scope expansion은 binding을
invalidate하며 baseline 뒤 이미 변경된 path를 뒤늦게 추가해 earlier work를 authorize할 수 없다.
이 result는 일반 file/command admission이 아니다.

Fresh resume baseline에서 새 material choice가 발견되지 않았다고 `choices=[]` Discovery를 제출하지
않는다. Host workflow는 retained Candidate inspection을 먼저 가리킨다. Continued bounded work가 있으면
caller는 prior stable choice identities를 fresh Goal/baseline/repository Source와 current scope에 대해
재평가하고, prior non-empty choices가 그대로 applicable하며 additional choice가 없다는 결론을 current
evidence로 한 번 record한 뒤 Materiality draft/review로 진행한다. Completed state를 read-only로
inspection/verification할 뿐 bounded work가 계속되지 않으면 Discovery나 Checkpoint를 제조하지 않는다.
이 두 경로는 empty Candidate를 success로 인정하지 않고 기존 inspect/record/read-only operation을
사용한다.

### Learning participation, assessment와 Deliberation

Materiality Review는 bounded work/session에 `inactive` 또는 `active` learning participation을
보존한다. Active는 exact current-host user-turn Source와 그 turn에 포함된 non-empty verbatim
statement가 필요하다. Proficiency, behavior, conversation style, prior choice나 explanation depth로
추론하지 않는다. Review revision은 state를 바꿀 수 있지만 Project/Goal/baseline 밖의 영구
preference로 승격하지 않는다.

각 discovered dimension은 authority와 별도로 `routine` 또는 `deliberation-worthy` learning value를
가진다. Deliberation-worthy는 consequence significance, future engineering problem에 대한
transferability와 non-obvious trade-off의 bounded evidence를 모두 요구한다. Credible alternatives는
Engineering Choice Discovery에서 온다. User-owned dimension은 learning value와 무관하게 기존
Question/Decision path를 사용한다. Settled fact/contract와 routine detail은 learning blocker를 만들지
않는다.

한 번 `deliberation-worthy`로 기록된 dimension은 agent가 implementation을 선택했다는 이유만으로
`routine`으로 낮출 수 없다. Downgrade는 exact dimension에 bind된 supported revision basis를 요구하고
prior/current assessment와 revision Analysis Snapshot을 Candidate에 보존한다. Supported basis는 current
Source-backed repository/research evidence가 credible trade-off를 제거한 경우, current Source-backed
prototype evidence가 uncertainty를 routine fact로 해소한 경우, 또는 exact current-host user-turn의
verbatim statement로 user가 learning participation을 withdraw/narrow한 경우다. 이 revision은 user
Decision을 만들지 않으며 agent preference, selected implementation이나 Learning Deliberation 회피는
basis가 아니다. Unsupported revision은 mutation 전에 거부하므로 prior deliberation-worthy state와
pending Learning Deliberation route가 유지된다.

Materiality draft와 workflow guidance는 이 독립성을 `authority_learning_routing`으로 기계 판독
가능하게 노출한다. Learn, alternatives 비교, implementation 전 reasoning 또는 학습을 위한
implementation approach 선택 요청 자체는 user-owned product authority의 증거가 아니다. Active
agent가 credible alternatives의 material-consequence counterfactual과 exact authority를 의미적으로
평가한다. Production은 typed provenance, identity, scope, freshness, lifecycle과 allowed transition만
결정적으로 검증하며 keyword/regular-expression ownership detection, prompt classifier 또는 provider
semantic classifier를 사용하지 않는다.

Active participation에서 agent-owned 또는 explicitly delegated agent choice가
deliberation-worthy일 때만 exact review dimension에 bind된 `Learning Deliberation` Session
Candidate가 필요하다. 최초 `awaiting_initial_response` state는 problem, established facts, discovered
choices, alternatives와 consequences를 제공하지만 round나 agent recommendation을 포함하지 않는다.
이 stage의 workflow guidance는 learner selection을 bounded learning/implementation basis이자
`canonical_decision: false`로 표시하고, 그 selection만 기록하기 위해
`candidate_manage.submit_question_from_materiality` 또는 `decision_record`를 사용하지 말라고
명시한다. 반대로 genuine user-owned material outcome은 active learning 여부와 무관하게 처음부터
Question/current-host Decision path에 남는다.
Transition은 다음 ordering을 따른다.

1. current-host user가 select, delegate, skip 또는 research/prototype를 요청하고 rationale를 선택적으로 남긴다.
2. select 뒤에만 agent feedback과 recommendation을 기록할 수 있다.
3. feedback 뒤 selected alternatives를 current bounded implementation basis로 complete하거나 explicit
   current-host reconsideration으로 다시 연다.

Delegate와 skip은 즉시 terminal non-Decision state이며 research/prototype request는 evidence-required
state다. 어느 learning-only state도 canonical Decision을 만들거나 user-owned authority blocker를
해결하지 않는다. Pending/feedback/reconsideration state는 restart 뒤에도 learning blocker이고,
completed/delegated/skipped만 affected work를 해제한다. Durable lesson이 Recall에 필요하면 별도 explicit
user Source를 가진 기존 `Context Item` role `Learning`을 사용하며 Candidate content에서 permanent
learner profile을 추론하지 않는다.

User-owned dimension을 Question Candidate로 옮길 때 review의 Source와 affected scope를 재사용하고
stable dimension scope token을 추가할 수 있다. 그 helper는 Candidate만 만들며 research,
duplicate/materiality readiness, promotion, frontier, explicit current-host response와 Decision
invariant를 우회하지 않는다. Resolution Decision은 same dimension token과 current applicability,
exact current-host response provenance를 모두 만족해야 한다.

`DelegatedImplementationChoice` label만으로는 authority가 생기지 않는다. Current-task
경로는 unrelated 또는 stale user turn, Goal basis에 없는 Source, agent-authored Context,
recommendation, convention과 implementation preference를 거부한다. Inquiry-time 경로는
delegation Decision과 exact response provenance를 계속 요구한다. Accepted contract나
일반 Decision으로 이미 정해진 outcome은 `SettledAuthority`로 남으며 delegated
disposition으로 재분류하지 않는다.

Current-task explicit-delegation evidence는 Goal Context identity, exact current-host user-turn
Source identity, non-empty bounded verbatim statement와 bounded affected scope를 하나의 typed
meaning으로 보존한다. 같은 statement가 여러 dimension을 실제로 포괄하면 각 dimension이 그
shared evidence와 포괄 scope를 inspectable하게 연결하고, 그렇지 않으면 dimension마다 별도
evidence가 필요하다. Research/prototype evidence, accepted contract, applicable Decision,
agent recommendation, library/convention과 implementation preference는 이 evidence를 대신하지
않는다. Inquiry-time delegation Decision과 current-task evidence를 같은 dimension authority로
혼합하지 않는다.

Production은 exact current user provenance, verbatim inclusion, current Goal identity,
freshness, dimension/work scope containment과 상충 authority representation 부재를 결정적으로
검사한다. 이 검사는 arbitrary natural-language prose가 semantic하게 진짜 위임인지 판정하는
classifier가 아니다. Provider call, keyword/regular-expression grammar, English-only heuristic 또는
숨은 semantic inference를 authority로 사용하지 않는다. Frozen task의 해당 문구가 실제로 그
choice를 위임하는지에 대한 semantic quality는 naturalistic Phase 8 evaluator가 관찰한다.

## 5. Question presentation

Frontier의 각 Question은 최소 다음을 함께 표시한다.

- Question identity와 displayed revision
- why it matters now와 material scope
- established facts와 Source/capability/freshness
- displayed alternatives와 각 consequence
- agent recommendation과 recommendation Source basis
- trade-offs, uncertainty, known limits와 omitted evidence
- prerequisite/outcome context와 answer가 여는 다음 branch
- 선택 외의 가능한 disposition: delegation, research, prototype, deferment 또는
  out-of-scope

`Agent Recommendation`은 Question revision에 연결된 agent-authored basis이며 user
choice와 별개다. Displayed alternative와 recommendation이 바뀌면 response가 참조할
revision도 바뀐다. Recommendation batch adoption은 각 Question identity/revision과
명시적으로 연결된 user response일 때만 개별 Decision transition으로 해석한다.

## 6. Current-host User Response Source

`User Response Source`는 현재 active host interaction의 explicit user turn을 가리키는
canonical Source다. Inquiry response로 인정하려면 다음을 확인할 수 있어야 한다.

- Project, host와 active session context
- exact user-turn identity
- 응답 대상 exact Question identity와 displayed revision
- user가 실제로 제출한 bounded response basis
- response time/order와 adapter observation provenance

Past conversation, inferred preference, agent paraphrase, 다른 interface의 미연결 input,
provider output과 일반적인 “좋아요”는 이 Source를 대신하지 않는다. 같은 user turn이
여러 Question에 명시적으로 답할 수는 있지만 각 Question identity/revision linkage를
독립적으로 확인한다.

## 7. Response Interpretation과 Decision 생성

`Response Interpretation`은 user-turn Source를 Question의 displayed alternatives 또는
terminal disposition에 mapping하는 bounded inquiry meaning이다. 새 core entity가
아니며 user text를 rewrite하거나 user rationale를 생성하는 authority가 아니다.

### Decision이 되기 위한 조건

User response가 canonical `Decision`을 만들려면 다음을 모두 만족한다.

- exact current-host User Response Source가 존재함
- Source가 exact current Question identity와 displayed revision을 가리킴
- Question이 아직 response를 받을 수 있고 Project/scope가 일치함
- explicit choice 또는 explicit delegation을 모호하지 않게 mapping할 수 있음
- user rationale가 있다면 응답과 구분하여 그대로 Source-linked basis로 보존함
- 당시 displayed alternatives, Agent Recommendation, uncertainty와 Source basis를
  추적할 수 있음
- resulting Decision applicability와 Question outcome이 일치함

Explicit choice는 Question outcome `answered`와 `answers` relation을 만든다. Explicit
delegation은 delegation Decision과 `delegated` outcome을 만들 수 있다. Research,
prototype 필요, deferment, scope exclusion과 supersession은 Question을 terminal로
만들 수 있지만 user choice/delegation 조건이 없으면 Decision으로 위조하지 않는다.

### Rejection

다음 input은 canonical transition 없이 거부하거나 clarification Candidate로 남긴다.

- displayed revision보다 오래된 stale response
- Question identity가 없거나 다른 Project를 가리키는 response
- 여러 alternative/outcome으로 해석 가능한 ambiguous response
- 이미 terminal/superseded된 revision에 대한 response
- agent recommendation을 user response로 재전송한 input
- host/session/user-turn provenance를 확인할 수 없는 input

Rejection은 Source/Decision/Question 일부만 성공한 것처럼 보고하지 않는다. 새 current
revision을 다시 표시하거나 ambiguity를 명확히 해 달라고 요청할 수 있지만 기존
Question의 의미를 추측으로 바꾸지 않는다.

## 8. Abstract atomic response boundary

한 response가 Decision을 만드는 logical operation은 최소 다음 네 meaning을 하나의
atomic boundary로 취급한다.

1. current-host User Response Source의 존재와 linkage
2. exact Question revision에 대한 Response Interpretation
3. provenance/applicability를 가진 Decision 생성
4. Question의 `answered` 또는 `delegated` transition

모두 관찰 가능하게 성공하거나 어느 것도 canonical success로 남지 않아야 한다.
Crash/retry 뒤 같은 response가 duplicate Decision을 만들거나 Question만 terminal로
남아서는 안 된다. User turn 하나가 여러 Question에 답하면 각 Question 단위의
atomic meaning과 전체 결과를 구분해 보고한다.

이 요구는 transaction, API, storage, event, lock 또는 idempotency mechanism을
선택하지 않는다. Production implementation은 이 invariant를 atomic commit 또는
동등하게 검증된 repair/idempotency behavior로 만족하고 later failure validation에
근거를 남겨야 한다.

### Canonical forgetting 중 Candidate boundary

Canonical Source 또는 Question forgetting이 commit되면 Inquiry owner는 전달받은
`CanonicalInvalidation`과 관련된 Candidate content를 idempotently cleanup한다. Cleanup과
destructive residue post-check가 아직 완료되지 않았으면 Candidate Inspection read basis는
관련 Candidate identity를 explicit withholding으로 표시하고 content, question research
basis와 bounded summary를 반환하지 않는다. 이 read barrier는 Candidate disposition을
조용히 바꾸지 않는다. 관련 Question Candidate promotion은 repair가 끝날 때까지
차단되며 unrelated Candidate lifecycle과 content는 유지된다.

## 9. Terminal outcome

Canonical Question의 terminal outcome vocabulary는 다음과 같다.

| Outcome | 의미 | Decision requirement |
|---|---|---|
| `answered` | user가 exact current revision에 명시적 choice를 제공함 | user Decision 필수 |
| `delegated` | user가 bounded choice를 agent/implementation owner에게 명시적으로 위임함 | delegation Decision 필수 |
| `resolved_by_research` | repository/environment Source가 선택 질문 없이 필요한 fact를 해결함 | user Decision을 만들지 않음 |
| `requires_prototype` | 대화/현재 evidence로 결정할 수 없어 prototype evidence branch로 전환함 | user Decision을 자동 생성하지 않음 |
| `deferred` | 이유, scope와 revisit basis를 보존하고 현재 답변을 의도적으로 미룸 | user Decision을 자동 생성하지 않음 |
| `out_of_scope` | 현재 Project/work scope에서 다루지 않기로 branch를 닫음 | user Decision을 자동 생성하지 않음 |
| `superseded` | upstream outcome, 동일 material meaning 또는 후속 Question/Decision이 이 branch를 대체함 | 대체 basis가 필요하며 user Decision을 자동 생성하지 않음 |

`answered`는 accepted Q1의 decision branch를 canonical Question outcome vocabulary로
표현한 이름이며 새 product choice가 아니다. Terminal outcome은 Question이 현재
답변을 기다리지 않는 이유다. 모든 outcome이 같은 dependent branch를 열거나 모든
outcome이 Decision을 만든다는 뜻이 아니다.

## 10. Inquiry round, persistence와 pause/resume

한 Inquiry round는 같은 frontier에서 표시한 independent Question들과 그 response,
research 또는 disposition 처리의 bounded unit이다.

- 다음 frontier를 표시하기 전에 각 accepted transition과 Source basis를 canonical로
  보존한다.
- Partial round는 성공한 Question과 실패/거부된 Question을 구분하며 전체 성공으로
  표현하지 않는다.
- User가 pause하면 current canonical Question/Decision state를 먼저 보존한다.
- Resume은 persisted frontier list를 replay하지 않고 current canonical state에서
  frontier를 deterministic하게 재계산한다.
- Already `answered`, `delegated` 또는 다른 terminal outcome인 material meaning을
  wording만 바꾸어 다시 묻지 않는다.
- Question/round 수에는 고정 상한을 두지 않는다. 모든 material branch가 terminal이
  되거나 explicit pause할 때까지 진행할 수 있다.

Process/session 종료 뒤에도 exact Question revisions, Source linkage, Decisions,
terminal outcomes와 unresolved dependencies를 읽어 같은 frontier를 복구할 수 있어야
한다.

## 11. Decision applicability

Inquiry는 `domain-model.md`의 Decision identity와 validity를 사용하며 다음 basis를
함께 평가한다.

- Project
- path, component 또는 work context scope
- explicit assumptions
- Source basis와 freshness/availability
- 당시 alternatives, Agent Recommendation과 user rationale
- expected consequence, known uncertainty와 known limit
- revisit trigger
- `supersedes`/`superseded_by`, contradiction과 `review_due` state

Projection omission, access frequency, 오래된 display time와 cache eviction은 Decision
validity를 바꾸지 않는다. Source가 stale/unavailable한 경우 historical Decision을
rewrite하지 않고 applicability uncertainty 또는 `review_due` basis로 제시한다.

## 12. Decision reuse와 re-questioning

Active Decision은 같은 Project, declared scope와 assumptions가 유지되고 Source basis에
unresolved conflict가 없으며 revisit trigger가 충족되지 않았을 때 재사용한다. 이미
선택한 방향의 직접적인 구현 세부사항을 다른 표현으로 다시 묻지 않는다.

다음 경우에는 reason과 changed basis를 제시한 새 Question으로 재검토할 수 있다.

- user가 explicit review를 요청함
- Project, path, component 또는 work context scope가 material하게 달라짐
- assumption 또는 Source basis/freshness가 material하게 달라짐
- declared revisit trigger가 충족됨
- repository/Context evidence와 Decision이 충돌함
- 후속 conflicting Decision 또는 supersession이 존재함
- 실제 consequence가 중요한 expected consequence와 다름

Preference inference, model change, access frequency와 새 agent session만으로 다시
묻지 않는다. Re-questioning이 user choice를 바꾸면 새 Decision이 기존 Decision을
supersede하며 in-place correction으로 숨기지 않는다.

## 13. Checkpoint interaction

Checkpoint는 pause/handoff에서 다음을 관찰해 기록할 수 있다.

- 해당 시점의 open Question identities/revisions
- 당시 계산된 frontier와 blocked dependency summary
- applied Decisions, research/prototype branch와 next meaningful step

이 목록은 `records_state_of` observation이며 Inquiry frontier의 두 번째 authority가
아니다. Resume은 Checkpoint의 frontier를 그대로 활성화하지 않고 canonical
Question/Decision/dependency에서 재계산한다. Checkpoint와 recomputed state가 다르면
Checkpoint를 rewrite하지 않고 snapshot/freshness 차이를 설명한다.

Inquiry round persistence는 Checkpoint 생성 여부에 의존하지 않는다. 단순 질문
표시나 response rejection만으로 meaningful work Checkpoint를 만들지 않는다.

Grounded Checkpoint가 repository change를 기록할 때 exact retained pre-write Analysis
Snapshot의 `Included` file fingerprint와 current compatible Analysis Snapshot을 비교한
bounded repository delta를 사용한다. Baseline dirty path는 별도 pre-existing evidence이며,
그 path가 baseline 뒤 다시 바뀌면 delta에 포함될 수 있고 바뀌지 않으면 포함되지 않는다.
이 관찰은 actor/process의 exclusive ownership을 주장하지 않는다. Exact baseline이
missing, stale, freshness-unknown, wrong-Project 또는 incompatible-source이면 attribution을
거부하며 current state나 post-work snapshot으로 추정하지 않는다.

같은 operation은 current Goal과 exact baseline에 bind된 retained Materiality Review를 찾아
current changed path/Decision applicability로 work authority를 다시 평가한다. Missing review,
wrong Goal/baseline, late first review, unresolved user-owned dimension, active pending Learning
Deliberation 또는 unfinished research/prototype는 Checkpoint publication 전에 거부한다. Resolved user-owned outcome과
Inquiry-time explicit delegation에 사용된 applicable Decision은 Checkpoint의 applied Decision
목록에도 명시되어야 한다. Current Goal의 exact user-turn Source와 typed verbatim/scope
evidence로 이미 주어진 bounded implementation delegation은 새 Decision을 제조하지 않고
explicit-delegation satisfied requirement로 남는다. Settled contract/repository fact/research
basis는 Decision이나 delegation으로 가장하지 않고 각자의
satisfied requirement로 남는다. Restart 뒤에도 Candidate revision과 canonical state에서 같은
evaluation을 재구성하므로 pause나 prior attempt가 unresolved authority를 완료로 바꾸지 않는다.
같은 evaluation은 learning Candidate revision/state도 읽으므로 restart나 Checkpoint attempt가 pending
learning opportunity를 ready로 바꾸지 않는다. Completed selection은 bounded implementation basis로,
delegate/skip은 non-Decision terminal state로 구분한다.

Executed verification fact는 existing `source_id`로 정확히 하나의 Command Source에
연결된다. Current host가 보낸 bounded label은 presentation이고 exact transient invocation은
trusted operation 안에서 SHA-256 fingerprint를 derive하는 input이다. Checkpoint나 Inquiry는
label을 execution correlation key로 사용하지 않으며 raw invocation을 canonical record,
Candidate 또는 resume state로 보존하지 않는다. `not_run`은 이 execution input이나 Source를
가질 수 없고, `passed`는 같은 Source의 `exited`/numeric `0` outcome을 요구한다.

## 14. Later-validation hooks

### V09 — Recall과 Checkpoint 정확성

V09는 최소 다음을 검증해야 한다.

- fresh session에서 canonical state로 frontier/Decision basis 복구
- terminal/answered Question 비반복과 deterministic ordering
- Checkpoint frontier listing과 current recomputation의 authority 분리
- stale Source, superseded Decision과 unresolved prerequisite 표시
- pause/handoff persistence와 unrelated greeting에서 inquiry/Recall 비개입
- work/verification/review/acceptance와 Question outcome의 독립성

### V11 — Combined journey

V11은 실제 repository task에서 다음을 결합 검증해야 한다.

- repository/environment fact research before ask
- Question Candidate와 canonical Question promotion boundary
- material Question이 실제 발생한 경우의 multi-round dependency frontier와 관련
  terminal outcome
- exact current-host turn/revision response와 stale/ambiguous rejection
- response Source/interpretation/Decision/Question atomicity와 retry behavior
- Decision applicability reuse와 evidence-driven re-questioning
- session/process restart 뒤 pause/resume와 Checkpoint non-authority
- Question 없음, research resolution, already-delegated choice reuse,
  prototype/research/deferment와 genuine user-owned Decision이 각각 적절한 task에서
  나타나는 behavior-class qualification; 모든 cycle에 동일 Question/Decision을 요구하지 않음

Automatic discovery와 materiality quality를 검증하지 않은 상태에서 완전한 Question
coverage를 주장하지 않는다.

## 15. Non-goals

이 문서는 Question generation model, materiality ranking algorithm, parser, provider,
database, serialized field, transaction mechanism, API, MCP method, host UI와 wire
representation을 선택하지 않는다. Portable conflict/merge, generated-document
rendering과 legacy workflow도 정의하지 않는다. General failure/recovery matrix는
[Failure와 Recovery 계약](failure-and-recovery.md)이 소유한다.
