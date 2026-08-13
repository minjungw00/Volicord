# V01 — Production structural parser qualification

## Status

Passed for Production parser selection. This report qualifies a parser and
adapter approach; it does not mark V01 passed before the Production adapters
exist. The original V01 report remains `Partial` until a later, independent
validation evaluates the Production implementation.

## Goal

Select a maintainable local structural parser approach for Java, Python,
JavaScript, TypeScript, C, C++, and Rust before changing Production parser
behavior. The comparison covers a common parser framework,
language-native/compiler-front-end sources, and a hybrid parser-independent
adapter boundary without promoting the disposable V01 lexical prototype.

## Accepted decisions being validated

- `open-decisions.md` Q2 retains all seven structural languages and permits
  parser technology to be selected from validation evidence.
- Product charter section 11 and acceptance scenarios B, C, E, and O require
  per-language/area capability, source ranges, honest degradation, and
  analyzer-independent inventory.
- `validation-plan.md` V01 and the Production promotion gate require stable
  same-snapshot output, bounded reanalysis, explicit limits, reproducible
  dependencies, and no legacy dependency.
- Repository Intelligence, Privacy, Versioning, and Failure owners require
  snapshot-bound Derived State, local operation, independent format versions,
  and bounded failures rather than empty success.

No accepted product decision or seven-language gate is changed.

## Input repositories and revisions

The Git authority was `d5902c0b11a6fdbfde133cee0618c28c1dfdf008`
(`feat: add repository intelligence inventory`). The qualification reads the
eight maintained V01 fixtures with expected declarations from
`rebuild/validation/shared/fixture-manifest.json`: the seven single-language
fixtures and the Java/Python/TypeScript polyglot fixture. Their content hashes,
self-authored origin, and CC0-1.0 fixture license remain owned by that manifest.
The out-of-set Go fixture remains the inventory-fallback input and is not fed
to a structural parser.

## Environment and tool versions

- Linux x86-64, WSL2 kernel `6.18.33.2-microsoft-standard-WSL2`, glibc 2.39.
- Rust `1.97.1`, Cargo `1.97.1`, Python `3.12.3`, GCC/G++ `13.3.0`.
- `javac`, `node`, `tsc`, and a `tree-sitter` CLI were unavailable. The CLI is
  not a runtime requirement because the Rust grammar crates link parsers into
  the binary.
- Qualified framework: `tree-sitter 0.26.12` (declared Rust 1.77 minimum).
- Qualified grammar crates: Java 0.23.5, Python 0.25.0, JavaScript 0.25.0,
  TypeScript 0.23.2, C 0.24.2, C++ 0.23.4, and Rust 0.24.2.

The standalone validation lock resolves 31 packages including the local
qualification package. Registry metadata declares MIT for Tree-sitter and all
seven grammars. Transitive metadata declares combinations of MIT, Apache-2.0,
Unlicense, and Unicode-3.0. The highest resolved transitive Rust requirement is
1.85, matching the reconstruction workspace minimum. Grammar build scripts use
the `cc` crate, so Linux source builds require a working C toolchain; no grammar
executable or runtime service is required.

## Candidate approaches

1. **Common parser framework:** Tree-sitter with seven official grammar crates
   was executed against every gate language. It supplies concrete syntax
   trees, error recovery, UTF-8 byte ranges, incremental edit/reparse support,
   and a uniform in-process Rust API.
2. **Language-native/compiler front ends:** Python `ast` exposed a
   machine-readable tree and end ranges. GCC, G++, and `rustc` accepted their
   fixtures but the probed commands emitted no normalized declaration graph.
   Java and JavaScript/TypeScript tools were unavailable. Production child
   process execution also remains deferred by the V10 condition.
3. **Hybrid boundary:** a parser-independent common core consumes
   Tree-sitter-backed language adapters for syntax while analyzer-independent
   inventory owns manifest/workspace/build observations. Future validated
   semantic or compiler sources can enter through separate capability and
   provenance paths instead of replacing structural facts.

The hybrid boundary was selected. It uses one maintained parser integration
surface without forcing one grammar normalizer or one ecosystem evidence source
across languages.

## Commands and configuration

The maintained qualification workspace is intentionally outside the Production
Cargo workspace. Its exact dependencies and checksums are pinned in its own
`Cargo.lock`.

