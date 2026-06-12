# Implementation guide

This guide gives implementers a stable route from Harness product scope to the owner documents that define runtime behavior. It is an implementation reading guide, not a product contract.

This guide does not define baseline scope, API behavior, schemas, storage effects, security guarantees, runtime locations, connector behavior, conformance authority, or example validity. Those contracts stay in the Reference owners.

## Implementer reading path

Read owner documents before encoding behavior.

Use this path for an implementation slice:

1. Confirm the active capability boundary in [Scope](../reference/scope.md).
2. Use the [Reference Index](../reference/README.md) to choose the exact owner for each contract question.
3. Read [Core Model](../reference/core-model.md) for authority concepts that cross APIs, storage, and close readiness.
4. Use [API Methods](../reference/api/methods.md) for the active public method list and method-owner routing.
5. Read the affected method owner, schema owner, storage owner, [Runtime Boundaries](../reference/runtime-boundaries.md), [Security](../reference/security.md), and [Conformance](../reference/conformance.md) together when behavior crosses those areas.
6. Use [Agent Integration](../reference/agent-integration.md) and [Surface Recipes](../use/surface-recipes.md) only for the surface or connector boundary they own.
7. Keep user-owned judgment, evidence, verification expectations, acceptance, close readiness, and residual risk as separate authority concepts.

## Baseline scope interpretation

[Scope](../reference/scope.md) is the baseline gate. A capability is implementable as baseline behavior only when Scope includes the capability family and the affected owners define the behavior, data shapes, storage consequences, runtime boundary, security wording, and conformance expectations that the implementation needs.

Treat Scope as a boundary document, not as a full design specification. Scope can say a capability family is included or excluded; exact method behavior, field shapes, storage effects, security guarantee level, and conformance assertions belong to the narrower owners.

Value names, examples, route summaries, and schema vocabulary do not activate behavior by themselves. Reserved values and profile-gated values remain non-baseline until Scope and the semantic owner both define them as active baseline behavior.

## Contract owner combinations

Most implementation work needs more than one owner. Start from the owner closest to the question, then add the neighboring owners that define the shape, effect, or guarantee.

| Implementation question | Read together |
|---|---|
| Is this capability in baseline scope? | [Scope](../reference/scope.md), then the affected semantic owner from the [Reference Index](../reference/README.md) |
| Which public method exists? | [API Methods](../reference/api/methods.md) and [API Value Sets](../reference/api/schema-value-sets.md) |
| What does one method do? | The method owner listed by [API Methods](../reference/api/methods.md), plus [API Schema Core](../reference/api/schema-core.md) for shared envelopes and response branches |
| Which fields and nested shapes are valid? | [API Schema Core](../reference/api/schema-core.md), [API State Schemas](../reference/api/schema-state.md), [API Artifact Schemas](../reference/api/schema-artifacts.md), [API Judgment Schemas](../reference/api/schema-judgment.md), and [API Value Sets](../reference/api/schema-value-sets.md) as applicable |
| Which public errors or close-readiness blocker routes are valid? | [API Errors](../reference/api/errors.md), plus the affected method and state-schema owners |
| What changes in storage? | [Storage Effects](../reference/storage-effects.md) first, then [Storage Records](../reference/storage-records.md), [Artifact Storage](../reference/storage-artifacts.md), or [Storage Versioning](../reference/storage-versioning.md) when the effect needs record, artifact, clock, lock, or migration detail |
| Where do product files, server files, and runtime data live? | [Runtime Boundaries](../reference/runtime-boundaries.md), with storage owners for data detail |
| What security wording or guarantee level is valid? | [Security](../reference/security.md), with [Scope](../reference/scope.md) for active availability and [API Value Sets](../reference/api/schema-value-sets.md) for exact value names |
| What should a conformance check assert? | [Conformance](../reference/conformance.md), then the API, schema, storage, security, runtime, and Core owners that make each asserted fact authoritative |
| How should a surface or connector behave? | [Agent Integration](../reference/agent-integration.md), [Surface Recipes](../use/surface-recipes.md), and the relevant API/security owners |
| What can a read-only display show? | [Projection Authority](../reference/projection-and-templates.md), [Template Bodies](../reference/template-bodies.md), and the state/schema owners for source facts |

