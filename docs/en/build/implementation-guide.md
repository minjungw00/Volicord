# Implementation guide

This guide is a reading path for implementers. It connects implementation questions to the Reference owners that define the contracts.

It does not define or override baseline scope, API behavior, schemas, storage effects, security guarantees, runtime boundaries, error behavior, close-readiness rules, connector behavior, conformance authority, or example validity. Those contracts stay in the Reference owners.

This guide may describe implementation reading paths, owner-document interpretation for implementation questions, and guide-level supported implementation shape. Decisions that change API behavior, storage effects, security guarantees, scope boundaries, schema shapes, error behavior, or Core authority semantics belong in the relevant Reference owner, not here.

Harness is the local work-authority product/system for AI-assisted product work. Core is the local authority record for Harness state.

## Baseline implementation reading path

Use this path when turning product scope into implementation work:

1. Confirm the baseline scope and supported behavior boundary in [Scope](../reference/scope.md).
2. Read [Implementation Architecture](architecture.md) for guide-level layer separation and the planned Rust workspace shape.
3. Use the [Reference Index](../reference/README.md) and [`docs/doc-index.yaml`](../../doc-index.yaml) to choose the applicable owner for each contract question.
4. Read [Core Model](../reference/core-model.md) for authority concepts that cross APIs, storage, current scope, user-owned judgment, and close readiness.
5. Use [API Methods](../reference/api/methods.md) for the supported public method list and method-owner routing.
6. Add the focused schema, storage, runtime, security, error, and conformance owners only when the implementation question touches those concerns.
7. Use [Agent Integration](../reference/agent-integration.md) for connector boundaries and [Surface Recipes](../use/surface-recipes.md) for surface-specific usage workflows.
8. Keep user-owned judgment, evidence, verification criteria, ordinary approval, write approval, sensitive-action approval, `Write Authorization`, final acceptance, close readiness, and residual-risk acceptance as distinct concepts. Core Model owns those distinctions.

## Owner route shortcuts

Use these as first-hop routes. Each route is a first stop; the focused owner carries the behavior.

| Concern | First-hop owner route |
|---|---|
| Baseline scope and out-of-scope boundaries | [Scope](../reference/scope.md) |
| API behavior and supported public methods | [API Methods](../reference/api/methods.md), then the method owner listed there |
| Schema shapes and value names | [API Schema Core](../reference/api/schema-core.md), [API State Schemas](../reference/api/schema-state.md), [API Artifact Schemas](../reference/api/schema-artifacts.md), [API Judgment Schemas](../reference/api/schema-judgment.md), and [API Value Sets](../reference/api/schema-value-sets.md) |
| Storage effects, records, artifacts, and versions | [Storage Effects](../reference/storage-effects.md), then [Storage](../reference/storage.md), [Storage Records](../reference/storage-records.md), [Artifact Storage](../reference/storage-artifacts.md), or [Storage Versioning](../reference/storage-versioning.md) |
| Runtime and file-location boundaries | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| Security boundaries and guarantee wording | [Security](../reference/security.md), with Scope for supported availability and API Value Sets for exact names |
| Error behavior and blocker routing | [API Error Family Index](../reference/api/errors.md), then [API Error Codes](../reference/api/error-codes.md), [API Error Precedence](../reference/api/error-precedence.md), [API Error Routing](../reference/api/error-routing.md), [API Blocker Routing](../reference/api/blocker-routing.md), or [API Error Details](../reference/api/error-details.md) |
| Conformance assertion authority | [Conformance](../reference/conformance.md), then the owner of each asserted fact |
| Connector boundaries and surface workflows | [Agent Integration](../reference/agent-integration.md) and [Surface Recipes](../use/surface-recipes.md) |
| Read-only display and templates | [Projection Authority](../reference/projection-and-templates.md) and [Template Bodies](../reference/template-bodies.md) |

If owners appear to disagree, the mismatch is an owner gap. Scope gates supported availability; focused owners define method behavior, schema shapes, storage effects, runtime locations, security wording, public error meanings, blocker routing, and assertion authority.

## Scope interpretation

A capability is ready for baseline implementation only when Scope includes it and the affected owners define the behavior, shape, storage effect, runtime boundary, security boundary, error behavior, and conformance basis the work needs.

Names in value sets, examples, conformance scenario IDs, route summaries, or schema vocabulary are not enough by themselves. Treat them as vocabulary or reserved surface area until Scope and the relevant owners define support.

## Use documents and reference contracts

Use documents explain workflows, reader decisions, and expected outcomes. They are useful for understanding how a user or agent should move through Harness, but they do not override Reference contracts.

Implementers may use [User Guide](../use/user-guide.md), [Agent Guide](../use/agent-guide.md), [Judgment Examples](../use/judgment-examples.md), and [Surface Recipes](../use/surface-recipes.md) to understand reader intent, surface workflow expectations, and judgment boundaries. For API payloads, storage effects, security guarantees, close-readiness rules, access boundaries, or error behavior, route back to the applicable Reference owner.

If a use document and a Reference owner seem to differ, the Reference owner is the authority. The route or guide mismatch is a documentation maintenance issue.

## Out-of-scope behavior

A named capability can remain outside baseline support. Scope and the affected owners decide when that capability becomes supported baseline behavior.

Until the affected owner documents define support, this guide only routes the question back to [Scope](../reference/scope.md) and those owners. Detailed handling is defined there, not in this guide.

## Conformance scenarios

[Conformance](../reference/conformance.md) owns documentation-level conformance meaning, assertion authority, and compact scenario routing. Scenarios are coverage prompts only; tests and checks get authority from owner-defined facts.

Scenario prose, generated summaries, rendered reports, documentation-check labels, and status display text are not runtime authority.

## Examples as implementation inputs

Examples are reading aids, not complete schemas or behavior sources. Use them to understand a representative branch, scenario, or compact request/response shape.

Fields, optionality, storage effects, security guarantees, out-of-scope behavior, and implementation shortcuts come from owners, not examples. If an example conflicts with a method, schema, storage, security, runtime, conformance, or error owner, the relevant owner is authoritative.

## Small baseline build shape

A small baseline build can stay narrow: one ordinary user task, [Scope](../reference/scope.md) for included capabilities, and the relevant owners for requests, responses, storage effects, errors, blockers, security wording, and conformance assertions.

This is an implementation shape, not a separate contract.

## Repository boundary

Runtime state, generated artifacts, evidence outputs, QA results, acceptance decisions, close-readiness state, residual-risk decisions, fixture outputs, and product implementation files are not stored in this documentation tree.

Implementation logs, PR notes, transient migration records, and one-off decision records do not belong in maintained documentation.

Path allowlists, route tables, and documentation batch boundaries in these docs are maintainer editing controls for the documentation set. They are not Harness runtime permissions, `Write Authorization`, sandbox guarantees, or proof of enforcement.
