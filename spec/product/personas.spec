// Personas — who interacts with SpecForge
//
// First-class entity kinds declared by @specforge/product.
// Referenced by journeys via the persona field (JourneyPersona edge).

persona developer "Developer" {
  description      "A software engineer who writes .spec files, runs CLI commands, and uses IDE features to specify, validate, and export structured context for AI agents."
  technical_level  expert
  status           active
  goals [
    "Write .spec files that compile cleanly on first attempt",
    "Get instant feedback on broken references and orphan entities",
    "Export structured context that makes AI agents produce correct output",
    "Format, validate, and trace specs as part of daily workflow",
  ]
  pain_points [
    "Cryptic error messages that don't pinpoint the root cause",
    "No structured format for feeding context to AI coding assistants",
  ]
  tags ["end_user", "primary"]
}

persona ci "CI Pipeline" {
  description      "An automated CI/CD pipeline that validates spec files, gates on coverage thresholds, and checks formatting as part of continuous integration."
  technical_level  non_technical
  status           active
  goals [
    "Fail the build when spec validation errors exist",
    "Gate merges on minimum spec coverage thresholds",
    "Enforce canonical formatting via specforge format --check",
    "Produce machine-readable output for downstream pipeline stages",
  ]
  pain_points [
    "No standard machine-readable format for quality gates",
    "Manual wiring of coverage thresholds to CI exit codes",
  ]
  tags ["automation", "primary"]
}

persona agent "AI Agent" {
  description      "An AI coding assistant, PM agent, compliance agent, or any LLM-based tool that consumes the SpecForge graph protocol for structured context. Interacts via CLI or MCP."
  technical_level  non_technical
  status           active
  goals [
    "Receive structured, typed context instead of ambiguous prose",
    "Query the graph at multiple resolutions to fit context windows",
    "Stay synchronized with spec changes via delta notifications",
    "Achieve 70-85% first-attempt accuracy on implementation tasks",
  ]
  pain_points [
    "Prose ambiguity leading to ~30% first-attempt accuracy",
    "No stable schema for structured context consumption",
  ]
  tags ["automation", "primary"]
}

persona architect "Architect" {
  description      "A technical lead or system architect who reviews traceability chains, visualizes the entity graph, and exports structured data for planning and analysis."
  technical_level  expert
  status           active
  goals [
    "Verify full traceability from features to tests",
    "Visualize the dependency graph for architectural review",
    "Export graph data for external dashboards and analysis tools",
    "Identify orphan entities and traceability gaps",
  ]
  pain_points [
    "Invisible traceability gaps between features and tests",
    "Manual cross-cutting review across disconnected artifacts",
  ]
  tags ["end_user", "secondary"]
}

persona product_manager "Product Manager" {
  description      "A product manager or technical program manager who uses SpecForge exports to track feature completion, review milestone progress, and communicate delivery status to stakeholders."
  technical_level  intermediate
  status           active
  goals [
    "Review milestone completion ratios and identify blocked features",
    "Trace deliverable scope through journeys and modules",
    "Export graph data for roadmap dashboards and stakeholder reports",
    "Understand feature dependency ordering for prioritization decisions",
  ]
  pain_points [
    "Scattered status information across disconnected tools",
    "No automated gap detection between plans and implementation",
  ]
  tags ["end_user", "secondary"]
}

persona contributor "Extension Contributor" {
  description      "A developer who authors SpecForge Wasm extensions, custom providers, or grammar contributions to extend the compiler with new domain vocabulary."
  technical_level  advanced
  status           active
  goals [
    "Scaffold a new Wasm extension with specforge extension init",
    "Define entity kinds, edge types, and validation rules in a manifest",
    "Test extensions locally before publishing to a registry",
    "Author custom grammars for extension-specific syntax highlighting",
  ]
  pain_points [
    "Undocumented extension APIs requiring source-code reading",
    "No local testing harness for Wasm extensions",
  ]
  tags ["end_user", "secondary"]
}
