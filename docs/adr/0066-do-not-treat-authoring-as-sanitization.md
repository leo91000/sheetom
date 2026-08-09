# Do not treat authoring as sanitization

SheetOM never fetches imports or URLs and never executes CSS, but it preserves valid authored URLs, imports, and browser features rather than sanitizing them. Callers remain responsible for trust and sanitization at the Rendering Boundary because silently removing valid CSS would violate the Authoring CSSOM contract.
