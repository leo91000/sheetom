---
status: accepted
---

# Remove the JavaScript syntax runtime

SheetOM's published package has no JavaScript runtime dependencies. The repository-owned Rust engine is the sole parser, declaration state machine, shorthand implementation and serializer; the TypeScript implementation and its npm Lightning CSS, css-tree and CSS tokenizer runtime paths are removed after native parity. Those packages may remain pinned development tools for generating or validating browser evidence, but they cannot be imported by production source, embedded into bundles or installed by a SheetOM consumer. There is no alternate runtime engine or behavioral fallback.
