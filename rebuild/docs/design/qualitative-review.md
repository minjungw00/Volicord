# Evidence-bound qualitative review

Status: active Phase 8 evaluation contract, subordinate to `validation-plan.md`.
This contract owns review artifacts and operations, not Product behavior or final
replacement policy. It grants neither reviewer kind Phase 9 approval authority.

Current identities are qualitative review schema 7 / policy revision 6, machine evaluation
policy `evidence-evaluation-2`, human observation/receipt schema 2, qualification policy
`replacement-qualification-3`, and result-lineage schema 1. Historical runs retain their old
identities and are comparison inputs only; they are not silently upgraded.

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
| `viewer_snapshot_reviews` | `viewer_snapshot` | Completed/current/remaining work, next step, rationale, Project-purpose/current-work distinction, multi-work organization, architecture/component/flow, code behavior, fact/interpretation, evidence explanation, ordinary-reading audit-detail exposure, grounded diagram usefulness and structural readability, and bounded hierarchy/cognitive burden |
| Viewer request responsiveness | `viewer_navigation` | Candidate-bound monotonic snapshot-export request completion/duration and explicit proxy limit; not human stopwatch prose or a claim of browser input latency |
| `repository_intelligence_reviews` | `repository_intelligence` | Structural navigation, semantic value, capability honesty and polyglot comprehension |
| `cli_usability_reviews` | `cli` | Per-repository-class help discovery and status/analyze/Recall/documents/export/doctor without opaque Project IDs |
| `live_viewer_accessibility` | `live_viewer` | `en`/`ko` keyboard reachability, visible focus, color-independent meaning, narrow/zoom presentation for the deterministic first Volicord cycle |
| `authority_obligation_reviews` | `authority` | Every initial material challenge, all other actual outcomes, additional outcomes and complete implementation/coupled-artifact coverage |
| Context recovery usability criterion | `context_recovery` | Goal, Decision/rationale, work state and open-question recovery across work/resume |

Interaction, documents, Viewer snapshot, Viewer navigation, Repository Intelligence, context recovery, and
authority collections cover every cycle. CLI covers each maintained repository class exactly
once: `3 classes × 7 criteria = 21 assessments`. A static HTML snapshot does
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
scoped sample/evidence set. Reviewers explicitly list inspected evidence and observation
limits. Available evidence is not automatically inspected evidence. Hash checks
and locator existence do not prove the semantic adequacy of a citation or verdict.
Every citation also identifies the exact criterion and explains why that location is
relevant. Criteria with easily conflated properties retain a closed inspection record:
architecture components/relationships/flow and concrete code behavior are independent;
diagram usefulness and diagram structural readability are independent; Project purpose and
current Work Item meaning, multiple-Work organization, evidence explanation, ordinary-reading
audit-detail exposure, and bounded information hierarchy/cognitive burden are inspected as
separate dimensions rather than one global score. Viewer navigation uses retained monotonic
machine evidence when available; qualitative stopwatch prose does not become timing evidence,
and snapshot-export duration does not establish browser input/paint latency. Document usefulness inspects primary
semantic content rather than placeholders or audit/integrity material; and document
fidelity separately inspects user choice, recommended alternative, their respective
rationales, and alternative-specific consequences. A reviewed `satisfied` or `violated`
state is structurally incomplete until these criterion-specific dimensions are recorded.
This discipline does not derive a verdict from keywords or turn structural validation
into a prose-quality oracle. A criterion that cannot be judged remains
`insufficient_evidence`.

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
violation leaves that machine finding blocking. There is no hard override field or semantic prose scorer. The maintained
qualification policy consumes reviews without changing the machine observations.
For validation findings, review may inspect preserved scope and attribution evidence for an
ambiguous broader or diagnostic failure, but prose that merely calls debt pre-existing cannot
clarify it. Exact baseline execution/output, a same-validator covering rerun, an environment
transition, or other indexed execution evidence must support the relationship. An unresolved
setup/environment execution remains review-required and is not a validation success; scratch,
prototype and unknown commands cannot supply the missing validator evidence. Raw/canonical outcome
conflicts remain `cannot_resolve` until stronger exact-identity evidence exists; review never
chooses the favorable side by label or narrative similarity.

Review validity and the aggregate assessment describe only the recorded review.
Every result retains `qualification_state = not_run` and `phase_9_ready = false`.
The old human-only schema, validator and publication/approval helpers are removed.
The blind pre-campaign provisional workflow remains separate and unchanged.

