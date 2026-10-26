// Who the phases serve, and the end-to-end journey the milestone must make work.

use "product/phases"

persona link_sharer "Link sharer" {
  description     "Shares links in chats, slides and printed material."
  technical_level intermediate
  status          active
  goals           ["Links short enough to type", "Links that never break"]
  key_features    [phase_shorten, phase_redirect]
}

channel web "Web" {
  description       "The shortener's web page and its redirects."
  interaction_model request_response
  status            active
}

journey share_a_link "Share a link" {
  persona  link_sharer
  channels [web]
  features [phase_shorten, phase_redirect]
  priority critical
  flow     [
    "Pastes a long URL and gets a short code",
    "Puts the short link on a slide",
    "A reader opens the short link and lands on the original page",
  ]
}
