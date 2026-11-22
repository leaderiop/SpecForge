plan 01-01 "The entry model and add" {
  phase            v1.0/01
  type             execute
  files_modified   ["Cargo.toml", "Cargo.lock", "src/lib.rs", "tests/entries.rs"]
  requirements     [LEDG-01]
  objective        "The entry model and `add`."
  purpose          "every later phase builds on entries in integer cents."
  output           "`src/lib.rs` with `Entry`, `add`, `line` and `parse`, and its tests."
  context_refs     ["spec/project.spec", "spec/v1.0.spec"]
  verification     ["`cargo test` passes."]
  success_criteria ["LEDG-01 holds: a user can add an entry in integer cents with a memo"]
}

task T1 "The entry model" {
  plan        v1.0/01/01-01
  type        auto
  files       ["Cargo.toml", "Cargo.lock", "src/lib.rs"]
  action      "Create the `ledger` package (no dependencies) and `src/lib.rs` with `Entry { cents: i64, memo: String }` and `add(entries, cents, memo)`, which refuses a memo holding a tab or a line break; `line` and `parse` write and read the store's `<cents>\t<memo>` lines."
  verify_text "cargo build"
  done        "add appends an entry and refuses a memo with a tab"
}

task T2 "Tests for entries" {
  plan        v1.0/01/01-01
  type        auto
  files       ["tests/entries.rs"]
  action      "Test that an entry keeps its cents and memo, that a memo with a tab is refused, and that a store line reads back as its entry."
  verify_text "cargo test --test entries"
  done        "The entry tests pass"
}

truth TR1 "An added entry keeps its cents and its memo" {
  plan v1.0/01/01-01
  text "An added entry keeps its cents and its memo"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-22T11:16:26Z"
    cited ["src/lib.rs", "tests/entries.rs"]
    inputs {
      cited "sha256:8d806444455bbedadb383ff90ddb7cfd47df1d254ada5fe8785579e4ba25d527"
      phase "sha256:76ba62b7f83934de8bc00643622e1759620cbfdddc86d1b000431e3de8e5cf2e"
      plan  "sha256:fc63a9fa2a8e09fb34f9f285adebef3eb37a13c03085efc69b57269e0154be43"
      tests "none"
      truth "sha256:c86d96a51c86b58f8cf68be094c1a9f9d1c78e6ac9eb553f638d48f68ecf2976"
    }
  }
}

truth TR2 "A memo with a tab or a line break is refused" {
  plan v1.0/01/01-01
  text "A memo with a tab or a line break is refused"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-22T11:16:29Z"
    cited ["src/lib.rs", "tests/entries.rs"]
    inputs {
      cited "sha256:8d806444455bbedadb383ff90ddb7cfd47df1d254ada5fe8785579e4ba25d527"
      phase "sha256:76ba62b7f83934de8bc00643622e1759620cbfdddc86d1b000431e3de8e5cf2e"
      plan  "sha256:fc63a9fa2a8e09fb34f9f285adebef3eb37a13c03085efc69b57269e0154be43"
      tests "none"
      truth "sha256:0b08cbb8ca3fa0d3951e1ca3cd4316628ac2163fd927c664865c8b6424e4cc17"
    }
  }
}

artifact AR1 "Entry and add" {
  plan     v1.0/01/01-01
  path     "src/lib.rs"
  provides "Entry and add"
  contains "pub fn add"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-22T11:16:22Z"
    inputs {
      artifact "sha256:8ebd47d01e81cc35b478dbacf6145ce45ed675719044f750c5a37c93c61e4e53"
      cited    "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      file     "sha256:e8b1829b23b587a8a7a87cc7575d06c5e4e765b23f290e6a10e406a85114949b"
      plan     "sha256:fc63a9fa2a8e09fb34f9f285adebef3eb37a13c03085efc69b57269e0154be43"
    }
  }
}

key_link KL1 "The tests use the entries" {
  plan v1.0/01/01-01
  from "tests/entries.rs"
  to   "src/lib.rs"
  via  "use ledger::{add, line, parse, Entry}"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-22T11:16:24Z"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      from  "sha256:85bd846176bf7d526981cd2346f7cedcb481ca2d439329d82c3a3512e78e8997"
      link  "sha256:923133b0e2cf4ec364fa46d243f655e48830cc6ad2c6b7896a9ad0b34d0917ad"
      plan  "sha256:fc63a9fa2a8e09fb34f9f285adebef3eb37a13c03085efc69b57269e0154be43"
      to    "sha256:6f38b4028d386a8d7a3859ce3a1ecce7100e8f7a664f634279c7186ad2880011"
    }
  }
}

