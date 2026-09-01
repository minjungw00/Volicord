# Phase 8 technical-entry candidate authority

- Replacement passage: `not_established`
- `phase_9_ready = false`
- Historical sealed pre-Dogfood production/test candidate:
  `6031641c46cf014a754442dcee3137caf265882e`
- Historical candidate's technical Phase 8 entry gate: `passed`;
  `phase_8_ready = true` only for that exact candidate HEAD
- Admission: `eligible`; exact final: `succeeded` with four commands and zero
  failures; official V11: `passed` with 54 of 54 required steps
- Live production-provider qualification: `passed`; provider `openai-codex`,
  model `gpt-5.6-sol`
- Credential-retention audit: `passed`; all recorded counts are zero
- Accepted-Decision revisit-trigger assessment: completed; no active Q1–Q14
  trigger reported
- Sanitized gate capsule: `verified`; SHA-256
  `a657b8c517a3135c8b64ce5b75ae98ef66a0e5f43573ef2c092e590e3eba7695`
- Sanitized evidence archive: `verified`; SHA-256
  `bed36fa89e82a60e198dd3dc3ef5b8864e9ce27332a8a60461f230a4f74adc0e`
- Naturalistic Dogfood for this candidate: `not_run`
- Automated Dogfood qualification: `not_run`
- Campaign-level human review: `not_provided`
- `replacement_pass_candidate = false`

## Maintained conclusion

Candidate `6031641c46cf014a754442dcee3137caf265882e` historically passed the
technical-entry gate and was eligible to begin a candidate-bound naturalistic
Dogfood campaign at that exact HEAD. Admission passed with a clean, unchanged
worktree; the exact final succeeded with all four commands, zero failures, and
a warning-clean clippy result; the separately authorized live production-provider
qualification passed; and the same-session official V11 passed all 54 required
steps. All
three authenticated Codex targets passed, the credential-retention audit passed
with zero recorded findings or scan errors, official V11 reported no active
accepted-Decision revisit trigger, and the sanitized archive was independently
verified. The capsule, archive, final, provider, and V11 evidence identify the
same candidate and gate invocation.

That capsule and archive remain historical evidence only. A later HEAD never
inherits technical-entry or Dogfood eligibility from an earlier successful
gate. Technical eligibility is authoritative only for the exact candidate HEAD
identified by a successful maintained gate and its verified capsule/evidence
archive; a failed or blocked gate establishes no eligibility for its candidate.

This technical entry does not qualify naturalistic Dogfood or replacement
passage. Naturalistic Dogfood is `not_run`. Automated qualification has not run
and human review has not been provided, so `replacement_gate = pending`,
`replacement_pass_candidate = false`, and `phase_9_ready = false` remain
unchanged for the historical candidate. The failed naturalistic campaign
recorded in `rebuild/validation/dogfood/report.md`, including its descriptors,
captures, Runtime Homes, workspaces, bundles, observations, and session
identities, remains diagnostic-only. It cannot be repaired, continued, or
reused for this or any future candidate.

The current human-facing surface gives generated documents a
comprehension-first body and keeps inspectable grounding and audit detail in a
separate trailing appendix or closed HTML disclosure. The live Viewer uses the
same human-first hierarchy. A static Viewer snapshot is a distinct,
self-contained, read-only share and review artifact; it is not interchangeable
with the four generated documents or their adoption lifecycle.

## Candidate-bound entry boundary

Any future campaign must begin from zero in a separate clean worktree whose
actual Git `HEAD` exactly matches the newly sealed candidate identified by its
own successful maintained gate and verified capsule. Supplying the historical
candidate `6031641c46cf014a754442dcee3137caf265882e`, or any other candidate
argument, cannot qualify a different worktree HEAD. The fresh campaign must
retain the maintained three-class, eight-cycle/sixteen-session reviewer-blind behavior-profile, distinct work/resume-session, automated,
replacement-required human-review, resource, and accessibility qualification contract.
The campaign worktree itself must be the sealed candidate; a different
support-branch HEAD cannot qualify by supplying only a candidate argument.
Every campaign helper transition that mutates candidate-bound review,
activation, collection, manifest, package, or human-review state must reject a
different current HEAD or dirty qualifying worktree before mutation. There is
no superseded-campaign reprocessing exception; immutable predecessor evidence
remains available only to read-only diagnostic helpers.

