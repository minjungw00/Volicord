# Evidence-bound qualitative review

Status: active Phase 8 evaluation contract, subordinate to `validation-plan.md`.
This contract owns review artifacts and operations, not Product behavior or final
replacement policy. It grants neither reviewer kind Phase 9 approval authority.

Current identities are qualitative review schema 13 / policy revision 12, machine evaluation
policy `evidence-evaluation-6`, human observation/receipt schema 3, qualification policy
`replacement-qualification-8`, and result-lineage schema 1. Historical runs retain their old
identities and are comparison inputs only; they are not silently upgraded.

## One rubric, explicit reviewers

`evaluation.json.qualitative_review_contract.common_criteria` owns the one criterion
inventory. `qualitative_review.rubric` loads that inventory without maintaining a
second copy and adds the criterion prompts, semantic dimensions and structural rules
that make it executable. The JSON workflow declaration and
`review_operations.workflow_contract()` are mechanically compared by the maintained
dogfood assertions, including the read-only `inspect-agent-review` operation. Reviewer
kind is explicitly `agent` or `human` and is fixed
with the preparation and recorded run. Changing kind requires a new run. An
agent's session/model authorship cannot be submitted as human authorship.
Kind is a declared role, not authenticated proof of a person's identity.

Every collected Work enters review, including failed, indeterminate and
unevaluated Works. Machine qualification is not a review prerequisite.
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
| `live_viewer_accessibility` | `live_viewer` | `en`/`ko` keyboard reachability, visible focus, color-independent meaning, narrow/zoom presentation for the Volicord journey-final Viewer |
| Live browser responsiveness | `live_viewer` | Direct human observation of input response and resulting paint in `en`/`ko`; snapshot-export request timing is not a substitute |
| `authority_obligation_reviews` | `authority` | Every initial material challenge, all other actual outcomes, additional outcomes and complete implementation/coupled-artifact coverage |
| Context recovery usability criterion | `context_recovery` | Goal, Decision/rationale, work state and open-question recovery across work/resume |

Interaction and authority collections cover every Work. Context recovery covers the three Work A
resume pairs. Documents, Viewer snapshot,
Viewer navigation, and Repository Intelligence use the three journey-final projections while retaining
the represented Work identities. CLI covers each maintained repository class exactly
once: `3 classes × 7 criteria = 21 assessments`. A static HTML snapshot does
not establish actual live keyboard/focus/zoom behavior; missing observation yields
insufficient evidence. Missing CLI captures similarly cannot establish usability.
No language or repository class is excluded because the implementation uses Rust.
The Volicord journey's one-Project/three-Work continuity is a structural machine
finding. Human comprehension of that organization is the Volicord journey-final
Viewer `multiple_work_organization` criterion; there is no parallel long-lived
observation group.

## Assessment and evidence discipline

Each criterion has exactly one state: `satisfied`, `violated`,
`insufficient_evidence`, `not_observed`, `not_applicable`, or
`not_reviewed`. `not_observed` means an optional naturalistic opportunity
did not occur. It is recorded separately from both pass and violation.
`insufficient_evidence` means the opportunity may matter but evidence cannot
support a judgment. Unreviewed means no judgment was recorded. Inapplicability
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

Authority findings retain actual material outcome, implementation commitment,
commitment state, resolution path, authority relation and chronology. The
authority validator requires actual Work evidence and canonical evidence for
Decision or prior authority. Unrelated or late authority, silent commitment and
production-committed avoidance/defer/prototype cannot satisfy a discovered
outcome. Additional independent outcomes require their own assessments;
complete actual-work coverage remains a separate required criterion. Generic interaction satisfaction cannot replace it.

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
Current campaign preparation freezes five ordinary Works and eight task bytes before
activation. Post-hoc review consumes observed Work and journey evidence without any
pre-execution semantic profile or provisional classification.

## Dedicated CLI observation

CLI usability evidence is collected separately from the eight uncoached naturalistic
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
`insufficient_evidence`; duplicated Work-level CLI assessments and `not_applicable` fillers
are not part of the current rubric.

## Current reviewer workflow

