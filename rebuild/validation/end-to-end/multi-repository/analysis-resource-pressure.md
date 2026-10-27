# Focused Analysis resource pressure

This is supporting local evidence, not V11 resource qualification. Budgets,
workload identity, graph coverage and historical receipts are unchanged. No gate,
Final, official V11 or provider transmission is part of these commands.

## Retained failure and restoration

The candidate is `792f791fcbd141891e856cd9e262d313fd9899cf`. Its V11 result
SHA-256 is `fafa43a303394e1528b0cbb81396e253716a85329118907f667877221ac0cf40`;
the per-call record SHA-256 is
`b99240ce1772cc2f3a386e450d3f2be669fe6ffa3695afc549e0e5af1121c95d`.
The 55 required functional leaves passed. Resource measurement was complete,
with zero sampling errors, but RSS was 4,500,410,368 bytes against
4,294,967,296 and journey duration was 930,609.855 ms against 900,000.
There were 505 MCP calls, 15 published manifests representing 8,074,535
inventory/entity/relation items, 616,113,089 logical storage bytes,
691,494,912 allocated bytes, and 524,598,031 post-warmup growth bytes against
536,870,912. Graph items sum retained snapshots; they are not unique graph
objects simultaneously held in process memory.

Original absolute gate paths no longer exist. The immutable local preservation
archive is `rebuild/.local/review-artifacts/review-evidence.tar.xz`, SHA-256
`3a87994cf8d2f95a2ca6583c24a5eadbea31285717fe0de3e546c49558f2862b`.
Its preservation instructions and compressed member manifest were inspected.
Only selected capsule, V11 result, per-call and subprocess result members were
copied under `rebuild/.local/analysis-pressure/evidence/`, with exact member
size/SHA-256 checks. No archive, original member or copied capsule was edited;
no Runtime was restored as executable state. The archive contains 82,870
members; selected-member verification is not a claim to reverify all members.

## What the collectors establish

`performance.py` samples current `VmRSS` and lifetime `VmHWM` every 50 ms,
with before/after samples. A later call can inherit an earlier high-water mark.
The first 4 GiB crossing was `mcp-011`, call 26, `materiality_review`, ending
at 4,415,647,744 bytes. The maximum 4,500,410,368-byte observation was during
`mcp-015` call 17, `checkpoint_record`; that call did not establish a new
before/after high-water mark. Neither observation establishes a leak.

RPC durations include transport/client decoding and the collector's post-call
manifest/storage inspection. They do not isolate Product CPU time. Total MCP
call time was 501,431.121 ms. Recorded CLI/setup subprocess durations summed
to 386,073.006 ms; the remaining 43,105.728 ms is unpartitioned journey work,
including orchestration. These are accounting categories, not CPU profiles.
The largest subprocess costs included derived repair (76,011.982 ms), corrupt
health inspection (48,559.187 ms), parser degradation (35,027.229 ms) and initial
analysis (33,279.341 ms). Provider/network and harness costs cannot be assigned
to an Analysis optimization from these records.

Storage observes completed lightweight manifests and referenced blob presence,
not decoded graph integrity. Corrupt/incomplete storage still counts toward
absolute/growth ceilings. Product readers independently own content/schema
integrity. Existing collector controls reject missing/error/NaN evidence and
all deliberate over-budget metrics. No reproduced collector defect currently
justifies changing RSS, duration or graph accounting.

## Focused reproduction before optimization

Run the existing `analysis_memory.py --repository-revision 792f791f...` through
`rebuild/scripts/validate focused`. It builds/installs release executables,
creates an isolated clone/Runtime, analyzes through CLI and MCP, records a
Discovery/Materiality Review, and inspects health. Setup is outside its reported
MCP duration. New diagnostic receipts retain setup stdout/stderr/exit/duration,
source HEAD/diff, tool/lock/executable identities, environment and limits;
failed MCP calls and process cleanup are retained too. `--self-check` verifies
actual successful and unsuccessful subprocess receipts without Product or
provider execution.

