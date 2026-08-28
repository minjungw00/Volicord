# Failure와 Recovery 계약

- 상태: active specialized architecture owner
- 소유 범위: cross-subsystem failure/degradation state, read/write consequence,
  propagation, retry, repair/rebuild, canonical transaction과 projection failure 구분,
  Guarded confirmation/execution failure, process/long-operation observation과 recovery
- 상위 architecture 기준: [논리 아키텍처](architecture.md)
- core domain 기준: [핵심 도메인 모델](domain-model.md)
- analysis 기준: [Repository Intelligence 계약](repository-intelligence.md)
- portable 기준: [Portable Context 계약](portable-context.md)
- version 기준: [Versioning 정책](versioning-policy.md)
- validation 기준: [기술 검증 계획 V08·V10·V11](validation-plan.md)
- 비소유 범위: normal entity/capability meaning, concrete storage/process/filesystem
  technology, retry algorithm, timeout value, supervisor topology와 merge algorithm

이 문서는 failure를 empty success나 다른 subsystem의 실패로 바꾸지 않고, 사용자가
무엇을 계속 읽고 신뢰하며 누가 어떤 recovery를 수행해야 하는지 정의한다. 상태는
identity, affected scope, Source/operation basis와 함께 표현하며 system-wide boolean으로
일반화하지 않는다.

## 1. Failure invariant

1. Canonical mutation은 전부 commit되거나 canonical success가 남지 않는다.
2. Projection, analyzer, provider, index, renderer와 process failure는 Canonical Context를
   조용히 수정·삭제·복구하지 않는다.
3. Partial/degraded result는 usable remainder와 omitted/failed scope를 함께 제공한다.
4. Retry는 실패한 operation의 owner가 판정하며 caller가 duplicate canonical mutation을
   만들지 않는다.
5. Rebuildable Derived State의 corruption/loss를 canonical loss로 확산하지 않는다.
6. Repair와 rebuild는 구분하며 어느 것도 user Decision이나 correction을 되돌리지
   않는다.
7. Failure result는 user-visible consequence, retry owner와 next safe action을 가진다.
8. Process observation은 complete stdout, stderr, exit/termination과 duration을 잃지
   않는다.
9. Guarded effect는 valid exact-match confirmation의 pre-dispatch validation보다 먼저
   dispatch되지 않는다.
10. Checkpoint verification execution은 presentation label이 아니라 trusted operation이
    transient exact invocation에서 derive한 fingerprint와 observed exit/termination으로
    correlate하며 raw invocation을 durable process evidence로 보존하지 않는다.

## 2. State matrix

아래 state는 cross-subsystem operation과 information health의 공통 vocabulary다.
Repository capability나 domain lifecycle owner가 더 좁은 meaning을 정의하면 이 문서는
그 state가 subsystem boundary를 지날 때의 consequence만 소유한다.

