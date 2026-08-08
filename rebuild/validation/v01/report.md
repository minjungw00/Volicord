# V01 — Polyglot structural analysis

## Status

Partial. The experiment supports a common normalized model and a hybrid
language-adapter boundary across all seven gate languages, but it does not
select or qualify production parser libraries. The disposable lexical parser
is fixture-sufficient evidence, not production structural capability.

## Goal

Test whether Java, Python, JavaScript, TypeScript, C, C++, and Rust structural
facts can share one representation without erasing language-specific limits,
and gather enough evidence to choose the model and adapter boundary before
production architecture work.

## Accepted decisions being validated

- `open-decisions.md` Q2: all seven first-gate languages retain structural
  capability, while every text repository retains inventory capability.
- Product charter section 11 and acceptance scenarios B, C, E, and O:
  capability is per language and area; parser facts, inventory, unsupported,
  partial, failed, and interpreted information remain distinct.
- `validation-plan.md` V01: stable identity, deterministic serialization,
  partial parsing, failure isolation, and changed-file reuse are required.

No accepted product decision is changed by this report.

## Input repositories and revisions

Repository baseline was `85d3f1c3c5690b5ac11a67f082bf4035bbcf3fc0` plus
the maintained experiment inputs in this commit. The fixture manifest is the
content authority:

- Java: `5b3579bc2a073420d519528b89d39de143b1253ea843504ca1ada6d7ed36246f`
- Python: `7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867`
- JavaScript: `8dd3d47acf0f5ddef3d21d33cc7c869d36db8d59983a3a3f6df036b1b62996a4`
- TypeScript: `ddcb5f043dde8b5e406c5d2092771510c2f16071343e6f0a072a99be2f6abc18`
- C: `f8fa2b884bcd665bb592144d0023454fd276e310554946c9b5daa42e912db044`
- C++: `6ec33083178041df947674b64d01602dd9e3ffbcc2cbc8726a9fc397a82a3ea4`
- Rust: `8161bab1034f30b88012a4dcc4ffba5ccfb43c6023561c4d30f51afa6f2fb899`
- Java/Python/TypeScript polyglot:
  `21d9b78a1ff12838932aeabeea2b1c12c1a901cf9db238e4dd25584bcff0baff`
- Out-of-set Go inventory fallback:
  `addbe4831b61f30e56149051ab441347d0cbb7604ee2c914bb0b4ddaf12e3a41`

All fixture content is self-authored, deterministic, and declared CC0-1.0 in
`fixture-manifest.json`.

## Environment and tool versions

- Linux x86-64, WSL2 kernel `6.18.33.2-microsoft-standard-WSL2`.
- Python `3.12.3`; native `ast` was available and executed.
- GCC and G++ `13.3.0`; syntax-only fixture probes exited 0.
- Rust `1.97.1`; a metadata-only fixture compilation exited 0.
- `javac`, `node`, `tsc`, and the `tree-sitter` CLI were not on `PATH`.
  No structural measurement is claimed for those unavailable executables.
- No dependency download or external parser installation was attempted.

## Candidate approaches

1. A common lexical parser framework with per-language normalizers executed for
   all seven languages. It deliberately exposes extensions and unsupported
   constructs but is only a disposable stand-in for a real incremental parser.
2. Language-native/compiler/direct sources were used where they provided
   observable local evidence. Python `ast` supplied machine-readable entities
   and ranges. GCC, G++, and rustc supplied syntax diagnostics only; their
   invoked commands did not expose a normalized declaration graph. Java and
   JavaScript/TypeScript executables were unavailable for comparison.
3. The hybrid run used Python `ast` behind the same adapter contract and the
   common lexical adapters for the other six languages.

## Commands and configuration

The maintained focused commands were:

```text
rebuild/scripts/validate focused v01-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/fixture-manifest.json
rebuild/scripts/validate focused v01-assertions -- rebuild/validation/v01/assertions.py
rebuild/scripts/validate focused v01-candidate-probes -- rebuild/validation/v01/prototype.py probe-candidates --output rebuild/.local/v01/candidate-probes.json
rebuild/scripts/validate focused v01-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/v01/report.md
```

