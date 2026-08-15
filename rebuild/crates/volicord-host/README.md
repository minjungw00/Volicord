# Volicord Codex host

`volicord-mcp` is a stdio MCP server exposing high-level Project, health,
Recall, repository understanding, Inquiry/Decision, Checkpoint, canonical and
Candidate lifecycle, privacy, document, analysis, and Guarded interaction
capabilities. It never exposes raw database operations or legacy methods.

`context_record` preserves a bounded statement that occurs verbatim in the
exact current-host user turn as a user-authored Context Item. It returns both
the canonical Source and Context Item identities; a `goal` recorded this way is
available to ordinary Recall without creating a Decision.

`candidate_manage` requires `submit_question` to declare `research_required` or
`ready_to_ask` with an explicit `research_state_basis`.
`attach_repository_research` binds evidence to the current Project Analysis
Snapshot and canonical Repository Source, while
`mark_research_ready` invokes the Candidate owner's sufficient-evidence guard.
Neither action promotes the Candidate. `promote_question` remains the explicit
canonical Question transition, and `dismiss`/`delete` remain Candidate-local.
`candidate_inspect` exposes the current research state and attached repository
basis; an explicit current-host answer continues separately through
`decision_record` and its Question-revision/User-Source linkage.

Register an installed server with the current Codex CLI:

```text
codex mcp add volicord --env VOLICORD_RUNTIME_DIR=/absolute/runtime -- /absolute/bin/volicord-mcp
```

When the current host does not advertise elicitation, `guarded_interaction`
returns viewer and CLI fallback arguments for the same request identity,
revision, and fingerprint.

`background_semantic_operation` exposes the production provider boundary as
three explicit actions. `prepare` reads only named repository-relative files
from the current Analysis Snapshot, applies the existing Project privacy policy,
and returns the exact Guarded request. `dispatch` accepts that same revision and
fingerprint after `guarded_interaction`; `inspect` reads the durable Guarded and
provider outcomes by their returned identities. The privacy opt-in remains a
separate prerequisite and can be managed with the existing `volicord privacy`
CLI surface.

Filtered source bodies are retained only in the live MCP server while the
prepared request remains valid and awaits confirmation or a retryable
pre-dispatch correction. Explicit denial, expiration, terminal Guarded
rejection, or consumption by an actual dispatch releases that material.
Restarting the host also drops it without authorizing or retrying transmission;
already recorded outcomes remain inspectable. This build has no selected
external semantic-provider transport, so the configured-adapter path truthfully
records `provider_unavailable`, keeps every manifest entry `not_transmitted`,
and leaves local operations available.
