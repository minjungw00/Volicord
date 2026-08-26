# Reconstruction validation assets

This directory contains maintained inputs, assertions, report summaries, and
small disposable experiment implementations. It is an internal validation
surface, not a Volicord product command or production architecture.

## Commands

- `rebuild/scripts/validate self-test` checks command execution, output and
  status preservation, signal reporting, and non-fail-fast aggregation with
  fake commands.
- `rebuild/scripts/validate gate-self-test` checks admission blocking,
  authorization separation, exact-once synthetic final/V11 orchestration,
  same-session artifact selection, credential-safe capsule projection, and
  no-retry behavior. It never invokes the real exact final or official V11.
- `rebuild/scripts/validate gate-entrypoint-self-test` copies the maintained
  admission/gate entry point into isolated temporary Git repositories and
  checks clean, initially dirty, and admission-generated dirty candidates
  without invoking the real exact final or official V11.
- `rebuild/scripts/validate evidence-archive-self-test` builds a synthetic
  gate/V11 artifact tree and exercises result discovery, logical repository and
  V11 target cwd projection, argv sanitization, archive creation, and independent
  verification. Its production-scale case uses the real collector and sanitizer
  for at least 178 process records, 1,116 argv entries, and 659 non-structural
  argument-role records. It also rejects a builder-side over-bound member,
  unknown external cwd values, retained prompts or absolute host paths, tampered
  content, missing members, changed POSIX modes, candidate mismatch, repository
  source bodies, malformed or misplaced provider-retention attestations, and
  credential-like prohibited content. Its positive integration case embeds the
  maintained successful-provider evidence shape, including the exact negative
  retention attestation. It invokes neither exact final nor official V11.
- `rebuild/scripts/verify-validation-archive <archive> [--expected-candidate
  <HEAD>]` independently verifies archive membership, content hashes, bounded
  size, candidate agreement, tracked/executable identities, tar modes, and the
  prohibited-content boundary without extracting the archive. Builder and
  verifier both enforce the manifest-declared current 256 KiB uncompressed
  per-member limit; the builder encodes and checks every member before writing
  the tar, while the existing 512 KiB compressed-archive limit remains separate.
- `rebuild/scripts/check-validation-report --self-test` proves generic report
  compatibility plus positive and negative capsule-backed semantics for
  admission-blocked, final-failed, provider-live-qualification-failed, V11-preflight-failed,
  official-V11-failed, and fully-passed lifecycles. It also rejects impossible
  stage combinations and missing success evidence; neither exact final nor
  official V11 is invoked.
- `rebuild/scripts/validate focused <label> -- <command> [arguments...]` runs
  the exact argument vector from the repository root and records the command,
  working directory, timestamps, duration, complete separate stdout/stderr,
  exit code, and termination details.
- `python3 rebuild/validation/shared/current_cli_parity.py --binary
  rebuild/target/debug/volicord` executes maintained Dogfood/V08 command shapes
  against the actual current Clap parser and requires the corresponding removed
  forms to fail as usage errors.
- `rebuild/validation/shared/strict_fake_volicord.py` is bounded self-test
  support for Dogfood campaign and repeated-resource paths. It accepts only the
  maintained repository-selected command ordering and fails every unexpected
  option, command, or subcommand.
- `rebuild/scripts/validate admission` evaluates the current clean candidate,
  runner/V11, architecture, realistic RI, Dogfood campaign/harness, and
  provider self-checks, required fixture identity/integrity, executables, writable disposable
  homes, the maintained resource estimate, loopback, Codex authentication,
  technical external-network state, and the exact bounded transmission
  authorization. It prints a structured result and retains `admission.json`
  below ignored validation state; a blocked result runs neither final nor V11.
- `rebuild/scripts/validate gate` repeats admission in its own session, invokes
  an immediate live clean-worktree/HEAD recheck, invokes the existing ordered
  four-command final owner exactly once, invokes the separately authorized
  production provider qualification once, passes only the returned summary to
  V11 preflight, invokes official V11 at most once, runs the V11
  credential-retention audit, creates and independently verifies the sanitized
  evidence archive, and only then prints a fully ready sanitized handoff capsule.
  Direct `rebuild/scripts/validate final` invocation is refused so an exact
  aggregate cannot bypass admission or become detached from V11.
