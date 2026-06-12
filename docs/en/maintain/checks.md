# Checks

Use these read-only documentation checks after documentation edits. This page defines maintenance procedures only. It does not define API, storage, schema, security, runtime, projection, evidence, QA, acceptance, close-readiness, or residual-risk contracts.

## 1. Check inputs

### CHK-IN-001: review scope inputs

Owner:
- [Authoring Guide](authoring-guide.md)
- [Reference Index](../reference/README.md)
- [doc-index.yaml](../../doc-index.yaml)

Check:
- Identify changed files, paired-language files, touched headings, and touched anchors.
- For each contract-like statement, identify one canonical owner from the Reference Index.
- For terminology questions, include [Terminology Map](../../terminology-map.yaml) as an input.

Failure:
- The review starts from an unspecified scope, a full Reference dump, a stale route, or both languages for the same `doc_id` when parity review is not needed.
- A strict contract is checked without naming its owner.

Fix:
- Reduce inputs to changed files, needed paired files, and the owner sections needed for the next check.
- Replace stale routes with compact active routes before continuing.

## 2. Check outputs

### CHK-OUT-001: maintenance result labels

Owner:
- [Authoring Guide](authoring-guide.md)
- [Checks](checks.md)

Check:
- Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance labels.
- Keep findings tied to file paths, owner documents, and suggested documentation fixes.

Failure:
- The report treats a check result as documentation acceptance, implementation routing, runtime conformance, final acceptance, QA, close readiness, residual-risk acceptance, or implementation authority.

Fix:
- Reword the output as a documentation maintenance result.
- Route any implementation question to [Implementation Guide](../build/implementation-guide.md).

### CHK-OUT-002: no generated runtime outputs

Owner:
- [Authoring Guide](authoring-guide.md)
- [Runtime Boundaries](../reference/runtime-boundaries.md)

Check:
- Confirm the check produced review notes only.
- Confirm it did not create or simulate Harness runtime records, generated projections, operational artifacts, executable fixtures, conformance reports, QA records, acceptance records, close records, residual-risk records, or product writes.

Failure:
- A documentation check leaves behind generated operational files, runtime-like state, fixture output, migration notes, archive copies, or one-off planning files.

Fix:
- Remove the generated or transient material.
- Keep the result in the final review report only.

## 3. Link and anchor checks

### CHK-LINK-001: broken links and stale routes

Owner:
- [Authoring Guide](authoring-guide.md)
- [doc-index.yaml](../../doc-index.yaml)
- [Reference Index](../reference/README.md)

Check:
- Validate changed relative links, file paths, anchors, route tables, and paired-language links.
- Confirm active navigation uses the compact active routes from the authoring owner.
- Confirm contract links point to the canonical owner, not to a convenient duplicate.

Failure:
- A link targets a missing file, missing anchor, stale route family, wrong-language owner, or deleted compatibility path.
- A route page links directly to deep contract detail where the Reference Index should choose the owner.

Fix:
- Update the link to the active route or canonical owner.
- Add or preserve anchors only where they are needed for stable links.

### CHK-LINK-002: hidden anchors

Owner:
- [Translation Guide](translation-guide.md)
- [Authoring Guide](authoring-guide.md)

Check:
- For Korean headings, keep visible headings natural Korean.
- When an English anchor must remain stable, use a hidden HTML anchor before the natural Korean heading.
- After heading changes, check inbound links in the changed language and its paired route.

Failure:
- Korean visible headings are made unnatural to preserve an English anchor.
- A heading change removes a stable anchor and breaks inbound links.

Fix:
- Restore the stable anchor with a hidden HTML anchor.
- Keep the visible heading natural and update links that should follow the new heading.

### CHK-LINK-003: route documents expose owner gaps

Owner:
- [Authoring Guide](authoring-guide.md)
- [Reference Index](../reference/README.md)
- [doc-index.yaml](../../doc-index.yaml)

Check:
- Inspect changed route documents, README files, indexes, and `doc-index.yaml` entries for questions whose exact canonical owner is missing or unclear.
- Confirm route text points to a current owner when one exists.
- Confirm a missing owner is exposed as a documentation gap instead of being hidden behind broad route prose, Maintain guidance, or copied contract detail.

