# API examples checks

Use these checks for API and reference examples. They check documentation example quality only; they do not validate product runtime conformance.

## CHK-EXAMPLE-001: durable API and Reference scenarios

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [API Methods](../../reference/api/methods.md)
- The affected Reference owner selected from [Reference Index](../../reference/README.md)

Evidence to inspect:
- Confirm API and Reference examples use stable product or user scenarios.
- When API method examples share a scenario across documents, confirm the shared scenario is consistent across affected API examples, paired Korean examples, checks, and routes.
- Confirm examples do not use documentation maintenance, refactoring, documentation transitions, route reshaping, or section restructuring as their scenario.
- Confirm documentation paths are used as example payload only when the document is specifically about documentation maintenance.
- Confirm paired English and Korean examples preserve equivalent scenario details.

Failure:
- Example payload includes internal documentation paths when the document is not about documentation maintenance.
- Example goal describes documentation maintenance instead of product or user behavior.
- Example baseline, artifact, run, or judgment names refer to documentation maintenance.
- A shared API example scenario changes in one language, route, or check but not the paired owner set.
- One language keeps a different scenario after paired updates.

Fix:
- Replace the example with a durable product or user scenario.
- Keep shared API example scenarios consistent across API examples, paired Korean examples, checks, and routes.
- Keep file paths only when the document is explicitly about documentation maintenance.
- Update paired English and Korean examples by meaning unit.

## CHK-EXAMPLE-002: Korean scenario wording

Check sources:
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Confirm shared example scenarios in Korean documents use natural Korean wording.
- Confirm Korean examples avoid compressed noun chains and do not preserve English noun order when a natural Korean phrase is clearer.
- Confirm repeated scenario phrases stay consistent across related examples.

Failure:
- A Korean shared example scenario uses compressed noun chains or preserved English noun order that makes the scenario harder to read.
- Repeated Korean scenario phrasing drifts across related examples without a scenario distinction.

Fix:
- Rewrite the Korean shared scenario wording as natural Korean while preserving equivalent scenario details.
- Keep repeated scenario phrases consistent after the scenario is introduced.

## CHK-EXAMPLE-003: API example internal consistency

Check sources:
- [Authoring Guide](../authoring-guide.md)
- The affected API method owner document

Evidence to inspect:
- Example refs are introduced or explicitly described as existing.
- A response snapshot does not include refs from a newer `state_version`.
- Sensitive approval reasons match the request's `sensitive_categories` or stated precondition.
- Artifact refs do not appear without staging, promotion, or existing-artifact context.
- Expiration timestamps use placeholders or dates that are later than the example's stated issue date.
- Cross-method examples that share a scenario do not contradict each other.
- Representative responses do not silently drop meaningful request fields unless labeled as abbreviated.

Failure:
- Status examples include newer-version supporting refs.
- Approval reasons do not match `sensitive_categories`.
- Artifact refs appear without lifecycle context.
- Staged handles have stale fixed expiration timestamps.
- Close-readiness evidence refers to missing run or judgment refs.
- Response examples drop `options` or `affected_refs` from a user-judgment request without saying the response is abbreviated.

Fix:
- Align refs, versions, sensitive categories, artifact lifecycle, timestamps, and shared scenario data.

## CHK-EXAMPLE-004: field-name consistency

Check sources:
- [Authoring Guide](../authoring-guide.md)
- The affected method, schema, or storage owner document

Evidence to inspect:
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

## CHK-EXAMPLE-005: API owner routing in examples

Check sources:
- [Reference Index](../../reference/README.md)
- [API Methods](../../reference/api/methods.md)
- The applicable API owner selected from the Reference Index or API Methods router

Evidence to inspect:
- Confirm example notes and surrounding prose route API concerns to the narrow owner selected from the Reference Index, API Methods router, or `doc-index.yaml`.
- Confirm broad API family indexes are not treated as focused contract owners.
- Confirm method-level owner routing is linked through the API Methods router rather than repeated in example guidance.
- Confirm examples do not redefine API methods, schema names, fields, values, or error codes outside the appropriate API owner.

Failure:
- A link points to a broad index when the reader needs a precise contract owner.
- A broad API family page is treated as the owner for split API concerns.
- A non-owner page reproduces request/response structure, response branches, error behavior, schema fields, enum-like values, or method semantics as if it owns them.

Fix:
- Retarget the link to the exact owner selected from the Reference Index, API Methods router, or `doc-index.yaml`.
- Replace duplicated contract text with a short reader consequence and owner link.
