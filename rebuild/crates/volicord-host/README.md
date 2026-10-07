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
caller-supplied current-host user turn as a user-authored Context Item. The
current stdio MCP boundary has no authenticated raw host-message identity or
content, so the returned `user_turn_content_provenance` explicitly reports
`caller_supplied_not_host_authenticated`. The Source retains the supplied turn;
the separate Context statement cannot rewrite it. A `goal` recorded this way is
available to ordinary Recall without creating a Decision. Evaluators that also
hold raw host capture require byte-identical `user_turn` content before treating
the Source as raw-host-consistent evidence.

After `project_resolve` returns `not_found`, `project_initialize` accepts the
repository without a display name and derives the initial Project display name
from the strongest unambiguous repository slug in a bounded local Git `origin`
lineage when available, without network or source-body access. Cyclic,
over-depth, unreadable, or malformed lineage retains a safe immediate hint,
then falls back to the canonical repository-root basename when no Git hint is
usable. A user-supplied display name is preserved; callers must not substitute
an ancestor directory or model guess. This hint does not rename an existing
Project or determine Project, clone, or worktree identity.

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
changed paths only from exact current, compatible same-Project snapshot evidence.
It reports the baseline dirty paths separately and uses the `Included` file
fingerprint delta from that retained pre-write baseline: a baseline-dirty path
that did not change afterward is not current work, while a tracked or untracked
baseline-dirty path whose fingerprint changed again is included in the bounded
delta. This observation does not claim exclusive actor or process ownership.
Missing, stale, freshness-unknown, wrong-Project, or incompatible-source
grounding still rejects canonical Checkpoint creation.
Research and prototype dispositions grant no tracked repository write authority.
Use read-only inspection, scratch outside the repository, or a separate disposable
worktree. Keep the original Goal/Discovery/baseline chain, resolve its Materiality
Review with evidence, and obtain ordinary executable scope before incorporating
results. A later baseline cannot replace blocked exploration. Repository changes
must be restored before that review can consume evidence completion.

Before the first ordinary write, `materiality_review` action `inspect` binds one
explicit executable scope to the current review. Its `paths`, `components`, and
`work_contexts` are typed independently from descriptive materiality scope;
parent repository paths cover descendants. The action is additive rather than a
new approval ceremony, and it rejects a late expansion over already-changed
paths.
When Checkpoint scope validation fails, the MCP error details contain every
uncovered changed path, component, and work context, the current executable
scope and review basis, and the maintained next action in one response.
It validates explicit applied Decision identities through
the current applicability contract and records executed verification as
command-execution Sources; the reported command outcome remains cooperative
host evidence rather than an OS attestation. User review and acceptance remain
independent and are not inferred by this operation. Recall prioritizes the latest Checkpoint so a restarted host can recover work
state, Decisions, verification, limits, and next step. Decision projection keeps
chosen and recommended alternative keys, user and recommendation rationales, and
alternative-specific consequences distinct. Its complete MCP result
(text plus structured content) is bounded to 256 KiB, with 768 KiB headroom
below the observed 1 MiB transport boundary. Whole-field or stable suffix
`transport_omission` reports carry exact omitted counts/size and parent inspection
basis; they are not semantic records. CLI and MCP share the same 56 KiB brief.
Learning resume contains only Candidate/revision and authority identities, current
state/outcome, response Source and bounded current implication. Full learning
rounds and discovery/Materiality graphs remain on `candidate_inspect` and
`learning_deliberation`. Large current fields require explicit inspection; Recall
never cuts JSON or fabricates an abbreviated authority statement.

`checkpoint_record.verification_basis` distinguishes ordinary changes from explicit
behavior preservation. A completed preservation claim or discovered compatibility
effect requires the relevant inspected repository surfaces, their preserved
contracts, focused verification indices, coverage rationale and preservation
rationale. Every completion surface must reference a passed command; base tests
that omit a known override/default-propagation hook are insufficient. The existing
source-linked verification outcomes retain this review for fresh Recall. Select
surfaces from repository evidence, without imposing irrelevant categories.

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

SessionStart and MCP initialization share a concise authority screen. After
repository facts and applicable accepted contracts and Decisions are inspected,
outcomes are classified in order as already settled, delegated implementation,
exploratory uncertainty, or unresolved material user-owned outcomes. Only the
last class stops for explicit user authority. Every independently material
dimension must be identified; a recommendation is not authority for another
dimension, while genuinely coupled dimensions may share one Question only when
all coupled material consequences are disclosed. Trivial implementation details
do not become Questions. The MCP tool descriptions and operations above own the
Candidate, frontier, and Decision procedure and validation.

