# Links and indexes checks

Use these checks for relative links, anchors, route tables, `README` pages, `doc-index.yaml`, and retrieval guidance. These checks keep navigation stable; they do not define the contracts being linked.

## CHK-LINK-001: broken links and stale routes

Owner:
- [Authoring Guide](../authoring-guide.md)
- [doc-index.yaml](../../../doc-index.yaml)
- [Reference Index](../../reference/README.md)

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
- Confirm route text points to a current owner when one exists.
- Confirm a missing owner is exposed as a documentation gap instead of being hidden behind broad route prose, Maintain guidance, or copied contract detail.

Failure:
- A route document answers a contract question without a current canonical owner.
- A route sends readers to a broad index or Maintain page when the question needs an owner that does not yet exist.
- `doc-index.yaml` names a default owner that cannot answer the routed question.

Fix:
- Retarget the route to the exact owner selected from the Reference Index.
- If no current owner exists, state the owner gap and route to the closest real owner, [Scope Reference](../../reference/scope.md), or [Implementation Guide](../../build/implementation-guide.md) as appropriate.
- Create or designate a real owner only in the same paired documentation batch that defines the owner boundary.

## CHK-LINK-004: check-page routing

Owner:
- [Checks Index](../checks.md)
- [doc-index.yaml](../../../doc-index.yaml)

Check:
- Confirm `checks.md` remains a short index to focused check pages.
- Confirm new check pages are paired under `docs/en/maintain/checks/` and `docs/ko/maintain/checks/`.
- Confirm `doc-index.yaml` contains route metadata for each active paired check page.

Failure:
- The index starts accumulating detailed check bodies again.
- A new check page exists in only one language without an owner-backed reason.
- `doc-index.yaml` routes only to the index when a focused check page is the expected owner.

Fix:
- Move detailed procedures to the focused page.
- Add or update the paired-language page and route metadata in the same documentation batch.

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
- Keep agent context to the current task summary, needed owner section, and needed language.

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
