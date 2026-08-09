# Promote fuzz failures into operation fixtures

Every pull request will run a short deterministic mutation and malformed-syntax campaign, while nightly and release gates run a longer seeded campaign. Reproducible failures are minimized into permanent Operation Fixtures, and crashes, hangs, invalid safe serialization, or broken state invariants block release.
