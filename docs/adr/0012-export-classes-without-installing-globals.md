# Export classes without installing globals

The package will export browser-named classes for consumers to import but will not modify `globalThis`. Applications that need browser-like globals may install those exports themselves, keeping global ownership and collision policy outside the library.
