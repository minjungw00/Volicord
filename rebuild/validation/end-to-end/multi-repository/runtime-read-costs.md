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
