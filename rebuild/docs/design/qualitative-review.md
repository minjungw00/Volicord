# Evidence-bound qualitative review

Status: active Phase 8 evaluation contract, subordinate to `validation-plan.md`.
This contract owns review artifacts and operations, not Product behavior or final
replacement policy. It grants neither reviewer kind Phase 9 approval authority.

## One rubric, explicit reviewers

`evaluation.json.qualitative_review_contract` and `qualitative_review.rubric`
maintain one rubric. Reviewer kind is explicitly `agent` or `human` and is fixed
with the preparation and recorded run. Changing kind requires a new run. An
agent's session/model authorship cannot be submitted as human authorship.
Kind is a declared role, not authenticated proof of a person's identity.

Every collected cycle enters review, including failed, indeterminate and
unevaluated cycles. Machine qualification is not a review prerequisite.
The following mapping preserves the former human rubric without another engine:

| Former review collection | Common criterion group | Preserved meaning |
| --- | --- | --- |
| `interaction_reviews` common fields | `interaction` | Question necessity/relevance, user ownership, source grounding, Decision comprehension, repetition, correct no-question behavior |
| behavior-specific interaction fields | `interaction` | Explicit material handling, hidden discovery, unnecessary interruption; learning fork value, alternatives/trade-offs, recommendation anchoring, feedback, fidelity, routine omission and proportional cost; exact maintained applicability and prompts |
| `document_reviews` | `documents` | Four-document fidelity, usefulness, grounding, remaining work and requested-language body quality |
| `viewer_snapshot_reviews` | `viewer_snapshot` | Completed/current/remaining work, next step, rationale, architecture/component/flow, code behavior, fact/interpretation and useful grounded diagrams |
| `repository_intelligence_reviews` | `repository_intelligence` | Structural navigation, semantic value, capability honesty and polyglot comprehension |
| `cli_usability_reviews` | `cli` | Help discovery and status/analyze/Recall/documents/export/doctor without opaque Project IDs |
| `live_viewer_accessibility` | `live_viewer` | `en`/`ko` keyboard reachability, visible focus, color-independent meaning, narrow/zoom presentation for the deterministic first Volicord cycle |
| `authority_obligation_reviews` | `authority` | Every initial material challenge, all other actual outcomes, additional outcomes and complete implementation/coupled-artifact coverage |
| Context recovery usability criterion | `context_recovery` | Goal, Decision/rationale, work state and open-question recovery across work/resume |

All non-accessibility collections cover every cycle. A static HTML snapshot does
not establish actual live keyboard/focus/zoom behavior; missing observation yields
insufficient evidence. Missing CLI captures similarly cannot establish usability.
No language or repository class is excluded because the implementation uses Rust.

## Assessment and evidence discipline

Each criterion has exactly one state: `satisfied`, `violated`,
`insufficient_evidence`, `not_applicable`, or `not_reviewed`. Insufficient and
unreviewed states remain distinct from violation and satisfaction. Inapplicability
requires a criterion-permitted reason: no user Decision in scope for Decision
comprehension, or single-language scope for polyglot comprehension. Missing data
is never such a reason; polyglot campaign scope cannot be declared single-language.

Each reviewed assessment retains bounded reasoning, uncertainty, evidence
references, and cited counterevidence or an explicit account of its absence.
References resolve an indexed evidence identity and typed locator in the same
cycle/evidence set. Reviewers explicitly list inspected evidence and observation
limits. Available evidence is not automatically inspected evidence. Hash checks
and locator existence do not prove the semantic adequacy of a citation or verdict.

Authority findings additionally retain material outcome, implementation commitment,
commitment state, resolution path, actual authority, relation and chronology.
The authority validator requires actual work evidence and canonical evidence for
Decision/prior authority; unrelated or late authority, silent commitment and
production-committed avoidance/defer/prototype cannot satisfy the obligation.
Initial concerns are rebuttable, non-exhaustive challenges. Additional independent
outcomes require their own assessments, and complete actual-work coverage remains
a separate required criterion. Generic interaction satisfaction cannot replace it.

## Binding, identity and machine relationships

A preparation binds candidate HEAD, evidence-set byte hash, optional machine run
ID/hash, rubric/policy revision and hash, and reviewer run identity. The same
`state`/`value` identity-claim primitive as document realization distinguishes
`unknown` from `self_reported`. Host, agent, model, session and person claims remain
explicitly unverified: no maintained attestation can verify these fields. An
arbitrary model string never becomes verified through evidence hashing.

An agent must name a session distinct from all evaluated work/resume sessions.
That correlation is self-reported, not authenticated session identity. The run
records the relationship and explicitly declines statistical independence: shared
model, context or preparation may correlate judgments even with different IDs.
Human review has no agent/model/session authorship; an operator may prepare a
human review for a person without claiming the operator is that reviewer.

Machine relationships are `agrees`, `clarifies_indeterminate`,
`probable_false_positive`, `probable_false_negative`, or `cannot_resolve`.
Clarification requires a review-required machine disposition and stronger,
surface-backed reviewer evidence. Disagreements are preserved without modifying
the machine run. Even a probable false-positive finding against a confirmed hard
violation leaves that machine finding blocking. There is no override field,
approval combiner or semantic prose scorer.

Review validity and the aggregate assessment describe only the recorded review.
Every result retains `qualification_state = not_run` and `phase_9_ready = false`.
The old human-only schema, validator and publication/approval helpers are removed.
The blind pre-campaign provisional workflow remains separate and unchanged.
