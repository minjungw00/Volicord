# V08 — Clean Linux and Codex integration

## Status

Passed against the maintained viewer/host baseline and the three corrective
Production commits `55271418ea9f7b621a31250bf086194e7ac92dfd`,
`ecef64e1a3516f4a1aa2ceaaebcc8b84f8b60183`, and
`369402c6065232b4ef0a0534340b1b2a447436ad`. The 61-requirement matrix now
covers the real viewer listener, runtime-discovered MCP schemas, supported
derived-analysis recovery, and an authenticated Codex product-tool turn. The
corrected deterministic evidence passed, and the authenticated Codex
product-tool probe passed. The reconstruction final aggregate has not yet been
run and is not claimed by this report.

## Goal

Verify the clean Linux installation and replacement Runtime Home journey at
the actual user/client boundaries: loopback HTTP viewer, installed MCP stdio
server, CLI recovery commands, and a bounded model-authenticated Codex turn.

## Accepted decisions being validated

Q1–Q5 require a Project-oriented local-first product, Codex integration,
source-grounded Recall, selectable explanation depth, and useful projections.
Q6–Q8 require correctable canonical memory and exact Guarded confirmation.
Q10–Q12 preserve non-canonical Candidate state, current-host Source linkage,
and single-use confirmation. The failure owner requires corrupt Derived State
to degrade observably and rebuild from authoritative input without changing
canonical judgment.

## Input repositories and revisions

The clean harness creates self-authored temporary README, Python, and Go
repositories. The viewer executable test creates a temporary Python Project.
The authenticated Codex probe creates a separate one-file repository and a
clean isolated Codex configuration/registration home. Production behavior is
fixed by the six commit identities in `v08-matrix.json`; the last three are the
corrective viewer, MCP schema, and analysis-recovery commits listed in Status.

## Environment and tool versions

Observed on Linux `6.18.33.2-microsoft-standard-WSL2` x86_64 with `rustc
1.97.1`, `cargo 1.97.1`, Python `3.12.3`, and `codex-cli 0.145.0`. The installed
CLI exposes bounded noninteractive `codex exec --ephemeral --json`, read-only
sandboxing, global approval policy, and MCP-specific tool approval modes. The
probe used the documented per-tool `approve` mode only for read-only
`volicord.project_health` in its isolated home.

## Candidate approaches

The maintained evidence uses black-box HTTP, stdio, CLI, and Codex event
observation. Rust test support creates precise canonical and Guarded fixture
state without adding a second product implementation. Python interprets only
the schemas returned by `tools/list` and orchestrates installed processes.

## Commands and configuration

```text
rebuild/scripts/validate focused codex-cli-help -- codex --help
rebuild/scripts/validate focused codex-exec-help -- codex exec --help
rebuild/scripts/validate focused codex-login-status -- codex login status
rebuild/scripts/validate focused viewer-executable-http -- cargo test --manifest-path rebuild/Cargo.toml -p volicord-viewer --test executable_http --all-features
rebuild/scripts/validate focused v08-corrected-harness -- python3 -B rebuild/validation/linux-codex-integration/harness.py
rebuild/scripts/validate focused v08-real-codex-product-tool -- python3 -B rebuild/validation/linux-codex-integration/codex_probe.py
rebuild/scripts/validate focused v08-assertions -- python3 -B rebuild/validation/linux-codex-integration/assertions.py
rebuild/scripts/validate focused v08-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v08-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/linux-codex-integration/report.md
```

The successful Codex child command is preserved exactly in the focused probe
log. It uses an isolated copied authentication file, isolated registration,
read-only sandbox, no interactive prompt, and the narrow configuration override
`mcp_servers.volicord.tools.project_health.approval_mode="approve"`.

## Observed results

The real viewer executable opened its loopback listener. Separate requests
selected `overview`, `working`, and `deep`; a canonical Context Item created
after process start appeared without restart. A correction submitted through
HTTP became revision 2 and was confirmed through a separately opened Local
Operations read. A revised Guarded request rendered its current exact identity,
revision, and fingerprint. Stale and mismatched responses were rejected, one
exact response succeeded with a canonical current-host Source, and reuse was
rejected. Unsupported paths, explanation values, and form fields left the
portable canonical bundle byte-identical.

The installed MCP process advertised 14 high-level tools. Every advertised
shape was a closed object with typed, described properties and explicit
required fields. The maintained client-side interpreter constructed Recall,
Checkpoint, and Guarded calls from those returned schemas. All three executed;
the Guarded response preserved exact identity and revision. Across the full
catalog, 29 missing-field or additional-property calls were rejected by the
advertised contract. Production Rust oracles independently match every schema
shape to handler-consumed fields and verify invalid requests fail before
mutation.

