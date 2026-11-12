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
  items       ["rust:ledger::Entry", "rust:ledger::add"]
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
  plan        v1.0/01/01-01
  text        "An added entry keeps its cents and its memo"
  requirement LEDG-01
  proof       ["rust:ledger::entries#an_entry_keeps_its_cents_and_memo"]
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-10"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      files "sha256:63d673c5b9c5ee157fd066b43f3744e2226855c8b52cdeaa0b245c1608bf249a"
      phase "sha256:84aaf6320ed2bbd8075c56f2f02c954c7956acff131015d4eee3bcfe6c61a2a5"
      plan  "sha256:fc63a9fa2a8e09fb34f9f285adebef3eb37a13c03085efc69b57269e0154be43"
      tests "sha256:5c2d9d8a9cfbb76aed16b13e75873114240d3d6e472d81899486b4a9763dead7"
      truth "sha256:5c0dcbf3d62cfedfc2a027192a00d991874c13a143d96d6c04643a3842855d02"
    }
  }
}

truth TR2 "A memo with a tab or a line break is refused" {
  plan        v1.0/01/01-01
  text        "A memo with a tab or a line break is refused"
  requirement LEDG-01
  proof       ["rust:ledger::entries#a_memo_with_a_tab_is_refused"]
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-10"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      files "sha256:63d673c5b9c5ee157fd066b43f3744e2226855c8b52cdeaa0b245c1608bf249a"
      phase "sha256:84aaf6320ed2bbd8075c56f2f02c954c7956acff131015d4eee3bcfe6c61a2a5"
      plan  "sha256:fc63a9fa2a8e09fb34f9f285adebef3eb37a13c03085efc69b57269e0154be43"
      tests "sha256:7b48111f72c3e52917163d195dcbd433e0f505cbd000ca11770c50b7fa935bd8"
      truth "sha256:1d94ca3dd70a14e2a0898a0da2632877c6af6e4ab0073ef94a5caef6f19beff2"
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
    at    "2026-11-10"
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
    at    "2026-11-10"
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
  items       ["rust:ledger::total"]
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
  plan        v1.0/01/01-02
  text        "The total is the sum of every entry"
  requirement LEDG-02
  proof       ["rust:ledger::totals#the_total_sums_every_entry"]
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-10"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      files "sha256:1459840987fb0a16372fced22c100dc52fc96150a5d085f426b6788f3d407748"
      phase "sha256:84aaf6320ed2bbd8075c56f2f02c954c7956acff131015d4eee3bcfe6c61a2a5"
      plan  "sha256:241622c4d942d74ac5cc08d6342c247077ac968e0b2c156ae47f973f54c4911e"
      tests "sha256:8c006cc312715af6d0612ab43b38d36b0b6352efa3b6413d5b3121eaf16f9f88"
      truth "sha256:9942605a77ab24e353738562d77037c4398c5e001c973169b2d5cb858380e819"
    }
  }
}

truth TR2 "A total beyond the range of an i32 is exact" {
  plan        v1.0/01/01-02
  text        "A total beyond the range of an i32 is exact"
  requirement LEDG-02
  proof       ["rust:ledger::totals#total_beyond_i32_is_exact"]
  verdict gsd/E501 verified {
    judge "gsd-verifier@1.1.0"
    model "test-model"
    at    "2026-11-10"
    inputs {
      cited "sha256:e02204fbbdd3035a64321ee96a30990a9af1f26d9ee1a344a1232a2b0020f2ff"
      files "sha256:1459840987fb0a16372fced22c100dc52fc96150a5d085f426b6788f3d407748"
      phase "sha256:84aaf6320ed2bbd8075c56f2f02c954c7956acff131015d4eee3bcfe6c61a2a5"
      plan  "sha256:241622c4d942d74ac5cc08d6342c247077ac968e0b2c156ae47f973f54c4911e"
      tests "sha256:d7a733503fd25fda5b3eb75939b2f918e4ee63f1916697245e68eb2b73e143cb"
      truth "sha256:067a5adefa18d3967bfa42380f67001823ecdc5922571050d4962ae947651578"
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
    at    "2026-11-10"
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
    at    "2026-11-10"
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
  plan        v1.0/02/02-01
  text        "An added entry is written to ledger.tsv"
  requirement LEDG-03
}

truth TR2 "A restarted ledger shows the entries it had" {
  plan        v1.0/02/02-01
  text        "A restarted ledger shows the entries it had"
  requirement LEDG-03
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