For a current-task delegated dimension, `materiality_review` accepts a dedicated
evidence object containing the exact Goal and current-host user Source identities,
a bounded verbatim Goal statement, and its affected scope. `research_basis` is
independent and does not prove delegation. `candidate_inspect` returns only that
bounded delegation evidence for audit, never the unrelated raw user turn.

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

`materiality_review` draft returns current Goal/user Source, discovery/review identities
and revisions, choice/alternative and dimension identities, closed judgment variant
names/required fields, and ready-to-fill `record_request.skeleton` and
`pre_write_materiality_closure.inspect_request.skeleton`. Read the existing
`tools/list` → `materiality_review.inputSchema` for nested authority, delegation,
learning, residual-fork, interaction and commitment fields and bounds. Fill every
null semantic placeholder and one judgment per choice; record binds behavioral
Context, while revise retains that binding. No malformed schema probe is needed.
The agent-owned variant requires `bounded_implementation_discretion_rationale`
and nonempty `discretion_counterfactuals` in both inputSchema and the derived
draft field table. Inquiry still checks exact alternative coverage and Source
grounding. Other dispositions can have either ownership assessment; when the
assessment is agent-owned, the same discretion evidence is semantically required.
Discovery's two-alternative requirement is conditional on `evidence_state=sufficient`;
unresolved research/prototype discoveries may have fewer alternatives.
After authority and learning are resolved, fill the exact inspect scope, six artifact
assessments and commitment closure. Full explanatory semantics remain in the
[Inquiry owner](../../docs/design/inquiry-and-decision.md); full current discovery,
review and learning records remain on `candidate_inspect` with optional `candidate_id`.

Draft shares the 256 KiB complete MCP result budget and 80 KiB structured budget.
Repeated full schemas and Goal text are absent. Optional Context detail has explicit
omissions; required identities and variant tables are never truncated. An exceptional
complete draft larger than 80 KiB returns a bounded error with targeted inspection
identities and tools/list assembly guidance; retrying the same draft is not a remedy.
Detailed inspection and the complete tools/list catalog retain their own size limits;
this transport budget applies to Recall and Materiality draft.

Every Materiality judgment now requires `learning_authority`. Active participation
uses `assessed` with `independent_user_authority`, a Source-grounded counterfactual
`rationale`, and `source_ids` from current ownership evidence. The server binds the
assessment to that judgment's current choice and material outcomes. Inactive
participation may use `inactive`. Learning-only selections cannot authorize a
canonical Decision; independently user-owned policies retain their separate
Question/Decision lifecycle. Draft projects the compact variant table once as
`learning_authority_input_alternatives`; tools/list remains the full schema owner.

Discovery now reviews five interaction axes, including `temporal_and_lifetime`.
Challenge preserve/reset timestamps, retain/renew expiry, replacement age, and
retry/recovery validity independently of replacement triggers. Every planned
commitment requires `temporal_effect`: `no_temporal_change` with the exact fixed
temporal outcome/result identity and rationale, or
`reviewed_temporal_outcome` with current temporal outcome/result IDs. The primary
choice or interaction binding must cover that same result. Unmapped commitments
revoke executable scope and require rediscovery and a new Materiality Review.
Compact draft includes `temporal_effect_variants` and the current temporal IDs in
`pre_write_materiality_closure`; full nested schemas remain in tools/list.

When a Goal already exists, `context_record(role=goal)` requires an explicit
`work_transition`. `continue` takes the exact recalled `goal_context_id` and
returns its original identity, revision and Source without a canonical write.
`start_new` takes a bounded verbatim `statement` and current `user_turn` and
creates a distinct Work. Only the first Goal permits omission of the transition.
Recall/analysis workflow guidance exposes both transitions; continuing still
requires the normal pre-work evidence and authority review.

Checkpoint failures preserve `details.workflow` and a bounded canonical
`details.cause` (`kind`, `message`). A cross-Work Decision additionally returns
`checkpoint_decision_work_mismatch` with `checkpoint_work_item_id`, `decision_id`,
`decision_work_item_id` and a canonical inspection next action. Store rejection
remains authoritative; storage backtraces and underlying source errors are absent.

