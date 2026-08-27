# Versioning 정책

- 상태: active specialized architecture owner
- 소유 범위: new-product canonical schema와 portable/analysis/derived/document format의
  독립 version boundary, current-only read/write behavior, deterministic serialization,
  non-current rejection과 validation responsibility
- 상위 architecture 기준: [논리 아키텍처](architecture.md)
- core domain 기준: [핵심 도메인 모델](domain-model.md)
- portable 기준: [Portable Context 계약](portable-context.md)
- validation 기준: [기술 검증 계획](validation-plan.md)
- 비소유 범위: concrete schema field, version number, serialization/storage technology,
  merge algorithm, release version과 legacy data handling

이 문서는 오래 보존되는 new-product data를 어떤 format contract로 해석하는지
정의한다. Format version은 제품 세대, release train, validation ID 또는 병행 제품
namespace가 아니다. 서로 다른 format은 변화와 rebuild 책임이 다르므로 하나의
global version으로 묶지 않는다.

## 1. Version boundary invariant

- 모든 durable format은 자신을 식별하는 format kind와 version을 가진다.
- Reader는 content를 domain meaning으로 해석하기 전에 format kind/version을 확인한다.
- Reader와 Writer는 format boundary별로 현재 implementation이 소유하는 정확히 하나의
  current version만 지원한다.
- Detected version이 current와 다르면 older/newer 방향과 무관하게 `unsupported`이며
  production reader가 decode, inspect-as-domain, upgrade 또는 write-back하지 않는다.
- Rebuildable format의 incompatibility는 canonical loss로 확산하지 않는다.
- Non-rebuildable canonical content는 non-current input이 발견되면 original bytes/state를
  보존하고 read/write를 거부한다.
- Validation ID `V02`, `V04` 같은 표기는 format/product version이 아니다.

## 2. Independent formats와 ownership

| Format boundary | Meaning owner | Version-policy owner | Current read/write와 validation responsibility |
|---|---|---|---|
| `Canonical schema version` | `domain-model.md`의 canonical meaning과 Canonical Context Kernel invariant | 이 문서의 current-only read/write/rejection policy | Phase 4 production canonical implementation; restart, atomicity, forgetting과 non-current rejection tests, V11 combined recovery |
| `portable bundle format version` | `portable-context.md`의 inclusion, identity, lineage와 conflict basis | 이 문서의 independent current-only portable behavior | Portable I/O implementation; V04 divergence/non-mutation interaction과 V11 another-clone journey |
| `Analysis Snapshot version` | `repository-intelligence.md`의 snapshot, envelope, capability와 provenance | 이 문서의 analysis read/write boundary | Repository Intelligence와 adapter conformance; V02 normalization, V11 multi-repository freshness |
| `Derived Index version` | 해당 derived owner의 index/cache meaning과 source basis | 이 문서의 rebuildable-version behavior | Owning index implementation; delete/rebuild/corruption tests, V10 process/filesystem와 V11 recovery |
| `generated-document metadata version` | `projections-and-documents.md`의 grounding, omission, adoption과 output boundary | 이 문서의 metadata read/write behavior | Projection/document implementation; V06 Markdown/HTML grounding과 V11 handoff |
| `Session Candidate store format version` | `domain-model.md`의 Candidate meaning과 `inquiry-and-decision.md`의 typed payload/lifecycle | 이 문서의 current-only local Candidate behavior | Inquiry production store; exact-current positive fixture, non-current rejection과 V09/V11 restart behavior |

Meaning owner는 field semantics를 정의하고 이 문서는 current admission/rejection behavior를
정의한다. 새 field나 version number를 이 표에서 미리 선택하지 않는다. 한 format의
version 변경이 다른 format의 version 증가를 자동 요구하지 않는다.

## 3. Canonical schema version

Canonical schema version은 active runtime이 Project, Source, Question, Decision,
Context Item, Checkpoint, revision, supersession와 forgetting meaning을 durable하게
읽고 mutation할 수 있는 계약을 식별한다.