The assertion program runs common and hybrid analysis twice, injects a
JavaScript analyzer failure, copies the fixtures to ignored local state, changes
one Python file, and reruns with a per-file cache. Serialization uses sorted JSON
and stable IDs derived from fixture, path, kind, and qualified name.

## Observed results

- Common and hybrid runs each matched all 70 expected declarations: fixture
  precision `1.0` and recall `1.0` for every fixture.
- All 70 expected declaration start ranges contained the named declaration.
- All 32 maintained hierarchy/import/include/export/inheritance/
  implementation/call/test expectations were present. The result also contains
  `contains`, `declares`, and `configures` relations.
- Repeated common outputs were byte-identical, as were repeated hybrid outputs.
- The deliberately unbalanced JavaScript file retained `stillVisible` and made
  JavaScript structural capability `partial`, rather than producing an empty
  success.
- Injecting failure for every JavaScript analyzer input recorded failed files
  while all eight unaffected fixture results remained byte-equivalent to the
  baseline.
- After changing one Python file, one file was analyzed and 34 were reused.
  Unaffected fixture graphs and unaffected Python entity IDs were unchanged.
- The Go fixture reported inventory `available` and structural `unavailable`.
- No agent interpretation was emitted by the prototype.

These precision and recall values describe only the small maintained fixtures;
they are not estimates for real repositories.

## Coverage and failures

The normalized output exercised repository, package, module, namespace, file,
class, interface, trait, struct, enum, type, function, method, field, test,
configuration, and document entities. It exercised contains, declares,
imports, includes, exports, inherits, implements, syntax-level calls, tests,
and configures relations.

JavaScript is intentionally partial because of malformed source. Macro
expansion, generated sources, conditional compilation, template instantiation,
runtime dispatch, reflection, TypeScript type evaluation, Rust trait
resolution, and cross-process polyglot flow are recorded as unsupported rather
than inferred. The injected analyzer failure is preserved separately from
unsupported and partial states.

The common lexical prototype can over-report syntax-level calls in constructs
that resemble calls and declarations; call precision was not measured beyond
the 32 expected relation checks. No report claim treats its output as a
production parser guarantee.

## Performance and resource observations

One focused assertion run observed:

- hybrid: 35 files, 147 entities, 222 relations, `11.204 ms`, `174,820`
  serialized bytes, process peak RSS `22,228 KiB`;
- common: 35 files, 147 entities, 215 relations, `8.479 ms`, `172,054`
  serialized bytes, process peak RSS `22,740 KiB`;
- injected failure: 31 analyzed files, `8.198 ms`, `164,213` bytes;
- incremental changed-file run: 1 analyzed and 34 reused files, `5.530 ms`,
  `175,770` bytes.

These are single warm-process fixture observations, not benchmarks. Python's
`ru_maxrss` is a process high-water mark, so per-run deltas are order-sensitive;
only the observed process peaks are retained.

## Privacy and external transmission

The experiment used self-authored local fixtures and Python standard-library
code. It made no network request, invoked no external semantic provider, and
transmitted no source. Compiler probes were local child processes with complete
outputs kept under ignored local validation state.

## Acceptance results

- Pass: all seven gate languages yielded the maintained declarations and
  ranges through the common adapter contract.
- Pass: inventory, parser-confirmed, unsupported, partial, failed, and empty
  interpretation classes are distinguishable.
- Pass: entity identity and serialization were stable for the same snapshot.
- Pass: package/module/file hierarchy and all required relation kinds were
  representable without filling language-inapplicable concepts.
- Pass: every single-language gate fixture detected a test.
- Pass: malformed-source partial parsing retained a declaration.
- Pass: one analyzer failure preserved all unaffected fixture results.
- Pass: the changed-file rerun avoided all 34 unaffected files.
- Pass with measurement limits: elapsed time, output bytes, and process peak
  memory were observed on the fixture set.
- Partial: production-grade parser accuracy, macro/generated-code behavior,
  and direct-parser output normalization were not established with the tools
  available in this environment.

## Known limits

- Fixtures are intentionally small and the disposable parser is tailored to
  syntax forms they contain.
- Declaration precision/recall does not cover arbitrary real-world syntax.
- Syntax-level call extraction is lexical and neither resolves targets nor
  proves runtime behavior.
