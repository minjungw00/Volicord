# Authoring guide

Use this guide when changing Harness documentation. This is an authoring and documentation-architecture guide only.

This guide does not authorize:

- Harness Server/runtime implementation
- product-repository writes
- generated operational files
- runtime state
- projections
- evidence records
- QA results
- acceptance decisions
- close-readiness state
- residual-risk decisions
- executable fixtures
- conformance runners

The documentation tree stores maintained product and system documentation. Runtime outputs, generated records, and product implementation files belong outside this tree.

## 1. Documentation architecture

Harness documentation is organized as a routed documentation set with canonical owner documents. Route documents help readers find the right place. Owner documents hold the durable meaning.

Use the compact maintained routes:

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

[`docs/doc-index.yaml`](../../doc-index.yaml) owns machine-readable documentation owner routing and paired-path metadata. It is not runtime configuration and not a contract owner. [`docs/terminology-map.yaml`](../../terminology-map.yaml) owns complete structured terminology metadata and bilingual wording controls. It does not own API, storage, schema, security, projection, or runtime behavior. The [Glossary](../reference/glossary.md) is a compact reader-facing guide to selected core terms.

In the terminology map, `primary_owner` should point to the focused owner document when one exists, and `related_references` should hold adjacent references. Glossary owner links should follow the same focused-owner rule. Use a broad index only when the concept is itself index-owned navigation or when the focused owner does not yet exist and the gap is named.

### Maintainer reading path

For documentation maintenance, read:

[Authoring Guide](authoring-guide.md) -> [Translation Guide](translation-guide.md) -> [Checks Index](checks.md) -> focused check pages -> [doc-index.yaml](../../doc-index.yaml) -> [Terminology Map](../../terminology-map.yaml).

The maintenance path helps editors choose inputs and owners. It does not create runtime state, acceptance, evidence, close-readiness state, or implementation authority.

## 2. Canonical owner rule

One concept, one canonical owner. A canonical owner is the one document allowed to define the normative meaning of a product concept, contract, schema family, storage effect, security guarantee, route, or terminology rule.

Other documents may contain a short one- or two-sentence summary plus a link. Do not copy long contract explanations into README, route, or maintain documents. If the same explanation appears in several files, keep the owner version and shrink the others to a reader consequence plus an owner link.

Use the [Reference Index](../reference/README.md) for human-readable owner lookup and [`docs/doc-index.yaml`](../../doc-index.yaml) for exact machine-readable metadata. For API method questions, use [`reference/api/methods.md`](../reference/api/methods.md) as the supported method list and first-hop method owner router.

When a question crosses owner boundaries, choose the applicable owner from the Reference Index or `doc-index.yaml`, then load only the owner sections needed for the edit. Do not recreate owner maps inside Maintain guidance.

Maintain docs own authoring rules and checks. They must not become secondary sources of truth for API, storage, schema, security, access class, close-readiness, projection, runtime, or product contracts.

Use "applicable owner path" for topic routing. Do not use `active` for owner routes, supported contracts, supported methods, or maintained reference documents. Reserve `active` for runtime or currently applied state, such as active scope, active Change Unit, active surface context, or exact status values. Do not turn documentation-routing concepts such as `applicable owner path` into product behavior, storage persistence, or runtime state.

Do not repeat the same owner map in multiple documents. Keep the full map in the canonical router or owner, and let other pages use a short purpose summary plus a link. If a repeated map is already present, remove the duplicate instead of refreshing both copies.

### Value status stabilization rules

A value name can exist in a schema, example, storage note, or route page without the baseline scope providing that behavior. Treat the name as vocabulary or reserved surface area until [Scope](../reference/scope.md) and the semantic owner both define the behavior as supported in the baseline scope.

Reserved and profile-gated values are not baseline guarantees. Mark them at the point of use and avoid default, required, supported, enforced, stronger-guarantee, detective, accepted, verified, or close-ready wording unless the semantic owner defines the profile and behavior as supported.

Value-set owner documents define exact value names, validation placement, and enum-like vocabulary. Semantic owner documents define what the value means, whether it is supported, what guarantee level it carries, and what reader consequence follows. If a value-set entry and a semantic owner appear to disagree, do not infer behavior from the value name. Fix the owner gap or route the reader to the correct owner.

