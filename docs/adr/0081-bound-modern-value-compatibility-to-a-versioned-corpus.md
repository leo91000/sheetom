# Bound modern value compatibility to a versioned corpus

SheetOM will describe support for modern and implementation-dependent property grammars through a release-versioned Value Capability Corpus containing measured positive and negative families, rather than claim support for every value in the latest browser or specification. Where CSS parsing legitimately depends on user-agent support, `setProperty()` follows the pinned Chromium compatibility baseline and records neighboring valid and invalid cases so parser upgrades cannot silently change a published Compatibility Baseline.
