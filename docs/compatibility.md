# Compatibility

SheetOM targets browser-compatible **Authoring CSSOM** behavior: parsing,
stylesheet and rule mutation, declaration state, and serialization outside a
DOM. Compatibility is measured at the public package boundary.

## What the promise covers

Release gates compare:

- accepted and rejected property mutations;
- atomic preservation after invalid mutations;
- `getPropertyValue()`, `getPropertyPriority()`, and `cssText`;
- declaration `length`, indexed names, and order;
- shorthand expansion, synthesis, priority, and mutation sequences;
- live object identity, rule parentage, and detachment;
- rule insertion, deletion, replacement, and hierarchy errors;
- malformed-input recovery and reparsable stylesheet serialization;
- native and WebAssembly backend behavior;
- process safety within documented Resource Budgets.

Rendering equivalence is useful evidence but cannot replace observable CSSOM
fidelity. Conversely, `serialize()` is a SheetOM extension: browsers reparse its
output and expose the equivalent object model rather than implementing the same
method.

## Browser precedence

SheetOM follows standards and behavior shared by the pinned browser engines.
When the engines genuinely disagree, a checked-in Compatibility Resolution
records the observations and rationale; Chromium is the final measured
fallback. This policy is explicit rather than a claim that all engines always
agree.

Each release report pins exact Chromium, Firefox, WebKit, WPT, runtime, and
vendored-source revisions. It separates explained browser divergences from
unexplained outcomes, and those versions or counts may advance only through
reviewed evidence.

## Evidence layers

| Evidence | Purpose |
| --- | --- |
| Unit and Rust tests | Local semantic, serialization, and boundary invariants. |
| Operation Fixtures | Stable public mutation sequences executed by SheetOM and browser adapters. |
| WPT mappings | Reviewed applicable subtests with pinned source path, title, and blob identity. |
| Grammar contracts | Positive branches and neighboring invalid values for supported property grammar. |
| Browser differentials | Batched generated operations comparing complete observable declaration state. |
| Native reparsing | Browser verification that `serialize()` output remains confined and reparsable. |
| Fuzzing and subprocess tests | Recovery, atomicity, resource limits, and process-crash resistance. |
| Backend evidence | Exact native packages plus direct, worker, bundler, memory, and performance WASM runs. |

Browser observations, Webref inputs, and WPT source are evidence only. Runtime
behavior comes from reviewed Rust grammar and CSSOM code, never from importing a
large recorded corpus or launching a browser.

## Release threshold

A stable release is blocked by any:

- known process crash on supported finite input;
- Chromium-accepted supported value that silently disappears;
- unexplained browser-oracle or corpus mismatch;
- public type declaration that disagrees with runtime construction or behavior;
- incomplete native or WebAssembly artifact cohort;
- compatibility report that cannot be reproduced from the exact release SHA.

Documented browser divergences are allowed only when observations, precedence,
and rationale are recorded. A green aggregate test count cannot hide an
unreviewed mismatch.

## Deliberate exclusions

SheetOM does not implement:

- DOM association or browser stylesheet collections;
- cascade, selector matching, inheritance, or computed/resolved values;
- layout, painting, animation execution, or rendering;
- network loading for `@import`, fonts, images, or other URLs;
- CSS sanitization, CSP enforcement, or remote-resource policy.

Generic and recovered rules may be retained even when SheetOM has no specialized
mutable interface for them. Retention is not a claim that every future rule API
already exists.

## Where the evidence lives

- `compatibility/baselines/` — immutable release reports
- `compatibility/fixtures/` — Operation Fixtures
- `compatibility/resolutions/` — browser-divergence decisions
- `compatibility/wpt-mappings.json` and `wpt.lock.json` — WPT provenance
- `compatibility/*contracts.json` — reviewed grammar contracts
- `compatibility/*observations.json` — pinned browser results

These files stay in the source repository and release evidence; they are not
shipped in the runtime npm tarball.
