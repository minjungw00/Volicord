# Resume and execution observation handoff

This is focused normalization support for the preserved eight-session campaign
whose Product candidate is `b57b47a5ced212d2867c4f0b8bcf856841381e33`.
Development started at `c52f607c`. Reading those bytes with a later evaluator does
not make them executions of that evaluator. No immutable evidence set, measured
session, authoritative gate, official V11, review verdict or approval is produced.

## Repeated reads

Intake derives one Work from every successfully observed Recall and any new
Checkpoint. Every read must independently bind valid structured Project, selected
Work, Checkpoint identity/revision and Goal identity. Failed reads remain observable
attempts; they neither erase a valid sibling nor establish identity themselves.
Same-Project resolution and same-Work read repetition are permitted. The earliest
read anchors pre-work ordering only; first and last reads are never semantic oracles.
Supported intervening mutations and explanation recording may change state/revision.

Hard boundaries remain raw/task/candidate/session/activation identity, conflicting
Project or Work, malformed structured identity, transport-incomplete Recall, wrong
same-Work continuation and independently confirmed action/grounding contradictions.
Every read reaches `answer_observations.observe()` with its own temporal window,
authoring receipts and exact immutable Checkpoint. Goal supporting Sources are
checked even with an unavailable explanation. Missing historical ordering/basis
is scoped indeterminate evidence requiring qualitative review. Read counts and
operation counts remain separate advisory procedure facts; ordering/recovery
proxies and usefulness remain qualitative. Failed Product reads retain their
integration outcome. No raw or Product behavior is rewritten to meet a count.

Authored controls reproduce the original two-read rejection before the rule change,
then challenge two/three same-Work reads, repeated resolution, later conflicting
identity, malformed/incomplete reads, failed reads in both orders, successful
Work/Decision explanation recording between unavailable/current reads, CAS changes
between reads, insufficient temporal basis and actual transactional collection.

## Closed execution grammar and outcomes

New support is whole-cell direct forwarding:

```javascript
text(await tools.exec_command({cmd:"cargo test"}));
text(await tools.write_stdin({session_id:73,chars:""}));
```

One to sixteen direct statements may appear sequentially. Each must have a literal
argument object and one separate forwarded JSON result item in statement order.
Exactly matched output cardinality and an empty completed-wrapper header are
required. Existing assigned result/JSON forwarding, immutable literal bindings,
numeric template/projection forwarding and bounded static `Promise.all`
indexed/named/labeled result grammars remain supported. Duplicate result JSON keys
fail closed. JavaScript, arbitrary ASTs and expressions are never executed.

Execution identity is `custom_call:<raw wrapper call ID>:<group index>`, scoped by
the raw capture hash and session. Command facts retain raw call/turn, invocation
and completion sequences and group index. `write_stdin` may join only the observed
numeric process session of its launch, keeping the original launch identity and
all continuation coordinates. Wrong continuing session identity is rejected;
unmatched launch or result is an explicit limitation. Ambiguous multi-call output
never assigns one call's outcome to another.

Wrapper completion, tool completion and shell completion remain separate. Numeric
zero/exited can establish whole-shell success; 143 remains nonzero, and negative
signal exits retain signal/termination. Missing final completion remains
indeterminate. Output has separately inspectable retained/truncated/missing/unknown
state. Whole-shell success does not certify every compound CLI statement. Product
JSON is usable only for a uniquely supported standalone CLI invocation with actual
zero/exited completion and parseable returned JSON; no assistant prose supplies it.

## Coverage and safe projections

`CodexCapture.execution_wrappers` contains bounded `ExecutionWrapperObservation`
records: invocation/completion coordinates, safe call/turn IDs, wrapper SHA-256,
lexically referenced tool names, known call count or null, state and finite reasons.
States are `normalized`, `unsupported`, `indeterminate`; a capture without command
or wrapper evidence is `not_observed`. Tool-like string/comment/regex/template text
creates no call or coverage marker. Template interpolation references are inspected
lexically as coverage only, never proof of actual execution.

Mixed MCP/state code, dynamic arguments/forwarding, `Promise.allSettled` callback
wrappers, ambiguous result cardinality and orphan continuations remain limited.
No wrapper/command/output body is copied into the coverage projection.

