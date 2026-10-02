# Focused Viewer reading findings

Synthetic diagnostic support only; no human observation or qualification.

Baseline HEAD: `8c81800f80e7afe292ccc7faa944caee57ff23ef`. Maintained workload: Viewer integration test
`representative_large_repository_page_is_deterministically_bounded`, 192 generated
Python modules plus its maintained setup, deep English rendering, one cold and
eight warm renders. Test source SHA-256: `263785226703b6e9831f5a7750b120da9c8e88846242dea2d427cd9deaef613d`.
Environment: Linux 6.18.33.2-microsoft-standard-WSL2 x86_64;
rustc: rustc 1.97.1 (8bab26f4f 2026-07-14); cargo: cargo 1.97.1 (c980f4866 2026-06-30); python: Python 3.12.3; git: git version 2.43.0. Debug test profile.

Command: `rebuild/scripts/validate focused viewer-reading-baseline -- cargo test
--manifest-path rebuild/Cargo.toml -p volicord-viewer --test viewer
representative_large_repository_page_is_deterministically_bounded -- --exact
--nocapture`. Numeric exit 0, no termination/spawn error. Cold render
92.012 ms; warm range
90.380–93.156 ms.
Each render: one projection pass, one analysis decode, zero health re-decodes.
Raw complete streams/result remain ignored under `rebuild/.local/validation/`.
This does not measure long-history exact selection, RSS or human reading cost;
no universal latency claim follows. Existing stage profiles were available; no
additional profiler was required. Re-run the named workload to compare changes.

Prototype input: `fixtures/viewer-reading/scenario.json` and independent
`expected.json`. Run `python3 rebuild/validation/end-to-end/multi-repository/
viewer_reading_prototype.py` (one unbroken script path) to recreate a disposable
self-contained HTML under ignored `rebuild/.local/viewer-reading/`.

Reading tasks: choose a Work by identity despite duplicate titles; follow its
Goal, changes, next step, scoped Decisions/code, then exact record evidence.
Prototype annotations are explicitly labeled and never canonical facts. No
listener, provider, new production Viewer, runtime home or database is created.

Observed by deterministic artifact inspection: identity anchors expose distinct
Works; tools sit separately; raw audit-heavy multilingual text requires explicit
disclosure. Missing rationale/result remains unavailable. Navigation alone cannot
explain long free prose; excerpts still require reading the original to determine
intent/results. Semantic Work realization, production navigation/selection controls,
read-cost optimization and human comprehension evidence remain unimplemented.

Baseline dependency identity: `rebuild/Cargo.lock` SHA-256 `02ed72ca48eafa2ec8162716d85fb557f0d9da5f5166486e259f33f604ce277a`.
Fixture setup and generator are in the hashed test source; no private input was used.

Maintained regression consumer: `volicord-operations --test viewer_reading` loads
the shared scenario and independent expectations, constructs real canonical
records through Local Operations/Store, and adds a declared synthetic semantic
binding between the C and TypeScript file entities. This binding is not a claim
that the structural analyzer resolves cross-language calls. The Python fixture
also contains real local-call and unresolved-call evidence; existing projection
flow tests check that unresolved endpoints never become fabricated entities.

Implemented reading limits: display quotations keep 384 Unicode scalar values,
report exact omitted bytes/characters, and retain full original text. Selected
Work/history/Decision details survive bounded parent lists; graph endpoint bounds
and omissions remain explicit. Failed, rejected and unverified observations keep
their own Checkpoint and Source identities; earlier passes expose later changes.
Missing explanatory text still cannot explain intent or results. Fixed bilingual
labels preserve original quote language rather than attesting translation.

Remaining read-cost work: canonical full-history reads and revision catalogs,
full latest-analysis decoding, canonical-scope clones, repeated per-Work/source
scans, and full selected-Work original/state arrays are not paginated or cached.
The earlier baseline measures the maintained default rendering workload only.
The foundation makes no measured speedup, long-history memory ceiling or human
comprehension claim. Later Viewer controls must use the maintained selector APIs
and owner contracts, not depend on this disposable HTML.


## Purpose-oriented navigation implementation

The Viewer README now owns the operative route, CLI and whole-snapshot procedures.
`ViewerView` and `CodeScope` map to the shared Work selector; `ProjectionDetail`
resolves Decision/entity detail before parent display bounds. Work and Decision
indexes have native previous/next links in pages of 64. Static export uses one
canonical/analysis projection, shared section functions, native details and only
existing unique internal fragments. Selected-Work code is explicitly omitted from
this bounded whole snapshot; repository code remains separately labeled.

Real Viewer render, in-memory HTTP and executable listener tests exercise the
maintained adversarial canonical fixture through Local Operations. Semantic assertions
cover exact old Work identity, all its observations, missing rationale, Goal-only
work, independent failed/rejected/unverified states, multilingual quotation labels,
strict invalid/not-found behavior, pagination and static fragment closure. Mutation,
forgetting/repair, document export, Guarded target/revision/Source and disconnect
checks remain actual entry-point checks. No markup check attests browser interaction
or human comprehension.

Focused navigation validation: Viewer (all targets/features), Operations CLI/reading,
projection/document/current-flow, V06, current CLI parity, V11 self-check, archive,
Dogfood capture and campaign self-tests passed. Listener and campaign special-socket
fixtures required sandbox escalation for local socket access. V08 assertions stopped
at the pre-existing historical Production-drift guard against `c17279bb`; it did not
run the integration journey. The guard was not weakened. Current executable Viewer
and Operations CLI checks supply scoped entry-point evidence only.

The reading task "explain a long free-prose Goal/result without reading the original"
remains unsupported where canonical input offers no semantic summary. Quotations,
labeled excerpts and explicit unavailable explanations are shown; fixed labels do
not attest translation. Browser keyboard, touch, zoom and narrow-screen interaction,
plus actual human comprehension, require independent next-session verification.


## Grounded code navigation

Focused Code tests cover long/duplicate symbol labels, distinguishing path suffixes,
variable node heights, SCC cycles, distinct parallel ports and self-loop geometry.
The real-entry-point 90-module fixture selects an entity omitted from the first map,
checks incoming/outgoing lists and retained Source/ranges, rejects unrelated Work
scope, and checks HTML escaping with the actual `unsafe<&>.py` locator. The original
polyglot fixture and unresolved structural calls remain in shared Operations tests.
No constituent relations are grouped; graph omissions and upstream omissions have
separate counts, and full names/endpoints remain in the native details/list path.
Snapshot links remain closed to actual unique targets.

No browser automation tool, Chromium binary or Python Playwright installation was
available in this session. Prototype and production artifacts were inspected through
maintained markup/semantic assertions, not browser interaction or visual/human review.
Independent browser checks must exercise SVG focus/fragment behavior, details keyboard
and touch operation, zoom, long-name node fit, dense/cyclic edge legibility and narrow
ordinary-text layout. This session does not implement that dedicated runner.
