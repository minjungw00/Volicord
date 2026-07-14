# Validation

Use this policy after maintained documentation edits. It separates structural
checks, human semantic review, Rust implementation validation, and result
reporting.

This is maintenance validation. It is not Volicord runtime conformance, product
acceptance, QA completion, close readiness, a security proof, or residual-risk
acceptance. The repository-local automated documentation validator is:

```sh
cargo run -p xtask -- docs-check
```

## Structural Checks

For documentation metadata, route, link, and terminology-path changes, run
`cargo run -p xtask -- docs-check` from the repository root. The command is
read-only and verifies the machine-checkable shape:

- `docs/doc-index.yaml` parses as YAML and has `version: 3`.
- Required top-level sections are present and unsupported top-level fields are
  rejected.
- The `owner_areas` catalog and `applicability` catalog use stable identifiers
  with string descriptions.
- Every shared entry uses only `doc_id`, `path`, `kind`, `summary`,
  `normative_level`, `owner_area`, `created_on`, `last_updated_on`,
  `last_verified_on`, `applies_to`, `primary_audience`, `journeys`,
  `canonical_for`, and `depends_on`.
- Every paired entry uses only `doc_id`, `path_en`, `path_ko`, `kind`,
  `summary`, `normative_level`, `translation_policy`, `owner_area`,
  `created_on`, `last_updated_on`, `last_verified_on`, `applies_to`,
  `primary_audience`, `journeys`, `canonical_for`, and `depends_on`.
- Required fields are present for each shared or paired entry.
- `owner_area` resolves to the top-level owner-area catalog.
- `applies_to` is a non-empty duplicate-free list and every value resolves to
  the top-level applicability catalog.
- `created_on`, `last_updated_on`, and `last_verified_on` use valid
  `YYYY-MM-DD` calendar dates ordered as
  `created_on <= last_updated_on <= last_verified_on`.
- `kind` values are only `landing`, `tutorial`, `how_to`, `explanation`,
  `reference`, or `maintenance`.
- `normative_level` values are only `contract`, `guide`, `example`, or
  `maintenance`.
- `translation_policy` is `semantic_parity` for maintained English/Korean
  pairs.
- `primary_audience`, `journeys`, `canonical_for`, and `depends_on` are lists
  when present.
- `doc_id` values are unique.
- Every indexed path exists.
- Every `depends_on` value resolves to an indexed `doc_id`.
- Every maintained paired Markdown file under `docs/en/` and `docs/ko/` is
  represented in the index with matching relative structure.
- The exact root pair `README.md` and `README.ko.md` is accepted as the only
  maintained root-level semantic-parity pair.
- If `README.ko.md` exists, it must be indexed with `README.md` as the root
  README pair; missing indexed root README paths are reported by the normal
  path-existence rules.
- Existing-file and duplicate-path rules apply to the root README pair in the
  same way they apply to other indexed paths.
- Relative links resolve to existing files.
- Fragment links and hidden anchors resolve where they are used.
- Maintained English/Korean pairs use equivalent local Markdown reader links
  after indexed targets are normalized to `doc_id`, valid non-indexed
  repository targets are normalized to repository-relative paths, and fragments
  are preserved. The exact root README pair uses this same local semantic-link
  and fragment parity mechanism. External links, images, and fenced-code text
  are ignored for this parity check.
- Executable `volicord` command examples in shell fences use supported public
  CLI command shapes and options.
- Terminology role metadata for identity-sensitive terms uses the allowed role
  set and includes the required roles for public selectors, storage internals,
  MCP process bindings, and diagnostics.
- `docs/terminology-map.yaml` primary-owner and related-reference paths exist
  and are represented in `doc-index.yaml`.
- Focused Reference owner pages that the documentation policy marks as needing
  surface labels include a `Surface Stability` section, link to the canonical
  vocabulary, and use only `stable`, `beta`, `internal`, or `diagnostic` labels.
- Public-output source avoids unqualified broad security words that would
  overstate Volicord guarantees; exact security guarantee meaning remains with
  the Security and brand-claim owners.

After automated structural validation, manually confirm repository hygiene:

- No generated records, runtime homes, SQLite files, generated logs, archive
  copies, conversion notes, scratch notes, local inventories, or work logs
  remain in maintained documentation.

## Human Semantic Review

For bilingual changes, compare English and Korean by meaning unit. Preserve
reader purpose, normative strength, owner routing, baseline and out-of-scope
boundaries, user-judgment boundaries, negative clauses, non-claims, guarantee
strength, headings, tables, lists, examples, links, and exact identifiers.

