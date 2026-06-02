# Codex Working Rules

This repo is in pre-MVP Harness documentation redesign / feedback incorporation and post-redesign review / documentation acceptance mode. It is documentation-only now and is intended to become the Harness Server source repository after documentation acceptance. Runtime/server implementation has not started, and the current docs must not be described as fully accepted or implementation-complete unless the maintainer handoff status explicitly says so. This file is a short always-on compass for agents working here, not a Harness runtime procedure, schema reference, or project history.

## Repo Phase

- Do not implement the Harness server, runtime code, product implementation code, generated operational files, or state/projection/artifact outputs.
- This repo is not the user's Product Repository and not a Harness Runtime Home.
- No Harness Server/runtime implementation exists here yet.
- Treat the current documentation as a post-redesign review baseline, not a final accepted implementation-ready state.
- Documentation edits are allowed in this phase.
- Do not run or simulate Harness runtime procedures for documentation edits: no `prepare_write`, MCP state transitions, `close_task`, runtime state, `task_events`, Write Authorizations, Evidence Manifests, Manual QA records, Acceptance records, Residual Risk records, Journey Cards, generated projections, or other generated operational/projection documents for docs work. These terms may be documented only as future Harness behavior.
- Final documentation handoff status lives in `docs/en/build/implementation-overview.md` and `docs/ko/build/implementation-overview.md`. Major implementation decisions found during review belong only in the MVP Plan decision-log sections, not scattered TODOs.
- When changing meaning, work in `docs/en` first and mirror semantic changes in `docs/ko` in the same batch.
- Maintain semantic parity between English and Korean docs, while allowing natural Korean headings and prose.
- Use the current documentation tree: `docs/*/learn/*`, `docs/*/use/*`, `docs/*/build/*`, `docs/*/reference/*`, `docs/*/maintain/*`, and `docs/*/roadmap.md`.
- Use small batches and report changed files.
- Do not create commits unless the user explicitly asks for commits.

## Documentation Redesign Compass

- The repository is in documentation review/redesign only; runtime/server implementation is not being started by these documentation edits.
- The redesign may change terminology, MVP staging, schema structure, projection structure, security wording, and document organization.
- Do not preserve existing prose merely for continuity if it conflicts with the clarified product thesis or implementation feasibility.
- Preserve the product thesis: Harness is not a prompt pack; it is a local authority record for scope, user-owned judgment, evidence, and close readiness. User-owned judgments, evidence/verification/QA/acceptance/risk boundaries, and Core-owned state/artifact authority must stay distinct.
- The detailed redesign scope, preserved principles, document-family ownership guidance, Korean quality rules, and [known issue tracker](docs/en/maintain/authoring-guide.md#known-redesign-issues-tracker) live in [Authoring Guide](docs/en/maintain/authoring-guide.md#current-redesign-scope).

## Harness Compass

- When Harness is connected, no startup phrase is required. Infer Harness use from task shape; users do not need to say "Harness" or know internal labels.
- Product/runtime writes are out of scope in this repo phase. In Harness-connected product work, product writes require compatible `prepare_write` / Write Authorization where applicable.
- User-owned product, material technical, QA/waiver, acceptance, and residual-risk judgment routes through Decision Packets or the documented decision path, not broad approval.
- Guard, freeze, and careful-mode wording must match the actual guarantee level. Cooperative or detective surfaces can hold by instruction or detect after action; only proven preventive profiles should claim pre-execution blocking.
- Do not imply early Harness provides OS-level permissions, arbitrary-tool sandboxing, tamper-proof local files, pre-tool blocking, or security isolation unless the exact mechanism being claimed is documented and proven for the covered operation. Preventive claims require a fixture-proven blocking path; isolation claims must name the separation boundary and must not imply OS sandboxing, permission isolation, or tamper-proof storage unless that exact mechanism is proven.
- Keep always-on context current and one screen or less: role, current phase, current task summary, active blockers, pending user-owned judgments, and next allowed action. Do not bury state, inject full reference docs or schemas, copy historical logs, or duplicate strict contracts here.
- Use phase-relevant context profiles instead of broad doc loading: session start, requirements clarification, decision request, prepare-write, run/evidence, close readiness, and recovery/error. Pull only the owner section needed for the next action; do not load full Storage DDL, Conformance catalogs, Template sets, unrelated Roadmap items, old task history, or full projection bodies by default.
- For detailed guidance, use [User Guide](docs/en/use/user-guide.md), [Agent Session Flow](docs/en/use/agent-session-flow.md), [Agent Integration Reference](docs/en/reference/agent-integration.md), and [Surface Cookbook](docs/en/reference/surface-cookbook.md). For docs edits, use [Authoring Guide](docs/en/maintain/authoring-guide.md).
