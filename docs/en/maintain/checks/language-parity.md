# Language parity checks

Use these checks when a documentation edit changes meaning in paired English and Korean pages, changes Korean prose, or affects identifiers that must remain stable across languages. These are documentation quality checks only; they do not make either language subordinate to the other or certify product runtime behavior.

Heading alignment is only a navigation signal. Passing heading parity does not prove semantic parity for paragraphs, tables, lists, examples, warnings, exceptions, or removed concepts. English/Korean parity is also not enough when both languages share the same wrong semantic label or structure.

Parity review boundary: these checks compare maintained documentation meaning. They do not certify the product behavior that the paired documents may describe.

## CHK-PARITY-001: English and Korean meaning parity

Check sources:
- [English Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
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

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Translation Guide](../translation-guide.md)

Evidence to inspect:
- Confirm exact identifiers remain unchanged in both languages.
- Confirm file paths, anchors, `doc_id` values, API methods, schema fields, enum values, table names, validator IDs, error codes, and product labels appear in backticks when prose clarity or searchability needs them.

Failure:
- An exact identifier is translated, localized, reformatted, or used as a reader-facing display label that changes its meaning.

Fix:
- Restore the exact identifier.
- Add a plain-language explanation next to the identifier when needed.

## CHK-PARITY-003: Korean structure preservation