- Read는 schema version을 transaction/mutation 전에 확인한다.
- Current가 아닌 older/newer schema는 모두 read/write를 거부하고 detected/current version,
  affected Project와 recovery choice를 user-visible하게 제공한다.
- Non-current schema를 일부 이해하거나 field가 우연히 같다는 이유로 canonical read/write를
  허용하지 않는다.

Canonical data는 Derived State에서 rebuild할 수 없으므로 best-effort field dropping,
silent downgrade 또는 fresh empty initialization으로 대체하지 않는다.

Command Source처럼 기존 durable Source payload의 correlation meaning이 바뀌면 Canonical
schema current version을 올린다. 같은 Source manifest를 운반하는 portable bundle도 그
새 meaning을 해석해야 하므로 applicable bundle current version을 함께 올린다. 이전
label-only Command Source를 fingerprint-bearing current Source로 추정·decode하거나 label을
hash input으로 재해석하지 않으며, exact-current positive fixture와 직전/이후 non-current
negative fixture만 유지한다.

## 4. Portable bundle format version

Portable bundle format version은 `portable-context.md`가 소유하는 content boundary,
deterministic representation, lineage/common-base와 conflict basis를 해석한다.

- Import는 전체 bundle을 canonical mutation 전에 version-check한다.
- Current가 아닌 older/newer bundle은 Project/record 일부를 import하거나 domain record로
  inspect하지 않고 format kind, detected version과 current version을 보고한다.
- Export는 current portable write version 하나만 사용하며 compatibility용 구버전
  writer를 동시에 운영하지 않는다.
- Non-current bundle rejection은 current canonical state, common-base와 merge provenance를
  변경하지 않는다.

## 5. Analysis Snapshot version

Analysis Snapshot version은 normalized Code Entity/Relation envelope, capability,
coverage, diagnostics, provenance와 freshness representation을 식별한다.

- Reader는 snapshot/version과 producing adapter contract를 확인한 뒤 result를 쓴다.
- Non-current snapshot을 empty success나 current coverage로 제공하지 않는다.
- Source repository와 supported adapter가 있으면 incompatible analysis를 폐기하고
  rebuild할 수 있다.
- Historical result를 보존할 필요가 있어도 unsupported content를 current semantic
  fact로 해석하지 않고 opaque/unavailable basis로 분리한다.
- Rebuild는 canonical Source, Decision, Context Item, Checkpoint와 user correction을
  수정하지 않는다.

Adapter-native cache/version은 이 common Analysis Snapshot version과 별개일 수 있지만
공통 consumer에게 native version만 노출해 normalized contract check를 생략할 수 없다.

## 6. Derived Index version

Derived Index version은 full-text/semantic index, graph, embedding, fingerprint cache,
ranking 또는 동등한 rebuildable representation의 compatible read boundary다.

- Index는 owning canonical/analysis source basis와 index version을 함께 가진다.
- Non-current, corrupt 또는 source-basis mismatch index는 read에 사용하지 않고
  `stale`, `corrupt` 또는 `repair_required`에 맞는 state로 격리한다.
- Canonical/Source basis에서 재생성할 수 있으면 non-current index를 decode하지 않고
  격리/delete한 뒤 current representation으로 rebuild한다.
- Rebuild 실패는 해당 search/projection capability를 degrade할 뿐 canonical data를
  rewrite하지 않는다.
- 여러 index technology를 parallel production authority로 유지해 format evolution을
  회피하지 않는다.

## 7. Generated-document metadata version

Generated-document metadata version은 Markdown/HTML document가 Project, snapshot,
Decision, Source, coverage, omission, uncertainty, generator와 adoption basis를 어떻게
기록하는지 식별한다. Content/output format과 metadata version은 구분한다.

- Preview/export/adoption reader는 metadata version을 먼저 확인한다.
- Non-current metadata는 document claim을 current grounded projection으로
  표시하거나 canonical Source로 adoption하지 않는다.
