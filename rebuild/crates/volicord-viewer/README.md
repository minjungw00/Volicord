# Volicord local viewer

This crate is the loopback-only local Project viewer. It renders existing
projection, privacy, health, document, and Guarded request data and delegates
all writes to `volicord-operations`. It owns no domain state or database.

Run it with an explicit Project and optional separate runtime:

```text
volicord-viewer --runtime /absolute/runtime --project PROJECT_ID
```

Export the current Viewer projection as one self-contained, read-only local
HTML snapshot and exit:

```text
volicord-viewer --runtime /absolute/runtime --project PROJECT_ID --snapshot /absolute/path/viewer.html
```

Snapshot publication is atomic, refuses a relative or existing destination,
opens no listener, and performs no upload or other network transmission. The
result keeps current degradation, privacy state, document previews, and a
closed Project/canonical/repository-analysis basis disclosure, but contains no
forms, authenticity token, mutation endpoint, script, or live-Viewer link. It
remains readable after the Runtime is no longer available. Sharing the file is
a separate user-controlled action outside this command.

Fixed product text is bundled in English and Korean. `--language` records an
arbitrary requested generated-content language without an allowlist. Because
the local Viewer has no active-host realizer, an arbitrary language displays a
truthful unavailable/degraded notice and never presents its fixed English body
as requested-language success.

Live reads use purpose-oriented views (default `overview`):

```text
/?view=overview
/?view=work&work=CANONICAL_GOAL_ID
/?view=code&scope=work&work=CANONICAL_GOAL_ID&entity=ENTITY_ID
/?view=code&scope=repository
/?view=decisions&decision=DECISION_ID
/?view=tools&tool=documents|memory|status|evidence
```

All links carry `locale=en|ko` and unrestricted requested `language`. Work and
Decision navigation supplies validated identities; no opaque-ID entry is needed.
The Work and Decisions root views list recorded choices in pages of 64;
`page=N` is a nonnegative decimal page index, accepted only on those lists.
Previous/next links preserve locale and language. Exact detail reads resolve
before display bounds. Malformed or mismatched selectors return 400; an identity
absent from this Project returns 404. Valid Work with no code remains readable
with a scoped gap. When stored analysis is unavailable, entity detail is explicitly
unverifiable; the page preserves canonical remainder without claiming that entity
exists or is absent. Removed `level` forms are usage errors/400.

`volicord viewer open --view overview` forwards to the installed sibling
`volicord-viewer`; direct launch also accepts `--view`. Named `code` launch opens
repository scope and `tools` opens health/privacy. Whole-snapshot export accepts
no `--view` or `--level`; it includes bounded Overview, Work, repository Code,
Decisions, health/privacy, and four document previews on one read basis. Internal
fragment links refer only to included sections; omitted Work-specific code detail
is explicit. Snapshot disclosure retains original audit text, not redacted text.

Memory correction, supersession and forgetting, explicit document export, and
exact Guarded confirmation use the existing Local Operations authority. Forms
return to parsed allowlisted views. Host, Origin, Fetch Metadata, authenticity,
CSP, exact target/revision and retry boundaries apply to every mutation. Ordinary
navigation performs no analysis, provider request or canonical mutation. No
arbitrary filesystem-path source endpoint is provided.

Work result/Goal/rationale text may be labeled original quotations or excerpts.
These do not claim a semantic summary or translation. Missing Purpose/result/user
rationale remains unavailable. Completed work, verification, user review and
acceptance are independently shown; failed/rejected history remains visible.
Code diagrams show static grounded evidence, not confirmed runtime ordering.
The same entities/relations have a keyboard/touch-readable list and disclosure
path. Only diagrams scroll horizontally; ordinary prose/evidence wraps.

Client disconnects and response-write failures end only the affected connection;
the listener continues serving subsequent requests without retrying a mutation.


Code exploration keeps SCC cycles together and derives edge direction only from
stored relations. Symbols lead diagram labels; shortest distinguishing path suffixes
and real range/kind distinguish duplicate names. Sized nodes and separate edge ports
retain self-loops/parallel edges. SVG links jump to full entity names and locators in
the equivalent native-details list, so full labels are available on keyboard/touch
without hover. Relationship lists preserve every constituent identity and count;
there is no relation grouping. Resolved endpoints have explicit repository-detail
links; unresolved endpoints never become nodes. Source inspection discloses existing
snapshot-bound Source/range data and performs no filesystem-path HTTP read.

Read requirements are explicit: `ProjectionReadRequirements { code, inspection }`
feeds `LocalOperations::project_projection_read_profiled`. Overview, Work and
Decisions request neither section, using the existing AnalysisMetadata reader for
snapshot identity, coverage and freshness. Code requests stored graph bodies but
not Candidate inspection; Memory requests inspection but not code. Documents and
whole snapshots request both. Evidence and Status request code; only Status runs
full stored-graph integrity diagnostics. Other pages expose that these diagnostics
were not requested, while store/recovery health and material metadata gaps remain
visible. Missing requested analysis is `Unavailable`; omitted materialization is
`NotRequested`. Document generation and narrative planning reject incomplete
requested-section input. Default Operations Recall/documents keep their full
contracts. No new persistent cache or invalidation policy is introduced.

`ViewerRenderProfile` exposes one projection pass, standalone metadata reads,
graph decode attempts (including failed attempts), Candidate basis reads, document
generation count and monotonic stage times. `render_snapshot_profiled` reports the
same one-basis snapshot path. Named fixture-specific timing budgets and reproduction
commands are in [Viewer findings](../../validation/end-to-end/multi-repository/viewer-reading-findings.md)
and [read budgets](../../validation/end-to-end/multi-repository/viewer-read-budgets.json).
They do not establish a universal latency guarantee or browser qualification.

The maintained nested-workspace installer places all three executables together:

```text
rebuild/install.sh --prefix /absolute/install-prefix --runtime-dir /absolute/runtime
```

From the bound repository, with `/absolute/install-prefix/bin` on PATH:

```text
VOLICORD_RUNTIME_DIR=/absolute/runtime volicord viewer open --view overview --language en
VOLICORD_RUNTIME_DIR=/absolute/runtime volicord viewer export --output /absolute/path/viewer.html --language en
```

Use a fresh absolute output destination. An explicit `--project PROJECT_ID` can
replace repository resolution. For development, build with `cargo build
--manifest-path rebuild/Cargo.toml -p volicord-operations -p volicord-viewer` and use
the sibling executables under `rebuild/target/debug`. Launch selects a view;
navigation supplies exact Work/Decision/entity identities. Export always renders
all bounded snapshot sections, regardless of the live entry view.

Selecting an entity focuses both diagrams and the equivalent entity/relation list
on that exact entity's bounded stored incoming/outgoing neighborhood, even when
it was omitted from the initial map. The selected node is retained and marked.
The Work/repository scope is still explicit; selecting a real one-hop neighbor
in Work scope does not assert canonical Work ownership of that neighbor. Resolved
relations retain both actual endpoints, unresolved targets remain evidence only,
and omitted relationships keep exact counts. A generic repository explanation
uses stored topology without claiming a canonical Work-seed link.
