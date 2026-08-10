# Gate an explicit runtime and platform matrix

Superseded by ADR 0104.

Version 0.1 will gate Node 22 and 24 on Linux x64, Windows x64, and macOS arm64, plus pinned Bun and Deno on Linux x64, by installing the packed tarball and exercising its ESM and CommonJS interfaces where supported. Node 26 begins as a scheduled non-blocking signal, and other Lightning CSS platforms remain best-effort until SheetOM tests them.
