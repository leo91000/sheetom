# Authoring CSSOM

This context models browser-compatible stylesheet authoring outside a browser. It covers parsing, mutable stylesheet rules and declarations, and serialization, but not DOM attachment, the cascade, layout, or computed styles.

## Language

**Authoring CSSOM**:
The browser-compatible object model for parsing, inspecting, mutating, and serializing a stylesheet without a DOM or rendering engine.
_Avoid_: Full CSSOM, virtual DOM, CSS compiler

**Rust CSS Engine**:
The repository-owned Rust module that parses and recovers CSS syntax, validates property grammars, expands and synthesizes shorthands, owns ordered declaration state, and produces reparsable syntax for the JavaScript Authoring CSSOM facade.
_Avoid_: Native helper, Lightning CSS wrapper, JavaScript parser

**JavaScript CSSOM Facade**:
The public JavaScript classes and proxies that own WebIDL coercion, indexed access, live rule identity, detachment, and runtime ergonomics while delegating syntax and declaration semantics to the Rust CSS Engine.
_Avoid_: CSS engine, parser binding, generated N-API classes

**Vendored Lightning Source**:
The complete upstream Lightning CSS source snapshot imported as ordinary repository files in one isolated commit, built through local Cargo paths and modified in later focused commits. Its recorded upstream revision and MPL notices make local changes reproducible and extractable for upstream contribution.
_Avoid_: npm lightningcss, Git dependency, subtree, submodule, opaque vendor binary

**Native Data Boundary**:
The narrow N-API contract between the JavaScript CSSOM Facade and Rust CSS Engine. It accepts strings and validated primitive inputs and returns owned domain DTOs; arbitrary JavaScript or Lightning AST objects never cross it.
_Avoid_: Visitor API, AST roundtrip, generic object bridge

**Process Safety Contract**:
The release-blocking invariant that every finite public CSS input within documented resource limits completes with a result, an atomic no-op, or a controlled JavaScript error without terminating the host process. It is enforced structurally at the Native Data Boundary and empirically by subprocess crash tests and grammar-oriented fuzzing.
_Avoid_: Panic catch, no known crashes, memory safety claim

**Resource Budget**:
The per-sheet, explicitly configurable upper bounds checked before native mutation for source bytes, declaration-value bytes, syntax depth, rule count, and declarations per block. RC6 defaults are 64 MiB, 1 MiB, 4,096, 1,000,000, and 100,000 respectively; exceeding a budget raises a controlled `RangeError` before state changes.
_Avoid_: Global limit, parser timeout, silent truncation

**Native Platform Package**:
An exact-version optional npm package containing one prebuilt Rust CSS Engine binary for a single supported operating-system, CPU, and libc target. The root `sheetom` package selects it locally and never downloads executable code during installation.
_Avoid_: Universal binary, postinstall download, optional engine

**Supported Native Matrix**:
The release-blocking set of real consumer environments for RC6: Linux x64 GNU, Linux ARM64 GNU, Windows x64, and macOS ARM64 for Node.js 22 and 24, with Bun and Deno additionally exercised on Linux x64. A platform outside this matrix fails explicitly instead of selecting a second syntax engine.
_Avoid_: Node-API compatible platforms, best-effort architecture, WASM fallback

**Shadow Engine Run**:
A test-only execution that applies the same operation to the incumbent TypeScript declaration engine and the candidate Rust CSS Engine, compares complete observable state and reparsable output, and has no authority to choose a result at runtime.
_Avoid_: Runtime fallback, dual implementation, differential fixture

**Observable Fidelity Gate**:
The release check requiring every supported Authoring CSSOM operation to match its Compatibility Resolution for getter text, `cssText`, declaration length and item order, priorities, atomicity, live identity and mutation sequences even when both implementations would render equivalently.
_Avoid_: Rendering equivalence, serialization safety, known P2 divergence

