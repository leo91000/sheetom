# Architecture

SheetOM is one authoring CSSOM implemented across a JavaScript facade and a
repository-owned Rust engine. Native and WebAssembly packages are transports,
not alternate behavior engines.

```text
Application
    |
    v
JavaScript CSSOM facade
  WebIDL conversion, live object identity, rule parentage
    |
    v
Private Engine Binding
  strings + validated primitives + owned results
    |
    +---------------------------+
    |                           |
    v                           v
N-API native binding       WebAssembly binding
    |                           |
    +-------------+-------------+
                  v
            Rust CSS engine
  syntax, recovery, declarations, shorthands, serialization
                  |
                  v
  vendored cssparser + vendored Lightning CSS sources
```

## Public facade

`src/index.ts` exposes browser-named CSSOM classes without modifying
`globalThis`. `CSSStyleSheet` is the only publicly constructible CSSOM class;
rules, declarations, lists, and maps are created internally from sheet state.

The facade owns JavaScript-facing concerns:

- WebIDL argument conversion and exception ordering
- stable live object identity and indexed/named property access
- parent rule and stylesheet relationships
- attachment and detachment behavior
- constructed-sheet versus regular-sheet policy
- SheetOM extensions such as `parseStyleSheet()` and `serialize()`

It does not reparse property values or maintain a second declaration engine.

## Rust engine

`crates/sheetom-core` owns the semantic CSS work:

- CSS Syntax tokenization, recovery, and rule consumption
- typed property values and property-specific grammar validation
- ordered declaration state and priority handling
- static shorthand expansion into canonical longhands
- shorthand synthesis from current longhand state
- pending-substitution provenance for `var()`, `env()`, `attr()`, and `if()`
- observable CSSOM serialization and reparsable safe serialization

The engine consumes the complete vendored `rust-cssparser` and Lightning CSS
source snapshots through Cargo path patches. SheetOM can fix those sources when
the pinned browser contract requires it, while keeping general corrections
separable for upstream contribution. npm consumers never install the
JavaScript Lightning CSS binding or another runtime parser.

## Recovered and semantic state

Malformed-but-accepted CSS needs two related representations:

1. recovered component values retain the lexical evidence required for
   browser-facing getters and `cssText`;
2. semantic property values drive validation, expansion, mutation, and safe
   output.

That separation prevents both common failure modes: blindly preserving raw
text can leak syntax into later declarations, while always printing the typed
AST can erase browser-observable recovery spelling.

Static shorthands never remain beside their longhands as competing state.
Expansion is atomic, and the canonical longhand records are authoritative.
Pending substitutions keep group provenance only until a longhand mutation
breaks the group.

## Engine binding

`src/internal/engine-binding.ts` is the private, transport-neutral seam. It
accepts strings, numeric resource limits, and simple enums; it returns strings,
owned JSON descriptions, or declaration-state handles. Arbitrary JavaScript
objects and parser AST nodes do not cross the boundary.

Every implementation reports an Engine ABI Identity containing:

- the binding ABI version;
- the exact SheetOM version;
- a hash of the vendored Syntax Engine Set.

The facade validates that identity before creating public state. Package-manager
overrides, stale binaries, or a partial release therefore fail closed.

## Native distribution

The root `sheetom` package contains no native binary. It declares exact-version
optional dependencies on thirteen public `@sheetom/native-<target>` packages.
npm selects the package matching the current operating system, CPU, and Linux
libc. The loader verifies its ABI identity and never downloads, builds, or
silently substitutes an engine at install time.

The root, WebAssembly, and all native packages form one Changesets fixed cohort
and one immutable release artifact set.

## WebAssembly distribution

`@sheetom/wasm` is an explicit ESM backend. Its asynchronous factory initializes
the same Rust core and returns a facade with the same public surface. Static,
instance-scoped generated factories keep the module graph analyzable by Vite,
esbuild, Rollup, and Webpack while allowing explicitly isolated engine
instances.

The WASM package is not a fallback for a missing native addon. An unexpected
trap poisons only that engine instance, which rejects later operations instead
of exposing potentially partial state.

## Process safety and budgets

All finite public input within the configured Resource Budget must produce a
result, an atomic no-op, or a controlled JavaScript error—never terminate the
host process. Subprocess crash tests enforce that contract because a native
abort cannot be caught by the current Vitest worker.

Budgets are per sheet and checked before mutation for source bytes, declaration
value bytes, syntax depth, rule count, and declarations per block. Parser and
rule-tree traversal use explicit work stacks where deep input would otherwise
risk a native or WebAssembly stack overflow.

See [Compatibility](./compatibility.md) for the observable contract and
[the ADR index](./adr/README.md) for individual decisions.
