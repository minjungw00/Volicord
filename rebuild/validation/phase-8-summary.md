# Phase 8 pre-Dogfood entry state

- `replacement_gate = pending`
- `phase_9_ready = false`
- Current pre-Dogfood production/test candidate:
  `d0f87f0376cbc6c9d8fe54e8d6ce17a4eabf93c6`
- Exact-final/V11 state: `passed`; `phase_8_ready = true`
- Naturalistic Dogfood for the current candidate: `not_run`
- `replacement_pass_candidate = false`

## Maintained conclusion

The current candidate is qualified to enter a fresh Phase 8 naturalistic
Dogfood campaign. The capsule-backed exact final succeeded with all four
commands and zero failures, and the same-session official V11 passed all 54
required steps. The official Decision assessment reports no active Q1–Q13
revisit trigger.

This is entry eligibility only. No naturalistic activation, Question relevance,
Decision comprehension, interruption cost, document usefulness, accessibility,
or sustained-resource result exists for candidate `d0f87f0`. The replacement
gate therefore remains pending and Phase 9 may not begin.

The earlier `a1efc336` campaign and its `387b7b52` repeated-dogfood candidate
remain nonqualifying prior evidence. Their ignored descriptors, workspaces,
rollouts, bundles, observations, and maintained raw report are not evidence for
the current candidate and are not reinterpreted by this summary.

## Current entry boundary

The current product surface adds `project_resolve`, a read-only repository-bound
lookup. It canonicalizes the supplied absolute repository path, returns the
existing Project plus current binding identity/revision when found, returns an
explicit `not_found` otherwise, and does not initialize, bind, or revise a
Project. Fresh-resume qualification now requires this resolution to find the
cycle's canonical Project before Recall and before repository inspection or
continued work.

A new campaign must create fresh descriptors whose hidden
`work_task_materiality_basis` is an exact bounded fragment of the ordinary
`work_user_task` after the maintained normalization. Work and resume prompts
must remain naturalistic and must not prescribe Volicord choreography or expose
the hidden decision oracle.

Full passage still requires three repository classes, two cycles per class,
and distinct work/resume sessions for every cycle: twelve distinct real VS Code
Codex sessions in total. Every required automatic, manual, resource, and
accessibility observation remains mandatory. Only the complete campaign may
set `campaign_complete`, `replacement_pass_candidate`, or `phase_9_ready` true.

A terminal failed work session may use the maintained `qualify-work-blocker`
path when its completed capture proves a machine-observable work-session
failure. That path records only a bounded blocker, leaves later sessions and
checks `not_run`, and cannot establish replacement passage.

## Remaining Phase 8 risks

- Whether agents independently resolve the repository-bound Project, Recall,
  investigate, promote a material Question, obtain an explicit user Decision,
  and create a grounded Checkpoint in naturalistic sessions.
- Whether Question relevance, Decision comprehension, interruption cost, and
  document fidelity/usefulness pass manual observation.
- Whether accessibility, latency, output size, peak memory, and sustained
  resource behavior pass across the complete campaign.

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
