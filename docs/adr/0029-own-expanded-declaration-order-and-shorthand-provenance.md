# Own expanded declaration order and shorthand provenance

Each declaration block will own an ordered set of unique expanded longhand and exact-case custom Declaration Records plus an indexed lookup. Shorthand Groups retain accepted-mutation provenance so getters and CSSOM serialization can synthesize shorthands when current values and priorities permit, while Lightning CSS remains limited to per-value parsing and canonicalization helpers.
