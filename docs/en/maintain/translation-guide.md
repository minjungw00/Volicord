# Translation guide

Use this guide when editing paired English and Korean Harness documentation. This is a documentation-maintenance standard only. It is not a runtime conformance record, implementation record, QA result, acceptance record, close record, or generated Harness artifact.

The canonical terminology map exists at [docs/terminology-map.yaml](../../terminology-map.yaml). Check it before adding or changing product terms, Korean prose terms, identifier explanations, or mixed-language bans. If this guide and the terminology map disagree, stop and align them in the same documentation batch.

## 1. Semantic parity

English and Korean documents are both active. Neither language is an archive, appendix, or translation-only copy.

Maintain parity by meaning unit, not by line count. A meaning unit can be a rule, warning, paragraph, table row, example, route, or checklist item. The Korean page may change sentence order, split or combine sentences, or use different paragraph rhythm when that makes the meaning clearer in Korean.

Semantic parity requires:

- the same reader purpose
- the same normative strength
- the same active/out-of-scope boundary
- the same owner routing
- the same user-judgment, evidence, verification, acceptance, and residual-risk boundaries
- the same security guarantee level
- the same exact identifiers

Do not finish a meaning-changing batch with only one language updated. If Korean editing exposes an English problem, fix the English too. If English editing introduces a product concept, add the natural Korean equivalent in the paired Korean document during the same batch.

## 2. Document pair and route parity

Every major active page should have an English/Korean pair at the matching route under `docs/en/` and `docs/ko/`. Paired documents do not need matching line numbers, but they must keep matching scope and reader intent.

Route and navigation text must use the compact active structure:

- `docs/doc-index.yaml`
- `docs/*/start.md`
- `docs/*/use/user-guide.md`
- `docs/*/use/agent-guide.md`
- `docs/*/use/judgment-examples.md`
- `docs/*/build/implementation-guide.md`
- `docs/*/reference/README.md`
- `docs/*/maintain/authoring-guide.md`
- `docs/*/maintain/translation-guide.md`
- `docs/*/maintain/checks.md`
- `docs/*/maintain/checks/*.md`

Use the paired [Reference Index](../reference/README.md) / [참조 색인](../../ko/reference/README.md) to choose contract owners. Do not turn route pages or maintain guides into duplicate owners for schemas, DDL, API contracts, security guarantees, projection behavior, or runtime state.

During normal agent work, load only one language for the same `doc_id`. Load both languages only for translation, parity review, or a bilingual edit where comparison is necessary.

## 3. Identifier preservation

Preserve exact identifiers unchanged in English and Korean. Put code-like, schema-like, route-like, or search-critical values in backticks when they appear in prose.

Preserve these exactly:

- file paths, anchors, and `doc_id` values
- API methods, tool names, resource names, and MCP method names
- schema names, schema fields, and object names
- enum values, status values, error codes, validator IDs, and table names
- DDL, column names, storage identifiers, template names, and code literals
    - Harness labels that are intentionally exact, such as `Write Authorization`, `Decision Packet`, and `Projection`

Do not translate exact strings inside code blocks, schema examples, API examples, file paths, field lists, or literal-value tables. Localized Korean display labels are reader-facing text; they do not replace canonical identifiers.

Good distinction:

- Identifier: `ArtifactRef`
- Korean explanation: 아티팩트 참조 스키마
- Prose term: 아티팩트

Some English words can be both code values and ordinary adjectives. Determine the context before preserving the word. Preserve `complete` in backticks only when it is an identifier, such as `intent=complete`; when it means full or entire, English prose should prefer "full" or "entire" and Korean prose should use natural phrases such as 전체, 전체 평가, or 전체 평가 순서.

## 4. Product concept terminology

Use [docs/terminology-map.yaml](../../terminology-map.yaml) as the canonical terminology map for product concepts and mixed-language bans. This guide gives the maintainer-facing standard; the map is the machine-readable control file.

Use one Korean term for one concept unless the terminology map explicitly distinguishes user-facing and reference-facing wording.

