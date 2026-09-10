# Validation and release operations

Paths and commands below are relative to the repository root. Inspect current
`package.json`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and
`scripts/publish-release.ts` before execution; their current contracts take precedence
over historical counts or versions. Discover the repository slug and PR/run IDs.

## Local evidence

Install the pinned toolchain/dependencies and required browser engines. After engine
identity and generated files have been updated, the current broad validation is:

```sh
npm run native:build
npm run build
npm run wasm:build
npm run css:target:check
npm run native:webref-property-branches
node scripts/check-webref-property-branches.ts --wasm
npm run check
npm run native:check
npm run wasm:check
npm run wasm:test
npm run ci:scripts:test
```

Run focused regressions first, then the relevant broader checks. Run browser/bundler,
performance, and memory gates when affected and require the applicable CI matrix.
When changing workflow browser setup, validate its workflow tests too. Once checks
pass, repeat them only after changed inputs or a failure requiring investigation.

## Implementation PR and main

Create a conventional commit/branch and push the implementation. Respect the project
PR convention: title only, `gh pr create --base main --title '<title>' --body ''`.
Avoid fixed PR numbers, release versions, package counts, or expected job totals.

Use `gh pr view` to capture `headRefOid` and `baseRefOid`, and `gh run list/view` to
identify the complete CI run for that head. `gh pr checks` helps inspect individual
checks, but an early all-green subset does not mean downstream jobs have finished.
Require the entire run to be completed with conclusion `success`, all required
checks successful, and only documented conditional skips. Fix failures and restart
the gate at the new head. Diagnose infrastructure failures before retrying them.

Read back the head immediately before merging and use:

```sh
gh pr merge <pr-number> --squash --match-head-commit <validated-head-sha>
```

Respect branch protections and required reviews. Verify the returned merge commit
and fetch main. Scan changed files for conflict markers after merges/rebases.
Wait for successful push CI on that exact main merge commit; a successful PR run
does not replace this gate. If main advances concurrently, reconcile the new state
and identify which successful main run Release actually uses.

## Release PR gate

Successful main CI triggers `release.yml`. Discover the release PR from that run
and its Changesets output, usually branch `changeset-release/main`. Reuse it; do not
create a competing release PR or manually bump published package versions.

The workflow gathers stable Chrome/Firefox and WASM evidence, records
`compatibility/baselines/<version>.json`, commits it, and explicitly dispatches CI
with the release base SHA. The evidence commit changes the PR head. Wait for this
final head, its complete dispatched CI, and successful `sheetom/release-validation`
status and release preparation workflow. A non-draft PR alone is not this gate.
Bot-triggered PR workflows may show `action_required`; inspect the explicit dispatch
and required checks instead of approving or merging based on an older run.

Review the diff against current main. Expect consumed Changesets, aligned package
and Cargo versions, changelogs, regenerated engine identity, and the versioned
compatibility report. Confirm the candidate includes the implementation merge,
understand every other included change, and investigate unexpected source edits.
Derive the fixed package cohort from `.changeset/config.json`. Verify the report
belongs to the current engine/version, has zero unexplained divergences, and carries
the required native/browser/WASM evidence. Documented browser resolutions may remain.
Use `scripts/verify-release.ts` and the workflow checks as the maintained verifier.

Merge with `--match-head-commit` only after reviewing and validating the final head.
Read back its merge SHA, scan for conflicts, and wait for successful main push CI
on that SHA. Then wait for the publication Release run. The workflow publishes the
tested artifact set via trusted publishing; local `npm publish` is not this path.

## Publication and recovery

Workflow `success` alone is insufficient: the publisher may intentionally retain a
GitHub draft while npm channel metadata propagates. Inspect publication logs and the
live registry/release state. Native packages, WASM, and the root package publish in
sequence, and registry scanning/propagation can take minutes between packages.

If publication reports pending channels, compare both the package metadata's
`dist-tags` and the dedicated dist-tags endpoint. Verify the intended version,
integrity, and any reported deprecation conditions using the current publisher's
channel assessment. Once propagation satisfies those conditions, rerun
`gh workflow run release.yml --ref main` to finalize the existing draft, provided
main is still the intended release commit. The publisher verifies and reuses
already-published artifacts. A timeout after partial publication can also be resumed
after inspecting its logs and registry state.

Retry only when evidence supports transient propagation/infrastructure trouble.
If the same unchanged failure recurs on two retries, stop that retry loop and report
the specific unresolved condition. Authentication/permission failures, unexpected
integrity, concurrent release changes, or genuine channel/deprecation mismatches
need diagnosis and a concrete repair before retrying. Do not force a tag/channel,
overwrite evidence, or manually promote a draft to bypass the release verifier.

## Published-release proof

- GitHub release `v<version>` exists, is non-draft and non-prerelease for a stable
  release, and its resolved tag commit equals the intended release merge SHA.
- Download `sheetom-release-manifest.json` from that release. Check the artifact
  cohort against current package configuration and confirm all expected assets.
- For every manifest package, query official npm version metadata and dist-tags.
  Verify the version exists, `latest` points to it for a stable release, and
  `dist.integrity` equals the manifest. Inspect provenance/attestation metadata;
  distinguish its presence from cryptographic verification.
- In a fresh temporary directory outside the repository, install the published
  root and WASM packages at the exact version. Run `npm audit signatures` and
  exercise the newly added public behavior and serialization round trips through
  both backends. Use the actual public APIs and avoid a workspace-linked install.
- From the repository with its pinned development dependencies installed, run
  `node scripts/check-installed-packages.ts --package-root=<fresh-install-prefix>
  --report-dir=<evidence-directory>`. The prefix contains `node_modules/sheetom`
  and `node_modules/@sheetom/wasm`. This maintained runner exercises the shared
  modern CSS contracts, pinned browser comparisons, and native/WASM Webref ratchet
  against the installed packages. It writes separate logs and JSON reports.
  Keep test sources and corpus paths in the repository; use the runner's explicit
  package selection instead of copying scripts or rewriting relative imports.
  It supplies bounded WASM bytes and fails if an installed backend is missing.
- Install the browser engines selected by the repository's Playwright version.
  The managed container provides their OS libraries. If launch fails, preserve the
  launch diagnostic and repair the container dependency layer; a dated library
  wrapper or altered assertion is not valid permanent evidence.

Report success only after these observations agree. If blocked, distinguish merged
code, published packages, and the remaining release state precisely.
