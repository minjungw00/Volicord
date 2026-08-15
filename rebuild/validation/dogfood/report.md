# V12 — Phase 8 repeated dogfood and replacement gate

## Status

Failed. The designated sanitized result records `status = failed` and
`replacement_pass_candidate = false` for candidate HEAD
`387b7b527ac588c9061120f6e295508a4bd92c81`. The repeated dogfood evaluation
does not satisfy the replacement gate, and Phase 9 may not begin.

The designated final-admission handoff records `status = skipped`,
`blocking_stage = phase8_dogfood`, `admission_invoked = false`,
`gate_invoked = false`, `final_aggregate_invocations = 0`, and
`official_v11_invocations = 0`. No designated `final-capsule.json` exists, so
there is no exact-final/V11-sealed Phase 8 candidate to interpret.

## Goal

Independently audit the sanitized two-cycle Phase 8 dogfood result for three
actual repositories, preserve observations separately from interpretation, and
decide whether the replacement gate passed and Phase 9 cutover planning may
begin. The audit also checks the designated final handoff without rerunning
exact final or official V11.

## Accepted decisions being validated

- Q1 staged Inquiry relevance and interruption behavior.
- Q2 seven-language structural coverage, three semantic ecosystems, polyglot
  honesty, and out-of-set inventory fallback.
- Q3 local-first operation and explicit external-transmission authority.
- Q4 Linux/Codex surfaces and Korean/English fixed UI.
- Q5 four useful, portable, source-grounded Markdown/HTML documents and their
  accessibility.
- Q6 portable Project identity, clone binding, divergence, and conflict.
- Q7 correction, supersession, and deletion.
- Q8-A Linux/Codex operation and Q8-B fresh-service legacy exclusion.
- Q9 bounded Recall, Q10 Candidate boundaries, Q11 Checkpoint grounding, Q12
  Guarded effect behavior, and Q13 Decision reuse without repetition.

## Input repositories and revisions

The sanitized result identifies three actual repository targets. The harness
records no repository-identity blocker, and its tracked identity check rejects
maintained fixtures as substitutes for these targets.

| Class | Origin | Revision | Observed repository shape |
| --- | --- | --- | --- |
| `volicord` | `https://github.com/minjungw00/Volicord.git` | `387b7b527ac588c9061120f6e295508a4bd92c81` | 1,046 files, 241 documentation files, all seven official structural languages |
| `small-python` | `https://github.com/pypa/sampleproject.git` | `621e4974ca25ce531773def586ba3ed8e736b3fc` | 12 files, one documentation file, Python only; MIT `LICENSE.txt` SHA-256 `71e0bd649395f47e82b500dc6261ce4b8e8d03774727f583e09f5b947e75de97` |
| `polyglot-medium` | `https://github.com/tree-sitter/tree-sitter.git` | `0e2af0d8d1089e750def69ee51e75dd7cc15f531` | 604 files, 76 documentation files, all seven official structural languages; MIT `LICENSE` SHA-256 `c5cfb43042b6b72045f4ba997834d0a7786d2793d91680868b5815b39f14fc78` |

The candidate worktree was clean before and after the complete evaluation.
Current Git inspection independently confirmed that the candidate is the
expected `test: add phase 8 dogfood evaluation harness` commit and that it adds
only `evaluation.json`, `harness.py`, and `assertions.py` under the dogfood
validation directory.

## Environment and tool versions

The sanitized dogfood result records Linux on `x86_64` with Python `3.12.3`.
It intentionally contains no local absolute paths, command logs, source bodies,
private prompts, credentials, or provider payloads. The result does not project
kernel, Git, Cargo, Rust, Codex CLI, or browser-engine versions, so this report
does not infer them.

## Candidate approaches

The tracked Phase 8 harness reuses the maintained V11 product journey for two
independent runtime cycles per actual repository. It separately runs the
maintained Phase 5 acceptance assertion for structural, fallback, and semantic
regression coverage and collects bounded manual-quality observations when
provided.

The observed run did not authorize the six fresh authenticated Codex turns.
Consequently, the product steps were still exercised locally, but the Codex/MCP
connection step and the fresh Codex-turn boundary were not qualified. No
fixture was substituted for an actual repository, and no old ignored validation
artifact is used to fill the missing final/V11 handoff.

## Commands and configuration

The sanitized result records the focused fixture regression command as:

```text
python3 rebuild/validation/repository-intelligence/phase-5-acceptance/assertions.py
```

That assertion maps maintained Production tests for all seven structural
languages, out-of-set inventory fallback, and Java/Maven, TypeScript/Node, and
Rust/Cargo semantic capability. The current result's evaluation-definition
SHA-256 is
`5a33c578812d4a4569770982a1fe91b3a6972ab40329135cb7b228f219ce7f36`.

