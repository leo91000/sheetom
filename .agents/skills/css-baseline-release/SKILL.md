---
name: css-baseline-release
description: Audit SheetOM against current CSS Baseline data, implement missing authoring support, and deliver it through implementation CI, main CI, the Changesets release PR, and verified publication. Use for Baseline refreshes or this full maintenance workflow; respect audit-only or no-release requests.
---

# CSS Baseline update and release

Run the requested workflow to completion. An explicit request to run this full
skill includes creating and pushing the implementation PR, merging after CI,
merging the validated release PR, and publishing through repository automation.
Automatic skill selection for an audit question does not authorize those writes.
Honor narrower user instructions; reuse authorization already given in the session.

## Establish the target

1. Locate the repository root and read its applicable instructions. Inspect git
   status, fetch origin, and record current main, open implementation/release PRs,
   active workflows, and the latest published version. Reuse an existing run when
   resuming. Preserve unrelated work; implement in an isolated worktree from
   current main with a conventional branch such as `feat/css-baseline-YYYY-MM-DD`.
   Read this tracked skill and its sibling `references/delivery.md` from that
   worktree. Discover optional instruction files with `git ls-files AGENTS.md
   '**/AGENTS.md'`; an empty result is normal. Discover workflow action paths with
   `git ls-files '.github/actions/*/action.yml'` before reading them. Keep optional
   searches separate from required commands: a search with no matches exits 1.
2. Read `compatibility/css-feature-target.json`, its generator
   `scripts/generate-css-feature-target.ts`, `compatibility/css-authoring-coverage.json`,
   `compatibility/css-authoring-probes.json`, and the current CSS evidence/ADR docs.
   These files define the current cutoff, source pins, and authoring contract.
3. Use today's date unless the user specifies a cutoff. Query the official npm
   registry for the latest `web-features` release and retrieve its data. Record the
   package version, archive URL, source revision, and SHA-256 of the actual data
   bytes. Consult [WebDX](https://github.com/web-platform-dx/web-features), the
   feature's linked specification, and browser compatibility evidence for changes;
   previous run dates, feature counts, and browser versions are historical.
4. Diff compatibility keys, including newly eligible branches in existing families,
   added families, corrected statuses, and changed specifications. Baseline here
   includes Newly Available (`low`) and Widely Available (`high`) branches whose
   `baseline_low_date` is at or before the cutoff. Select eligible individual
   `status.by_compat_key` entries even when the whole family is not Baseline.
   Explain undated or withdrawn entries rather than silently assigning eligibility.
5. Classify every candidate as supported with executable evidence, missing authoring
   support, DOM/evaluation outside scope, or not yet eligible. SheetOM covers parsing,
   authored CSSOM state, mutation, support queries, and serialization; cascade,
   computed styles, selector matching, layout, and mixin expansion are outside this
   contract. Keep pinned experimental features separate from Baseline claims.
   An evidence-map entry or parser accepting arbitrary tokens is not proof of support.

Done when the dated audit accounts for every changed eligible branch and any
previously unresolved authoring gaps. If nothing needs changing, report the checked
source/cutoff and stop without creating an empty PR or release. If only inventory
or evidence changes, update those honestly without manufacturing a runtime change.

## Implement and validate

1. Reproduce each gap through the public API and a suitable browser reference.
   Cover valid/invalid grammar and observable state, not merely `CSS.supports`.
   Preserve typed expressions across compatible property contexts. Put shared
   semantics in `crates/sheetom-core` or the owned parser stack; expose consistent
   native and WASM facades.
2. Add focused regression and differential evidence for parsing, mutations,
   invalid-write atomicity, WebIDL/descriptor behavior where relevant, and semantic
   serialization/reparse. Keep native/WASM cases shared through
   `scripts/test-modern-css-backends.ts`. Compare canonicalized values/state when
   equivalent spellings differ. Document browser disagreements explicitly; a
   pinned browser predating a newly Baseline feature needs a measured alternate
   oracle or reviewed pin update, not a false unsupported result.
3. Update the dated target generator's cutoff, source pins, and derived assertions
   together. Its no-argument mode checks the checked-in snapshot; it does not fetch
   new data. Regenerate with `--source=<data-file> --record`, then recheck with that
   source. Regenerate coverage with `node scripts/generate-css-authoring-coverage.ts
   --record` and add actual probes for new branches. Keep historical audits intact
   and link a current evidence document; explain material contract decisions in an ADR.
4. Follow the existing vendor revision and engine identity conventions for engine
   edits. Stage newly added engine files before `npm run record:engine-abi`, because
   its source manifest uses `git ls-files`. Add an appropriate Changeset for public
   behavior changes (normally minor for new support); let `.changeset/config.json`
   and release automation own package cohort versions.
5. Read [validation and release operations](references/delivery.md) before running
   the validation sequence. Fix all relevant failures. Build native and WASM
   artifacts sequentially before checks consuming them: concurrent builds can
   clear shared output directories. Ensure CI installs every browser used by new
   probes through the existing shared Playwright action.

Done when every in-scope gap is implemented and evidenced on both backends, generated
files are reproducible, documentation is current, and required checks pass. State
any remaining gap explicitly; never relabel it as covered to finish the workflow.

## Deliver through Changesets

Follow the exact-commit gates in [delivery.md](references/delivery.md):

1. Push the implementation PR with a title and empty body. Wait for its entire CI
   run to succeed, review the final diff, and merge the validated head into main.
2. Wait for CI on that main merge commit. Then wait for Release automation to create
   or refresh its release PR, record compatibility evidence, and validate the final
   release head. Review the resulting version, cohort, changelog, and evidence.
3. Merge that release PR only after its final CI and release validation succeed.
   Wait for CI on the release merge commit and the subsequent publication workflow.
4. Verify the stable GitHub release/tag, every package's version/channel/integrity,
   provenance, and a cold native/WASM smoke test of the added features. Handle a
   propagated npm release still in GitHub draft using the documented retry path.

Keep a compact record of PR numbers, head/merge SHAs, workflow IDs, candidate version,
and the last completed gate so a resumed run continues at the right phase. While
waiting, poll at reasonable intervals and provide concise progress updates at least
once a minute. A queued, cancelled, failed, or pending run is not a passed gate.

Completion is either an evidenced no-change audit, the narrower outcome requested
by the user, or a verified published release. Report implemented features, remaining
scope limits, PR/release links, and validation results. Safely fast-forward the user's
local main when possible while preserving unrelated files.
