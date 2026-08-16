# sheetom

## 0.1.0

### Minor Changes

- f80e926: Introduce the first release candidate of SheetOM with mutable authoring CSSOM
  rules, browser-compatible declaration recovery, safe stylesheet serialization,
  and versioned browser/WPT compatibility evidence.
- 95af764: Parse, mutate, and serialize Backgrounds Level 4 `border-area` and compound text clipping through typed vendored grammar and Chromium-compatible CSSOM state.
- d5f039d: Accept and canonicalize complete `border-image-slice` fill values without letting scalar numeric fallbacks shadow typed compound grammars.
- 3a4924e: Expand and synthesize the complete Chromium `columns` shorthand, including typed heights, longhand mutations, aliases, and atomic invalid replacement.
- 870d4d7: Accept and canonicalize combined size and scroll-state container types.
- 17e8eed: Own compound one-axis and two-axis branches of the `contain-intrinsic-size` shorthand.
- 4ff9d61: Support anchored self alignment and bare legacy item justification throughout longhands and place shorthands.
- 9144901: Support complete font-palette keywords and recursive palette mixing.
- b44105e: Accept bare positive and negative infinite `superellipse()` exponents across every corner-shape longhand and shorthand.
- bdd86ff: Own Chromium's complete `initial-letter` size and sink grammar in the native CSSOM engine.
- b3a6648: Parse intrinsic flex basis values and typed `calc-size()` calculations while preserving Chromium shorthand expansion, CSSOM serialization, and atomic invalid replacement.
- 41f7e84: Accept and canonicalize Chromium's complete math layout value branches.
- 4794cdc: Own unitless-number, percentage, cross-dimension, list, and legacy perspective grammars in Rust while rejecting nonzero unitless values for ordinary CSS lengths.
- 2a49ccb: Support Chromium overscroll chaining in physical, logical, and shorthand forms.
- 8a37cba: Own the complete Chromium rule-inset shorthand and longhand grammar, including slash-separated cap and junction values, negative length-percentages, overlap joins, canonical CSSOM synthesis, and atomic invalid replacement.
- aa34395: Support complete rule-break and rule-visibility-items partition grammars.
- 7b8d662: Accept and canonicalize complete `calc-size()` values across preferred, minimum, maximum, logical, and legacy sizing properties.
- e1339e2: Accept every Chromium `text-box` component order and omission branch.
- 0476ac2: Accept and canonicalize Chromium text-decoration error-line branches.
- 5ff2390: Accept the complete Chromium `text-fit` grammar, including line strategies, limits, and deferred percentage math.
- a514bc8: Support mixed and repeated `none` entries throughout timeline name lists and shorthands.
- 35cc43c: Own the complete Chromium `transform-origin` grammar, including typed depth lengths and calculations, WebKit alias state, canonical axis ordering, explicit zero units, atomic invalid replacement, and safe round-trip serialization.
- ad99232: Own the complete Chromium `-webkit-mask-box-image` shorthand and longhand grammar, including one-to-four component lists, optional slash sections, escaped identifiers, `image-set()` crash safety, canonical CSSOM state, and atomic invalid replacement.
- 948ad94: Own omitted and unordered CSS Text level 4 components in the `white-space` shorthand.
- 6de29fa: Derive canonical and browser-observable declaration serialization from one owned semantic value and recovered component-value tree, removing the parallel raw-text recovery scanner.
- 11cab4c: Remove the legacy JavaScript syntax engine and all JavaScript runtime dependencies. SheetOM now uses its repository-owned Rust engine exclusively, with no behavioral fallback.
- 4de83b0: Parse static generated content and legacy box reflection values through owned typed browser extensions, and remove the raw-string grammar fallback.
- 7a35e33: Type context-dependent CSS Math throughout composite properties and shorthands, including animation lists, flex components, grid lines, aspect ratios, columns, and border-image families.
- 209562a: Parse and serialize anchor-positioned inset values, including recursive fallbacks, CSS math, shorthand expansion, and atomic invalid replacement.

### Patch Changes

