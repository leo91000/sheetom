# Prepare release pull requests with Changesets

Normal changes will carry Changesets entries, and the Changesets release pull request will update the package version, lockfile, and changelog together with the matching Compatibility Report before the exact merged commit is tagged. Changesets prepares releases but never publishes them; initial local web-auth publication and later staged OIDC publication remain separately controlled operations.
