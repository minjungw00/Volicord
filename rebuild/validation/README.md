# Reconstruction validation assets

This directory contains maintained inputs, assertions, report summaries, and
small disposable experiment implementations. It is an internal validation
surface, not a Volicord product command or production architecture.

## Commands

- `rebuild/scripts/validate self-test` checks command execution, output and
  status preservation, signal reporting, and non-fail-fast aggregation with
  fake commands.
- `rebuild/scripts/validate gate-self-test` checks admission blocking,
  authorization separation, exact-once synthetic final/V11 orchestration,
  same-session artifact selection, credential-safe capsule projection, and
  no-retry behavior. It never invokes the real exact final or official V11.
- `rebuild/scripts/validate gate-entrypoint-self-test` copies the maintained
  admission/gate entry point into isolated temporary Git repositories and
  checks clean, initially dirty, and admission-generated dirty candidates
  without invoking the real exact final or official V11.
- `rebuild/scripts/validate evidence-archive-self-test` builds a synthetic
  gate/V11 artifact tree and exercises result discovery, logical repository and
  V11 target cwd projection, argv sanitization, archive creation, and independent
  verification. It also rejects unknown external cwd values, retained prompts or
  absolute host paths, tampered content, missing members, changed POSIX modes,
  candidate mismatch, repository source bodies, and credential-like prohibited
  content. It invokes neither exact final nor official V11.
- `rebuild/scripts/verify-validation-archive <archive> [--expected-candidate
  <HEAD>]` independently verifies archive membership, content hashes, bounded
  size, candidate agreement, tracked/executable identities, tar modes, and the
  prohibited-content boundary without extracting the archive.
- `rebuild/scripts/check-validation-report --self-test` proves generic report
  compatibility plus positive and negative capsule-backed semantics for
  admission-blocked, final-failed, V11-preflight-failed,
  official-V11-failed, and fully-passed lifecycles. It also rejects impossible
  stage combinations and missing success evidence; neither exact final nor
  official V11 is invoked.
- `rebuild/scripts/validate focused <label> -- <command> [arguments...]` runs
  the exact argument vector from the repository root and records the command,
  working directory, timestamps, duration, complete separate stdout/stderr,
  exit code, and termination details.
- `rebuild/scripts/validate admission` evaluates the current clean candidate,
  runner/V11 self-checks, fixture integrity, executables, writable disposable
  homes, the maintained resource estimate, loopback, Codex authentication,
  technical external-network state, and the exact bounded transmission
  authorization. It prints a structured result and retains `admission.json`
  below ignored validation state; a blocked result runs neither final nor V11.
- `rebuild/scripts/validate gate` repeats admission in its own session, invokes
  an immediate live clean-worktree/HEAD recheck, invokes the existing ordered
  four-command final owner exactly once, passes only the returned summary to
  V11 preflight, invokes official V11 at most once, runs the V11
  credential-retention audit, creates and independently verifies the sanitized
  evidence archive, and only then prints a fully ready sanitized handoff capsule.
  Direct `rebuild/scripts/validate final` invocation is refused so an exact
  aggregate cannot bypass admission or become detached from V11.
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
  or Viewer product-entry-point tests. Its `--self-test` mode rejects
  internal-primitive-only coverage, missing entrypoints, canonical-only or
  over-broad forgetting, ignored cleanup/repair failure, Candidate
  error-to-empty conversion, and configured-provider failure reported as
  commercial semantic-provider success.

The architecture checker is deterministic internal test support. It does not
define domain meaning, judge conceptual correctness, choose implementation
technology, or treat validation IDs as product-format versions.

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

## Scripted conformance and naturalistic dogfood

V11 is the maintained scripted conformance boundary. Its deterministic journey
proves the installed product path and remains the reusable Phase 8 regression;
it is not evidence that an agent independently discovered and used the accepted
experience in a real repository session.

Phase 8 real sessions are naturalistic behavioral dogfood. Each cycle
descriptor carries the exact plain work user task, the exact plain fresh-resume
user task, repository/cycle/revision identity, hidden evaluation material, and
the bounded capture and canonical-bundle references used for qualification.
The user tasks state a real repository outcome and ordinary safety or scope
constraints. They do not prescribe the material Question, alternatives,
recommendation, expected choice, Volicord operation order, Checkpoint contents,
a path reserved for the next session, or an instruction to perform Recall.

When a latent material Decision needs a stable review basis, its user-owned
dimension, established repository facts, reason inspection cannot decide the
choice, viable alternatives, recommendation, and material consequence live in
the hidden decision oracle. The oracle is evaluator input only. The observed
Question is still compared with those facts and the user-owned dimension, and
that comparison does not automatically pass the subjective Question-relevance
observation.

Work-session research, Inquiry, current-host Decision provenance, ordinary
work, numeric-exit verification, and Checkpoint creation are observed from the
actual Codex rollout and canonical bundle rather than disclosed as prompt
choreography. The first work turn is bound directly to the canonical Goal
Source and identity. The Checkpoint references that Goal identity and supplies
the next meaningful state or step; post-Recall continuation is judged from the
observed repository change and separate validation, not from a predeclared
reserved path.