The CLI recovery journey established two Projects and analyses, corrupted only
one Project-owned analysis file, observed `degraded` plus the exact corrupt
scope, and repaired it from current repository/canonical input. Health returned
to `healthy`. A later `reindex` created a different Analysis Snapshot containing
the newly added source file. Portable canonical bytes remained identical after
both operations; the unrelated Project analysis remained byte-identical; the
Project directory retained one current snapshot. Unsupported canonical repair
failed and changed none of the compared state.

The authenticated Codex event stream showed the model select
`volicord.project_health`, construct `{"project_id": ...}` from tool metadata,
complete the MCP call, receive `connection: connected` and `capability_state:
healthy`, and report those values. This is model-driven product-tool evidence,
not registration, startup, or a manually constructed stdio RPC.

## Coverage and failures

Final focused results used for this report:

- real viewer executable: passed in `498.833 ms`;
- deterministic clean/MCP/recovery harness: passed in `345.362 ms` with warm
  release artifacts;
- authenticated Codex product-tool probe: passed in `13,839.621 ms`.

The authenticated probe requires a completed event with `status: completed`,
no tool error, and an actual structured result; selection or startup alone does
not pass.

## Performance and resource observations

The deterministic fixture is intentionally small and does not establish
large-repository latency or concurrent-client limits. Viewer requests completed
inside a five-second socket bound. Host EOF cleanup completed inside ten
seconds. The authenticated turn completed in about 13.7 seconds at the child
boundary. Complete stdout/stderr, exact argv, duration, exit/termination, and
wrapper result remain in ignored focused-validation artifacts.

## Privacy and external transmission

Deterministic viewer, MCP, and recovery checks sent nothing externally. The
authenticated Codex probe necessarily sent its explicit Project-health prompt,
advertised tool metadata, and returned health result to the authenticated
OpenAI model service. Its repository contained only the self-authored README;
no background semantic provider was enabled. The copied authentication file,
Codex configuration, Runtime Home, installation prefix, and repository existed
only in the temporary probe directory and were removed afterward. The bait
legacy Runtime Home remained byte- and timestamp-identical.

## Acceptance results

- Pass: real HTTP requests exercise current viewer state, all three explanation
  levels, memory correction, exact Guarded fallback, stale/mismatch/reuse
  rejection, and malformed-request no-mutation.
- Pass: `tools/list` exposes concrete closed schemas for every public tool;
  representative read, mutation, and Guarded calls are constructed from those
  schemas and execute successfully.
- Pass: missing required fields, malformed values, and additional properties
  fail consistently before mutation.
- Pass: authenticated Codex selects and completes a Volicord read-only product
  tool from the isolated registered server and returned schema.
- Pass: repair and reindex recover only replacement-owned derived analysis,
  preserve canonical bytes and another Project, and reject unsupported scope.
- Pass: clean install, lifecycle, degradation, process cleanup, canonical
  preservation, and legacy exclusion remain intact.

## Known limits

V08 remains a small Linux integration fixture. It does not qualify multiple
concurrent HTTP/MCP clients, abrupt power loss, hostile filesystem races,
large-repository ceilings, accessibility, other operating systems, or the V11
multi-repository product journey. The authenticated model result is
environment-dependent and must be rerun when credentials, network, Codex CLI,
or model-service behavior changes; deterministic workspace tests do not depend
on it.

## Recommended implementation choice

Retain the thin viewer and high-level reconstruction-owned MCP edge. Retain
closed tool schemas as the executable client contract and Project-scoped
derived replacement for repair/reindex. Keep the authenticated Codex probe
explicit and separate from ordinary deterministic workspace tests.

## Rejected alternatives and reasons

Not applicable. This validation evaluates current Production behavior and does
not select or discard an alternate implementation.

## Reusable primitive decision

`production_evidence`. The HTTP listener, Local Operations, MCP schema
validation, installer, runtime layout, and Project-owned analysis replacement
remain Production responsibilities. The matrix and Python probes are maintained
black-box acceptance orchestration only.

## Decision revisit trigger status

Not triggered. The corrected evidence preserves local-first operation,
Project-scoped canonical and derived authority, current-host Source provenance,
exact Guarded confirmation, and honest failure/recovery. Codex requires an
explicit MCP approval-mode configuration for a bounded noninteractive call;
that is a current client integration fact and does not change product meaning.

## Follow-up work

- Run the single exact reconstruction final aggregate only after the test commit
  is complete and the worktree is clean.
- Start V11/Phase 8 only if that aggregate passes and this maintained V08 gate
  remains supported.
- Keep Phase 9 cutover and legacy deletion outside this validation boundary.

## Artifacts

Maintained artifacts are this report, `assertions.py`, `harness.py`,
`codex_probe.py`, `v08-matrix.json`, the shared fixture-manifest entry, the real
viewer executable test, mapped host/operations tests, and the Phase 7 summary.
Focused command metadata and complete raw streams remain under ignored
`rebuild/.local/validation/`.
