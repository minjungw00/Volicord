# Explicit recorded-code review evidence

`prepare-qualitative-review --code-source WORK_SLOT=REPOSITORY` optionally adds
code bodies to the existing private review index. Repeat the option for distinct
Works. Nothing reads a repository when this option is absent. For example:

```text
rebuild/scripts/dogfood-campaign prepare-qualitative-review \
  --campaign-root /absolute/collected-campaign \
  --output /absolute/new-review --reviewer-kind agent \
  --review-session-id independent-review-session \
  --code-source journey-volicord-work-a=/absolute/native-repository
```

The selector uses only the Work's already recorded single-parent commit boundaries
and complete changed-path lists in its immutable journey Git observations. It
reads regular UTF-8 files from the exact parent and resulting Git objects; a newly
added or removed file retains explicit absence. Symlinks, gitlinks, binary text,
unrecorded boundaries and foreign Work identities are refused. This optional
committed-source reader does not require other Works to commit, narrow Repository
Intelligence languages, or replace the existing dirty-state/capture evidence.

Bodies have normal indexed line/JSON locators and Work-scoped authority aliases
under the existing `repository_state` surface. The review preparation/package and
rubric formats do not change. The separate `dogfood_recorded_git_code` witness has
exact-current schema 1: original commit bytes and a minimal ancestor-tree closure
prove each retained blob's path and Git identity. Candidate, evidence-set hash,
Work, revision, parents and paths are checked against indexed native observations.
Qualification and copied lineage also bind those native artifacts to the original
evidence-set inventory. Copied verification reads no original repository or Git.

Each body is at most 4 MiB, each witness 4 MiB and all selected bodies total at most
32 MiB, within the existing review package bounds. The source-read limit is applied
before buffering Git output. Sensitive-payload rejection still applies to code.
At most sixteen revision views and sixty-four changed paths per view are admitted
for each selected Work; omitted revision views, extra trees and orphan bodies fail.

These are explicit post-session source observations, not earlier returned model
context, recorded execution or prospective user authority. They cannot repair a
truncated original response, prove a test ran, supply user reasons, change Learning
applicability or establish Human understanding. The reviewer must still inspect
each applicable criterion, retain capture/authority guards, and independently
explain a judgment. Existing preparations, failures and recorded reviews remain
unchanged; stronger evidence requires a new prepared run.
