# ADR 0118: Own names, scopes, counters, and paired strings

## Status

Accepted for RC6.

## Context

Several Chromium properties share token families without sharing the same grammar. Anchor and timeline names require dashed identifiers, but only selected scopes accept `all`. Counter properties share a repeated identifier/integer shape while assigning different omitted integers. `page` accepts one custom identifier, `will-change` accepts a comma list with reserved exclusions, and `quotes` requires complete string pairs.

Treating these as generic identifier or token lists would accept invalid mixtures such as `all --name`, odd quote counts, fractional counters, or reserved `will-change` values. Exact-value tables would reject valid author-defined names.

## Decision

SheetOM owns these families as typed Rust values:

- dashed-identifier lists encode `none`, optional `all`, and comma-separated names as distinct states;
- keyword-or-name properties encode their finite keywords separately from one dashed identifier;
- counter lists store every custom identifier with its explicit or property-specific default integer;
- quote lists store complete open/close string pairs;
- paint order stores the semantic three-component ordering and emits Chromium's shortest reconstructible prefix;
- `will-change`, page names, visibility flags, and string-or-keyword properties have dedicated invariants.

The shared Chromium differential now covers 165 composite branches. Every valid branch is followed by a property-specific invalid neighbor, and the full declaration state must remain unchanged after that attempted replacement.

## Consequences

- Sixteen previously unsupported properties now accept arbitrary grammar-valid names and values rather than observed literals only.
- Sixty new branches and invalid neighbors match Chromium 151.
- Duplicate names remain accepted where Chromium accepts them, while exclusivity and separators remain property-specific.
- Eight ordinary complex properties remain explicitly unsupported pending their shape or trigger codecs.
