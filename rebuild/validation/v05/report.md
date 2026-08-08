# V05 — Inquiry frontier and session resume

## Status

Passed for the maintained deterministic inquiry fixture. Every accepted
terminal outcome was exercised, deterministic facts were resolved before user
questioning, and the same unresolved frontier was recovered after a separate
process was terminated with SIGKILL. This remains disposable experiment code.

## Goal

Validate that material Questions with explicit identities, revisions, and
dependencies can be presented as a current frontier; that user responses,
recommendations, and facts remain distinct; and that inquiry can pause and
resume across process/session loss without repetition or a fixed question-count
limit.

## Accepted decisions being validated

- `open-decisions.md` Q1: independent frontier batching, delegation, research,
  prototype, deferment, exclusion, supersession, pause/resume, and no fixed
  question or round count.
- Product charter sections 4, 8, 9, and 10 and acceptance scenarios F, I, and
  L: research repository/environment facts first, preserve user judgment and
  agent recommendation separately, bind a response to the shown Question
  revision, and do not repeat settled decisions.
- `validation-plan.md` V05: dependency accuracy, fact-before-ask behavior,
  terminal branches, process restart, deterministic ordering, and rephrased
  Question suppression.
- V03's committed experimental boundary: durable canonical rows and restart
  behavior may be reused only as test support and are not a production kernel
  contract.

No accepted product decision is changed by this report.

## Input repositories and revisions

The experiment starts from committed V03 baseline
`692bb00b` (`test: validate canonical context portability`) and imports its
prototype explicitly as `rebuild/validation/v03/prototype.py`.

The maintained `v05-inquiry-scenario` fixture has fixture-manifest SHA-256
`7967a0c665f962694a2c1b35d6bcab894137cc33bacd63c3049e1f4afcb5986f`.
It contains ten Questions: a deterministic implementation-language fact, five
initially independent user-facing Questions, a rephrased duplicate, a branch
made immaterial by an upstream choice, a delegable implementation choice, and
a UX choice requiring a prototype. The separate fact source fixes `Rust` at
source revision `fixture-v05-facts-1`. Both files are self-authored and declared
CC0-1.0.

## Environment and tool versions

- Linux x86-64, WSL2 kernel `6.18.33.2-microsoft-standard-WSL2`.
- Python `3.12.3`.
- Python standard-library SQLite `3.45.1`, accessed only through the committed
  V03 experimental module.
- No dependency download, LLM, host integration, network service, or external
  semantic provider was used.

## Candidate approaches

1. Recompute the current frontier deterministically from canonical Question
   status, dependency outcomes, semantic keys, and upstream Decision records.
   The executable prototype uses this as authority.
2. Persist the frontier list at pause time and replay that list as session
   authority. The experiment records this candidate in a Checkpoint and then
   compares it with recomputation before SIGKILL and after restart. The lists
   matched, so the snapshot is useful as an inspectable pause observation, but
   it is not needed as a second mutable authority.

The experiment therefore evaluates both recomputed state and persisted pause
snapshot behavior while keeping only canonical Questions/Decisions authoritative.

## Commands and configuration

The maintained focused commands are:

```text
rebuild/scripts/validate focused v05-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/fixture-manifest.json
rebuild/scripts/validate focused v05-assertions -- rebuild/validation/v05/assertions.py
rebuild/scripts/validate focused v05-assertions-repeat -- rebuild/validation/v05/assertions.py
rebuild/scripts/validate focused v05-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/v05/report.md
```

The assertion program initializes a V03 experimental store, computes and
answers two inquiry rounds with a close/reopen after each, records a pause
Checkpoint, starts a separate frontier reader that publishes its output and is
then SIGKILLed, starts another reader against the same database, and verifies
every final Question state.

## Observed results

- The deterministic fact source resolved the implementation-language Question
  to `Rust` with source ID and source revision before the first user frontier.
  That Question was never displayed to the user.
- The initial frontier contained five independent Questions in fixture order:
  privacy value, unknown recovery UX evidence, product collaboration scope,
  future synchronization, and enterprise identity scope.
- A deliberately stale response with the wrong Question revision was rejected
  before any user-turn Source or response record was persisted.
