# Authoring guide

Use this guide when changing Harness documentation. It is an authoring and documentation-architecture guide only.

It does not authorize:

- Harness Server/runtime implementation
- product-repository writes
- generated operational files
- runtime state
- projections
- evidence records
- QA records
- acceptance records
- close records
- residual-risk records
- executable fixtures
- conformance runners

This repository remains documentation-only unless the maintainer handoff owner says otherwise in [MVP Plan](../build/mvp-plan.md). Treat the docs as source material for a future Harness Server, not as accepted implementation-ready runtime behavior.

## 1. Documentation architecture

Harness documentation is organized as a routed documentation set with canonical owner documents. Route documents help readers find the right place. Owner documents hold the durable meaning.

Use the compact active routes:

- `docs/doc-index.yaml`
- `docs/*/start.md`
- `docs/*/use/user-guide.md`
- `docs/*/use/agent-guide.md`
- `docs/*/use/judgment-examples.md`
- `docs/*/build/mvp-plan.md`
- `docs/*/reference/README.md`
- `docs/*/later/index.md`
- `docs/*/maintain/authoring-guide.md`
- `docs/*/maintain/translation-guide.md`
- `docs/*/maintain/checks.md`

[docs/doc-index.yaml](../../doc-index.yaml) owns retrieval and routing metadata. It is not runtime configuration and not a contract owner. [docs/terminology-map.yaml](../../terminology-map.yaml) owns bilingual terminology controls when it exists. It does not own API, storage, schema, security, projection, or runtime behavior.

### Maintainer reading path

Use this path for documentation maintenance:

[Authoring Guide](authoring-guide.md) -> [Translation Guide](translation-guide.md) -> [Checks](checks.md) -> [doc-index.yaml](../../doc-index.yaml) -> [Terminology Map](../../terminology-map.yaml).

The maintain path helps editors choose inputs and owners. It does not create runtime state, implementation readiness, acceptance, evidence, close records, or permission to start server coding.

## 2. Canonical owner rule

One concept, one canonical owner. A canonical owner is the one document allowed to define the normative meaning of a product concept, contract, schema family, storage effect, security guarantee, route, or terminology rule.

Other documents may contain a short 1-2 sentence summary plus a link. Do not copy long contract explanations into README, route, or maintain documents. If the same explanation appears in several files, keep the owner version and shrink the others to a reader consequence plus an owner link.

Use these single-owner routes before repeating details:

| Topic | Canonical owner |
|---|---|
| Current MVP boundary and active/later status | [Active MVP Scope](../reference/active-mvp-scope.md) |
| Common API envelopes and response branches | [API Schema Core](../reference/api/schema-core.md) |
| Public error codes and error routing | [Errors](../reference/api/errors.md) |
| Storage effects | [Storage Effects](../reference/storage-effects.md) |
| Security guarantees and access-boundary wording | [Security](../reference/security.md) |
| Product definitions | [Glossary](../reference/glossary.md) |
| Documentation retrieval routes | [doc-index.yaml](../../doc-index.yaml) |

Use these multi-owner routes when the question crosses an owner boundary:

- API method contracts:
  - [MVP API router](../reference/api/mvp-api.md)
  - the method owner documents it lists
- API schema families:
  - [API State Schemas](../reference/api/schema-state.md)
  - [API Artifact Schemas](../reference/api/schema-artifacts.md)
  - [API Judgment Schemas](../reference/api/schema-judgment.md)
  - [API Value Sets](../reference/api/schema-value-sets.md)
- Projection and template owners:
  - [Projection Authority Reference](../reference/projection-and-templates.md)
  - [Template Bodies](../reference/template-bodies.md)
- Surface and connector owners:
  - [Surface Recipes](../use/surface-recipes.md)
  - [Agent Integration](../reference/agent-integration.md)
- Later-candidate routing:
  - [Later Candidate Index](../later/index.md)
  - [Active MVP Scope](../reference/active-mvp-scope.md)
