# Agent working rules

This repository contains the maintained Harness documentation set.

Do not describe documentation as implemented runtime behavior. Runtime code, generated runtime objects, and product implementation outputs are outside this repository's editing scope.

## Repository boundaries

- Read this `AGENTS.md` before working in this repository.
- Before documentation edits, read the relevant maintainer guidance:
  - For English-facing edits, read `docs/en/maintain/authoring-guide.md`.
  - For Korean-facing edits, read `docs/ko/maintain/authoring-guide.md`.
  - For bilingual edits or terminology-affecting edits, read the relevant translation guidance and terminology sources.
- Keep repository changes within documentation. Do not implement Harness server/runtime code, product implementation code, generated operational files, runtime state, projections, artifacts, executable fixtures, conformance runners, or Harness runtime objects.
- This repository is not the user's Product Repository and not a Harness Runtime Home.
- Treat documentation files as reference material for a Harness Server, not as Harness runtime state, generated artifacts, projections, evidence, QA, acceptance, residual-risk records, close records, or implementation output.
- Do not run or simulate Harness runtime procedures for documentation edits. Do not create `prepare_write`, MCP state-transition, `close_task`, runtime-state, judgment, evidence, QA, acceptance, residual-risk, projection, operational, or fixture outputs.
- Path allowlists and batch boundaries for documentation edits are maintainer editing controls, not Harness runtime override capabilities.
- Use small documentation batches and report changed files.
- Do not create commits unless the user explicitly asks for commits.

## Entry routes

Compact entry routes are first-hop navigation aids. Use compact entry routes only to choose a documentation family.

Entry-route families include:

- `docs/*/start.md` for product orientation.
- `docs/*/use/` for user and agent workflow guidance.
- `docs/*/build/implementation-guide.md` for implementation routing.
- `docs/*/reference/README.md` for the reference family index.
- `docs/*/maintain/` for authoring, translation, and check guidance.

Do not assume the compact route list is the full owner list. Split canonical owner documents live below these families, especially under `docs/*/reference/`, and may not appear as compact entry routes.

## Canonical owner lookup

For exact owner routing, use `docs/doc-index.yaml`. It is the stable machine-readable route table for `doc_id`, owner family, canonical ownership, related documents, and language paths.

LLM agents should read this `AGENTS.md` first for repository boundaries and retrieval rules. Then use `docs/doc-index.yaml` for exact owner routing. Use `docs/*/reference/README.md` as the human-readable owner router, and keep it aligned with `docs/doc-index.yaml`.

For high-signal terminology, example-scenario, API example-consistency, field-name consistency, scope, storage, and owner questions, check `question_routes.routes` in `docs/doc-index.yaml` before broad keyword retrieval. If a route matches, load the canonical owner first and load listed supporting owners only when the question spans that boundary.

When `docs/doc-index.yaml` lists an exact owner for the question or concept, load that owner first. Pull related documents only when the owner, index metadata, or maintainer guidance sends you there.

One concept should have one canonical owner. Edit the owner when the change affects normative meaning, including active MVP scope, API behavior, schemas, storage effects, security wording, access boundaries, close readiness, product terminology, or out-of-scope promotion rules.

API routing shortcut:

- `docs/*/reference/api/methods.md` owns the active public API method list and method owner routing, not detailed method behavior.
- Route method behavior to the method-specific owners: `api/method-intake.md`, `api/method-update-scope.md`, `api/method-status.md`, `api/method-prepare-write.md`, `api/method-stage-artifact.md`, `api/method-record-run.md`, `api/method-user-judgment.md`, and `api/method-close-task.md`.
- Route response branch schemas and nested API shapes to the schema owner documents: `api/schema-core.md`, `api/schema-state.md`, `api/schema-artifacts.md`, `api/schema-judgment.md`, and `api/schema-value-sets.md`.
- Route method payload field questions to the affected method owner when the field is method-specific; route shared envelope fields and nested schema fields to the schema owners.
- Route method storage effects to `docs/*/reference/storage-effects.md` first, then to narrower storage owners when needed.
- Route API example consistency and field-name consistency questions to `docs/*/maintain/authoring-guide.md` and `docs/*/maintain/checks.md`; open the affected method, schema, or storage owner only when checking the concrete example field or value.

