# API examples checks

Use these checks for API and reference examples. They check documentation example quality only; they do not test or certify product runtime behavior, API conformance, product acceptance, or close readiness.

API method reference examples are method-local minimal examples. They may use stable product or user nouns, but each method owner document must be reviewable on its own and must match the applicable method, schema, value-set, and storage-effect owners. Conformance scenarios may be linked only as conceptual references; they are not payload sources for method examples.

Documentation review boundary: schema, value-set, response-branch, and scenario-spine checks audit example text against owner documents; they are not implementation or conformance tests.

String-like review rule: classify string-like example fields from the owner documents before judging the example. A string-like field may be a controlled value, an opaque identifier or classification string, or a free-form display string. The check outcome is about whether the documentation example uses that class correctly.

## CHK-EXAMPLE-001: durable method-local API and Reference scenarios

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [API Methods](../../reference/api/methods.md)
- The affected Reference owner selected from [Reference Index](../../reference/README.md)

Evidence to inspect:
- Confirm API and Reference examples use stable product or user scenarios.
- Confirm each API method example is minimal for the method it documents.
- Confirm each method example introduces every ref, `state_version` fact, artifact ref, run ref, judgment ref, blocker ref, and file path it uses, or explicitly states it as a method-local precondition.
- Confirm no method example depends on payload data, refs, state versions, artifact refs, run refs, judgment refs, blocker refs, or file paths introduced by another method reference document.
- Confirm conformance scenarios, `scenario_id`s, and scenario-level documents are not used as shared payload sources for method examples.
- Confirm conceptual links to conformance do not copy conformance payloads, refs, paths, state versions, or response snapshots into method examples for consistency.
- Confirm the check does not require all API examples to use one fixed product task or scenario.
- Confirm examples do not use documentation maintenance, migration, refactoring, documentation transitions, route reshaping, or section restructuring as their scenario.
- Confirm documentation paths are used as example payload only when the document is specifically about documentation maintenance.
- Confirm paired English and Korean examples preserve equivalent scenario details and owner boundaries.

Failure:
- An example cannot be reviewed without assuming hidden sample data, unstated setup, or another method document's scenario context.
- A method example asks readers to reuse a payload, ref, path, `state_version`, artifact ref, run ref, judgment ref, blocker ref, or response snapshot from another method reference document.
- A method example copies a conformance payload, ref, path, `state_version`, artifact ref, run ref, judgment ref, blocker ref, or response snapshot for consistency.
- A method example treats a conformance scenario as a shared fixture or example spine.
- A check requires all API examples to use one fixed sample task instead of accepting any stable, method-local product or user scenario.
- Example payload includes internal documentation paths when the document is not about documentation maintenance.
- Example goal describes documentation maintenance instead of product or user behavior.
- Example baseline, artifact, run, or judgment names refer to documentation maintenance.
- One language keeps a different scenario or owner boundary after paired updates.

Fix:
- Replace the example with a durable product or user scenario that stands on its own inside the method owner document.
- Shrink method examples to the minimum request, local preconditions, and representative response needed for that method.
- Document cross-method or end-to-end concerns as scenario-level criteria in conformance or another scenario-level owner when they are needed outside method examples.
- Keep conceptual conformance links when useful, but replace copied scenario payloads with method-local preconditions and distinct refs.
- Keep file paths only when the document is explicitly about documentation maintenance.
- Update paired English and Korean examples by meaning unit.

## CHK-EXAMPLE-002: Korean scenario wording

Check sources:
- [Korean Translation Guide](../../../ko/maintain/translation-guide.md)
- [Authoring Guide](../authoring-guide.md)

Evidence to inspect:
- Confirm Korean examples use natural Korean wording for method-local product or user scenarios.
- Confirm Korean examples avoid compressed noun chains and do not preserve English noun order when a natural Korean phrase is clearer.
- Confirm repeated Korean nouns, paths, and refs inside one method example stay consistent without making the example depend on another method document.

Failure:
- A Korean method-local example uses compressed noun chains or preserved English noun order that makes the scenario harder to read.
- Korean wording blurs whether a ref, path, or state fact is local to the method example or imported from another method document.
- Paired English and Korean examples differ in scenario details, preconditions, or owner boundaries.

Fix:
- Rewrite the Korean wording as natural Korean while preserving exact identifiers, paths, schema fields, enum values, status values, and method names.
- Keep repeated wording consistent inside the method example.
- Update paired English and Korean examples by meaning unit.

## CHK-EXAMPLE-003: API example internal consistency

Check sources:
- [Authoring Guide](../authoring-guide.md)
- The affected API method owner document
- [API Schema Core](../../reference/api/schema-core.md)
- [API State Schemas](../../reference/api/schema-state.md)
- [API Artifact Schemas](../../reference/api/schema-artifacts.md)
- [API Judgment Schemas](../../reference/api/schema-judgment.md)
- [Storage Effects](../../reference/storage-effects.md), when the example claims a storage effect