| English concept | Korean prose | Identifier or label rule |
|---|---|---|
| close readiness, reference-facing | 닫기 준비 상태 | Preserve identifiers such as `CloseReadinessBlocker`. |
| close readiness, user-facing | 닫기 가능 여부 | Use when explaining to end users whether a task can be closed. |
| close readiness evaluation | 닫기 준비 상태 평가 | Never use "close 가능성 평가". |
| `complete` as an identifier | `complete` | Preserve only when it is an enum value or identifier, such as `intent=complete`; use "full" or "entire" for ordinary adjective meaning. |
| full evaluation order | 전체 평가 순서; in close-readiness context, 전체 닫기 준비 상태 평가 순서 | Do not write `complete` 평가 순서, complete 평가 순서, or `complete` 닫기 준비 상태 순서. |
| artifact | 아티팩트 | Preserve `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle`. |
| surface | 접점 | Preserve `surface_id`; do not make it sound like proof of authority. |
| lifecycle | 생명주기 | Do not leave "lifecycle" in Korean prose unless it is an identifier. |
| projection | 상태 보기 | Use `Projection` when the exact Harness label matters. |
| user-owned judgment | 사용자 소유 판단 | Keep distinct from acceptance and residual-risk acceptance. |
| sensitive-action approval | 민감 동작 승인 | Do not treat it as Write Authorization. |
| Write Authorization | 쓰기 권한 부여, or `Write Authorization` as a label | Preserve the exact label when naming the Harness record. |
| cooperative guarantee | 협력형 보장 | Do not strengthen into detective, sandboxing, enforcement, or stronger-isolation wording. |
| detective guarantee | 탐지형 보장 | Use only when the documented observable scope supports it. |
| baseline scope | 기준 범위 | Do not translate out-of-scope capabilities into current requirements. |
| out-of-scope capability | 지원 범위 밖 기능 | Keep deferred material clearly deferred. |

When a term is missing, add it to the terminology map and both translation guides before spreading a new variant across the docs.

## 5. General prose translation

Translate ordinary English nouns and noun phrases into Korean prose. Do not preserve English just because the English source used a compact noun phrase.

Use English unchanged only when it is:

- an exact identifier
- a file path or anchor
- a code literal, schema value, enum value, table/field name, or API method
- an intentional Harness product label that must remain searchable
- an industry term that is more natural in Korean as borrowed technical vocabulary, such as API, SDK, MCP, YAML, JSON, QA, or CLI

Avoid "English noun + Korean particle" when the English noun is not an identifier. Prefer a Korean concept first, then add the exact English value only if the reader needs contract precision.

Examples:

| Avoid | Use |
|---|---|
| artifact를 저장한다 | 아티팩트를 저장한다 |
| surface에서 보인다 | 접점에서 보인다 |
| lifecycle 의미 | 생명주기의 뜻 |
| staged handle을 전달한다 | 스테이징된 아티팩트 핸들을 전달한다 |
| `surface_id`를 접점 권한으로 본다 | `surface_id`는 접점 식별자일 뿐 권한 증거가 아니다 |

## 6. Korean technical writing style

Korean documents should read as native Korean technical documentation, not as mirrored English.

Write Korean with:

- natural Korean headings
- short explanatory sentences
- Korean concept-first phrasing in user-facing prose
- consistent terms from the terminology map
- enough context that the Korean reader does not need to mentally reconstruct the English
- exact identifiers preserved in backticks where needed

Do not mirror English sentence order when it produces stiff translationese. It is acceptable to reorder clauses, split long English sentences, combine short repetitive sentences, and replace English abstract nouns with Korean verbs when the meaning stays aligned.

Visible Korean headings should be natural Korean. Do not keep an English heading visible only to preserve an existing link. Use the hidden anchor policy instead.

## 7. Hidden anchor policy

Stable English anchors may be needed for existing links, old references, or external bookmarks. Preserve those anchors with hidden HTML anchors before a natural Korean heading.

Use this pattern:

```markdown
<a id="close-readiness"></a>
## 닫기 준비 상태
```

