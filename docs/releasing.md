# Releasing SheetOM

Changesets prepares versions and changelogs but never publishes. Publishing is
a separate maintainer operation from an exact reviewed commit.

## First release candidate

1. From a clean `main`, enter Changesets prerelease mode with `npm exec
   changeset pre enter rc`, then run `npm exec changeset version`. Review the
   resulting `0.1.0-rc.0` version, changelog, lockfile, and consumed Changeset.
2. Run every local gate: `npm run check`, `npm run test:browser:matrix`,
   `SHEETOM_FUZZ_RUNS=1000 npm run fuzz`, `npm run conformance:drift`, and
   `npm run benchmark`.
3. Download the stable native-WPT artifacts and record the immutable evidence
   from that version with `SHEETOM_RECORD_BASELINE=1 npm run
   conformance:record -- --wpt-report=chrome=reports/chrome.json
   --wpt-report=firefox=reports/firefox.json`. Move the reviewed
   draft to `compatibility/baselines/0.1.0-rc.0.json`, rerun
   `npm run conformance:validate`, and commit the release preparation through a
   deliberately created pull request with an empty body.
4. Require green CI plus a manual `Native browser oracles` run. Download and
   review the stable Chrome and Firefox WPT reports.
5. On the exact clean merge commit, run `npm run release:verify`, then `npm
   pack`. Preserve the resulting tarball; this is the artifact that is tested,
   published, and attached to GitHub.
6. Authenticate locally with `npm login --auth-type=web`. Publish only the
   verified tarball with `npm publish ./sheetom-0.1.0-rc.0.tgz --tag next`.
   The first RC intentionally has no npm provenance because local web auth is
   the approved bootstrap path.
7. Verify `npm view sheetom@0.1.0-rc.0 dist.integrity` and install the public
   version in a fresh directory before creating the GitHub Release.
8. Create a draft GitHub prerelease for tag `v0.1.0-rc.0`, attach the exact npm
   tarball and matching Compatibility Report, verify both downloads, then
   publish the prerelease without marking it latest.

After the first verified release, enable immutable GitHub Releases. Do not
enable immutability earlier: it would prevent correcting a bootstrap draft
before its first publication.

Native Safari WPT is deferred until a compatible no-cost runner is available.
Pinned Playwright WebKit remains part of the browser differential matrix, but
it is not presented as evidence from actual Safari.

## Stable release

Exit prerelease mode with `npm exec changeset pre exit`, prepare and review the
stable Changesets release, repeat every gate, then publish the exact tarball
with `--tag latest`. Mark the GitHub Release latest only after npm serves that
same version and integrity.

## Later trusted publishing

OIDC trusted publishing is deliberately staged after package ownership exists
on npm. Adding it requires a separate reviewed workflow and npm trusted-publisher
configuration. The workflow must publish an already-built and tested artifact;
it must not combine version preparation with publication.