| State | Owning subsystem | Information class | Read allowance | Write allowance | User visibility | Retry owner | Rebuildability | Operation-failure effect |
|---|---|---|---|---|---|---|---|---|
| `unavailable` | 현재 Source/capability/resource를 여는 subsystem | availability metadata 또는 operation result; underlying canonical/derived identity는 유지 | historical canonical과 unaffected result read 허용, unavailable content read 금지 | unavailable basis에 의존하지 않는 scoped write만 허용; freshness를 확인한 것처럼 쓰기 금지 | missing resource, affected scope, usable remainder와 bind/configure action 표시 | resource를 획득·bind·invoke하는 Local Operations 또는 producing subsystem | Source는 rebuild 대상 아님; Derived는 basis가 돌아오면 rebuild 가능 | affected operation만 미수행/실패; aggregate는 degraded/partial로 계속 가능 |
| `unsupported` | format/capability meaning owner | capability/format diagnostic; canonical content가 아님 | opaque identity/metadata와 unaffected content read 허용 | unsupported meaning을 drop/coerce한 write 금지 | detected kind/version/scope와 supported alternative 표시 | 자동 retry 없음; owner upgrade 또는 capability implementation 필요 | Derived는 supported producer가 있으면 rebuild, canonical은 supported upgrade 필요 | requested scope 실패; unsupported를 empty success로 보고 금지 |
| `partial` | bounded result를 생산한 subsystem | Derived result, projection 또는 aggregate operation result | provenance가 완전한 usable subset만 read 허용 | subset을 complete basis로 canonical write 금지; independent successful writes는 명시적으로 유지 가능 | completed/failed/omitted scope와 coverage 표시 | failed child scope의 owning subsystem | 입력이 있으면 affected Derived subset rebuild 가능 | aggregate는 partial이며 complete success가 아님; successful independent outcomes는 보존 |
| `failed` | operation을 실행한 subsystem | operation result와 diagnostic; partial artifact는 별도 분류 | 이전 committed/current state와 verified partial result만 read 허용 | failed operation의 uncommitted write 금지 | exact operation, scope, error, outputs, exit/termination과 retry consequence 표시 | operation owner가 idempotency와 precondition 확인 후 결정 | Derived operation은 가능하면 rebuild; canonical failure는 retry/repair 전 새 success 없음 | 해당 operation 실패, aggregate는 나머지를 계속하되 failure count 보존 |
| `stale` | Source/analysis/projection owner | Source availability, Derived result 또는 projection freshness metadata | historical basis임을 표시한 read 허용, current fact로 read 금지 | stale basis로 validity/coverage를 current로 갱신하는 write 금지 | old/current snapshot basis와 refresh/review action 표시 | source observer 또는 derived producer | Derived는 refresh/rebuild 가능; canonical history는 rebuild하지 않음 | current-grounded request는 degraded/partial; historical inspection은 성공 가능 |
| `contradicted` | Canonical Context Kernel이 relation/state meaning을 소유 | Canonical lifecycle state와 provenance-bearing review Candidate | 양쪽 claim/Decision과 Source read 허용 | automatic overwrite/resolve 금지; explicit correction, evidence, supersession 또는 user review만 허용 | conflict identities, basis, applicability consequence와 unresolved state 표시 | Inquiry/user review owner; analyzer/provider 자동 retry 대상 아님 | rebuild 불가; 새 evidence/semantic resolution 필요 | current applicability를 단정하는 operation은 blocked/degraded, unrelated operation은 계속 |
| `degraded` | aggregate/health를 제공하는 Local Operations 또는 projection owner | operational health 또는 aggregate projection | declared usable capability와 canonical read 허용 | degraded dependency와 무관한 owner-validated write 허용 | lost capability, cause, safe remainder와 recovery owner 표시 | 원인 subsystem owner; aggregate layer는 retry를 대신 결정하지 않음 | 원인이 Derived/resource면 가능, canonical semantic conflict면 불가 | 전체 실패가 아니며 requested guarantee를 축소한 success도 아님; degraded outcome으로 종료 |
| `corrupt` | 해당 durable/derived format owner | integrity diagnostic과 quarantined state | 검증된 unaffected partition만 read; corrupt bytes를 domain record로 read 금지 | corrupt state에 in-place normal write 금지 | affected format/scope, integrity evidence와 data-at-risk 표시 | format owner가 repair/rebuild 판단 | Derived는 canonical/source basis에서 rebuild; canonical은 검증된 repair/restore만 가능 | affected operation 실패, quarantine하며 empty/fresh state로 가장 금지 |
| `repair_required` | invariant를 복구해야 하는 authoritative owner | operation/health state; repair evidence는 provenance를 가짐 | owner가 정의한 safe read-only subset만 허용 | 일반 write 중단; validated repair operation만 허용 | invariant, affected scope, repair owner, risk와 post-check 표시 | authoritative subsystem/Local Operations가 조정 | Derived는 대개 rebuild; canonical은 deterministic repair만 가능 | dependent mutation 실패/차단, unrelated subsystem read/operation은 격리해 계속 가능 |

State를 전환할 때 prior state와 evidence를 지우지 않는다. `degraded`는 aggregate
consequence이고 원인 state를 대체하지 않는다. `repair_required`는 repair 명령이
존재한다는 제품 API 약속이 아니라 normal write가 안전하지 않은 invariant 상태다.

