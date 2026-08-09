# Automate releases after reviewed Changesets pull requests

Successful `main` CI runs maintain a draft Changesets version pull request.
Stable Chrome and Firefox evidence is recorded on that branch before the pull
request becomes ready, so merging it remains the human release approval. A
separate least-privilege job then packs and tests one artifact, creates and
verifies a draft GitHub Release, publishes that exact tarball through npm OIDC,
and publishes the GitHub Release only after registry verification. The process
is idempotent and never stores a long-lived npm publication token.