`inquiry_frontier.questions[]` includes `identity`, `revision`,
`presentation_receipt_id`, `prompt`, `why_now`, `material_scope`, `established_facts`
(with statement, Source basis, capability and freshness), `alternatives`,
`trade_offs`, `uncertainty`, `known_limits`, `prerequisites` (exact revision,
required/blocked/superseding outcomes and Source basis),
`allowed_non_choice_dispositions` and `what_unlocks`. Initial presentation keeps
`recommendation_state=withheld_until_initial_response` and contains no
recommendation or rationale. The successful exact receipt-bound `decision_record`
response retains the existing post-choice recommendation feedback.

Use `engineering_choice_discovery(action=draft, project_id,
baseline_analysis_snapshot_id)` after Recall/Goal and pre-work analysis. This
read-only operation returns the current Goal, its original host Source, bounded
behavioral Context, `record_request`, fixed enum inventories, and schema-derived
`variant_templates` with ready-to-fill skeletons. Choose variants and fill all
semantic nulls from current Source evidence. Source and graph-slot identities are
prefilled; they do not assert semantic truth. The caller may add alternatives,
choices and interaction results as needed, retaining stable IDs during resume.
Submit `record_request` through the same tool (`action=record`, the default when
omitted). `tools/list` remains the sole complete record schema. Continue through
the existing Materiality draft → record/revise → inspect path.

Pre-work errors retain exact schema `details.problems` and a supported
`details.next_supported_action`. Semantic closure/alternative-accounting failures
also carry `details.authoring_location` and the corresponding MCP input
`details.field_path`. Materiality errors resolve current Goal/baseline identities
from the retained Discovery/Review when available. Invalid submissions preserve
Candidates and never establish work authority. Other domain failures retain their
named invariant and draft/inspection recovery path; no semantic conclusions are
inferred from diagnostic prose.

Decision inputs use the exact selected user answer in `user_turn`. For
`request_user_input_async`, copy only the selected `reply.answer` and supply
`async_reply: {"request_call_id": "…", "question_index": 0}` using its
`questionItemId`. Verify the actual accepted request, Question, current raw
session/task and ordering first; uncertain replies remain unresolved. Preserve
the original envelope in the host rollout, separately from the canonical answer
Source. MCP rejects envelopes in `user_turn`; request references are caller-supplied
correlation and do not authenticate chat content. Independent evaluation must
verify raw evidence and persisted Source/Decision provenance together.

`decision_record` returns `user_response_host_session` as server-generated
correlation. Independent verification compares it to the persisted answer Source's
session and compares the returned `async_reply` to the actual request/index; neither
field replaces separately captured raw host evidence.

Exact retained evidence can be expanded through the existing inspection tools.
`canonical_inspect` with `offset: 0` pages all retained identities, revisions and
`detail_fields`; continue with `next_offset` and `expected_fingerprint`.
For a field, pass `project_id`, `record_kind`, `record_id`, `revision`, `field`
and optionally its exact `work_item_id`. Candidate detail uses `candidate_inspect`
with `project_id`, `candidate_id`, `revision`, `field` (the returned
`detail_fields` name retained typed sections). Each response carries up to 2,048
UTF-8 bytes of compact JSON in `chunk`; concatenate chunks before decoding JSON.
Continue with the same selector, `next_offset` as `offset`, and `fingerprint` as
`expected_fingerprint`. Changed bases reject continuation. Completion applies to
the retained field, not Source completeness or original omission counts.
Older unavailable revisions, missing/foreign/forgotten records and unrecorded
fields stay explicit unavailable states. Inspection opens no source paths, reads
no full turns or historical stdout/stderr, and requests no provider work.

A detail's `state` reports transport progress only. `metadata.retained_state`,
`source_status`, `content_omission` and `forgotten_source_count` preserve the
independent evidence state. A fully delivered verification field can therefore
remain partially retained or have unavailable Source support. Request
`field: "source_status"` to expand a bounded source-status list. Inspection does
not reset the original Recall/excerpt omission counts. The current record-only
reader cannot expose an old corrected revision merely because its revision number
is known; it reports `revision_unavailable`. A missing command body is
`historical_body_not_retained`; a full current-host turn body is
`source_body_policy_withheld`. Original raw-source truncation is `unobservable`.
