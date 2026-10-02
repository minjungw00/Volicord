# Reconstruction validation assets

This directory contains maintained inputs, assertions, report summaries, and
small disposable experiment implementations. It is an internal validation
surface, not a Volicord product command or production architecture.

## Commands

- `rebuild/scripts/validate self-test` checks command execution, output and
  status preservation, signal reporting, and non-fail-fast aggregation with
  fake commands.
- `rebuild/scripts/validate gate-self-test` checks admission blocking,
  including independent failure of either contract-coverage mode, authorization
  separation, exact-once synthetic final/V11 orchestration,
  same-session artifact selection, credential-safe capsule projection, and
  no-retry behavior. It never invokes the real exact final or official V11.
- `rebuild/scripts/validate gate-entrypoint-self-test` copies the maintained
  admission/gate entry point into isolated temporary Git repositories and
  checks clean, initially dirty, and admission-generated dirty candidates
  without invoking the real exact final or official V11.
- `rebuild/scripts/validate evidence-archive-self-test` builds a synthetic
  gate/V11 artifact tree and exercises result discovery, logical repository and
  V11 target cwd projection, argv sanitization, archive creation, and independent
  verification. Its production-scale case uses the real collector and sanitizer
  for at least 178 process records, 1,116 argv entries, and 659 non-structural
  argument-role records. It also rejects a builder-side over-bound member,
  unknown external cwd values, retained prompts or absolute host paths, tampered
  content, missing members, changed POSIX modes, candidate mismatch, repository
  source bodies, malformed or misplaced provider-retention attestations, and
  credential-like prohibited content. Its positive integration case embeds the
  maintained successful-provider evidence shape, including the exact negative
  retention attestation. It invokes neither exact final nor official V11.
- `rebuild/scripts/verify-validation-archive <archive> [--expected-candidate
  <HEAD>]` independently verifies archive membership, content hashes, bounded
  size, candidate agreement, tracked/executable identities, tar modes, and the
  prohibited-content boundary without extracting the archive. Builder and
  verifier both enforce the manifest-declared current 256 KiB uncompressed
  per-member limit; the builder encodes and checks every member before writing
  the tar, while the existing 512 KiB compressed-archive limit remains separate.
- `rebuild/scripts/check-validation-report --self-test` proves generic report
  compatibility plus positive and negative capsule-backed semantics for
  admission-blocked, final-failed, provider-live-qualification-failed, V11-preflight-failed,
  official-V11-failed, and fully-passed lifecycles. It also rejects impossible
  stage combinations and missing success evidence; neither exact final nor
  official V11 is invoked.
- `rebuild/scripts/validate focused <label> -- <command> [arguments...]` runs
  the exact argument vector from the repository root and records the command,
  working directory, timestamps, duration, complete separate stdout/stderr,
  exit code, and termination details.
- `python3 rebuild/validation/shared/current_cli_parity.py --binary
  rebuild/target/debug/volicord` executes maintained Dogfood/V08 command shapes
  against the actual current Clap parser and requires the corresponding removed
  forms to fail as usage errors.
- `rebuild/validation/shared/strict_fake_volicord.py` is bounded self-test
  support for Dogfood campaign and repeated-resource paths. It accepts only the
  maintained repository-selected command ordering and fails every unexpected
  option, command, or subcommand.
- `rebuild/scripts/validate admission` is optional cheap diagnostic preflight:
  candidate cleanliness, tools, writable disposable homes, resources, loopback,
  authentication, network, exact model and both transmission authorizations.
  It starts no deterministic support, Final, provider or V11 work. Exit 0 means
  `preflight_passed`, with `preflight_eligible = true` and `eligible = false`.
  Support checks are explicitly `not_run` with a reason. This diagnostic supplies
  no trusted artifact to the gate and need not immediately precede it.