The fresh-resume task is likewise ordinary user language and does not mention
Recall or contain a Project ID. Qualification requires successful
`project_resolve` from the current repository binding to the canonical bundle's
same Project before Recall. Recall must then precede repository inspection or
continued work, and the resume session must not initialize a replacement
Project merely to obtain an identity. Context recovery is derived from that
Recall result. Deterministic journey success remains independent from manual
Question relevance, Decision comprehension, interruption cost, and document
fidelity/usefulness observations.

The hidden decision oracle includes one bounded
`work_task_materiality_basis`: exact ordinary user-task text that makes the
user-owned dimension material to the work outcome. After case-folding and
whitespace-collapse normalization, that basis must occur in `work_user_task` itself; an
occurrence only in `fresh_resume_user_task` is invalid. The basis must not
disclose hidden alternatives, the recommendation, or the expected choice.

The work capture must show Candidate submission, source-grounded repository
research, and reviewed material promotion through `candidate_manage` before
the resulting Question can appear in `inquiry_frontier`. Only an explicit
current-host user response can qualify `decision_record`; the agent's own
recommendation or implementation preference cannot.

Full replacement qualification remains the `run` path: three repository
classes, two cycles per class, and two globally distinct fresh VS Code Codex
sessions per cycle, for twelve distinct real sessions, plus every required
automatic, manual, resource, and accessibility check. Only this complete path
may set campaign completion, replacement passage, or Phase 9 readiness true.

A completed work session with a machine-observable terminal work blocker may
be classified without executing later qualifying sessions:

```text
python3 rebuild/validation/dogfood/harness.py qualify-work-blocker \
  --candidate-head <current-candidate-head> \
  --descriptor <one-cycle-descriptor.json> \
  --work-capture <completed-work-rollout.jsonl> \
  --output <blocker-result.json>
```

The failure-only result kind is `phase8_dogfood_blocker_result`. Missing
required high-level Project, Goal Context, repository baseline, material
Question Candidate/promotion, explicit current-host Decision, or grounded
Checkpoint operations are terminal when absent from a completed work capture;
a later resume cannot retroactively put them in that session. If the capture
cannot prove a required semantic fact, the harness requires normal full
qualification instead of inventing a blocker. A positive work session cannot
be converted into an early failure. The result always records
`campaign_complete = false`, `replacement_pass_candidate = false`, and
`phase_9_ready = false`, identifies later sessions/checks as `not_run`, and
retains only bounded identities, failed checks, and the capture hash—not task
text, the hidden oracle, source bodies, credentials, or raw provider content.

This distinction does not change admission, exact final, official V11, gate
ownership, or the capsule lifecycle described below.

The current maintained pre-Dogfood entry state is summarized in
`phase-8-summary.md`: `replacement_gate = pending` and
`phase_9_ready = false`. Exact final and same-session official V11 passed for
production/test candidate `4b1c87e31caec9ef88865467610c9ddc8a20c14e`:
admission was `eligible`, exact final succeeded with zero failures, all 54
required V11 steps passed, the credential-retention audit passed with zero
recorded findings or scan errors, no active accepted-Decision revisit trigger
was reported, the sanitized evidence archive was independently verified, and
`phase_8_ready = true`. This candidate is eligible for a
fresh naturalistic Dogfood campaign, but its Dogfood state is `not_run`.

Predecessor Dogfood descriptors, captures, Runtime Homes, workspaces, bundles,
observations, and session identities remain non-reusable for this candidate.
Replacement passage remains pending/false, and Phase 9 may not begin. The
documentation-only conclusion is outside the sealed production/test candidate.

## Admission, authorization, and handoff

Technical network availability and authorization are separate current-
invocation inputs. `--external-network available` asserts only that the
execution environment can reach the service. It does not authorize a
transmission. Missing or escalation-dependent technical access is an
`environment_blocked` admission result.

The only accepted authorization assertion is:

```text
--authorize-external-transmission v11-openai-codex-project-health-three-targets
```

It covers the maintained journey's three authenticated Codex turns for the
`volicord`, `small-python`, and `polyglot-medium` targets. Their destination is
the OpenAI Codex service used by the installed Codex CLI; their purpose is to
select the installed `project_health` MCP tool; their intended source scope is
the bounded V11 prompt, Project identity, and tool result, not repository
source bodies. Credentials, generic network access, sandbox escalation,
Project provider opt-in, an earlier report, or an earlier session cannot
supply this assertion. Admission records only whether the exact assertion was
present, not operator authorization prose or credential contents.

Preflight-only example:

```text
rebuild/scripts/validate admission \
  --external-network available \
  --authorize-external-transmission v11-openai-codex-project-health-three-targets
```

Exact gate example (reserved for the one authorized final/V11 session):

```text
rebuild/scripts/validate gate \
  --external-network available \
  --authorize-external-transmission v11-openai-codex-project-health-three-targets
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

The portable archive is a bounded structured projection, not a replacement for
the gate's ignored execution truth. Complete stdout/stderr, detailed V11 local
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
or credential-like values. Tar member modes are preserved and checked; the
archive file itself is created mode `0600` in ignored local state.

The versionless current capsule has `kind = validation_handoff_capsule`. It is
one stage-dependent contract rather than separate success and failure schemas.
Its bounded cross-session evidence is:

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
- same-gate final artifact production and consumption facts for V11 preflight
  and official V11;
- official V11 status/result hash and status counts, authenticated target
  classifications, credential-audit result/counts, `phase_8_ready`, and active
  Decision revisit-trigger state;
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
evidence. A V11-preflight failure requires the successful same-gate final and
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
