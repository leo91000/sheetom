# ADR 0154: Own modern clip paths and cursor URL sets

## Status

Accepted for RC6.

## Context

The pinned Chromium/Webref cross-product exposed five rejected branches: modern `path()` forms for `clip-path` and `-webkit-clip-path`, plus a `cursor` whose source is an `image-set()`. Lightning CSS 1.33 parses the older clip shapes but does not cover the newer `path()`, `rect()`, `xywh()`, and `shape()` branches consistently. Its cursor grammar accepts only a single URL even though Chromium accepts `<url-set>` sources with resolutions, file types, and hotspots.

An exact-literal override would leave geometry-box ordering, path command recovery, invalid image-set members, and neighboring hotspot syntax unvalidated. It would also recreate the native AST round-trip boundary that caused the RC5 `image-set()` process abort.

## Decision

- Parse modern clip-path shapes into SheetOM's semantic geometric model, using the vendored `cssparser` token stream and SVG path parser.
- Accept every Chromium geometry box except `half-border-box`, allow the box before or after the shape, serialize the shape first, and omit the default `border-box`.
- Patch the vendored Lightning CSS cursor AST so a cursor image source is either a URL or an `image-set()` containing URL options only.
- Reject negative resolutions, non-URL image-set members, incomplete hotspots, and missing source-list commas atomically.
- Canonicalize legacy `-webkit-image-set()` to the unprefixed observable cursor value without crossing a JavaScript or N-API AST boundary.
- Differentially verify reviewed branches plus the full shape-by-box-by-order matrix, invalid neighbors, removal, and SheetOM-to-Chromium round trips.
- Execute representative clip-path and cursor image-set cases in isolated native and public subprocesses.

## Consequences

The Webref acceptance mismatch count falls from eight to three while preserving zero atomicity mismatches. The geometric differential grows from 199 to 317 branches, including 112 generated clip-path permutations. The runtime owns token semantics rather than observed literals, and malformed or unsupported neighbors remain atomic no-ops.
