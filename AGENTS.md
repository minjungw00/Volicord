# Agent Working Rules

These rules apply to agents and maintainers editing Harness documentation in this repository. Keep the documentation set stable, owner-routed, and bilingual.

Do not describe documentation text, examples, route metadata, maintainer controls, or check results as Harness runtime behavior or product implementation output.

## Maintainer Inputs

- Read this `AGENTS.md` before working in this repository.
- Before documentation edits, read the relevant maintainer guidance:
  - For English-facing edits, read `docs/en/maintain/authoring-guide.md`.
  - For Korean-facing edits, read `docs/ko/maintain/authoring-guide.md`.
  - For bilingual edits or terminology-affecting edits, read `docs/en/maintain/translation-guide.md`, `docs/ko/maintain/translation-guide.md`, `docs/terminology-map.yaml`, and the relevant glossary entries.
- Use small documentation batches and report changed files.
- Keep planning in the conversation unless the user explicitly asks for a maintained planning document.
- Do not create commits unless the user explicitly asks for commits.

## Entry Routes

Compact entry routes are first-hop navigation aids. Use them to choose a documentation family, then route to the canonical owner.

Entry-route families include:

- `docs/*/start.md` for product orientation.
- `docs/*/use/` for user and agent workflow guidance.
- `docs/*/build/implementation-guide.md` for implementation routing.
- `docs/*/reference/README.md` for the reference family index.
- `docs/*/maintain/` for authoring, translation, and check guidance.

Do not assume the compact route list is the full owner list. Split canonical owner documents live below these families, especially under `docs/*/reference/`, and may not appear as compact entry routes.

## Canonical Owner Lookup

Use `docs/doc-index.yaml` for exact owner routing. It is the stable machine-readable route table for `doc_id`, language paths, `role`, `owner_for`, `not_owner_for`, `depends_on`, `normative_level`, and `audience`.

LLM agents should read this `AGENTS.md` first for repository editing rules, then use `docs/doc-index.yaml` for exact owner lookup. Use entries in `shared_documents` and `documents` to find the matching `doc_id`, the owning `path` or `path_en`/`path_ko`, the entry's `owner_for` scope, and any `depends_on` support documents.

Use `docs/*/reference/README.md` as the human-readable reference owner index, not as a replacement for `docs/doc-index.yaml`. Keep route documents aligned with the index without copying contract details into them.

When an entry's `owner_for` matches the question or concept, load that owner first. Use `not_owner_for` to avoid routing a question to a nearby but non-owning document. Pull `depends_on` documents only when the owner, index metadata, or maintainer guidance sends you there. Use `role`, `normative_level`, and `audience` to distinguish route, guide, reference, build, and maintenance documents before editing.

One concept has one canonical owner. Edit the owner when the change affects normative meaning, including baseline scope, API behavior, schemas, storage effects, security wording, access boundaries, close readiness, product terminology, or out-of-scope promotion rules.

If an entry route, README, or maintain document cannot point to a current owner, do not fill the gap with duplicate contract prose. Name the owner gap or route to the closest current owner.

API routing shortcut:

- `docs/*/reference/api/methods.md` owns the active public API method list and method owner routing, not detailed method behavior.
- Route method behavior to the method-specific owner linked from the API method router.
- Route response branch schemas and nested API shapes to the schema owner documents listed in `docs/doc-index.yaml` and `docs/*/reference/README.md`.
- Route method payload field questions to the affected method owner when the field is method-specific; route shared envelope fields and nested schema fields to the schema owners.
- Route method storage effects to `docs/*/reference/storage-effects.md` first, then to narrower storage owners when needed.
- Route API example consistency and field-name consistency questions to `docs/*/maintain/authoring-guide.md` and `docs/*/maintain/checks.md`; open the affected method, schema, or storage owner only when checking the concrete example field or value.

## Language Selection

English and Korean docs are both active. Neither language is an archive, appendix, or translation-only copy.

Read one language version of the same `doc_id` unless checking translation parity, doing bilingual editing, or resolving a terminology/parity issue that requires comparison.

For normal agent retrieval, use the language that matches the user request or the default language in `docs/doc-index.yaml`. Do not inject paired English and Korean docs for the same `doc_id` into the same context unless a parity review requires both versions.

Do not finish a meaning-changing documentation batch with only one language updated when the changed document has an active paired path.

For bilingual edits, preserve the same reader purpose, normative strength, owner routing, current/out-of-scope boundary, user-judgment boundary, and security guarantee level by meaning unit. Matching line counts are not required.

## Korean Documentation Rule

Korean documentation edits must use natural Korean technical prose, not literal translations. Preserve meaning by unit while allowing natural Korean sentence order, paragraph rhythm, headings, and terminology.

Product names, API methods, schema names, field names, enum values, file paths, and code identifiers remain in English exactly as written. Preserve exact identifiers in backticks when prose clarity or searchability needs them.

