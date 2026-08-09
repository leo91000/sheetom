# Accept strings and use forgiving sheet parsing

Stylesheet replacement, insertion, and parsing APIs will accept text rather than encoded byte buffers. Whole-sheet replacement and `parseStyleSheet` will use Forgiving Sheet Parse behavior with optional diagnostics, while `insertRule` retains browser-compatible strict parsing, exceptions, and atomicity.
