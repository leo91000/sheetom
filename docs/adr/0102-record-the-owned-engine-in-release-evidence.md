---
status: accepted
---

# Record the owned engine in release evidence

Compatibility Report schema version 5 identifies the repository-owned native engine by its public revision, upstream Lightning CSS commit, and a SHA-256 manifest of every tracked Cargo, engine, loader, and vendored source file. It records executed Native Grammar Inventory and Process Safety reports rather than naming development-only npm parsers. Release verification recomputes the source and corpus hashes and rejects partial execution counts. This supersedes ADR 0077 for RC6 and later native releases; historical reports retain their original syntax-engine set.
