# Contributing to SheetOM

SheetOM treats compatibility evidence as part of the implementation. A change
is complete only when the public package, native and WebAssembly backends, and
the relevant browser oracle agree with its documented contract.

## Development environment

Install:

- Node.js 24.12 or newer for repository scripts
- npm 11.16 or the version recorded in `packageManager`
- stable Rust 1.88 or newer
- the `wasm32-unknown-unknown` Rust target for WebAssembly work

The published package still supports Node.js 22. The newer contributor baseline
lets repository tooling run TypeScript directly without a separate script
transpilation layer.

```sh
npm ci
npm run native:build
npm run check
```

`npm run check` validates repository shape, generated artifacts, TypeScript,
unit behavior, conformance data, documentation, bundles, runtime boundaries,
and the installed tarball. Rust-specific changes should also run:

```sh
npm run native:check
npm run native:test
```

WebAssembly changes should additionally run:

```sh
npm run wasm:check
npm run wasm:build
npm run wasm:test
```

Browser, fuzz, and performance commands are listed in `package.json`. Run the
narrowest relevant command while iterating, then the complete affected gate
before opening a pull request.

## Repository boundaries

- `src/` owns the public JavaScript CSSOM facade, WebIDL conversion, and live
  object identity.
- `crates/sheetom-core/` owns parsing, declaration semantics, recovery,
  shorthand behavior, and serialization.
- `crates/sheetom-native/` and `crates/sheetom-wasm/` are narrow transports for
  the same core engine.
- `vendor/cssparser/` and `vendor/lightningcss/` are complete pinned source
  snapshots selected through Cargo path patches.
- `compatibility/` contains reviewed contracts, browser observations, WPT
  provenance, resolutions, and immutable release baselines.

Keep the engine boundary narrow: strings and validated primitives enter, owned
domain data leaves, and parser AST objects never cross N-API or WebAssembly.
Do not add browser probes, compatibility corpora, or fallback parsers to the
runtime bundle.

## Vendored parser changes

Do not update vendored sources as an incidental dependency bump. Record the
upstream revision and license provenance, import a new snapshot separately from
SheetOM modifications, and keep behavioral patches focused enough to review or
upstream independently.

General CSS parser corrections belong near the vendored implementation.
SheetOM-specific ordering, browser precedence, diagnostics, and public API
policy belong in SheetOM modules. Every parser change needs a regression test
at the lowest useful layer and a public compatibility witness when observable.

## Compatibility evidence

Use checked-in generators for mechanical corpora. Do not hand-edit generated
browser observations or immutable release baselines. Recording commands are
deliberate maintainer operations; ordinary CI runs their `--check` forms and
must fail closed when source inputs drift.

When a browser differential finds a mismatch:

1. Minimize the operation sequence.
2. Determine whether the behavior is shared, specified, or an engine divergence.
3. Add or update a stable Operation Fixture or grammar contract.
4. Record the browser observations explicitly.
5. Add a Compatibility Resolution when engines disagree.
6. Verify browser-facing text, semantic state, and reparsable output.

Rendering equivalence alone is not sufficient for a CSSOM compatibility claim.

## Pull requests and releases

Add a Changeset for every consumer-visible API, behavior, compatibility,
runtime-support, or dependency change:

```sh
npm run changeset
```

Keep generated release pull requests unchanged while their full matrix runs.
Changesets prepares versions and changelogs; it does not independently publish
packages. See [the release procedure](./docs/releasing.md) for the atomic
fifteen-package artifact flow and channel reconciliation checkpoints.

Security vulnerabilities should not be filed publicly. Follow
[SECURITY.md](./SECURITY.md).