## 3. Canonical mutation failure와 projection failure

Canonical mutation과 projection은 서로 다른 authority와 success boundary를 가진다.

### Canonical mutation failure

- Kernel invariant validation, transaction, durable publication 또는 atomic response
  boundary가 완료되지 않으면 mutation은 `failed`다.
- Question response처럼 여러 canonical meaning을 묶는 operation은 Source linkage,
  interpretation, Decision과 Question transition이 모두 성공하거나 아무 canonical
  success도 남지 않아야 한다.
- Caller에게 response를 render하지 못했더라도 commit이 성공했다면 canonical mutation
  success와 response projection failure를 각각 보고한다. 같은 mutation을 blind retry해
  duplicate record를 만들지 않는다.
- Commit 여부를 확인할 수 없으면 success로 추측하지 않고 `repair_required` 또는
  operation-specific indeterminate diagnostic로 authoritative owner의 확인을 요구한다.

### Projection failure

- Recall selection, map, document generation, preview, render 또는 export presentation이
  실패해도 input canonical record와 Repository work result는 그대로다.
- Projection이 일부 section만 만들면 `partial`과 omission/failure scope를 보고하며
  complete document로 adoption하지 않는다.
- Projection retry/regeneration은 source snapshot, canonical read revision과 generator
  basis를 다시 확인한다.
- Requested-language generated body를 실현할 수 없거나 실현 여부를 확인할
  수 없으면 affected projection/document는 `unavailable` 또는 `degraded`다.
  Requested-language metadata만 보존하거나 영어 body를 반환한 것을 성공으로
  바꾸지 않는다. Fixed UI locale fallback은 이 generated-content failure와 별개다.
- Projection 실패를 canonical transaction failure, repository work failure 또는 user
  Decision rollback으로 보고하지 않는다.

따라서 canonical commit 뒤 response projection이 실패한 경우 “작업 전체 실패”로
재시도하거나 “projection만 실패했으므로 canonical commit도 없었다”고 가정하지
않는다. 두 result identity와 retry ownership을 분리한다.

### Guarded confirmation and execution failure contract

Guarded confirmation outcome은 confirmation request/revision, user-response Source와
operation identity를 보존하며 다음 behavior를 구분한다.

| Confirmation/execution condition | Required result |
|---|---|
| `missing` | explicit response가 없으므로 `not_dispatched`; general consent나 silence로 대체하지 않음 |
| `denied` | explicit denial Source를 보존하고 `not_dispatched`; 같은 request를 승인으로 reinterpret하지 않음 |
| `stale` | request/current Candidate basis가 달라 `not_dispatched`; current revision으로 새 confirmation 필요 |
| `expired` | expiration을 지났으므로 `not_dispatched`; 새 expiration/revision의 confirmation 필요 |
| `mismatched` | action, target, expected effect, scope, revision 또는 fingerprint가 다르므로 `not_dispatched` |
| `reused` | 이미 consumed된 confirmation을 reject하고 `not_dispatched`; 다른 effect나 retry에 transfer하지 않음 |
| `dispatch_failed` | validation/consumption은 확인됐지만 dispatch 실패가 확정됨; `not_dispatched`와 consumed state를 함께 보고 blind reuse하지 않음 |
| `execution_failed` | dispatch는 확인됐고 execution failure도 확인됨; `dispatched_and_failed`로 보고 success로 바꾸지 않음 |
| `execution_outcome_indeterminate` | termination/communication loss 뒤 dispatch/effect completion을 확정할 수 없음; success를 주장하거나 silently retry하지 않음 |

Missing, denied, stale, expired, mismatched와 reused confirmation은 Guarded effect를
dispatch하지 않는다. Changed action/target/expected effect/scope/request revision은 새
confirmation 없이 mismatch를 repair할 수 없다. Confirmation transport/display failure도
approval로 해석하지 않고 Host and User Adapters가 같은 logical request를 viewer/CLI
fallback으로 전달하거나 `not_dispatched`로 끝낸다.

