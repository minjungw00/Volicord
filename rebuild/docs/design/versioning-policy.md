# Versioning 정책

- 상태: active specialized architecture owner
- 소유 범위: new-product canonical schema와 portable/analysis/derived/document format의
  독립 version boundary, read/write behavior, deterministic serialization, upgrade와
  validation responsibility
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
- Writer는 현재 implementation이 소유하는 하나의 current write version만 낸다.
- Format upgrade는 new-product 자체 format evolution이며 excluded legacy service의
  data transition이 아니다.
- Rebuildable format의 incompatibility는 canonical loss로 확산하지 않는다.
- Non-rebuildable canonical content는 지원 여부와 upgrade 결과가 확인되기 전까지
  overwrite하지 않는다.
- Validation ID `V02`, `V04` 같은 표기는 format/product version이 아니다.

## 2. Independent formats와 ownership

| Format boundary | Meaning owner | Version-policy owner | Upgrade/validation responsibility |
|---|---|---|---|
| `Canonical schema version` | `domain-model.md`의 canonical meaning과 Canonical Context Kernel invariant | 이 문서의 read/write/upgrade policy | Phase 4 production canonical implementation; restart, atomicity, forgetting과 unsupported-version tests, V11 combined recovery |
| `portable bundle format version` | `portable-context.md`의 inclusion, identity, lineage와 conflict basis | 이 문서의 independent portable version behavior | Portable I/O implementation; V04 divergence/upgrade interaction과 V11 another-clone journey |
| `Analysis Snapshot version` | `repository-intelligence.md`의 snapshot, envelope, capability와 provenance | 이 문서의 analysis read/write boundary | Repository Intelligence와 adapter conformance; V02 normalization, V11 multi-repository freshness |
| `Derived Index version` | 해당 derived owner의 index/cache meaning과 source basis | 이 문서의 rebuildable-version behavior | Owning index implementation; delete/rebuild/corruption tests, V10 process/filesystem와 V11 recovery |
| `generated-document metadata version` | `projections-and-documents.md`의 grounding, omission, adoption과 output boundary | 이 문서의 metadata read/write behavior | Projection/document implementation; V06 Markdown/HTML grounding과 V11 handoff |

Meaning owner는 field semantics를 정의하고 이 문서는 version transition behavior를
정의한다. 새 field나 version number를 이 표에서 미리 선택하지 않는다. 한 format의
version 변경이 다른 format의 version 증가를 자동 요구하지 않는다.

## 3. Canonical schema version

Canonical schema version은 active runtime이 Project, Source, Question, Decision,
Context Item, Checkpoint, revision, supersession와 forgetting meaning을 durable하게
읽고 mutation할 수 있는 계약을 식별한다.

- Read는 schema version을 transaction/mutation 전에 확인한다.
- Supported older new-product schema는 production-owned upgrade를 성공시킨 뒤 current
  writer로 열 수 있다.
- Upgrade는 provenance, identity, lifecycle, privacy deletion과 relation을 보존하며
  실패하면 이전 bytes/state를 current success로 가장하지 않는다.
- Unsupported newer schema는 read/write를 거부하고 detected/current-supported version,
  affected Project와 recovery choice를 user-visible하게 제공한다.
- Newer schema를 일부 이해한다는 이유로 canonical write를 허용하지 않는다.

Canonical data는 Derived State에서 rebuild할 수 없으므로 best-effort field dropping,
silent downgrade 또는 fresh empty initialization으로 대체하지 않는다.

## 4. Portable bundle format version

Portable bundle format version은 `portable-context.md`가 소유하는 content boundary,
deterministic representation, lineage/common-base와 conflict basis를 해석한다.

- Import는 전체 bundle을 canonical mutation 전에 version-check한다.
- Supported older new-product bundle은 current canonical meaning으로 명시적 upgrade하거나
  read-only inspection한 뒤 import한다.
- Unsupported newer bundle은 Project/record 일부를 import하지 않고 format kind,
  detected version과 supported range를 보고한다.
- Export는 current portable write version 하나만 사용하며 compatibility용 구버전
  writer를 동시에 운영하지 않는다.
- Version conversion이 common-base/merge provenance를 바꾸면 conversion basis를
  lineage에 보존한다.

Portable version upgrade는 legacy Runtime Home, schema 또는 record를 읽는 importer가
아니다.

## 5. Analysis Snapshot version

Analysis Snapshot version은 normalized Code Entity/Relation envelope, capability,
coverage, diagnostics, provenance와 freshness representation을 식별한다.

- Reader는 snapshot/version과 producing adapter contract를 확인한 뒤 result를 쓴다.
- Unsupported newer snapshot을 empty success나 current coverage로 제공하지 않는다.
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
- Unsupported, corrupt 또는 source-basis mismatch index는 read에 사용하지 않고
  `stale`, `corrupt` 또는 `repair_required`에 맞는 state로 격리한다.
- Canonical/Source basis에서 재생성할 수 있으면 in-place semantic upgrade보다
  delete/rebuild를 기본 책임으로 삼는다.
- Rebuild 실패는 해당 search/projection capability를 degrade할 뿐 canonical data를
  rewrite하지 않는다.
- 여러 index technology를 parallel production authority로 유지해 format evolution을
  회피하지 않는다.

## 7. Generated-document metadata version

Generated-document metadata version은 Markdown/HTML document가 Project, snapshot,
Decision, Source, coverage, omission, uncertainty, generator와 adoption basis를 어떻게
기록하는지 식별한다. Content/output format과 metadata version은 구분한다.

