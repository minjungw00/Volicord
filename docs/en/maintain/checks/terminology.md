# Terminology checks

Use these checks when an edit changes product terms, Korean prose terms, mixed-language expressions, identifier explanations, close-readiness wording, or access/security wording. The terminology map owns maintainer terminology controls; product contracts remain in their reference owners.

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
- Link to Security for guarantee semantics, Agent Integration for connector context, and Scope for current availability when needed.

## CHK-TERM-005: terminology-map alignment

Owner:
- [Terminology Map](../../../terminology-map.yaml)
- [English Translation Guide](../translation-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)

Check:
- Compare changed terminology guidance with `docs/terminology-map.yaml`.
- Confirm forbidden mixed-language examples in the guides use concrete strings, not vague descriptions.
- Confirm any new forbidden expression appears in the terminology map and both translation guides.

Failure:
- The guides and terminology map disagree.
- A Korean guide describes a banned mixed-language pattern without a searchable real string such as "artifact를 저장한다".

Fix:
- Align the map and both guides in the same documentation batch.
- Replace vague placeholders with concrete examples that can be searched.