Confirmation consumption과 dispatched operation은 `architecture.md`의 한
`operation_identity`로 연결한다. `execution_outcome_indeterminate`이면 Local Operations는
known confirmation/dispatch observations, external target의 inspectable status와 recovery
scope를 제시한다. Outcome을 안전하게 판정할 수 있을 때만 scoped reconciliation을
수행하며, 그렇지 않으면 `repair_required`로 두고 새 confirmation을 받아도 같은 effect를
silent retry하지 않는다. Cooperative confirmation은 이 recovery contract를 OS sandbox나
security enforcement 보증으로 바꾸지 않는다.

## 4. Failure propagation matrix

| Failure | Immediate owner/state | Preserved behavior | Forbidden propagation | Recovery owner/action |
|---|---|---|---|---|
| one language analyzer failure | Repository Intelligence, affected language/area `failed` 또는 `partial` | inventory, other languages/areas, prior historical snapshot와 Canonical Context | repository-wide empty success나 total canonical failure | adapter/Repository Intelligence retry; scope-specific reanalysis |
| semantic-provider unavailable | Provider Boundary `unavailable`, provider-backed capability `degraded` | local inventory/structural, Inquiry, Decision, Checkpoint, Recall과 canonical read/write | opt-in 확대, local core failure 또는 old annotation을 current로 표시 | Repository Intelligence/Local Operations가 config/availability 뒤 explicit retry |
| Derived Index corruption | index owner `corrupt`, dependent search/projection `degraded` | Canonical Context, Source basis와 non-index read | canonical loss, corrupt index query 또는 silent empty index | owning subsystem이 quarantine/delete 후 rebuild; Local Operations가 progress 관찰 |
| Canonical transaction failure | Kernel `failed` 또는 commit 불명 시 `repair_required` | last verified committed state와 unrelated derived/history read | partial canonical success, projection-only degradation으로 축소 또는 blind retry | Kernel이 commit state 확인, idempotent retry/validated repair |
| Canonical forgetting cleanup interruption | Local Operations의 durable operation이 `prepared`, `canonical_committed`, `repair_required` 또는 `completed`를 보존 | tombstone이 확인된 뒤 관련 Candidate/managed Derived content는 read barrier로 withheld; unrelated record는 read 허용 | local cleanup/post-check 전 complete success, 관련 Candidate promotion 또는 provider deletion success 추론 | Local Operations가 같은 operation identity와 invalidation으로 owner cleanup과 residue post-check를 reconcile |
| bundle conflict | Portable Context의 named conflict; unresolved operation `partial`/`failed` | both histories, common-base/lineage와 unaffected additions | Decision/Question/delete-modify silent overwrite | user-owned resolution 또는 branch; Portable I/O가 result/provenance 보존 |
| document-generation failure | Projections and Documents `failed`/`partial` | canonical records, analysis, ordinary work와 prior adopted Sources | repository work/Checkpoint rollback 또는 failed draft 자동 adoption | Projection owner가 same/current basis 확인 후 retry/regenerate |
| Recall-projection failure | Projections and Documents `failed`/`partial` | direct canonical inspect, existing Question/Decision/Checkpoint와 ordinary work | hidden fallback memory를 authority로 사용하거나 canonical mutation | Projection owner retry; user에게 unavailable sections와 direct inspection path 제공 |
| Candidate Inspection failure | Projections and Documents가 affected `candidate_inspection` scope와 `unavailable`/`unsupported`/`corrupt`/`repair_required`/`failed` root cause를 보존한 `failed`/`partial` 또는 `degraded` | 검증된 Candidate identity/lifecycle data와 canonical state; direct scoped inspection | Candidate-empty success, Candidate promotion, deletion, disposition/retention rewrite 또는 interpretation change | Projection owner가 read basis를 확인해 retry; privacy/domain owner만 explicit mutation 수행 |
| Checkpoint work authority discovery/review missing/stale/late/unresolved | Inquiry/Local Operations `failed`; no Checkpoint publication | Engineering Choice Discovery와 Materiality Review Candidate, canonical Goal/Question/Decision, Analysis Snapshot과 ordinary repository work | pause/completion success 추론, broad Goal authority, post-work discovery/review backfill, recommendation/convention을 authority로 승격 | missing discovery/review는 새 bounded work의 pre-work boundary에서만 기록; every-choice mapping, exploratory evidence나 existing Question/Decision lifecycle을 완료하고 same Goal/baseline review를 revise한 뒤 retry; late first review는 이미 수행한 work를 retroactively repair하지 못함 |
| required Learning Deliberation missing/pending/reconsidered | Inquiry/Local Operations `failed`; affected work는 `learning_deliberation_pending`, no Checkpoint publication | authority result, exact discovery/review basis, current Candidate rounds와 explicit participation Source | inactive로 추론, user-owned blocker 대체, pre-response recommendation 삽입, restart를 terminal로 취급 | exact dimension Candidate를 begin/continue; select 뒤 feedback과 completion, delegate/skip terminal 또는 bounded research/prototype 후 readiness 재평가 |
| Guarded confirmation invalid/unavailable | Host/Adapter 또는 Local Operations `failed`; operation `not_dispatched` | ordinary non-Guarded work와 exact request/response observation | fallback 없이 approval 추론, invalid confirmation consumption 또는 Guarded dispatch | Host가 viewer/CLI fallback; Local Operations가 exact new confirmation 전 dispatch 차단 |
| Guarded execution indeterminate | Local Operations `repair_required` 또는 operation-specific indeterminate | request/response/consumption/dispatch observation과 unaffected work | success claim, silent retry 또는 confirmation reuse | scoped external-state reconciliation; 안전하게 판정 못 하면 repair_required 유지 |
| repository-scoped Codex activation conflict/failure | Host/Adapter setup `failed`; repository and canonical state unchanged | unrelated project config/hooks, Runtime Home와 other repositories | tracked/project-owned config overwrite, global fallback registration, silent MCP disable 또는 automatic trust | user가 named conflict를 해소한 뒤 exact enable retry; broken required MCP는 Codex startup/resume에서 visible failure |
| forced process termination | Local Operations/process owner `failed`, termination recorded | committed state, complete captured streams와 unaffected subsystem state | exit success 추정, child 상태 은폐 또는 partial canonical publish | process owner가 child cleanup/commit check 뒤 scoped retry; V10 technology 선택 |
| unavailable source repository | Local Operations/Source `unavailable` | source-independent canonical read, Inquiry history, Decision와 Checkpoint | fabricated current code link/freshness나 Project loss | explicit bind/rebind; Repository Intelligence refresh after availability |
| unsupported format version | Versioning owner `unsupported` | original bytes/state와 safely inspectable metadata | partial import, field dropping, empty initialization 또는 write-back | format owner의 supported upgrade, Derived rebuild 또는 newer implementation |

