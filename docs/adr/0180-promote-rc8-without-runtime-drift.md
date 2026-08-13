---
status: superseded
superseded_by: ADR-0183
---

# ADR 0180: Promote RC8 without runtime drift

## Decision

The first stable release will promote the exact RC8 source line without adding
runtime behavior. Its CI compares the complete fifteen-package cohort with the
public RC8 release, normalizing only lockstep version metadata and the resulting
version-derived Engine ABI digest. Package topology and every non-binary runtime
file must otherwise match. Native and WebAssembly binaries are rebuilt for the
stable version, so byte equality is not meaningful; their unchanged source is
enforced by a release-only diff allowlist and their behavior remains covered by
the complete platform, browser, fuzz, process-safety, and seven-night soak gates.

## Rationale

RC8 is the accepted behavior candidate. Rebuilding is necessary to publish a
stable lockstep version, but it must not become an opportunity to add unsoaked
runtime changes. Comparing normalized package contents and constraining the
release diff makes that boundary executable without pretending version-bearing
native binaries are reproducible byte for byte.
