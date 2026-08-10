---
status: accepted
---

# Ship one tarball with eight native binaries

RC6 ships one dependency-free npm tarball containing the reviewed native engine for Linux x64 and ARM64 with GNU or musl libc, Windows x64 and ARM64, and macOS x64 and ARM64. CI builds each binary on its matching platform, assembles the exact tarball once, and exercises those same bytes across Node 22 and 24 plus the documented Bun and Deno paths. The loader selects only an included matching binary and fails explicitly on unsupported targets; there are no platform subpackages, install-time downloads, or behavioral fallbacks. This supersedes ADR 0045 and ADR 0096.
