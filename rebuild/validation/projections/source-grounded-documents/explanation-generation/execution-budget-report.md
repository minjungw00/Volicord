# Explanation execution investigation — 2026-10-10

Status: bounded local controls pass; representative model completion remains
unmeasured. This is experimental adapter support, not a Product architecture,
quality verdict, provider qualification, gate or V11 result.

The inspected starting HEAD was `89bdf1613d4658bea3c1e4b0b8cd54de80d52192`, with a
clean worktree. The local `origin/rebuild` tracking ref also pointed there; this
does not attest the current server ref. No fetch, reset or history rewrite was
performed. Generation producer was `56d0cabec70e94bde45cee8858c6a10341ae89f0`.
Original cohort: `rebuild/.local/explanation-generation/comparison-20261010-0fe2f6458665/`.
The six receipts, public event/context captures, MCP protocols, retrieval ledgers,
renderer integrity, original note, partial-review handoff and retained validation
receipts were inspected. No private reasoning was inspected or reconstructed.

## Retained timelines

Seconds below are retained monotonic capture durations, including local startup
and cleanup. Probe/parent preparation cannot be separated more finely from old
records. Source-return timestamps are UTC wall-clock observations from public
tool envelopes; they are not monotonic measurements. The machine's recorded wall
elapsed times exceed monotonic durations by approximately 6–18 seconds per model
call. The cause is unobserved. Do not mix the two domains or treat that difference
as provider/model execution time.

| Attempt | Probe seconds | Model seconds / exits | Model stdin bytes | Nonempty read returns, first → last (UTC, 2026-10-09) | Verified reads / ledger calls / charged bytes |
| --- | ---: | --- | --- | --- | --- |
| Work B direct | 1.439 | 178.330 / -15 | 94180 | 16:57:40.350 → 16:59:43.364 | 27 / 29 / 114900 |
| Work B staged | 1.353 | analysis 178.387 / -15; no prose stage | 94000 | 17:00:58.967 → 17:01:39.017 | 11 / 12 / 59405 |
| Click direct | 1.182 | 178.569 / -15 | 82949 | 17:04:25.303 → 17:05:29.014 | 26 / 34 / 128676 |
| Click staged | 1.119 + 1.054 | analysis 123.299 / 0; prose 54.143 / -15 | 82769; 87937 | analysis 17:07:40.125 → 17:08:41.601; prose 17:10:01.493 → 17:10:31.362 | 31 / 43 / 90179 |
| Work B independent review | 1.097 | 178.853 / -15 | 9653 | 17:12:01.880 → 17:13:16.118 | 11 / 22 / 130775 |
| Click independent review | 0.960 | 178.931 / -15 | 41158 | 17:12:04.836 → 17:12:51.069 | 21 / 32 / 118004 |

All -15 returns have explicit timeout cause, SIGTERM and complete group/workspace
cleanup. Process streams are complete captures of the terminated processes;
that does not make the generation or assessment complete. Only Click's analysis
produced an original response: 5082 bytes with eight valid spans. No final prose
or final independent assessment exists. Both independent reviews ran concurrently
with the same model/provider; no statistical independence is established.

Public tool batches containing ledger results total about 0.251, 0.129, 0.202,
0.214 + 0.096, 0.286 and 0.263 wall seconds respectively. These batch envelopes
include host orchestration and may contain parallel reads; they cannot provide
per-read timing or exclusive model time. Individual batches completed in tens of
milliseconds. There is no demonstrated host retrieval latency bottleneck, provider
error, stream exhaustion or cleanup failure in these six attempts. Read volume was
substantial, especially near byte exhaustion in Click direct and Work B review,
but no causal test shows that retrieval count alone caused timeout.

The completed Click note reports 392848 input, 325376 cached input, 4120 output
and 1471 reasoning-output tokens. These are runtime-reported aggregate counters,
not a private reasoning trace or a measurement of initial prompt size. Interrupted
calls have no reported usage. Provider queue/network time, token throughput,
authentication identity and price remain unknown. Long intervals between public
tool events can include model execution, output formation or provider delay;
none can be assigned a measured exclusive duration.

Generation stdin contains 81–94 KB of initial metadata; the prose call repeats it
alongside the note. The compact preparation reduces mechanically serialized
initial JSON from 92375 to 50671 bytes for Work B, and 81144 to 47089 for Click.
All 163/151 starting records, four/zero diff entries and all 1474/317 authorized
entries remain reachable. No campaign function, correct anchor or editorial
answer is selected. This measured byte reduction does not prove a latency or
quality improvement. Reviewer prompts are 9.7/41.2 KB and embed renderer HTML
including duplicate prose in its original-transport disclosure. That duplication
is observed; it is not established as the main review timeout cause.

## Reproduced defects and bounded condition

