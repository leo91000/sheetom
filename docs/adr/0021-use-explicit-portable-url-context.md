# Use explicit portable URL context

`parseStyleSheet` will accept optional `href` and `baseURL` metadata, with `baseURL` defaulting to `href` and then `about:blank`. Constructed sheets retain standard null `href`, and no API will infer URL context from a process working directory, keeping behavior deterministic across Node, Bun, and Deno.