Reviewer capture schema 3 / `naturalistic-review-capture-3` retains coverage records,
aggregate counts, launch/continuation identity, numeric outcome and output state.
Semantic completeness is separate from execution coverage. Missing parsed commands
cannot prove absence; unsupported text cannot prove success. Unsupported execution
after a validation boundary leaves that observation indeterminate. Available
confirmed numeric failure keeps its independently hard authority.

## Read-only corpus results

The diagnostic uses original task hashes to identify slots and compares each raw
SHA-256 to the before-fix observation. All eight had zero parsed commands before.
Numbers below are diagnostic facts, never acceptance thresholds or inner-operation
success counts. Completion means numeric whole-shell completion, including failure.

| Original session slot | Commands | Numeric completions | Nonzero | Unsupported wrappers | Indeterminate wrappers |
| --- | ---: | ---: | ---: | ---: | ---: |
| Volicord A start | 33 | 32 | 3 | 9 | 3 |
| Volicord A resume | 18 | 17 | 2 | 8 | 8 |
| Volicord B start | 23 | 21 | 3 | 7 | 3 |
| Volicord C start | 29 | 29 | 1 | 7 | 0 |
| Small Python A start | 37 | 32 | 8 | 22 | 16 |
| Small Python A resume | 16 | 13 | 2 | 7 | 4 |
| Polyglot A start | 48 | 42 | 12 | 9 | 7 |
| Polyglot A resume | 23 | 23 | 4 | 10 | 5 |

Across these bytes, 227 commands have raw identities, 209 have numeric completion
and 35 have nonzero outcomes. The 278 shell-reference wrappers comprise 153
normalized, 79 unsupported and 46 indeterminate observations. Fourteen literal
prepare/record hint locators remain inspectable without asserting inner execution.
Redirected/compound preparation and recording or dynamic result transformations
retain explicit limits; none disappears as proof of no execution. Authored standalone
prepare/record/readback controls additionally exercise actual JSON through
`measured_cli_operations()` and `collection_index()`, including failed outcomes.

All eight original inputs now pass raw capture/execution observation normalization
and the explanation collection-index diagnostic without changing their raw bytes.
Volicord A resume resolves one Work with both Recall observations retained.
This does **not** establish complete collection or review readiness: the current
reviewer conversation projector independently rejects an unnormalized user
interaction in each original rollout. That existing conversation-transport limit
was exposed, preserved and not waived by shell coverage. Session 2 must account for
it before claiming reviewer-package readiness. No evidence set is published here.

## Session 2 interfaces

Consume the current implementations together, without relying on commit subjects:

- `campaign.observed_project_ids()`, `observed_work_item_ids()`, `inspect_resume()`
  and `normalize_batch()` use all successful identity reads, without cardinality admission.
- `answer_observations.returned_recalls()/observe()` preserve all read windows and
  historical basis; `harness.real_session_evidence()` carries these machine facts.
- `codex_events.parse_custom_call()/parse_codex_capture()`, `CommandObservation`,
  `ExecutionWrapperObservation`, `CodexCapture.execution_evidence()` own normalized
  execution, continuation identity and explicit coverage.
- `explanation_evidence.measured_cli_operations()/collection_index()` retain numeric
  outcomes, unresolvable `execution_observation` entries and raw/hash bindings.
  JSON round trips must match immutable index verification.
- `harness.meaningful_resume_validation()`, `required_validation_machine_status()`
  and Checkpoint verification reconciliation distinguish unavailable parsing from
  missing execution and retain scoped review uncertainty.
- `interaction_diagnostics.work_summary()` and procedure machine facts expose exact
  successful/attempt counts, read sequences/outcomes and execution coverage.
- `review_captures.project()/validate()` emit schema 3; `review_operations` consumes
  that projection and the measured index; copied `result_lineage` compares returned
  meaning hashes to that index. Consume current machine policy/source hashes and
  original Product candidate identity when appending evaluation/review artifacts.

Private before/after support is under ignored `rebuild/.local/remediation/`.
`execution_corpus.py --campaign-root ROOT --before BEFORE.json --output NEW.json`
repeats the read-only diagnostic with a new ignored output, separate evaluator
HEAD/worktree state/source hashes, exact raw hashes and safe result/coverage locators.
It performs no collection/publication and leaves unrelated conversation limitations
explicit. Focused runner artifacts retain complete stdout/stderr, numeric exits and
termination. The original campaign inventory and initial raw/task hashes were
independently rechecked; explanation/resource inputs and rejected diagnostic remain
untouched. Authoritative gate, direct Final, official V11 and fresh chats were not run.