Out-of-scope capability promotion requirements may name the kinds of owners that must change when the capability enters the baseline scope. They must not name a nonexistent owner as if it were already an existing owner document. If no applicable owner exists, say that promotion requires creating or designating that owner, then updating baseline scope, schemas, API behavior, storage, templates, checks, and paired-language docs as applicable.

Route documents must expose canonical owner gaps rather than hide them. If a README, index, Start page, Use page, Scope page, or `doc-index.yaml` route cannot point to an applicable owner for the question, do not fill the gap with route prose. Say what is missing, route to the closest real owner, and leave the normative definition out of the route document.

Write excluded-scope logic directly. Prefer "excluded until promoted by the scope owner" or "supported only when the scope owner and affected owners define support" over double negatives such as "not excluded", "not unsupported", or "not outside support".

## 3. When to edit an existing owner

Edit an existing owner when the change affects normative meaning. This includes baseline scope, API behavior, schema meaning, error meaning, storage effects, security wording, access-boundary wording, close-readiness meaning, product terminology, or any rule another document should link to.

Start in the owner, then update route and user-facing documents only as needed. A non-owner edit should usually explain what the reader can expect or where to go next, not restate the contract.

If a duplicate explanation is stale, do not refresh the duplicate. Replace it with a short summary and a link to the owner. If the owner is unclear, fix the owner first.

## 4. When to create a new document

Create a new document only when no existing owner can responsibly hold the concept. The new page must have a stable reader purpose, a clear owner boundary, and a paired English/Korean route when it is part of the maintained documentation set.

Do not create a new document for one-off planning notes, one-off conversion notes, review leftovers, one-off summaries, or duplicated contract extracts. Put implementation decisions in [Implementation Guide](../build/implementation-guide.md). Put contract definitions in the appropriate Reference owner. Put compact selected term meanings in [Glossary](../reference/glossary.md), structured terminology metadata in [Terminology Map](../../terminology-map.yaml), and bilingual parity or Korean prose rules in [Translation Guide](translation-guide.md).

When adding a real new owner, update [Reference README](../reference/README.md) or the appropriate route index so readers can find it. Update [doc-index.yaml](../../doc-index.yaml) only as documentation retrieval metadata.

Split or reroute an existing owner when one reference document starts carrying unrelated concerns. A single owner should not define API behavior, schemas, storage effects, security guarantees, error precedence, templates, and examples simply because a workflow mentions them together. Keep the narrow owner definition in the document that owns the concern, and route neighboring concerns with short links.

When a large owner is split, the new owner must not become the next catch-all document. Give the split document a narrow reader purpose, keep neighboring concerns routed to their focused owners, and update `owner_for` and `not_owner_for` metadata so the boundary remains reviewable.

Build guidance and route metadata should use durable implementation meaning. Avoid build-moment labels, current-work labels, or interim-stage labels that would stop making sense after the implementation path matures.

<a id="baseline-scope-api-method-split-threshold"></a>
### API method owners

[`reference/api/methods.md`](../reference/api/methods.md) is the stable route document for the baseline API method family. It owns the supported public method list and routes each method to the method owner.

When baseline method behavior changes, edit the method owner first. Then update the API router, [Reference README](../reference/README.md), [doc-index.yaml](../../doc-index.yaml), paired-language owner, and practical inbound links that should land on the method owner.

Keep [`reference/api/methods.md`](../reference/api/methods.md) as a route and shared-reading document. It should not duplicate method-specific request bodies, response bodies, result branches, blocked-result details, or storage-effect detail already owned by a method owner.

API error documentation is split by concern. [`reference/api/errors.md`](../reference/api/errors.md) is the family index only. Public `ErrorCode` meanings belong in [API error codes](../reference/api/error-codes.md), precedence and conflict selection belong in [API error precedence](../reference/api/error-precedence.md), rejected-response, blocked-result, and `dry_run` branch routing belongs in [API error routing](../reference/api/error-routing.md), close-readiness blocker routing belongs in [API blocker routing](../reference/api/blocker-routing.md), and machine-readable `ToolError.details` belongs in [API error details](../reference/api/error-details.md).

