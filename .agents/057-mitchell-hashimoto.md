# 057 — Mitchell Hashimoto

**Cluster:** C8 — Registries, packaging & supply-chain trust
**Roster role:** Terraform/Vagrant registry & extension model (SpecForge's extension design is explicitly Terraform-inspired); Ghostty
**SpecForge anchors:** specforge add/remove/publish (crates/specforge-cli/src/add.rs, remove.rs, publish.rs), extension manifest model (ManifestV2, crates/specforge-wasm/src/manifest_bridge.rs), extensions/{software,product,governance,formal}

## Why this engineer
Hashimoto co-created the two defining extension-registry designs of the last decade: Vagrant's box distribution and Terraform's provider/module Registry — the model SpecForge's Wasm extension system consciously copies (namespaced names, versioned published artifacts, a thin CLI talking to a dumb registry). His "Abandoning Rubygems" post is a firsthand postmortem of exactly the registry-design mistakes SpecForge's specforge add/publish and specforge-registry-server must avoid.

## References for SpecForge
**Key works**
- [Terraform](https://github.com/hashicorp/terraform) — GitHub, hashicorp/terraform, 2014. The lineage of SpecForge's zero-core-plus-extensions model: a tiny declarative core that gets all capabilities from registry-distributed plugins.
- [Terraform Registry docs](https://developer.hashicorp.com/terraform/registry) — HashiCorp Developer. The canonical reference for registry UX conventions (namespacing, versioned zips, checksums, publish requirements) that specforge publish and specforge-registry-server should track.
- [Vagrant](https://github.com/hashicorp/vagrant) — GitHub, hashicorp/vagrant, 2010. Early precedent for versioned binary artifacts fetched by a thin CLI — the same shape as specforge add pulling signed .wasm blobs.
- [Abandoning Rubygems](https://mitchellh.com/writing/abandoning-rubygems) — mitchellh.com, 2013. Why Vagrant left a public gem registry for its own versioned distribution: direct evidence for controlling your artifact channel.
- **Vagrant: Up and Running** — O'Reilly, 2013. The write-up of the workflow-first CLI DX that specforge init/add/check mirrors.

## Study first
1. Terraform Registry publishing model: versioned artifacts + SHASUMS, namespacing, marketplace requirements
2. "Abandoning Rubygems" (2013) — registry control vs convenience
3. Ghostty 1.0 reflection (mitchellh.com/writing/ghostty-1-0-reflection) — shipping durable OSS tooling in a systems language
