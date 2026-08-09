# Use a caret Lightning CSS dependency through an adapter

The package will declare `lightningcss` as an ordinary `^1.33.0` dependency so package managers can deduplicate it with a consumer's compatible installation. All access will pass through one internal adapter, and CI will test the minimum plus newest satisfying version so parser or AST behavior changes can be isolated from the CSSOM model.