Keep [API blocker routing](../reference/api/blocker-routing.md) within its owner boundary. It should not own method-specific behavior, `CloseReadinessBlocker` schema shape, blocker category value sets, Core close-readiness authority, or display wording. Route those questions to the method owner, API State Schemas, API Value Sets, Core Model, or Template Bodies as applicable.

## 5. Route documents and README files

README files, Start pages, Use pages, Build pages, Maintain pages, Scope pages, and reference indexes route readers. They may say what a document is for, who should read it, and what practical result the reader should expect.

They should not carry copied API response branches, blocker schema details, access class lists, storage effect specifics, security guarantee details, or close-readiness contract explanations. Use a short summary plus a link to the owner instead.

When a README or route document starts to need tables of fields, status values, guarantee levels, storage effects, or error behavior, the content belongs in an owner. Keep the route page focused on navigation.

Negative rules can also turn a route document into a contract document. A route page may state a short boundary such as what it does not own, but it should not accumulate "Not allowed", "Does not imply", exception, or "must not" lists that define a concept outside the owner. Move durable prohibitions, exceptions, and non-claims to the owner, then keep the route page to a practical consequence plus a link.

## 6. User-facing vs reference-facing writing

User-facing docs explain what the reader can decide, expect, or do. Avoid internal schema names unless the exact identifier is necessary for the reader's task. Prefer plain outcomes and link to the owner for contract details.

Reference-facing docs may use schema names, API method names, enum values, table names, and error codes, but exact identifiers must stay in backticks. A reference page may define only the contract it owns. When it mentions a neighboring contract, summarize briefly and link to that owner.

Display wording owners define rendered body guidance, labels, and display phrasing only. They do not own API semantics, close-readiness blocker semantics, storage records, or wording for out-of-scope rendered bodies. Route those concerns to the focused API, blocker, storage, scope, or terminology owner.

Maintain docs should sound like editing instructions. They can name owner routes and duplication rules, but they should not reproduce technical contract bodies.

### Reference semantic skeletons

Important Reference sections should have a semantic skeleton before prose is written or reshaped. A semantic skeleton is the intended sequence of meaning units for the section. It is an authoring control, not a product contract, runtime behavior, rendered label set, or replacement for the owner document.

Common skeletons include:

- `Purpose`
- `Conditions`
- `Result`
- `Non-claim`
- `Owner boundary`
- `Related references`

Another common skeleton is:

- `Meaning`
- `Contract`
- `Boundary`
- `Related references`

Use semantic labels only for the content they name:

- `Not allowed`: prohibited actions or states.
- `Required behavior`: required behavior.
- `Result`: outcomes or reader-visible consequences.
- `Does not imply`: non-implications and non-claims.
- `Owner boundary`: ownership boundaries and routing limits.

Give conditional prohibitions an extra pass. A `Not allowed` unit should name the prohibited behavior. If a prohibition also carries an `unless`, `only when`, or `except when` condition, consider splitting the unit into:

- `Conditions`: when the condition or exception applies.
- `Not allowed`: the behavior that remains prohibited.
- `Owner boundary`: the document that owns the condition, authority boundary, or route.

Do not hide a condition that tells readers when something may be treated as authority inside a prohibition bullet when that makes the sentence hard to parse.

Do not fix a mislabeled unit by copying the same label into both languages. Check the content type first: required behavior is not a prohibited action, an effect is not a non-effect, and a route is not a contract. Rename the label, move the content, or route to the owner so the label and content agree.

Use the same skeleton for the same section in English and Korean. Korean may use natural sentence order, split or combine sentences, and choose natural headings or bullets, but it must not introduce extra labels such as `조건`, `결과`, `비주장`, or `허용되지 않는 것` unless the English section has the equivalent meaning unit. English must not use `Not allowed`, `Does not imply`, or `Non-claim` sections unless Korean has the equivalent meaning unit. Sentence count may differ; meaning-unit placement and normative strength must match.

### Durable examples

Examples in reference and API documentation should use stable product or user scenarios. They should remain useful after the maintenance context is forgotten.

Example field names follow owner boundaries. When an example reuses method payload data, use the field name from the method or schema owner. If a different field name is used as storage summary data, say explicitly that it is storage-owned summary data. Do not mix field names for the same concept across related examples. Route field-definition questions to the method, schema, or storage owner document.

