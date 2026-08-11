# ADR 0115: Record the full property/value cross product

## Status

Accepted for RC6.

## Context

The Chromium property manifest proves that a property name is recognized, but it does not prove that SheetOM accepts the property's grammar. A small collection of hand-picked values can leave entire grammar families unimplemented while all existing release checks remain green.

The existing shorthand and CSS Math corpora provide deep evidence for their selected domains. They do not give a horizontal signal across every recognized ordinary property, and they cannot reveal a valid declaration that silently becomes a no-op outside those domains.

## Decision

SheetOM records a Chromium-versioned cross product of every manifested property and a reviewed set of grammar-oriented values. The evidence includes acceptance, observable value, declaration serialization, expanded item order, and proof that every rejected value is an atomic no-op after a valid value.

The checked-in browser observations are conformance evidence only. Runtime code must not import them or use exact observed values as a parser table. Rust codecs continue to own semantic state and serialization.

The broad matrix complements, rather than replaces, property-specific branch corpora. A broad probe can identify an unsupported family; a dedicated branch contract must then cover the valid branches and their invalid neighbors before that family is considered complete.

During the RC6 implementation sequence, the report command may be run with explicit mismatch reporting so each focused PR can reduce the inventory. The RC6 release gate runs the strict form and requires zero acceptance, observable, serialization, item-order, or atomicity mismatches. The current pinned matrix contains 66,123 checks across 711 properties and 93 probes; its execution report and both browser-evidence inputs are hashed into the release Compatibility Report.

The matrix runner can still gate named dimensions independently while investigating future browser drift. Pull-request and release CI now execute all dimensions together; an acceptance mismatch is never silently counted as an observable pass.

## Consequences

- The baseline accounts for every property/value pair and fails closed when the property manifest or probe set changes.
- Browser drift is reviewed as a deterministic snapshot change.
- Compatibility evidence grows without increasing the published runtime bundle or package payload.
- RC6 cannot claim complete ordinary declaration coverage from property-name recognition alone.
