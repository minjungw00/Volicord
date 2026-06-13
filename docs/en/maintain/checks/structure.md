# Structure checks

Use these checks for documentation architecture, owner boundaries, route-page structure, display wording boundaries, storage record references, reference-claim placement, and final report shape. These are documentation quality checks only; they do not certify product runtime behavior.

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
- The review starts from an unspecified scope, a full reference dump, a stale route, or both languages for the same `doc_id` when parity review is not needed.
- A strict contract is checked without naming its owner.

Fix:
- Reduce inputs to changed files, needed paired files, and owner sections needed for the next check.
- Replace stale routes with compact maintained routes before continuing.

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
- A documentation check leaves behind generated operational files, runtime-like state, fixture output, one-off conversion notes, archive copies, or one-off planning files.

Fix:
- Remove generated or transient material.
- Keep the result in the final review report only.

## CHK-STRUCT-004: no transient maintenance leftovers

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Checks Index](../checks.md)

Check:
- Inspect changed files and newly added files for one-off planning files, working-note remnants, review leftovers, archive copies, transition notes, one-off conversion notes, scratch files, generated runtime records, and unresolved task markers such as `FIXME` or other all-caps placeholders.
- Confirm documentation-maintenance findings live in the final report or the appropriate maintained documentation page, not in ad hoc files.

Failure:
- The final tree contains a one-off plan, working-note remnant, review note, archive copy, scratch file, generated runtime-like record, one-off conversion note, or unresolved task marker from the documentation batch.
- A maintained page contains a task marker that names deferred work instead of a durable maintenance rule.

Fix:
- Remove the transient file or task marker.
- Convert durable guidance into the appropriate owner document only when it has stable reader value.

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
- Confirm method-level owner maps appear only in [API Methods](../../reference/api/methods.md).
- Confirm those pages route readers instead of defining contracts.
- Confirm a route or index page does not become the broad contract document for several focused owners.

Failure:
- A route page becomes useful as a standalone technical contract.
- A route or index page grows into a broad contract document because it accumulates API, schema, storage, security, error, display, or close-readiness details.
- A route list tries to enumerate every contract detail, owner subcase, status value, schema branch, storage effect, or security guarantee.
- A non-method-router page repeats the supported public API method owner table.

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
- Confirm supported behavior, support availability, guarantee level, and reader consequence come from the semantic owner.

Failure:
- A value name is treated as supported behavior or a baseline guarantee merely because it appears in a schema, example, storage note, route page, or out-of-scope list.
- Reserved or profile-gated values appear without their reserved/profile-gated status at the point of use.

Fix:
- Reword the statement as reserved, profile-gated, deferred, or vocabulary-only until the semantic owner says the behavior is supported.
- Link to the semantic owner for meaning and support availability.
- If no semantic owner exists, expose the owner gap instead of inferring behavior from the value name.

## CHK-OWNER-004: owner granularity

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
- Inspect changed Reference owners for unrelated concerns accumulated in one document.
- Confirm a single owner is not defining multiple distinct contract families, such as API behavior, schema fields, storage effects, security guarantees, API error precedence, templates, and examples.
- Confirm neighboring concerns are routed with short links when another owner already defines them.

Failure:
- A Reference page becomes the practical owner for several unrelated concerns because the workflow mentions them together.
- A page's `owner_for` scope in `doc-index.yaml` is broad enough that readers cannot tell which document owns a specific contract question.
- A new section adds a second contract family instead of routing to the existing owner.

Fix:
- Split the concern into a focused paired owner when no owner exists, or route to the existing owner when one exists.
- Update the paired owner, Reference Index, `doc-index.yaml`, and inbound links in the same documentation batch when a real split is made.

## CHK-OWNER-005: duplicate owner maps

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
- Search route pages, `README` files, Maintain pages, Reference introductions, and agent guidance for repeated owner maps.
- Confirm the full map for a concern appears only in the canonical route or owner document.
- Confirm other pages use a short purpose summary plus a link to the map.

Failure:
- The same owner table, method map, API error map, schema map, storage map, security map, or route matrix appears in multiple documents.
- Two maps describe the same routing surface with different owners, terms, order, or omissions.

Fix:
- Keep the canonical map and shrink duplicates to a short route link.
- If the canonical map is wrong, update it first, then update links that point to it.

## CHK-OWNER-006: display wording owner boundaries

Owner:
- [Template Bodies](../../reference/template-bodies.md)
- [Authoring Guide](../authoring-guide.md)
- The focused API, blocker, storage, scope, or terminology owner selected from the Reference Index

Check:
- Inspect display wording owners, template pages, and rendered-label guidance for contract claims.
- Confirm display wording owners define rendered body guidance, labels, and display phrasing only.
- Confirm API semantics, close-readiness blocker semantics, storage records, and unsupported package wording route to the focused owner instead of being defined by display text.

Failure:
- A display wording owner defines `ErrorCode` meaning, response branch behavior, close-readiness blocker meaning, storage record authority, or storage layout.
- A rendered label, message, or package phrase is treated as the canonical API value, blocker value, storage record, or supported concept.
- Unsupported package wording remains in display guidance solely as a negative example and makes the unsupported concept look official.

