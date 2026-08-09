# Authoring CSSOM

This context models browser-compatible stylesheet authoring outside a browser. It covers parsing, mutable stylesheet rules and declarations, and serialization, but not DOM attachment, the cascade, layout, or computed styles.

## Language

**Authoring CSSOM**:
The browser-compatible object model for parsing, inspecting, mutating, and serializing a stylesheet without a DOM or rendering engine.
_Avoid_: Full CSSOM, virtual DOM, CSS compiler

**CSSOM Surface Serialization**:
The browser-compatible text exposed by declaration and rule getters such as `cssText`. It may retain browser quirks and is not guaranteed to be safe stylesheet input.
_Avoid_: Final output, source text

**Safe Stylesheet Serialization**:
Serialization of the observable Authoring CSSOM state into reparsable stylesheet output, including repair of recoverable syntax. It does not preserve source comments, whitespace, quoting choices, or original bytes.
_Avoid_: CSSOM getter text, source-preserving serialization, round-trip formatting

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

**Declaration Record**:
One ordered, unique expanded-longhand or exact-case custom-property entry containing its browser-facing text and recovered semantic representation together with its priority and shorthand provenance.
_Avoid_: Raw declaration, parallel stylesheet entry

**Shorthand Group**:
Provenance connecting expanded Declaration Records to the accepted shorthand mutation from which they came, allowing getters and serialization to reconstruct a shorthand only while current values and priorities permit it.
_Avoid_: Shorthand declaration, serialized shorthand

**Supported Property Manifest**:
A checked-in list of ordinary property names accepted by the named Chromium compatibility baseline, generated offline from real-browser probes.
_Avoid_: Specification property list, Lightning CSS property list

**Value Gate**:
The layered decision that accepts or rejects one independently parsed property value using typed parsing, validated substitutions, grammar matching, and recorded browser quirks.
_Avoid_: Lightning CSS parse result, text heuristic

**Live CSSOM Object**:
A stable JavaScript object whose reads and writes reflect the current shared stylesheet state, including after related objects mutate that state.
_Avoid_: Snapshot, value object, detached copy

**Detached Rule**:
A retained Live CSSOM Object that no longer belongs to a stylesheet or parent rule. It keeps its last rule state and remains independently mutable.
_Avoid_: Invalid rule, deleted wrapper

**Generic Rule**:
A live rule whose normalized token representation and position are preserved even though its specialized browser rule interface is not implemented yet.
_Avoid_: Unsupported rule, opaque source fragment

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
The named browser and dependency versions against which a package release's measured behavior is reported.
_Avoid_: Universal CSSOM compatibility, latest browsers

**Reference Workload**:
The server and build-time authoring scale used to evaluate SheetOM performance: stylesheets up to roughly one megabyte or ten thousand rules and mutation bursts around ten thousand operations.
_Avoid_: Animation workload, stress maximum
