# Phase 8 pre-Dogfood entry state

- `replacement_gate = pending`
- `phase_9_ready = false`
- Current pre-Dogfood production/test candidate:
  `914e0f0b5b343ce2bf363f2610fac278d12555d1`
- Technical Phase 8 entry gate: `passed`; eligibility: `eligible`;
  `phase_8_ready = true`
- Admission state: `eligible`
- Exact-final state: `succeeded`; `failure_count = 0`
- Official V11 state: `passed`; 54 of 54 required steps passed
- Credential-retention audit: `passed`; all recorded counts are zero
- Accepted-Decision revisit-trigger assessment: completed; no active Q1–Q13
  trigger reported
- Sanitized gate capsule: `verified`; SHA-256
  `043a90577aa3d7da882d650b8a7e8b3a85b38688c106e4c1451539148f270ded`
- Sanitized evidence archive: `verified`; SHA-256
  `f7513e8c2519c2d1f8b1f7b657418ad7963f2625c63698dcb8c6df64eef93cec`
- Naturalistic Dogfood for the current candidate: `not_run`
- `replacement_pass_candidate = false`

## Maintained conclusion

The current candidate is technically eligible to enter a fresh Phase 8
naturalistic Dogfood campaign. The candidate passed admission with a clean,
unchanged worktree; the exact final succeeded with all four commands and zero
failures; and the same-session official V11 passed all 54 required steps. All
three authenticated Codex targets passed, the credential-retention audit passed
with zero recorded findings or scan errors, and official V11 reported no active
accepted-Decision revisit trigger. The capsule and its referenced final and V11
artifacts identify the same candidate and gate invocation.

This is technical entry eligibility only. Naturalistic Dogfood is `not_run` for
the current candidate, so `replacement_gate = pending`,
`replacement_pass_candidate = false`, and `phase_9_ready = false` remain
unchanged. Predecessor Dogfood descriptors, captures, Runtime Homes,
workspaces, bundles, observations, or session identities cannot qualify or be
reused for this candidate.

## Current entry boundary

The fresh campaign must begin from zero in a separate clean worktree whose
actual Git `HEAD` is exactly the sealed production/test candidate
`914e0f0b5b343ce2bf363f2610fac278d12555d1`. It must retain the maintained
three-class, two-cycle, distinct work/resume-session, automatic, manual,
resource, and accessibility qualification contract. The documentation-only
conclusion commit is not part of the sealed production/test candidate and must
not be used for qualification by supplying the sealed commit only as a harness
candidate argument.

Use `rebuild/scripts/dogfood-campaign` for routine campaign preparation,
rollout intake, activation and blocker validation, Project identity and bundle
collection, bounded Runtime summaries, descriptor completion, manifest
assembly, four-document Markdown/HTML evidence collection, observation-schema
validation, and bounded review packaging. The evaluator/control agent researches
the repositories, independently reviews hidden materiality/oracle data, and
seals each descriptor without exposing it to the operator. The operator
inspects and trusts the intended repository, explicitly approves the
SessionStart hook, runs all twelve fresh naturalistic VS Code Codex sessions
using only the frozen tasks, supplies genuine material-Question answers, and
records genuine subjective and accessibility observations. Ordinary
independent review requires the separate raw rollout archive plus the bounded
review package, not a full Runtime Home.

## Remaining Phase 8 risks

- Naturalistic discovery and promotion of a material Question Candidate, plus
  an explicit user-owned Decision before implementation.
- Local repository-analysis behavior in actual Codex risk handling,
  trustworthy numeric Checkpoint evidence, and repository resolution and
  Recall before continuation in fresh sessions.
- Question relevance, Decision comprehension, interruption cost, and document
  usefulness under real work.
- Manual Viewer accessibility and bounded real-page behavior, plus repeated
  resource qualification across the complete campaign.

No production behavior, test behavior, harness behavior, accepted Decision,
runtime contract, replacement-gate result, or Phase 9 action is changed by this
documentation conclusion.

## Maintained references

- `rebuild/validation/end-to-end/multi-repository/report.md`
- `rebuild/validation/phase-7-summary.md`
- `rebuild/validation/README.md`
- `rebuild/docs/design/open-decisions.md`
- `rebuild/docs/design/acceptance-scenarios.md`
- `rebuild/docs/design/validation-plan.md`
- `rebuild/docs/design/cutover-plan.md`
