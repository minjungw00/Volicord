# Authoring guide

Use this guide when changing maintained Harness documentation. It explains documentation roles, canonical owner editing, and how to use maintained metadata.

This is a documentation-maintenance guide. It does not define product behavior, API behavior, storage effects, security guarantees, runtime behavior, projections, evidence records, QA results, acceptance decisions, close-readiness state, residual-risk decisions, executable fixtures, or conformance runners.

The documentation tree stores maintained product and system documentation. Runtime outputs, generated records, product implementation files, and transient maintenance files belong outside this tree.

## 1. Canonical metadata

Use [`docs/doc-index.yaml`](../../doc-index.yaml) as the machine-readable owner route. It owns `doc_id`, paired paths, document role, owner scope, non-owner scope, dependencies, normative level, and audience metadata. It is documentation metadata only; it is not runtime configuration and not a product contract.

Use [`docs/terminology-map.yaml`](../../terminology-map.yaml) as the terminology and identifier-preservation source of truth. It owns structured term metadata, exact identifier handling, preferred expressions, discouraged expressions, and Korean wording controls. It does not define API, storage, schema, security, projection, or runtime behavior.

Use the [Reference Index](../reference/README.md) as a human-readable route to owner documents. Use the [Glossary](../reference/glossary.md) as a compact reader-facing subset of selected terms. Do not copy detailed owner maps from `doc-index.yaml` or full terminology records from `terminology-map.yaml` into Maintain guidance.

## 2. Documentation roles

Owner documents define durable meaning for the specific concern they own. A product concept, API behavior, schema family, storage effect, security guarantee, value meaning, or close-readiness concept should have one canonical owner.

Classify route-only pages and indexes from `doc-index.yaml` metadata such as `role`, `owner_for`, and `normative_level`, not from a broad page-family label. `reference.scope` is a contract owner for baseline scope, the supported boundary, the out-of-scope boundary, the profile-gated boundary, and reserved behavior; treat it as the owner for those questions while routing API, storage, security, and other focused details to their own owners.

Route documents help readers choose the next document. README files, Start pages, Use pages, Build pages, Maintain pages, and reference indexes may state purpose, audience, and next steps, but they should not become technical contracts.

Maintain documents guide authors, translators, and reviewers. They may explain how to find owners, preserve terminology, and run checks. They must not become secondary sources for API behavior, storage effects, schemas, security guarantees, access boundaries, runtime state, close-readiness contracts, or product implementation.

Check documents describe read-only documentation checks. `PASS`, `WARN`, `FAIL`, and `SKIP` are documentation-quality outcomes only. A passing documentation check is not runtime conformance, product acceptance, QA completion, close readiness, or residual-risk acceptance.

## 3. Owner Editing

Edit the canonical owner when a change affects normative meaning. This includes baseline scope, API behavior, schema meaning, error meaning, storage effects, security wording, access boundaries, close-readiness meaning, product terminology, out-of-scope promotion rules, and value-set meaning.

Non-owner documents should usually give a short reader consequence and link to the owner. If a duplicate explanation is stale, shrink it to that summary and link instead of refreshing a second contract body.

When a question crosses owner boundaries, choose the focused owner from `doc-index.yaml` or the Reference Index, then read only the owner sections needed for the edit. If no focused owner exists, name the owner gap or route to the closest applicable owner. Do not fill the gap with contract prose in a README, route page, or Maintain page.

Create a new maintained document only when no existing owner can responsibly hold the concept. The new document needs a stable reader purpose, a clear owner boundary, paired English and Korean routes when it belongs to the maintained set, and updated route metadata.

Keep baseline behavior separate from reserved, profile-gated, and out-of-scope material. A value name can appear in schemas, examples, storage notes, or route pages without becoming baseline behavior. Use the semantic owner and the value-set owner for meaning, support status, validation placement, and reader consequence.

Keep reference meaning units reviewable. Conditions, results, exceptions, non-claims, and owner boundaries should not be hidden inside dense paragraphs or oversized table cells. When paired English and Korean sections use a meaning-unit structure, keep the same structure by meaning rather than by line count.

## 4. Route Pages

