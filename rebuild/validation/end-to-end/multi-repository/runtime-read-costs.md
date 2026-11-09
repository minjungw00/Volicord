# Record inspection materialization

The CLI record and Candidate commands returned canonical/Candidate fields but
requested the default complete Project projection. That unnecessarily restored
the current code graph and built its map. The correction requests the existing
metadata/inspection projection. It preserves record selection, every returned
record/Candidate value, admission and bounds, metadata/freshness and their issues.
`code_graph: not_requested` makes the integrity scope explicit. The two omission
notices for an unreturned map are no longer emitted. Full graph consumers and
`doctor check` retain their integrity responsibility.

## Controlled observations

The baseline is `5e68276ff079d8be41e40aa4eb575bb17868f80a`. Baseline and changed
executables used the same release profile/toolchain/lock and independent processes
against the same private Runtime copies. There were no competing task jobs.
OS page caches were uncontrolled: these are new-process observations, not claims
of cold disk I/O, a percentile, an SLA or a whole-gate guarantee.

| Input / command | Baseline wall ms | Corrected wall ms | Baseline peak KiB | Corrected peak KiB |
| --- | ---: | ---: | ---: | ---: |
| Smaller retained Work / records | 136.666 | 77.258 | 50,084 | 24,116 |
| Smaller retained Work / Candidates | 127.457 | 71.752 | 50,476 | 24,340 |
| Large accumulated analysis / records | 6,292.836 | 105.600 | 1,779,236 | 71,572 |
| Large accumulated analysis / Candidates | 6,172.520 | 98.114 | 1,778,504 | 71,180 |

Each cell is one preserved attempt. All eight processes exited 0. The comparison
checked all returned record/Candidate values and health, not just counts. On both
record inputs the only removed issues were the unreturned map's entity/relation
omission notices. Candidate output grew by the explicit integrity-scope field;
reducing output bytes is not the measured cause.

An additional request-profile comparison on the large retained input observed
canonical read 0.982 ms, graph read 2,541.423 ms and projection 3,072.400 ms for the
full request (5,925.396 ms request total), versus canonical 1.038 ms, metadata
60.539 ms and projection 13.695 ms (77.484 ms total). Graph decodes changed from
one to zero; metadata decodes changed from zero to one. No persistent cache or
authority/invalidation shortcut was introduced. The large original manifest
describes 37,427 entities and 537,619 relations; its complete graph/history remains
retained and independently readable by graph consumers.

Raw streams, numeric exits, executable hashes, resource observations and input
bindings remain under ignored `rebuild/.local/runtime-reading-20261107/`:
`baseline-01`, `materialization-01`, and `public-comparison-01`. The failed diagnostic
build/identity attempts remain separately preserved; they are not timing samples.

## Analysis and limits of attribution

A newly labelled release reproduction on the retained source tree observed fresh
analysis at 30,151.123 ms and accumulated analysis at 29,839.245 ms, producing
60,034,882 / 60,034,873 stdout bytes. Product-reported analysis durations were
28,393.339 / 28,037.708 ms. The remaining intervals include startup, CLI summary
construction/serialization, delivery to file and teardown; they do not isolate
those components. Both retained 63,293 diagnostics and 76 capability reports.
No causal analysis/storage change follows from these two observations.

The historical `7d27a0d4` gate passed. Its official-V11 payload directory is absent
locally; retained invocation records, capsule and archive cannot recreate missing
original phase telemetry. Neither the historical 80.75 s records-list observation
nor the 75.12 s analysis observation has been causally explained by this work.
Model investigation/authoring computation and tokens were not instrumented;
development-session or CLI duration is not a substitute. `perf` and `strace` were
unavailable. Lock wait is not separately measured by the request profile.

## Outcome controls