For contract-adjacent edits, confirm exact API behavior, schema meaning, error
meaning, storage effects, security wording, access boundaries, close-readiness
meaning, value-set meaning, and Core authority semantics remain in the focused
Reference owner. Non-owner pages should summarize and link, not become second
contract bodies.

For terminology changes, check the terminology map for exact identifiers,
preferred expressions, avoid expressions, Korean mixed-language controls, and
owner path integrity.

For brand-presentation or broad-claim changes, check the [Brand Guidelines](brand-guidelines.md)
for Volicord spelling, official bilingual brand copy, component presentation,
test harness term boundaries, visual principles, and claim restrictions. Confirm
exact product behavior, API behavior, storage effects, schemas, security
guarantees, and Core authority semantics still route to their Reference owners.

For API and Reference examples, check method-local consistency, request and
response shape, field names, required fields, nullability, enum-like values,
`state_version`, refs, artifact refs, run refs, judgment refs, close-readiness
blockers, response branches, and links to applicable owners where relevant.

For Architecture Guide changes caused by code movement, confirm the relevant
Architecture Guide documents describe durable crates, modules, entry points, execution
stages, and responsibility boundaries without turning implementation detail into
product contract text.

The automated `docs-check` command includes local documentation-link parity for
maintained English/Korean pairs, but it does not perform semantic bilingual
review, contract-owner review, technical-accuracy review, translation judgment,
API example consistency review, or product meaning review. A passing local-link
parity check only confirms the machine-comparable local reader routes. The
remaining checks stay manual and owner-routed.

## Durable Tests And One-Time Audits

When a documentation or implementation change suggests a new automated check,
decide whether it is a durable contract test or a one-time audit. A durable test
belongs in the repository when it asserts current durable behavior, a contract,
a state transition, user value, a stable abstraction boundary, or a maintained
validation rule. A one-time audit belongs in the change process when it only
proves that cleanup-specific text, flags, fields, or examples were removed.

File length, document length, and LOC counts are not durable quality checks.
Use [Product and Maintenance Charter](product-maintenance-charter.md) for the
quality-gate boundary, and prefer checks that validate owner routing,
contracts, links, examples, state transitions, and reader usability.

For implementation-layer placement and test-authoring examples, use
[Testing Strategy](../architecture-guide/testing-strategy.md). This validation policy
owns the maintenance-check, review, and reporting boundaries for those checks.

Do not add permanent tests whose only assertion is a cleanup-specific string
search such as "the old option name no longer appears." Run those searches as
audits when useful, then report them outside repository files. If the absence
matters as a durable contract, test the positive current shape instead:

- CLI help exposes only the current public option allowlist for the command.
- Maintained shell examples use supported `volicord` commands and options.
- Storage schema checks assert current canonical SQL, tables, columns, indexes,
  constraints, initialization, and validation behavior.
- MCP preflight and transport/schema checks assert current startup behavior,
  public tool exposure, and public schema projection. Public MCP schemas must
  keep hiding internal envelope and invocation fields as a stable abstraction
  contract.
- Terminology validation checks identity-sensitive role metadata instead of
  adding broad prose forbidden-word searches for identifiers such as
  `connection_id` or `project_id`.

Name durable tests after the current contract, for example
`connect_help_exposes_only_public_connect_options`,
`documented_volicord_commands_match_public_cli_contract`,
`export_help_lists_authority_bundle`,
`mcp_public_schema_hides_internal_envelope_fields`,
`terminology_map_defines_identity_sensitive_roles`, or
`storage_registry_contains_current_contract_columns`. Avoid names and structures
such as `removed_options_are_gone`, `legacy_flags_are_removed`,
`old_strings_do_not_remain`, and `cleanup_removed_project_id`.

## Maintainability Report

Use the maintainability report when reviewers need quick visibility into large
or complex repository surfaces:

```sh
cargo run -p xtask -- maintainability-report
```

The report is reviewer guidance. It lists signals such as the largest Rust,
test, and Markdown files, heuristic command parsing/execution/rendering mixes,
and obvious test-coverage hints where those can be inferred cheaply. It does
not define LOC limits, LOC exception allowlists, invalid long-file states, or a
requirement to split cohesive files by line count. Treat reported sizes and
signals as prompts for review questions about ownership, readability, test
coverage, and source structure.

When the report runs in CI, a failing exit status means the command could not
inspect the repository. A large file, long document, mixed signal, or coverage
hint is not a CI failure by itself.

## Onboarding Usability Validation

Use representative-user usability validation when maintained onboarding,
installation, agent-host setup, troubleshooting, or owner-routing documentation
is added or materially changed. This is human usability testing with actual
participants. It is separate from automated `docs-check`, Rust implementation
tests, conformance checks, human semantic review, and an agent-performed desk
review. An agent desk review may find documentation-maintenance blockers, but it
is not evidence that first-time human readers can complete the flow.

