# Implementation guide

This guide gives implementers a stable route from Harness product scope to the owner documents that define runtime behavior.

It does not define product scope, API behavior, schemas, storage effects, security guarantees, or connector behavior. Those contracts stay in the Reference owners.

## Owner-first rule

Build from owner documents, not from route summaries.

Use this order when planning an implementation slice:

1. Confirm the active capability boundary in [Scope](../reference/scope.md).
2. Find exact contract owners through the [Reference Index](../reference/README.md).
3. Use [API Methods](../reference/api/methods.md) for the public method list and method-owner routing.
4. Use the method owner, schema owner, storage owner, and security owner together for behavior that crosses boundaries.
5. Keep user-owned judgment, evidence, verification expectations, acceptance, close readiness, and residual risk as separate authority concepts.

## First runtime slice

The smallest useful Harness runtime slice carries one ordinary user task through:

- plain-language intake
- active scope confirmation
- status review
- write compatibility checking
- artifact staging when needed
- run and evidence recording
- user-owned judgment when required
- close-readiness evaluation

The slice should use current contracts only. Out-of-scope or reserved capabilities become implementation inputs only after [Scope](../reference/scope.md) and the affected owner documents describe them as active.

## Contract routes

| Need | Owner |
|---|---|
| Current scope and exclusions | [Scope](../reference/scope.md) |
| Public method list and routing | [API Methods](../reference/api/methods.md) |
| Method behavior | Method owners listed by [API Methods](../reference/api/methods.md) |
| Common API envelopes and response branches | [API Schema Core](../reference/api/schema-core.md) |
| State and close-readiness schemas | [API State Schemas](../reference/api/schema-state.md) |
| Artifact schemas | [API Artifact Schemas](../reference/api/schema-artifacts.md) |
| User-owned judgment schemas | [API Judgment Schemas](../reference/api/schema-judgment.md) |
| API value sets | [API Value Sets](../reference/api/schema-value-sets.md) |
| Public error routing | [API Errors](../reference/api/errors.md) |
| Storage effects | [Storage Effects](../reference/storage-effects.md) |
| Storage records | [Storage Records](../reference/storage-records.md) |
| Artifact lifecycle | [Artifact Storage](../reference/storage-artifacts.md) |
| State versioning and migrations | [Storage Versioning](../reference/storage-versioning.md) |
| Runtime and repository boundaries | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| Security claims and non-claims | [Security](../reference/security.md) |
| Surface and connector behavior | [Agent Integration](../reference/agent-integration.md), [Surface Recipes](../use/surface-recipes.md) |
| Projection authority and template bodies | [Projection Authority](../reference/projection-and-templates.md), [Template Bodies](../reference/template-bodies.md) |

## Build discipline

Implementation tasks should name the owner documents they depend on. If a slice needs behavior that no owner defines, update the owner first instead of encoding an inferred rule.

Runtime state, generated artifacts, evidence records, QA records, acceptance records, close records, residual-risk records, fixture outputs, and product implementation files are not stored in this documentation tree.

Path allowlists, route tables, and batch boundaries in these docs are editing controls for the documentation set. They are not Harness runtime permissions, write authorizations, sandbox guarantees, or proof of enforcement.

## Example smoke path

A representative first smoke path can use the API example scenario:

- Start a task to add explicit confirmation before account data export.
- Confirm account deletion behavior remains out of scope.
- Prepare a compatible product-file write.
- Stage or link representative test output as an artifact when the artifact owners allow it.
- Record a run and compact evidence summary.
- Ask for required user-owned judgment.
- Evaluate close readiness through `harness.close_task`.

The smoke path is a planning shape. Exact requests, responses, storage effects, error behavior, and close-readiness blockers belong to the relevant Reference owners.
