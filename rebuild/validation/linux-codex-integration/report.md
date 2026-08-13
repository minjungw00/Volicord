# V08 — Clean Linux and Codex integration

## Status

Passed against viewer commit `a6355a9edf5a587a17ad93eeb8357d1de977ba54`,
host/install commit `85c876033c35acb5ad95eee3dec223fc91213f50`,
and current-host Source fix `bec6424ee0e7a7f378f2fc799bb58e201cc0c00f`.
The maintained acceptance matrix covers 37 requirements through two Rust test
targets with nine discovered tests. The real Codex CLI registration/discovery
journey, deterministic MCP process journey, and mapped Rust semantic oracles
passed. The reconstruction final aggregate has not yet been run and is not
claimed by this report.

## Goal

Verify that a clean Linux-style user environment can install the replacement
product, initialize only its new runtime, bind a Project, discover its
high-level host capabilities through Codex, use the essential Project calls,
recover across host lifecycle changes, understand degradation, use the viewer
in English and Korean, and uninstall/reinstall without losing canonical data.

## Accepted decisions being validated

Q1 and Q2 require a Project-oriented local product with Codex as an integration
host rather than a storage authority. Q3 keeps canonical meaning and structural
processing local by default. Q4 and Q5 require source-grounded Recall,
explanations, and four generated documents. Q6 and Q7 preserve Decision,
Checkpoint, correction, supersession, and forgetting meaning. Q8 requires
Guarded high-risk effects to bind one exact request and user response. Q10 and
Q11 keep Candidates and bounded Recall visible without creating a second
canonical store. No accepted Decision is broadened by this transport evidence.

## Input repositories and revisions

Production behavior is fixed at the three commits listed in Status. The clean
journey creates a self-authored temporary repository containing one README and
uses separate temporary HOME, XDG data, Codex configuration, installation
prefix, replacement Runtime Home, and legacy-runtime sentinel paths. The V08
fixture is the maintained `v08-matrix.json`; it maps the integration journey to
the V06, V07, and V10-backed product boundaries and to direct viewer and host
tests without copying their domain implementations.

## Environment and tool versions

The evaluation ran on Linux `6.18.33.2-microsoft-standard-WSL2` x86_64 with
`rustc 1.97.1`, `cargo 1.97.1`, Python `3.12.3`, and the real
`codex-cli 0.145.0`. Registration and discovery use the installed Codex CLI in
an isolated writable `CODEX_HOME`. No authenticated model turn or network
service is required; MCP product calls use the installed line-delimited stdio
process deterministically.

## Candidate approaches

1. Reusing the legacy MCP protocol would have made removed workflow methods and
   wire compatibility part of the replacement. The selected host exposes only
   high-level Project capabilities over a small reconstruction-owned MCP edge.
2. Driving an authenticated model turn would make V08 depend on credentials,
   service availability, and non-deterministic interpretation. The selected
   split uses the real Codex CLI for setup/discovery and exact MCP requests for
   product behavior.
3. Giving the viewer or host direct database CRUD would create competing domain
   authority. Both adapters call Local Operations and existing projections.
4. Deleting runtime data during uninstall would make lifecycle behavior unsafe.
   The selected installer removes executables and registration while preserving
   the explicit Runtime Home.

## Commands and configuration

```text
rebuild/scripts/validate focused v08-assertions -- rebuild/validation/linux-codex-integration/assertions.py
rebuild/scripts/validate focused v08-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused v08-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/linux-codex-integration/report.md
```

The assertion discovers and executes the mapped `volicord-host` and
`volicord-viewer` Rust integration targets, checks workspace/dependency and
legacy-path exclusions, and invokes the clean harness once. The harness runs
`rebuild/install.sh` with explicit prefix and runtime paths, uses `codex mcp
get/list --json`, speaks MCP to the installed process, and repeats installation
after an explicit uninstall.

## Observed results

The real Codex executable reported `codex-cli 0.145.0`, accepted the exact
`volicord-mcp` command plus `VOLICORD_RUNTIME_DIR`, and discovered the registered
server through both `mcp get` and `mcp list`. Installation produced three
executable PATH-resolvable binaries and four separate current-product stores.
Project initialization returned an exact canonical repository binding.

The installed host initialized, listed 14 high-level tools, reported a healthy
connected Project, returned read-only Recall, and recorded a source-linked
handoff Checkpoint. EOF ended each process successfully. A new host process
reconnected to the same Project. With the bound repository temporarily absent,
the live connection remained `connected` while capability health became
`degraded`; a missing MCP executable was separately observed as launch failure.

Mapped Rust tests recorded a current-host Decision and Checkpoint from separate
user-authored Codex turn Sources. Guarded host presentation, viewer fallback,
CLI fallback, and confirmation used the same request identity, revision, and
effect fingerprint. Viewer tests rendered English and Korean fixed text at all
three explanation levels and accepted a non-allowlisted generated language.
Uninstall removed all binaries and Codex registration, retained the canonical
store, and reinstall returned an identical Recall projection. The isolated
legacy sentinel retained the same bytes and modification time.

The first maintained assertion attempt exposed that Local Operations omitted
the required agent observer from a current-host user Source, so the Decision
owner rejected the response as unverified. The defect was corrected in the
separate minimal Production fix listed in Status. Focused host and Local
Operations suites then passed before V08 was evaluated again; the failed attempt
is retained in focused validation artifacts rather than hidden.

