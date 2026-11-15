// Features: a done one with every field a requirement keeps, and a deprecated one without a reason.

use "product/refs"

feature store_entries "Store Entries" {
  status       done
  problem      "Entries are lost when the editor closes."
  solution     "Write each entry to the store as it is saved."
  description  "The store is a folder of files, one per entry."
  priority     high
  effort       s
  owner        "Alice Martin"
  contributors ["Bob Stone"]
  refs         [gh.issue:42]
  tags         ["storage"]
  acceptance   ["A saved entry is on disk after a crash"]
}

feature old_export "Old Export" {
  status  deprecated
  problem "Entries could only be exported as one large file."
}