- Preview/export/adoption reader는 metadata version을 먼저 확인한다.
- Unsupported newer metadata는 document claim을 current grounded projection으로
  표시하거나 canonical Source로 adoption하지 않는다.
- Generated draft는 rebuildable하지만 이미 explicit adoption된 preserved Source는
  canonical provenance를 잃지 않도록 supported upgrade path 또는 clear unsupported
  read state가 필요하다.
- Regeneration은 adopted user edits나 historical Source identity를 overwrite하는
  metadata upgrade가 아니다.
- Omission metadata가 per-identity list에서 bounded scope와 exact count로 바뀌는
  것처럼 durable meaning/shape가 바뀌면 current writer version을 올리고 하나의
  current representation만 쓴다. Compatibility를 위한 dual writer/reader를 두지 않는다.

## 8. Read-time version checks

모든 format reader는 domain parsing이나 mutation 전에 다음을 판정한다.

1. expected format kind인지
2. version field가 존재하고 well-formed인지
3. version이 current reader가 지원하는 범위인지
4. integrity/source/common-base 같은 format-specific precondition이 맞는지
5. read-only, upgrade-required, rebuild-required 또는 unsupported 중 어떤 결과인지

Result는 최소 detected format/version, supported range, affected scope, usable safe
remainder, required owner action과 user-visible consequence를 제공한다. Malformed version과
unsupported newer version을 corrupt content나 empty state로 합치지 않는다.

Unsupported newer-version behavior는 다음을 지킨다.

- Canonical schema/bundle: mutation과 partial import 금지, original state 보존
- Analysis Snapshot/index: incompatible result 격리, 가능한 local rebuild 제안
- Generated draft metadata: current grounding claim 금지, source basis가 있으면 regenerate
- Adopted document Source: original artifact/provenance를 보존하고 unsupported 상태 표시

## 9. Write-version behavior

Writer는 해당 format의 current supported write version만 생성한다. Read support 범위가
write support 범위보다 넓을 수 있지만 다음을 허용하지 않는다.

- old/new schema에 동시에 canonical write
- 같은 operation의 dual bundle output을 long-lived compatibility surface로 제공
- reader마다 다른 meaning으로 동일 version을 기록
- runtime flag로 parallel production implementation을 선택
- version field 없이 “latest”를 environment나 binary에 암묵적으로 결합

Upgrade가 필요한 durable input은 validated upgrade 완료 뒤 current version으로 쓴다.
Failure/cancellation 전후 어떤 version이 authoritative한지 명확해야 하며 partial upgraded
state를 current success로 보고하지 않는다.

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

Upgrade는 format owner가 아닌 production implementation의 명시적 operation이며
다음 책임을 가진다.

- source and destination format kind/version 확인
- upgrade 전 validation과 recoverable publication boundary
- identity, provenance, lifecycle, privacy/forgetting과 common-base basis 보존
- complete stdout/stderr/exit/termination을 포함한 operational result가 필요한 경우
  [Failure와 Recovery 계약](failure-and-recovery.md) 준수
- success/failure/cancellation 뒤 authoritative version을 한 개로 유지
- supported version fixtures, deterministic repeat와 fault-injection tests
- user-visible unsupported/rebuild/repair consequence

Canonical schema와 portable bundle의 upgrade는 non-rebuildable meaning을 다루므로
production Rust semantics와 integration/property tests가 소유한다. Analysis Snapshot,
Derived Index와 generated draft처럼 rebuildable한 format은 source basis가 충분하면
discard/rebuild 또는 regenerate할 수 있다. Source basis가 unavailable하면 rebuildable
data를 current로 가장하지 않고 historical unavailable state를 표시한다.

## 12. New-product evolution과 excluded legacy service

Future format evolution은 이 문서가 정의한 current Volicord format 사이의 변화다.
다음은 format evolution 범위가 아니다.

- legacy Runtime Home detection 또는 read
- legacy database/schema decoder
- legacy record migration/importer 또는 historical export
- old identifier, command, API나 workflow compatibility
- dual readable/writable schemas로 transition을 무기한 유지
- old/new decoder를 병렬 production authority로 유지
- reconstruction, replacement, next-generation 같은 product-generation label을 public
  namespace나 format kind로 사용

Git history에 이전 implementation이 있다는 사실은 supported input format을 만들지
않는다. New-product format upgrade test에 legacy fixture를 섞지 않으며 clean runtime
boundary를 유지한다.

## 13. Validation hooks

- **V02:** Analysis Snapshot version과 adapter output normalization, unsupported/newer
  snapshot degradation, rebuild/freshness basis를 검증한다.
- **V04:** portable bundle version, common-base lineage, upgrade 뒤 conflict provenance와
  unsupported bundle non-mutation을 검증한다.
- **V06:** generated-document metadata version, Markdown/HTML equivalence, adoption 전후
  supported/unsupported metadata behavior를 검증한다.
- **V10:** canonical/index/process publication과 upgrade failure, termination, repair/rebuild
  primitive 책임을 검증한다.
- **V11:** 모든 format의 independent version check, supported upgrade, unsupported newer
  behavior와 combined recovery를 실제 journey에서 검증한다.

## 14. Non-goals

이 문서는 version number, database/schema field, migration engine, serializer, checksum,
process/filesystem technology, release numbering과 support window를 선택하지 않는다.
Legacy decoder, dual read/write, compatibility mode와 parallel production implementation은
새 format evolution path가 아니다.
