# Bounded Project inspection delivery review

Date: 2026-10-20 (Asia/Seoul). Technical and browser support are separate from
human acceptance, which remains pending. No new privilege is proposed or required.
The inspected execution baseline was clean `4d6a0a51`; historical `7cbb628e` was
compared, not checked out. Later Work/Decision article and verification changes
were already present and were retained.

The privacy owner's [deny/allow table](../../docs/design/privacy-and-provider-boundary.md#bounded-project-data-inspection-threat-model)
owns authority; this report records implementation and coverage, not new policy.
All changed production paths are under `rebuild/`. No legacy path, durable schema,
provider grant, language/runtime dependency or policy/audit store changed.

## Changed and retained routes/functions

| Entry / user purpose | Delivery | Verified consumer |
| --- | --- | --- |
| `?view=tools&tool=memory` | Changed: purpose-led Work/Goal inspector; choose Context, Checkpoint, Decision, Question, Source, Candidate or Derived metadata | HTTP, adapter, desktop/mobile browser |
| `?view=tools&tool=evidence` | Changed: Source inspector; full inventory, Checkpoint history, Project identity/revision and diagnostics retained under deliberate disclosure | HTTP, browser inventory/history expansion, large repository bound test |
| `/inspection` | New native GET query form; Project-bound selector, literal visible-summary/ID search, typed role/state, 1–20 rows, stable ID ordering, exact identity/detail/relations and useful return links | Independent HTTP security matrix; 1440/390 px browser |
| `/inspection.json` | Same authorized Operations projection; query fingerprints contain permitted fields only | Actual HTTP transport and listening executable |
| Source detail | Existing exact reader reused; bounded `reference` exposes already inspectable canonical locator/snapshot metadata as text, excluding user-turn body; body still requires exact retained Work scope | HTTP allowed reference/body, wrong Work, no Work, expired copy controls |
| Candidate detail | Existing content admission before summary search, count, fingerprint and field read; withheld field is 403, permitted metadata survives | HTTP expired, pending forgetting, hidden-content fingerprint controls |
| Derived inspection | Metadata, expiry and independently admitted canonical links only; no general body query | HTTP hidden-body/purpose, foreign-link and pending forgetting controls |
| `?view=tools&tool=status` | Retained specialized Analysis/privacy health, coverage, failure and CLI recovery guidance | Browser and unchanged health/recovery tests; polyglot fixtures preserved |
| `?view=tools&tool=documents`, `POST /documents/export` | Retained four grounded previews and explicit authenticated export | Browser preview, executable HTTP publication/authentication tests |
| Document adoption | Existing non-adoption boundary retained: export never adopts. Baseline has no Viewer/Operations adoption endpoint to preserve; no new adoption authority was invented | Existing export noncanonical tests and explicit UI notice; broader adoption journey remains outside this change |
| Work/Decision documents and evidence index | Retained narrative/verification/claim-linked index and native original-record links; inspector links back to documents | Browser exact Work and Decision evidence-index deep links; reading/explanation/citation regressions |
| `POST /memory/context/correct`, `/memory/decision/correct`, `/memory/decision/supersede`, `/memory/forget` | Existing domain Operations retained for displayed exact targets, closed by default; inspector forms return actual identity/revision/completion receipts | HTTP authenticated correction/forgetting/Decision supersession, exact receipt scope, stale revision and cleanup tests |
| `/guarded/ID`, `POST /guarded/confirm` | Retained exact effect fingerprint/revision, Origin/authenticity and user response Source; no automatic dispatch | Executable HTTP and transport positive/negative confirmation tests |
| Static Viewer snapshot | Existing bounded publication retained; included records under native disclosures, no query/form/script/token or general DB export | Script-disabled browser, all fragment targets, current disclosure barriers and publication regression tests |

Project switching remains one separately launched viewer per authorized Project.
This is an explicit existing trust boundary, not a global Project enumeration.
Search covers displayed summaries (240 characters) and identities only. It does
not promise full-text Source/Derived search. Every exact field chunk rechecks the
current owner; continuation never renews retention. Recorded Question basis revision
and current target revision are distinct on relationships.

## Independent security matrix and results

