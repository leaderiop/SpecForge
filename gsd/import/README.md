# `@gsd/import`

**Status: being designed; nothing is built yet.** `specforge gsd import`: reads an existing `.planning/` once and returns edit plans that write
`spec/gsd/` (K5). One way only; after the import no Markdown is read again. The parity suite imports real GSD
output with it to compare graphs. Every GSD file it reads maps to kinds in
[the file table](../core/spec/README.md#every-gsd-file-maps-to-a-kind); a file it cannot map is reported, never dropped.
