# Checks

Use these read-only documentation checks after documentation edits and before major review handoff. This page defines review procedures only. It does not define API, storage, schema, security, runtime, projection, evidence, QA, acceptance, close-readiness, or residual-risk contracts.

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
- The report treats a check result as documentation acceptance, implementation readiness, runtime conformance, final acceptance, QA, close readiness, residual-risk acceptance, or permission to start server coding.

Fix:
- Reword the output as a documentation review result.
- Route any implementation-readiness question to [MVP Plan](../build/mvp-plan.md).

### CHK-OUT-002: no generated runtime outputs

Owner:
- [Authoring Guide](authoring-guide.md)
- [Runtime Boundaries](../reference/runtime-boundaries.md)

Check:
- Confirm the check produced review notes only.
- Confirm it did not create or simulate Harness runtime records, generated projections, operational artifacts, executable fixtures, conformance reports, QA records, acceptance records, close records, residual-risk records, or product writes.

Failure:
- A documentation check leaves behind generated operational files, runtime-like state, fixture output, migration notes, archive copies, or temporary planning files.

Fix:
- Remove the generated or temporary material.
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

## 4. Bilingual semantic parity checks

### CHK-PARITY-001: English and Korean meaning parity

Owner:
- [English Translation Guide](translation-guide.md)
- [Korean Translation Guide](../../ko/maintain/translation-guide.md)

Check:
- Compare paired files by meaning unit when the edit changes meaning.
- Confirm the paired files keep the same reader purpose, normative strength, owner routing, active/later boundary, user-judgment boundary, and security guarantee level.
- Allow natural Korean structure instead of line-by-line translation.

Failure:
- One language misses a meaning-changing edit.
- One language strengthens, weakens, or reroutes a rule compared with the paired file.

Fix:
- Update both languages in the same documentation-only batch.
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

## 7. Active/later boundary checks

### CHK-SCOPE-001: active/later leakage

Owner:
- [Active MVP Scope](../reference/active-mvp-scope.md)
- [Later Index](../later/index.md)
- [MVP Plan](../build/mvp-plan.md)

Check:
- Inspect changed active docs, examples, route text, and summaries for later candidates presented as current MVP behavior.
- Confirm profile-gated or later-only values are labeled at the point of use.

Failure:
- A later candidate, future operation, profile-gated value, or unproved behavior is described as a default active requirement.

Fix:
- Reword it as deferred and route to the Later Index, or promote it through the active owner before using active language.

### CHK-SCOPE-002: implementation-readiness wording

Owner:
- [MVP Plan](../build/mvp-plan.md)
- [Authoring Guide](authoring-guide.md)

Check:
- Confirm documentation edits do not imply the server, runtime, conformance runner, generated projections, or implementation-complete behavior already exists.
- Confirm permission to start server coding is not claimed unless the MVP Plan handoff owner explicitly says so.

Failure:
- Active docs describe documentation source material as accepted runtime behavior or implementation-ready handoff without the MVP Plan owner.

Fix:
- Reword as planning documentation.
- Route readiness decisions to the MVP Plan.

## 8. API contract reference checks

### CHK-API-001: API summaries point to owners

Owner:
- [MVP API](../reference/api/mvp-api.md)
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
- Confirm storage-related documentation edits remain source material for future implementation.
- Confirm the edit does not create operational records, runtime home content, generated projections, or executable fixture outputs.

Failure:
- A documentation batch creates, simulates, or describes its own output as Harness runtime state.

Fix:
- Delete generated runtime-like output.
- Reword the documentation as planning source material and link to the storage or runtime-boundary owner.

## 10. Security-claim checks

### CHK-SEC-001: security non-claim clarity

Owner:
- [Security](../reference/security.md)
- [Runtime Boundaries](../reference/runtime-boundaries.md)

Check:
- Inspect wording around cooperative, detective, preventive, guard, freeze, careful mode, sandbox, permission, blocking, tamper-proof, isolation, local access, and capability claims.
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

## 13. Final review report format

### CHK-REPORT-001: final review report format

Owner:
- [Checks](checks.md)
- [Authoring Guide](authoring-guide.md)

Check:
- The final report lists review scope, changed files, checks run, findings by file, owner links for each finding, skipped checks with reasons, and suggested fixes.
- The report states that results are documentation-maintenance findings only.

Failure:
- Findings omit file paths, owners, or fixes.
- The report claims acceptance, runtime conformance, implementation readiness, QA completion, close readiness, or residual-risk acceptance.

Fix:
- Rewrite the report in this format:
  - Scope:
  - Changed files:
  - Checks run:
  - Findings:
  - Skipped checks:
  - Residual documentation risks:
