# Viewer frontend architecture tradeoffs

Status: measured comparison; conditional recommendation; human review and Product
adoption pending. Evaluation date: 2026-10-16 (Asia/Seoul). Production was not
migrated. No authoritative Final/V11, provider call or new explanation generation.

**Prefer Rust rendering with bounded progressive enhancement for the demonstrated
reading tasks.** Both implementations provide the required layout and interaction.
Vue improves Work navigation in this experiment, but establishes no comprehension
advantage or distinguishable code-selection advantage. Its extra build/dependency
and separate offline-renderer responsibilities are not justified by these tasks
alone. This is a lowest-total-cost inference, not measured maintenance hours or a
binding framework decision. Revisit for substantial client-side state, repeated
cross-Work navigation, or human feedback that the Rust candidate cannot satisfy.

## Identity, inputs and review

The actual entry HEAD was `33d447654292db0ef983cd03386475cb536d0cfd`, with a clean
worktree. Root/scoped AGENTS and the charter, decisions, architecture, domain,
projection, privacy and validation owners were inspected. Only this validation
directory changed. All nine nested packages and local dependencies remain under
`rebuild/`; no legacy dependency or polyglot capability change was introduced.

Support commit: `f6718482e31cfb80d75f39d6c10b33e3284bfa10`.
The final browser result binds the support HEAD, driver, configuration, compiled
executables and build inputs by SHA-256. Vue is **3.5.44**, esbuild **0.28.2**,
production runtime-only, minified, tree-shaken and without development features.
The Rust candidate uses the same standalone Rust HTTP scaffold, CSS, admitted
content and local assets, with a 1,131-byte progressive-enhancement script.

The retained `rebuild/.local/reader-documents/` package was available. Its
`review/identities.json` and 16 named original assets were reverified, including
the frozen campaign manifest. Exact original file hashes, public readback,
realization fingerprints, claim-byte hashes and complete admitted code ranges
were checked before building. The shared normalized data SHA-256 is
`ce3cc725756c4f05c929c3d5465bbe90d306a8e520d32eb5945a5e249c940261`.

| Case | Historical Work | Fresh authored control | Preserved material |
| --- | --- | --- | --- |
| B | `8d2bd548b3c992616f798045ec72d5da` | `1d2ec2b7905bf60780ddab837eb0a9e6` | 9 units, 7 Source ranges; absent/stale next actions, three detailed limitations versus 32 retained coverage rows; aggregate exit 101 and later recheck |
| Click | `e4edec546fd360cc41c178479802560e` | `af3fa5c56ffd169acf53bfa6df776a6d` | 11 units, 6 Source ranges; relative lazy/eager and absolute lazy observations, first I/O, stream reuse; no patch, 14 passed/100 deselected, historical 16 scratch cases |

The Click control ID above is verified in the private manifest; all historical
and current identities remain separately inspectable. These are previously
authored, Source-derived evaluation explanations, not newly generated Volicord
output. Both candidates retain every unit, original Goal, full selected-Work audit,
status distinction and exact available Source. All comparisons here have
`missing_before_state`; no invented before/after diff is shown. Historical
completed status is not current control completion or acceptance. Captured
`current` describes the preserved readback, not renewed Source freshness.

The campaign ZIP declaring SHA-256
`30e91cae404a5be362fd32be911b34ee13deb92ba0b8a75432590bdf9efadf6b`
was not found or independently rehashed. The narrower retained-member verification
is sufficient for this local comparison, not archive qualification. Local reuse
is within this request; no campaign/source content was sent to a provider or
published. Click's retained BSD license accompanies the local packages. No public
redistribution permission for private campaign material is asserted.

Open `rebuild/.local/viewer-architecture/review/index.html` for exact instructions,
original baseline snapshots, common script-free exports, and actual desktop,
768/390/320px screenshots of reading, selected code and failure states. Run either
`rebuild/.local/viewer-architecture/run-3/rust-reader` or `vue-reader` and open its
printed loopback URL for the interactive comparison. Stop with Ctrl-C.
[README.md](README.md) gives the two Source-bound tasks and reproduction commands.
The local review manifest hashes the artifacts and links the original lineage.

## Experience and causes

Both redesigned candidates put a meaningful heading, synopsis and critical limits
before detailed narrative. Desktop uses a sticky code pane beside the explanation;
the selected claim is repeated beside its exact code. Mobile selection provides a
code-focused view and restores the original paragraph, keyboard focus and scroll
position. Code is 15px with 1.8 leading; long lines scroll within the pane. Prose
is 17px, the synopsis 19px. Shared typography, spacing, content and styling prevent
framework preference from being confounded with a more polished candidate.