The missing Phase 8 authorization assertion is
`phase8-openai-codex-project-health-six-real-repository-cycles`. Its maintained
purpose is one bounded `project_health` turn in each of two fresh cycles for
the three actual repositories. No exact-final, gate, or official V11 command
was run for this candidate.

## Observed results

Each repository has two distinct Project identities and
`independent_fresh_runtime_cycles = true`. Within all six cycles,
`restart_recall` passed. This establishes six distinct runtime cycles and the
maintained restart/Recall step; it does not establish fresh authenticated Codex
turns because that transmission was not authorized.

Across 108 required product-step outcomes, 102 passed and six were
`environment_blocked`. Every cycle has 17 passed steps and one
`environment_blocked` `codex_mcp_connection` step. There were no step-level
`failed`, `partial`, `unsupported`, or `skipped` outcomes, but every repository
aggregate is `environment_blocked` and therefore is not a passed repeated
journey.

Across 72 quality observations, 48 passed and 24 were partial. Every cycle
passed Context recovery accuracy, Decision non-repetition, source grounding,
capability honesty, coverage, correction/supersession/deletion, portability,
and recovery. Every cycle left Question relevance, Decision comprehension,
interruption cost, and document fidelity/usefulness at `partial`; the basis is
the corresponding automated journey step, not a bounded human-subject or
authorized fresh-agent observation.

The maintained structural/fallback regression passed, covers the official
structural fixtures and out-of-set fallback fixture, and explicitly records
that fixtures did not substitute for the first three actual repositories. The
tracked Phase 5 conclusion and V02 report remain the qualification references
for the seven structural languages and the three selected semantic ecosystems;
this dogfood result does not widen those bounded claims.

Correction/deletion, another-clone portability, divergent conflict handling,
provider/parser/derived-index recovery, capability honesty, and source
grounding all passed their automated step bases in both cycles for all three
repositories. These are step observations, not evidence that the four partial
human-facing quality criteria succeeded.

## Coverage and failures

The dogfood result preserves these blockers:

- the exact bounded fresh-Codex-turn authorization was absent;
- one or more accepted Decision revisit triggers are active;
- accessibility has a blocker or unqualified criterion;
- all three repeated repository journeys did not pass; and
- all six cycles have partial Question relevance, Decision comprehension,
  interruption cost, and document fidelity/usefulness.

Accessibility records `document_html_language = failed` in all cycles.
Korean and English fixed UI evaluation is `environment_blocked` because the
viewer did not start. Keyboard reachability, visible focus, not-color-only,
headings/labels, and narrow/zoom behavior have no recorded result and remain
unobserved. The bounded accessibility claim is structural only: no standards
certification, human-subject qualification, or browser-layout-engine
qualification occurred.

No current exact-final or official V11 evidence exists for the dogfood
candidate. The earlier maintained V11 report sealed
`80dd08e8828d7159ac7b8839178ccdd9f9013851` only for Phase 8 entry; it neither
seals candidate `387b7b52` nor proves Phase 8 completion.

## Performance and resource observations

| Repository | Cycle duration (ms) | Inventory (ms) | Recall (ms) | Repair/reindex (ms) | Per-document generation (ms) | Per-document bytes | Runtime home bytes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `volicord` | 102,901.229; 102,959.413 | 3,888.243; 3,716.715 | 2,452.930; 2,359.667 | 3,847.505; 4,290.097 | 7,045.134–7,352.455 | 97,461,095–148,901,706 | 621,254,612 in both cycles |
| `small-python` | 568.780; 564.260 | 9.470; 7.997 | 3.117; 3.153 | 8.481; 9.155 | 6.372–6.975 | 28,931–75,113 | 671,780 in both cycles |
| `polyglot-medium` | 23,220.861; 23,452.527 | 1,239.265; 1,224.322 | 458.250; 478.661 | 1,439.693; 1,735.238 | 1,377.356–1,560.362 | 19,525,167–29,859,595 | 127,452,102 in both cycles |

Portable bundles were stable between cycles at 30,195, 30,211, and 30,223
bytes for the three repository classes respectively. Recorded derived-state
bytes were zero in every cycle. The close two-cycle values are bounded repeat
observations, not sustained-duration or resource-ceiling qualification. Peak
memory is explicitly `unsupported`; therefore no memory ceiling is claimed.

The Volicord and medium-polyglot document outputs are large, while document
fidelity/usefulness remained partial. The evidence does not establish that
users would accept these sizes or that the outputs are useful enough for
handoff.

## Privacy and external transmission

The dogfood journey required six bounded Codex transmissions, but
`codex_transmission_authorized = false`, the authorization assertion ID is
absent, and no fresh Codex turn is claimed. The sanitized result contains no
raw source or credential content. It also makes no commercial semantic-provider
success claim; passed unavailable-provider recovery is not commercial-provider
qualification.

## Acceptance results