- 05c93a1: Add a deterministic grammar-oriented declaration differential across Chromium, Firefox, and WebKit.
- dcfeb83: Gate performance against a publisher-shaped multi-stylesheet workload with distributed mutations and repeated serialization.
- d970977: Add the explicit, ESM-only `@sheetom/wasm` backend with the same private Engine Binding, parser, resource limits, and browser-shaped facade as the native package.
- 1b3dbd7: Match Chromium observable values, declaration serialization, indexed names, and mutation atomicity across the complete property-value matrix, including legacy aliases and composite shorthand branches.
- c3163c5: Align the published TypeScript interface with CSSOM construction rules and expose pinned Chromium named properties directly on `CSSStyleDeclaration`.
- 9d92e23: Add ordered declaration mutation batches, bounded shared parse reuse, and lower-peak whole-sheet serialization without changing CSSOM validation semantics.
- f21c896: Reject declaration-boundary escapes atomically, preserve logical custom-property
  names and legacy media syntax, canonicalize Chromium property aliases, and gate
  release evidence on every Operation Fixture adapter.
- b8b7787: Canonicalize every Chromium grid placement shorthand and longhand branch, including unordered span syntax.
- 6b1c860: Reject standards branches and legacy alias values that the pinned Chromium
  baseline does not accept, including invalid Grid auto-repeat tracks, without
  weakening CSS-wide keywords or pending substitutions.
- bdc8634: Match Chromium indexed declaration state for marker, mask position, table spacing, animation delay, legacy background size, and cursor hotspot branches.
- ab8be29: Match Chromium acceptance and observable declaration state across the complete 711-property value matrix, including typed property domains, modern timeline, anchor positioning and flex wrapping branches, and atomic shorthand expansion.
- 295bae4: Make every pinned Chromium property branch serialize idempotently while preserving shorthand observability and atomic invalid-value rejection.
- 7e1e99c: Expand shorthand compatibility across reviewed grammar branches, including
  logical multi-values, compound placement, transition and timeline lists,
  text-box edges, and browser-facing animation, overscroll, and background
  synthesis. Ship exact Chromium evidence for 92 cases across all 23 codec
  profiles and block releases when that evidence is incomplete.
- 5f3a547: Close the Chromium/Webref acceptance gap with complete hyphenation-limit and math-font-size grammars.
- bd9aef2: Match Chromium getters and declaration text across every pinned Webref property-value branch.
- 32506b1: Match Chromium's `base-select`, `color-scheme`, animation, transition-time, and
  `attr()` grammar boundaries without weakening atomic invalid-value handling.
- b1b5b0c: Hydrate font-feature, property, and generic rules entirely from Rust-owned rule
  DTOs, and align their public map and descriptor behavior with Chromium.
- 532fbfe: Complete the Chromium 151 shorthand codec registry with 129 concrete capability fixtures, safe round-trips, and browser-backed rendering witnesses for high-risk families.
- 1449410: Enforce configurable per-sheet resource budgets before native parsing and CSSOM mutation.
- 06c7c41: Expand every standard border-image grammar branch through the vendored typed AST while preserving Chromium observable values.
- 8d44286: Publish RC6 after its complete immutable-SHA validation while retaining the seven-night soak as a mandatory gate for the first stable release.
- 2757c81: Match Chromium's observable expanded mask image and multi-layer position longhands without changing shorthand synthesis.
- 48c0edf: Preserve mask mode and composite values when expanding standard and prefixed mask shorthands.
- 9c8e123: Expose Chromium-compatible interfaces for layer statements, namespaces, font palette values, and view transitions.
- 367a336: Synchronize native release metadata and require seven nightly full-matrix validations on the unchanged RC6 release SHA.
- ef09cb9: Normalize non-finite CSS Math and deferred relative-color functions like
  Chromium, make bundled media-query ordering deterministic, and gate the full
  vendored parser test suites.
- cc6a43c: Hydrate and serialize deeply nested rule trees iteratively so inputs at the configured syntax-depth boundary remain usable without JavaScript stack overflows.
- 5f7ac68: Accept measured modern value families and classify deferred substitutions from original tokens while rejecting neighboring Chromium-incompatible values atomically.
- 16024a7: Route ordinary CSS declaration blocks through the process-safe Rust engine, including complete shorthand branch validation and browser-observable state.
- dda22c1: Validate and serialize `@font-face` declaration blocks in the Rust engine with Chromium-compatible descriptor grammar.
- 95cafc8: Move exact top-level CSS rule scanning into the process-safe Rust engine while preserving recovered source bytes.
- 442f6b4: Add a process-safe Rust rule parser that returns owned, parser-independent rule trees across Node-API.
- f7b19a1: Validate and canonicalize selectors, media lists, supports conditions, container
  queries, and scope boundaries through the native Rust CSS engine.
