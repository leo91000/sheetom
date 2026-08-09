# Protect source and release references

The public repository will protect `main` with required CI and disabled force-push and deletion, protect `v*` tags from rewriting, and place later OIDC publication behind a protected GitHub environment. It will not require a second reviewer while maintained by one person, avoiding a protection rule that the project cannot satisfy.
