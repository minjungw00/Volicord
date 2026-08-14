# V08 — Clean Linux and Codex integration

## Status

Passed against the eight Production commit identities fixed by
`v08-matrix.json`, ending with `5c20f53a1aa7c0cf64767a3c10e54c0b719f5d6a`
and `3b48545bd9e2a224d6feb75ae1c743d1af31f4cf`. The 77-requirement matrix
covers the real viewer listener and request-authenticity boundary,
runtime-discovered MCP schemas, repository-Source-grounded analysis recovery,
and an authenticated Codex product-tool turn. The deterministic evidence
passed, and the authenticated Codex product-tool probe passed. The
reconstruction final aggregate has not yet been run and is not claimed by this
report.

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
fixed by the eight commit identities in `v08-matrix.json`.

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
rebuild/scripts/validate focused v08-provenance-trust-harness -- python3 -B rebuild/validation/linux-codex-integration/harness.py
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

The real viewer executable bound `127.0.0.1:0`, reported its actual ephemeral
authority, and served only requests carrying that exact Host. The page supplied
the request-authenticity value through the browser-received HTML. Separate
requests selected `overview`, `working`, and `deep`; a canonical Context Item
created after process start appeared without restart. Authenticated same-origin
HTTP corrected that Context Item to revision 2, confirmed through a separately
opened Local Operations read, and published an explicit document to the
requested temporary path. A revised Guarded request rendered its current exact
identity, revision, and fingerprint. One exact authenticated response succeeded
with a canonical current-host Source; stale, mismatched, and reused responses
were rejected.

Missing and incorrect authenticity values, a cross-origin Origin, cross-site
Fetch Metadata, and an alternate Host were rejected before domain routing. For
each request the test independently re-read canonical memory, the pending
Guarded response, and the publication destination: no current-host Source or
canonical record changed, no Guarded confirmation was recorded or available to
dispatch, and no file or parent directory was created. An alternate Host could
not retrieve the page or request-authenticity value. The value was absent from
request URLs and redirects, portable output, canonical inspection, and the
ordinary generated document.

The installed MCP process advertised 14 high-level tools. Every advertised
shape was a closed object with typed, described properties and explicit
required fields. The maintained client-side interpreter constructed Recall,
Checkpoint, and Guarded calls from those returned schemas. All three executed;
the Guarded response preserved exact identity and revision. Across the full
catalog, 29 missing-field or additional-property calls were rejected by the
advertised contract. Production Rust oracles independently match every schema
shape to handler-consumed fields and verify invalid requests fail before
mutation.

The CLI recovery journey established two Projects and analyses, recorded stable
user-owned Source, Checkpoint, and forgetting state, then changed repository
content and corrupted only one Project-owned analysis file. Health exposed
`degraded` plus the exact corrupt scope. `repair` read the changed file and
published a new Repository Snapshot and Analysis Snapshot that both referenced
the fresh canonical repository Source for that observation. A later repository
change followed by `reindex` produced another fresh Source/Snapshot lineage and
included the later file.

Portable output changed after both scans, as required by repository observation
provenance. Table-by-table comparison showed exactly one added
`repository_snapshot` Source per scan; all prior Sources and every other
canonical table remained unchanged. Rust recovery oracles separately preserve
active and terminal Questions, active and superseded Decisions, corrected
Context Items, Checkpoints, forgetting state, non-repository user Sources, and
their revisions. Earlier repository Sources retained their original snapshot
basis, the unrelated Project remained byte-identical in both canonical and
derived state, and the Project analysis directory retained one current derived
snapshot. Unsupported canonical repair failed without changing compared state.

The authenticated Codex event stream showed the model select
`volicord.project_health`, construct `{"project_id": ...}` from tool metadata,
complete the MCP call, receive `connection: connected` and `capability_state:
healthy`, and report those values. This is model-driven product-tool evidence,
not registration, startup, or a manually constructed stdio RPC.

## Coverage and failures

Final focused results used for this report:

- real viewer executable: passed in `246.121 ms`;
- deterministic clean/MCP/recovery harness: passed in `414.967 ms` with warm
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

- Pass: real HTTP requests use the executable's actual ephemeral authority and
  browser-received request-authenticity value for memory correction, exact
  Guarded fallback, and explicit document publication.
- Pass: missing/wrong authenticity, cross-origin, cross-site, and alternate-Host
  requests fail with no canonical, Guarded, or filesystem side effect; an
  untrusted Host cannot retrieve the page value and the value is not durable.
- Pass: `tools/list` exposes concrete closed schemas for every public tool;
  representative read, mutation, and Guarded calls are constructed from those
  schemas and execute successfully.
- Pass: missing required fields, malformed values, and additional properties
  fail consistently before mutation.
- Pass: authenticated Codex selects and completes a Volicord read-only product
  tool from the isolated registered server and returned schema.
- Pass: repair and reindex recover replacement-owned derived analysis from the
  current repository observation, add only required repository provenance to
  canonical state, preserve user meaning and another Project, retain historical
  Source basis, and reject unsupported scope.
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

Retain the thin viewer and high-level reconstruction-owned MCP edge. Retain the
bound-authority and request-authenticity checks, closed tool schemas, and fresh
repository Source observation as executable contracts. Keep the authenticated
Codex probe explicit and separate from ordinary deterministic workspace tests.

## Rejected alternatives and reasons

Not applicable. This validation evaluates current Production behavior and does
not select or discard an alternate implementation.

## Reusable primitive decision

`production_evidence`. The HTTP listener, Local Operations, MCP schema
validation, installer, runtime layout, and Project-owned analysis replacement
remain Production responsibilities. The matrix and Python probes are maintained
black-box acceptance orchestration only.

## Decision revisit trigger status

Not triggered. The evidence preserves local-first operation,
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
