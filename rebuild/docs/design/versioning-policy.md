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

현재 canonical schema version은 `18`이다. `Project Purpose`와 work `Goal`을 구분하는
Context role, Checkpoint의 explicit Work Item identity와 Decision의 typed work scope는 durable
meaning이므로 이 version에서만 읽고 쓴다. 이전 record의 Goal text, path 또는 chronology를
Work Item identity로 추정하는 decoder를 두지 않는다.

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

현재 portable bundle format version은 `11`이다. Source-grounded `Project Purpose` role,
Checkpoint Work Item identity와 Decision work scope를 그대로 운반한다. Older bundle의 Goal을
Purpose로, 또는 Checkpoint/Decision text를 Work Item identity로 추론하거나 dual decode하지 않는다.

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

Current local persistence는 exact-current Analysis Snapshot JSON token stream을 normalized
shape/value content-addressed blob, 별도 압축된 typed metadata blob과 lightweight manifest로
분할한다. Metadata blob은 같은 current-only storage representation의 일부이며 manifest의
content hash와 exact Project/Analysis identity에 bind한다. Manifest는 current
Analysis Snapshot kind/version을 먼저 노출하고 blob integrity/binding을 검증하며, reader는
복원 후 같은 단일 current domain decoder만 호출한다. Shape chunk, complete value base와
value delta는 이 current representation 내부 storage strategy이지 서로 다른 숫자 version,
legacy input 또는 parallel production decoder가 아니다. 이전 full-JSON Runtime Home을
감지·decode·migrate하는 호환 경로는 제공하지 않는다.

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

Increasing the read-time aggregate of already admitted complete Source windows from
16,384 to 32,768 bytes changes no retained explanation or document metadata shape.
The explanation remains exact-current v6; no decoder, conversion or historical
rebinding is added. Source capture and each embedded window remain bounded to
16,384 bytes, and exact citation read chunks to 4,096. Existing publications are
revalidated before showing additional permitted windows. Work name/summary plans,
retained identities and expiry are unchanged, so this reading change alone requires
zero short-publication regeneration. Repository flow-role admission separately
changes Repository preparation fingerprints, as specified by Projection.

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
- Writer version `10`은 active-host realization의 exact requested-language attestation,
  all-generated-prose attestation과 realized-body fingerprint를 plan fingerprint와 함께 기록한다.
  Version 8 introduced typed Reading/Evidence section roles, exact explanation provenance and
  current-build canonical read equality binding for publication. Version 9 includes
  exact Source-to-prose citation bindings and recorder-owned semantic-approval limits.
  Version 10 adds the exact retained explanation realization fingerprint, separating
  output identity from preparation identity when regenerated/edited wording shares a plan.
  이 durable meaning이 없는 이전 metadata를 host-realized requested-language success로 읽지 않는다.

### Session Candidate store format

Current Candidate store format은 Engineering Choice Discovery, Question Candidate, typed
Materiality Review와 Learning Deliberation payload를 함께 해석하는 version `24` 하나다. Discovery의
closed material-boundary coverage와 real choice/no-independent-fork Source basis, current-task
delegation의 exact dimension/choice/scope/consequence/effect boundary와 semantic rationale,
각 dimension의 exact-authority counterfactual, fact/contract/Decision authority의 bounded
coverage, authority 적용 뒤 remaining credible alternative와 unique-outcome rationale, deterministically observed late
work-authority/readiness revision, typed pre-write no-new/new-outcome closure, server-bound review/discovery/current Source provenance와
exact planned-artifact scope, persistent pending reassessment 및 typed executable work-scope binding과 supported
deliberation-worthy interruption counterfactual/participation-scope alignment 및 supported
deliberation-worthy-to-routine revision basis를 함께 보존하므로 이전
version을 decode하거나 missing coverage/boundary/chronology/revision evidence를 ready authority로 추정하지 않는다.
현재 format은 추가로 concrete reviewed outcomes와 typed no-fork basis, alternative의 structured
residual-fork closure 및 current choice Source linkage를 가진 material decomposition closure, tracked prototype의
bounded mutation scope, observable material outcome의 user ownership proof와 normative
exact authority를 constraint/precedent와 구분하는 typed Source evidence를 보존한다.
Interaction axis별 concrete scenario/result identity와 Source-grounded closure 및 atomic implementation comparison을 함께 보존한다.
Pre-write closure는 concrete planned commitment identity/path와 current choice/alternative 또는 interaction/result binding, explicit private equivalence를 보존한다.
Learning authority assessment는 active participation의 각 current choice/outcome에 independent user authority와 Source-grounded counterfactual을 보존한다.
Required temporal/lifetime interaction review와 planned commitment의 typed temporal assessment/result binding을 함께 보존한다.
Current positive store만 reopen하며 version `23`과 다른 non-current value는
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

