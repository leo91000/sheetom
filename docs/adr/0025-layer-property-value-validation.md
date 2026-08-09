# Layer property-value validation

The synchronous Value Gate will accept typed Lightning CSS values, then validate recognized arbitrary substitutions recursively, use `css-tree` for remaining ordinary property grammars, and reject unresolved ambiguity with a diagnostic. This prevents Lightning's deliberately permissive unparsed fallback from retaining invalid values while preserving browser-valid deferred substitution.