각 propagation result는 root cause, affected scope, downstream degradation과 independent
success를 함께 보존한다. Aggregate caller는 모든 bounded child outcome을 모은 뒤
failure/partial/degraded를 판정하며 첫 실패에서 나머지 독립 작업을 조용히 생략하지
않는다.

## 5. Repair와 rebuild

`repair`는 authoritative 또는 durable state가 invariant를 만족하도록 검증된
transformation/cleanup을 수행하는 일이다. `rebuild`는 disposable Derived State를
Canonical Context와 Source basis에서 다시 계산하는 일이다.

| Responsibility | Repair | Rebuild |
|---|---|---|
| Typical target | canonical transaction marker, supported schema publication, binding/integrity metadata, managed durable artifact | index, graph, embedding, cache, layout, Analysis Snapshot, generated preview |
| Required basis | authoritative last-known state, invariant, repair plan과 post-check | current supported Source/canonical basis와 producer version |
| Allowed semantic change | 없음; user judgment/lifecycle을 발명하지 않음 | 없음; derived meaning만 재계산 |
| Failure result | prior safe state 유지 또는 `repair_required`; success로 추측 금지 | target capability `failed`/`degraded`; canonical은 유지 |
| User visibility | affected durable scope, risk, backup/restore가 아니라 new-product repair consequence와 outcome | deleted/rebuilt scope, progress, coverage, unavailable input과 outcome |

