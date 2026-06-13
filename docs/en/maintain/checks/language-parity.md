# Language parity checks

Use these checks when a documentation edit changes meaning in paired English and Korean pages, changes Korean prose, or affects identifiers that must remain stable across languages. These checks do not make either language subordinate to the other.

Heading alignment is only a navigation signal. Passing heading parity does not prove semantic parity for paragraphs, tables, lists, examples, warnings, exceptions, or removed concepts.

## CHK-PARITY-001: English and Korean meaning parity

Owner:
- [English Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
- Compare paired files by meaning unit when the edit changes meaning.
- Confirm the paired files keep the same reader purpose, normative strength, owner routing, baseline/out-of-scope boundary, user-judgment boundary, and security guarantee level.
- Do not treat matching headings as sufficient; inspect the meaning units below the headings.
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
- Treat heading parity as an index to the comparison, not the comparison itself.
- When headings change, compare the paired English and Korean files by heading meaning and reading structure.
- When sections are added, removed, split, or merged, confirm the paired files still expose the same meaning units through equivalent headings or nearby text.
- Confirm each language exposes the same major meaning units, owner routes, warnings, exceptions, and checklist scope.
- Confirm equivalent headings do not hide missing or changed tables, lists, exceptions, non-claims, or owner-boundary rules.
- Confirm Korean visible headings stay natural; use hidden anchors when a stable English anchor must remain available.

Failure:
- A heading change in one language adds, removes, weakens, or reroutes a meaning unit compared with the paired file.
- The section hierarchy makes a rule easier to find in one language and materially harder to find in the other.
- Korean headings preserve English wording or order only to match an anchor.

Fix:
- Update the paired heading and nearby context in the same documentation batch.
- Preserve stable anchors with hidden anchors when needed, while keeping visible Korean headings natural.

## CHK-PARITY-007: table semantic parity

Owner:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Compare table count within the changed sections of paired files.
- Confirm corresponding tables have equivalent header meanings, with exact identifiers preserved unchanged.
- Compare table row meanings, not only row counts; row splits or merges are acceptable only when every condition, value, owner route, exception, and non-claim remains present by meaning.
- Confirm table placement relative to sections is equivalent so the table applies to the same topic, scope, and owner boundary.

Failure:
- One language adds, removes, or moves a table so a rule, mapping, owner route, exception, or warning applies under a different section.
- Table headers translate to a different concept, omit an exact identifier, or change the reader's interpretation of the rows.
- A row meaning is added, removed, weakened, strengthened, or collapsed into a broader row without preserving the same condition or exception.

Fix:
- Update paired tables in the same documentation batch.
- Split or merge rows only when the same meaning units remain reviewable.
- Move the table to the equivalent section, or add nearby text that preserves the same scope and owner boundary.

## CHK-PARITY-008: list semantic parity

Owner:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Compare normative lists, allowed and not-allowed clauses, does-not-imply clauses, exceptions, and owner-boundary lists by meaning unit.
- Confirm list ordering changes do not change priority, evaluation order, routing order, or reader consequence.
- Confirm Korean may use natural sentence rhythm while preserving every list item that carries a rule, exception, non-claim, or owner link.

Failure:
- A paired list has matching length but different rule meaning.
- A list item that limits a claim, names an exception, or routes to an owner is omitted or absorbed into a broad paragraph.
- Allowed/not-allowed wording, does-not-imply wording, or owner-boundary wording is stronger, weaker, or broader in one language.

Fix:
- Restore the missing meaning unit as a list item or nearby named block.
- Rewrite the paired list naturally while keeping normative strength and owner routing aligned.

## CHK-PARITY-009: removed-concept translation residue

Owner:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Terminology Map](../../../terminology-map.yaml)

Check:
- When a concept label, owner route, value name, display phrase, or example term is removed in one language, search the paired language for exact strings, paraphrases, translations, and mixed-language variants.
- Confirm a removed English label does not survive through Korean paraphrase, translation, table rows, list items, headings, glossary text, metadata, or display wording.
- Preserve a removed expression only when a Maintain or terminology owner intentionally keeps it as a searchable forbidden expression.

Failure:
- The English label is gone literally, but the paired Korean text still preserves the removed concept by meaning.
- A table, list, heading, or glossary entry keeps a translated residue that makes the removed concept look current, supported, or owner-routed.

Fix:
- Remove the residue or replace it with the stable category and applicable owner link.
- If maintainers must keep the expression searchable, move it to the terminology owner or focused Maintain guidance as a forbidden expression.