- 594d302: Expand and synthesize animation ranges through canonical parallel longhand lists with Chromium-compatible CSSOM observability.
- ad7ea09: Match Chromium background getters and longhands across layered boxes, two-value sizes, image sets, and CSS math.
- 6a2a495: Match Chromium border shorthand ordering, omitted logical longhands, and safe serialization without suffix-based property inference.
- 2fc83f4: Accept Chromium keyword longhands through a typed Rust registry and preserve legacy alias observability across substitutions and removal.
- e3d3853: Match current Chromium keyword, ordered-set, and legacy break alias grammars without weakening invalid-neighbor atomicity.
- 4e27640: Parse, expand, mutate and synthesize the complete Chromium `font-variant`
  grammar with browser-compatible longhand and component ordering.
- 2775c00: Parse, expand, mutate and synthesize the complete Chromium CSS Gaps rule-list
  grammar, including integer and auto repeats, across row, column and combined
  rule shorthands.
- 279fff9: Own Chromium geometric property grammars, SVG path data, shape commands, image observability, and process-safe large-value handling in the native engine.
- 613b372: Parse, expand, mutate and synthesize every generated Chromium `grid` and
  `grid-template` grammar branch with browser-compatible CSSOM serialization.
- efdf98c: Match Chromium modern clip-path shapes and cursor URL-set grammars with atomic invalid-neighbor rejection and crash-safe serialization.
- d8cdeca: Own Chromium name, scope, counter, quote, paint-order, visibility, and will-change grammars as typed Rust state.
- 8a3612c: Parse, expand and synthesize complete offset paths, rays, coordinate boxes and
  motion shorthand values with Chromium-compatible CSSOM serialization.
- 3f9c9cd: Parse and synthesize complete position-try fallback lists, physical tactics and
  unordered named fallback components with Chromium-compatible CSSOM output.
- 6caae5a: Canonicalize border radii and preserve browser-compatible outline and list-style component provenance.
- 408ce2f: Match Chromium scalar zero aliases, optional-pair compression, font-style defaults, and text-shadow ordering.
- 8db8227: Retain owned semantic declaration values as CSSOM state and derive observable
  and reparsable projections from that authority.
- 75608ba: Retain every accepted font-face descriptor as owned semantic state, preserve CSSOM math-function structure, and differentially gate descriptor grammar branches against Chromium.
- 41b998a: Store expanded shorthand longhands as owned semantic values, add typed Chromium longhand grammars, and gate the supported branches against Chromium without retaining textual Codec authority in style declarations.
- 80cfefb: Expand static shorthands into canonical longhand state so later longhand mutations cannot reactivate historical shorthand values.
- a255d80: Own `clip`, `dynamic-range-limit`, `animation-trigger`, and `position-area` as typed Chromium-compatible grammars, and reserve containing-rule depth so every accepted declaration remains safely serializable.
- d47bb33: Preserve explicit text-emphasis defaults, canonical component order, and omitted-longhand CSSOM state across standard and WebKit aliases.
- d5ca92b: Parse, expand and synthesize complete Chromium timeline-trigger lists, typed
  scroll and view sources, ranges and safe stylesheet serialization.
- 389ebab: Match Chromium transition shorthand observability while retaining unambiguous, idempotent safe stylesheet output.
- ed1fca9: Parse and canonicalize `@counter-style` names and descriptors through the
  panic-contained Rust engine, including browser-matched mutation semantics and
  grammar branches.
- eef53e9: Parse every pinned Chromium relative-color branch through the owned semantic AST, match CSSOM observable serialization, and gate all 1,306 accepted and rejected WPT cases without adding proof data to the runtime bundle.
- 0c9ebc7: Parse and recover public stylesheet rule trees through the panic-contained Rust
  engine while preserving malformed declarations and legacy conditional preludes.
- 8f4734b: Pin the complete syntax engine dependency set and make release-channel reconciliation explicit before immutable GitHub publication.
- a2477b4: Replace prerelease implementation notes with concise product, architecture,
  compatibility, contribution, and release documentation for the stable package.
