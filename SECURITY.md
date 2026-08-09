# Security policy

## Supported releases

Before 1.0, only the latest published `0.x` minor and its active prereleases receive security fixes.

## Reporting a vulnerability

Use GitHub's **Report a vulnerability** form in the repository Security tab. Do not open a public issue for a suspected vulnerability.

Reports should include the affected SheetOM version, impact, reproduction steps, and any known mitigations. You can expect an initial acknowledgement within seven days when practical, but this is a maintainer response target rather than a contractual service-level agreement.

SheetOM parses and serializes CSS but is not a sanitizer. It does not fetch resources or execute CSS; callers are responsible for validating untrusted output before it crosses a rendering boundary.
