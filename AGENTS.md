# Agent Working Rules

These rules apply to agents and maintainers editing Harness documentation in this repository. They are repository editing guidance only. Do not describe documentation text, examples, route metadata, maintainer controls, or check results as Harness runtime behavior or product implementation output.

## First Reads

- Read this `AGENTS.md` before changing repository documentation.
- Use `docs/doc-index.yaml` as the canonical machine-readable owner route. It owns `doc_id`, paired paths, role, owner scope, non-owner scope, dependencies, normative level, and audience metadata.
- Use `docs/terminology-map.yaml` as the terminology and identifier-preservation source of truth.
- For English-facing edits, read `docs/en/maintain/authoring-guide.md`.
- For Korean-facing edits, read `docs/ko/maintain/authoring-guide.md`.
- For bilingual edits or terminology-affecting edits, read both translation guides, `docs/terminology-map.yaml`, and the relevant glossary entries.
- After documentation edits, use the relevant check guidance under `docs/*/maintain/checks.md`.

## Route And Owner Lookup

README files, Start pages, Use pages, Build pages, Maintain pages, and reference indexes are route documents. They help readers choose the next document; they do not define API behavior, storage effects, security guarantees, schemas, close-readiness contracts, or detailed owner maps.

For exact owner lookup, read `docs/doc-index.yaml` first. Use the human-readable `docs/*/reference/README.md` index when a reader-facing reference route is useful, but do not copy its owner map into `AGENTS.md`, README files, or maintain guidance.

When a change affects normative meaning, edit the canonical owner document selected from `docs/doc-index.yaml`. This includes baseline scope, API behavior, schema meaning, storage effects, security wording, access boundaries, close readiness, product terminology, out-of-scope promotion rules, and value-set meaning.

If an entry route or maintain document cannot point to an applicable owner, do not fill the gap with duplicate contract prose. Name the owner gap or route to the closest applicable owner.

## Language And Terminology

English and Korean documentation are both maintained. Neither language is an archive, appendix, or translation-only copy.

For ordinary lookup, read the language that matches the request or the default language in `docs/doc-index.yaml`. Read both paired documents when doing bilingual editing, translation review, parity review, or terminology work.

Do not finish a meaning-changing documentation batch with only one language updated when the changed document has a maintained paired path. Preserve the same reader purpose, normative strength, owner routing, baseline and out-of-scope boundaries, user-judgment boundary, and security guarantee level by meaning unit.

Korean documentation must use natural Korean technical prose. Preserve exact identifiers, file paths, API methods, schema names, field names, enum values, status values, product labels, anchors, and code literals exactly as written, with backticks where clarity or searchability requires them.

Use the terminology map's Harness/Core distinction:

- Harness is the local work-authority product/system for AI-assisted product work.
- Core is the local authority record for Harness state.

## Editing Rules

- Keep route documents short and navigational. If a route page starts to need field tables, status-value tables, storage-effect detail, error behavior, or guarantee levels, move that content to the applicable owner.
- Keep user-owned judgments distinct from Core-owned state and artifact authority. Evidence, verification criteria, QA, acceptance, waiver, and residual-risk boundaries must not collapse into one broad approval.
- Keep baseline behavior separate from reserved, profile-gated, and out-of-scope material. Do not describe out-of-scope capabilities as baseline requirements.
- Match guard, freeze, careful-mode, and security wording to the guarantee level documented by the security owner.
- Use stable product or user scenarios in examples. Do not make documentation maintenance, route reshaping, or section restructuring the API example scenario unless the document is specifically about documentation maintenance.
- Keep API examples internally consistent across request data, visible response state, `state_version`, refs, paths, artifact refs, run refs, judgment refs, sensitive approval reasons, and close-readiness evidence.
- Preserve exact identifiers in prose, tables, examples, and route metadata.
- Put major implementation decisions in `docs/en/build/implementation-guide.md` and `docs/ko/build/implementation-guide.md`, not in README files or broad route pages.
- Treat path allowlists and documentation batch boundaries as maintainer editing controls, not Harness runtime override capabilities.
- Use durable maintenance wording. Avoid task-specific, PR-specific, or short-lived wording in maintained documentation.

## Plans And Scratch Notes

Keep planning in the conversation unless the user explicitly asks for a maintained documentation artifact. Scratch notes, archive copies, conversion notes, unresolved review notes, generated runtime records, and work logs do not belong in the repository.

If a user asks for a maintained planning document, place it only in an appropriate maintained documentation path and make sure it has durable reader value.

## Validation

After edits, run or perform the checks that match the changed files. For route and entry changes, include structure, links/indexes, terminology, and language parity checks when applicable.

Before finishing, confirm changed links, file paths, anchors, paired-language links, and terminology. Confirm no scratch files, archive copies, generated records, or work logs remain from the edit.
