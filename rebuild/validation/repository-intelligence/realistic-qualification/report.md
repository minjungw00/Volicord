# V01 — Realistic Repository Intelligence qualification

## Status

Passed for the maintained seven-language Tier 1 corpus and for the three
revision-pinned Tier 2 public repositories executed on 2026-08-23. This result
is bounded to the declared source scopes, current Production adapters, and the
curated semantic queries. It does not claim compiler completeness or transfer a
successful language/area result to the whole repository.

## Goal

Reveal false structural or semantic success, unstable entity/range behavior,
failure coupling, and unsupported cross-language generalization using realistic,
reproducible repositories rather than declaration-only samples.

## Accepted decisions being validated

Q2 requires structural capability for Java, Python, JavaScript, TypeScript, C,
C++, and Rust, at least three independently grounded semantic ecosystems, and
per-language/per-area capability honesty. Acceptance scenarios C, D, E, and O
require stable source-bound results, truthful partial/unresolved reporting,
cross-component grounding, and usable remainder after one adapter failure. The
Repository Intelligence and Failure/Recovery owners prohibit empty success and
repository-wide completeness inference.

## Input repositories and revisions

Tier 1 is the self-authored CC0-1.0 fixture
`repository-intelligence-realistic-v1`, catalogued in
`rebuild/validation/shared/fixture-manifest.json` with content SHA-256
`df249af327700395d99953374bc9837b7435eb94a562dd6f5fd231ff25b47d36`.
It contains seven multi-file single-language repositories and one Java/Python/
TypeScript repository whose HTTP and file/schema boundary is declared by
`system.json` and `greeting.request.v1.schema.json`.

Tier 2 used sparse ignored checkouts with these bounded identities:

- pallets/click, `https://github.com/pallets/click.git`, revision
  `2c8cd3ac958a7eb316d67f2d316c27086c4c0369`, BSD-3-Clause, bounded to
  `src/click`, `tests`, `pyproject.toml`, and `LICENSE.txt`;
- BurntSushi/ripgrep, `https://github.com/BurntSushi/ripgrep.git`, revision
  `3fce3b5bb0236da2df6d99672afb8a719642eca7`, Unlicense OR MIT, bounded to
  `crates`, `tests`, `Cargo.toml`, `UNLICENSE`, and `LICENSE-MIT`;
- tree-sitter/tree-sitter, `https://github.com/tree-sitter/tree-sitter.git`,
  revision `74b7d0c951ebdab16a8a4d64e7cf81e56046408a`, MIT, bounded to `lib`,
  `cli`, `test`, `Cargo.toml`, `package.json`, and `LICENSE`.

No third-party source body is committed. Checkout and input-identity data live
under ignored `rebuild/.local/repository-intelligence/external-corpus/`.

## Environment and tool versions

The qualification ran on Linux x86-64 with the workspace Rust toolchain,
Cargo, Git, Python 3, Tree-sitter 0.26.12, and the seven grammar versions locked
by `rebuild/Cargo.lock`. It used the Production in-process structural adapters
and Java/Maven, TypeScript/Node, and Rust/Cargo source-semantic adapters. No
compiler, LSP, SCIP indexer, or background provider was required.

## Candidate approaches

The selected two-tier approach keeps adversarial self-authored repositories in
Git for deterministic regression and keeps real third-party sources in ignored,
revision-verified sparse checkouts. A corpus containing copied third-party source
bodies was rejected because it would duplicate upstream code and license surface.
Tiny declaration-only fixtures remain useful for narrow adapter conformance but
are insufficient generalization evidence.

## Commands and configuration

```text
rebuild/scripts/validate focused realistic-external-fetch -- python3 rebuild/validation/repository-intelligence/realistic-qualification/external_corpus.py fetch
rebuild/scripts/validate focused realistic-corpus-qualification -- python3 rebuild/validation/repository-intelligence/realistic-qualification/assertions.py
rebuild/scripts/validate focused realistic-fixture-manifest -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/validate focused realistic-report-shape -- rebuild/scripts/check-validation-report rebuild/validation/repository-intelligence/realistic-qualification/report.md
rebuild/scripts/validate focused phase5-maintained-acceptance -- rebuild/validation/repository-intelligence/phase-5-acceptance/assertions.py
```

`assertions.py` runs the Production Rust realistic test target, the injected
single-adapter failure case on the realistic polyglot fixture, and the external
qualification only when every pinned checkout passes origin, revision, license,
and bounded-input inspection. Missing checkouts produce `environment_blocked`.

## Observed results

All seven Tier 1 repositories produced multi-file structural facts with valid
half-open zero-based UTF-8 ranges and a `partial` language report caused by an
inspectable damaged source. Same-name declarations in separate paths/scopes kept
distinct identities. C/C++ preprocessor cases and Rust macro/cfg cases retained
construct-limit diagnostics and usable direct-source syntax. Dynamic imports,
decorators, declaration merging, templates, generated/macro meaning, and runtime
behavior were not upgraded into resolved semantic facts.

Repeated same-snapshot TypeScript analysis was byte deterministic. After a
source change, incremental analysis reused unaffected files, reparsed the changed
dependency slice, preserved the unaffected entity range, and produced the same
normalized entity/relation projection as a full analysis of the changed source
snapshot. Injected TypeScript adapter failure preserved Java and Python facts and
inventory, and the retry carried `prior_failure` invalidation.

