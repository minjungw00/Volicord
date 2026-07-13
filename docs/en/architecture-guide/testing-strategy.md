# Testing strategy

This guide explains which implementation test layer to use for common Volicord
Rust changes. Tests verify owner-defined facts; they do not define product
contracts, prove security, complete QA, establish close readiness, or record
product acceptance.

For exact behavior, use the [Reference Index](../reference/README.md). For
crate-by-crate source orientation, use the [Codebase Tour](codebase-tour.md).
For workspace shape and the dependency-boundary overview, use
[Implementation Architecture](architecture.md). For exact Cargo dependency
edges, read the workspace and crate `Cargo.toml` manifests. For change workflow,
use the [Implementation Guide](change-guide.md). For documentation
command-example validation, terminology role validation, bilingual link parity,
and validation reporting boundaries, use the [Validation](../maintain/validation.md)
policy.

## Test layers

| Layer | Actual package or path | Use it for | Avoid using it as |
|---|---|---|---|
| Module unit tests | Colocated tests in implementation modules such as [`crates/volicord-types/src/lib.rs`](../../../crates/volicord-types/src/lib.rs), [`crates/volicord-core/src/pipeline.rs`](../../../crates/volicord-core/src/pipeline.rs), [`crates/volicord-store/src/core_pipeline.rs`](../../../crates/volicord-store/src/core_pipeline.rs), [`crates/volicord-store/src/sqlite.rs`](../../../crates/volicord-store/src/sqlite.rs), setup, connection, guard, host integration, and MCP modules. | Local helper behavior, typed parsing, canonical hashing, policy helpers, Store transaction edges, schema validation, setup workflow branches, connection selection/output helpers, guard integration planning/audit helpers, and small branch checks close to the code. | A cross-layer acceptance test or a product contract source. |
| Platform filesystem facade tests | Colocated tests in [`crates/volicord-platform-fs/src/lib.rs`](../../../crates/volicord-platform-fs/src/lib.rs), package `volicord-platform-fs`. | Safe result classification around platform-native namespace operations and target-specific facade behavior. | Managed-file ownership policy, caller recovery behavior, a filesystem-wide portability claim, or a security proof. |
| Core method tests | [`crates/volicord-core/src/methods/tests/`](../../../crates/volicord-core/src/methods/tests/) in package `volicord-core`, split into method-focused files such as `status.rs`, `intake.rs`, and `prepare_write.rs`. | Method planning, shared preflight through `CoreService`, dry-run/no-effect/commit branches, replay, state-version effects, artifact staging distinction, and method-visible Store effects. | MCP transport coverage or full public behavior authority. |
| Storage DDL contract test | [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), target `storage_ddl_contract`, package `volicord-store`. | Owner-to-implementation consistency for Storage DDL, canonical SQL sources, schema initialization, schema validation, tables, columns, constraints, indexes, and maintained triggers. | General storage-effect behavior or runtime conformance. |
| Binary tests for administrative CLI | [`crates/volicord-cli/tests/binary_admin.rs`](../../../crates/volicord-cli/tests/binary_admin.rs), target `binary_admin`, package `volicord-cli`. | The `volicord` binary, Runtime Home setup through `volicord init`, setup workflow effects that must be observed through the binary, project detection, `volicord status`, `volicord connection add`, `volicord connection list`, `volicord connection status/verify/mode/remove`, connection output, `volicord inbox ...`, zero-write dry runs, host-state verification, connected-project membership lifecycle, generated guard output drift through guarded init/status cases, residual-effect reporting, host config writes, preflight failure handling, doctor diagnostics, and command-line error paths. | Public API method behavior. |
| Guard command tests | [`crates/volicord-cli/tests/guard_command.rs`](../../../crates/volicord-cli/tests/guard_command.rs), target `guard_command`, package `volicord-cli`. | Guard hook lifecycle behavior for session start, pre-tool, post-tool, prompt capture, and stop; recorded observations; expected-write matching; write-ticket coverage; host-native rendering; prompt-capture command handling; and guarded lifecycle fixtures. | A security proof, human approval record, product acceptance record, or replacement for Core method tests. |
| Binary tests for MCP transport | [`crates/volicord-cli/tests/mcp_transport.rs`](../../../crates/volicord-cli/tests/mcp_transport.rs), target `mcp_transport`, package `volicord-cli`. | The `volicord mcp` subcommand, help/version, `--check`, stdio framing, JSON-RPC behavior, reconnect cases, and response wrapping. | Core method semantics. |
| Local HTTP serve transport tests | [`crates/volicord-cli/tests/serve_transport.rs`](../../../crates/volicord-cli/tests/serve_transport.rs), target `serve_transport`, package `volicord-cli`. | The `volicord serve --transport local-http` process path, loopback listener startup, token and origin checks, HTTP session behavior, defensive headers, and MCP request routing through the local HTTP transport. | A general MCP method test or security proof. |
| Opt-in live host configuration smoke tests | [`crates/volicord-cli/tests/live_host_smoke.rs`](../../../crates/volicord-cli/tests/live_host_smoke.rs), target `live_host_smoke`, package `volicord-cli`. | Explicit configuration checks against an installed Codex or Claude Code executable, selected with `VOLICORD_RUN_CODEX_SMOKE=1` or `VOLICORD_RUN_CLAUDE_SMOKE=1`. | Evidence that the host delivered a final-output event, displayed fixed UI, or completed a User Judgment round trip. |
| Opt-in live final-output matrix | The four Codex/Claude Code by Record/Detective tests in `live_host_smoke`. | Separately recording managed configuration-fixture, generated-wrapper wire, actual host event, actual fixed-UI, Detective decision, status-fallback, and exact-replay evidence. | Treating fixture or direct-wrapper output as proof of actual host delivery or UI, or treating one host/profile cell as proof of another. |
| Opt-in live Judgment round trips | The Codex and Claude Code `*_live_user_action_round_trip_is_opt_in` tests in `live_host_smoke`. | Authenticated, human-in-the-loop host-native Judgment selection and its resulting authority records. | Final-output matrix evidence; Judgment elicitation and final-output disclosure are separate validation concerns. |
| Opt-in live evidence-observation local-web round trips | The Codex and Claude Code `*_live_evidence_observation_round_trip_is_opt_in` tests in `live_host_smoke`. | An actual installed host creates and resumes an evidence-observation request while a human submits the canonical form through the loopback `local_web_consent` User Channel path. | Native Judgment elicitation, CLI recovery, or final-output matrix evidence; each remains a separate release-validation cell. |
| Opt-in live CLI-fallback round trips | The Codex and Claude Code `*_live_cli_fallback_round_trip_is_opt_in` tests in `live_host_smoke`. | A human-selected choice submitted by the actual CLI User Channel, exact CLI retry, and same-Agent-Connection host resume through the installed host. | Native Judgment elicitation, evidence-observation local-web, or final-output matrix evidence; all release-validation surfaces remain separate. |
| MCP integration tests | [`tests/integration/mcp_connection.rs`](../../../tests/integration/mcp_connection.rs), target `mcp_connection`, package `volicord-integration-tests`. | Cross-layer MCP, Core, Store, connection binding, `operation_category` derivation, tool exposure, replay-context binding, and storage no-effect checks visible through MCP. | A replacement for focused method tests or Reference owners. |
| Public contract snapshot tests | [`tests/integration/public_contract_snapshots.rs`](../../../tests/integration/public_contract_snapshots.rs), target `public_contract_snapshots`, package `volicord-integration-tests`. | Generated API request-schema and MCP tool-contract snapshot drift against the current source projection. | Hand-edited generated snapshots, semantic Reference review, or proof that the public contract is correct. |
| Conformance implementation tests | [`tests/conformance/baseline.rs`](../../../tests/conformance/baseline.rs), target `baseline`, package `volicord-conformance-tests`. | Baseline cross-method scenarios through Core-facing APIs, including replay, write tickets, artifacts, judgments, close readiness, error routing, and corruption handling. | Product acceptance, security proof, close readiness, or the sole source of a product rule. |
| Shared test support | [`crates/volicord-test-support/src/lib.rs`](../../../crates/volicord-test-support/src/lib.rs), package `volicord-test-support`. | Disposable Runtime Home fixtures, registered project and Agent Connection setup, request builders, Store inspection helpers, and shared fixture composition. | Production behavior or a durable runtime home. |
| CLI integration support | [`crates/volicord-cli/tests/support/`](../../../crates/volicord-cli/tests/support/). | Binary fixtures, fake hosts, fake MCP processes, guard lifecycle fixtures, JSON helpers, and assertion helpers reused by `binary_admin`, `guard_command`, `mcp_transport`, and `serve_transport`. | A source of product contracts or durable runtime state. |
| Documentation maintenance tooling tests | [`xtask/tests/docs_check.rs`](../../../xtask/tests/docs_check.rs), package `xtask`. | The read-only documentation validator, metadata parsing, bilingual coverage, local link and anchor checks, terminology path and role checks, command-example validation, public-language checks, and temporary fixture behavior. | Semantic translation review, technical-accuracy review, or a product contract source. |

