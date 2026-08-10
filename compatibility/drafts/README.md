# Baseline drafts

The release workflow builds the reviewed host engine, executes the native grammar and process-safety contracts, and then runs `conformance:record` after all pinned oracle jobs complete. Review the generated draft and its source/evidence hashes before it is committed as `compatibility/baselines/<version>.json`; the recorder intentionally rejects missing execution reports.
