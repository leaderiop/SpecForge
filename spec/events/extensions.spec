// Extension registration and registry events

use types/wasm
use types/zero-entity-core
use behaviors/extensions

// ── Extension Registration Events ──────────────────────────────

event extension_entity_types_registered "Extension Entity Types Registered" {
  trigger   register_extension_entity_types
  channel   "extensions.entity_types_registered"

  payload {
    extensionName     string
    entityKindCount   integer
    edgeTypeCount     integer
  }

  // observability-only — emitted for tracing/logging, no downstream consumers
  consumers []

  verify integration "emits after extension entity types are registered"

}

event provider_schemes_registered "Provider Schemes Registered" {
  trigger   register_provider_schemes
  channel   "extensions.provider_schemes_registered"

  payload {
    extensionName     string
    schemeCount       integer
  }

  // observability-only — emitted for tracing/logging, no downstream consumers
  consumers []

  verify integration "emits after provider schemes are registered"

}

event registry_integrity_verified "Registry Integrity Verified" {
  trigger   verify_registry_integrity
  channel   "extensions.registry_integrity_verified"

  payload {
    extensionName     string
    hashMatch         boolean
  }

  // observability-only — emitted for tracing/logging, no downstream consumers
  consumers []

  verify integration "emits after registry integrity check completes"

}

// ── Provider Events ──────────────────────────────────────

event provider_configured "Provider Configured" {
  trigger   load_provider_configurations
  channel   "extensions.provider_configured"

  payload {
    alias           string
    extensionName   string
    schemeCount     integer
    kindCount       integer
  }

  consumers [register_provider_schemes]

  verify integration "emits provider_configured with correct alias and schemeCount"

}

event provider_ref_validated "Provider Ref Validated" {
  trigger   validate_provider_refs
  channel   "extensions.provider_ref_validated"

  payload {
    scheme          string
    kind            string
    identifier      string
    valid           boolean
    providerAlias   string
  }

  verify integration "emits provider_ref_validated with correct scheme and validation result"

}

// ── Registry Events ─────────────────────────────────────────

event registry_resolved "Registry Resolved" {
  trigger   resolve_registry_source
  channel   "extensions.registry_resolved"

  payload {
    extensionName   string
    registryAlias   string
    version         string
    wasmSizeBytes   integer
  }

  consumers [verify_registry_integrity]

  verify integration "emits registry_resolved with correct extensionName and version"
  verify integration "consumer verify_registry_integrity receives event"

}

event registry_search_completed "Registry Search Completed" {
  trigger   search_registry
  channel   "extensions.registry_search_completed"

  payload {
    query           string
    totalResults    integer
    registriesQueried integer
  }

  consumers []

  verify integration "emits registry_search_completed with correct totalResults"

}

event extension_published_to_registry "Extension Published to Registry" {
  trigger   publish_to_registry
  channel   "extensions.extension_published_to_registry"

  payload {
    extensionName   string
    version         string
    registryAlias   string
    sha256          string
    publishTimeMs   integer
  }

  consumers []

  verify integration "emits extension_published_to_registry with correct extensionName and sha256"

}

event registries_configured "Registries Configured" {
  trigger   configure_registries
  channel   "extensions.registries_configured"

  payload {
    registryCount   integer
    configs         RegistryConfig[]
  }

  consumers [resolve_registry_source]

  verify integration "emits registries_configured after registry configuration is parsed"

}

// ── Registry Authentication Events ──────────────────────────

event registry_authenticated "Registry Authenticated" {
  trigger   authenticate_registry_request
  channel   "extensions.registry_authenticated"

  payload {
    registryUrl     string
    authMethod      string
    success         boolean
    timestamp       timestamp
  }

  consumers []

  verify integration "emits registry_authenticated with correct registryUrl and authMethod"

}

event registry_credentials_validated "Registry Credentials Validated" {
  trigger   validate_registry_credentials
  channel   "extensions.registry_credentials_validated"

  payload {
    registryUrl     string
    valid           boolean
    timestamp       timestamp
  }

  consumers []

  verify integration "emits registry_credentials_validated with correct registryUrl and valid status"

}

event registry_logged_out "Registry Logged Out" {
  trigger   logout_registry
  channel   "extensions.registry_logged_out"

  payload {
    registryAlias   string
    timestamp       timestamp
  }

  consumers []

  verify integration "emits registry_logged_out with correct registryAlias"

}

// support_private_registries has no dedicated event — it is a composite
// behavior that delegates to authenticate_registry_request (which emits
// registry_authenticated) and resolve_registry_source (which emits
// registry_resolved). No additional event is needed.

event keyword_extension_index_generated "Keyword Extension Index Generated" {
  trigger   generate_keyword_extension_index
  channel   "extensions.keyword_extension_index_generated"

  payload {
    keywordCount    integer
    extensionCount  integer
  }

  consumers []

  verify integration "emits after keyword extension index is generated"

}

event registry_request_retry_exhausted "Registry Request Retry Exhausted" {
  trigger   retry_registry_request
  channel   "extensions.registry_request_retry_exhausted"

  payload {
    registryUrl     string
    lastStatusCode  integer
    retryCount      integer
    timestamp       timestamp
  }

  consumers []

  verify integration "emits after all retry attempts are exhausted"

}

event extension_removed {
  trigger   [remove_extension, provide_mcp_remove_extension_tool]
  channel   "extensions.extension_removed"

  payload {
    extension_name             string
    entity_kinds_unregistered  string[]
  }

  consumers []

  verify integration "removing extension emits event with unregistered kinds"

}
