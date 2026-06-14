# Translation guide

Use this guide when editing paired English and Korean Harness documentation. It owns bilingual semantic parity, natural Korean technical prose, identifier preservation guidance, mixed-language Korean style rules, and hidden-anchor practice.

This is a documentation-maintenance guide. It does not define product behavior, API behavior, storage effects, security guarantees, projection behavior, runtime behavior, schema contracts, glossary entries, or owner-routing indexes. When contract detail is needed, link to the focused owner instead of restating it here.

Complete structured terminology metadata lives in [`docs/terminology-map.yaml`](../../terminology-map.yaml). The [Glossary](../reference/glossary.md) is a compact human-readable guide to selected core terms. Check the terminology map before adding or changing product terms, Korean prose terms, identifier explanations, product labels, or Korean mixed-language examples. If this guide and the terminology map disagree, align them in the same documentation batch.

## 1. Semantic parity

English and Korean documents are both maintained. Neither language is an archive, appendix, or translation-only copy.

Maintain parity by meaning unit, not by line count. A meaning unit can be a rule, warning, paragraph, table row, example, route, or checklist item. The Korean page may change sentence order, split or combine sentences, or use different paragraph rhythm when that makes the meaning clearer in Korean.

Semantic parity requires:

- the same reader purpose
- the same normative strength
- the same baseline/out-of-scope boundary
- the same treatment of owner references already present in the paired material
- the same treatment of user-judgment, evidence, verification, acceptance, and residual-risk boundaries already present in the paired material
- the same treatment of security guarantee wording already present in the paired material
- the same exact identifiers and exact product labels where they are named

Do not finish a meaning-changing batch with only one language updated. If Korean editing exposes an English problem, fix the English too. If English editing introduces a product concept, add the natural Korean equivalent in the paired Korean document during the same batch.

Do not add Korean-only structure just to make a sentence easier to scan. Labels such as `조건`, `결과`, `비주장`, or `허용되지 않는 것` belong in Korean only when the English document has the same meaning unit, such as a condition, result, non-claim, or not-allowed clause. Korean can express that unit with natural wording, but it must not create a new rule, exception, warning, or prohibition that the English page does not carry.

## 2. Document pair and route parity

Every major maintained page should have an English/Korean pair at the matching route under `docs/en/` and `docs/ko/`. Paired documents do not need matching line numbers, but they must keep matching scope and reader intent.

Preserve route and owner references that already exist in the paired material by meaning. Do not use this guide to recreate full owner maps, API method maps, storage-effect summaries, schema field tables, security rules, or product contract text. Owner lookup belongs to the [Authoring Guide](authoring-guide.md), [`docs/doc-index.yaml`](../../doc-index.yaml), and the applicable reference owner.

During normal agent work, load only one language for the same `doc_id`. Load both languages only for translation, parity review, or a bilingual edit where comparison is necessary.

## 3. Identifiers, labels, and ordinary concepts

`docs/terminology-map.yaml` owns the systematic identifier classes, exact identifier lists, product labels that require exact naming, and Korean mixed-language expressions to avoid. Preserve those values unchanged in English and Korean. Put code-like, schema-like, route-like, or search-critical values in backticks when they appear in prose.

Do not translate exact strings inside code blocks, schema examples, API examples, file paths, field lists, literal-value tables, or machine-readable metadata. Localized Korean display labels are reader-facing text; they do not replace canonical identifiers or exact product labels.

Use this distinction:

- Exact identifier: `ArtifactRef`
- Korean explanation of the identifier: 아티팩트 참조 스키마
- Ordinary prose concept: 아티팩트
- Exact product label: `Product Repository`, `Harness Runtime Home`, `Projection`, or `Write Authorization` when naming the label itself
- Korean reader-facing prose: 제품 저장소, 런타임 홈, 상태 보기, or 쓰기 권한 부여 when the exact label is not the subject

Some English words can be both code values and ordinary adjectives. Determine the context before preserving the word. Preserve `complete` in backticks only when it is an identifier, such as `intent=complete`. When the English means full or entire, English prose should prefer "full" or "entire" and Korean prose should use the terminology map's ordinary-prose replacement.

## 4. Recurring terminology

Use [`docs/terminology-map.yaml`](../../terminology-map.yaml) as the complete structured terminology metadata for product concepts, product labels, identifiers, and Korean mixed-language controls. This section is a compact writing aid. It is not a glossary replacement, owner-routing index, or product contract summary.