Preparation reads an intact `evidence-set.json` and its bound artifacts. It may
optionally bind a published machine evaluation, including hard-blocked or
review-required runs. It does not require automated passage, finalize-manifest,
the candidate binary, a Runtime Home, a provider, or a naturalistic session rerun.
The output directory must be outside the Campaign. Historical candidates remain
read-only inputs; a new review policy never relabels them as the current candidate.
An optional machine run is validated against the current machine schema/finding semantics
and its intact content-derived run ID and publication receipt, while retaining its recorded
policy identity. Review binding explicitly states `recorded_identity_not_current_equivalence`;
a changed review rubric does not rewrite or relabel that machine run. Qualification continues
to require exact current policy equality. There is no numeric-version compatibility branch.

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
`--include-raw-rollouts` to leave Work/resume observations unavailable. With the flag,
immutable raw rollouts are projection inputs; the package contains bounded reviewer-safe
`work_capture` / `resume_capture` JSON artifacts under `evidence/`, never byte-identical
raw JSONL. Review packages still require private handling. No upload or background
transmission is performed.

### Naturalistic review-capture projection

`review_captures.py` owns the one current `naturalistic_review_capture` schema 1 /
`naturalistic-review-capture-1` policy. Preparation verifies source bytes/SHA-256 against
both the immutable evidence-set member and session binding before parsing those exact
bytes through the shared Codex normalizer. It rechecks immutable Campaign bindings before
publication. Unsupported/malformed conversation transports, conflicting message copies,
source mismatch, inconsistent projections and retained sensitive payload fail closed.
The immutable Campaign and Product candidate identity remain unchanged.

Each capture binds `origin.kind/path/raw_bytes/raw_sha256`, Codex `session_id`, `role`
(`start` or `resume`), `candidate_head` and `evidence_set_sha256`. Ordered `records` retain
source sequence(s), turn/message/user-turn/call identity as applicable, semantic role,
operation and outcome. The evidence index separately binds whole-artifact
`projection.review_bytes/review_sha256`, current schema/policy, limits and counts;
`origin.raw_sha256` never identifies projected bytes. The artifact cannot include its own
whole-artifact hash; that binding lives in the index and package inventory. Source-independent
package validation rechecks artifact/index agreement and projection consistency; provenance
hashes are integrity evidence, not authenticated capture authorship.

The positive allowlist retains normalized actual user turns, user-visible agent messages,
async Question titles/options, Volicord operation identity/sequence/outcome with scalar
identity/action/state fields, turn start/terminal/interruption and compaction boundaries,
normalized execution command-role/digest/exit/termination facts with explicit
`output_retention = non_semantic_by_design`, and typed transport issues. Duplicate agent
item/response transports must agree and retain their original source coordinates. Operation
prose is inspected in actual conversation and the separately selected canonical bundle.
System/developer/skill/plugin instructions, reasoning, environment bodies, arbitrary
repository source, command stdout/stderr and generic tool/MCP payloads are excluded.
No repository name, credential spelling or test-token convention is allowlisted.

Current `item_completed.AgentMessage` content uses `Text` parts with string `text`
bodies. Projection preserves these bodies and checks agreement with duplicate
`response_item` messages; unsupported content still fails closed.
Host-owned daemon recovery context is excluded only with its exact
`daemon_recovery.internal_context` content-kind metadata, known turn identity and
`codex_internal_context` / `daemon_recovery` wrapper. It is not an actual user turn;
unbound user prose or ambiguous recovery transports still fail closed.

Text proposed for retention passes the unchanged review-plane sensitive-payload policy.
An unsafe body is wholly omitted with `body.state = omitted`, reason `sensitive_payload`,
source body bytes/SHA-256 and immutable coordinates, without retaining its value. A body
above the 1 MiB bound is similarly omitted with `body_limit`. `source_body_encoding`
distinguishes exact normalized user-turn `utf8_text` from `selected_json` encoding of
selected agent/Question/operation structures. Body digests never identify entire raw events.
The source limit is 64 MiB / 200,000 events and the projected artifact limit is 32 MiB.

