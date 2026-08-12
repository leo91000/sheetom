# ADR 0176: Version the artifact set as one cohort

## Decision

SheetOM, `@sheetom/wasm`, and all thirteen `@sheetom/native-*` packages form one Changesets fixed group. A version change to any member versions every member, generates a changelog for every workspace, and keeps the exact optional-dependency graph aligned before artifact construction.

The native synchronization script remains a fail-closed projection of the target registry. It validates package metadata after Changesets rather than acting as a second versioning authority.

## Rationale

The public root package can load only an implementation with the same Engine ABI identity and release version. Manually projecting the root version into implementation manifests preserved that runtime invariant, but Changesets did not know that the generated packages belonged to the release. Its GitHub Action consequently tried to publish them without generated changelogs and could not update the release pull request.

A single fixed cohort makes the versioning model match the already atomic fifteen-package artifact and publication model. The checked-in contract test requires the group to contain exactly the root, WebAssembly, and registry-owned native packages, so adding or removing a target cannot silently leave the release graph incomplete.