This table is a human-readable projection of the public operating contract in
`rebuild/validation/dogfood/evaluation.json`; it does not own a separate
campaign definition.

<!-- phase8-public-campaign-contract:start -->
| Public campaign field | Current requirement |
| --- | --- |
| `qualification_cycles` | `8` |
| `sessions_per_cycle` | `2` |
| `fresh_sessions` | `16` |
| `repository_cycles` | `volicord=3, small-python=3, polyglot-medium=2` |
| `provisional_reviews_before_reveal` | `8` |
| `sealed_descriptors_and_reviews` | `8` |
| `complete_batch_raw_rollouts` | `16` |
<!-- phase8-public-campaign-contract:end -->

Use `rebuild/scripts/dogfood-campaign` for routine campaign preparation and
evidence handling. The evaluator/control agent researches the repositories,
creates all eight bounded blind-first reviewer preparations, records every provisional
classification and materiality conclusion before exposing any evaluator basis,
uses the hash-bound `reviewer/provisional-review-contract.json` and non-mutating
`validate-provisional-review` operation to apply the recorder's reviewer-visible
semantics before submission, fixes them through the opaque-slot
`record-provisional-review` operation, verifies
`provisional_count = 8`, reveals and validates the private qualification profile, and then
seals each descriptor against its immutable review without exposing evaluator
material to the operator. Recording validates only reviewer-visible identity,
schema, provenance bounds, and self-consistency derived from the reviewer's own
classification; a well-formed evaluator disagreement receives the same successful
`provisional_recorded` transition. After reveal, a structured comparison must mark
matching conclusions `agreed`, resolve every classification/materiality/disclosure
difference from inspectable evidence, or block sealing as `unresolved_conflict`.
The original provisional bytes and hash are never rewritten by that comparison.
Repository and SessionStart
hook trust remain explicit user actions. `activate-all` verifies the owned static manifest, MCP entry,
SessionStart hook and exact candidate-local executable/Runtime binding, but this does not prove VS Code
executed SessionStart. If setup is uncertain, the operator inspects it before sending a frozen task;
runtime SessionStart evidence remains required for every capture. The operator then runs all sixteen
fresh naturalistic VS Code Codex chats using only the frozen tasks, answers
only genuine material Questions, preserves every raw rollout without
per-session evidence-processing interruptions, and provides the sixteen files
once for batch ingestion. The helper maps the cycles and automatically derives
canonical bundles, bounded Runtime/activation summaries, all four document
kinds in Markdown and self-contained HTML, and read-only static Viewer
snapshots.

Automated Dogfood can complete without a human review. Campaign-level human
review is prepared and recorded separately against the immutable
automated result. Its absence leaves replacement qualification pending rather
than failing automated Dogfood, and a human pass cannot override any machine
failure. Ordinary independent review uses the byte-exact raw rollout archive
plus the bounded review package, not a full Runtime Home.

The candidate-specific gate capsule and verified evidence archive, rather than
this tracked summary, own the technical-entry result. This summary therefore
does not require a post-gate commit to name a new current candidate: any later
tracked commit would create a different HEAD and cannot inherit the capsule's
authority.

## Remaining Phase 8 risks

- Naturalistic selection of the appropriate Question/no-question, research,
  delegated-choice, prototype, or defer outcome, plus explicit user-owned
  Decision provenance only when required.
- Local repository-analysis behavior in actual Codex risk handling,
  trustworthy numeric Checkpoint evidence, and repository resolution and
  Recall before continuation in fresh sessions.
- Question relevance, Decision comprehension, interruption cost, generated
  document readability, and static Viewer usability under real work.
- Live Viewer accessibility and bounded real-page behavior, plus repeated
  resource qualification across the complete campaign.

The current-task delegation evaluator now consumes the production typed,
verbatim evidence semantics without imposing a research requirement and keeps
the Inquiry-time delegation Decision path separate. This alignment does not
change provider transport, Repository Intelligence parsers, Viewer rendering,
or CLI taxonomy. It does not establish a replacement-gate result or authorize
Phase 9.

## Maintained references

- `rebuild/validation/end-to-end/multi-repository/report.md`
- `rebuild/validation/phase-7-summary.md`
- `rebuild/validation/README.md`
- `rebuild/docs/design/open-decisions.md`
- `rebuild/docs/design/acceptance-scenarios.md`
- `rebuild/docs/design/validation-plan.md`
- `rebuild/docs/design/cutover-plan.md`