The final browser run passed 23 control groups, including **240 citation round
trips**, at four widths. Controls independently compared all prose, full diagnostics,
Goal and cited code against retained inputs; tested keyboard activation/focus,
scroll restoration, long original text, hostile long headings/prose/Source text,
unknown Work, injected Source failure, Vue data-fetch failure and script-free
offline citation/return. Hostile text remained text, with no injected image/script
or remote request. Actual candidate requests stayed on the active loopback origin.
Native zoom, screen-reader use, human understanding and subjective visual
preference remain unobserved. Screenshots are inspection artifacts, not acceptance.

| Observed cause | Evidence and implication |
| --- | --- |
| Whole-snapshot scope | Historical B: 990,580 bytes, 3,495 elements, 239 details, 51,693px document height at desktop. Click: 471,262 bytes, 3,531 elements, 238 details. These are snapshot counts, not live latency or peak memory. |
| Live versus snapshot | Current live Work has 330/384 elements for B/Click, versus Rust 316/327 and Vue 318/329. The historical thousands of elements do not demonstrate a Rust-renderer problem. Snapshot generation combines multiple reading/inspection/document surfaces. |
| Hierarchy and scrolling | Existing `reading.rs` separates narrative from `render_source_excerpts`; native fragment navigation makes a long jump. Grid/sticky layout, focused mobile reading and explicit return solve the demonstrated interaction in either renderer. |
| Hidden diagnostic cost | Complete audits still contribute 59,160/72,786 UTF-8 bytes for B/Click. Closing a details element does not remove transferred bytes or DOM. Diagnostics were retained in both candidates. |
| Citation precision | Green backgrounds identify each admitted range line by line, separately from diff semantics. The fixture supplies a range, not a finer token/line entailment proof; the UI does not invent one. |
| Initial Source selection | Both panes initially show the first retained range (B routing / Click conversion). The explicit mechanism citation selects next-action / first-open code. Better default relevance remains selection-policy work, not a framework advantage. |
| Information and selection | Authored headings/refs are already good inputs. Neither framework fixes missing before-state, poor autonomous Source selection, unsupported claims or generation quality. Those need separate Product evidence. |

## Measurements

Final timing evidence: `rebuild/.local/viewer-architecture/browser-full-3/result.json`,
SHA-256 `e7f1a01e630555774551de61222f950931b9d76633b1517ece6c7a4cd4e046f7`.
Resource/transfer evidence: `resources-5/result.json`, SHA-256
`7dbeab187f55657ec78098a06b4ce3665886f398c9e54bdacd6672bbdc146f4f`.
Both paths are relative to the same ignored comparison root.

Environment: Ubuntu 24.04.4, Linux/WSL2, i7-13700K, 16 visible logical CPUs,
84,336,533,504 bytes visible RAM; Rust/Cargo 1.97.1, Node 24.19.0, npm 11.17.0,
Chromium headless 149.0.7790.0, Playwright Core 1.64.1 and Noto Sans KR.
The font file SHA-256 is
`018174e8cdd366aff67c8d366c7dc806a7babbe7f2819fc23b00d59f7b8353d0`.
No CPU/network throttle; local loopback; 1440×900 timing viewport. Four responsive
widths were checked without mobile CPU emulation. Exact configuration and paths
are in the result. Measurements were sequential; no task-owned compile ran during
the final browser/resource samples. OS/background-service load was not eliminated.

There are seven fresh-browser repetitions per case/candidate, each followed by a
warm navigation: **84 rows**. Cold means fresh browser/context, not flushed OS/font
or disk cache; the server process is reused. Warm retains browser process/cache
after intervening task activity. Prototype order alternates; current production
baseline runs separately. Each interaction starts from the same task position:
Overview → selected Work → mechanism citation → original claim. Times include
Playwright dispatch, readiness and two animation frames. All raw rows, CDP paint
and navigation entries, DOM counters and process memory are retained.

Milliseconds below are median [min–max], n=7 per row:

