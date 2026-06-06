# Authoring Guide

Use this guide before changing Harness documentation. It is a maintenance manual for documentation work only. It does not authorize Harness Server/runtime implementation, product-repository writes, generated operational artifacts, conformance runners, runtime state, projections, evidence records, QA records, Acceptance records, close records, or residual-risk records.

The repository is still in documentation review/redesign. The current docs are a post-redesign review baseline, not an accepted implementation-ready server plan. Bold rewrites are allowed when old prose conflicts with the clarified product thesis, owner boundaries, Korean quality rules, or implementation feasibility.

## Mandatory pre-edit checklist

- [ ] Read root `AGENTS.md` first.
- [ ] Read this guide before any documentation edit.
- [ ] For bilingual edits or terminology-affecting edits, read `docs/en/maintain/translation-guide.md`.
- [ ] Before touching Korean docs, read `docs/ko/maintain/authoring-guide.md` and `docs/ko/maintain/translation-guide.md`.
- [ ] Confirm the work is documentation-only. Do not implement server/runtime code, product code, generated operational files, runtime state, executable fixtures, conformance runners, projections, or artifact outputs.
- [ ] Identify the document family: Start, Use, Build, Reference, Maintain, Later, or Roadmap.
- [ ] Identify the owner document for any strict contract you might touch. One strict contract has one owner document.
- [ ] Decide whether the edit changes meaning, only fixes wording, only fixes links, renames/moves content, or deletes content.
- [ ] For a meaning change, work in `docs/en` first and mirror the same meaning in `docs/ko` in the same batch.
- [ ] For a Korean edit, preserve exact identifiers and write natural Korean technical prose. Do not make Korean a line-by-line English copy.
- [ ] For user-facing docs, start from the user-visible situation. Do not start from internal labels such as `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`.
- [ ] For stage or MVP wording, confirm the active stage boundary in the relevant Build owner. Do not present later-profile or Roadmap material as MVP requirements.
- [ ] For security wording, identify the actual guarantee level before writing the claim.
- [ ] For moves, renames, splits, merges, or deletions, plan the link and anchor updates in both languages before editing.

## Mandatory post-edit checklist

- [ ] Verify no runtime/server implementation, product code, generated operational artifact, runtime state, executable fixture, conformance runner, projection, or artifact output was created.
- [ ] Verify documentation files are described as source material, not runtime state, generated projections, evidence, QA, Acceptance, residual-risk records, close records, or operational truth.
- [ ] Verify strict schemas, DDL, enum values, state transitions, gate rules, algorithms, fixture body shapes, template bodies, storage rules, security guarantees, and official definitions remain in their owner documents.
- [ ] If the edit touches schema/API/storage contract wording, verify active schema blocks contain only active enum values; later/profile values stay in `schema-later.md`, Later docs, or explicitly labeled later/profile owner sections.
- [ ] Verify prose-only stage gating does not substitute for actual schema, API, or DDL separation. If a value or field is not active, the active owner contract must exclude it or route it to a later/profile owner.
- [ ] Verify active API docs reference only schema types defined by active API schema owners, or explicitly route later/profile types to `schema-later.md`.
- [ ] Verify API, Core, and Storage owner docs cover the same fields and value boundaries for shared concepts such as Write Authorization.
- [ ] Replace non-owner duplicate contracts with a short reader-facing summary plus an owner link.
- [ ] Verify localized display labels, including `display_label` values, are never treated as canonical schema, enum, storage, or API values.
- [ ] Verify Discovery or `shared_design` output says whether it is canonical active state, projection/derived display, support text, or later/profile material.
- [ ] Verify MVP close response wording does not expose later verification, Manual QA, or full Evidence Manifest semantics as active `CloseTaskResponse` behavior.
- [ ] Verify conformance examples use structured state, storage, event, error, artifact-ref, blocker, or guarantee assertions instead of prose-only expectations.
- [ ] Verify Build docs do not redefine exact schema, DDL, API, storage, or transition contracts owned by Reference docs.
- [ ] Verify cleanup work did not incidentally change readiness, handoff, implementation acceptance, coding acceptance, or final documentation acceptance status values owned by maintainer handoff or decision-log sections.
- [ ] Verify user-facing docs begin with ordinary user situations, expected agent clarification, visible blocks, needed judgment, or close outcome before internal terms.
- [ ] Verify stage wording does not overclaim implementation status and does not treat later-profile or Roadmap material as an MVP requirement.
- [ ] Verify security wording matches the documented guarantee level and does not imply OS permissions, arbitrary-tool sandboxing, tamper-proof files, pre-tool blocking, or isolation unless that exact mechanism is proven and owned.
- [ ] Verify English/Korean paired docs preserve the same meaning, owner links, stable identifiers, and active file coverage.
- [ ] Verify Korean prose is natural Korean and preserves exact identifiers, file paths, schema/API names, enum values, error codes, validator IDs, and code-like strings.
- [ ] Verify moved, renamed, split, merged, or deleted content has updated links, anchors, README routes, paired-language links, and old title/path references in the same batch.
- [ ] Run the relevant docs-maintenance checks you can run locally, or state which checks were not run.
- [ ] Before final documentation acceptance or a major review handoff, use [Documentation Checks](documentation-checks.md) and report whether each check was manual, scriptable, or future-runtime-only.
- [ ] For final redesign handoff, create or update [Rewrite Acceptance Review](rewrite-acceptance-review.md) without claiming runtime conformance or implementation readiness.
- [ ] Route any remaining issue as documentation drift, schema/design decision, stage boundary decision, implementation-readiness criterion, or future Roadmap item. Do not leave vague TODOs scattered through active docs.
- [ ] Report changed files and any residual risks or unresolved checks in the handoff.

## Core principles preserved during redesign

Preserve these principles even when terminology, schemas, document structure, or stage boundaries change:

- Harness is not a prompt pack. It is a local authority record for scope, user-owned judgment, evidence, verification, QA expectations, final acceptance, residual-risk status, and close readiness.
- User-owned judgment stays with the user unless an owner contract explicitly says otherwise. Product decisions, material technical decisions, QA expectations, waivers, final acceptance, and residual-risk acceptance must not be silently delegated to the agent.
- Evidence, verification, Manual QA, final acceptance, close readiness, and residual risk are separate records and judgments. None substitutes for another.
- Chat, connector output, Markdown-rendered projections, and generated documents are not operational truth. Core-owned local state and artifact references are the future operational authority.
- Documentation files are source material for understanding and implementing Harness. They are not Harness runtime objects, runtime state, generated artifacts, projections, evidence, QA, Acceptance, close, or residual-risk records.
- Current documentation is a review baseline. Do not describe it as fully accepted, implementation-complete, implementation-ready, or ready for server coding unless the maintainer handoff owner explicitly says so.

