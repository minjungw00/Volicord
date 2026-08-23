# Background Provider Qualification

This maintained qualification exercises the production `openai-codex`
background semantic provider with one bounded, self-authored, non-sensitive
Rust Source. It is separate from V11 and never transmits repository or private
project source.

The harness does not grant authority. A live caller must supply the exact
source-transmission assertion
`openai-codex-background-semantic-bounded-rust-v1` at the maintained
entrypoint:

```bash
rebuild/scripts/validate focused background-provider-live -- \
  python3 rebuild/validation/privacy/background-provider-qualification/harness.py \
  --live \
  --authorize-source-transmission openai-codex-background-semantic-bounded-rust-v1 \
  --model gpt-5.6-sol \
  --evidence-output rebuild/validation/privacy/background-provider-qualification/evaluation.json
```

The assertion authorizes only transmission of
`fixtures/bounded-rust/src/lib.rs` to the authenticated OpenAI Codex service,
using provider identity `openai-codex`, the explicitly named model, purpose
`qualify the bounded background semantic provider fixture`, and requested
capability `semantic_annotation`. It does not authorize V11 source, other
repository source, additional destinations, or later invocations.

Before live use, run the network-free admission check:

```bash
rebuild/scripts/validate focused background-provider-self-test -- \
  python3 rebuild/validation/privacy/background-provider-qualification/harness.py --self-test
```

The live Rust test constructs a temporary Project, records explicit Project
opt-in, prepares and exactly confirms the Guarded effect, then dispatches
through `dispatch_guarded_provider_with_configured_adapter`. It verifies one
transmission manifest entry and a grounded semantic annotation with provider,
model, Analysis Snapshot, and Source provenance. It then selects a missing
executable to verify `provider_unavailable`, no transmission, consumed Guarded
confirmation, and continued local canonical work.

`evaluation.json` is a sanitized projection. It must never contain fixture
source bodies, provider response bodies, Codex event streams, credentials, or
raw operation artifacts. The production adapter removes its ephemeral raw
artifacts. Provider-side deletion remains unsupported by this adapter and is
reported as such rather than inferred from local cleanup.
