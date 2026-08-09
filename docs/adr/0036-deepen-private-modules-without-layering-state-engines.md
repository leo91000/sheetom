# Deepen private modules without layering state engines

SheetOM will keep its compact external interface while replacing the prototype incrementally with private `DeclarationBlock`, `ValueGate`, `ShorthandRegistry`, `RuleTree`, and `Serializer` modules. Each vertical slice moves ownership to one new module and removes the corresponding old path immediately; a parallel second state engine was rejected because dual mutation authorities would make identity and serialization conformance harder to reason about.