**CSSOM Surface Serialization**:
The browser-compatible text exposed by declaration and rule getters such as `cssText`. It may retain browser quirks and is not guaranteed to be safe stylesheet input.
_Avoid_: Final output, source text

**Reparsable Stylesheet Serialization**:
Serialization of the observable Authoring CSSOM state into reparsable stylesheet output, including repair of recoverable syntax. It does not preserve source comments, whitespace, quoting choices, or original bytes.
_Avoid_: Safe serialization, CSSOM getter text, source-preserving serialization, round-trip formatting

**Measured Substitution Equivalence**:
The release-baseline evidence that a recovered custom property produces equivalent browser behavior when substituted into named consuming properties, without promising identical computed custom-property text.
_Avoid_: Computed-value identity, universal semantic equivalence, SheetOM computed style

**Rendering Boundary**:
The caller-owned point where authored CSS is attached to a document or another environment that can fetch resources or apply browser behavior; SheetOM does not cross or sanitize this boundary.
_Avoid_: Serialization, stylesheet parsing, import loading

**Declaration Mutation**:
An atomic update produced by parsing one property value independently from its containing stylesheet. Invalid names, values, or priorities leave the existing declaration unchanged.
_Avoid_: Stylesheet reparse, permissive property insertion

**Mutation Diagnostic**:
An optional out-of-band explanation of an ignored or recovered mutation. It does not alter the mutation's browser-compatible return value, exception, or resulting CSSOM state.
_Avoid_: Mutation error, thrown validation error

**Diagnostic Queue**:
An opt-in, per-sheet sequence of Mutation Diagnostics that consumers drain explicitly without affecting the operation that produced them.
_Avoid_: Error callback, diagnostic event

**Diagnostic Code**:
The stable machine-readable classification of a Mutation Diagnostic; its accompanying human-readable message is explanatory and may change.
_Avoid_: Error message, exception name

**Unsupported Shorthand Value**:
The diagnostic classification for a browser-accepted static shorthand that SheetOM cannot expand completely through its registered codec. The mutation remains atomic, its message describes a codec limitation rather than invalid CSS, and any occurrence in a positive Grammar Branch Case is a release-blocking implementation defect.
_Avoid_: Invalid property value, parser rejection, opaque shorthand state

**Declaration Record**:
One ordered, unique expanded-longhand or exact-case custom-property entry containing its browser-facing text and recovered semantic representation together with its priority and shorthand provenance.
_Avoid_: Raw declaration, parallel stylesheet entry

**Recovered Token Text**:
Browser-facing value text produced by CSS token recovery while preserving category-specific omitted EOF structures after lexical effects such as comment removal and escaped-code-point replacement.
_Avoid_: Raw input, repaired stylesheet value, universal canonical form

**Static Shorthand Codec**:
The single internal expansion and synthesis seam for one semantic family of static shorthands, converting accepted values into complete canonical longhand records and reconstructing browser-facing shorthand text only from current longhand state.
_Avoid_: Shorthand state, cssstyle delegation, parser fallback

**Shorthand Codec Profile**:
The shared semantic grammar and expansion shape implemented by one or more Static Shorthand Codecs, such as repeated sides, semantic pairs, parallel lists, or layered values. Every registered profile owns a reviewed Grammar Branch Contract.
_Avoid_: Property name, test category, parser fallback

**Pending Shorthand Group**:
Provenance connecting expanded Declaration Records to one shorthand mutation containing a genuinely deferred substitution, retaining recovered tokens only until a longhand mutation breaks the group.
_Avoid_: Static shorthand state, serialized shorthand, original declaration

**Shorthand Coverage Gate**:
The release check requiring every multi-longhand property in the Supported Property Manifest to have an atomic Static Shorthand Codec, complete positive and neighboring negative evidence for every production branch in the Versioned Grammar Inventory, longhand mutation and removal sequences, and reparsable round-trip evidence. A canonical or literal value alone can never establish support.
_Avoid_: Best-effort expansion, supported examples, cssstyle coverage

