# Candidate MCP resource observation

The maintained foreground interface is `resource_observer.py start|attach|expect|stop`.
Start and attach use the same discovery path and can observe an already running
candidate. Neither starts a host, changes direct executable integration, grants
Codex trust nor signals an MCP. For a prepared clean-candidate campaign:

```text
python3 rebuild/validation/dogfood/resource_observer.py start --campaign-root ROOT --output NEW_OBSERVER_DIRECTORY --duration-seconds 3600 --interval-ms 1000
python3 rebuild/validation/dogfood/resource_observer.py stop --output NEW_OBSERVER_DIRECTORY
rebuild/scripts/dogfood-campaign record-resources --campaign-root ROOT --input NEW_OBSERVER_DIRECTORY/resource.json
```

Run start in an operator-owned terminal before the chats. Stop in another terminal,
or use Ctrl-C. Stop writes a cooperative stop request, bounded by the next sampling
interval; it never kills an external host process. Duration expiry also stops.
`attach --binary ABSOLUTE_MCP --runtime ABSOLUTE_RUNTIME --output NEW_DIRECTORY`
provides standalone local proof without campaign qualification. Campaign attachment
requires its frozen candidate bytes and Runtime/repository hashes, then copies
one create-only `resources/observation.json` before collection. Missing observation
remains `not_observed`; it cannot become zero memory. Historical campaign/resource
schemas are retained as history and never interpreted through a second decoder.

On Linux the production stdio executable publishes a private bounded JSON entry
under `Runtime/observations/mcp/INSTANCE.json`, outside all canonical stores. It
contains a random instance identity, boot ID, PID, process start ticks, executable
path/hash, Runtime/cwd hashes, and the actual server-generated MCP session correlation.
The session is correlation only; Codex session and operation attribution stay unknown.
Registration does not grant source, Decision, background-provider or host-trust authority.
EOF publishes stopped/lifetime; a restart has a new random identity and start binding.
SIGKILL can leave running registration; /proc disappearance records the observed gap.
Registration failure prints a fixed stderr diagnostic and leaves stdio usable.
No registration content is printed on MCP stdout. The private registry is bounded
at 1024 entries. The operator may delete stopped entries after retaining required
observations; hitting the bound explicitly loses telemetry, never canonical work.

The observer selects only these exact Runtime entries, without a host-wide PID scan.
It compares boot/start before and after the RSS read and hashes `/proc/PID/exe`,
checking path plus dev/inode/size/mtime/ctime before reusing that executable hash.
PID reuse, executable changes, foreign Runtime/cwd, malformed registration and
changed instance identity cannot supply valid samples. A terminated process at
attachment is an unsampled lifecycle, not a measured zero. Concurrent/restarted
instances retain independent samples and lifecycle results. Missing/inaccessible
processes, scheduling gaps (> twice the requested interval), observer failure,
zero samples and sample/instance bounds retain finite error codes and partial/failed
or unobserved states. No error message copies arbitrary input or an environment.

Resource schema 3 has a closed key allowlist and closed lifecycle identity, tick,
sample, privacy and measurement objects. RSS is `/proc/PID/status` VmRSS in kernel
kB, converted at 1024 bytes per unit; timing is monotonic nanoseconds from attachment.
This kernel counter is an approximate observation. `peak_rss_bytes` is the greatest
observed individual-instance sample, not an absolute maximum, concurrent sum or
operation peak. A measured state requires samples and no recomputed coverage limitations or recorded gaps/errors;
partial and failed states preserve valid earlier samples. Neither status attests
continuous whole-session coverage. Tick/sample timestamps expose attachment delay
and actual coverage. A stopped pre-attachment instance remains unsampled.

Maximum duration is 24 hours, interval 50–60000 ms, 20000 ticks and 20000 total RSS
samples across at most 1024 instances. Observer CPU duration covers executable
verification and the observation loop; observer peak RSS uses its own Linux
getrusage high-water mark, distinct from candidate samples. Candidate registration
reports elapsed hashing/setup cost before publication; the real probe separately
measures full startup-to-initialize and EOF/observer teardown. Existing resource
ceilings are unchanged. Technical V11 harness-tree RSS retains separate ownership.

