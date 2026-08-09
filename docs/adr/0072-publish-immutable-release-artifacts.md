# Publish immutable release artifacts

Each GitHub Release will attach the exact verified npm tarball and matching Compatibility Report, with RCs marked prerelease and published under npm's `next` tag while stable releases use `latest`. Releases become immutable after verification so their tag and evidence cannot be replaced; a GitHub Release is published only after the corresponding npm version is publicly installable.
