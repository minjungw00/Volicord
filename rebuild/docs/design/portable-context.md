# Portable Context 계약

- 상태: active specialized architecture owner
- 소유 범위: portable bundle inclusion/exclusion, Project와 local clone binding,
  source-independent canonical read, export/import determinism, divergence와 conflict
  vocabulary, resolution authority와 merge provenance
- 상위 architecture 기준: [논리 아키텍처](architecture.md)
- core domain 기준: [핵심 도메인 모델](domain-model.md)
- privacy 기준: [Privacy와 provider 경계](privacy-and-provider-boundary.md)
- version 기준: [Versioning 정책](versioning-policy.md)
- validation 기준: [기술 검증 계획 V04·V11](validation-plan.md)
- 비소유 범위: canonical entity/lifecycle 의미, storage schema, concrete merge
  algorithm, conflict UI, serialization technology와 local filesystem implementation

이 문서는 Project의 Canonical Context를 clone과 computer 사이에서 옮기고
divergence를 안전하게 드러내는 계약이다. Bundle은 local runtime의 복사본이나
repository archive가 아니며, import는 의미 충돌을 조용히 해결하는 authority가
아니다.

## 1. Authority와 information boundary

Canonical entity와 lifecycle 의미는 `domain-model.md`가 소유하고 Canonical Context
Kernel만 authoritative mutation을 완료한다. Portable Context는 그 meaning을
이동시키는 representation, common-base basis와 conflict/resolution provenance를
소유한다.

- Export는 canonical read이며 Source, Decision 또는 다른 record를 수정하지 않는다.
- Import는 supported new-product bundle을 검증하고 Kernel operation에 제출한다.
- Local Operations는 file selection, publication과 clone binding을 조정하지만
  Project identity나 conflict meaning을 발명하지 않는다.
- Derived State와 Session Candidate는 bundle에 들어갔다는 이유로 canonical이 되지
  않는다.
- Bundle import event와 conflict resolution은 origin author/Source provenance를
  대체하지 않는다.

## 2. Portable bundle inclusion

Portable bundle은 선택한 Project의 다음 canonical meaning을 포함한다.

| Included content | Portable obligation |
|---|---|
| `Project identity` | path/remote와 독립적인 stable identity, portable Project 설정과 origin basis |
| `Source manifest` | Source identity, kind, portable locator/snapshot basis, availability를 재평가할 정보; Command Source는 presentation label, Volicord-derived invocation fingerprint와 exit/termination을 포함하지만 raw invocation/body 전체는 아님 |
| `Question` | identity, displayed revision, dependency, materiality basis와 terminal outcome |
| `Decision` | exact Question linkage, choice/delegation, provenance, applicability, rationale basis와 revisit state |
| `Context Item` | statement role, provenance, applicability와 lifecycle state |
| `Checkpoint` | source-grounded state basis, work/verification/review/acceptance의 독립 상태 |
| `revision` | 같은 identity의 non-semantic correction history와 ordering/basis |
| `supersession` | semantic replacement identity와 relation, active/history 구분 |
| minimum tombstone metadata | referential integrity와 forgetting propagation에 필요한 최소 non-content identity/basis |

Tombstone은 forgotten 원문, 민감 metadata 또는 recoverable content hash를 포함하지
않는다. 어떤 relation을 위해 tombstone이 필요한지는 inspectable해야 하며 필요가
없는 content를 audit 명목으로 보존하지 않는다.

Export scope가 일부 record/history를 의도적으로 제외할 수 있다면 selection과
omission reason을 bundle basis에 기록한다. Partial export를 complete Project history로
표시하지 않는다.

## 3. Default exclusions

다음은 기본 portable canonical bundle에서 제외한다.

- embeddings
- full-text와 semantic indexes
- parser caches
- generated graph layouts
- raw tool traffic
- exact command invocation, argv와 environment material
- full chat transcripts
- raw source copies
- temporary Session Candidates

이 항목은 rebuildable Derived State, local observation 또는 별도 Source content다.
명시적인 generated artifact adoption이 있더라도 adopted artifact만 preserved Source가
될 수 있으며 repository raw source 전체나 원래 tool/session traffic까지 포함하는
근거가 되지 않는다.

Provider request payload, local absolute path, runtime journal, lock, log와 credential도
portable canonical meaning이 아니다. Derived State를 별도 전달하는 미래 기능이
생겨도 canonical bundle과 독립된 authority, retention과 version boundary를 가져야
한다.

## 4. Project identity와 local clone binding

Project identity는 초기화할 때 생성하고 repository path, directory name, remote URL,
Git origin이나 content similarity만으로 추론하지 않는다. Product Repository에
tracked identity marker를 자동 생성하지 않는다.

