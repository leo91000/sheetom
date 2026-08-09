# Test conformance through the public interface

Behavioral conformance tests exercise SheetOM only through its public interface, while tests may cross private seams only for isolated pure algorithms. This keeps compatibility tied to caller-observable behavior and permits the internal modules to be replaced without rewriting the conformance corpus.
