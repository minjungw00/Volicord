# Volicord Codex host

`volicord-mcp` is a stdio MCP server exposing high-level Project, health,
Recall, repository understanding, Inquiry/Decision, Checkpoint, canonical and
Candidate lifecycle, privacy, document, analysis, and Guarded interaction
capabilities. It never exposes raw database operations or legacy methods.

For an arbitrary requested document language, `document_preview` first returns
`realization_required` with a bounded fingerprinted plan. The active host/model
returns the exact realized section and claim identities in a second call;
Volicord validates topology and protected code/path terms, retains the plan's
grounding, and records generator/agent/model provenance. No hidden provider or
recursive model call is used. English/Korean fixed-locale previews remain
deterministic, while a missing host realization never reports the requested
language as complete.

`context_record` preserves a bounded statement that occurs verbatim in the
exact current-host user turn as a user-authored Context Item. It returns both
the canonical Source and Context Item identities; a `goal` recorded this way is
available to ordinary Recall without creating a Decision.

After `project_resolve` returns `not_found`, `project_initialize` accepts the
repository without a display name and derives the initial Project display name
from an unambiguous bounded repository slug in local Git `origin` metadata when
available, without network access, and otherwise from the canonical
repository-root basename. A user-supplied display name is preserved; callers
must not substitute an ancestor directory or model guess. This hint does not
rename an existing Project or determine Project, clone, or worktree identity.

`repository_analyze` returns the existing Analysis and Repository Snapshot
identities needed to bound an ordinary work unit. Every fresh initialized or
resumed meaningful repository-work session calls it after initialization or
successful Recall and before the first ordinary repository write, then retains
the returned Analysis Snapshot identity for its eventual grounded Checkpoint.
An Analysis Snapshot first captured after the bounded work is not a valid
conceptual baseline; current provenance cannot prove edit ordering, so callers
must preserve this pre-write order rather than infer it later. For Git worktrees, that exact
Analysis Snapshot owns the machine-observed baseline dirty paths; callers do
not submit them. `checkpoint_record` takes the canonical Goal Context identity
and baseline Analysis identity, observes the repository again, and derives
changed paths only from compatible same-Project snapshot evidence. A baseline-
dirty path changed again is ambiguous and rejects canonical Checkpoint creation.
It validates explicit applied Decision identities through
the current applicability contract and records executed verification as
command-execution Sources; the reported command outcome remains cooperative
host evidence rather than an OS attestation. User review and acceptance remain
independent and are not inferred by this operation. Recall exposes the complete
latest Checkpoint so a restarted host can recover work state, repository
changes, Decisions, verification, limits, and next step.

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

Authorize an installed server for one repository:

```text
volicord --runtime /absolute/runtime codex enable /absolute/repository
```

This writes only repository-local Codex MCP and SessionStart configuration.
Codex project and hook trust remain explicit user-controlled host state. Use
`volicord codex disable /absolute/repository` before removing the installed
binaries; disabling leaves Runtime Home and canonical Project data untouched.

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
