# Terminology checks

Use these checks when an edit changes product terms, Korean prose terms, mixed-language expressions, identifier explanations, documentation-routing terms, close-readiness wording, or access/security wording. The terminology map owns maintainer terminology controls; product contracts remain in their reference owners.

## CHK-TERM-001: close-readiness terminology

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Translation Guide](../translation-guide.md)

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

## CHK-TERM-002: terminology drift

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Translation Guide](../translation-guide.md)

Check:
- Search changed prose for new product terms, mixed-language Korean, and alternate spellings of existing concepts.
- Confirm each new durable term is owned by the glossary, the terminology map, or the relevant reference owner.
- Confirm Korean sentences translate ordinary English noun phrases unless the English is an identifier, intentional product label, or natural technical borrowing.

Failure:
- The same concept appears under multiple prose terms without an owner-backed distinction.
- A Korean sentence keeps an English noun phrase that is not an identifier, intentional product label, or natural technical borrowing.

Fix:
- Align wording with the terminology owner.
- Add or revise owner terminology only when the new distinction is intentional.

## CHK-TERM-003: `complete` intent ambiguity

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [API Value Sets](../../reference/api/schema-value-sets.md)
- [API Methods](../../reference/api/methods.md)

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

## CHK-TERM-004: surface, access, and security wording

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Security](../../reference/security.md)
- [Agent Integration](../../reference/agent-integration.md)
- [Translation Guide](../translation-guide.md)

Check:
- Confirm `surface_id`, surface, connector, capability, and access-class wording does not imply authority, approval, or binding proof unless the owner says so.
- Confirm access-related terms preserve the distinction between documentation guidance and runtime enforcement.
- Confirm cooperative, detective, prevention, guard, freeze, careful mode, sandbox, permission, blocking, tamper-proof, isolation, and capability wording stays within owner-backed terminology.

Failure:
- A surface or access term is used as proof of permission, user judgment, Write Authorization, security isolation, or runtime enforcement without owner support.
- Security wording implies stronger isolation, sandboxing, permission enforcement, or tamper-proof behavior merely because route text, examples, or out-of-scope material mentions it.

Fix:
- Reword the statement as identification, routing, or documented guidance as appropriate.
- Link to Security for guarantee semantics, Agent Integration for connector context, and Scope for support availability when needed.

## CHK-TERM-005: terminology-map alignment

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [English Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Check:
- Compare changed terminology guidance with `docs/terminology-map.yaml`.
- Confirm `owner_documents` targets point to the focused owner document when one exists, not to a broad index or family page.
- Confirm glossary owner links follow the same focused-owner rule.
- Confirm forbidden mixed-language examples in the guides use concrete strings, not vague descriptions.
- Confirm any new forbidden expression appears in the terminology map and both translation guides.

Failure:
- The guides and terminology map disagree.
- A terminology-map or glossary owner target points to an index when a focused owner already owns the term's meaning, value set, API concern, storage concern, or display wording.
- A Korean guide describes a banned mixed-language pattern without a searchable real string such as "artifact를 저장한다".

Fix:
- Align the map and both guides in the same documentation batch.
- Retarget owner links to the focused owner, using an index only for navigation concepts or explicit owner gaps.
- Replace vague placeholders with concrete examples that can be searched.

## CHK-TERM-006: `active` versus supported or applicable

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Core Model](../../reference/core-model.md)
- [Agent Integration](../../reference/agent-integration.md)

Check:
- Search changed prose for `active`.
- Confirm `active` is used only for runtime or currently applied state, exact identifiers, exact status values, active scope, active Change Unit, or active surface context.
- Confirm supported contracts, supported API methods, supported values, maintained documents, and owner routing use terms such as "supported", "applicable", "maintained", or "current", not `active`.

Failure:
- A document uses `active` for an owner path, contract, API method, reference document, or other documentation route when it means applicable, supported, or maintained.
- Korean prose translates `active` as "활성" for a documentation contract or owner route instead of using the appropriate Korean term.

Fix:
- Replace `active` with the owner-backed term: "applicable owner path", "supported API method", "supported value", "maintained document", or "current state".
- Keep `active` only when it is an exact identifier, status value, or currently applied runtime/session state.

## CHK-TERM-007: retired or unsupported concept names

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Scope](../../reference/scope.md)
- [Reference Index](../../reference/README.md)
- [Authoring Guide](../authoring-guide.md)
- [Template Bodies](../../reference/template-bodies.md)

Check:
- Search maintained Reference docs, glossary entries, `docs/terminology-map.yaml`, `doc-index.yaml`, display wording owners, and changed metadata for retired, deleted, or unsupported concept names.
- Confirm unsupported capability names are used only when a semantic owner still needs the exact name, or when a Maintain/terminology owner intentionally lists a searchable forbidden expression.
- Confirm Reference owners describe stable categories, owner gaps, or out-of-scope capability families instead of preserving obsolete names as examples.
- Confirm negative examples do not make removed names look like supported concepts, owner routes, storage record families, or display packages.

Failure:
- A glossary entry, terminology-map entry, metadata route, Reference page, or display wording owner keeps a removed or unsupported concept name solely to say that it is not supported.
- A negative example causes retrieval to treat the old name as a supported contract, supported capability, or owner route.
- A display wording owner or storage-related note keeps an unsupported package or pseudo-family name that becomes searchable as an official concept.

Fix:
- Remove the stale name or replace it with the stable category and the applicable owner link.
- Move searchable banned terminology to the Terminology Map and translation guides when the term needs to remain searchable for maintainers.
- Remove unsupported display or storage-like names unless a terminology owner intentionally preserves them as forbidden terminology.

## CHK-TERM-008: documentation-routing terms stay documentary

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [Glossary](../../reference/glossary.md)
- [Authoring Guide](../authoring-guide.md)

Check:
- Search changed prose for documentation-routing terms such as `applicable owner path`, owner route, owner target, route metadata, and owner gap.
- Confirm these terms describe documentation navigation, authoring, retrieval, or metadata only.
- Confirm they are not described as product behavior, storage persistence, runtime state, evidence authority, close-readiness state, or API support.

Failure:
- A documentation-routing term is used as if it were a persisted product field, runtime status, API value, storage record, support guarantee, or close-readiness result.
- A guide says a product behavior is available because an owner path applies, instead of because Scope and the semantic owner define support.

Fix:
- Reword the term as documentation routing or metadata.
- Route product behavior, storage persistence, runtime state, and API support to the focused product owner.
