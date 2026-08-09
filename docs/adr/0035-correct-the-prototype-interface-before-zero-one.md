# Correct the prototype interface before 0.1

Until `0.1.0`, browser conformance may require incompatible corrections to the unpublished prototype interface despite ADR 0034's intended 0.1 surface. Intentional SheetOM extensions remain limited to `parseStyleSheet`, `serialize`, and `takeDiagnostics`, but observed prototype behavior does not become permanent merely because it was implemented first; after `0.1.0`, normal semantic-versioning compatibility applies.
