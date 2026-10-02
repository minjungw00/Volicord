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
