# Privacy와 provider 경계 계약

- 상태: active specialized architecture owner
- 소유 범위: local processing, current-host interactive source access,
  background semantic-provider authority, Project opt-in, inspectable transmission,
  Candidate collection opt-out/retention, revoke와 managed deletion boundary
- 상위 architecture 기준: [논리 아키텍처](architecture.md)
- core domain 기준: [핵심 도메인 모델](domain-model.md)
- analysis 기준: [Repository Intelligence 계약](repository-intelligence.md)
- validation 기준: [기술 검증 계획 V07·V11](validation-plan.md)
- 비소유 범위: provider/model 선택, general authorization architecture, network/process
  topology, secret detector 구현, portable bundle format과 cross-subsystem recovery matrix

이 문서는 source access가 가능하다는 사실과 background transmission consent를
분리한다. Privacy setting은 provider 사용을 강제하는 동의 절차가 아니며,
local-only operation은 정상적인 first-class product mode다.

## 1. 세 authority boundary

다음 세 boundary는 서로 다른 authority와 provenance를 사용한다.

| Boundary | Authority | 허용되는 기본 scope | 필수 provenance |
|---|---|---|---|
| `local_structural_processing` | 사용자가 연결한 local Project와 local operation intent | repository inventory, local structural/ecosystem analysis, canonical operation | local observer/analyzer, Repository/Analysis Snapshot, source scope, operation time와 diagnostics |
| `interactive_current_host_access` | 현재 active host interaction에서 사용자가 요청한 작업과 host가 이미 부여받은 source access | 그 interaction을 위한 bounded read, explanation과 agent-assisted interpretation | host/session, current request, accessed Source/snapshot, purpose, agent/model identity와 generated time |
| `background_semantic_provider_processing` | 별도의 explicit Project-scoped opt-in과 현재 유효한 transmission scope | inspectable scope 안의 background/batch semantic request만 | opt-in basis, provider/model, purpose, transmitted source manifest, exclusions, filtering, retention와 request/result outcome |

Local Project binding은 interactive host authority가 아니고, host가 source를 읽을 수
있다는 사실은 background provider opt-in이 아니다. 이전 interactive request,
다른 Project의 opt-in, provider credential 존재나 일반 privacy notice를 background
authority로 재사용하지 않는다.

Codex repository authorization은 그 repository에서 local Volicord MCP와 activation
context를 노출할 host-integration authority일 뿐 background provider opt-in이 아니다.
Binary/runtime install, 다른 repository의 authorization, Codex project trust 또는
SessionStart 실행을 source transmission consent로 재사용하지 않는다. Authorization
hook 자체는 Runtime Home, canonical Project data와 repository content를 읽지 않는다.

Adapter와 Local Operations는 authority kind를 손실 없이 전달한다. Repository
Intelligence만 Optional Semantic Provider Boundary를 통해 background analysis를
요청하며 provider result는 canonical write authority나 user provenance를 얻지 않는다.

## 2. Local structural processing

다음 처리는 local boundary 안에서 작동한다.

- Canonical Context의 create/read/correct/supersede/forget operation
- repository inventory와 language/source boundary observation
- 설치된 local analyzer로 가능한 structural과 ecosystem analysis
- local Source navigation, capability/coverage/freshness reporting
- local Derived State의 build, invalidate, delete와 rebuild

Local structural processing에는 외부 provider 전송이 필요하지 않다. Local analyzer가
child process이거나 별도 process라는 배치 선택은 authority를 background external
processing으로 바꾸지 않지만, 외부 endpoint로 source를 보내는 순간 background
provider boundary를 따라야 한다.

## 3. Interactive current-host source access

첫 공식 host가 현재 interaction에 대해 source access를 이미 가지고 있고 사용자가
repository 설명이나 작업을 요청한 경우, host는 그 권한 범위 안에서 source를 읽고
`agent_assisted` explanation을 만들 수 있다.

Interactive access는 다음 조건을 가진다.

- 현재 host, session과 user request에 bound된다.
- access한 Source, Repository Snapshot, purpose와 generated interpretation provenance를
  확인할 수 있다.
