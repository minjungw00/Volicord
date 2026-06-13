# Language parity checks

Use these checks when a documentation edit changes meaning in paired English and Korean pages, changes Korean prose, or affects identifiers that must remain stable across languages. These checks do not make either language subordinate to the other.

## CHK-PARITY-001: English and Korean meaning parity

Owner:
- [English Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
- Compare paired files by meaning unit when the edit changes meaning.
- Confirm the paired files keep the same reader purpose, normative strength, owner routing, baseline/out-of-scope boundary, user-judgment boundary, and security guarantee level.
- Allow natural Korean structure instead of line-by-line translation.

Failure:
- One language misses a meaning-changing edit.
- One language strengthens, weakens, or reroutes a rule compared with the paired file.

Fix:
- Update both languages in the same documentation batch.
- Rewrite Korean naturally while preserving the same meaning.

## CHK-PARITY-002: exact identifier preservation

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Translation Guide](../translation-guide.md)

Check:
- Confirm exact identifiers remain unchanged in both languages.
- Confirm file paths, anchors, `doc_id` values, API methods, schema fields, enum values, table names, validator IDs, error codes, and product labels appear in backticks when prose clarity or searchability needs them.

Failure:
- An exact identifier is translated, localized, reformatted, or used as a reader-facing display label that changes its meaning.

Fix:
- Restore the exact identifier.
- Add a plain-language explanation next to the identifier when needed.

## CHK-PARITY-003: Korean structure preservation

Owner:
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Korean Authoring Guide](../../../ko/maintain/authoring-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- For Korean reference edits, compare conditions, results, exceptions, boundary caveats, owner links, and close-readiness consequences as meaning units.
- Confirm Korean prose may differ in line count and sentence order while keeping important caveats and owner boundaries visible.
- Inspect dense Korean paragraphs for merged rules that hide a condition, exception, or boundary caveat.

Failure:
- Korean text preserves the broad topic but collapses separate condition/result/exception or boundary-caveat structure.
- A Korean paragraph makes an owner boundary, baseline/out-of-scope boundary, security boundary, or close-readiness consequence harder to detect than in the paired meaning unit.

Fix:
- Split the Korean prose into natural paragraphs or bullets that preserve the meaning units.
- Keep exact identifiers unchanged and preserve semantic parity without forcing line-by-line translation.

## CHK-PARITY-004: Korean storage structure

Owner:
- The applicable paired storage owner selected from [Reference Index](../../reference/README.md) and `doc-index.yaml`
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Check:
- For Korean storage reference edits, compare the paired English storage source docs by meaning unit.
- Confirm conditions, effects, exceptions, boundary caveats, and owner links remain visibly separate in Korean.
- Inspect dense Korean paragraphs for merged storage rules that hide a condition, exception, or boundary caveat.

Failure:
- Important storage conditions, effects, exceptions, or boundary caveats are collapsed into dense Korean paragraphs.
- Korean prose preserves the broad topic but makes the storage boundary harder to review than the paired English meaning unit.

Fix:
- Rewrite the Korean storage prose using natural paragraphs, lists, or tables that keep the meaning units visible.
- Keep exact identifiers unchanged and link to storage owners instead of duplicating contract detail in Maintain guidance.

## CHK-PARITY-005: Korean user-facing readability

Owner:
- [Korean Authoring Guide](../../../ko/maintain/authoring-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Check:
- Inspect Korean user-facing prose for natural Korean technical writing, Korean concept-first phrasing, and consistent terms.
- Confirm exact identifiers remain searchable but are not exposed as ordinary display labels.
- Confirm common English nouns and noun chains are translated into Korean unless they are exact identifiers, intentional product labels, or natural technical borrowings.
- Confirm Korean prose is not a literal mirror of English sentence order when natural Korean would make the same meaning clearer.

Failure:
- Korean prose mirrors English sentence order, keeps avoidable English noun phrases, or hides the reader action behind internal identifiers.
- Korean prose keeps an ordinary English common noun only because it appears in the paired English sentence.
- Korean keeps a compact English noun chain as visible prose when the words are not identifiers or intentional labels.

Fix:
- Rewrite in natural Korean while preserving identifiers and semantic parity.

## CHK-PARITY-006: paired headings and reading structure

Owner:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- When headings change, compare the paired English and Korean files by heading meaning and reading structure.
- When sections are added, removed, split, or merged, confirm the paired files still expose the same meaning units through equivalent headings or nearby text.
- Confirm each language exposes the same major meaning units, owner routes, warnings, exceptions, and checklist scope.
- Confirm Korean visible headings stay natural; use hidden anchors when a stable English anchor must remain available.

Failure:
- A heading change in one language adds, removes, weakens, or reroutes a meaning unit compared with the paired file.
- The section hierarchy makes a rule easier to find in one language and materially harder to find in the other.
- Korean headings preserve English wording or order only to match an anchor.

Fix:
- Update the paired heading and nearby context in the same documentation batch.
- Preserve stable anchors with hidden anchors when needed, while keeping visible Korean headings natural.
