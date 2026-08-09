# Wave 1 validation conclusion

- Evidence baseline: `5bf248b3e05adbfed92ef8f741c0d13af1660aa9`
- Independent replay: 2026-08-09 KST
- Architecture Gate: `ready`

## Validation status

| Validation | Status | Current evidence-backed conclusion |
|---|---|---|
| V01 — Polyglot structural analysis | `partial` | The maintained assertions pass for all nine V01-owned fixtures while the shared manifest also contains V03 and V05 entries. The evidence covers Java, Python, JavaScript, TypeScript, C, C++, Rust, the three-language polyglot fixture, and the out-of-set inventory fallback. It supports the common structural envelope and language-adapter responsibility boundary, but does not qualify a production parser library or establish real-repository accuracy. |
| V03 — Canonical Context and portable bundle | `passed` | The maintained fixture demonstrates canonical/derived independence, deterministic export/import, revision and supersession, restart and pre/post-commit crash behavior, explicit clone rebinding, managed sensitive deletion with no observed byte residue, and independence from analyzers, providers, and hosts. It supports separate persistence, portability, local binding, revision, supersession, and deletion responsibilities without promoting either experimental store. |
| V05 — Inquiry frontier and session resume | `passed` | The maintained fixture demonstrates prerequisite-correct deterministic frontiers, fact resolution before user questioning, exact Question identity/revision and user-turn linkage, recommendation/choice separation, all seven terminal outcomes, restart recovery, and semantic-key non-repetition. It supports durable Question state and deterministic frontier recomputation as separate responsibilities while keeping the pause snapshot observational. |

## Candidate responsibility boundaries supported by evidence

- Repository Intelligence can use a small source-bound structural envelope while language adapters own language-specific extraction, extensions, capability degradation, and failure details. Inventory remains independent of structural analyzer availability; structural facts remain distinct from interpretation.
- Canonical Context can remain independent of Repository Intelligence, LLM providers, and host integration. Portable canonical records can be separated from local clone bindings and rebuildable derived state, with correction revisions, Decision supersession, and managed deletion remaining explicit.
- Inquiry can read deterministic facts and canonical Question state, recompute the current frontier in a stable order, and bind each accepted response to the exact displayed Question revision and user-turn Source. Agent recommendation remains separate from user choice, and a Checkpoint need not become a second frontier authority.
- All experiment implementations remain disposable validation support. No Wave 1 experiment is approved for production-code promotion by this conclusion.

## Known limits relevant to Phase 3

- V01 uses small self-authored fixtures and a fixture-sufficient lexical prototype. Production parser selection, broader repository accuracy, macro/generated-code behavior, semantic relations, token-exact end ranges, call precision, build context, and dependency-aware invalidation remain unvalidated.
- V03 does not establish concurrency, large-state behavior, corruption repair, schema upgrades, encryption, multi-project layout, merge semantics, or a final tombstone policy. Residue checks cover managed experiment files on the tested filesystem, not external filesystem or backup layers; cross-clone deletion propagation remains for V04.
- V05 does not validate automatic Question discovery, materiality quality, general paraphrase recognition, concurrent host turns, authorization, or malicious inputs. The disposable response path is not one atomic host-turn transaction, and production prerequisite semantics and bounded resume presentation remain undefined.
- V02, V04, V06, and V09 still own semantic adapters, divergent merge, source-grounded documents, and Recall/Checkpoint extension evidence. `ready` does not treat those later validations as complete and does not satisfy the production-code promotion gate.

## Decision revisit triggers

- Q2 polyglot capability: inactive. All seven accepted structural languages and the out-of-set fallback remain represented; the evidence does not show the common envelope infeasible.
- Q6 Project identity and portability: inactive. Stable identity, deterministic portability, and explicit another-clone binding passed for the maintained fixture.
- Q7 revision, supersession, and deletion: inactive. The managed deletion checks passed. Cross-clone deletion propagation is an untested V04 concern, not a demonstrated trigger.
- Q1 Inquiry: inactive. The maintained fixture represented the full accepted terminal vocabulary, deterministic resumption, and an unbounded material branch set without reducing the accepted contract.
- No accepted product decision requires reopening from the current Wave 1 evidence.

## Architecture Gate conclusion

`ready`. V01 provides sufficient evidence to carry the common structural model and language-adapter boundary into Phase 3 while retaining its production-parser limits. V03 provides sufficient evidence for the canonical storage, portable bundle, revision, supersession, deletion, and binding responsibility boundaries. V05 provides sufficient evidence for durable Question state, deterministic frontier recomputation, exact response linkage, and recommendation/choice separation. Each report records known limits and rejected alternatives, the canonical responsibility remains separable from analyzer and host concerns, and no accepted Decision revisit trigger is active.

This conclusion permits Phase 3 architecture work only. It does not promote experiment code, certify production behavior, or complete the later validation waves.

## Maintained reports and reproducibility references

- Runner contract: `rebuild/scripts/validate`; ownership and usage: `rebuild/AGENTS.md`; maintained validation policy: `rebuild/docs/design/validation-plan.md` and `rebuild/validation/README.md`.
- Shared fixture authority: `rebuild/validation/shared/fixture-manifest.json`; manifest check: `rebuild/scripts/check-fixture-manifest`.
- V01 report and exact focused command set: `rebuild/validation/repository-intelligence/polyglot-structural/report.md`; executable evidence: `assertions.py` and `prototype.py` beside that report.
- V03 report and exact focused command set: `rebuild/validation/canonical-context/portability/report.md`; executable evidence: `assertions.py` and `prototype.py` beside that report.
- V05 report and exact focused command set: `rebuild/validation/inquiry/frontier-resume/report.md`; executable evidence: `assertions.py` and `prototype.py` beside that report.
- Independent runner self-test artifacts: `rebuild/.local/validation/20260808T211857.237828Z-runner-self-test-0f2w2get`.
- Independent V01 focused replay artifacts: `rebuild/.local/validation/20260808T211917.652017Z-v01-fixture-manifest-audit-wf_bwn61`, `rebuild/.local/validation/20260808T211917.657045Z-v01-assertions-audit-c4n9kvzm`, `rebuild/.local/validation/20260808T211917.662695Z-v01-candidate-probes-audit-uypg9_ze`, and `rebuild/.local/validation/20260808T211917.670216Z-v01-report-shape-audit-vybnfgrn`.
- Independent V03 focused replay artifacts: `rebuild/.local/validation/20260808T211917.672647Z-v03-fixture-manifest-audit-hhlrlut6`, `rebuild/.local/validation/20260808T211917.717733Z-v03-assertions-audit-got3ivnf`, `rebuild/.local/validation/20260808T211917.724432Z-v03-assertions-repeat-audit-6xlpabcj`, and `rebuild/.local/validation/20260808T211917.684679Z-v03-report-shape-audit-u63hy__n`.
- Independent V05 focused replay artifacts: `rebuild/.local/validation/20260808T211917.734704Z-v05-fixture-manifest-audit-73umzt1c`, `rebuild/.local/validation/20260808T211917.696349Z-v05-assertions-audit-e_3h9myp`, `rebuild/.local/validation/20260808T211917.713364Z-v05-assertions-repeat-audit-3j45o3me`, and `rebuild/.local/validation/20260808T211917.701339Z-v05-report-shape-audit-7m578unp`.

The `.local` paths are ignored replay evidence. The maintained reports, fixtures, assertions, and runner are the reproducibility surface.
