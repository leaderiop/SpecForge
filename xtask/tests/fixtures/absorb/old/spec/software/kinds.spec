// The software kinds absorb drops whole: an event, a type with one annotated member, and a port with a method and an
// obligation.

event entry_saved "Entry Saved" {
  channel "store.entry_saved"
  payload SavedEntry
}

type SavedEntry "Saved Entry" {
  path string
  note string @optional
}

port EntryStore "Entry Store" {
  direction outbound
  category  "storage"
  method save(path: string, text: string) -> Result<SavedEntry, string>
  verify integration "a saved entry is read back by its path"
}
