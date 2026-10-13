# Source-grounded completion diagnosis

Status: observable execution characterized; no demonstrated generic execution fix
or new measured generation. Inspected baseline: `c81badfe`. Date: 2026-10-13.
This is experimental support, independent of Product qualification, gate and V11.

## Retained opportunities and observed boundaries

The two requested failures are the original `cross-work-20261013/frozen-plan-2/
measured/work-b-repeat` and `cross-domain-expansion-20261013/measured/known-polyglot`
attempts. The earlier completed `admissible-secondary-173cf460/work-b-first-measured`
and earlier failed `work-directed-b27e1316/measured-1` provide separately identified
comparisons. They are not retries created by this diagnosis. All original bytes,
streams, receipts and attempt identities remain unchanged.

Times below use the captured monotonic process clock. First/last read times are
local Source-server execution relative to subprocess spawn, qualified by the exact
independent Host/ledger/Reader join. They do not measure full tool round trips.

| Opportunity | Preparation / context probe seconds | Model subprocess seconds | Verified nonempty reads / inventory calls | First / last read seconds | Deadline remaining at last read | Final response |
| --- | --- | --- | --- | --- | --- | --- |
| Earlier completed Work B | 0.271 / 1.107 | 818.053 | 168 / 8 | 10.074 / 615.228 | 283.230 s | 23,266 original bytes; final Host message matches; one completed turn |
| Requested Work B failure | 0.268 / 1.477 | 898.320 | 178 / 12 | 12.054 / 355.313 | 542.784 s | Absent; four short Host messages; no completed turn |
| Requested tree-sitter failure | 0.201 / 1.338 | 898.427 | 159 / 8 | 13.786 / 203.747 | 694.461 s | Absent; four short Host messages; no completed turn |
| Earlier Work B failure | 0.332 / 1.079 | 898.696 | 159 / 25 | 9.733 / 678.761 | 219.727 s | Absent; four short Host messages; no completed turn |

The requested Work B has a later inventory return with 145.041 seconds remaining.
Its stdout contains three public errors reporting reconnect attempts 2/5, 3/5 and
4/5 and high demand. These are observed CLI transport behavior, despite the
experiment adapter's zero retry budget. Their exact latency and contribution to
the watchdog are unavailable. The tree-sitter stream contains no public error;
absence of a reported error does not prove provider availability.

All three failed subprocesses ended with numeric returncode `-15`, signal 15,
`stop_cause: timeout`, complete retained streams, complete owned-process-group
cleanup and complete temporary-workspace teardown. Each contacted only the
watchdog ceiling. Neither requested failure has an unmatched ledger row, pending
public tool call, Reader denial, compaction, response file or final turn event.
The useful investigation does not count as explanation completion.

Inventory is observable as tool calls and returned bytes; investigation is
observable as completed reads and public messages. The intervals after reads
cannot be partitioned into hidden investigation, model processing, provider wait
or response finalization. No hidden reasoning was inspected. In particular, a
long period without Source reads is not evidence of an adapter deadlock or of
time spent assembling citation metadata.

| Opportunity | Verified Source / inventory returned bytes | Repeated verified Source byte coverage | Local read-service seconds | Retained aggregate streams |
| --- | --- | --- | --- | --- |
| Earlier completed Work B | 1,062,549 / 80,368 | 17,687 | 0.150 | 1,743,979 |
| Requested Work B failure | 1,705,932 / 117,664 | 9,983 | 0.157 | 2,522,972 |
| Requested tree-sitter failure | 1,196,160 / 414,573 | 14,197 | 0.095 | 2,086,960 |
| Earlier Work B failure | 1,510,360 / 1,036,353 | 484,165 | 0.166 | 2,996,585 |

Repeated coverage is the overlap with already returned byte ranges of the same
opaque Source identity, counting the prior union once. It does not establish
unnecessary exploration: reinspection can be justified. The earlier failed Work B
also has two returned ledger rows (sequences 33/34; 86,680 bytes) without matching
Host results. Its original and recomputed audits both retain that gap. Those bytes
qualify neither Source timing nor coverage in these tables; ledger return alone
does not prove model-visible delivery. Local service totals exclude inventory, transport and
model work; they are not a latency breakdown for the complete opportunity.

