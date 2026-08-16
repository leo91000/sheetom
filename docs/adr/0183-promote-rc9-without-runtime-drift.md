---
status: superseded by ADR-0186
---

# Promote RC9 without runtime drift

## Decision

The first stable release will promote the exact RC9 source line without adding
runtime behavior. RC9 supersedes RC8 as the accepted behavior candidate because
it preserves the independent invalidation semantics of longhands containing
deferred substitutions. The stable CI compares the complete fifteen-package
cohort with the public RC9 release, normalizing only lockstep version metadata
and the resulting version-derived Engine ABI digest.

Native and WebAssembly binaries are rebuilt for the stable version, so byte
equality is not meaningful. Their unchanged source is enforced by the
release-only diff allowlist, while behavior remains covered by the complete
platform, browser, fuzz, process-safety, and seven-night soak gates.

## Rationale

Promoting RC8 would knowingly restore a serialization path that can couple
otherwise independent computed-value invalidation. Restarting the prerelease
and soak cycle on RC9 keeps the stable promotion exact without weakening the
release evidence.
