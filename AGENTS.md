# Codex working rules

This repository is a documentation-only Harness planning repository. It is in pre-MVP documentation redesign, review, and acceptance work.

Runtime/server implementation has not started. Do not describe the current docs as implementation-complete, accepted runtime behavior, or permission to start server coding unless the maintainer handoff status in `docs/*/build/mvp-plan.md` explicitly says so.

## Repo phase

- Always read this `AGENTS.md` first before working in this repository.
- Before documentation edits, read the relevant maintainer guidance:
  - For English-facing edits, read `docs/en/maintain/authoring-guide.md`.
  - For Korean-facing edits, read `docs/ko/maintain/authoring-guide.md`.
  - For bilingual edits or terminology-affecting edits, read both translation guides: `docs/en/maintain/translation-guide.md` and `docs/ko/maintain/translation-guide.md`.
- Keep all work documentation-only. Do not implement Harness server/runtime code, product implementation code, generated operational files, runtime state, projections, artifacts, executable fixtures, conformance runners, or Harness runtime objects.
- This repository is not the user's Product Repository and not a Harness Runtime Home.
- Treat documentation files as source material for a future Harness Server, not as Harness runtime state, generated artifacts, projections, evidence, QA, acceptance, residual-risk records, close records, or implementation output.
- Do not run or simulate Harness runtime procedures for documentation edits. Do not create `prepare_write`, MCP state-transition, `close_task`, runtime-state, judgment, evidence, QA, acceptance, residual-risk, projection, operational, or fixture outputs.
- Path allowlists and batch boundaries for docs edits are maintainer editing controls, not Harness runtime override capabilities.
- Use small batches and report changed files.
- Do not create archive copies or temporary migration notes.
- Do not create commits unless the user explicitly asks for commits.

## Current documentation routes

Use only the compact active structure:

- `docs/doc-index.yaml`
- `docs/*/start.md`
- `docs/*/use/user-guide.md`
- `docs/*/use/agent-guide.md`
- `docs/*/use/judgment-examples.md`
- `docs/*/build/mvp-plan.md`
- `docs/*/reference/README.md`
- `docs/*/later/index.md`
- `docs/*/maintain/authoring-guide.md`
- `docs/*/maintain/translation-guide.md`
- `docs/*/maintain/checks.md`

Do not route README or Maintain guidance outside this compact structure.

Use `docs/*/reference/README.md` to choose exact contract owners instead of turning reference subpages into top-level routes. If an old path appears during review, replace it with the current compact route or delete the stale route wording.

## Bilingual documentation rules

- English and Korean docs are both active. Neither language is an archive, appendix, or translation-only copy.
- Every major active doc should have a paired English/Korean path.
- Maintain semantic parity between paired docs. Line-by-line translation is not required, and natural Korean technical prose is expected in Korean files.
- Do not finish a meaning-changing documentation batch with only one language updated.
- Preserve exact identifiers in both languages, including file paths, `doc_id` values, API method names, schema fields, enum values, table names, validator IDs, and error codes.
- When editing Korean docs, use natural Korean terms such as "한영 문서 동시 유지", "의미 일치", "줄 단위 번역 아님", "에이전트 중복 주입 금지", "현재 MVP", and "담당 문서" where they fit.

## Agent context rules

- Load only one language for the same `doc_id` in a single prompt. Do not inject paired English and Korean docs for the same `doc_id` into the same agent context unless the task is translation/parity review and the comparison is necessary.
- Keep current context small. Include only what the next action needs, such as:
  - task summary and work shape
  - scope/non-goals
  - pending user judgments
  - blockers and next safe actions
  - evidence gaps and close blockers
  - residual-risk summary
  - guarantee level
  - source refs/freshness
- Pull owner docs only when needed for the next edit or check. Prefer `docs/*/reference/README.md` to choose the owner instead of loading the whole Reference set.
- Do not bury state by injecting full reference docs, full schemas, full DDL, historical logs, projection bodies, artifact contents, unrelated templates, future catalog material, or both language versions of the same document.

## Documentation redesign compass

- The repository is in documentation review/redesign only; these edits do not start runtime/server implementation.
- The redesign may change terminology, the delivery/later candidate model, schema structure, projection structure, security wording, and document organization.
- Rewrite, move, merge, shrink, or delete old prose when it conflicts with the product thesis, owner boundaries, Korean quality rules, active/later boundaries, or implementation feasibility.
- Remove stale improvement goals, resolved review records, old cleanup notes, legacy history, migration notes, and one-language-primacy guidance from active docs.
- Do not list profile-gated values as default active MVP values, and do not describe later candidates as active MVP requirements.
- Check stale route wording, active/profile-gated confusion, unsupported security claims, Korean natural prose, and one-language-per-`doc_id` retrieval before finishing documentation batches.
- Preserve the product thesis: Harness is not a prompt pack. It is a local authority record for scope, user-owned judgment, evidence, verification expectations, acceptance, close readiness, and residual risk.
- Keep user-owned judgments distinct from Core-owned state/artifact authority. Evidence, verification, QA, acceptance, waiver, and residual-risk boundaries must not collapse into one broad approval.
- Major implementation-readiness decisions belong in `docs/en/build/mvp-plan.md` and `docs/ko/build/mvp-plan.md`, not scattered TODOs.

## Harness compass

- When Harness is connected, no startup phrase is required. Infer Harness use from task shape; users do not need to say "Harness" or know internal labels.
- Product/runtime writes are out of scope in this repo phase. In Harness-connected product work outside this repository, product writes require compatible `prepare_write` / Write Authorization where applicable.
- User-owned product, material technical, QA/waiver, final acceptance, and residual-risk judgment routes through the documented `user_judgment` / owner path. Decision Packet is only an optional full-format later presentation.
- Sensitive-action approval, final acceptance, residual-risk acceptance, waiver, and reconciliation remain distinct. Broad approval does not substitute for any of them.
- Guard, freeze, careful-mode, and security wording must match the actual guarantee level documented in `docs/*/reference/security.md`; only documented preventive mechanisms should claim preventive behavior.