`volicord-operations --test cli` owns
`record_commands_preserve_records_without_materializing_or_certifying_code_graphs`.
It uses Python/Rust/TypeScript source, compares complete record fields with the
full projection and independently accounts for canonical identities. Corrupting
the durable graph leaves record/Candidate results unchanged while doctor reports
corruption. Corrupting metadata preserves canonical records and reports the
affected analysis scope. Reads neither repair damaged bytes nor mutate canonical
history. Existing Candidate empty/unsupported/corrupt/unavailable controls remain.

These focused outcomes do not establish exact-candidate qualification,
transferable explanation quality, human comprehension or assistive acceptance.

## Work composition and recording

New actual Work production observed 2.8–2.9 seconds in each composition/recording
call. Current-code validation restored a full graph solely to obtain repository
Source identity, snapshot identity and inventory. Repository articles already used
verified metadata for the same responsibility. Work now uses that reader too;
file hashes, identity bindings and fresh included-inventory comparison remain.

A fixed actual retained repository, separate from the edited Product checkout,
provided the controlled input. A new current-task investigation Goal and exact
observed `rebuild/Cargo.toml` Source were recorded normally in a private Runtime.
Two exact copies supplied the same preparation, canonical IDs, Source, body and
draft to baseline `10fe4ef7` and corrected release CLIs. Composition and recording
each ran once in a new process, with no competing task jobs. Complete realization,
evidence and Source statuses were equal; normal publication IDs/times stayed
independent. This bounded control is not a representative authored article or a
comprehension test.

| Public operation | Baseline wall ms | Corrected wall ms | Baseline peak KiB | Corrected peak KiB |
| --- | ---: | ---: | ---: | ---: |
| Work compose | 2,822.367 | 65.124 | 1,549,916 | 40,024 |
| Work record | 2,769.623 | 70.537 | 1,550,224 | 40,032 |

All four processes exited 0. Raw complete inputs/streams/resource observations and
binary identities remain in ignored `runtime-reading-20261107/source-validation/`.
OS caches were uncontrolled; the instrumentation and measurement windows were
identical. These samples do not predict model time, a percentile or gate duration.

The actual MCP regression first failed with `Analysis blob content identity is
corrupt` despite a permitted unchanged file and healthy metadata. After correction,
healthy and graph-damaged composition return the identical realization, recording
works, and doctor independently reports graph corruption. Damaged metadata still
rejects both compose and record without replacing prior publications. No graph,
manifest, history or storage format is altered. Explicit Work file/command capture
still restores its complete recorded worktree basis: the observed roughly 2.7-second
capture cost is separate and has not been removed or relabelled as model cost.

## Selected reads with accumulated retained bodies

The new bounded workload separates three axes before observation:

- Seven small structural-language files, one Work, one Analysis Snapshot and one
  exact file Source; 0/32/256 additional synthetic managed bodies, each 164,864
  bytes. These additional records are explicitly labelled synthetic, unselected
  retained-content controls, not authored articles or genuine Works.
- 1/8/32 selected synthetic Works plus the shared fixture's one distractor Goal,
  with eight Checkpoints per selected Work. Complete canonical values and every
  selected Work's eight records must survive process reconstruction.
- The existing `analysis_storage_footprint` inputs independently exercise 48-file
  graph storage, shared blobs, cleanup/restart, and eight repeated polyglot
  observations followed by a changed input. They preserve immutable manifests,
  complete historical graphs and Source-bound identities.

The evaluation criterion for these additional conditions is complete-result and
durable-history equivalence, explicit lifecycle failure, and observed memory/time
variation. No additional capacity ceiling or Product limit is established. V11's
4 GiB RSS and 512 MiB post-warmup budget remain exclusive to their named official
workload. None of the smaller controls qualifies those budgets.

The original exact Source reader called general Project privacy inspection, which
deserialized every managed body and all provider history before discarding Source
content. The corrected reader loads dependency metadata without bodies or provider
history. Publication selection fetches only the selected body after lifecycle
checks, rejects changed metadata and retains insertion-order replacement without
older-publication fallback. Source and code-origin readers retain their existing
exact identity, association, permission, expiry and integrity checks. No durable
format, index, cache, analyzer identity, graph, retention or history is changed.

