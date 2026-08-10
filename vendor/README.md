# Vendored CSS engine sources

SheetOM builds its Rust CSS Engine from complete upstream source snapshots stored as ordinary repository files. Git metadata is intentionally excluded; licenses and upstream test data remain in each directory.

| Directory | Upstream | Version | Commit | Source import commit |
| --- | --- | --- | --- | --- |
| `vendor/lightningcss` | `https://github.com/parcel-bundler/lightningcss` | 1.33.0 | `c6a0c3cebf3395635e61075d2c81a96a710d4910` | `bf2bb9711d9198c159568d6ab8cebe636f2f88f3` |
| `vendor/cssparser` | `https://github.com/servo/rust-cssparser` | 0.37.0 | `4c49486494fb24dc01390e3baca9698ef1744c71` | `51acfde57f79dfea8333a0e6f39654d7d671d7b5` |

To verify an untouched import, clone the upstream repository, check out the recorded commit, and compare it with the corresponding directory while excluding only upstream `.git` metadata:

```sh
git clone <upstream> /tmp/sheetom-upstream
git -C /tmp/sheetom-upstream checkout --detach <commit>
diff -qr --exclude=.git /tmp/sheetom-upstream vendor/<directory>
```

Later SheetOM modifications are kept in focused commits. General parser corrections should include upstream-style regression tests and remain separable for contribution to the recorded upstream project.
