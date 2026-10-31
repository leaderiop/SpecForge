import assert from "node:assert/strict";
import { test } from "node:test";

import { createVault, open, seal } from "../src/vault.js";

const key = Buffer.alloc(32, 7);

test("a sealed vault opens to what was sealed", () => {
  const vault = createVault();
  vault.secrets.db = "hunter2";
  assert.deepEqual(open(seal(vault, key), key).secrets, { db: "hunter2" });
});

test("a tampered vault does not open", () => {
  const blob = seal(createVault(), key);
  blob[blob.length - 1] ^= 1;
  assert.throws(() => open(blob, key));
});

test("another key does not open it", () => {
  assert.throws(() => open(seal(createVault(), key), Buffer.alloc(32, 8)));
});
