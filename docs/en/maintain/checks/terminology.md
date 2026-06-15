# Terminology checks

Use these documentation quality checks when an edit changes:

- product terms
- Korean prose terms
- mixed-language expressions
- identifier explanations
- documentation-routing terms
- glossary owner labels
- close-readiness wording
- access/security wording

These checks do not describe Harness product behavior. The terminology map owns complete structured terminology metadata and bilingual wording controls. Product contracts remain in their reference owners.

When this page lists discouraged or forbidden strings, treat them as search patterns for reviewers. They are not wording to keep in ordinary documentation prose.

Exception: the surrounding text may explicitly identify them as search patterns or quoted legacy examples.

Terminology review boundary: terminology failures are wording, routing, or metadata failures in documentation. They are not product, API, storage, security, or close-readiness failures.

## CHK-TERM-001: close-readiness terminology

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Translation Guide](../translation-guide.md)

Evidence to inspect:
- Korean reference prose uses "닫기 준비 상태".
- Korean user-facing prose may use "닫기 가능 여부".
- Korean prose does not use "close 가능성 평가" or "닫기 가능성 평가" except in forbidden-expression lists or quoted legacy examples.

Failure:
- A forbidden mixed expression appears outside a forbidden-expression list or quoted legacy example.
- A user-facing phrase and a reference-facing phrase are swapped in a way that changes reader meaning.

Fix:
- Replace the phrase according to the Terminology Map.
- If the map is incomplete, update the terminology owner and paired guidance before spreading a new term.

## CHK-TERM-002: terminology drift

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Translation Guide](../translation-guide.md)

Evidence to inspect:
- Search changed prose for new product terms, mixed-language Korean, and alternate spellings of existing concepts.
- Confirm each new durable term has terminology-map metadata or a relevant reference owner before it appears in glossary or repeated prose.
- Confirm Korean sentences translate ordinary English noun phrases unless the English is an identifier, intentional product label, or natural technical borrowing.

Failure:
- The same concept appears under multiple prose terms without an owner-backed distinction.
- A Korean sentence keeps an English noun phrase that is not an identifier, intentional product label, or natural technical borrowing.

Fix:
- Align wording with `docs/terminology-map.yaml` or the relevant reference owner.
- Add or revise owner terminology only when the new distinction is intentional.

## CHK-TERM-003: `complete` intent ambiguity

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [API Value Sets](../../reference/api/schema-value-sets.md)
- [API Methods](../../reference/api/methods.md)

Evidence to inspect:
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

## CHK-TERM-004: surface, access, and security wording

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Security](../../reference/security.md)
- [Agent Integration](../../reference/agent-integration.md)
- [Translation Guide](../translation-guide.md)

Evidence to inspect:
- Confirm `surface_id`, surface, connector, capability, and access-class wording is not presented as authority, approval, or binding proof unless the owner says so.
- Confirm `Write Authorization` is not collapsed into ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, or broad user-owned judgment.
- Confirm access-related terms preserve the distinction between documentation guidance and runtime enforcement.
- Confirm cooperative, detective, prevention, guard, freeze, careful mode, sandbox, permission, blocking, tamper-proof, isolation, and capability wording stays within owner-backed terminology.

Failure:
- A surface or access term is used as proof of permission, user judgment, Write Authorization, security isolation, or runtime enforcement without owner support.
- A sentence treats ordinary approval, sensitive-action approval, final acceptance, or residual-risk acceptance as if it were the product label `Write Authorization`.
- Security wording implies stronger isolation, sandboxing, permission enforcement, or tamper-proof behavior merely because route text, examples, or out-of-scope material mentions it.

Fix:
- Reword the statement as identification, routing, or documented guidance as appropriate.
- Preserve `Write Authorization` as the exact product label when that concept is named, and use separate wording for ordinary approval concepts.
- Link to Security for guarantee semantics, Agent Integration for connector context, and Scope for support availability when needed.

