# Return canonical CSS strings

The safe stylesheet `serialize` extension will return one deterministic, readable CSS string. Byte encoding, file output, source maps, and minification remain caller or future compile concerns so serialization has one portable contract across Node, Bun, and Deno.