The staged executor assigned the entire remaining shared deadline to analysis;
it had no time or response allocation for prose. An authored real subprocess
reproduced this: a stalled analysis received about 2.43 seconds from a 2.5-second
control condition despite a declared 1.7-second prose reserve. A second control
wrote an oversized response then slept; the old capture stopped only on timeout.
Both failing assertions and numeric exit 1 are retained. These are concrete
allocation defects, not evidence that a different model/budget would be better.

`conditions-bounded.json`, identity `bounded-explanation-20261010`, is explicitly
selected rather than retroactively replacing `conditions.json`. The original
condition hash remains
`bb403d7a07a57a7bb5c8cd6ec21ca6982299ac2684631464f26bb3fc30a52b2f`.
Both approaches retain equivalent initial evidence, authorized inventory and
180-second aggregate time/read/response/stream/retry budgets. Direct remains a
single generation call. Staged analysis is limited to 90 seconds/6144 response
bytes, preserving 90 seconds/10240 bytes for prose. Reads stop before explicit
finalization reserves, and tool results expose remaining resources. Large
response files stop the subprocess while retaining original bytes. Local
manifest verification/freezing is separately measured before execution; stage
setup and local probes consume the shared execution deadline. Cleanup and local
postprocessing can extend elapsed operation time and are recorded separately
from successful completion, not extra generation opportunities.
If cleanup/grounding consumes the prose reserve before the next call, the new
condition reports `finalization_reserve_unavailable` and retains only the note.

The 90/90 time split and 30/60-second finalization reserves are provisional
experimental allocations to test the reproduced starvation, not empirically
sufficient prose thresholds. The old 123-second note cannot fit unchanged into
the new analysis allocation. If investigation/note formation still needs that
time, the new condition must report incomplete analysis without dispatching prose.
An increased total budget was not justified or introduced. The original six
attempts do not demonstrate substantive completion within 180 seconds, nor prove
that such completion is impossible with narrower context/bounded investigation.

## Integrity, controls and readiness

Both reviewers' sequence 0 requested an invalid inventory limit (100 for Work B,
200 for Click). The original tool advertisement supplied only integer types and
omitted the Reader's enforced maximum of 80. An actual stdio consumer reproduced
that inconsistency: schema lacked `maximum`, while requests above 80 were denied.
The advertisement now gives the same range/default in schema and description,
which remains visible even when the host renders a simpler tool declaration.
This removes a demonstrated preparation defect that created an unmatched failure
before either reviewer could finish; it does not establish that timeouts are fixed.
Offset and read-limit minima, remaining-byte constraints and UTF-8 boundaries are
also explicit. The denied-call policy and all ledger accounting remain unchanged.

Work B's unmatched sequence 9 is a different failure: offset 65000 split UTF-8
bytes, and the exact-byte Reader rejected decoding. It is not a measured provider
delay or proof of an interrupted MCP request. Both reviews were terminated before
final output, but their unmatched rows are observed host-failed results of these
Reader denials. None is retroactively matched or promoted to successful inspection.
The original strict handoff assertion correctly exited 1; the separate partial
handoff exited 0 while retaining the gaps and `semantic_success: false`. Authored
consumer controls independently accept a complete exact join and reject failed,
in-progress, forged and unmatched joins. Existing maintained batch inspection and
partial-assessment consumers are reused, with no audit relaxation or new review
framework. Review prompt duplication remains a hypothesis for later measured work.

Real locally authored subprocesses exercise complete direct and staged output,
exact scoped reads and downstream rendering; stalled analysis with a live child,
response-budget termination and exit-zero/missing-output cases preserve their
actual outcomes. Deadline denials remain charged across restarts, and unmatched
ledger rows still fail strict audits. Existing controls cover foreign Project/Work,
stale bytes, unavailable before-state, malformed ranges, exhausted reads/bytes,
valid unrelated helpers and instruction-like source. Exact condition copies and
support/input/instruction hashes are retained. Old authorization is rejected for
the new condition. Synthetic prose tests transport/completion only.

Raw new evidence is under
`rebuild/.local/explanation-generation/bounded-execution-20261010/` and focused
runner receipts under `rebuild/.local/validation/`. The six-timeline audit can
verify changed producer files from their original Git objects without modifying
old receipts or restoring old code over current files. It still verifies original
runtime bytes at their recorded locations. Historical launchers/verifiers that
write into the cohort were not re-executed. A before/after cohort inventory binds
all 181 retained files; source inventories and earlier preservation evidence are
also checked through the original manifest/preservation receipts.

The next generation condition is executable under authored controls. A
representative source-reading model pilot and completed independent assessment
remain blocked by missing **current authorization for the changed condition and
source/producer identity**. Old one-opportunity permission is exhausted and does
not carry forward. No new model invocation, comparison result, semantic verdict,
human feedback or Product-source capability was fabricated. Product source bodies
and the cutoff-bound current-format baseline remain unavailable. No workspace
aggregate, authoritative gate, official V11 or legacy validation was run.
