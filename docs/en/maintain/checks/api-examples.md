# API examples checks

Use these checks for API and reference examples. They check documentation example quality only; they do not validate product runtime conformance.

## CHK-EXAMPLE-001: durable API and Reference scenarios

Owner:
- [Authoring Guide](../authoring-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [API Methods](../../reference/api/methods.md)
- The affected Reference owner selected from [Reference Index](../../reference/README.md)

Check:
- Confirm API and Reference examples use stable product or user scenarios.
- For supported API method examples, confirm they use the shared account data export confirmation sample task unless the documentation batch intentionally replaces that sample across the API examples, paired Korean examples, checks, and routes.
- Confirm examples do not use documentation maintenance, refactoring, migration, route cleanup, or section restructuring as their scenario.
- Confirm documentation paths are used as example payload only when the document is specifically about documentation maintenance.
- Confirm paired English and Korean examples preserve equivalent scenario details.

Failure:
- Example payload includes internal documentation paths when the document is not about documentation maintenance.
- Example task goal describes rewriting the Harness documentation set.
- Example baseline, artifact, run, or judgment names refer to documentation maintenance.
- The shared API sample task changes in one language, route, or check but not the paired owner set.
- One language keeps a different scenario after paired updates.

Fix:
- Replace the example with a durable product or user scenario.
- Use the shared account data export confirmation sample task, or replace the sample consistently across API examples, paired Korean examples, checks, and routes.
- Keep file paths only when the document is explicitly about documentation maintenance.
- Update paired English and Korean examples by meaning unit.

## CHK-EXAMPLE-002: Korean scenario wording

Owner:
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Check:
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

Owner:
- [Authoring Guide](../authoring-guide.md)
- The affected API method owner document

Check:
- Example refs are introduced or explicitly described as existing.
- A response snapshot does not include refs from a newer `state_version`.
- Sensitive approval reasons match the request's `sensitive_categories` or stated precondition.
- Artifact refs do not appear without staging, promotion, or existing-artifact context.
- Expiration timestamps use placeholders or clearly future example dates.
- Cross-method examples that share a scenario do not contradict each other.
- Representative responses do not silently drop meaningful request fields unless labeled as abbreviated.

Failure:
- Status examples include future-version supporting refs.
- Approval reasons do not match `sensitive_categories`.
- Artifact refs appear without lifecycle context.
- Staged handles have stale fixed expiration timestamps.
- Close-readiness evidence refers to missing run or judgment refs.
- Response examples drop `options` or `affected_refs` from a user-judgment request without saying the response is abbreviated.

Fix:
- Align refs, versions, sensitive categories, artifact lifecycle, timestamps, and shared scenario data.

## CHK-EXAMPLE-004: field-name consistency

Owner:
- [Authoring Guide](../authoring-guide.md)
- The affected method, schema, or storage owner document

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

## CHK-EXAMPLE-005: API owner routing in examples

Owner:
- [Reference Index](../../reference/README.md)
- [API Methods](../../reference/api/methods.md)
- The applicable API owner selected from the Reference Index or API Methods router

Check:
- Confirm example notes and surrounding prose route method behavior, common envelopes, state schemas, artifact schemas, judgment schemas, value sets, and API error concepts to the narrow owner.
- For API error concepts, route public code meanings to [API error codes](../../reference/api/error-codes.md), precedence and conflicts to [API error precedence](../../reference/api/error-precedence.md), error-versus-blocker routing to [API error routing](../../reference/api/error-routing.md), and machine-readable details to [API error details](../../reference/api/error-details.md).
- Confirm method-level owner routing is linked through the API Methods router rather than repeated in example guidance.
- Confirm examples do not redefine API methods, schema names, fields, values, or error codes outside the appropriate API owner.

Failure:
- A link points to a broad index when the reader needs a precise contract owner.
- A non-owner page reproduces request/response structure, response branches, error behavior, schema fields, enum-like values, or method semantics as if it owns them.

Fix:
- Retarget the link to the exact owner selected from the Reference Index.
- Replace duplicated contract text with a short reader consequence and owner link.