Canonical Context를 Derived State에서 rebuild하지 않는다. User Decision, correction,
supersession, forgetting과 conflict resolution은 repair algorithm이 자동 선택할 수 없는
domain operation이다. Conversely, rebuildable index corruption에 canonical migration이나
semantic repair를 요구하지 않는다.

이미 user-authorized canonical forgetting이 commit된 operation의 recovery는 새
forgetting 의미를 선택하는 semantic repair가 아니다. Content-minimal operation record는
target identity, authorization Source identity와 `prepared` → `canonical_committed` /
`repair_required` → `completed` 진행 상태만 보존한다. Reconciliation은 tombstone을
확인한 뒤 missing Candidate/managed Derived cleanup과 destructive residue post-check만
idempotently 수행한다. `completed`는 세 owner postcondition이 모두 검증된 뒤에만
기록하고, provider deletion outcome은 local completion state에 합치지 않는다.
Local Operations는 이 invariant에 한해 durable forgetting operation identity를 받는
explicit recovery를 제공한다. CLI의 safe next action은
bound repository에서 `volicord doctor repair --forgetting <OPERATION_ID>`이며,
repository resolution을 사용할 수 없는 source-independent recovery에서는
`--project <PROJECT_ID>`를 추가한다. stored target과 authorization Source를 사용해
missing owner cleanup/post-check만 reconcile한다. Caller가 새 user fact나
provider deletion success를 제공했다고 추론하지 않는다.

## 6. Retry ownership

Retry에는 original operation identity, input/source version, idempotency/commit state,
attempt number와 prior outcome이 필요하다.

- Kernel은 canonical mutation retry와 duplicate prevention을 소유한다.
- Repository Intelligence adapter는 language/area analysis retry를 소유한다.
- Provider Boundary는 opt-in/scope를 재확인한 request retry를 소유한다.
- Projection owner는 current grounding basis의 render/generation retry를 소유한다.
- Portable Context owner는 import/export validation과 conflict result retry를 소유한다.
- Local Operations는 scheduling, cancellation과 observation을 조정하지만 lower owner의
  semantic retry safety를 추측하지 않는다.

Automatic retry는 bounded하고 같은 failure를 숨기지 않는다. User input, opt-in,
conflict judgment, unavailable resource 또는 unsupported version이 필요한 경우 반복
실행으로 해결하려 하지 않는다.

## 7. Process와 long-operation result

Analyzer, provider adapter, document renderer, export/import, rebuild, repair와 다른
long-running operation은 최소 다음 result contract를 가진다.

- exact operation identity와 requested scope
- start/end time과 monotonic `duration`
- complete `stdout` artifact와 complete `stderr` artifact를 서로 분리
- numeric `exit status` 또는 spawn failure
- signal/forced `termination` kind와 child cleanup outcome
- user/host `cancellation` request, observation time와 completion outcome
- configured `timeout`, timeout trigger와 실제 post-timeout process state
- `bounded progress`: phase/unit, completed/total 또는 total이 unknown이라는 사실,
  last update와 stalled/active state
- partial-result manifest, completed/failed/omitted units와 publication state
- retry owner, safe retry basis와 next action

Complete stdout/stderr는 maintained document나 canonical record에 무제한 저장한다는
뜻이 아니다. Full streams는 ignored managed operational artifact에 보존하고 UI/host에는
bounded preview, truncation count와 artifact reference를 제공할 수 있다. Secret/source
retention boundary는 그대로 적용한다.

Ordinary Checkpoint verification을 host가 이미 실행한 경우 durable Command Source에는
Source identity, bounded human-readable label, exact invocation의 SHA-256 fingerprint와
numeric exit/termination outcome만 남긴다. Exact invocation/raw argv는 fingerprint derivation
동안만 transient하며 long-operation artifact, canonical storage, portable bundle 또는
projection으로 복사하지 않는다. `not_run`은 execution fingerprint나 process outcome을
만들지 않고, caller가 digest를 asserted한 것만으로 observed execution을 만들 수 없다.

### Cancellation, timeout과 termination

- Cancellation intent와 process termination 완료는 별도 사실이다.
- Timeout은 종료 요청의 원인이며 actual exit/termination/child cleanup을 대신하지
  않는다.
