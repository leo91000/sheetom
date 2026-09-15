# CSS Baseline audit — 14 September 2026

The official npm registry's current `web-features` release is **3.38.0**,
revision `cbb306322da9bede139385e284de32577a48779c`. The downloaded
[source archive](https://registry.npmjs.org/web-features/-/web-features-3.38.0.tgz)
contains `data.json` with SHA-256
`a7e5aac0c7ac62329fbc59bf0d64393206054b9a7bd746688392a643fb0766be`.
The cutoff is **2026-09-14**. [WebDX](https://github.com/web-platform-dx/web-features)
provides availability data; executable authoring evidence remains a separate gate.

## Source changes and scope

Compared with the 9 September target (3.37.0), eight compatibility branches
advance from Newly Available (`low`) to Widely Available (`high`). All retain
their 2024-03-05 newly available date and were already eligible. There are no
added or removed families, compatibility keys, eligible branches, or specification
links. The target remains 328 families, 4,230 unique keys and 3,755 eligible
records. The 475 deferred records comprise 412 not-Baseline and 63 after-cutoff
records; none is silently assigned eligibility, and no previously eligible
branch has been withdrawn.

| Changed compatibility branch | Disposition and executable evidence |
| --- | --- |
| `css.properties.rx` | Existing authoring support; Webref profile `webref-profile.8669c8d8fce2bd0f`. |
| `css.properties.ry` | Existing authoring support; the same length/percentage/auto profile. |
| `css.properties.text-decoration-thickness.percentage` | Existing authoring support; percentage sample in `webref-profile.ea2fddfb0d70aaa7`. |
| `css.properties.text-wrap` | Existing authoring support; shorthand profile `webref-profile.b8ba2c5ea16ea1e6`. |
| `css.selectors.backdrop.inherit_from_originating_element` | Computed inheritance, outside scope. The separate `selector.backdrop` probe covers selector authoring. |
| `api.SVGFESpecularLightingElement.kernelUnitLengthX` | SVG DOM state, outside scope. |
| `api.SVGFESpecularLightingElement.kernelUnitLengthY` | SVG DOM state, outside scope. |
| `svg.elements.feSpecularLighting.kernelUnitLength` | SVG element attribute/rendering, outside scope. |

Primary grammar references are [SVG geometry](https://svgwg.org/svg2-draft/geometry.html#RxProperty),
[text-decoration thickness](https://drafts.csswg.org/css-text-decor-4/#propdef-text-decoration-thickness),
and [text-wrap](https://drafts.csswg.org/css-text-4/#text-wrap-shorthand).
[Backdrop styling](https://drafts.csswg.org/css-position-4/#backdrop) describes
rendered pseudo-elements; authoring its selector cannot demonstrate inheritance.
The evidence generator now explicitly excludes that inheritance key, correcting
its previous mapping to selector syntax. Coverage therefore has 2,647 authoring
records and 1,108 DOM/evaluation records across the same 603 authoring surfaces.

## Validation and remaining scope

Fresh npm installs of published `sheetom@0.2.0` and `@sheetom/wasm@0.2.0` passed
the repository's `check-installed-packages.ts` runner with unchanged test sources
and corpus. Both backends passed 21 shared modern CSS contracts, including the
four September gap groups, and depth checks through 4,000. The pinned browser
authoring comparisons and native/WASM Webref ratchets passed with zero mismatches.
The Webref runner checks acceptance, observable state, declaration text, indexed
items, invalid-write atomicity and reparsing.

A fresh 12-case comparison with Chromium 151.0.7922.34 also verified both
backends on `rx`/`ry` (`auto`, `1px`, `10%`, `calc(1px + 5%)`), percentage
text-decoration thickness, and `text-wrap` (`wrap`, `nowrap`, `balance`). Each
case checked support, accepted declaration state, rejection of an invalid write
without state change, and serialization/reparse. This supplements the maintained
profiles; it does not claim exhaustive grammar or rendering conformance.

Both generated inventories reproduce from the downloaded data, and all 51 CI
script tests pass. No runtime change or new Changeset is warranted by this audit.
The existing release PR is reused for the already pending package updates.

The four gaps from [the historical September audit](css-baseline-gap-audit-2026-09-09.md)
were implemented in [PR #219](https://github.com/leo91000/sheetom/pull/219) and
are covered by [the September contracts](css-september-2026-parity-evidence.md).
No newly missing in-scope authoring feature was found. Cascade, computed styles,
selector matching, layout, SVG DOM APIs, and mixin expansion remain outside the
contract; pinned mixin/function experiments remain separate from Baseline.
