# Structure checks

Use these checks for documentation architecture, owner boundaries, route-page structure, label-content consistency, display wording boundaries, storage record references, reference-claim placement, and final report shape. These are documentation quality checks only; they do not certify product runtime behavior, API conformance, QA completion, close readiness, or product guarantees.

Structure review boundary: these checks judge where documentation claims belong and how reviewable they are. They do not judge whether Harness implements the claim.

## CHK-STRUCT-001: review scope inputs

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
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

Check sources:
- [Checks Index](../checks.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance check outcomes.
- Keep findings tied to file paths, owner documents, and suggested documentation fixes.

Failure:
- The report treats a check result as documentation acceptance, implementation routing, runtime conformance, API conformance, final acceptance, QA, close readiness, residual-risk acceptance, product guarantee, or implementation authority.

Fix:
- Reword the output as a documentation maintenance result.
- Route implementation questions to [Implementation Guide](../../build/implementation-guide.md).

## CHK-STRUCT-003: no generated runtime outputs

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Runtime Boundaries](../../reference/runtime-boundaries.md)

Evidence to inspect:
- Confirm documentation checks produced review notes only.
- Confirm they did not create or simulate Harness runtime state, generated projections, operational artifacts, executable fixtures, conformance reports, QA results, acceptance decisions, close-readiness state, residual-risk decisions, or product writes.

Failure:
- A documentation check leaves behind generated operational files, runtime-like state, fixture output, one-off conversion notes, archive copies, or one-off planning files.

Fix:
- Remove generated or transient material.
- Keep the result in the final review report only.

## CHK-STRUCT-004: no transient maintenance leftovers

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Checks Index](../checks.md)

Evidence to inspect:
- Inspect changed files and newly added files for one-off planning files, working-note remnants, review leftovers, archive copies, transition notes, one-off conversion notes, ad hoc files, generated runtime records, and unresolved fix markers or other all-caps placeholders.
- Search changed maintained prose and Maintain check pages for task-context residue. Include English patterns such as `T[O]D[O]`, `draf(t|ted)`, `current\s+PR`, `this\s+(work|task|change|cleanup)`, `rewrite\s+plan`, `later\s+cleanup`, `temp(orary)?`, `re[- ]?work`, and `migration\s+note`.
- Search changed Korean maintained prose and Maintain check pages for task-context residue. Include Korean patterns such as `할\s*일`, `초안`, `현재\s*PR`, `이번\s*PR`, `이\s*(작업|변경|수정)`, `이번\s+(작업|변경|수정)`, `작업\s*에서\s+(바[뀐]|수정[한])`, `다시\s*쓰기\s*계획`, `재작성\s*계획`, `나중에\s+정리`, `나중\s+정리`, `추후\s+정리`, `임시`, and `재작업`.
- In Maintain check docs, keep those strings only when the surrounding wording clearly identifies them as search patterns, quoted legacy examples, or forbidden-pattern examples.
- Confirm general maintenance conditions use stable wording such as `changed`, `edited`, `when a document changes`, `변경된`, `편집된`, `문서 변경 시`, and `점검 대상`.
- Confirm documentation-maintenance findings live in the final report or the appropriate maintained documentation page, not in ad hoc files.

Failure:
- The final tree contains a one-off plan, working-note remnant, review note, archive copy, ad hoc file, generated runtime-like record, one-off conversion note, or unresolved task marker from the documentation batch.
- A maintained page contains one of the listed work-specific residue markers, or an equivalent Korean marker, instead of a durable maintenance rule.
- A maintained page uses wording tied to a specific maintenance episode instead of stable documentation-quality wording.
- Task-context wording appears outside an explicit search-pattern list, quoted prohibited-pattern list, or other explicit Maintain check example.