`volicord-viewer/tests/inspection.rs` contains ten Product HTTP cases plus a
review-fixture utility. Every case uses real Operations stores and public HTTP
consumers. SQL is used only for isolated test fault injection, never Product input.

| Case | Allowed output | Forbidden output / actual denial |
| --- | --- | --- |
| Mixed Projects | Own exact Goal and admitted Source metadata | Foreign/missing exact identity both 404; foreign term count 0; unrelated foreign insertion leaves own fingerprint unchanged |
| Bounded literal queries | Typed state/role and admitted summaries/IDs; deterministic ID pages with no duplicates | SQL-like text matches 0; malformed grammar, unknown keys, overflow, duplicate keys and invalid sizes 400 |
| Candidate lifetime | Admitted bounded summary; kind, times, retention and omission metadata | Expired term count 0, no snippets or content-derived relations; changing hidden text leaves public fingerprint unchanged; withheld exact field 403 |
| Pending canonical forgetting | Safe Candidate/Derived identity/state metadata while cleanup is blocked | Candidate/Derived marker text and linked forgotten Source absent before cleanup completes |
| Source scope/lifetime | Retained body only with exact Work; full locator reference remains text | User-turn body absent, no-Work and expired body unavailable, wrong Work no bytes, body term count 0; no path evaluation |
| Relationship admission | Explicit canonical IDs with same-Project independently admitted targets; bounded continuation | Foreign target identity absent, omission explicit without hidden count; matching path/title does not join Work |
| Correction/supersession/forget after list | Current corrected revision; explicit domain result | Old page and old revision 409; forgotten exact identity 404 and removed term count 0 |
| Authority/escaping | Correct local Host/Origin, escaped stored HTML and literal search; no-store | Wrong Host 421; wrong Origin/fetch/authenticity 403 before mutation; no script execution or private error detail |
| Dependency failures/read-only | Canonical remainder and explicit 503 unavailable/unsupported/corrupt classification | No fake empty count, database detail, hidden content, Source capture, canonical mutation, provider call or retention extension |

Source, forgetting, snapshot, guarded and document-publication regression fixtures
supply additional original owner checks. Synthetic controls prove their asserted
mechanical behaviors only; they are not naturalistic dogfood or article-quality
approval. No Final/V11 or external provider run was requested or performed.

## Validation and browsable review

Use `rebuild/scripts/validate focused` for every check. Original stdout/stderr,
numeric exit, termination and per-case results remain in ignored
`rebuild/.local/validation/`. Failed setup/test-assumption attempts remain there;
they are not represented as passing executions.

Direct checks cover the complete viewer targets, Projections targets, Operations
`local_operations`, `source_observations`, `source_observation_root`, strict Clippy
for all three affected packages, nested formatting, nine owner contracts and Cargo
metadata isolation. Listener tests and Chromium require loopback access outside
the default socket-denying sandbox. That environment failure was distinguished
from Product failures and rerun with local listener permission.

The maintained browser driver is `inspection_browser.cjs`. Its config points to an
actual built viewer and the retained review fixture, using the existing local Korean
font configuration. It checks desktop/mobile native search/focus, filter/pagination,
exact fields, Work/Decision evidence-index links, expired Candidate, private Source
body, Derived metadata, empty Project, Analysis and Documents. Offline is opened
with scripting disabled and every native fragment target is checked. The fixture
uses ten meaningful Work/Goal records, Checkpoint history, Decisions, Candidates and
Derived restrictions through maintained Product fixture APIs; no model was called.

Current review entry: ignored
`rebuild/.local/inspection-review/browser/current.html`; `current.json` identifies
its unique run directory. Each new run preserves its own `result.json`, executable,
fixture and candidate fingerprints, screenshots and script-free Product snapshot.
The final committed rerun is reported in the conversation, including its exact
commit and numeric outcomes. Review packaging is not added to Product navigation.

Remaining limits: one Project per viewer; metadata-only Derived inspection; search
on visible bounded summaries; existing missing document-adoption endpoint; human,
screen-reader and native-zoom acceptance remain separate. The inspector does not
replace repository/Work explanatory documents or claim their semantic quality.

## Internal query scaling follow-up