Keep route-only and index pages short and navigational. If a route-only page or index starts to need field tables, status-value tables, storage-effect detail, error behavior, guarantee levels, or long lists of prohibitions and exceptions, move that material to the applicable owner and leave a short route link.

Contract owner pages may contain the contract detail that belongs to their `owner_for` scope. They should still avoid duplicating other owners' API behavior, schema fields, storage effects, security guarantees, or adjacent contracts.

Do not repeat the same owner map in multiple places. Keep the full machine-readable map in `doc-index.yaml`, keep human-readable owner routing in the relevant reference route, and let Maintain pages explain how to use those sources.

Treat documentation routes, path allowlists, check scopes, and batch boundaries as maintainer controls. Do not describe them as Harness runtime behavior, product override capability, persisted Core state, or product implementation output.

## 5. Examples

Examples in API and Reference documentation should be stable, self-contained product or user scenarios. They should illustrate the documented shape without creating product policy.

API method reference examples must be method-local minimal examples. Each method owner document may use a stable product or user scenario, but the example must introduce every ref, `state_version` fact, artifact ref, run ref, judgment ref, blocker ref, and file path it needs within that method document or explicitly state it as a method-local precondition.

A method example must not depend on refs, state versions, artifact refs, run refs, judgment refs, blocker refs, or file paths introduced by another method reference document. Repeated business nouns, path prefixes, ref prefixes, judgment ID families, run ID families, artifact ID families, blocker code families, or scenario descriptions across three or more method owner documents form a cross-method scenario spine. API method reference examples must not use such a spine, even when the repeated data matches a conformance scenario or another scenario-level document.

Conformance scenarios may define scenario-level criteria, but they are not shared fixtures for method references. A method example may link conceptually to a conformance scenario, but it must not copy that scenario's payload, refs, paths, `state_version`, artifact refs, run refs, judgment refs, blocker refs, or response snapshots for consistency. API method examples do not define conformance criteria, and conformance scenarios must not require method reference documents to reuse their payloads.

Review API examples against schema owners, value-set owners, method-local input rules, and storage-effect owners where applicable. Unsupported enum-like values in examples are documentation failures unless the field is explicitly free-form; stale response shapes in examples are documentation failures.

For string-like fields in examples, first identify the owner-defined class: controlled value string, opaque identifier or classification string, or free-form display string. Controlled values must match the value-set owner, opaque strings must not be treated as global enums, and display strings must not be used as canonical schema values.

Do not use documentation maintenance, migration, refactoring, route reshaping, or section restructuring as ordinary product API example payloads. Repository-internal documentation paths, including paths under `docs/`, should appear as example data only when the document is specifically about documentation maintenance.

Use [API examples checks](checks/api-examples.md) for detailed example review, including field-name consistency, required and nullable fields, enum-like values, `NextActionSummary`, response branches, refs and `state_version`, artifact refs, judgment refs, close-readiness blockers, and cross-method scenario spine detection.

## 6. Terminology And Language

English and Korean documentation are both maintained. Do not finish a meaning-changing edit with only one language updated when the document has a maintained paired path.

Use the [Translation Guide](translation-guide.md) for bilingual meaning parity, Korean style, identifier preservation, and hidden-anchor practice. Preserve exact identifiers, file paths, anchors, API methods, schema fields, enum values, status values, product labels, and code literals exactly where the terminology map requires it.

Apply these terminology decisions from `terminology-map.yaml`:

- Harness is the local work-authority product/system for AI-assisted product work. Core is the local authority record for Harness state.
- Use "verification criteria" for user-visible criteria used to check work.
- Use "current scope" or "currently applied scope" in prose context, and preserve exact identifiers that contain `active`.
- Keep `Write Authorization` distinct from ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, and broad user-owned judgment.
- In Korean reference prose, close readiness is "닫기 준비 상태".

## 7. Validation

After documentation edits, run or perform the checks that match the changed files. Start from [Checks](checks.md), then use focused pages such as Structure checks, Links and indexes checks, Language parity checks, Terminology checks, and API examples checks as applicable.

Before finishing, confirm changed links, file paths, anchors, paired-language links, owner routing, and terminology. Confirm no generated records, archive copies, transient maintenance files, or unresolved placeholders remain in the repository.