Evidence to inspect:
- Audit the example against the applicable schema owners before accepting field names, required fields, nullable fields, response branches, refs, timestamps, and value classes.
- Confirm field names, field presence, value meaning, schema shape, value-set routing, and method-local rules come from owner documents, not from inference across examples.
- Example refs are introduced inside the method document or explicitly described as method-local existing refs.
- The example includes enough local context to explain its request payload, visible response state, `state_version`, refs, artifact lifecycle, judgment context, and close-readiness example evidence.
- `expected_state_version`, `base.state_version`, state snapshots, and `Write Authorization` basis values follow the method owner and storage-version owner.
- A response snapshot does not include refs from a newer `state_version` than the example's stated response or local preconditions allow.
- Sensitive approval reasons match the request's `sensitive_categories` or stated precondition.
- Artifact refs do not appear without local staging, promotion, or existing-artifact context.
- `ArtifactInput` examples populate exactly one source field and leave the other source field `null`.
- Judgment refs, options, `affected_refs`, and resolution data match the judgment schemas and method owner behavior.
- Close-readiness blockers cite only locally introduced or locally pre-existing run, judgment, artifact, or state refs.
- Expiration timestamps use placeholders or dates that are later than the example's stated issue date.
- Representative responses do not silently drop meaningful request fields unless labeled as abbreviated.
- Storage-effect claims match the method owner and storage-effect owner.

Failure:
- An example cannot be reviewed as documentation without assuming a hidden fixture, unstated sample task, or external scenario.
- The example uses field names, value meanings, schema shape, value-set routing, or method-local rules that cannot be traced to the owner documents.
- Status examples include newer-version supporting refs.
- `expected_state_version`, `base.state_version`, or `Write Authorization` basis values conflict with the example's local preconditions.
- Approval reasons do not match `sensitive_categories`.
- Artifact refs appear without lifecycle context.
- An `ArtifactInput` has both source fields populated, neither source field populated, or a source field that conflicts with `source_kind`.
- Judgment refs or response fields appear without the owner-defined judgment context.
- Close-readiness evidence refers to missing run, judgment, artifact, or state refs.
- Staged handles have stale fixed expiration timestamps.
- Response examples drop `options` or `affected_refs` from a user-judgment request without saying the response is abbreviated.
- A storage-effect statement contradicts the method owner or storage-effect owner.

Fix:
- Align refs, versions, sensitive categories, artifact lifecycle, judgment context, blocker context, timestamps, response snapshots, and storage-effect claims with the applicable owners.
- Align field names, field presence, value meanings, schema shape, value-set routing, and method-local rules with the applicable owners.
- Add concise method-local preconditions when a ref or state is intentionally pre-existing.
- Remove response fields or storage-effect claims that the owner documents do not support.

## CHK-EXAMPLE-004: field-name, required-field, and nullability consistency

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [API Schema Core](../../reference/api/schema-core.md)
- The affected method, schema, or storage owner document

Evidence to inspect:
- Example field names match the owner method or schema document.
- Required fields in the owner shape are present in examples unless the example is explicitly labeled as abbreviated.
- Fields documented as `Type | null` are present and may be `null`; they are not silently omitted when the owner shape requires presence.
- Array fields use arrays, object fields use objects, and nullable fields do not use sentinel strings such as `"none"` unless the value-set owner supports that string.
- `NextActionSummary` examples use the owner shape from [API State Schemas](../../reference/api/schema-state.md): `action_kind`, `owner_method`, `label`, `blocking_question`, and `required_refs`.
- A storage/effect example that reuses method payload data does not use a different field name unless it is explicitly described as a storage-owned summary field.

Failure:
- A method example uses `intended_paths`, while a related storage example uses `affected_paths` for the same concept without explanation.
- A field name appears in an example but is not owned by the relevant method, schema, or storage summary section.
- Required owner fields such as `base`, `response_kind`, `effect_kind`, or schema-owned child fields are missing from a non-abbreviated response.
- A nullable required field is omitted instead of shown with `null`.
- A `NextActionSummary` example uses obsolete fields, omits required fields, or adds fields not owned by the state schema.
- Two related examples use different field names for the same concept without an owner-boundary note.

Fix:
- Use the owner method/schema field name, or clearly mark the field as storage-owned summary data.
- Restore required fields and represent nullable fields according to the owner notation.
- Update `NextActionSummary` examples to the state-schema shape.
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
- Confirm examples do not redefine API methods, schema names, fields, values, error codes, storage effects, or response branches outside the appropriate owner.

Failure:
- A link points to a broad index when the reader needs a precise contract owner.
- A broad API family page is treated as the owner for split API concerns.
- A non-owner page reproduces request/response structure, response branches, error behavior, schema fields, enum-like values, method semantics, or storage effects as if it owns them.

