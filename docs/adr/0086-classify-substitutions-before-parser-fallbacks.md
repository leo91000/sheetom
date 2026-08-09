# Classify substitutions before parser fallbacks

The Value Gate will classify deferred substitutions directly from the original CSS tokens before interpreting any Lightning CSS AST variant. It will then accept genuinely typed Lightning values, try `css-tree` for remaining ordinary grammars, apply narrow Value Capability Validators backed by positive and negative corpus cases, and reject unresolved ambiguity atomically. A parser's `unparsed` or `custom` variant is therefore transport information rather than proof that a value is pending, valid, or invalid.