- host가 읽지 않았거나 access할 수 없는 scope를 covered로 주장하지 않는다.
- interactive output을 background corpus, persistent annotation 또는 provider retention
  동의로 일반화하지 않는다.
- host/model 동작의 외부성은 background provider opt-in을 대신하지 않는다. 제품은
  두 authority를 user-visible하게 구분한다.

Active-host realization의 exact model/author identity를 independent source로 검증할 수
없으면 unknown 또는 explicitly self-reported로 기록한다. Session environment identifier는
bounded correlation일 뿐 exact model/authorship attestation이 아니다. Campaign preparation의
candidate HEAD와 local MCP executable hash를 verified binding으로 보존해도 host/model에
그 verification authority를 전이하지 않는다. 이를 확인하려고 credential이나 auth file을 읽지 않는다.
Codex host가 bounded rollout의 maintained `session_meta`와 `turn_context`에서 exact source,
originator, session과 model을 제공하면 recorder는 raw bytes/hash와 일관성을 확인한 뒤
`runtime_observed`로 기록할 수 있다. 이는 임의 self-report보다 강한 host-recorded
observation이지만 realization authorship attestation이나 `verified` identity는 아니다.
Runtime observation과 충돌하는 덜 구체적 self-report는 stronger identity로 승격하지 않으며,
exact identity가 없으면 계속 `unknown`을 허용한다. Model 이름은 allowlist로 qualification하지 않는다.

Naturalistic Codex capture는 CLI와 VS Code extension에 같은 integrity contract를 적용한다.
`source`, `originator`, client/CLI version, thread/source metadata, provider/model,
session과 cwd는 immutable raw bytes 안에 그대로 보존한다. Bounded projection도 observed
value를 바꾸지 않는다. UI surface는 independent evidence가 없으면 `unknown`이며 source나
originator 문자열에서 UI/authorship를 추론하지 않는다. 이 local evidence contract는
background transmission consent를 부여하지 않으며 현재 non-Codex capture support는 없다.

Host가 제공하지 않는 권한을 Volicord가 발명하지 않는다. Current-host interaction의
구체적 UI나 wire representation은 이 문서의 계약이 아니다.

### Explicit local explanation retention

The active host may explicitly prepare canonical Work or Decision evidence and submit its
interpretation through `work explain record` or `decision explain record`. This operation expresses local
retention intent for that Project/subject/language only; it grants no background
generation or transmission consent. Preparation reads recorded Source observations,
never an arbitrary filesystem path or credential. Read/navigation/export perform
no host or provider invocation. Missing host access or generation authority remains
a blocker; local preparation does not supply it.

The existing managed CachedSummary store retains disposable prose plus exact
evidence identities/revisions/fields, Source snapshot/status and host/session,
nullable agent/model, recording time and self-reported identity status. Original
evidence content is not duplicated in the retained envelope. Canonical links include
every used record and Source, so correction invalidation and forget read barriers
apply. Explicit subject explanation deletion removes all retained languages/history and sanitizes
the local SQLite content/WAL through existing managed cleanup. Failed sanitization
reports incomplete cleanup and supports explicit retry; local success certifies
neither host memory nor provider-side deletion. No remote request was introduced.

