# Structure checks

Use these checks for documentation architecture, owner boundaries, route-page structure, reference-claim placement, and final report shape. These are documentation quality checks only; they do not certify product runtime behavior.

## CHK-STRUCT-001: review scope inputs

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
- Identify changed files, paired-language files, touched headings, and touched anchors.
- For each contract-like statement, identify one canonical owner from the Reference Index or `doc-index.yaml`.
- For terminology questions, include [Terminology Map](../../../terminology-map.yaml) as an input.

Failure:
- The review starts from an unspecified scope, a full Reference dump, a stale route, or both languages for the same `doc_id` when parity review is not needed.
- A strict contract is checked without naming its owner.

Fix:
- Reduce inputs to changed files, needed paired files, and owner sections needed for the next check.
- Replace stale routes with compact active routes before continuing.

## CHK-STRUCT-002: maintenance result labels

Owner:
- [Checks Index](../checks.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance labels.
- Keep findings tied to file paths, owner documents, and suggested documentation fixes.

Failure:
- The report treats a check result as documentation acceptance, implementation routing, runtime conformance, final acceptance, QA, close readiness, residual-risk acceptance, or implementation authority.

Fix:
- Reword the output as a documentation maintenance result.
- Route implementation questions to [Implementation Guide](../../build/implementation-guide.md).

## CHK-STRUCT-003: no generated runtime outputs

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Runtime Boundaries](../../reference/runtime-boundaries.md)

Check:
- Confirm documentation checks produced review notes only.
- Confirm they did not create or simulate Harness runtime records, generated projections, operational artifacts, executable fixtures, conformance reports, QA records, acceptance records, close records, residual-risk records, or product writes.

Failure:
- A documentation check leaves behind generated operational files, runtime-like state, fixture output, migration notes, archive copies, or one-off planning files.

Fix:
- Remove generated or transient material.
- Keep the result in the final review report only.

## CHK-OWNER-001: canonical owner violations

