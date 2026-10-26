// Phases of the roadmap. Each phase states its goal (problem), its approach
// (solution) and its success criteria (acceptance).

feature phase_shorten "Phase 1: shorten a link" {
  status     done
  priority   critical
  effort     s
  owner      "tech-lead"
  problem    "A long URL is hard to share in a chat message or on a slide."
  solution   "Accept a URL and return a short code that stands for it."
  acceptance ["A valid URL gets a code of 7 characters", "An invalid URL is refused with a reason"]
}

feature phase_redirect "Phase 2: follow a short link" {
  status     in_progress
  priority   critical
  effort     s
  owner      "tech-lead"
  problem    "A short code is useless until it takes the reader to the original URL."
  solution   "Resolve a code to its URL and redirect; unknown codes get a clear not-found page."
  acceptance [
    "A known code redirects to exactly the URL that was shortened",
    "An unknown code returns not found, never a redirect",
  ]
  depends_on [phase_shorten]
}

feature phase_click_counts "Phase 3: count clicks" {
  status     proposed
  priority   medium
  effort     m
  owner      "tech-lead"
  problem    "The person who shared a link cannot tell whether anyone used it."
  solution   "Count each redirect against its code and show the total."
  acceptance ["The count goes up by one for each redirect"]
  depends_on [phase_redirect]
}
