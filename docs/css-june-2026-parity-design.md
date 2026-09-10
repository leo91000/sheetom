# June 2026 CSS authoring parity design

Status: all design decisions and final shared understanding accepted.
Implementation is authorized and implemented in the feature branch; see the
[implementation and evidence](./css-june-2026-parity-evidence.md) for the current
contract, executable coverage mapping, and explicit discrepancy resolutions.

## Accepted scope

The user accepted all six recommendations across the first two rounds:

- Full authoring CSSOM parity, including specialized editable rule interfaces;
  no cascade, computed styles, rendering, or mixin expansion.
- All CSS features Newly available by June 30, 2026, including earlier years,
  with exact compatibility data and browser revisions pinned.
- Explicit experimental inclusion of native CSS mixins and custom functions,
  with their status documented and existing function support audited.
- Experimental mixins follow the current CSSWG draft pinned to an exact
  revision. Browser evidence supports that contract with explicit differences;
  the existing stable browser compatibility policy remains unchanged.
- Specialized mixin support is available by default, without a runtime opt-in.
- Completion requires a complete inventory, zero known unimplemented authoring
  surfaces, and applicable per-feature parsing, mutation, serialization,
  invalid-input, and native/WASM evidence. Missing tests do not establish
  support; browser differences require explicit resolutions.

See [ADR 0188](./adr/0188-target-june-2026-authoring-cssom-and-explicit-experiments.md)
and [ADR 0189](./adr/0189-expose-pinned-draft-mixin-interfaces-by-default.md),
and the [glossary](./glossary.md).

## Initial design evidence (before implementation)

The initial source inspection used commit
`49d941a` (`sheetom` package version `0.1.1`).

- [The API](./api.md) already exposes specialized `CSSFunctionRule` and function
  declaration interfaces. [Function tests](../tests/function-rules.test.ts)
  exercise parameters, conditional bodies, mutation, and deferred calls.
- No specialized native mixin implementation was found in first-party source,
  tests, or documentation. Generic rule retention is not editable mixin support;
  exact mixin round-trip behavior has not yet been tested.
- [The compatibility contract](./compatibility.md) already requires full
  observable CSSOM evidence and native/WASM coverage, with documented browser
  divergence resolutions. This work extends that contract.
- The checked-in [0.1.1 baseline](../compatibility/baselines/0.1.1.json) pins
  Chromium 151.0.7922.34, Firefox 153.0, and WebKit 26.5. This is separate from
  the Web Platform Baseline feature cutoff.
- The served [mixins draft CSSOM](https://drafts.csswg.org/css-mixins-1/#cssom)
  defines separate block and statement interfaces for apply and contents rules.
  Chromium's experimental source instead exposes `CSSApplyMixinRule` and
  `CSSContentsMixinRule`. These are different experimental contracts, not an
  established cross-browser consensus. No runnable flagged-browser oracle has
  yet been verified for the repository's pinned build.
- The current public options have no experimental feature switch. Adding one
  would introduce a new API concept; existing custom functions are already
  available without such a switch.
- The pinned [feature selection inventory](./css-june-2026-feature-inventory.md)
  contains 311 candidate families, including 18 first reaching Baseline during
  January-June 2026. It records 364 compatibility subkeys requiring special
  cutoff disposition; source matches and family selection are not passing
  conformance evidence.

## Decision tree

| Decision | Prerequisite | Status |
| --- | --- | --- |
| Authoring rather than evaluation scope | None | Accepted |
| Cumulative Newly available cutoff of June 30, 2026 | None | Accepted |
| Include native mixins and custom functions explicitly | None | Accepted |
| Current revision-pinned draft contract for mixins | Experimental inclusion | Accepted |
| Default availability and explicit browser discrepancy evidence | Experimental inclusion; inspect existing policy and browser availability | Accepted |
| Complete inventory, no known authoring gaps, per-feature evidence | Authoring scope and feature target | Accepted |
| Concrete feature selection inventory and implementation slices | Selected target data and experimental policy | Prepared; branch-level audit is implementation step 1 |
| Confirm shared understanding before implementation | Six accepted decisions and concrete implementation plan | Accepted |

No product-design question remains open. Detailed branch auditing and explicit
specification discrepancy resolutions are implementation work within the agreed
scope, not permission to change that scope.

## Implementation plan

The following sequence implements the accepted scope within the existing
[architecture](./architecture.md). It does not introduce a second parser or
declaration authority, runtime browser profiles, or a CSS evaluation engine.

1. **Freeze the target and audit the applicable branches.** Produce a
   machine-readable feature manifest from the pinned WebDX data, retaining each
   feature's provenance, compatibility keys, cutoff eligibility, authoring
   surface, and linked evidence. Audit CSS-related APIs and ungrouped features
   as well as CSS group membership. An umbrella feature's early Baseline date
   must not silently include its later additions. Retain existing newer
   SheetOM behavior as regression coverage. Record rendering-only operations as
   outside authoring scope; never use that exclusion to hide missing syntax
   support. Mark unmapped or untested surfaces open.
2. **Implement the experimental mixin family.** Extend the shared Rust rule
   parser and existing transport-neutral rule descriptions, then expose
   `CSSMixinRule`, `CSSApplyBlockRule`, `CSSApplyStatementRule`,
   `CSSContentsBlockRule`, and `CSSContentsStatementRule` through the JavaScript
   facade and WASM facade. Cover typed/default parameters, argument preservation,
   optional empty parentheses, statement versus empty-block distinctions,
   interleaved declarations wrapped in `CSSNestedDeclarations`, contextual rule
   validity, and live grouping mutation. Reuse existing function-parameter and
   declaration machinery where semantics agree. Audit supporting draft syntax
   and document any missing standardized interface rather than inventing one.
3. **Close stable-target and existing-function gaps by family.** Map the
   manifest to current grammar contracts and public operation fixtures. Repair
   actual missing selector, condition, value, declaration, descriptor, or rule
   behavior in the existing owning module, with focused positive and neighboring
   invalid cases. Recheck custom functions under their established contract.
   Source or type-name matches alone never establish conformance.
4. **Prove the selected contract.** Exercise full observable state, parentage,
   identity, detach/reinsert sequences, rejected-mutation atomicity, resource
   limits, observable serialization, safe/strict serialization, and reparsing
   through the public native and WASM packages. Reuse applicable mapped WPT and
   browser differential infrastructure. Pin a runnable experimental Chromium
   build separately if needed; classify unsupported browser observations and
   draft discrepancies explicitly. Complete relevant repository checks and
   performance regressions. Every selected applicable surface must have evidence
   or remain incomplete.

The pinned draft contains editorial inconsistencies, including references to an
`@contents` parameter although its grammar says any mixin may receive a contents
block, and old `@result` examples alongside the current direct-body grammar.
Those need traceable specification resolutions during implementation, with
normative grammar and the agreed draft contract as the starting point. They do
not authorize importing a different experimental browser API or silently
omitting the affected surface.

## Completion gate

The target is complete only when every applicable inventory branch is accounted
for, no known authoring implementation gap remains, required evidence passes on
both backends, and all observed browser differences have explicit resolutions.
The scope does not promise evaluation, rendering, all WPT, unselected future CSS,
or mathematical proof for every possible source string. Incremental slices do
not lower the final completion gate.

The user confirmed the completed shared understanding and authorized runtime
implementation. No product-design question remains pending.

## Documentation placement

The glossary lives at `docs/glossary.md` because the existing repository
documentation checker explicitly forbids the obsolete root `CONTEXT.md` path.
The glossary contains terminology only; scope decisions live in ADRs and
unresolved questions live in this interview document.
