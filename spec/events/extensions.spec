// Extension registration and registry events

use "types/wasm"
use "types/zero-entity-core"
// ── Extension Registration Events ──────────────────────────────

event extension_entity_types_registered "Extension Entity Types Registered" {
  channel   "extensions.entity_types_registered"

  payload {
    extensionName     string
    entityKindCount   integer
    edgeTypeCount     integer
  }

  // observability-only — emitted for tracing/logging, no downstream consumers

  verify integration "emits after extension entity types are registered"

}

event provider_schemes_registered "Provider Schemes Registered" {
  channel   "extensions.provider_schemes_registered"

  payload {
    extensionName     string
    schemeCount       integer
  }

  // observability-only — emitted for tracing/logging, no downstream consumers

  verify integration "emits after provider schemes are registered"

}

event registry_integrity_verified "Registry Integrity Verified" {
  channel   "extensions.registry_integrity_verified"

  payload {
    extensionName     string
    hashMatch         boolean
  }

  // observability-only — emitted for tracing/logging, no downstream consumers

  verify integration "emits after registry integrity check completes"

}

// ── Provider Events ──────────────────────────────────────

event provider_configured "Provider Configured" {
  channel   "extensions.provider_configured"

  payload {
    alias           string
    extensionName   string
    schemeCount     integer
    kindCount       integer
  }


  verify integration "emits provider_configured with correct alias and schemeCount"

}

event provider_ref_validated "Provider Ref Validated" {
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
  channel   "extensions.registry_resolved"

  payload {
    extensionName   string
    registryAlias   string
    version         string
    wasmSizeBytes   integer
  }


  verify integration "emits registry_resolved with correct extensionName and version"
  verify integration "consumer verify_registry_integrity receives event"

}

event registry_search_completed "Registry Search Completed" {
  channel   "extensions.registry_search_completed"

  payload {
    query           string
    totalResults    integer
    registriesQueried integer
  }


  verify integration "emits registry_search_completed with correct totalResults"

}

event extension_published_to_registry "Extension Published to Registry" {
  channel   "extensions.extension_published_to_registry"

  payload {
    extensionName   string
    version         string
    registryAlias   string
    sha256          string
    publishTimeMs   integer
  }


  verify integration "emits extension_published_to_registry with correct extensionName and sha256"

}

event registries_configured "Registries Configured" {
  channel   "extensions.registries_configured"

  payload {
    registryCount   integer
    configs         RegistryConfig[]
  }


  verify integration "emits registries_configured after registry configuration is parsed"

}

// ── Registry Authentication Events ──────────────────────────

event registry_authenticated "Registry Authenticated" {
  channel   "extensions.registry_authenticated"

  payload {
    registryUrl     string
    authMethod      string
    success         boolean
    timestamp       timestamp
  }


  verify integration "emits registry_authenticated with correct registryUrl and authMethod"

}

event registry_credentials_validated "Registry Credentials Validated" {
  channel   "extensions.registry_credentials_validated"

  payload {
    registryUrl     string
    valid           boolean
    timestamp       timestamp
  }


  verify integration "emits registry_credentials_validated with correct registryUrl and valid status"

}

event registry_logged_out "Registry Logged Out" {
  channel   "extensions.registry_logged_out"

  payload {
    registryAlias   string
    timestamp       timestamp
  }


  verify integration "emits registry_logged_out with correct registryAlias"

}

// support_private_registries has no dedicated event — it is a composite
// behavior that delegates to authenticate_registry_request (which emits
// registry_authenticated) and resolve_registry_source (which emits
// registry_resolved). No additional event is needed.

event keyword_extension_index_generated "Keyword Extension Index Generated" {
  channel   "extensions.keyword_extension_index_generated"

  payload {
    keywordCount    integer
    extensionCount  integer
  }


  verify integration "emits after keyword extension index is generated"

}

event registry_request_retry_exhausted "Registry Request Retry Exhausted" {
  channel   "extensions.registry_request_retry_exhausted"

  payload {
    registryUrl     string
    lastStatusCode  integer
    retryCount      integer
    timestamp       timestamp
  }


  verify integration "emits after all retry attempts are exhausted"

}

event extension_removed {
  channel   "extensions.extension_removed"

  payload {
    extension_name             string
    entity_kinds_unregistered  string[]
  }


  verify integration "removing extension emits event with unregistered kinds"

}
