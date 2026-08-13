# Projections

`volicord-projections` owns the smallest Production read boundary needed for
Candidate Inspection and first-project-scoped bounded Recall.

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

The crate has no dependency in the reverse direction from Context, Repository
Intelligence, or Inquiry, and it exposes no write, renderer, CLI, MCP, viewer,
or external process responsibility.
