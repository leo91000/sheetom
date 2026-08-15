# API reference

The generated declaration reference is the exhaustive signature source. Run
`npm run docs:build` to render it under `site/api`. This page records the
behavioral contract that TypeScript declarations cannot express.

## Construction

`new CSSStyleSheet(options?)` is the only standards-defined constructible CSSOM
class. `options` accepts `baseURL`, `media`, `disabled`, and the SheetOM-only
`diagnostics` and `resourceBudget` options. `parseStyleSheet(cssText, options?)` is a SheetOM extension
for existing regular sheets; it preserves valid `@import` rules without loading
them and accepts `href` as source metadata.
Rules rejected by the constructed CSSOM parser are retained as immutable opaque
rules so a regular-sheet round trip does not silently delete authored source.

All exported rule, rule-list, declaration, media-list, and feature-map classes
exist for `instanceof` and typing. Direct construction throws `TypeError`, as it
does in browsers. Obtain their objects from a sheet or parent rule.

The browser WebAssembly backend exports `createSheetOM(source?)` from
`@sheetom/wasm`. Its promise resolves to a frozen object containing the same
public constructors and functions as `sheetom`. Calls without a source share
one concurrent default initialization per JavaScript realm. An explicit
`URL`, `Response`, `ArrayBuffer`, or `WebAssembly.Module` creates an independent
backend and independent class identities. Applications must compare objects
with constructors from the same returned facade. Initialization and traps use
`SheetOMWasmBindingError` with stable `SHEETOM_WASM_*` codes; ordinary CSS and
Resource Budget failures retain the native facade contract.

## Stylesheets and identity

`CSSStyleSheet.cssRules` is stable and live. `insertRule` and `deleteRule` use
WebIDL unsigned-long conversion and throw `IndexSizeError`, `SyntaxError`, or
`HierarchyRequestError` before mutating. `replaceSync` replaces atomically and
strips imports on constructed sheets; `replace` resolves to the same sheet.

Rules and nested lists keep stable identity while attached. Removing or
replacing a rule detaches the old object recursively by setting `parentRule`
and `parentStyleSheet` to `null`; the detached object remains independently
readable and mutable. Assigning any `CSSRule.cssText` is a successful no-op.

`serialize()` is the SheetOM extension that emits reparsable CSS from current
state. This can differ deliberately from browser-facing `cssText` when CSS
Syntax end-of-input recovery preserved malformed source text. It remains
resilient when CSS text cannot exactly encode an imperative pending-shorthand
state and emits an opt-in `UNREPRESENTABLE_PENDING_SHORTHAND` diagnostic.
`serializeStrict()` throws `SheetOMSerializationError` for that state instead.

| Surface | Contract |
| --- | --- |
| `CSSStyleDeclaration.cssText` | Browser-compatible observable declarations, including recovery quirks. |
| `CSSRule.cssText` | Browser-compatible observable text for the current rule. |
| `CSSStyleSheet.serialize()` | Deterministic Reparsable Stylesheet Serialization with repairs confined to their declaration or rule. |
| `CSSStyleSheet.serializeStrict()` | Exact-only reparsable serialization; rejects states CSS text cannot faithfully represent. |

“Reparsable” guarantees syntactic reparse, no declaration/rule leakage, and
preservation of SheetOM’s valid semantic state. It is not URL sanitization,
CSP enforcement, remote-resource control, or a guarantee that valid untrusted
CSS is safe to attach at the Rendering Boundary.

## Resource budgets

`resourceBudget` configures per-sheet limits and is inherited by every parsed,
nested, inserted, and later detached rule or declaration object from that
sheet. Omitted fields use these defaults:

| Field | Default | Measures |
| --- | ---: | --- |
| `maxStylesheetBytes` | 64 MiB | UTF-8 bytes in one stylesheet or rule source input. |
| `maxDeclarationValueBytes` | 1 MiB | UTF-8 bytes in one declaration value. |
| `maxSyntaxDepth` | 4,096 | Maximum nested CSS component-value and rule-block depth. |
| `maxRuleCount` | 1,000,000 | Rules in the resulting sheet or detached tree, including descendants. |
| `maxDeclarationsPerBlock` | 100,000 | Expanded declaration records in the resulting block. |

Fields must be non-negative integer numbers. Byte, rule, and declaration
limits may be raised through the N-API unsigned-long maximum; syntax depth may
be raised to 16,384. Internal parser wrappers do not consume the caller's
source or depth budget.

An input over budget throws `RangeError` with a stable `SHEETOM_*_LIMIT`
prefix before changing CSSOM state. The check applies equally to
`replaceSync`, `insertRule`, `cssText` replacement, `setProperty`, selector and
condition mutation, and specialized descriptor mutation. Resource Budgets are
process-safety controls, not timeouts, sanitization, URL policy, or rendering
isolation.

## Declarations

`CSSStyleDeclaration` exposes live indexed names, named JavaScript properties,
`cssText`, `item`, `getPropertyValue`, `getPropertyPriority`, `setProperty`, and
`removeProperty`. Ordinary names are ASCII-case-insensitive; custom-property
names retain case and logical text.

`setProperty` performs WebIDL conversion first. `null` values remove, explicit
`undefined` becomes the string `"undefined"`, and a missing required argument
throws `TypeError`. Invalid priorities, unsupported names, invalid grammars,
and embedded priority tokens are atomic no-ops. Diagnostics are optional and
never change those return values or mutations.

For compilers that have several mutations for one block, SheetOM adds
`applyMutations(operations)`. It crosses the engine boundary once while
executing the same ordered declaration state transitions:

