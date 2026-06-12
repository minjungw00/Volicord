# Build: MVP plan

This page is the Build handoff for planning the first Harness Server implementation batch.

It records:

- readiness posture
- assumptions
- sequencing
- smoke-target intent
- exit criteria

It does not define:

- canonical product scope
- API behavior
- schemas
- storage effects
- security guarantees

<a id="documentation-acceptance-status"></a>
## Repository status

Maintainer handoff status: **not accepted for server coding**.

This repository is still documentation-only source material for a future Harness Server.

It is not:

- a Harness Server implementation
- a `Product Repository`
- a `Harness Runtime Home`
- a runtime record store
- a generated projection store
- an evidence store
- a QA record
- an acceptance record
- a close record

For the canonical current scope, see [Active MVP scope](../reference/active-mvp-scope.md). Runtime location boundaries are owned by [Runtime Boundaries](../reference/runtime-boundaries.md).

The active documentation set has paired English and Korean Start, Use, Build, Reference, Later, and Maintain routes.

This Build plan should:

- route planning, status, and handoff decisions
- leave canonical contracts in Reference owners
- explain implementation planning only after maintainers are ready to start a server build

Server coding must not begin from this repository until every item in [Decisions before server coding](#decisions-before-server-coding) has one of these outcomes:

- accepted
- resolved
- explicitly deferred with named scope impact

## Planning assumptions

- This Build page supports implementation planning, not runtime implementation.
- Active MVP scope is owned by [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md); this plan does not repeat the scope list.
- API owners:
  - method routing: [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md)
  - method behavior: the method owner documents listed there
  - common API envelopes and response branches: [`../reference/api/schema-core.md`](../reference/api/schema-core.md)
  - state, artifact, judgment, and value-set schemas: their API schema references
  - this plan does not repeat request, response, branch, or error behavior
- Storage owners:
  - storage effects: [`../reference/storage-effects.md`](../reference/storage-effects.md)
  - this plan does not define tables, migrations, artifact lifecycle, or state effects
- Security and runtime-boundary owners:
  - security claims: [`../reference/security.md`](../reference/security.md)
  - runtime-home and access boundaries: [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md)
- Later candidates remain outside the current MVP unless maintainers promote them through the appropriate owner documents.

## Implementation sequence

Use this sequence for the first implementation plan after maintainer handoff:

1. Confirm the current MVP boundary in [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md).
2. Choose the smallest server slice that can exercise one ordinary user work loop without relying on later candidates.
3. Map each planned server surface to its Reference owner before designing code structure.
4. Implement contract-neutral scaffolding only after the API, schema, storage, security, and runtime-boundary owners are accepted for that slice.
5. Add durable storage behavior only from [`../reference/storage-effects.md`](../reference/storage-effects.md).
6. Add API/tool behavior only from [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md), the relevant method owner documents, and the relevant API schema owners.
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

Maintainers must record one outcome for each item before implementation begins:

- accepted for the first server slice
- blocked with named impact
- deferred with named impact

Decision items:

- Build handoff:
  - Maintainers confirm this page is the active Build entry point for implementation planning.
- Current MVP scope:
  - Maintainers accept the boundary in [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md), or name the unresolved scope impact.
- API and schemas:
  - Maintainers accept the relevant slice of [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md).
  - Maintainers also accept the affected method owner documents and needed API schema owners.
- Storage effects:
  - Maintainers accept the relevant slice of [`../reference/storage-effects.md`](../reference/storage-effects.md).
  - This happens before any runtime storage files, DDL, or artifact storage are created.
- Security and runtime boundaries:
  - Maintainers accept the relevant claims and non-claims in [`../reference/security.md`](../reference/security.md).
  - Maintainers also accept the relevant boundaries in [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md).
- Smoke target:
  - Maintainers accept the first internal smoke target as an implementation-planning target, not as a conformance claim.
- Deferred material:
  - Maintainers confirm no later candidate is required for the first server slice unless it has been promoted by its owner.

## Documentation-only boundary

Edits in this repository do not create runtime behavior.

Do not add:

- server code
- runtime state
- generated operational files
- generated projections
- evidence records
- QA records
- acceptance records
- close records
- residual-risk records
- executable fixtures
- conformance runner output

Path allowlists, batch boundaries, owner links, and planning sequence are documentation-maintenance controls.

They are not:

- Harness runtime permissions
- write authorizations
- sandbox guarantees
- proof of enforcement

Passing this plan's exit criteria only means the documentation is ready to guide a future implementation batch. It does not implement Harness, prove runtime conformance, or authorize `Product Repository` writes.

## Reference owners

Use these owner routes instead of repeating contracts in this Build plan:

- Scope:
  - current MVP scope: [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md)
- API:
  - method routing: [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md)
  - method behavior: the method owner documents listed by the API router
  - common envelopes and response branches: [`../reference/api/schema-core.md`](../reference/api/schema-core.md)
  - state and close-readiness schemas: [`../reference/api/schema-state.md`](../reference/api/schema-state.md)
  - artifact schemas: [`../reference/api/schema-artifacts.md`](../reference/api/schema-artifacts.md)
  - user-owned judgment schemas: [`../reference/api/schema-judgment.md`](../reference/api/schema-judgment.md)
  - API value sets: [`../reference/api/schema-value-sets.md`](../reference/api/schema-value-sets.md)
- Storage:
  - storage effects: [`../reference/storage-effects.md`](../reference/storage-effects.md)
- Security and runtime boundaries:
  - security guarantees and non-claims: [`../reference/security.md`](../reference/security.md)
  - runtime-home and access boundaries: [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md)

For neighboring Reference pages and navigation, use [`../reference/README.md`](../reference/README.md).

## Exit criteria

Implementation planning can exit only when:

- maintainers mark this Build plan accepted as the implementation-planning entry point
- each item in [Decisions before server coding](#decisions-before-server-coding) has an accepted, blocked, or deferred outcome with named impact
- the first server slice can be described using owner links instead of duplicated contract text
- English and Korean Build pages preserve the same reader purpose, owner routing, and handoff status
- no later candidate is presented as a current MVP requirement
- this repository contains no:
  - temporary planning files
  - generated runtime records
  - executable fixtures
  - conformance results
  - product implementation outputs

After these criteria are met, the next step is a maintainer-approved implementation batch outside the documentation-only repository. Until then, the repository remains planning material.