Execution baseline: clean `c06468e9ede07619b81eac205344fdddf307978d`.
The old inspection implementation was compiled with the same controlled profiling
fixture/test driver; no old checkout or legacy workspace was used. Retained stores
under ignored `inspection-cost-goals` were reused sequentially. All **96 complete
HTTP JSON responses** (six datasets × eight classes × two reads), including counts,
relationships and fingerprints, matched byte for byte. Fixtures explicitly assert
20/110/1,010 Goal records; the corresponding Checkpoint counts are 50/410/4,010 and
Source counts 16/106/1,006. Candidate and Derived queries each add 11/101/1,001
records. Dense variants concentrate Checkpoint edges on one Work with unchanged
record counts. These are synthetic store-scale controls cloned from maintained
producer rows, including absent historical metadata, not genuine user Works.

Warm HTTP query time, milliseconds (spread relationships; response limit seven):

| Added scale | Work old → current | Checkpoint old → current | Candidate old → current |
| --- | --- | --- | --- |
| 10 | 11.049 → 7.865 | 11.131 → 10.040 | 10.197 → 9.928 |
| 100 | 107.721 → 42.541 | 122.002 → 61.514 | 108.217 → 48.185 |
| 1,000 | 6,081.856 → 441.248 | 6,298.231 → 613.634 | 6,187.169 → 443.965 |

Largest first-read Work time was 6,156.643 → 442.106 ms; dense warm Work was
6,085.471 → 431.574 ms. These are request-first/repeated reads, not cleared OS page
caches or an isolated-host SLA. JSON sizes stayed identical: largest spread Work
7,720 bytes, Checkpoint 10,521, Candidate 5,822, Derived 4,933 and Source 4,072.
Actual large HTML GET returned seven rows in 110,137 bytes at 1,215.769/1,193.926 ms
and left canonical state unchanged; this HTML measurement has no paired old timing.
A separate 10/100/1,000-Goal-only family keeps the other classes small and reports
its own timings without extrapolating the relationship-heavy improvement.

Current cost counters for the largest spread dataset: 6,038 indexed canonical
records, 32,200 forward/reverse admitted edges, 1,010 Work rows visited/matched,
seven retained, and 1,012,127 fingerprint bytes. The Checkpoint query visits all
4,010 matches and hashes 5,631,570 bytes. Canonical read was 351–359 ms; index build
35–37 ms. Dense Work has 4,007 relationships on one transient row. Thus the rich
page is bounded, while owner input, identity/edge index, exact scan/hash and one
high-degree row retain the explicit input-dependent costs in the projection owner.
There is no full matched-row JSON allocation, repeated full-row edge lookup or
second relationship-query rebuild. Bulk Candidate admission indexes forgetting IDs
and shares the existing content-omission precedence without graph/round cloning.

`validate focused inspection-goal-baseline` exited 0; its process CPU user/system
was 217.35/1.33 s and peak RSS 121,504 KiB. `inspection-goal-current` exited 0;
CPU was 31.46/2.53 s and peak RSS 113,152 KiB. The latter additionally ran the
counter passes, large HTML consumer and Goal-only family; these aggregate values
are observations of those complete processes, not paired per-query allocation or
RSS deltas. The per-query table compares equal HTTP requests. Allocator counts and
constant-time side-channel guarantees remain unmeasured. Earlier setup failures,
lock-contended runs and a Context-cloning fixture that failed to vary actual Goals
remain preserved privately and are excluded from these comparative claims.

Direct existing support: 11 inspection HTTP tests, 41 Operations consumer tests,
45 Projections tests, strict Clippy on the three affected packages, and nested
workspace package/dependency isolation. No provider, Final/V11, new authority,
public schema, analyzer-language scope or legacy path changed. Independent stress
and browser results remain separate from human acceptance.

A later malformed-fixture attempt also reproduced an existing canonical decoder
failure: a little-endian count presented to the big-endian Source-ID decoder
requested 1,152,921,504,606,846,976 allocation bytes and aborted with signal 6.
The structured-value reader now rejects counts exceeding retained input before
reservation; Source IDs enforce their 16-byte width. Focused positive/truncated/
wrong-endian/maximum-count controls pass. A separate public HTTP regression corrupts
a real Goal's encoded applicability count: both HTML and JSON return 503 below
2,048 bytes, with no record data or invented zero count. This is a reproduced local
corrupt-state resource defect, not evidence of a remote SQL or file-read exploit.

