# Reconstruction validation assets

This directory contains maintained inputs, assertions, report summaries, and
small disposable experiment implementations. It is an internal validation
surface, not a Volicord product command or production architecture.

## Commands

- `rebuild/scripts/validate self-test` checks command execution, output and
  status preservation, signal reporting, and non-fail-fast aggregation with
  fake commands.
- `rebuild/scripts/validate gate-self-test` checks admission blocking,
  including independent failure of either contract-coverage mode, authorization
  separation, exact-once synthetic final/V11 orchestration,
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
- `rebuild/scripts/validate admission` is optional cheap diagnostic preflight:
  candidate cleanliness, tools, writable disposable homes, resources, loopback,
  authentication, network, exact model and both transmission authorizations.
  It starts no deterministic support, Final, provider or V11 work. Exit 0 means
  `preflight_passed`, with `preflight_eligible = true` and `eligible = false`.
  Support checks are explicitly `not_run` with a reason. This diagnostic supplies
  no trusted artifact to the gate and need not immediately precede it.
- `rebuild/scripts/validate gate` owns authoritative admission. Cheap blockers
  record all support as `not_run` with blocking check IDs. Once cheap checks pass,
  it runs the deterministic runner/V11/gate/entrypoint/archive/report, contracts,
  architecture, RI, Dogfood, provider and fixture support once, preserving every
  result even after a failure. It then rechecks clean HEAD and invokes the ordered
  four-command Final once. Final's Cargo metadata and test output must show each
  mapped contract test's actual successful execution before provider qualification
  starts. Only fresh evidence from this invocation is used for the provider, V11
  preflight, official V11, credential audit and independently verified archive.
  Direct `final` is refused. Local support ownership and the deduplication evidence
  are maintained in [validation-plan.md](../docs/design/validation-plan.md#31-maintained-final-provider-qualification-v11과-documentation-handoff-lifecycle).
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
  or Viewer product-entry-point tests as direct, plain `#[test]` Rust integration
  tests. It rejects comments/literals, ordinary functions, ignored/conditional or
  nested declarations. The gate binds them to actual Final workspace targets and
  successful libtest records; static success alone does not prove execution or
  the semantic adequacy of a test body. Its `--self-test` mode also rejects
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

<!-- phase8-active-operations:start -->
Phase 8 real sessions are naturalistic behavioral dogfood. Each Work
descriptor carries the exact plain work user task and, for Work A in each
journey, the exact plain fresh-resume user task. It also carries
repository/journey/Work/revision identity, hidden evaluation material, and
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
safe path in the Work's exact pinned target revision. Qualification re-reads
those Git objects and rejects inactive owner documents, traversal, missing
files, wrong revisions, and stale hashes. Any assigned class in the maintained
behavior vocabulary requires accepted independent review; typed provenance does
not mechanically prove the classification.

During Phase A, the reviewer may inspect the prepared source workspace and listed
owner documents needed for the task, but uses only the prepared reviewer plane as
campaign evidence. Qualification-control implementation and evaluator/steward state
are outside that blind evidence plane. This is workflow isolation, not OS secrecy.

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
be the correct passing outcome. The delegated positive control uses the actual
`delegated_implementation_choice` Materiality disposition and must cite the
exact frozen Goal ID and current-host user-turn Source ID together with a
bounded verbatim statement or excerpt present in both the Goal and frozen task,
plus the affected scope. That typed evidence must cover every delegated
dimension. It does not require `research_basis`, and research, recommendation,
convention, accepted contract, or Decision evidence cannot be relabeled as
current-task delegation. The distinct Inquiry-time delegation Decision path is
still supported; `settled_authority` is a different product meaning.

Machine qualification accepts one or more uniquely identified Materiality
dimensions and correlates revisions by `dimension_id`, independent of array
order. Fact, settled, delegated, exploratory, and user-owned dimensions may
coexist. Every recorded unresolved user-owned dimension remains blocking, and
every resolved one must name a Decision whose current-host provenance and
materiality scope cover that exact dimension. Separate Decisions may resolve
independent dimensions; one Decision may cover coupled dimensions only when
its recorded scope includes each identity. A dimension may not disappear from
a later authority revision.

Before those product behaviors are judged, work-capture intake verifies that
the repository-scoped SessionStart activation context is present. Its absence
is an operator/environment setup failure and stops that campaign path without
attributing missing Inquiry behavior to Volicord. Repository and hook trust
remain explicit operator actions.

The qualification result separates evidence validity, exact-candidate technical gate,
machine findings, qualitative completion, human escalations, operator approval,
replacement qualification and Phase 9 readiness. Collection requires three
repository journeys, five Works and eight globally distinct fresh sessions across
the unchanged three repository classes. The private materiality-obligation profile
remains blind until all five provisional reviews are fixed.

The following table is a human-readable projection of the public operating
contract in `dogfood/evaluation.json`; it does not own a separate campaign
definition.

<!-- phase8-public-campaign-contract:start -->
| Public campaign field | Current requirement |
| --- | --- |
| `repository_journeys` | `3` |
| `work_items` | `5` |
| `work_slots_by_repository` | `volicord=A/B/C, small-python=A, polyglot-medium=A` |
| `fresh_resume_pairs` | `3` |
| `fresh_sessions` | `8` |
| `provisional_reviews_before_reveal` | `5` |
| `sealed_descriptors_and_reviews` | `5` |
| `complete_batch_raw_rollouts` | `8` |
<!-- phase8-public-campaign-contract:end -->

`harness.py inspect-work --candidate-head <original-candidate> --descriptor <descriptor>
--repository <pinned-repository> --work-capture <completed-raw> --output <new-file>`
preserves bounded work observations. Semantic missing-operation/count/ordering findings
are review signals, not terminal campaign decisions. This read-only diagnostic does not
collect or qualify evidence.

SessionStart identity is owned by
`rebuild/crates/volicord-operations/src/session_start_identity.txt`, consumed by
the production `codex::activation_context` renderer and Dogfood `codex_events`.
The marker binds SHA-256 of canonical cwd UTF-8, NUL, and host session ID UTF-8.
It contains no source content and is correlation evidence, not authentication.
The hook emits protocol JSON; total additional context remains below 1,792 bytes. Checkpoint verification guidance
requires actual numeric exit/termination in host-visible results and preserves execution
identity through the terminal result of polled commands. It does not guarantee capture
by every external host tool implementation or permit claims of unobserved verification.
Detection requires an exact bound marker line in developer context before the
first user task. Campaign intake separately checks candidate, workspace,
revision, fresh session and sealed role identity. MCP use only corroborates it.
Human guidance wording is independent; stale prose is not a supported identity.
`campaign_self_test.py::assert_production_session_start` builds the current CLI,
generates the eight disposable start/resume hook outputs, and exercises the same
`load_codex_capture`, mapping and `collect_batch` paths used for real intake.
Activation failures retain bounded evidence state and attribution: absent or
late context is an `environment` setup failure; malformed/unsupported identity
or a cwd/session binding mismatch is an `evidence` failure. A contradiction
between valid production identity evidence and the validator's activation result
is `validation_internal`. The latter two remain `evidence_failed`, never a pass
or an operator setup failure. Batch summaries keep these diagnostics in
`activation_invalid_diagnostics`, separate from `environment_invalid_diagnostics`.
Candidate/revision/workspace/role mismatches remain pre-mutation mapping errors.
`assert_activation_failure_attribution` exercises all eight session slots
for each failure class, including an injected validator false negative over
unmodified production-generated evidence.

The diagnostic result kind is `dogfood_work_observation`. Original failed checks remain
inspectable alongside policy-typed findings and `qualification_state = not_run`.
Hard measurement integrity remains non-overridable; semantic uncertainty stays reviewable.
No diagnostic claims campaign completion, replacement qualification or Phase 9 readiness.

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
candidate-local install, privately commits the realized qualification profile
and assignments, and creates three revision-pinned journey workspaces with
fresh Runtime Homes. The three Volicord Works share that journey's workspace,
Runtime Home, and Project; the other journeys each contain Work A. Evaluator descriptor/review inputs live
under the private evaluator plane; the run sheet and separate campaign-level
human-review artifact live under the operator plane. A preparation/control agent completes an evaluator input and invokes `prepare-review`.
The hash-bound `reviewer/provisional-review-contract.json` supplies the blind assessment,
critique, and adjudication vocabulary and decomposition guidance.

A fresh primary blind reviewer fills the mutable `reviewer/drafts/<slot>.json` discovery draft
from the reviewer-safe preparation and workspace. `validate-discovery` checks its bounded
assessments and reviewer-visible provenance without changing campaign state;
`record-discovery` fixes its exact bytes at `reviewer/discovery/<slot>.json` and advances the
Work to `discovery_recorded`. The reviewer keeps the evaluator/profile plane closed.

An independent fresh blind critic uses `prepare-critique`, the same reviewer-safe surfaces,
and the immutable discovery. The critic may propose a missing or split scope, equivalence,
reclassification, authority or applicability correction, or removal of implementation-detail
inflation. Each bounded proposal cites reviewer-visible evidence. `validate-critique` and
`record-critique` fix the critique at `reviewer/critique/<slot>.json`, advancing the Work to
`critique_recorded`. A fresh blind adjudicator then calls `prepare-adjudication`, edits the
mutable final draft, and records an evidence-backed disposition for every proposal. Accepted
or partially accepted concerns enter final assessments only through explicit lineage; rejected
concerns do not enter, and equivalent concerns do not duplicate an assessment.

`validate-provisional-review` checks the adjudicated final against both immutable hashes,
all dispositions, and every final dimension's discovery/critic lineage. It cannot publish.
`record-provisional-review` alone fixes its exact bytes at
`reviewer/provisional/<slot>.json` and increments `provisional_count`. All three roles remain
blind to evaluator descriptors and the qualification profile. The campaign owns opaque role
artifacts and hashes; actual fresh conversation independence is an operator procedure, not a
machine-attested identity claim. Five complete pipelines and intact inventory bindings are
required before `reveal-qualification-profile`. A scope missed by both blind roles remains a
real `blind_coverage_gap` after reveal; explicit `not_applicable` remains a distinct fixed
negative assessment.

For each opaque slot, use the current candidate and campaign root with these operations
in order. The generated critique and adjudication drafts carry their campaign-owned run
identities; a fresh conversation is still required for each role.

```sh
campaign=/absolute/private/campaign
candidate="<exact-current-clean-head>"
slot="<opaque-review-slot-id>"
rebuild/scripts/dogfood-campaign validate-discovery --campaign-root "$campaign" --candidate-head "$candidate" --review-slot-id "$slot" --discovery "$campaign/reviewer/drafts/$slot.json"
rebuild/scripts/dogfood-campaign record-discovery --campaign-root "$campaign" --candidate-head "$candidate" --review-slot-id "$slot" --discovery "$campaign/reviewer/drafts/$slot.json"
rebuild/scripts/dogfood-campaign prepare-critique --campaign-root "$campaign" --candidate-head "$candidate" --review-slot-id "$slot"
rebuild/scripts/dogfood-campaign validate-critique --campaign-root "$campaign" --candidate-head "$candidate" --review-slot-id "$slot" --critique "$campaign/reviewer/critique-drafts/$slot.json"
rebuild/scripts/dogfood-campaign record-critique --campaign-root "$campaign" --candidate-head "$candidate" --review-slot-id "$slot" --critique "$campaign/reviewer/critique-drafts/$slot.json"
rebuild/scripts/dogfood-campaign prepare-adjudication --campaign-root "$campaign" --candidate-head "$candidate" --review-slot-id "$slot"
rebuild/scripts/dogfood-campaign validate-provisional-review --campaign-root "$campaign" --candidate-head "$candidate" --review-slot-id "$slot" --provisional-review "$campaign/reviewer/drafts/$slot.json"
rebuild/scripts/dogfood-campaign record-provisional-review --campaign-root "$campaign" --candidate-head "$candidate" --review-slot-id "$slot" --provisional-review "$campaign/reviewer/drafts/$slot.json"
```

Only then may the steward invoke `prepare-reconciliation --repository-class <class>
--work <A|B|C>`, edit the returned campaign-owned draft, and invoke
`validate-reconciliation` with the same campaign/class/Work arguments. `seal-work`
uses those arguments only and consumes the exact validated draft; it accepts no descriptor path.
Sealing reads that
immutable recorded review and, after evaluator reveal, verifies the class, pinned
revision, active-owner or target-repository provenance and content hashes. Its
structured classification comparison must report exact classification/materiality/
disclosure differences as `agreed`, evidence-backed `resolved_from_evidence`, or
blocking `unresolved_conflict`; disagreement cannot masquerade as agreement or rewrite
the provisional review. Sealing then
stores the authoritative hidden descriptor, freezes its semantic hash, and
regenerates the run sheet from only the exact work/resume tasks and operational
paths. `activate-journey`, `activate-all`, and rollout collection reject an
incompletely sealed campaign. The run-sheet
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
  preserves all eight raw rollouts, and provides them once after the sessions finish.
- The helper owns campaign setup, sealed-descriptor validation, operator
  run-sheet generation, byte-exact rollout intake and hashing,
  activation/setup classification, early blocker gating, Project-ID
  extraction, canonical bundle export, bounded Runtime summaries, all four
  generated document kinds in Markdown and self-contained HTML, descriptor
  evidence completion, repository-manifest assembly, deterministic
  campaign-level review sampling, and bounded review packaging.

After all five Work descriptors are sealed, `activate-all` may enable the three
journey-scoped repository integrations before the chats begin. `activate-journey
--repository-class <class>` is the corresponding single-journey operation. Neither grants
repository or hook trust. It re-reads the owned manifest, MCP entry, SessionStart
hook, and exact candidate-local executable/Runtime binding after each enable;
any static inconsistency blocks activation completion. This does not prove VS
Code executed SessionStart. If trust or activation setup is uncertain, inspect
it before sending a frozen task. Every raw work/resume capture must still contain
real SessionStart evidence. `collect-batch` accepts either eight explicit
`--raw-rollout` paths or one `--rollout-directory` containing exactly eight files.
Before changing campaign
state it maps the unordered captures to the sealed work/resume slots using the
frozen first task, exact workspace and revision, VS Code source/originator,
fresh session identity, and SessionStart activation. Ambiguous, missing,
duplicate, mismatched, or session-reused input is rejected globally.

For requested-language documents outside the fixed viewer locale (`en`/`en-*` or
`ko`/`ko-*`), prepare and fix active-host realizations after all eight raw rollouts
exist and before any intake. The candidate's `volicord-mcp` executable is required
and hash-bound during campaign preparation. The steward runs:

```text
rebuild/scripts/dogfood-campaign prepare-document-realizations \
  --campaign-root /absolute/private/campaign \
  --rollout-directory /absolute/private/raw-rollouts
```

This uses the same read-only global input mapping and Product `document_preview`
to obtain fingerprinted plans. It does not qualify sessions or change canonical
Project state. Give the active host only `realizer/index.json` and its referenced
plans/drafts. That plane contains opaque document identities, Project/candidate,
requested language, and bounded Product source plans; it contains no evaluator
assignment, expected outcome, or raw session data. Never put it in tracked state.

The active host fills each draft's `realization` with the exact plan fingerprint,
translated `title`, ordered `sections` (`key`, `title`, `claims`), and ordered claims
(`identity`, `text`), preserving every protected code/path term. Draft schema version 3
has exactly `schema_version`, `preparation_sha256`, `requested_language`,
`all_generated_prose_realized`, `provenance`, and `realization`. The realization has
no caller-supplied `generator`. Keep the prepared hash/language and set the prose
confirmation to `true` only after reviewing the complete body.

Keep `provenance.preparation_binding` unchanged: it verifies the prepared candidate and
local MCP executable hash, not the author or exact model. Without a bound runtime rollout,
each of `host`, `agent`, and `model` is either `{"state":"unknown","value":null}` or
`{"state":"self_reported","value":"the reported identity"}` and
`runtime_observation.state` remains `not_provided`. No exact host/model authorship
attestation is available on this path, so `verified` claims are rejected. The recorder
derives Product generator metadata with explicit unverified labels, including
`unknown (unverified)` for an unavailable model identity. The accepted structured
provenance and its exact bytes/hash remain immutable.

When the realizing Codex session's raw rollout is available, bind it before recording:

```text
rebuild/scripts/dogfood-campaign bind-document-realization-provenance \
  --campaign-root /absolute/private/campaign \
  --realization-id <opaque-document-id> --draft <private-draft-path> \
  --runtime-rollout /absolute/private/realizer.rollout.jsonl
```

The helper accepts one consistent host-recorded `turn_context.model` without a
model-name allowlist and records source/originator/model
as `runtime_observed`, bound to rollout hash, session and CLI version. This is stronger
than self-report but remains explicitly not an authorship attestation. Supply the same
`--runtime-rollout` to validate and record. A conflicting weaker self-report is rejected;
absent exact runtime metadata remains `unknown`.

Python never writes translated prose. Field text is bounded to 4,096 UTF-8 bytes;
each private preparation/draft is bounded to 2 MiB. The host then runs:

```text
rebuild/scripts/dogfood-campaign validate-document-realization \
  --campaign-root /absolute/private/campaign \
  --realization-id <opaque-document-id> --draft <private-draft-path>
rebuild/scripts/dogfood-campaign record-document-realization \
  --campaign-root /absolute/private/campaign \
  --realization-id <opaque-document-id> --draft <private-draft-path>
```

Preflight reads the realizer preparation, input, and inventory membership only.
Fix errors in the mutable draft and rerun preflight. Recording asks Product to
validate both formats against the current plan, then fixes the exact bytes/hash;
later draft edits cannot change that record. Fixed records cannot be overwritten.
All required records must be fixed before `collect-batch` can create staging or
publish immutable evidence. The preparation also binds all eight raw hashes and the
campaign hash. Missing realizer evidence is a preparation blocker, not a Product
crash. The single `collect-batch` collection path cannot bypass this step.
Product checks structure, grounding, protected terms and provenance; the active
host confirmation and existing human review still own semantic language quality.

After mapping and required realization checks succeed, `collect-batch` copies and hashes every rollout
byte-for-byte and publishes a candidate-bound `evidence-set.json`. Semantic work/resume
checks do not run during collection. Invalid activation and deterministic identity/hash
violations block publication. For each journey it derives the Project ID,
invokes the installed candidate's repository-selected
`context export --output`, completes descriptor evidence references and hashes,
and invokes the supported same-locale `document export`
path for `project-architecture-guide`, `decision-report`,
`implementation-plan`, and `handoff-resume` in both Markdown and
self-contained HTML. Cross-locale documents instead re-derive the current MCP
NarrativePlan and submit the exact fixed realization to Product `document_preview`.
The same realization serves both formats only after their plans compare equal;
the Product fingerprint is format-independent. Only Product-returned content is
saved as document evidence. Deterministic Work summaries retain raw and
Checkpoint evidence; each journey-final summary records every kind/format
status, bounded failure basis or relative evidence path, bytes, and
SHA-256; export failure remains explicitly failed. The public
`volicord-viewer --snapshot` capability also produces one self-contained,
read-only HTML snapshot with its Project/candidate basis, relative path, bytes,
and SHA-256. A bounded operator review
index lists the produced paths without evaluator material. The helper also
writes a bounded Runtime Home summary containing managed logical names and sizes,
derived-analysis size, configuration presence, and activation booleans; it
never reads or copies store, derived-analysis, credential, provider-payload,
prompt, or source-body contents. Production collection uses only `collect-batch`; per-Work extraction helpers are
limited to synthetic fixtures.

The generated documents and Viewer serve different review needs. Each document
uses a comprehension-first body and moves opaque identities, hashes, complete
capability inventory, and claim-level grounding into a distinct trailing audit
appendix or default-closed HTML disclosure. The live Viewer uses the current
human-first hierarchy. Its static snapshot is a separate read-only,
self-contained share/review artifact that works without a Runtime or listener;
it is not interchangeable with a generated document and does not share the
document-adoption lifecycle.

`finalize-manifest` deterministically assembles immutable `repositories.json` from collected
evidence. Collection is not qualification. Run non-mutating semantic analysis with:

```text
rebuild/scripts/dogfood-campaign evaluate --campaign-root /absolute/private/campaign
```

Evaluation publishes a new sibling directory with `evaluation.json` and a receipt;
`--output` chooses another new directory outside the Campaign. `--previous-evaluation`
retains an earlier run ID/hash for comparison. Product identity stays bound to the
evidence set even when the evaluator HEAD or policy changes. Raw files, Campaign
metadata and earlier results are never rewritten.

Prepare, validate, record and package common agent/human reviews using
[`qualitative-review.md`](../docs/design/qualitative-review.md). Every collected
Work is reviewable, including hard-blocked and indeterminate runs. The bounded
review package excludes evaluator answers, private profile, runtime and source
copies. Raw work/resume rollouts enter only with `--include-raw-rollouts` and remain
private. Schema-valid review is not Product passage.

Collect the seven maintained CLI journeys independently of accidental command use in
naturalistic chats. The collector uses one isolated ephemeral workspace and Runtime Home per
repository class, retains bounded process evidence, and leaves the Campaign unchanged:

```sh
rebuild/scripts/dogfood-campaign collect-cli-observations \
  --campaign-root /absolute/private/campaign \
  --output /absolute/private/cli-observation-run

rebuild/scripts/dogfood-campaign prepare-qualitative-review \
  --campaign-root /absolute/private/campaign \
  --output /absolute/private/review-run \
  --reviewer-kind agent \
  --review-session-id <actual-review-session-id> \
  --machine-evaluation /absolute/private/evaluation-run/evaluation.json \
  --cli-observations /absolute/private/cli-observation-run
```

Work-specific criteria cover all five Works, and journey-final criteria cover
all three retained projections. CLI criteria are generated once per repository
class, producing 21 required assessments rather than Work-level duplicates.
Missing one class observation leaves only that class's seven criteria
unresolved; it is never converted to `not_applicable`.

```sh
rebuild/scripts/dogfood-campaign qualify \
  --campaign-root /absolute/private/campaign \
  --candidate-head <original-product-candidate> \
  --machine-evaluation /absolute/private/evaluation-run/evaluation.json \
  --review-root /absolute/private/recorded-agent-review \
  --review-root /absolute/private/recorded-human-review \
  --gate-capsule /absolute/private/candidate-capsule.json \
  --gate-archive /absolute/private/candidate-archive.tar.gz \
  --output /absolute/private/qualification-run
```

This rechecks the existing candidate-specific technical gate using the maintained
independent archive verifier and capsule contract. It does not execute final,
provider qualification or V11. Unique naturalistic observations remain in the
machine run and common rubric. Missing technical evidence/review stays unresolved;
hard violations stay blocked. Only policy-permitted evidence-backed semantic
criteria can be resolved by agents. Live accessibility and actual user Decision
comprehension, plus explicit high-impact conflicts, require targeted human review.

An operator may authorize a fully qualified result with `approve-phase-9
--qualification <run>/qualification.json --operator <identity> --authorization
approve-phase-9 --output <new-approval-run>`. This explicit action verifies all
original inputs again and publishes an immutable approval containing the final
state. Evaluation and review never call it. Approval cannot fill missing evidence.
`validate-qualification --qualification <run>/qualification.json` independently
rechecks the maintained policy and exact inputs without publishing a new run.

This distinction does not change admission, exact final, official V11, gate
ownership, or the capsule lifecycle described below.

For a new campaign, the candidate is the current clean Git `HEAD`, and it is
valid only when an independently verified exact-candidate technical-gate
capsule and archive identify that same commit.

<!-- phase8-active-operations:end -->

## Historical Phase 8 result record

The maintained Phase 8 candidate-authority summary is `phase-8-summary.md`.
The values below record prior results only and do not select a new campaign
candidate.

The sealed candidate retains current Codex exec/command-role evidence normalization,
exact successor-Review Decision lineage, separate redundant-Question detection,
and active-host cross-locale realization preparation/record/collection. Active
Codex guidance now consumes a clear answer using its existing valid presentation
receipt and current user turn, and executes explicitly requested resume verification
instead of stopping at Recall or inspection. Legitimate clarification, inspection-only
requests and numeric failure/indeterminate outcomes retain their existing boundaries.

The latest terminal campaign, `phase8-naturalistic-20260910-60ffad042346`, used
`de4c6d1bea16947616ed6fb78baa8f8c321ef4ff` and remains failed, diagnostic-only,
and non-reusable. Its five bounded findings and committed remediation are recorded
in `dogfood/report.md`. Historical learning-continuity and older campaigns remain
separate diagnostics. No fresh naturalistic campaign has run for the new candidate;
human review is `not_provided`, replacement qualification remains pending,
`replacement_pass_candidate = false`, and `phase_9_ready = false`.

The operator workflow is batch-first: after hidden
evaluator material is independently reviewed and sealed, the user approves
repository/hook trust, completes all eight fresh naturalistic chats without
per-session evidence processing, and supplies the raw rollouts once to
`collect-batch`. The helper derives journey/Work/session mapping, three
journey-final bundles, bounded Runtime and activation summaries, four document
kinds per journey, and static Viewer snapshots.
Evaluation may complete without qualitative review; unresolved criteria stay
unresolved and confirmed hard violations cannot be waived by any reviewer.

Predecessor Dogfood descriptors, captures, Runtime Homes, workspaces, bundles,
observations, and session identities remain non-reusable for a future candidate.
Any predecessor Small Python cycle is historical diagnostic evidence only and is not qualifying
evidence for the redesigned campaign.
Replacement passage remains not established, and Phase 9 may not begin.
Qualifying Dogfood must run from a separate clean worktree whose actual Git
`HEAD` exactly matches the candidate identified by its own
successful maintained gate and verified capsule/evidence archive. Historical
candidate `6031641c46cf014a754442dcee3137caf265882e` and any later documentation
HEAD remain distinct; neither can qualify a different HEAD through a helper
argument.

Technical V11 is a deterministic rehearsal across the three maintained repositories
and fresh Runtime Homes. Its `ordinary_work` writes a disposable marker after
readiness and checks that the Guarded store hash is unchanged; it does not perform
naturalistic source/test/config behavior work. Phase 8's repeated campaign and
qualitative review own work quality, practical context recovery, Question relevance,
interruption cost and document usefulness. A successful technical gate opens
technical entry only. Phase 9 requires the separate qualified state and explicit
operator approval.

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

Optional cheap diagnostic example (the gate performs its own admission):

```text
rebuild/scripts/validate admission \
  --external-network available \
  --authorize-external-transmission v11-openai-codex-project-health-three-targets \
  --authorize-provider-source-transmission openai-codex-background-semantic-bounded-rust-v1 \
  --provider-model <exact-model>
```

Authoritative exact-candidate gate example (no preceding admission is required):

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

Independent archive verification proves membership, hashes, modes, bounds,
candidate agreement and prohibited-content integrity. It does not replay Final,
provider or V11 commands, inspect mapped test semantics, or independently establish
naturalistic work quality. The gate owns technical execution truth. Complete stdout/stderr, detailed V11 local
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

- mapped contract-test execution owner/status/count from Final metadata and test output;
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
evidence. A successful Final with failed mapped-test execution remains blocked,
with `contract_coverage_execution_failed` and no provider or V11 invocation. A V11-preflight failure requires the successful same-gate final and
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

The finite machine authority/rationale/owner table is `dogfood/machine-policy.json`.
Intact older evidence uses `dogfood-campaign evaluate` without changing its Product
candidate. An incomplete historical campaign without `evidence-set.json` may use
`dogfood-campaign diagnose --campaign-root <old> --output <new>` for a separate, explicitly
non-qualifying inventory diagnostic. Neither path rewrites historical rejection/raw data.

Use `validate-approval --approval <approval-run>/approval.json --qualification
<qualification-run>/qualification.json` to recheck a recorded authorization without
creating another approval. The original qualification input references must remain
available for this independent check.

Naturalistic target journey collection observes Git state without committing or cleaning it.
The pinned baseline and chronological descendant session HEADs remain required. Final HEAD may
remain unchanged with staged, unstaged or untracked task changes. Journey-final evidence retains
repository-state.json, staged.patch and unstaged.patch with deterministic content/mode hashes and
an overall fingerprint; ignored files are outside this Git observation boundary. Publication
rechecks the final target state and rolls back on change. Historical inspection verifies retained
bytes without requiring the original workspace. This does not establish actor attribution or
verification success. The Product candidate workspace remains strictly clean and exact-HEAD-bound.

Blind preparation now requires `assessments` (1–32 independently discovered dimensions).
Each has reviewer-local stable `dimension_id`, bounded `outcome_scope`, one reviewer-safe
classification, explicit `applicable` or `not_applicable` judgment, materiality/unavoidability/
disclosure conclusions, bounded source-grounded reasoning and typed reviewer-visible
provenance indices. The scalar classification remains a
summary only and cannot establish independent coverage. A missing scope has no assessment; it is never inferred to be an explicit negative judgment.
The fixed negative judgment names a reviewer-discovered scope and class with reviewer-visible
source reasoning. Its exact bytes and hash are bound before reveal. Neither preflight nor recording reads
private assignments, profile placement or evaluator scopes. Provisional bytes remain immutable.
After all five provisionals are fixed and the profile is revealed, `obligation_coverage` maps each
evaluator obligation to a distinct fixed dimension, retaining its exact reviewer scope and naming
the evaluator scope with source-grounded equivalence reasoning. This is a steward semantic judgment,
not a text-similarity, Question or alternative oracle. Missing/unseen/reused dimensions or rewritten
reviewer scope remain explicit `blind_coverage_gap`. A matching fixed `not_applicable`
assessment instead becomes `applicability_disagreement`, with per-obligation evidence-bound
`evaluator_correct`, `reviewer_correct`, or `unresolved_conflict` resolution. An unresolved conflict
blocks sealing. `resolved_from_evidence` can settle facts, authority, or applicability for a scope
fixed before reveal; it cannot discover another dimension for coverage. A reviewer-correct result
preserves the assigned positive evaluator obligation as `evaluator_obligation_invalid` and blocks
replacement qualification. Sealing and qualification reject genuine gaps independently of common
qualitative review. Compatible learning deliberation and routine-control obligations require
separate blind assessments.

The current steward reconciliation lifecycle is reveal → `prepare-reconciliation` → edit/compare
→ `validate-reconciliation` → `seal-work`. Preparation copies the maintained private evaluator
input and binds the immutable provisional; it does not fill in missing independent assessments.
Mutable staging is `evaluator/reconciliation/<review_slot_id>/draft.json` under the private
campaign root, outside the operator/reviewer planes. `validation.json` binds exact draft bytes,
candidate, slot, preparation and provisional hashes, and the normalized descriptor semantic hash.
Any draft edit makes validation stale and requires revalidation. `inspect-reconciliation` reports
not prepared, unvalidated, validated, stale validation or sealed state without changing evidence.
Sealing has no external descriptor argument; it consumes only the campaign-owned validated draft,
rechecks the same review/provenance contract, and creates the authoritative descriptor exactly once.
The validation receipt becomes inventory-bound immutable evidence at seal. The draft remains
non-authoritative mutable staging outside artifact inventory; after seal it may be removed without
changing the immutable receipt/descriptor or provisional evidence. Mutable staging is never copied
into batch evidence, operator run sheets, blind reviewer preparation, or reviewer archives. A
symlink or path escape cannot redirect reconciliation outside the private campaign root. `/tmp`
is not required for this workflow. Before seal, preserve private staging for diagnosis rather than
editing immutable evidence or bypassing failed validation.

The maintained campaign self-test and remediation integration include real-Git collector,
publication-tamper/atomicity, blind coverage-gap, and campaign-private reconciliation regressions.
Admission's dogfood support checks consume these same tests. `gate-self-test` also guards dirty
Product candidate rejection without executing a real final aggregate/provider/V11.
These deterministic checks are support evidence only; changed candidates require fresh naturalistic
rollouts, independent review and replacement qualification. Previous failed captures remain diagnostic.

`classification_comparison.provisional_classification` retains the sorted unique positive
classifications of the immutable assessment collection. Disagreements are calculated per obligation
against its paired fixed dimension's applicability/classification/materiality/unavoidability/
disclosure conclusions; the scalar summary does not determine agreement. Fully matching multi-obligation mappings use `agreed`.
Evidence-resolved paired disagreements remain explicit, even when a scalar summary matches the
evaluator. Additional independently reviewed dimensions remain preserved without serving as an oracle.