```ts
const results = rule.style.applyMutations([
  { kind: "set", property: "padding", value: "8px 16px" },
  { kind: "set", property: "width", value: "20px; color: red" },
  { kind: "remove", property: "padding-left" },
]);

// results[0] => { kind: "set", accepted: true, diagnostic: null }
// results[1] => { kind: "set", accepted: false, diagnostic: { ... } }
// results[2] => { kind: "remove", value: "16px" }
```

Results correspond by index. A set is `accepted: true` when `setProperty`
would apply it, including an accepted no-op or empty-value removal. Rejections
are atomic for that operation and return a diagnostic even when the sheet
diagnostic queue is disabled. When diagnostics are enabled, the same object is
also queued and can be drained once after the batch. Removes return the prior
observable value exactly like `removeProperty()`.

The batch itself is ordered, not globally atomic. A Resource Budget error
throws at the same operation as sequential calls and retains earlier commits.
The complete operation array is shape-checked before mutation and deliberately
requires string properties/priorities and string-or-null values rather than
performing arbitrary WebIDL coercion. Group operations by
`CSSStyleDeclaration`; do not concatenate them into CSS text, because that
would change duplicate, priority, invalid-recovery and removal semantics.

Parser support for modern or implementation-dependent grammar is described by
the release-versioned Value Capability Corpus. SheetOM uses checked-in
validators synchronously and never launches or contacts a browser at runtime.

Shorthands expose their expanded longhands through indexing. SheetOM retains
no parallel static shorthand declaration: getters and serialization reconstruct
one only from the complete current longhand state. SheetOM retains
pending-substitution provenance so recovered values such as
`72px var(--space, var(--space,` remain observable exactly as Chromium exposes
them. Mutating one longhand breaks shorthand reconstruction.

The supported-property manifest contains 129 multi-longhand shorthands. Every
one has a concrete Chromium breadth seed and mutation probe; the 24 private
codec profiles additionally have 96 reviewed grammar-branch cases. This is an
exact, release-gated compatibility baseline for the named cases, not a universal
promise that unmeasured future shorthand grammar is already supported. A value
accepted by the value gate but not expandable by its codec is an atomic no-op
with the optional `UNSUPPORTED_SHORTHAND_VALUE` diagnostic.

## Rules

- `CSSStyleRule` combines `style`, nested `cssRules`, and selector mutation.
- `CSSMediaRule`, `CSSSupportsRule`, `CSSContainerRule`, `CSSLayerBlockRule`,
  `CSSScopeRule`, and `CSSStartingStyleRule` expose live grouping lists.
- `CSSLayerStatementRule` exposes a fresh frozen `nameList` snapshot;
  `CSSNamespaceRule` exposes immutable `namespaceURI` and `prefix` values.
- `CSSImportRule` never fetches; `media` is live and `href` resolves against the
  sheet base URL.
- `CSSFontFaceRule`, `CSSPageRule`/`CSSMarginRule`, `CSSPositionTryRule`, and
  `CSSNestedDeclarations` expose declaration blocks.
- `CSSKeyframesRule` provides `appendRule`, `deleteRule`, and last-match
  `findRule`; each `CSSKeyframeRule` exposes mutable `keyText` and `style`.
- `CSSCounterStyleRule` validates mutable descriptors.
- `CSSFontFeatureValuesRule` exposes six live map-like categories with WebIDL
  `unsigned long` value conversion.
- `CSSFontPaletteValuesRule` exposes immutable `name`, `fontFamily`,
  `basePalette`, and `overrideColors` descriptors.
- `CSSViewTransitionRule` exposes immutable `navigation` and a frozen,
  same-object `types` list.
- `CSSPropertyRule` exposes immutable `name`, `syntax`, `inherits`, and
  `initialValue` descriptors.
- `CSSFunctionRule` exposes immutable `name` and `returnType`, returns fresh
  parameter records from `getParameters()`, and owns a live grouping list.
  Declaration runs are `CSSFunctionDeclarations` rules whose
  `CSSFunctionDescriptors` retain only custom properties and `result`.
  Conditional rules in the body recursively expose the same descriptor
  semantics. As in Chromium 151, assigning `result` through its named setter
  or `setProperty()` is a no-op; `cssText` replacement and `removeProperty()`
  remain live.

Unknown metadata and future rules can be retained and serialized, but are
read-only until a standards-defined mutable interface is implemented.

## Errors, diagnostics, and scope

Required WebIDL arguments throw `TypeError`. Parse and hierarchy failures use
`DOMException` names matching browser APIs. `takeDiagnostics()` drains
SheetOM-only mutation and serialization warnings when diagnostics were enabled;
otherwise it returns an empty array. `SheetOMDiagnostic.code` is the exported
`SheetOMDiagnosticCode` string union. Serialization recovery diagnostics expose
`conflictingLonghands`; mutation diagnostics retain their complete input string
until drained. Callers handling untrusted or unusually large inputs should
disable diagnostics or drain the queue promptly.

SheetOM does not implement DOM association, style-sheet collections, cascade,
selector matching, resolved/computed values, layout, fetching, or sanitizing.
See the release Compatibility Report for measured engine versions, WPT source,
known divergences, explicit exclusions, and backend-specific evidence. The
WebAssembly dimension includes direct and buffered loading, independent
instances, main-thread and worker execution, four bundlers in three browser
engines, a memory soak, and an absolute Publisher-shaped workload.

The broader evidence and browser-precedence policy are documented in
[Compatibility](./compatibility.md).