Fix:
- Retarget the link to the exact owner selected from the Reference Index, API Methods router, or `doc-index.yaml`.
- Replace duplicated contract text with a short reader consequence and owner link.

## CHK-EXAMPLE-006: enum-like value and response branch consistency

Check sources:
- [API Value Sets](../../reference/api/schema-value-sets.md)
- [API Schema Core](../../reference/api/schema-core.md)
- [API State Schemas](../../reference/api/schema-state.md)
- The affected method owner document
- [API error routing](../../reference/api/error-routing.md), when the example shows rejected or blocked branches

Evidence to inspect:
- Classify every string-like example field with the applicable owner before checking the literal value.
- Controlled value strings must use supported values from the applicable value-set owner.
- Opaque identifiers and opaque classification strings may appear in examples only as carried identifiers or local classifications; do not treat them as global exhaustive value sets.
- Free-form display strings may show human-facing text; do not treat them as canonical schema values, error codes, blocker codes, storage identifiers, or value-set entries.
- Audit value-like example fields against the applicable value-set owner before accepting them as current example values.
- Enum-like values in examples appear in the value-set owner unless the field is explicitly free-form text, an opaque identifier, or an opaque classification string.
- Method names, `response_kind`, `effect_kind`, `access_class`, `record_kind`, lifecycle values, close-state values, artifact `source_kind`, judgment values, `presentation` values, `required_for`, `UserJudgment.status`, `CloseReadinessBlocker.category`, `ValidatorResult.status`, `ValidatorResult.severity`, and `GuaranteeDisplay.level` match their value-set owners.
- Rendered labels are not used as canonical schema values.
- Each response example uses exactly one branch: method-specific result, `ToolRejectedResponse`, or `ToolDryRunResponse` when the method owner defines a dry-run preview.
- Rejected and dry-run examples do not carry method-result-only fields such as `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, `decision`, or `close_state`.
- Method result examples carry `base: ToolResultBase` and only fields owned by that method result branch.
- `response_kind`, `effect_kind`, and response body shape agree with the method owner and common branch owner.

Failure:
- An unsupported enum-like value appears in an example for a field that is not explicitly free-form or opaque.
- A controlled value string is accepted without matching the applicable value-set owner.
- An opaque identifier or opaque classification string is treated as a supported global value.
- A free-form display string is used as a canonical schema value, error code, blocker code, storage identifier, or value-set entry.
- A localized display label is used as a canonical value.
- A stale response shape mixes method result fields into `ToolRejectedResponse` or `ToolDryRunResponse`.
- A stale response shape omits `base`, uses obsolete branch fields, or contradicts the method owner's result branch.
- A blocker, validator, guarantee-display, artifact, or judgment value is not listed by the applicable value-set owner.

Fix:
- Replace unsupported values with supported value-set entries, or route the field to the owner that explicitly defines it as free-form or opaque.
- When a string is opaque or free-form, make the example or nearby note follow that owner-defined class instead of inventing a value set.
- Update stale response shapes to the common branch owner and method owner.
- Remove branch fields that belong to a different response branch.

## CHK-EXAMPLE-007: cross-method scenario spine detection

Check sources:
- [Authoring Guide](../authoring-guide.md)
- [Conformance Reference](../../reference/conformance.md)
- [API Methods](../../reference/api/methods.md)
- The affected API method owner documents and their paired Korean documents

Evidence to inspect:
- Inspect method owner examples as a set, not only one file at a time.
- Treat a repeated business noun, path prefix, ref prefix, judgment ID family, run ID family, artifact ID family, blocker code family, or scenario description across three or more method owner documents as a cross-method scenario spine.
- Confirm method examples do not refer to previous or later method examples for setup, refs, paths, `state_version`, artifact refs, run refs, judgment refs, blocker refs, or response snapshots.
- Confirm cross-method or end-to-end scenarios, when needed, appear as scenario-level criteria in conformance or another scenario-level owner, not as a method-reference payload spine.
- Confirm conformance scenarios do not require method reference documents to reuse conformance payloads.
- Confirm method examples may link to conformance only conceptually and do not copy conformance payloads for consistency.

Failure:
- Three or more method owner documents repeat the same scenario identity through business nouns, paths, ref prefixes, ID families, blocker code families, or scenario description.
- A method example relies on another method reference document to introduce setup or refs.
- A conformance or scenario-level document requires method reference examples to reuse its payloads.
- Method examples depend on a scenario that spans multiple method documents.

Fix:
- Rewrite affected method examples into independent method-local examples with local preconditions and distinct refs.
- Document cross-method or end-to-end concerns as scenario-level criteria in conformance or another scenario-level owner.
- Remove wording that permits cross-method dependency or conformance-payload reuse in method reference examples.
