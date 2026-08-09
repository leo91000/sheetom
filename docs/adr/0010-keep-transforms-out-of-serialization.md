# Keep transforms out of serialization

Safe stylesheet serialization will emit current CSSOM state without target compilation, prefixing, rule merging, or other Lightning CSS transformations. A future `compile` operation may transform a Compile Snapshot so optimization cannot alter live rule identity or observable CSSOM structure.
