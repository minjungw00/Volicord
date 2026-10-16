# Task-centered Viewer alternatives and review criteria

Status: inspectable design candidate; human feedback and adoption pending.
Observed on 2026-10-16 (Asia/Seoul). Entry HEAD was
`69616b292fc2e785aad3827d4a5ea19bd9a17ed3`, with a clean worktree. Only disposable
validation support changed; no Product, canonical record, legacy path, provider
boundary or polyglot capability changed. No frontend framework was selected.

## Reference and independent content

The discovered visual reference is
`rebuild/.local/prototype/prototype-v2-as-reviewed.html`, 156,969 bytes, SHA-256
`68f9a612e735efb48b2018f05eb8caf51df080b8b6e8760580fb8e844b489185`.
Its HTML/CSS and event-handling code were inspected without executing its scripts.
Adopted principles: concise comparative Work rows, editorial title/lead/body,
Work/Code separation, conditional observations and purposeful reading points.
Rejected weaknesses: generic navigation without an exact origin, small wrapped
code, mobile code below the reading viewport, arbitrary first-Source selection,
and indistinguishable whole-range change highlighting.

The content basis is the separate retained `reader-documents` review package:
frozen authored input, exact public Product readbacks, original file bytes,
canonical historical reports and retained Click stdout. All 16 historical asset
identities were reverified; the executable reference, its feedback and editorial
fixture are not content inputs. No prototype prose, rationale, locator, test
result or status was used as a new fact or expected answer. Admitted authored
explanations were independently matched to original Source bytes and paragraph
citations. Matching establishes binding, not semantic approval.

B/Click preserve all **20 units, 13 Source ranges, full Goals, seven independent
facts per Work and complete selected-Work audits**. B retains aggregate exit 101,
later recheck and unresolved fixture nondeterminism. Click retains the three
actual conditional observations, no patch, historical test counts and unverified
platform/policy limits. Both have missing before-state and no related Decision;
no diff, execution sequence, user rationale or Decision was invented. Captured
Current does not mean current freshness or a new execution. The historical
campaign ZIP was not independently verified.

## Composition and alternatives

The current production `reading.rs` renders the full narrative/fact sections
before `render_source_excerpts`; native fragments move between distant document
positions. `navigation()` offers Overview, Work, Code, Decisions, Analysis and
Documents, while scoped Code repeats Work meaning before its excerpts and graph.
Cards combine title, identity, interpretation, state and record disclosures;
metadata competes with the explanation even though audit sections are closed.
This is a composition/selection problem, not evidence that Rust cannot render a
usable interface. The earlier Rust/Vue comparison held nearly the same UI fixed;
it provides no evidence of either design's superiority.

The target replaces that DOM with compact rows, document sections, a dedicated
critical-limit band, a quiet recorded-status strip, grouped conditional cards and
one Source point per exact unit/citation. Synopsis is expandable in discovery;
full prose remains ordinary document reading. Full Goal, facts, provenance,
original quotation and existing baseline Decision/diagnostic surfaces are reachable.
No Source is selected until the user chooses its explanatory purpose.

Desktop opens an exact-claim/code workspace beside the document; 768 px and below
open code as a separate focused composition. Return/Escape restores the originating
paragraph and scroll; native history works. Script-free documents use ordinary
fragment/return links. Code stays at 15 px, preserves indentation and scrolls within
a keyboard-focusable region. Neutral lines and visible range boundaries avoid
claiming a finer line focus or before/after diff. Critical failures remain amber
and ordinary; audit identifiers remain in disclosures.

Visual inspection at all four widths found clear hierarchy, readable code and
restrained status treatment. The first mobile list was too tall; the final list
has a shorter introduction and synopsis disclosures, while retaining full critical
limits. Long limits still occupy substantial mobile space, and long code lines
require horizontal scrolling. Native zoom, screen-reader behavior and human
comprehension/preference remain unobserved. These are design judgments and
technical observations, not human usability acceptance.