Requested failures expose no token usage because no `turn.completed` was retained.
Zero tokens or a cost estimate must not be substituted. The completed comparison
reports 1,793,244 input tokens, 1,621,248 cached input tokens, 28,572 output tokens
and 13,252 reasoning-output tokens; only numeric accounting is used. Provider
price, typical cost and production affordability remain unmeasured.

Wall elapsed time differs from monotonic capture by approximately +97.332 seconds
for the requested Work B and +1.104 seconds for tree-sitter (+59.965 and +58.714
for the earlier comparisons). Public message timestamps remain wall-clock facts;
they are not subtracted from monotonic read times. Exact monotonic process-end and
response-file-write timestamps were not retained.

## Exact comparison identity

All four requested `gpt-6.1-sol`, reasoning effort `high`, OpenAI Codex service,
Korean, archive-diagnostic lane and one direct model opportunity. Exact executable
SHA-256 is `12eb3e81114588aca3b7998f4f19e8997b056aca08e57a7ca7c8a3ec8c652aad`.
This is requested/runtime-observed identity, not provider authorship attestation.
The unchanged safety envelope is 900 seconds, 2,048 evidence calls, 8,388,608
returned evidence bytes, 1,048,576 response bytes, 67,108,864 stream bytes,
4 seconds cleanup and zero adapter retries, with no reserved finalization stage.

Common instruction identity is 6,888 bytes,
`920faf6769e477fba025d8db11dbd556c43a169728a5da505eae50e46b58c54a`;
common response-schema identity is 4,142 bytes,
`f0708abc8458abc2b6a7ddd1a51582095553189f1302b63fb47a69bca7be5568`.

| Opportunity | Original producer HEAD | Input-manifest SHA-256 | Condition SHA-256 | Prompt UTF-8 bytes |
| --- | --- | --- | --- | --- |
| Earlier completed Work B | `173cf4602e9a849e1902f56bbaec0dbd1addad58` | `c5bb58767e07597f49c6bd10dd91ee2dd57fdf54043c45d16d75ed7e59711806` | `e520c602be584f47cc2f9ce3af79599b59def9a81a19cac81a59dc6c787dad0c` | 60,420 |
| Requested Work B failure | `5dd37b2ee0ecc5b2525b196a0065ae97d47d4d61` | `7449864e8e6c85442ede0ae90cd222992616b478f7b59d3db1d68b5dfe93e08b` | `e520c602be584f47cc2f9ce3af79599b59def9a81a19cac81a59dc6c787dad0c` | 60,420 |
| Requested tree-sitter failure | `7ad6ec74ad53b6da1f844917087712b2d1129e3f` | `604f7b04c18cb1a9c7001e3827791c95c6e28584edf6918fe4d7d43a19ed149a` | `e520c602be584f47cc2f9ce3af79599b59def9a81a19cac81a59dc6c787dad0c` | 67,990 |
| Earlier Work B failure | `b27e13161c6756a31234cad36cd730dfc2b6563a` | `c5bb58767e07597f49c6bd10dd91ee2dd57fdf54043c45d16d75ed7e59711806` | `dca857e1849bd33b448a19b695e29d8d384022f65da4a13a7b6b4ace5c844ea3` | 59,155 |

The completed and requested failed Work B have equal prompt sizes, conditions,
instructions, schemas, executable and core execution support hashes, but different
input/initial-inventory identities and reader-contract/recorded-work/source-reading
support hashes. Exact frozen support, prompt, stream and initial-input identities
are preserved and checked by the private audits. This is not a controlled
single-factor comparison. Additional Source metadata, model strategy and response
requirements remain hypotheses, not established timeout causes.

Among the two specifically requested failing opportunities, observed incompletion
is **2/2**. Among the four explicitly listed historical opportunities, final
transport completion is **1/4**, with **3/4** watchdog failures. This retrospective
selection is not a representative failure rate, Product SLA or explanation-quality
score. Completion, provenance binding, semantic correctness, reading quality and
affordability remain separate assessments.

