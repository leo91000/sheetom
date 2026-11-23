# CSS Baseline audit for 23 November 2026

The cutoff is **2026-11-23**. The official npm registry reported **web-features
3.47.0**, source revision `a006a32ce337e249433fe5bb04d39c71f4c3fbe5`.
The archive matches registry integrity
`sha512-sD8j2ZQrev47vR5AGtT8gAJWk2+COnbTcopAI+UPo/+kXQClkPfhTWKnTlVOZhZnmH1bA5exPLhNeMtk6l3Ifg==`.
The extracted `data.json` has SHA-256
`8ee1119d41ebb7b1cd89cc6cd55fc10ec530c196fcc12c5bcf2049a138a0ed83`.
The audit started from SheetOM main `ee3feb81fb73f45721e8a16f58fb0af5cc5e2484`.
[WebDX source revision](https://github.com/web-platform-dx/web-features/commit/a006a32ce337e249433fe5bb04d39c71f4c3fbe5) ·
[npm archive](https://registry.npmjs.org/web-features/-/web-features-3.47.0.tgz) ·
[Browser Compatibility Data 8.1.10](https://github.com/mdn/browser-compat-data/tree/v8.1.10)

## Inventory

Compared with the [18 November audit](css-baseline-audit-2026-11-18.md), WebDX
adds 16 Widely Available keys and one Newly Available key. It promotes 24
already eligible keys to Widely Available and withdraws one. No eligible
family enters or leaves the target. Retained families keep their
specification links and eligibility dates. The snapshot contains 334
families, 4,511 keys, 4,031 eligible records and 480 deferred records.
The deferred records comprise 417 without Baseline availability and 63
with dates after the cutoff. The coverage map classifies 2,898 authoring records across 610 surfaces and
1,133 DOM or evaluation records.

| Family | Changed compatibility records | Disposition |
| --- | --- | --- |
| Background blending | Sixteen added `background-blend-mode` keywords: `color`, `color-burn`, `color-dodge`, `darken`, `difference`, `exclusion`, `hard-light`, `hue`, `lighten`, `luminosity`, `multiply`, `normal`, `overlay`, `saturation`, `screen`, `soft-light` | Existing authoring covers every keyword, comma-separated lists, priority and invalid list mutation. Layer blending is rendering. [Compositing and Blending](https://drafts.csswg.org/compositing-2/#background-blend-mode). |
| Container queries | `css.properties.container-type.weak_containment` is newly catalogued as low since 2025-03-31 | This record describes the layout containment effect of existing `container-type` values. Existing value grammar tests remain applicable. [CSS Conditional Rules](https://drafts.csswg.org/css-conditional-5/#container-type) · [BCD record](https://github.com/mdn/browser-compat-data/blob/v8.1.10/css/properties/container-type.json). |
| Font variants | `css.at-rules.font-feature-values.historical-forms` moves from high to undated and ineligible | WebDX retains Safari support and removes Chromium/Firefox support. The branch leaves the Baseline claim. [CSS Fonts](https://drafts.csswg.org/css-fonts-4/#font-variant-alternates-prop). |
| Scheme-dependent colors | `css.types.color.light-dark` moves to high | Existing color authoring preserves both branches, including nested colors. The image form is covered separately by the September contracts. Scheme selection is evaluation. [CSS Color](https://drafts.csswg.org/css-color-5/#light-dark). |
| Stepped math | `css.types.round`, `css.types.mod`, `css.types.rem` and each `.mixed_type_parameters` subkey move to high | Existing length-percentage authoring and new parser corrections for optional numeric steps and invalid mixed types. [CSS Values and Units](https://drafts.csswg.org/css-values-4/#round-func). |
| Custom states | `css.selectors.state`, `api.ElementInternals.states`, and all 11 `api.CustomStateSet` records move to high | Existing `:state()` authoring passes valid and invalid selector mutation. The DOM interface, state storage, and matching are outside the authoring contract. [HTML Standard](https://html.spec.whatwg.org/multipage/custom-elements.html#custom-state-pseudo-class). |
| Conditional imports | `css.at-rules.import.supports` and `api.CSSImportRule.supportsText` move to high | Existing conditional import support is corrected for quoted contents and following media functions. The live media list, read-only condition, invalid insertion and regular-sheet reparse have executable coverage. Import fetching and query matching are browser behavior. [CSS Cascade](https://drafts.csswg.org/css-cascade-5/#at-import) · [CSSOM](https://drafts.csswg.org/cssom/#the-cssimportrule-interface). |
| Text wrapping | `css.properties.text-wrap.balance` moves to high | Existing shorthand expansion and `text-wrap-style` state cover `balance` with and without `nowrap`. Line balancing is layout. [CSS Text](https://drafts.csswg.org/css-text-4/#valdef-text-wrap-style-balance). |
| Zoom | `css.properties.zoom` moves to high | Existing numeric, percentage, and `normal` authoring. Related stepped-math type validation also applies. Zoom effects are layout. [CSS Viewport](https://drafts.csswg.org/css-viewport/#zoom-property). |
| Overflow | `css.properties.overflow.auto` corrects desktop Firefox 1 to 3.5 | The date and high status remain unchanged. The `auto` shorthand and longhand state pass direct probes. [CSS Overflow](https://drafts.csswg.org/css-overflow-3/#propdef-overflow). |

WebDX renames 16 undated page-margin keys by dropping the `at_` prefix.
All stay ineligible. It updates browser versions for seven already deferred
media pseudo-classes. The 102 added `corner-shape` subkeys are undated and
ineligible. Six new undated families also remain outside the dated target:
`field-content`, `hyphenate-limit-lines`, `random-item`,
`text-decoration-inset`, `white-space-trim`, and `wrap-inside`. The
September authoring gap groups and pinned mixin/function experiments retain
their existing executable contracts. [September evidence](css-september-2026-parity-evidence.md)

## Runtime evidence

The published 0.4.3 native and WASM facades rejected number-valued
`round()` without a step and accepted incompatible number/percentage
operands in properties including `opacity`, `zoom`, and `line-height`.
They also exposed incorrect supports and media state for valid conditional
imports with a quoted parenthesis. Valid general-enclosed media functions
following `supports()` produced opaque imports.

The new declaration probes use mixed length-percentage arguments in
`width` and `line-height`. The BCD subkey description also mentions mixing
numbers and percentages. Chromium 151.0.7922.34, Firefox 153.0, and WebKit
26.5 rejected that combination in the tested contexts. CSS Values keeps
these types distinct. The three engines reject the newer draft
`round(line-width, 2.5px)` strategy, which has no separate eligible WebDX record.
They expose `calc(-1)` for
`round(-1.5)` and `calc(10)` for `round(up, 6, -5)`. Chromium keeps the
unresolved `round(up, sign(1em))` form. Firefox and WebKit fold that case to
`calc(1)`. [BCD stepped-math data](https://github.com/mdn/browser-compat-data/blob/v8.1.10/css/types/round.json) ·
[CSS Values type checking](https://drafts.csswg.org/css-values-4/#calc-type-checking)

Chromium and Firefox normalize declaration separators in `supportsText`
and preserve quoted parentheses. Chromium rejects an empty `supports()`
during insertion. Firefox treats it as a media function, and WebKit allows
an empty condition. The differential probe uses the pinned Chromium
insertion behavior. Existing lossless regular-sheet parsing retains
otherwise invalid source as opaque CSS. In a nested `light-dark()` case,
the browsers expose the system color as `canvas` and SheetOM exposes
`Canvas`. Both spellings identify the same CSS color.

[ADR 0195](adr/0195-preserve-conditional-imports-and-stepped-math.md) records
the parser and CSSOM decisions. The engine revision advances to
`lightningcss-1.33.0-c6a0c3ce-sheetom.85`. The Changeset classifies these
grammar and state corrections as a patch. The dated browser corpus and
shared native/WASM runner exercise the corrected behavior and serialization.


The local native and WASM builds each pass 36 shared contracts and depth
checks through 4,000. The browser comparison passes 713 probes, 12,544
support queries and 264 escape checks per backend. The two Webref runs
each cover 11,815 declaration cases across 667 properties with zero
mismatches. With Rust 1.88.0, wasm-bindgen 0.2.127, and the existing pinned
Binaryen profile, the WASM binary stays within the existing size budgets.
