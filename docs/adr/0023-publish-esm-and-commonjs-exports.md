# Publish ESM and CommonJS exports

The npm package will publish ESM and CommonJS entry points backed by one TypeScript declaration surface and one runtime-independent implementation. This supports Node and Bun module conventions while giving Deno a native ESM path without introducing runtime-specific CSSOM behavior.
