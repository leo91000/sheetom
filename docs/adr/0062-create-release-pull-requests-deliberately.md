# Create release pull requests deliberately

Maintainers will run the Changesets CLI and create release pull requests deliberately with an empty body rather than use `changesets/action` or custom release-PR automation. This preserves review of version, changelog, lockfile, prerelease state, and Compatibility Report while following the repository's pull-request convention.