- `rebuild/scripts/check-architecture-contracts` checks the nine active Phase 3
  owner documents, routing, relative links, traceability IDs, capability-based
  validation paths, prohibited supported paths, Phase 4 handoff structure,
  canonical relation orientation, Candidate inspection/privacy/lifecycle
  structure, and Guarded confirmation/dispatch structure.
- `rebuild/scripts/check-architecture-contracts --self-test` copies maintained
  inputs to isolated temporary fixtures and demonstrates positive validation
  plus independent structural failures, including direction reversal and
  Candidate/Guarded omissions, without modifying active documents.
- `python3 rebuild/validation/shared/contract_coverage.py` verifies that each
  mapped cross-owner behavior names its required Local Operations, CLI, MCP,
  or Viewer product-entry-point tests. Its `--self-test` mode rejects
  internal-primitive-only coverage, missing entrypoints, canonical-only or
  over-broad forgetting, ignored cleanup/repair failure, Candidate
  error-to-empty conversion, and configured-provider failure reported as
  commercial semantic-provider success.

The architecture checker is deterministic internal test support. It does not
define domain meaning, judge conceptual correctness, choose implementation
technology, or treat validation IDs as product-format versions.

Portable process payloads carry one top-level `sanitized_argv_policy`. That
policy declares the sanitized projection, identifies exact raw argv as ignored
local execution evidence, and defines omitted per-argument role entries as
allowlisted structural tokens. Each execution then stores argv plus exactly one
compact `[argument index, classification, semantic role]` record for every
projected or redacted argument. The independent verifier rejects the superseded
duplicated per-execution accounting fields; there is no alternate reader or
numeric format branch.

## Maintained and generated boundaries

Commit self-authored fixtures, their manifest entries, experiment source,
assertions, report templates, and reviewed report summaries. Fixture entries
record purpose, expectations, unsupported constructs, a deterministic content
hash, origin, and license.

Do not commit raw analyzer output, generated graphs, copied source repositories,
logs, measurement scratch data, caches, or local model output. The runner writes
these to ignored `rebuild/.local/validation/`. Maintained reports may cite an
artifact path and hash, but local artifacts are reproducibility evidence rather
than durable design truth.

V01, V03, and V05 remain stable report and metadata identifiers. Tracked
validation assets use capability-based paths:

```text
shared/fixture-manifest.json
shared/report-template.md
repository-intelligence/polyglot-structural/
repository-intelligence/realistic-qualification/
repository-intelligence/phase-5-acceptance/
canonical-context/portability/
inquiry/frontier-resume/
inquiry/phase-6-acceptance/
wave-1-summary.md
phase-4-summary.md
phase-5-summary.md
phase-6-summary.md
```

The shared fixture manifest is the single fixture catalog. Each capability
directory owns its fixtures, assertions, disposable prototype, and maintained
report. The three validations share the report shape in
`shared/report-template.md` without turning spike code into a production
dependency.

The Phase 6 acceptance orchestrator maps V09 requirement identities to named
Production Rust tests. It validates orchestration and evidence completeness
only; Candidate, frontier, Decision, Checkpoint, Recall, and inspection
semantics remain owned by the Production Rust crates.

V07 product acceptance maps canonical forgetting through Local Operations and
the public CLI, MCP, and Viewer adapters. The lower-level canonical tombstone
and privacy cleanup tests remain separately classified owner tests and cannot
by themselves satisfy the cross-owner forgetting requirements.

The V11 multi-repository harness is under
`end-to-end/multi-repository/`. It performs a non-fail-fast rehearsal against
the installed CLI and MCP surfaces, writes child-operation evidence and its
structured result only below ignored `rebuild/.local/`, and classifies missing
public product paths as `unsupported` instead of substituting validation-only
domain behavior. Its maintained journey seeds linked and unrelated Candidate
and managed-Derived controls, inspects them after public forgetting and
restart, and injects unavailable, corrupt, and unsupported Candidate stores to
require an explicit degraded dependency state while canonical inspection
remains usable.

## Scripted conformance and naturalistic dogfood

