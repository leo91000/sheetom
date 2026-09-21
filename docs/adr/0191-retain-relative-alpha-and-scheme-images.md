---
status: accepted
---

# Retain relative alpha colors and scheme-dependent images

The 21 September 2026 WebDX refresh makes alpha colors and image-valued
light-dark eligible. Parsing `alpha(from currentColor / .5)` must not require
resolving the origin. Reuse the owned relative-color AST with zero color channels
and an optional typed alpha expression. Only the alpha channel name is allowed.
Image-valued light-dark owns two parsed image-or-none branches in the vendor AST;
neither branch is selected during authoring. Shared parser changes serve both
native and WASM facades and apply inside compatible property contexts.

Chromium 151.0.7922.34 and Firefox 153.0 reject alpha colors in measured probes.
An additional exact playwright-core 1.63.0 alias supplies Chromium 153.0.8010.12
for explicitly marked alpha probes. The established browser cohort stays pinned;
the shared CI installer installs this additional reference. This is a measured
per-feature oracle decision, not an inference from a version number in WebDX.

WebDX 3.39.0 withdraws all seven media-state selector Baseline statuses. Their
existing supported authoring APIs and Firefox evidence remain, separately from
the dated Baseline target. DOM matching, current-color resolution, scheme
selection, and scroll/anchor layout remain outside the authoring contract.
