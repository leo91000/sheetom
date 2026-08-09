# Limit the product to Authoring CSSOM

The library will model stylesheet and rule objects, declaration mutation, and stylesheet serialization. It will not implement DOM attachment, the cascade, layout, or computed styles, because those require a rendering environment and are unnecessary for the intended stylesheet-authoring workflow.