V11 is the maintained scripted conformance boundary. Its deterministic journey
proves the installed product path and remains the reusable Phase 8 regression;
it is not evidence that an agent independently discovered and used the accepted
experience in a real repository session.

Phase 8 real sessions are naturalistic behavioral dogfood. Each cycle
descriptor carries the exact plain work user task, the exact plain fresh-resume
user task, repository/cycle/revision identity, hidden evaluation material, and
the bounded capture and canonical-bundle references used for qualification.
The user tasks state a real repository outcome and ordinary safety or scope
constraints. They do not prescribe the material Question, alternatives,
recommendation, user selection, Volicord operation order, Checkpoint contents,
a path reserved for the next session, or an instruction to perform Recall.

Each descriptor carries a bounded evaluator-only `evaluation_basis`: the
behavior class, repository facts, accepted contract constraints, delegated
boundaries, non-exhaustive possible material concerns, consequences, facts the
agent should research instead of asking the user, and current relevance. It
does not define one valid Question wording, alternative set, recommendation,
or user selection.

Every descriptor also carries a hidden `behavior_review` prepared by the
campaign control session. Before seeing the evaluator basis, an independent
reviewer receives only a hash-bound preparation artifact containing candidate
and repository identity, revision, exact frozen tasks, work scope, and owner
document locations. The reviewer records a provisional classification and
materiality conclusion; only then may the evaluator basis and counterfactual
analysis be compared. Typed provenance can bind content hashes either to one of
the nine current active architecture owners at the candidate revision or to a
safe path in the cycle's exact pinned target revision. Qualification re-reads
those Git objects and rejects inactive owner documents, traversal, missing
files, wrong revisions, and stale hashes. All five maintained behavior classes
require accepted independent review; typed provenance does not mechanically
prove the classification.

Work-session research, Inquiry, current-host Decision provenance, ordinary
work, numeric-exit verification, and Checkpoint creation are observed from the
actual Codex rollout and canonical bundle rather than disclosed as prompt
choreography. The first work turn is bound directly to the canonical Goal
Source and identity. The selected terminal Checkpoint references that Goal
identity and supplies the next meaningful state or step. A work capture may
contain earlier pause or handoff Checkpoints; the latest Checkpoint candidate
after the last meaningful repository change is selected, and a malformed final
candidate cannot be hidden by falling back to earlier history.
Successful repository analyses are not globally unique. The harness selects
the analysis whose snapshot identity is explicitly retained by the applicable
Checkpoint, then verifies the same Project and its completion after the Goal or
Recall boundary and before the first meaningful write. Later analyses remain
valid evidence but cannot replace that selected pre-work baseline.

The fresh-resume task is likewise ordinary user language and does not mention
Recall or contain a Project ID. Qualification requires successful
`project_resolve` from the current repository binding to the canonical bundle's
same Project before Recall. Recall must then precede repository inspection or
continued work, and the resume session must not initialize a replacement
Project merely to obtain an identity. Context recovery is derived from that
Recall result. Every applicable resume Checkpoint is checked against its exact
retained pre-work snapshot rather than a session-wide analysis count.
Continuation may either make a relevant change and validate it,
or inspect and numerically verify an already-completed recalled state without
an artificial mutation. Paused or in-progress work cannot use the no-change
path, and Recall without later inspection and validation does not qualify.
Deterministic journey success remains independent from the optional
campaign-level review of Question relevance, Decision comprehension,
interruption cost, document fidelity/readability, and Viewer usability.

For `explicit_user_owned_decision` and `hidden_user_owned_decision`, the work
capture must show Candidate submission, source-grounded repository research,
reviewed material promotion through `candidate_manage`, and a Question in
`inquiry_frontier` before the first affected ordinary write. Only an explicit
current-host user response can qualify `decision_record`; the agent's own
recommendation or implementation preference cannot. The hidden-class prompt
must not disclose that a material choice exists or identify its outcome; its
review must establish that complete work necessarily encounters that choice.
For
`research_or_no_question`, `delegated_implementation_choice`, and
`exploratory_uncertainty`, the absence of Candidate, Question, and Decision can
be the correct passing outcome.