The initial uninstrumented run used the clean candidate and fixed source
revision above: seven calls, three snapshots, 67,236.571 ms MCP lifetime,
4,290,170,880-byte peak, zero sampling errors. Materiality Review was
26,770.248 ms and 4,181,016,576 bytes. Contract counts were 1,726 inventory
entries, 35,768 entities, 500,806 relations and 56,940,350 scalars;
capabilities were 16 available/18 partial/19 unavailable/23 unsupported.
The contract fingerprint was
`4353460316f8a23d380177fcac01ee383616a089364a7546575cd0dab820c558`.
Installed CLI/MCP SHA-256:
`3ad41e2ecae20fc0cd8cc60c2be75e279001ee10a8526bd62fe800dcc95b2cfa` /
`6ae3016c117fdde099b2fbf05ba6db03e908a6755257bdc059f746e940f9864b`.

A second, temporarily instrumented run measured 4,181,061,632 bytes and
25,462.114 ms Materiality Review. Its phase marks were not preserved by the
original diagnostic consumer; the unsuccessful attribution attempt is retained,
not treated as a phase profile. A separate instrumented CLI publication on a
copy of the first Runtime retained phase stdout/stderr and `/usr/bin/time -v`:
the 139,487,537-byte shape occupied a 268,435,456-byte allocation after its
compressed chunks were complete, while values occupied 374,727,071 bytes.
Unpacking the value base raised current RSS to 2,648,156 KiB; wait-time
`ru_maxrss` was 2,647,972 KiB. The small discrepancy and observed VmHWM
variation are retained. This is evidence of overlapping temporary allocations,
not proof that they explain the entire V11 overrun. Temporary instrumentation
is preserved only in ignored local files and is not production code.

`analysis_io::tests::resource_pressure_roundtrip` uses actual small polyglot
Product output by default. `VOLICORD_ANALYSIS_RESOURCE_MANIFEST=<exact manifest>`
selects the same producer/consumer test with retained large input. It loads a
live baseline/current pair, encodes with the actual reusable value base,
fingerprints produced bytes, reconstructs the full graph three times without
read caches, rejects corrupted values through both readers, and checks that
the original manifest is unchanged. Full streamed graph fingerprints cover
Source/snapshot identities, graph content, coverage and diagnostics, not just
counts. Separate test processes cover restart; existing storage/Host tests own
fresh-source refresh and downstream consumer behavior.

Before changes, the normal test passed; all 14 Analysis I/O tests passed. The
uncached large input manifest SHA-256 was
`900448c63e3a50fff6b11cd58b7c09ff898207bfa7f8ed01cf7af891b2d64cdb`.
It measured 4,351,264 KiB HWM after baseline/current loading and 4,500,556 KiB
after repeated durable reads; independent `/usr/bin/time` reported the latter
exactly, with exit 0. Functional success is separate from its excessive
resource observation. The release test executable SHA-256 was
`4ecdeb6c359b8ab91e8dd2f14d83e55c30240a269653c35e5ec86bfd8c6d972a`.

## Isolated allocation changes

The read-side change alone bounds `packed_values` and decoded delta ownership
inside value reconstruction, and releases packed base bytes after unpacking.
Previously a 260,123,085-byte decoded delta remained live while the reconstructed
JSON and typed graph were allocated. On the same cached large input, its removal
reduced repeated-durable-read HWM by 254,148 KiB (260,247,552 bytes). The writer
then releases normalized shape immediately after chunk compression and normalized
values after value packing/delta selection. The 268 MiB shape capacity is not
all resident: the observed writer HWM reduction is approximately its 139 MB
used/touched region, not its entire reserved capacity.

| Same retained delta input, KiB VmHWM | Before | Reader only | Reader and writer |
| --- | ---: | ---: | ---: |
| Two typed graphs loaded | 2,970,296 | 2,970,256 | 2,970,208 |
| Encoding with live baseline | 4,038,000 | 4,037,964 | 3,901,676 |
| After three durable reads | 4,547,612 | 4,293,464 | 4,293,516 |