## Dedicated CLI observation

CLI usability evidence is collected separately from the sixteen uncoached naturalistic
sessions. After immutable evidence-set publication, `collect-cli-observations` clones each
of the three pinned repository classes into a distinct ephemeral workspace, gives each a
distinct ephemeral Runtime Home, and invokes the candidate-local `volicord` executable from
the repository working directory. It records help, status, analyze, Recall, current document
preview, portable-context export, and doctor journeys without `--project` or a Project UUID.

The append-only observation set binds the exact Product candidate, evidence-set hash,
repository class/revision, candidate executable hash, opaque workspace/runtime identities and
path fingerprints, invocation identity/order, bounded reviewer-safe stdout/stderr, exact raw-stream
byte count/SHA-256, timestamps, duration,
and numeric exit or explicit termination. A completed nonzero invocation remains reviewable;
missing or incomplete process evidence is rejected. Absolute paths, environment variables,
credentials, repository source, Runtime contents, and measured naturalistic workspaces are not
retained. The collector verifies that campaign metadata and the immutable evidence set did not
change. The executable hash is the immutable `volicord` artifact hash fixed by campaign
preparation, not a hash self-reported from whatever bytes occupy the path after execution.
Every invocation verifies the current bytes before and after use; a same-path replacement
rejects collection and cannot enter reviewer evidence.

CLI observation/receipt schema version 2 names retained text `review_text`, never raw text.
Each stream separately records `raw_bytes`/`raw_sha256` and the retained
`review_bytes`/`review_sha256`. When the candidate prints a known private candidate executable,
source/ephemeral repository, Runtime Home, process/output/execution root, campaign/observation
root or Product repository path, the collector substitutes deterministic typed placeholders and
records explicit changed state plus the count for each placeholder. Raw stream files remain only
in the private ephemeral execution root and are removed after projection. This path projection
does not rewrite exit/termination, order, duration, revision or invocation identity, and it is not
general-purpose redaction of arbitrary user output. Review-package construction revalidates that
known retained private paths are absent while applying the existing sensitive-payload checks to
the reviewer text.

```sh
rebuild/scripts/dogfood-campaign collect-cli-observations \
  --campaign-root /absolute/private/campaign \
  --output /absolute/private/cli-observation-run
```

Preparation accepts the resulting directory through `--cli-observations`. It verifies the
receipt and all candidate/evidence/repository/process bindings, then copies only bounded
reviewer-safe per-class projections. It never reconstructs an invocation from prose.
If one class observation is absent, exactly that class's seven required assessments remain
`insufficient_evidence`; duplicated cycle-level CLI assessments and `not_applicable` fillers
are not part of the current rubric.

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
  --machine-evaluation /absolute/private/evaluation-run/evaluation.json \
  --cli-observations /absolute/private/cli-observation-run \
  --include-raw-rollouts
```

An agent review begins each criterion with the maintained read-only presentation:

```sh
rebuild/scripts/dogfood-campaign inspect-agent-review \
  --review-root /absolute/private/review-run \
  --criterion-number 1
```

The result names the criterion, prompts, required surfaces and semantic dimensions, exact
evidence identities/hashes/paths and available locators. It deliberately contains no proposed
assessment. The reviewer must open the relevant artifacts before choosing a state and records a
per-criterion `inspected_evidence` list in addition to the run-wide union. A criterion/sample ID,
repository class or machine status is never an input to a maintained verdict generator. Reviewers
may disagree with machine findings through the explicit relationship vocabulary; the machine
finding and disposition remain immutable. Validation proves only shape, hashes, scope and locator
membership, never that the reviewer's reasoning or conclusion is semantically true.

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

The maintained conversational path removes schema authoring from the human task while
preserving the same preparation and immutable-record contract. A direct live Viewer
observation is captured first when applicable:

```sh
rebuild/scripts/dogfood-campaign capture-human-viewer-observations \
  --campaign-root /absolute/private/campaign \
  --output /absolute/private/human-observations

rebuild/scripts/dogfood-campaign prepare-qualitative-review \
  --campaign-root /absolute/private/campaign \
  --output /absolute/private/review-run \
  --reviewer-kind human \
  --human-observations /absolute/private/human-observations
