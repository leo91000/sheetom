---
status: accepted
---

# Separate standard expansion from CSSOM synthesis

Vendored Lightning Source parses standard property grammars and expands standard static shorthands into owned typed longhands; SheetOM adds only typed pinned-browser grammar extensions and owns CSSOM shorthand synthesis, declaration ordering, priorities and mutation semantics. SemanticPropertyValue directly retains owned Lightning property variants rather than duplicating them in a parallel SheetOM AST or erasing them into generic tokens. This keeps reusable CSS grammar upstreamable while isolating browser-facing Authoring CSSOM policy in the Rust CSS Engine.
