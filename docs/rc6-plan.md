# RC6 implementation plan

RC6 replaces SheetOM's JavaScript syntax-engine stack with one repository-owned Rust CSS Engine while preserving the existing public Authoring CSSOM API. No partial RC6 package is published during migration.

## Pull request sequence

1. **Architecture RC6**
   - Land the glossary and ADRs defining the Rust/JavaScript boundary, vendoring policy, safety contract, compatibility claim, packaging model and release gate.

2. **Import Lightning CSS**
   - Clone the complete tracked upstream snapshot at `c6a0c3cebf3395635e61075d2c81a96a710d4910` into `vendor/lightningcss/`, excluding only `.git`.
   - Keep the snapshot commit free of SheetOM modifications.
   - Record origin, revision, license and a reproducible tree-verification command in a following commit.

3. **Workspace Rust**
   - Add `sheetom-core` and the narrow N-API package.
   - Resolve Lightning and its local crates through repository paths and Cargo patches.
   - Set an unwind-capable release profile, clippy, rustfmt, unit tests, licensing checks and cached Rust CI.
   - Keep the TypeScript engine authoritative.

4. **Process safety**
   - Accept only strings and validated primitive inputs across N-API and return owned DTOs.
   - Enforce Resource Budgets before mutation.
   - Add subprocess crash fixtures, cargo-fuzz targets and the published `image-set()` regressions for background, mask and prefixed mask.
   - Isolate general vendored-Lightning safety corrections in upstreamable commits.

5. **Grammar engine**
   - Generate the Versioned Grammar Inventory from the pinned browser and specification inputs.
   - Add positive, neighboring negative, recovery and mutation cases for every production branch.
   - Correct vendored Lightning or add general Rust grammar implementations for every gap, including animation `auto`, system fonts, four-value background positions, row rules and rule shorthands.
   - Remove literal runtime overrides before completion.

6. **Declaration engine**
   - Move ordered declaration state, priority, custom-property identity, static shorthand expansion and synthesis, pending substitutions and atomic mutation into Rust.
   - Run the TypeScript and Rust engines in test-only shadow mode across unit, operation, WPT and generated mutation sequences.
   - Resolve every observable difference before changing authority.

7. **Rules and serialization**
   - Route rule parsing, conditional syntax, selector normalization, recovery and reparsable serialization through the Rust CSS Engine.
   - Meet the Observable Fidelity Gate for getters, `cssText`, length/item order, priority, detachment and repeated serialization.
   - Remove the old TypeScript syntax engine and npm `lightningcss` dependency after shadow parity.

8. **Native packaging**
   - Assemble one dependency-free tarball containing all eight tested GNU, musl, Windows and macOS x64/ARM64 binaries.
   - Test Node.js 22 and 24 on the claimed platforms plus the documented Bun and Deno paths using those exact bytes.
   - Never download binaries in `postinstall` or select a behavioral fallback.

9. **Conformance and stabilization**
   - Record a fresh Pinned Browser Baseline and immutable compatibility report.
   - Complete applicable WPT, differential generation, native reparsing, extended fuzzing and the Publisher Performance Regression Gate.
   - Require seven consecutive nightly full-CI runs on the unchanged release pull request SHA before publishing RC6.

## Completion contract

RC6 is ready only when all supported inputs are process-safe within their Resource Budgets, every branch in the Versioned Grammar Inventory has positive and negative evidence, no Chromium-accepted manifested value is silently lost, no known observable divergence remains unresolved, all native consumer packages are verified, and no old runtime parser or fallback path remains.
