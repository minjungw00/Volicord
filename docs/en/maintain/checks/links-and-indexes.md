# Links and indexes checks

Use these checks for relative links, anchors, route tables, `README` pages, `doc-index.yaml`, and retrieval guidance. These checks keep navigation stable; they do not define the contracts being linked.

## CHK-LINK-001: broken links and stale routes

Owner:
- [Authoring Guide](../authoring-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)
- [Reference Index](../../reference/README.md)

Check:
- Validate changed relative links, file paths, anchors, route tables, and paired-language links.
- Confirm maintained navigation uses the compact maintained routes from the authoring owner.
- Confirm contract links point to the canonical owner, not to a convenient duplicate.
- For API error links, use [API errors](../../reference/api/errors.md) as the family index only; route public code meanings, precedence, response branch routing, close-readiness blocker/API response boundaries, public-code-to-blocker boundaries, and machine-readable details to their focused API owners.

Failure:
- A link targets a missing file, missing anchor, stale route family, wrong-language owner, or deleted compatibility path.
- A route page links directly to deep contract detail where the Reference Index should choose the owner.

Fix:
- Update the link to the maintained route or canonical owner.
- Add or preserve anchors only where they are needed for stable links.

## CHK-LINK-002: hidden anchors

Owner:
- [Translation Guide](../translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

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

## CHK-LINK-003: route documents expose owner gaps

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Reference Index](../../reference/README.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
- Inspect changed route documents, `README` files, indexes, and `doc-index.yaml` entries for questions whose exact canonical owner is missing or unclear.
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

Owner:
- [Checks Index](../checks.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
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

Owner:
- [API Methods](../../reference/api/methods.md)
- [doc-index.yaml](../../../doc-index.yaml)
- [Authoring Guide](../authoring-guide.md)

Check:
- Confirm the supported public API method list and method-level owner table live in API Methods.
- Confirm `AGENTS.md`, Reference indexes, and Maintain docs link to API Methods instead of repeating the full method map.
- Confirm `doc-index.yaml` paths for the method router and method owners match existing files.

Failure:
- A non-method-router page repeats the supported public API method owner table.
- A method route points to a missing file, wrong language path, or stale method owner.
- `doc-index.yaml` omits or misroutes the method router or a method owner path.

Fix:
- Keep the full method list in API Methods and shrink other pages to a short route link.
- Update the affected path in API Methods or `doc-index.yaml`.

## CHK-LINK-006: `doc-index.yaml` structure references

Owner:
- [doc-index.yaml](../../../doc-index.yaml)
- [Authoring Guide](../authoring-guide.md)

Check:
- Inspect prose, route tables, prompts, and check guidance that name `docs/doc-index.yaml` structures.
- Confirm they refer only to structures and keys that exist, such as `shared_documents`, `documents`, `entry_schema`, `doc_id`, `path`, `path_en`, `path_ko`, `role`, `owner_for`, `not_owner_for`, `depends_on`, `normative_level`, and `audience`.
- Confirm a document does not describe missing sections, generated indexes, or current runtime state inside `doc-index.yaml`.

Failure:
- Text tells maintainers to read a nonexistent map, key, section, or language path field.
- A route names a `doc_id` or owner metadata entry that is absent from `doc-index.yaml`.
- Text treats `doc-index.yaml` as runtime config or product contract data.

Fix:
- Reword the documentation to match the actual YAML structure, or update `doc-index.yaml` as retrieval metadata in the same documentation batch.
- Route contract detail to the owner instead of extending `doc-index.yaml`.

## CHK-LINK-007: API error owner routing

Owner:
- [API errors family index](../../reference/api/errors.md)
- [API error codes](../../reference/api/error-codes.md)
- [API error precedence](../../reference/api/error-precedence.md)
- [API error routing](../../reference/api/error-routing.md)
- [API blocker routing](../../reference/api/blocker-routing.md)
- [API error details](../../reference/api/error-details.md)
- [Authoring Guide](../authoring-guide.md)

Check:
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

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [doc-index.yaml](../../../doc-index.yaml)
- [Reference Index](../../reference/README.md)

Check:
- Inspect terminology-map `owner_documents`, glossary owner links, `doc-index.yaml` owner metadata, and route tables touched by the edit.
- Confirm each owner target points to the focused owner document when one exists.
- Use a broad index only when the concept is index-owned navigation, a first-hop route, or an explicitly named owner gap.
- Confirm API error code meanings, error precedence, API response branch routing, close-readiness blocker routing, and `ToolError.details` targets stay separate.

Failure:
- A terminology, glossary, metadata, or route target points to a broad index when a focused owner exists.
- An API error family index is used as the owner for public code meanings, precedence, response branch routing, close-readiness blocker routing, or machine-readable details.
- `doc-index.yaml` metadata makes a route/index document look like the owner of focused contract detail.

Fix:
- Retarget the link or metadata field to the focused owner.
- Keep indexes as navigation unless they truly own the route concept.
- If the focused owner is missing, name the owner gap instead of routing the contract to an index.

## CHK-LLM-001: duplicate contract text creates retrieval noise

Owner:
- [doc-index.yaml](../../../doc-index.yaml)
- [Reference Index](../../reference/README.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Inspect agent guidance, `README` pages, maintain docs, and summaries for duplicate contract text that could be retrieved instead of the owner.
- Confirm retrieval guidance points agents to one owner section for the next action.

Failure:
- The same API, storage, security, schema, blocker, access-class, projection, or runtime-boundary contract appears in multiple non-owner places.
- Always-on context examples include full reference docs, full schemas, full DDL, historical logs, generated outputs, or both languages for the same `doc_id`.

Fix:
- Shrink duplicates to route text and owner links.
- Keep agent context to the current work summary, needed owner section, and needed language.

## CHK-LLM-002: one language per `doc_id`

Owner:
- [Translation Guide](../translation-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
- Confirm normal agent retrieval loads only one language for a given `doc_id`.
- Confirm paired English/Korean docs are loaded together only for translation, semantic parity review, or bilingual editing.

Failure:
- Agent instructions encourage loading both language versions by default.
- A prompt template injects paired docs for the same `doc_id` when comparison is not needed.

Fix:
- Reword retrieval guidance to one language per `doc_id`.
- Add the paired document only for parity-specific checks.
