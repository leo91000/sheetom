# Defer native Safari until a compatible no-cost runner exists

Release gates require stable Chrome and Firefox native WPT reports alongside
the pinned Playwright Chromium, Firefox, and WebKit differential matrix. Native
Safari WPT is deferred until a compatible no-cost runner is available because
both GitHub-hosted macOS runner architectures hung before producing a report,
and a paid runner is outside the project's release budget. WebKit differential
results must not be described as actual Safari evidence. This supersedes the
native Safari release requirement in ADR-0063.