Fix:
- Remove the transient file or task marker.
- Replace task-specific wording with stable maintenance conditions such as `changed`, `edited`, and `when a document changes`.
- Replace Korean task-context wording with stable maintenance expressions such as `변경된`, `편집된`, `문서 변경 시`, and `점검 대상`.
- If a check document needs the string for review, keep it inside an explicit search-pattern or forbidden-pattern context.
- Convert durable guidance into the appropriate owner document only when it has stable reader value.

## CHK-STRUCT-005: maintenance label taxonomy

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Checks Index](../checks.md)

Applies to:
- `docs/en/maintain/checks.md`
- `docs/ko/maintain/checks.md`
- `docs/en/maintain/checks/*.md`
- `docs/ko/maintain/checks/*.md`
- `docs/en/maintain/authoring-guide.md`
- `docs/ko/maintain/authoring-guide.md`

Evidence to inspect:
- Confirm check-card basis documents use `Check sources`.
- Confirm checked files or document families use `Applies to`.
- Confirm inspected documents, fields, examples, labels, links, anchors, or route metadata use `Evidence to inspect`.
- Confirm success criteria use `Pass condition`.
- Confirm route destinations use `Route` or `Reference route`.
- Confirm maintenance companion pages use `Check sources` when they define the basis, or `Maintained with` when they are paired pages to update or review.
- Confirm check cards do not use `Owner`, `Primary owner`, or other owner labels for check basis documents, checked file lists, evidence lists, route destinations, pass criteria, or companion maintenance pages.

Pass condition:
- Maintain check cards use role-specific labels for basis, scope, evidence, and success criteria, and reserve ownership labels for actual term or contract ownership.

Failure:
- A maintain check card uses an owner label for check sources.
- A check card lists checked files or document families under `Check sources` when they are not a basis for the check.
- Evidence or success criteria are buried under an ambiguous label instead of `Evidence to inspect` or `Pass condition`.
- A route destination, adjacent reference, or companion maintenance page is labeled as ownership.

Fix:
- Rename check basis lists to `Check sources`.
- Move checked files or document families into `Applies to`.
- Move inspected material into `Evidence to inspect`.
- Move success criteria into `Pass condition`.
- Rename navigation targets to `Route` or `Reference route`.
- Rename companion maintenance pages to `Maintained with`, unless they define the check basis.

## CHK-STRUCT-006: semantic label-content consistency

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Translation Guide](../translation-guide.md)
- [Reference Index](../../reference/README.md)

Applies to:
- Changed Reference and Maintain sections that use named semantic labels, labeled table rows, or labeled list groups.

Evidence to inspect:
- Inspect labels such as `Not allowed`, `Required behavior`, `Result`, `Does not imply`, and `Owner boundary`.
- Confirm `Not allowed` and Korean `허용되지 않는 것` contain prohibited actions or states, not requirements, outcomes, examples, or route links.
- For prohibitions with `unless`, `only when`, or `except when`, confirm the condition, prohibited behavior, and owner boundary remain separately reviewable when one sentence would make the label unclear.
- Confirm conditions that define when something may be treated as authority are not hidden inside a `Not allowed` bullet or row.
- In Korean text under `허용되지 않는 것`, inspect sentences that use `되어야 합니다` or `해야 합니다`; confirm they are not actually required behavior that belongs under a requirement or condition label.
- Confirm `Required behavior` contains required behavior, not prohibitions, optional guidance, outcomes, or non-claims.
- Confirm `Result` contains outcomes or reader-visible consequences, not preconditions, requirements, or owner routes.
- Confirm `Does not imply` contains non-implications or non-claims, not effects, requirements, or prohibitions.
- Confirm `Owner boundary` contains ownership boundaries or routing limits, not the contract body itself.
- Classify each labeled unit before checking parity: required versus prohibited, effect versus non-effect, and route versus contract.

Pass condition:
- Each label matches the semantic content underneath it, conditional prohibitions remain clear, and each content unit is classified before it is routed, translated, or compared for parity.

