# Releasing SheetOM

Changesets prepares versions and changelogs. The release workflow publishes only an exact reviewed `main` commit after its CI succeeds.

## Release candidates

1. Add a patch Changeset for every consumer-visible behavior, dependency, compatibility promise, or runtime-support change.
2. Merge the implementation pull requests only after their complete CI is green.
3. Successful `main` CI maintains a draft `chore: release packages (rc)` pull request. The workflow versions the package, runs native Chrome and Firefox oracles plus the independent WASM backend oracle, records the immutable Compatibility Report, and marks the pull request ready.
4. Review the version, changelog, npm and Cargo lockfiles, Native Engine evidence, Compatibility Report, and green release-PR CI. Reports must identify the exact vendored source manifest, every manifested shorthand, the complete versioned grammar and value-capability corpora, the Chromium property/value cross-product, the Webref-derived property branches, and complete native/public process-safety execution. The verifier derives corpus counts from reviewed release evidence so this guide cannot silently become a stale numeric allowance.
5. Leave the generated release pull request unchanged while its complete CI matrix runs on the final evidence commit. Any update invalidates that run and requires a fresh complete validation.
6. Changesets versions `sheetom`, `@sheetom/wasm`, and all thirteen `@sheetom/native-*` packages as one fixed cohort and generates a changelog for every workspace. Successful `main` CI then builds a binary-free root tarball, the exact-version `@sheetom/wasm` tarball, and all thirteen exact-version `@sheetom/native-*` tarballs. It executes the WASM bytes in Chromium, Firefox and WebKit and every native package on its matching operating system, CPU, and Linux libc, including foreign-architecture Node through QEMU, before uploading one immutable artifact set. The set contains a deterministic manifest with every package name, role, target, size, SHA-256 digest and npm integrity. The Release workflow downloads those tested bytes, rejects any incomplete, reordered, changed or misplaced artifact, publishes implementation packages before the root under `next`, and attaches all fifteen tarballs, the manifest and Compatibility Report to the draft GitHub prerelease. It never repacks the checkout.

The first release containing new `@sheetom/*` packages pauses before publishing the root package. Download the exact successful CI artifact, authenticate once with `npm login --auth-type=web`, and run the release script with `SHEETOM_BOOTSTRAP_IMPLEMENTATIONS=1` and `SHEETOM_RELEASE_TARBALL` pointing to that artifact directory. This publishes only the thirteen native packages and `@sheetom/wasm`; it cannot publish or expose a partial root release. Configure the same GitHub Actions Trusted Publisher used by `sheetom` for every new scoped package, then rerun the Release workflow. Later versions use OIDC for the complete artifact set.

Before the first stable release, npm's `latest` and `next` channels for `sheetom` and `@sheetom/wasm` must both point to the active release candidate. Native implementation packages use `next`; users do not install them directly. Trusted Publishing cannot mutate dist-tags or deprecations, so the initial publish leaves the GitHub Release as a draft and reports a maintainer checkpoint when reconciliation is needed.

Authenticate with npm on the web, point `latest` to the active RC, and deprecate every replaced prerelease:

```sh
npm login --auth-type=web
npm dist-tag add sheetom@<active-rc> latest --auth-type=web
npm dist-tag add @sheetom/wasm@<active-rc> latest --auth-type=web
npm deprecate sheetom@<replaced-rc> \
  "Contains known compatibility bugs. Upgrade to sheetom@next." \
  --auth-type=web
```

Deprecate replaced prereleases of `@sheetom/wasm` and every native package as the same checkpoint once prior versions exist. Verify the channel policy and deprecations, then rerun the Release workflow from `main`. The rerun reads the complete npm metadata for all fifteen packages, verifies every published integrity, publishes the GitHub prerelease, and verifies its immutable assets.

## Stable releases

Exit Changesets prerelease mode in a reviewed change, then follow the same release-PR and CI flow. Leave the generated `0.1.0` release pull request unchanged while the First Stable Soak workflow records seven consecutive nightly full-CI successes on its exact SHA. Any update resets the evidence; manual runs are diagnostic and do not count. Merge only after all seven dated statuses are present. The publication places the stable version under `latest`. Before the GitHub Release can become public, authenticate with npm, remove `next`, and deprecate every replaced prerelease:

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
- Every release carries an exact Native Engine source manifest and executed grammar/process-safety evidence. Engine upgrades require reviewed SheetOM changes and a newly recorded Compatibility Baseline; consumer overrides are outside support.
- Grammar Branch Contracts are reviewed inputs. Ordinary CI may only check their pinned Chromium observations; only an explicit recording change may replace those observations.
- Native Safari WPT remains deferred until a compatible no-cost runner exists. Pinned Playwright WebKit remains part of the differential matrix but is not presented as evidence from actual Safari.
- Confirm the final npm dist-tags, deprecation state, provenance, immutable GitHub Release, all fifteen attached tarballs, artifact manifest, Compatibility Report, and clean post-release CI before announcing a release.