- d0b57a8: Preserve legacy and functional container-query syntax while exposing browser-compatible structured container getters.
- f9969f6: Teach the vendored CSS value engine to retain authored math-function structure for CSSOM descriptor serialization without changing its default optimization behavior.
- 9791b89: Preserve authored pending shorthands after longhand mutations, add resilient best-effort serialization diagnostics for states CSS text cannot represent exactly, and expose `serializeStrict()` for exact-only callers.
- 61a0c21: Preserve independently authored longhands during reparsable serialization when shorthand synthesis would couple deferred-substitution invalidation.
- 11231cd: Preserve compact position and size separators in background and mask CSSOM getters by projecting their typed layered state.
- 254fd6d: Promote the exact RC11 behavior to the first stable release after the complete
  release validation, without a calendar-duration soak.
- e92635e: Publish the root, WebAssembly, and thirteen native implementation packages from one verified lockstep artifact set.
- 387b702: Publish the exact multi-platform tarball that passed the complete consumer matrix instead of rebuilding a host-only package during the release workflow.
- 024c29c: Record the exact repository-owned native engine, grammar execution, and process-safety execution in RC6 compatibility reports.
- 88ba8bc: Record a Chromium-wide property/value conformance matrix and atomic rejection evidence for the RC6 grammar closure.
- 8aee179: Record and verify WebAssembly browser, bundler, memory, and performance evidence in every RC7-or-later Compatibility Report.
- 4086e3e: Record the complete zero-divergence Webref property-branch execution in the immutable RC6 Compatibility Report.
- 1660d90: Match Chromium's category-aware observable serialization for EOF-recovered typed values, pending substitutions, and custom properties.
- 7a02480: Reject invalid keyword ranges in the `@font-face` `font-stretch` descriptor while retaining valid percentage ranges.
- 160465a: Reject incompatible percentage calculations without invoking a recoverable native panic or writing to stderr.
- 2ec2350: Remove the cssstyle runtime dependency, own observable-value serialization,
  and require every static shorthand to use an explicit browser-backed codec.
- 024c29c: Accept, canonicalize, and safely reparse Chromium's numeric math-function branches for `z-index`, including constants and neighboring invalid values.
- 2cb6a04: Reject incompatible CSS engine binaries through a generated, fail-closed ABI identity before creating CSSOM state.
- 491b361: Retain concrete per-engine operation-fixture observations and add shorthand rendering witnesses to the Chromium differential suite.
- 5f72f3f: Keep immutable compatibility reports as verified GitHub Release assets without
  including the evidence corpus in the runtime npm package.
- 195c3a8: Ship the Rust declaration engine as fail-fast prebuilt Node-API bindings for
  glibc and musl Linux, macOS, and Windows on x64 and arm64.
- dde8730: Remove conformance evidence from the npm tarball, drop obsolete JavaScript parser dependencies, and gate the root package below 200 kB compressed and 750 kB unpacked.
- 43a5fda: Move every native addon into an exact, platform-specific `@sheetom/native-*` optional package so the root package contains only the portable JavaScript facade and loader.
- f0e70f7: Replace the native-only large-stack parser thread with stack-safe shared rule, custom-function, value and JSON traversal for native and WebAssembly engines.
- 8154940: Expose custom `@function` rules, typed parameters, return types, conditional
  declaration runs, and live function descriptors through a Rust-owned parser
  and Chromium-differential CSSOM interface. Preserve internal token-stream
  comments and CSS-significant Unicode whitespace, and keep recovered group-rule
  insertion atomic when trailing input is invalid.
- 367a336: Keep the embedded Rust crates and Cargo lockfile on the exact SheetOM version prepared by Changesets.
- 9aa310e: Accept 105 Chromium composite-longhand grammar branches with typed state, canonical serialization, and atomic invalid-neighbor handling.
- 50f5368: Preserve context-dependent CSS Math number results in typed values and verify 840 property-level Chromium observations, including dynamic products and quotients.
- cd34900: Validate CSS Math result dimensions in the vendored semantic AST, preserve non-finite constants, and gate 94 accepted and rejected math branches against Chromium.
- be3a8c5: Expose diagnostic codes as a stable string union and document the opt-in queue's complete-input memory contract.
- 225e84d: Parse `contrast-color()` and computed-context `color-mix()` values through the vendored typed color AST across every Chromium color property.
- 05c93a1: Verify reparsable output and malformed custom-property substitution equivalence in native browser engines.
- d1de3bf: Version the root, WebAssembly, and native implementation packages as one lockstep Changesets cohort.

## 0.1.0-rc.11

### Patch Changes

- 9791b89: Preserve authored pending shorthands after longhand mutations, add resilient best-effort serialization diagnostics for states CSS text cannot represent exactly, and expose `serializeStrict()` for exact-only callers.