All runs returned exit 0 and identical input/manifest, value and metadata hashes:
input and manifest `f9c7241509ccceaa6016736573da766bde87730d328ae60a66c1eef4456777da`,
values `d6255cec8a919b6dcdc68fc3405ff47b35d4311dac42280465cbd72ed47fd4a9`,
metadata `c8287936f227634dd695ffcc362d1e8332ad5a04b6962d116e7c9e861796c246`;
4,263 shape hashes are themselves bound by the identical manifest. This also
covers the exact 1,185,655,399-byte JSON representation and full graph meaning.
The independent wait-time HWM agrees with final process marks in the retained
receipts. Compiler cost in `time cargo test` is not used as Product latency.

The test executables are preserved under ignored `analysis-pressure/binaries/`:
before SHA-256 `4ecdeb6c359b8ab91e8dd2f14d83e55c30240a269653c35e5ec86bfd8c6d972a`,
reader-only `69bd9871231deb6bb3f7e352e5dffbba468488d35b1203cb888f7523508f1930`,
both changes `6dc376baef97b00c5a29604edf74cfa37363977564b13f9f97ad11ca5ca6adf3`.
The source basis is `050db9aab991d61dc8697ba65db83a663b2c3203`, respectively
plus no diff, reader diff SHA-256
`1a6e0257cc3e201d47a488274681188e9ae4a3944515d86b6aad2f01a0939586`,
or both-changes diff SHA-256
`49d1492125a41556e92109ba12aa282cfc0c4c34c2e30fea21fc5b9d68a45b0a`.
These large synthetic read/use/write processes still exceed 4 GiB: they are
functional/integrity controls with measured resource pressure, not a V11 pass.

The seven-call installed MCP workload is held at repository revision
`792f791fcbd141891e856cd9e262d313fd9899cf` and the same host/build/filesystem.
The second uninstrumented pre-change run measured 4,180,795,392 bytes and
65,577.415 ms; first post-change run measured 4,041,138,176 bytes and
71,857.990 ms. All graph/capability/diagnostic counts and contract fingerprints
matched the original run. The post-change CLI/MCP hashes were
`97644b30cfdae2a8d11c9135300127c244064e153c27d333445f4d914170f917` /
`a8c4bfa748e14221c3ac03a35216fd98abc3ce2590c3d026215b4dd11327f8c9`.
The exact dirty code diff is the both-changes diff above; after-first `build.json`
records it independently. This observed memory reduction supports the allocation
change. Timing worsened in this sample; no journey speedup is claimed.

## Repeated requests and restart

A bounded local consumer of the existing MCP/Collector performed four
`repository_understanding`/`project_health` pairs in each of two fresh processes
per executable, against the same retained three-snapshot Runtime. Before and
after each comparison it checked immutable manifest byte hashes. All eight
complete response fingerprints were equal across repetitions, restarts and
executables; all health reads were healthy, cleanup exits were 0 and resource
sampling errors were 0. Peaks were 2,935,345,152 / 2,886,012,928 bytes;
durations were 81,503.940 / 86,937.726 ms (before/after). The script, every call
and partial/final results are retained under ignored `analysis-pressure/`.
These observations cover the actual read workload/process lifetimes only and
do not prove absence of a leak at arbitrary request counts. They again support
lower memory pressure, not a speedup. Normal small polyglot roundtrips,
changed-input/restart/history/cache-corruption storage cases and downstream
Product consumers are separately exercised by maintained Rust tests.

## Environment and interpretation limits

Linux x86_64 WSL2 kernel `6.18.33.2-microsoft-standard-WSL2`, Python 3.12.3,
Cargo 1.97.1, rustc 1.97.1; release profile, same filesystem and fixed cloned
source revision. Local raw receipts are under `rebuild/.local/validation/`,
`analysis-memory/` and `analysis-pressure/`. The focused test/source tree has
test-only changes relative to the clean original executable; it is not a
second historical V11 candidate. Independent CLI publication uses a copied
Runtime and an explicitly instrumented executable, so its numbers are not
compared directly to an uninstrumented full journey.