- Round one recorded two `answered` branches, one user "I do not know" branch
  as `resolved_by_research`, one `deferred`, and one `out_of_scope`. Closing and
  reopening the database retained every outcome.
- The collaboration-scope answer made the team-permission branch
  `superseded`. The answered privacy semantic key also made its rephrased
  duplicate `superseded`; neither appeared in a later frontier.
- After round one, the frontier contained only
  `question-storage-implementation` followed by `question-resume-ux`. Their
  prerequisites were positive terminal outcomes (`answered` or
  `resolved_by_research`), not merely arbitrary terminal states.
- The privacy Question retained the agent recommendation. Its Decision retained
  only the user choice and exact Question ID/revision. The separate response
  Context Item retained the exact user turn, response, outcome, Question ID,
  and displayed revision.
- The pause Checkpoint recorded the same two open IDs. A helper wrote a
  572-byte frontier and exited by SIGKILL (`-9`); a fresh helper and a fresh
  engine produced the same canonical bytes, SHA-256
  `f320194939cafaefada0a86cb04083ffe6df74d3114128cd129ec8b8080d793a`.
- Round two terminated the implementation branch as `delegated` and the UX
  branch as `requires_prototype`. On the final restart, all ten Questions were
  terminal, the frontier was empty, and the observed outcome set was exactly
  `answered`, `delegated`, `resolved_by_research`, `requires_prototype`,
  `deferred`, `out_of_scope`, and `superseded`.

## Coverage and failures

The fixture covers facts, user values, delegated implementation, prototype UX,
"I do not know", upstream supersession, semantic-key rephrasing, multiple
independent Questions, positive prerequisites, pause, close/reopen, SIGKILL,
and two rounds. It validates all seven requested terminal outcomes and both
exact identity and exact revision linkage.

No maintained assertion failed in the evidence run. Materiality ranking,
natural-language similarity, LLM question generation, host UI rendering,
concurrent user turns, and production Inquiry APIs are intentionally excluded.
The fixture's semantic key makes the rephrased duplicate deterministic rather
than claiming general paraphrase detection.

## Performance and resource observations

The two final focused runs completed their internal assertions in `137.854 ms`
and `141.331 ms` (`169.022 ms` and `171.974 ms` including the validation
runner). Each final experimental database
was 86,016 bytes and each serialized resumed frontier was 572 bytes. These are
small deterministic-fixture observations, not throughput, latency, or scale
benchmarks. Peak memory was not measured.

## Privacy and external transmission

All Questions, facts, responses, and sources are self-authored fixture data.
The experiment made no network request, used no provider or LLM, and
transmitted no source. Raw databases, frontier snapshots, summaries, and
complete command output remain under ignored `rebuild/.local/` state.

## Acceptance results

- Pass: only Questions with positively satisfied prerequisites were shown.
- Pass: a deterministic repository/environment fact source resolved its
  Question before any user-facing frontier.
- Pass: agent recommendation and user choice occupy separate canonical fields
  and record kinds.
- Pass: every accepted user response records the exact Question identity,
  displayed revision, and user-turn Source; a stale revision was rejected.
- Pass: pause and every inquiry round persisted before close/reopen.
- Pass: SIGKILL plus process restart restored byte-identical open frontier.
- Pass: ordering was deterministic across repeated calls and processes.
- Pass: an answered semantic key suppressed a deliberately rephrased duplicate.
- Pass: "I do not know" became research rather than a coerced choice.
- Pass: all seven terminal outcomes closed all ten material branches with no
  fixed Question or round count.

## Known limits

- The decision tree and materiality are maintained fixture inputs; this run
  does not validate automatic Question discovery or relevance quality.
- Semantic-key equality is an explicit identity mechanism, not general natural
  language paraphrase recognition.
- The experiment is single-writer. It does not address simultaneous host turns,
  divergent bundle merge, optimistic retries, authorization, or malicious
  response sources.
- One response currently uses sequential V03 experimental record operations.
  A crash between its Source, response, Decision, and Question revision was not
  injected; production work must define one atomic host-turn transaction or an
  idempotent repair rule before promotion.
- Positive prerequisite outcomes are fixed in the disposable prototype. A
  production domain contract must define whether particular delegated,
  research, or prototype results unlock each dependent branch.