Failure:
- A route document answers a contract question without a current canonical owner.
- A route sends readers to a broad index or Maintain page when the question needs an owner that does not yet exist.
- `doc-index.yaml` names a default owner that cannot answer the routed question.

Fix:
- Retarget the route to the exact owner selected from the Reference Index.
- If no current owner exists, state the owner gap and route to the closest real owner, [Scope Reference](../reference/scope.md), or [Implementation Guide](../build/implementation-guide.md) as appropriate.
- Create or designate a real owner only in the same paired documentation batch that defines the owner boundary.

## 4. Bilingual semantic parity checks

### CHK-PARITY-001: English and Korean meaning parity

Owner:
- [English Translation Guide](translation-guide.md)
- [Korean Translation Guide](../../ko/maintain/translation-guide.md)

Check:
- Compare paired files by meaning unit when the edit changes meaning.
- Confirm the paired files keep the same reader purpose, normative strength, owner routing, active/out-of-scope boundary, user-judgment boundary, and security guarantee level.
- Allow natural Korean structure instead of line-by-line translation.

Failure:
- One language misses a meaning-changing edit.
- One language strengthens, weakens, or reroutes a rule compared with the paired file.

Fix:
- Update both languages in the same documentation batch.
- Rewrite Korean naturally while preserving the same meaning.

### CHK-PARITY-002: exact identifier preservation

Owner:
- [Terminology Map](../../terminology-map.yaml)
- [Translation Guide](translation-guide.md)

Check:
- Confirm exact identifiers remain unchanged in both languages.
- Confirm file paths, anchors, `doc_id` values, API methods, schema fields, enum values, table names, validator IDs, error codes, and product labels appear in backticks when prose clarity or searchability needs them.

Failure:
- An exact identifier is translated, localized, reformatted, or used as a reader-facing display label that changes its meaning.

Fix:
- Restore the exact identifier.
- Add a plain-language explanation next to the identifier when needed.

### CHK-PARITY-003: Korean reference structure preservation

Owner:
- [Korean Translation Guide](../../ko/maintain/translation-guide.md)
- [Korean Authoring Guide](../../ko/maintain/authoring-guide.md)
- [Authoring Guide](authoring-guide.md)

Check:
- For Korean reference edits, compare conditions, results, exceptions, non-claims, owner links, and close-readiness consequences as meaning units.
- Confirm Korean prose may differ in line count and sentence order but still keeps important caveats and owner boundaries visible.
- Inspect dense Korean paragraphs for merged rules that hide a condition, exception, or non-claim.

Failure:
- Korean text preserves the broad topic but collapses separate condition/result/exception or non-claim structure.
- A Korean paragraph makes an owner boundary, active/out-of-scope boundary, security non-claim, or close-readiness consequence harder to detect than in the paired meaning unit.

Fix:
- Split the Korean prose into natural paragraphs or bullets that preserve the meaning units.
- Keep exact identifiers unchanged and preserve semantic parity without forcing line-by-line translation.

### CHK-EXAMPLE-TIMELESS-SCENARIO: timeless API and Reference scenarios

Owner:
- [Authoring Guide](authoring-guide.md)
- [Korean Translation Guide](../../ko/maintain/translation-guide.md)
- [API Methods](../reference/api/methods.md)
- The affected Reference owner selected from [Reference Index](../reference/README.md)

Check:
- Confirm API and Reference examples use stable product or user scenarios.
- For current API method examples, confirm they use the shared account data export confirmation sample task unless the documentation batch intentionally replaces that sample across the API examples, paired Korean examples, checks, and routes.
- Confirm examples do not use documentation maintenance, refactoring, migration, or section restructuring as their scenario.
- Confirm documentation paths are used as example payload only when the document is specifically about documentation maintenance.
- Confirm example wording does not narrate a documentation maintenance process instead of a product or user scenario.
- Confirm paired English and Korean examples preserve equivalent scenario details.
- Confirm shared example scenarios in Korean documents use natural Korean wording, avoid compressed noun chains, and do not preserve English noun order when a natural Korean phrase is clearer.

