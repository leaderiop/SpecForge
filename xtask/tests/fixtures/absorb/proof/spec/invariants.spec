// One invariant with three obligations: the crate's tests state two of them.

invariant writes_are_durable "Writes Are Durable" {
  guarantee "An acknowledged write survives a crash."
  risk      high
  verify unit "a killed writer leaves the old entry"
  verify unit "an acknowledged write is on disk"
  verify unit "the store never reads half an entry"
}