If an entry route, README, or maintain document cannot point to a current owner, do not fill the gap with duplicate contract prose. Name the owner gap or route to the closest current owner.

## Language selection

English and Korean docs are both active. Neither language is an archive, appendix, or translation-only copy.

Read one language version of the same `doc_id` unless checking translation parity, doing bilingual editing, or resolving a terminology/parity issue that requires comparison.

For normal agent retrieval, use the language that matches the user request or the default language in `docs/doc-index.yaml`. Do not inject paired English and Korean docs for the same `doc_id` into the same context without a parity reason.

Do not finish a meaning-changing documentation batch with only one language updated when the changed document has an active paired path.

## Editing rules

- Preserve the product thesis: Harness is not a prompt pack. It is a local authority record for scope, user-owned judgment, evidence, verification expectations, acceptance, close readiness, and residual risk.
- Keep user-owned judgments distinct from Core-owned state/artifact authority. Evidence, verification, QA, acceptance, waiver, and residual-risk boundaries must not collapse into one broad approval.
- Keep active MVP behavior separate from reserved, profile-gated, and out-of-scope material. Do not describe out-of-scope capabilities as active MVP requirements.
- Guard, freeze, careful-mode, and security wording must match the actual guarantee level documented by the security owner. Only documented preventive mechanisms should claim preventive behavior.
- Rewrite, move, merge, shrink, or delete old prose when it conflicts with current owner boundaries, active/out-of-scope boundaries, Korean quality rules, or implementation feasibility.
- Remove stale route wording, legacy history, resolved cleanup notes, one-language-primacy guidance, and scattered unresolved notes from active docs when encountered in scope.
- Preserve exact identifiers in backticks, including file paths, `doc_id` values, API method names, schema fields, enum values, status values, table names, validator IDs, error codes, anchors, and code literals.
- Major implementation decisions belong in `docs/en/build/implementation-guide.md` and `docs/ko/build/implementation-guide.md`, not scattered across route or maintain documents.

## Contract duplication rule

Do not duplicate long technical contracts into README, route, or maintain documents.

README files, Start pages, Use pages, Build pages, Maintain pages, and reference indexes may summarize reader purpose, expected result, and where to go next. They should use short summaries plus links to canonical owners.

Do not copy API response branches, schema field tables, DDL, storage effects, access class lists, security guarantees, projection behavior, close-readiness contracts, or error-code contracts into non-owner documents. If a duplicate explanation is stale, shrink it to a practical consequence and link to the owner instead of refreshing the duplicate.

## Temporary file rule

Do not create archive copies, temporary transition notes, scratch notes, unresolved note-only files, review leftovers, generated runtime records, or one-off planning files in the repository.

Do not leave temporary transition notes, scratch notes, or unresolved note-only files in the repo. Remove scratch notes before finishing.

If planning is needed, keep it in the conversation or in the requested target document. If the user explicitly asks for a planning document, place it only in an appropriate maintained documentation path and make sure it has durable reader value.

## Korean documentation rule

Korean documentation edits must use natural Korean technical prose, not literal English translation.

When editing Korean docs, use `docs/terminology-map.yaml` and `docs/ko/reference/glossary.md`. Also consult `docs/ko/maintain/translation-guide.md` for Korean style, semantic parity, hidden anchors, and forbidden mixed-language patterns.

Preserve identifiers in backticks. Do not translate exact identifiers, file paths, anchors, `doc_id` values, API methods, schema names, schema fields, enum values, table names, validator IDs, error codes, or code literals.

Do not literal-translate English prose into Korean. Maintain semantic parity by meaning unit while allowing natural Korean sentence order, paragraph rhythm, terminology, and heading style.

Use Korean concept-first phrasing for ordinary prose. Avoid mixed-language patterns where the English word is not an identifier or intentional product label. Prefer terms from the terminology map and Korean glossary, such as `현재 MVP`, `이후 후보`, `담당 문서`, `의미 일치`, `닫기 준비 상태`, `닫기 가능 여부`, `사용자 소유 판단`, `아티팩트`, `접점`, and `상태 보기` where they fit.