```text
rebuild/scripts/validate focused structural-parser-qualification -- cargo run --manifest-path rebuild/validation/repository-intelligence/production-structural-qualification/Cargo.toml --locked --offline
rebuild/scripts/validate focused structural-native-probes -- python3 rebuild/validation/repository-intelligence/production-structural-qualification/native-probes.py
rebuild/scripts/validate focused structural-parser-qualification-metrics -- /usr/bin/time -v cargo run --manifest-path rebuild/validation/repository-intelligence/production-structural-qualification/Cargo.toml --locked --offline
cargo metadata --manifest-path rebuild/validation/repository-intelligence/production-structural-qualification/Cargo.toml --locked --offline --format-version 1
rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/check-validation-report rebuild/validation/repository-intelligence/production-structural-qualification/report.md
```

The Rust probe parses files from the maintained manifest, validates known
declaration tokens and parser-owned entity ranges, requires error recovery for
the malformed JavaScript fixture, performs a TypeScript `InputEdit`, and emits
sorted deterministic JSON. It neither imports nor invokes `prototype.py`.

## Observed results

- All 70 maintained declarations across 19 source files and eight fixtures
  were found in parser-owned declaration nodes at the exact expected symbol
  row. All 70 entity byte ranges were non-empty, within the source, and carried
  zero-based UTF-8 byte coordinates from Tree-sitter.
- The malformed JavaScript fixture produced a tree with an error and still
  retained `stillVisible`; a parser error was therefore observable without an
  empty result.
- Repeating the locked offline probe produced deterministic, sorted
  declaration output. Tree-sitter supplies no durable entity IDs; the stable-ID
  implication is that the common core must derive IDs from the Analysis
  Snapshot plus normalized source path, language, kind, qualified-name basis,
  and parser range rather than process-local node IDs or traversal order.
- Replacing one TypeScript declaration with an `InputEdit` reported one changed
  range covering 25 bytes of a 486-byte file. This establishes parser-level
  bounded reparse support; repository cache reuse and dependency invalidation
  remain Production common-core responsibilities.
- Python `ast` returned four class/function declarations with start/end ranges.
  GCC, G++, and `rustc` syntax/metadata probes exited zero but returned no common
  declaration graph. Missing Java/Node/TypeScript tools kept native-only
  coverage below the seven-language gate.
- The parser probe ran fully offline after dependency resolution. No source or
  result was transmitted externally.

JavaScript and TypeScript used distinct grammars: TypeScript adds interface,
type alias, enum, signature, typed parameter, and property nodes that JavaScript
does not own. C and C++ used distinct grammars: C++ adds namespace, class,
method, qualified declarator, inheritance, alias, and template syntax beyond
C's declaration model. These differences remain adapter-owned rather than
being inferred from file extensions after parsing.

## Coverage and failures

The probe covers maintained class/interface/trait/struct/enum/type,
function/method/test, and field declarations plus exact declaration containment
and malformed-source recovery. It does not treat parse recovery as full
coverage: error and missing nodes must produce `partial` diagnostics.

Tree-sitter observes source syntax, not macro expansion, annotation processing,
generated source that is absent from the snapshot, active conditional-build
selection, TypeScript type evaluation, Python runtime mutation/reflection,
JavaScript dynamic module behavior, C/C++ template instantiation, Rust trait
resolution, or runtime dispatch. C/C++ preprocessor and Rust `cfg`/macro nodes
may be reported as syntax with explicit limitations; inactive/expanded meaning
is not fabricated. Ecosystem manifests can establish declared configuration,
but unresolved build context remains partial.

The linked grammar removes runtime parser-executable unavailability. Language
setup errors, parse cancellation, malformed output, and injected adapter errors
can be returned as bounded failed/partial outcomes. A native fault inside an
in-process generated C parser could terminate the process and cannot be caught
as a Rust error; Production must preserve prior snapshot/inventory state across
restart and must not claim per-request success after such termination. Moving
parsers to child processes is not selected before V10 validates process
technology.

## Performance and resource observations

One warm locked/offline measurement parsed and checked the 19 files in about
0.02 seconds elapsed with 32,148 KiB maximum RSS and emitted 24,815 bytes of deterministic
JSON. The validation wrapper observed 29.289 ms including process startup. The
first dependency build took about 3.4 seconds. These are single small-fixture
observations, not real-repository benchmarks or memory ceilings.

## Privacy and external transmission

All parsing, native probes, and serialization ran locally. The offline replay
made no network request, invoked no external semantic provider, and transmitted
no source. Network access was used only once to resolve crate metadata and
download the pinned public dependencies; repository fixture content was never
part of those requests.

## Acceptance results

- Pass: one maintainable approach retains all seven accepted structural
  languages behind the common adapter envelope.
- Pass: maintained declarations and source ranges are reproducibly recovered;
  malformed syntax remains visible as partial rather than empty success.
- Pass: parser-level incremental reanalysis is bounded and same-input output is
  deterministic.