Managed Derived `content` is bounded separately from short text metadata: nonempty,
NUL-free content bodies have an inclusive 147,456 UTF-8 byte limit. Purpose,
retention basis and other existing short fields retain their 16,384-byte limits;
ephemeral background Source body limits are independent. Managed content admission rejects oversized bodies with safe measured/allowed sizes.
Inspection/get preserve accepted complete content without imposing the short-field
limit or interpreting explanation JSON; its producer's decoder enforces the same
147,456-byte read bound and withholds oversized/corrupt prose. Metadata inspection
and deletion remain available for corrupt disposable content. An
invalid draft is rejected before ID/time allocation or SQL insertion and cannot
replace an earlier record. The explanation producer owns its preparation reserve
and compact realization limits in [Projection](projections-and-documents.md#explanation-byte-contract).
No second store, privacy schema change or new remote effect is introduced.

## 4. Background provider opt-in

Background 또는 batch semantic-provider processing은 기본적으로 꺼져 있고 다음
조건을 모두 만족할 때만 Project 안에서 활성화된다.

- explicit Project identity
- provider와 model identity
- 구체적인 analysis purpose
- 포함할 repository/source scope
- exclusions와 secret filtering policy
- retention policy와 예정된 deletion/revoke behavior
- opt-in의 current state와 user intent provenance

Opt-in은 Project-scoped다. 한 Project의 동의를 다른 Project, clone identity 또는
unrelated source에 적용하지 않는다. Source scope를 넓히거나 provider/model/purpose,
exclusion, filtering 또는 retention meaning을 material하게 바꾸는 경우 기존 동의로
조용히 처리하지 않고 inspectable update가 필요하다.

`enabled` 상태만으로 request가 허용되는 것은 아니다. 각 background request는
opt-in과 scope가 현재 유효한지 확인하고 transmitted source manifest를 남겨야 한다.
Revoke 뒤의 새 invocation은 차단한다.

첫 replacement qualification은 mock/stub과 truthful failure/degradation만으로 완료하지
않고 current production background semantic-provider path의 실제 성공을 최소 한 번
검증한다. 이 성공은 설정된 production dispatcher/transport가 실제 provider를
호출하고 authorized Source manifest에 반환 result가 연결되며 provenance,
coverage, retention와 outcome이 inspection된 경우다. 특정 provider, model 또는
transport technology를 제품 계약으로 고정하지 않는다.

Technical network availability, credential/authentication, sandbox permission과 ordinary host
access는 source transmission authorization이 아니다. Qualification invocation은 해당
Project opt-in에 더해 exact provider/purpose/Source scope의 별도 source-transmission
authorization을 요구한다. 이 authorization이 없으면 production success check는
`authorization_blocked`/`not_run`으로 남고 local-only journey나 truthful provider
degradation 결과를 실제 transmission success로 대체하지 않는다.

### Current Linux/Codex production provider realization

첫 supported production adapter의 inspectable identity는 `openai-codex`이며 model은
Project opt-in과 exact request에 기록된 explicit Codex model string이다. Transport는 설치된
authenticated Codex CLI의 bounded non-interactive `codex exec`를 재사용한다. Credential은
Codex가 소유하며 Volicord는 token, auth file 또는 credential fingerprint를 읽거나 복사하거나
저장하지 않는다. Source payload는 privacy filtering과 Guarded confirmation을 통과한 뒤에만
stdin으로 전달되고 argv, repository file, portable context 또는 maintained evidence에는 넣지
않는다. Provider response는 schema-bounded annotation으로 normalize하며 transmitted manifest에
없는 Source locator를 참조하면 request 전체를 `provider_failed`로 reject한다.

Codex executable 또는 login이 unavailable이면 payload subprocess는 시작하지 않고
`provider_unavailable`로 남긴다. Timeout, cancellation, nonzero/invalid response failure,
partial과 stale은 distinct outcome이며 local canonical/structural capability에는 전파하지
않는다. Codex CLI transport는 provider-side deletion operation을 expose하지 않으므로 local
annotation deletion과 별개로 `unsupported`를 보고한다. Provider의 service-side retention이나
deletion을 Codex login 또는 local artifact cleanup에서 추론하지 않는다.

## 5. Inspectable provider state

사용자는 Project마다 최소 다음 상태를 확인할 수 있어야 한다.

- opt-in: never enabled, enabled, disabled 또는 revoked인지
- provider와 model identity
- purpose와 requested capability
- allowed source scope와 실제 transmitted Source manifest
- excluded path, file class, binary/vendor/generated policy
- secret-like content filtering policy와 filter outcome
- request time, result state와 provider diagnostics
- provider-side/local retention expectation과 known limit
- annotation retention state와 deletion request/result
- local derived cache의 존재와 deletion/rebuild state

`excluded`, `filtered`, `not_transmitted`, `transmitted`, `provider_unavailable`와
`provider_failed`를 구분한다. Secret filtering은 완전한 secret absence 보증으로
표현하지 않으며 known blind spot과 user-visible consequence를 제공한다.

Provider가 자체 retention/deletion을 보증할 수 없는 경우 그 한계를 opt-in 전과
deletion 결과에 표시한다. Local deletion 성공을 provider-side deletion 성공으로
위조하지 않는다.

## 6. Local-only normal mode

Semantic provider가 설정되지 않았거나 disabled, revoked, unavailable 또는 failed여도
다음 capability는 정상적으로 유지된다.

- Project와 canonical `Source`, `Question`, `Decision`, `Context Item`, `Checkpoint`
  inspect와 허용된 mutation
- 모든 text repository의 `inventory`
- 설치된 local adapter가 지원하는 `structural`과 `ecosystem` analysis
- current host가 허용하는 bounded `agent_assisted` explanation
- Inquiry, user Decision, Checkpoint와 bounded read-only Recall
- capability, coverage, freshness, unsupported와 failure reporting
- derived index/cache 삭제와 local rebuild
- provider result 없이 가능한 projection과 generated document
- current Viewer projection의 user-specified local read-only HTML snapshot 생성

Provider-backed `semantic` 또는 annotation이 없다는 사실은 해당 capability에
`unavailable` 또는 상황에 맞는 state로 표시한다. Project 전체나 canonical journey를
failure로 표현하지 않는다. Local-only mode를 기능 사용 전의 trial/degraded consent
screen처럼 취급하지 않는다.

Local Viewer snapshot 생성은 local read/projection/publication operation이다. 이 operation은
background provider opt-in을 만들거나 artifact를 upload/transmit하지 않는다. 생성된 파일을
다른 사람 또는 external service와 공유하는 행위는 별도의 user-controlled effect이며 snapshot
command의 authority나 결과에서 추론하지 않는다.

### Candidate collection opt-out and retention contract

Privacy and Provider Boundary는 automatic Candidate collection의 scope별 opt-out과
retention policy 책임을 소유한다. User는 최소 Project, session, source/operation area
또는 지원되는 Candidate kind처럼 inspectable한 selected scope에 대해 collection을
끌 수 있다. 구체적인 UI나 storage field는 이 계약이 정하지 않는다.

| Concern | Required behavior |
|---|---|
| `scope_opt_out` | effective 시점부터 selected scope의 새 automatic Candidate collection을 중단 |
| `existing_candidate_visibility` | opt-out 전에 존재한 Candidate는 explicit deletion, dismissal, promotion 또는 retention expiry까지 Candidate Inspection으로 inspectable하게 유지 |
| `retention_policy` | collection scope/kind별 retained-until 또는 expiry basis와 cleanup outcome을 inspectable하게 유지 |
| `explicit_deletion` | user-selected Candidate content와 관련 managed Derived copy를 삭제하되 canonical target이나 unrelated Candidate를 삭제하지 않음 |
| `policy_change` | 새 policy/effective basis를 이후 collection에 적용하고 기존 Candidate lifecycle을 조용히 rewrite하지 않음 |

Opt-out은 existing Candidate를 promote, dismiss, expire, delete, correct 또는
reinterpret하지 않는다. Existing Candidate를 scope 밖으로 재분류해 보이지 않게 하거나
opt-out을 promotion authorization으로 사용할 수 없다. Retention expiry/cleanup은
Candidate content와 managed copy에 한정되며 이미 explicit promotion으로 생성된
canonical record를 삭제하거나 바꾸지 않는다.

Automatic collection은 계속 최소 structured observation으로 제한한다. Full prompt,
full tool argument, full Source body와 unlimited stdout/stderr는 default long-term
retention 대상이 아니다. Candidate Inspection에 필요한 provenance, scope와 bounded
observation metadata가 이 기본 제외를 우회하는 raw-content 보존 근거가 되지 않는다.

Typed Engineering Choice Discovery와 Materiality Review는 ambient automatic capture가 아니라
explicit current work operation의 Session Candidate다. Candidate collection opt-out을 무시해
background content를 수집하는 경로가 아니며, Project/Goal/baseline identity, bounded
choice/dimension summary, alternative/consequence/effect/coupling metadata와 evidence reference만
보존한다.
Current-task delegation을 주장할 때는 exact Goal/user-turn identity와 Goal에 실제 포함된 bounded
verbatim delegation statement 및 적용 scope만 inspectable evidence로 추가 보존한다. 이는 full
user turn이나 unrelated prompt content를 보존하는 허가가 아니다. Full prompt, 그 밖의 Source
body, raw command와 provider payload는 review content가 아니다. Retention,
explicit deletion, canonical forgetting read barrier와 Candidate Inspection 규칙은 다른 Candidate와
동일하게 적용되고, deletion/expiry가 canonical Question이나 Decision을 바꾸지 않는다.

Learning participation과 Learning Deliberation도 같은 local Candidate/privacy boundary를 사용한다.
Active participation은 bounded verbatim opt-in과 current-host user Source identity만 보존하고 full turn,
inferred proficiency 또는 permanent learner profile을 만들지 않는다. Deliberation round는 bounded
response/rationale, 후속 agent feedback/recommendation과 terminal/reconsideration state만 보존한다.
Recall에 남길 lesson은 별도의 explicit user Source와 canonical `Learning` Context Item operation을
사용하며 Candidate retention만으로 자동 승격하지 않는다.

Checkpoint verification의 exact command invocation도 같은 transient boundary를 따른다.
Current host는 실제 실행과 일치하는 bounded invocation material을 trusted Volicord
operation에 제공하고, operation은 exact UTF-8 bytes에서 SHA-256 fingerprint를 derive한 뒤
raw invocation을 버린다. Canonical Command Source, portable bundle, Candidate, projection,
generated document와 Viewer는 fingerprint와 별도의 human-readable label/exit/termination만
사용하며 raw command/argv를 long-term content로 보존하지 않는다. Caller-supplied digest는
이 derivation을 대신하지 않는다.

## 7. Raw source와 portable context 분리

Raw source body는 repository binding을 통해 접근하는 Source content이며 portable
Canonical Context와 다른 boundary다.

- Canonical `Source` identity와 locator가 raw body 전체 보존을 뜻하지 않는다.
- Portable context에는 raw repository copy나 provider request payload를 기본 포함하지
  않는다.
- Provider transmission scope는 portable canonical record scope에서 추론하지 않는다.
- Adopted generated artifact나 bounded observation이 preserved `Source`가 되어도 원래
  raw repository 전체를 자동 포함하지 않는다.
- Source가 unavailable해도 Project, Decision과 Checkpoint를 읽을 수 있으며 current
  code verification이 unavailable하다는 사실을 표시한다.

Portable bundle의 concrete content, clone binding, divergence와 conflict resolution은
active [Portable Context 계약](portable-context.md)이 소유한다. 이 문서는 raw source와 portable context를 같은
consent 또는 retention unit으로 합치지 않는 privacy boundary만 정의한다.

## 8. Semantic annotation retention과 deletion

`Semantic Annotation`은 Derived State이며 provider/model, purpose, Repository와
Analysis Snapshot, included Source refs, generated time, uncertainty와 freshness를
보존한다.

- Annotation retention은 Project/provider policy와 연결되고 inspectable하다.
- Stale annotation은 current semantic fact로 제공하지 않는다.
- User는 annotation을 범위별로 삭제하고 background generation을 revoke할 수 있다.
- Managed deletion은 annotation, local indexes, embedding, cached summary, preview와
  provider result copy 등 관련 Derived State를 invalidate/delete한다.
- Derived cache 삭제는 Canonical Context를 삭제하거나 Decision applicability를
  자동 변경하지 않는다.
- Reanalysis가 허용돼도 deleted annotation의 historical text를 canonical record에서
  복원하거나 user correction을 덮어쓰지 않는다.

Provider-side retained input/output은 provider의 deletion capability와 observed
outcome을 별도로 표시한다. 삭제 전송 실패, unsupported deletion 또는 unknown
retention을 local success에 묻지 않는다. 개인정보 forgetting의 canonical meaning은
`domain-model.md`가 소유한다.

Canonical forgetting의 local privacy completion은 관련 Candidate content와 managed
Derived content를 owner store에서 제거하고, ordinary SQLite database/WAL 경로에
forgotten content가 남지 않도록 secure deletion, checkpoint/truncation과 destructive
residue post-check를 통과한 상태다. Canonical commit 뒤 이 과정이 중단되면 관련
Candidate/managed Derived content는 cleanup 완료 전 inspection에서 withheld된다.
Managed Derived inspection은 이 상태를 `invalidated`와 explicit withholding identity로
표시하며 content를 반환하지 않는다. Unrelated record는 유지한다. Retry는 같은
canonical invalidation과 operation identity를 사용해 idempotent cleanup만 반복하며
forgotten canonical content를 복원하지 않는다. Provider deletion은 `not_requested`,
`unsupported`, `failed`, `unknown` 또는 observed success로 계속 별도 보고한다.

## 9. User correction protection

User Correction, explicit adoption, Decision과 canonical Context는 analyzer/provider
output보다 높은 user-owned canonical boundary에 있다.

- Reanalysis는 새 Semantic Result/Annotation을 만들 수 있지만 canonical correction을
  in-place overwrite, revert 또는 delete하지 않는다.
- 새 result가 correction과 충돌하면 provenance를 보존한 contradiction/review
  Candidate를 만들 수 있다.
- User가 generated interpretation을 채택했어도 origin, Source basis와 uncertainty를
  유지한다. Adoption은 provider output을 parser-confirmed fact로 바꾸지 않는다.
- Correction 이전의 stale cache가 projection에서 current truth로 다시 나타나지 않게
  invalidate한다.

어떤 provider confidence, model upgrade, repeated result나 access frequency도 silent
overwrite authorization이 아니다.

## 10. Provider degradation

Background request outcome은 최소 다음을 구분한다.

- `provider_unavailable`: provider/model/config/network/dependency가 없어 요청을
  시작할 수 없거나 현재 사용할 수 없음
- `provider_failed`: 요청을 시도했으나 오류, timeout, termination 또는 invalid
  response로 완료하지 못함
- `partial`: 일부 authorized scope만 결과를 얻음
- `stale`: 결과가 다른 Repository Snapshot 또는 만료된 freshness basis에 bound됨

Degradation은 affected Project/source/capability scope, diagnostics, transmitted 여부,
usable result와 retry/review consequence를 보존한다. Provider failure는 unaffected
inventory, local structural result, canonical judgment와 prior historical annotation을
삭제하지 않는다. Partial result를 complete로 표시하거나 transmission이 없었는데
있었던 것으로, 전송 후 실패했는데 전송이 없었던 것으로 표시하지 않는다.

Cross-subsystem retry, process cleanup과 repair matrix는 active
[Failure와 Recovery 계약](failure-and-recovery.md)이 소유한다.

Naturalistic resource observation은 local process identity, candidate executable hash,
bounded RSS/sample/error와 observer lifecycle만 보존할 수 있다. RPC argument, Source body,
provider response, credential과 user conversation content는 memory measurement를 위해 수집하지
않는다. Observer가 external host-owned MCP의 exact process/lifetime을 bind할 수 없으면
`unsupported`를 기록하며 unrelated harness process 측정을 naturalistic MCP evidence로
승격하지 않는다.

## 11. Later-validation hooks

### V07 — Privacy와 local-only mode

V07은 최소 다음을 실행 증거로 남겨야 한다.

- provider 미설정 상태의 canonical, inventory, supported structural, Inquiry,
  Checkpoint와 Recall journey
- Project opt-in 전 background provider/network invocation 부재
- opt-in 후 provider/model/purpose/source scope/exclusion/filtering의 user-visible state
- actual transmitted Source manifest와 configured scope 비교
- excluded file과 secret-like fixture의 filter outcome과 known limit
- revoke 후 new background invocation 차단
- Semantic Annotation과 local derived cache의 managed deletion
- Candidate collection opt-out이 selected scope의 새 automatic collection을 중단하고
  existing Candidate visibility/lifecycle을 조용히 바꾸지 않는 성질
- Candidate retention/expiry와 explicit deletion이 Candidate, canonical target 및
  related Derived content의 서로 다른 boundary를 보존하는 성질
- raw source body가 portable context에 기본 포함되지 않는 성질
- provider unavailable/failed/partial degradation과 unaffected local capability 유지

### V11 — Combined journey

V11은 single-language, polyglot와 Volicord repository journey에서 다음을 결합
검증해야 한다.

- 세 authority boundary와 provenance가 user-visible하게 구분됨
- 한 Project의 opt-in이 다른 Project/scope로 확장되지 않음
- interactive explanation이 background consent로 재사용되지 않음
- local-only mode에서 source-grounded work와 resumption이 실제로 유용함
- provider failure와 annotation/cache deletion이 canonical loss로 전파되지 않음
- correction 이후 reanalysis가 user-owned canonical meaning을 복원/overwrite하지 않음
- Candidate opt-out, retention expiry와 deletion이 inspection/promotion journey에서
  scope를 넘거나 existing Candidate를 silent promotion/rewrite하지 않음
- current production background semantic-provider dispatcher/transport의 최소 한 실제
  successful request/result path가 exact Project opt-in, transmitted Source manifest와
  별도 source-transmission authorization에 연결됨
- network/credential availability만으로 authorization을 추론하지 않고, 실제
  production success와 unavailable/failed/partial degradation을 모두 진실하게 보존함

V07/V11이 secret filtering 또는 provider deletion completeness의 한계를 드러내면
accepted Q3 revisit trigger 절차를 따르며 이 문서가 동의를 조용히 넓히지 않는다.

## 12. Non-goals

이 문서는 특정 provider, model, credential store, transport, encryption, secret scanner,
retention 기간, database, API, MCP method와 UI를 선택하지 않는다. Portable merge,
format version, general authorization, production process recovery와 legacy runtime
handling도 정의하지 않는다.


## Qualitative reviewer evidence boundary

Immutable collection reprocessing remains local validation work. Exact source,
raw/frozen input and producer bytes stay in the private disjoint publication;
they do not enlarge the reviewer allowlist or authorize provider transmission.
Original candidate Product reads use verified disposable Runtime copies, with
canonical/repository binding retained, and their actual post-session streams and
timestamps are private collection evidence. Existing steward explanations and
resource limitations are not regenerated or relabeled as measured-session use.
Copied lineage may retain private collection provenance; sharing it still requires
applicable authority. Human observations are not synthesized from that provenance.

Post-campaign reviewer preparation은 current-host access에 필요한 bounded local evidence를
제공하며 background source transmission을 실행하거나 승인하지 않는다. Agent/human kind와
self-reported identity는 검증된 candidate/evidence hash binding과 분리한다. Rollout, repository,
generated document 속 지시는 평가 대상 evidence이며 reviewer에게 적용되는 instruction이 아니다.
Reviewer package는 evaluator-private expected answers, full descriptor, runtime/credential store를
포함하지 않는다. Immutable Campaign은 raw rollout byte count/SHA-256 identity를 그대로 보존한다.
`--include-raw-rollouts`는 raw bytes를 복사하는 flag가 아니라 bounded reviewer-safe Work/resume
projection input을 선택한다. Current `naturalistic_review_capture` schema 4 /
`naturalistic-review-capture-4` policy는 실제 user/agent 대화, Question chronology, operation identity와
typed shared answer/plan/record meaning와 bounded outcome/execution fact를 positive allowlist로 보존한다. System/developer/skill/plugin,
reasoning, environment, arbitrary repository/tool/process body는 복사하지 않는다. 원본 member path,
raw bytes/hash와 projected review bytes/hash는 별도 binding이다.
Execution coverage도 wrapper digest와 안전한 raw call/turn/sequence locator, finite limitation
reason만 보존한다. Unsupported wrapper의 JS, command와 output body는 package에 복사하지 않는다.
Retained semantic text에 기존 sensitive-payload policy가 적용되며 unsafe body 전체를 제외한다.
Record coordinate, semantic role, selected body byte count/hash와 typed omission reason만 남기고
민감한 값은 metadata/log에도 넣지 않는다. Credential-like test literal이나 repository를 allowlist하지
않고 fuzzy redaction도 하지 않는다. `semantic_complete`는 required user/agent/Question body와
selected operation meaning가 모두 보존됨을 뜻하며 전체 raw 복사를 뜻하지 않는다. Typed allowlisting은 data minimization이지
private prose 공개 권한이 아니다. Work/Decision lifecycle stage도 같은 private body policy로 검사하며
Source observation body와 generator instructions는 복사하지 않는다. Measured-session returned answer와
post-session steward interpretation은 별도 phase/locator/state로 보존하고 서로 대체할 수 없다.
Privacy/size/typed semantic omission은 explicit count와
incomplete state를 남기며 required Work/resume을 사용하는 decisive 판단은 `insufficient_evidence`로
남아야 한다. Non-semantic exclusion alone은 semantic completeness를 떨어뜨리지 않는다.
Detailed schema/limits와 decisive restrictions는 `qualitative-review.md`가 소유한다.
Unavailable captures, CLI 또는 live accessibility observation을 감추거나 satisfied로 대체하지 않는다.
Conversational human-review capture도 이 local reviewer plane 안에서만 동작한다. Human이 제공한
observation, reasoning, relevance, uncertainty와 conflict-resolution confirmation만 보존하며
provider를 호출하거나 누락된 human semantics를 생성하지 않는다. 도구가 생성하는 candidate,
evidence, criterion, locator, reviewer-run과 receipt binding은 human judgment가 아니라 검증 가능한
구조 metadata다. Sensitive-payload 검사는 observation capture와 generated draft publication 전에
동일하게 적용하고, immutable record 전에는 human이 generated `draft.json`을 검사할 수 있어야 한다.
Candidate-bound CLI process stream은 raw bytes를 reviewer package에 복사하지 않는다. Private
ephemeral capture에서 exact raw byte count/SHA-256를 먼저 고정하고, known campaign candidate,
repository/workspace, Runtime Home, process/output/execution root와 campaign/observation/Product
repository absolute path를 deterministic typed placeholder로 바꾼 별도 `review_text`와 그
byte count/SHA-256만 보존한다. Transformation changed state와 placeholder별 substitution count는
inspectable해야 하며 raw identity를 safe text identity로 위장하지 않는다. Exit/termination과
process ordering/timing은 projection 대상이 아니다. 이 bounded known-path 처리는 arbitrary user
text/secret의 general redaction 보증이 아니며 기존 credential/private-prompt payload rejection을
약화하지 않는다.

### Local MCP lifecycle and resource observation

Linux MCP lifecycle registration and candidate-bound observer telemetry are local
operational evidence outside canonical task/Decision authority. Retained fields are
closed process instance/start/executable binding, opaque Runtime/cwd hashes,
server-generated session correlation, bounded RSS/timing, per-Runtime operator expectation/authority and tick coverage, lifecycle and finite
error codes. Neither RPC arguments, full environments, conversation/source bodies,
provider responses nor credentials may be collected. Registration/observer failure
cannot block canonical work or grant host trust/transmission authorization. Managed
registry and observer artifacts may be deleted independently of canonical memory.
The maintained schema and retention limits are in
[resource observation](../../validation/dogfood/resource-observation.md).

### Local live-display capture

The [Viewer display capture](../../validation/dogfood/viewer-observation.md) is an
explicit local read under the operator's existing Viewer/source access. It attaches
the maintained browser driver to one exact local tab, retains renderer basis and
DOM/screenshot hashes, and invokes no provider or host activation. JSON context
contains no DOM/source/conversation body or authenticity token. Screenshots retain
actual displayed local content and stay in operator-owned local capture directories;
they are distinct from the body-free MCP resource telemetry contract. Sharing or
transmitting them requires its own existing authority. Human review packages retain
closed context metadata and image hashes; this does not supply person identity,
comprehension, external authentication or a provider opt-in. Deleting observational
artifacts changes no canonical memory.

Typed Codex Page host metadata has the narrow review-plane contract in
`qualitative-review.md`: null selection is a typed coordinate/reason exclusion;
a selected Page identity is bounded context under the unchanged sensitive/body
policy. Neither supplies user authority or resource body access. Actual user markup
remains semantic evidence. Private copied result lineage retains immutable raw replay
inputs for source comparison; reviewer packages and technical archives retain their
existing separate allowlists. Local verification grants no transmission authority.
