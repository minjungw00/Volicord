# Privacy와 provider 경계 계약

- 상태: active specialized architecture owner
- 소유 범위: local processing, current-host interactive source access,
  background semantic-provider authority, Project opt-in, inspectable transmission,
  retention, revoke와 managed deletion boundary
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

Host가 제공하지 않는 권한을 Volicord가 발명하지 않는다. Current-host interaction의
구체적 UI나 wire representation은 이 문서의 계약이 아니다.

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

Provider-backed `semantic` 또는 annotation이 없다는 사실은 해당 capability에
`unavailable` 또는 상황에 맞는 state로 표시한다. Project 전체나 canonical journey를
failure로 표현하지 않는다. Local-only mode를 기능 사용 전의 trial/degraded consent
screen처럼 취급하지 않는다.

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
`domain-model.md`가 소유하며 구체적인 cross-system recovery guarantee는 이 문서가
선택하지 않는다.

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

Cross-subsystem retry, process cleanup과 repair matrix는 future
`failure-and-recovery.md`가 소유한다.

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

V07/V11이 secret filtering 또는 provider deletion completeness의 한계를 드러내면
accepted Q3 revisit trigger 절차를 따르며 이 문서가 동의를 조용히 넓히지 않는다.

## 12. Non-goals

이 문서는 특정 provider, model, credential store, transport, encryption, secret scanner,
retention 기간, database, API, MCP method와 UI를 선택하지 않는다. Portable merge,
format version, general authorization, production process recovery와 legacy runtime
handling도 정의하지 않는다.