### Shared explanation content

Typed applicability, lifecycle causes and dynamic read-comparison time are
ephemeral reading metadata over existing current retained content. They do not
alter authored units, provenance, canonical facts or document metadata v10 and
require no migration. Export basis describes one static artifact; it is not a
new compatibility input format.

Complete-window allocation and `embedded_source_scope` are read-time projections;
exact v6 citations already supply their complete windows and original claim
identity. No focus field, retained-schema migration or old-output conversion is
introduced. The live 4 KiB citation page and 16 KiB shared inclusion bounds remain
independent.

The compact public publication table and identity-bound UTF-8 pages are ephemeral
read projections. They preserve v6 retained meaning and need no retained-format,
canonical-schema or document-metadata migration. Page tokens are current-build
equality checks including managed publication identity, never portable versions.

Format 6 adds independently admitted `work_name` and `work_summary` publication
subjects referencing the existing Goal-backed Work, alongside `work` article.
Each has its own exact preparation, realization, managed purpose, dependencies and
replacement. All use one current writer/decoder. Format 5 is Unsupported; there is
no old-output split, conversion, rebinding or dual reader. Canonical, Privacy SQL,
bundle and Analysis formats are unchanged. Document provenance includes every
separately used publication; its existing subject/plan/realization fields suffice.

Retained Work/Decision/repository explanation JSON inside the existing managed CachedSummary store
has format kind `volicord_explanation` and exact current format version 6
in its realization header. Record/read check that header before typed domain
decode. Non-current content is Unsupported and its prose is withheld; malformed
current content is Corrupt. Explicit delete and preparation/generation from current
canonical evidence provide recovery. There is no dual decoder or implicit conversion.
The enclosing privacy SQL schema, canonical schema, portable bundle, Analysis Snapshot
formats do not change; generated-document metadata is current version 10. A preparation fingerprint binds evidence
and language; it is neither a format version nor an authenticity certificate.

Explanation format 6 uses one reader-unit envelope (`question`, `text`,
`evidence_keys`, `citations`) for authored heading, synopsis, narrative, observation,
comparison, critical limits and reading-point guidance. All units share exhaustive
validation, retrieval/dependency traversal, retention, budgeting and output identity.
There is no alternate title, table-cell, relation-label or guidance store. The current
vocabulary adds section_heading, narrative, adjacent comparison_condition/comparison_result and flow_step units. Section names and order are authored; evidence roles do not prescribe screen headings. Each heading, cell, flow label and reading point is a complete flat grounded unit. The vocabulary has no nested generated fields; adding any requires extending this common
traversal. Partial documents omit unavailable sections without filler. Heading and
synopsis are each optional singleton authored units; without them consumers preserve
original facts rather than extracting a title/result from another paragraph.
Observation, comparison and reading-point units require exact citations. The recorder
owns semantic_approval_status. Multiple narrative units preserve reader order. The old format is Unsupported with prose
withheld; explicit delete/regenerate is the recovery, with no alternate decoder,
conversion or historical campaign rewrite. Privacy SQL does not change.
The current Privacy schema is 2 as specified under Bounded Source observation copies.
Ephemeral current preparation now includes a mandatory `retention_budget`, hashed
with the plan's evidence/instructions/language; callers must prepare afresh. No
old-plan conversion, alternate retained decoder or version-dependent limit exists.
Historical campaign bytes must not be rewritten to supply current preparation fields;
fresh validation uses fresh disposable inputs, distinct from original observations.
Repository preparation now commits to complete inventory and coverage sets with
counts/hashes instead of embedding all paths. This changes ephemeral preparation
fingerprints: earlier repository prose becomes Stale and requires explicit fresh
prepare/Source investigation/compose/record. The single retained format 6 and
Analysis/Source formats are unchanged; no old-shape decoder, conversion or
retrospective Source lifetime renewal is introduced.
Current ephemeral preparation additionally requires `question_grounding`, derived
from the subject and actual evidence by the same rule used at admission. This and
the clarified instructions change preparation fingerprints, so earlier retained
interpretations may read Stale; they are not rewritten, deleted or rebound. This earlier preparation-only change did not change realization or storage formats;
the current reader vocabulary above independently replaces the explanation format.

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

