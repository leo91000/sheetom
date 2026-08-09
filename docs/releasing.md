# Releasing SheetOM

Changesets prepares versions and changelogs. The release workflow publishes only an exact reviewed `main` commit after its CI succeeds.

## Release candidates

1. Add a patch Changeset for every consumer-visible behavior, dependency, compatibility promise, or runtime-support change.
2. Merge the implementation pull requests only after their complete CI is green.
3. Successful `main` CI maintains a draft `chore: release packages (rc)` pull request. The workflow versions the package, runs the native Chrome and Firefox oracles, records the immutable Compatibility Report, and marks the pull request ready.
4. Review the version, changelog, lockfile, Syntax Engine Set, Compatibility Report, and green release-PR CI. The report must identify 24 shorthand codec profiles, 96/96 passing Grammar Branch Cases, and SHA-256 hashes for both the reviewed contracts and pinned Chromium observations. Then merge the release pull request.
5. Successful `main` CI builds and tests one tarball through Node, Bun, and Deno, creates a draft GitHub prerelease with that tarball and report, and publishes the exact tarball to npm through Trusted Publishing under `next`.

Before the first stable release, npm's `latest` and `next` channels must both point to the active release candidate. Trusted Publishing cannot mutate dist-tags or deprecations, so the initial publish leaves the GitHub Release as a draft and reports a maintainer checkpoint when reconciliation is needed.

Authenticate with npm on the web, point `latest` to the active RC, and deprecate every replaced prerelease:

```sh
npm login --auth-type=web
npm dist-tag add sheetom@<active-rc> latest --auth-type=web
npm deprecate sheetom@<replaced-rc> \
  "Contains known compatibility bugs. Upgrade to sheetom@next." \
  --auth-type=web
```

Verify `latest === next === <active-rc>` and the deprecation, then rerun the Release workflow from `main`. The rerun reads the complete npm package metadata, verifies the published integrity and channel policy, publishes the GitHub prerelease, and verifies its immutable assets.

## Stable releases

Exit Changesets prerelease mode in a reviewed change, then follow the same release-PR and CI flow. The publication places the stable version under `latest`. Before the GitHub Release can become public, authenticate with npm, remove `next`, and deprecate every replaced prerelease:

```sh
npm login --auth-type=web
npm dist-tag rm sheetom next --auth-type=web
npm deprecate sheetom@<replaced-rc> \
  "A stable release is available. Upgrade to sheetom@latest." \
  --auth-type=web
```

Rerun the Release workflow after reconciliation. It marks a stable GitHub Release as latest only after npm serves the same version and integrity, `latest` points to it, and `next` is absent.

## Verification and support

- Never run a second `npm publish` for an existing version. The release script is idempotent and resumes from the existing package and draft GitHub Release.
- Every release carries an exact Syntax Engine Set. Parser upgrades require reviewed SheetOM changes and a newly recorded Compatibility Baseline; consumer overrides are outside support.
- Grammar Branch Contracts are reviewed inputs. Ordinary CI may only check their pinned Chromium observations; only an explicit recording change may replace those observations.
- Native Safari WPT remains deferred until a compatible no-cost runner exists. Pinned Playwright WebKit remains part of the differential matrix but is not presented as evidence from actual Safari.
- Confirm the final npm dist-tags, deprecation state, provenance, immutable GitHub Release, attached tarball, Compatibility Report, and clean post-release CI before announcing a release.