## 0.1.0-rc.10

### Patch Changes

- 9d92e23: Add ordered declaration mutation batches, bounded shared parse reuse, and lower-peak whole-sheet serialization without changing CSSOM validation semantics.

## 0.1.0-rc.9

### Patch Changes

- 61a0c21: Preserve independently authored longhands during reparsable serialization when shorthand synthesis would couple deferred-substitution invalidation.

## 0.1.0-rc.8

### Patch Changes

- c3163c5: Align the published TypeScript interface with CSSOM construction rules and expose pinned Chromium named properties directly on `CSSStyleDeclaration`.
- a2477b4: Replace prerelease implementation notes with concise product, architecture,
  compatibility, contribution, and release documentation for the stable package.
- 5f72f3f: Keep immutable compatibility reports as verified GitHub Release assets without
  including the evidence corpus in the runtime npm package.
- dde8730: Remove conformance evidence from the npm tarball, drop obsolete JavaScript parser dependencies, and gate the root package below 200 kB compressed and 750 kB unpacked.

## 0.1.0-rc.7

### Patch Changes

- d970977: Add the explicit, ESM-only `@sheetom/wasm` backend with the same private Engine Binding, parser, resource limits, and browser-shaped facade as the native package.
- 11231cd: Preserve compact position and size separators in background and mask CSSOM getters by projecting their typed layered state.
- e92635e: Publish the root, WebAssembly, and thirteen native implementation packages from one verified lockstep artifact set.
- 8aee179: Record and verify WebAssembly browser, bundler, memory, and performance evidence in every RC7-or-later Compatibility Report.
- 2cb6a04: Reject incompatible CSS engine binaries through a generated, fail-closed ABI identity before creating CSSOM state.
- 43a5fda: Move every native addon into an exact, platform-specific `@sheetom/native-*` optional package so the root package contains only the portable JavaScript facade and loader.
- f0e70f7: Replace the native-only large-stack parser thread with stack-safe shared rule, custom-function, value and JSON traversal for native and WebAssembly engines.
- d1de3bf: Version the root, WebAssembly, and native implementation packages as one lockstep Changesets cohort.

## 0.1.0-rc.6

### Minor Changes

- 95af764: Parse, mutate, and serialize Backgrounds Level 4 `border-area` and compound text clipping through typed vendored grammar and Chromium-compatible CSSOM state.
- d5f039d: Accept and canonicalize complete `border-image-slice` fill values without letting scalar numeric fallbacks shadow typed compound grammars.
- 3a4924e: Expand and synthesize the complete Chromium `columns` shorthand, including typed heights, longhand mutations, aliases, and atomic invalid replacement.
- 870d4d7: Accept and canonicalize combined size and scroll-state container types.
- 17e8eed: Own compound one-axis and two-axis branches of the `contain-intrinsic-size` shorthand.
- 4ff9d61: Support anchored self alignment and bare legacy item justification throughout longhands and place shorthands.
- 9144901: Support complete font-palette keywords and recursive palette mixing.
- b44105e: Accept bare positive and negative infinite `superellipse()` exponents across every corner-shape longhand and shorthand.
- bdd86ff: Own Chromium's complete `initial-letter` size and sink grammar in the native CSSOM engine.
- b3a6648: Parse intrinsic flex basis values and typed `calc-size()` calculations while preserving Chromium shorthand expansion, CSSOM serialization, and atomic invalid replacement.
- 41f7e84: Accept and canonicalize Chromium's complete math layout value branches.
- 4794cdc: Own unitless-number, percentage, cross-dimension, list, and legacy perspective grammars in Rust while rejecting nonzero unitless values for ordinary CSS lengths.
- 2a49ccb: Support Chromium overscroll chaining in physical, logical, and shorthand forms.
- 8a37cba: Own the complete Chromium rule-inset shorthand and longhand grammar, including slash-separated cap and junction values, negative length-percentages, overlap joins, canonical CSSOM synthesis, and atomic invalid replacement.
- aa34395: Support complete rule-break and rule-visibility-items partition grammars.
- 7b8d662: Accept and canonicalize complete `calc-size()` values across preferred, minimum, maximum, logical, and legacy sizing properties.
- e1339e2: Accept every Chromium `text-box` component order and omission branch.
- 0476ac2: Accept and canonicalize Chromium text-decoration error-line branches.
- 5ff2390: Accept the complete Chromium `text-fit` grammar, including line strategies, limits, and deferred percentage math.
- a514bc8: Support mixed and repeated `none` entries throughout timeline name lists and shorthands.
- 35cc43c: Own the complete Chromium `transform-origin` grammar, including typed depth lengths and calculations, WebKit alias state, canonical axis ordering, explicit zero units, atomic invalid replacement, and safe round-trip serialization.
- ad99232: Own the complete Chromium `-webkit-mask-box-image` shorthand and longhand grammar, including one-to-four component lists, optional slash sections, escaped identifiers, `image-set()` crash safety, canonical CSSOM state, and atomic invalid replacement.
- 948ad94: Own omitted and unordered CSS Text level 4 components in the `white-space` shorthand.
- 6de29fa: Derive canonical and browser-observable declaration serialization from one owned semantic value and recovered component-value tree, removing the parallel raw-text recovery scanner.
- 11cab4c: Remove the legacy JavaScript syntax engine and all JavaScript runtime dependencies. SheetOM now uses its repository-owned Rust engine exclusively, with no behavioral fallback.
- 4de83b0: Parse static generated content and legacy box reflection values through owned typed browser extensions, and remove the raw-string grammar fallback.
- 7a35e33: Type context-dependent CSS Math throughout composite properties and shorthands, including animation lists, flex components, grid lines, aspect ratios, columns, and border-image families.
- 209562a: Parse and serialize anchor-positioned inset values, including recursive fallbacks, CSS math, shorthand expansion, and atomic invalid replacement.

