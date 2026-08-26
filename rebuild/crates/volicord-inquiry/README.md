# Staged Inquiry

`volicord-inquiry` owns the Production Session Candidate lifecycle and the
deterministic read-side calculation that identifies canonical Questions ready
to present. It does not own canonical Questions or Decisions.

## Candidate boundary

- `CandidateStore` requires an explicit SQLite path and uses schema kind
  `volicord-inquiry-candidates`, version 2. Candidate identity and persistence
  are physically separate from `volicord-context`; Candidate rows never enter
  a canonical portable bundle.
- The six Candidate kinds preserve bounded origin, collection scope,
  observation basis, timestamps, retention, disposition, opt-out basis, and
  optional content. Text, lists, and the encoded record have fixed admission
  limits; full prompts, raw Source bodies, unbounded command streams, and
  provider-private payloads have no storage field.
- Automatic submission reports `CollectionDisabled` when any applicable
  Project/session/source-operation/kind policy is opted out. Explicit
  user-directed submission remains distinguishable and is not blocked by an
  automatic-collection policy. Changing policy does not rewrite, hide, or
  dispose existing Candidates.
- Promotion, dismissal, and pending disposition remain separate from the
  explicit cleanup record (kind, basis, and time). Explicit deletion and
  retention expiry remove only Candidate content and managed Candidate copies;
  cleanup never opens Canonical Context. A promoted Candidate retains its
  disposition and canonical Question identity, while a dismissed Candidate
  retains its reason, so later cleanup and retry reconciliation remain
  inspectable.
- `CandidateReadBasis` is an owned immutable snapshot with no store or mutation
  handle. It contains only the bounded Candidate records and current scoped
  policies admitted by this store.

## Question Candidate and promotion boundary

Question Candidates preserve known facts, assumptions, uncertainty, material
scope, prerequisite proposals, canonical Source and Repository Intelligence
basis, freshness, duplicate/supersession assessment, explicit presentation
order, and the four-state materiality assessment. Research evidence may be
attached and its sufficiency recorded without promotion or canonical mutation.

Promotion requires a pending, research-ready, material, non-duplicate Candidate
in the same Project, an explicit presentation order, and valid canonical Source
bases. It submits one `QuestionDraft` to `volicord-context`. The Candidate
identity is also the canonical `OperationId`, so a retry after the canonical
commit but before local reconciliation replays the same Question rather than
creating a duplicate. Promotion is not complete locally until the Candidate
records the canonical target.

Repository Intelligence remains Derived State. Its snapshots can support a
bounded research assessment but cannot become canonical facts by mutation.
Research that establishes a sufficient canonical Source basis may use
`resolve_question_by_research`; that function invokes Canonical Context's
non-user disposition authority and creates no user response or Decision.

Before promotion, host guidance screens every independently material unresolved
user-owned dimension after owner, Decision/contract, and repository research.
A recommendation, preferred implementation, or one API dimension is not
authority for a separate unstated material policy. Independently user-owned
dimensions require independently explicit authority; genuinely coupled
dimensions may share one Question only when its alternatives disclose every
coupled material consequence. Trivial implementation multiplicity remains
agent-owned when it has no material consequence.

## Frontier boundary

`compute_frontier` is pure over an immutable canonical read basis and explicit
Inquiry scope. It selects only open, material, research-ready Questions whose
dependencies match the exact required terminal outcome and Source basis.
Missing prerequisites, invalid revision/basis, unsatisfied or superseding
outcomes, and dependency cycles remain explicit diagnostics. Results order by
canonical presentation order and then typed Question identity; discovery order,
wording, scores, timestamps, and map iteration are not authority.

Resume always recomputes that frontier from the current canonical read basis.
The latest Checkpoint's Question list is returned only as a historical
observation so a caller can explain any difference without treating it as a
second frontier authority.

## Response and Decision review boundary

Response interpretation requires an existing available canonical Source whose
payload exactly identifies the current host, session, and user turn, whose
actor is the user, and whose observing adapter is identified. It also requires
the exact open Question revision and the exact displayed alternatives and
recommendation. Ambiguity, stale display state, an echoed recommendation, a
wrong Project, and terminal Questions are rejected before calling the Kernel.

Accepted choice and delegation drafts are passed to
`Store::record_question_response`; batch adoption invokes that operation once
per Question and reports success, rejection, replay, and failure independently.
The read-only Decision applicability evaluator preserves scope, assumptions,
Source freshness, displayed basis, uncertainty, limits, revisit triggers,
contradiction, review-due, and supersession evidence. Explicit review-due
intents use the Kernel operation, while re-questioning first creates a bounded
Question Candidate.

## Checkpoint evaluation boundary

Checkpoint evaluation compares caller-supplied baseline and current Repository
Intelligence inventories from an exact retained pre-write basis. Paths already
dirty at the baseline are reported separately. An unchanged baseline-dirty path
is not current work; a tracked or untracked baseline-dirty path whose `Included`
file fingerprint changes after that baseline is part of the bounded repository
delta. The delta does not prove exclusive actor or process ownership. Missing,
stale, freshness-unknown, wrong-Project, incompatible-source, and other
ungrounded bases remain rejected. The evaluator accepts only meaningful
completion, explicit pause, or handoff with existing canonical Source basis.
Status-only reads and source gaps remain rejected Candidates. A ready draft is
persisted only through
`Store::record_checkpoint`, preserving independent work, verification, user
review, and user acceptance facts. No Git or command subprocess is introduced.
