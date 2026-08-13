# V02 — Semantic adapter qualification

## Status

Passed for selecting three Production source-semantic adapters. This qualification does not claim V02 complete before the Production adapters and their degradation contracts pass focused tests.

## Goal

Select at least three realistic local semantic ecosystems that can normalize definition, reference, type, implementation, and override-style relations with source ranges while preserving same-name targets, diagnostics, snapshot binding, and broken-build remainder.

## Accepted decisions being validated

Q2 retains the seven structural languages and requires at least three semantic ecosystems. The Repository Intelligence, Privacy, Failure, and Versioning owners require distinct semantic provenance, local-only normal operation, honest bounded degradation, and current-version snapshot output. The selected approach must not acquire canonical judgment authority.

## Input repositories and revisions

The authority was Git `85530cb1` (`feat: add polyglot structural analysis`). The probe reuses the maintained Java/Maven, TypeScript/Node, and Rust/Cargo V01 fixture directories and the shared manifest records their self-authored CC0-1.0 origin and content hashes. Java and TypeScript contain interface and implementation methods with the same name; Rust contains trait and explicit trait-implementation methods with the same name.

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
```

The disposable probe emits deterministic normalized JSON, checks distinct same-name declaration identities, validates all five required relation kinds and zero-based UTF-8 source ranges, repeats the same snapshot, and injects an unresolved dependency marker per ecosystem to assert `partial` plus usable remainder.

## Observed results

All three ecosystems produced `defines`, `references`, `type_of`, `implements`, and `overrides` relations with source ranges and `semantic_result` provenance. Same-name interface/trait and implementation methods retained separate source-bound identities. Repeated runs produced byte-identical canonical output. Each injected missing dependency produced `partial` with an explicit unresolved diagnostic while retaining the independently valid relation set.

Local manifest/module evidence was sufficient for the maintained intra-repository relations: Maven package paths, TypeScript relative ES-module paths and `tsconfig.json`, and Cargo workspace/module/explicit trait-impl structure. The selected source index does not claim downloaded dependency bodies, generated sources, macro expansion, reflection, runtime dispatch, or compiler-exact generic/trait/type-level evaluation.

## Coverage and failures

The fixtures establish known local definitions, call references, return/type basis, explicit implementation, matching override-style relations, same-name distinction, ranges, deterministic rebuild, and broken-dependency degradation. An adapter not selected or not run must remain `unavailable` and emit no semantic result. Parse/read failure is `failed` for the affected area; recoverable syntax or unresolved dependencies are `partial` with diagnostics and usable remainder. Structural and inventory results remain independent.

## Performance and resource observations

The qualification probe completes in a few milliseconds on three small fixtures and emits a bounded JSON graph. This is setup and normalization evidence, not a real-repository throughput or memory ceiling. The selected in-process path adds no language-server startup or external index cache; Production still needs deterministic rebuild and source/dependency invalidation tests.

## Privacy and external transmission

The probe reads only local maintained fixtures. It performs no network access, starts no analyzer process, sends no source to a provider, and creates no background-provider authority. Output is disposable Derived evidence.

## Acceptance results

- Pass: three realistic typed ecosystems normalize all required relation families with source ranges.
- Pass: same-name interface/trait and implementation targets remain distinct.
- Pass: broken/missing dependency evidence is partial with diagnostics and usable remainder rather than empty success.
- Pass: setup is reproducible, offline, local, and practical for the supported Linux path using already-linked grammar dependencies.
- Pass: the selected Production path requires no child process, so V10 is not triggered or claimed complete.
- Not claimed: compiler-complete overload/generic/type inference, external dependency bodies, runtime behavior, real-repository accuracy, or V02 completion before Production validation.

## Known limits

The fixtures are small and self-authored. The disposable probe asserts the normalization contract and known relations, not a general compiler. Source-only resolution must degrade honestly for ambiguous overloads, wildcard/re-export complexity, generated code, Java annotation processing/reflection, TypeScript declaration merging/type-level evaluation, Rust macro expansion/cfg and implicit trait selection, and unavailable external packages.

## Recommended implementation choice

Add one in-process Production semantic common core with Java/Maven, TypeScript/Node, and Rust/Cargo adapters on the existing Tree-sitter structural results. Derive deterministic symbol identities from language, qualified name, declared arity/type basis, locator, and snapshot; keep source ranges and semantic analyzer identity separate from structural provenance; resolve only unambiguous local targets; and preserve unresolved targets and diagnostics. Rebuild from source rather than introduce a second persistent authority.

## Rejected alternatives and reasons

Do not select LSP, SCIP generators, or native compiler subprocesses in this phase: each would require V10 technology evidence before Production execution, multiple tool distributions, and more complex restart/stream/cache normalization. Do not treat Tree-sitter structural facts alone as semantics; the selected semantic index is a distinct pass and provenance class. Do not select Python/JavaScript merely for easy syntax matching because their dynamic semantics would weaken the current type/implementation evidence. Do not require C/C++ before compile-command and preprocessing evidence is available.

## Reusable primitive decision

`reference_only`. The probe and report are maintained validation evidence, not a Production API or parallel analyzer. No legacy process, filesystem, storage, workflow type, or Runtime Home behavior is reused. V10 classifications are not required because the selected path introduces no child process.

## Decision revisit trigger status

Not triggered. The three selected ecosystems fit the common semantic envelope without narrowing the seven-language structural contract or local-only mode.

## Follow-up work

Implement the three adapters, semantic capability/degradation reports, current Analysis Snapshot writer update, deterministic rebuild/freshness, semantic search, grounded explanation basis, and read-only canonical identity linkage. Add focused unavailable/failure, broken-build, provenance, no-mutation, and architecture-contract validation.

## Artifacts

Maintained artifacts are this report, `assertions.py`, the three V02 entries in the shared fixture manifest, and the reused V01 fixture sources. Raw probe output and runner results remain under ignored `rebuild/.local/validation/`.