The retained journey lacks allocator stacks, live-object accounting, per-phase
CPU/I/O traces and host-load history. RSS after a call may include reusable
allocator pages; increasing lifetime HWM alone cannot distinguish that from
retained live graphs. No full V11 resource qualification is established here;
it remains unverified until the authoritative gate runs on a committed candidate.

## Focused validation of the change

Every named check used `rebuild/scripts/validate focused`; numeric exits were 0
and complete streams/results remain in its ignored directories. After the
allocation change: Operations library 30 tests and storage footprint 4 tests
passed; the selected real MCP Work/Question/Decision/readiness consumer passed;
CLI analysis/Recall/portable I/O consumer passed; static and degraded Viewer
snapshot consumers both passed; the seven-structural-language/three-semantic-
ecosystem observation-order/current-binding test passed. Operations library/test
Clippy with all features and denied warnings, nested-workspace formatting and
metadata passed. Metadata confirms all nine packages/dependencies are within
the replacement workspace or external dependencies, with no legacy crate path.
Diagnostic failed/successful setup receipt checks and existing resource collector
negative controls passed. Exact retained-large serialization/graph, corruption,
repeat-read and process-restart controls passed as described above.

Workspace-wide Clippy/tests, Final, gate and official V11 were intentionally not
run for this focused task. No RSS/time/storage limit, collector formula, language
capability, graph entity/relation, Source evidence or schema was changed.

## Integrated retained-baseline reader regression

The later clean candidate `ab6b364fa4929513afb04b579d4ceb7b1254a617` was processed
through the authoritative gate. Its 55 functional leaves, Final, rehearsal,
provider, preflight and credential audit passed; V11 qualification failed solely
for 4,386,426,880-byte MCP RSS against 4,294,967,296. Duration was 850,746.920 ms,
post-warmup growth 525,526,343 bytes, and sampling errors zero. The preserved
candidate-bound archive independently verified with its original failure intact.
The first 4 GiB crossing was `mcp-011` call 27, `materiality_review`; the maximum
was call 30, `checkpoint_record`. These are process high-water observations,
not isolated allocation attribution or a leak diagnosis.

The ordinary cache-miss reader reconstructed an entire JSON allocation before
typed decoding. Reading a current graph while retaining a baseline reproduced
the pressure independently. `resource_pressure_roundtrip` now offers explicit
`VOLICORD_ANALYSIS_RESOURCE_PAIRED_READER=1` support: the copied publication has no
cache, three normal reads must preserve the full graph fingerprint, and both
normal/durable readers must reject corrupt content without changing the input.
The default mode still checks three complete durable history reads.

On the same retained input, the paired-reader process peak fell from 4,479,024
to 3,886,124 KiB VmHWM; test-body durations were 39.51 and 41.74 seconds. Compiler
time is excluded. Both runs returned 0 and retained identical input, manifest,
values, metadata and all 4,270 shape identities. Input SHA-256 is
`35558ed24dcf542f41ff17a9e8331c2c197dcdc5f161eaf23a58ff6e6a4a11fe`;
manifest SHA-256 is `4323908c98c979072222630d387f6eafc9d8ea6d148f9cfe623fccbf619c7400`.
Complete streams and the pre-change executable/diff are retained under ignored
`grounded-review/` and the focused `analysis-paired-reader-before/after` receipts.

Normal cache misses now decode the existing normalized representation through a
bounded buffered stream with an exact-length overflow sentinel and EOF/schema
checks. Explicit per-snapshot history inspection retains bulk decoding, preserving
the health/repair cost boundary. Cache policy, complete graph content, durable
bytes, identities and corruption checks remain the same. The observed reduction
qualifies this focused correction only; the changed clean candidate must still
pass the authoritative gate, including its original RSS and duration ceilings.

## 2026-10-28 publication cost attribution

