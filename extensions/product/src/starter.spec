// {project} — product specification
//
// Uses @specforge/product entity kinds: feature, journey, milestone, deliverable, etc.
//
// Try: specforge check

spec "{project}" {
  version "{version}"
}

persona developer "Software developer" {
  description     "Builds and ships the product from the command line"
  technical_level expert
  status          active
}

channel cli "Command-line interface" {
  description "The terminal the developer works in"
  status      active
}

feature user_auth "User authentication" {
  status   proposed
  priority high
  problem  "Users need secure access to the system"
}

journey onboarding "New user onboarding" {
  persona  developer
  channels [cli]
  features [user_auth]
  flow     """
    1. Developer installs the CLI
    2. Developer signs in with their credentials
  """
}

module core "Core module" {
  features [user_auth]
}

milestone mvp "Minimum Viable Product" {
  status        planned
  features      [user_auth]
  modules       [core]
  exit_criteria ["Core auth flow works end-to-end"]
}

deliverable app "Application" {
  status        draft
  artifact_type cli
  journeys      [onboarding]
  modules       [core]
  milestones    [mvp]
}
