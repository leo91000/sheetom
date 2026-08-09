---
status: accepted
---

# Stage releases around channel reconciliation

When npm channel reconciliation or deprecation is required, release automation will publish the verified tarball but keep the GitHub Release as a draft and report the pending maintainer checkpoint without failing CI. After web-authenticated channel changes, a manual rerun verifies the complete npm package metadata and only then publishes the immutable GitHub Release. This two-phase protocol preserves tokenless Trusted Publishing and prevents an apparently complete release from leaving bare installs on an obsolete version.
