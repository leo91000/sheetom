---
status: accepted
---

# Preserve conditional import conditions and type stepped math before folding

The regular stylesheet API exposes the parsed URL, layer, supports condition,
and media of an import. It now reads those fields from the owned rule tree.
Splitting serialized text at parentheses can treat a quoted parenthesis inside
a supports condition as its closing delimiter and lose the remaining media
query. The CSS parser also retains a following general-enclosed media function
as media when the import already has a supports condition. SheetOM enables
this during CSSOM parsing and mutation. The supports value
is read-only, uses the browser's declaration separator spacing, and survives
edits to the live media list and serialization. Invalid import mutations fail
atomically. Regular stylesheet parsing retains invalid source as opaque CSS
under the existing lossless authoring contract.

The shared math parser checks result types before it simplifies `round()`,
`mod()`, and `rem()`. A length and percentage can combine in a
length-percentage context. A plain number and percentage remain distinct
types. A number-valued `round()` may omit its step, which defaults to 1.
A numeric unit step is omitted in canonical output if the function remains
unresolved. When folding a scalar round, the step uses its positive magnitude and nearest
ties go toward the upper multiple.

The changes preserve expressions that still depend on context. Chromium
retains `round(up, sign(1em))` in authored state. Firefox and WebKit fold this
example to `calc(1)`. The differential probe follows Chromium for the
unresolved expression. Related color selection, media matching, and used
values remain browser evaluation.

[CSS Values and Units](https://drafts.csswg.org/css-values-4/#round-func) ·
[CSS Cascade](https://drafts.csswg.org/css-cascade-5/#at-import) ·
[CSSOM](https://drafts.csswg.org/cssom/#the-cssimportrule-interface)
