# Candidate MCP resource observation

The maintained foreground interface is `resource_observer.py start|attach|stop`.
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

Resource schema 2 has a closed key allowlist and closed lifecycle identity, tick,
sample, privacy and measurement objects. RSS is `/proc/PID/status` VmRSS in kernel
kB, converted at 1024 bytes per unit; timing is monotonic nanoseconds from attachment.
This kernel counter is an approximate observation. `peak_rss_bytes` is the greatest
observed individual-instance sample, not an absolute maximum, concurrent sum or
operation peak. A measured state requires samples and no recorded gaps/errors;
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
