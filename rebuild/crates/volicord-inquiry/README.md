# Staged Inquiry

`volicord-inquiry` owns the Production Session Candidate lifecycle and the
deterministic read-side calculation that identifies canonical Questions ready
to present. It does not own canonical Questions or Decisions.

## Candidate boundary

- `CandidateStore` requires an explicit SQLite path and uses schema kind
  `volicord-inquiry-candidates`, version 1. Candidate identity and persistence
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
- Dismissal, explicit content deletion, and retention expiry are explicit
  local lifecycle transitions. Cleanup never opens Canonical Context. A
  promoted Candidate retains its canonical Question identity independently of
  later content cleanup so retry reconciliation remains inspectable.
- `CandidateReadBasis` is an owned immutable snapshot with no store or mutation
  handle. It contains only the bounded Candidate records and current scoped
  policies admitted by this store.

## Question Candidate and promotion boundary

Question Candidates preserve known facts, assumptions, uncertainty, material
scope, prerequisite proposals, canonical Source and Repository Intelligence
basis, freshness, duplicate/supersession assessment, explicit presentation
order, and the four-state materiality assessment. Research evidence may be
attached and its sufficiency recorded without promotion or canonical mutation.

Promotion requires a pending, material, non-duplicate Candidate in the same
Project, an explicit presentation order, and valid canonical Source bases. It
submits one `QuestionDraft` to `volicord-context`. The Candidate identity is
also the canonical `OperationId`, so a retry after the canonical commit but
before local reconciliation replays the same Question rather than creating a
duplicate. Promotion is not complete locally until the Candidate records the
canonical target.

Repository Intelligence remains Derived State. Its snapshots can support a
bounded research assessment but cannot become canonical facts by mutation.
Research that establishes a sufficient canonical Source basis may use
`resolve_question_by_research`; that function invokes Canonical Context's
non-user disposition authority and creates no user response or Decision.

## Frontier boundary

`compute_frontier` is pure over an immutable canonical read basis and explicit
Inquiry scope. It selects only open, material, research-ready Questions whose
dependencies match the exact required terminal outcome and Source basis.
Missing prerequisites, invalid revision/basis, unsatisfied or superseding
outcomes, and dependency cycles remain explicit diagnostics. Results order by
canonical presentation order and then typed Question identity; discovery order,
wording, scores, timestamps, and map iteration are not authority.