Fix:
- Keep the display wording only when it is needed for rendered text.
- Route API meaning, blocker meaning, storage records, support availability, and terminology to their focused owners.
- Remove unsupported package wording unless a terminology owner intentionally preserves a searchable banned expression.

## CHK-SCOPE-001: baseline/out-of-scope leakage

Owner:
- [Scope](../../reference/scope.md)
- [Implementation Guide](../../build/implementation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Inspect changed maintained docs, examples, route text, and summaries for out-of-scope capabilities presented as baseline scope behavior.
- Confirm profile-gated or reserved values are labeled at the point of use.
- Confirm out-of-scope promotion wording describes missing owners as owners to create or designate, not as existing owners.

Failure:
- An out-of-scope capability, reserved operation, profile-gated value, or unproved behavior is described as a default baseline requirement.
- Promotion wording names a nonexistent owner as if it already existed.
- Promotion wording omits the need to update baseline scope and paired English/Korean docs when meaning changes.

Fix:
- Reword as out of scope and route to Scope, or promote it through the semantic owner before using baseline-support language.
- Link existing owners only when they actually exist.
- If promoting the capability, update baseline scope, relevant owners, routes, checks, and paired-language docs in the same documentation batch.

## CHK-SCOPE-002: implementation wording

Owner:
- [Implementation Guide](../../build/implementation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Confirm documentation edits do not imply the server, runtime, conformance runner, generated projections, or runtime behavior exists because of documentation alone.
- Confirm implementation authority is not claimed outside the Implementation Guide owner.
- Confirm implementation guidance, build pages, and metadata use durable implementation wording rather than build-moment labels, transfer labels, current-work labels, or interim-stage labels.

Failure:
- Maintained docs describe documentation reference material as accepted runtime behavior or implementation authority without the Implementation Guide owner.
- Implementation guidance or metadata depends on a short-lived phase label that will stop being true after the implementation path matures.

Fix:
- Reword as planning or reference documentation.
- Route baseline implementation reading-path questions to the Implementation Guide.
- Replace phase labels with durable baseline, supported-scope, owner-route, or implementation-reading-path language.

## CHK-SCOPE-003: excluded-scope wording

Owner:
- [Scope](../../reference/scope.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Inspect excluded-scope, out-of-scope, reserved, and profile-gated wording for double negatives.
- Confirm the text states support or exclusion directly, using owner-backed conditions.
- Confirm unsupported or excluded concepts are not described as supported by phrases such as "not excluded", "not unsupported", or "not outside support".

Failure:
- A sentence makes readers infer support from a double negative.
- Excluded-scope logic is written so that a route page, example, or value-set mention appears to promote the capability.

Fix:
- Rewrite the sentence as "excluded until..." or "supported only when..." with the applicable owner link.
- Route promotion requirements to Scope and the affected owners.

## CHK-REFERENCE-001: API, storage, and security summaries point to owners

Owner:
- [Reference Index](../../reference/README.md)
- The applicable owner selected from the Reference Index or `doc-index.yaml`

Check:
- Inspect non-owner API, storage, and security mentions for short purpose summaries and owner links.
- Confirm API methods, schema fields, storage effects, DDL-like details, access boundaries, and security guarantees are not redefined outside their owners.
- Confirm security wording stays within the documented guarantee level.

Failure:
- A non-owner page reproduces request/response structure, response branches, error behavior, schema fields, storage lifecycle rules, versioning behavior, or security claims as if it owns them.
- Text implies OS-level permissions, arbitrary-tool sandboxing, tamper-proof local files, default pre-tool blocking, security isolation, or detective capability without owner support.

Fix:
- Replace duplicated contract text with a short reader consequence and a link to the precise owner.
- Reword security claims to the documented guarantee level or explicit boundary.

## CHK-REFERENCE-002: storage record family references

Owner:
- [Storage Records](../../reference/storage-records.md)
- [Storage Effects](../../reference/storage-effects.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Inspect storage mentions in changed docs, examples, route pages, metadata, and display wording owners.
- Confirm storage record references focus on persisted record families defined by the storage owner.
- Confirm unsupported pseudo-families are not preserved as negative examples in a way that turns them into official storage concepts.
- Confirm API shapes, display labels, and documentation-routing terms are not described as storage record families.

Failure:
- A non-storage page invents or names a storage-like family that the storage owner does not define.
- A negative example gives an unsupported pseudo-family enough structure, naming, or repetition that it becomes searchable as an official concept.
- A schema, rendered label, owner route, or metadata key is described as a persisted storage record.

Fix:
- Retarget the statement to the persisted record family named by Storage Records, or remove the unsupported name.
- Route storage effects to Storage Effects and storage layout to Storage Records.
- Use stable categories when explaining non-storage concepts instead of inventing pseudo-family names.

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
- Inspect changed Reference and Maintain paragraphs for multiple conditions, exceptions, boundary caveats, owner links, or effects hidden in one dense paragraph.
- Confirm tables are used only for short mappings, comparisons, or owner routing.
- Confirm long conditions, exceptions, boundary caveats, effects, owner links, and list-like examples sit outside table cells.

Failure:
- A paragraph requires the reader to infer condition/result/exception boundaries.
- A table cell contains multiple sentences, multiple conditions, hidden exceptions, boundary caveats, effects, owner links, or list-like sequences.
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
