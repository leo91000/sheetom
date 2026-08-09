---
status: accepted
---

# Keep bare installs on the active release

Before SheetOM has a stable release, npm's `latest` and `next` channels will both identify the active prerelease so a bare `npm install sheetom` cannot select an obsolete release candidate. After the first stable release, `latest` remains stable, `next` identifies only an active prerelease and is removed on stable publication, and superseded release candidates are deprecated; this refines ADR 0072 while preserving its stable-versus-prerelease channel distinction. Because npm Trusted Publishing cannot mutate dist-tags or deprecations, prerelease channel reconciliation will be an explicit web-authenticated maintainer checkpoint rather than reintroducing a long-lived npm write token.
