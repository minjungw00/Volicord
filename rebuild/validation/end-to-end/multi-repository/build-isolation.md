# Build isolation and the unresolved scoped-check rejection

The validation owner is [validation-plan.md](../../../docs/design/validation-plan.md).
This record links changed responsibilities to their maintained execution paths;
raw sources, compiler output, attempts and runtime state remain ignored.

| Contract | Producer / consumer | Executable check |
| --- | --- | --- |
| Qualification never reuses diagnostic targets or compiler settings | `scripts/validate gate` calls `shared/build_boundary.py::prepare` before admission; every runner child checks the same input/configuration/tool basis before and after execution | `gate-entrypoint-self-test`, including `build_boundary_self_test.py` |
| A gate owns a new, initially empty target and native Cargo test runner | `build/context.json` binds source bytes, candidate, tools, configuration and owned environment; compiler and execution receipts share its hash and attempt | Real Cargo compilation/test execution in a disposable gate candidate; existing diagnostic output remains byte-identical |
| Installation selects this Cargo invocation's output | `build_binaries` reads Cargo compiler-artifact JSON; `install.sh` copies and verifies those exact paths/hashes; the actual-product realization check uses the same selector | Gate support installer control and ordinary installer with a custom target |
| Compiler/test identities remain associated | Compiler receipts retain compiler identity, argv, working directory, source entry points and output hashes; Cargo test runner requires a matching compiler receipt before execution and records its exit | Real toolchain controls; no marker-string oracle |
| Existing rehearsal/archive dependency identity includes changed producers | `rehearsal_contract.py` includes the shared build boundary; the archive inventory includes the new orchestration control | Rehearsal and evidence-archive self-tests |

Gate refusal is exit 2 with `build-refusal.json`, before admission/support/Final.
A changed input during a runner child preserves the child's numeric exit and marks
its wrapper as `build_boundary_refused`, exit 2. These are blocked executions, not
qualification passes. Compiler entry points outside the recorded candidate or Cargo registry are refused;
late child-process flag overrides and uncompiled executables are also rejected.
Rustup-selected compiler/Clippy identities are explicitly recorded and checked, and
the observer preserves Cargo jobserver descriptors. Compiler receipts publish
atomically so concurrent readers
cannot consume a partially written receipt. Interrupted executions retain a start
without manufacturing a finish.

The current workspace has no admitted Cargo/toolchain configuration files. The
gate therefore refuses ambient ancestor/workspace/Cargo-home configurations and
inherited compiler, target, runner, cache, profile or linker overrides. It preserves
them rather than deleting or silently clearing them. Cargo's download cache remains
available, while compiled outputs are new per gate and compiler cache wrappers are
refused. A future supported configuration needs an explicit reviewed input policy;
adding an unrestricted override would reopen this boundary. Ordinary focused checks
continue to support existing Cargo configuration and target selection and do not
require a full rebuild. The installer follows actual Cargo artifacts in both modes.
A qualification checkout path containing whitespace is explicitly refused because
Cargo's environment runner string splits on whitespace.

## Original evidence recovered locally

All following paths are repository-root-relative. They were inspected locally;
an archive entry or navigation index alone was not treated as an original.

- Failed original: `rebuild/.local/validation/20261024T122653.146077Z-final-r5ownfl6/04/`
  contains the exact workspace Cargo command, separate stdout/stderr and numeric
  exit **101**. Stdout lines 708–720 identify
  `check_target_fingerprints_support_non_ascii_empty_and_polyglot_files_without_claiming_execution_coverage`
  and `cannot record scoped check Checkpoint` /
  `inconsistent scoped check/run/Work/assertion basis`.
- That attempt's stderr names `rebuild/target/debug/deps/scoped_check_targets-0d67a64523511451`. Its execution-time
  hash, effective compiler environment and rejected fixture's retained state were
  not recorded there. The fixture uses a disposable `TempDir`; a current binary or
  a later successful fixture cannot supply those missing facts.
- Historical diagnostic producer:
  `rebuild/.local/visual-reading-20261024-task/diagnose-polyglot-check.py`.
  Its explicit `CARGO_TARGET_DIR` points to the shared `rebuild/target` while its
  manifest points into `polyglot-diagnostic-source/rebuild/`. Adjacent
  `polyglot-diagnostic-results/build.stdout`, `build.stderr`, `result.json` and
  numbered streams preserve compilation and 200 diagnostic executions. The
  isolated `verification.rs` adds a rejection print; the maintained source does not.
- `rebuild/.local/visual-reading-20261024-task/preserved-target-after-diagnostic/`
  and `post-diagnostic-binary-drift.json` preserve the contaminated output and
  observed executable changes. Neither the old target nor failed originals were
  modified by this work.