The participant set must include at least:

- two technically capable users with no prior Volicord experience
- one MCP host operator with no prior Volicord experience
- one implementer who needs to navigate API or schema Reference material

The tasks must cover whether participants can:

1. Determine whether their environment is documented as suitable.
2. Build or select the executables.
3. Verify executable readiness.
4. Choose and follow one Codex or Claude Code setup path.
5. Interpret `action_required` and identify the required next action.
6. Recover from an unavailable or incorrectly selected executable.
7. Interpret a state with no allowed project or ambiguous project selection.
8. Explain what remains after safe removal.
9. Find the detailed schema owner for `StateRecordRef` or `EvidenceSummary`.

Record observations needed to improve the maintained documentation, including
where participants stop, questions they ask without prompting, incorrect state
interpretations, unsafe write or deletion attempts, whether success was
self-verified, whether recovery completed, the number and type of document
transitions, and search terms that failed.

Passing usability validation requires first-time users to complete executable
preparation and one host path without author explanation, identify documented
success independently, avoid treating `action_required` as an unexplained fatal
failure, recover without deleting unrelated user configuration or product data,
and find the detailed schema owner without author assistance. Critical blockers
include any issue that prevents task completion, causes an unsafe write or
deletion attempt, produces a wrong success interpretation, or breaks an owner
route. Correct critical blockers in the applicable maintained owner documents,
keep paired English and Korean meaning aligned when a paired document changes,
rerun matching automated and manual maintenance checks, and retest the affected
task with the relevant participant profile before treating the blocker as
resolved.

Report usability validation results in the conversation or another
repository-approved durable research location, not as individual test records in
maintained documentation. Do not commit participant notes, screenshots,
recordings, session logs, work logs, fabricated completion rates, fabricated
quotations, or private participant data to maintained docs. Do not claim
representative-user testing occurred unless actual representative participants
performed the tasks and their participation is verifiable. Automated validation
proves only the machine-checkable properties it owns, Rust tests prove only
implementation checks, and an agent desk review proves only that a maintainer
reviewed the documents for objective blockers.

<a id="live-host-connection-readiness-sequence"></a>
## Live-Host Connection-Readiness Sequence

Apply this sequence before every task-bound live-host checklist below. First,
start the installed host on the exact prepared Agent Connection while the
selected Product Repository has no active `Task`, and observe a read-only
`volicord.status` call. After that host exits, run administrative
`volicord connection verify ... --json` for the same connection and require the
owner-defined `complete` result. Only then create or activate the workflow
`Task` and start the task-bound host run. The first observation does not replace
administrative verification.