## Coverage and failures

Covered behavior includes clean install, permissions and PATH, current Runtime
Home creation, Project initialization/binding, real Codex registration and
discovery, high-level capability discovery, Recall, Decision, Checkpoint,
canonical and Candidate inspection, documents, exact Guarded transport and
fallback, viewer localization, host EOF/restart, connected degradation,
connection launch failure, uninstall, reinstall, and canonical preservation.

The final maintained V08 assertion command passed. Earlier development runs
retained one report-wording failure, the current-host observer defect described
above, and one overly specific assertion about Codex's missing-registration
error text; none is hidden or relabeled as a pass. A model-authenticated
interactive Codex agent turn is deliberately not claimed: it is unnecessary
for verifying registration or the exact MCP protocol and would add
credentials/network variability. V08 is not V11; no multi-repository dogfood
or replacement rehearsal was run.

## Performance and resource observations

The final maintained assertion completed in under one second with release build
artifacts already warm; it includes two supported installer invocations. This
elapsed time therefore reflects local Cargo build reuse rather than cold-build
or MCP latency. Host requests and cleanup completed within the harness's
ten-second shutdown bound. The one-file fixture does not establish
large-repository analysis latency, memory ceilings, concurrent clients, or
long-running process cancellation behavior.

## Privacy and external transmission

No repository content or canonical data was transmitted externally. Codex was
used only for local configuration registration and discovery. Product calls
went directly to the local stdio process. Background provider state remained
local-only, no provider consent was inferred, and the temporary legacy-runtime
sentinel was not opened or changed by the supported install journey.

## Acceptance results

- Pass: installation creates executable CLI, viewer, and MCP host surfaces in
  an explicit prefix and initializes only the separate replacement Runtime Home.
- Pass: the real available Codex CLI registers and discovers the exact installed
  MCP command and runtime environment in an isolated configuration home.
- Pass: high-level Project, health, Recall, repository, Inquiry/Decision,
  Checkpoint, inspection, privacy, document, analysis, and Guarded capabilities
  are discoverable without legacy methods.
- Pass: current-host Decision, Checkpoint, and Guarded responses preserve
  user-turn Source linkage; Guarded host and fallback surfaces preserve one
  exact logical request identity, revision, and fingerprint.
- Pass: connection failure, connected capability degradation, EOF cleanup,
  restart, and reconnection remain distinguishable and observable.
- Pass: English/Korean viewer rendering, all explanation levels, and arbitrary
  requested generated language remain available without viewer-owned mutation.
- Pass: uninstall/reinstall preserves canonical Recall and never reads or
  migrates the legacy-runtime sentinel.
- Pass: all reconstruction packages remain under `rebuild/` without legacy
  workflow, MCP, or Runtime Home dependencies.

## Known limits

Codex setup/discovery is exercised with the real executable, but V08 does not
require an authenticated model session or external network access. The fixture
is local, temporary, deterministic, and small. It does not qualify multiple
simultaneous MCP clients, abrupt power loss during mutation, a large repository,
accessibility with assistive technology, other Linux distributions, macOS, or
Windows. Decision and Checkpoint transport is semantically exercised in the
Rust host oracle because the clean CLI-created Project intentionally contains no
fabricated open Question.

## Recommended implementation choice

Retain the viewer as a thin read/projection and Local Operations adapter, retain
the high-level reconstruction-owned MCP edge, and retain the explicit
`VOLICORD_RUNTIME_DIR` Codex registration. Keep real CLI discovery separate from
deterministic semantic transport tests so unavailable credentials or network do
not create false negatives or fabricated passes.

## Rejected alternatives and reasons

Do not add a legacy MCP decoder, old method alias, dual Runtime Home search,
migration warning, implicit data deletion, viewer store, or generalized host
approval. Do not describe Codex registration alone as a model-authenticated
conversation, and do not make V08 depend on external network access when exact
local protocol behavior is directly observable.

## Reusable primitive decision

`production_evidence`. The viewer adapter, Local Operations boundary, MCP host,
installer, runtime layout, and process cleanup are Production responsibilities.
The Python harness and matrix are maintained acceptance orchestration only; they
do not implement a second Project, protocol, persistence, projection, or
Guarded-effect engine.

## Decision revisit trigger status

Not triggered. The evidence preserves local-first operation, exact current-host
user provenance, one Guarded request, Project-scoped privacy, and explicit
degradation. The lack of a model-authenticated agent turn does not weaken these
contracts because the real Codex configuration and exact installed protocol are
independently observed. A future Codex CLI registration format change or host
elicitation capability change requires revisiting only the adapter/setup edge,
not canonical or Guarded meaning.

## Follow-up work

- Run the single exact reconstruction final aggregate after the V08 evidence
  commit is complete and the worktree is clean.
- Treat V11/Phase 8 as the next independent multi-repository dogfood and product
  quality boundary only if the Phase 7 final gate passes.
- Defer legacy deletion, root installer/workflow replacement, and product
  cutover to the Phase 9 cutover gate.

## Artifacts

Maintained artifacts are this report, `assertions.py`, `harness.py`, the V08
matrix and shared fixture-manifest entry, the Phase 7 summary, direct host/viewer
Rust tests, and the production install/setup documentation. Focused command
argv, complete stdout/stderr, exit status, timestamps, and duration remain under
ignored `rebuild/.local/validation/`.
