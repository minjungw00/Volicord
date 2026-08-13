# Phase 6 Inquiry and Recall conclusion

- Phase 7 implementation gate: `ready`
- Scope: local Session Candidate collection and inspection, staged Inquiry,
  current-host Question response, Decision applicability, current-grounded
  Checkpoint creation, historical Checkpoint reading, and bounded Recall over
  the Production Rust Canonical Context, Inquiry, Repository Intelligence, and
  Projection boundaries
- Excluded claim: this conclusion does not certify V07, V10, V11, host UI or
  transport integration, generated documents, background-provider behavior,
  installation, or product cutover

## Candidate information class and lifecycle

Session Candidates are local, non-canonical observations stored in a database
that is physically and schematically separate from Canonical Context. A
Candidate preserves stable identity, revision, kind, collection mode, actor and
subsystem origin, collection scope, observation basis, freshness, bounded
content, retention policy, and the opt-out state observed at collection.

Automatic collection is Project-scoped and may be disabled for a matching
session, source operation, or Candidate kind. Explicit user-directed collection
remains distinguishable and is not blocked by an automatic-collection opt-out.
Existing Candidates remain inspectable after opt-out, and re-enabling collection
does not rewrite prior origin, collection, or lifecycle state.

Pending, promoted, and dismissed disposition is independent from content
cleanup. Explicit deletion or retention expiry removes Candidate content and
records cleanup kind, basis, and time. Cleanup does not erase a promoted
canonical Question identity or a dismissal reason, cannot create a canonical
target, and is idempotent across restart. Candidate content is bounded, and an
immutable `CandidateReadBasis` grants no mutation authority.

## Question Candidate, research, and promotion

A Question Candidate carries prompt basis, known facts, assumptions,
uncertainty, affected scope, possible prerequisites, Source and Repository
Intelligence basis, duplicate assessment, materiality, presentation content,
allowed non-choice outcomes, and research state. Repository or environment
facts are investigated before asking; attaching research evidence changes only
Candidate state and never fabricates a user Decision.

Promotion requires a current same-Project canonical read basis, an inspectable
Source and prerequisite basis, explicit materiality, and a promotable Question
Candidate. `not_material`, cleaned pending content, missing basis, and invalid
Project linkage fail before canonical creation. Promotion uses the Candidate
identity as its idempotent operation basis, so a canonical commit followed by a
retry reconciles to one Question rather than creating a duplicate.

## Frontier and terminal outcomes

The current frontier is recomputed from canonical Question state. A dependency
is satisfied only by its exact required terminal outcome, minimum canonical
revision, and required Source basis. Wrong outcomes do not satisfy a branch.
Missing prerequisites, cycles, invalid revisions, invalid Source basis,
blocked outcomes, and superseding outcomes remain explicit deterministic
diagnostics.

Independent Questions are ordered by presentation order and stable Question
identity, independent of discovery or insertion order. Terminal Questions do
not return to the current frontier after restart. A frontier stored in a
Checkpoint is a historical observation for comparison, never current resume
authority.

The seven terminal outcomes are `answered`, `delegated`,
`resolved_by_research`, `requires_prototype`, `deferred`, `out_of_scope`, and
`superseded`. Answer and delegation require an explicit current-host user
response and create a Decision. Research, prototype, deferment, exclusion, and
supersession terminate the Question without inventing a user Decision.

## Current-host response and Decision applicability

A response is linked to the exact Project, Question identity, displayed
Question revision, displayed alternatives and recommendation, host, session,
turn, and current user-authored `CurrentHostUserTurn` Source. Stale, ambiguous,
missing, wrong-Project, mismatched, recommendation-only, terminal, or
superseded input is rejected. Accepted responses preserve the agent
recommendation separately from the user's choice or delegation.

One user turn may address multiple Questions through explicit per-Question
items. Each item reports success, replay, rejection, or failure truthfully; one
rejected item neither rolls back nor disguises another committed item. A
rejected canonical response operation leaves no partial Source, Decision,
Question transition, or unrelated mutation.

A Decision is reusable when Project, applicable paths/components/work context,
assumptions, current evidence, and revisit triggers remain valid. A new agent
session or model change alone does not invalidate it. Scope or assumption
change, stale/unavailable/unknown material Source basis, a met revisit trigger,
contradiction, or `review_due` produces inspectable review state and may create
a new Question Candidate; it does not silently rewrite the Decision. A
superseded Decision is not reusable, and a changed semantic user choice creates
a superseding Decision rather than an in-place correction.

## Checkpoint, pause, and resume

Completion, pause, and handoff are distinct meaningful boundaries. Status-only
reads, unchanged explanations, response rejection alone, source-less
speculation, and empty completion do not create a canonical Checkpoint.
Completion requires attributable state change, changed paths, an applied
Decision, executed verification, or a new known limit. Pause and handoff
preserve their explicit work state and target without claiming unsupplied work.

Repository change attribution compares current baseline and current analysis
snapshots for the same canonical Repository Source. Paths already dirty at the
baseline are not attributed to bounded work. A path changed again from an
already-dirty basis is ambiguous and prevents Checkpoint creation; unavailable
or mismatched attribution fails closed.

