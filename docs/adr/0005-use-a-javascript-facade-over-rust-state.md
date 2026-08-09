---
status: superseded by ADR-0011
---

# Use a JavaScript facade over Rust state

The public API will use browser-shaped JavaScript proxy objects backed by a persistent Rust-owned stylesheet model. This split keeps parsing and mutation native while allowing numeric indices, dynamic property names, stable object identity, live updates, and detached-rule behavior that ordinary napi-rs classes cannot expose faithfully on their own.