## Owner-valid scale, independent stress and packaging

The first scaling matrix above is a read-side partial-state control: its cloned
Checkpoint support and Context revision history were incomplete, so a later owner
mutation rejected that state. It is not evidence of a healthy Project passing at
that scale. The subsequent seed copies exact Goal/revision/Source support, avoids
foreign-Work applied-Decision injection, and completes a real canonical Source
mutation before preserving its measurement inputs. Original failed attempts and
partial controls remain private, separately identified.

On those owner-valid retained stores, `inspection-valid-baseline` and
`inspection-valid-current` both exited 0. Thirty-six complete HTTP responses
(Work/Checkpoint/Source × six datasets × two reads) match byte for byte, including
counts, order, relationships and fingerprints. The current run also measures all
other classes. The largest Project has 1,010 Works, 4,010 Checkpoints, 1,007 Sources
and 1,001 additional Candidates/Derived records on the respective queried views.
Spread warm Work: **6,009.078 → 405.722 ms**; Checkpoint: **6,040.342 → 567.706 ms**;
Source: **5,814.055 → 429.169 ms**. Work first request: 5,941.790 → 445.309 ms.
Payloads remain 8,623 / 9,554 / 5,109 bytes respectively.

The current index has 6,039 canonical records and 26,200 admitted forward/reverse
edges. Work visits/matches 1,010 rows, retains seven, hashes 1,141,127 bytes;
Checkpoint visits 4,010 rows and hashes 5,079,570 bytes. Canonical read/index/hash
samples were 333/31/40 ms for Work and 335/31/206 ms for Checkpoint. Process peak
RSS was 117,428 KiB for the old three-class run and 106,080 KiB for the current
all-class/counter run; CPU user/system was 82.32/0.56 s and 27.47/2.04 s. Those
process workloads differ, so these are observations, not paired per-call deltas.
No allocator counts, strict constant-time property or absolute resource SLA was
measured. Canonical input decoding and high-degree relationship storage still
scale with input.

The paired actual HTML consumer also exits 0: warm old/current 7,255.250 /
1,177.653 ms, 110,080 / 110,137 response bytes, seven rows, and unchanged canonical
state. HTML byte identity is not asserted. Both native and Product browser drivers
exercise real pages; technical articles are explicitly authored synthetic controls
recorded through existing prepare/record APIs, not new generation or user approval.
Closed evidence disclosures are opened natively and offline older-Work navigation
uses the catalog. No Production UI or authority was changed for these fixtures.

Independent HTTP/API cases now include 1,000 exact expected Goal IDs over 50 pages,
typed overlapping Work/Source IDs, Unicode byte limits, hidden Source mutation,
256 additional explicit Checkpoint edges with exact continuation, distinct recorded
and current Question revisions, foreign Candidate/Derived envelopes, and concurrent
correction/forgetting. Fifteen inspector tests plus a separate corrupt-count HTTP
test pass. All 103 Canonical tests and 45 Projections tests pass. A full Viewer run
found one duplicate-fragment snapshot test failure; the same test fails against
original `c06468e9` Production sources. With only that independently reproduced
baseline failure filtered, all 141 Viewer tests pass; the three explicit profiling
cases are ignored by default and have their separately recorded executions.

Packaging reproduced copying credential-looking stderr and SQLite bytes renamed
to an allowed HTML name. The current boundary excludes raw logs/live HTML/status
JSON and only copies driver-sealed, format-checked static outputs. Eighteen tests
cover source/DB/log/credential-looking neighbors, disguised bodies, symlinks/parent
paths, Unicode-safe literal names, traversal, changed outputs, active content,
normalized PNGs, bounded reads, archive structure, candidate/manifest tampering,
optimized-Python identity checks and immutable original bytes. Preview delivery
independently verifies with 96 members; exact final candidate/archive identity and
post-commit executions are reported in the conversation. No historical private
archive was reclassified as a leak or rewritten; shareable output uses a new path.
No legacy, schema, provider, Final/V11 or human-acceptance authority changed.