During redesign, optimize for clarity, implementability, and the product thesis. Rewrite, move, merge, shrink, or delete old prose when it makes Harness look like a broad workflow engine, ALM system, evaluation harness, QA automation platform, report generator, generic MCP wrapper, or prompt pack.

## Document-family ownership rules

Use the documentation tree as an ownership split:

| Family | Role | Boundary |
|---|---|---|
| Start | Explain why Harness exists, what concepts mean, how one ordinary task feels, and where the current guarantee boundary sits. | Do not define strict schemas, gates, DDL, implementation sequences, or fixture mechanics. |
| Use | Explain how users and agents interact with Harness. | Name low-level contracts only when needed for user trust, a visible blocker, a judgment request, evidence gap, recovery path, or close result. |
| Build | Explain implementation sequence after documentation acceptance and a separate implementation-planning readiness decision. | Keep stage goals, sequencing, runnable-slice planning, and exit criteria. Link to Reference for exact schemas, gates, DDL, APIs, storage, fixtures, and security contracts. |
| Reference | Own exact contracts, schemas, algorithms, storage, APIs, security model, projection behavior, and official definitions. | Include enough context to understand the contract. Do not turn Reference docs into tutorials, reader journeys, or staged implementation plans. |
| Maintain | Govern documentation maintenance. | Define authoring, translation, review, link, ownership, and docs-maintenance rules only. Do not define runtime behavior, runtime conformance pass/fail, or implementation readiness. |
| Later and Roadmap | Hold future or candidate material outside the active MVP path. | Do not make candidate material look like an active stage requirement unless an owner document promotes it with scope, fallback behavior, and proof expectations. |

README pages are routing pages before they are explanations. They should briefly say what Harness is and is not, then route first-time readers, users, implementers, Reference readers, and maintainers to the right owner docs.

## One-owner-per-contract rule

Every strict contract has one owner document. The owner is the only place to define exact fields, enum values, DDL, schemas, algorithms, state transitions, gate rules, fixture body shapes, template bodies, storage rules, security guarantees, error precedence, and official definitions.

Other document families may name the reader-facing consequence and link to the owner. They must not create a second definition. If a local explanation needs a full table, schema block, DDL block, transition matrix, fixture mini-language, gate matrix, validator table, template body, or algorithm, it belongs in the owner Reference document.

When a non-owner document repeats normative language, fix it in this order:

1. Identify the owner document or owner section.
2. Update the owner first if the contract itself is wrong or incomplete.
3. Replace the duplicate with one plain-language sentence, one owner link, and any local consequence for the reader.
4. Mirror semantic changes in the paired language file.
5. Repair links and anchors after the owner boundary is clear.

### Reference contract owner map

Use this map before adding a strict rule:

| Contract area | Owner document | Owner boundary |
|---|---|---|
| Core Model | [Core Model Reference](../reference/core-model.md) | Invariants, entity relationship semantics where they affect state, lifecycle and state transitions, gates, `prepare_write`, Write Authorization, `record_run`, close semantics, waivers, and non-substitution rules. |
| MCP API | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md), and [API Schema Later](../reference/api/schema-later.md) | Active MVP-1 tools, public MCP resources, common envelopes, request/response schemas, shared refs, public errors, idempotency/replay, state conflict behavior, `ValidatorResult`, API `ArtifactRef`, and later-profile API material. |
| Storage | [Storage](../reference/storage.md) | Runtime home layout, persisted state, SQLite DDL profiles, storage-owned JSON `TEXT`, enum hardening, migrations, locks, artifact storage, baseline capture, projection job table, and validator-run storage. |
| Projection | [Projection And Templates Reference](../reference/projection-and-templates.md) and [Template Reference](../reference/templates/README.md) | Derived view rules, output tiers, managed blocks, human-editable sections, artifact-ref rendering, projection freshness/failure behavior, and full rendered template bodies. |
| Security | [Security Reference](../reference/security.md) | Threat model, assets, trust boundaries, threat/control categories, high-risk control expectations, local access security posture, guarantee-level meanings, and honest-display rules. |
| Conformance | [Conformance Fixtures Reference](../reference/conformance-fixtures.md) and [Future Fixtures](../later/future-fixtures.md) | Conformance Fixtures owns the three-layer boundary, core conformance model, MVP behavior examples, future fixture body shape, future runner behavior, assertion semantics, future fixture profiles, suite metadata boundaries, current-phase status, and reduced Kernel Smoke queue. Future Fixtures owns compact future scenario-family inventory, promotion criteria, suite-family labels, and catalog-only future candidates outside the active MVP path. |
| Operations | [Operations And Conformance Reference](../reference/operations-and-conformance.md) | Operator behavior, staged operator surface, diagnostics, `connect`, `doctor`, `serve mcp`, projection refresh, reconcile, recover, export, artifact checks, future conformance run entrypoint, and documentation-check/docs-maintenance reporting boundary. |
| Agent Integration | [Agent Integration Reference](../reference/agent-integration.md) and [Surface Cookbook](../reference/surface-cookbook.md) | Connector capability profiles, generated manifests, context push/pull profiles, fallback semantics, Role Lens, reference-surface behavior, connector conformance overview, and surface-specific recipes. |
| Glossary | [Glossary Reference](../reference/glossary.md) | Public and internal terminology definitions, capitalization, official term wording, record-name orientation, and owner routing. |
| Runtime Architecture | [Runtime Architecture Reference](../reference/runtime-architecture.md) | The three spaces, Core process placement, Core-only canonical mutation authority, transaction ordering, artifact/projection/reconcile placement, and architecture-level failure and recovery overview. |
| Design Quality | [Design Quality Policies](../reference/design-quality-policies.md) | Policy contracts, policy-to-validator mapping, stable validator IDs, severity composition, policy waiver semantics, evidence expectations, and design-quality gate/close impact. |

### Pre-implementation repair target owner map

Use this map for known documentation repair targets in the pre-implementation review baseline. It routes drift to the owner document family before future edits repair summaries or links. It does not accept the docs, prove implementation readiness, authorize server/runtime implementation, or create Harness runtime objects.