Before those product behaviors are judged, work-capture intake verifies that
the repository-scoped SessionStart activation context is present. Its absence
is an operator/environment setup failure and stops that campaign path without
attributing missing Inquiry behavior to Volicord. Repository and hook trust
remain explicit operator actions.

The current result schema reports `automated_qualification`, `human_review`,
and `replacement_qualification` separately. The automated `run` path covers
three repository classes, five behavior-class cycles per class, and two
globally distinct fresh VS Code Codex sessions per cycle: fifteen cycles and
thirty sessions, plus every maintained machine check.
It exits successfully when those checks pass even if human review is
`not_provided`; in that state replacement is `pending_human_review` and neither
`replacement_pass_candidate` nor Phase 9 readiness is true.

A completed work session with a machine-observable terminal work blocker may
be classified without executing later qualifying sessions:

```text
python3 rebuild/validation/dogfood/harness.py qualify-work-blocker \
  --candidate-head <current-candidate-head> \
  --descriptor <one-cycle-descriptor.json> \
  --repository <exact-pinned-cycle-repository> \
  --work-capture <completed-work-rollout.jsonl> \
  --output <blocker-result.json>
```

The failure-only result kind is `phase8_dogfood_blocker_result`. Missing
required high-level Project, Goal Context, repository baseline, behavior-class
evidence, or grounded Checkpoint operations are terminal when absent from a
completed work capture. Material Question Candidate/promotion and explicit
current-host Decision are additionally required only for the two user-owned
decision classes; a later resume cannot retroactively put them in that session.
If the capture
cannot prove a required semantic fact, the harness requires normal full
qualification instead of inventing a blocker. A positive work session cannot
be converted into an early failure. The result always records
`campaign_complete = false`, `replacement_pass_candidate = false`, and
`phase_9_ready = false`, identifies later sessions/checks as `not_run`, and
retains only bounded identities, failed checks, and the capture hash—not task
text, hidden evaluation material, source bodies, credentials, or raw provider content.

The maintained internal campaign helper reduces evidence handling without
creating or coaching a naturalistic session:

```text
rebuild/scripts/dogfood-campaign prepare \
  --campaign-root /absolute/private/campaign \
  --campaign-id <new-campaign-identity> \
  --candidate-head <clean-candidate-head> \
  --repositories <three-repository-input.json>
```

`prepare` verifies the clean candidate and source identities, performs a
candidate-local install, and creates fifteen revision-pinned disposable repository
workspaces with fresh Runtime Homes. Evaluator descriptor/review inputs live
under the private evaluator plane; the run sheet and separate campaign-level
human-review artifact live under the operator plane. A preparation/control
agent completes an evaluator input and invokes `prepare-review`. The
independent reviewer records the provisional review from that bounded artifact
before receiving the evaluator basis. The control agent then invokes:

```text
rebuild/scripts/dogfood-campaign record-provisional-review \
  --campaign-root /absolute/private/campaign \
  --candidate-head <candidate> \
  --review-slot-id <opaque-id> \
  --provisional-review <path>
```

This successful reviewer-plane operation verifies
the exact campaign candidate, opaque preparation identity and strict provisional
schema/self-consistency from the reviewer's own classification without reading an
evaluator descriptor or comparing evaluator truth, then atomically fixes the private
artifact, hash inventory and `provisional_recorded` state. Correct and evaluator-wrong
well-formed classifications have the same successful non-oracle result shape. The
control agent next invokes `seal-cycle --descriptor <path>`. Sealing reads only that
immutable recorded review and, after evaluator reveal, verifies the class, pinned
revision, active-owner or target-repository provenance and content hashes. Its
structured classification comparison must report exact classification/materiality/
disclosure differences as `agreed`, evidence-backed `resolved_from_evidence`, or
blocking `unresolved_conflict`; disagreement cannot masquerade as agreement or rewrite
the provisional review. Sealing then
stores the authoritative hidden descriptor, freezes its semantic hash, and
regenerates the run sheet from only the exact work/resume tasks and operational
paths. `activate-cycle`, `activate-all`, and rollout collection reject unsealed cycles. The run-sheet
leak check rejects exact hidden evaluation/review material and deliberately marked
evaluator-only sentinels. This is workflow/evidence isolation, not an OS
security boundary against deliberately opening evaluator files. The helper
never grants Codex repository or hook trust; the operator reviews and grants
trust in VS Code.