- Translation and bilingual terminology:
  - [Translation Guide](translation-guide.md)
  - [Terminology Map](../../terminology-map.yaml)

Maintain docs own authoring rules and checks. They must not become secondary sources of truth for API, storage, schema, security, access class, close-readiness, projection, runtime, or product contracts.

### Value status stabilization rules

A value name can exist in a schema, example, storage note, route page, or later-candidate list without the current MVP providing that behavior. Treat the name as vocabulary or reserved surface area until [Active MVP Scope](../reference/active-mvp-scope.md) and the semantic owner both say the behavior is active.

Reserved and profile-gated values are not active guarantees. Mark them at the point of use and avoid default, required, supported, enforced, preventive, detective, accepted, verified, or close-ready wording unless the active owner says the profile and behavior are available.

Value-set owner documents define exact value names, validation placement, and enum-like vocabulary. Semantic owner documents define what the value means, whether it is currently available, what guarantee level it carries, and what reader consequence follows. If a value-set entry and a semantic owner appear to disagree, do not infer behavior from the value name. Fix the owner gap or route the reader to the correct owner.

Later-candidate promotion requirements may name the kinds of owners that must change at promotion time. They must not name a non-existing owner as if it were already a current active owner document. If no current owner exists, say that promotion requires creating or designating that owner at promotion time, then updating active scope, schemas, API behavior, storage, templates, checks, and paired-language docs as applicable.

Route documents must expose canonical owner gaps rather than hide them. If a README, index, Start page, Use page, Later page, or `doc-index.yaml` route cannot point to a current owner for the question, do not fill the gap with route prose. Say what is missing, route to the closest real owner or deferred owner, and leave the normative definition out of the route document.

## 3. When to edit an existing owner

Edit an existing owner when the change affects normative meaning. This includes active MVP scope, API behavior, schema meaning, error meaning, storage effects, security wording, access-boundary wording, close-readiness meaning, product terminology, or any rule another document should link to.

Start in the owner, then update route and user-facing documents only as needed. A non-owner edit should usually explain what the reader can expect or where to go next, not restate the contract.

If a duplicate explanation is stale, do not refresh the duplicate. Replace it with a short summary and a link to the owner. If the owner is unclear, fix the owner first.

## 4. When to create a new document

Create a new document only when no existing owner can responsibly hold the concept. The new page must have a stable reader purpose, a clear owner boundary, and a paired English/Korean route when it is part of the active documentation set.

Do not create a new document for temporary planning notes, migration notes, review leftovers, one-off summaries, or duplicated contract extracts. Put implementation-readiness decisions in [MVP Plan](../build/mvp-plan.md). Put contract definitions in the appropriate Reference owner. Put terminology choices in [Glossary](../reference/glossary.md), [Translation Guide](translation-guide.md), or [Terminology Map](../../terminology-map.yaml).

When adding a real new owner, update [Reference README](../reference/README.md) or the appropriate route index so readers can find it. Update [doc-index.yaml](../../doc-index.yaml) only as documentation retrieval metadata.

<a id="active-mvp-api-method-split-threshold"></a>
### Active MVP API method owners

[`reference/api/mvp-api.md`](../reference/api/mvp-api.md) is the stable route document for the active MVP API method family. Method-specific owner documents own active MVP method behavior:

- `reference/api/method-intake.md`
- `reference/api/method-update-scope.md`
- `reference/api/method-status.md`
- `reference/api/method-prepare-write.md`
- `reference/api/method-stage-artifact.md`
- `reference/api/method-record-run.md`
- `reference/api/method-user-judgment.md`
- `reference/api/method-close-task.md`

When active method behavior changes, edit the method owner first. Then update the API router, [Reference README](../reference/README.md), [doc-index.yaml](../../doc-index.yaml), paired-language owner, and practical inbound links that should land on the method owner.