When owners appear to disagree, do not resolve the mismatch in implementation code. Treat it as an owner gap: Scope gates active availability, method owners define method behavior, schema owners define shapes, storage owners define effects, Runtime Boundaries define locations, Security defines guarantee wording, and Conformance defines assertion authority.

## Use documents and reference contracts

Use documents explain workflows, reader decisions, and expected outcomes. They are useful for understanding how a user or agent should move through Harness, but they do not override Reference contracts.

Implementers may use [User Guide](../use/user-guide.md), [Agent Guide](../use/agent-guide.md), [Judgment Examples](../use/judgment-examples.md), and [Surface Recipes](../use/surface-recipes.md) to understand reader intent, surface behavior, and judgment boundaries. For API payloads, storage effects, security guarantees, close-readiness rules, access boundaries, or error behavior, route back to the Reference owner.

If a use document and a Reference owner seem to differ, implement the Reference owner and report the route or guide mismatch as documentation maintenance work.

## Out-of-scope behavior

Do not implement an excluded capability because it is named in Scope, examples, conformance scenario IDs, schema value sets, or route summaries. A name may exist as vocabulary, compatibility surface area, or a reserved value without being supported behavior.

An out-of-scope capability becomes implementable only after Scope and the affected owners define a narrow supported contract. That owner set may include method behavior, schemas, storage effects, runtime boundaries, security guarantees, conformance checks, template behavior, and bilingual terminology.

Implementation code should reject, ignore, or avoid out-of-scope behavior according to the active owners. It should not silently add fallback behavior that the owners do not define.

## Conformance scenarios

[Conformance](../reference/conformance.md) explains documentation-level conformance meaning, assertion authority, and compact scenario routing. It does not provide executable fixture files, generated reports, runtime proof, or new API behavior.

Use conformance scenarios as coverage prompts. For each scenario, bind every assertion to an owner-defined fact before writing a test or check. A valid implementation check should compare structured method responses, Core state, storage effects, artifact facts, public error codes, security guarantee display, or required absence of forbidden side effects only when the relevant owner defines that fact.

Do not treat scenario prose, generated summaries, rendered reports, documentation-check labels, or status display text as runtime authority unless a specific owner promotes that fact.

## Examples as implementation inputs

Examples are aids for reading contracts. They can show a representative branch, a durable scenario, or a compact request/response shape, but they are not complete schemas and do not create behavior by themselves.

Use examples to understand:

- how the owner expects a scenario to be read
- which owner documents the example combines
- what a compact representative request or response may look like

Do not use examples to infer:

- fields that the schema owner does not define
- optionality from omitted fields in an abbreviated response
- storage effects not owned by storage documents
- security guarantees not owned by Security
- out-of-scope behavior from a scenario mention
- implementation shortcuts from example values, refs, or timestamps

If an example conflicts with a method, schema, storage, security, runtime, or conformance owner, the owner wins. Fix the example or route text instead of implementing the conflict.

## Minimal baseline slice

The smallest useful baseline slice carries one ordinary user task through the active owner path:

- plain-language intake
- active scope confirmation
- status review
- write compatibility checking
- artifact staging or compatible artifact linking when the artifact owners allow it
- run and evidence recording
- user-owned judgment when required
- close-readiness evaluation

This slice is a stable implementation shape, not a separate contract. Exact requests, responses, storage effects, errors, blockers, security wording, and conformance assertions belong to the relevant Reference owners.

## Repository boundary

Runtime state, generated artifacts, evidence records, QA records, acceptance records, close records, residual-risk records, fixture outputs, and product implementation files are not stored in this documentation tree.

Path allowlists, route tables, and documentation batch boundaries in these docs are maintainer editing controls for the documentation set. They are not Harness runtime permissions, write authorizations, sandbox guarantees, or proof of enforcement.