Failure:
- Example payload includes internal documentation paths when the document is not about documentation maintenance.
- Example task goal describes rewriting the Harness documentation set.
- Example baseline, artifact, run, or judgment names refer to documentation maintenance.
- Example wording describes a documentation maintenance process instead of a product or user scenario.
- The shared API sample task changes in one language, route, or check but not the paired owner set.
- One language keeps a different scenario after paired updates.
- A Korean shared example scenario uses compressed noun chains or preserved English noun order that makes the scenario harder to read.

Fix:
- Replace the example with a durable product or user scenario.
- Use the shared account data export confirmation sample task, or replace the sample consistently across the API examples, paired Korean examples, checks, and routes.
- Keep file paths only when the document is explicitly about documentation maintenance.
- Remove process-only wording and make the scenario durable.
- Update paired English and Korean examples by meaning unit.
- Rewrite the Korean shared scenario wording as natural Korean while preserving equivalent scenario details.

### CHK-EXAMPLE-INTERNAL-CONSISTENCY: API example internal consistency

Owner:
- [`maintain/authoring-guide.md`](authoring-guide.md)
- affected API method owner document

Check:
- Example refs are introduced or explicitly described as existing.
- A response snapshot does not include refs from a later `state_version`.
- Sensitive approval reasons match the request's `sensitive_categories` or stated precondition.
- Artifact refs do not appear without staging, promotion, or existing-artifact context.
- Expiration timestamps use placeholders or clearly future example dates.
- Cross-method examples that share a scenario do not contradict each other.
- Representative responses do not silently drop meaningful request fields unless labeled as abbreviated.

Failure:
- status examples include future-version supporting refs
- approval reasons do not match `sensitive_categories`
- artifact refs appear without lifecycle context
- staged handles have stale fixed expiration timestamps
- close-readiness evidence refers to missing run or judgment refs
- response examples drop `options` or `affected_refs` from a user-judgment request without saying the response is abbreviated

Fix:
- Align refs, versions, sensitive categories, artifact lifecycle, timestamps, and shared scenario data.

### CHK-EXAMPLE-FIELD-NAME-CONSISTENCY: example field-name consistency

Owner:
- [`maintain/authoring-guide.md`](authoring-guide.md)
- affected method or schema owner document

Check:
- Example field names match the owner method or schema document.
- A storage/effect example that reuses method payload data does not use a different field name unless it is explicitly described as a storage-owned summary field.
- Field names shared across examples are consistent.

Failure:
- A method example uses `intended_paths`, while a related storage example uses `affected_paths` for the same concept without explanation.
- A field name appears in an example but is not owned by the relevant method, schema, or storage summary section.
- Two related examples use different field names for the same concept without an owner-boundary note.

Fix:
- Use the owner method/schema field name, or clearly mark the field as storage-owned summary data.
- Add an owner link when needed.

## 5. Terminology checks

### CHK-TERM-001: close readiness terminology

Owner:
- [Terminology Map](../../terminology-map.yaml)
- [Glossary](../reference/glossary.md)
- [Translation Guide](translation-guide.md)

Check:
- Korean reference prose uses "닫기 준비 상태".
- Korean user-facing prose may use "닫기 가능 여부".
- Korean prose does not use "close 가능성 평가" or "닫기 가능성 평가" except in forbidden-expression lists or quoted legacy examples.

Failure:
- A forbidden mixed expression appears outside a forbidden-expression list or quoted legacy example.
- A user-facing phrase and a reference-facing phrase are swapped in a way that changes reader meaning.

Fix:
- Replace the phrase according to the Terminology Map.
- If the map is incomplete, update the terminology owner and paired guidance before spreading a new term.

### CHK-TERM-002: terminology drift

Owner:
- [Terminology Map](../../terminology-map.yaml)
- [Glossary](../reference/glossary.md)
- [Translation Guide](translation-guide.md)

Check:
- Search changed prose for new product terms, mixed-language Korean, and alternate spellings of existing concepts.
- Confirm each new durable term is owned by the glossary, the terminology map, or the relevant reference owner.

