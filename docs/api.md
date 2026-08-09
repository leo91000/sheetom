# API reference

The generated declaration reference is the exhaustive signature source. Run
`npm run docs:build` to render it under `site/api`. This page records the
behavioral contract that TypeScript declarations cannot express.

## Construction

`new CSSStyleSheet(options?)` is the only standards-defined constructible CSSOM
class. `options` accepts `baseURL`, `media`, `disabled`, and the SheetOM-only
`diagnostics` flag. `parseStyleSheet(cssText, options?)` is a SheetOM extension
for existing regular sheets; it preserves valid `@import` rules without loading
them and accepts `href` as source metadata.

All exported rule, rule-list, declaration, media-list, and feature-map classes
exist for `instanceof` and typing. Direct construction throws `TypeError`, as it
does in browsers. Obtain their objects from a sheet or parent rule.

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
Syntax end-of-input recovery preserved malformed source text.

| Surface | Contract |
| --- | --- |
| `CSSStyleDeclaration.cssText` | Browser-compatible observable declarations, including recovery quirks. |
| `CSSRule.cssText` | Browser-compatible observable text for the current rule. |
| `CSSStyleSheet.serialize()` | Deterministic Reparsable Stylesheet Serialization with repairs confined to their declaration or rule. |

“Reparsable” guarantees syntactic reparse, no declaration/rule leakage, and
preservation of SheetOM’s valid semantic state. It is not URL sanitization,
CSP enforcement, remote-resource control, or a guarantee that valid untrusted
CSS is safe to attach at the Rendering Boundary.

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

Shorthands expose their expanded longhands through indexing. SheetOM retains
no parallel static shorthand declaration: getters and serialization reconstruct
one only from the complete current longhand state. SheetOM retains
pending-substitution provenance so recovered values such as
`72px var(--space, var(--space,` remain observable exactly as Chromium exposes
them. Mutating one longhand breaks shorthand reconstruction.

## Rules

- `CSSStyleRule` combines `style`, nested `cssRules`, and selector mutation.
- `CSSMediaRule`, `CSSSupportsRule`, `CSSContainerRule`, `CSSLayerBlockRule`,
  `CSSScopeRule`, and `CSSStartingStyleRule` expose live grouping lists.
- `CSSImportRule` never fetches; `media` is live and `href` resolves against the
  sheet base URL.
- `CSSFontFaceRule`, `CSSPageRule`/`CSSMarginRule`, `CSSPositionTryRule`, and
  `CSSNestedDeclarations` expose declaration blocks.
- `CSSKeyframesRule` provides `appendRule`, `deleteRule`, and last-match
  `findRule`; each `CSSKeyframeRule` exposes mutable `keyText` and `style`.
- `CSSCounterStyleRule` validates mutable descriptors.
- `CSSFontFeatureValuesRule` exposes six live map-like categories.

Unknown metadata and future rules can be retained and serialized, but are
read-only until a standards-defined mutable interface is implemented.

## Errors, diagnostics, and scope

Required WebIDL arguments throw `TypeError`. Parse and hierarchy failures use
`DOMException` names matching browser APIs. `takeDiagnostics()` drains
SheetOM-only warnings when diagnostics were enabled; otherwise it returns an
empty array. `SheetOMDiagnostic.code` is the exported
`SheetOMDiagnosticCode` string union. The optional queue retains each complete
input string until it is drained; callers handling untrusted or unusually large
inputs should disable diagnostics or drain the queue promptly.

SheetOM does not implement DOM association, style-sheet collections, cascade,
selector matching, resolved/computed values, layout, fetching, or sanitizing.
See the release Compatibility Report for measured engine versions, WPT source,
known divergences, and explicit exclusions.
