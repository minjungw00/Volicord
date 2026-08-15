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
  credential-retention audit, and prints the sanitized handoff capsule.
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

The V11 multi-repository harness is under
`end-to-end/multi-repository/`. It performs a non-fail-fast rehearsal against
the installed CLI and MCP surfaces, writes child-operation evidence and its
structured result only below ignored `rebuild/.local/`, and classifies missing
public product paths as `unsupported` instead of substituting validation-only
domain behavior.

The current maintained conclusion is recorded in
`end-to-end/multi-repository/report.md` and summarized in
`phase-7-summary.md`. Exact final sealed production/test candidate
`80dd08e8828d7159ac7b8839178ccdd9f9013851`; the same-session official V11
passed all 54 required steps, all three authenticated Codex targets, and the
credential-retention audit. Official V11 assessed the accepted Decision
register and reported no active revisit trigger or blocker, so
`phase_8_ready = true`. Phase 8 may begin, but is not itself certified by that
V11 result. The later documentation conclusion records the result and is not
part of the exact-final candidate.

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
stdout so it can be copied before that directory disappears.

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
  Decision revisit-trigger state.

It excludes environment-variable or home-directory dumps, usernames,
credentials, `auth.json` contents, reusable credential fingerprints, source
bodies, full command logs, raw provider payloads, and private prompt bodies. A
later documentation-only session uses the copied capsule and maintained tracked
inputs; it never needs, searches for, or substitutes an ignored final or V11
artifact from another session.

Capsule semantic checking follows the stages actually reached. A blocked
admission or pre-final check requires its supporting check outcome and no later
evidence. A final failure requires complete exact-final evidence and no V11
evidence. A V11-preflight failure requires the successful same-gate final and
preflight consumption but no official result. An official-V11 failure requires
the successful final, same-session ownership, actual V11 result/status, and
only the authenticated targets attempted. Full success additionally requires
all maintained targets, the credential audit, and every artifact-flow fact.
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