## Fixture and support structure

Shared fixture structure is part of test strategy. `volicord-test-support`
owns disposable Runtime Home fixtures, registered project and Agent Connection
setup, request builders, Store inspection helpers, controlled-corruption
helpers, and shared assertions used by Core, conformance, and integration
tests. Its fixtures support implementation verification; they are not
production runtime state or contract owners.

`crates/volicord-cli/tests/support/` owns CLI integration helpers for binary
execution, fake hosts, fake MCP processes, guard lifecycle setup, JSON parsing,
and reusable assertions. Host contract fixtures under
`crates/volicord-cli/tests/fixtures/host_contracts/` support host-adapter and
guard lifecycle tests. Assertions made through those helpers still route to the
Reference owner for the fact being checked.

## Opt-in live host smoke tests

`live_host_smoke` is a normal Cargo test target whose live host checks carry
`#[ignore]`, so an ordinary workspace test run reports them as ignored. Its
pure result-path and operator-token checks and its disposable MCP-to-Core
regressions are not ignored and run in ordinary CI. Run a live check only in an
environment where that host executable is installed and the matching opt-in
variable is set.

The host-configuration checks remain separate:

```sh
VOLICORD_RUN_CODEX_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_smoke_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_smoke_is_opt_in -- --ignored --nocapture
```