**Shorthand Capability Corpus**:
The versioned, manifest-bound browser evidence containing concrete breadth cases, Grammar Branch Cases, mutation scenarios, and ordered Chromium observations for manifested shorthands. It is published with the package and release for offline audit, but is conformance evidence and never an authority that makes a runtime value valid.
_Avoid_: Runtime allowlist, initial-value smoke test, exhaustive grammar, one fixture file per property

**Grammar Branch Contract**:
The finite, reviewed inventory of positive and neighboring negative forms that every registered Shorthand Codec Profile must recognize and measure, including every relevant arity, separator, list form, optional component, ordering alternative, CSS-wide value, substitution, and recovery form from the Versioned Grammar Inventory. It exhausts the named baseline inventory rather than claiming compatibility with unversioned future CSS.
_Avoid_: Complete CSS grammar, examples list, parser implementation branches

**Versioned Grammar Inventory**:
The release-pinned inventory of property and shorthand productions supported by the named Chromium baseline, derived jointly from CSSWG/Webref productions, the measured browser property manifest, applicable WPT, relevant Chromium tests, systematic oracle probes, and reviewed dispositions for grammar defined in prose. It is the exhaustive semantic compatibility boundary for one release and is revised explicitly when the browser baseline changes.
_Avoid_: Every future CSS value, curated examples, parser feature list

**Grammar Branch Case**:
A browser-observed positive or negative shorthand mutation assigned to one named branch of a Grammar Branch Contract and required to pass that branch's expansion, observation, mutation, and round-trip checks. Its evidence includes the semantic branch actually selected by the codec, not only a curated label, and a negative case names the positive state against which rejection must be atomic.
_Avoid_: Unit-test example, literal runtime override, breadth seed

**Measured Literal Override**:
A temporary shadow-migration exception for one exact property-value form that the pinned browser accepts but the Rust CSS Engine cannot yet validate. It can localize a known gap while implementing a grammar correction, but cannot satisfy a Grammar Branch Contract or exist in an RC6 runtime artifact.
_Avoid_: Grammar validator, corpus case, permissive fallback

**Authoring Roundtrip Witness**:
A browser probe that reparses Reparsable Stylesheet Serialization and compares the resulting Authoring CSSOM declaration state, without relying on cascade, layout, or computed style.
_Avoid_: Rendering Witness, SheetOM-only idempotence, computed-style test

**Supported Property Manifest**:
A checked-in list of ordinary property names accepted by the named Chromium compatibility baseline, generated offline from real-browser probes.
_Avoid_: Specification property list, Lightning CSS property list

**Value Capability Corpus**:
A release-versioned set of positive and negative property-value families measured against the pinned browser baseline, used to bound SheetOM's claim for modern or implementation-dependent CSS grammar.
_Avoid_: Complete CSS grammar, latest-browser support, string allowlist

**Value Capability Validator**:
A narrow property-family validator that fills a measured grammar gap or corrects a known parser mismatch, backed by neighboring positive and negative cases in the Value Capability Corpus.
_Avoid_: Literal value allowlist, permissive unparsed fallback, browser runtime probe

**Value Gate**:
The layered decision that accepts or rejects one independently parsed property value by classifying substitutions from original tokens before typed parsing, grammar matching, and measured Value Capability Validators.
_Avoid_: Lightning CSS parse result, text heuristic

**Accepted Property Value**:
The internal structured result transported from the Value Gate to mutation codecs, retaining either a typed parser declaration, validated grammar tokens, or recovered pending-substitution tokens together with observable and reparsable serializations.
_Avoid_: Reparsed value string, public value object, raw input

**Syntax Engine Set**:
The exact, release-versioned Rust CSS Engine source revision, Vendored Lightning Source snapshot and local patch set, plus any remaining parser or tokenizer dependencies whose joint behavior underpins a Compatibility Baseline.
_Avoid_: Compatible dependency range, lockfile snapshot, npm Lightning CSS version