`retained_record_count` counts normalized records with their selected bodies/facts retained;
`omitted_record_count` counts omitted selected bodies plus excluded raw records.
`excluded_records` lists unused raw event sequences with `non_semantic_by_design`.
Selected operation/execution records also exclude all unallowlisted payload fields by design.
`semantic_omission_count` counts omitted required user, agent, async Question or selected
Volicord operation identity/action/state bodies;
`non_semantic_omission_count` accounts for the remaining omissions. `semantic_complete = true`
means all required user/agent/Question bodies and selected operation fields were retained,
not that the whole
rollout was copied. Non-semantic exclusion alone leaves it true. Limits and omissions are
available in the capture, evidence index, inspection presentation and Work availability view.

A decisive `interaction_coverage_adequacy = satisfied|violated` requires inspection of every
frozen task and every required semantically complete Work/resume projection. All other
criteria whose required surfaces include Work/resume (interaction, authority, context recovery
and Repository Intelligence) likewise require every applicable required capture to be complete;
a journey-scoped criterion cannot inspect only a favorable Work. No current alternative path
supplies omitted actual interaction. Privacy or size omissions require `insufficient_evidence`
with explicit inspected omission/availability evidence and limits, never automatic Product
failure or silent satisfaction. This changes structural admissibility, not semantic scoring.

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
the phrase as observation prose. `ALREADY COVERED` and `SAME AS PREVIOUS` reuse only the prior
criterion's inspected observation/evidence context; the reviewer must still supply the current
criterion's verdict, reasoning, relevance, uncertainty, criterion-specific dimensions,
applicability and counterevidence conclusion. They never clone a semantic judgment. A reference
may only stay within the same sample/group. `SAME AS ENGLISH` is the sole stronger reuse rule: it
must bind the matching English criterion name and records `exact_semantic_judgment` provenance
while rebinding the citation to the corresponding locale observation. `insufficient_evidence` may have no
citation: it preserves the per-criterion inspected-evidence set (possibly empty) and a bounded
account of what is missing. Only satisfied/violated judgments receive follow-up for still-required
semantic dimensions, grouped in one confirmation rather than repetitive per-dimension prompts.

Give the reviewer `REVIEW.md`, `preparation.json` and the indexed evidence files.
Preparation contains the maintained rubric and its revision/hash, bounded initial
concerns and their original descriptor-field hashes, pinned owner bytes when
available, canonical bundles, four generated documents in both formats, static
Viewer snapshots and optional raw work/resume evidence. It excludes full evaluator
descriptors, expected answers/alternatives, original
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
live accessibility, browser input/paint responsiveness, Volicord journey-final Viewer
multi-Work comprehension and actual user Decision comprehension
permit agent review with the
required evidence. Missing surfaces remain insufficient, and a static HTML proxy never
establishes human-observed usability. Human review may resolve only the remaining criteria.
A high-impact authority/context-recovery insufficiency or conflicting review requires an
explicit human assessment whose `resolves_review_runs` maps the criterion ID to the other
review run IDs addressed. Agent reviews must leave that map empty. This is evidence-bound
judgment, not voting or statistical independence.

Naturalistic summaries use exact criterion identities. Multi-Work Viewer comprehension
uses only `journey-volicord/viewer_snapshot/multiple_work_organization`; browser input/paint
requires every locale listed in the maintained definition's `live_viewer_locales`.
A required violation makes the summary violated, all required criteria resolved makes it
satisfied, and any remaining required gap keeps it unresolved. Result validation uses
the same aggregation rule.

