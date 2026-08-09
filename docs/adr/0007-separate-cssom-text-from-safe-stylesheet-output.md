# Separate CSSOM text from safe stylesheet output

Browser-facing getters and `cssText` will preserve Chromium-compatible malformed-value behavior, including omitted closing delimiters accepted by `setProperty`. The custom whole-sheet serializer will instead emit the recovered token structure with required delimiters so its output reparses without silently losing declarations; browser fidelity at the CSSOM surface must not make final stylesheet output unsafe.
