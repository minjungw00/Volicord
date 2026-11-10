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
and negative expectations. `import_bindings` additionally checks explicit local
Java package/public-static targets and TypeScript named relative exports. Binding
needs its actual lexical import, verified file/declaration and supported module
context; a relative module edge alone grants neither symbol nor alias binding.
JPMS, inherited/altered TypeScript module lookup, alias/default/re-export and
inferred external receivers remain explicitly unsupported. Synthetic obsolete-producer injection is labelled in
the test and must never be presented as captured historical output.

`object_scope.ts` independently executes object-method, object `this`, named
function-expression and named class-expression scope controls. Object receivers
and expression-owned declarations absent from the structural model stay
unresolved; neither may fall back to an enclosing class or same-spelled outer
function. Run it with Node through the same focused runner.

`Outer.java` checks Java receiver ownership independently from the analyzer. Java
Language Specification [15.8.3](https://docs.oracle.com/javase/specs/jls/se25/html/jls-15.html#jls-15.8.3),
[15.9.5](https://docs.oracle.com/javase/specs/jls/se25/html/jls-15.html#jls-15.9.5)
and [15.27.2](https://docs.oracle.com/javase/specs/jls/se25/html/jls-15.html#jls-15.27.2)
establish that an anonymous class introduces its own `this`, whereas a lambda
retains the surrounding `this`. The executable counter oracle distinguishes the
outer, anonymous, deeper anonymous, named nested and static methods, including a
lambda in an anonymous constructor's argument list. Run `java
rebuild/validation/repository-intelligence/semantic-binding/Outer.java` through
`validate focused` on a JDK supporting source-file launch; unavailable Java tools
are a skipped oracle, never a compiled pass.

The bounded analyzer leaves anonymous `this.ping()` unresolved, including inside
an anonymous-owned lambda, while retaining named nested, lexical, static and
outer-lambda bindings. Operations `--lib semantic_receiver_tests` exercises the
actual durable graph, restart, warm cache, Search and grounding on this fixture.
Host `--test semantic_targets` uses smaller coupled controls to require exact
resolved/unresolved Projection, MCP and deterministic-explanation rows without
hiding a counterexample behind a section limit. Repository Intelligence
`--test semantic` and `--test import_bindings` retain the original TypeScript and
supported import controls, and synthetic obsolete-producer tests explicitly
include analyzer `7`. These are focused engineering checks, not qualification.
