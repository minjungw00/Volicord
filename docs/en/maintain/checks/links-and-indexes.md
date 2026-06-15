# Links and indexes checks

Use these checks for relative links, anchors, route tables, `README` pages, `doc-index.yaml`, and retrieval guidance. These checks keep navigation stable; they do not define or certify the contracts being linked.

Use `docs/doc-index.yaml` as the canonical machine-readable owner route. It owns route metadata for:

- `doc_id`
- paired paths
- role
- owner scope and non-owner scope
- dependencies
- normative level
- audience

Use the Reference Index as the human-readable route when readers need a navigation page.

This route metadata is documentation-maintenance input only. It is not runtime configuration, product state, or an API conformance source.

Navigation review boundary: link failures are documentation route failures. They are not failures of the API, storage, security, or product behavior named by a target page.

## CHK-LINK-001: broken links and stale routes

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)
- [Reference Index](../../reference/README.md)

Evidence to inspect:
- Validate changed relative links, file paths, anchors, route tables, and paired-language links.
- Confirm owner-routing metadata is checked against `docs/doc-index.yaml` before relying on a README, route page, or prose summary.
- Confirm maintained navigation uses the compact maintained routes from the authoring owner.
- Confirm contract links point to the canonical owner, not to a convenient duplicate.
- For API error links, use [API errors](../../reference/api/errors.md) as the family index only.
- Route public code meanings, precedence, response branch routing, close-readiness blocker/API response boundaries, public-code-to-blocker boundaries, and machine-readable details to their focused API owners.

Failure:
- A link targets a missing file, missing anchor, stale route family, wrong-language owner, or deleted compatibility path.
- A route or check uses a prose owner summary as the machine-readable owner route when `docs/doc-index.yaml` has the canonical metadata.
- A route page links directly to deep contract detail where the Reference Index should choose the owner.

Fix:
- Update the link to the maintained route or canonical owner.
- Add or preserve anchors only where they are needed for stable links.

## CHK-LINK-002: hidden anchors