The roles remain separate throughout a campaign:

- The evaluator/control agent researches the actual repositories, prepares and
  independently reviews the hidden evaluation basis and behavior review, and seals each
  descriptor through the helper. Evaluator data stays out of operator-facing
  instructions and examples; the operator is never asked to inspect or edit a
  descriptor.
- The naturalistic operator inspects and trusts the intended repository,
  explicitly approves the SessionStart hook, opens every required fresh VS
  Code Codex session, and sends only the frozen work/resume tasks from the run
  sheet. The operator supplies answers only to genuine material Questions,
  preserves all thirty raw rollouts, and provides them once after the sessions finish.
- The helper owns campaign setup, sealed-descriptor validation, operator
  run-sheet generation, byte-exact rollout intake and hashing,
  activation/setup classification, early blocker gating, Project-ID
  extraction, canonical bundle export, bounded Runtime summaries, all four
  generated document kinds in Markdown and self-contained HTML, descriptor
  evidence completion, repository-manifest assembly, deterministic
  campaign-level review sampling, and bounded review packaging.

After all fifteen descriptors are sealed, `activate-all` may enable the fifteen
repository-scoped integrations before the chats begin. It never grants
repository or hook trust. It re-reads the owned manifest, MCP entry, SessionStart
hook, and exact candidate-local executable/Runtime binding after each enable;
any static inconsistency blocks activation completion. This does not prove VS
Code executed SessionStart. If trust or activation setup is uncertain, inspect
it before sending a frozen task. Every raw work/resume capture must still contain
real SessionStart evidence. `collect-batch` accepts either thirty explicit paths
or one directory containing exactly thirty files. Before changing campaign
state it maps the unordered captures to the sealed work/resume slots using the
frozen first task, exact workspace and revision, VS Code source/originator,
fresh session identity, and SessionStart activation. Ambiguous, missing,
duplicate, mismatched, or session-reused input is rejected globally.

After mapping succeeds, `collect-batch` copies and hashes every rollout
byte-for-byte. It preserves a terminal work blocker even when the matching
resume exists and continues parsing later captures only for bounded diagnostic
and extractable evidence. Missing activation remains
`operator_environment_invalid`. For each safely identifiable cycle it derives
the Project ID, invokes the installed candidate's repository-selected
`context export --output`, completes descriptor evidence references and hashes,
and invokes the supported `document export`
path for `project-architecture-guide`, `decision-report`,
`implementation-plan`, and `handoff-resume` in both Markdown and
self-contained HTML. A deterministic per-cycle summary records every
kind/format status, bounded failure basis or relative evidence path, bytes, and
SHA-256; export failure remains explicitly failed. The public
`volicord-viewer --snapshot` capability also produces one self-contained,
read-only HTML snapshot with its Project/candidate basis, relative path, bytes,
and SHA-256. A bounded operator review
index lists the produced paths without evaluator material. The helper also
writes a bounded Runtime Home summary containing managed logical names and sizes,
derived-analysis size, configuration presence, and activation booleans; it
never reads or copies store, derived-analysis, credential, provider-payload,
prompt, or source-body contents. `collect-work` and `collect-resume` remain
available as non-default focused diagnostics; they are not the ordinary
operator workflow.

The generated documents and Viewer serve different review needs. Each document
uses a comprehension-first body and moves opaque identities, hashes, complete
capability inventory, and claim-level grounding into a distinct trailing audit
appendix or default-closed HTML disclosure. The live Viewer uses the current
human-first hierarchy. Its static snapshot is a separate read-only,
self-contained share/review artifact that works without a Runtime or listener;
it is not interchangeable with a generated document and does not share the
document-adoption lifecycle.

`finalize-manifest` deterministically assembles `repositories.json` after all
fifteen resume captures. Run the automated Dogfood evaluation without subjective
inputs. If replacement qualification is needed, create one campaign-level
review artifact from the immutable automated result:

```text
rebuild/scripts/dogfood-campaign prepare-human-review \
  --campaign-root /absolute/private/campaign \
  --automated-result /absolute/private/automated-result.json
```

