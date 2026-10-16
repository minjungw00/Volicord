# Viewer review: card navigation, timestamps and section spacing

Disposable design revision over entry HEAD
`9f33c728ebeb59c314ef40cf37d47a9861ebe76f`, clean on entry. Previous review
packages, inputs, reports and prototype are preserved. Only validation support
changed; no Product contract, dependency, runtime record or legacy path changed.

Open `rebuild/.local/viewer-task-design/card-details/review/works.html`, or run
`python3 serve.py` inside that package. The sibling `index.html` retains the full
baseline/target comparison, screenshots, Source identities and process evidence.

- Work-page guidance now describes content and progress without a fixture count.
- Each card has one interactive element: its reading button. Titles are plain
  text, with no link, tabindex or click handler.
- Cards display creation/completion timestamps in KST. Creation uses the first
  Goal revision's recorded time; a later revision cannot establish creation.
  Completion uses the completed completion Checkpoint's recorded time, explicitly
  scoped in a tooltip. It does not infer an execution end or user acceptance.
  Missing evidence displays as missing. Machine-readable time attributes retain
  the exact original microsecond values and timezone-aware ISO timestamp.
- The reader breadcrumb no longer appends `/ 보존된 Work`.
- Limits are wrapped in the same divided, padded section structure as narrative
  sections, for both admitted reader documents and canonical-record detail.

Focused validation under `rebuild/.local/validation/`:

| Run | Outcome |
| --- | --- |
| `20261016T101745.338414Z-viewer-card-details-controls-fc46a0_z` | Exit 0; 16 controls, including no invented times for later Goal revisions or incomplete Work |
| `20261016T101748.456972Z-viewer-card-details-browser-ap64zbo9` | Exit 0; 1440/768/390/320 px, one card control, exact original timestamps and independently formatted KST, neutral guidance, breadcrumb removal, divider/spacing, content/Source parity and 132 citation round trips |
| `20261016T101836.532203Z-viewer-card-details-package-controls-qif_d9f8` | Exit 0; final package identities and active local links passed |

Wrapper results, complete stdout/stderr and browser inner results were inspected.
Screens show readable time metadata without ordinary-page overflow at all four
widths, and visible separation before limits. Technical and agent visual checks
are not human usability acceptance. Full Rust/workspace/legacy/aggregate/provider
checks were not run for this standalone presentation-only scope; no required tool
was unavailable. Earlier limitations still apply, including unobserved screen
reader/native zoom and canonical-report-only A/C detail. No new generation or
execution is asserted by the historical dates.
