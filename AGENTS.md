# Codex Working Rules

This repo is in pre-MVP Harness documentation redesign / feedback incorporation and post-redesign review / documentation acceptance mode. It is documentation-only now and is intended to become the Harness Server source repository after documentation acceptance. Runtime/server implementation has not started, and the current docs must not be described as fully accepted or implementation-complete unless the maintainer handoff status explicitly says so. This file is a short always-on compass for agents working here, not a Harness runtime procedure, schema reference, or project history.

## Repo Phase

- Always read this `AGENTS.md` first before working in this repository.
- Before any documentation edit, read `docs/en/maintain/authoring-guide.md`.
- Before bilingual edits or terminology-affecting edits, read `docs/en/maintain/translation-guide.md`.
- Before touching Korean docs, read `docs/ko/maintain/authoring-guide.md` and `docs/ko/maintain/translation-guide.md`. Korean docs must follow the Korean translation guidance: natural Korean technical prose, short sentences where possible, exact identifiers preserved.
- Do not implement the Harness server, runtime code, product implementation code, generated operational files, or state/projection/artifact outputs.
- This repo is not the user's Product Repository and not a Harness Runtime Home.
- No Harness Server/runtime implementation exists here yet.
- Treat the current documentation as a post-redesign review baseline, not a final accepted implementation-ready state.
- Documentation edits are allowed in this phase.
- Do not treat documentation files as Harness runtime objects, runtime state, generated artifacts, projections, evidence, QA, Acceptance, residual-risk records, or close records.
- Do not run or simulate Harness runtime procedures for documentation edits: no `prepare_write`, MCP state transitions, `close_task`, runtime state, `task_events`, Write Authorizations, Evidence Manifests, Manual QA records, Acceptance records, Residual Risk records, Journey Cards, generated projections, or other generated operational/projection documents for docs work. These terms may be documented only as future Harness behavior.
- Path allowlists and batch boundaries for docs edits are maintainer editing controls, not Harness runtime override capabilities.
- Final documentation handoff status and major server-coding decisions live in `docs/en/build/mvp-plan.md` and `docs/ko/build/mvp-plan.md`. Major implementation decisions found during review belong only in the MVP Plan decision section, not scattered TODOs.
- When changing meaning, work in `docs/en` first and mirror semantic changes in `docs/ko` in the same batch.
- Maintain semantic parity between English and Korean docs, while allowing natural Korean headings and prose.
- Use the current documentation tree: `docs/*/start.md`, `docs/*/use/*`, `docs/*/build/*`, `docs/*/reference/*`, `docs/*/later/*`, `docs/*/maintain/*`, and `docs/*/roadmap.md`.
- Use small batches and report changed files.
- Do not create commits unless the user explicitly asks for commits.

## Documentation Redesign Compass

- The repository is in documentation review/redesign only; runtime/server implementation is not being started by these documentation edits.
- The redesign may change terminology, the stage/profile model, schema structure, projection structure, security wording, and document organization.
- Do not preserve existing prose merely for continuity if it conflicts with the clarified product thesis or implementation feasibility.
- Feel free to rewrite, move, merge, shrink, or delete old prose when it conflicts with the clarified product thesis, owner boundaries, Korean quality rules, or implementation feasibility.
- Preserve the product thesis: Harness is not a prompt pack; it is a local authority record for scope, user-owned judgment, evidence, and close readiness. User-owned judgments, evidence/verification/QA/acceptance/risk boundaries, and Core-owned state/artifact authority must stay distinct.
- Mandatory documentation workflow, preserved principles, document-family ownership guidance, stage boundaries, Korean quality rules, and [redesign risk/regression checks](docs/en/maintain/authoring-guide.md#known-redesign-issues-and-regression-checks) live in [Authoring Guide](docs/en/maintain/authoring-guide.md).
- Future documentation validation checklist lives in [Documentation Checks](docs/en/maintain/documentation-checks.md); it is editorial docs-maintenance, not runtime conformance or implementation readiness.
- Future rewrite triage categories live in [Rewrite Plan](docs/en/maintain/rewrite-plan.md) and [Korean Rewrite Plan](docs/ko/maintain/rewrite-plan.md).

## Harness Compass

- When Harness is connected, no startup phrase is required. Infer Harness use from task shape; users do not need to say "Harness" or know internal labels.
- Product/runtime writes are out of scope in this repo phase. In Harness-connected product work, product writes require compatible `prepare_write` / Write Authorization where applicable.
- User-owned product, material technical, QA/waiver, acceptance, and residual-risk judgment routes through the documented `user_judgment` / owner path, with Decision Packet only as an optional full-format presentation. Sensitive-action approval, work acceptance, residual-risk acceptance, waiver, and reconciliation remain distinct; broad approval does not substitute for any of them.
- Guard, freeze, and careful-mode wording must match the actual guarantee level. Cooperative or detective surfaces can hold by instruction or detect after action; only proven preventive profiles should claim pre-execution blocking.
- Do not imply early Harness provides OS-level permissions, arbitrary-tool sandboxing, tamper-proof local files, pre-tool blocking, or security isolation unless the exact mechanism being claimed is documented and proven for the covered operation. Preventive claims require a fixture-proven blocking path; isolation claims must name the separation boundary and must not imply OS sandboxing, permission isolation, or tamper-proof storage unless that exact mechanism is proven.
- Keep always-on context current and one screen or less: current task summary, work shape, scope/non-goals, pending user judgments, active blockers, next safe actions, evidence gaps, close blockers, residual-risk summary, guarantee level, and source refs/freshness. Do not bury state, inject full reference docs, full schemas, full DDL, historical logs, full projection bodies, full artifact contents, unrelated templates, or future catalog material here.
- Use phase-relevant context profiles instead of broad doc loading: planning/clarification, write preparation, execution/run recording, evidence review, close readiness, user judgment request, and recovery/error. Pull only the owner section needed for the next action. Judgment requests must preserve the decision, options, consequences, uncertainty, and what the agent is not deciding for the user.
- For detailed guidance, use [User Guide](docs/en/use/user-guide.md), [Agent Guide](docs/en/use/agent-guide.md), [Agent Integration Reference](docs/en/reference/agent-integration.md), and [Surface Cookbook](docs/en/reference/surface-cookbook.md). For docs edits, use [Authoring Guide](docs/en/maintain/authoring-guide.md).