The artifact requires review of every automated-passed interaction cycle.
Each cycle covers Question necessity, unnecessary interruption, explicit
material-decision handling quality, hidden material-decision discovery quality,
user ownership, Decision comprehension when applicable, repository-analysis and
structural-navigation usefulness, semantic value and honesty, CLI usability,
Viewer understanding, and all four documents' fidelity, usefulness, remaining
work accuracy, grounding distinction, and requested-language body. Polyglot
cycles additionally cover cross-language/component/config/API/process
comprehension. Volicord live Viewer samples in `en` and `ko` cover keyboard,
focus, color, narrow layout, and zoom accessibility.
Every human criterion is `not_provided` initially. After bounded review,
`qualify-review` combines that artifact with the byte-identical automated
result and does not rerun the sessions or machine evaluation. A human failure
preserves an automated pass but fails replacement; a human pass cannot
override any automated failure.

`package-review`
then creates a deterministic bounded archive containing campaign metadata,
the manifest, fifteen descriptors, blind-first reviewer preparations,
provisional reviews, derived review views, hash inventory,
canonical bundles, the campaign-level human review when present,
runtime/activation summaries, blocker records
when present, and all generated-document summaries, review indexes, Markdown,
and HTML evidence. Raw rollouts are excluded by
default and enter only with `--include-raw-rollouts`. Full Runtime Homes,
SQLite files and sidecars, derived directories, installation files, source
repositories, credentials, private prompts, and provider payloads are never
selected by the default packager. Keep the campaign root in ignored private
state. Ordinary independent review requires both the byte-exact raw rollout
archive and the bounded review package; transfer them as separate private
artifacts. It does not require, and must not substitute, a full Runtime Home.

This distinction does not change admission, exact final, official V11, gate
ownership, or the capsule lifecycle described below.

The current maintained pre-Dogfood entry state is summarized in
`phase-8-summary.md`: `replacement_gate = pending` and
`phase_9_ready = false`. Exact final and same-session official V11 passed for a
prior predecessor-contract candidate:
admission was `eligible`, exact final succeeded with zero failures, all 54
required V11 steps passed, the credential-retention audit passed with zero
recorded findings or scan errors, no active accepted-Decision revisit trigger
was reported, the sanitized evidence archive was independently verified, and
`phase_8_ready = true` under that prior contract. The redesigned campaign has
no sealed candidate; its technical entry and Dogfood state are `not_run`.

Automated Dogfood has not run for the redesigned campaign and campaign-level human
review is `not_provided`. The operator workflow is batch-first: after hidden
evaluator material is independently reviewed and sealed, the user approves
repository/hook trust, completes all thirty fresh naturalistic chats without
per-session evidence processing, and supplies the raw rollouts once to
`collect-batch`. The helper derives cycle mapping, bundles, bounded Runtime and
activation summaries, four document kinds, and static Viewer snapshots.
Automated qualification may complete without human review; absent review keeps
replacement pending rather than failing automation, while human review can
never override a machine failure.

Predecessor Dogfood descriptors, captures, Runtime Homes, workspaces, bundles,
observations, and session identities remain non-reusable for a future candidate.
Any predecessor Small Python cycle is diagnostic only and is not qualifying
evidence for the redesigned campaign.
Replacement passage remains pending/false, and Phase 9 may not begin. After a
later technical-entry gate seals a candidate, qualifying Dogfood must run from
a separate clean worktree whose actual Git `HEAD` is exactly that candidate. A
different support-branch HEAD cannot qualify by passing only a candidate
argument.

## Admission, authorization, and handoff

Technical network availability and authorization are separate current-
invocation inputs. `--external-network available` asserts only that the
execution environment can reach the service. It does not authorize a
transmission. Missing or escalation-dependent technical access is an
`environment_blocked` admission result.

The two accepted authorization assertions are independent:

```text
--authorize-external-transmission v11-openai-codex-project-health-three-targets
--authorize-provider-source-transmission openai-codex-background-semantic-bounded-rust-v1
```

