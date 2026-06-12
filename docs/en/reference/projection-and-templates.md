# Projection authority reference

This document owns projection authority and read-only derived-display boundaries. Exact rendered body expectations live in [Template Bodies](template-bodies.md).

This is documentation reference material only. It is not a runtime projection, runtime state, generated artifact, evidence record, QA record, final-acceptance record, residual-risk record, close record, or implementation-ready server plan.

## Owns / Does not own

This document owns:

- projection authority boundaries
- read-only derived-display rules
- freshness and unavailable-state wording for projected output
- the rule that rendered labels are display text, not canonical schema values
- the later-only boundary for projection reconcile, persistent projection jobs, and managed block drift repair

This document does not own:

- status card, judgment request, run/evidence summary, close result, or agent context packet bodies; see [Template Bodies](template-bodies.md)
- source-of-truth Core state; see [Core Model](core-model.md)
- storage records or projection storage candidates; see storage owners and [Scope Reference](scope.md)
- public API schemas; see API schema owners
- connector behavior; see [Agent Integration](agent-integration.md)

## Authority boundary

Authority remains with:

- Core-owned state
- user-owned judgment records
- close records
- residual-risk records
- storage rows
- persisted `ArtifactRef` records

Projections are derived display only. They are not owner state.

Display or support context only:
- projection
- status card
- Markdown report
- rendered template
- chat message
- connector output
- agent context packet

Rendered output may quote owner values, summarize owner records, or link to owner records. It is not a second state store and is not authority just because it is well written, manually edited, copied into a Product Repository, or injected into an agent context.

## Rendered display cannot

A rendered label, status badge, Markdown section, projection, template body, chat summary, connector output, or agent context packet cannot by itself:

- authorize writes
- create evidence or a persistent `ArtifactRef`
- satisfy verification, QA, evidence, acceptance, or other gates
- create final acceptance or accept residual risk
- create close readiness or remove a `CloseReadinessBlocker`
- close a Task
- mutate Core, storage, artifact, user-judgment, acceptance, residual-risk, or close records

If an owner record exists for one of those outcomes, the display may show or link to it. The display text is not the reason the outcome exists.

## Derived display and source state

Projection output is computed from current owner records at read time unless a future owner promotes a persisted projection job. It may help a person read scope, evidence gaps, blockers, freshness, next safe action, residual risk, and current guarantee wording.

Generated display must preserve omission, redaction, blocked-artifact, and unavailable notes without reconstructing hidden source values. A display that cannot read required owner state must show that condition instead of inventing a friendly-looking status.

## Freshness and source-state boundary

Projected output must keep its source boundary visible enough for the reader to judge it:

- Show source state version, source refs, observation time, or an equivalent freshness cue when the source provides one.
- Preserve stale, partial, unavailable, conflicted, or capability-limited source conditions.
- Keep display labels separate from canonical enum values and schema fields.
- Link back to the relevant owner when a reader needs the authority record.
- Treat hand-edited or stale display as display to discard or recompute, not as Core repair input.

## Template body owner

[Template Bodies](template-bodies.md) owns the body expectations for:

- status cards
- judgment requests
- run/evidence summaries
- close results
- agent context packets

This document may link to that owner, but it must not redefine those body sections.

## Later boundary

The current MVP has no active reconcile queue, editable projection input path, projection-to-Core repair path, persistent projection job, or managed block drift repair. Those remain out-of-scope capabilities until promoted with scope, fallback behavior, non-substitution rules, and proof expectations.

## Related owners

- [Template Bodies](template-bodies.md) for exact rendered body expectations.
- [Core Model](core-model.md) for Core authority, user-owned judgment, close readiness, final acceptance, and residual-risk boundaries.
- [API State Schemas](api/schema-state.md) for state-shaped data used by displays.
- [API Judgment Schemas](api/schema-judgment.md) for user-judgment and accepted-risk input shapes.
- [API Artifact Schemas](api/schema-artifacts.md) for `ArtifactRef` display inputs.
- [Security](security.md) for guarantee wording.
- [Scope Reference](scope.md) for projection reconcile and persistent projection job candidates.
