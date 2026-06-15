# Checks

Use these read-only documentation checks after documentation edits. This index routes to focused maintenance procedures only. It does not define API, storage, schema, security, runtime, projection, evidence, QA, acceptance, close-readiness, or residual-risk contracts.

Check pages may name forbidden strings as search patterns. Treat those strings as review inputs only, not as documentation statements to keep or publish outside explicit search-pattern lists.

In check pages, `Evidence to inspect` means documentation evidence for the reviewer. `Failure` means a documentation quality failure, and `Fix` means a direction for repairing documentation.

Run the checks that match the edit. For most documentation batches, start with [Structure checks](checks/structure.md) and [Links and indexes checks](checks/links-and-indexes.md), then add the focused pages that match the changed content.

## Check pages

[Structure checks](checks/structure.md) cover owner placement and document shape. Use them for owner boundaries and granularity, route-page over-detailing, index-as-owner errors, check-card labels, semantic label/content consistency, display wording boundaries, storage-record references, baseline/out-of-scope wording, implementation wording, reference-claim placement, residue search patterns, readability, and final reports.

[Language parity checks](checks/language-parity.md) cover paired English/Korean meaning. Use them for semantic-strength parity, meaning-unit skeletons, headings, tables, lists, negative clauses, removed-concept residue, identifier preservation, Korean structure, Korean technical style, and cases where both languages share the same wrong label or structure.

[Terminology checks](checks/terminology.md) cover terminology-map alignment and wording discipline. Use them for Harness/Core wording, verification-criteria wording, current-scope wording, `Write Authorization` distinctions, glossary scope, owner-label usage, glossary links, mixed-language Korean, blocker terminology, `active` wording, `complete` ambiguity, retired names, close-readiness wording, and access/security wording.

[API examples checks](checks/api-examples.md) cover documentation examples in API and Reference pages. Use them for durable method-local scenarios, field/value/schema/value-set correctness, string-like value boundaries, response snapshot consistency, refs, timestamps, cross-method scenario spine detection, no fixed shared sample task requirement, and API owner routing in examples.

[Links and indexes checks](checks/links-and-indexes.md) cover navigation and route metadata. Use them for relative links, anchors, moved-concept anchors, `README` routes, `doc-index.yaml` as the canonical machine-readable owner route, terminology and metadata targets, glossary-link correctness, reserved/profile-gated value routes, index-as-owner errors, owner gaps, API error owner routing, method-router placement, and LLM retrieval routes.

## Result labels

Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance check outcomes. A passing documentation check is not runtime conformance, implementation acceptance, QA completion, close readiness, product guarantee, or residual-risk acceptance.

Tie findings to file paths, owner documents, and suggested documentation fixes. If a check is skipped, state the reason.

## Report shape

Use a compact report shape after meaningful documentation edits:

- Scope:
- Changed files:
- Checks run:
- Findings:
- Skipped checks:
- Residual documentation risks:

The report should identify results as documentation-maintenance findings when that distinction could be unclear.