Work state, automated verification, user review, and user acceptance are
independent facts. Executed verification requires a current command-execution
Source. Observed review and accepted or rejected acceptance require a current
user-authored current-host turn Source. `not_run`, pending, and not-requested
states do not claim a Source.

Every Source used to make a new Checkpoint eligible must be current, available,
same-Project, and correct for its semantic role. Missing, stale, unavailable,
unknown, wrong-kind, and wrong-Project bases remain distinguishable and create
no Checkpoint or unrelated mutation. A current Repository Intelligence result
cannot override a non-current canonical Repository Source.

A Checkpoint validly recorded from a current Source basis remains readable when
that Source later becomes stale or unavailable. Recall exposes the degraded
basis and proposes investigation; it does not rewrite or delete historical
Checkpoint state. After restart, terminal Questions and linked Decisions remain
durable, current frontier is recomputed, and the Checkpoint frontier remains an
observation.

## Candidate Inspection

Candidate Inspection is a bounded read-only projection over an immutable
Candidate basis. It exposes existence, health, revision, kind, origin,
collection and observation scope, retention and expiry, promotion or dismissal
disposition, promotion target, cleanup basis, current applicable opt-out state,
and bounded content when policy permits.

Cleaned content and policy-withheld content are explicit partial states;
unexpectedly unavailable content is degraded rather than presented as clean or
complete. Missing Candidate identity is explicit. Inspection receives no store
handle and leaves Candidate and canonical state unchanged.

## Bounded Recall and Resume Brief

Automatic Recall is session-local. An unrelated greeting or request does not
trigger it. The first Project-scoped request does, later requests in the same
session do not repeat it, and a fresh session has an independent first-request
trigger.

The read-only Resume Brief deterministically selects bounded goals and why,
active and historical Decision rationale, latest meaningful Checkpoint, open
Questions and current-frontier state, risks, assumptions, known limits,
Repository Intelligence coverage/freshness, used Sources, and the next
meaningful step. Current, stale, unavailable, review-required, and superseded
Decision or Source meaning remains distinct.

Each repeatable section is bounded. Omitted records retain identity, kind,
reason, and an expandable inspection basis, and `omitted_count` matches the
reported omissions. Projection accepts no canonical or Candidate mutation
handle; repeated selection is deterministic and no-mutation.

## Maintained V09 evidence

V09 is `passed` at the Phase 6 Production boundary. The maintained acceptance
map contains 83 requirements across Candidate lifecycle, promotion/frontier,
response/Decision, Checkpoint, and Recall/inspection. Its orchestrator discovers
and executes 47 Production Rust tests across `volicord-context`,
`volicord-inquiry`, and `volicord-projections`; it implements no parallel domain
engine. The deterministic repeat, fixture-manifest validation, validation-report
shape, and architecture-contract checks pass.

## Known limits

- Fixtures are local, deterministic, self-authored, and small. They do not
  establish natural-language Question relevance, large-context ranking,
  latency, or resource ceilings.
- Automatic Question and Candidate discovery quality and LLM interpretation
  are not evaluated.
- Codex, CLI, MCP, viewer, installer, generated-document, and background
  provider integration are not implemented or certified by this phase.
- V07 privacy journeys, V10 process/filesystem primitive qualification, and
  V11 multi-repository rehearsal remain incomplete and are not claimed.
- Phase 6 does not implement Phase 7 behavior or certify product cutover.

## Decision and Phase 7 status

No accepted Q1–Q13 Decision revisit trigger is active. Candidate privacy and
opt-out behavior remains Project-scoped; current-host user authority remains
exact; historical degradation does not become forgetting; and Decision reuse
does not expand beyond unchanged applicability.

Phase 7 entry is `ready`. The independent Phase 6 audit, maintained V09
Production evidence, deterministic repeat, and focused architecture and report
checks support the Candidate, Inquiry, Decision, Checkpoint, inspection, and
Recall responsibilities needed by the next phase. This decision does not imply
V07, V10, V11, or cutover completion.

## Maintained references

- Active owners: `rebuild/docs/design/architecture.md`,
  `rebuild/docs/design/domain-model.md`,
  `rebuild/docs/design/inquiry-and-decision.md`,
  `rebuild/docs/design/projections-and-documents.md`,
  `rebuild/docs/design/privacy-and-provider-boundary.md`,
  `rebuild/docs/design/failure-and-recovery.md`,
  `rebuild/docs/design/validation-plan.md`, and
  `rebuild/docs/design/versioning-policy.md`.
- V05 foundation: `rebuild/validation/inquiry/frontier-resume/report.md`.
- V09 acceptance report:
  `rebuild/validation/inquiry/phase-6-acceptance/report.md`.
- V09 Production evidence:
  `rebuild/validation/inquiry/phase-6-acceptance/assertions.py` and
  `rebuild/validation/inquiry/phase-6-acceptance/fixtures/phase-6-matrix.json`.
- Fixture authority: `rebuild/validation/shared/fixture-manifest.json`.
- Production boundaries: `rebuild/crates/volicord-context/`,
  `rebuild/crates/volicord-inquiry/`, and
  `rebuild/crates/volicord-projections/`.