Keep [`reference/api/mvp-api.md`](../reference/api/mvp-api.md) as a route and shared-reading document. It should not duplicate method-specific request bodies, response bodies, result branches, blocked-result details, or storage-effect detail already owned by a method owner.

## 5. Route documents and README files

README files, Start pages, Use pages, Build pages, Later indexes, Maintain pages, and reference indexes route readers. They may say what a document is for, who should read it, and what practical result the reader should expect.

They should not carry copied API response branches, blocker schema details, access class lists, storage effect specifics, security guarantee details, or close-readiness contract explanations. Use a short summary plus a link to the owner instead.

When a README or route document starts to need tables of fields, status values, guarantee levels, storage effects, or error behavior, the content belongs in an owner. Keep the route page focused on navigation.

## 6. User-facing vs reference-facing writing

User-facing docs explain what the reader can decide, expect, or do. Avoid internal schema names unless the exact identifier is necessary for the reader's task. Prefer plain outcomes and link to the owner for contract details.

Reference-facing docs may use schema names, API method names, enum values, table names, and error codes, but exact identifiers must stay in backticks. A reference page may define only the contract it owns. When it mentions a neighboring contract, summarize briefly and link to that owner.

Maintain docs should sound like editing instructions. They can name owner paths and duplication rules, but they should not reproduce technical contract bodies.

### Durable examples

Examples in reference and API documentation should use stable product or user scenarios. They should remain useful after the maintenance context is forgotten.

API examples must be internally consistent. They may share one scenario across method documents, but cross-method examples that share a scenario must use compatible refs, paths, `state_version` values, artifact refs, run refs, judgment refs, and close-readiness evidence. Those values must describe the same timeline and must not contradict each other.

Representative responses may omit unrelated fields, but they must not contradict the request, the visible response state, or the shared scenario. A response snapshot must not include refs from a later `state_version` than the snapshot's `base.state_version` or visible state summary.

Sensitive approval reasons must match request inputs or explicitly stated preconditions. Do not add approval reasons unsupported by `sensitive_categories`, `SensitiveActionScope`, intended paths, intended operation, or the scenario setup.

Artifact refs must be introduced by staging, promotion, or an explicit existing-artifact statement before they appear as evidence, judgment context, or close-readiness support.

Expiration timestamps should use placeholders or clearly future example dates.

The API reference sample task is: add explicit confirmation before account data export, update account data export confirmation tests, and record account data export confirmation test output as representative run/evidence data. When the sample task changes, update the API examples, paired Korean examples, checks, and routes together.

API examples must not use documentation maintenance as the scenario.

Do not make the example scenario documentation maintenance, migration, refactoring, route cleanup, or section restructuring. Repository-internal documentation paths, including paths under `docs/`, should appear as example data only when the document is explicitly about documentation maintenance.

API examples should avoid self-referential documentation edits as task payloads, request examples, response examples, run summaries, artifact descriptions, or user judgment prompts.

## 7. Long paragraph and chunking rules

Reference docs should keep rule boundaries visible. Do not combine condition, effect, exception, non-claim, and owner routing in one dense paragraph. A paragraph should not require the reader to infer where a rule applies, what it permits, what it forbids, which caveat applies, or which owner carries the canonical detail.

Split a dense reference paragraph when it combines more than one rule type, such as a condition, allowed effect, not-allowed effect, exception, non-claim, or owner link.

Use named blocks when a rule has multiple parts:

- Conditions: when the rule applies.
- Allowed effects: what the rule allows or requires.
- Not allowed: what the rule does not allow and what the text does not claim.
- Exceptions: when a different rule or caveat applies.
- Owner links: where the canonical detail lives.

Prefer short paragraphs, compact bullets, and small route tables. If a sentence contains several "must not", "does not", or "only when" clauses, consider a list or named block.

Use Markdown tables only for short mappings, comparisons, or owner routing. The table maintainability rule applies to all Reference documents, not only storage references.