| Acceptance area | Conclusion |
| --- | --- |
| Candidate identity and cleanliness | Passed for dogfood candidate `387b7b527ac588c9061120f6e295508a4bd92c81`; clean before and after |
| Three actual repository identities | Passed harness identity checks; exact origins and revisions recorded above |
| Two independent runtime cycles per repository | Passed; six distinct Project identities |
| Fresh authenticated Codex turns | Environment-blocked; exact assertion absent |
| Required product steps | Not passed as an aggregate: 102 passed, six environment-blocked |
| Human-facing quality qualification | Not passed: 24 partial observations across four criteria |
| Structural/fallback regression | Passed |
| Semantic qualification reference | Present through maintained Phase 5 acceptance and V02 evidence; no wider real-repository semantic claim |
| Accessibility | Failed and partly unobserved |
| Decision revisit assessment | Failed: observed active Q5 trigger |
| Current exact-final/V11 handoff | Missing by design after dogfood failure; zero final/V11 invocations |
| Replacement gate | `replacement_gate = failed` |
| Phase 9 readiness | `phase_9_ready = false` |

## Known limits

- The four subjective quality criteria use agent-observed bases and remain
  partial; no human-subject usability conclusion is available.
- Peak memory, sustained resource ceilings, browser layout, five accessibility
  checks, non-Linux operation, and commercial-provider behavior are not
  qualified.
- Sanitization excludes raw command logs, source bodies, private prompts,
  provider payloads, credentials, and local paths. This report does not infer
  details absent from the bounded handoff.
- The maintained fixture regression establishes bounded structural, fallback,
  and semantic contracts; it does not turn the first three repositories into
  fixtures or prove population-wide semantic accuracy.

## Recommended implementation choice

Keep the replacement gate closed and do not begin Phase 9. Preserve candidate
`387b7b52` as the dogfood evidence identity, while recognizing that no current
exact-final/V11-sealed Phase 8 candidate exists. Route findings only to their
maintained owners before any separate remediation work is designed.

## Rejected alternatives and reasons

- Do not infer a pass from 102 successful product steps: every repeated journey
  contains an environment-blocked required step.
- Do not treat distinct runtime Project identities as proof of fresh Codex
  turns: the exact transmission authority was absent.
- Do not promote partial automated bases to human-facing quality success.
- Do not reuse the earlier Phase 7 V11 capsule or search old ignored validation
  runs: it sealed a different candidate and only opened Phase 8 entry.
- Do not rerun exact final or V11 after a failed dogfood result in this
  documentation-only session.

## Reusable primitive decision

`reference_only` for production. The Phase 8 harness and sanitized result are
validation evidence and do not own product semantics or authorize production
changes.

## Decision revisit trigger status

Active. The maintained Decision-register assessment reports no previously
active trigger, but the current dogfood evidence reports Q5 active because
`document_html_language` failed. Q5's recorded revisit condition includes
Markdown/HTML failing required accessibility. This active trigger independently
forces `replacement_gate = failed` and `phase_9_ready = false`.

The Q5 contract remains accepted until a separate user decision changes it.
This report records the trigger and does not silently narrow the document or
accessibility contract.

## Follow-up work

Concrete finding ownership is:

- Q5 HTML language failure, large document outputs, and partial document
  fidelity/usefulness: `projections-and-documents.md`, with Q5 review governed
  by `open-decisions.md`.
- Viewer-start blocking and unobserved fixed-UI/accessibility checks:
  `architecture.md` Host and User Adapters plus `failure-and-recovery.md`.
- Missing current-invocation Codex authorization and the six unqualified fresh
  turns: the Phase 8 dogfood validation boundary and
  `privacy-and-provider-boundary.md`; absence is not user consent.
- Partial Question relevance and Decision comprehension:
  `inquiry-and-decision.md`.
- Partial interruption cost: the accepted Q1/Q12 user-flow boundaries in
  `open-decisions.md` and `architecture.md`.
- Unsupported peak memory and sustained-resource qualification: the Phase 8
  validation evidence owner; no product ceiling is inferred.

No implementation fix, contract change, or generic remediation design is part
of this documentation conclusion.

## Artifacts

- Designated sanitized dogfood result:
  `rebuild/.local/phase8/dogfood-result.json`, SHA-256
  `a9a4cab2934fc153e6e9780d88f8803667e198f9d4d8b191ccbc6b599f125db5`.
- Designated final-admission handoff:
  `rebuild/.local/phase8/final-admission.json`, SHA-256
  `e4178e36abb4e4b3bcf1695fc7155340c38a7ea10e17b8a5f2dc75e78ae7860c`.
- Designated final capsule: absent; the final-admission handoff records
  `capsule_produced = false`.
- Maintained inputs: `evaluation.json`, `harness.py`, `assertions.py`, Phase 5
  acceptance assertions and reports, the active design owners, and this report.

Raw Phase 8 state remains ignored and is not copied into maintained documents.