The observer never reads argv, full environments, RPC arguments, conversation or
source bodies, provider responses or credentials. Runtime/cwd paths are retained
only as opaque hashes; executable path is an explicit candidate binding. All telemetry
is local operational evidence and may be deleted without affecting canonical memory.
The campaign inventory, review index and source-independent lineage validator bind
its exact bytes and recompute counts/observed peaks/completeness. Hash integrity and
closed schemas do not authenticate an operator or attest the truth of manually
supplied measurements. Real sibling-process proof is `resource_observer_self_test.py
--binary ABSOLUTE_MCP`; simulated failures in that test are negative support only.
EOF/protocol/privacy behavior also runs through Rust `volicord-host/mcp_lifecycle`.

The real proof creates a fresh ignored `rebuild/.local/validation/mcp-observer-proof-*`
directory by default (`--output NEW_DIRECTORY` selects an explicit create-only
local destination). It retains actual registration/resource files, full subprocess
stdout/stderr, numeric exits/signals, durations, EOF-to-exit and cooperative observer
stop-to-exit timing. All test-owned observers are reaped on failure; external MCP
shutdown remains the host's responsibility. These are focused synthetic protocol
fixtures, not a Naturalistic campaign or authenticated host activation.

## Runtime expectations and coverage

Each configured Runtime starts `unknown`. Before starting the observer, the operator
may independently declare not-yet-launched Homes with repeated `--waiting-runtime
ABSOLUTE_RUNTIME`. This means armed/waiting, not proof that any absent process is idle.
Retain the declaration in the observer's per-Runtime initial expectation. Immediately
before a launch, declare its expected activity through the maintained operator surface:

```text
python3 rebuild/validation/dogfood/resource_observer.py expect --output OBSERVER_DIRECTORY --runtime ABSOLUTE_RUNTIME --state active
```

After the host confirms work completion and actual MCP EOF shutdown, use the same
command with `--state waiting`. An unknown launch/completion must use `--state unknown`.
These operator assertions never authenticate host activity or stop a process. Do not
put observer operations or evaluator instructions into frozen task text. One observer
can arm all known inactive Homes before sequential chats; actual registration/sample
facts independently record activity even while a Home was declared waiting.

Each tick retains every configured Runtime's expectation/authority, registered instance
states and sampled identities. Running requires positive Product registration and
verified RSS. Stopped requires Product `state=stopped` with lifetime; registry absence,
SIGKILL or /proc disappearance remain uncertain (`gone`). Rejected identity and
inaccessible processes retain their errors; an exited pre-attachment instance is
unsampled. Restart requires a distinct instance/start identity. No registration under
`unknown` means unknown coverage; absence under `active` means missing expected activity.
A running registration that cannot be sampled remains incomplete. Confirmed stopped
instances with earlier samples preserve those samples; stopped-only attachment is not
measurement. Explicit active expectation remains an incomplete open window at observer
detachment, even with valid samples. Operator stop and interruption never close active
work; interruption retains its own fact and makes otherwise sampled evidence partial.

The validator recomputes status, peaks, counts, scheduling gaps, sample/tick agreement,
unknown/expected coverage and lifecycle limitations from these facts. No-sample campaigns
remain `not_observed` (or blocked/failed), including all-waiting Homes. Current schema
3 is the sole decoder; historical schema-2 files remain immutable diagnostic evidence.
No continuous coverage, absolute peak, Codex-session or operation attribution follows.
Runtime/cwd remain hashes; declarations retain no argv, environment, source or secrets.

At each tick and Runtime, the sampled identity set must equal the registered
`running` identity set. Each reference binds exactly one sample from that instance
within the tick's half-open interval (the final interval includes detachment).
Removing both a sample and its reference while retaining `running` is invalid,
even after counts, peaks, status or wrapper hashes are recalculated. Another
instance's sample and global/tick/lifetime errors cannot supply that coverage.
Schema 3 already records scoped failure through the same tick's registered
`inaccessible`, `gone` or `identity_rejected` fact; these recompute limitations,
never measured success. Later stop/failure facts cannot excuse an earlier missing
running sample. No schema transition or historical evidence rewrite is needed.
`resource_coverage_self_test.py` independently tests these primitive relationships
and their campaign/qualification consumers; the real observer proof invokes it.
