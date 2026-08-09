# Block on stale WPT mappings

Any changed source blob makes its manual WPT Mappings stale until reviewed, while a named deterministic generator may update mappings only when its checked structural fingerprint still matches. Removed or renamed subtests require reviewed WPT Tombstones, so upstream drift can never silently reduce conformance coverage.
