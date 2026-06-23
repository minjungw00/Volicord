# Projection and template display boundaries

This document owns the distinction between authoritative Volicord records and read-only projected, status, or template views. A `Projection` is a view or rendered state surface; it does not create authority. Current rendered body guidance, display phrasing, and user-facing labels live in [Template Bodies](template-bodies.md).

## Owns / Does not own

This document owns:

- authority boundaries for projections, status views, and template views
- read-only derived-display rules
- source-state visibility requirements for display output
- the rule that rendered labels are display text, not canonical schema values
- routing between authority questions and display-wording questions

This document does not own:

- status card, judgment request, run/evidence summary, close result, agent context packet, or public-error display wording; see [Template Bodies](template-bodies.md)
- source-of-truth Core state, user-owned judgment, evidence, acceptance decisions, residual-risk decisions, or close-readiness state; see [Core Model](core-model.md)
- storage records, artifact records, or storage effects; see storage owners through [Reference Index](README.md)
- public API schemas or method behavior; see API owners through [Reference Index](README.md)
- surface registration, current surface context, or capability declarations; see [Agent Integration](agent-integration.md)

## Authority boundary

Authority remains with owner records, not with rendered views. Authoritative records include Core-owned state, user-owned judgments, owner-recorded evidence and artifacts, acceptance decisions, residual-risk decisions, close-readiness state, and storage records owned by the storage documents.

Projected, status, and template views are read-only display. They may quote owner values, summarize owner records, or link to owner records. They are not a second state store, even when they are clear, manually edited, copied into a `Product Repository`, or injected into agent context.

## Views cannot create authority

A rendered label, status badge, Markdown section, projection, template body, chat summary, surface output, or agent context packet cannot by itself:

- create `Write Authorization`
- create evidence or a persistent `ArtifactRef`
- satisfy verification, QA, evidence, acceptance, or other gates
- create final acceptance or accept residual risk
- create close readiness or remove a `CloseReadinessBlocker`
- close a Task
- mutate Core, storage, artifact, user-judgment, acceptance decisions, residual-risk decisions, or close-readiness state

If an owner record exists for one of those outcomes, a view may show or link to it. The display text is not the reason the outcome exists.

## Source state in display

Derived display must keep the source boundary visible enough for a reader or agent to know what the display is based on.

Display output must:

- show source refs, `state_version`, observation time, or an equivalent source cue when the owner result provides one
- preserve stale, partial, unavailable, redacted, blocked-artifact, conflicted, or capability-limited source conditions
- keep display labels separate from canonical enum values and schema fields
- link back to the relevant owner when a reader needs the authority record
- treat hand-edited or stale display as display to discard or recompute, not as Core repair input

## Template and label boundary

[Template Bodies](template-bodies.md) owns current rendered body guidance for status cards, judgment requests, run/evidence summaries, close results, agent context packets, and public-error display labels.

This document may say whether a view is authority or display. It must not define the exact wording, body sections, or localized labels for that view.

Rendered labels may refer to semantic owners to help readers understand owner records, but labels do not redefine those semantics or rename API values, storage fields, `ErrorCode` values, or blocker codes.

## Owner links

- [Template Bodies](template-bodies.md) owns current rendered body guidance, display phrasing, and user-facing labels.
- [Core Model](core-model.md) owns Core authority, user-owned judgment, close readiness, final acceptance, and residual-risk boundaries.
- [Reference Index](README.md) routes API, storage, artifact, and security owner questions.
- [Agent Integration](agent-integration.md) owns surface registration, current surface context, and capability declarations.
