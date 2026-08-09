# Own the Rule Tree and isolate syntax codecs

SheetOM's Rule Tree owns live node identity, parentage, declarations, recovered observable values, and both serialization contracts. CSS Tree frames forgiving syntax and recovery, while Lightning CSS is isolated behind a Rule Codec for recognized valid structures and safe printing; neither third-party AST becomes the persistent CSSOM model or leaks through exported types.