Baseline release binaries were built from `98d8bbdb8`; corrected binaries use the
same compiler/profile/lock. The same retained 256-body Runtime and seven-file
repository were read alternately in three independent processes per binary, with
no competing task build/profile. Complete stdout bytes were identical in all six
reads, SHA-256 `eb7e6615a9aa40be852fc29daddc64f935cfa6af706c9f096596a5264c623342`.
Every pre-existing non-journal Runtime file retained its hash. Independent typed
reads also compared every original added record, including its full body.

| Same-input exact Source read | Wall ms, three attempts | Peak RSS KiB, three attempts |
| --- | --- | --- |
| Baseline | 31.316 / 25.197 / 27.243 | 53,020 / 53,168 / 53,064 |
| Corrected | 30.041 / 28.567 / 28.593 | 13,020 / 13,008 / 13,108 |

This supports reduced body materialization memory, not improved latency. SQLite
still scans JSON records and complete dependency metadata grows with history.
New-process, repeated and reconstructed Operations reads are separate observations;
OS caches are uncontrolled, so none is labelled cold-disk I/O. Build/setup is
outside per-read measurements. API complete-publication controls compare whole
page values before/after growth and after Operations reconstruction. The CLI
Source read is a full-result comparison, not a transport prefix or count check.

Raw declarations, commands, stdout/stderr, numeric exits, binary hashes, resource
observations and preserved inputs are in ignored
`rebuild/.local/accumulated-use-20261109/{baseline,corrected,same-input}/`.
The first compile failure (incorrect test field names) and canonical-axis assertion
failure (the fixture's declared distractor Goal) remain preserved under validation
receipts; both were control-authoring errors, not Product failures. These checks
are focused support, not the authoritative gate, naturalistic evidence, independent
quality review or human comprehension. The complete original gate still requires
current explicit transmission authority on the clean committed candidate.

## Work change attribution and observation dictionaries

The exact-candidate gate at `e95a1359a2808622b4e590a6be0c641cd9df40f2`
completed all 55 functional V11 steps but failed its named resource budgets:
MCP peak RSS 5,332,635,648 bytes exceeded 4 GiB, and post-warmup logical Analysis
storage growth 590,995,633 bytes exceeded 512 MiB. The original capsule, numeric
exit, performance calls and independently verified archive remain in ignored
`20261109T140904.499664Z-v11-gate-6rbthv4h`. This is an actual measured failure,
not an inference from historical headroom. Materiality Review and Checkpoint calls
established the largest process high-water values.

The declared frozen input has 38,214 entities and 582,448 relations in its first
complete snapshot, 1,466,390,284 serialized JSON bytes, and 15 retained Volicord
snapshots. The original second snapshot's values delta occupies 33,754,146 bytes.
Its complete reconstruction, identity and provenance are the equivalence oracle;
neither files nor required graph/history are removed.

Work attribution and authority consumers only use snapshot/Repository/Project/Source
identity, complete inventory, original Git/worktree observation and freshness.
`RepositoryChangeBasis` now names that exact scoped view instead of retaining a
complete pre-work graph throughout another complete analysis and publication.
The reader checks current format, manifest/blob integrity and exact bindings; it
never represents this view as an empty-graph Analysis Snapshot or certifies graph
meaning. Full graph readers keep their own typed validation. Discovery, Review,
executable-scope and Checkpoint predicates and current canonical authority remain
unchanged. Fresh Work observations analyze all supported inputs and publish the
complete graph. They can trade optional previous-graph structural reuse for lower
live memory; the explicit structural-refresh API retains its independent reuse
contract. No retention, admission, schema or graph/history limit changes.

An actual installed MCP `engineering_choice_discovery` draft read used the same
unchanged disposable Runtime and exact retained C Work baseline for both binaries.
It invoked no analysis, Source capture, publication or cleanup. Complete 13,840-byte
drafts were identical across all six calls, including provenance, authoring fields
and original Source identity, SHA-256
`bff9b122be29a4e112d819b60f63e6c3ab14934626d074d2dd61612d25032193`.
All five logical store hashes remained unchanged. Before/after binaries, requests,
responses and original timing/memory observations are retained separately.

| Installed MCP draft read | New-process / warm / restarted RPC ms | Process lifetime high-water KiB |
| --- | --- | --- |
| Original `e95a1359a` | 2,470.985 / 2,012.169 / 2,478.536 | 2,557,824 / 2,709,348 / 2,557,524 |
| Scoped Work reader | 1,225.984 / 1,211.999 / 1,232.588 | 960,404 / 992,760 / 960,336 |

These are declared same-input observations, not percentiles or whole-host memory.
Warm calls share a server; restarted calls use a new one. OS caches are uncontrolled,
so new-process does not mean cold disk. Setup and independent store hashing are
outside RPC timing and remain separate; process high water includes initialization.

A separate disposable Runtime copy kept all 15 original snapshots while performing
three complete Work-analysis/publication phases through the production scoped
helper. Complete serialized graph/provenance fingerprints of every original matched
before and after. Current full readbacks matched their returned change views.
Analysis/publication took 30,218 / 29,866 / 29,490 ms; full current-graph oracle reads
took 2,317 / 2,341 / 2,322 ms. Original-history oracles took 52,628 / 51,077 ms.
Whole-process high-water RSS was 3,329,256 KiB, including all oracle work; none of
that work is represented as model authoring time or hidden in a different process.
This is a supplemental analysis-phase control, not an official V11 replay or a
claim that its named MCP ceiling has qualified.

The storage correction stops preloading an earlier symbol dictionary into the
current snapshot's normalizer. Fresh observation identities can otherwise shift
reference runs throughout a large unchanged graph. Current first-seen ordering
keeps those runs reusable by the existing byte-delta codec. The frozen original
second snapshot encoded to 26,310,275 values bytes, with complete JSON/metadata and
repeated durable-read equality. Earlier current-format symbol orders still decode
through the same reader, and original manifests/blobs remain unchanged. Stronger
compression experiments on the unchanged original delta increased size and were
not adopted. No compression ceiling, alternate format or migration is introduced.

Raw declarations, immutable baseline test executable identity, full-graph oracles,
failed support attempts and controlled results remain under ignored
`rebuild/.local/accumulated-use-20261109/` (`v11-resource-*`, `draft-read-*`) and its
focused validation receipts. The source-byte identity of the large Work control
is recorded separately. Cache/no-cache equivalence and damaged-blob rejection
have normal Rust coverage. These observations justify scoped materialization and
symbol-order changes; the new exact clean candidate still requires its own gate.
Naturalistic conversations, independent quality review and human comprehension
are separate evidence and are not supplied by these controls.

### Changed-inventory scalar-slot locality

The next exact candidate `1939caf9e` passed all 55 functional V11 checks and the
4 GiB MCP ceiling (3,326,300,160 bytes), but failed the unchanged 512 MiB
post-warmup storage-growth ceiling: 629,613,201 bytes. Its total V11 wall time
was 852,647 ms against 900,000 ms. This original failure is retained; a fixed
snapshot encoding gain did not establish growth behavior after inventory changes.

The normalizer now optionally plans existing scalar slots using the verified
complete base's first lexical owner/field and scalar category. Exact current
tokens retain their slots; fresh tokens reuse obsolete slots in the same group,
then append. This prevents a new file hash from displacing observation-identity
references through a large graph. The lexical groups infer no semantic identity.
All references are checked and remapped to the exact current scalar. The same
persisted representation and single decoder read every earlier ordering. Original
manifests, blobs, inventories, complete graphs and history remain unchanged.

A declared changed-inventory snapshot from the disposable 15-history control
used 40,583,485 values bytes before this correction and 26,622,611 after it.
Complete JSON and metadata matched, three independent durable readbacks matched,
and corruption was rejected. That encoder control deliberately retained two
complete graphs; its 4,275,456 KiB whole-process peak is reported separately and
does not claim the official MCP ceiling. Input declarations, failed alternatives,
timings and raw outcomes remain under ignored `symbol-plan-*` artifacts and
focused validation receipts. Full Work-refresh and exact-candidate gate results
remain separate evidence; no universal capacity or time guarantee follows.

The subsequent disposable copy of that failed candidate's complete 15-snapshot
Runtime produced three new complete observations (38,260 entities / 582,917
relations each). Analysis/publication took 31,991 / 32,604 / 32,521 ms, with
26,623,982 / 26,625,632 / 26,616,635 values bytes. Current full-graph readbacks
matched their scoped views. All original manifest bytes and complete serialized
graph/provenance fingerprints matched before and after; the original Runtime
was not edited. Whole-process high-water RSS was 3,299,540 KiB, including the
independent full-history oracles. These changed-input observations are not a
same-input speed comparison with the earlier Work control or a new capacity
limit. The exact clean candidate still needs the unchanged official gate.

### Native failure controls at the reader's process boundary

The `ccee86644` gate passed admission, metadata, format and Clippy, but its
Viewer test process terminated with `SIGSEGV` (execution receipt `-11`, outer
exit 245). Final failed; rehearsal/provider/V11 did not run. The first native
worker `signal:6` log was an intentional fault fixture, not this root outcome.
All original streams, compiler/execution identity and the failed capsule remain.

Three bounded ordinary exact-binary runs and three address-sanitized runs passed.
A later default-parallel debugger run reproduced the native fault with a damaged
stack in string processing; the underlying native defect remains unidentified.
LeakSanitizer's unsupported sandbox run and the debugger's initial SIGPIPE stop
are preserved as unsuitable diagnostic attempts, not successful memory evidence.

The failure controls previously constructed and disposed damaged native VMs
inside the reader test process. They now exercise the already maintained
disposable-worker contract: actual throws/jobs/interrupt and all original bounds,
geometry rejection, explicit child reaping and fresh successful geometry. Positive
handled-rejection reuse and host restrictions remain required. The Rust-handle
probe remains inside the interrupt child when native execution returns; a native
signal cannot claim that probe passed. The renderer and production process
ownership are unchanged. This corrects the fixture's execution boundary; it does
not claim a causal native-memory repair, raise a ceiling or erase the Final failure.

Further controls rejected two dependency-update alternatives: the 0.14.0 copy
failed a valid mixed-script layout after three complete passing repeats, and a
0.12.0/core-sys-0.12.2 copy failed recovery with protocol rejection. Neither was
promoted; the maintained Rust 1.85 and exact 0.11.0 family/notices remain unchanged.
One hundred fresh initializations each in the original 2 MiB test thread, an
8 MiB diagnostic test thread and the installed release main all passed. This did
not explain the broader failure and justified no host-stack override.

The isolated same-source development profile with only `rquickjs-sys` optimized
at level 2 passed ten complete default-parallel Viewer runs (39 active controls
each). The selected correction keeps that package optimized in dev/test while
retaining all original native bounds and the rest of the Rust development
profile. Four declared synthetic inputs (chain, cycle/parallel, 12-node/20-edge
maximum and isolate/self) each produced eight complete ordered responses in
both original and optimized binaries. Every JSON field matched, including GWT
`$H` counters. An earlier private oracle incorrectly required warm counters to
equal the first request; its failure and the corrected full-sequence oracle are
retained. No field was normalized or omitted. These are bounded controls, not
proof of the unidentified native defect's causal repair or a replacement for
the new exact candidate's gate and installed-reader checks.
