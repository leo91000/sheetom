---
status: accepted
---

# Classify observable value serialization

The Value Gate will classify accepted declarations as typed ordinary values, pending substitutions, or custom properties and delegate browser-facing text to an explicit category-aware Observable Value Codec. Lightning CSS remains the acceptance and reparsable-output authority; token semantics retain recovered text for substitutions and custom properties, while typed properties use explicitly verified property-aware serializers and may use cssstyle only as a helper. This refines ADRs 0007 and 0009 and rejects both raw-input preservation and a single generic canonical serializer.