Exact connection-verification behavior and state meaning remain with
[Agent Connection](../reference/agent-connection.md) and
[Administrative CLI](../reference/admin-cli.md#agent-connection-result-states);
this section owns release-validation order only.

<a id="live-host-final-output-release-validation"></a>
## Live-Host Final-Output Release Validation

Use this checklist before publishing a release that claims managed final-output
authority disclosure for Codex or Claude Code in the Record or Detective
profile. This is authenticated, human-in-the-loop validation against the exact
release candidate. Host-configuration fixtures, direct generated-wrapper
output, ordinary workspace tests, and Judgment round trips cannot replace it.
Exact product behavior remains with [Agent Connection](../reference/agent-connection.md#managed-final-output-authority-disclosure),
[Administrative CLI](../reference/admin-cli.md#managed-final-output-authority-disclosure),
and their focused dependencies; this checklist owns release-validation
execution and evidence separation only.

Prepare an approved release-record location outside the source repository and
record the release-candidate identity and installed host identities. Every
`VOLICORD_LIVE_HOST_RESULT_PATH` must be a different new absolute path with an
existing parent directory. Run all four ignored host/profile tests separately:

| Host | Record profile | Detective profile |
|---|---|---|
| Codex | `codex_record_live_final_output_is_opt_in` | `codex_detective_live_final_output_is_opt_in` |
| Claude Code | `claude_code_record_live_final_output_is_opt_in` | `claude_code_detective_live_final_output_is_opt_in` |

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/codex-record-final-output.json VOLICORD_RUN_CODEX_RECORD_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_record_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/codex-detective-final-output.json VOLICORD_RUN_CODEX_DETECTIVE_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_detective_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/claude-record-final-output.json VOLICORD_RUN_CLAUDE_RECORD_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_record_live_final_output_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/claude-detective-final-output.json VOLICORD_RUN_CLAUDE_DETECTIVE_FINAL_OUTPUT_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_detective_live_final_output_is_opt_in -- --ignored --nocapture
```

For each cell, inspect a bounded result with the matching `host` and `profile`.
It must keep these evidence fields separate; no field can be inferred from or
replaced by another:

1. `config_fixture` identifies the managed host configuration checked for that
   profile. A passing fixture does not show that the installed host loaded the
   configuration.
2. `generated_wrapper_direct_wire.status_fallback` and
   `generated_wrapper_direct_wire.authority_receipt` check the exact generated
   wrapper through direct invocation and keep the two complete bounded response
   branches separate. Both must be `verified`, but neither shows that the
   installed host delivered an event or displayed UI.
3. `actual_host_event.status_fallback_event` and
   `actual_host_event.authority_receipt_event` separately record actual host
   delivery to the handler for both branches.
   Record intentionally creates no persistent Guard observation, so its event
   entries cite the authenticated host-owned managed-UI delivery while the
   count check proves that no Guard event or Agent Session was added. This is
   delivery evidence, not an invented durable observation.
   `actual_host_fixed_ui.authority_receipt` separately records the complete
   active-Task receipt on the host-owned fixed UI, distinct from model prose,
   and binds its Project, Task, `state_version`, latest Run, close state, and
   blocker count. `actual_host_fixed_ui.status_fallback` independently records
   the no-active-Task fixed-UI confirmation. Both nested statuses must be
   `verified` for the cell.
4. `detective_decision` is `not_applicable` for Record only when the result also
   confirms `non_observing` and `non_gating` and finds no Guard event or
   decision. For Detective it must cover both `allow` and `block`; an `allow`
   result cannot stand in for `block`.
5. The top-level `status_fallback` separately binds the no-active-Task UI
   confirmation to the generated `volicord status --json` command and absence
   of a task-bound command. Direct-wrapper fallback wire does not establish the
   UI observation. The operator copies the complete taskless message from the
   managed UI, and the harness requires exact equality so a task-bound variant
   cannot be confirmed by a command-only token. Each cell must verify this
   evidence and both branches under
   `actual_host_fixed_ui`; none replaces another.
6. `exact_replay.generated_wrapper_identical_payload` records identical-payload
   replay through the generated wrapper, while
   `exact_replay.actual_host_replay` records replay through an actual host entry
   point. For Record, repeated delivery remains non-observing and non-gating
   while refreshing the read-only display. For Detective, actual replay leaves
   the immutable historical Guard event and decision unchanged while the
   separate UI reads current authority again. The generated-wrapper check
   advances Task authority between identical deliveries and requires a newer
   current receipt while the stored historical event remains byte-for-byte
   unchanged.

Evidence statuses are `verified`, `unavailable`, `not_applicable`, or `failed`.
These are validation-harness facts rather than product response fields. A cell
passes only when every applicable evidence item is `verified`; the Record-only
Detective decision is the sole expected `not_applicable` case. If the installed
host has no safe `block` entry, no actual-host replay entry point, no active-Task
receipt UI, or no no-active-Task fallback UI, record the corresponding evidence
as `unavailable` and keep the overall `result=incomplete`. Generated-wrapper
identical-payload replay cannot replace actual-host replay.

An unavailable executable, authentication environment, interactive TTY,
event-delivery surface, active-Task receipt UI, no-active-Task fallback UI, safe
Detective `block` entry, or actual-host replay entry is never a pass. Preserve
the structured `unavailable` or `incomplete` result where the harness can write
one, then report the release-validation outcome as `SKIP` or `FAIL`. Do not
upgrade it from fixture, direct-wire, or another matrix cell. All four cells
must pass before a release claim covers both maintained hosts and both profiles.

The result recorder first writes a bounded `result=running` record and replaces
it atomically with the final result. Treat a surviving `running` record as
interrupted, and `result=incomplete` as incomplete evidence, never as a pass.
Keep the bounded results and release approver's checklist outside the source
repository. Do not commit result files, Runtime Homes, screenshots,
transcripts, recordings, credentials, secrets, full prompts, or private
operator input. The evidence applies only to the observed host, profile,
release candidate, and environment; it is not portable host conformance, a
security proof, product acceptance, close readiness, or a general correctness
claim.

<a id="live-host-judgment-release-validation"></a>
## Live-Host Judgment Release Validation

Use this checklist before publishing a release that claims the maintained Codex
or Claude Code Judgment path. This is an authenticated, human-in-the-loop release
validation against the exact release candidate. It is not replaced by schema
checks, fixtures, ordinary workspace tests, or a live test reported as ignored.
It is also separate from the four-cell final-output checklist above: evidence
from either checklist cannot satisfy the other.
Exact status and receipt behavior remains with the [status method](../reference/api/method-status.md)
and [API State Schemas](../reference/api/schema-state.md); this checklist owns
release-validation execution and evidence handling only.

Prepare an approved release-record location outside the source repository, then
record the exact release-candidate identity and both host identities. Each
result path below must be absolute, have an existing parent directory, and not
exist before the test starts; the harness rejects an existing path so a stale
`result=passed` cannot be attributed to a later run.

```sh
volicord --version
codex --version
claude --version
```

Run both ignored Judgment tests separately. Give each test a different absolute
result path in the approved external location:

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/codex-user-action.json VOLICORD_RUN_CODEX_USER_ACTION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_user_action_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/claude-code-user-action.json VOLICORD_RUN_CLAUDE_USER_ACTION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_user_action_round_trip_is_opt_in -- --ignored --nocapture
```

For each host, confirm all of these observations against the release candidate:

1. The host-native Judgment selector is visibly presented, and the human
   operator—not the Agent—selects one offered option. After the host exits, the
   operator enters `choice:route_alpha` or `choice:route_beta`; the harness
   requires that confirmation to equal the stored `selected_option_id`.
2. The Agent creates an `advisor` Task, creates its current Change Unit and
   baseline with `volicord.update_scope`, consumes the default compact result's
   selected option, and records the option-mapped no-write `shaping_update` Run
   with a non-null minimal close assessment.
3. Fresh status reports `close_state=ready`, no close blockers, and an
   `AuthorityReceipt.latest_run_ref`. The harness reads exactly that Run rather
   than choosing a row by timestamp or ID ordering.
4. The matching `user_action_requested`, `user_action_resolved`, and
   `run_recorded` authority-event payloads preserve the request, resolution,
   selected option, Run, kind, and no-write fact, and their event sequences prove
   that the selected resolution was recorded before that Run.
5. Exactly one new Task-bound Detective Stop event appears after the run's
   pre-host cursor, records `allow` with no reasons or close blockers, and stores
   the same complete `AuthorityReceipt` as fresh status. The operator copies the
   complete canonical receipt JSON from the separate host-owned managed UI; the
   harness requires exact equality, not a state-version-only confirmation.
6. The bounded JSON result reports a unique validation `run_id`, start and
   record times, host version, Volicord `build_id`, exact Agent Connection ID,
   operator-confirmed and stored choice, authority-event order, consumed Run,
   and final `result=passed` without including a transcript or prompt body. The
   external file is replaced through a same-directory temporary file and rename,
   so readers do not observe a partially written final JSON object.

The Task-bound Stop event and complete fresh receipt UI in item 5 are required
Judgment-completion evidence for this test. They still do not fill any evidence
item in the four-cell final-output matrix, whose host/profile, no-active-Task
fallback, Record behavior, block behavior, and replay checks remain separate.
Other final-output observations made during the Judgment test are diagnostic
only for that run.

If native elicitation is unavailable, the test must verify that the pending item
is visible in `volicord inbox` and that the current `volicord inbox resolve`
command shape is available. It emits bounded command templates without the
fixture's temporary paths or IDs, writes `result=failed_native_elicitation`, and
fails. The disposable Runtime Home is deleted after the test, so those templates
are not runnable recovery commands. Preserve that failed result for diagnosis,
but do not count the CLI fallback as a successful native round trip. This
Judgment inbox fallback is User Channel recovery evidence, not final-output
`status_fallback` evidence. Executable CLI recovery is owned by the separate
[live-host CLI-fallback checklist](#live-host-cli-fallback-release-validation)
below and cannot upgrade this native cell.

An unavailable executable, authentication environment, interactive TTY,
trust/approval surface, or native selector is `SKIP` or `FAIL`, never `PASS`.
Both host-specific validations must pass for a release claim that covers both
maintained hosts.

The harness requires the external result path and first writes a bounded
`result=running` record. Any ordinary early return or panic before an explicit
final result atomically replaces it with `result=failed_before_completion`.
Treat a surviving `running` record as an interrupted test, never as a pass.

Keep each bounded JSON result and the release approver's checklist record in the
approved release location outside the source repository. Do not commit result
files, Runtime Homes, screenshots, transcripts, recordings, credentials,
secrets, full prompts, or private operator input to maintained documentation or
the source repository. The structured result is release-validation evidence for
the observed host and environment only; it is not portable host conformance, a
security proof, product acceptance, close readiness, or a general correctness
claim.

<a id="live-host-evidence-observation-release-validation"></a>
## Live-Host Evidence-Observation Release Validation

Use this checklist before publishing a release that claims the maintained
Codex or Claude Code evidence-observation path through local web consent. This
is authenticated, human-in-the-loop validation against the exact release
candidate. It requires an actual installed host to create and resume the
request, negotiate the exact model-invisible capability, present the host-only
handoff outside model context, and let a human submit the canonical form in a
local browser. An ignored test, fixture-only check, ordinary workspace test,
direct MCP-adapter test, native Judgment result, CLI-fallback result, or final-
output result cannot replace it.

Exact request and resume behavior remains with
[`volicord.request_user_action`](../reference/api/method-request-user-action.md),
resolution authority remains with
[`volicord.resolve_user_action`](../reference/api/method-resolve-user-action.md),
common request and resolution fields remain with
[API User Action Schemas](../reference/api/schema-user-action.md),
Run and evidence effects remain with
[`volicord.record_run`](../reference/api/method-record-run.md), and local-web
routing remains with [MCP Transport](../reference/mcp-transport.md#local-web-consent-fallback).
Exact status and receipt behavior remains with the
[status method](../reference/api/method-status.md) and
[API State Schemas](../reference/api/schema-state.md). This checklist owns only
release-validation execution, evidence separation, and safe result retention.

Prepare an approved release-record location outside the source repository and
record the exact release-candidate and installed-host identities. A local
browser must be able to reach the loopback consent listener. Each result path
must be a different new absolute path with an existing parent directory; the
harness rejects an existing path. Run both ignored cells separately from an
interactive TTY in the ordinary authenticated host environment:

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/codex-evidence-observation.json VOLICORD_RUN_CODEX_EVIDENCE_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_evidence_observation_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/claude-code-evidence-observation.json VOLICORD_RUN_CLAUDE_EVIDENCE_OBSERVATION_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_evidence_observation_round_trip_is_opt_in -- --ignored --nocapture
```

For each host, confirm all of these observations against the release candidate:

1. Store inspection observes no `UserActionRequest` before host launch and,
   afterward, exactly one request created by the installed host on the prepared
   Agent Connection. Its target, artifact candidate, and `required_for` facts
   match the fixture and the request and schema owners linked above.
2. The captured initialization exchange contains exact boolean `true` at
   `params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`.
   The create response contains the closed local-web handoff only at
   `CallToolResult._meta["io.volicord/user-channel"]`, and the installed host
   visibly presents that handoff on a host-owned surface outside model context.
   The human operator—not the Agent or harness—uses that surface to open the
   loopback form and submit the prepared target and artifact, `supported`, and a
   bounded non-secret summary. This assertion covers exact capability
   negotiation, selected path, and human participation, not secret detection or
   native elicitation.
3. Store inspection observes one immutable resolution and compares it with the
   focused resolution and schema owners. The persisted fields are
   `resolved_by_actor_source=local_user`,
   `channel_kind=local_web_consent`, and a
   `resolved_verification_basis` equal to the recognized local-web basis supplied
   by the User Channel adapter. This checklist does not establish a new stable
   basis value. The stored body matches the prepared target and `ArtifactRef`,
   `supported`, and the operator's bounded summary. Operator re-entry after the
   host exits confirms exact equality with the stored summary.
4. Same-connection diagnostics and Store inspection observe one resume for the
   exact request, `agent_workflow_result_replayed=true`, no second request or
   resolution, and later consumption of the exact resolution ref. A bounded
   exchange observer checks the actual create and resume model-visible
   projections, including MCP `content`, `structuredContent`, compatibility and
   diagnostic text, and the replayed Agent Workflow body. The historical
   pending request is exactly
   `{user_action_request_id, status=pending, next_actor=user}`, and the complete
   request, question, options, context, form, capture path, command, raw URL,
   bearer token, user note, and evidence summary are absent.
5. Store inspection compares the consuming Run, one evidence observation,
   producer and relevance anchors, exact artifact, Core-derived observation
   time, required-criterion coverage, and request-resolution-Run event order
   with the `record_run` and state-schema owners linked above. These are
   observed owner-conformance assertions, not definitions supplied by this
   checklist.
6. While the request is pending, the cell also observes one status result, one
   blocked close result, and the first exact operation-result page. Their model-
   visible pending projection is the same exact three-field summary, all
   forbidden fields from item 4 are absent, and the operation-result page is
   withheld unless the entire stored response satisfies the current closed
   shape. After resolution, fresh status follows
   `AuthorityReceipt.latest_run_ref` to the consuming Run and the cell compares
   the observed ready state and empty blocker set with the status and state
   owners. It also requires one new Task-bound Detective Stop `allow` event
   after the pre-host cursor and exact equality among the stored Stop receipt,
   fresh status receipt, and the complete receipt copied from the separate host-
   owned managed UI.
7. The closed, bounded external JSON records
   `kind=live_host_evidence_observation_release_validation`, safe validation
   coordinates, owner-comparison results, model-invisible capability and host-
   presentation Booleans, per-projection safe-shape Booleans and digests, and
   only an exact-match Boolean and bounded character count for the operator
   summary. It must not contain the consent URL, bearer token, raw tool body,
   raw summary, prompt or transcript content, screenshots or recordings,
   credentials, secrets, or private operator input.

The result recorder first writes a bounded `result=running` record. A missing
host executable, non-interactive TTY, omitted or malformed exact capability,
missing host-only presentation surface, or inability to distinguish host-only
`_meta` from model-visible result data is recorded as `result=unavailable`.
Fixture setup failure, abnormal termination of a selected host, or a stored
state, Stop, receipt, or result-validator invariant failure is recorded as
`result=failed` with only a safe stage identifier. Authentication and browser
failures cannot always be classified before host launch; if they cause the
selected host run to fail, the result is `failed`, not `unavailable`. Only an
unexpected unwind retains `result=failed_before_completion`. Treat a surviving
`running` record, every non-`passed` result, a test merely reported as ignored,
or a run without its opt-in variable as not passed.

Both host-specific cells must pass before a release claim covers both
maintained hosts. This cell proves only the observed evidence-observation
local-web path. It does not satisfy, and cannot be satisfied by, the native
Judgment, executable CLI-fallback, host-configuration, or final-output cells.
Keep the bounded results and release approver's checklist outside the source
repository. Do not commit them or any Runtime Home. The evidence applies only
to the observed host, release candidate, and environment; it is not portable
host conformance, a security proof, native elicitation evidence, product
acceptance, close readiness, or a general correctness claim.

<a id="live-host-cli-fallback-release-validation"></a>
## Live-Host CLI-Fallback Release Validation

Use this checklist before publishing a release that claims executable CLI User
Channel recovery for the maintained Codex or Claude Code host path. This is an
authenticated, human-in-the-loop release validation against the exact release
candidate. It is separate from both the native Judgment cells and the four-cell
final-output matrix. A command template, ordinary CLI integration test, native
elicitation result, or final-output result cannot satisfy this checklist.
Exact CLI and resume behavior remains with [Administrative CLI](../reference/admin-cli.md#user-channel-commands),
[Agent Connection](../reference/agent-connection.md), and
[`volicord.resolve_user_action`](../reference/api/method-resolve-user-action.md);
this checklist owns release-validation execution and evidence separation only.

Prepare an approved release-record location outside the source repository and
record the exact release-candidate and installed-host identities. Each result
path must be a different new absolute path with an existing parent directory.
Run both ignored cells separately:

```sh
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/codex-cli-fallback.json VOLICORD_RUN_CODEX_CLI_FALLBACK_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke codex_live_cli_fallback_round_trip_is_opt_in -- --ignored --nocapture
VOLICORD_LIVE_HOST_RESULT_PATH=/path/to/approved-release-records/claude-code-cli-fallback.json VOLICORD_RUN_CLAUDE_CLI_FALLBACK_SMOKE=1 cargo test -p volicord-cli --test live_host_smoke claude_code_live_cli_fallback_round_trip_is_opt_in -- --ignored --nocapture
```

For each host, confirm all of these observations against the release candidate:

1. The harness prepares one current pending two-option product-decision request
   for an `advisor` Task, current Change Unit, baseline, and the exact Detective
   Agent Connection that the installed host will use. The human operator—not
   the Agent—chooses `route_alpha` or `route_beta`.
2. The actual `volicord inbox --json` result shows that exact request once. The
   harness submits the human choice through the actual
   `volicord inbox resolve ... --choice ... --json` command, then runs the exact
   same command and arguments again. The two JSON byte sequences must be
   identical, the first resolution must advance state once, and the retry plus
   fresh status must preserve the committed `state_version`.
3. The stored resolution has one resolution ID,
   `resolved_by_actor_source=local_user`, `channel_kind=cli`, and a
   `resolved_verification_basis` recognized for the actual CLI User Channel
   path by the resolution owner. Its selected option must equal the operator
   choice. A path-free command template or `--help` result does not meet this
   item.
4. The installed host starts on the prepared Agent Connection and calls
   `volicord.request_user_action` with `request.operation=resume` for the exact
   request ID. Same-connection diagnostics must observe replay of the
   originating result, and the Task must still contain exactly one
   product-decision request. The host then records exactly one option-mapped
   no-product-write `shaping_update` Run whose `created_by_actor_source` names
   that Agent Connection.
5. Matching `user_action_requested`, `user_action_resolved`, and `run_recorded`
   authority events bind the request, resolution, CLI channel, exact Run, kind,
   and no-write fact in request-before-resolution-before-Run order. Fresh status
   must follow its `AuthorityReceipt.latest_run_ref` to that Run and report
   `close_state=ready` with no blockers.
6. Exactly one new Task-bound Detective Stop event after the pre-host cursor
   records `allow` with no reasons or close blockers and stores the same complete
   receipt as fresh status. The operator copies the complete canonical receipt
   from the separate host-owned managed UI, and the harness requires exact
   equality rather than a state-version-only token.
7. The bounded JSON result has
   `kind=live_host_cli_fallback_release_validation`, `result=passed`, the CLI
   basis and exact-retry facts, same-connection resume evidence, mapped Run and
   event order, Stop coordinates, receipt coordinates, and complete managed-UI
   confirmation. Its evidence scope explicitly identifies this CLI-fallback
   cell and excludes native Judgment and final-output matrix cells.

The result path is mandatory. The recorder writes `result=running` before the
host launches and atomically replaces it with the bounded final or
`failed_before_completion` record. Treat a surviving `running` record, any
non-`passed` result, an unavailable executable, authentication environment,
interactive TTY, same-connection resume path, Task-bound Stop, or complete
receipt UI as `SKIP` or `FAIL`, never as a pass. Both host-specific cells must
pass before a release claim covers both maintained hosts.

Keep the bounded results and release approver's checklist outside the source
repository. Do not commit result files, Runtime Homes, screenshots, transcripts,
recordings, credentials, secrets, full prompts, or private operator input. This
evidence applies only to the observed host, release candidate, and environment;
it is not portable host conformance, a security proof, native Judgment
elicitation evidence, final-output matrix evidence, product acceptance, close
readiness, or a general correctness claim.

## Rust Implementation Validation

If no Rust source, Cargo manifest, test, fixture, or build configuration is
changed, Rust validation is not required.

After Rust implementation edits, run the applicable Rust validation from the
workspace or changed crate:

- `cargo fmt`
- `cargo clippy --all-targets --all-features`
- `cargo test --all-targets --all-features`

Use narrower Cargo commands only when the repository structure or task scope
clearly calls for them, and report the reason.

## Generated Reference And Contract Drift Checks

Generated or source-derived reference surfaces use stable check commands:

- `cargo run -p xtask -- docs-check` checks maintained documentation structure,
  generated/source-derived documentation surfaces, executable `volicord`
  command examples, terminology metadata owner paths and roles, and canonical
  Storage DDL SQL blocks against `crates/volicord-store/src/schema/registry.sql`
  and `crates/volicord-store/src/schema/project.sql`.
- `cargo test -p volicord-integration-tests --test public_contract_snapshots`
  checks generated public contract snapshots for API request schema projections
  and MCP workflow/read-only tool projections against their Rust sources.
- To regenerate those public contract snapshots after an intentional source
  change, run
  `VOLICORD_UPDATE_CONTRACT_SNAPSHOTS=1 cargo test -p volicord-integration-tests --test public_contract_snapshots`
  and review the generated files under `tests/integration/snapshots/`.

The public contract snapshot files are generated test artifacts marked with
`_generated`. Do not edit them by hand; update the schema or MCP source first,
then regenerate. CLI public command drift remains covered by executable
documentation examples and CLI help/output tests such as the `volicord-cli`
`binary_admin` and `mcp_transport` test targets rather than by a separate CLI
JSON schema.

## Storage DDL Contract Check

When editing Storage DDL, `volicord-store` canonical SQL, or schema validation
code, run the focused owner-to-implementation consistency check:

```sh
cargo test -p volicord-store --test storage_ddl_contract
```

This check compares the authoritative English and Korean Storage DDL SQL with
the schemas initialized from canonical registry/project SQL in in-memory SQLite
databases. It checks schema semantics such as tables, columns, defaults,
constraints, foreign keys, indexes, partial indexes, and maintained triggers
without comparing Markdown prose or SQL formatting.

The repository documentation check also validates that the marked canonical SQL
blocks in English and Korean Storage DDL match the canonical registry/project
SQL source files.

This is a repository maintenance and implementation consistency check. It is
distinct from general documentation structure validation, public runtime
conformance, product acceptance, QA completion, close readiness, security
proof, and residual-risk acceptance.

## Reporting

Report validation results in the conversation, not in repository files. Include
changed files, checks performed, results, skipped checks with reasons, and
remaining documentation risks.

Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance or
implementation-check outcomes. Do not describe a passing validation step as
Volicord runtime conformance, product acceptance, QA completion, close readiness,
a security guarantee, or residual-risk acceptance.