Local `volicord_analysis_attempt` receipts use exact current format version 2,
independent of Analysis Snapshot and canonical formats. The read checks kind/version
before typed decode and rejects non-current, malformed, oversized or identity-mismatched
receipts as unknown attempt history. There is no migration or alternative decoder.
These local operational receipts are neither portable canonical content nor analysis
freshness evidence; missing historical receipts imply no knowledge of earlier attempts.

### Bounded Source observation copies

Privacy SQL current schema is `2`, adding the managed `source_observation` kind.
Non-current Privacy stores are rejected before domain read/write; no migration
or implicit conversion is supplied. Canonical schema `18` and bundle `11` carry the scoped Checkpoint run; analysis
formats are unchanged. Source observation content has exact kind
`volicord_source_observation`, version `4`, checked before typed decode, inside the
existing 147,456-byte managed body limit. Unsupported/corrupt bodies are withheld;
metadata and explicit deletion remain available. Exact local Source/Work links,
revision, selected snapshots, observation time and bounded representation belong
to this disposable format, not a replacement canonical schema or explanation plan.

Source observation version 4 is the single current envelope for whole-file,
bounded file excerpt and command observations. Non-current copies are unsupported
and withheld; deletion and fresh explicit observation remain available, without
migration or a second decoder. File excerpt metadata includes full-file identity,
size, excerpt hash, measured absolute byte/line coordinates and an identity/time
basis digest, bound by the
existing canonical File Source's opaque snapshot string. This adds no canonical
field or entity and does not change Privacy SQL, portable bundle, Analysis Snapshot
or retained explanation formats. Read-time explanation source excerpts additionally
expose `source_representation` and `observation_region`; they are not persisted
copies inside retained explanation records.

### Repository explanation and Source copy formats

The former `volicord_explanation` format 4 added the explicit repository subject and
retained repository scope/coverage metadata in its analysis evidence, and read-time
Source excerpts have an optional Work revision (absent for repository scope).
Every non-current format (including 3, 4 and 5) is Unsupported with prose withheld;
explicit current preparation/composition/recording is recovery. Work and Decision
use this same single current writer/decoder; there is no v3 compatibility reader.
Canonical, bundle, Analysis Snapshot and document-metadata versions are unchanged.
Repository-scoped disposable copies have kind
`volicord_repository_source_observation`, exact current version 2; existing Work
copies use their distinct `volicord_source_observation` version 4 contract.
These are different authority associations, never alternate decoders for one scope.
Non-current repository copies are Unsupported; explicit bounded observation is
recovery and historical evidence is never rewritten or imported into a new scope.

Repository Source copy format 2 binds the complete observation metadata and original
retention expiry into its canonical File Source fingerprint. Non-current copies
are Unsupported; there is no version-1 decoder or copy conversion. Scoped body
retrieval verifies exact hashed Analysis metadata/inventory rather than decoding
the entire graph for every Source; this does not certify graph integrity or semantic
completeness. Repository explanation preparation/admission/publication validates
verified analysis metadata, included hashes and current repository identity, with
no full graph decode. The exact historical worktree remains available through the
explicit analysis evidence operation; metadata-only repository Source copies
record that uninspected detail as null, never infer a clean or non-Git state.
Work copies retain their existing worktree observation. Current file/claim hashes,
binding and retention dependencies remain validated on every publication.

### Reader identity production and event projection

The current 48 KiB explanation realization policy (formerly 40 KiB) changes prepared
budget/fingerprints, not retained fields or the current explanation format. Existing 144 KiB retained content and other
transport/Source limits stay independent. Old publications become stale under
the changed plan; fresh explicit preparation/admission is required. Historical
output and failed admissions retain their own identities and original limits.