This correction starts at clean `97d87c6f8b3d13fdbcf12dc15d5a1899c155b3f5`.
The reviewed failed candidate remains `774aad5ac2a46fe449f0291dbea9b77881e00f4c`.
Its actual original V11 result and per-call logs were found under
`rebuild/.local/validation/20261027T181302.148992Z-v11-gate-2zvh519h/official-v11/`.
Their SHA-256 values are respectively
`192550dee1a8ee05f27fbde7423e4469d61a69bc173d457b64eefacb0c92c57f` and
`84d59c49f0694389d13b38c4514efa2d2e86dec35ea38d163da10755a8d1abbb`.
No original measurement, Runtime, shared context or design reference was changed.
All reference manifest entries and the supplied ZIP hash verified. The current
native Reader was checked using the actual baseline release Viewer before the
comparative measurements; repository/Work, exact owning location, native code
entry/return/history, direct file and live-no-JS journeys passed at 390px.
This is technical support, not renewed design or human acceptance.

### Original process and timing boundary

The original 55-leaf, three-target workload identity is
`three-target-installed-journey-volicord-three-work-1`. Its wall timer encloses
all three target journeys, including setup, CLI, provider/host and driver work.
Duration was 953,404.898 ms against 900,000. MCP RSS is the maximum observed
Linux process lifetime high-water/current RSS, sampled before/after calls and
every 50 ms, across separately identified MCP processes; process peaks are not
added. The first 4 GiB crossing was `mcp-011` call 26, `materiality_review`.
The maximum was 4,315,680,768 bytes at `mcp-015` call 17, `checkpoint_record`,
which established a new high-water mark. Its sampled current peak was
4,283,150,336 bytes: a coarse sampled current peak is not the lifetime HWM.

Of 526,589.315 ms recorded MCP call time, Materiality Review accounted for
145,573.098, repository analysis 141,048.566 and Checkpoint recording
140,261.266 ms. Calls include transport/decoding and collector storage scans.
Sequential subprocess receipts sum to 379,922.200 ms; 46,893.383 ms remains
unpartitioned. Largest subprocesses were derived repair (81,340.430 ms), corrupt
health (53,380.646), parser degradation (37,842.673) and initial analysis
(35,802.220). The original records do not partition Product CPU, allocator
ownership, provider wait or host load. None is assigned an invented share.
Catalogue iteration and HTML growth are not implicated by this evidence.

Storage still uses the original first completed-manifest warmup basis and counts
all retained physical/logical files. Original post-warmup growth was
531,003,440 / 536,870,912 bytes. No timer, warmup, sampling, graph scope, threshold
or qualification collector was changed.

### Hypotheses and discriminating observations

Before selecting a correction, `resource-correction-20261028/measurement-plan.json`
fixed three hypotheses: overlapping publication scratch, generation/rebinding
cost, and driver storage-inspection cost. It fixed the original completed graph,
source revision, release profile, sequential paired sampling and retention of all
attempts. `perf`, `heaptrack`, `valgrind` and `strace` were unavailable. Temporary
stage marks used `/proc` memory/CPU/I/O and timestamps; their patch and original
receipts remain ignored. Production instrumentation was removed before comparison.

An instrumented CLI analysis on a disposable copy of the original Runtime measured
generation 11.501 s / 11.44 CPU s, normalization 6.959 s / 5.75 CPU s and cache
publication 6.194 s / 6.46 CPU s. These are stage observations on WSL2, not an exact
partition of the older journey. In the retained two-graph producer/consumer, RSS
rose from 3,908,880 to 4,215,480 KiB when the complete base was unpacked for delta
construction. This reproduced a specific overlapping allocation; it does not
establish a leak or localize every original RSS contributor.

The selected Product correction stays in Analysis publication:

- Stream the verified packed base in 64 KiB reads with at most 32 KiB decoded
  chunks and a 63-byte rolling window. Current-chunk candidates, seen base hashes,
  matches and operations scale with input chunk counts and are discarded within
  the call. Exact first-base selection, SHA lookup followed by byte comparison,
  global chunk boundaries, full EOF/length checks and raw/compressed fallback stay
  unchanged. No complete decoded base remains alongside both graphs/current values.