### Patch Changes

- 1b3dbd7: Match Chromium observable values, declaration serialization, indexed names, and mutation atomicity across the complete property-value matrix, including legacy aliases and composite shorthand branches.
- b8b7787: Canonicalize every Chromium grid placement shorthand and longhand branch, including unordered span syntax.
- 6b1c860: Reject standards branches and legacy alias values that the pinned Chromium
  baseline does not accept, including invalid Grid auto-repeat tracks, without
  weakening CSS-wide keywords or pending substitutions.
- bdc8634: Match Chromium indexed declaration state for marker, mask position, table spacing, animation delay, legacy background size, and cursor hotspot branches.
- ab8be29: Match Chromium acceptance and observable declaration state across the complete 711-property value matrix, including typed property domains, modern timeline, anchor positioning and flex wrapping branches, and atomic shorthand expansion.
- 295bae4: Make every pinned Chromium property branch serialize idempotently while preserving shorthand observability and atomic invalid-value rejection.
- 5f3a547: Close the Chromium/Webref acceptance gap with complete hyphenation-limit and math-font-size grammars.
- bd9aef2: Match Chromium getters and declaration text across every pinned Webref property-value branch.
- 32506b1: Match Chromium's `base-select`, `color-scheme`, animation, transition-time, and
  `attr()` grammar boundaries without weakening atomic invalid-value handling.
- b1b5b0c: Hydrate font-feature, property, and generic rules entirely from Rust-owned rule
  DTOs, and align their public map and descriptor behavior with Chromium.
- 1449410: Enforce configurable per-sheet resource budgets before native parsing and CSSOM mutation.
- 06c7c41: Expand every standard border-image grammar branch through the vendored typed AST while preserving Chromium observable values.
- 8d44286: Publish RC6 after its complete immutable-SHA validation while retaining the seven-night soak as a mandatory gate for the first stable release.
- 2757c81: Match Chromium's observable expanded mask image and multi-layer position longhands without changing shorthand synthesis.
- 48c0edf: Preserve mask mode and composite values when expanding standard and prefixed mask shorthands.
- 9c8e123: Expose Chromium-compatible interfaces for layer statements, namespaces, font palette values, and view transitions.
- 367a336: Synchronize native release metadata and require seven nightly full-matrix validations on the unchanged RC6 release SHA.
- ef09cb9: Normalize non-finite CSS Math and deferred relative-color functions like
  Chromium, make bundled media-query ordering deterministic, and gate the full
  vendored parser test suites.
- cc6a43c: Hydrate and serialize deeply nested rule trees iteratively so inputs at the configured syntax-depth boundary remain usable without JavaScript stack overflows.
- 16024a7: Route ordinary CSS declaration blocks through the process-safe Rust engine, including complete shorthand branch validation and browser-observable state.
- dda22c1: Validate and serialize `@font-face` declaration blocks in the Rust engine with Chromium-compatible descriptor grammar.
- 95cafc8: Move exact top-level CSS rule scanning into the process-safe Rust engine while preserving recovered source bytes.
- 442f6b4: Add a process-safe Rust rule parser that returns owned, parser-independent rule trees across Node-API.
- f7b19a1: Validate and canonicalize selectors, media lists, supports conditions, container
  queries, and scope boundaries through the native Rust CSS engine.