- `rebuild/scripts/validate gate` owns authoritative admission. Cheap blockers
  record all support as `not_run` with blocking check IDs. Once cheap checks pass,
  it runs the deterministic runner/V11/gate/entrypoint/archive/report, contracts,
  architecture, RI, Dogfood, provider and fixture support once, preserving every
  result even after a failure. It then rechecks clean HEAD and invokes the ordered
  four-command Final once. Final's Cargo metadata and test output must show each
  mapped contract test's actual successful execution before provider qualification
  starts. Only fresh evidence from this invocation is used for the provider, V11
  preflight, official V11, credential audit and independently verified archive.
  Direct `final` is refused. Local support ownership and the deduplication evidence
  are maintained in [validation-plan.md](../docs/design/validation-plan.md#31-maintained-final-provider-qualification-v11과-documentation-handoff-lifecycle).
- `rebuild/scripts/check-architecture-contracts` checks the nine active Phase 3
  owner documents, routing, relative links, traceability IDs, capability-based
  validation paths, prohibited supported paths, Phase 4 handoff structure,
  canonical relation orientation, Candidate inspection/privacy/lifecycle
  structure, and Guarded confirmation/dispatch structure.
- `rebuild/scripts/check-architecture-contracts --self-test` copies maintained
  inputs to isolated temporary fixtures and demonstrates positive validation
  plus independent structural failures, including direction reversal and
  Candidate/Guarded omissions, without modifying active documents.
- `python3 rebuild/validation/shared/contract_coverage.py` verifies that each
  mapped cross-owner behavior names its required Local Operations, CLI, MCP,
  or Viewer product-entry-point tests as direct, plain `#[test]` Rust integration
  tests. It rejects comments/literals, ordinary functions, ignored/conditional or
  nested declarations. The gate binds them to actual Final workspace targets and
  successful libtest records; static success alone does not prove execution or
  the semantic adequacy of a test body. Its `--self-test` mode also rejects
  internal-primitive-only coverage, missing entrypoints, canonical-only or
  over-broad forgetting, ignored cleanup/repair failure, Candidate
  error-to-empty conversion, and configured-provider failure reported as
  commercial semantic-provider success.

The architecture checker is deterministic internal test support. It does not
define domain meaning, judge conceptual correctness, choose implementation
technology, or treat validation IDs as product-format versions.

Portable process payloads carry one top-level `sanitized_argv_policy`. That
policy declares the sanitized projection, identifies exact raw argv as ignored
local execution evidence, and defines omitted per-argument role entries as
allowlisted structural tokens. Each execution then stores argv plus exactly one
compact `[argument index, classification, semantic role]` record for every
projected or redacted argument. The independent verifier rejects the superseded
duplicated per-execution accounting fields; there is no alternate reader or
numeric format branch.

## Maintained and generated boundaries

Commit self-authored fixtures, their manifest entries, experiment source,
assertions, report templates, and reviewed report summaries. Fixture entries
record purpose, expectations, unsupported constructs, a deterministic content
hash, origin, and license.

Do not commit raw analyzer output, generated graphs, copied source repositories,
logs, measurement scratch data, caches, or local model output. The runner writes
these to ignored `rebuild/.local/validation/`. Maintained reports may cite an
artifact path and hash, but local artifacts are reproducibility evidence rather
than durable design truth.

V01, V03, and V05 remain stable report and metadata identifiers. Tracked
validation assets use capability-based paths:

```text
shared/fixture-manifest.json
shared/report-template.md
repository-intelligence/polyglot-structural/
repository-intelligence/realistic-qualification/
repository-intelligence/phase-5-acceptance/
canonical-context/portability/
inquiry/frontier-resume/
inquiry/phase-6-acceptance/
wave-1-summary.md
phase-4-summary.md
phase-5-summary.md
phase-6-summary.md
```

The shared fixture manifest is the single fixture catalog. Each capability
directory owns its fixtures, assertions, disposable prototype, and maintained
report. The three validations share the report shape in
`shared/report-template.md` without turning spike code into a production
dependency.

The Phase 6 acceptance orchestrator maps V09 requirement identities to named
Production Rust tests. It validates orchestration and evidence completeness
only; Candidate, frontier, Decision, Checkpoint, Recall, and inspection
semantics remain owned by the Production Rust crates.

V07 product acceptance maps canonical forgetting through Local Operations and
the public CLI, MCP, and Viewer adapters. The lower-level canonical tombstone
and privacy cleanup tests remain separately classified owner tests and cannot
by themselves satisfy the cross-owner forgetting requirements.

The V11 multi-repository harness is under
`end-to-end/multi-repository/`. It performs a non-fail-fast rehearsal against
the installed CLI and MCP surfaces, writes child-operation evidence and its
structured result only below ignored `rebuild/.local/`, and classifies missing
public product paths as `unsupported` instead of substituting validation-only
domain behavior. Its maintained journey seeds linked and unrelated Candidate
and managed-Derived controls, inspects them after public forgetting and
restart, and injects unavailable, corrupt, and unsupported Candidate stores to
require an explicit degraded dependency state while canonical inspection
remains usable.

The Viewer snapshot consumer treats whole export's labeled repository Code scope
separately from current-Work MCP components. It checks shared Analysis Snapshot
basis, native entity/list fragment and label identity, exact relation endpoints,
retained resolved edges and inspectable reduced diagrams. Attribute order has no
semantic meaning. These supporting checks do not replace the browser runner's
independent canonical/source expectations or direct human reading observations.

## Scripted conformance and naturalistic dogfood

V11 is the maintained scripted conformance boundary. Its deterministic journey
proves the installed product path and remains the reusable Phase 8 regression;
it is not evidence that an agent independently discovered and used the accepted
experience in a real repository session.

<!-- phase8-active-operations:start -->
A new Naturalistic campaign binds the current clean Git `HEAD`, three actual
pinned repositories, and eight UTF-8 task files before activation. The task
manifest contains exactly five Work-slot mappings with `start` files and
`resume` files for the three Work A slots. `prepare --repositories <json>
--tasks <json>` creates three independent journeys and an eight-entry run
sheet. Volicord A → Resume A → B → C shares one Project, workspace and Runtime
Home while retaining three Work identities. Small-Python and Polyglot each
have their own Project, workspace and Runtime Home.

The operator explicitly controls repository and SessionStart-hook trust,
runs eight distinct fresh Codex CLI or VS Code extension sessions with the frozen task bytes,
and preserves their raw rollouts. `activate-all` requires complete frozen
preparation. `collect-batch` verifies candidate, task, repository/revision,
session, Project/Work, resume and raw hash integrity, then publishes one
immutable evidence set with journey-final bundle, documents, Viewer and Git
state. `evaluate` appends factual machine findings. Semantic observations
remain available for post-hoc `prepare-qualitative-review`, agent/human
review, and `qualify`; no evaluator-private profile, provisional review,
reveal, reconciliation or Work seal is needed to execute.

A `not_observed` assessment records an optional opportunity that did not
arise and is separate from satisfied, violated and insufficient evidence.
Five frozen workload intents intentionally vary ordinary user requests without semantic
expected answers or Question/Learning count requirements. Explicit learning/collaboration
in Volicord A requires review of runtime recognition, meaningful forks and recovery;
absent Learning behavior cannot silently become `not_observed`. Campaign-level
`interaction_coverage_adequacy` is required: satisfied may qualify, insufficient evidence
leaves qualification unresolved, and substantive observed Product violation blocks.
Review consumes raw interactions, factual diagnostics, source authority and independent
agent semantic judgment. Optional opportunities outside required intent coverage may
still be `not_observed`. Exact-candidate
technical and evidence-integrity failures remain hard; direct human Viewer,
browser and applicable Decision comprehension cannot be supplied by agents.
Qualification never grants Phase 9 readiness. Only explicit
`approve-phase-9` authorization can do so. Result-lineage publication and
verification preserve the exact evidence and decision chain.
<!-- phase8-active-operations:end -->

`prepare-qualitative-review --include-raw-rollouts` selects immutable raw Work/resume
inputs for bounded reviewer-safe conversation projections under the current
`naturalistic_review_capture` schema 1 / `naturalistic-review-capture-1` policy.
It copies no complete raw rollout. Each index entry distinguishes origin member/raw
bytes/SHA-256 from projected review bytes/SHA-256 and exposes limits, omission counts
and semantic completeness. Irrelevant tool/source/process bodies are excluded by
allowlist; sensitive required conversation bodies are omitted whole. No credential-like
literal is allowlisted. A semantically incomplete required capture forces evidence
insufficiency for decisive interaction judgments, including campaign coverage. See
`docs/design/qualitative-review.md` for the maintained schema and exact restrictions.

Naturalistic target Git state is factual review evidence. Dogfood requires no Work
commit, clean distinct-Work boundary or clean final target. Zero/multiple commits,
dirty cross-Work state and later combined commits remain collectible. The run sheet
follows task/repository-owned Git policy; compliance is assessed in post-hoc review.
Canonical Work identity remains independent of commits, paths and HEAD intervals.

### Naturalistic workload selection

The existing 3 journeys / 5 Works / 3 resume pairs / 8 sessions fit five ordinary
user intents. `workload_intents.py.contract()` owns the mapping:

| Work (and fresh resume when present) | Workload intent | Selection requirement |
| --- | --- | --- |
| Volicord A + Resume A | `learning_collaborative` | The frozen first turn explicitly requests learning/collaboration on important alternatives and trade-offs, while permitting routine details without interruption. Resume recovers the relevant learning/context state. |
| Volicord B | `decision_rich` | Ordinary user-facing policy or UX work leaves realistic materially different outcomes open; current source authority and commitments determine whether a Question was needed in post-hoc review. |
| Volicord C | `routine_bounded` | Clear work with strong repository authority and routine implementation freedom supplies a control for unnecessary interruption. |
| Small-Python A + Resume A | `exploratory_debugging` | Cause or viable implementation path is genuinely uncertain; investigation, research and deferment remain valid. |
| Polyglot-Medium A + Resume A | `cross_stack_integration` | Useful work spans relevant language/component boundaries; review assesses source grounding and cross-component understanding. |

Each task-manifest Work mapping contains `workload_intent`,
`learning_collaboration_statement` (null outside the learning Work), and its exact
`start`/`resume` file paths. The learning excerpt must be nonempty and verbatim in
the first frozen task; its meaning remains reviewable, with no keyword classifier.
These fields describe task selection, never expected answers, user ownership,
Materiality dimensions, required Questions or Learning operation counts. They
are frozen and hash-bound with the descriptor; only the exact task bytes become
the user request. Tasks must use natural wording without internal operation names
or evaluator vocabulary. No blind semantic preflight or semantic seal exists.

The five Works need no additional sessions: one Work owns each intent and each
A resume continues that Work with recovery, investigation or verification as
appropriate to its actual state. Volicord B/C must remain independently useful
under reasonable earlier outcomes; avoid a later task that assumes a particular
choice in A/B. The maintained `fixtures/workload-tasks.json` demonstrates bounded,
ordinary tasks under this topology, without semantic expected outcomes. It is
regression support, not qualifying real-use evidence or a substitute for selecting
tasks against the actual pinned repositories before preparation.

## Historical Phase 8 result record

The maintained Phase 8 candidate-authority summary is `phase-8-summary.md`.
The values below record prior results only and do not select a new campaign
candidate.

The sealed candidate retains current Codex exec/command-role evidence normalization,
exact successor-Review Decision lineage, separate redundant-Question detection,
and active-host cross-locale realization preparation/record/collection. Active
Codex guidance now consumes a clear answer using its existing valid presentation
receipt and current user turn, and executes explicitly requested resume verification
instead of stopping at Recall or inspection. Legitimate clarification, inspection-only
requests and numeric failure/indeterminate outcomes retain their existing boundaries.

The latest terminal campaign, `phase8-naturalistic-20260910-60ffad042346`, used
`de4c6d1bea16947616ed6fb78baa8f8c321ef4ff` and remains failed, diagnostic-only,
and non-reusable. Its five bounded findings and committed remediation are recorded
in `dogfood/report.md`. Historical learning-continuity and older campaigns remain
separate diagnostics. No fresh naturalistic campaign has run for the new candidate;
human review is `not_provided`, replacement qualification remains pending,
`replacement_pass_candidate = false`, and `phase_9_ready = false`.

The operator workflow is batch-first: after all five ordinary Works and eight
tasks are frozen, the user approves repository/hook trust, completes eight
fresh naturalistic chats without per-session evidence processing, and supplies
the raw rollouts once to `collect-batch`. The helper derives journey/Work/session mapping, three
journey-final bundles, bounded Runtime and activation summaries, four document
kinds per journey, and static Viewer snapshots.
Evaluation may complete without qualitative review; unresolved criteria stay
unresolved and confirmed hard violations cannot be waived by any reviewer.

Predecessor Dogfood descriptors, captures, Runtime Homes, workspaces, bundles,
observations, and session identities remain non-reusable for a future candidate.
Any predecessor Small Python cycle is historical diagnostic evidence only and is not qualifying
evidence for the redesigned campaign.
Replacement passage remains not established, and Phase 9 may not begin.
Qualifying Dogfood must run from a separate clean worktree whose actual Git
`HEAD` exactly matches the candidate identified by its own
successful maintained gate and verified capsule/evidence archive. Historical
candidate `6031641c46cf014a754442dcee3137caf265882e` and any later documentation
HEAD remain distinct; neither can qualify a different HEAD through a helper
argument.

Technical V11 is a deterministic rehearsal across the three maintained repositories
and fresh Runtime Homes. Its `ordinary_work` writes a disposable marker after
readiness and checks that the Guarded store hash is unchanged; it does not perform
naturalistic source/test/config behavior work. Phase 8's repeated campaign and
qualitative review own work quality, practical context recovery, Question relevance,
interruption cost and document usefulness. A successful technical gate opens
technical entry only. Phase 9 requires the separate qualified state and explicit
operator approval.

## Admission, authorization, and handoff

Technical network availability and authorization are separate current-
invocation inputs. `--external-network available` asserts only that the
execution environment can reach the service. It does not authorize a
transmission. Missing or escalation-dependent technical access is an
`environment_blocked` admission result.

The two accepted authorization assertions are independent:

```text
--authorize-external-transmission v11-openai-codex-project-health-three-targets
--authorize-provider-source-transmission openai-codex-background-semantic-bounded-rust-v1
```

The first covers the maintained journey's three authenticated Codex turns for the
`volicord`, `small-python`, and `polyglot-medium` targets. Their destination is
the OpenAI Codex service used by the installed Codex CLI; their purpose is to
select the installed `project_health` MCP tool; their intended source scope is
the bounded V11 prompt, Project identity, and tool result, not repository
source bodies.
The second covers one production-provider qualification transmission to that
same service for the sole maintained source
`rebuild/validation/privacy/background-provider-qualification/fixtures/bounded-rust/src/lib.rs`
(at most 4096 bytes), for the purpose of qualifying bounded semantic analysis;
it excludes every other repository source. Credentials, generic network access,
sandbox escalation,
Project provider opt-in, an earlier report, or an earlier session cannot
supply either assertion, and neither assertion supplies the other. Provider
qualification also requires `--provider-model <exact-model>`. Admission records
only the exact assertion IDs and model, not operator authorization prose or
credential contents.

Optional cheap diagnostic example (the gate performs its own admission):

```text
rebuild/scripts/validate admission \
  --external-network available \
  --authorize-external-transmission v11-openai-codex-project-health-three-targets \
  --authorize-provider-source-transmission openai-codex-background-semantic-bounded-rust-v1 \
  --provider-model <exact-model>
```

Authoritative exact-candidate gate example (no preceding admission is required):

```text
rebuild/scripts/validate gate \
  --external-network available \
  --authorize-external-transmission v11-openai-codex-project-health-three-targets \
  --authorize-provider-source-transmission openai-codex-background-semantic-bounded-rust-v1 \
  --provider-model <exact-model>
```

Admission writes its current structured result to the path reported on
stderr. The gate writes `admission.json`, `gate-result.json`, and `capsule.json`
under the reported ignored run directory and prints the complete capsule to
stdout so it can be copied before that directory disappears. It also builds
and independently verifies `validation-evidence-<candidate-prefix>.tar.gz`, then
reports the archive path and SHA-256 on stderr. A reviewer may copy that
archive out of ignored local state and verify it with
`rebuild/scripts/verify-validation-archive`.

Before archive verification completes, the retained capsule and gate result are
non-passing and identify the archive as pending. A successful exact final,
official V11, and credential audit therefore cannot become consumer-visible as
a passed top-level gate by themselves. Archive creation or verification failure
publishes a corresponding blocked capsule with `phase_8_ready = false`; no final
or V11 retry is performed.

Independent archive verification proves membership, hashes, modes, bounds,
candidate agreement and prohibited-content integrity. It does not replay Final,
provider or V11 commands, inspect mapped test semantics, or independently establish
naturalistic work quality. The gate owns technical execution truth. Complete stdout/stderr, detailed V11 local
artifacts, and raw command evidence remain under `rebuild/.local/validation/`.
The archive contains only the capsule, sanitized admission/gate/final/process
records, candidate-bound tracked validation-tool identities and executable
modes, plus a member hash and POSIX-mode manifest. Repository-root cwd is
represented as logical `.`, and maintained official-V11 repository/clone
execution roots are represented by bounded target/root identities. Arbitrary
external cwd values are rejected. Portable argv retains only executable,
command, subcommand, flag, closed-value, and owned-path roles that an explicit
command-family policy classifies as structural. Recognized paths are projected;
private prompts, inline programs, config/message bodies, identities, content
operands, and every unknown role are redacted regardless of lexical shape. Each
argument records whether it is structural, projected, or redacted. The vector
is labeled as a sanitized portable projection, while exact raw argv remains
only in ignored local evidence.
The structured records retain timestamps, duration, exit/wrapper status,
termination, and spawn state. The capsule retains the original final-summary
hash, dependency/fixture identities, official-V11 state, credential audit,
same-session ownership, and verified archive identity.

The archive excludes stdout/stderr bodies, detailed provider artifacts,
environment dumps, repository source bodies, prompts, `auth.json` contents,
credentials and reusable credential fingerprints. The verifier reads members
in memory, refuses unsafe or unexpected member types and paths, and rejects
missing or additional files, hash/size/mode drift, internal or caller-supplied
candidate mismatch, invalid tracked/executable evidence, and prohibited keys
or credential-like values. The only sensitive-name exception is the exact
`capsule.json.live_provider_qualification.evidence.retained_evidence` object:
it must contain exactly `source_body`, `provider_response_body`, and
`credential`, and every value must be the literal boolean `false`. The same
keys at any other location, any other value, a missing or additional field, or
a malformed container is rejected independently. Tar member modes are
preserved and checked; the archive file itself is created mode `0600` in
ignored local state.

The versionless current capsule has `kind = validation_handoff_capsule`. It is
one stage-dependent contract rather than separate success and failure schemas.
Its bounded cross-session evidence is:

- mapped contract-test execution owner/status/count from Final metadata and test output;
- validated candidate HEAD, sanitized admission check name/status, pre-final
  check, and any gate blocker;
- Linux OS/release/platform, machine/architecture, and Python runtime identity;
- bounded Python, Git, Cargo, Rust compiler, and installed Codex CLI version
  probes, including explicit unavailable/error state;
- SHA-256 identities for `rebuild/Cargo.lock`, `rebuild/Cargo.toml`, and the
  maintained fixture manifest, plus the required V11 fixture identities;
- the exact reproducible gate `argv`, technical network assertion, bounded
  authorization assertion ID, and maintained destination, purpose, target
  scope, and source scope;
- exact-final aggregate status, failure count, summary hash, and each command's
  actual `argv`, outcome, exit/termination/spawn state, and duration;
- same-gate final artifact production, provider live-qualification identity/result,
  and consumption facts for V11 preflight
  and official V11;
- official V11 status/result hash and status counts, authenticated target
  classifications, credential-audit result/counts, `phase_8_ready`, and active
  Decision revisit-trigger state;
- production provider live-qualification status/evidence hash, bounded provider/model/source
  scope, usable success/degradation outcome, and raw-material non-retention state;
- sanitized evidence archive identity, size/member count, prerequisite state,
  and independent verifier outcome.

It excludes environment-variable or home-directory dumps, usernames,
credentials, `auth.json` contents, reusable credential fingerprints, source
bodies, full command logs, raw provider payloads, and private prompt bodies. A
later documentation-only session uses the copied capsule and maintained tracked
inputs; it never needs, searches for, or substitutes an ignored final or V11
artifact from another session. Capsule-backed semantic checking resolves the
provided input and refuses capsules in any Git-ignored repository-local runtime
area, including `rebuild/.local`; the operator must pass an explicit copy from
an external handoff location.

Capsule semantic checking follows the stages actually reached. A blocked
admission or pre-final check requires its supporting check outcome and no later
evidence. A final failure requires complete exact-final evidence and no V11
evidence. A successful Final with failed mapped-test execution remains blocked,
with `contract_coverage_execution_failed` and no provider or V11 invocation. A V11-preflight failure requires the successful same-gate final and
preflight consumption but no official result. An official-V11 failure requires
the successful final, same-session ownership, actual V11 result/status, and
only the authenticated targets attempted. Full success additionally requires
all maintained targets, the credential audit, every artifact-flow fact, and a
successfully created and independently verified sanitized evidence archive.
Only that final state may set `phase_8_ready = true`.

Generic maintained reports keep the one-argument shape check. A V11 conclusion
must use the capsule-backed semantic mode so the checker compares structured
capsule values to the relevant report sections:

```text
rebuild/scripts/check-validation-report \
  --capsule /path/to/copied-capsule.json \
  rebuild/validation/end-to-end/multi-repository/report.md
```

The report records the capsule field labels and values; it may render them as
tables or prose. Statements that versions or commands were not projected do
not satisfy this mode when the capsule contains those values.

Examples:

```text
rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/check-validation-report rebuild/validation/repository-intelligence/polyglot-structural/report.md
rebuild/scripts/check-validation-report --self-test
```

The Phase 5 acceptance orchestrator maps maintained fixture and requirement
identifiers to Production Rust tests. It owns orchestration and evidence
accounting only; analyzer and product semantics remain in the Production Rust
subsystem.

The realistic Repository Intelligence qualification adds a maintained Tier 1
seven-language adversarial corpus and an optional Tier 2 public-repository
corpus. Tier 2 source is never committed: `external_corpus.py fetch` places
revision-pinned sparse checkouts under ignored `rebuild/.local/` state, while
`status` reports an absent checkout as `environment_blocked`. Run both tiers
through the repository-local focused runner:

```text
rebuild/scripts/validate focused realistic-external-fetch -- python3 rebuild/validation/repository-intelligence/realistic-qualification/external_corpus.py fetch
rebuild/scripts/validate focused realistic-corpus-qualification -- python3 rebuild/validation/repository-intelligence/realistic-qualification/assertions.py
```

The finite machine authority/rationale/owner table is `dogfood/machine-policy.json`.
Intact older evidence uses `dogfood-campaign evaluate` without changing its Product
candidate. An incomplete historical campaign without `evidence-set.json` may use
`dogfood-campaign diagnose --campaign-root <old> --output <new>` for a separate, explicitly
non-qualifying inventory diagnostic. Neither path rewrites historical rejection/raw data.

Use `validate-approval --approval <approval-run>/approval.json --qualification
<qualification-run>/qualification.json` to recheck a recorded authorization without
creating another approval. The original qualification input references must remain
available for this independent check.

Naturalistic target journey collection observes Git state without changing it.
The pinned baseline and known session/final HEADs remain repository-history-bound.
Missing raw Git metadata remains unknown. Session chronology, optional structured
status/HEAD, canonical-baseline dirty paths, observed changes, commits and net path
correlation are retained as advisory facts. Dirty target state and absent commits
never block collection or create/merge Work identity. Task/repository-owned Git policy
remains reviewable; no machine commit obligation is invented without maintained
deterministic requirement evidence. Journey-final evidence retains repository-state.json,
staged.patch and unstaged.patch with deterministic content/mode hashes and an overall fingerprint;
ignored files are outside this Git observation boundary. Publication
rechecks the final target state and rolls back on change. Historical inspection verifies retained
bytes without requiring the original workspace. This does not establish actor attribution or
verification success. The Product candidate workspace remains strictly clean and exact-HEAD-bound.

Journey-final `git-observations.json` is inventory/hash-bound and selected as a
`repository_state` reviewer surface even without a machine evaluation or raw-rollout
selection. It exposes the retained Git facts and limitations without patch/source
bodies. Its content must agree with the evidence-set observation at publication and
historical verification. The original Git workspace is not needed for later review.

Current preparation freezes the five Work mappings and eight task files.
Machine evaluation retains raw observations; post-hoc reviewers assess actual
opportunities and may record `not_observed` for optional semantic events.
The maintained campaign self-test covers task freezing, activation completeness,
session/Project/Work identity, immutable raw hashes and publication. Resume,
document realization and real-Git repository-state tests remain separate
deterministic support. A changed Product candidate requires fresh sessions
and a new qualification.


The purpose-oriented Viewer route/CLI/snapshot contract is maintained in
[its README](../crates/volicord-viewer/README.md), with fixture identities and
scoped reading limitations in [Viewer findings](end-to-end/multi-repository/viewer-reading-findings.md).
`volicord-viewer --test reading` reuses the real canonical fixture constructor from
`volicord-operations/tests/support/reading_fixture.rs`; its independent HTTP
assertions cover selection, scope, history, pagination, fragment closure and purity.
This is focused support, not browser or human comprehension qualification.

Viewer requested-section cost checks use `volicord-viewer --test reading
requested_sections_on_large_repository`. The maintained
[read budgets](end-to-end/multi-repository/viewer-read-budgets.json) bind the named
synthetic polyglot/192-module workload, debug profile, WSL2/i7-13700K environment
and individual stages. Read/decode/document invariants run unconditionally; setting
`VOLICORD_VIEWER_BUDGETS=1` explicitly checks these environment-specific timing
ceilings. Run the focused command in the [findings](end-to-end/multi-repository/viewer-reading-findings.md)
without concurrent expensive validation. First-request cold means a fresh adapter,
not flushed operating-system caches. Neither these samples nor markup/semantic
checks provide a population p95, universal SLA, browser interaction or human-review
qualification. Existing V11/resource budgets are unchanged.

### Browser support for Viewer reading (V11 owner)

The narrow `end-to-end/multi-repository/work_explanation_browser.py` runner consumes
a fresh canonical fixture after actual authorized active-host `work explain prepare/record` and `decision explain prepare/record`.
It checks ordinary answers for five Work cases and two Decision cases in English and Korean,
current provenance disclosure and mutation-free GETs; it never generates or preloads
prose. Seed, response format and reproduction commands are maintained in the
[Viewer README](../crates/volicord-viewer/README.md#reproduce-the-narrow-actual-host-reading-proof).
It uses the same explicit Chromium/Playwright and Korean-font prerequisites below.

`end-to-end/multi-repository/viewer_browser.py` is an additional supporting
check, invoked through the existing focused recorder. It is **not registered
inside the authoritative gate** and supplies no human verdict. Final browser
execution requires the exact clean committed candidate independently of the gate.

Prerequisites are Python 3, Node >=20, a preinstalled `playwright-core` package,
and Linux Chromium/Chrome for Testing with unpacked Manifest V3 extension support.
The verified path uses Playwright 1.62.1 and Chrome for Testing 151.0.7922.34.
The WSL2 environment additionally needs `libnspr4`, `libnss3` and `libasound2t64`;
`--library-path` can identify already extracted shared libraries. Chromium and
loopback socket access must be permitted. No display server is required: the
maintained path uses full headless Chromium, not the separate headless shell.
Fontconfig (`fc-list`) and a Korean-capable font are required; missing glyph coverage
is recorded as `font_prerequisite_blocked`. Install a Korean font separately or
set `FONTCONFIG_FILE` to an operator-prepared configuration with a local font
directory. The runner records Korean font paths/hashes and any explicit configuration.
The runner never installs tools, connects to providers or reads credentials.

If tools are absent, explicitly install them **before** the check, using an
operator-authorized network/bootstrap step. One driver/browser setup is:

```text
npm install --prefix rebuild/.local/viewer-browser-tools playwright-core@1.62.1
PLAYWRIGHT_BROWSERS_PATH=rebuild/.local/viewer-browser-tools/browsers node rebuild/.local/viewer-browser-tools/node_modules/playwright-core/cli.js install chromium
```

Use the installed full Chromium executable's actual absolute path; resolve
missing shared libraries separately. Existing authorized installations can be
used directly. Neither setup step is a Product runtime dependency.

```text
rebuild/scripts/validate focused viewer-browser-self-tests -- python3 rebuild/validation/end-to-end/multi-repository/viewer_browser_self_test.py
rebuild/scripts/validate focused viewer-browser -- python3 rebuild/validation/end-to-end/multi-repository/viewer_browser.py --chromium /absolute/chromium/chrome --playwright-module /absolute/node_modules/playwright-core --require-clean --enforce-read-budgets
```

Add `--library-path /absolute/extracted/usr/lib/x86_64-linux-gnu` if needed.
Timing enforcement requires the unchanged environment in `viewer-read-budgets.json`;
other hardware can run the counts/functional checks without that flag, retaining
`timing_enforced = false`. This is an explicit limitation, not named-budget success.

The runner builds the current nested-workspace CLI/Viewer, seeds real disposable
Runtime records through a Rust integration fixture, then launches **`volicord
viewer open`** and exports **`volicord viewer export`**. Expectations come from
self-authored scenario/independent invariants and source facts, rather than the
production projection helper. Canonical identity order deliberately places the
old Work beyond the first page; equal title/path/commit Works remain distinct.
It covers en/ko, 390/768/1440 CSS-pixel base viewports, Overview/Work/Decision/code
and native evidence disclosures at 100% and actual 200% tab zoom. A test-owned
extension calls `chrome.tabs.setZoom`/`getZoom`; factor, settings, CSS width and
pixel ratio are recorded. No CSS zoom or device-scale substitution is permitted.

Native Tab/Enter actions exercise views, paged Work selection, SVG fragments,
relationship endpoints and retained Source details. DOM geometry checks ordinary
page overflow separately from graph wheel scrolling and SVG label fit. Static
and browser checks require closed snapshot fragments and reject scripts, forms,
tokens, event handlers, live links and external assets. Snapshots are read as
`file:` URLs with browser networking disabled after the server exits and Runtime
is physically unavailable. Purpose absence, failed/unverified/rejected states,
review-due Decisions and stale/unavailable analysis remain supporting observations.

Controlled mutations operate on disposable copies of actual product HTML:
removed wrapping, common-prefix truncation, missing fragment, another Work's
facts with the selected identity retained, and an inserted live snapshot link.
Each must fail for its intended assertion and the restored actual candidate must
pass. Browser evaluation is test instrumentation, never shipped Product JavaScript.

The V11 recorder preserves child argv, complete streams, numeric exits/signals,
timeouts and process-group cleanup. Separate files retain actual HTTP completion
samples (including loopback transport), browser NavigationTiming/native PaintTiming
marks and keyboard-navigation input-to-FCP samples, disclosure input/two-frame samples,
and the existing 45 cold/warm Rust stage profiles/counts on the maintained cost
workload. Two animation frames are diagnostic scheduling observations, not complete
incremental-paint completion, human responsiveness or Naturalistic MCP memory evidence.
Navigation paint marks use the browser's first-paint/first-contentful-paint entries;
keyboard input-to-FCP uses browser time origins across the actual document navigation.
Thresholds and existing V11/resource budgets are unchanged; no percentile guarantee
is inferred. Cold means fresh adapter in the Rust workload, first route request in
the HTTP samples, and never flushed OS caches.

Every run gets a fresh ignored `rebuild/.local/validation/viewer-browser-*` root
with `result.json`, candidate/input/environment/executable hashes, retained tested
executables, fixture bindings, full logs, screenshots and screenshot hashes.
Reading/disclosure screenshots use `Page.captureScreenshot` with `fromSurface=false`
at the relevant section, recording scroll/viewport geometry; this avoids blank
surface captures at deep offsets with native tab zoom. Inspect screenshots directly;
DOM geometry and font presence alone do not establish visual comprehension.
Browser and driver absence, loader/launch/loopback blocking, candidate drift and
actual product assertions are distinct outcomes; no missing execution is passed.
These local artifacts may contain original synthetic/source/audit text and absolute
paths. Keep them out of portable gate summaries and reviewer packages. The existing
gate capsule/archive remain a separate evidence class and use the existing verifier.