Owner:
- [Reference Index](../../reference/README.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- For API, schema, storage, security, access-boundary, projection, template, close-readiness, judgment, error, and runtime-boundary statements, confirm the strict definition lives in one canonical owner.
- Confirm non-owner documents use only a short reader consequence plus an owner link.

Failure:
- `README`, Start, Use, Build, Maintain, example, or non-owner Reference text creates a second normative definition.
- A non-owner repeats field lists, response branches, storage details, guarantee levels, blocker details, access-class rules, or template bodies instead of linking to the owner.

Fix:
- Keep the owner definition.
- Shrink duplicates to a short consequence and owner link.

## CHK-OWNER-002: route-page over-detailing

Owner:
- [Reference Index](../../reference/README.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Inspect `README` files, Start pages, Use pages, Build pages, Maintain pages, Scope pages, and route indexes for contract tables, long field explanations, status-value lists, security guarantee details, storage-effect details, and API branch summaries.
- Confirm those pages route readers instead of defining contracts.

Failure:
- A route page becomes useful as a standalone technical contract.
- A route list tries to enumerate every contract detail, owner subcase, status value, schema branch, storage effect, or security guarantee.

Fix:
- Move normative detail to the canonical owner if it is missing there.
- Replace route-page detail with reader purpose, expected result, and owner links.

## CHK-OWNER-003: value-set names versus semantic ownership

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [Scope](../../reference/scope.md)
- [API Value Sets](../../reference/api/schema-value-sets.md)

Check:
- For status values, enum-like values, profile-gated values, reserved values, access classes, guarantee labels, blocker categories, and display values, identify both the value-set owner and the semantic owner.
- Confirm the value-set owner is used for exact names and validation placement only.
- Confirm active behavior, current availability, guarantee level, and reader consequence come from the semantic owner.

Failure:
- A value name is treated as active behavior or an active guarantee merely because it appears in a schema, example, storage note, route page, or out-of-scope list.
- Reserved or profile-gated values appear without their reserved/profile-gated status at the point of use.

Fix:
- Reword the statement as reserved, profile-gated, deferred, or vocabulary-only until the semantic owner says the behavior is active.
- Link to the semantic owner for meaning and current availability.
- If no semantic owner exists, expose the owner gap instead of inferring behavior from the value name.

## CHK-SCOPE-001: active/out-of-scope leakage

Owner:
- [Scope](../../reference/scope.md)
- [Implementation Guide](../../build/implementation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Inspect changed active docs, examples, route text, and summaries for out-of-scope capabilities presented as baseline scope behavior.
- Confirm profile-gated or reserved values are labeled at the point of use.
- Confirm out-of-scope activation wording describes missing owners as owners to create or designate, not as existing current owners.

Failure:
- An out-of-scope capability, reserved operation, profile-gated value, or unproved behavior is described as a default active requirement.
- Promotion wording names a non-existing owner as if it were already active.
- Activation wording omits the need to update active scope and paired English/Korean docs when meaning changes.

Fix:
- Reword as out of scope and route to Scope, or promote it through the active owner before using active language.
- Link existing current owners only when they actually exist.
- If activating the capability, update active scope, relevant owners, routes, checks, and paired-language docs in the same documentation batch.

## CHK-SCOPE-002: implementation wording

Owner:
- [Implementation Guide](../../build/implementation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Confirm documentation edits do not imply the server, runtime, conformance runner, generated projections, or runtime behavior exists because of documentation alone.
- Confirm implementation authority is not claimed outside the Implementation Guide owner.

Failure:
- Active docs describe documentation reference material as accepted runtime behavior or implementation authority without the Implementation Guide owner.

Fix:
- Reword as planning or reference documentation.
- Route implementation sequence questions to the Implementation Guide.

## CHK-REFERENCE-001: API, storage, and security summaries point to owners

Owner:
- [Reference Index](../../reference/README.md)
- [API Methods](../../reference/api/methods.md)
- [Storage Effects](../../reference/storage-effects.md)
- [Security](../../reference/security.md)

Check:
- Inspect non-owner API, storage, and security mentions for short purpose summaries and owner links.
- Confirm API methods, schema fields, storage effects, DDL-like details, access boundaries, and security guarantees are not redefined outside their owners.
- Confirm security wording stays within the documented guarantee level.

Failure:
- A non-owner page reproduces request/response structure, response branches, error behavior, schema fields, storage lifecycle rules, versioning behavior, or security claims as if it owns them.
- Text implies OS-level permissions, arbitrary-tool sandboxing, tamper-proof local files, default pre-tool blocking, security isolation, or detective capability without owner support.

Fix:
- Replace duplicated contract text with a short reader consequence and a link to the precise owner.
- Reword security claims to the documented guarantee level or explicit non-claim.

## CHK-READ-001: user-facing readability

Owner:
- [User Guide](../../use/user-guide.md)
- [Agent Guide](../../use/agent-guide.md)
- [Judgment Examples](../../use/judgment-examples.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Inspect user-facing docs for raw schema names, field lists, enum-like values, storage language, and internal API branch language.
- Keep exact identifiers only when the reader needs them for the task.

Failure:
- User-facing prose reads like a schema or storage contract instead of explaining what the reader can decide, expect, or do.

Fix:
- Move contract detail to the reference owner.
- Replace overloaded prose with plain reader outcomes and a link when needed.

## CHK-STYLE-001: paragraph and table scannability

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Checks Index](../checks.md)

Check:
- Inspect changed Reference and Maintain paragraphs for multiple conditions, exceptions, non-claims, owner links, or effects hidden in one dense paragraph.
- Confirm tables are used only for short mappings, comparisons, or owner routing.
- Confirm long conditions, exceptions, non-claims, effects, owner links, and list-like examples sit outside table cells.

Failure:
- A paragraph requires the reader to infer condition/result/exception boundaries.
- A table cell contains multiple sentences, multiple conditions, hidden exceptions, non-claims, effects, owner links, or list-like sequences.
- A source line is hard to review.

Fix:
- Split dense prose into named blocks or bullets.
- Keep table rows as short mappings and put details below as bullets or named blocks.
- Move contract detail to the canonical owner.

## CHK-STYLE-002: English heading case

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Checks Index](../checks.md)

Check:
- Inspect changed English section headings for sentence case.
- Preserve exact identifiers, product labels, acronyms, and code literals when their casing is meaningful.
- After heading changes, check inbound links and paired-language route links when relevant.

Failure:
- English headings drift into title case, inconsistent capitalization, or identifier casing changes that reduce searchability.

Fix:
- Rewrite headings in sentence case while preserving exact identifiers and acronyms.
- Update anchors or inbound links only when the heading change requires it.

## CHK-REPORT-001: final review report format

Owner:
- [Checks Index](../checks.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- The final report lists review scope, changed files, checks run, findings by file, owner links for each finding, skipped checks with reasons, and suggested fixes.
- The report states that results are documentation-maintenance findings only when that distinction could be unclear.

Failure:
- Findings omit file paths, owners, or fixes.
- The report claims acceptance, runtime conformance, implementation routing, QA completion, close readiness, or residual-risk acceptance.

Fix:
- Rewrite the report in the compact shape from the [Checks Index](../checks.md).