Source-first authoring guidance and shared section grouping change preparation
fingerprints and may make earlier explanations Stale. Fresh generation/admission
is required; old prose is never rebound. Section metadata consists only of indices
into the admitted flat stream and reading layers. No retained field, canonical
meaning, decoder or numeric format changes. Empty section headings now fail the
same shared structural validation on compose, record and current read.

The [reader experience](reader-experience-design.md) first path retains current
explanation v6 with exhaustive reader-unit traversal and independent Work
name/summary/article publication validity. Ephemeral Work composition adds no retained decoder or parallel store.
Canonical revision-1 event read metadata is derived from existing revision history;
it changes no canonical SQL or portable format. No numeric version is raised for
an unchanged durable representation. Missing/non-current content is never converted
into a current title.

### Reader article and collection evolution

[Reader AC-17–25](reader-experience-design.md) changes experience/admission contracts,
not existing durable representations by itself. Reusing SourceObservation v2,
CommandExecution, before_source_id and DecisionReading recommendation_rationale
requires no canonical/bundle/analysis/explanation version bump. Current CLI wiring
and validation applicability reports add no Product decoder. If later typed
attempt/verification relations or generated nested structures change durable meaning,
update their owning format and exhaustive consumers together; no compatibility-only
branch, historical rewrite or version bump for prose alone. Unsupported current
capabilities stay explicit instead of coercing old records into new evidence.

The independent verification report, caution/evidence index and Viewer publication
proofs are ephemeral read metadata over existing Checkpoint/Command Source and
SourceObservation v4. The ephemeral reads introduce no canonical schema upgrade
or alternative article decoder. The command integrity correction below separately
replaces the disposable Work copy format. Exact-current explanation v6 admits independent Work publications; each retains
its full own dependencies.

### Command observation artifact integrity

Work SourceObservation is exact current version 4. Each current copy includes its
original expiry; command copies additionally reference an immutable File Source
for the serialized managed observation artifact. Its `volicord-observation:SOURCE`
locator identifies that artifact only, never a repository path or access grant.
Its SHA-256 binds Project/Work/revision, original Command Source, snapshots,
operation/time, streams/completeness/outcome and expiry; the anchor ID is excluded
from its own preimage. This reuses existing File Source hash meaning and managed
links, without canonical/Privacy SQL or portable schema changes. The command's
invocation fingerprint retains its original argv-only meaning. No raw output is
canonicalized and the artifact Source is not a second execution or a user adoption.

Only the v4 reader/writer exists. Non-current Work copies are Unsupported; explicit
new observation is recovery. Historical artifacts stay unchanged; no old-copy
conversion or retrospective integrity assertion exists. Mutated or rehashed copies
cannot replace the immutable Source anchor. Every citation dependency includes
that anchor, and scoped verification/publication uses the same validated copy.

Scoped Checkpoint verification uses canonical schema 18 and bundle format 11.
The nullable `scoped_run` column is part of the single current representation;
null retains historical quotation-only meaning, never inferred typed identities.
Non-current canonical stores/bundles are refused before mutation; no migration or
old decoder is supplied. Work SourceObservation current format is 4, sealing the
optional local check declaration and sampled artifact hashes with the existing
integrity Source. Old observations remain Unsupported; fresh explicit capture is
the only recovery. No explanation-document format or Analysis format changes.

Command Sources in schema 18/bundle 11 optionally retain a scoped capture fingerprint
in addition to the invocation fingerprint. It is a domain-separated hash of measured
Work/revision, check declaration, operation/window, snapshots and artifact samples;
assertion interpretation/evidence and later retest links are excluded. The Source
commits this basis before Checkpoint adoption, so another same-argv execution or
modified target/Work cannot substitute for the original capture. Historical/report-only
commands have no capture fingerprint and cannot acquire a scoped run by inference.
Non-current canonical/bundle formats remain refused with no compatibility decoder.

### Focused Viewer route retirement

[Reader D-28–33 / AC-26–31](reader-experience-design.md) removes presentation
selectors, routes and offline fragments, with controlled errors and no aliases.
This changes no canonical, bundle, Analysis, retained explanation or document
metadata format. Existing CLI/MCP/Operations readers keep exact-current decoding.
Offline HTML is a publication, never an ingestible compatibility format.