- 594d302: Expand and synthesize animation ranges through canonical parallel longhand lists with Chromium-compatible CSSOM observability.
- ad7ea09: Match Chromium background getters and longhands across layered boxes, two-value sizes, image sets, and CSS math.
- 6a2a495: Match Chromium border shorthand ordering, omitted logical longhands, and safe serialization without suffix-based property inference.
- 2fc83f4: Accept Chromium keyword longhands through a typed Rust registry and preserve legacy alias observability across substitutions and removal.
- e3d3853: Match current Chromium keyword, ordered-set, and legacy break alias grammars without weakening invalid-neighbor atomicity.
- 4e27640: Parse, expand, mutate and synthesize the complete Chromium `font-variant`
  grammar with browser-compatible longhand and component ordering.
- 2775c00: Parse, expand, mutate and synthesize the complete Chromium CSS Gaps rule-list
  grammar, including integer and auto repeats, across row, column and combined
  rule shorthands.
- 279fff9: Own Chromium geometric property grammars, SVG path data, shape commands, image observability, and process-safe large-value handling in the native engine.
- 613b372: Parse, expand, mutate and synthesize every generated Chromium `grid` and
  `grid-template` grammar branch with browser-compatible CSSOM serialization.
- efdf98c: Match Chromium modern clip-path shapes and cursor URL-set grammars with atomic invalid-neighbor rejection and crash-safe serialization.
- d8cdeca: Own Chromium name, scope, counter, quote, paint-order, visibility, and will-change grammars as typed Rust state.
- 8a3612c: Parse, expand and synthesize complete offset paths, rays, coordinate boxes and
  motion shorthand values with Chromium-compatible CSSOM serialization.
- 3f9c9cd: Parse and synthesize complete position-try fallback lists, physical tactics and
  unordered named fallback components with Chromium-compatible CSSOM output.
- 6caae5a: Canonicalize border radii and preserve browser-compatible outline and list-style component provenance.
- 408ce2f: Match Chromium scalar zero aliases, optional-pair compression, font-style defaults, and text-shadow ordering.
- 8db8227: Retain owned semantic declaration values as CSSOM state and derive observable
  and reparsable projections from that authority.
- 75608ba: Retain every accepted font-face descriptor as owned semantic state, preserve CSSOM math-function structure, and differentially gate descriptor grammar branches against Chromium.
- 41b998a: Store expanded shorthand longhands as owned semantic values, add typed Chromium longhand grammars, and gate the supported branches against Chromium without retaining textual Codec authority in style declarations.
- a255d80: Own `clip`, `dynamic-range-limit`, `animation-trigger`, and `position-area` as typed Chromium-compatible grammars, and reserve containing-rule depth so every accepted declaration remains safely serializable.
- d47bb33: Preserve explicit text-emphasis defaults, canonical component order, and omitted-longhand CSSOM state across standard and WebKit aliases.
- d5ca92b: Parse, expand and synthesize complete Chromium timeline-trigger lists, typed
  scroll and view sources, ranges and safe stylesheet serialization.
- 389ebab: Match Chromium transition shorthand observability while retaining unambiguous, idempotent safe stylesheet output.
- ed1fca9: Parse and canonicalize `@counter-style` names and descriptors through the
  panic-contained Rust engine, including browser-matched mutation semantics and
  grammar branches.
- eef53e9: Parse every pinned Chromium relative-color branch through the owned semantic AST, match CSSOM observable serialization, and gate all 1,306 accepted and rejected WPT cases without adding proof data to the runtime bundle.
- 0c9ebc7: Parse and recover public stylesheet rule trees through the panic-contained Rust
  engine while preserving malformed declarations and legacy conditional preludes.