### Final-output host/profile matrix

The final-output checks form an explicit four-cell matrix. Each cell has its
own opt-in variable and test; a result from one cell cannot satisfy another.

| Host | Record profile | Detective profile |
|---|---|---|
| Codex | `codex_record_live_final_output_is_opt_in` with `VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE=1` | `codex_detective_live_final_output_is_opt_in` with `VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE=1` |
| Claude Code | `claude_code_record_live_final_output_is_opt_in` with `VOLICORD_RUN_CLAUDE_RECORD_FINAL_OUTPUT_SMOKE=1` | `claude_code_detective_live_final_output_is_opt_in` with `VOLICORD_RUN_CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE=1` |

The four commands are:

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/codex-record.json VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_record_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/codex-detective.json VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_detective_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/claude-record.json VOLICORD_RUN_CLAUDE_RECORD_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_record_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/claude-detective.json VOLICORD_RUN_CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_detective_live_final_output_is_opt_in -- --ignored --nocapture
```

Each bounded result names its `host`, `profile`, and overall `result`, then
keeps `config_fixture`, `generated_wrapper_direct_wire`, `actual_host_event`,
`actual_host_fixed_ui`, `detective_decision`, `status_fallback`, and
`exact_replay` as independent evidence. The evidence statuses are `verified`,
`unavailable`, `not_applicable`, or `failed`; these are validation-harness
facts, not a product response schema.

The evidence layers are deliberately non-substitutable:

- `config_fixture` checks the managed configuration shape. It does not prove
  that an installed host loaded it or delivered an event.
- `generated_wrapper_direct_wire.status_fallback` and
  `generated_wrapper_direct_wire.authority_receipt` invoke the generated
  wrapper directly and keep the two bounded host-response branches separate.
  Both must be verified, but neither proves actual host dispatch or fixed-UI
  presentation.
- `actual_host_event.status_fallback_event` and
  `actual_host_event.authority_receipt_event` separately record both deliveries
  by the installed host, while
  `actual_host_fixed_ui.authority_receipt` separately requires a complete
  active-Task receipt on fixed UI rather than model prose. Neither proves the
  other. `actual_host_fixed_ui.status_fallback` independently confirms the
  no-active-Task fixed-UI branch.
  Record deliberately has no persistent Guard observation; its event entries
  identify authenticated host-owned UI delivery, while before/after counts
  prove that no Guard event or Agent Session was created. This does not invent
  a durable Record observation.
- The top-level `status_fallback` evidence binds that no-active-Task UI
  confirmation to the exact generated `volicord status --json` fallback.
  Direct-wire output cannot replace the UI observation, and a fixed-UI receipt
  cannot replace the fallback. The operator copies the complete taskless
  managed-UI message and the harness checks exact equality, including absence
  of a task-bound command. Every cell must verify both branches under
  `actual_host_fixed_ui` and the separate command evidence.
- `exact_replay.generated_wrapper_identical_payload` records repeated identical
  payload delivery through the generated wrapper, while
  `exact_replay.actual_host_replay` records replay through an actual host entry
  point. The generated-wrapper check advances Task authority between the two
  identical deliveries, requires a newer current receipt on the second wire,
  and for Detective requires the stored historical Stop row to remain exactly
  unchanged. A direct wrapper replay cannot be reported as actual host replay.

For the Record profile, the final-output path is non-gating and non-observing.
Its `detective_decision` evidence is `not_applicable` only when the result also
confirms the absence of a Guard event or decision and confirms that final
output was not gated. Repeated delivery must refresh the read-only display
without creating an observation.

For the Detective profile, decision evidence covers both `allow` and `block`. An
exact replay preserves the immutable historical Guard event and decision while
the separate fixed UI refreshes current authority; a later current receipt may
therefore differ from the historical receipt. If the installed host exposes no
safe `block` entry or no actual-host exact-replay entry point, the corresponding
evidence is `unavailable` and the overall result remains `incomplete`. The same
applies when the executable, authentication environment, interactive TTY,
event-delivery surface, active-Task receipt UI, or no-active-Task fallback UI
cannot be used. Such a run remains `incomplete` and is reported as `SKIP` or
`FAIL`, never `PASS`; fixture or generated-wrapper evidence does not upgrade it.

Use the paired [live-host final-output release-validation checklist](../maintain/validation.md#live-host-final-output-release-validation)
before a release claim covers these host/profile paths. Exact final-output,
receipt, replay, and fallback behavior remains in the applicable Reference
owners, including [Agent Connection](../reference/agent-connection.md#managed-final-output-authority-disclosure)
and [Administrative CLI](../reference/admin-cli.md#managed-final-output-authority-disclosure).

### Judgment round trips

The Judgment checks remain separate from the final-output matrix:

```sh
VOLICORD_RUN_CODEX_USER_ACTION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_user_action_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_RUN_CLAUDE_USER_ACTION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_user_action_round_trip_is_opt_in -- --ignored --nocapture
```

The Judgment variants are human-in-the-loop checks. They create a disposable
Runtime Home and Product Repository, configure the selected host, then launch
the installed host interactively with an initial no-write instruction. They
reuse the runner's normal host authentication environment; the fixture does
not copy credentials into its isolated Runtime Home. The operator must approve
the project/MCP entry when the host requires it, choose the answer in the
host-native MCP elicitation UI, and exit the host after status is reported.

A passing Judgment variant verifies marker Task and Judgment creation,
host-native prompt/response recording with
`mcp_elicitation_user_channel`, the resulting Task-state transition, authority
events, and the matching content-free session diagnostic. The Task uses advisor
mode, then creates a current Change Unit and baseline before requesting the
Judgment. The Judgment has two fixed route options. After the human selects one,
the agent must consume the default compact Judgment outcome and record a
no-product-write `shaping_update` Run whose exact summary and close-assessment
marker are mapped to that option. After the host exits, the operator confirms
the selected fixed option; the harness requires it to equal the stored
`selected_option_id`. It then follows the fresh receipt's `latest_run_ref` to the
exact Run row and requires the matching user-Judgment and Run authority-event
payloads and event sequence to prove selection-before-consumption order. If
native elicitation is unavailable, the harness verifies pending inbox visibility
and the current answer-command shape, emits path-free command templates, and
fails the native-round-trip check. The disposable fixture is then deleted, so
the templates are not runnable recovery commands and do not turn the failed
live-native check into a pass.

The Judgment variants also capture the host's `--version` output and Volicord
`build_id`, read a fresh CLI status, and require its `authority_receipt` to bind
the same Project, Task, exact Run, ready close state, empty blocker set, and
`state_version`. They also require exactly one new Task-bound Detective Stop
event after the pre-host cursor, an `allow` decision with no reasons or close
blockers, and a stored receipt equal to fresh status. The operator must copy the
complete canonical receipt JSON from the separate host-owned managed UI, and
the harness checks exact equality rather than accepting a state-version-only
token. A bounded JSON summary is printed. Every authenticated live-host test
requires `VOLICORD_LIVE_HOST_RESULT_PATH` to name a new absolute approved path
outside the source repository; omitting it fails before the host is launched.
The harness rejects an existing file, writes a run-identified
`running` record, and atomically replaces it with the bounded final or
early-failure record. It contains validation facts, not a transcript,
credential, secret, or full prompt.

The Task-bound Stop event and complete receipt UI are required evidence that the
Judgment run reached its authoritative completion state. They cannot fill a
cell or evidence field in the four-cell final-output matrix, and final-output
matrix evidence cannot establish native Judgment elicitation. Other
final-output, fallback, or replay observations made during the Judgment run are
diagnostic only for that run.
The Judgment inbox fallback is User Channel recovery evidence, not final-output
`status_fallback` evidence.

Before publishing a release that claims either maintained host Judgment path,
follow the paired [live-host Judgment release-validation checklist](../maintain/validation.md#live-host-judgment-release-validation).
It requires both host-specific runs against the release candidate and owns
external result retention, UI confirmation, fallback, and skipped-validation
reporting. An unavailable host, authentication environment, interactive TTY,
or native elicitation surface is not a passing round trip.

### Evidence-observation local-web round trips

The evidence-observation checks are two additional host cells, separate from
native Judgment elicitation, executable CLI recovery, host-configuration
smoke tests, and the final-output matrix:

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/codex-evidence-observation.json VOLICORD_RUN_CODEX_EVIDENCE_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_evidence_observation_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/claude-code-evidence-observation.json VOLICORD_RUN_CLAUDE_EVIDENCE_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_evidence_observation_round_trip_is_opt_in -- --ignored --nocapture
```