**Live CSSOM Object**:
A stable JavaScript object whose reads and writes reflect the current shared stylesheet state, including after related objects mutate that state.
_Avoid_: Snapshot, value object, detached copy

**Detached Rule**:
A retained Live CSSOM Object that no longer belongs to a stylesheet or parent rule. It keeps its last rule state and remains independently mutable.
_Avoid_: Invalid rule, deleted wrapper

**Generic Rule**:
A live rule whose normalized token representation and position are preserved even though its specialized browser rule interface is not implemented yet.
_Avoid_: Unsupported rule, opaque source fragment

**Opaque Recovered Rule**:
An immutable rule record holding the exact tokenizer span for source that a forgiving semantic parser would otherwise drop. It can be deleted or reordered but exposes no invented specialized mutation interface.
_Avoid_: Generic Rule, invalid declaration, normalized rule

**Rule Codec**:
An internal adapter that converts between isolated CSS syntax and the owned Rule Tree without owning live identity, parentage, declaration state, or observable serialization.
_Avoid_: Rule model, stylesheet state, parser authority

**Constructed Sheet**:
A stylesheet created with the standard `CSSStyleSheet` constructor. It follows browser replacement and insertion restrictions, including removal or rejection of `@import` rules.
_Avoid_: Parsed stylesheet, regular stylesheet

**Regular Authoring Sheet**:
A stylesheet created from existing CSS source outside a DOM, retaining valid `@import` rules without loading their targets.
_Avoid_: Constructed sheet, imported stylesheet graph

**URL Context**:
The explicit base against which relative CSS references are understood. It comes from sheet metadata and never implicitly from the process working directory.
_Avoid_: Working directory, import loader

**Forgiving Sheet Parse**:
Whole-stylesheet parsing that recovers or drops invalid CSS while returning the resulting sheet instead of throwing for CSS syntax errors.
_Avoid_: Strict parse, rule insertion

**Compile Snapshot**:
An isolated copy of stylesheet state on which target compilation, prefixing, rule merging, or minification may operate without changing live CSSOM objects.
_Avoid_: Live transformation, serialization

**Divergence Fixture**:
A recorded CSSOM operation whose observable result differs between browser engines, together with the compatibility outcome selected by the project's precedence policy.
_Avoid_: Browser snapshot, flaky expectation

**Compatibility Baseline**:
The named browser-engine and dependency versions against which a package release's measured behavior is reported.
_Avoid_: Universal CSSOM compatibility, latest browsers

**Pinned Browser Baseline**:
The exact Playwright Chromium, Firefox, and WebKit builds recorded for one release candidate and used throughout its oracle recording, differential tests, and compatibility report. Updating any build creates a new baseline review rather than silently changing expected behavior.
_Avoid_: System Chrome, current stable, reusable previous-release baseline

**Compatibility Report**:
The immutable, schema-validated release artifact containing a Compatibility Baseline, WPT Dispositions, Oracle Observations, Compatibility Resolutions, shorthand corpus and branch-model hashes, exact gate outcomes, and their summary counts.
_Avoid_: Release notes, mutable dashboard, test log

**Baseline Draft**:
A reviewable candidate Compatibility Report produced by an explicit recording run before it is accepted as immutable release evidence.
_Avoid_: Updated snapshot, CI artifact, current baseline

**Supported Release**:
The latest published SheetOM `0.x` minor and its active prereleases, which alone receive fixes before version 1.0.
_Avoid_: Every published version, maintenance branch, latest commit

**Release Channel**:
An npm installation track: before the first stable release, `latest` and `next` both identify the active prerelease; afterward, `latest` identifies the active stable release and `next` exists only while an active prerelease exists.
_Avoid_: Support branch, version range, every dist-tag

