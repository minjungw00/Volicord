# Live Viewer display observation

The live renderer supplies escaped JSON in `meta[name="volicord-observation"]`.
Context schema 1 identifies each render, the actual Linux executable bytes and
PID/boot/start instance, opaque Runtime binding, Project, locale, requested
language, parsed view and selected Work/Decision. It retains the canonical read
fingerprint, a hash of bounded Source availability/freshness/snapshot metadata,
Analysis identities/freshness, and materialized subjects' explanation states.
Current explanations additionally identify the prepared fingerprint, recording
time and realization hash. Non-current content remains withheld. These are
read-side diagnostics, outside canonical authority; no host/provider is invoked.
Unavailable process/render binding leaves normal reading usable with a fixed
unavailable meta marker; capture fails instead of inventing a bound display.
The subject list covers the request's bounded materialization, including Overview
inputs, rather than claiming every entry is visible on the screen.

Capture a tab already displayed in an operator-owned local Chromium browser with
its loopback CDP endpoint enabled. Use the existing installed Playwright module;
this interface does not install or replace a browser:

```text
python3 rebuild/validation/end-to-end/multi-repository/capture_viewer_context.py --campaign-root ROOT --cdp-url http://127.0.0.1:9222 --page-url 'http://127.0.0.1:3219/?view=work&work=WORK_ID&locale=en&language=en' --playwright-module /absolute/playwright-core --output NEW_EN_CAPTURE
```

The exact URL must identify one existing tab. The maintained browser driver
attaches, captures its current viewport without navigating or generating,
checks DOM/URL/geometry stability across capture and disconnects. It neither
closes the browser nor supplies a human verdict. `display-context.json` binds the
actual renderer context, DOM hash, screenshot bytes/hash, browser version and
viewport geometry. Complete process streams and numeric exit/timeout records are
retained. Authentication form values and DOM bodies are not retained in JSON.
Screenshots can contain displayed local content: retain them locally under the
same authority as the original Viewer, independently of body-free MCP telemetry.
No browser-wide page contents are harvested.

Campaign capture checks the current clean candidate and frozen Viewer executable
before/after capture, Runtime and Project. For disposable supporting proof only,
explicit `--binary ABS_VIEWER --runtime ABS_RUNTIME --project PROJECT_ID` can
replace campaign selection; it binds actual executable bytes and repository HEAD
without claiming campaign qualification or independently attested build provenance.
Snapshot export retains its existing deterministic offline contract and carries
no live render/process context. Snapshot export timing, constructor executable
hashing, server/render profiles, browser navigation PaintTiming/input and human
experience remain separate measurements.

Repeat capture for Korean and for each screen needed by the observation. Preserve
distinct directories before generation, after generation, after mutation/stale
state and after regeneration; never overwrite or relabel an older receipt.
Changing locale, subject, view, canonical/Source/Analysis or generation basis
changes the observation identity. The closed validator rejects foreign candidate,
Runtime, Project, locale, subject, snapshot mode and non-current content claiming
current realization. Receipt/screenshot hashes detect changed retained artifacts;
they do not authenticate a manually authored observation or prove comprehension.

```text
rebuild/scripts/dogfood-campaign capture-human-viewer-observations --campaign-root ROOT --viewer-context NEW_EN_CAPTURE --viewer-context NEW_KO_CAPTURE --output NEW_HUMAN_CAPTURE
```

Repeat `--viewer-context` for multiple views/phases. Human observation/receipt
schema 5 retains those closed contexts and an explicit `personally_observed`
declaration for each locale before accepting narrative or `SAME AS ENGLISH`.
Both locales require their own actual contexts and personal inspection; a locale
reference reuses declared prose only. Capture rechecks the receipt and screenshot
bytes after the conversation. It does not turn a browser artifact into a human
judgment. Human preparation copies/hashes declared observations and context into
the existing review package. Copied-package/lineage verification checks candidate,
executable, Runtime/Project, locale and exact context bytes without consulting a
running Viewer or original Runtime. Screenshots remain in the original local
capture directories; copied review metadata binds their hashes but does not claim
to re-inspect unavailable screenshot bytes. Retain the original capture directories
when later reviewers need the images.

Actual native zoom is reported only by the existing `chrome.tabs.setZoom` driver,
with automatic per-tab settings and verified geometry. Attachment receipts leave
zoom null when it was not independently measured; DPR/viewport size alone do not
establish 200% zoom. Human claims and limits remain explicit separate declarations.

Focused reproducible supporting proof:

```text
rebuild/scripts/validate focused viewer-display-context -- python3 rebuild/validation/end-to-end/multi-repository/viewer_context_browser.py --chromium /absolute/chromium --playwright-module /absolute/playwright-core --bin-dir /absolute/current-candidate/bin --require-clean
```

Supply `--library-path` and existing fontconfig configuration when needed. It uses
the maintained Rust explanation seed, real public prepare/record/correct commands
and the existing browser driver. Labeled fixture prose establishes structural
state binding only. Both locales' absent/current/stale/regenerated Work states,
Decision absence/current state, real 200% zoom, attach/detach and foreign-executable
rejection retain full local evidence. The test cleans up only its owned Viewer
and browser. It is neither an external authenticated probe nor a naturalistic
campaign, human judgment or authoritative gate.

The person supplies a direct experience block, with optional explicit `LIMITS:`.
No schema fields are requested; raw answer text stays exact and unreported limits
stay `not_reported`. The operator maps supported live criteria in the new review
through `apply-human-observation-assessments`, preserving that answer rather than
asking for evidence/reasoning/dimension/counterevidence fields again. Clarify only
missing, ambiguous or contradictory experience. Separate color-only meaning from
general grouping difficulty. Neither experience proves historical snapshot or
conversation fidelity. See the qualitative-review owner for mapping fields and
append-only recording. Counts of unreviewed rubric fields are not question counts.

For a changed Product candidate, generate `human_observation_plan.py --bin-dir BIN
--output NEW_PLAN` after technical validation, then pass `--observation-plan
NEW_PLAN/preparation.json` to capture. Required en/ko views and two actual Works plus
an applicable displayed Decision are checked before questioning. The exact-candidate
preparation contains only scope and blank observation obligations. Substantial changes
require new keyboard/focus/narrow/native-zoom/input-paint experience too; old successes
remain diagnostic historical observations, not evidence of the new display.
