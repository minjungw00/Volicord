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