plan 01-02 "The total" {
  phase            v1.0/01
  type             execute
  depends_on       [01-01]
  files_modified   ["src/lib.rs", "src/main.rs", "tests/totals.rs"]
  requirements     [LEDG-02]
  objective        "The total and the command line."
  purpose          "a user gets the total of a store without writing code."
  output           "`total` in `src/lib.rs`, and `src/main.rs` with `add` and `total`."
  context_refs     ["spec/project.spec", "spec/v1.0.spec", "spec/plans.spec"]
  verification     ["`cargo test` passes and `ledger total` prints the sum of a store."]
  success_criteria ["LEDG-02 holds: the total of the entries is exact to the cent"]
}

task T1 "The total" {
  plan        v1.0/01/01-02
  type        auto
  files       ["src/lib.rs", "tests/totals.rs"]
  action      "Add `total(entries)`: the sum of every entry's cents as an `i64`, so no realistic ledger overflows it. Test a mixed sum and a sum beyond `i32::MAX`."
  verify_text "cargo test --test totals"
  done        "The total tests pass"
}

task T2 "The command line" {
  plan        v1.0/01/01-02
  type        auto
  files       ["src/main.rs"]
  action      "Create `src/main.rs`: `add <cents> <memo>` prints the entry's store line, `total` reads a store on stdin and prints its total, anything else prints usage and exits 2."
  verify_text "printf '100\\ta\\n250\\tb\\n' | cargo run -q -- total"
  done        "The command prints the total of the store"
}

truth TR1 "The total is the sum of every entry" {
  plan v1.0/01/01-02
  text "The total is the sum of every entry"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-22T11:16:37Z"
    cited ["src/lib.rs", "tests/totals.rs"]
    inputs {
      cited "sha256:3803df3719927c59068f89f563a255937c82a6c0de55e21a42891fde04c05680"
      phase "sha256:76ba62b7f83934de8bc00643622e1759620cbfdddc86d1b000431e3de8e5cf2e"
      plan  "sha256:241622c4d942d74ac5cc08d6342c247077ac968e0b2c156ae47f973f54c4911e"
      tests "none"
      truth "sha256:061ad3a412efd9bdbc230a2fcd74bc658ce7ab317750b1f39824b05cc29b699d"
    }
  }
}

truth TR2 "A total beyond the range of an i32 is exact" {
  plan v1.0/01/01-02
  text "A total beyond the range of an i32 is exact"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-22T11:16:39Z"
    cited ["src/lib.rs", "tests/totals.rs"]
    inputs {
      cited "sha256:3803df3719927c59068f89f563a255937c82a6c0de55e21a42891fde04c05680"
      phase "sha256:76ba62b7f83934de8bc00643622e1759620cbfdddc86d1b000431e3de8e5cf2e"
      plan  "sha256:241622c4d942d74ac5cc08d6342c247077ac968e0b2c156ae47f973f54c4911e"
      tests "none"
      truth "sha256:921c9b02fd8892671461c808fec33466cc5a3defab925cc766b6b1e65e8f87d2"
    }
  }
}

artifact AR1 "ledger add and ledger total" {
  plan     v1.0/01/01-02
  path     "src/main.rs"
  provides "ledger add and ledger total"
  contains "ledger::total"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-22T11:16:32Z"
    inputs {
      artifact "sha256:7de8537a704d13adb5a6fb269e0b6aba85ce72bb143aa74d9a153fcd7c82c141"
      cited    "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      file     "sha256:593968e0f15a17778e298e0eb22381d8e13fadf554512e12f42f8b6241717c3d"
      plan     "sha256:241622c4d942d74ac5cc08d6342c247077ac968e0b2c156ae47f973f54c4911e"
    }
  }
}

key_link KL1 "The command line totals through the library" {
  plan v1.0/01/01-02
  from "src/main.rs"
  to   "src/lib.rs"
  via  "ledger::total(&entries)"
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-22T11:16:34Z"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      from  "sha256:46c00d71d061cb6854c44001f4941db663cac3b28065dd4cffd10acd38e2bac2"
      link  "sha256:7b621e7c0158ad0dc541fd3c3f1e75de77d386095aaa8283a1af0915ca9f3433"
      plan  "sha256:241622c4d942d74ac5cc08d6342c247077ac968e0b2c156ae47f973f54c4911e"
      to    "sha256:6f38b4028d386a8d7a3859ce3a1ecce7100e8f7a664f634279c7186ad2880011"
    }
  }
}

