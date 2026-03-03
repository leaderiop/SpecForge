spec "test-chain" {
  version "1.0"
  plugins ["@specforge/product"]
  persona developer "Developer" {
    role "A software developer"
  }
  surface cli "CLI" {
    type terminal
  }
}

invariant chain_data_integrity "Chain Data Integrity" {
  guarantee """all chain data MUST be validated"""
  enforced_by [chain_validate]
  risk high
}

behavior chain_validate "Chain Validate" {
  invariants [chain_data_integrity]
  contract """the system MUST validate chain input"""
  verify unit "check chain validation"
}

feature chain_validation "Chain Validation" {
  behaviors [chain_validate]
  problem """chain data can be invalid"""
  solution """validate chain data before processing"""
  status active
}

capability chain_data_entry "Chain Data Entry" {
  persona developer
  surface cli
  features [chain_validation]
  flow """developer enters data via CLI"""
}

deliverable chain_cli_tool "Chain CLI Tool" {
  type app
  capabilities [chain_data_entry]
}