The first covers the maintained journey's three authenticated Codex turns for the
`volicord`, `small-python`, and `polyglot-medium` targets. Their destination is
the OpenAI Codex service used by the installed Codex CLI; their purpose is to
select the installed `project_health` MCP tool; their intended source scope is
the bounded V11 prompt, Project identity, and tool result, not repository
source bodies.
The second covers one production-provider qualification transmission to that
same service for the sole maintained source
`rebuild/validation/privacy/background-provider-qualification/fixtures/bounded-rust/src/lib.rs`
(at most 4096 bytes), for the purpose of qualifying bounded semantic analysis;
it excludes every other repository source. Credentials, generic network access,
sandbox escalation,
Project provider opt-in, an earlier report, or an earlier session cannot
supply either assertion, and neither assertion supplies the other. Provider
qualification also requires `--provider-model <exact-model>`. Admission records
only the exact assertion IDs and model, not operator authorization prose or
credential contents.

Preflight-only example:

```text
rebuild/scripts/validate admission \
  --external-network available \
  --authorize-external-transmission v11-openai-codex-project-health-three-targets \
  --authorize-provider-source-transmission openai-codex-background-semantic-bounded-rust-v1 \
  --provider-model <exact-model>
```

Exact gate example (reserved for the one authorized final/V11 session):

```text
rebuild/scripts/validate gate \
  --external-network available \
  --authorize-external-transmission v11-openai-codex-project-health-three-targets \
  --authorize-provider-source-transmission openai-codex-background-semantic-bounded-rust-v1 \
  --provider-model <exact-model>
```

Admission writes its current structured result to the path reported on
stderr. The gate writes `admission.json`, `gate-result.json`, and `capsule.json`
under the reported ignored run directory and prints the complete capsule to
stdout so it can be copied before that directory disappears. It also builds
and independently verifies `validation-evidence-<candidate-prefix>.tar.gz`, then
reports the archive path and SHA-256 on stderr. A reviewer may copy that
archive out of ignored local state and verify it with
`rebuild/scripts/verify-validation-archive`.

Before archive verification completes, the retained capsule and gate result are
non-passing and identify the archive as pending. A successful exact final,
official V11, and credential audit therefore cannot become consumer-visible as
a passed top-level gate by themselves. Archive creation or verification failure
publishes a corresponding blocked capsule with `phase_8_ready = false`; no final
or V11 retry is performed.

The portable archive is a bounded structured projection, not a replacement for
the gate's ignored execution truth. Complete stdout/stderr, detailed V11 local
artifacts, and raw command evidence remain under `rebuild/.local/validation/`.
The archive contains only the capsule, sanitized admission/gate/final/process
records, candidate-bound tracked validation-tool identities and executable
modes, plus a member hash and POSIX-mode manifest. Repository-root cwd is
represented as logical `.`, and maintained official-V11 repository/clone
execution roots are represented by bounded target/root identities. Arbitrary
external cwd values are rejected. Portable argv retains only executable,
command, subcommand, flag, closed-value, and owned-path roles that an explicit
command-family policy classifies as structural. Recognized paths are projected;
private prompts, inline programs, config/message bodies, identities, content
operands, and every unknown role are redacted regardless of lexical shape. Each
argument records whether it is structural, projected, or redacted. The vector
is labeled as a sanitized portable projection, while exact raw argv remains
only in ignored local evidence.
The structured records retain timestamps, duration, exit/wrapper status,
termination, and spawn state. The capsule retains the original final-summary
hash, dependency/fixture identities, official-V11 state, credential audit,
same-session ownership, and verified archive identity.

The archive excludes stdout/stderr bodies, detailed provider artifacts,
environment dumps, repository source bodies, prompts, `auth.json` contents,
credentials and reusable credential fingerprints. The verifier reads members
in memory, refuses unsafe or unexpected member types and paths, and rejects
missing or additional files, hash/size/mode drift, internal or caller-supplied
candidate mismatch, invalid tracked/executable evidence, and prohibited keys
or credential-like values. The only sensitive-name exception is the exact
`capsule.json.live_provider_qualification.evidence.retained_evidence` object:
it must contain exactly `source_body`, `provider_response_body`, and
`credential`, and every value must be the literal boolean `false`. The same
keys at any other location, any other value, a missing or additional field, or
a malformed container is rejected independently. Tar member modes are
preserved and checked; the archive file itself is created mode `0600` in
ignored local state.

