# Adapt WPT with reviewed hybrid mappings

Imperative, identity, exception, asynchronous, and mixed-scope WPT cases will map to manually authored Operation Fixtures, while stable declarative tables may use narrow deterministic generators that fail closed when their pinned source shape changes. SheetOM will not build a general WPT JavaScript translator or maintain a WPT fork because either would become a second, fragile CSSOM implementation.