Each cell uses fixture-only setup to establish disposable starting state, then
requires the actual installed host to create and resume one evidence-observation
request on the prepared Agent Connection. A human opens the loopback consent
form and submits the prepared target and artifact, `supported`, and a bounded
non-secret summary. The host must then consume the resulting resolution in a
Run. Store inspection, authority-event order, fresh status, the Task-bound Stop
event, and full managed-UI receipt confirmation provide the observed
cross-layer assertions. Fixture setup and adapter-only checks remain identified
separately and cannot stand in for the installed-host observations.

The non-secret credential-shaped artifact marker makes the conservative
User Channel route deterministic for this fixture. The cell observes that the
local-web path was selected; it does not prove secret detection, native
elicitation, or the security of the external host.

These are release-test assertions, not a second API contract. Exact request and
resume behavior belongs to
[`volicord.request_user_action`](../reference/api/method-request-user-action.md),
resolution behavior to
[`volicord.resolve_user_action`](../reference/api/method-resolve-user-action.md),
common request and resolution shapes to
[API User Action Schemas](../reference/api/schema-user-action.md), consuming Run
and evidence effects to
[`volicord.record_run`](../reference/api/method-record-run.md), and local-web
routing to [MCP Transport](../reference/mcp-transport.md#local-web-consent-fallback).
Fresh status and receipt comparisons use the
[status method](../reference/api/method-status.md) and
[API State Schemas](../reference/api/schema-state.md).

The live cell intentionally retains no host transcript. It proves the observed
same-request replay and downstream resolution-ref consumption, but it does not
directly inspect the resumed response for omission of the user's raw summary.
That omission remains covered by the focused schema and adapter regression
tests against the owners linked above.

The bounded external result records safe validation coordinates and summary
match facts, never the consent URL, bearer token, raw summary, prompt, or
transcript. Its result lifecycle and retention rules are maintained by the
paired checklist linked below.

These ignored cells require the installed host executable, its ordinary
authentication environment, an interactive TTY, a usable local browser, host
trust or approval where requested, and the fresh external result path. An
ordinary Cargo report that the test was ignored, a run without the opt-in
variable, or an unavailable prerequisite is not a pass. Follow the paired
[live-host evidence-observation release-validation checklist](../maintain/validation.md#live-host-evidence-observation-release-validation)
before making a release claim. Both host-specific cells must pass for a claim
covering both maintained hosts, and neither cell can satisfy or be satisfied by
a native Judgment, CLI-fallback, configuration, or final-output cell. Exact
product behavior remains in the focused owners linked above.

### CLI-fallback Judgment round trips

The executable CLI-fallback checks are two additional host cells, separate from
both native Judgment elicitation and the four-cell final-output matrix:

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/codex-cli-fallback.json VOLICORD_RUN_CODEX_CLI_FALLBACK_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_cli_fallback_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/claude-code-cli-fallback.json VOLICORD_RUN_CLAUDE_CLI_FALLBACK_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_cli_fallback_round_trip_is_opt_in -- --ignored --nocapture
```

Each cell prepares an `advisor` Task, current Change Unit, baseline, and current
pending two-option product-decision request for the selected Detective Agent
Connection. The human operator chooses `route_alpha` or `route_beta`; the
harness verifies the request in the actual `volicord inbox --json` result and
submits that choice through the actual
`volicord inbox resolve ... --choice ... --json` command. It repeats that exact
command and requires byte-identical JSON plus an unchanged `state_version`.
This is an executed User Channel resolution, not the path-free command-template
diagnostic emitted by a failed native Judgment cell.

The harness then launches the installed host on the same prepared Agent
Connection. The host must call `volicord.request_user_action` with
`request.operation=resume` and the exact request ID, consume the CLI-selected
option without creating another product-decision request, and record the mapped
no-product-write `shaping_update` Run through that Agent Connection. Fresh CLI
status must bind the exact Run in a ready `AuthorityReceipt` with no blockers.
The same live host path must also produce one new Task-bound Detective Stop
`allow` event whose stored receipt equals fresh status, and the operator must
copy the complete canonical receipt from the separate host-owned managed UI for
exact confirmation.

The bounded external result uses
`kind=live_host_cli_fallback_release_validation` and records the CLI resolution
ID, `actor_source=local_user`, `channel_kind=cli`,
`verification_basis=cli_direct_user_channel`, both CLI state versions, exact
retry facts, same-connection resume evidence, mapped Run and authority-event
order, Stop coordinates, fresh receipt, and managed-UI confirmation. It also
marks the native Judgment and final-output matrix scopes false. A result from
this cell cannot satisfy either of those surfaces, and their evidence cannot
satisfy this cell.

Before a release claim covers executable CLI recovery for either maintained
host, use the paired [live-host CLI-fallback release-validation checklist](../maintain/validation.md#live-host-cli-fallback-release-validation).
Both host-specific cells must pass for a claim covering both hosts. An
unavailable executable, authentication environment, interactive TTY, same-
connection resume path, Task-bound Stop, or complete receipt UI is `SKIP` or
`FAIL`, never a passing fallback.

An explicitly selected check cannot pass when its opt-in variable, host
executable, or another required live prerequisite is unavailable. Report the
case as `SKIP` or `FAIL` under the applicable checklist. Passing confirms only
the assertions that the installed host and local test environment allowed the
smoke test to observe. It does not prove portable host behavior, host trust,
approval, credential availability, network availability, security enforcement,
or general product correctness.

## Generated output and documentation validation

Generated output drift checks verify that generated or projected repository
artifacts still match their current sources. `public_contract_snapshots`
checks generated API request-schema and MCP tool-contract snapshots. CLI
binary and guard tests check generated host and guard outputs where those
outputs are visible through supported test paths. A drift failure means the
source change, owner document, validation expectation, or regeneration step
needs review; it is not a correctness proof.

For maintained documentation, `cargo run -p xtask -- docs-check` is the
repository structural check for `docs/doc-index.yaml`, maintained paths,
links and anchors, bilingual local-link parity, terminology owner paths and
roles, command-example shape, and public-language checks. It is documentation
and public-language validation; it does not replace semantic bilingual review,
technical-accuracy review, Reference-owner review, or product conformance.

## Validation map by change area

Use this map after the Codebase Tour or Architecture page has identified the
affected crates or documents. It names likely checks; it is not a rule that
every small edit runs every listed test.

| Change area | Likely code or doc area | Start with | Add when |
|---|---|---|---|
| Architecture Guide, documentation routes, metadata, links, or terminology | `docs/en/`, `docs/ko/`, `docs/doc-index.yaml`, `docs/terminology-map.yaml`; `xtask` when validator behavior changes. | `cargo run -p xtask -- docs-check`, plus manual semantic parity, owner-routing, and terminology review. | `xtask` tests when adding or changing deterministic docs-check rules. |
| Public schemas, shared request/result types, value sets, identifiers, or request hashing | `crates/volicord-types/src/` and the applicable Reference owners. | `volicord-types` unit tests. | Core method tests when method planning changes; `public_contract_snapshots` or MCP integration when tool schemas or exposure change; docs-check when maintained docs change. |
| Platform filesystem facade or adapter-managed conditional file replacement | `crates/volicord-platform-fs/src/lib.rs`, the calling adapter such as `crates/volicord-cli/src/guard_integration/files.rs`, and the applicable CLI/runtime/system owners. | `volicord-platform-fs` and caller-module unit tests. | Target-specific compile or test coverage when native code changes; `binary_admin` when the administrative result is binary-visible; docs-check when maintained owners or Architecture Guide pages change. |
| Public method behavior, Core pipeline behavior, policy helpers, replay, or effect branches | `crates/volicord-core/src/pipeline.rs`, `crates/volicord-core/src/methods/`, and `crates/volicord-core/src/policy/`. | Core colocated unit tests and the method-focused files under `crates/volicord-core/src/methods/tests/`. | `tests/conformance/baseline.rs` for cross-method baseline scenarios; `tests/integration/mcp_connection.rs` for adapter-visible context or tool exposure. |
| Store DDL, canonical SQL, persistence helpers, transaction boundaries, storage effects, or artifact storage | `crates/volicord-store/src/`, [`crates/volicord-store/tests/storage_ddl_contract.rs`](../../../crates/volicord-store/tests/storage_ddl_contract.rs), and storage Reference owners. | Store colocated unit tests; `cargo test -p volicord-store --test storage_ddl_contract` for Storage DDL, canonical SQL, or schema validation changes. | Core method, conformance, or MCP integration tests when public-method-visible storage behavior changes. |
| MCP startup, stdio or local HTTP transport, tool listing, `tools/call`, project selection, or Agent Connection invocation context | `crates/volicord-mcp/src/`, `crates/volicord-cli/tests/mcp_transport.rs`, `crates/volicord-cli/tests/serve_transport.rs`, and `tests/integration/mcp_connection.rs`. | `volicord-mcp` unit tests, `mcp_transport`, or `serve_transport` for the transport being changed. | `public_contract_snapshots` when generated API or MCP tool projections change; `mcp_connection` when Core/Store behavior must be observed through MCP; docs-check when MCP docs change. |
| Setup workflow behavior and output | `crates/volicord-cli/src/setup_command.rs`, `setup_command/`, `doctor_command.rs`, and `crates/volicord-cli/tests/binary_admin.rs`. | Setup module tests for workflow branches and rendering; `binary_admin` when setup behavior must be observed through the binary. | Store tests when bootstrap, registry, inspection, schema initialization, or installation-profile persistence changes; docs-check when setup docs change. |
| Connection provisioning, status, verification, and output | `crates/volicord-cli/src/connection_command.rs`, `connection_command/`, `crates/volicord-store/src/bootstrap.rs`, `agent_connections.rs`, and `crates/volicord-cli/tests/binary_admin.rs`. | Connection command module tests and `binary_admin`. | `mcp_transport` when process/preflight behavior changes; `tests/integration/mcp_connection.rs` when MCP/Core/Store behavior must be observed through MCP; docs-check when CLI docs change. |
| Guard integration files, capability records, and audit facts | `crates/volicord-cli/src/guard_integration/`, connection status/output code, `doctor_command.rs`, and guarded init/status tests in `binary_admin`. | Guard integration module tests and `binary_admin` guarded init/status cases for generated guard output drift. | `guard_command` when lifecycle observations consume the generated capability facts; docs-check when guard CLI docs change. |
| Guard hook lifecycle behavior and host-native rendering | `crates/volicord-cli/src/guard_command.rs`, `guard_command/`, and `crates/volicord-cli/tests/guard_command.rs`. | `guard_command` tests plus colocated parsing/rendering tests. | Core method, conformance, or storage tests when the hook path depends on owner-defined Core or Store behavior. |
| Host config adapters | `crates/volicord-cli/src/host_integration/`, especially `host_integration/codex/`, `host_integration/claude_code/`, `config_edit.rs`, `contracts.rs`, `generic.rs`, and `verification.rs`. | Host adapter module tests and `binary_admin`. | `guard_command` when host-native hook output changes; `mcp_transport` when launch or preflight behavior changes. |
| Conformance scenario or shared fixture behavior | `tests/conformance/baseline.rs`, `crates/volicord-test-support/src/lib.rs`, and `crates/volicord-cli/tests/support/` for CLI integration fixtures. | The focused crate/unit tests for the behavior first, then the affected conformance or CLI scenario. | Consuming integration or method tests when fixture behavior changes what another layer observes. |

## Durable tests and one-time audits

Use [Validation](../maintain/validation.md) for the full rule that separates a
durable test from a cleanup-only audit. At the implementation layer, test the
current supported shape rather than the history of removed text or options.

Current examples show the intended style:

- `binary_help_options_match_supported_contracts` checks the current CLI help
  allowlists in `crates/volicord-cli/tests/binary_admin.rs`.
- `initial_schemas_satisfy_connection_storage_contract` checks current schema
  structure in `crates/volicord-store/tests/storage_ddl_contract.rs`.
- `public_mcp_arguments_reject_internal_envelope_and_invocation_fields` checks
  the public MCP schema boundary in `tests/integration/mcp_connection.rs`.
- `reports_required_terminology_role_failure` and
  `accepts_supported_volicord_shell_command_examples` protect current
  documentation validation rules in `xtask/tests/docs_check.rs`.

These tests protect implementation or maintenance boundaries. The focused
Reference owner still defines the product fact being checked.

## Tests that demonstrate boundaries

Some tests are especially useful for understanding architecture boundaries:

- `tool_sets_follow_connection_mode_and_exclude_user_only_recording`,
  `volicord_mcp_subcommand_tools_list_respects_connection_mode_and_schema_boundary`,
  `generated_mcp_workflow_tool_contract_snapshot_matches_sources`, and
  `generated_mcp_read_only_tool_contract_snapshot_matches_sources` cover
  tool-set projection and stdio `tools/list` exposure at their respective
  layers.
- `status_is_read_only_including_dry_run` and
  `status_include_false_omits_optional_sections_without_effect` cover the Core
  status branches; `mcp_status_succeeds_with_readonly_storage` and
  `mcp_status_does_not_advance_state_version` cover the corresponding
  MCP-visible read-only properties without asserting full response equivalence.
- `rejected_branch_has_no_storage_effect`, `dry_run_branch_has_no_storage_effect`,
  and `read_only_branch_has_no_storage_effect` protect no-commit branches.
- `committed_mutation_increments_state_version_once` and Store transaction
  replay tests protect the atomic commit boundary.
- `stage_artifact_creates_transient_handle_without_core_commit` protects the
  staging path from being confused with normal Core mutation commit.
- `no_effect_branches_state_version_and_idempotency_are_stable` demonstrates
  cross-method no-effect and replay stability through Core-facing APIs.

These tests are implementation checks. They are not Volicord runtime
conformance claims, product acceptance records, QA completion, security proof,
close-readiness results, or residual-risk acceptance.

## Validation defaults

For Rust implementation edits, the repository default is:

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

For documentation-only edits, use the applicable documentation checks. When a
documentation task asks for source verification, `cargo metadata --no-deps
--format-version 1`, repository search, and the requested test command are
appropriate implementation checks.

For maintained documentation structural checks, run:

```sh
cargo run -p xtask -- docs-check
```

Then complete the manual semantic bilingual review, contract-owner review, and
technical-accuracy review that match the changed documents.
