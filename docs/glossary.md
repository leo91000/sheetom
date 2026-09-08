# SheetOM authoring compatibility

SheetOM represents stylesheets for inspection, editing, and serialization outside
a rendering environment.

## Language

**Authoring CSSOM parity**:
Agreement with the selected CSSOM contract for parsing, observable rule and
declaration state, mutation, and serialization, including specialized rule
interfaces. It does not include cascade evaluation, computed styles, rendering,
or expanding mixins into ordinary declarations.
_Avoid_: Full CSS support, rendering parity

**June 2026 CSS target**:
The cumulative set of CSS features that reached Web Platform Baseline Newly
available on or before June 30, 2026, including features from earlier years.
_Avoid_: Baseline Widely available, June-only features

**Explicit experimental scope**:
Native CSS mixins and custom functions included in the authoring compatibility
work independently of their membership in the June 2026 CSS target. Their
experimental status remains explicit.
_Avoid_: Sass mixins, all experimental CSS

**Compatibility Baseline**:
A release-specific record of SheetOM compatibility evidence tied to exact
browser, specification, test, and dependency revisions. It is distinct from Web
Platform Baseline's classification of browser availability.

**Rule retention**:
Keeping a rule available for serialization without claiming a specialized
editable interface or evaluation of its effects.
_Avoid_: Full rule support

**Experimental mixin contract**:
The exact CSSWG draft revision selected for native mixin authoring interfaces,
available by default and explicitly identified as experimental. Browser
observations are supporting evidence rather than a substitute for that contract.

**Authoring surface**:
The applicable syntax, rule interfaces, observable state, mutations, and
serialization behavior associated with a CSS feature. A feature's rendering
effects are outside its SheetOM authoring surface.

**Parity completion**:
Coverage of the complete selected feature inventory with no known unimplemented
authoring surfaces, supported by applicable parsing, mutation, serialization,
invalid-input, and native/WASM evidence. Missing evidence is an open item, and
browser differences require explicit compatibility resolutions.
