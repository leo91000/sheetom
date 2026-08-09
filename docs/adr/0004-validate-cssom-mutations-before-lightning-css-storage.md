# Validate CSSOM mutations before Lightning CSS storage

Lightning CSS will provide parsing and transformation machinery but will not define which `setProperty` mutations are browser-compatible. A CSSOM validation layer will parse each mutation independently, reject invalid known-property values and unsupported non-custom names without changing state, and preserve valid custom properties before committing a value to the stylesheet model.
