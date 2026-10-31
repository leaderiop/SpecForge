#!/usr/bin/env node
// vault set <name> <secret>, vault get <name>, vault list
import { readFileSync, writeFileSync } from "node:fs";

import { createVault, open, seal } from "./vault.js";

const file = "vault.bin";
const key = Buffer.from(process.env.VAULT_KEY ?? "", "hex");
const [command, name, secret] = process.argv.slice(2);

let vault;
try {
  vault = open(readFileSync(file), key);
} catch {
  vault = createVault();
}

if (command === "set") {
  vault.secrets[name] = secret;
  writeFileSync(file, seal(vault, key));
} else if (command === "get") {
  console.log(vault.secrets[name] ?? "");
} else if (command === "list") {
  console.log(Object.keys(vault.secrets).join("\n"));
} else {
  console.error("usage: vault set <name> <secret> | get <name> | list");
  process.exit(2);
}
