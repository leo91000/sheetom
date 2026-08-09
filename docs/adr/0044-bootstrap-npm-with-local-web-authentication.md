# Bootstrap npm with local web authentication

The first `0.1.0-rc.0` publication will be built, verified, and published from a local npm CLI session authenticated through npm's web flow, never through a GitHub workflow or repository token. That release is explicitly the one-time non-provenance bootstrap; once the package exists, every subsequent release will use GitHub-hosted OIDC trusted publishing with provenance, staged publication, and explicit 2FA approval.