- Source range checks validate maintained declaration starts and containment,
  not token-perfect end columns for every grammar.
- The cache demonstrates unaffected-file reuse, not transitive dependency
  invalidation.
- No macro expansion, build graph, generated source, semantic relation, or
  external package resolution is provided.
- Missing Java and JavaScript/TypeScript tools prevented local native/direct
  executable comparisons; unavailable evidence was not replaced with a claim.

## Recommended implementation choice

Use a small common structural envelope with stable source-bound entities,
typed relations, per-file snapshot identity, fact provenance, capability state,
unsupported/failure details, and language-specific extension properties. Put a
strict adapter boundary between that envelope and each parser or compiler
source. Adapters own grammar-specific normalization and may choose a common
incremental parser, a native source, or both per language.

This is a hybrid boundary recommendation, not a parser-library decision. Keep
inventory independent of structural adapters so missing or failed analyzers do
not disable repository registration or other languages. Keep compiler/build
semantic information outside parser-confirmed syntax facts for V02.

## Rejected alternatives and reasons

- Reject one universal lexical/regex parser as production architecture: the
  fixture prototype cannot establish macro, malformed grammar, call precision,
  or real-repository coverage.
- Reject native-only architecture as the common boundary: only Python exposed
  machine-readable native structure in this environment, while installed C,
  C++, and Rust compiler commands supplied diagnostics but no shared graph.
- Reject one language-neutral graph with no extensions or capability states:
  the fixtures require TypeScript types, Rust traits/cfg limits, C/C++ macro
  limits, Python dynamic limits, and partial/failed states to remain visible.
- Do not reject or select Tree-sitter or another common incremental parser from
  this run: its executable/library was unavailable, so no measurement exists.

## Reusable primitive decision

`reference_only`. Preserve the fixture manifest shape, normalized vocabulary,
capability/failure distinctions, stable-ID inputs, and assertion scenarios as
evidence. Do not promote `prototype.py` or its regex normalizers into a product
crate. Any production parser integration must receive a new responsibility,
dependencies, real-repository fixtures, and tests under the production
promotion gate.

## Decision revisit trigger status

Not triggered. The shared envelope represented every accepted gate language
without reducing the language set. The partial status concerns production
implementation evidence, not evidence that Q2 is infeasible. No new product
question is opened.

## Follow-up work

- Evaluate production incremental parser libraries and language grammars with
  dependency/license review and broader real-repository fixtures.
- Add call-relation precision expectations and token-exact end-range checks.
- Define dependency-aware invalidation after the production snapshot model is
  selected.
- Run V02 separately for semantic sources and at least three ecosystems.
- Do not extend this experiment into V03, V05, or production Repository
  Intelligence code.

## Artifacts

Maintained inputs are under `rebuild/validation/fixtures/v01/`, the manifest,
`prototype.py`, `assertions.py`, and this report. Raw artifacts remain ignored:

- `rebuild/.local/v01/assertions-rgvpwjfl/summary.json`, SHA-256
  `2d9e81519ff2beda27c5119e34dcd7360d06d771f26ca2456f1534e2bcf77810`;
- hybrid graph, SHA-256
  `146bb9ea54093daefd618c67368ea82718e634296d403b599cee19580783e6d9`;
- common graph, SHA-256
  `ff3461e2130e1b5ec0b3cf63e78f313634e1123aace56f83b0ec10267c9d6351`;
- injected-failure graph, SHA-256
  `3540c95bb4e527ceaaf59a0c08a18fbe459aa6d3bfe275fdfc9f441696e1ae0e`;
- `rebuild/.local/v01/candidate-probes.json`, SHA-256
  `b5c7222f6578d1822827ad88ad37e3e24abe521b8d03d3bf4223c10835e7b45c`.

Focused runner logs are under:

- `rebuild/.local/validation/20260808T201503.799086Z-v01-fixture-manifest-qycd1baj`;
- `rebuild/.local/validation/20260808T201739.326016Z-v01-assertions-final-odjyjedx`;
- `rebuild/.local/validation/20260808T201510.086844Z-v01-candidate-probes-89swnwtv`.