- Generated draft는 rebuildable하지만 이미 explicit adoption된 preserved Source는
  canonical provenance와 original artifact를 보존한 clear unsupported read state로 둔다.
- Regeneration은 adopted user edits나 historical Source identity를 overwrite하는 format
  conversion이 아니다.
- Omission metadata가 per-identity list에서 bounded scope와 exact count로 바뀌는
  것처럼 durable meaning/shape가 바뀌면 current writer version을 올리고 하나의
  current representation만 쓴다. Compatibility를 위한 dual writer/reader를 두지 않는다.
- Generated-content language request와 normalized HTML language-tag metadata처럼 서로 다른
  meaning이 분리되거나 rendered-field omission contract가 추가되면 current writer
  metadata shape/version에서만 함께 기록한다. 이전 shape decoder, dual metadata
  representation 또는 compatibility write를 추가하지 않는다.

### Session Candidate store format

Current Candidate store format은 Engineering Choice Discovery, Question Candidate와 typed
Materiality Review payload를 함께 해석하는 version `5` 하나다. Discovery choice identity,
alternative/consequence/effect/coupling/evidence state와 Review의 exact choice reference가
추가되었으므로 이전 version을 decode하거나 broad Goal/Materiality dimension에서 current
meaning을 추정하지 않는다. Current positive store만 reopen하며 version `4`와 다른 non-current value는
domain decode/mutation 전에 reject한다. Candidate는 portable bundle에 포함되지 않으므로 이
변경은 canonical schema나 portable bundle version을 바꾸지 않는다.

## 8. Read-time version checks

모든 format reader는 domain parsing이나 mutation 전에 다음을 판정한다.

1. expected format kind인지
2. version field가 존재하고 well-formed인지
3. version이 current reader의 정확한 current version과 같은지
4. integrity/source/common-base 같은 format-specific precondition이 맞는지
5. current read, rebuild-required 또는 unsupported 중 어떤 결과인지

Result는 최소 detected format/version, current version, affected scope, usable safe
remainder, required owner action과 user-visible consequence를 제공한다. Malformed version과
well-formed non-current version을 corrupt content나 empty state로 합치지 않는다.

Non-current version behavior는 older/newer 방향에 관계없이 다음을 지킨다.

- Canonical schema/bundle: mutation과 partial import 금지, original state 보존
- Analysis Snapshot/index: incompatible result 격리, 가능한 local rebuild 제안
- Generated draft metadata: current grounding claim 금지, source basis가 있으면 regenerate
- Adopted document Source: original artifact/provenance를 보존하고 unsupported 상태 표시

## 9. Write-version behavior

Writer는 해당 format의 current write version만 생성하며 Reader도 같은 exact current
version 하나만 admit한다. 다음을 허용하지 않는다.

- old/new schema에 동시에 canonical write
- 같은 operation의 dual bundle output을 long-lived compatibility surface로 제공
- reader마다 다른 meaning으로 동일 version을 기록
- runtime flag로 parallel production implementation을 선택
- version field 없이 “latest”를 environment나 binary에 암묵적으로 결합

Non-current durable input을 만난 operation은 write를 시작하지 않는다. Rebuildable target은
current canonical/Source basis에서 별도 rebuild할 수 있지만 input bytes를 decode하거나
partial conversion한 state를 current success로 보고하지 않는다.

## 10. Deterministic serialization

Portable bundle, canonical export, Analysis Snapshot과 generated metadata처럼 bytes를
비교·보존하는 format은 같은 semantic state와 format version에서 deterministic
serialization을 제공한다.

- stable ordering과 tie-breaker
- defined text/number/boolean/null representation
- canonical escaping과 normalization
- timezone/locale/current path/process order에 독립적인 output
- default/omitted field meaning의 version-specific definition
- unknown field handling과 integrity calculation의 stable boundary

