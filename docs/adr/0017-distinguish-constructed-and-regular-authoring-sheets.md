# Distinguish constructed and regular authoring sheets

`new CSSStyleSheet()` will retain browser constructable-sheet behavior, including stripping `@import` during replacement and rejecting its insertion. A Node-specific `parseStyleSheet(css)` entry point will instead create a Regular Authoring Sheet that preserves valid imports as rules without fetching them, enabling arbitrary existing stylesheets to be mutated and serialized.