- Batch MessagePack writes through a 64 KiB buffer before compression, propagating
  drain errors before publication. Draining does not force an extra compressor
  flush. The direct control observes over 100,000 original small writes and bounds
  buffered writes by payload bytes / 65,536 plus one.

There is no cross-request cache, new body retention, publication/Project/language
substitution, authority bypass or format change. Analysis integrity and current
selection remain with their existing owners. Canonical observations, currentness,
retained publications, Source permissions and exports retain their existing paths.

### Equivalent-input results and uncertainty

The existing retained-graph roundtrip used immutable manifest SHA-256
`eaac0e8255eaddf8b576cbfb900de087a2c6ad57941d814932153ca1388f54f3`,
1,259,187,404 logical JSON bytes and 60,448,393 scalars. Both normal release test
processes loaded two graphs, encoded with the existing base, published a cache,
repeated complete reads in one process, and rejected corruption. Both returned 0.
All normalized output bytes were identical: manifest/input hash above, values
`2ddfb73dbabb53b2dedf7aba9470d8a6eb3c3acf9c8bf8fcde95ae8ab98f8ff7`,
metadata `b34da753a725f932bea6616a6c9cf57763b4f325a0e71b7fefdd4a83ff58d655`,
and all 4,525 shape identities bound by that manifest. Full graph fingerprints
include every identity, Source, relation, capability, provenance and diagnostic.
Process HWM was 4,176,756 → 4,072,684 KiB; test-body wall time 51.84 → 50.42 s,
user CPU 50.64 → 49.27 s and system CPU 3.65 → 3.34 s. This single pair is a
mechanism/semantic control, not a distribution or public-call latency estimate.

The public comparison reused the maintained seven-call `analysis_memory.py`
workload with supplied, frozen, uninstrumented release binaries. Every sample
locally cloned exact source revision `774aad5a`, initialized a disposable Runtime,
ran the original CLI analysis, then initialization/discovery, context, MCP analysis,
Engineering Choice Discovery, Materiality Review and health in one MCP process.
CLI setup remains separately timed; it was not removed from the focused wrapper.
No build, browser, export or other task-owned benchmark ran during these samples.
Order was AB, BA, AB; all six attempts are retained, with exit 0 and zero sampling
errors. OS caches were uncontrolled. Source counts, supported/degraded language
states and diagnostic contract fingerprint were identical across all samples:
`60bfca00327f6643139f450622983c69c2fd82c100fde585ccfdf7e67fa75423`.
Each retained three snapshots: 1,863 inventory entries, 37,304 entities, 534,484
relations and 60,448,229 scalars in the selected graph.

| Order | Side | MCP lifetime ms | Review ms | Review process HWM bytes | Whole MCP peak bytes | Wrapper ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | before | 64,377.561 | 24,748.002 | 4,084,867,072 | 4,162,048,000 | 101,601.274 |
| 2 | after | 62,683.434 | 23,884.306 | 3,956,162,560 | 4,161,904,640 | 95,560.533 |
| 3 | after | 62,070.352 | 23,409.200 | 3,956,469,760 | 4,162,281,472 | 94,638.258 |
| 4 | before | 65,811.119 | 25,215.890 | 4,084,629,504 | 4,161,642,496 | 99,732.910 |
| 5 | before | 65,758.365 | 25,043.878 | 4,085,039,104 | 4,161,712,128 | 99,672.712 |
| 6 | after | 62,847.224 | 23,882.372 | 3,956,338,688 | 4,162,035,712 | 95,648.647 |

Every Review established a new HWM, so its lower peak is attributable to that
actual public call, not an inherited unrelated mark. Later `project_health`
history inspection established every whole-process peak: that metric did not
improve. MCP lifetime medians were 65,758.365 → 62,683.434 ms, with all individual
values above retained. This supports a bounded local time/Review-memory benefit,
not a whole-V11 savings projection. Three pairs establish neither p95 nor robust
qualification. Health/repair and long-lived repeated Work/Checkpoint cost remain.