Failure:
- The same concept appears under multiple prose terms without an owner-backed distinction.
- A Korean sentence keeps an English noun phrase that is not an identifier, intentional product label, or natural technical borrowing.

Fix:
- Align wording with the terminology owner.
- Add or revise owner terminology only when the new distinction is intentional.

### CHK-TERM-COMPLETE-INTENT: `complete` intent ambiguity

Owner:
- [Terminology Map](../../terminology-map.yaml)
- [Glossary](../reference/glossary.md)
- [API Value Sets](../reference/api/schema-value-sets.md)
- [API Methods](../reference/api/methods.md)

Check:
- Preserve `complete` only when it is an identifier or enum value.
- Do not leave `complete` in Korean prose when the English means full or entire.
- Confirm Korean prose does not use phrases like "complete 닫기 준비 상태 순서".
- In English, prefer "Full ..." when "Complete ..." could be confused with the enum value.

Failure:
- A Korean sentence preserves `complete` when the English means full or entire.
- A heading makes `complete` look like `intent=complete` when it is not.

Fix:
- Use "전체", "전체 평가", or another natural Korean expression.
- In English, prefer "Full ..." when needed to avoid enum ambiguity.

## 6. Owner-boundary checks

### CHK-OWNER-001: canonical owner violations

Owner:
- [Reference Index](../reference/README.md)
- [Authoring Guide](authoring-guide.md)

Check:
- For API, schema, storage, security, access-boundary, projection, template, close-readiness, judgment, error, and runtime-boundary statements, confirm the strict definition lives in one canonical owner.
- Confirm non-owner documents use only a short reader consequence plus an owner link.

Failure:
- README, Start, Use, Build, Later, Maintain, example, or non-owner Reference text creates a second normative definition.
- A non-owner repeats field lists, response branches, storage details, guarantee levels, blocker details, access-class rules, or template bodies instead of linking to the owner.

Fix:
- Keep the owner definition.
- Shrink the duplicate to a short consequence and link to the owner.

### CHK-OWNER-002: README over-detailing

Owner:
- [Reference Index](../reference/README.md)
- [Authoring Guide](authoring-guide.md)

Check:
- Inspect README and route pages for contract tables, long field explanations, status-value lists, security guarantee details, storage-effect details, and API branch summaries.
- Confirm those pages route readers instead of defining the contract.

Failure:
- A README or route page becomes useful as a standalone technical contract.

Fix:
- Move normative detail to the canonical owner if it is missing there.
- Replace README detail with navigation text and an owner link.

### CHK-README-ROUTE-LENGTH: README route length

Owner:
- [Reference Index](../reference/README.md)
- [Authoring Guide](authoring-guide.md)
- [doc-index.yaml](../../doc-index.yaml)

Check:
- Inspect changed README files, route sections, Start pages, Use pages, Scope pages, and Maintain route lists for scannability.
- Confirm route lists stay short enough to choose the next owner without carrying contract tables, long owner summaries, or technical branch detail.
- Confirm long or specialized routing needs are delegated to `doc-index.yaml`, the Reference Index, or the canonical owner.

Failure:
- A route list becomes hard to scan because it tries to enumerate every contract detail, owner subcase, status value, schema branch, storage effect, or security guarantee.
- A reader could use the README route section as a substitute for the canonical owner.

Fix:
- Shorten the route to reader purpose, expected result, and owner links.
- Move or keep technical detail in the canonical owner and use `doc-index.yaml` for retrieval metadata.

### CHK-OWNER-003: value-set names versus semantic ownership

Owner:
- [Authoring Guide](authoring-guide.md)
- [Reference Index](../reference/README.md)
- [Scope](../reference/scope.md)
- [API Value Sets](../reference/api/schema-value-sets.md)

Check:
- For status values, enum-like values, profile-gated values, reserved values, access classes, guarantee labels, blocker categories, and display values, identify both the value-set owner and the semantic owner.
- Confirm the value-set owner is used for exact names and validation placement only.
- Confirm active behavior, current availability, guarantee level, and reader consequence come from the semantic owner.

