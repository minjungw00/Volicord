# Build: MVP plan

This page is the Build handoff for planning the first Harness Server implementation batch. It records readiness posture, assumptions, sequencing, smoke-target intent, and exit criteria.

It does not define:

- canonical product scope
- API behavior
- schemas
- storage effects
- security guarantees

<a id="documentation-acceptance-status"></a>
## Repository status

Maintainer handoff status: **not accepted for server coding**.

This repository is still documentation-only source material for a future Harness Server. It is not a Harness Server implementation, Product Repository, Harness Runtime Home, runtime record store, generated projection store, evidence store, QA record, acceptance record, or close record.

For the canonical current scope, see [Active MVP scope](../reference/active-mvp-scope.md). Runtime location boundaries are owned by [Runtime Boundaries](../reference/runtime-boundaries.md).

The active documentation set has paired English and Korean Start, Use, Build, Reference, Later, and Maintain routes. Canonical contracts live in Reference owners; this Build plan only explains how implementation planning should proceed once maintainers are ready to start a server build.

Server coding must not begin from this repository until the decisions in [Decisions before server coding](#decisions-before-server-coding) have been accepted, resolved, or explicitly deferred with named scope impact.

## Planning assumptions

- The current task is implementation planning, not runtime implementation.
- Active MVP scope is owned by [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md); this plan does not repeat the scope list.
- API method behavior is owned by [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md); this plan does not repeat request, response, branch, or error behavior.
- Common API envelopes and response branches are owned by [`../reference/api/schema-core.md`](../reference/api/schema-core.md). State, artifact, judgment, and value-set schemas are owned by their split API schema references.
- Storage effects are owned by [`../reference/storage-effects.md`](../reference/storage-effects.md); this plan does not define tables, migrations, artifact lifecycle, or state effects.
- Security claims are owned by [`../reference/security.md`](../reference/security.md), and runtime-home/access boundaries are owned by [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md).
- Later candidates remain outside the current MVP unless maintainers promote them through the appropriate owner documents.

## Implementation sequence

Use this sequence for the first implementation plan after maintainer handoff:

1. Confirm the current MVP boundary in [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md).
2. Choose the smallest server slice that can exercise one ordinary user work loop without relying on later candidates.
3. Map each planned server surface to its Reference owner before designing code structure.
4. Implement contract-neutral scaffolding only after the API, schema, storage, security, and runtime-boundary owners are accepted for that slice.
5. Add durable storage behavior only from [`../reference/storage-effects.md`](../reference/storage-effects.md).
6. Add API/tool behavior only from [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md) and the relevant split API schema owners.
7. Add status/display behavior as derived reads of owner-defined state, not as independent authority.
8. Keep acceptance, residual-risk acceptance, evidence, verification, and close readiness distinct in implementation tasks.

This sequence is intentionally small. If a step needs contract detail, update or accept the owning Reference document instead of expanding this Build plan.

<a id="first-internal-smoke-target"></a>
## First internal smoke target

The first internal smoke target should check that the first server slice can carry one ordinary task from intake to close-readiness evaluation using only accepted current MVP contracts.

The smoke target should cover these planning checkpoints:

- task intake or resume from ordinary user language
- current MVP scope classification through the scope owner
- user-owned judgment kept separate from Core-owned state
- evidence and verification expectations referenced without fabricating records
- close readiness reported from the schema/API owners
- storage writes limited to accepted storage effects
- status output shown as derived, freshness-aware information
- unavailable or unsupported authority reported plainly

This target is not a conformance suite, not a fixture specification, and not proof that Harness is implemented. Exact examples, method calls, schemas, storage behavior, and error behavior belong to the Reference owners.

<a id="implementation-decisions-before-server-coding"></a>
## Decisions before server coding

Maintainers must record one of these outcomes for each item before implementation begins: accepted for the first server slice, blocked with named impact, or deferred with named impact.

| Decision item | Required outcome before coding |
|---|---|
| Build handoff | Maintainers confirm this page is the active Build entry point for implementation planning. |
| Current MVP scope | Maintainers accept the boundary in [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md), or name the unresolved scope impact. |
| API and schemas | Maintainers accept the relevant slice of [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md) and the needed API schema owners. |
| Storage effects | Maintainers accept the relevant slice of [`../reference/storage-effects.md`](../reference/storage-effects.md) before any runtime storage files, DDL, or artifact storage are created. |
| Security and runtime boundaries | Maintainers accept the relevant claims and non-claims in [`../reference/security.md`](../reference/security.md) and [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md). |
| Smoke target | Maintainers accept the first internal smoke target as an implementation-planning target, not as a conformance claim. |
| Deferred material | Maintainers confirm no later candidate is required for the first server slice unless it has been promoted by its owner. |

## Documentation-only boundary

Edits in this repository do not create runtime behavior. Do not add server code, runtime state, generated operational files, generated projections, evidence records, QA records, acceptance records, close records, residual-risk records, executable fixtures, or conformance runner output.

Path allowlists, batch boundaries, owner links, and planning sequence are documentation-maintenance controls. They are not Harness runtime permissions, write authorizations, sandbox guarantees, or proof of enforcement.

Passing this plan's exit criteria only means the documentation is ready to guide a future implementation batch. It does not implement Harness, prove runtime conformance, or authorize product-repository writes.

## Reference owners

Use these owners instead of repeating contracts in this Build plan:

| Topic | Owner |
|---|---|
| Current MVP scope | [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md) |
| API method behavior | [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md) |
| Common API envelopes and response branches | [`../reference/api/schema-core.md`](../reference/api/schema-core.md) |
| State schemas and close-readiness structures | [`../reference/api/schema-state.md`](../reference/api/schema-state.md) |
| Artifact schemas | [`../reference/api/schema-artifacts.md`](../reference/api/schema-artifacts.md) |
| User-owned judgment schemas | [`../reference/api/schema-judgment.md`](../reference/api/schema-judgment.md) |
| API value sets | [`../reference/api/schema-value-sets.md`](../reference/api/schema-value-sets.md) |
| Storage effects | [`../reference/storage-effects.md`](../reference/storage-effects.md) |
| Security guarantees and non-claims | [`../reference/security.md`](../reference/security.md) |
| Runtime-home and access boundaries | [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md) |

For neighboring Reference pages and navigation, use [`../reference/README.md`](../reference/README.md).

## Exit criteria

Implementation planning can exit only when:

- maintainers mark this Build plan accepted as the implementation-planning entry point
- each item in [Decisions before server coding](#decisions-before-server-coding) has an accepted, blocked, or deferred outcome with named impact
- the first server slice can be described using owner links instead of duplicated contract text
- English and Korean Build pages preserve the same reader purpose, owner routing, and handoff status
- no later candidate is presented as a current MVP requirement
- no temporary planning files, generated runtime records, executable fixtures, conformance results, or product implementation outputs remain in this repository

After these criteria are met, the next step is a maintainer-approved implementation batch outside this documentation-only edit. Until then, the repository remains planning material.