| Case/candidate | Cold content | Warm content | Cold locate Work | Warm locate Work |
| --- | ---: | ---: | ---: | ---: |
| B production | 196.2 [181.8–210.3] | 104.0 [103.5–104.2] | 154.1 [137.0–156.9] | 144.7 [141.2–150.8] |
| B Rust | 104.8 [93.1–106.7] | 39.7 [38.6–55.4] | 108.1 [100.6–108.9] | 93.0 [91.2–104.3] |
| B Vue | 102.7 [83.7–248.7] | 39.1 [38.6–39.5] | 56.1 [54.7–71.9] | 58.3 [57.2–74.8] |
| Click production | 269.3 [191.0–368.7] | 99.7 [87.4–104.4] | 133.8 [129.6–136.2] | 138.2 [125.2–140.0] |
| Click Rust | 103.6 [99.2–149.2] | 54.4 [39.1–56.0] | 106.6 [95.4–138.4] | 92.5 [89.1–93.3] |
| Click Vue | 87.2 [78.0–106.4] | 54.0 [38.8–55.7] | 65.4 [53.2–69.2] | 58.3 [57.3–72.4] |

Selection and return medians are 66.5–66.8ms in every group, with individual
samples approximately 66.1–67.2ms. They are dominated by the driver/frame floor;
this instrumentation cannot rank sub-frame event handling. Vue's client navigation
avoids a new HTML request and shows a measurable advantage for this task. Cold
content ranges overlap, including a 248.7ms Vue B outlier. Smaller JS did not
automatically produce faster cold content. Earlier runs varied materially and
are retained; they are not pooled into these final values.

Fresh browser launch medians are 20.8/20.8/20.5ms for production/Rust/Vue, ranges
19.9–24.6/20.2–28.2/19.6–22.0ms. Recorded FCP ranges across cold/warm cases are
56–332/16–124/28–244ms; FCP alone does not mean the explanation is usable.

| Candidate | CDP script ms cold/warm | Main-thread task ms cold/warm | Browser RSS / PSS MiB cold → warm | JS heap MiB cold → warm |
| --- | ---: | ---: | ---: | ---: |
| Production | .052 / .024 | 125.1 / 27.8 | 433.8 / 290.5 → 491.5 / 329.2 | 2.91 → 9.10 |
| Rust | .404 / .192 | 72.4 / 16.7 | 396.4 / 261.1 → 439.7 / 289.2 | 2.15 → 7.33 |
| Vue | .979 / .517 | 75.0 / 19.1 | 388.7 / 257.8 → 421.6 / 279.8 | 2.78 → 7.23 |

These are medians over both cases (14 samples per temperature). Script counters
include browser automation; production HTML itself has no JS. Task time includes
layout/font work. Heap counters are not GC-normalized; warm pages have intervening
navigation/automation history. RSS/PSS sums cover CDP-listed browser processes
after the task, exclude no observed entry, and are **not peak memory**. RSS sums
double-count shared pages; PSS apportions them. No memory-leak or universal
framework-efficiency conclusion follows.

Live Rust server RSS ranges were 20,092–25,972 KiB production, 2,356–2,616 KiB
Rust prototype and 2,344–2,444 KiB Vue prototype. Production request CPU was 3–5
Linux ticks (100 ticks/s); prototypes were 0–1 tick. Zero ticks is below sampling
resolution, not zero work. Separate n=7 prototype lifecycle measurements (startup,
two Work GETs, assets and wrong-Host rejection) gave median user CPU 1.233/.866ms
and median RSS 2,596/2,356 KiB. Startup medians .529/.514ms have overlapping
.489–.758/.489–.744ms ranges. **The prototypes serve frozen authored data and
omit Local Operations, live canonical validation and SQLite. Their advantage over
production is not an isolated renderer improvement or a production memory budget.**

Exact bytes, with offline gzip size in parentheses:

| Asset | Production | Rust candidate | Vue candidate |
| --- | ---: | ---: | ---: |
| B live HTML | 104,066 (18,658) | 98,328 (12,799) | 3,381 (1,397) |
| Click live HTML | 130,051 (20,191) | 115,667 (13,489) | 3,381 (1,397) |
| JS | 0 | 1,131 (495) | 60,339 (23,969) |
| Additional data | inline | inline, selected Work | 163,305 (23,563), both Works |
| CSS | embedded existing CSS | 3,084, embedded in HTML | same 3,084, embedded in HTML |
| Standalone executable | Viewer 19,289,504 | 4,749,096 | 4,805,656 |
| Local executable tar / tar.gz | see three-product total below | 4,761,600 / 1,210,084 | 4,812,800 / 1,233,139 |

