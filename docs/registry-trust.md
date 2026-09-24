# Registry trust model (v1)

How SpecForge keeps `specforge add` / `specforge update` trustworthy when
installing third-party extensions from a registry, what the system guarantees,
and what it deliberately does not (yet).

## What is verified

Every registry install performs three checks, in order:

1. **Integrity** — the downloaded wasm bytes must match the `sha256` the
   registry reports (transfer-level check).
2. **Publisher signature** — packages uploaded via `specforge publish` carry an
   Ed25519 signature over the canonical payload
   `{name, version, wasmSha256, manifestSha256, signedAt}`. The client
   re-derives the payload from the **downloaded wasm bytes** and the
   **manifest the registry serves** and verifies the signature with the public
   key carried inside the signature object. A tampered binary *or* a swapped
   manifest breaks verification.
3. **Key policy (TOFU)** — on the first verified install the publisher's key id
   is **pinned** (recorded in `specforge.lock` and in
   `~/.specforge/known-keys.json`). Later installs must present the same key.

The registry stores and serves signatures, but is **not the trust anchor**:
even a fully compromised registry cannot forge a valid publisher signature —
it can only replay previously signed packages (see [residual risks](#residual-risks)).

## Failure behavior

| Situation | Behavior |
|---|---|
| Package unsigned | Refused. `--allow-unsigned` accepts the risk explicitly |
| Signature invalid (tampered wasm/manifest) | **Always refused** — `--allow-unsigned` does not bypass a broken signature |
| Key differs from the pinned key | Refused with both key ids; interactive re-pin offered (`y/N`); `--yes` accepts non-interactively for CI |
| Key on your `denied_keys` list | Refused outright (config-level revocation) |
| Key on your `trusted_keys` allowlist | Accepted without a prior pin (pre-seeded trust) |

## Managing keys

All publisher key state lives in `~/.specforge/known-keys.json`:

```json
{
  "pins": { "@acme/tools": "c0a2809dc5bb6c54" },
  "trusted_keys": ["9be31c0f77aa2d10"],
  "denied_keys": ["deadbeef00000000"]
}
```

- **Rotate your own key**: sign with the new key; consumers see the key-change
  flow and re-pin after explicit consent.
- **Revoke a key**: add its key id to `denied_keys` (yours or a compromised
  third party's). There is no server-side revocation in v1 — every consumer
  decides via their own config.
- **Pre-seed trust**: ship a `known-keys.json` with `trusted_keys` for
  air-gapped environments.

`specforge.lock` records the key id beside the wasm hash for every signed
registry install, so pins are auditable per project.

## Namespace ownership

- **First claim wins**: the first publish into a scope (`@acme/...`) registers
  that scope to the publishing token's publisher identity.
- **Only the owner publishes** into a claimed scope; anyone else gets
  `SCOPE_OWNED`. Tokens scoped by the registry admin (`@web`-style) are
  additionally limited to their own scope and cannot claim others.
- **Publisher identity**: the registry assigns an account id
  (`acct_...`) at first claim; `publisher` metadata is that id — never a
  self-chosen label. There are no Verified/Community badges; the signature
  key id is the trust signal.
- Scope reassignment (recovery after losing a token) is a registry-admin
  operation in v1.

## Residual risks (stated plainly)

- **Registry-controlled version lists**: `specforge update` resolves "latest"
  from the registry, and a compromised registry can serve an *older*, still
  validly signed version (a downgrade within the semver range you accept).
  Version-list signatures (TUF-style) are deferred past v1. Pin exact versions
  via `specforge.lock` when this matters.
- **Replay**: a signed package stays signed forever. If a publisher's *old*
  release is vulnerable, its signature is still valid; review the version you
  install, not just the signature.
- **TOFU first contact**: the first install trusts whichever key signs the
  first package you see. If your first install happens over a compromised
  channel, the pin anchors the wrong key. Pre-seed `trusted_keys` for
  high-assurance setups.
- **No key transparency**: v1 has no log proving a publisher uses one key
  consistently across all users. Compare pins with your team out of band.
- **Unsigned packages**: registries may accept them (enforcement is a server
  policy). `specforge add` refuses them unless you pass `--allow-unsigned`.

## Runtime sandbox note

Extension sandboxing is enforced at execution time by the host (see
[extension protocol](extension-protocol.md)). The pinned wasm runtime
currently carries an open advisory (RUSTSEC-2026-0269); remote installs are
gated on clearing it. See `.cargo/audit.toml` and the tracking issue for the
current status.
