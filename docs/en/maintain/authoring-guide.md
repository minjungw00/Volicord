# Authoring guide

Use this guide when changing Harness documentation. It is an authoring and documentation-architecture guide only. It does not authorize Harness Server/runtime implementation, product-repository writes, generated operational files, runtime state, projections, evidence records, QA records, acceptance records, close records, residual-risk records, executable fixtures, or conformance runners.

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

## 2. Canonical owner rule

One concept, one canonical owner. A canonical owner is the one document allowed to define the normative meaning of a product concept, contract, schema family, storage effect, security guarantee, route, or terminology rule.

Other documents may contain a short 1-2 sentence summary plus a link. Do not copy long contract explanations into README, route, or maintain documents. If the same explanation appears in several files, keep the owner version and shrink the others to a reader consequence plus an owner link.

Use these owner routes before repeating details:

| Topic | Canonical owner |
|---|---|
| Current MVP boundary and active/later status | [Active MVP Scope](../reference/active-mvp-scope.md) |
| API method contracts | [MVP API](../reference/api/mvp-api.md) |
| Shared core schemas and close readiness reference terms | [Core Schema](../reference/api/schema-core.md) |
| Public error codes and error routing | [Errors](../reference/api/errors.md) |
| Storage effects | [Storage Effects](../reference/storage-effects.md) |
| Security guarantees and access-boundary wording | [Security](../reference/security.md) |
| Product definitions | [Glossary](../reference/glossary.md) |
| Translation and bilingual terminology practice | [Translation Guide](translation-guide.md) and [Terminology Map](../../terminology-map.yaml) |
| Documentation retrieval routes | [doc-index.yaml](../../doc-index.yaml) |

Maintain docs own authoring rules and checks. They must not become secondary sources of truth for API, storage, schema, security, access class, close-readiness, projection, runtime, or product contracts.

## 3. When to edit an existing owner

Edit an existing owner when the change affects normative meaning. This includes active MVP scope, API behavior, schema meaning, error meaning, storage effects, security wording, access-boundary wording, close-readiness meaning, product terminology, or any rule another document should link to.

Start in the owner, then update route and user-facing documents only as needed. A non-owner edit should usually explain what the reader can expect or where to go next, not restate the contract.

If a duplicate explanation is stale, do not refresh the duplicate. Replace it with a short summary and a link to the owner. If the owner is unclear, fix the owner first.

## 4. When to create a new document

Create a new document only when no existing owner can responsibly hold the concept. The new page must have a stable reader purpose, a clear owner boundary, and a paired English/Korean route when it is part of the active documentation set.

Do not create a new document for temporary planning notes, migration notes, review leftovers, one-off summaries, or duplicated contract extracts. Put implementation-readiness decisions in [MVP Plan](../build/mvp-plan.md). Put contract definitions in the appropriate Reference owner. Put terminology choices in [Glossary](../reference/glossary.md), [Translation Guide](translation-guide.md), or [Terminology Map](../../terminology-map.yaml).

When adding a real new owner, update [Reference README](../reference/README.md) or the appropriate route index so readers can find it. Update [doc-index.yaml](../../doc-index.yaml) only as documentation retrieval metadata.

## 5. Route documents and README files

README files, Start pages, Use pages, Build pages, Later indexes, Maintain pages, and reference indexes route readers. They may say what a document is for, who should read it, and what practical result the reader should expect.

They should not carry copied API response branches, blocker schema details, access class lists, storage effect specifics, security guarantee details, or close-readiness contract explanations. Use a short summary plus a link to the owner instead.

When a README or route document starts to need tables of fields, status values, guarantee levels, storage effects, or error behavior, the content belongs in an owner. Keep the route page focused on navigation.

## 6. User-facing vs reference-facing writing

User-facing docs explain what the reader can decide, expect, or do. Avoid internal schema names unless the exact identifier is necessary for the reader's task. Prefer plain outcomes and link to the owner for contract details.

Reference-facing docs may use schema names, API method names, enum values, table names, and error codes, but exact identifiers must stay in backticks. A reference page may define only the contract it owns. When it mentions a neighboring contract, summarize briefly and link to that owner.

Maintain docs should sound like editing instructions. They can name owner paths and duplication rules, but they should not reproduce technical contract bodies.

## 7. Long paragraph and chunking rules

Long paragraphs should be split into condition/result/exception/owner-link blocks. Use this shape when a paragraph is trying to carry a rule, a consequence, and a caveat at the same time:

- Condition: when the rule applies.
- Result: what the editor should do.
- Exception: when the rule does not apply.
- Owner link: where the canonical detail lives.

Prefer short paragraphs, compact bullets, and small tables for routing. If a paragraph needs several examples of fields, status values, storage outcomes, or guarantee levels, that is usually a sign the text has become contract material and should move to the owner.

## 8. Cross-language editing

English and Korean docs are both active. Do not finish a meaning-changing batch with only one language updated.

Korean docs must not be literal translations. Maintain semantic parity by meaning unit while allowing natural Korean sentence order, paragraph rhythm, and terminology. Preserve exact identifiers in both languages, including file paths, `doc_id` values, API method names, schema fields, enum values, table names, validator IDs, and error codes.

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
- [ ] Meaning-changing edits were made in both English and Korean.
- [ ] Korean prose is natural, not a literal translation, and exact identifiers are preserved.
- [ ] User-facing docs avoid internal schema names unless necessary.
- [ ] Reference docs keep schema names and other exact identifiers in backticks.
- [ ] Long paragraphs were split into condition/result/exception/owner-link blocks where useful.
- [ ] Links point to active routes and canonical owners.
- [ ] New or changed terminology was checked against [Terminology Map](../../terminology-map.yaml).
- [ ] No temporary planning files, archive copies, generated runtime records, or migration notes remain.
- [ ] Relevant checks in [Checks](checks.md) were run or reported as skipped.