Servers send uncompressed bodies; gzip is potential compression, not observed
wire encoding. First Work payload totals are 99,459/116,798 bytes Rust and 227,025
bytes Vue, before HTTP headers. Overview-to-Work Rust navigation downloads another
HTML/script pair with `no-store`; Vue preloads both Works once and navigates locally.
Thus selected-page and full-journey transfer are different comparisons. Audit/source
duplication and eager data selection contribute to Vue's JSON size; neither is a
necessary property of Vue. HTML process-authenticity tokens make production hashes
vary between listeners despite identical substantive output.

The current release-built CLI/Viewer/MCP are 30,677,408 / 19,289,504 / 32,995,064
bytes: **82,961,976 bytes installed**, individual gzip totals 22,635,617 bytes.
Prototype packages include Click notices, and Vue includes its MIT notice; their
small fixed-data scaffold is not the full Product. No projected full-Product Vue
package size is claimed.

## Installation, security and maintenance

* Current `rebuild/install.sh` builds all three Rust executables with Cargo, installs
  them and runs doctor. Node-free PATH installation passed on the host (17.52s
  wrapper, existing target/cache). A clean Debian 12 container then ran the **actual
  installer from an exact-entry-HEAD Git archive**, with a new build target,
  preinstalled Rust/C tools, a read-only Cargo download cache, no Node/npm and no
  network. It passed installation and doctor. Wrapper monotonic duration: 88.83s,
  n=1. Toolchain/package download time is excluded. Container realtime subtraction
  reported 93.80s, inconsistent with the parent clock; it is not used as benchmark
  time. This is a clean target build, not an empty dependency-cache measurement.
* Both prototype sources built and served HTTP in clean Debian without Node/npm.
  Their Vue source distribution contains the prebuilt, hash-checked frontend asset;
  a Rust compiler/linker is still required. A matching shell checksum check and
  repeated production asset builds were exercised. Current prebuilt Product
  CLI/MCP also ran in clean Ubuntu 24.04 without Node/npm or network. Host-built
  Product binaries **failed on Debian 12 because GLIBC_2.39 is absent**; the fresh
  Debian source build worked. A Linux ABI baseline is a real release concern.
* Vue developer setup installs 25 npm packages, 28,688,982 file bytes in this
  environment; the lock covers 50 entries including optional platforms. The bundle
  includes five Vue runtime modules plus the authored reader. Developer Node/npm,
  esbuild's platform binary, lock review and dependency updates have real costs.
  A future Node-free **Git-source** frontend build would require checked release
  assets or a verified asset-acquisition policy; this experiment changes neither
  the production repository asset policy nor installer. The source-distribution
  demonstration must not be presented as an already shipped Vue installer.
* Vue is MIT; the full developer lock includes MIT/ISC/BSD-2-Clause/BSD-3-Clause
  metadata. Initial npm installation reported zero audit findings; that is not an
  ongoing security guarantee. Production adoption needs patch ownership, tested
  lock updates, retained notices and signed/trusted release asset provenance.
  Checksums in the same package detect mismatch, not publisher authenticity.
* `strace` of actual current CLI doctor and MCP initialize/EOF each showed one
  execve and zero socket/connect/listen calls: no additional UI process or network
  service. The CLI starts Viewer only in its explicit Viewer open/export dispatch;
  MCP's stdio startup has no Viewer launch. No production code/assets were added
  to either executable here, so this experiment adds no CLI/MCP resident service.
  Arbitrary analyzer/provider operations were not exhaustively traced.
* Existing HTTP validates loopback Host and mutation authenticity/Origin and uses
  a script-blocking CSP. Prototype GET-only listeners reject wrong Host, escape
  Rust text or use Vue text nodes, expose no mutation operation, and allow scripts
  only from the same host. They are not replacements for that HTTP security layer.
  Source failure is a **display-injection control** over an authorized frozen
  readback; it does not test Source revocation/deletion or permit serving withheld
  bodies. Adopting either enhancement needs a deliberate CSP/asset route review.
* No remote fonts, CDN, telemetry, Node runtime server, Electron or always-on UI
  was introduced. Network observations cover the actual browser tasks. Both retain
  script-free, self-contained offline reading; Vue needs the separate Rust export
  path and its parity tests. Shared server rendering reduces that maintenance
  split for the recommended architecture.

## Feasibility and decision boundary

