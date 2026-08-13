# V02 — Semantic adapter qualification

## Status

Passed at the Production Repository Intelligence subsystem boundary for
Java/Maven, TypeScript/Node, and Rust/Cargo. The three local source-semantic
adapters and their provenance, range, deterministic rebuild, same-name/overload,
unavailable/failed, and broken-build degradation contracts pass maintained
Production Rust tests. This status is bounded to source-local evidence and does
not claim compiler/LSP completeness or real-repository performance.

## Goal

Select at least three realistic local semantic ecosystems that can normalize definition, reference, type, implementation, and override-style relations with source ranges while preserving same-name targets, diagnostics, snapshot binding, and broken-build remainder.

## Accepted decisions being validated

Q2 retains the seven structural languages and requires at least three semantic ecosystems. The Repository Intelligence, Privacy, Failure, and Versioning owners require distinct semantic provenance, local-only normal operation, honest bounded degradation, and current-version snapshot output. The selected approach must not acquire canonical judgment authority.

## Input repositories and revisions

Qualification began from `85530cb1` (`feat: add polyglot structural analysis`)
and selected the adapters in `f81343da`; Production behavior is implemented by
`5ab7d529`. Final evaluation reuses the maintained Java/Maven,
TypeScript/Node, and Rust/Cargo V01 fixture directories, whose self-authored
CC0-1.0 origin and hashes are recorded in the shared manifest. Java and
TypeScript contain interface and implementation methods with the same name;
Rust contains trait and explicit trait-implementation methods with the same
name. A separate Java overload case distinguishes one- and two-argument local
targets.

## Environment and tool versions

Qualification ran on Linux x86-64 with Python 3.12.3 and the already-qualified in-process Tree-sitter 0.26.12 Production structural boundary. No `javac`, Java language server, TypeScript compiler/language server, rust-analyzer, SCIP generator, or tree-sitter executable is a runtime prerequisite for the selected adapters.

## Candidate approaches

1. LSP provides mature navigation and compiler-backed overload resolution, but each selected language needs a distributable server, workspace lifecycle, request coordination, restart/cache behavior, and child-process containment. That would trigger V10 before Production execution.
2. SCIP offers a strong language-neutral index and stable ranges, but still depends on ecosystem-specific external indexers, their build/toolchain setup, cache lifecycle, and child-process execution. Coverage and Linux packaging are uneven across the three selected ecosystems.
3. Compiler/native analyzer output offers the strongest ecosystem truth, but tool output and invocation differ substantially, incomplete builds can suppress results, and local child execution remains gated by V10.
4. The selected approach combines the qualified in-process Tree-sitter adapters with a separate source-semantic symbol index. It resolves local declarations, arity/type-signature identity, local imports/modules, explicit interface/trait implementations, matching overrides, and call references while preserving unresolved external targets and source-only limits.

Java/Maven, TypeScript/Node, and Rust/Cargo were selected because all three have typed declarations, explicit implementation constructs, realistic local package/workspace manifests, same-name interface/trait and implementation methods, mature future compiler/LSP upgrade paths, and distinct syntax that tests normalization rather than one grammar family. Python and JavaScript remain valuable later candidates but their dynamic behavior makes the current local source-only semantic guarantee materially narrower. C/C++ native semantic accuracy depends heavily on compile commands, preprocessing, templates, and an external compiler frontend.

## Commands and configuration

```text
rebuild/scripts/validate focused semantic-qualification -- python3 rebuild/validation/repository-intelligence/semantic-qualification/assertions.py
rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/check-validation-report rebuild/validation/repository-intelligence/semantic-qualification/report.md
rebuild/scripts/validate focused phase5-maintained-acceptance -- rebuild/validation/repository-intelligence/phase-5-acceptance/assertions.py
```

The disposable probe emits deterministic normalized JSON, checks distinct same-name declaration identities, validates all five required relation kinds and zero-based UTF-8 source ranges, repeats the same snapshot, and injects an unresolved dependency marker per ecosystem to assert `partial` plus usable remainder.

## Observed results

All three ecosystems produced `defines`, `references`, `type_of`, `implements`, and `overrides` relations with source ranges and `semantic_result` provenance. Same-name interface/trait and implementation methods retained separate source-bound identities. Repeated runs produced byte-identical canonical output. Each injected missing dependency produced `partial` with an explicit unresolved diagnostic while retaining the independently valid relation set.

Local manifest/module evidence was sufficient for the maintained intra-repository relations: Maven package paths, TypeScript relative ES-module paths and `tsconfig.json`, and Cargo workspace/module/explicit trait-impl structure. The selected source index does not claim downloaded dependency bodies, generated sources, macro expansion, reflection, runtime dispatch, or compiler-exact generic/trait/type-level evaluation.

