# Releasing SheetOM

Changesets prepares versions and changelogs. The release workflow publishes only an exact reviewed `main` commit after its CI succeeds.

## Release candidates

1. Add a patch Changeset for every consumer-visible behavior, dependency, compatibility promise, or runtime-support change.
2. Merge the implementation pull requests only after their complete CI is green.
3. Successful `main` CI maintains a draft `chore: release packages (rc)` pull request. The workflow versions the package, runs the native Chrome and Firefox oracles, records the immutable Compatibility Report, and marks the pull request ready.
4. Review the version, changelog, npm and Cargo lockfiles, Native Engine evidence, Compatibility Report, and green release-PR CI. RC6 reports must identify the exact vendored source manifest, 129/129 shorthands, 24 codec profiles, 96/96 grammar branches, 10/10 property branches, 36/36 value capabilities, and complete native/public process-safety execution.
5. Leave the generated release pull request unchanged while the RC6 Soak workflow records seven consecutive nightly full-CI successes on its exact SHA. Any update resets the evidence. Manual workflow runs are diagnostic and do not count. Merge only after all seven dated statuses are present.
6. Successful `main` CI builds one tarball containing all eight supported native binaries and tests that exact artifact across the complete Node, Bun, Deno, glibc, musl, Windows and macOS consumer matrix. The Release workflow resolves and verifies the seven-night evidence, downloads the artifact from that successful CI run, rejects an incomplete native set, creates a draft GitHub prerelease with it and the report, and publishes those exact bytes to npm through Trusted Publishing under `next`. A manual Release rerun resolves the successful CI run for the unchanged release commit; it never repacks the checkout.

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
- Every RC6-or-later release carries an exact Native Engine source manifest and executed grammar/process-safety evidence. Engine upgrades require reviewed SheetOM changes and a newly recorded Compatibility Baseline; consumer overrides are outside support.
- Grammar Branch Contracts are reviewed inputs. Ordinary CI may only check their pinned Chromium observations; only an explicit recording change may replace those observations.
- Native Safari WPT remains deferred until a compatible no-cost runner exists. Pinned Playwright WebKit remains part of the differential matrix but is not presented as evidence from actual Safari.
- Confirm the final npm dist-tags, deprecation state, provenance, immutable GitHub Release, attached tarball, Compatibility Report, and clean post-release CI before announcing a release.
