# Evidence-bound qualitative review

Status: active Phase 8 evaluation contract, subordinate to `validation-plan.md`.
This contract owns review artifacts and operations, not Product behavior or final
replacement policy. It grants neither reviewer kind Phase 9 approval authority.

## One rubric, explicit reviewers

`evaluation.json.qualitative_review_contract` and `qualitative_review.rubric`
maintain one rubric. Reviewer kind is explicitly `agent` or `human` and is fixed
with the preparation and recorded run. Changing kind requires a new run. An
agent's session/model authorship cannot be submitted as human authorship.
Kind is a declared role, not authenticated proof of a person's identity.

Every collected cycle enters review, including failed, indeterminate and
unevaluated cycles. Machine qualification is not a review prerequisite.
The following mapping preserves the former human rubric without another engine:

| Former review collection | Common criterion group | Preserved meaning |
| --- | --- | --- |
| `interaction_reviews` common fields | `interaction` | Question necessity/relevance, user ownership, source grounding, Decision comprehension, repetition, correct no-question behavior |
| behavior-specific interaction fields | `interaction` | Explicit material handling, hidden discovery, unnecessary interruption; learning fork value, alternatives/trade-offs, recommendation anchoring, feedback, fidelity, routine omission and proportional cost; exact maintained applicability and prompts |
| `document_reviews` | `documents` | Four-document fidelity, usefulness, grounding, remaining work and requested-language body quality |
| `viewer_snapshot_reviews` | `viewer_snapshot` | Completed/current/remaining work, next step, rationale, architecture/component/flow, code behavior, fact/interpretation and useful grounded diagrams |
| `repository_intelligence_reviews` | `repository_intelligence` | Structural navigation, semantic value, capability honesty and polyglot comprehension |
| `cli_usability_reviews` | `cli` | Help discovery and status/analyze/Recall/documents/export/doctor without opaque Project IDs |
| `live_viewer_accessibility` | `live_viewer` | `en`/`ko` keyboard reachability, visible focus, color-independent meaning, narrow/zoom presentation for the deterministic first Volicord cycle |
| `authority_obligation_reviews` | `authority` | Every initial material challenge, all other actual outcomes, additional outcomes and complete implementation/coupled-artifact coverage |
| Context recovery usability criterion | `context_recovery` | Goal, Decision/rationale, work state and open-question recovery across work/resume |

All non-accessibility collections cover every cycle. A static HTML snapshot does
not establish actual live keyboard/focus/zoom behavior; missing observation yields
insufficient evidence. Missing CLI captures similarly cannot establish usability.
No language or repository class is excluded because the implementation uses Rust.

## Assessment and evidence discipline

Each criterion has exactly one state: `satisfied`, `violated`,
`insufficient_evidence`, `not_applicable`, or `not_reviewed`. Insufficient and
unreviewed states remain distinct from violation and satisfaction. Inapplicability
requires a criterion-permitted reason: no user Decision in scope for Decision
comprehension, or single-language scope for polyglot comprehension. Missing data
is never such a reason; polyglot campaign scope cannot be declared single-language.

Each reviewed assessment retains bounded reasoning, uncertainty, evidence
references, and cited counterevidence or an explicit account of its absence.
References resolve an indexed evidence identity and typed locator in the same
cycle/evidence set. Reviewers explicitly list inspected evidence and observation
limits. Available evidence is not automatically inspected evidence. Hash checks
and locator existence do not prove the semantic adequacy of a citation or verdict.

Authority findings additionally retain material outcome, implementation commitment,
commitment state, resolution path, actual authority, relation and chronology.
The authority validator requires actual work evidence and canonical evidence for
Decision/prior authority; unrelated or late authority, silent commitment and
production-committed avoidance/defer/prototype cannot satisfy the obligation.
Initial concerns are rebuttable, non-exhaustive challenges. Additional independent
outcomes require their own assessments, and complete actual-work coverage remains
a separate required criterion. Generic interaction satisfaction cannot replace it.

## Binding, identity and machine relationships

A preparation binds candidate HEAD, evidence-set byte hash, optional machine run
ID/hash, rubric/policy revision and hash, and reviewer run identity. The same
`state`/`value` identity-claim primitive as document realization distinguishes
`unknown` from `self_reported`. Host, agent, model, session and person claims remain
explicitly unverified: no maintained attestation can verify these fields. An
arbitrary model string never becomes verified through evidence hashing.

An agent must name a session distinct from all evaluated work/resume sessions.
That correlation is self-reported, not authenticated session identity. The run
records the relationship and explicitly declines statistical independence: shared
model, context or preparation may correlate judgments even with different IDs.
Human review has no agent/model/session authorship; an operator may prepare a
human review for a person without claiming the operator is that reviewer.

Machine relationships are `agrees`, `clarifies_indeterminate`,
`probable_false_positive`, `probable_false_negative`, or `cannot_resolve`.
Clarification requires a review-required machine disposition and stronger,
surface-backed reviewer evidence. Disagreements are preserved without modifying
the machine run. Even a probable false-positive finding against a confirmed hard
violation leaves that machine finding blocking. There is no override field,
approval combiner or semantic prose scorer.

