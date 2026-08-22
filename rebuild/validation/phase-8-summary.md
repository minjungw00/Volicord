# Phase 8 pre-Dogfood entry state

- `replacement_gate = pending`
- `phase_9_ready = false`
- Current pre-Dogfood production/test candidate:
  `fc3f9cac54e9c82e838fc2940b59d00166acc142`
- Technical Phase 8 entry gate: `passed`; eligibility: `eligible`;
  `phase_8_ready = true`
- Admission state: `eligible`
- Exact-final state: `succeeded`; `failure_count = 0`
- Official V11 state: `passed`; 54 of 54 required steps passed
- Credential-retention audit: `passed`; all recorded counts are zero
- Accepted-Decision revisit-trigger assessment: completed; no active Q1–Q13
  trigger reported
- Sanitized gate capsule: `verified`; SHA-256
  `841540e55d05dcc18bece5ed40c5bdd3206a660407ab85e39e325ef39a1a2954`
- Sanitized evidence archive: `verified`; SHA-256
  `2a2080ad1a02c6840b7d1619ab8a78851fc57ec5cccc4cef9fd955591ea32f5a`
- Naturalistic Dogfood for the current candidate: `not_run`
- Automated Dogfood qualification: `not_run`
- Campaign-level human review: `not_provided`
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
the current candidate. Automated qualification has not run and optional human
review has not been provided, so `replacement_gate = pending`,
`replacement_pass_candidate = false`, and `phase_9_ready = false` remain
unchanged. Predecessor Dogfood descriptors, captures, Runtime Homes,
workspaces, bundles, observations, or session identities cannot qualify or be
reused for this candidate. Any predecessor Small Python cycle remains diagnostic
only and is not qualifying evidence for this candidate.

The current human-facing surface gives generated documents a
comprehension-first body and keeps inspectable grounding and audit detail in a
separate trailing appendix or closed HTML disclosure. The live Viewer uses the
same human-first hierarchy. A static Viewer snapshot is a distinct,
self-contained, read-only share and review artifact; it is not interchangeable
with the four generated documents or their adoption lifecycle.

## Current entry boundary

The fresh campaign must begin from zero in a separate clean worktree whose
actual Git `HEAD` is exactly the sealed production/test candidate
`fc3f9cac54e9c82e838fc2940b59d00166acc142`. It must retain the maintained
three-class, two-cycle, distinct work/resume-session, automated, optional
human-review, resource, and accessibility qualification contract. The
documentation-only conclusion commit is not part of the sealed production/test
candidate and must not be used for qualification by supplying the sealed commit
only as a harness candidate argument.

Use `rebuild/scripts/dogfood-campaign` for routine campaign preparation and
evidence handling. The evaluator/control agent researches the repositories,
independently reviews hidden materiality/oracle data, and seals each descriptor
without exposing that material to the operator. Repository and SessionStart
hook trust remain explicit user actions. The operator then runs all twelve
fresh naturalistic VS Code Codex chats using only the frozen tasks, supplies
genuine material-Question answers, preserves every raw rollout without
per-session evidence-processing interruptions, and provides the twelve files
once for batch ingestion. The helper maps the cycles and automatically derives
canonical bundles, bounded Runtime/activation summaries, all four document
kinds in Markdown and self-contained HTML, and read-only static Viewer
snapshots.

Automated Dogfood can complete without a human review. Optional campaign-level
human review is prepared and recorded separately against the immutable
automated result. Its absence leaves replacement qualification pending rather
than failing automated Dogfood, and a human pass cannot override any machine
failure. Ordinary independent review uses the byte-exact raw rollout archive
plus the bounded review package, not a full Runtime Home.

## Remaining Phase 8 risks

- Naturalistic discovery and promotion of a material Question Candidate, plus
  an explicit user-owned Decision before implementation.
- Local repository-analysis behavior in actual Codex risk handling,
  trustworthy numeric Checkpoint evidence, and repository resolution and
  Recall before continuation in fresh sessions.
- Question relevance, Decision comprehension, interruption cost, generated
  document readability, and static Viewer usability under real work.
- Live Viewer accessibility and bounded real-page behavior, plus repeated
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