Derived index의 internal bytes가 deterministic할 필요가 없는 경우에도 같은 source
basis/version에서 observable query meaning과 coverage를 재현할 수 있어야 한다. Exact
serialization technology와 canonicalization algorithm은 implementation validation이
선택한다.

## 11. Upgrade responsibility

현재 production contract는 older new-product decoder나 upgrade operation을 제공하지
않는다. 이 절은 그 부재와 future version change의 책임을 명확히 한다.

- Format current version을 바꾸기 전에는 preserved durable data의 disposition과 rollout
  boundary를 별도 accepted contract로 확정해야 한다. 이 문서는 upgrade path를 미리
  약속하지 않는다.
- 현재 implementation은 non-current input을 발견하면 domain parse와 mutation 전에
  실패하며 original bytes/state와 마지막 verified current state를 보존한다.
- Canonical schema와 portable bundle은 rebuild할 수 없으므로 matching current reader가
  없으면 unsupported 상태로 남고 empty initialization, field dropping 또는 partial import로
  대체하지 않는다.
- Analysis Snapshot, Derived Index와 generated draft는 current Source/canonical basis가
  충분할 때 incompatible bytes를 해석하지 않고 discard/rebuild 또는 regenerate한다.
- Source basis가 unavailable하면 rebuildable data도 current로 가장하지 않고 unavailable
  또는 unsupported state를 표시한다.
- Validation은 각 format의 exact current positive fixture와 older/newer/malformed negative
  fixture를 분리하며 모든 non-current case가 mutation 전에 실패하는지 확인한다. Numeric
  older-version success fixture, decoder branch 또는 migration path를 추가하지 않는다.

## 12. New-product evolution과 excluded legacy service

Future format change는 current representation을 교체하기 전에 별도 contract review를
요구한다. 현재 production reader/writer가 지원하는 transition path로 추론하지 않는다.
다음은 현재 version behavior 범위가 아니다.

- legacy Runtime Home detection 또는 read
- legacy database/schema decoder
- legacy record migration/importer 또는 historical export
- old identifier, command, API나 workflow compatibility
- dual readable/writable schemas로 transition을 무기한 유지
- old/new decoder를 병렬 production authority로 유지
- reconstruction, replacement, next-generation 같은 product-generation label을 public
  namespace나 format kind로 사용

Git history에 이전 implementation이 있다는 사실은 supported input format을 만들지
않는다. Non-current rejection test에 legacy fixture를 섞지 않으며 clean runtime boundary를
유지한다.

## 13. Validation hooks

- **V02:** Analysis Snapshot version과 adapter output normalization, older/newer snapshot
  rejection, rebuild/freshness basis를 검증한다.
- **V04:** portable bundle version, common-base lineage, non-current rejection 뒤 conflict
  provenance와 unsupported bundle non-mutation을 검증한다.
- **V06:** generated-document metadata version, Markdown/HTML equivalence, adoption 전후
  current/non-current metadata behavior를 검증한다.
- **V10:** canonical/index/process publication과 non-current failure, termination, repair/rebuild
  primitive 책임을 검증한다.
- **V11:** 모든 format의 independent exact-current check, older/newer rejection-before-mutation,
  rebuildable-data recovery와 combined recovery를 실제 journey에서 검증한다.

## 14. Non-goals

이 문서는 version number, database/schema field, migration engine, serializer, checksum,
process/filesystem technology, release numbering과 support window를 선택하지 않는다.
Legacy decoder, dual read/write, compatibility mode와 parallel production implementation은
새 format evolution path가 아니다.

Current read-only Viewer HTML snapshot은 production reader가 다시 domain data로 ingest,
decode 또는 adopt하는 durable input format이 아니므로 별도 versioned authority를 만들지
않는다. Snapshot 안의 bounded Project/canonical/repository-analysis basis는 inspection과
freshness 설명용이며 canonical schema, portable bundle 또는 generated-document metadata로
해석하지 않는다. Future에 snapshot ingest/adoption을 지원하려면 그때 별도 current-only
format contract가 필요하다.