Failure:
- A value name is treated as active behavior or an active guarantee merely because it appears in a schema, example, storage note, route page, or out-of-scope list.
- A value-set owner is used to define security, storage, close-readiness, user-judgment, template, or runtime semantics that belong elsewhere.
- Reserved or profile-gated values appear without their reserved/profile-gated status at the point of use.

Fix:
- Reword the statement as reserved, profile-gated, deferred, or vocabulary-only until the semantic owner says the behavior is active.
- Link to the semantic owner for meaning and current availability.
- If no semantic owner exists, expose the owner gap instead of inferring behavior from the value name.

## 7. Active and out-of-scope boundary checks

### CHK-SCOPE-001: active/out-of-scope leakage

Owner:
- [Scope](../reference/scope.md)
- [Scope Reference](../reference/scope.md)
- [Implementation Guide](../build/implementation-guide.md)

Check:
- Inspect changed active docs, examples, route text, and summaries for out-of-scope capabilities presented as baseline scope behavior.
- Confirm profile-gated or reserved values are labeled at the point of use.

Failure:
- An out-of-scope capability, reserved operation, profile-gated value, or unproved behavior is described as a default active requirement.

Fix:
- Reword it as out of scope and route to the Scope Reference, or promote it through the active owner before using active language.

### CHK-SCOPE-LIST-STRUCTURE: baseline scope list structure

Owner:
- [Scope](../reference/scope.md)

Check:
- Confirm included and excluded baseline scope items are represented as scannable lists, tables, or equivalent structured blocks.
- Confirm the structure makes it easy to tell what is included and what is excluded.
- Confirm Korean may use natural phrasing, but it does not collapse many scope items into one long sentence.

Failure:
- Included or excluded baseline scope appears as a long comma-separated sentence.
- A reader cannot quickly tell which items are included or excluded.
- One language version omits or compresses important scope items compared with the other.

Fix:
- Convert the prose to a bullet list, table, or equivalent structured block.
- Compare English and Korean by meaning unit and keep the scope contract in the active-scope owner.

### CHK-SCOPE-002: implementation wording

Owner:
- [Implementation Guide](../build/implementation-guide.md)
- [Authoring Guide](authoring-guide.md)

Check:
- Confirm documentation edits do not imply the server, runtime, conformance runner, generated projections, or runtime behavior exists because of documentation alone.
- Confirm implementation authority is not claimed outside the Implementation Guide owner.

Failure:
- Active docs describe documentation reference material as accepted runtime behavior or implementation authority without the Implementation Guide owner.

Fix:
- Reword as planning documentation.
- Route readiness decisions to the Implementation Guide.

### CHK-SCOPE-003: out-of-scope activation owner wording

Owner:
- [Scope Reference](../reference/scope.md)
- [Scope](../reference/scope.md)
- [Authoring Guide](authoring-guide.md)

Check:
- Inspect out-of-scope activation requirements for owner names and owner paths.
- Confirm existing current owners are linked when the activation requirement depends on a current owner.
- Confirm non-existing owners are described as owners to create or designate during activation, not as current active owner documents.

Failure:
- Promotion wording names a non-existing owner as if it were already an active owner document.
- An out-of-scope capability sounds active because its activation checklist uses current-owner language without the active scope owner.
- Activation wording omits the need to update active scope and paired English/Korean docs when meaning changes.

Fix:
- Reword the checklist as owner creation, designation, or owner update during activation.
- Link existing current owners only when they actually exist.
- If activating the capability, update active scope, the relevant owners, routes, checks, and paired-language docs in the same documentation batch.

### CHK-SCOPE-OWNER-PLACEHOLDER: owner placeholders

Owner:
- [Scope Reference](../reference/scope.md)
- [Scope](../reference/scope.md)
- [Authoring Guide](authoring-guide.md)

Check:
- Inspect out-of-scope requirements for owner names, owner-type labels, and placeholders.
- Confirm wording such as "Assurance owner update" is clear that the owner is created, designated, or updated during activation when no current owner exists.
- Confirm placeholder wording does not send readers to a missing current owner.

Failure:
- An owner placeholder is named as if it were an existing current owner.
- A phrase like "Assurance owner update" appears without clarifying owner creation, designation, or update.

