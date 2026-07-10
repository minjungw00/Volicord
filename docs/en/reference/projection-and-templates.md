# Projection and template display boundaries

This document separates authoritative Volicord records from read-only
projections, status views, and template output. A `Projection` is a read-only
view derived from owner state; it does not create authority. Current rendered
body guidance, display phrasing, and user-facing labels live in
[Template Bodies](template-bodies.md).

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
- Agent Connection registration, current connection context, or connection capability declarations; see [Agent Connection](agent-connection.md)

## Authority boundary

Authority remains with owner records, not with rendered views. These records
include Core-owned state, user-owned judgments, owner-recorded evidence and
artifacts, acceptance and residual-risk decisions, close-readiness state, and
storage-owned records.

Projected, status, and template views are read-only display. They may quote,
summarize, or link to owner records. They do not become a second state store
when someone edits them, copies them into a `Product Repository`, or injects
them into agent context.

## Views cannot create authority

A rendered label, status badge, Markdown section, projection, template body, chat summary, rendered output, or agent context packet cannot by itself:

- create write tickets
- create evidence or a persistent `ArtifactRef`
- satisfy verification, QA, evidence, acceptance, or other gates
- create final acceptance or accept residual risk
- create close readiness or remove a `CloseReadinessBlocker`
- create, retire, or re-authorize a project continuity record
- close a Task
- mutate Core, storage, artifact, user-judgment, acceptance decisions, residual-risk decisions, or close-readiness state

If an owner record exists for one of those outcomes, a view may show or link to
it. The display text does not create the outcome.

## Keep source state visible

Derived display must show enough of the source boundary for a reader or agent
to know what the display is based on.

Display output must:

- show source refs, `state_version`, observation time, or an equivalent source cue when the owner result provides one
- preserve stale, partial, unavailable, redacted, blocked-artifact, conflicted, or capability-limited source conditions
- preserve evidence-provenance limits and continuity carry-forward cues when the owner result provides them
- keep display labels separate from canonical enum values and schema fields
- link back to the relevant owner when a reader needs the authority record
- treat hand-edited or stale display as display to discard or recompute, not as Core repair input

## Template and label boundary

[Template Bodies](template-bodies.md) owns current rendered body guidance for
status cards, judgment requests, run/evidence summaries, close results, agent
context packets, and public-error display labels.

This document may classify a view as authority or display. It does not define
the view's exact wording, body sections, or localized labels.

Rendered labels may link to semantic owners. They do not redefine owner
semantics or rename API values, storage fields, `ErrorCode` values, or blocker
codes.