API examples must be internally consistent. They may share one scenario across method documents, but cross-method examples that share a scenario must use compatible refs, paths, `state_version` values, artifact refs, run refs, judgment refs, and close-readiness evidence. Those values must describe the same timeline and must not contradict each other.

When a shared example scenario appears in both languages, the Korean version should use natural Korean phrasing rather than mechanically preserving the English noun chain. After the scenario is introduced, Korean wording may be shorter as long as the meaning remains the same. Repeated scenario phrases should stay consistent across related examples.

Representative responses may omit unrelated fields, but they must not contradict the request, the visible response state, or the shared scenario. A response snapshot must not include refs from a newer `state_version` than the snapshot's `base.state_version` or visible state summary.

Sensitive approval reasons must match request inputs or explicitly stated preconditions. Do not add approval reasons unsupported by `sensitive_categories`, `SensitiveActionScope`, intended paths, intended operation, or the scenario setup.

Artifact refs must be introduced by staging, promotion, or an explicit existing-artifact statement before they appear as evidence, judgment context, or close-readiness support.

Expiration timestamps should use placeholders or clearly future example dates.

The API reference sample task is: add explicit confirmation before account data export, update account data export confirmation tests, and record account data export confirmation test output as representative run/evidence data. When the sample task changes, update the API examples, paired Korean examples, checks, and routes together.

API examples must not use documentation maintenance as the scenario.

Do not make the example scenario documentation maintenance, migration, refactoring, route reshaping, or section restructuring. Repository-internal documentation paths, including paths under `docs/`, should appear as example data only when the document is explicitly about documentation maintenance.

API examples should avoid self-referential documentation edits as task payloads, request examples, response examples, run summaries, artifact descriptions, or user judgment prompts.

## 7. Long paragraph and chunking rules

Reference and Maintain docs should keep rule boundaries visible. Do not combine condition, effect, exception, non-claim, and owner routing in one dense paragraph. A paragraph should not require the reader to infer where a rule applies, what it permits, what it forbids, which caveat applies, or which owner carries the canonical detail.

Split a dense paragraph when it combines more than one rule type, such as a condition, allowed effect, not-allowed effect, exception, non-claim, or owner link.

Use named blocks when a rule has multiple parts:

- Conditions: when the rule applies.
- Allowed effects: what the rule allows or requires.
- Disallowed effects: what the rule forbids and what the text does not claim.
- Exceptions: when a different rule or caveat applies.
- Owner links: where the canonical detail lives.

Prefer short paragraphs, compact bullets, and small route tables. If a sentence contains several "must not", "does not", or "only when" clauses, consider a list or named block.

For check entries, use named blocks instead of dense table cells. Use non-owner labels for check-card document lists:

- Check sources: documents that define the check basis.
- Applies to: files or document families the check applies to.
- Evidence to inspect: concrete documents, examples, links, fields, labels, or route metadata to inspect.
- Pass condition: success criteria.
- Related checks: related check IDs or sections.

Every maintain-check card should expose its basis, scope, evidence, and success criterion with `Check sources`, `Applies to`, `Evidence to inspect`, and `Pass condition` when those roles are present. Remediation labels such as `Failure` and `Fix` may appear in addition to those blocks, but they do not replace the pass condition.

Use ownership labels only for term or contract ownership:

- Primary owner: the single focused owner for a glossary term or terminology-map `primary_owner`.
- See also or Related references: adjacent term or reference documents that help the reader, but do not own the term.
- Route or Reference route: documentation navigation to a reader destination, not ownership.
- Check sources: documents that define a maintenance check basis.
- Applies to: files or document families inspected by a maintenance check.
- Maintained with: companion maintenance pages that should change or be reviewed in the same batch.

Do not use the old owner label for check-card source lists. Do not use `Primary owner` for check inputs, adjacent references, route destinations, or companion maintenance pages.

Use Markdown tables only for short mappings, comparisons, or owner routing. The table maintainability rule applies to all documentation, including Reference and Maintain docs.

Use a summary row plus a detail block when a cell would need any of these:

- multiple sentences or conditions
- exceptions or non-claims
- allowed or forbidden effects
- owner links
- list-like field, status, guarantee, effect, or route examples

Split the table when a source line becomes hard to review. Move contract detail to the owner instead of hiding it in a dense table cell.