Fix:
- Use standard activation owner wording.
- Link only existing current owners; otherwise state that promotion requires creating or designating the owner before active wording can be used.

## 8. API contract reference checks

### CHK-API-001: API summaries point to owners

Owner:
- [API Methods](../reference/api/methods.md)
- [Core Schema](../reference/api/schema-core.md)
- [State Schema](../reference/api/schema-state.md)
- [Artifact Schema](../reference/api/schema-artifacts.md)
- [Judgment Schema](../reference/api/schema-judgment.md)
- [Value Sets](../reference/api/schema-value-sets.md)
- [Errors](../reference/api/errors.md)

Check:
- Inspect non-owner API mentions for short purpose summaries and owner links.
- Confirm API methods, schema names, fields, values, and error codes are not redefined outside the appropriate API owner.

Failure:
- A non-owner page reproduces request/response structure, response branches, error behavior, schema fields, enum-like values, or method semantics as if it owns them.

Fix:
- Replace duplicated contract text with a short reader consequence and a link to the precise API owner.

### CHK-API-002: API owner selection

Owner:
- [Reference Index](../reference/README.md)

Check:
- Confirm API links choose the narrow owner for the question: method behavior, common envelope, state schema, artifact schema, judgment schema, value set, or public error.

Failure:
- A link points to a broad index when the reader needs a precise contract owner.
- A link points to the wrong API owner for the concept being checked.

Fix:
- Retarget the link to the exact owner selected from the Reference Index.

## 9. Storage-effect reference checks

### CHK-STORAGE-001: storage effect summaries point to owners

Owner:
- [Storage Records](../reference/storage-records.md)
- [Storage Effects](../reference/storage-effects.md)
- [Artifact Storage](../reference/storage-artifacts.md)
- [Storage Versioning](../reference/storage-versioning.md)

Check:
- Inspect non-owner storage mentions for short reader consequences and owner links.
- Confirm storage records, storage effects, artifact lifecycle, idempotency, locking, migration, and versioning details are not redefined outside their owners.

Failure:
- A non-owner page repeats DDL-like structure, state-effect rules, storage lifecycle rules, or versioning behavior as standalone contract text.

Fix:
- Keep the storage contract in the owner.
- Replace the duplicate with a short summary and owner link.

### CHK-STORAGE-002: documentation edits do not create runtime state

Owner:
- [Runtime Boundaries](../reference/runtime-boundaries.md)
- [Authoring Guide](authoring-guide.md)

Check:
- Confirm storage-related documentation edits remain reference material for implementation.
- Confirm the edit does not create operational records, runtime home content, generated projections, or executable fixture outputs.

Failure:
- A documentation batch creates, simulates, or describes its own output as Harness runtime state.

Fix:
- Delete generated runtime-like output.
- Reword the documentation as planning reference material and link to the storage or runtime-boundary owner.

### CHK-KO-STRUCT-STORAGE: Korean storage structure

Owner:
- [Korean Storage Records](../../ko/reference/storage-records.md), [Korean Storage Effects](../../ko/reference/storage-effects.md), [Korean Artifact Storage](../../ko/reference/storage-artifacts.md), [Korean Storage Versioning](../../ko/reference/storage-versioning.md)
- [Storage Records](../reference/storage-records.md), [Storage Effects](../reference/storage-effects.md), [Artifact Storage](../reference/storage-artifacts.md), [Storage Versioning](../reference/storage-versioning.md)
- [Korean Translation Guide](../../ko/maintain/translation-guide.md)

Check:
- For Korean storage reference edits, compare the paired English storage source docs by meaning unit.
- Confirm conditions, effects, exceptions, non-claims, and owner links remain visibly separate in Korean.
- Inspect dense Korean paragraphs for merged storage rules that hide a condition, exception, or non-claim.

Failure:
- Important storage conditions, effects, exceptions, or non-claims are collapsed into dense Korean paragraphs.
- Korean prose preserves the broad topic but makes the storage boundary harder to review than the paired English meaning unit.

Fix:
- Rewrite the Korean storage prose using natural paragraphs, lists, or tables that keep the meaning units visible.
- Keep exact identifiers unchanged and link to the storage owners instead of duplicating contract detail in Maintain guidance.