Do not make the visible Korean heading unnatural to match the English anchor. The anchor preserves link stability; the heading serves the reader.

Link text must match the visible heading's meaning. If the visible heading is `## 닫기 준비 상태`, use link text such as "닫기 준비 상태", not "close readiness" or "close 가능성".

When changing headings in one language, check paired-language links and anchors in the same batch.

## 8. User-facing vs reference-facing terminology

User-facing docs explain what the reader can decide, expect, or do. Reference-facing docs define contracts, schemas, owner boundaries, and exact behavior. Korean terminology may differ by audience while preserving the same product meaning.

Use user-facing Korean when the reader needs a plain operational meaning:

- 닫기 가능 여부
- 확인한 것
- 다음 안전한 행동
- 에이전트가 스스로 판단해도 되는 범위
- 하네스가 확인할 수 있는 수준

Use reference-facing Korean when the page defines a product concept or contract:

- 닫기 준비 상태
- 닫기 준비 상태 평가
- 닫기 차단 사유
- 사용자 소유 판단
- 협력형 보장, 탐지형 보장, 예방형 보장
- `Projection`(읽기 전용 상태 보기) on first reference when the exact label matters

Do not expose raw enum names or schema fields as user-facing labels unless the exact raw value is the subject. A Korean display label is localized text, not a replacement for the canonical value.

## 9. Forbidden mixed-language patterns

The following patterns are forbidden in Korean prose unless they appear inside a code block or are being cited as a bad example in this guide.

| Forbidden | Use instead |
|---|---|
| close 가능성 평가 | 닫기 준비 상태 평가 |
| 닫기 가능성 평가 | 닫기 준비 상태 평가 |
| `complete` 평가 순서 | 전체 평가 순서 |
| complete 평가 순서 | 전체 평가 순서 |
| `complete` 닫기 준비 상태 순서 | 전체 닫기 준비 상태 평가 순서 |
| complete 닫기 준비 상태 순서 | 전체 닫기 준비 상태 평가 순서 |
| artifact 저장 | 아티팩트 저장, or 아티팩트를 저장 |
| artifact bytes | 아티팩트 본문 바이트 |
| staged handle | 스테이징된 아티팩트 핸들, or `StagedArtifactHandle` when naming the identifier |
| checksum, size 검증 | 체크섬과 크기 검증 |
| ToolEnvelope 봉투 | `ToolEnvelope` 요청 래퍼 |
| lifecycle 의미 | 생명주기 의미, or 생명주기의 뜻 |
| surface 정보 | 접점 정보, or `surface_id` when naming the field |
| close blocker를 확인한다 | 닫기 차단 사유를 확인한다 |

Mixed English/Korean may be correct when the English part is an identifier, for example `ArtifactRef`를 보존한다 or `surface_id`는 권한 증거가 아니다. When the English part is ordinary prose, translate it.

## 10. Review checklist

- [ ] The edit stayed documentation and did not imply runtime implementation.
- [ ] English and Korean pages match by meaning unit, not line count.
- [ ] Meaning-changing edits were made in both languages in the same batch.
- [ ] Paired files keep matching reader purpose, route role, owner routing, and active/out-of-scope boundary.
- [ ] Identifiers, API methods, file paths, enum values, schema names, table names, validator IDs, error codes, anchors, and code literals are preserved.
- [ ] Exact identifiers appear in backticks where prose clarity or searchability needs them.
- [ ] General English nouns were translated into Korean prose unless they are identifiers or intentional labels.
- [ ] Korean prose avoids "English noun + Korean particle" for non-identifiers.
- [ ] Visible Korean headings are natural Korean.
- [ ] Stable English anchors, when needed, are preserved with hidden anchors.
- [ ] Link text matches the visible heading's meaning.
- [ ] User-facing Korean and reference-facing Korean use the right level of terminology.
- [ ] Forbidden mixed-language patterns were removed except where cited as examples in this guide.
- [ ] New or changed terminology was checked against [docs/terminology-map.yaml](../../terminology-map.yaml).
- [ ] No one-off planning files, archive copies, generated runtime records, or migration notes remain.
