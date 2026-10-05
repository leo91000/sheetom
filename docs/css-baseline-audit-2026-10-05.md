# CSS Baseline audit — 5 October 2026

The official npm registry's current `web-features` release is **3.40.1**, revision
`8606657a71535d688f5473579712717fe9442c0f`. The downloaded
[source archive](https://registry.npmjs.org/web-features/-/web-features-3.40.1.tgz)
matches npm's SHA-512 integrity. Its actual `data.json` bytes have SHA-256
`60e0e919d7fece714fa309a1f7c5a327f867bfda744789bdeac1355292895e4d`.
The cutoff is **2026-10-05**. [WebDX](https://github.com/web-platform-dx/web-features)
records availability; executable authoring evidence is assessed separately.

## Complete target delta

Relative to the [28 September audit](css-baseline-audit-2026-09-28.md), upstream
adds 24 eligible CSS keyword branches, three eligible SVG geometry API branches,
and one deferred rendering branch. No family is added or removed, no retained
branch changes status or dates, and no specification link or display name changes.
No eligible branch is withdrawn. The 331 families now contain 4,246 unique keys,
3,786 eligible records and 460 deferred records. Coverage maps 2,675 authoring
records to 598 surfaces and excludes 1,111 DOM/evaluation records. Deferred records
comprise 397 not-Baseline and 63 after-cutoff entries; there are no undated entries.
No absent date is assigned eligibility.

| Added compatibility branches | Disposition and executable evidence |
| --- | --- |
| `css.properties.background-position-x.center`, `.left`, `.right` | Already supported; Webref profile `webref-profile.f579430e89230528` and explicit `baseline-october.background-position-x.*` browser probes. |
| `css.properties.background-position-y.bottom`, `.center`, `.top` | Already supported; Webref profile `webref-profile.63d43cc47580bcc2` and explicit `baseline-october.background-position-y.*` browser probes. |
| `css.properties.clip-path.none` | Already supported; Webref profile `webref-profile.89b8de451b8a9014` and explicit `baseline-october.clip-path.none` browser probe. |
| `css.properties.font-size.large`, `.larger`, `.medium`, `.small`, `.smaller`, `.x-large`, `.x-small`, `.xx-large`, `.xx-small` | Already supported; Webref profile `webref-profile.09ac30b0f4b440a5` and explicit `baseline-october.font-size.*` browser probes. |
| `css.properties.pointer-events.all`, `.auto`, `.fill`, `.none`, `.painted`, `.stroke`, `.visible` | Already supported; Webref profile `webref-profile.8abb239490a6f000` and explicit `baseline-october.pointer-events.*` browser probes. |
| `css.properties.text-shadow.none` | Already supported; Webref profile `webref-profile.771e28ab72f516f6` and explicit `baseline-october.text-shadow.none` browser probe. |
| `api.SVGPathElement.getPointAtLength`, `.getTotalLength`, `.pathLength` | Outside scope: SVG DOM geometry and evaluation, not authored CSSOM state. |
| `css.properties.display.contents.focusable_elements` | Not Baseline and undated; focus/rendering behavior is also outside scope. Existing `display: contents` authoring evidence remains separate. |

All 24 CSS branches are Widely Available in this source, with availability dates
in 2015, 2016 or 2020. These are newly catalogued branches, not new runtime syntax.
The added declaration probes exercise important authored values, invalid-write
atomicity, rule attachment/detachment, public state, support queries and semantic
serialization/reparse on both facades against the pinned Chromium oracle.
Existing Webref profiles additionally check grammar, indexed declarations and
mutation state. A mapping or arbitrary-token acceptance is not used as proof.

Primary contracts are [background positions](https://drafts.csswg.org/css-backgrounds-3/#background-position),
[clip paths](https://drafts.csswg.org/css-masking-1/#the-clip-path),
[font sizes](https://drafts.csswg.org/css-fonts-4/#font-size-prop),
[pointer events](https://drafts.csswg.org/css-ui-4/#pointer-events-control),
[text shadows](https://drafts.csswg.org/css-text-decor-4/#text-shadow-property),
and [SVG geometry](https://svgwg.org/svg2-draft/types.html#InterfaceSVGGeometryElement).
Cascade, computed styles, selector matching, layout, DOM geometry and mixin
expansion remain outside the authoring contract. Pinned mixin/function experiments
and withdrawn media-state selector statuses remain separate from Baseline claims.
No previously unresolved in-scope authoring gap is reclassified or hidden.

## Delivery decision

This refresh updates source pins, the dated inventory, its evidence mapping and
executable probes. It requires no engine, public API or package behavior change,
so no Changeset or new release is warranted. Existing published **0.4.0** packages
are used for cold native/WASM validation, without workspace-linked artifacts.

## Validation

Source-backed target regeneration and the coverage generator reproduce the
checked-in files. Repository dependencies were installed with the pinned
npm 11.16.0 using `npm ci --include=dev`. Script typechecking, all 52 CI-script
tests, documentation validation and the diff whitespace check pass.

A fresh registry installation of `sheetom@0.4.0` and `@sheetom/wasm@0.4.0` passes
`check-installed-packages.ts` with the repository's unchanged test sources and
corpus. Both backends pass 25 shared modern CSS contracts and depth checks through
4,000. Chromium 151.0.7922.34, Firefox 153.0 and the existing alpha oracle Chromium
153.0.8010.12 compare 348 authoring probes, 12,009 support queries and 264 escape
checks per backend with zero mismatches. Native and WASM Webref ratchets each
check 667 properties, 370 profiles and 8,415 branches through 11,640 cases, with
zero acceptance, observable-state, declaration-text, indexed-item, atomicity or
reparse mismatches. Focused checks also reject invalid keywords atomically for
all 24 added branches on both installed backends.

The existing [stable 0.4.0 release](https://github.com/leo91000/sheetom/releases/tag/v0.4.0)
is non-draft and non-prerelease; its tag resolves to
`6cc1224b4cf010f199ed015fa7102c2173dd002f`. All 15 cohort packages match the release
manifest's versions and integrity, have `latest` set to 0.4.0 and expose attestation
metadata. `npm audit signatures` verifies registry signatures and attestations for
the three installed packages. This distinguishes cohort-wide metadata presence
from cryptographic verification of the installed platform subset.
