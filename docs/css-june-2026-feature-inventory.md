# CSS through June 2026: feature selection inventory

Historical June snapshot. The [current September target](css-september-2026-parity-evidence.md) advances the cutoff and corrects individual-branch selection.

This is design evidence for the accepted cumulative June 30, 2026 Baseline cutoff. It inventories candidate feature families; it does not certify SheetOM support, completed branch coverage, or passing tests. The separately accepted experimental CSS mixin family is additive and available by default, with its own revision-pinned draft contract.

## Frozen source

- Dataset: the W3C WebDX Community Group maintained [web-features](https://github.com/web-platform-dx/web-features), npm package **web-features@3.37.0**.
- Package gitHead: `60e8982caeaa28c4913f45c2cb20b5f6114de510`.
- Immutable version URL: [https://registry.npmjs.org/web-features/-/web-features-3.37.0.tgz](https://registry.npmjs.org/web-features/-/web-features-3.37.0.tgz).
- Registry SHA-512 integrity: `sha512-47x5zchtpsvvH/EEQ0h3d3E9kZyFxFyk6LkmYHXDZjLF0RAa3OV2s8dMRqphp/PTxxrd/iZwz37dte6xWgV7mQ==`; verified against the downloaded archive.
- SHA-256 of the archive member `package/data.json`: `7633fe15d2ac393c69682150cd2e20b1164f4b8396fd9ecbc8226e25777dcac0`.
- Retrieved September 7, 2026. This is a retrospective date filter against this frozen release, not a claim that the dataset was published on June 30.
- Data model and Baseline policy: [package documentation at the source revision](https://github.com/web-platform-dx/web-features/blob/60e8982caeaa28c4913f45c2cb20b5f6114de510/README.md), [Baseline definition](https://github.com/web-platform-dx/web-features/blob/60e8982caeaa28c4913f45c2cb20b5f6114de510/docs/baseline.md).

## Deterministic selection

Use canonical `kind: feature` records only. A CSS candidate satisfies at least one of these tests: its group is `css` or descends transitively from that group; a `compat_features` key begins with `css.`; or a compatibility key matches `^api\.(CSS|StyleSheet|MediaList)`. Retain candidates whose aggregate status is `low` or `high` and whose `baseline_low_date` is at most `2026-06-30`. Earlier years remain included. Sort by feature ID.

```js
const cutoff = "2026-06-30";
const inCssGroup = (id) => id === "css" ||
  Boolean(data.groups[id]?.parent && inCssGroup(data.groups[id].parent));
const selected = Object.entries(data.features).filter(([, feature]) =>
  feature.kind === "feature" &&
  ((feature.group ?? []).some(inCssGroup) ||
   (feature.compat_features ?? []).some((key) =>
     key.startsWith("css.") || /^api\.(CSS|StyleSheet|MediaList)/.test(key))) &&
  ["low", "high"].includes(feature.status?.baseline) &&
  typeof feature.status.baseline_low_date === "string" &&
  feature.status.baseline_low_date <= cutoff
);
```

**Result: 311 candidate feature families**, including 18 first reaching Baseline in January–June 2026. Group ancestry plus CSS keys selects 310; the CSS API extension adds `constructed-stylesheets`. Group ancestry alone omits 53 selected CSS-key families.

## Classification and completeness caveats

- These are feature families, not grammar branches or conformance tests. A family can combine HTML, JavaScript, CSS parsing, and rendering. For example, `popover`, `svg`, `mathml`, `webvtt`, and `autonomous-custom-elements` enter through CSS keys; only their authoring CSS surfaces are in SheetOM scope. Record exclusions per surface rather than dropping the entire family.
- Do not filter only by current `baseline: low`; that would drop older widely available features. Do not select only CSS groups: masking, scrolling, registered properties, and other CSS surfaces are grouped elsewhere.
- **364 compatibility-key records inside selected families do not themselves satisfy the cutoff**: they are unavailable, undated, or first available after June 30. The aggregate family date therefore cannot authorize every present or future subfeature. Freeze a branch-level manifest using each key's own dated status, retain later keys as explicitly classified additions or exclusions, and preserve support SheetOM already provides. Examples include `css.at-rules.keyframes.named_range_keyframes`, `css.properties.animation-duration.auto`, and constructed stylesheet `baseURL`.
- A specification-link cross-check additionally finds ten CSS-related DOM/rendering-only families outside the selection: `check-visibility`, `document-caretpositionfrompoint`, `element-from-point`, `font-loading`, `matchmedia`, `screen`, `scroll-elements`, `scroll-into-view`, `scrollend`, and `visual-viewport`. Their recorded APIs require DOM association, measurements, events, rendering, or fetching and remain outside Authoring CSSOM. They are accounted for here rather than silently omitted.
- The dataset is a discovery source, not the runtime grammar authority. Applicable WPT, revision-pinned specifications, browser observations, and reviewed grammar branches still determine the implementation contract. The candidate heuristic is reproducible, but an inventory review must classify all keys and cross-check CSS specifications/Webref to catch catalog omissions.
- `mixin` and `function` have `baseline: false` in this package. Mixins are an explicit experimental addition; existing custom-function support must remain. `progress-function` first reaches Baseline on September 1, 2026 and is outside this cutoff unless separately selected. A date cutoff must never remove existing accepted syntax.

## Source-grounded implementation follow-up

- The confirmed missing specialized family is `@mixin` / `@apply` / `@contents`: first-party source has no corresponding public classes or dedicated tests. Unknown-rule retention is not equivalent to structured editable CSSOM. See [rule fallback](../crates/sheetom-core/src/rules.rs) and the existing [custom-function family](../tests/function-rules.test.ts).
- Several 2026 candidates already have source support: `contrast-color()` in [semantic values](../crates/sheetom-core/src/semantic_value.rs), `field-sizing` in [browser longhands](../crates/sheetom-core/src/browser_longhand.rs), container style conditions in [grouping tests](../tests/grouping-rules.test.ts), `:open` and `:active-view-transition` in [vendored selectors](../vendor/lightningcss/src/selector.rs), and `text-indent` hanging/each-line in [vendored text grammar](../vendor/lightningcss/src/properties/text.rs). These source hits are not full parity proof.
- The broad unresolved gap is **traceability**, not a measured count of runtime failures: no reviewed mapping from these 311 families and eligible compatibility keys to native/WASM public-interface evidence was established in this bounded review. Build that matrix before marking a family complete; generic retention cannot satisfy an editable authoring surface.
- Completion requires zero known unimplemented selected authoring surfaces, explained oracle differences, explicit non-authoring exclusions, and passing native plus WASM evidence. No tests were executed for this inventory.

## Selected families

`group` means transitive CSS grouping; `css-key` means a CSS compatibility key; `css-api` means the API prefix cross-check. The date is the family's first Baseline Newly available date, even if it is now widely available. Every row awaits branch-level implementation/evidence disposition.

| Feature ID | Name | First Baseline date | Selection evidence |
| --- | --- | --- | --- |
| `abs-sign` | abs() and sign() | 2025-06-26 | group, css-key |
| `absolute-positioning` | Absolute positioning | 2015-07-29 | group, css-key |
| `active-view-transition` | Active view transition | 2026-01-13 | group, css-key |
| `align-content-block` | align-content in block layouts | 2024-04-16 | group, css-key |
| `all` | all | 2020-01-15 | group, css-key |
| `alt-text-generated-content` | Alt text for generated content | 2024-07-09 | group, css-key |
| `animation-composition` | animation-composition | 2023-07-04 | css-key |
| `animations-css` | Animations (CSS) | 2015-09-30 | group, css-key, css-api |
| `appearance` | appearance | 2022-03-14 | group, css-key |
| `aspect-ratio` | aspect-ratio | 2021-09-20 | group, css-key |
| `attr-contents` | attr() (content only) | 2015-07-29 | group, css-key |
| `autonomous-custom-elements` | Autonomous custom elements | 2020-01-15 | css-key |
| `backdrop` | ::backdrop | 2022-03-14 | group, css-key |
| `backdrop-filter` | backdrop-filter | 2024-09-16 | group, css-key |
| `background` | background | 2015-07-29 | group, css-key |
| `background-blend-mode` | background-blend-mode | 2020-01-15 | group, css-key |
| `background-clip` | background-clip | 2015-07-29 | group, css-key |
| `background-color` | background-color | 2015-07-29 | group, css-key |
| `background-image` | background-image | 2015-07-29 | group, css-key |
| `background-origin` | background-origin | 2015-07-29 | group, css-key |
| `background-position` | background-position | 2015-07-29 | group, css-key |
| `background-repeat` | background-repeat | 2016-09-20 | group, css-key |
| `background-size` | background-size | 2015-07-29 | group, css-key |
| `baseline-shift` | baseline-shift | 2026-03-24 | group, css-key |
| `before-after` | ::before and ::after | 2015-07-29 | css-key |
| `border-image` | Border images | 2017-02-01 | group, css-key |
| `border-radius` | border-radius | 2015-07-29 | group, css-key |
| `borders` | Borders | 2015-07-29 | group, css-key |
| `box-shadow` | box-shadow | 2015-07-29 | group, css-key |
| `box-sizing` | box-sizing | 2015-07-29 | group, css-key |
| `calc` | calc() | 2015-07-29 | group, css-key |
| `calc-constants` | calc() keywords | 2023-06-06 | group, css-key |
| `cap` | cap unit | 2023-12-11 | group, css-key |
| `caret-color` | caret-color | 2020-01-15 | group, css-key |
| `cascade-layers` | Cascade layers | 2022-03-14 | group, css-key, css-api |
| `case-insensitive-attributes` | Case-insensitive attribute selector | 2020-01-15 | group, css-key |
| `ch` | ch unit | 2015-07-29 | group, css-key |
| `charset` | @charset | 2015-07-29 | group, css-key |
| `clip-path` | clip-path | 2021-01-21 | css-key |
| `clip-path-animatable` | Animatable clipping paths | 2020-01-15 | css-key |
| `clip-path-boxes` | Clip path boxes | 2023-11-02 | css-key |
| `color` | Color | 2015-07-29 | group, css-key |
| `color-function` | color() | 2023-05-09 | group, css-key |
| `color-gamut` | color-gamut media query | 2023-02-14 | group, css-key |
| `color-mix` | color-mix() | 2023-05-09 | group, css-key |
| `color-scheme` | color-scheme | 2022-02-03 | group, css-key |
| `colrv0` | COLRv0 | 2020-01-15 | group, css-key |
| `column-fill` | column-fill | 2017-03-07 | group, css-key |
| `column-span` | column-span | 2020-07-28 | group, css-key |
| `conic-gradients` | Conic gradients | 2020-11-17 | group, css-key |
| `constructed-stylesheets` | Constructed stylesheets | 2023-03-27 | css-api |
| `contain` | contain | 2022-03-14 | group, css-key |
| `contain-inline-size` | Inline-size containment | 2022-09-12 | group, css-key |
| `contain-intrinsic-size` | contain-intrinsic-size | 2023-09-18 | group, css-key |
| `contain-layout` | Layout containment | 2022-03-14 | group, css-key |
| `contain-paint` | Paint containment | 2022-03-14 | group, css-key |
| `contain-size` | Size containment | 2022-03-14 | group, css-key |
| `contain-style` | Style containment | 2022-07-26 | group, css-key |
| `container-queries` | Container queries | 2023-02-14 | group, css-key, css-api |
| `container-style-queries` | Container style queries | 2026-05-19 | group, css-key |
| `content` | Content | 2015-07-29 | group, css-key |
| `content-visibility` | content-visibility | 2025-09-15 | group, css-key |
| `contrast-color` | contrast-color() | 2026-04-10 | group, css-key |
| `counter-set` | counter-set | 2023-12-11 | group, css-key |
| `counter-style` | @counter-style | 2023-09-18 | group, css-key, css-api |
| `counters` | Counters (CSS) | 2015-07-29 | group, css-key |
| `crisp-edges` | crisp-edges | 2026-05-07 | css-key |
| `css-escape` | CSS.escape() | 2020-01-15 | group, css-api |
| `css-object-model` | CSS object model | 2015-09-30 | group, css-api |
| `css-supports` | CSS.supports() | 2020-01-15 | group, css-api |
| `cubic-bezier-easing` | cubic-bezier() easing | 2015-07-29 | group, css-key |
| `currentcolor` | currentColor | 2015-07-29 | group, css-key |
| `custom-properties` | Custom properties | 2017-04-05 | group, css-key |
| `default` | :default | 2020-01-15 | group, css-key |
| `details-content` | ::details-content | 2025-09-16 | group, css-key |
| `dir-pseudo` | :dir() | 2023-12-07 | group, css-key |
| `display` | Display | 2015-07-29 | group, css-key |
| `display-flow-root` | display: flow-root | 2020-01-15 | group, css-key |
| `display-list-item` | display: list-item | 2015-07-29 | group, css-key |
| `display-table` | display: table | 2015-07-29 | group, css-key |
| `dominant-baseline` | dominant-baseline | 2020-01-15 | group, css-key |
| `dynamic-range` | dynamic-range media query | 2022-05-03 | group, css-key |
| `em-unit` | em unit | 2015-07-29 | group, css-key |
| `empty` | :empty | 2015-07-29 | group, css-key |
| `ex` | ex unit | 2015-07-29 | group, css-key |
| `exp-functions` | Exponential functions (CSS) | 2023-12-07 | group, css-key |
| `field-sizing` | field-sizing | 2026-06-16 | group, css-key |
| `file-selector-button` | ::file-selector-button | 2021-04-26 | group, css-key |
| `filter` | filter | 2016-09-07 | group, css-key |
| `first-letter` | ::first-letter | 2015-07-29 | group, css-key |
| `first-line` | ::first-line | 2015-07-29 | group, css-key |
| `fit-content` | fit-content | 2021-11-02 | group, css-key |
| `fixed-positioning` | Fixed positioning | 2015-07-29 | group, css-key |
| `flexbox` | Flexbox | 2015-09-30 | group, css-key |
| `flexbox-gap` | Flexbox gap | 2021-04-26 | group, css-key |
| `float-clear` | float and clear | 2015-07-29 | group, css-key |
| `focus-visible` | :focus-visible | 2022-03-14 | group, css-key |
| `focus-within` | :focus-within | 2020-01-15 | group, css-key |
| `font-display` | font-display | 2020-01-15 | group, css-key |
| `font-face` | @font-face | 2016-09-20 | group, css-key, css-api |
| `font-family` | font-family | 2015-07-29 | group, css-key |
| `font-family-math` | Math font family | 2026-03-24 | group, css-key |
| `font-family-system` | System font | 2021-09-07 | css-key |
| `font-feature-settings` | font-feature-settings | 2017-04-05 | group, css-key |
| `font-kerning` | font-kerning | 2020-01-15 | group, css-key |
| `font-optical-sizing` | font-optical-sizing | 2020-03-24 | group, css-key |
| `font-palette` | font-palette | 2022-11-15 | group, css-key, css-api |
| `font-shorthand` | Font shorthand | 2015-07-29 | group, css-key |
| `font-size` | font-size | 2015-07-29 | group, css-key |
| `font-size-adjust` | font-size-adjust | 2024-07-25 | group, css-key |
| `font-stretch` | font-stretch | 2020-01-15 | group, css-key |
| `font-style` | font-style | 2015-07-29 | group, css-key |
| `font-synthesis` | font-synthesis | 2022-01-06 | group, css-key |
| `font-synthesis-small-caps` | font-synthesis-small-caps | 2023-03-27 | group, css-key |
| `font-synthesis-style` | font-synthesis-style | 2023-03-27 | group, css-key |
| `font-synthesis-weight` | font-synthesis-weight | 2023-03-27 | group, css-key |
| `font-variant` | font-variant | 2015-07-29 | group, css-key |
| `font-variant-alternates` | font-variant-alternates | 2023-03-13 | group, css-key, css-api |
| `font-variant-caps` | font-variant-caps | 2020-01-15 | group, css-key |
| `font-variant-east-asian` | font-variant-east-asian | 2020-01-15 | group, css-key |
| `font-variant-ligatures` | font-variant-ligatures | 2020-01-15 | group, css-key |
| `font-variant-numeric` | font-variant-numeric | 2020-01-15 | group, css-key |
| `font-variation-settings` | font-variation-settings | 2018-09-05 | group, css-key |
| `font-weight` | font-weight | 2015-07-29 | group, css-key |
| `forced-colors` | Forced colors | 2022-09-12 | group, css-key |
| `form-validity-pseudos` | Form validity pseudo-classes | 2015-07-29 | group, css-key |
| `get-computed-style` | getComputedStyle() | 2015-07-29 | group |
| `gradient-interpolation` | Gradient interpolation | 2024-06-11 | group, css-key |
| `gradients` | Gradients | 2015-07-29 | group, css-key |
| `grid` | Grid | 2017-10-17 | group, css-key |
| `grid-animation` | Grid animation | 2022-10-27 | group, css-key |
| `has` | :has() | 2023-12-19 | group, css-key |
| `highlight` | Custom highlights | 2026-03-24 | css-key, css-api |
| `host` | Host | 2020-01-15 | group, css-key |
| `hsl` | HSL | 2020-01-15 | group, css-key |
| `hwb` | HWB | 2022-04-28 | group, css-key |
| `hyphenate-character` | Hyphenate character | 2023-09-18 | group, css-key |
| `hyphens` | Hyphenation | 2023-09-18 | group, css-key |
| `ic` | ic unit | 2022-10-03 | group, css-key |
| `image-orientation` | image-orientation | 2020-04-13 | css-key |
| `image-rendering` | image-rendering | 2021-10-05 | css-key |
| `image-set` | image-set() | 2023-09-18 | group, css-key |
| `import` | @import | 2015-07-29 | group, css-key, css-api |
| `indeterminate` | :indeterminate | 2020-01-15 | css-key |
| `individual-transforms` | Individual transform properties | 2022-08-05 | group, css-key |
| `inherit-value` | inherit | 2015-07-29 | group, css-key |
| `initial-value` | initial | 2015-11-12 | group, css-key |
| `input-selectors` | Input selectors | 2015-07-29 | group, css-key |
| `interaction` | Interaction media queries | 2018-12-11 | group, css-key |
| `is` | :is() | 2021-01-21 | group, css-key |
| `isolation` | isolation | 2020-01-15 | group, css-key |
| `lab` | Lab and LCH | 2023-05-09 | group, css-key |
| `lang` | :lang() | 2015-07-29 | group, css-key |
| `layout-direction-override` | Layout direction override | 2020-01-15 | group, css-key |
| `letter-spacing` | letter-spacing | 2015-07-29 | group, css-key |
| `lh` | lh unit | 2023-11-21 | group, css-key |
| `light-dark` | light-dark() | 2024-05-13 | group, css-key |
| `line-break` | line-break | 2020-07-28 | group, css-key |
| `line-height` | line-height | 2015-07-29 | group, css-key |
| `linear-easing` | linear() easing | 2023-12-11 | group, css-key |
| `link-selectors` | Link selectors | 2020-01-15 | group, css-key |
| `list-elements` | <ol>, <ul>, and <li> | 2015-07-29 | group |
| `list-style` | List style | 2015-07-29 | group, css-key |
| `logical-properties` | Logical properties | 2021-09-20 | group, css-key |
| `margin` | margin | 2015-07-29 | group, css-key |
| `mask-type` | mask-type | 2020-01-15 | css-key |
| `masks` | Masks | 2023-12-07 | css-key |
| `mathml` | MathML | 2023-01-12 | css-key |
| `media-queries` | Media queries | 2015-07-29 | group, css-key |
| `media-query-range-syntax` | Media query range syntax | 2023-03-27 | group, css-key |
| `min-max-clamp` | min(), max(), and clamp() | 2020-07-28 | group, css-key |
| `min-max-content` | min-content and max-content | 2020-01-15 | group, css-key |
| `min-max-width-height` | Min and max width and height | 2015-07-29 | css-key |
| `mix-blend-mode` | mix-blend-mode | 2020-01-15 | group, css-key |
| `modal` | :modal | 2022-09-02 | group, css-key |
| `motion-path` | Motion path | 2022-09-12 | group, css-key |
| `multi-column` | Multi-column layout | 2017-03-07 | group, css-key |
| `named-color` | Named colors | 2015-07-29 | group, css-key |
| `namespace` | @namespace | 2015-07-29 | group, css-key, css-api |
| `nesting` | Nesting | 2023-12-11 | group, css-key, css-api |
| `not` | :not() | 2021-01-21 | group, css-key |
| `nth-child` | :nth-child() | 2015-07-29 | group, css-key |
| `nth-child-of` | :nth-child() of <selector> | 2023-05-09 | group, css-key |
| `nth-of-type` | :nth-of-type() pseudo-classes | 2015-07-29 | group, css-key |
| `object-fit` | object-fit | 2020-01-15 | css-key |
| `object-position` | object-position | 2020-01-15 | css-key |
| `oklab` | Oklab and OkLCh | 2023-05-09 | group, css-key |
| `opacity` | opacity | 2015-07-29 | group, css-key |
| `open-pseudo` | :open | 2026-05-11 | group, css-key |
| `outline` | outline | 2023-03-27 | css-key |
| `outlines` | Outlines | 2017-04-05 | css-key |
| `overflow` | Overflow media queries | 2023-09-18 | group, css-key |
| `overflow-clip` | overflow: clip | 2022-09-12 | group, css-key |
| `overflow-shorthand` | overflow | 2020-03-24 | group, css-key |
| `overflow-wrap` | overflow-wrap | 2018-10-02 | group, css-key |
| `padding` | padding | 2015-07-29 | group, css-key |
| `page-breaks` | Page breaks | 2019-01-29 | group, css-key |
| `page-setup` | Page setup | 2024-12-11 | group, css-key |
| `paint-order` | paint-order | 2024-03-22 | group, css-key |
| `physical-properties` | Physical properties | 2015-07-29 | group, css-key |
| `placeholder` | ::placeholder | 2020-01-15 | group, css-key |
| `placeholder-shown` | :placeholder-shown | 2020-01-15 | group, css-key |
| `pointer-events` | pointer-events | 2015-07-29 | group, css-key |
| `popover` | Popover | 2025-01-27 | css-key |
| `position` | Position | 2015-07-29 | group, css-key |
| `prefers-color-scheme` | prefers-color-scheme media query | 2020-01-15 | group, css-key |
| `prefers-contrast` | prefers-contrast media query | 2022-05-31 | group, css-key |
| `prefers-reduced-motion` | prefers-reduced-motion media query | 2020-01-15 | group, css-key |
| `print-color-adjust` | print-color-adjust | 2025-05-01 | css-key |
| `q-unit` | Q unit | 2020-03-24 | group, css-key |
| `quotes` | Quotes | 2021-04-26 | group, css-key |
| `rcap` | rcap unit | 2026-01-13 | group, css-key |
| `rch` | rch unit | 2026-01-13 | group, css-key |
| `read-write-pseudos` | :read-only and :read-write | 2020-07-28 | css-key |
| `rect-xywh` | rect() and xywh() | 2024-01-23 | css-key |
| `registered-custom-properties` | Registered custom properties | 2024-07-09 | css-key, css-api |
| `relative-color` | Relative colors | 2024-09-16 | group, css-key |
| `relative-positioning` | Relative positioning | 2015-07-29 | group, css-key |
| `rem` | rem | 2015-07-29 | group, css-key |
| `resolution` | resolution media query | 2022-09-12 | group, css-key |
| `resolution-compat` | resolution media query (compatibility prefixes) | 2018-10-23 | group, css-key |
| `revert-value` | revert | 2020-07-27 | group, css-key |
| `rex` | rex unit | 2026-01-13 | group, css-key |
| `rgb` | RGB | 2020-01-15 | group, css-key |
| `ric` | ric unit | 2026-01-13 | group, css-key |
| `rlh` | rlh unit | 2023-11-21 | group, css-key |
| `root` | :root | 2015-07-29 | group, css-key |
| `round-mod-rem` | round(), mod(), and rem() | 2024-05-17 | group, css-key |
| `ruby-align` | ruby-align | 2024-12-11 | css-key |
| `ruby-position` | ruby-position | 2024-12-11 | css-key |
| `safe-area-inset` | Safe area inset environment variables | 2020-01-15 | group, css-key |
| `scope` | @scope | 2026-03-24 | group, css-key, css-api |
| `scope-pseudo` | :scope (pseudo-class) | 2020-01-15 | group, css-key |
| `scripting` | scripting media query | 2023-12-07 | group, css-key |
| `scroll-behavior` | scroll-behavior | 2022-03-14 | css-key |
| `scroll-snap` | Scroll snap | 2020-01-15 | css-key |
| `scrollbar-color` | scrollbar-color | 2025-12-12 | css-key |
| `scrollbar-gutter` | scrollbar-gutter | 2024-12-11 | css-key |
| `scrollbar-width` | scrollbar-width | 2024-12-11 | css-key |
| `selectors` | Selectors (core) | 2015-07-29 | group, css-key |
| `shadow-parts` | Shadow parts | 2020-07-28 | group, css-key |
| `shape-function` | shape() | 2026-02-24 | css-key |
| `shape-outside` | shape-outside | 2020-01-15 | css-key |
| `shapes` | shapes | 2020-01-15 | css-key |
| `slot` | <slot> | 2020-01-15 | css-key |
| `starting-style` | @starting-style | 2024-08-06 | group, css-key, css-api |
| `state` | :state() | 2024-05-17 | css-key |
| `static-positioning` | Static positioning | 2015-07-29 | group, css-key |
| `steps-easing` | steps() easing | 2020-09-16 | group, css-key |
| `sticky-positioning` | Sticky positioning | 2019-09-19 | group, css-key |
| `style-attr` | style (attribute) | 2015-07-29 | group |
| `subgrid` | Subgrid | 2023-09-15 | group, css-key |
| `supports` | @supports | 2015-09-30 | group, css-key, css-api |
| `supports-compat` | @supports (compatibility prefix) | 2016-09-20 | group, css-key |
| `svg` | SVG | 2020-01-15 | css-key |
| `svg-filters` | SVG filters | 2015-07-29 | css-key |
| `system-color` | System colors | 2015-07-29 | group, css-key |
| `tab-size` | tab-size | 2021-08-10 | group, css-key |
| `table` | Tables | 2015-07-29 | css-key |
| `target` | :target | 2015-07-29 | group, css-key |
| `target-text` | ::target-text | 2024-12-11 | css-key |
| `text-align` | text-align | 2015-07-29 | group, css-key |
| `text-align-last` | text-align-last | 2022-09-12 | group, css-key |
| `text-combine-upright` | text-combine-upright | 2022-03-14 | group, css-key |
| `text-decoration` | text-decoration | 2015-07-29 | css-key |
| `text-decoration-skip-ink` | text-decoration-skip-ink | 2022-03-14 | css-key |
| `text-decoration-skip-ink-all` | text-decoration-skip-ink: all | 2026-05-07 | css-key |
| `text-decoration-spelling-grammar` | Spelling and grammar text decorations | 2025-12-12 | css-key |
| `text-emphasis` | text-emphasis | 2022-03-03 | css-key |
| `text-indent` | text-indent | 2015-07-29 | group, css-key |
| `text-indent-each-line` | text-indent: each-line | 2026-03-13 | group, css-key |
| `text-indent-hanging` | text-indent: hanging | 2026-03-13 | group, css-key |
| `text-orientation` | text-orientation | 2020-09-16 | group, css-key |
| `text-overflow` | Text overflow | 2015-07-29 | group, css-key |
| `text-shadow` | text-shadow | 2015-07-29 | group, css-key |
| `text-stroke-fill` | Text stroke and fill  (compatibility prefixes) | 2017-04-05 | group, css-key |
| `text-transform` | text-transform | 2015-07-29 | group, css-key |
| `text-underline-offset` | text-underline-offset | 2020-11-19 | group, css-key |
| `text-underline-position` | text-underline-position | 2020-07-28 | group, css-key |
| `text-wrap` | text-wrap | 2024-10-17 | group, css-key |
| `text-wrap-balance` | text-wrap: balance | 2024-05-13 | group, css-key |
| `touch-action` | touch-action | 2019-09-19 | css-key |
| `transform-box` | transform-box | 2024-04-16 | css-key |
| `transforms2d` | 2D transforms | 2015-09-30 | group, css-key |
| `transforms3d` | 3D transforms | 2022-03-14 | group, css-key |
| `transition-behavior` | transition-behavior | 2024-08-06 | group, css-key |
| `transitions` | Transitions (CSS) | 2015-09-30 | group, css-key, css-api |
| `trig-functions` | Trigonometric functions (CSS) | 2023-03-13 | group, css-key |
| `two-value-display` | Two-value display property | 2023-07-21 | group, css-key |
| `unset-value` | unset | 2016-03-21 | group, css-key |
| `update` | Update frequency media query | 2023-09-18 | group, css-key |
| `user-action-pseudos` | User action pseudo-classes | 2015-07-29 | group, css-key |
| `user-pseudos` | :user-valid and :user-invalid | 2023-11-02 | group, css-key |
| `vertical-align` | vertical-align | 2015-07-29 | group, css-key |
| `vertical-form-controls` | Vertical form controls | 2024-04-18 | group, css-key |
| `view-transition-class` | view-transition-class | 2025-10-14 | css-key |
| `view-transitions` | View transitions | 2025-10-14 | css-key |
| `viewport-unit-variants` | Small, large, and dynamic viewport units | 2022-12-05 | css-key |
| `viewport-units` | Viewport units | 2017-10-17 | group, css-key |
| `visibility` | visibility | 2015-07-29 | group, css-key |
| `webvtt` | WebVTT | 2015-07-29 | css-key |
| `where` | :where() | 2021-01-21 | group, css-key |
| `white-space` | white-space | 2015-07-29 | group, css-key |
| `white-space-collapse` | white-space-collapse | 2024-03-19 | group, css-key |
| `width-height` | Width and height | 2015-07-29 | group, css-key |
| `will-change` | will-change | 2020-01-15 | group, css-key |
| `word-break` | word-break | 2015-09-30 | group, css-key |
| `word-spacing` | word-spacing | 2015-07-29 | group, css-key |
| `writing-mode` | writing-mode | 2017-03-27 | group, css-key |
| `z-index` | z-index | 2015-07-29 | group, css-key |
| `zoom` | zoom | 2024-05-14 | css-key |