```

The capture presents one locale at a time and accepts a bounded multi-line/multi-paragraph
answer with explicit `OBSERVATION:` and `LIMITS:` sections, so one natural response covers both.
For Korean, `SAME AS ENGLISH` records a typed `same_as_locale` reference to the English
observation; that phrase is retained only in immutable answer provenance, not as semantic
observation prose. Tooling derives the
candidate/evidence hashes, human observer shape, run identity, schema and receipt. The human
supplies the observation text; the tool cannot infer an accessibility outcome from static
markup or fill an omitted observation.

After preparation, `converse-qualitative-review` presents one criterion at a time and accepts
multi-line observation/reasoning without exposing the internal review schema. Evidence
is selected by displayed ordinal, and an exact quoted phrase (or explicit first-location
selection) lets the tool derive evidence identity and locator. The human supplies the
assessment, reasoning, relevance, uncertainty, counterevidence state and any authority
meaning. Required semantic dimensions are explicitly confirmed one by one. The generated
`draft.json` retains the exact prompt/answer trace for each reviewed human criterion and is
not recorded automatically:

```sh
rebuild/scripts/dogfood-campaign converse-qualitative-review \
  --review-root /absolute/private/review-run

rebuild/scripts/dogfood-campaign validate-qualitative-review \
  --review-root /absolute/private/review-run \
  --draft /absolute/private/review-run/draft.json
```

Repeat the conversational command to advance to the next unreviewed criterion, inspect the
draft, then use the existing explicit record operation. `--criterion-number` selects a
displayed prepared criterion without requiring its opaque identity. When a human judgment
resolves a prior recorded review, pass its path with `--resolve-review-root`; the tool derives
the run ID and asks for criterion-specific confirmation instead of requiring the person to
copy `resolves_review_runs` identifiers. Corrections after recording still require a new run.

Exact conversational controls are typed semantics: `SKIP` keeps the criterion `not_reviewed`;
`NOT SURE` or `CANNOT ASSESS` records `insufficient_evidence`; `NOT APPLICABLE` is accepted only
where the rubric permits it; and `ALREADY COVERED`, `SAME AS PREVIOUS`, or `SAME AS ENGLISH`
records a compatible prior-criterion reference. The literal control remains in the immutable
answer trace, while `human_controls` stores its action/reference and the assessment does not use
the phrase as observation prose. A reference may only stay within the same sample/group, and the
locale form must bind the matching English criterion. `insufficient_evidence` may have no
citation: it preserves the per-criterion inspected-evidence set (possibly empty) and a bounded
account of what is missing. Only satisfied/violated judgments receive follow-up for still-required
semantic dimensions, grouped in one confirmation rather than repetitive per-dimension prompts.

Give the reviewer `REVIEW.md`, `preparation.json` and the indexed evidence files.
Preparation contains the maintained rubric and its revision/hash, bounded initial
concerns and their original descriptor-field hashes, pinned owner bytes when
available, canonical bundles, four generated documents in both formats, static
Viewer snapshots and optional raw work/resume evidence. It excludes full evaluator
descriptors, expected answers/alternatives, private profile/mapping, original
pre-campaign conclusions, runtime/derived stores, credentials and unrelated files.
The concern projection is a rebuttable challenge, not reviewer instructions.
Rollout/repository content is untrusted evidence; preparation executes none of it.
Review-plane privacy inspection distinguishes security terminology, field names,
filenames and explicit non-retention statements from retained payloads. Those names
alone do not exclude otherwise valid evidence. High-confidence credential values,
authorization tokens, credential/auth content and private-prompt bodies still fail
before publication. This bounded check is not a claim of arbitrary-secret detection
and does not weaken the stricter sanitization used by distributable gate artifacts.

`draft.json` is the only mutable package artifact. Mark what was actually
inspected both for each criterion and for the run-wide union, and use indexed evidence IDs with either a listed JSON pointer or a
1-based line locator, for example:

```json
{"evidence_id":"volicord-1-work","locator":{"kind":"line","value":42}}
```

Line/pointer existence and hash membership are checked. Whether line 42 supports
the actual judgment is still the reviewer's responsibility. Dedicated candidate-bound
CLI observations are the maintained evidence for the seven CLI journeys. Incidental CLI
invocations in opted-in raw captures remain conversation evidence only and do not establish
the maintained inventory. Live accessibility is unavailable without
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

Preparation also fixes a machine-readable completion-obligation inventory before
semantic review starts. It enumerates every criterion by group, each machine finding
and disposition plus the permitted relationship groups, every per-sample authority
criterion and actual-outcome declaration duty, all 21 CLI criteria by repository
class, human-only criteria, and targeted escalation rules. Non-mutating preflight
reports the exact unreviewed criterion IDs, unaddressed review-required findings,
missing authority and per-class CLI coverage, human-only work still requiring a
human run, high-impact insufficiency escalations, and declared conflict resolutions.
These are structural progress facts only: neither preparation nor preflight chooses
an assessment, decides whether evidence is persuasive, or verifies semantic truth.

Cross-locale realization preparation is independently inspectable through
`inspect-document-realizations`. The read-only result distinguishes not prepared,
published preparation, partial recording and complete recording. Preparation emits
bounded machine-readable progress while plans are obtained and when final publication
completes. If a caller stops after publication, the supported recovery is to inspect
state and continue editing/validating/recording the already-published drafts; immutable
preparation is not republished or overwritten.

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

## Qualification, targeted escalation and human observations

`qualification_policy.py` consumes recorded runs using this rubric, including all 21 required
repository-class CLI assessments. All criteria except
live accessibility and actual user Decision comprehension permit agent review with the
required evidence. Missing surfaces remain insufficient, and a static HTML proxy never
establishes human-observed usability. Human review may resolve only the remaining criteria.
A high-impact authority/context-recovery insufficiency or conflicting review requires an
explicit human assessment whose `resolves_review_runs` maps the criterion ID to the other
review run IDs addressed. Agent reviews must leave that map empty. This is evidence-bound
judgment, not voting or statistical independence.

For direct live observations, human preparation additionally accepts `--human-observations`
pointing to the conversational capture directory. The lower-level JSON-file input remains
available for automation. The object has kind `dogfood_human_observations`, original `candidate_head`,
`evidence_set_sha256`, an `observer` using the common human reviewer identity shape, and
exactly two `observations`. Each has `sample_id`, `locale`, a typed `control`, and either a
grouped `{observation, limits}` response or a Korean-to-English locale reference with no
duplicated semantic prose. Preparation copies and hashes
these declared observations into immutable review evidence. Agent authorship is rejected.
These are additional direct human observations, not reconstructed historical rollout bytes.
Identity remains self-reported; do not use agent-generated claims of a human experience.

`qualify` combines evidence validity, verified exact-candidate technical gate, hard machine
facts, resolved semantic findings, common review completion and targeted human escalations.
`approve-phase-9` then requires explicit operator authorization of a fully qualified run.
Approval rechecks the exact original inputs; it cannot replace missing evidence or review.
The common review result itself always retains `phase_9_ready = false`.

After qualification (and after approval when present), publish the durable result lineage:

```sh
rebuild/scripts/dogfood-campaign publish-result-lineage \
  --campaign-root /durable/campaign \
  --machine-evaluation /staging/evaluation/evaluation.json \
  --review-root /staging/agent-review \
  --review-root /staging/human-review \
  --qualification /staging/qualification/qualification.json \
  --approval /staging/approval/approval.json