## 10. Security-claim checks

### CHK-SEC-001: security non-claim clarity

Owner:
- [Security](../reference/security.md)
- [Runtime Boundaries](../reference/runtime-boundaries.md)

Check:
- Inspect wording around cooperative, detective, prevention, guard, freeze, careful mode, sandbox, permission, blocking, tamper-proof, isolation, local access, and capability claims.
- Confirm every security claim routes to the security owner and stays within the documented guarantee level.
- Confirm non-claims are explicit where a reader could otherwise infer stronger security behavior.

Failure:
- Text implies OS-level permissions, arbitrary-tool sandboxing, tamper-proof local files, default pre-tool blocking, security isolation, or detective capability without an owner-backed mechanism.

Fix:
- Reword to the documented guarantee level or non-claim.
- Link to the security owner for the exact boundary.

### CHK-SEC-002: surface and access wording

Owner:
- [Security](../reference/security.md)
- [Agent Connector Reference](../reference/agent-integration.md)
- [Terminology Map](../../terminology-map.yaml)

Check:
- Confirm `surface_id`, surface, connector, capability, and access-class wording does not imply authority, approval, or binding proof unless the owner says so.
- Confirm access-related terms preserve the distinction between documentation guidance and runtime enforcement.

Failure:
- A surface or access term is used as proof of permission, user judgment, Write Authorization, security isolation, or runtime enforcement without owner support.

Fix:
- Reword the statement as identification, routing, or documented guidance as appropriate.
- Link to the security or connector owner.

### CHK-GUARANTEE-STRONGER-ISOLATION: stronger isolation wording

Owner:
- [Security](../reference/security.md)
- [Scope](../reference/scope.md)

Check:
- Confirm stronger isolation, sandboxing, permission-enforcement, or tamper-proof wording routes to Security for guarantee semantics.
- Confirm stronger isolation is not described as supported merely because a route page, example, or out-of-scope material mentions it.

Failure:
- Any prose says or implies current active isolation, default isolation, enforced isolation, sandboxing, or tamper-proof behavior without owner support.

Fix:
- Reword to the documented guarantee level or explicit non-claim.
- Link to Security for semantics and Scope for current availability.

## 11. User-facing readability checks

### CHK-READ-001: user-facing docs avoid internal schema overload

Owner:
- [User Guide](../use/user-guide.md)
- [Agent Guide](../use/agent-guide.md)
- [Judgment Examples](../use/judgment-examples.md)
- [Authoring Guide](authoring-guide.md)

Check:
- Inspect user-facing docs for raw schema names, field lists, enum-like values, storage language, and internal API branch language.
- Keep exact identifiers only when the reader needs them for the task.

Failure:
- User-facing prose reads like a schema or storage contract instead of explaining what the reader can decide, expect, or do.

Fix:
- Move contract detail to the reference owner.
- Replace overloaded prose with plain reader outcomes and a link when needed.

### CHK-READ-002: Korean user-facing readability

Owner:
- [Korean Authoring Guide](../../ko/maintain/authoring-guide.md)
- [Korean Translation Guide](../../ko/maintain/translation-guide.md)

Check:
- Inspect Korean user-facing prose for natural Korean technical writing, Korean concept-first phrasing, and consistent terms.
- Confirm exact identifiers remain searchable but are not exposed as ordinary display labels.

Failure:
- Korean prose mirrors English sentence order, keeps avoidable English noun phrases, or hides the reader action behind internal identifiers.

Fix:
- Rewrite in natural Korean while preserving identifiers and semantic parity.

## 12. LLM retrieval checks

### CHK-LLM-001: duplicate contract text creates retrieval noise

Owner:
- [doc-index.yaml](../../doc-index.yaml)
- [Reference Index](../reference/README.md)
- [Authoring Guide](authoring-guide.md)

Check:
- Inspect agent guidance, README pages, maintain docs, and summaries for duplicate contract text that could be retrieved instead of the owner.
- Confirm retrieval guidance points agents to one owner section for the next action.