- Forced termination 뒤 canonical commit 여부와 partial publication을 owner가 확인한다.
- Child cleanup failure를 operation completion으로 숨기지 않는다.
- Cancellation 전에 safely committed independent units가 있으면 manifest에 보존하되
  aggregate complete success로 표시하지 않는다.

현재 `openai-codex` background provider subprocess도 이 process observation primitive를
사용한다. Login-status preflight와 source-bearing execution은 서로 다른 observed child
operation이고, preflight unavailable은 source execution을 만들지 않는다. Source-bearing
execution의 timeout은 provider `timed_out`, cancellation은 `cancelled`, nonzero exit·stream
observation failure·invalid structured response는 `provider_failed`로 보존한다. Partial과
stale provider response는 process success와 별개의 normalized semantic outcome이다. Complete
stdout/stderr, final raw response와 schema는 private ephemeral artifact에서 bounded parse 후
제거하고, durable provider request에는 exit/termination/cleanup/stream byte count/duration의
content-free summary만 남긴다.

### Bounded progress와 partial result

Progress는 heartbeat만으로 success를 뜻하지 않는다. Total work를 모르면 percentage를
만들지 않고 current phase/unit과 unknown total을 표시한다. Partial result는 owning
subsystem이 independently valid하다고 판정한 unit만 publish하며 coverage와 missing
scope를 함께 제공한다.

## 8. Technology deferral

Process group/session, signal strategy, pipe capture, filesystem publication, lock,
transaction, journal, timeout primitive, child-tree cleanup과 repair implementation은 V10이
비교·검증한 뒤 선택한다. 이 문서는 process crate, database, API, command 또는 daemon
topology를 정하지 않는다.

기존 process/filesystem code는 V10의 `adopt_as_new_primitive`,
`reimplement_from_behavior`, `reference_only`, `reject` 판정과 새 responsibility test를
거치기 전 production implementation 이름으로 사용하지 않는다.

## 9. Validation hooks

### V08 — Linux install과 Codex integration

V08은 clean Linux/Codex journey에서 startup, health, adapter lifecycle, connection
failure/degradation, process cleanup, locale rendering과 unsupported/failed user-visible
result를 검증한다. Guarded request/response를 current host가 운반하고 host가 elicitation할
수 없을 때 local viewer/CLI fallback이 같은 identity/revision/Source contract를 유지하는
성질도 검증한다. Install/reinstall failure가 canonical user data를 조용히 삭제하지
않고 legacy runtime 접근으로 복구하지 않는 성질도 확인한다.

### V10 — Process/filesystem primitives

V10은 complete stdout/stderr, exit/termination, duration, cancellation, timeout, bounded
progress, child cleanup, partial publication, atomicity, corruption/repair/rebuild와 retry
ownership을 fault injection으로 검증한다. 구체적인 primitive 선택은 이 evidence 뒤에
이뤄지며 legacy workflow type이나 Wave 1 prototype을 production 이름으로 승격하지
않는다.

### V11 — Combined recovery journey

V11은 세 repository에서 analyzer/provider/index/source/process failure, canonical
transaction fault, bundle conflict, document/Recall projection failure와 unsupported
newer format을 결합해 다음을 확인한다.

- canonical mutation과 projection result 분리
- unaffected capability와 independent success 보존
- state matrix의 read/write/user visibility/retry/rebuild consequence
- restart 뒤 committed-state recovery와 duplicate prevention
- repair/rebuild 후 provenance, coverage와 user correction 유지
- long-operation result와 forced-termination child cleanup
- Guarded missing/denied/stale/expired/mismatched/reused rejection, no-dispatch-before-valid
  confirmation, exact operation outcome과 indeterminate no-silent-retry behavior
- Candidate Inspection failure가 promotion/deletion/retention/disposition mutation으로
  전파되지 않는 성질

## 10. Non-goals

이 문서는 database, transaction engine, process supervisor, signal/tree cleanup library,
filesystem atomicity primitive, retry count/backoff, timeout value, CLI/API와 repair command
catalog를 선택하지 않는다. Legacy recovery, migration, dual-runtime fallback과 parallel
production implementation은 recovery path가 아니다.
