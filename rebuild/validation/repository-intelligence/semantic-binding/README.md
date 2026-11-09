# Semantic binding control

Self-authored deterministic fixture, CC0-1.0. It is test input, not historical
Source evidence or user acceptance.

`receiver.ts` has separate top-level `log(value: string)` and `demo()` declarations.
The one-argument `console.log("host")` in `demo` selects the host console property;
it never binds the top-level `log`. `local()` selects that lexical declaration.
The counter checks those expectations by execution, independently of Volicord.

Run `rebuild/scripts/validate focused semantic-binding-language-oracle -- node
rebuild/validation/repository-intelligence/semantic-binding/receiver.ts` with a
Node version that supports TypeScript type stripping. The Rust `semantic` test
target runs the real production analyzer and independent per-language positive
and negative expectations. Imported symbols and inferred external receivers
remain explicitly unsupported; a resolved relative module edge grants neither
symbol nor alias binding. Synthetic obsolete-producer injection is labelled in
the test and must never be presented as captured historical output.