## Review artifact and execution evidence

Open `rebuild/.local/viewer-task-design/review/index.html`, or run `python3 serve.py`
inside that package. All active paths are package-local. The entry links accurate
baseline snapshots, redesigned discovery/Work/code, Source-unavailable controls,
script-free documents, screens and complete check output. Baseline capture HEAD is
`33d447654292db0ef983cd03386475cb536d0cfd`; its entire Product crate/Cargo source is
unchanged at entry HEAD and at the support commit. Thus layout is current while
content/state remain explicitly historical.

Support commit: `7c990db64ad2edcb5db52fbbf8a7d2cbf183e576`; exact package support
copies were compared after committing. The preserved browser candidate manifest
SHA-256 is `a41899a9fbe5c89cfcd1e87845e4a4acb23cc29f7500b1a0dcc89bb276ae868f`.
The final package manifest SHA-256 is
`e40bdf08241f80e9a72e7f4bc1534c9f8c55c4d27c19a275997eba6c8ce143a5`.
The browser result SHA-256 is
`42ee159ecc4063926e0122f13dc63bf454af79a933ca0a931b4ef64a86267888`.
Per-file hashes, exact control/historical Work IDs and realization fingerprints
are in that manifest. Packaging adds only observations and the review entry link.

| Focused evidence under `rebuild/.local/` | Outcome |
| --- | --- |
| `validation/20261016T080747.099742Z-viewer-listener-reproduce-v_210vzu` | Original idle timeout: exit 1, errno 11 WouldBlock; PID exited |
| `validation/20261016T080905.337490Z-viewer-listener-committed-yzod5npa` | Repaired Rust listener: exit 0; idle/partial/disconnect/slow/rejections, same PID alive and later GET 200 |
| `validation/20261016T082035.540385Z-viewer-design-preview-connections-y209x6t2` | Delivered Python preview: same connection controls passed, PID alive; explicit SIGTERM cleanup |
| `viewer-task-design/review/observations/result.json` | 12 browser groups; 132 exact citation round trips; 1440/768/390/320, history/return, code scrolling, failures, malicious long text and offline documents |
| `validation/20261016T082418.968589Z-viewer-design-committed-rs4k683k` | 12 input/range/association/hostile/package controls passed after support commit |
| `validation/20261016T082331.076235Z-viewer-design-manual-package-vet42eea` | Exact manual launch; final entry/evidence links, mouse return, mobile code and full Source access passed |
| `validation/20261016T082136.473587Z-viewer-design-workspace-mzs8ophj` | Metadata exit 0; all nine packages and every local dependency under rebuild; no legacy dependency |
| `validation/20261016T082136.537951Z-viewer-listener-format-7z4nbodr` | Standalone Rust formatting exit 0 |

Complete wrapper results/stdout/stderr and browser inner results were inspected.
Initial restricted-sandbox binding failed; the authorized loopback rerun reproduced
the actual defect. Earlier browser check failures are preserved under ignored
paths; the final evidence above does not pool them into success. Aggregate/gate,
Final/V11, workspace Clippy/tests, provider runs and legacy validation were not run:
this task changes standalone disposable support only. No required tool remained
unavailable. [README.md](README.md) documents focused reproduction.

## Provisional review contract

Ask a reviewer to find B, explain its next-action mechanism and critical failure,
open the matching code, then return to the same explanation. Repeat with Click:
compare the three conditions, explain first-open versus stream reuse, and preserve
no-patch/missing-before/platform uncertainty. Compare current and target layouts
with the same retained content at desktop and mobile widths.

Unresolved user decisions are whether the document hierarchy and conditional cards
improve understanding, whether 768 px should use a focused code screen or a split
workspace, and whether the visible critical-limit/detail balance feels right.
Feedback may justify further design work; it does not silently select a framework,
change Product contracts or approve migration. No user action is required to finish
this repository task. Actual preference and adoption require the user's future review.
