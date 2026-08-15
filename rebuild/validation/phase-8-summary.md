# Phase 8 replacement-gate conclusion

- `replacement_gate = failed`
- `phase_9_ready = false`
- Dogfood candidate HEAD:
  `387b7b527ac588c9061120f6e295508a4bd92c81`
- Current exact-final/V11-sealed Phase 8 candidate: none
- Dogfood result: `failed`; `replacement_pass_candidate = false`
- Final handoff: admission and gate skipped; exact-final invocations `0`;
  official V11 invocations `0`; capsule not produced

## Maintained conclusion

Phase 8 did not pass and Phase 9 may not begin. The sanitized repeated-dogfood
result covers three actual repositories and two independent runtime cycles per
repository, but all six cycles are environment-blocked at the required
Codex/MCP connection step. Across the full run, 102 of 108 required product
steps passed and six were environment-blocked. The exact bounded authorization
for six fresh Codex turns was absent.

The result also records 48 passed and 24 partial quality observations. Context
recovery, Decision non-repetition, source grounding, capability honesty,
coverage, correction/deletion, portability, and recovery passed their automated
bases in all cycles. Question relevance, Decision comprehension, interruption
cost, and document fidelity/usefulness remained partial in all cycles.

Accessibility failed because generated HTML did not satisfy the document
language check. Korean/English fixed UI was environment-blocked by viewer
startup failure; five other accessibility checks were unobserved. The HTML
failure activates accepted Decision Q5's revisit trigger. An active revisit
trigger alone prevents replacement-gate passage.

The designated final-admission handoff truthfully stops after failed dogfood:
it records no admission, gate, exact-final, or official V11 invocation, and the
designated final capsule is absent. Therefore candidate `387b7b52` is not a
sealed exact-final/V11 candidate. The earlier Phase 7 V11 candidate
`80dd08e8828d7159ac7b8839178ccdd9f9013851` established Phase 8 entry only; it
does not seal the later dogfood candidate or prove Phase 8 completion. This
later documentation-only commit is likewise outside exact final.

## Repository and repeated-use evidence

| Class | Actual repository identity | Revision | Two-cycle status |
| --- | --- | --- | --- |
| Volicord | `https://github.com/minjungw00/Volicord.git` | `387b7b527ac588c9061120f6e295508a4bd92c81` | two distinct runtime Project identities; both environment-blocked |
| Small Python | `https://github.com/pypa/sampleproject.git` | `621e4974ca25ce531773def586ba3ed8e736b3fc` | two distinct runtime Project identities; both environment-blocked |
| Medium polyglot | `https://github.com/tree-sitter/tree-sitter.git` | `0e2af0d8d1089e750def69ee51e75dd7cc15f531` | two distinct runtime Project identities; both environment-blocked |

The candidate was clean before and after the evaluation. Each cycle passed the
restart/Recall, Decision non-repetition, source grounding, correction/deletion,
portable-clone/conflict, capability-honesty, and recovery bases. The maintained
Phase 5 regression passed all seven structural-language, out-of-set fallback,
and three selected semantic-ecosystem mappings. Fixtures remained regression
inputs and did not replace the actual dogfood repositories.

## Performance, size, privacy, and bounded claims

Volicord cycles took about 102.9 seconds each, medium-polyglot cycles about
23.2–23.5 seconds, and small-Python cycles about 0.56 seconds. Volicord document
outputs ranged from 97,461,095 to 148,901,706 bytes per output, and
medium-polyglot outputs from 19,525,167 to 29,859,595 bytes. Their usefulness
remained partial. Two-cycle runtime sizes were stable, but two cycles do not
qualify sustained resource behavior; peak memory is unsupported.

No fresh Codex transmission was authorized or claimed. The sanitized handoff
contains no raw source or credential content and makes no commercial semantic-
provider success claim. Accessibility evidence is structural and
environment-bounded, not human certification or browser-engine qualification.

## Owner-routed unresolved findings

- Q5 active revisit trigger, HTML language failure, document size, and partial
  usefulness: `projections-and-documents.md` and `open-decisions.md`.
- Viewer-start/environment-blocked fixed UI and unobserved accessibility:
  `architecture.md` Host and User Adapters and `failure-and-recovery.md`.
- Missing bounded Codex-turn authorization: the dogfood validation boundary and
  `privacy-and-provider-boundary.md`.
- Partial Question relevance and Decision comprehension:
  `inquiry-and-decision.md`.
- Partial interruption cost: Q1/Q12 in `open-decisions.md` and the adapter/local
  operation flow in `architecture.md`.
- Unsupported peak memory and sustained-resource qualification: the Phase 8
  validation evidence owner.
- Missing current final/V11 capsule: Phase 8 final admission remains prohibited
  while the dogfood result is not a replacement-pass candidate.

No production behavior, test behavior, harness behavior, product contract, or
Phase 9 cutover action is changed by this conclusion.

## Maintained references

- `rebuild/validation/dogfood/report.md`
- `rebuild/validation/dogfood/evaluation.json`
- `rebuild/validation/phase-5-summary.md`
- `rebuild/validation/end-to-end/multi-repository/report.md`
- `rebuild/validation/phase-7-summary.md`
- `rebuild/docs/design/open-decisions.md`
- `rebuild/docs/design/acceptance-scenarios.md`
- `rebuild/docs/design/validation-plan.md`
- `rebuild/docs/design/cutover-plan.md`