- f9969f6: Teach the vendored CSS value engine to retain authored math-function structure for CSSOM descriptor serialization without changing its default optimization behavior.
- 387b702: Publish the exact multi-platform tarball that passed the complete consumer matrix instead of rebuilding a host-only package during the release workflow.
- 024c29c: Record the exact repository-owned native engine, grammar execution, and process-safety execution in RC6 compatibility reports.
- 88ba8bc: Record a Chromium-wide property/value conformance matrix and atomic rejection evidence for the RC6 grammar closure.
- 4086e3e: Record the complete zero-divergence Webref property-branch execution in the immutable RC6 Compatibility Report.
- 7a02480: Reject invalid keyword ranges in the `@font-face` `font-stretch` descriptor while retaining valid percentage ranges.
- 160465a: Reject incompatible percentage calculations without invoking a recoverable native panic or writing to stderr.
- 2ec2350: Remove the cssstyle runtime dependency, own observable-value serialization,
  and require every static shorthand to use an explicit browser-backed codec.
- 024c29c: Accept, canonicalize, and safely reparse Chromium's numeric math-function branches for `z-index`, including constants and neighboring invalid values.
- 195c3a8: Ship the Rust declaration engine as fail-fast prebuilt Node-API bindings for
  glibc and musl Linux, macOS, and Windows on x64 and arm64.
- 8154940: Expose custom `@function` rules, typed parameters, return types, conditional
  declaration runs, and live function descriptors through a Rust-owned parser
  and Chromium-differential CSSOM interface. Preserve internal token-stream
  comments and CSS-significant Unicode whitespace, and keep recovered group-rule
  insertion atomic when trailing input is invalid.
- 367a336: Keep the embedded Rust crates and Cargo lockfile on the exact SheetOM version prepared by Changesets.
- 9aa310e: Accept 105 Chromium composite-longhand grammar branches with typed state, canonical serialization, and atomic invalid-neighbor handling.
- 50f5368: Preserve context-dependent CSS Math number results in typed values and verify 840 property-level Chromium observations, including dynamic products and quotients.
- cd34900: Validate CSS Math result dimensions in the vendored semantic AST, preserve non-finite constants, and gate 94 accepted and rejected math branches against Chromium.
- 225e84d: Parse `contrast-color()` and computed-context `color-mix()` values through the vendored typed color AST across every Chromium color property.

## 0.1.0-rc.5

### Patch Changes

- 7e1e99c: Expand shorthand compatibility across reviewed grammar branches, including
  logical multi-values, compound placement, transition and timeline lists,
  text-box edges, and browser-facing animation, overscroll, and background
  synthesis. Ship exact Chromium evidence for 92 cases across all 23 codec
  profiles and block releases when that evidence is incomplete.

## 0.1.0-rc.4

### Patch Changes

- 532fbfe: Complete the Chromium 151 shorthand codec registry with 129 concrete capability fixtures, safe round-trips, and browser-backed rendering witnesses for high-risk families.

## 0.1.0-rc.3

### Patch Changes

- 5f7ac68: Accept measured modern value families and classify deferred substitutions from original tokens while rejecting neighboring Chromium-incompatible values atomically.
- 80cfefb: Expand static shorthands into canonical longhand state so later longhand mutations cannot reactivate historical shorthand values.
- 491b361: Retain concrete per-engine operation-fixture observations and add shorthand rendering witnesses to the Chromium differential suite.

## 0.1.0-rc.2

### Patch Changes

- 05c93a1: Add a deterministic grammar-oriented declaration differential across Chromium, Firefox, and WebKit.
- dcfeb83: Gate performance against a publisher-shaped multi-stylesheet workload with distributed mutations and repeated serialization.
- 8f4734b: Pin the complete syntax engine dependency set and make release-channel reconciliation explicit before immutable GitHub publication.
- d0b57a8: Preserve legacy and functional container-query syntax while exposing browser-compatible structured container getters.
- 1660d90: Match Chromium's category-aware observable serialization for EOF-recovered typed values, pending substitutions, and custom properties.
- be3a8c5: Expose diagnostic codes as a stable string union and document the opt-in queue's complete-input memory contract.
- 05c93a1: Verify reparsable output and malformed custom-property substitution equivalence in native browser engines.

## 0.1.0-rc.1

### Patch Changes

- f21c896: Reject declaration-boundary escapes atomically, preserve logical custom-property
  names and legacy media syntax, canonicalize Chromium property aliases, and gate
  release evidence on every Operation Fixture adapter.

## 0.1.0-rc.0

### Minor Changes

- f80e926: Introduce the first release candidate of SheetOM with mutable authoring CSSOM
  rules, browser-compatible declaration recovery, safe stylesheet serialization,
  and versioned browser/WPT compatibility evidence.