Check sources:
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Korean Authoring Guide](../../../ko/maintain/authoring-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
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

Check sources:
- The applicable paired storage owner selected from [Reference Index](../../reference/README.md) and `doc-index.yaml`
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Evidence to inspect:
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

Check sources:
- [Korean Authoring Guide](../../../ko/maintain/authoring-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Evidence to inspect:
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

Check sources:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
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

Check sources:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
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

Check sources:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
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

Check sources:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Terminology Map](../../../terminology-map.yaml)

Evidence to inspect:
- When a concept label, owner route, value name, display phrase, or example term is removed in one language, search the paired language for exact strings, paraphrases, translations, and mixed-language variants.
- Confirm a removed English label does not survive through Korean paraphrase, translation, table rows, list items, headings, glossary text, metadata, or display wording.
- Preserve a removed expression only when a Maintain or terminology owner intentionally keeps it as a searchable forbidden expression.
- When a removed expression is preserved as a search pattern, confirm the surrounding wording makes it clear that the string is a review pattern, not a current documentation statement.

Failure:
- The English label is gone literally, but the paired Korean text still preserves the removed concept by meaning.
- A table, list, heading, or glossary entry keeps a translated residue that makes the removed concept look current, supported, or owner-routed.

Fix:
- Remove the residue or replace it with the stable category and applicable owner link.
- If maintainers must keep the expression searchable, move it to the terminology owner or focused Maintain guidance as a forbidden expression.

## CHK-PARITY-010: negative-clause strength and placement

Check sources:
- [Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Search paired changed sections for negative-clause markers, including English `Not allowed`, `Does not imply`, `Not implied`, and `must not`, and Korean `허용되지 않는 것`, `의미하지 않는 것`, and `해서는 안 됩니다`.
- Search conditional prohibition markers, including English `unless`, `only when`, and `except when`, and Korean conditional or exception phrasing such as `단`, `일 때만`, and `경우를 제외하고`.
- Compare each prohibition, exception, and non-claim by meaning unit.
- Confirm a conditional prohibition does not hide the condition that permits, limits, or routes a claim of authority inside a prohibition label in one language when the paired language exposes it as a condition or owner boundary.
- Confirm one language does not impose a stronger prohibition, broader exception, narrower exception, or stronger non-claim than the other.
- If one language places a prohibition or non-claim in a table, list, named block, or nearby sentence, confirm the paired language keeps that meaning unit in the corresponding place.

Failure:
- A negative clause appears only in one language and changes what the reader is told to avoid.
- One language uses stronger wording, such as changing a caution into a prohibition or turning a narrow exception into a broad exception.
- A conditional prohibition is split into condition and prohibition in one language but buried under `Not allowed` or `허용되지 않는 것` in the paired language.
- A condition for treating something as authority is easier to find in one language and hidden inside a prohibition bullet in the other.
- A prohibition lives in a visible table or list in one language but is buried, moved to a different scope, or omitted in the paired language.

Fix:
- Restore the missing prohibition, exception, or non-claim in the paired language.
- Match normative strength and placement by meaning unit while keeping Korean prose natural.
- Split unclear conditional prohibitions into corresponding condition, not-allowed, and owner-boundary units in both languages.
- If the negative rule defines a product contract, move it to the owner and leave both paired non-owner pages with a short boundary plus owner link.

## CHK-PARITY-011: semantic skeleton parity

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Korean Authoring Guide](../../../ko/maintain/authoring-guide.md)
- [Translation Guide](../translation-guide.md)

Evidence to inspect:
- For changed paired sections that use or imply a semantic skeleton, identify the meaning-unit skeleton before comparing prose. Important Reference sections should have the skeleton identified before prose is written or reshaped.
- Common skeletons include `Purpose`, `Conditions`, `Result`, `Non-claim`, `Owner boundary`, and `Related references`; another acceptable skeleton is `Meaning`, `Contract`, `Boundary`, and `Related references`.
- When a conditional prohibition would otherwise be unclear, use the same semantic skeleton in both languages for `Conditions`, `Not allowed`, and `Owner boundary`.
- Confirm the paired English and Korean sections use the same skeleton for the same section.
- Confirm sentence count may differ, but meaning-unit placement and normative strength remain aligned.
- Confirm matching skeletons are checked against the content they label; parity does not make a mislabeled unit correct.
- Search Korean changed sections for structural labels such as `조건`, `결과`, `비주장`, and `허용되지 않는 것`; each label must have the corresponding English meaning unit in the paired section.
- Confirm Korean natural wording does not add labels such as `조건`, `결과`, `비주장`, or `허용되지 않는 것` unless English has the equivalent meaning unit.
- Confirm English does not add `Not allowed`, `Does not imply`, or `Non-claim` sections unless Korean has the equivalent meaning unit.

Pass condition:
- Paired sections use the same meaning-unit skeleton, and each label matches the content type it names in both languages.

Failure:
- Paired sections match headings or tables but use different semantic skeletons.
- One language adds, removes, or relocates a condition, result, non-claim, owner boundary, or related-reference meaning unit.
- One language exposes a conditional prohibition as `Conditions`, `Not allowed`, and `Owner boundary`, while the other hides the condition inside a prohibition unit.
- Both languages use the same skeleton, but a label names the wrong kind of content.
- Korean introduces `조건`, `결과`, `비주장`, or `허용되지 않는 것` as visible structure without an English counterpart.
- One language adds a label or negative section that changes normative strength or makes a rule easier to find in only that language.

Fix:
- Define or restore the same skeleton in both languages for the same section.
- Rewrite Korean naturally while preserving the same meaning-unit placement, owner routing, and normative strength.
- Remove one-sided labels or add the paired meaning unit when the owner-backed meaning belongs in both languages.

## CHK-PARITY-012: both-languages-wrong semantic labels

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Structure checks](structure.md)
- [English Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Evidence to inspect:
- After comparing paired files, run the semantic label-content check on each language independently.
- Inspect matching labels such as `Not allowed`, `Required behavior`, `Result`, `Does not imply`, and `Owner boundary`, plus their Korean equivalents or nearby Korean prose.
- Confirm both languages classify content correctly before accepting parity: required versus prohibited, effect versus non-effect, and route versus contract.
- Confirm both languages do not share an unclear conditional prohibition where an `unless`, `only when`, or `except when` condition belongs in `Conditions` or `Owner boundary`, but both pages leave it under `Not allowed` or `허용되지 않는 것`.
- In Korean, inspect `허용되지 않는 것` sentences that use `되어야 합니다` or `해야 합니다`; do not accept them merely because English has the same prohibition structure.
- Do not accept a Korean label only because it mirrors the English label, and do not accept an English label only because the Korean page has the same structure.

Pass condition:
- Paired languages match by meaning unit, and each language also uses labels and structure that correctly describe the content.

Failure:
- English and Korean both put required behavior under a prohibition label, or prohibited behavior under a requirement label.
- English and Korean both hide a condition for treating something as authority inside a prohibition bullet or row.
- English and Korean both use a prohibition label for a sentence that should be split into condition, prohibited behavior, and owner boundary.
- English and Korean both place an effect under a non-effect label, or a non-effect under a result label.
- English and Korean both present a route or owner-boundary note as if it were contract text.
- The parity review passes because both languages match, even though both share the same wrong semantic label or structure.

Fix:
- Correct the semantic label or structure in both languages in the same documentation batch.
- Preserve natural Korean prose while keeping the corrected meaning unit, owner route, and normative strength aligned.
- When the content is contract detail, move it to the canonical owner and leave both paired non-owner pages with a short consequence plus owner link.