The versionless current capsule has `kind = validation_handoff_capsule`. It is
one stage-dependent contract rather than separate success and failure schemas.
Its bounded cross-session evidence is:

- validated candidate HEAD, sanitized admission check name/status, pre-final
  check, and any gate blocker;
- Linux OS/release/platform, machine/architecture, and Python runtime identity;
- bounded Python, Git, Cargo, Rust compiler, and installed Codex CLI version
  probes, including explicit unavailable/error state;
- SHA-256 identities for `rebuild/Cargo.lock`, `rebuild/Cargo.toml`, and the
  maintained fixture manifest, plus the required V11 fixture identities;
- the exact reproducible gate `argv`, technical network assertion, bounded
  authorization assertion ID, and maintained destination, purpose, target
  scope, and source scope;
- exact-final aggregate status, failure count, summary hash, and each command's
  actual `argv`, outcome, exit/termination/spawn state, and duration;
- same-gate final artifact production, provider live-qualification identity/result,
  and consumption facts for V11 preflight
  and official V11;
- official V11 status/result hash and status counts, authenticated target
  classifications, credential-audit result/counts, `phase_8_ready`, and active
  Decision revisit-trigger state;
- production provider live-qualification status/evidence hash, bounded provider/model/source
  scope, usable success/degradation outcome, and raw-material non-retention state;
- sanitized evidence archive identity, size/member count, prerequisite state,
  and independent verifier outcome.

It excludes environment-variable or home-directory dumps, usernames,
credentials, `auth.json` contents, reusable credential fingerprints, source
bodies, full command logs, raw provider payloads, and private prompt bodies. A
later documentation-only session uses the copied capsule and maintained tracked
inputs; it never needs, searches for, or substitutes an ignored final or V11
artifact from another session. Capsule-backed semantic checking resolves the
provided input and refuses capsules in any Git-ignored repository-local runtime
area, including `rebuild/.local`; the operator must pass an explicit copy from
an external handoff location.

Capsule semantic checking follows the stages actually reached. A blocked
admission or pre-final check requires its supporting check outcome and no later
evidence. A final failure requires complete exact-final evidence and no V11
evidence. A V11-preflight failure requires the successful same-gate final and
preflight consumption but no official result. An official-V11 failure requires
the successful final, same-session ownership, actual V11 result/status, and
only the authenticated targets attempted. Full success additionally requires
all maintained targets, the credential audit, every artifact-flow fact, and a
successfully created and independently verified sanitized evidence archive.
Only that final state may set `phase_8_ready = true`.

Generic maintained reports keep the one-argument shape check. A V11 conclusion
must use the capsule-backed semantic mode so the checker compares structured
capsule values to the relevant report sections:

```text
rebuild/scripts/check-validation-report \
  --capsule /path/to/copied-capsule.json \
  rebuild/validation/end-to-end/multi-repository/report.md
```

The report records the capsule field labels and values; it may render them as
tables or prose. Statements that versions or commands were not projected do
not satisfy this mode when the capsule contains those values.

Examples:

```text
rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
rebuild/scripts/check-validation-report rebuild/validation/repository-intelligence/polyglot-structural/report.md
rebuild/scripts/check-validation-report --self-test
```

The Phase 5 acceptance orchestrator maps maintained fixture and requirement
identifiers to Production Rust tests. It owns orchestration and evidence
accounting only; analyzer and product semantics remain in the Production Rust
subsystem.

The realistic Repository Intelligence qualification adds a maintained Tier 1
seven-language adversarial corpus and an optional Tier 2 public-repository
corpus. Tier 2 source is never committed: `external_corpus.py fetch` places
revision-pinned sparse checkouts under ignored `rebuild/.local/` state, while
`status` reports an absent checkout as `environment_blocked`. Run both tiers
through the repository-local focused runner:

```text
rebuild/scripts/validate focused realistic-external-fetch -- python3 rebuild/validation/repository-intelligence/realistic-qualification/external_corpus.py fetch
rebuild/scripts/validate focused realistic-corpus-qualification -- python3 rebuild/validation/repository-intelligence/realistic-qualification/assertions.py
```
