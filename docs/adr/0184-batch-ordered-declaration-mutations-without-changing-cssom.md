---
status: accepted
---

# Batch ordered declaration mutations without changing CSSOM

SheetOM exposes `CSSStyleDeclaration.applyMutations()` as a performance
extension for compilers that already know all mutations for one declaration
block. The engine executes the operations in array order through the same
`setProperty` and `removeProperty` state machine. Each operation observes the
state left by its predecessor. The batch is not parsed as declaration text,
does not weaken the Value Gate, and is not globally transactional.

A set result reports whether the engine accepted the value and carries its
operation-local diagnostic when rejected. A remove result carries the same
previous value as `removeProperty()`. Opt-in sheet diagnostics still receive
the same rejected-mutation diagnostics in the same order. Resource Budget
errors throw at the operation where the equivalent sequential call would
throw; earlier operations remain committed.

The JavaScript facade validates the complete typed operation array before
entering the engine. Unlike the browser-defined methods, this SheetOM
extension does not perform arbitrary WebIDL string coercion. It accepts string
property names and priorities plus string or null set values, which keeps the
Native Data Boundary explicit and prevents a user conversion hook from
running inside a partially applied batch.

The native transport is columnar and returns one compact result vector. It
does not round-trip parser ASTs or declaration text. The Rust engine avoids
cloning an entire Declaration State for each operation: it parses and projects
a candidate first, verifies the projected record count, then commits through
the existing ordered record algorithm. A bounded per-thread parse cache may
share immutable parsed candidates across declaration blocks only when the
canonical property, source alias, value, priority and every Resource Limit
match. Input and depth budgets are checked before every cache lookup. Cache
entries are count- and byte-bounded, large values bypass it, and cloned
records receive independent shorthand provenance before mutation.

Reparsable Stylesheet Serialization walks all top-level rules into one output
builder rather than materializing one complete string per rule before the
final join. `serialize()` still returns one JavaScript string; a streaming API
is not added because measured large-sheet peak growth is already close to the
unavoidable returned string and the Publisher requires the complete CSS
value. A future sink API must preserve identical serialization bytes and may
not become a second serializer.

Rejected alternatives were parsing a concatenated declaration block, which
changes duplicate, invalid-value, priority, removal and diagnostic semantics;
a JavaScript-only batching helper, which retains every native crossing; and a
parallel bulk state model, which would duplicate CSSOM ordering and shorthand
logic.