Ordinary concepts should use Korean unless `docs/terminology-map.yaml` says otherwise. Use Korean concept-first phrasing and prefer terms from the terminology map and Korean glossary, such as `기준 범위`, `지원 범위 밖 기능`, `담당 문서`, `의미 일치`, `닫기 준비 상태`, `닫기 가능 여부`, `사용자 소유 판단`, `아티팩트`, `접점`, and `상태 보기` where they fit.

Avoid mixed-language Korean patterns where the English word is not an identifier, an intentional product label, or a standard borrowed technical term.

## Document Types

Owner documents define normative meaning. Route documents help readers find owners. Maintain documents define editing procedures and checks. User-facing guides explain practical workflows and expected reader outcomes. Reference documents define the contracts they own.

README files, Start pages, Use pages, Build pages, Maintain pages, and reference indexes may summarize reader purpose, expected result, and where to go next. Use short summaries plus links to canonical owners.

Do not copy API response branches, schema field tables, DDL, storage effects, access class lists, security guarantees, projection behavior, close-readiness contracts, or error-code contracts into non-owner documents. If a duplicate explanation is stale, shrink it to a practical consequence and link to the owner instead of refreshing the duplicate.

Maintain docs may name owner paths and duplication rules, but they must not become secondary sources of truth for API, storage, schema, security, access class, close-readiness, projection, runtime, or product contracts.

## Examples

Examples in reference and API documentation should use stable product or user scenarios. They should remain useful after the maintenance context is forgotten.

Keep examples, reference contracts, and guide text separate:

- Reference examples must match the method, schema, storage, or value-set owner for the fields and values they show.
- Guide examples should illustrate reader decisions and workflows without redefining the underlying contract.
- Route and maintain examples should not stand in for owner contract text.

API examples must be internally consistent across request data, visible response state, `state_version`, refs, paths, artifact refs, run refs, judgment refs, sensitive approval reasons, and close-readiness evidence.

API examples must not use documentation maintenance, migration, refactoring, route reshaping, or section restructuring as the scenario. Repository-internal documentation paths, including paths under `docs/`, should appear as example data only when the document is explicitly about documentation maintenance.

## Editing Rules

- Preserve the product thesis: Harness is not a prompt pack. It is a local authority record for scope, user-owned judgment, evidence, verification expectations, acceptance, close readiness, and residual risk.
- Keep user-owned judgments distinct from Core-owned state/artifact authority. Evidence, verification, QA, acceptance, waiver, and residual-risk boundaries must not collapse into one broad approval.
- Keep baseline behavior separate from reserved, profile-gated, and out-of-scope material. Do not describe out-of-scope capabilities as baseline requirements.
- Guard, freeze, careful-mode, and security wording must match the actual guarantee level documented by the security owner. Only documented preventive mechanisms should claim preventive behavior.
- Replace dated project narration with durable owner-routed guidance or remove it.
- Rewrite, move, merge, shrink, or delete prose when it conflicts with current owner boundaries, active/out-of-scope boundaries, Korean quality rules, or implementation feasibility.
- Preserve exact identifiers in backticks, including file paths, `doc_id` values, API method names, schema fields, enum values, status values, table names, validator IDs, error codes, anchors, and code literals.
- Major implementation decisions belong in `docs/en/build/implementation-guide.md` and `docs/ko/build/implementation-guide.md`, not scattered across route or maintain documents.
- Path allowlists and batch boundaries for documentation edits are maintainer editing controls, not Harness runtime override capabilities.

## One-Off File Rule

Do not create archive copies, one-off transition notes, scratch notes, unresolved note-only files, review leftovers, generated runtime records, one-off planning files, migration notes, or work logs in the repository.

Remove scratch notes and transient review material before finishing.

If the user explicitly asks for a planning document, place it only in an appropriate maintained documentation path and make sure it has durable reader value.

## Validation After Edits

After documentation edits, use the relevant checks in `docs/*/maintain/checks.md` and report checks that were run or skipped.

Validate changed relative links, file paths, anchors, route tables, and paired-language links. When headings change, check inbound links and the paired-language document. Korean visible headings should stay natural; use hidden anchors when a stable English anchor must be preserved.

Validate terminology against `docs/terminology-map.yaml` and the relevant glossary entries. Confirm exact identifiers remain unchanged and are backticked where prose clarity or searchability requires.

For meaning-changing bilingual edits, compare paired files by meaning unit. Confirm the paired files keep the same reader purpose, normative strength, owner routing, current/out-of-scope boundary, user-judgment boundary, and security guarantee level while allowing natural Korean structure.

For example edits, validate scenario durability, field-name consistency, response snapshot consistency, artifact-ref lifecycle context, sensitive approval reasons, and timestamp handling against the affected owner documents and `docs/*/maintain/checks.md`.

Before finishing, confirm no one-off planning files, archive copies, scratch files, migration notes, generated runtime records, or work logs remain from the edit.