Production final evaluation publishes `defines`, `references`, `resolves_to`,
`type_of`, `implements`, and `overrides` with semantic analyzer provenance and
snapshot-bound half-open zero-based UTF-8 ranges in all three ecosystems.
Missing local imports are exercised independently in Java, TypeScript, and
Rust: each result is `partial`, carries an unresolved-dependency diagnostic,
and retains structural facts plus usable semantic relations. Disabled and
injected-failed adapters publish no semantic fact for the affected language.
Same-snapshot rebuild is byte-deterministic, and grounded search/explanation
keeps structural facts, semantic results, and agent interpretations separate.

## Coverage and failures

The fixtures establish known local definitions, call references, return/type basis, explicit implementation, matching override-style relations, same-name distinction, ranges, deterministic rebuild, and broken-dependency degradation. An adapter not selected or not run must remain `unavailable` and emit no semantic result. Parse/read failure is `failed` for the affected area; recoverable syntax or unresolved dependencies are `partial` with diagnostics and usable remainder. Structural and inventory results remain independent.

## Performance and resource observations

The qualification probe completes in a few milliseconds on three small fixtures and emits a bounded JSON graph. This is setup and normalization evidence, not a real-repository throughput or memory ceiling. The selected in-process path adds no language-server startup or external index cache; Production still needs deterministic rebuild and source/dependency invalidation tests.

## Privacy and external transmission

The probe reads only local maintained fixtures. It performs no network access, starts no analyzer process, sends no source to a provider, and creates no background-provider authority. Output is disposable Derived evidence.

## Acceptance results

- Pass: three Production typed ecosystems normalize all required relation
  families with source ranges.
- Pass: same-name interface/trait and implementation targets remain distinct.
- Pass: broken/missing dependency behavior is partial with diagnostics and
  usable remainder rather than empty success in each selected ecosystem.
- Pass: setup is reproducible, offline, local, and practical for the supported Linux path using already-linked grammar dependencies.
- Pass: unavailable or failed semantic adapters preserve structural results and
  publish no affected-language semantic fact.
- Pass: the selected Production path requires no child process, so V10 is not
  triggered or claimed complete.
- Not claimed: compiler-complete overload/generic/type inference, external
  dependency bodies, runtime behavior, or real-repository accuracy.

## Known limits

The fixtures are small and self-authored. The disposable probe asserts the normalization contract and known relations, not a general compiler. Source-only resolution must degrade honestly for ambiguous overloads, wildcard/re-export complexity, generated code, Java annotation processing/reflection, TypeScript declaration merging/type-level evaluation, Rust macro expansion/cfg and implicit trait selection, and unavailable external packages.

## Recommended implementation choice

Retain the current in-process semantic common core with Java/Maven,
TypeScript/Node, and Rust/Cargo adapters on qualified structural results. Keep
deterministic source-bound symbol identity, separate semantic provenance,
unambiguous local-only resolution, unresolved diagnostics, and rebuildable
Derived State. Future compiler/LSP adapters require separate evidence and must
not become a parallel Production authority.

## Rejected alternatives and reasons

Do not select LSP, SCIP generators, or native compiler subprocesses in this phase: each would require V10 technology evidence before Production execution, multiple tool distributions, and more complex restart/stream/cache normalization. Do not treat Tree-sitter structural facts alone as semantics; the selected semantic index is a distinct pass and provenance class. Do not select Python/JavaScript merely for easy syntax matching because their dynamic semantics would weaken the current type/implementation evidence. Do not require C/C++ before compile-command and preprocessing evidence is available.

## Reusable primitive decision

`reference_only`. The probe and report are maintained validation evidence, not a Production API or parallel analyzer. No legacy process, filesystem, storage, workflow type, or Runtime Home behavior is reused. V10 classifications are not required because the selected path introduces no child process.

## Decision revisit trigger status

Not triggered. The three selected ecosystems fit the common semantic envelope without narrowing the seven-language structural contract or local-only mode.

## Follow-up work

- Exercise larger and externally dependent projects under V11 before making
  accuracy, scale, or packaging claims beyond the maintained Linux fixtures.
- Evaluate a compiler/LSP path only after its child-process and distribution
  boundary passes V10.
- Preserve the documented source-only limits until new bounded evidence supports
  a wider semantic claim.

## Artifacts

Maintained artifacts are this report, the reference-only qualification
`assertions.py`, the three V02 entries in the shared fixture manifest, the
reused V01 fixture sources, the Production Rust semantic tests, and
`rebuild/validation/repository-intelligence/phase-5-acceptance/assertions.py`.
Raw probe output and runner results remain under ignored
`rebuild/.local/validation/`.