## CHK-TERM-005: terminology-map alignment

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [doc-index.yaml](../../../doc-index.yaml)
- [API Value Sets](../../reference/api/schema-value-sets.md)
- [English Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Applies to:
- Terminology guidance, glossary content, terminology-map metadata, translation guides, and `doc-index.yaml` metadata touched by a terminology edit.

Evidence to inspect:
- Compare changed terminology guidance with `docs/terminology-map.yaml`.
- Confirm `docs/terminology-map.yaml` remains the complete structured terminology metadata source.
- Inspect glossary content by role, regardless of whether it is represented as a compact table, compact entries, or another human-readable view.
- Confirm the glossary remains compact and reader-facing.
- Confirm the glossary is not required to mirror every terminology-map term.
- Confirm checks do not require a specific glossary layout.
- Confirm checks do not require expanded per-term detail; role-level validation is enough.
- Confirm every term included in the glossary has matching terminology-map metadata.
- Confirm Markdown links to the glossary are used only when the linked context refers to a term included in the curated glossary.
- Confirm terms that exist only in `docs/terminology-map.yaml` route to the terminology map or focused owner, not to the glossary.
- Confirm detailed value and metadata contexts route to focused owners or `docs/terminology-map.yaml`.
- Detailed contexts include schema fields, enum values, API values, helper values, storage details, and translation-control terms.
- Add a glossary link for a detailed context only when the exact term is intentionally included as a core glossary term.
- Confirm reserved or profile-gated value contexts route to [API Value Sets](../../reference/api/schema-value-sets.md) and `docs/terminology-map.yaml`.
- Add a glossary link for a reserved or profile-gated value only when the linked term is included in the curated glossary.
- Confirm `primary_owner` targets point to the focused owner document when one exists.
- Confirm `related_references` hold adjacent routes instead of broadening ownership.
- Confirm glossary `Primary owner` values match terminology-map `primary_owner` for included terms.
- Confirm glossary `See also` or `Related references` values do not contradict terminology-map `related_references`.
- Confirm each glossary term has exactly one `Primary owner`; adjacent documents belong under `See also`, `Related references`, or terminology-map `related_references`.
- Confirm terminology-map `primary_owner`, glossary `Primary owner`, and `doc-index.yaml` owner metadata stay synchronized by concept when those records exist.
- Confirm forbidden mixed-language examples in the guides use concrete strings, not vague descriptions.
- Confirm any new forbidden expression appears in the terminology map and both translation guides.

Pass condition:
- The terminology map remains the complete structured terminology metadata source.
- The glossary remains a compact reader-facing subset.
- Every glossary-included term has matching terminology-map metadata, the same primary owner, and non-contradictory related references.
- Detailed value, schema, helper, storage, and translation-control contexts route to focused owners or `docs/terminology-map.yaml`.
- No check requires the glossary to mirror the full map or use a specific layout.

Failure:
- The guides and terminology map disagree.
- A glossary-included term is missing from the terminology map or lacks matching terminology-map metadata.
- A check or route requires the glossary to include every terminology-map term.
- A check requires a specific glossary layout.
- A check treats missing expanded per-term detail as a glossary failure.
- A Markdown link points to the glossary for a term that is not included in the curated glossary.
- A terminology-map-only term is routed to the glossary instead of `docs/terminology-map.yaml` or its focused owner.
- A detailed context routes to the glossary even though the exact term is not intentionally included as a core glossary term.
- Detailed contexts include schema fields, enum values, API values, helper values, storage details, and translation-control terms.
- A reserved or profile-gated value context routes to the glossary instead of API Value Sets and `docs/terminology-map.yaml`.
- Exception: the linked term is included in the curated glossary.
- A terminology-map or glossary owner target points to an index when a focused owner already owns the term's meaning, value set, API concern, storage concern, or display wording.
- A glossary-included term lists multiple primary owners or treats related references as primary owners.
- A terminology-map `primary_owner`, glossary `Primary owner`, or `doc-index.yaml` entry names a different primary owner for the same term without an intentional split term or explicit owner gap.
- A glossary `See also` or `Related references` value contradicts terminology-map `related_references` for the same term.
- A Korean guide describes a banned mixed-language pattern without a searchable real string such as "artifact를 저장한다".

Fix:
- Align the map and both guides in the same documentation batch.
- Add the term to the terminology map before including it in the glossary, or remove it from the compact glossary view.
- Retarget owner links to the focused owner, using an index only for navigation concepts or explicit owner gaps.
- Keep one glossary `Primary owner` and move adjacent documents to `See also` or `Related references`.
- Synchronize glossary content, terminology-map entries, and `doc-index.yaml` metadata when the primary owner changes.
- Keep terminology-map-only terms out of the glossary unless readers need compact glossary coverage.
- Retarget glossary links to the focused owner or `docs/terminology-map.yaml` when the linked context is not a curated glossary term.
- Retarget detailed value, schema, helper, storage, and translation-control links to the focused owner or `docs/terminology-map.yaml`.
- Route reserved and profile-gated value contexts to API Value Sets and `docs/terminology-map.yaml`.
- Keep glossary links only for included core glossary terms.
- Replace vague placeholders with concrete examples that can be searched.

Related checks:
- [CHK-TERM-011](#chk-term-011-glossary-entry-focus)
- [CHK-TERM-012](#chk-term-012-owner-routing-label-usage)
- [CHK-TERM-013](#chk-term-013-glossary-link-route-semantics)
- [CHK-LINK-008](links-and-indexes.md#chk-link-008-terminology-and-metadata-owner-targets)

## CHK-TERM-006: `active` versus current, supported, or applicable

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Core Model](../../reference/core-model.md)
- [Agent Integration](../../reference/agent-integration.md)

Evidence to inspect:
- Search changed prose for `active`, `active scope`, `active Change Unit`, `active surface context`, and Korean "활성".
- Confirm exact identifiers, exact status values, and code literals that contain `active` are preserved unchanged.
- Confirm current runtime or session context uses current-context wording in prose.
- Current-context wording includes "current scope", "currently applied scope", "current Change Unit", and "current surface context".
- Do not use "active scope", "active Change Unit", or "active surface context" for that prose context.
- Confirm supported contracts, supported API methods, supported values, maintained documents, and owner routing use terms such as "supported", "applicable", "maintained", or "current", not `active`.

Failure:
- A document uses `active` for an owner route, contract, API method, reference document, or other documentation route when it means applicable, supported, or maintained.
- Prose uses "active scope", "active Change Unit", or "active surface context" for the current context instead of current/currently applied wording.
- Korean prose translates `active` as "활성" for a documentation contract or owner route instead of using the appropriate Korean term.
- Korean prose uses "활성 범위", "활성 Change Unit", "활성 접점", or "활성 접점 맥락" for the current context outside an exact identifier, status value, search pattern, or quoted legacy example.

Fix:
- Replace `active` with the owner-backed term: "applicable owner path", "supported API method", "supported value", "maintained document", "current scope", "currently applied scope", or "current state".
- Keep `active` only when it is an exact identifier, status value, code literal, or intentionally listed search pattern.

## CHK-TERM-007: retired or unsupported concept names

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Scope](../../reference/scope.md)
- [Reference Index](../../reference/README.md)
- [Authoring Guide](../authoring-guide.md)
- [Template Bodies](../../reference/template-bodies.md)

Evidence to inspect:
- Search maintained Reference docs, glossary entries, `docs/terminology-map.yaml`, `doc-index.yaml`, display wording owners, and changed metadata for retired, deleted, or unsupported concept names.
- When an English concept label is removed, search Korean prose for paraphrases, translations, mixed-language variants, table rows, list items, and headings that preserve the same removed concept.
- Confirm unsupported capability names are used only when a semantic owner still needs the exact name, or when a Maintain/terminology owner intentionally lists a searchable forbidden expression.
- Confirm Reference owners describe stable categories, owner gaps, or out-of-scope capability families instead of preserving obsolete names as examples.
- Confirm negative examples do not make removed names look like supported concepts, owner routes, storage record families, or rendered body families.

Failure:
- A glossary entry, terminology-map entry, metadata route, Reference page, or display wording owner keeps a removed or unsupported concept name solely to say that it is not supported.
- The removed English label is gone literally, but a Korean paraphrase or translation still makes the removed concept look current.
- A negative example causes retrieval to treat the old name as a supported contract, supported capability, or owner route.
- A display wording owner or storage-related note keeps an unsupported rendered-body or storage-like family name that becomes searchable as an official concept.

Fix:
- Remove the stale name or replace it with the stable category and the applicable owner link.
- Move searchable banned terminology to the Terminology Map and translation guides when the term needs to remain searchable for maintainers.
- Remove unsupported display or storage-like names unless a terminology owner intentionally preserves them as forbidden terminology.

## CHK-TERM-008: documentation-routing terms stay documentary

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Search changed prose for documentation-routing terms such as `applicable owner path`, owner route, owner target, route metadata, and owner gap.
- Confirm these terms describe documentation navigation, authoring, retrieval, or metadata only.
- Confirm owner-route terms are not the grammatical actor for product behavior, storage persistence, API support, evidence authority, close-readiness state, or user-visible display.
- Confirm they are not described as product behavior, storage persistence, runtime state, evidence authority, close-readiness state, or API support.

Failure:
- A documentation-routing term is used as if it were a persisted product field, runtime status, API value, storage record, support guarantee, or close-readiness result.
- A sentence says an owner route, documentation route, or metadata entry performs, blocks, allows, stores, verifies, accepts, displays, or authorizes product behavior.
- A guide says a product behavior is available because an owner route applies, instead of because Scope and the semantic owner define support.

Fix:
- Reword the term as documentation routing or metadata.
- Name the owner document or owner contract only as the source of definition, then route product behavior, storage persistence, runtime state, and API support to the focused product owner.

## CHK-TERM-009: Korean blocker terminology

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Translation Guide](../translation-guide.md)

Evidence to inspect:
- Korean prose uses "닫기 차단 사유" for close-readiness blocker prose.
- Korean prose uses "차단 사유 범주" for blocker category prose and preserves `CloseReadinessBlocker.category` when naming the exact field.
- Korean prose uses "차단 사유 처리 경로" for blocker routing prose and preserves exact owner routes or identifiers when naming them.
- Confirm variants such as "close blocker", "blocker category", "blocker 처리 경로", and "blocker 라우팅" appear only in forbidden-expression lists or quoted examples.

Failure:
- A Korean page mixes blocker terminology variants for the same concept without a terminology-owner distinction.
- A non-identifier English blocker phrase remains in Korean prose where the terminology map provides the Korean term.
- The exact schema or field identifier is translated instead of preserved.

Fix:
- Replace prose terms with "닫기 차단 사유", "차단 사유 범주", or "차단 사유 처리 경로" according to the concept.
- Preserve exact identifiers such as `CloseReadinessBlocker` and `CloseReadinessBlocker.category` when naming schemas or fields.

## CHK-TERM-010: Korean compressed owner-link and blocker-routing prose

Check sources:
- [Korean Authoring Guide](../../../ko/maintain/authoring-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Terminology Map](../../../terminology-map.yaml)

Evidence to inspect:
- Inspect Korean owner-link, route, and blocker-routing sentences for long compressed noun phrases.
- Confirm Korean sentences separate the concept, the owner route, the boundary or non-claim, and the reader action when those parts all appear.
- Confirm exact identifiers stay searchable, but ordinary English nouns are translated into natural Korean prose.

Failure:
- A Korean owner-link or blocker-routing sentence compresses several concepts into one unreadable noun phrase.
- A sentence chains owner, exception, prohibition, and route target so tightly that the reviewer cannot tell which concept the owner link applies to.
- Korean prose keeps an English noun chain where the terminology map provides a Korean term.

Fix:
- Rewrite the sentence as natural Korean, using two sentences or bullets when that makes the owner route and boundary clearer.
- Keep identifiers unchanged, and use terminology-map Korean terms for ordinary prose concepts.

## CHK-TERM-011: glossary entry focus

Check sources:
- [Glossary](../../reference/glossary.md)
- [Terminology Map](../../../terminology-map.yaml)
- [Translation Guide](../translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Applies to:
- Terms included in the glossary, in any compact human-readable layout, and paired glossary content touched by the edit.

Evidence to inspect:
- Inspect changed glossary content by role: term label, Korean term, compact meaning, focused primary owner, and any short usage context or adjacent reference.
- Confirm every term included in the glossary exists in the terminology map.
- Confirm every included term has matching terminology-map metadata for the term and any glossary roles it uses.
- Confirm each included term's `Primary owner` matches the terminology-map `primary_owner` for the same term.
- Confirm each included term has only one `Primary owner`; use `See also`, `Related references`, or terminology-map `related_references` for adjacent documents.
- Confirm glossary `See also` or `Related references` values do not contradict terminology-map `related_references`.
- Confirm the glossary can be represented as a compact table, compact entries, or another human-readable view; checks must not require a specific layout.
- Confirm checks do not require expanded per-term detail or a full terminology-map mirror.
- Confirm included terms explain the term and route to the primary owner.
- Confirm included terms do not carry long avoid lists, identifier-preservation lists, owner-routing maps, or documentation-quality checklists.
- Confirm the glossary remains a compact reader-facing term guide rather than the complete structured terminology metadata source.
- Confirm terminology-map terms do not need glossary coverage unless the compact glossary view includes them.
- Confirm the glossary does not duplicate the translation guide's prose-style rules.
- Confirm the glossary does not duplicate `doc-index.yaml` retrieval metadata role.
- Confirm the glossary does not duplicate reference owners' API, storage, schema, security, projection, runtime, or method contracts.
- Confirm API behavior, storage effects, security guarantees, method behavior, and detailed response/schema contracts remain in their focused owners.
- Confirm Korean glossary content uses natural Korean technical prose and preserves exact identifiers unchanged.

Pass condition:
- The glossary remains a compact reader-facing view of selected terms.
- Every included term has matching terminology-map metadata.
- Each included term has one primary owner matching the terminology map and non-contradictory related references.
- Detailed contracts, style rules, and complete structured terminology metadata stay in their owners.

Failure:
- A glossary-included term becomes a translation guide, identifier-preservation policy, owner-routing map, or reference contract.
- A glossary-included term is missing from the terminology map or lacks matching terminology-map metadata.
- A glossary-included term lists multiple primary owners or promotes adjacent documents to primary-owner status.
- A glossary-included term and terminology metadata disagree about the term's `Primary owner`.
- A glossary `See also` or `Related references` value contradicts terminology-map `related_references`.
- The glossary becomes a broad owner-routing map, complete structured terminology metadata source, or layout-specific system instead of a compact term guide.
- A check requires the glossary to mirror every terminology-map term.
- A check requires expanded per-term detail before accepting a compact glossary entry.
- A usage note accumulates repeated "do not", "must not", or avoid-list wording that belongs in the terminology map, translation guide, authoring guide, or focused checks.
- A glossary-included term repeats `doc-index.yaml` route metadata or reference contract detail instead of linking to the owner.
- A glossary-included term copies API, storage, security, schema, projection, or method behavior instead of linking to the owner.

Fix:
- Shrink the glossary content for the term to compact reader-facing meaning, primary owner, and optional adjacent references.
- Align the glossary `Primary owner` with the terminology-map `primary_owner` and focused owner target.
- Keep exactly one primary owner per term and move adjacent documents to related-reference fields.
- Move complete structured terminology metadata and systematic identifier controls to the terminology map.
- Move Korean prose style guidance to the translation guide.
- Move review procedures to Maintain checks.
- Route contract detail to the applicable reference owner.
- Keep terminology-map-only terms out of the glossary unless readers need them in the compact glossary view.

Related checks:
- [CHK-TERM-005](#chk-term-005-terminology-map-alignment)
- [CHK-TERM-012](#chk-term-012-owner-routing-label-usage)
- [CHK-LINK-008](links-and-indexes.md#chk-link-008-terminology-and-metadata-owner-targets)

## CHK-TERM-012: owner-routing label usage

Check sources:
- [Glossary](../../reference/glossary.md)
- [Terminology Map](../../../terminology-map.yaml)
- [Authoring Guide](../authoring-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)

Applies to:
- Glossary labels, terminology-map labels, route prose, owner-routing prose, and Maintain checks touched by the edit.

Evidence to inspect:
- Inspect glossary entries, terminology-map entries, route prose, and Maintain checks that use `Primary owner`, `Related references`, `owner contract`, `primary_owner`, or `related_references`.
- Confirm `Primary owner` and `primary_owner` name the canonical owner for the term or concept.
- Confirm `Related references` and `related_references` name adjacent documents only; they must not be presented as alternate owners or owner contracts.
- Confirm Maintain-check basis documents use `Check sources`, checked file families use `Applies to`, and maintenance companion documents use `Check sources` or `Maintained with` according to their role.
- Confirm documentation navigation uses `Route` or `Reference route`, not owner labels, unless the text is naming the canonical owner.
- Confirm owner contract terminology means the contract defined by the relevant owner document, not a document path, route label, related reference, or index.
- Confirm index documents are not labeled as primary owners for detailed terms when focused owners exist.

Pass condition:
- Owner labels name only focused owner documents.
- Related-reference labels name only adjacent context.
- Maintain-check labels describe check sources, scope, evidence, pass criteria, routes, or companion documents without implying terminology ownership.

Failure:
- `Primary owner` and `Related references` are used interchangeably.
- A related reference is described as the owner contract or as another primary owner.
- A Maintain check uses ownership language for check sources, checked file families, route destinations, or companion maintenance pages.
- A route or reference route is described as the primary owner when it only helps navigation.
- An index or route page is labeled as the primary owner for a detailed term, API concern, schema concern, storage concern, security concern, or display wording concern already owned by a focused document.
- Owner contract wording points to a route label or metadata entry instead of the contract defined by the focused owner.

Fix:
- Restore `Primary owner` for the focused owner and move adjacent documents to `Related references`.
- Restore `Check sources`, `Applies to`, `Route`, `Reference route`, or `Maintained with` for non-owner labels.
- Split the glossary term when one label is trying to cover multiple canonical owners.
- Reword owner contract usage so it points to the focused owner document's contract.
- Keep indexes as navigation or related references unless the indexed concept itself is the route concept.

## CHK-TERM-013: glossary link route semantics

Check sources:
- [Glossary](../../reference/glossary.md)
- [Terminology Map](../../../terminology-map.yaml)
- [API Value Sets](../../reference/api/schema-value-sets.md)
- [API error details](../../reference/api/error-details.md)
- [Authoring Guide](../authoring-guide.md)

Applies to:
- Markdown links whose target is the glossary, and the nearby prose that gives each link its meaning.

Evidence to inspect:
- Inspect each glossary link's text and surrounding sentence. The link is valid only when that context refers to a term that actually appears in the curated glossary.
- Confirm glossary links are used for core reader-facing concept summaries, not for the complete structured terminology inventory.
- Confirm terms that exist only in `docs/terminology-map.yaml` route to the terminology map or focused owner, not to the glossary.
- Confirm detailed value and metadata contexts route to focused owners or `docs/terminology-map.yaml`.
- Detailed contexts include schema fields, enum values, API values and value sets, helper values, storage record details, and translation-control terms.
- Do not route detailed contexts to the glossary unless the exact term is intentionally included as a core glossary term.
- Confirm reserved or profile-gated value contexts route to [API Value Sets](../../reference/api/schema-value-sets.md) and `docs/terminology-map.yaml`.
- Do not route reserved or profile-gated value contexts to the glossary unless the linked term actually appears in the glossary.
- Confirm the check reviews semantic route correctness, not only file or anchor existence.

Pass condition:
- Every glossary link refers to a curated glossary term in a core reader-facing concept-summary context.
- Terminology-map-only terms route to the terminology map or focused owners.
- Detailed value, schema, helper, storage, and translation-control contexts route to their focused owners or `docs/terminology-map.yaml`.
- Exception: the exact linked term is intentionally included as a core glossary term.

Failure:
- A Markdown link points to the glossary for a term that is absent from the curated glossary.
- A link uses the glossary as if it were the complete structured terminology inventory.
- A terminology-map-only term links to the glossary.
- A schema field, enum value, API value or value set, helper value, storage record detail, or translation-control term links to the glossary instead of its focused owner or `docs/terminology-map.yaml`.
- Exception: the exact term is intentionally included as a core glossary term.
- A reserved or profile-gated value context links to the glossary instead of API Value Sets and `docs/terminology-map.yaml`.
- Exception: the linked term appears in the glossary.
- A glossary link resolves as a Markdown link but points readers to the wrong semantic owner.

Fix:
- Retarget the link to the focused owner, `docs/terminology-map.yaml`, [API Value Sets](../../reference/api/schema-value-sets.md), or [API error details](../../reference/api/error-details.md).
- For detailed contexts, retarget to the relevant storage owner or translation guidance as appropriate.
- Add or adjust a glossary term only when readers need compact glossary coverage and the term has terminology-map metadata and one focused primary owner.
- Keep glossary links for selected core reader-facing terms only.

Related checks:
- [CHK-TERM-005](#chk-term-005-terminology-map-alignment)
- [CHK-TERM-011](#chk-term-011-glossary-entry-focus)
- [CHK-LINK-008](links-and-indexes.md#chk-link-008-terminology-and-metadata-owner-targets)
- [CHK-LINK-010](links-and-indexes.md#chk-link-010-glossary-link-route-correctness)

## CHK-TERM-014: Harness/Core and judgment-boundary terminology

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Authoring Guide](../authoring-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Evidence to inspect:
- Confirm "Harness" names the local work-authority product/system for AI-assisted product work, not the local authority record.
- Confirm "Core" names the local authority record for Harness state.
- Confirm user-visible criteria for checking work use "verification criteria"; avoid "verification expectations" and "inspection expectations" except as explicit search patterns or quoted legacy examples.
- Confirm prose uses "current scope" or "currently applied scope" for current context, while preserving exact identifiers such as `active_change_unit_ref`.
- Confirm `Write Authorization` remains distinct from ordinary write approval, sensitive-action approval, final acceptance, residual-risk acceptance, and broad user-owned judgment.
- In Korean, confirm the paired terms use "검증 기준" and "현재 적용 범위".
- Avoid prose "활성 범위" for current context.
- Preserve `Write Authorization` as an exact product label with Korean explanation when needed.

Failure:
- Harness is described as the local authority record, or Core is described as the product/system.
- A document uses "verification expectations", "inspection expectations", "확인 기대", or "검증 기대치" as ordinary prose for user-visible criteria.
- A document uses prose "active scope" or "활성 범위" for current context outside an exact identifier, status value, search pattern, or quoted legacy example.
- `Write Authorization` is treated as the same thing as ordinary approval, final acceptance, residual-risk acceptance, or broad user-owned judgment.

Fix:
- Align the wording with `docs/terminology-map.yaml`.
- Use "Harness" for the product/system and "Core" for the authority record.
- Use "verification criteria" and Korean "검증 기준".
- Use "current scope", "currently applied scope", and Korean "현재 적용 범위" for current context.
- Preserve `Write Authorization` as the exact product label, and use separate wording for ordinary approval or acceptance concepts.