`local clone binding`은 한 environment에서 stable Project를 실제 repository root와
연결하는 local operational record다.

- 하나의 Project는 여러 environment/clone binding을 가질 수 있다.
- 하나의 path나 remote가 같다는 사실만으로 Project equality를 만들지 않는다.
- Binding은 portable canonical record에 local absolute path를 삽입하지 않는다.
- 다른 clone에서 import한 뒤 user의 explicit bind intent로 binding을 만든다.
- Rebind는 current locator/availability를 바꿀 수 있지만 historical Source snapshot과
  provenance를 rewrite하지 않는다.
- Binding candidate가 다른 Project 또는 source basis와 충돌하면
  `source_binding_conflict`로 드러내고 자동 동일시하지 않는다.

## 5. Source-independent canonical read

Bundle을 읽는 데 source repository, analyzer, provider 또는 original clone이 필요하지
않다. Source가 없는 environment에서도 Project, Question, Decision, Context Item,
Checkpoint, revision, supersession와 tombstone relation을 inspect할 수 있어야 한다.

Unavailable Source는 identity와 historical snapshot basis를 유지한다. Code link나
source navigation은 다음처럼 동작한다.

- 현재 binding과 snapshot을 확인할 수 있으면 link를 `available` 또는 freshness에
  맞는 state로 제공한다.
- repository, file, symbol 또는 historical revision을 확인할 수 없으면 link를
  `unavailable`로 표시하고 fabricated current path로 이동시키지 않는다.
- current content가 historical basis와 다르면 `stale`을 표시하고 과거 range를 current
  navigation guarantee로 사용하지 않는다.
- Source unavailable은 canonical record absence나 successful verification을 뜻하지
  않는다.

Source를 나중에 bind/rebind하면 derived navigation과 freshness를 다시 계산할 수
있지만 canonical history는 그대로 유지한다.

## 6. Deterministic export와 validated import

같은 supported bundle format version, canonical state, export scope와 common-base
basis에 대한 export는 byte-identical한 결과를 만들어야 한다. 이를 위해
implementation은 최소 다음을 보장한다.

- record, relation, revision과 manifest ordering에 stable total order 사용
- locale, current path, process ordering과 map iteration에 독립적인 representation
- timestamp나 operation observation이 필요하면 export 실행마다 새 값으로 canonical
  payload를 오염시키지 않고 정의된 basis에 포함
- value, identifier와 text의 canonical escaping/normalization
- excluded/local-only content가 bytes에 섞이지 않음
- bundle 전체를 읽기 전에 format/version과 integrity basis를 확인할 수 있음

Import는 mutation 전에 bundle identity, supported version, Project identity, record와
relation integrity, common-base metadata와 conflict possibility를 검증한다. Invalid,
corrupt 또는 unsupported input의 일부를 canonical success로 남기지 않는다. 같은
bundle의 반복 import는 duplicate identity를 만들지 않으며 결과가 already present인지,
conflict인지 또는 mutation이 필요한지 명시한다.

구체적인 serialization, checksum, signature, compression, atomic publication과 storage
transaction technology는 이 문서가 선택하지 않는다.

## 7. Common base와 divergence

`common base`는 두 portable histories가 함께 확인할 수 있는 마지막 canonical
revision/bundle basis다. Base는 단순 export timestamp나 file name이 아니며 Project,
record history와 bundle lineage를 비교할 수 있는 inspectable basis를 가진다.

`divergence`는 common base 이후 두 환경에서 canonical histories가 달라진 상태다.
각 side의 record/revision, deletion과 Source basis를 보존하며 어느 쪽을 current로
가정하지 않는다.

- Common base를 확인할 수 있으면 base, incoming과 local meaning을 비교한다.
- Base가 없거나 신뢰할 수 없으면 `common_base_unavailable`이며 two-way guess를
  three-way resolution처럼 표시하지 않는다.
- Export/import와 merge 뒤에도 사용한 common-base identity와 각 input bundle basis를
  보존한다.
- Context branch를 선택해도 branch origin과 Project relation을 inspect할 수 있게 한다.

## 8. Conflict vocabulary

Portable comparison은 최소 다음 conflict/result class를 사용한다.