For a fresh campaign's direct Viewer observation, use the candidate-local launch
and whole-snapshot export procedures in the [Viewer README](../../crates/volicord-viewer/README.md).
Read Overview, select Work A/B through Work navigation (including pagination),
follow a Decision's user rationale/recommendation and typed scope, then related
code, relationship endpoints and retained Source details. Inspect failed,
unverified, pending/rejected, review-due and stale/unavailable states and Purpose
present/absent cases where evidence provides them. Preserve observed gaps rather
than inventing missing events or interpreting summary-unavailable as comprehension.
Use en/ko with keyboard/focus/non-color cues, narrow 390/768/1440 base widths and
actual browser 200% zoom; record the actual input/paint experience and limits.
The automated [browser supporting check](../../validation/README.md#browser-support-for-viewer-reading-v11-owner)
can expose regressions but cannot supply these human observations. Multi-Work
organization, applicable Decision comprehension, Learning/semantic value and
Naturalistic-memory criteria retain their separate identities and authorities.
No prior Campaign evidence is rewritten and implementation fixtures are not a
fresh naturalistic campaign.

For direct live observations, human preparation additionally accepts `--human-observations`
pointing to the conversational capture directory. The lower-level JSON-file input remains
available for automation. The object has kind `dogfood_human_observations`, original `candidate_head`,
`evidence_set_sha256`, an `observer` using the common human reviewer identity shape, and
exactly two `observations`: English and Korean live Viewer observations. Long-lived
one-Project/multiple-Work continuity is read from the Volicord journey evidence rather than a
separate user-authored observation. Each live observation has `sample_id`, typed `surface`, optional `locale`,
a typed `control`, and either a grouped `{observation, limits}` response or a Korean-to-English
locale reference with no duplicated semantic prose. Preparation copies and hashes
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

Verification also replays `qualification_policy.combine()` from the packaged evaluation and
recorded reviews and compares every derived aggregate field with the preserved qualification.
Required scope is reconstructed through the common `criterion_specs()` from the evaluation's
Work identities, resume flags, workload intents and journey identities; current naturalistic
scope has no preassigned authority obligations. Recorded review preparations must contain that
same scope. Reviewer-declared additional outcomes and targeted human resolutions remain inputs
to the existing policy. A no-review package replays with an empty review set and retains every
required gap; missing or incompatible replay inputs fail closed, including for success claims.
No additional artifact, external staging dependency or alternate schema reader is introduced.
The preserved technical summary remains a separate input; this replay establishes internal
consistency, not review truth, external authentication or new operator approval.

A valid `not_applicable` assessment for Decision comprehension when no user Decision is
in scope may be established by an agent from the permitted evidence; it does not require
a human to experience a nonexistent Decision. Applicable comprehension remains human-only.
The single machine disposition table is `machine-policy.json`; check names and historical
domains alone do not determine authority. Exact hard facts remain non-overridable even
when a broad procedural check in the same Work is review-required or advisory.

`validate-approval --approval <approval-run>/approval.json --qualification
<qualification-run>/qualification.json` rechecks the immutable approval, preserved
qualification bytes and every original input without exercising approval again.

Current reviewer preparation selects actual Work captures, canonical bundle,
documents, Viewer and repository-state evidence from the immutable campaign.
It assigns no expected semantic class. Every Work exposes the common interaction
criteria and optional behavior opportunities; the reviewer records
`not_observed` when an optional event never arose, with inspected evidence and
reasoning. Learning-specific criteria may be `not_observed` only outside the required
learning/collaborative Work when runtime participation was not active. Active or uncertain participation requires review
or an explicit insufficiency. Reviewers add independently observed material
outcomes and assess actual authority/commitment chronology without relying on
a frozen evaluator concern. Agent disagreement is preserved; a high-impact
conflict or insufficiency requires targeted human resolution. Direct live
Viewer/browser and applicable Decision comprehension remain human-owned.

Journey-final `repository_state` is an additional immutable, bounded reviewer surface shared by
that journey's Works. It exposes status, modes and content/diff hashes without copying private
reconciliation or patch/source bodies. A dirty observed target state alone does not establish a
Product violation, actor attribution, unrelated/pre-existing-dirty attribution, or truthful
verification. Review those independent claims using task, Source/Analysis Snapshot, Checkpoint,
validation-command and raw observation evidence. The original target can later change or disappear;
review-package and evidence-set integrity verify retained bytes rather than the live worktree.

Git history/status and per-session path correlation are factual, advisory review
support. A missing Work commit, dirty Work transition or dirty final target alone
cannot invalidate a campaign or redefine Work identity. Zero/multiple commits and
later commits spanning Works are collectible. Assess any commit/clean requirement
against the actual user's task or repository/workflow instructions; current machine
evidence contains no deterministic task Git-policy requirement and invents none.
Uncorrelated/reverted paths and unknown session-end state remain visible limitations.
Candidate-worktree cleanliness and immutable evidence/publication integrity are separate
requirements and remain strict.

Journey-final `git-observations.json` is inventory/hash-bound and selected as a
`repository_state` reviewer surface even without a machine evaluation or raw-rollout
selection. It exposes the retained Git facts and limitations without patch/source
bodies. Its content must agree with the evidence-set observation at publication and
historical verification. The original Git workspace is not needed for later review.

### Interaction diagnostics and coverage adequacy

`interaction_diagnostics.py` projects raw normalized start/resume captures and
canonical bundle identities into factual Work and campaign summaries. It reports
user turns, observed Question Candidate/promoted Question identities,
unique validated current-host response events matched to successful response operations
(using the shared Decision provenance facts and Codex response interpretation),
source-scoped canonical Decisions,
Materiality Review activity, explicit participation observations and Learning Context
identities, Learning Deliberation activity, fresh resumes/Recall and workload intent.
Capture gaps remain unknown, with retained identity/hash/sequence basis and explicit
limits. Successful-call counts do not attest correct recognition, authority, quality
or adequate interaction coverage. Follow-up turns without matched response operations
remain visible as user turns. The machine run and reviewer selection expose the same
raw-derived summary, including when no machine run or raw-rollout selection was supplied.
Immutable raw rollouts require explicit selection as inputs to the reviewer-safe conversation projection.

`campaign/campaign_interaction/interaction_coverage_adequacy` is a required campaign
criterion. Review considers planned and actually executed intents, projected actual interactions,
machine diagnostic facts, repository/source authority, and a separately recorded
independent agent semantic review. Complete task and semantically complete Work/resume projection inspection is required
for a decisive coverage judgment. The reviewer authors semantic truth; structural
validation checks inspection/citation discipline rather than semantic answers.

- `satisfied`: all required intents actually executed and enough evidence assesses
  important Question/Learning behavior, including correct non-question behavior.
- `insufficient_evidence`: execution occurred but evidence cannot support a reliable
  replacement judgment. Weak task selection or sparse interactions belong here;
  qualification remains unresolved/incomplete, with no automatic Product failure.
- `violated`: observed Product behavior substantively violates the interaction rubric.
  Current high-impact qualitative blocking/conflict-resolution policy applies.

Required campaign coverage cannot be `not_observed` or `not_applicable`. Missing review
also remains unresolved. Conflicting judgments or high-impact coverage insufficiency
need targeted human resolution naming the other reviewed runs; direct human observations
remain required under their existing policy. A human review cannot replace the independent
agent semantic inspection. No count threshold supplies a verdict or qualification.

For `learning_collaborative`, inspect whether the full frozen explicit request was
recognized in runtime Learning participation and relevant context survived fresh resume.
No interaction does not automatically mean `not_observed`: distinguish a source-grounded
absence of a meaningful agent-owned learning-worthy fork, missing evidence, and failure
to honor participation. Learning criteria require a judgment or insufficiency in this Work.
Routine details remain non-interrupting, and no fixed Learning call count is required.
For decision-rich zero-Question work, inspect repository/source authority and actual
commitments; satisfaction needs evidence that no user-owned material Question was needed.
For routine bounded work, review unexpected Questions for necessity and interruption cost.
Other genuinely optional opportunities retain `not_observed` under criterion rules.

Focused regressions cover all five intents, missing intent/request rejection, actual
Learning/resume evidence, absent Learning without silent not-observed, source-grounded
zero-Question decision-rich review, routine Question review, zero-count non-failure,
coverage insufficiency leaving qualification unresolved, substantive high-impact violation,
independent agent review and required human observations. These remain synthetic support.
A successful authoritative gate enables a wholly fresh campaign, never Dogfood success
or Phase 9 approval. All prior failed campaigns remain historical evidence only and
cannot be rebound to the changed candidate.