| Repair axis | Canonical owner family | Non-owner docs may summarize or link from | Docs-maintenance `FAIL` symptom |
|---|---|---|---|
| Owner contract collisions | The strict owner in the [Reference contract owner map](#reference-contract-owner-map); [Glossary Reference](../reference/glossary.md) for term definitions | Start, Use, Build, Maintain, README pages, and Reference index routes | Two active docs define the same strict contract differently, or a non-owner contains a full schema, DDL, transition table, gate matrix, enum table, validator table, template body, projection table, or glossary definition instead of linking to the owner. |
| API/schema collisions | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md), and [API Schema Later](../reference/api/schema-later.md) | Use/Agent guides, Build stages, Operations/Conformance summaries, Surface Cookbook, and README routes | Method names, request/response fields, shared refs, envelopes, errors, `ValidatorResult`, `ArtifactRef`, or later-profile branches disagree with API owners or appear as active requirements before the owning tool/profile is active. |
| Active schema and later/profile enum leakage | [API Schema Core](../reference/api/schema-core.md), [API Schema Later](../reference/api/schema-later.md), [MVP API](../reference/api/mvp-api.md), and active Build stage owners | Build summaries, Use examples, Reference introductions, and Maintain checks | An active schema block lists later/profile enum values, compatibility-only values, locale labels, or Roadmap values, or relies on prose nearby to say those active-schema values are not actually active. |
| Undefined active API schema types | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Schema Later](../reference/api/schema-later.md), and [API Errors](../reference/api/errors.md) | Build docs, Surface Cookbook, Agent Integration, Operations summaries, and conformance examples | An active API method or response names a schema type, shared ref, error shape, `CloseTaskResponse`, `ArtifactRef`, `ValidatorResult`, or envelope member that is not defined by the active schema owner or clearly routed to a later/profile owner. |
| Storage/DDL collisions | [Storage](../reference/storage.md) | Build implementation sequencing, Runtime Architecture, Operations diagnostics, Conformance fixture notes, and README routes | Table names, DDL profiles, JSON `TEXT`, enum hardening, migration/lock/artifact/projection-job storage rules, or persisted status values differ from Storage or become duplicated outside Storage. |
| Core transition collisions | [Core Model Reference](../reference/core-model.md), with architecture placement in [Runtime Architecture Reference](../reference/runtime-architecture.md) | Start/Use task explanations, Build stage summaries, API/storage/projection references, and template summaries | Task/run/gate state transitions, `prepare_write`, Write Authorization, `record_run`, waiver, close, non-substitution rules, or Core-only mutation authority conflict with Core owner language or are treated as display-only/report-only behavior. |
| Cross-owner field coverage gaps | [Core Model Reference](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), and [Storage](../reference/storage.md) for shared concepts | Build stage summaries, Surface Cookbook, Operations summaries, conformance examples, and README routes | A shared concept such as Write Authorization has fields, status/value sets, row boundaries, response semantics, or storage implications in one owner but not the others, or summaries add/drop fields without routing the mismatch as a schema/design decision. |
| Active MVP vs later/profile boundary drift | [MVP Plan](../build/mvp-plan.md) and Later profile docs | Reference introductions, Use examples, README routes, Roadmap entries, and Build summaries outside the stage owner | Engineering Checkpoint, Kernel Smoke, MVP-1, later-profile, or Roadmap material is promoted or required by wording without the stage/profile owner promoting it with scope, fallback behavior, and proof expectations. |
| Locale label vs schema value drift | [API Schema Core](../reference/api/schema-core.md), [MVP API](../reference/api/mvp-api.md), [Glossary Reference](../reference/glossary.md), and [Translation Guide](translation-guide.md) | Use examples, Korean docs, template display examples, status cards, and migration notes | Locale-specific labels such as `제품 판단`, `기술 판단`, `범위 판단`, or `display_label` strings are written as canonical schema identifiers, enum values, storage values, API fields, or owner contract names instead of rendered labels derived from stable identifiers such as `judgment_kind`. |
| Discovery and shared design authority ambiguity | [Core Model Reference](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [Projection And Templates Reference](../reference/projection-and-templates.md), and relevant Later profile owners | Start/Use Discovery examples, Build discovery summaries, `shared_design` examples, templates, and status/projection summaries | Discovery or `shared_design` output does not state whether it is canonical active state, projection/derived display, support text, or later/profile material, or it treats support text/projection content as Core-owned state. |
| Evidence sufficiency and close-readiness ambiguity | [Core Model Reference](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), and [Design Quality Policies](../reference/design-quality-policies.md) where design validators affect close | User Guide, One Task, user-owned judgment examples, Build close criteria, Projection/template summaries, and README routes | Evidence, verification, Manual QA, final acceptance, residual-risk acceptance, waiver, and close readiness substitute for each other, or close is described as possible while required evidence, judgment, QA, verification, acceptance, or risk blockers remain unresolved. |
| MVP close response overexposure | [Core Model Reference](../reference/core-model.md), [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), and Later profile docs for future assurance material | Use close examples, Build close criteria, Projection/template summaries, Conformance examples, and Operations summaries | Active MVP `CloseTaskResponse` wording exposes later verification, Manual QA, detailed Evidence Manifest, detached verification, Eval, or full assurance-profile semantics as active response fields or active close requirements. |
| Security/local-access guarantee overclaim risk | [Security Reference](../reference/security.md), plus the exact API, storage, Core, connector, operations, or conformance owner for the covered operation | Start/Use status wording, Agent Integration, Surface Cookbook, Build stage summaries, Operations diagnostics, and README routes | Cooperative or detective behavior is described as preventive, isolated, OS-permission, arbitrary-tool sandboxing, tamper-proof, or pre-tool blocking without an owner-documented mechanism and proof path for that operation. |
| Conformance proof scope ambiguity | [Conformance Fixtures Reference](../reference/conformance-fixtures.md), [Future Fixtures](../later/future-fixtures.md), and [Operations And Conformance Reference](../reference/operations-and-conformance.md) for entrypoints/reporting | Build readiness summaries, Documentation Checks, Reference README, Roadmap, and operations summaries | Documentation checks, MVP behavior examples, future fixtures, and runtime conformance are blurred; examples are called runnable suites or current pass/fail criteria, or docs-maintenance results are treated as runtime conformance. |
| Conformance assertion structure drift | [Conformance Fixtures Reference](../reference/conformance-fixtures.md), [Operations And Conformance Reference](../reference/operations-and-conformance.md), and [Future Fixtures](../later/future-fixtures.md) | Build readiness summaries, Documentation Checks, Reference README, Roadmap, and operations summaries | Conformance examples rely on prose-only expectations or rendered Markdown instead of structured assertions over Core state, storage rows, events when stable, errors, artifact refs, blockers, and guarantee facts. |
| User output vs agent context mixing | [Agent Integration Reference](../reference/agent-integration.md), [Surface Cookbook](../reference/surface-cookbook.md), and [Projection And Templates Reference](../reference/projection-and-templates.md) | User Guide, Agent Guide, templates, README routes, Start examples, and Operations summaries | User-facing summaries, status cards, projections, retrieved context, or chat memory are treated as operational authority, write authorization, or enough to satisfy gates, user judgments, evidence, QA, final acceptance, residual-risk acceptance, or close. |
| Design-quality policy overactivation | [Design Quality Policies](../reference/design-quality-policies.md), with close/gate consequences in [Core Model Reference](../reference/core-model.md) | Use examples, Build stage docs, MVP/later profile docs, templates, README routes, and design summaries | Design validators, Manual QA/TDD expectations, policy waivers, or evidence expectations apply to ordinary work beyond the owner activation rules, or optional/later design-quality checks become MVP blockers by summary wording. |
| Build/reference contract redefinition | Reference owner docs named in the [Reference contract owner map](#reference-contract-owner-map), with Build owners limited to sequencing and stage criteria | Build stages, implementation overview, engineering checkpoint, MVP-1 planning docs, and README routes | A Build doc defines exact schema, DDL, enum, table/column, API response, transition, fixture body, or storage rule instead of linking to Reference, or its local summary disagrees with the Reference owner. |
| Readiness/status value drift | [MVP Plan](../build/mvp-plan.md) and maintainer-owned status sections | Maintain docs, rewrite reviews, Build summaries, README routes, and cleanup batch notes | Cleanup work changes documentation acceptance, implementation-planning readiness, server-coding acceptance, coding decision, handoff, or final documentation acceptance status values without an explicit maintainer decision. |

## Stage/MVP boundary rules

Keep documentation review, implementation-planning readiness, and runtime implementation status separate:

- Documentation review status: the current documentation set is in post-redesign review and is a documentation acceptance candidate for maintainer review.
- Implementation-planning readiness: not accepted yet. Maintainers must deliberately confirm implementation-readiness criteria before first runtime-batch planning.
- Runtime implementation status: not started. This repository is documentation-only now. Its intended future role is the Harness Server source repository, but server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision.

Use active delivery labels consistently:

| Label | Boundary |
|---|---|
| Engineering Checkpoint | Internal authority-loop smoke. It is not the product MVP and not the first user-value slice. |
| Kernel Smoke | Narrow future smoke-check authoring label under Engineering Checkpoint. It is not a stage name. |
| MVP-1 User Work Loop | First narrow user-value milestone. |
| Assurance Profile | Later hardening for agency assurance behavior. |
| Operations Profile | Later hardening for operations and handoff behavior. |
| Roadmap | Future scope unless owner docs promote and prove an item. |

Do not present later-profile, diagnostic, operations, conformance-runner, or Roadmap material as MVP requirements. Reference-schema presence does not by itself expand the smallest runnable slice. Required fields apply when the owning tool, record, or profile is active or used.

Major implementation decisions found during review belong in [MVP Plan: Implementation decisions before server coding](../build/mvp-plan.md#implementation-decisions-before-server-coding). Do not leave major decisions as scattered `TODO_DECISION` markers in active docs. The short maintainer handoff status is owned by [MVP Plan: Repository status](../build/mvp-plan.md#repository-status).

Documentation editing in this repository does not require Harness runtime procedures. Do not run or simulate `prepare_write`, MCP state transitions, `close_task`, runtime state, `task_events`, Write Authorizations, Evidence Manifests, Manual QA records, Acceptance records, Residual Risk records, Journey Cards, generated projections, or generated operational/projection documents for docs work.

## User-facing terminology rules

User-facing docs must start from the user-visible situation, not internal labels.

Write what the user can ask, what the agent should clarify, what Harness preserves, what is blocked, what judgment is needed, what evidence exists, and what close means. Introduce internal terms only after the plain situation is clear and only when the term helps the reader act, interpret a visible blocker, or follow a Reference link.

Do not require users to know or say labels such as `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`. Prefer ordinary examples such as:

```text
Help me clarify the plan before implementation.
Show what I need to decide and what you can verify.
Tell me before the work scope expands.
```

Introduce concepts through examples before strict definitions. A Start or Use page should not open with a dense definition list unless it is explicitly a glossary or reference table.

Use docs should stay at the user trust boundary. They may mention a contract that explains a visible hold, blocker, decision prompt, evidence gap, close result, or recovery path, but they should not expose low-level field lists, storage rows, gate matrices, or validator internals unless the user needs that detail to make a judgment.

## Security wording rules

Security wording must match the actual documented guarantee level.

- Use cooperative wording when Harness can guide or record expected behavior but cannot technically block the action.
- Use detective wording when Harness can detect or report after the action.
- Use preventive wording only when the documented surface can block before the covered action and the blocking path is proven for that operation.
- Use isolated wording only when a documented separation boundary exists. Name the boundary and do not imply broader OS sandboxing or permission isolation.

Do not imply early Harness provides OS-level permissions, arbitrary-tool sandboxing, tamper-proof local files, pre-tool blocking, or security isolation unless the exact mechanism is documented and proven for the covered operation. Write Authorization is a cooperative Harness record/check, not OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation.

[Security Reference](../reference/security.md) owns threat concepts and honest guarantee display. Exact API, storage, Core, connector, operations, and conformance behavior stays with those owners.

## Bilingual parity rules

English docs define the reference meaning for the bilingual documentation set. Korean docs preserve that meaning, but they should read like natural Korean technical documentation.

- Keep paired English/Korean files semantically aligned: same active file map, reader purpose, section coverage, owner links, and contractual detail.
- Meaning changes in `docs/en` must be mirrored in `docs/ko` in the same batch. The reverse is also true.
- Heading text and paragraph grouping may differ when the Korean version is clearer and reviewability remains intact.
- Preserve exact API names, schema names, enum values, DDL names, code identifiers, field names, file names, path names, stable identifiers, error codes, validator IDs, and other code-like strings.
- In Korean user-facing prose, prefer natural Korean first. Add stable English identifiers in parentheses only when recognition, searchability, or exact contract alignment matters.
- Keep Korean sentences short where useful. Do not make Korean prose mostly English nouns joined by Korean particles.

For terminology details, use the relevant [English Translation Guide](translation-guide.md) and [Korean Translation Guide](../../ko/maintain/translation-guide.md).

## Link, rename, and deletion hygiene

When you rename, move, split, merge, or delete documentation, update links in both languages in the same batch.

Before the edit, identify old paths, old anchors, old headings, old title text, README routes, owner links, template/reference links, and paired-language links. After the edit, search for them and update every active reference.

Prefer links to owner documents or owner sections over links to secondary summaries. Do not point active owner links to removed migration context, inactive paths, or old structures.

If old names, old structures, or migration decisions must be retained for review, keep them in a clearly labeled non-active migration record. Active docs should describe the current structure and link to current owners.

When deleting content, decide whether it is obsolete, duplicated outside an owner, moved to another owner, or future Roadmap material. Replace active references with the new owner link or remove the reference in the same batch. Do not leave dangling anchors or stale route text.

## Known redesign issues and regression checks

Use this section as an actionable review checklist for drift that commonly returns during redesign. It replaces the old known-issue tracker. These checks are documentation-maintenance risks, not runtime implementation tasks, runtime conformance, acceptance records, or proof of implementation readiness.

When a risk is confirmed, route it to exactly one of these categories and name the affected owner, stage, and action needed:

| Category | Use when |
|---|---|
| Documentation drift | The fix is wording, owner-boundary cleanup, link repair, open-marker hygiene, terminology, or English/Korean parity. |
| Schema/design decision | The fix requires a real choice in schema, state, API, DDL, security guarantee, fixture semantics, or another owner contract. |
| Stage boundary decision | The fix requires deciding whether a capability belongs in Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, Operations Profile, or Roadmap. |
| Implementation-readiness criterion | The item must be true before maintainers accept first runtime-batch planning. |
| Future Roadmap item | The item is useful later and remains outside Engineering Checkpoint through Operations Profile unless promoted. |

Do not call a finding non-blocking unless you name what stage it does not block and what later stage it may block. Do not hide implementation-readiness concerns under vague follow-up wording.

| Redesign risk | Regression check | Default route if confirmed |
|---|---|---|
| Repository identity drifts into "implementation exists now." | Entrypoints and handoff sections say the repo is documentation-only now, in post-redesign review, intended as the future Harness Server source repository, and not ready for server/runtime implementation without documentation acceptance plus a separate implementation-planning readiness decision. | Implementation-readiness criterion |
| Stage names imply Engineering Checkpoint or Kernel Smoke is the product MVP. | Engineering Checkpoint is described as an internal authority-loop smoke, Kernel Smoke as a narrow future smoke-check authoring label, and MVP-1 User Work Loop as the first user-value slice. | Stage boundary decision |
| User-facing docs open with heavy implementation disclaimers. | Start and Use openings start with what users can ask, what the agent clarifies, what Harness preserves, and what users see; detailed status warnings route to README, Build handoff, and Maintain docs. | Documentation drift |
| User-facing docs overuse internal terms. | User-visible situations come before labels such as `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, and `task_events`. | Documentation drift |
| Discovery converges too early on a Change Unit. | Requirements clarification leaves room for shared understanding and user-owned judgment before requiring a scoped implementation unit. | Stage boundary decision |
| Judgment terminology reintroduces legacy aliases or display text as current axes. | Active owner docs prefer `UserJudgment` / `user_judgment`, `harness.request_user_judgment`, `judgment_kind`, and `presentation`; user-facing labels are rendered from `judgment_kind` and locale, and `display_label` is compatibility or response-only display text when mentioned. Legacy aliases stay compatibility-only. | Schema/design decision |
| Decision Packet examples are too heavy for small judgments. | Small judgments use `presentation=short` and fit on one screen; full-format Decision Packet presentation is optional, later-profile, or for complex judgments. | Schema/design decision |
| Sensitive-action Approval, final acceptance, and residual-risk acceptance blur together. | Examples and routing keep permission for a named sensitive step, acceptance of the work result, and acceptance of visible remaining risk separate. | Schema/design decision |
| Storage, API, or DDL reference material becomes an early-stage requirement by wording drift. | Reference-schema presence is separated from staged implementation requirement; required fields apply only when the owning tool, record, or profile is active or used. | Stage boundary decision |
| Conformance fixture docs imply executable suites exist now. | Fixture docs stay future-oriented and staged. MVP behavior examples are not described as runnable fixture files or current pass/fail criteria. | Implementation-readiness criterion |
| Operations entrypoints appear required too early. | Operator entrypoints stay staged and future-oriented unless the relevant Build stage explicitly includes them. | Stage boundary decision |
| Korean user-facing docs accumulate English technical nouns. | Korean prose uses natural public concepts first and preserves exact English identifiers only where precision, searchability, or contract alignment requires them. | Documentation drift |
| Decision-ledger status overclaims readiness. | Entrypoints, handoff, and this guide stay aligned with the MVP-1 decision ledger and do not convert documentation acceptance into implementation-planning readiness or server-coding authorization. | Implementation-readiness criterion |
| MVP scope grows while core user-visible value is deferred. | Build docs name the tension and keep staging decisions with Build and Reference owners. | Stage boundary decision |
| Projection/template scope becomes too broad for early implementation. | Projection/template docs separate active early scope from later display, export, and reporting candidates. | Stage boundary decision |
| Security wording overstates enforcement. | Cooperative, detective, preventive, and isolated claims match documented mechanisms and proof level. | Schema/design decision |
| Agent context strategy overloads the prompt. | Always-on context stays current and one screen or less; detailed contracts route to owner docs or retrieval paths. | Implementation-readiness criterion |
| Documentation reads like runtime state. | Docs are source material, not runtime objects, generated projections, operational records, or conformance artifacts. | Documentation drift |
| Roadmap candidates drift into active delivery. | Roadmap items stay in Roadmap unless an owner promotes them with scope, fallback behavior, proof expectations, and no projection-as-canonical dependency. | Future Roadmap item |

Use these exact-contract regression checks for schema/API/storage/build/conformance edits. They are documentation-maintenance checks only; they do not create acceptance status, runtime conformance, implementation readiness, or a roadmap item.

| Contract drift risk | Regression check | Default route if confirmed |
|---|---|---|
| Active schema blocks include later/profile values. | Active schema blocks list only values active for the owning tool, record, or profile. Later/profile enum values stay in `schema-later.md`, Later docs, or clearly labeled later/profile owner sections. | Schema/design decision |
| Prose-only stage gating hides missing schema separation. | Stage notes may explain when a value is used, but active schemas, API blocks, and DDL must still exclude inactive values or route them to later/profile owners. | Schema/design decision |
| Active API docs reference undefined schema types. | Every active method, response, shared ref, envelope member, error shape, and `CloseTaskResponse` field resolves to an active schema owner or is clearly labeled later/profile. | Schema/design decision |
| API/Core/Storage cover different fields for a shared concept. | Shared concepts such as Write Authorization have matching fields, value sets, row boundaries, and response/storage semantics across API, Core, and Storage owners. | Schema/design decision |
| Locale labels become schema values. | `display_label` and labels such as `제품 판단`, `기술 판단`, and `범위 판단` remain localized rendering text derived from stable identifiers, not canonical schema, enum, API, or storage values. | Documentation drift |
| Discovery or `shared_design` output has unclear authority. | Each Discovery/shared-design output states whether it is canonical active state, projection/derived display, support text, or later/profile material. | Schema/design decision |
| MVP close response exposes later assurance semantics. | Active MVP `CloseTaskResponse` wording does not expose later verification, Manual QA, detailed Evidence Manifest, detached verification, Eval, or full assurance-profile semantics as active fields or requirements. | Stage boundary decision |
| Conformance examples are prose-only. | Future conformance examples specify structured state, storage, event, error, artifact-ref, blocker, and guarantee assertions; rendered Markdown or prose expectations alone are not sufficient. | Schema/design decision |
| Build docs redefine Reference-owned contracts. | Build docs describe sequence, scope, exit criteria, and local consequences, and link to Reference owners for exact schema, DDL, API, storage, and transition definitions. | Documentation drift |
| Cleanup changes maintainer status values. | Readiness, handoff, implementation acceptance, coding acceptance, and final documentation acceptance values change only by explicit maintainer decision in the owner sections. | Implementation-readiness criterion |

### Docs-maintenance checks

Docs-maintenance checks are read-only Markdown/documentation quality checks. They may report documentation drift, owner mismatch, link integrity problems, terminology consistency problems, stage-boundary drift, security wording drift, user-language issues, English/Korean parity issues, duplicate normative text outside the owner, broken links or anchors, and open-marker hygiene problems.

For the final reviewer-facing checklist with what to inspect, common failures, pass meanings, and review type labels, use [Documentation Checks](documentation-checks.md).

Docs-maintenance is not runtime conformance or implementation readiness. It does not execute fixture actions, seed runtime state, compare runtime state/events/artifacts/projections/errors, count toward runtime fixture pass/fail, or create/update canonical state, runtime state, `task_events`, artifacts, Evidence Manifests, QA results, QA state, Manual QA records, Acceptance records, acceptance state, residual-risk acceptance, Residual Risk records, close readiness, projection refreshes, generated conformance artifacts, generated operational reports, or implementation readiness.

Docs-maintenance `PASS`, `WARN`, and `FAIL` labels may help manual review prioritize fixes, but they are not manual acceptance, final acceptance, close readiness, implementation readiness, or runtime fixture results. Runtime conformance applies only to implemented Core/API/storage/surface behavior and is judged by executable fixtures and state assertions, not documentation prose.

A docs-maintenance review or future checker should report:

- item category: documentation drift, schema/design decision, stage boundary decision, implementation-readiness criterion, or future Roadmap item
- result: `PASS`, `WARN`, or `FAIL`
- file path
- heading or anchor when available
- owner document and expected source section
- observed drift
- suggested fix
- runtime effect note: none; no canonical state transition or runtime fixture result was recorded
- maintenance note when a finding needs extra context

Use these result meanings:

| Result | Meaning |
|---|---|
| `FAIL` | Drift can make active docs contradictory or non-actionable, such as broken owner links, enum mismatch, API field mismatch, lifecycle status mismatch, schema/DDL/table/column/stable event/`ValidatorResult`/`ProjectionKind` mismatch, owner document mismatch, later/profile material presented or translated as active material, missing paired active files, missing semantic section coverage, or non-owner text redefining an owner contract. |
| `WARN` | Drift should be cleaned up but does not yet contradict an owner contract, such as minor glossary phrasing drift, duplicate explanatory prose that is not normative, stale cross-reference wording whose affected stage is explicit, or understandable but incomplete open-marker metadata. |
| `PASS` | No relevant drift is found for the category. |

Required check categories:

| Category | Required check |
|---|---|
| English/Korean file structure parity | `docs/en` and `docs/ko` keep the same active document paths, README entries, and paired route expectations unless an exception is explicitly documented. |
| English/Korean semantic section parity | Paired files keep the same reader purpose, semantic section coverage, owner links, stable identifiers, and contractual detail. |
| Contract identifier and lifecycle parity | API method names, enum values, field names, table names, column names, stable event names, validator IDs, projection/template kind names, record ID prefixes, lifecycle statuses, and file paths remain exact in both languages. Enum mismatch, API field mismatch, lifecycle status mismatch, table/column mismatch, owner document mismatch, and later/profile material translated or presented as active material are `FAIL`. |
| Opening convention compliance | Active docs make the reader's next useful step clear near the top. Start and Use may use workflow-first openings; Reference, Build, and Maintain may use structured openings; template references use template-specific openings. |
| Broken cross-reference detection | Markdown links, heading anchors, template/reference links, same-language README routes, paired-language entry links, and owner-section links resolve to active docs and current anchors. |
| Owner-boundary drift | Exact contracts and active owner concepts stay in their active owners. Non-owner docs summarize and link instead of redefining those contracts. |
| Repair-target owner routing | Known pre-implementation repair axes route to the canonical owner family in the [Pre-implementation repair target owner map](#pre-implementation-repair-target-owner-map); listed `FAIL` symptoms are docs-maintenance failures, not acceptance or implementation-readiness decisions. |
| Active schema/later-profile separation | Active schema blocks contain only active enum values, and later/profile values stay in `schema-later.md`, Later docs, or explicitly labeled later/profile owner sections. |
| Active API schema resolution | Active API docs reference only schema types, shared refs, envelopes, errors, and response fields defined by active owners, unless clearly routed to later/profile owners. |
| Cross-owner field coverage | API, Core, and Storage cover the same fields, value boundaries, row semantics, and response/storage implications for shared concepts such as Write Authorization. |
| Localized label boundary | `display_label` and locale labels such as `제품 판단`, `기술 판단`, and `범위 판단` remain rendering text, not canonical schema, enum, API, or storage values. |
| Discovery/shared-design output authority | Discovery or `shared_design` output states whether it is canonical active state, projection/derived display, support text, or later/profile material. |
| MVP close response boundary | Active MVP `CloseTaskResponse` wording does not expose later verification, Manual QA, detailed Evidence Manifest, detached verification, Eval, or full assurance-profile semantics as active fields or requirements. |
| Fixture/action schema drift | Future runtime fixture examples keep `action` and future executable `input` aligned with public MCP request schemas, shared API schemas, and the `ToolEnvelope` expansion convention. MVP behavior examples are not executable fixtures yet. |
| Structured conformance assertions | Conformance examples use structured state, storage, event, error, artifact-ref, blocker, and guarantee assertions rather than prose-only expectations or rendered Markdown. |
| Enum, event, validator, and projection drift | State/gate/result values, event names, error values, stable validator IDs, `ProjectionKind` values, storage values, template implementation classes, and projection freshness behavior match their owners. |
| Glossary and source-of-truth phrasing drift | Official terms, capitalization, record ID prefixes, source-of-truth wording, and authority-boundary phrases match `reference/glossary.md` and relevant owner docs. |
| Open-marker hygiene | `TODO_DECISION` and `TODO_IMPLEMENT` use allowed meanings, name the gap clearly, include enough owner/context to act on, and do not leave `TODO_REWRITE` markers in finished canonical sections. |
| Non-owner duplicate full contracts | Full schemas, DDL, transition tables, fixture mini-languages, template bodies, enum tables, validator tables, projection tables, or glossary definitions outside the owner are replaced with a short summary plus owner link. |
| Build/reference separation | Build docs do not redefine exact schema, DDL, API, storage, transition, or fixture contracts owned by Reference docs. |
| Maintainer readiness/status boundary | Cleanup work does not change readiness, handoff, implementation acceptance, coding acceptance, or final documentation acceptance status values without explicit maintainer action in the owner sections. |

### Final pre-acceptance review

Before maintainers accept the documentation set for implementation planning, do one final docs-maintenance pass. Use [Documentation Checks](documentation-checks.md) as the practical validation checklist, and summarize the final redesign handoff in [Rewrite Acceptance Review](rewrite-acceptance-review.md). Check English/Korean active file map parity, semantic section parity in paired files, broken links and anchors, owner-boundary drift, non-owner duplicate contracts, terminology drift for Approval, Decision Packet, Evidence, Verification, Manual QA, Acceptance, Residual Risk, Projection, and Guarantee Level, and open-marker hygiene.

Also check the documentation-planning exit criteria in [MVP Plan](../build/mvp-plan.md#exit-criteria-for-documentation-planning): repository identity, user-facing flow without internal-term burden, Discovery as requirements clarification rather than premature Change Unit convergence, canonical `user_judgment` naming with mapped legacy aliases, proportional judgment prompts, Approval/final acceptance/residual-risk acceptance separation, coherent stages, Core Model/API/storage/reference agreement, staged Storage/API scope, staged projection/template scope, honest security guarantee wording, agent context strategy, staged future-oriented conformance fixture plan, staged operations surface, Korean user-facing readability, and clean links/terminology/open markers. Authoring Guide and Translation Guide drift checks may detect enum, API, and owner-boundary drift in docs, but they remain documentation-quality checks and do not prove the server works.

This final review is still editorial review. It summarizes whether the docs are coherent enough for maintainer handoff; it does not create runtime conformance, canonical state, evidence, QA, Acceptance, residual-risk acceptance, close readiness, or implementation readiness.

## Detailed reference material

Use this section when a checklist item needs more detail. Do not treat it as a second set of mandatory steps.

### Opening patterns

Every active document should begin with a compact opening that makes the reader's path clear.

Reference, Build, and Maintain docs may use this structured opening when it helps:

- `What this document helps you do`: state the useful outcome in plain language.
- `Read this when`: name the situation that makes the document relevant.
- `Before you read`: name the assumed context, prior document, or prerequisite.
- `Main idea`: give the reader the central model or claim.

Start and Use docs may start with ordinary requests, practical examples, or user workflow. A workflow-first opening should show what users can ask, what the agent should clarify, what Harness preserves, what users should expect to see, and where exact owner details live.

Heading text differences are not drift when the document serves the reader situation, necessary context is present, owner links remain valid, exact contract details stay in Reference owners, and English/Korean versions are semantically aligned.

### Template reference opening

Template reference files use a specialized opening pattern. Docs-maintenance identifies them by path: `docs/*/reference/templates/README.md` for the directory index and non-README Markdown files under `docs/*/reference/templates/` for individual templates.

The directory README should begin with `Used when`, then output tiers and template implementation classes. It should explain that the directory owns rendered template bodies and display card shapes while projection rules, freshness behavior, and authority boundaries stay with their Reference owners.

Each individual template file should begin with these sections, in this order:

- `Used when`
- `Source records`
- `Rendered sections`
- `Full template`

Template files must make the non-authority boundary visible: a template is rendered display, not canonical state, gate authority, sensitive-action approval, final acceptance, residual-risk acceptance, evidence, schema, DDL, or runtime behavior.

### Conformance and fixture layering

Use three layers consistently:

- Documentation checks: editorial checks over Markdown docs. They are not runtime conformance and do not create generated operational or conformance artifacts.
- MVP behavior examples: small Engineering Checkpoint and MVP-1 design examples that describe expected behavior. They are not executable fixtures yet, not generated runtime artifacts, and not current pass/fail criteria.
- Runtime conformance: future server implementation tests and runners that materialize exact-shape fixtures, execute Core or operator actions, and compare captured state, events, artifacts, projection/freshness facts, and errors.

Do not call MVP behavior examples a runnable suite. Do not say documentation checks pass or fail runtime conformance. Do not generate fixture files, conformance reports, runtime state, projections, or operational artifacts during documentation work.

### Repetition rule

Do not repeat long source-of-truth paragraphs across docs. When another document needs the same idea, write a short local summary and link to the owner. If the source text changes, update the owner first, then check summaries for drift.

Repeated explanatory examples are allowed when they serve different readers. Repeated normative contract language is a drift risk.

For boundaries that are easy to repeat, use these owners:

| Boundary | Owner for exact wording |
|---|---|
| Context Index and retrieved/indexed context | [Roadmap: Candidate Inventory](../roadmap.md#candidate-inventory) for future feature boundary; [Agent Integration: Context Push/Pull Principles](../reference/agent-integration.md#context-pushpull-principles) for connector context handling |
| Local Derived Metrics | [Roadmap: Candidate Inventory](../roadmap.md#candidate-inventory) |
| Role Lens | [Agent Integration: Role Lens Behavior](../reference/agent-integration.md#role-lens-behavior) |
| Review Stages | [Design Quality Policies: Two-stage Review Display](../reference/design-quality-policies.md#two-stage-review-display) |
| Release Handoff and export | [Operations And Conformance: Release Handoff Export Profile](../reference/operations-and-conformance.md#release-handoff-export-profile); rendered shape in the later-profile [EXPORT Template](../reference/templates/later-profile/export.md) |
| Docs-maintenance | [Authoring Guide: Docs-maintenance checks](#docs-maintenance-checks) for rule bodies; [Documentation Checks](documentation-checks.md) for the final validation checklist; [Operations And Conformance: Docs-maintenance profile](../reference/operations-and-conformance.md#docs-maintenance-profile) for operator reporting |
| Projection and report surfaces | [Projection And Templates Reference](../reference/projection-and-templates.md); rendered shapes in [Template Reference](../reference/templates/README.md) |
| Security assets, trust boundaries, threat categories, control categories, guarantee-level meanings, and high-risk cooperative/detective/preventive/isolated security expectations | [Security Reference](../reference/security.md) for threat concepts and honest guarantee display; exact API, storage, Core, connector, operations, and conformance behavior stays with those owners |

### Owner-link summary pattern

When you find duplicated normative language outside its owner, do not polish the duplicate in place. Use this shape:

```text
Product writes need current Change Unit scope and Write Authorization. Exact write-gate behavior is owned by [Core Model Reference](../reference/core-model.md), and the public request shape is owned by [MVP API](../reference/api/mvp-api.md).
```

Do not paste the gate matrix, request schema, DDL block, fixture body, template body, enum table, validator table, projection table, or glossary definition into Start, Use, Build, or Maintain docs.

### Diagram rule

Use diagrams only when they reduce cognitive load. A diagram is useful when it shows a relationship, sequence, boundary, or lifecycle more clearly than prose.

Every diagram should have nearby prose that explains what to notice. If a diagram and the prose disagree, fix the owning prose or Reference contract first.

### Reference ownership map

Use this map for broad document routing. For strict Reference contracts, use the [Reference contract owner map](#reference-contract-owner-map).

| Subject | Active owner |
|---|---|
| Repo and docs entrypoints, reader routes, language choice, document list, target tree summary | repo root `README.md`; docs root `docs/README.md`; language entrypoints `docs/en/README.md` and `docs/ko/README.md` |
| Shared reader mental model, first-reader authority overview, ordinary work-loop story, small core concept introduction, project purpose, target users, values, scope, non-goals, automation philosophy, and current guarantee boundary | `start.md` |
| Strategic thesis, failure model, MVP boundary, principle groups | `start.md` for reader explanation; `reference/design-quality-policies.md` and `reference/core-model.md` for exact contract impact |
| Core entities, lifecycle, gates, state transitions, close semantics, `prepare_write`, `close_task` | `reference/core-model.md` |
| Runtime architecture, three spaces in implementation detail, Core process model, artifact architecture, projection/reconcile architecture, guarantee-level display placement | `reference/runtime-architecture.md` |
| Security assets, trust boundaries, threat categories, control categories, guarantee-level meanings, and high-risk cooperative/detective/preventive/isolated security expectations | `reference/security.md` for threat concepts and honest guarantee display; exact enforcement, API, storage, Core, connector, operations, and conformance behavior stays with those owners |
| MCP resources/tools, request/response schemas, error taxonomy, validator result schema, artifact ref shape | `reference/api/mvp-api.md`, `reference/api/schema-core.md`, `reference/api/errors.md`, `reference/api/schema-later.md` |
| SQLite DDL, migrations, storage layout, lock policy, artifact directory layout, baseline capture format, projection job table | `reference/storage.md` |
| MVP implementation order, smoke target, and planning exit criteria | `build/mvp-plan.md` |
| Markdown-rendered projection principles, authority matrix, managed blocks, human-editable sections, artifact reference rendering, output tiers, template implementation classes, projection freshness/failure rules | `reference/projection-and-templates.md` |
| All projection template bodies and display card shapes | `reference/templates/*.md` |
| Design-quality policy contracts, validators, severity composition, waiver semantics, evidence expectations, close impact | `reference/design-quality-policies.md` |
| User-facing conversation, status reading, user judgments, close checklist | `use/user-guide.md` |
| Practical user-owned judgment examples and user-facing judgment request patterns | `use/judgment-examples.md` for examples; `reference/core-model.md` and `reference/api/mvp-api.md` for exact user judgment behavior |
| User/agent session procedure | `use/agent-guide.md` |
| Agent surface capability profiles, common connector contract, fallback semantics, Role Lens, connector conformance overview | `reference/agent-integration.md` |
| Surface-specific recipes | `reference/surface-cookbook.md` |
| Generic capability profile examples | `reference/agent-integration.md` |
| Operator procedures, future conformance run overview, doctor/recover/reconcile/export/artifact integrity, documentation-check/docs-maintenance reporting | `reference/operations-and-conformance.md` |
| Core conformance model, MVP behavior examples, exact future fixture body shape, future runner execution, assertion semantics, current-phase status, future fixture profiles by proven behavior, suite metadata boundaries, reduced Kernel Smoke authoring queue | `reference/conformance-fixtures.md` |
| Compact future scenario-family inventory, promotion criteria, suite-family labels, catalog-only future candidates outside the MVP path | `later/future-fixtures.md` |
| Official term definitions and capitalization | `reference/glossary.md` |
| Roadmap candidates | `roadmap.md` |
| Documentation authoring rules | `maintain/authoring-guide.md` |
| Translation and bilingual prose rules | `maintain/translation-guide.md` |
| Rewrite planning categories and redesign triage | `maintain/rewrite-plan.md` |
| Final redesign acceptance review | `maintain/rewrite-acceptance-review.md` |
