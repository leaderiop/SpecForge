// The vault: a map of names to secrets, sealed with AES-256-GCM under a key from the environment.
import { createCipheriv, createDecipheriv, randomBytes } from "node:crypto";

export function createVault() {
  return { secrets: {} };
}

export function seal(vault, key) {
  const iv = randomBytes(12);
  const cipher = createCipheriv("aes-256-gcm", key, iv);
  const data = Buffer.concat([cipher.update(JSON.stringify(vault.secrets), "utf8"), cipher.final()]);
  return Buffer.concat([iv, cipher.getAuthTag(), data]);
}

export function open(blob, key) {
  const decipher = createDecipheriv("aes-256-gcm", key, blob.subarray(0, 12));
  decipher.setAuthTag(blob.subarray(12, 28));
  const text = Buffer.concat([decipher.update(blob.subarray(28)), decipher.final()]).toString("utf8");
  return { secrets: JSON.parse(text) };
}