plan 02-01 "Save and load ledger.tsv" {
  phase            v1.0/02
  type             execute
  files_modified   ["src/store.rs", "src/lib.rs", "src/main.rs", "tests/store.rs"]
  requirements     [LEDG-03]
  objective        "Save and load `ledger.tsv`."
  purpose          "an entry is never lost between two runs."
  output           "`src/store.rs`, and the command line reading and writing the store around each command."
  context_refs     ["spec/project.spec", "spec/v1.0.spec", "spec/plans.spec"]
  verification     ["`cargo test` passes and a second run of the command line sees the first run's entry."]
  success_criteria ["LEDG-03 holds: entries survive a restart"]
}

task T1 "The store" {
  plan        v1.0/02/02-01
  type        auto
  files       ["src/store.rs", "src/lib.rs", "tests/store.rs"]
  action      "Create `store::load(path)` (a missing file is an empty ledger) and `store::save(path, entries)` (write to a temporary file, then rename), over `line` and `parse`. Test a save followed by a load, and a missing file."
  verify_text "cargo test --test store"
  done        "The store tests pass"
}

task T2 "The command line saves" {
  plan        v1.0/02/02-01
  type        auto
  files       ["src/main.rs"]
  action      "Load the entries from `ledger.tsv` in the working directory before each command, save them after `add`, and make `total` read the store instead of stdin."
  verify_text "cargo run -q -- add 100 a && cargo run -q -- total"
  done        "The second command totals the entry the first one added"
}

truth TR1 "An added entry is written to ledger.tsv" {
  plan v1.0/02/02-01
  text "An added entry is written to ledger.tsv"
}

truth TR2 "A restarted ledger shows the entries it had" {
  plan v1.0/02/02-01
  text "A restarted ledger shows the entries it had"
}

artifact AR1 "load and save of ledger.tsv" {
  plan     v1.0/02/02-01
  path     "src/store.rs"
  provides "load and save of ledger.tsv"
  contains "pub fn save"
}

key_link KL1 "The command line loads the store" {
  plan v1.0/02/02-01
  from "src/main.rs"
  to   "src/store.rs"
  via  "ledger::store::load"
}

summary 01-01-SUMMARY "The entry model and add: summary" {
  plan                   v1.0/01/01-01
  status                 complete
  one_liner              "Entries in integer cents with `add`, and the store's one-line format."
  subsystem              "entries"
  tags                   ["rust", "std"]
  provides               ["Entry and add"]
  affects                ["01-02", "persistence"]
  tech_patterns          ["an entry is plain data: { cents, memo }"]
  requirements_completed [LEDG-01]
}

deliverable D1 "An added entry keeps its cents and its memo" {
  summary        v1.0/01/01-01/01-01-SUMMARY
  description    "An added entry keeps its cents and its memo"
  requirement    LEDG-01
  human_judgment false
  checks         [
    { kind unit ref "tests/entries.rs#an_entry_keeps_its_cents_and_memo" }
    { kind unit ref "tests/entries.rs#a_memo_with_a_tab_is_refused" }
  ]
}

summary 01-02-SUMMARY "The total: summary" {
  plan                   v1.0/01/01-02
  status                 complete
  one_liner              "`total` as an `i64` of cents, and `ledger add` and `ledger total` over it."
  subsystem              "entries"
  tags                   ["rust", "cli"]
  provides               ["total", "ledger add and ledger total"]
  affects                ["persistence", "listing"]
  tech_patterns          ["the command line is a thin switch over src/lib.rs"]
  requirements_completed [LEDG-02]
}

deliverable D1 "The total is exact, beyond the range of an i32 too" {
  summary        v1.0/01/01-02/01-02-SUMMARY
  description    "The total is exact, beyond the range of an i32 too"
  requirement    LEDG-02
  human_judgment false
  checks         [
    { kind unit ref "tests/totals.rs#the_total_sums_every_entry" }
    { kind unit ref "tests/totals.rs#total_beyond_i32_is_exact" }
  ]
}

deliverable D2 "The command line prints the total of a store" {
  summary        v1.0/01/01-02/01-02-SUMMARY
  description    "The command line prints the total of a store"
  human_judgment true
  rationale      "The output wording is a judgement a person makes at the terminal"
}