- The pause Checkpoint is compared in full. A production resume view may need a
  bounded representation without making truncation a second authority.
- V03 storage and V05 inquiry code are both experiments; restart success does
  not promote either schema or API into `volicord-context`.

## Recommended implementation choice

Make canonical Question identity, revision, semantic key, prerequisite IDs,
status, and upstream supersession rules the durable basis. Recompute the
frontier in deterministic `(order, Question ID)` order from current canonical
state. A Checkpoint may record the observed pause frontier for inspection, but
must not become a competing state machine.

Resolve deterministic facts through an explicit source adapter before
frontier presentation. Record every host response against the exact displayed
Question revision and user-turn Source. Preserve recommendation on the
Question and user choice/delegation on the Decision. Map "I do not know" to
research, prototype, or deferment explicitly. Use semantic identity, not text
alone, to prevent an already answered decision from being presented again.

This is an experimental contract recommendation for later architecture review,
not a production API decision.

## Rejected alternatives and reasons

- Reject a persisted frontier list as resume authority: the observed snapshot
  matched deterministic recomputation, while making it authoritative would
  duplicate Question status and dependency state and permit stale lists.
- Reject a fixed scripted Question sequence: the first and second frontiers
  differed based on facts and answers, and two branches closed by supersession
  without being asked.
- Reject treating every terminal prerequisite as satisfied: deferred or
  out-of-scope branches should not automatically unlock dependent work.
- Reject text-only repetition handling: the deliberately rephrased Question
  needs a stable semantic key to connect it to the answered material decision.
- Reject storing agent recommendation in the Decision choice: separate records
  proved sufficient and made provenance assertions direct.
- Reject asking the user for deterministic fixture facts: the explicit source
  resolved the fact before the first user frontier.

## Reusable primitive decision

`reference_only`. Preserve the fixture graph, terminal vocabulary, exact-turn
and revision assertions, positive-prerequisite distinction, deterministic
ordering, pause/restart process scenario, and semantic-key non-repetition case
as validation evidence. Do not promote `prototype.py`, its dynamic V03 import,
or the experiment schema into production code.

## Decision revisit trigger status

Not triggered. Every Q1 terminal branch and resume behavior was represented
without a fixed count or a product-scope reduction. The known limits concern
production implementation and question-quality validation, not evidence that
the accepted Inquiry contract is infeasible. No product question is reopened.

## Follow-up work

- Define production Question and response responsibilities only after Wave 1
  architecture review, separately from the V03/V05 schemas.
- Validate Question materiality and user comprehension with non-scripted tasks
  and host interaction later; do not add LLM generation to this fixture.
- Exercise bounded Recall/Checkpoint selection in V09 and divergent Question
  state in V04.
- Add concurrent-turn and stale-response retry scenarios before production
  promotion.

## Artifacts

Maintained inputs are the V05 fixture-manifest entry,
`rebuild/validation/fixtures/v05/`, `prototype.py`, `assertions.py`, and this
report. Raw evidence remains ignored:

- `rebuild/.local/v05/assertions-t54gtwpl/summary.json`, SHA-256
  `012877d6a8e1f0a2cbfbc7ef88caa687702bda901f3a6fb29b3f3252a5e90f67`;
- repeat summary `rebuild/.local/v05/assertions-9z_0dg8t/summary.json`,
  SHA-256
  `0ac4555786d618005f1c073f4ef5143e387870ad30fbc8b21a8b06f214862050`;
- resumed frontier, SHA-256
  `f320194939cafaefada0a86cb04083ffe6df74d3114128cd129ec8b8080d793a`;
- final focused assertions:
  `rebuild/.local/validation/20260808T203738.365075Z-v05-assertions-6_6napm4`;
- deterministic repeat:
  `rebuild/.local/validation/20260808T203738.360728Z-v05-assertions-repeat-iz9vzbi9`;
- fixture and report checks:
  `rebuild/.local/validation/20260808T203738.359783Z-v05-fixture-manifest-fz5dcy64`
  and
  `rebuild/.local/validation/20260808T203738.369583Z-v05-report-shape-bcn15xmm`.