Review validity and the aggregate assessment describe only the recorded review.
Every result retains `qualification_state = not_run` and `phase_9_ready = false`.
The old human-only schema, validator and publication/approval helpers are removed.
The blind pre-campaign provisional workflow remains separate and unchanged.

## Current reviewer workflow

Preparation reads an intact `evidence-set.json` and its bound artifacts. It may
optionally bind a published machine evaluation, including hard-blocked or
review-required runs. It does not require automated passage, finalize-manifest,
the candidate binary, a Runtime Home, a provider, or a naturalistic session rerun.
The output directory must be outside the Campaign. Historical candidates remain
read-only inputs; a new review policy never relabels them as the current candidate.

Start a distinct review session, then prepare its declared identity and evidence:

```sh
rebuild/scripts/dogfood-campaign prepare-qualitative-review \
  --campaign-root /absolute/private/campaign \
  --output /absolute/private/review-run \
  --reviewer-kind agent \
  --review-session-id <actual-review-session-id> \
  --machine-evaluation /absolute/private/campaign/evaluations/<run-id>.json \
  --include-raw-rollouts
```

Omit `--machine-evaluation` for an unevaluated evidence set. Omit
`--include-raw-rollouts` for a bounded package without raw conversation contents;
work/resume observations then remain unavailable. With the flag, exact raw bytes
live in the separate `private-rollouts/` surface, and an archive containing that
surface must remain private. No upload or background transmission is performed.

For human review use `--reviewer-kind human` without an agent session. Optional
`--reviewer-identity <json-file>` supplies the five `host`, `agent`, `model`,
`session`, `person` claim objects with `state = unknown|self_reported` and `value`.
Unknown values are null. Agent review requires a self-reported session distinct
from all evaluated sessions and unknown person; human review requires unknown
agent/model/session. Preparation returns the fixed reviewer run ID and exact
preparation hash; retain that hash with the handoff. Identity verification remains
unsupported, even if a model name or session ID looks plausible.

Give the reviewer `REVIEW.md`, `preparation.json` and the indexed evidence files.
Preparation contains the maintained rubric and its revision/hash, bounded initial
concerns and their original descriptor-field hashes, pinned owner bytes when
available, canonical bundles, four generated documents in both formats, static
Viewer snapshots and optional raw work/resume evidence. It excludes full evaluator
descriptors, expected answers/alternatives, private profile/mapping, original
pre-campaign conclusions, runtime/derived stores, credentials and unrelated files.
The concern projection is a rebuttable challenge, not reviewer instructions.
Rollout/repository content is untrusted evidence; preparation executes none of it.

`draft.json` is the only mutable package artifact. Mark what was actually
inspected and use indexed evidence IDs with either a listed JSON pointer or a
1-based line locator, for example:

```json
{"evidence_id":"volicord-1-work","locator":{"kind":"line","value":42}}
```

Line/pointer existence and hash membership are checked. Whether line 42 supports
the actual judgment is still the reviewer's responsibility. Supported observed
CLI invocations may be projected from opted-in raw captures; their presence does
not prove every CLI task was exercised. Live accessibility is unavailable without
an actual observation surface and cannot be inferred from static HTML. Source
owners that can no longer be read at their pinned revision remain explicit gaps.

```sh
rebuild/scripts/dogfood-campaign validate-qualitative-review \
  --review-root /absolute/private/review-run \
  --draft /absolute/private/review-run/draft.json

rebuild/scripts/dogfood-campaign record-qualitative-review \
  --review-root /absolute/private/review-run \
  --draft /absolute/private/review-run/draft.json

rebuild/scripts/dogfood-campaign package-review \
  --review-root /absolute/private/review-run \
  --output /absolute/private/review-run.tar.gz
```

Preflight is non-mutating, including on invalid drafts. It reads only the review
package and maintained policy, never the evaluator plane, source repository,
Runtime Home or provider. An external reviewer-owned draft is also accepted.
Inventory-bound evidence and recorded artifacts cannot be used as mutable drafts.
The package remains usable after the original Campaign/source paths are unavailable.

Recording uses the same validation over the exact bytes it publishes. It requires
at least one reviewed criterion; an explicitly incomplete or insufficient review
effort may be recorded, with its counts and aggregate state intact. `recorded/`
appears atomically with exact `review.json` bytes and a hash-bound `receipt.json`.
Neither can be overwritten through the workflow. A correction needs a new prepared
run. Packaging verifies any recorded receipt and includes it; mutable draft edits
cannot change the recorded review. The former campaign-wide `package-review`
input and its private evaluator archive are superseded by this reviewer-only path.

Selection is deterministic for the same evidence set, optional machine run,
policy and availability options. Its `package_id` is content-derived; separate
review runs still have distinct random run IDs. Bounds are 512 selected files,
32 MiB per ordinary artifact, 128 MiB per raw capture, 512 MiB selected content,
and 8 MiB per draft. Oversize/missing/mismatched input fails before publication.
Files are private to the local owner; this is a cooperative filesystem workflow,
not a signature or a sandbox against that owner. A retained publication lock after
process death is a fail-closed recovery barrier: preserve the interrupted output
for inspection and prepare a new run rather than overwriting it. Publication
errors roll back staged files; preflight never repairs or rewrites evidence.