Failure:
- A prohibited action is placed under `Required behavior`, or a requirement is placed under `Not allowed` or `허용되지 않는 것`.
- A `Not allowed` unit mixes a prohibition with an `unless`, `only when`, or `except when` condition so the reader cannot tell which behavior is prohibited and which condition permits or routes the exception.
- A condition that tells readers when something may be treated as authority is buried inside a prohibition bullet or row.
- Korean `허용되지 않는 것` text uses `되어야 합니다` or `해야 합니다` for content that is actually required behavior.
- An effect or outcome is placed under `Does not imply`, or a non-effect is presented as a `Result`.
- A route or owner-boundary note is written as if it were the contract, or contract detail is hidden under `Owner boundary`.
- English and Korean use matching labels, but the shared label is wrong for the content.

Fix:
- Rename the label, move the content to the correct meaning unit, or split the unit so requirements, prohibitions, effects, non-effects, routes, and contracts are separately reviewable.
- When a conditional prohibition is unclear, split it into `Conditions`, `Not allowed`, and `Owner boundary` units.
- If the content defines a contract, route it to the canonical owner and leave non-owner pages with a short consequence plus owner link.

## CHK-STRUCT-007: implementation-architecture alignment

Check sources:
- The paired [Implementation architecture](../../build/architecture.md) page
- Root `Cargo.toml`
- Relevant `crates/*/Cargo.toml` and `tests/*/Cargo.toml` files
- Major `lib.rs` and `main.rs` entry points
- Documented module and test paths

Evidence to inspect:
- Confirm every current Cargo workspace member appears in the architecture map, including crate packages and test packages.
- Confirm internal dependency arrows match the Cargo manifests, and normal implementation dependencies are not confused with test-only or dev dependencies.
- Confirm documented major module paths, entry points, binary-test paths, integration-test paths, and conformance-test paths exist.
- Confirm MCP direct Store access is described only for its actual bounded startup and session-validation responsibilities.
- Confirm administrative CLI boundaries and public Core method boundaries match the implementation dependency shape.
- Confirm common Core preflight, method-specific planning, staging, and mutation commit remain separate execution stages.
- Confirm artifact staging is not described as a normal Core mutation commit.
- Confirm test packages and binary tests are represented with accurate responsibilities and are not treated as product-contract owners.
- Confirm the architecture page routes exact product behavior to canonical owners instead of defining it itself.
- Confirm the page remains a durable guide-level map, not a function-level or exhaustive source inventory.

Failure:
- The architecture page omits a workspace member or test package, or names one that is no longer present.
- It invents, omits, or reverses internal dependency edges, or merges normal implementation dependencies with test-only dependencies.
- It names stale module, entry-point, or test paths.
- It misrepresents MCP, CLI, Core, Store, staging, or mutation-commit boundaries.
- It treats tests, fixtures, or scenarios as product-contract owners.
- It uses fragile source line numbers, function-level coverage, or exhaustive file inventory as architecture guidance.
- It duplicates product-contract detail that belongs in a Reference owner.

Fix:
- Correct the guide-level workspace map, dependency map, source path references, execution-stage descriptions, or owner-routing links.
- Move exact product behavior, storage effects, API behavior, security guarantees, and Core authority semantics to the applicable Reference owner instead of adding them to Maintain guidance or the architecture guide.
- Remove line-number, exhaustive-inventory, implementation-history, and test-as-contract wording.

## CHK-OWNER-001: canonical owner violations

Documentation boundary: owner findings are documentation placement findings, not product behavior findings.

