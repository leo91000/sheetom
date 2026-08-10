---
status: accepted
---

# Build property grammar on vendored CSS syntax

The Rust CSS Engine uses a complete vendored rust-cssparser snapshot as its sole CSS tokenizer and parser foundation; raw-string parsing with nom, prefix tests or a parallel scanner cannot decide property grammar. Standard reusable productions belong in Vendored Lightning Source, while SheetOM owns only typed pinned-browser extensions and CSSOM policy. Both vendored projects are imported with exact provenance and MPL notices, selected through local Cargo patches, tested with their upstream suites, and modified only in focused commits suitable for upstream contribution.