## Change, controls and diagnostic limitation

`execution_audit.py` now reports independently joined local read clocks even when
public code-mode output filtered away ledger sequence identities. It also reports
public errors, message sizes, final-file/Host-message equality, completed-turn
count, context compactions and unavailable measurements, with exact frozen schema
and producer-snapshot verification. It changes no invocation, Reader, grounding,
rendering, historical attempt or generation-success classification.

Six authored actual-process controls cover complete output with and without a
final Host envelope, exit-zero/absent response, watchdog/descendant cleanup,
interrupted tool observation, and absent/foreign clocks/private reasoning exclusion.
An explicit transport-error control preserves a matching complete final response
without converting that transport fact to semantic success. Existing invocation,
allocation, Source Reader, citation-handle, grounding and rendering tests remain
the execution owners for their responsibilities; no evaluator framework was added.

No generic execution deadlock, unbounded adapter loop or final-output defect was
reproduced. The independently controlled watchdog, missing-output rejection and
cleanup boundaries behave correctly. Consequently the proposed fix commit is
omitted, and no speculative investigation cutoff or schema relaxation is frozen.
The existing verified-reference mechanism derives byte metadata from exact joined
reads; its eight real-stdio controls pass. It currently requires a retained
observation and supplies no live model tool. Integrating it into generation would
change the mechanism and require a distinct frozen condition and current exact
transmission authorization; its metadata correctness does not establish a
completion-reliability benefit.

No fresh model opportunity ran. The request makes external invocation conditional
on exact current approval; it supplies no new exact input/condition/producer
authorization. Historical permissions and installed credentials were not reused.
There is therefore no new denominator, model-authored substitute, explanation
quality verdict or final-reading result. A narrower mechanism remains an
unmeasured hypothesis, rather than an invented fix or synthetic success.

## Validation and retained evidence

Nine focused scripts executed 153 tests: 151 passed and two explicit optional
checks skipped (retained citation-diagnostic fixture root; real browser paths).
Each used `rebuild/scripts/validate focused <label> -- python3 -B
rebuild/validation/projections/source-grounded-documents/explanation-generation/<script> -v`:

| Script | Tests / skipped |
| --- | --- |
| `execution_audit_self_test.py` | 31 / 0 |
| `invocation_self_test.py` | 10 / 0 |
| `execution_self_test.py` | 9 / 0 |
| `approach_self_test.py` | 16 / 1 |
| `reference_reading_self_test.py` | 8 / 0 |
| `reader_self_test.py` | 18 / 0 |
| `source_reading_self_test.py` | 21 / 0 |
| `render_self_test.py` | 24 / 1 |
| `work_reader_self_test.py` | 16 / 0 |

The six new actual-process controls also retain their complete private execution
artifacts. Four explicit historical audits pass, verifying streams, Source joins,
original producer objects and frozen snapshots. A before/after inventory checks
all 126 files in the four original attempt directories.
Focused nested-workspace Cargo metadata passes: all nine packages and their local
dependencies reside under `rebuild/`, with no legacy crate dependency.

The initial development control run failed with five fixture-constructor errors;
the constructor was corrected before the actual-process controls ran successfully.
The historical diagnostic driver initially rejected the earlier Work B's already
recorded unmatched rows. Its expected limitation was corrected and the final audit
preserves those rows; no Source join rule or historical receipt was changed.
Both failed development receipts remain retained alongside the passing results.

Raw evidence and new audits remain under ignored
`rebuild/.local/explanation-generation/completion-diagnosis-20261013/`; focused
runner receipts remain under `rebuild/.local/validation/`. The three other audits
have complete exact Source joins; the earlier failed Work B audit preserves its
two unmatched observations and does not assert complete retrieval verification.
Full hashes and bytes,
not rendered previews, supply execution evidence. No raw stream, Source copy,
credential, private reasoning or generated prose is committed. Repository-wide
aggregate validation, gate, official V11 and legacy validation were not run, as
requested. Production code and the polyglot capability contract are unchanged.
