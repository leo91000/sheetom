# Serialize semantic CSSOM state

Serialization will reproduce observable CSSOM state and browser-compatible normalization rather than preserve the original source text. Source comments, whitespace, formatting, and equivalent spelling choices may change, because preserving them would require a separate lossless source-editing model that conflicts with CSSOM mutation semantics.