Failure:
- The same API, storage, security, schema, blocker, access-class, projection, or runtime-boundary contract appears in multiple non-owner places.
- Always-on context examples include full reference docs, full schemas, full DDL, historical logs, generated outputs, or both languages for the same `doc_id`.

Fix:
- Shrink duplicates to route text and owner links.
- Keep agent context to the current task summary, needed owner section, and needed language.

### CHK-LLM-002: one language per `doc_id`

Owner:
- [Translation Guide](translation-guide.md)
- [doc-index.yaml](../../doc-index.yaml)

Check:
- Confirm normal agent retrieval loads only one language for a given `doc_id`.
- Confirm paired English/Korean docs are loaded together only for translation, semantic parity review, or bilingual editing.

Failure:
- Agent instructions encourage loading both language versions by default.
- A prompt template injects paired docs for the same `doc_id` when comparison is not needed.

Fix:
- Reword retrieval guidance to one language per `doc_id`.
- Add the paired document only for parity-specific checks.

## 13. Editorial style checks

### CHK-REFERENCE-PARAGRAPH-SCANNABILITY: paragraph scannability

Owner:
- [Authoring Guide](authoring-guide.md)
- [Checks](checks.md)

Check:
- Inspect changed Reference and Maintain paragraphs for multiple conditions, exceptions, non-claims, owner links, or effects hidden in one dense paragraph.
- If a paragraph contains more than one rule type, confirm it is split into named blocks or bullets.
- Confirm allowed and disallowed behavior are visually separated.

Failure:
- A paragraph requires the reader to infer condition/result/exception boundaries.
- A paragraph combines allowed and disallowed behavior without visual separation.
- A paragraph contains several "must not", "does not", or "only when" clauses that would be clearer as a list.

Fix:
- Split dense prose into Conditions, Allowed effects, Not allowed, Exceptions, and Owner links as appropriate.
- For check descriptions, use Owner, Check, Failure, and Fix blocks with bullets.

### CHK-TABLE-SOURCE-MAINTAINABILITY: Markdown table source maintainability

Owner:
- [Authoring Guide](authoring-guide.md)
- [Checks](checks.md)

Check:
- Confirm tables are used only for short mappings, comparisons, or owner routing.
- Confirm the table rule covers all documentation, including Maintain docs.
- Confirm dense cells are rewritten as summary rows plus detail blocks.
- Confirm long conditions, exceptions, non-claims, effects, owner links, and list-like examples sit outside cells.

Failure:
- A cell contains multiple sentences or multiple conditions.
- A cell hides an exception, non-claim, effect, or owner link.
- A cell carries a list-like sequence.
- A source line is hard to review.

Fix:
- Keep the table row as the short mapping.
- Put detail below the table as bullets or named blocks.
- Rewrite long check rows as named blocks instead of preserving table cells.
- Move contract detail to the canonical owner.

### CHK-EN-HEADING-CASE: English heading case

Owner:
- [Authoring Guide](authoring-guide.md)
- [Checks](checks.md)

Check:
- Inspect changed English section headings for sentence case.
- Preserve exact identifiers, product labels, acronyms, and code literals when their casing is meaningful.
- After heading changes, check inbound links and paired-language route links when relevant.

Failure:
- English headings drift into title case, inconsistent capitalization, or identifier casing changes that reduce searchability.

Fix:
- Rewrite headings in sentence case while preserving exact identifiers and acronyms.
- Update anchors or inbound links only when the heading change requires it.

## 14. Final review report format

### CHK-REPORT-001: final review report format

Owner:
- [Checks](checks.md)
- [Authoring Guide](authoring-guide.md)

Check:
- The final report lists review scope, changed files, checks run, findings by file, owner links for each finding, skipped checks with reasons, and suggested fixes.
- The report states that results are documentation-maintenance findings only.

Failure:
- Findings omit file paths, owners, or fixes.
- The report claims acceptance, runtime conformance, implementation routing, QA completion, close readiness, or residual-risk acceptance.

Fix:
- Rewrite the report in this format:
  - Scope:
  - Changed files:
  - Checks run:
  - Findings:
  - Skipped checks:
  - Residual documentation risks:
