# Run the short RC cycle on main

After feature completion, the single package will enter Changesets `rc` prerelease mode on `main`, receive stabilization fixes there, and exit prerelease mode for the stable release. A separate release branch would add divergence without protecting any unrelated workspace packages.
