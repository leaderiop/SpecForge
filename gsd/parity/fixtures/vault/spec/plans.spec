plan 01-01 "The sealed store" {
  phase            v1.0/01
  type             execute
  files_modified   ["package.json", "src/vault.js", "test/vault.test.js"]
  requirements     [SEC-01]
  objective        "The sealed store."
  purpose          "a secret is encrypted before it is written, which is the core value of the project."
  output           "`src/vault.js` with `createVault`, `seal` and `open`, and its tests."
  context_refs     [".planning/PROJECT.md", ".planning/ROADMAP.md", ".planning/STATE.md"]
  verification     ["`npm test` passes."]
  success_criteria ["SEC-01 holds: a secret is stored sealed"]
}

task T1 "Seal and open" {
  plan        v1.0/01/01-01
  type        auto
  files       ["package.json", "src/vault.js"]
  action      "Create `package.json` and `src/vault.js`: `seal` encrypts the secrets with AES-256-GCM under a fresh IV, `open` reverses it and throws when the tag does not verify."
  verify_text "node -e \"import('./src/vault.js').then(m => console.log(m.seal(m.createVault(), Buffer.alloc(32)).length))\""
  done        "seal returns a blob that open turns back into the vault"
}

task T2 "Tests for seal and open" {
  plan        v1.0/01/01-01
  type        auto
  files       ["test/vault.test.js"]
  action      "Test a round trip, a tampered blob and another key."
  verify_text "npm test"
  done        "The vault tests pass"
}

trust_boundary TB1 "process to vault.bin" {
  plan          v1.0/01/01-01
  boundary      "process → vault.bin"
  description   "the sealed file is on a disk other users can read and write"
  data_crossing "encrypted secrets"
}

threat T-01-01 "Seal under a key that stays off the disk" {
  plan        v1.0/01/01-01
  category    information_disclosure
  component   "src/vault.js seal"
  severity    high
  disposition mitigate
  mitigation  "AES-256-GCM under a key that never reaches the disk"
}

threat T-01-02 "Verify the auth tag" {
  plan        v1.0/01/01-01
  category    tampering
  component   "src/vault.js open"
  severity    high
  disposition mitigate
  mitigation  "the GCM auth tag is verified; a tampered file throws"
}

threat T-01-SC "Package installs" {
  plan        v1.0/01/01-01
  category    tampering
  component   "npm/pip/cargo installs"
  severity    high
  disposition mitigate
  mitigation  "package-legitimacy gate + blocking human checkpoint for [ASSUMED]/[SUS]"
}

plan 01-02 "The command line" {
  phase            v1.0/01
  type             execute
  depends_on       [01-01]
  files_modified   ["src/cli.js"]
  requirements     [SEC-01, SEC-02]
  objective        "The command line."
  purpose          "a user drives the vault without writing code."
  output           "`src/cli.js` with `set`, `get` and `list`."
  context_refs     [
    ".planning/PROJECT.md",
    ".planning/ROADMAP.md",
    ".planning/phases/01-core/01-01-SUMMARY.md",
  ]
  verification     ["`npm test` passes and `get` prints what `set` stored."]
  success_criteria ["SEC-02 holds: a stored secret is read back by its name"]
}

task T1 "set, get and list" {
  plan        v1.0/01/01-02
  type        auto
  files       ["src/cli.js"]
  action      "Read the key from `VAULT_KEY` (hex), open `vault.bin` when it exists, run the command and seal the vault back after `set`."
  verify_text "node src/cli.js get db"
  done        "get prints what set stored"
}

trust_boundary TB1 "shell to process" {
  plan        v1.0/01/01-02
  boundary    "shell → process"
  description "the secret is typed as an argument and the key is an environment variable"
}

threat T-01-02 "Secret on argv" {
  plan        v1.0/01/01-02
  category    information_disclosure
  component   "src/cli.js argv"
  severity    high
  disposition mitigate
  mitigation  "accept the secret on stdin instead of argv"
  status      open
}

threat T-01-03 "List shows names" {
  plan        v1.0/01/01-02
  category    information_disclosure
  component   "src/cli.js list"
  severity    low
  disposition accept
  mitigation  "list prints names only, never values"
  status      closed
}

threat T-01-SC "Package installs" {
  plan        v1.0/01/01-02
  category    tampering
  component   "npm/pip/cargo installs"
  severity    high
  disposition mitigate
  mitigation  "package-legitimacy gate + blocking human checkpoint for [ASSUMED]/[SUS]"
}

plan 02-01 "Export a secret for a recipient" {
  phase            v1.0/02
  type             execute
  files_modified   ["src/share.js", "src/cli.js", "test/share.test.js"]
  requirements     [SHARE-01]
  objective        "Export a secret for a recipient."
  purpose          "a secret reaches another person without the vault key."
  output           "`src/share.js`, and the command line `share` command."
  context_refs     [
    ".planning/PROJECT.md",
    ".planning/ROADMAP.md",
    ".planning/phases/01-core/01-02-SUMMARY.md",
  ]
  verification     ["`npm test` passes."]
  success_criteria ["SHARE-01 holds"]
}

task T1 "shareSecret" {
  plan        v1.0/02/02-01
  type        auto
  files       ["src/share.js", "test/share.test.js"]
  action      "Create `shareSecret(vault, name, recipientKey)` returning one secret sealed under the recipient's key. Test that the wrong key does not open it."
  verify_text "npm test"
  done        "The share tests pass"
}

task T2 "The share command" {
  plan        v1.0/02/02-01
  type        auto
  files       ["src/cli.js"]
  action      "Add `vault share <name> <recipient-key-hex>`, which prints the sealed secret as hex."
  verify_text "node src/cli.js share db 00"
  done        "share prints a hex string"
}