| Class | Meaning | Automatic boundary |
|---|---|---|
| `independent_additions` | common base 뒤 서로 다른 canonical identity가 독립적으로 추가됨 | relation/invariant 충돌이 없음을 입증한 경우에만 자동 결합 가능 |
| `same_record_revision` | 같은 entity/revision line이 양쪽에서 달라짐 | byte/text 차이만으로 semantic equivalence를 가정하지 않음; non-semantic correction임이 검증된 좁은 경우 외에는 review |
| `semantic_decision_conflict` | 같은 Question/applicability에 양립할 수 없는 Decision choice, delegation 또는 supersession이 존재함 | user semantic judgment 없이 해결 금지 |
| `delete_modify_conflict` | 한 side가 forget/delete했고 다른 side가 같은 identity나 protected relation을 수정함 | modified content 또는 deleted content를 자동 복원/삭제 금지 |
| `source_binding_conflict` | Project/Source portable identity에 대해 incompatible local repository binding 후보가 존재함 | path/remote similarity로 자동 동일시 금지 |
| `common_base_unavailable` | trustworthy common base를 찾거나 검증할 수 없음 | automatic semantic merge 금지; inspectable branch/import 선택 필요 |

Conflict class는 user-facing consequence, affected identities, base/local/incoming basis,
available Source와 uncertainty를 함께 제공한다. 한 import에 여러 class가 있으면 하나의
대표 상태로 나머지를 숨기지 않는다.

## 9. Resolution authority와 automatic limits

Automatic resolution은 identity, provenance와 domain invariant로 의미 변화가 없음을
결정적으로 입증할 수 있는 경우로 제한한다. `independent_additions`도 relation,
forgetting, Question dependency와 Decision applicability가 충돌하지 않을 때만 안전하다.

다음은 user-owned resolution이다.

- Decision choice, delegation, rationale 또는 applicability의 semantic conflict
- Question meaning, dependency 또는 terminal outcome의 semantic conflict
- delete/modify에서 privacy와 보존 중 무엇을 적용할지에 대한 판단
- common base가 없는 histories를 같은 branch로 합칠지에 대한 판단
- source binding 후보가 실제로 같은 Project/source를 가리키는지에 대한 판단

`Decision`, `Question` 또는 `delete_modify_conflict`를 last-writer-wins, import order,
timestamp, model recommendation이나 access frequency로 조용히 overwrite하지 않는다.
User는 base/local/incoming과 consequence를 보고 side 선택, explicit merge 또는 context
branch를 선택할 수 있다. Resolution이 새 material user judgment를 포함하면 exact
current-host Source와 해당 Question/Decision contract를 따라 canonical하게 기록한다.

## 10. Merge provenance

Merged result는 최소 다음 basis를 보존한다.

- Project identity와 result bundle lineage
- common-base identity 또는 `common_base_unavailable` reason
- local/incoming bundle identity, format version과 export scope
- input record/revision/tombstone identities
- detected conflict class와 automatic/user-owned 판정
- automatic rule basis 또는 explicit user resolution Source
- preserved branch/supersession/forgetting relation
- unresolved conflict와 unavailable Source

Merge event는 input record의 original actor, statement role, Source와 revision provenance를
rewrite하지 않는다. Resolution 뒤에도 어떤 input이 채택, 결합, branch 또는 unresolved
상태인지 inspect할 수 있어야 한다. Derived indexes는 merged canonical result에서
rebuild하며 merge tool output을 canonical fact로 재사용하지 않는다.

## 11. Validation hooks

### V04 — Divergent bundle merge

V04는 concrete merge algorithm을 선택하고 검증하는 owner이며 이 문서가 그 algorithm을
미리 정하지 않는다. 최소 다음 evidence를 남긴다.

- deterministic common-base discovery와 unavailable-base behavior
- `independent_additions`, `same_record_revision`, `semantic_decision_conflict`,
  `delete_modify_conflict`, `source_binding_conflict`, `common_base_unavailable` 전부
- automatic resolution이 semantic judgment를 넘지 않는 성질
- base/local/incoming과 consequence의 inspectable presentation
- explicit user resolution Source와 merge provenance
- unavailable repository에서 canonical conflict resolution
- merge 후 deterministic export/import, Recall과 deletion propagation

V04 결과가 user-owned conflict를 안전하게 표현하지 못하면 Q6/Q7 revisit 절차를
따르며 silent overwrite로 통과시키지 않는다.

### V11 — Combined multi-repository journey

V11은 Volicord, single-language와 polyglot repository의 another-clone journey에서
다음을 결합 검증한다.

- path-independent Project identity와 explicit binding
- source-independent canonical read와 unavailable code link
- default exclusion과 derived rebuild
- divergent histories, user resolution과 preserved common-base basis
- conflict 뒤 Decision/Question applicability, Recall과 document grounding
- unsupported/corrupt bundle이 partial canonical mutation을 남기지 않는 성질

## 12. Non-goals

이 문서는 merge algorithm, database, serialization library, hash/ID scheme, archive
container, filesystem publication, conflict UI, network synchronization과 team
collaboration protocol을 선택하지 않는다. Legacy data decoder, migration/importer,
historical export, compatibility alias 또는 parallel runtime path를 제공하지 않는다.