Check sources:
- [Translation Guide](../translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- For Korean headings, keep visible headings natural Korean.
- When an English anchor must remain stable, use a hidden HTML anchor before the natural Korean heading.
- After heading changes, check inbound links in the changed language and its paired route.

Failure:
- Korean visible headings are made unnatural to preserve an English anchor.
- A heading change removes a stable anchor and breaks inbound links.

Fix:
- Restore the stable anchor with a hidden HTML anchor.
- Keep the visible heading natural and update links that should follow the new heading.

## CHK-LINK-003: route documents expose owner gaps

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
- Inspect changed route documents, `README` files, indexes, and `doc-index.yaml` entries for questions whose exact canonical owner is missing or unclear.
- Treat `docs/doc-index.yaml` as the canonical machine-readable owner route and the Reference Index as a reader-facing route.
- Confirm route text points to an applicable owner when one exists.
- Confirm a missing owner is exposed as a documentation gap instead of being hidden behind broad route prose, Maintain guidance, or copied contract detail.

Failure:
- A route document answers a contract question without an applicable canonical owner.
- A route sends readers to a broad index or Maintain page when the question needs an owner that does not yet exist.
- `doc-index.yaml` names a default owner that cannot answer the routed question.

Fix:
- Retarget the route to the exact owner selected from the Reference Index.
- If no applicable owner exists, state the owner gap and route to the closest real owner, [Scope Reference](../../reference/scope.md), or [Implementation Guide](../../build/implementation-guide.md) as appropriate.
- Create or designate a real owner only in the same paired documentation batch that defines the owner boundary.

## CHK-LINK-004: check-page routing

Check sources:
- [Checks Index](../checks.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
- Confirm `checks.md` remains a short index to focused check pages.
- Confirm new check pages are paired under `docs/en/maintain/checks/` and `docs/ko/maintain/checks/`.
- Confirm `doc-index.yaml` contains route metadata for each maintained paired check page.

Failure:
- The index starts accumulating detailed check bodies again.
- A new check page exists in only one language without an owner-backed reason.
- `doc-index.yaml` routes only to the index when a focused check page is the expected owner.

Fix:
- Move detailed procedures to the focused page.
- Add or update the paired-language page and route metadata in the same documentation batch.

## CHK-LINK-005: method owner routing placement

Check sources:
- [API Methods](../../reference/api/methods.md)
- [doc-index.yaml](../../../doc-index.yaml)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Confirm the supported public API method list and method-level owner table live in API Methods.
- Confirm `AGENTS.md`, Reference indexes, and Maintain docs link to API Methods instead of repeating the full method map.
- Confirm `doc-index.yaml` paths for the method router and method owners match existing files.

Failure:
- A non-method-router page repeats the supported public API method owner table.
- A method route points to a missing file, wrong language path, or stale method owner.
- `doc-index.yaml` omits or misroutes the method router or a method owner route.

Fix:
- Keep the full method list in API Methods and shrink other pages to a short route link.
- Update the affected path in API Methods or `doc-index.yaml`.

## CHK-LINK-006: `doc-index.yaml` structure references

Check sources:
- [doc-index.yaml](../../../doc-index.yaml)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Inspect prose, route tables, prompts, and check guidance that name `docs/doc-index.yaml` structures.
- Confirm prose describes `docs/doc-index.yaml` as documentation owner-route metadata only, not as runtime configuration, product state, or an API conformance source.
- Confirm they refer only to structures and keys that exist.
- Existing structures and keys include `shared_documents`, `documents`, `entry_schema`, `doc_id`, `path`, `path_en`, `path_ko`, `role`, `owner_for`, `not_owner_for`, `depends_on`, `normative_level`, and `audience`.
- Confirm a document does not describe missing sections, generated indexes, or current runtime state inside `doc-index.yaml`.

Failure:
- Text tells maintainers to read a nonexistent map, key, section, or language path field.
- A route names a `doc_id` or owner metadata entry that is absent from `doc-index.yaml`.
- Text treats `doc-index.yaml` as runtime config, product contract data, API conformance data, or product guarantee data.

Fix:
- Reword the documentation to match the actual YAML structure, or update `doc-index.yaml` as documentation owner-route metadata in the same documentation batch.
- Route contract detail to the owner instead of extending `doc-index.yaml`.

## CHK-LINK-007: API error owner routing

Check sources:
- [API errors family index](../../reference/api/errors.md)
- [API error codes](../../reference/api/error-codes.md)
- [API error precedence](../../reference/api/error-precedence.md)
- [API error routing](../../reference/api/error-routing.md)
- [API blocker routing](../../reference/api/blocker-routing.md)
- [API error details](../../reference/api/error-details.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Use [API errors](../../reference/api/errors.md) as the family index only.
- Route public `ErrorCode` meanings to [API error codes](../../reference/api/error-codes.md).
- Route precedence, conflict selection, and stale-state ordering to [API error precedence](../../reference/api/error-precedence.md).
- Route rejected-response, blocked-result, and `dry_run` response branch routing to [API error routing](../../reference/api/error-routing.md).
- Route close-readiness blocker/API response boundaries and the public-code-to-blocker boundary to [API blocker routing](../../reference/api/blocker-routing.md).
- Route `harness.close_task` method-specific blocker behavior to [`harness.close_task`](../../reference/api/method-close-task.md).
- Route machine-readable `ToolError.details` fields, helper values, and detail-value meanings to [API error details](../../reference/api/error-details.md).

Failure:
- A document sends all API error questions to the family index or to one broad error page.
- A method, schema, storage, conformance, or Maintain page redefines code meanings, precedence, routing, or details outside the focused API error owner.
- A repeated API error owner map drifts from the focused owners.

Fix:
- Retarget each link to the narrow API error owner.
- Shrink broad error explanations to a reader consequence plus owner links.
- Keep owner maps in the canonical API error route or owner pages.

## CHK-LINK-008: terminology and metadata owner targets

Check sources:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [doc-index.yaml](../../../doc-index.yaml)
- [Reference Index](../../reference/README.md)
- [API Value Sets](../../reference/api/schema-value-sets.md)
- [API error details](../../reference/api/error-details.md)

Applies to:
- Terminology-map owner targets, glossary owner and related-reference targets, `doc-index.yaml` owner metadata, and terminology route tables touched by the edit.

Evidence to inspect:
- Inspect terminology-map `primary_owner` and `related_references`, glossary owner and related-reference targets, `doc-index.yaml` owner metadata, and route tables touched by the edit.
- Check `doc-index.yaml` first for machine-readable owner routing, then use the glossary or Reference Index only for reader-facing term or route context.
- Inspect glossary content by role, regardless of whether it is represented as a compact table, compact entries, or another human-readable view.
- Confirm `docs/terminology-map.yaml` remains the complete structured terminology metadata source.
- Confirm the glossary remains compact and reader-facing.
- Confirm the glossary is not required to mirror every terminology-map term.
- Confirm checks do not require a specific glossary layout.
- Confirm every term included in the glossary has matching terminology-map metadata.
- Confirm Markdown links to the glossary point only from contexts that refer to terms included in the curated glossary.
- Confirm terminology-map-only terms route to `docs/terminology-map.yaml` or focused owners, not to the glossary.
- Confirm detailed value and metadata contexts route to focused owners or `docs/terminology-map.yaml`.
- Detailed contexts include schema fields, enum values, API values, helper values, storage details, and translation-control terms.
- Add a glossary link for a detailed context only when the exact term is intentionally included as a core glossary term.
- Confirm reserved or profile-gated value contexts route to [API Value Sets](../../reference/api/schema-value-sets.md) and `docs/terminology-map.yaml`.
- Add a glossary link for a reserved or profile-gated value only when the linked term is included in the curated glossary.
- Confirm each owner target points to the focused owner document when one exists.
- Confirm glossary `Primary owner` values and terminology-map `primary_owner` targets match for the same included term unless an explicit owner gap is named.
- Confirm `doc-index.yaml` `owner_for` and `not_owner_for` metadata does not contradict the focused owner named by the glossary or terminology map for the same concept.
- Confirm `doc-index.yaml` does not overclaim ownership for a focused term by making a route, index, or broad document look primary when the glossary or terminology map names a focused owner.
- Confirm terminology-map `related_references` and glossary `See also` or `Related references` hold adjacent context only; they must not be used as alternate primary owners or contradict each other.
- Use a broad index only when the concept is index-owned navigation, a first-hop route, or an explicitly named owner gap.
- Confirm API error code meanings, error precedence, API response branch routing, close-readiness blocker routing, and `ToolError.details` targets stay separate.

Pass condition:
- Terminology routes and metadata point to focused owners.
- The terminology map remains the complete structured terminology metadata source.
- The glossary remains a compact reader-facing subset, and every glossary-included term has matching terminology-map metadata, the same primary owner, and non-contradictory related references.
- Detailed value, schema, helper, storage, and translation-control contexts route to focused owners or `docs/terminology-map.yaml`.
- Route and index metadata do not overclaim focused ownership.

Failure:
- A terminology, glossary, metadata, or route target points to a broad index when a focused owner exists.
- A glossary-included term is missing from the terminology map or lacks matching terminology-map metadata.
- A check or route requires the glossary to include every terminology-map term.
- A check requires a specific glossary layout.
- A Markdown link points to the glossary for linked text or nearby prose that is not a curated glossary term.
- A terminology-map-only term is linked to the glossary instead of `docs/terminology-map.yaml` or its focused owner.
- A detailed context links to the glossary even though the exact term is not intentionally included as a core glossary term.
- Detailed contexts include schema fields, enum values, API values, helper values, storage details, and translation-control terms.
- A reserved or profile-gated value context links to the glossary instead of API Value Sets and `docs/terminology-map.yaml`.
- Exception: the linked term is included in the curated glossary.
- A glossary-included term points to one primary owner while the terminology map points to another.
- `doc-index.yaml` metadata makes a different document look primary for the same concept without a documented owner split or owner gap.
- `doc-index.yaml` overclaims ownership for a focused term, API concern, schema concern, storage concern, security concern, or display wording concern.
- A terminology-map `related_references`, glossary `See also`, or glossary `Related references` entry is treated as a second primary owner or contradicts the adjacent references for the same term.
- An API error family index is used as the owner for public code meanings, precedence, response branch routing, close-readiness blocker routing, or machine-readable details.
- `doc-index.yaml` metadata makes a route/index document look like the owner of focused contract detail.

Fix:
- Retarget the link or metadata field to the focused owner.
- Narrow `doc-index.yaml` `owner_for` metadata or add `not_owner_for` metadata so route and index pages do not overclaim focused terms.
- Synchronize glossary content, the terminology map, and `doc-index.yaml` metadata in the same documentation batch when the owner target changes.
- Add the term to the terminology map before including it in the glossary, or remove it from the compact glossary view.
- Keep terminology-map-only terms out of the glossary unless readers need compact glossary coverage.
- Retarget glossary links to focused owners or `docs/terminology-map.yaml` when the linked context is outside the curated glossary subset.
- Retarget detailed value, schema, helper, storage, and translation-control links to the focused owner or `docs/terminology-map.yaml`.
- Route reserved and profile-gated value contexts to API Value Sets and `docs/terminology-map.yaml`.
- Keep glossary links only for included core glossary terms.
- Move adjacent documents from primary-owner fields into related-reference fields.
- Keep indexes as navigation unless they truly own the route concept.
- If the focused owner is missing, name the owner gap instead of routing the contract to an index.

Related checks:
- [CHK-TERM-005](terminology.md#chk-term-005-terminology-map-alignment)
- [CHK-TERM-011](terminology.md#chk-term-011-glossary-entry-focus)
- [CHK-TERM-012](terminology.md#chk-term-012-owner-routing-label-usage)
- [CHK-TERM-013](terminology.md#chk-term-013-glossary-link-route-semantics)
- [CHK-LINK-003](#chk-link-003-route-documents-expose-owner-gaps)

## CHK-LINK-009: moved-concept and owner-boundary anchors

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
- Inspect hidden anchors and explicit anchor IDs for concepts whose owner moved or whose owner boundary changed.
- Confirm the stable anchor for a concept lives in the document that now owns the concept.
- Confirm redirect-style hidden anchors do not remain in old documents when they make the old document look like the owner.
- Confirm an anchor ID does not imply that an old document, broad index, or route page still owns a moved concept.

Failure:
- A hidden anchor for a moved concept remains in the old document and receives owner-like inbound links.
- A route or index page keeps an anchor ID that names a contract it no longer owns.
- Inbound links land on a compatibility anchor instead of the actual owner section, causing readers or retrieval to treat the old page as canonical.

Fix:
- Move or add the stable anchor on the actual owner section.
- Retarget inbound links to the owner.
- Remove redirect-style anchors from old documents when they create owner confusion; keep only short route links where navigation is still useful.

## CHK-LINK-010: glossary link route correctness

Check sources:
- [Glossary](../../reference/glossary.md)
- [Terminology Map](../../../terminology-map.yaml)
- [Reference Index](../../reference/README.md)
- [API Value Sets](../../reference/api/schema-value-sets.md)
- [API error details](../../reference/api/error-details.md)
- [Authoring Guide](../authoring-guide.md)

Applies to:
- Markdown links whose target is `glossary.md` or a relative path to the glossary.

Evidence to inspect:
- Validate that each glossary link resolves to the intended file or anchor.
- Inspect the link text and nearby prose to confirm the linked context refers to a term that appears in the curated glossary.
- Confirm the glossary link is a route to a core reader-facing concept summary, not a route to the complete structured terminology inventory.
- Confirm terms that exist only in `docs/terminology-map.yaml` route to the terminology map or focused owner.
- Confirm detailed value and metadata contexts route to focused owners or `docs/terminology-map.yaml`.
- Detailed contexts include schema fields, enum values, API values and value sets, helper values, storage record details, and translation-control terms.
- Add a glossary link for a detailed context only when the exact term is intentionally included as a core glossary term.
- Confirm reserved or profile-gated value contexts route to [API Value Sets](../../reference/api/schema-value-sets.md) and `docs/terminology-map.yaml`.
- Do not route reserved or profile-gated value contexts to the glossary unless the linked term actually appears in the glossary.
- Use the Reference Index or `doc-index.yaml` when the linked context needs an owner that is not the glossary.

Pass condition:
- Every glossary Markdown link is resolvable.
- Each glossary link is semantically correct for a curated glossary term.
- Other terminology, value, schema, helper, storage, and translation-control contexts route to their focused owners or `docs/terminology-map.yaml`.
- Exception: the exact linked term is intentionally included as a core glossary term.

Failure:
- A glossary link is unbroken but semantically points to the wrong owner.
- Linked text or surrounding prose names a term that is absent from the curated glossary.
- A terminology-map-only term links to the glossary.
- A detailed context links to the glossary even though the exact term is not intentionally included as a core glossary term.
- Detailed contexts include schema fields, enum values, API values or value sets, helper values, storage record details, and translation-control terms.
- A reserved or profile-gated value context links to the glossary when the linked term is not included in the glossary.

Fix:
- Retarget the Markdown link to the focused owner, Reference Index route, `docs/terminology-map.yaml`, or [API Value Sets](../../reference/api/schema-value-sets.md).
- For detailed contexts, retarget to the applicable schema, storage, error-detail, or translation owner.
- Add a glossary link only when the linked term is included in the curated glossary and the surrounding context needs a compact reader-facing concept summary.

Related checks:
- [CHK-LINK-001](#chk-link-001-broken-links-and-stale-routes)
- [CHK-LINK-008](#chk-link-008-terminology-and-metadata-owner-targets)
- [CHK-TERM-013](terminology.md#chk-term-013-glossary-link-route-semantics)

## CHK-LLM-001: duplicate contract text creates retrieval noise

Check sources:
- [doc-index.yaml](../../../doc-index.yaml)
- [Reference Index](../../reference/README.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Inspect agent guidance, `README` pages, maintain docs, and summaries for duplicate contract text that could be retrieved instead of the owner.
- Confirm retrieval guidance points agents to one owner section for the next action.

Failure:
- The same API, storage, security, schema, blocker, access-class, projection, or runtime-boundary contract appears in multiple non-owner places.
- Always-on context examples include full reference docs, full schemas, full DDL, historical logs, generated outputs, or both languages for the same `doc_id`.

Fix:
- Shrink duplicates to route text and owner links.
- Keep agent context to a focused summary, needed owner section, and needed language.

## CHK-LLM-002: one language per `doc_id`

Check sources:
- [Translation Guide](../translation-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)

Evidence to inspect:
- Confirm normal agent retrieval loads only one language for a given `doc_id`.
- Confirm paired English/Korean docs are loaded together only for translation, semantic parity review, or bilingual editing.

Failure:
- Agent instructions encourage loading both language versions by default.
- A prompt template injects paired docs for the same `doc_id` when comparison is not needed.

Fix:
- Reword retrieval guidance to one language per `doc_id`.
- Add the paired document only for parity-specific checks.