Use a summary row plus a detail block when a cell would need any of these:

- multiple sentences or conditions
- exceptions or non-claims
- allowed or forbidden effects
- owner links
- list-like field, status, guarantee, effect, or route examples

Split the table when a source line becomes hard to review. Move contract detail to the owner instead of hiding it in a dense table cell.

## 8. Cross-language editing

English and Korean docs are both active. Do not finish a meaning-changing batch with only one language updated.

Korean docs must not be literal translations. Maintain semantic parity by meaning unit while allowing natural Korean sentence order, paragraph rhythm, and terminology. Preserve exact identifiers in both languages, including file paths, `doc_id` values, API method names, schema fields, enum values, table names, validator IDs, and error codes.

Korean reference docs must preserve structural meaning units, not just broad topic coverage. Conditions, effects, exceptions, non-claims, owner links, and close-readiness consequences must remain visible as separate meaning units when the English owner uses that structure. Matching line counts are not required, but do not collapse important structure into dense paragraphs that hide a caveat or owner boundary.

Use [Translation Guide](translation-guide.md) and [Terminology Map](../../terminology-map.yaml) for bilingual wording. During normal agent work, load only one language for the same `doc_id`; load both only for translation, parity review, or a bilingual edit where comparison is necessary.

## 9. Link and anchor rules

Link to the canonical owner, not to a convenient duplicate. Prefer the exact section anchor when the owner has one. Use README files and route indexes only when the reader needs navigation rather than contract detail.

Use relative links inside the documentation tree. Keep exact file paths, anchors, identifiers, API methods, schema fields, enum values, table names, validator IDs, and error codes in backticks when they appear in prose.

When changing headings, check inbound links and the paired-language document. Korean headings should stay natural; use hidden anchors when a stable English anchor must be preserved.

Do not route active documentation through stale legacy paths. If an old path appears during review, replace it with the compact current route or remove the stale route wording.

## 10. Pre-merge checklist

- [ ] The edit stayed documentation-only and did not imply runtime implementation.
- [ ] Each concept still has one canonical owner.
- [ ] README, route, and maintain documents use short summaries plus owner links instead of copied contract explanations.
- [ ] API, storage, schema, security, access-boundary, and close-readiness details live in the appropriate Reference owner.
- [ ] Value names are not treated as current MVP behavior merely because they exist in schemas, examples, storage notes, or later-candidate lists.
- [ ] API examples are internally consistent across response snapshots, `state_version`, refs, paths, artifact refs, sensitive approval reasons, expiration timestamps, and shared scenario evidence.
- [ ] Reserved and profile-gated values are labeled where used and are not described as active guarantees.
- [ ] Value-set owners define names; semantic owners define meaning, current availability, guarantees, and reader consequences.
- [ ] Later-candidate promotion wording does not present non-existing owners as current active owner documents.
- [ ] Route documents expose canonical owner gaps instead of hiding them with broad route text.
- [ ] Meaning-changing edits were made in both English and Korean.
- [ ] Korean prose is natural, not a literal translation, and exact identifiers are preserved.
- [ ] Korean reference docs preserve condition, effect, exception, non-claim, and owner-link structure by meaning unit.
- [ ] User-facing docs avoid internal schema names unless necessary.
- [ ] Reference docs keep schema names and other exact identifiers in backticks.
- [ ] Dense reference paragraphs were split into conditions, allowed effects, not-allowed effects, exceptions, and owner links where useful.
- [ ] Tables in all Reference documents use short mappings, and dense cells were moved into summary rows plus detail blocks.
- [ ] Links point to active routes and canonical owners.
- [ ] New or changed terminology was checked against [Terminology Map](../../terminology-map.yaml).
- [ ] No temporary planning files, archive copies, generated runtime records, or migration notes remain.
- [ ] Relevant checks in [Checks](checks.md) were run or reported as skipped.