Own-process CPU (user + system) was 65.83/67.64/67.34 s before and
64.36/63.70/64.56 s after; child CPU was separately recorded at 0.12–0.13 s.
The new slice invokes no provider. Physical reads were 0–4,096 bytes with cached
inputs, while rchar/wchar and physical writes were retained independently.
Driver storage scans cost 787.093–815.467 ms total per seven-call sample and
825.409–852.521 ms of driver CPU. These supplements do not alter the collector's
reported durations. WSL2 clock/accounting and allocator variation remain visible;
no environmental explanation for the old failure is claimed.

Logical storage ranges overlap: before 217,588,265–218,460,650 bytes, after
217,744,548–218,148,819; allocated ranges were 233,897,984–234,770,432 and
234,053,632–234,459,136 bytes. Fresh Source/time identities make these separate
legitimate publications, not identical storage byte sequences. Exact-byte equality
is instead established on the immutable graph above. Full cache consumers preserve
the entire decoded graph; the direct smaller cache control also pins its compressed
bytes. No universal compressor-byte or storage-growth improvement is inferred.

All raw receipts, failed controls, immutable executable copies, environment/tool
versions, original stage patch, comparison scripts and every sample remain under
ignored `rebuild/.local/resource-correction-20261028/`,
`rebuild/.local/analysis-memory/resource-20261028-*` and focused validation outputs.
The host was Linux x86_64 WSL2, Cargo/rustc 1.97.1, Python 3.12.3; exact environment
and executable identities are retained in those receipts. No raw profile or Runtime
is committed. Normal/negative consumer validation is recorded with the change.
Official V11, Final and repository-wide aggregates were deliberately not run;
the original resource failure remains authoritative for its candidate.

### Scoped validation and remaining verification

All following checks ran through `rebuild/scripts/validate focused`, with full
streams and numeric exit 0 inspected under its ignored output:

- Release Operations library (37 tests), analysis recovery (8), storage footprint
  (4) and Work authority (75). These include restart, immutable history, corrupt
  and non-current input, wrong Project/header substitution, fresh observation,
  failed Source recording and real baseline/Review/Checkpoint transitions.
- Release Viewer citation reading (30) and navigation (6). Complete original
  citation bytes, normal/negative Project/publication/language bindings,
  same-wording republication, concurrent reads, Source denial/deletion, ordinary
  expiry/cleanup/restart, retained reading and native location remain covered.
- Release Host `mcp_workflow_guides_material_question_to_explicit_decision_and_ready_work`
  through the actual public adapter (one selected test).
- Operations release library/tests Clippy, all features, `-D warnings`; nested
  workspace formatting; maintained workspace-boundary/metadata check (nine
  packages under `rebuild/`, no legacy dependencies, unchanged ignored context).
- `analysis_memory.py --self-check` (successful/failed setup receipts and supplied
  executable rejection controls) and the unchanged `performance.py` negative
  controls for sampling, completed-manifest warmup and storage accounting.
- Actual baseline and candidate release native browser consumers: selected Korean
  repository and Work at 390px, live-no-JS, byte-identical snapshot HTTP delivery
  and direct files, section/figure/claim/code entry, exact owning location, return,
  reload/history and unknown-target handling. Candidate files were newly exported
  through the ordinary CLI. These are scoped navigation checks, not a new visual
  matrix or human/assistive acceptance.

The first new cache control failed because `BufWriter::flush` forced an extra
compressor block; the corrected drain preserves the direct reference bytes and
propagates sink failure. Its failing receipt is retained. The initial browser
attempt was blocked by sandbox loopback `EPERM`; a separate authorized local
attempt passed without external requests. Neither failed attempt was overwritten.

Post-commit checks are reported in the task conversation. The exact candidate's
whole gate remains unrun: the authorized scope excludes Final/official V11 and
external replay. A later separately authorized verification must retain all target
journeys and original ceilings, and inspect long-lived Review/Checkpoint peaks,
health/repair CPU/RSS and the narrow storage-growth bound. The local improvements
do not establish that the 953.4-second/4-GiB failure has been fully corrected.