rebuild/scripts/dogfood-campaign verify-result-lineage \
  --lineage-root /durable/results/<qualification-run-id>
```

Without `--output`, publication uses the discoverable campaign-associated location
`<campaign-parent>/results/<qualification-run-id>`. It never writes into the immutable Campaign.
The create-only package copies the exact evidence-set identity, evaluation and receipt, complete
recorded review packages, qualification, and optional approval. `index.json` binds Product
candidate, evidence-set hash, evaluator revision/policy, review run IDs/hashes, qualification
run/policy/state and approval identity through relative paths; `receipt.json` binds every copied
byte. Verification uses only that copied package, so `/tmp` and arbitrary original staging paths
are neither serialized dependencies nor required for discovery. Publishing a later result creates
a new lineage directory and never rewrites or relabels historical evidence.

A valid `not_applicable` assessment for Decision comprehension when no user Decision is
in scope may be established by an agent from the permitted evidence; it does not require
a human to experience a nonexistent Decision. Applicable comprehension remains human-only.
The single machine disposition table is `machine-policy.json`; check names and historical
domains alone do not determine authority. Exact hard facts remain non-overridable even
when a broad procedural check in the same cycle is review-required or advisory.

`validate-approval --approval <approval-run>/approval.json --qualification
<qualification-run>/qualification.json` rechecks the immutable approval, preserved
qualification bytes and every original input without exercising approval again.