## 8. Cross-language editing

English and Korean docs are both maintained. Do not finish a meaning-changing batch with only one language updated.

Korean docs must not be literal translations. Maintain semantic parity by meaning unit while allowing natural Korean sentence order, paragraph rhythm, and terminology. Preserve exact identifiers in both languages, including file paths, `doc_id` values, API method names, schema fields, enum values, table names, validator IDs, and error codes.

Korean reference docs must preserve structural meaning units, not just broad topic coverage. Conditions, effects, exceptions, non-claims, owner links, and close-readiness consequences must remain visible as separate meaning units when the English owner uses that structure. Matching line counts are not required, but do not collapse important structure into dense paragraphs that hide a caveat or owner boundary.

Heading parity is not semantic parity. Use matching headings as the start of comparison, then check the meaning units under those headings.

Table parity includes table count in changed sections, header meanings, row meanings, and placement relative to sections. Row counts may differ only when each condition, value, exception, non-claim, and owner route remains reviewable by meaning.

List parity includes normative lists, allowed and not-allowed clauses, does-not-imply clauses, exceptions, and owner-boundary lists. Korean prose may use a natural rhythm, but it must not drop or absorb a list item that carries a rule, exception, non-claim, or owner link.

Negative-clause parity includes one-sided prohibition, exception, and non-claim markers. Check English markers such as `Not allowed`, `Does not imply`, `Not implied`, and `must not`, and Korean markers such as `허용되지 않는 것`, `의미하지 않는 것`, and `해서는 안 됩니다`. One language must not impose stronger prohibitions, broader exceptions, or stronger non-claims than the other. If one language puts a prohibition in a table, list, or named block, keep the paired meaning unit in the corresponding place.

When a label or concept is removed, search the paired language for exact strings, paraphrases, translations, and mixed-language variants. A removed English label must not survive through Korean prose unless a terminology owner intentionally preserves it as a searchable forbidden expression.

Use [Translation Guide](translation-guide.md) and [Terminology Map](../../terminology-map.yaml) for bilingual wording. During normal agent work, load only one language for the same `doc_id`; load both only for translation, parity review, or a bilingual edit where comparison is necessary.

## 9. Link and anchor rules

Link to the canonical owner, not to a convenient duplicate. Prefer the exact section anchor when the owner has one. Use README files and route indexes only when the reader needs navigation rather than contract detail.

Use relative links inside the documentation tree. Keep exact file paths, anchors, identifiers, API methods, schema fields, enum values, table names, validator IDs, and error codes in backticks when they appear in prose.

When changing headings, check inbound links and the paired-language document. Korean headings should stay natural; use hidden anchors when a stable English anchor must be preserved.

Hidden anchors for a concept belong in the document that actually owns the concept. Do not leave redirect-style hidden anchors in an old document when they make that document look like the owner. Anchor IDs should not imply that a route page, broad index, or former owner still owns a moved concept.

Do not route maintained documentation through stale legacy paths. If an old path appears during review, replace it with the compact maintained route or remove the stale route wording.

## 10. Pre-merge checklist