### Reader evidence hierarchy and outline

[Reader D-34–37 / AC-32–35](reader-experience-design.md) change presentation only.
Quiet heading/lead provenance, bounded expanded outlines and optional routine
repository diagnostics preserve all retained v6 words/citations and independent
publication identities. No durable format, decoder, admission or budget change
is authorized by reducing visible actions. A later evidenced authoring/admission
change must assess its own format/fingerprint consequences under this owner.

Repository-wide investigation guidance changes the Repository plan fingerprint and
therefore makes an earlier interpretation stale under the changed producer. Explicit
fresh preparation and recording is recovery; failed recording preserves retained
records without reviving stale prose. Current v6 units already express integrative
overview and distinct flows through cited narrative, so this change introduces no
new format/version, decoder, publication scope or dependency pruning. Work name,
summary, article and Decision plans retain their independent contracts and budgets.

### Work progress preparation

Field-selected Work progress changes ephemeral summary preparation, instructions and
admission fingerprints; earlier Goal/report-only summaries become Stale and require
explicit prepare/compose/record. Goal-only name and article contracts retain their
independent meaning. Existing v6 evidence already encodes record/revision/field and
Sources, so no durable format/version, decoder, canonical schema or migration changes.
Original historical outputs remain unchanged; no automatic regeneration or rebinding.

### Scoped Source discovery preparation

Preparation payloads now advertise the existing exact-field metadata selector instead
of inline Work Source inventories; Goal-only name preparation has no inventory.
CLI Source list returns the same 2,048-byte `compact_json_utf8` pages and consistency
fingerprint as `canonical_inspect`. This is an ephemeral public read/preparation
change, not a retained representation or permission change. No durable version bump,
legacy decoder or automatic regeneration is introduced; the progress plan change
above separately requires explicit regeneration of affected summaries.

### Grounded diagram current-only format

The prior explanation formats were 6 (prose) and 7 (concept/sequence); the single
writer/decoder uses exact-current `volicord_explanation` version 8, including an
explicit `diagrams` collection (empty for plain prose). This
section supersedes earlier current-v6 statements, which describe prior changes.
The added semantic meaning is durable even when labels reuse existing grounded units.
Preparation, Work/repository compose, admission, retained decode, ordinary answers,
complete public pages, CLI/MCP and Viewer/offline consumers advance together.

Older **and** newer explanations are Unsupported before decoding; malformed current
content is Corrupt. Fresh explicit preparation/composition/recording from permitted
original evidence is recovery. No compatibility decoder, automatic rebind, extracted
old prose or stale figure fallback exists. The global revision also affects unchanged
Work name/summary examples: explicitly regenerate them with their own original Goal
and progress records. Article failure still cannot invalidate an independently valid
current-format short publication. Old attempts and originals remain unchanged.
Canonical schema, portable bundle, Analysis Snapshot, Source-copy and generated-
document metadata versions are unchanged. SVG renderer identity is presentation
metadata, not an additional retained semantic format or a version-conversion path.

Version 8 adds complete component, guarded flowchart and explicitly owned state-transition figures. Node/edge class, owner, conditions and event labels share exhaustive grounding, semantic seals and current-only rejection. Existing v7 figures must be explicitly regenerated, never relabelled. No other format advances.

The embedded critical-code aggregate is now 49,152 UTF-8 bytes under Projection's
measured critical-evidence policy. This changes read-only inclusion, not retained
v8 shape, preparation, Source capture, citation pages or document metadata formats.
Nonvisual diagram IDs are presentation of already admitted identity; no migration
or compatibility decoder is introduced.

Generated-document metadata current writer is **11**. Embedded explanation
provenance now includes its store-owned publication ID, independently of identical
realization bytes or generated time. Earlier exports retain their historical basis. Explicit current production records
the new identity; no compatibility conversion or rewriting of old output is added.
The ID is ephemeral reader/receipt metadata on RetainedExplanation and is excluded
from its stored v8 envelope, preparation equality, response and semantic seals.
Exact citation transport additionally requires the ID and rejects replacement.
This is a related document-provenance change, not a canonical/bundle format change.