- Pass: JavaScript/TypeScript and C/C++ grammar differences remain explicit.
- Pass: operation is local/offline and integrates through Rust crates on Linux.
- Pass: dependency versions, sources, checksums, declared licenses, Rust
  requirements, and C-build implication are reproducible from the lock and
  Cargo metadata.
- Pass with an explicit limit: adapter errors are isolatable; an in-process
  native parser crash is process-fatal and requires restart/recovery rather than
  a fabricated empty result.
- Not claimed: real-repository accuracy, token-exact ranges beyond the maintained
  assertions, dependency-aware invalidation, macro expansion, generated-code
  discovery, semantic resolution, or Production capability.

The evidence is sufficient to select the Production approach without reducing
the product contract.

## Known limits

- The fixtures are small and self-authored; 70/70 is fixture accuracy, not a
  population estimate for real repositories.
- The probe validates parser-owned entity ranges and exact symbol rows but does
  not assert every end column against an independent compiler oracle.
- Incremental `changed_ranges` demonstrates parser locality, not Repository
  Intelligence cache correctness or transitive dependency invalidation.
- Tree-sitter error recovery can retain misleading nodes near severe damage;
  any error-bearing file requires partial coverage and diagnostics.
- Static syntax cannot establish resolved calls, build-selected code, macro
  expansion, generated sources, dynamic dispatch, or external runtime behavior.
- In-process native parser faults are not independently crash-contained before
  V10.

## Recommended implementation choice

Use Tree-sitter 0.26.12 and the pinned seven grammar crates as the Production
syntax source. Implement seven narrow language adapters on the existing
parser-independent common core. Each adapter owns grammar-specific entity,
range, hierarchy, relation, extension, error, and unsupported normalization.
Keep manifest/workspace/build observations in the inventory/ecosystem path, and
keep future native/compiler semantic sources behind a distinct semantic
provenance boundary.

Production must derive deterministic snapshot-bound entity/relation identities,
cache by file fingerprint plus adapter/configuration basis, propagate explicit
file/dependency invalidation, isolate adapter outcomes, and expose grounded
search results. It must not copy the lexical V01 prototype or keep a second
Production parser path.

## Rejected alternatives and reasons

- Reject the V01 lexical/regex prototype: it remains fixture-tailored and cannot
  own grammar ranges, error recovery, calls, macro limits, or real syntax.
- Reject native/compiler-only structural analysis: the local probes do not
  expose a normalized declaration graph for six of seven languages, several
  tools are unavailable, toolchain/build requirements vary, and Production
  child-process execution is not yet qualified under V10.
- Reject Python-`ast`-only hybrid special casing: it adds a second coordinate,
  failure, distribution, and normalization path without evidence of a needed
  structural capability unavailable from the qualified Python grammar. Native
  Python analysis can be reconsidered for semantic/ecosystem evidence.
- Reject treating Tree-sitter syntax as semantic or build truth: the evidence
  does not qualify those claims.

## Reusable primitive decision

`reference_only`. Keep this standalone probe, lockfile, report, and maintained
fixture assertions as validation support. Do not copy its expected-declaration
matching code into Production and do not expose it as a parallel analyzer. The
Production implementation receives its own Rust responsibility and tests.

## Decision revisit trigger status

Not triggered. The common envelope and qualified parser set retain all seven
languages. The documented limits require partial/unsupported diagnostics but do
not establish that accepted Q2 is infeasible.

## Follow-up work

- Implement and directly test the seven Production normalizers, deterministic
  serialization, fingerprints, freshness/invalidation, failure isolation, and
  source-grounded search.
- Add maintained Production assertions for hierarchy, imports/includes/exports,
  confirmed inheritance/implementation, justified syntax calls, tests, and
  ecosystem observations.
- Preserve real-repository accuracy, semantic capability, child-process
  containment, and final V01 status for their owning later validation sessions.

## Artifacts

- Maintained: this report; the standalone `Cargo.toml`, `Cargo.lock`, Rust parser
  probe, and native probe beside it; the shared fixture manifest and V01
  fixtures.
- Ignored successful parser artifact:
  `rebuild/.local/validation/20260813T120107.345021Z-structural-parser-qualification-o3ae23kj`.
- Ignored native-probe artifact:
  `rebuild/.local/validation/20260813T120232.602717Z-structural-native-probes-xkj2qjwu`.
- Ignored metrics artifact:
  `rebuild/.local/validation/20260813T120144.374153Z-structural-parser-qualification-metrics-8jl9xxuo`;
  deterministic output SHA-256
  `d67256ee67df118b7e11bf3227b29ff531fa3fb943f61de7b3bff2566372fbda`.
- Qualification lock SHA-256:
  `654cfe1b83e55dafc1b04d8230f841939811aa2f4725ee5d5b35ab17a0179e3b`.