| Approach | Practical fit | Costs/limits | Conclusion |
| --- | --- | --- | --- |
| Rust HTML + bounded JS | Demonstrated full reading task, native fallback/export, small event state | Manual focus/scroll/state code; current Work navigation reloads | Lowest demonstrated total cost; conditional recommendation |
| Vue component reader | Demonstrated component/state approach and faster local navigation | Developer Node/build chain; JS/data startup; separate Rust offline output | Feasible alternative; no evidence requiring migration |
| React | Official existing-project/createRoot and standalone-build paths fit a Rust HTTP backend | Similar client build/export split; actual bundle, memory and speed unmeasured | Feasible, not ranked against measured Vue |
| Dioxus Web | Rust components, WASM DOM and SSR/hydration preserve Rust application language | Additional WASM/dx toolchain, browser glue and export/hydration validation; costs unmeasured | Applicable Rust-native alternative if state complexity or strict Rust ownership warrants a spike |

Primary feasibility references: [Vue performance](https://vuejs.org/guide/best-practices/performance),
[Vue tooling](https://vuejs.org/guide/scaling-up/tooling),
[React existing-project integration](https://react.dev/learn/add-react-to-an-existing-project),
[React standalone build](https://react.dev/learn/build-a-react-app-from-scratch),
[Dioxus Web](https://dioxuslabs.com/learn/0.7/guides/platforms/web/).
Vendor performance claims are not substituted for measurements here. No additional
framework prototype was needed to resolve the demonstrated reading requirements;
React/Dioxus runtime costs remain explicitly unmeasured.

The charter and `open-decisions.md` Q2 accept **Rust as implementation language**;
section 19 delegates Viewer framework choice within accepted contracts. That does
not independently authorize a TypeScript or JavaScript production frontend. A
Vue/React production recommendation must first identify the bounded language
exception/change in `product-charter.md` and `open-decisions.md` for explicit user
judgment. Even the bounded enhancement recommendation needs an explicit treatment
of browser glue under that Rust decision before shipping. If all authored logic
must remain Rust, retain the script-free native reader or investigate Rust/WASM;
do not use “framework choice delegated” as a language-contract loophole. This
evaluation records no accepted Product decision and requests no migration approval.

## Execution evidence and limits

All commands used the existing focused runner; inspect numeric status, stdout,
stderr and inner results under `rebuild/.local/validation/`:

| Check | Exact artifact directory suffix | Result |
| --- | --- | --- |
| Final production-like build, repeated asset equality, Rust malicious renderer | `20261016T060344.742362Z-viewer-architecture-responsive-fix-ai1tjio2` | 0 |
| Correctly executed syntax/nested-workspace checks | `20261016T060605.378310Z-viewer-architecture-source-checks-executed-2vn5wt10` | 0 |
| Post-support-commit browser, 84 rows / 23 controls | `20261016T060643.027292Z-viewer-architecture-committed-browser-pobs6f5n` | 0 |
| Post-support-commit resource/bytes/trace capture | `20261016T060846.785590Z-viewer-architecture-committed-resources-5cjvw0qj` | 0 |
| Actual clean Debian Product source install | `20261016T060009.171018Z-viewer-architecture-product-source-linux-zyy6f87c` | 0 |
| Clean Ubuntu prebuilt Node-free runtime | `20261016T060125.383761Z-viewer-architecture-node-free-runtime-6x57n5wd` | 0 |
| Report/artifact consistency and complete live baseline prose parity | `20261016T061354.601374Z-viewer-architecture-report-consistency-d31tyu70` | 0 |

Original failed attempts remain preserved: sandbox listener denial; test-driver
blank-line extraction and navigation-label mistakes; relative runtime path;
Debian prebuilt ABI failure; and the genuine hostile-heading overflow, fixed by
shared wrapping CSS. An early stdin-based syntax invocation executed no assertions
because the runner uses DEVNULL; it is excluded and replaced by the file-backed
check above. Final candidate evidence is not replaced by earlier successes.

Skipped intentionally: repository-wide aggregation, legacy validation, Final/V11,
new provider execution, production framework migration and human acceptance.
Remaining uncertainty: user comprehension, accessibility beyond keyboard checks,
other browsers/CPUs, true cold downloads/toolchain installation, peak/steady-state
memory, production integrated framework costs, autonomous Source/content quality,
and any product-language/asset-policy approval. Raw local artifacts are not durable
Product truth and may disappear; missing retained originals must block reproduction
rather than silently change the dataset.
