---
status: accepted
---

# Ship one native engine through platform packages

RC6 will ship the Rust CSS Engine as exact-version optional npm packages for Linux x64 GNU, Linux ARM64 GNU, Windows x64 and macOS ARM64. The root package selects the matching local binary, tests Node.js 22 and 24 on their claimed platforms plus Bun and Deno on Linux x64, and fails explicitly outside that matrix; it will not bundle every binary, download one during installation, or ship a behavioral WASM fallback. Platform packages are published and verified before the root package so a missing binary cannot create a partially usable release.
