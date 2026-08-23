# Projections

`volicord-projections` owns Production read models for Candidate Inspection,
first-project-scoped bounded Recall, project understanding, and grounded
documents.

- `SessionRecallTrigger` is in-memory session state. Unrelated requests do not
  consume it; exactly the first Project-scoped request requests automatic
  Recall. It is never canonical.
- Candidate Inspection accepts only `CandidateReadBasis`, the immutable bounded
  snapshot exposed by Inquiry. It reports identity, kind, provenance, scope,
  observation, retention policy, disposition, promotion target, independent
  cleanup kind/basis/time, current opt-out, and explicit content
  omission/degradation. It has no Candidate lifecycle handle.
- Resume Brief reads canonical and Repository Intelligence snapshots and uses
  Inquiry's pure applicability/frontier evaluators. Each repeatable section is
  deterministically bounded with typed omission records and expandable
  identities. Source gaps return a separate proposal; Recall never promotes it.
- Project projection adds overview, Repository Map, Decision–Context–Code links,
  a Checkpoint timeline with independent work/verification/review/acceptance
  facts, canonical inspection, capability gaps, and Candidate Inspection
  aggregation. Deterministic section-bound omissions are represented once per
  affected scope with an exact omitted count; Analysis and Source degradation
  stays explicit and retains its concrete identity.
- `ProjectUnderstanding` is the bounded, human-oriented read model over that
  immutable projection. It directly separates current, completed, and
  remaining work; grounded next steps; active Decision rationale and code
  impact; material Questions; risks and limits; evidence/freshness; and
  deterministic architecture topology. Agent interpretations remain in a
  separate collection and never become canonical or repository facts.
- Document generation produces Project & Architecture Guide, Decision Report,
  Implementation Plan, and Handoff / Resume bodies. Structural Fact, Semantic
  Result, and explicit Agent Interpretation claims remain distinct and carry
  their Source, Decision, and Analysis Snapshot bases.
- Arbitrary requested-language prose uses a fingerprinted `NarrativePlan` and
  an active-host `NarrativeRealization`. Realization must preserve the exact
  section/claim topology and protected code/path terms; grounding is copied
  from the plan and generator/agent/model provenance is required. Without a
  realizer, metadata reports `Unavailable` rather than treating the fixed
  English body as requested-language success.
- Markdown and self-contained HTML render from the same semantic body. Their
  ordinary reading path starts with current human meaning; versioned metadata,
  opaque identities, and direct per-claim basis remain in a trailing Markdown
  audit appendix or a closed HTML audit disclosure. A resolved Question's
  former choice ambiguity remains historical audit basis rather than current
  uncertainty. The returned publication artifact may carry an explicitly
  requested destination, but this crate never writes it.

The crate has no dependency in the reverse direction from Context, Repository
Intelligence, or Inquiry, and it exposes no canonical write, CLI, MCP, viewer,
or external process responsibility. Its bounded Markdown/HTML renderer returns
artifacts but owns no filesystem publication authority.