Curated locally resolvable `references` and `implements` queries scored precision
1.0 and recall 1.0 independently for Java/Maven, TypeScript/Node, and Rust/Cargo.
Unresolved results with uncertainty remained present outside the scored subset.
The polyglot fixture grounded its component paths and shared contract identifier
in source, schema, and configuration; its HTTP endpoint was grounded in Java and
TypeScript source. No repository-wide structural capability report was created.

All three Tier 2 repositories passed origin/revision/license checks and Production
structural analysis for their expected language areas. The tree-sitter checkout
retained separate C, JavaScript, and Rust reports rather than a single polyglot
completeness result.

## Coverage and failures

Covered: all seven official structural languages; nested packages/modules/
namespaces; cross-file imports/includes/exports; types, classes, interfaces,
traits, functions, methods, tests; overloads; wildcard/re-export syntax;
decorators/dynamic imports; TypeScript declarations; C preprocessing; C++
templates/namespaces; Rust macro/cfg; syntax recovery; incremental refresh;
bounded analyzer failure; three semantic ecosystems; and a source-grounded
polyglot boundary.

The corpus exposed one Production defect: C++ namespace free functions were
reported as methods. It reproduced before and is protected after the focused
fix in commit `c4c1afc8`. No other Production false-success, range, identity,
degradation, or relation defect was required for this qualification.

External checkout absence is `environment_blocked`; origin/revision/license
mismatch is `failed`. Neither state is counted as a pass. This execution had no
blocked or failed Tier 2 repository.

## Performance and resource observations

The retained focused realistic qualification completed in about 5.6 seconds;
the external-only Production analysis portion completed in about 4.7 seconds on
the bounded sparse checkouts. These observations show practical bounded execution
for the selected inputs, not a general repository-size or peak-memory ceiling.

## Privacy and external transmission

Tier 1 reads only maintained local fixtures. Tier 2 downloads public source from
the three declared Git origins into ignored state. Production analysis is local
and in-process; no repository source is sent to an LLM or background semantic
provider. Raw third-party bodies and analyzer output are not placed in maintained
reports or portable context.

## Acceptance results

- Pass: all seven structural languages have realistic maintained multi-file qualification.
- Pass: difficult, damaged, dynamic, generated, preprocessor, template, macro, and cfg cases remain partial, unresolved, or explicitly limited.
- Pass: a single adapter failure leaves unrelated language results usable.
- Pass: three semantic ecosystems retain independently scored 1.0 precision and recall on curated supported queries.
- Pass: the polyglot case has grounded HTTP and file/schema component boundaries, not just inventories.
- Pass: same-snapshot determinism and incremental range/projection stability are asserted.
- Pass: Tier 2 origins, exact revisions, licenses, and bounded input scopes are recorded and executed without committed third-party bodies.
- Pass: no language or area result is generalized into repository-wide completeness.
- Not claimed: compiler-complete semantics, runtime behavior, macro/generated expansion, or universal real-repository accuracy.

## Known limits

Semantic scoring covers only explicitly curated locally source-resolvable queries;
unsupported and unresolved cases are inspected for honesty but are not treated as
false negatives. Tier 2 is three representative repositories, not a statistical
sample of every framework or repository layout. Sparse input excludes unlisted
upstream paths. Peak memory and very-large-monorepo scaling remain unqualified.

## Recommended implementation choice

Retain per-language adapters and per-area capability reports, source-only semantic
resolution with explicit unresolved targets, and snapshot-bound deterministic
derived identities. Keep Tier 1 in the maintained focused suite and use Tier 2
as revision-pinned optional evidence whose environmental status is explicit.

## Rejected alternatives and reasons

Do not replace realistic fixtures with parser micro-samples: they do not expose
scope collisions, damaged-file remainder, cross-file behavior, or component
boundaries. Do not commit upstream repository bodies. Do not lower semantic
thresholds by scoring unsupported relations as supported, and do not infer a
repository-wide pass from a successful language subset.

## Reusable primitive decision

`reference_only`. Fixture orchestration and qualification assertions are test
support, not a second Production analyzer or validation runner. The only promoted
Production change is the separately committed C++ callable-kind correction and
its direct Rust regression.

## Decision revisit trigger status

Not triggered. The realistic evidence supports the accepted seven-language and
three-semantic-ecosystem contract without narrowing it. Remaining limitations are
already represented by partial/unresolved/unsupported states.

## Follow-up work

Repeat Tier 2 when pinned revisions intentionally change, add a new public
repository only with bounded origin/revision/license input identity, and retain
human usefulness/comprehension evaluation for Phase 8 rather than importing it
into this focused Repository Intelligence task.

## Artifacts

Maintained artifacts are `qualification.json`, `external-repositories.json`,
`external_corpus.py`, `assertions.py`, this report, the Tier 1 fixtures, the shared
fixture-manifest entry, and Production Rust qualification tests. Raw command
results are under ignored `rebuild/.local/validation/`; fetched repositories and
input identities are under ignored
`rebuild/.local/repository-intelligence/external-corpus/`.
