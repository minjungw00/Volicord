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
