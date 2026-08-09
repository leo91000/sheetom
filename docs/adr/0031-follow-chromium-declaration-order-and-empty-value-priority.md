# Follow Chromium declaration order and empty-value priority

Where engines diverge, initial declaration parsing will use Chromium's ordering of surviving normal declarations before important declarations, and an empty-value `setProperty` with invalid priority will leave state unchanged. Each behavior will remain a Divergence Fixture under the established Chromium fallback policy.