Use one Korean expression for one concept unless the terminology map intentionally distinguishes user-facing and reference-facing wording. When a durable term is missing, add it to the terminology map before spreading a new variant across the docs.

Use the map fields for term-specific choices:

- `ko_reference` for reference-facing Korean.
- `ko_user` for user-facing Korean.
- `ko_contextual` and `ko_explanation` for context-specific wording and identifier explanations.
- `preserve_identifier`, `preserve_as_identifier`, and `preserve_as_label` for exact strings that stay English.
- `avoid_ko` and top-level `avoid.ko` for searchable Korean mixed-language expressions to replace.

Do not copy the terminology map's preferred-expression or avoid-expression lists into this guide or the glossary. Add a glossary row only when readers need a compact term meaning and primary owner, not merely to preserve a translation choice.

## 5. Translating ordinary prose

Translate ordinary English nouns and noun phrases into Korean prose. Do not preserve English just because the English source used a compact noun phrase.

Use English unchanged only when it is:

- an exact identifier
- a file path or anchor
- a code literal, schema value, enum value, table/field name, or API method
- an intentional Harness product label that must remain searchable
- an industry term that is more natural in Korean as borrowed technical vocabulary, such as API, SDK, MCP, YAML, JSON, QA, or CLI

Avoid "English noun + Korean particle" when the English noun is not an identifier. Prefer a Korean concept first, then add the exact English value only if the reader needs contract precision or searchability.

Use the terminology map for concrete replacements. For example, strings such as `artifact 저장` and `blocker 라우팅` belong in the map-owned avoid list; do not recreate that list here.

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

When Korean uses named blocks, bullets, or tables for readability, compare them with the English meaning units. Korean may expose an existing condition, result, exception, non-claim, or prohibition more naturally, but it must not add a Korean-only structural claim such as `조건`, `결과`, `비주장`, or `허용되지 않는 것` when the paired English text has no corresponding meaning unit.

## 7. Hidden anchor policy

Stable English anchors may be needed for existing links or external bookmarks. Preserve those anchors with hidden HTML anchors before a natural Korean heading.

Use this pattern:

```markdown
<a id="close-readiness"></a>
## 닫기 준비 상태
```

Do not make the visible Korean heading unnatural to match the English anchor. The anchor preserves link stability; the heading serves the reader.

Link text must match the visible heading's meaning. If the visible heading is `## 닫기 준비 상태`, use link text such as "닫기 준비 상태", not "close readiness" or "close 가능성".

When changing headings in one language, check paired-language links and anchors in the same batch.

## 8. Audience-sensitive Korean terms

User-facing docs explain what the reader can decide, expect, or do. Reference-facing docs define the contracts they own. Korean terminology may differ by audience while preserving the same meaning.

Use the terminology map's `ko_user` field when the reader needs a plain operational meaning. Use `ko_reference` when a page names or defines a maintained concept. Use `ko_reference_first_reference`, `ko_explanation`, or related explanation fields when the exact label or identifier matters.

Do not maintain a second audience-specific term list in this guide. Put durable audience distinctions in the terminology map, then use the glossary only for compact human-readable meanings of selected terms.

Do not expose raw enum names or schema fields as user-facing labels unless the exact raw value is the subject. A Korean display label is localized text, not a replacement for the canonical value.

## 9. Mixed-language Korean rule

Korean prose should not keep ordinary English nouns, noun chains, or adjective labels when the terminology map provides a Korean expression. Search changed Korean for English words joined to Korean particles, suffixes, or common nouns, then replace them with natural Korean unless the English string is an exact identifier, exact product label, or natural borrowed technical term.

Keep concrete avoid examples short and searchable. The structured avoid list belongs in [`docs/terminology-map.yaml`](../../terminology-map.yaml); this guide explains how to apply it in prose.

Mixed English/Korean may be correct when the English part is an identifier, for example `ArtifactRef`를 보존한다 or `surface_id` 필드를 보존한다. When the English part is ordinary prose, translate it.

## 10. Review

After translation edits, run the focused Maintain checks instead of using this guide as a checklist:

- [Language parity checks](checks/language-parity.md) for meaning-unit parity, natural Korean structure, headings, tables, lists, and identifier preservation.
- [Terminology checks](checks/terminology.md) for terminology-map alignment, mixed-language Korean, glossary entry focus, `active` wording, `complete` ambiguity, and related term controls.
- [Links and indexes checks](checks/links-and-indexes.md) when headings, anchors, relative links, terminology targets, or route metadata changed.
