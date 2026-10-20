# Volicord local explanation reader

The Viewer has precisely three reading destinations: **Overview / Repository
explanation / Work**. The brand is left and the single primary navigation is
right. Read-only status sits outside navigation. The maintained contract is
[D-01–D-33 / AC-01–AC-31](../../docs/design/reader-experience-design.md).

```sh
volicord viewer open --view overview
volicord viewer open --view repository --language en
volicord viewer open --view work --language ko
volicord viewer export --output /absolute/new/viewer.html --language en
```

Run these from a bound repository, or supply global `--runtime` and `--project`
options. The standalone executable supports `--runtime`, `--project`, `--locale`,
`--language`, `--bind` and `--view overview|repository|work`. `--snapshot` exports
one complete bounded publication without opening a listener; it cannot be combined
with `--view`, `--bind` or `--guarded`.

Overview preserves the canonical Project name and Purpose (or truthful absence),
plus recent Work. Work lists and exact Work URLs use Goal identity and explicit
Checkpoint association. Cards retain independent generated names/summaries and
original creation/latest-work-record times. Missing, stale, expired, deleted,
withheld and unsupported explanation states never become invented prose.

Work detail connects its actual related Decisions, alternatives, recommendation
and recorded reason, user choice and separately retained user rationale. Only
explicit Work scope or applied Decision relations admit them. Project-wide and
unassociated Decisions remain readable through CLI `decisions`, MCP
`canonical_inspect` and Operations; Viewer never fabricates a Work relation.

Repository explanation is independent of Work. Its authored narrative explains
relevant responsibilities and mechanisms with analysis scope, coverage, freshness
and uncertainty. A generic Code catalog is outside this reader. Relevant code is
accessible through the explanation's exact claim-specific Source panel.

Citations preserve Project, subject, language, realization, unit/range and original
bytes. Desktop panels and narrow focused reading return to the exact claim. Live
`GET /citation` reads bounded permitted copies, rechecking currentness, lifetime,
hashes and scope on every chunk. It never opens an arbitrary path, captures new
Sources, extends retention, generates prose or invokes a provider. Untrusted text
is escaped. Host/Origin and fetch-metadata protections remain enforced.

Static output contains the three destinations, bounded Work/Decision explanations,
selected admitted Sources and truthful omissions/currentness. Native fragments and
disclosures work without JavaScript or Runtime access. It includes no management
forms, authenticity token, live endpoints, document previews, bulk Code/diagnostic
catalog or Tools fragments. Atomic publication refuses relative/existing targets
and revalidates canonical, explanation and verification dependencies. Export
performs no upload and grants no sharing/provider authority.

General inspection and administration remain outside Viewer:

| Task | Existing interface |
| --- | --- |
| Canonical records / exact fields | CLI `advanced records list`; MCP `canonical_inspect` |
| Correction / supersession / forgetting | CLI `advanced records correct-context`, `correct-decision`, `supersede-decision`, `forget`; MCP `canonical_mutate` |
| Analysis / privacy / recovery | CLI `status`, `analyze`, `privacy`, `doctor`; MCP repository/health/privacy operations |
| Documents | CLI `document preview` / `document export`; MCP `document_preview` and existing realization operations |
| Scoped record query / relationships | Operations `inspect_project_records`, `inspect_record_relationships`, `inspect_project_record_field` |

`/inspection`, `/inspection.json`, memory management POSTs and `/documents/export`
are retired and return not-found without effects. `view=tools`, `view=decisions`
and `view=code` fail explicitly; they never fall back to Overview. No compatibility
aliases or alternate renderer exist.

Guarded confirmation is a separate, non-navigable request-scoped fallback. MCP
without host elicitation returns a launch command with the exact Runtime, Project
and `--guarded REQUEST_ID`. The executable prints `/guarded/REQUEST_ID`; that page
shows the exact effect, revision, fingerprint, scope and expiration. Only the
same-origin authenticated `POST /guarded/confirm` records an explicit user response;
it neither dispatches an effect nor grants general consent. CLI `advanced guarded` and host
elicitation retain their independent authority.

Reads load required explanation/context projections, without generating four
documents or loading privacy inventories, bulk graphs or global health diagnostics.
Important Work verification, historical adversity and material Source limits stay
visible in context. CLI doctor supplies bulk diagnostics.

Explicit host production remains `work explain prepare/compose/record` and
`repository explain prepare/compose/record`; reading does not author content.
Use `work source` / `repository source` to explicitly observe/read permitted
Sources. Existing current formats, privacy, correction and forgetting apply to
all generated fields. See the Projection and Privacy owners for exact admission.

Focused controls run through `rebuild/scripts/validate focused`: Viewer reading,
navigation, grounded reader, citation, scoped checks, publication and executable
HTTP tests; Operations inspection/memory/documents; Host guarded and explanation
callers. Actual browser and script-free captures support technical assessment.
Historical UI output was rejected; new samples remain unassessed by the user.