Check sources:
- [Reference Index](../../reference/README.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- For API, schema, storage, security, access-boundary, projection, template, close-readiness, judgment, error, and runtime-boundary statements, confirm the strict definition lives in one canonical owner.
- Confirm non-owner documents explain only within their guide, example, or
  maintenance scope and link to the owner when exact behavior matters.

Failure:
- `README`, Start, Use, Build, Maintain, example, or non-owner Reference text creates a second normative definition.
- A non-owner presents field lists, response branches, storage details,
  guarantee levels, blocker details, access-class rules, or template bodies as
  contract detail instead of reader-facing explanation with an owner link.

Fix:
- Keep the owner definition.
- Shrink contract duplicates to reader-facing explanation and an owner link.

## CHK-OWNER-002: route-page over-detailing

Check sources:
- [Reference Index](../../reference/README.md)
- [Authoring Guide](../authoring-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
- Classify pages from `doc-index.yaml` metadata such as `role`, `owner_for`, and `normative_level` before applying route-page checks.
- Treat `reference.scope` (`docs/en/reference/scope.md` and `docs/ko/reference/scope.md`) as a contract owner for baseline scope, the supported boundary, the out-of-scope boundary, the profile-gated boundary, and the reserved behavior boundary.
- Inspect documents classified by metadata as route-only or index pages,
  including any `README`, Start, Use, Build, Maintain, or reference-index page
  with that actual role, for contract tables, long field explanations,
  status-value lists, security guarantee details, storage-effect details, and
  API branch summaries.
- Confirm method-level owner maps appear only in [API Methods](../../reference/api/methods.md).
- Confirm route-only and index pages route readers instead of defining contracts.
- Confirm a route or index page does not become the broad contract document for several focused owners.
- Confirm an index document is not listed or treated as the primary owner for detailed terms, schema/API/storage/security concepts, or owner contracts when a focused owner exists.
- Confirm route and index pages do not accumulate detailed negative rules, exception lists, non-claim tables, or "must not" clauses that effectively define a contract outside its owner.

Failure:
- A route page becomes useful as a standalone technical contract.
- A route or index page grows into a broad contract document because it accumulates API, schema, storage, security, error, display, or close-readiness details.
- An index, `README`, or family route is used as the primary owner for a detailed term or contract even though a focused owner can answer the question.
- A route list tries to enumerate every contract detail, owner subcase, status value, schema branch, storage effect, or security guarantee.
- A route or index page becomes a contract by collecting enough prohibitions, exceptions, "Not allowed" rows, or "Does not imply" rows to define the concept negatively.
- A non-method-router page repeats the supported public API method owner table.
- A contract owner such as `reference.scope` is flagged as over-detailed merely because it contains contract detail that belongs to its `owner_for` scope.

Fix:
- Move normative detail to the canonical owner if it is missing there.
- Replace route-page detail with reader purpose, expected result, and owner links.
- Retarget the primary owner to the focused owner; keep indexes as first-hop routes or related references only.
- Keep only the short boundary a route reader needs, then link to the owner for durable prohibitions, exceptions, and non-claims.
- For `reference.scope`, evaluate whether scope detail stays within its owner boundary and routes API, storage, security, and other focused details to their owners.

## CHK-OWNER-003: value-set names versus semantic ownership

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [Scope](../../reference/scope.md)
- [API Value Sets](../../reference/api/schema-value-sets.md)

Evidence to inspect:
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

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
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

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
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

Check sources:
- [Template Bodies](../../reference/template-bodies.md)
- [Authoring Guide](../authoring-guide.md)
- The focused API, blocker, storage, scope, or terminology owner selected from the Reference Index

Evidence to inspect:
- Inspect display wording owners, template pages, and rendered-label guidance for contract claims.
- Confirm display wording owners define rendered body guidance, labels, and display phrasing only.
- Confirm API semantics, close-readiness blocker semantics, storage records, and out-of-scope rendered-body names route to the focused owner instead of being defined by display text.

Failure:
- A display wording owner defines `ErrorCode` meaning, response branch behavior, close-readiness blocker meaning, storage record authority, or storage layout.
- A rendered label, message, or package phrase is treated as the canonical API value, blocker value, storage record, or supported concept.
- An out-of-scope rendered-body name remains in display guidance solely as a negative example and makes the name look official.

Fix:
- Keep the display wording only when it is needed for rendered text.
- Route API meaning, blocker meaning, storage records, support availability, and terminology to their focused owners.
- Remove out-of-scope rendered-body names unless a terminology owner intentionally preserves a searchable banned expression.

## CHK-OWNER-007: blocker-routing owner boundary

Check sources:
- [API blocker routing](../../reference/api/blocker-routing.md)
- [API Methods](../../reference/api/methods.md)
- [API State Schemas](../../reference/api/schema-state.md)
- [API Value Sets](../../reference/api/schema-value-sets.md)
- [Core Model](../../reference/core-model.md)
- [Template Bodies](../../reference/template-bodies.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Inspect blocker-routing docs, route summaries, Maintain guidance, and API error routing text that mention blocker routing.
- Confirm blocker-routing material stays within its owner boundary and does not become the owner for method behavior, schema shape, value sets, Core authority, or display wording.
- Confirm method-specific `harness.close_task` behavior routes to the method owner.
- Confirm `CloseReadinessBlocker` schema shape routes to API State Schemas, blocker category values route to API Value Sets, Core close-readiness meaning routes to Core Model, and rendered wording routes to Template Bodies.

Failure:
- A blocker-routing document defines request validation, evaluation order, result branches, `CloseReadinessBlocker` fields, category values, Core close-readiness meaning, or rendered body wording.
- A route, index, or Maintain page treats blocker routing as the broad owner for all blocker-related method, schema, value-set, Core, or display questions.

Fix:
- Replace the borrowed detail with a short owner link.
- Move missing detail to the focused owner, or expose the owner gap instead of expanding blocker-routing scope.

## CHK-OWNER-008: split owner size and scope

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
- When a Reference owner is newly split or narrowed, compare its introduction, headings, `owner_for`, `not_owner_for`, and inbound routes.
- Confirm the split owner has a narrow reader purpose and does not collect every neighboring concern left outside the previous owner.
- Confirm adjacent API behavior, schema, storage, security, error, display wording, template, example, and route concerns point to their focused owners.

Failure:
- A split document becomes a broad catch-all owner for a workflow, feature family, or leftover concern group.
- The document's scope is so wide that readers cannot tell whether another focused owner should answer a specific contract question.
- New sections accumulate unrelated contract families instead of routing them.

Fix:
- Narrow the owner purpose and move or route unrelated concerns to the focused owners.
- Split further only when a distinct stable owner is needed.
- Update the paired owner, Reference Index, `doc-index.yaml`, and inbound links in the same documentation batch when the split changes routing.

## CHK-SCOPE-001: baseline/out-of-scope leakage

Documentation boundary: scope checks inspect wording in documentation. They do not promote or remove product support.

Check sources:
- [Scope](../../reference/scope.md)
- [Implementation Guide](../../build/implementation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
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

Check sources:
- [Implementation Guide](../../build/implementation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Confirm documentation edits do not imply the server, runtime, conformance runner, generated projections, or runtime behavior exists because of documentation alone.
- Confirm implementation authority is not claimed outside the Implementation Guide owner.
- Confirm implementation guidance, build pages, and metadata use durable implementation wording rather than build-moment labels, transfer labels, batch-specific labels, or interim-stage labels.

Failure:
- Maintained docs describe documentation reference material as accepted runtime behavior or implementation authority without the Implementation Guide owner.
- Implementation guidance or metadata depends on a short-lived phase label that will stop being true after the implementation path matures.

Fix:
- Reword as planning or reference documentation.
- Route baseline implementation reading-path questions to the Implementation Guide.
- Replace phase labels with durable baseline, supported-scope, owner-route, or implementation-reading-path language.

## CHK-SCOPE-003: excluded-scope wording

Check sources:
- [Scope](../../reference/scope.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
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

Check sources:
- [Reference Index](../../reference/README.md)
- The applicable owner selected from the Reference Index or `doc-index.yaml`

Evidence to inspect:
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

Check sources:
- [Storage Records](../../reference/storage-records.md)
- [Storage Effects](../../reference/storage-effects.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Inspect storage mentions in changed docs, examples, route pages, metadata, and display wording owners.
- Confirm storage record references focus on persisted record families defined by the storage owner.
- Confirm storage-like names outside the persisted record families are not preserved as negative examples in a way that turns them into official storage concepts.
- Confirm API shapes, display labels, and documentation-routing terms are not described as storage record families.

Failure:
- A non-storage page invents or names a storage-like family that the storage owner does not define.
- A negative example gives an unsupported storage-like name enough structure, naming, or repetition that it becomes searchable as an official concept.
- A schema, rendered label, owner route, or metadata key is described as a persisted storage record.

Fix:
- Retarget the statement to the persisted record family named by Storage Records, or remove the unsupported name.
- Route storage effects to Storage Effects and storage layout to Storage Records.
- Use stable categories when explaining non-storage concepts instead of inventing storage-like family names.

## CHK-READ-001: user-facing readability

Check sources:
- [User Guide](../../use/user-guide.md)
- [Agent Guide](../../use/agent-guide.md)
- [Judgment Examples](../../use/judgment-examples.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Inspect user-facing docs for raw schema names, field lists, enum-like values, storage language, and internal API branch language.
- Keep exact identifiers only when the reader needs them for the task.

Failure:
- User-facing prose reads like a schema or storage contract instead of explaining what the reader can decide, expect, or do.

Fix:
- Move contract detail to the reference owner.
- Replace overloaded prose with plain reader outcomes and a link when needed.

## CHK-STYLE-001: paragraph and table scannability

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Checks Index](../checks.md)

Evidence to inspect:
- Inspect changed Reference and Maintain paragraphs for multiple conditions, exceptions, boundary caveats, owner links, or effects hidden in one dense paragraph.
- Confirm important Reference sections have a defined semantic skeleton before prose is written or reshaped.
- Confirm the skeleton keeps conditions, results, non-claims, owner boundaries, and related references visible instead of hiding them in dense prose.
- Confirm tables are used only for short mappings, comparisons, or owner routing.
- Confirm long conditions, exceptions, boundary caveats, effects, owner links, and list-like examples sit outside table cells.

Failure:
- A paragraph requires the reader to infer condition/result/exception boundaries.
- A Reference section lacks a clear skeleton, causing conditions, results, non-claims, owner boundaries, or related references to drift.
- A table cell contains multiple sentences, multiple conditions, hidden exceptions, boundary caveats, effects, owner links, or list-like sequences.
- A source line is hard to review.

Fix:
- Define a small skeleton, such as `Purpose` / `Conditions` / `Result` / `Non-claim` / `Owner boundary` / `Related references`, or `Meaning` / `Contract` / `Boundary` / `Related references`.
- Split dense prose into named blocks or bullets.
- Keep table rows as short mappings and put details below as bullets or named blocks.
- Move contract detail to the canonical owner.

## CHK-STYLE-002: English heading case

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Checks Index](../checks.md)

Evidence to inspect:
- Inspect changed English section headings for sentence case.
- Preserve exact identifiers, product labels, acronyms, and code literals when their casing is meaningful.
- After heading changes, check inbound links and paired-language route links when relevant.

Failure:
- English headings drift into title case, inconsistent capitalization, or identifier casing changes that reduce searchability.

Fix:
- Rewrite headings in sentence case while preserving exact identifiers and acronyms.
- Update anchors or inbound links only when the heading change requires it.

## CHK-REPORT-001: final review report format

Check sources:
- [Checks Index](../checks.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- The final report lists review scope, changed files, checks run, findings by file, owner links for each finding, skipped checks with reasons, and suggested fixes.
- The report states that results are documentation-maintenance findings only when that distinction could be unclear.

Failure:
- Findings omit file paths, owners, or fixes.
- The report claims acceptance, runtime conformance, API conformance, implementation routing, QA completion, close readiness, product guarantee, or residual-risk acceptance.

Fix:
- Rewrite the report in the compact shape from the [Checks Index](../checks.md).
