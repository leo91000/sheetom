---
status: accepted
---

# Migrate the CSS engine through shadow evidence

The TypeScript and Rust declaration engines may coexist only in tests while migration is underway. Shadow Engine Runs apply identical operations and compare complete browser-facing declaration state plus reparsable output; Rust becomes authoritative only after every difference is resolved, then the TypeScript engine and npm Lightning CSS binding are removed without a runtime fallback. RC6 treats known observable differences in getters, `cssText`, item order, priority, atomicity or mutation sequences as release blockers even when rendering is equivalent.
