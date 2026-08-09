# Preserve live identity after rule detachment

Rule lists, rules, and declaration objects will have stable cached identities backed by Rule Records rather than array indices. Deletion and whole-sheet replacement will detach old rules by clearing their sheet and parent relationships while retaining independently mutable state, following Chromium where Firefox diverges on mutation after detachment.