- [ ] The edit stayed documentation and did not imply runtime implementation.
- [ ] Each concept still has one canonical owner.
- [ ] No single reference owner now carries unrelated API, schema, storage, security, error, template, and example concerns that should be split or routed.
- [ ] A newly split Reference owner did not grow into another broad catch-all owner.
- [ ] Terminology-map and glossary owner targets point to focused owners when focused owners exist, not to broad indexes.
- [ ] Glossary entries name exactly one `Primary owner`; adjacent documents use `See also` or `Related references`.
- [ ] Maintain check cards use `Check sources` for check basis documents and `Applies to` for checked files or document families.
- [ ] Repeated owner maps were reduced to the canonical map plus links.
- [ ] README, route, and maintain documents use short summaries plus owner links instead of copied contract explanations.
- [ ] Route and index documents do not define contracts through accumulated negative rules, exception lists, or non-claim tables.
- [ ] API, storage, schema, security, access-boundary, and close-readiness details live in the appropriate Reference owner.
- [ ] API error code meanings, precedence, response branch routing, close-readiness blocker routing, and machine-readable details route to their separate API owners.
- [ ] API blocker-routing docs do not own method behavior, schema shape, value sets, Core authority, or display wording.
- [ ] Display wording owners do not define API semantics, blocker semantics, storage records, or out-of-scope rendered-body wording.
- [ ] Value names are not treated as baseline scope behavior merely because they exist in schemas, examples, storage notes, or out-of-scope lists.
- [ ] Removed or unsupported concept names do not remain in glossary, terminology-map, metadata, negative examples, or display wording owners unless a terminology owner intentionally preserves a searchable banned expression.
- [ ] Storage record references name persisted record families from the storage owner and do not create storage-like family names through negative examples.
- [ ] `active` is used only for runtime or currently applied state, exact identifiers, or status values, not for supported contracts or owner routing.
- [ ] Documentation-routing concepts such as `applicable owner path` are not described as product behavior, storage persistence, runtime state, or actors for product behavior.
- [ ] Implementation guidance and metadata use durable implementation wording, not build-moment or interim-stage labels.
- [ ] Example field names come from the method, schema, or storage owner, and storage-owned summary data is labeled where it uses a different field name.
- [ ] API examples are internally consistent across response snapshots, `state_version`, refs, paths, artifact refs, sensitive approval reasons, expiration timestamps, and shared scenario evidence.
- [ ] Reserved and profile-gated values are labeled where used and are not described as baseline guarantees.
- [ ] Value-set owners define names; semantic owners define meaning, support availability, guarantees, and reader consequences.
- [ ] Out-of-scope promotion wording does not present non-existing owners as existing owner documents.
- [ ] Excluded-scope logic is written directly, without double negatives.
- [ ] Route documents expose canonical owner gaps instead of hiding them with broad route text.
- [ ] References to `docs/doc-index.yaml` name structures and keys that actually exist.
- [ ] Meaning-changing edits were made in both English and Korean.
- [ ] Important Reference sections have a defined semantic skeleton before prose is written or reshaped.
- [ ] Conditional prohibitions split condition, prohibited behavior, and owner boundary when one sentence would make the label unclear.
- [ ] Paired English/Korean headings keep equivalent meaning and reading structure.
- [ ] Heading parity was not treated as sufficient for bilingual semantic parity.
- [ ] Paired English/Korean sections use the same semantic skeleton for the same section.
- [ ] Korean `허용되지 않는 것` text that uses `되어야 합니다` or `해야 합니다` was checked for required behavior.
- [ ] Paired tables were checked for count, headers, row meanings, and placement relative to sections.
- [ ] Paired normative lists, allowed/not-allowed clauses, does-not-imply clauses, exceptions, and owner-boundary lists were checked by meaning unit.
- [ ] One-sided negative clauses were checked so one language does not impose stronger prohibitions, exceptions, or non-claims than the other.
- [ ] Negative clauses kept corresponding placement when one language used a table, list, or named block.
- [ ] Removed concept labels do not survive through Korean paraphrase, translation, mixed-language variants, tables, lists, headings, or metadata.
- [ ] Korean prose is natural, not a literal translation, and exact identifiers are preserved.
- [ ] Korean prose avoids unnecessary English common nouns when they are not identifiers, product labels, or natural technical borrowings.
- [ ] Korean reference docs preserve condition, effect, exception, non-claim, and owner-link structure by meaning unit.
- [ ] Hidden anchors for moved concepts live in the actual owner and do not make an old document look like it still owns the concept.
- [ ] User-facing docs avoid internal schema names unless necessary.
- [ ] Reference docs keep schema names and other exact identifiers in backticks.
- [ ] Dense reference paragraphs were split into conditions, allowed effects, not-allowed effects, exceptions, and owner links where useful.
- [ ] Tables in all documentation use short mappings, and dense cells were moved into summary rows plus detail blocks.
- [ ] Check descriptions use named blocks and bullets instead of dense table cells.
- [ ] Links point to maintained routes and canonical owners.
- [ ] New or changed terminology was checked against [Terminology Map](../../terminology-map.yaml).
- [ ] No one-off planning files, archive copies, working-note remnants, ad hoc files, generated runtime records, unresolved task markers, one-off conversion notes, or review leftovers remain.
- [ ] Relevant checks in [Checks](checks.md) and its focused check pages were run or reported as skipped.
