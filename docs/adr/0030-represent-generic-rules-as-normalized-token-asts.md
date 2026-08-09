# Represent Generic Rules as normalized token ASTs

Generic Rules will retain Lightning CSS's normalized token AST for unknown or not-yet-specialized valid rules. Forgiving Sheet Parse will continue to drop invalid rules, and SheetOM will not add a Rust raw-source scanner merely to retain invalid input or original formatting that the semantic serialization contract excludes.