**Applicable Conformance Test**:
A specification or Web Platform Test scenario that exercises Authoring CSSOM without requiring the excluded DOM, cascade, layout, or computed-style capabilities.
_Avoid_: Every WPT, browser test

**Operation Fixture**:
A schema-versioned declarative sequence of Authoring CSSOM operations and stable object handles that executes unchanged through SheetOM and Engine Oracle adapters. Significant cross-engine resolutions and multi-step API regressions use fixtures, while codec grammar breadth remains in Grammar Branch Cases instead of being duplicated here.
_Avoid_: Vitest case, browser script, unit test

**Boundary Value**:
A tagged fixture value representing a JavaScript input that JSON cannot preserve directly, including `undefined`, non-finite numbers, BigInt, Symbol, and controlled coercion behavior.
_Avoid_: Magic string, JSON null, fixture expression

**Oracle Observation**:
An immutable, structured record of what one Engine Oracle exposed while executing an Operation Fixture under a specific Compatibility Baseline.
_Avoid_: Expected result, golden truth, latest snapshot

**Compatibility Resolution**:
A reviewed choice of shared behavior, specification behavior, Chromium fallback, or Scope Exclusion made from Oracle Observations, with its rationale retained separately from the Operation Fixture.
_Avoid_: Snapshot update, test expectation

**Scope Exclusion**:
A reviewed conformance scenario that requires capability outside Authoring CSSOM and records that specific reason instead of appearing as an unexplained failure or silently disappearing.
_Avoid_: Skipped test, unsupported test

**WPT Disposition**:
The reviewed classification of one pinned Web Platform Test subtest as applicable, partially applicable, or a Scope Exclusion, retaining its upstream path, exact subtest title, and reason.
_Avoid_: WPT file status, test percentage

**WPT Mapping**:
The provenance connection from a pinned WPT subtest and source blob to independently stable Operation Fixture identifiers, including whether coverage is full or partial and whether translation is manual or generated.
_Avoid_: Copied test, fixture expectation

**WPT Tombstone**:
The reviewed record that a previously mapped upstream WPT subtest was removed or renamed, preserving its historical provenance and resolution instead of silently deleting it from coverage.
_Avoid_: Deleted fixture, ignored upstream change

**Conformance Gate**:
The complete set of Applicable Conformance Tests and Divergence Fixtures that a release must pass or explicitly classify against its Compatibility Baseline.
_Avoid_: Test suite, coverage target

**Acceptance Candidate Gate**:
The final prerelease gate requiring the Process Safety Contract, Conformance Gate, Observable Fidelity Gate, complete Versioned Grammar Inventory, Supported Native Matrix, package-consumer tests, Performance Regression Gate, and seven consecutive scheduled validation runs to pass before RC6 is published.
_Avoid_: Release checklist, first green CI run, external beta testing

**Engine Oracle**:
A pinned browser build that executes an Authoring CSSOM scenario to provide empirical behavior for differential comparison; it informs compatibility but does not override specifications or Web Platform Tests by itself.
_Avoid_: Source of truth, browser profile

**Rendering Witness**:
An out-of-scope browser probe that attaches reparsable SheetOM output and compares selected computed longhands solely as evidence that serialization preserved the authored state.
_Avoid_: SheetOM computed style, conformance API, rendering engine

**Reference Workload**:
The server and build-time authoring scales used to evaluate SheetOM performance: both a large single sheet with concentrated mutations and a publisher-shaped set of shared and page sheets with distributed rules, declarations, grouping, mutations, and final serialization.
_Avoid_: Animation workload, stress maximum

**Performance Regression Gate**:
The same-runner comparison of warmed, repeated median measurements for the Reference Workload across Linux x64 and ARM64, covering native cold import, parse, distributed mutation, deletion, first and second serialization, peak RSS, and installed package size. A regression above 15% requires explicit review and cannot be hidden by a permissive absolute ceiling.
_Avoid_: One-shot benchmark, microbenchmark, fixed wall-clock SLO