- The later clean-target historical gate is separately recorded at
  `rebuild/.local/validation/20261024T183915.920428Z-v11-gate-w547u3xa/`.
  Its passed result is historical. It neither diagnoses the earlier invariant nor
  qualifies the current candidate.

## Bounded investigation on the historical source basis

The frozen pre-execution plan is
`rebuild/.local/build-isolation-investigation/plan.json`; its script, archive hash,
isolated source/target, per-command argv/streams/numeric exits, selected executable
hashes and summary are adjacent. The source basis is `75d2c91e`; diagnostic changes
exist only in that ignored copy. No diagnostic target is a qualification input.

Predictions and stopping rule were recorded before running comparisons:

- Input/order: unchanged or reversed declared targets would expose a reproducible
  path/count/hash disagreement if ordering were sufficient to cause the failure.
- State/concurrency: related binding/lifecycle checks or four simultaneously run
  independent fixtures would reject while the standalone target passed if the
  exercised shared state or ordering were sufficient.
- Time: an end before start would expose the failing window predicate; actual
  wall-clock windows were observed alongside monotonic command durations.
- Build selection: the historical compiler output associates diagnostic source
  with a shared target. This predicts contamination, independently of the earlier
  rejection and without diagnosing it.

The finite plan stopped after standalone original, related serial/default-thread
suites, four concurrent independent fixtures and reversed declared-target order.
All **16** command/process rows (including two builds) exited **0**. Diagnostic
admission logging observed 1,066 validation windows, with no reversed window in
these comparisons. This is **non-reproduction**, not proof against rare clock,
environment, concurrency, original-input or other causes. No retry-until-success,
Product repair, assertion weakening or new canonical behavior was introduced.

The original risk remains a rejected legitimate scoped Checkpoint capture whose
specific inconsistent predicate/state is unavailable. No incorrect admission was
observed; missing original state prevents excluding other result-integrity risk.
The claim that this invariant defect is fixed is withheld. Unrelated visual work
need not wait for an invented diagnosis. A future occurrence should preserve the
first failed state and compiler/executable basis before any changed comparison.

## Current check evidence

- Runner and integrated gate-entrypoint checks: preserved under
  `rebuild/.local/validation/` with `build-isolation-*` labels.
- Actual contamination control fixtures: `build-isolation-controls-*/controls.json`
  under that validation directory, with their own candidate-local gate contexts,
  compiler receipts, complete build streams and preserved poison targets.
- The first control failed because Cargo's top-level binary and its `deps` output
  have different paths. Selection now associates identical output bytes with the
  compiler receipt and still requires the gate-owned target. The failed control
  remains preserved as `20261024T215615.797191Z-build-contamination-controls-mn9v1l1h`.
- Intermediate compiler-enforcement controls also failed (outer exit 1) while
  the boundary initially rejected Rustup's automatically selected toolchain.
  `build-isolation-compiler-enforcement` / `build-isolation-compiler-environment`
  retain those attempts. Another control exposed dropped Cargo jobserver file
  descriptors and a foreign-fixture workspace setup error. These implementation
  and control defects were corrected before adoption; they do not explain the
  historical Product invariant. The final real-entrypoint control adds Clippy
  with `-D warnings`, late flags/foreign-source rejection (Cargo exit 101) and
  rejection of an uncompiled executable (observer exit 2).
- Normal maintained Product checks use `build-isolation-normal-product` and a
  distinct `rebuild/.local/build-isolation-normal-target`; this is scoped support,
  not Final/V11 or provider qualification.

The first post-commit actual-Product compiler smoke at `b6f2f154` failed:
`rebuild/.local/validation/20261024T221222.008117Z-observed-product-clippy-_yxlmquz/`
preserves Cargo exit 101; the following scoped tests were not run in that smoke.
Cargo carries its Clippy workspace-wrapper environment into dependency invocations
that use rustc directly. The initial observer rejected this legitimate combination.
The follow-up fix permits only the already recorded compiler/driver identities and
adds an excluded local dependency with a build-script compiler probe to the
real-entrypoint regression fixture. Build-script-injected `RUSTC`/`RUSTDOC` are
also accepted only when they select the recorded toolchain binaries. A subsequent
actual integration-target compile exposed Cargo's injected `CARGO_TARGET_TMPDIR`;
it is permitted only at this attempt's owned `target/tmp`, with an integration
target added to the same real-entrypoint regression. This
is a build-boundary defect discovered by the new direct check, separate from the
historical invariant. Existing failed evidence and the first commit remain intact.

No real authoritative aggregate, official V11 or provider qualification is run by
this task. Synthetic gate controls deliberately end at support exit 37, preserving
Final and external stages as not run. Technical isolation, the unresolved invariant
and actual human visual acceptance remain separate conclusions.
