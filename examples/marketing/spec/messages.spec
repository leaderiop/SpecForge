// What we say, and the brand rules each message obeys.

use "brand"

message hero "Edit in half the time" {
  headline "Lumen Pro edits a full shoot in half the time."
  body     "Batch edits, presets your clients can approve, and export to every format."
  obeys    [name_the_product, plain_words]
}

message upgrade_offer "Upgrade offer" {
  headline "Move to Lumen Pro: your first month is on us."
  body     "Keep every photo and preset. Cancel any time."
  obeys    [name_the_product, no_unapproved_discounts, plain_words]
}

message team_seats "Seats for the whole studio" {
  headline "One Lumen Pro plan for the whole studio."
  body     "Add or remove a seat in a click. One invoice a month."
  obeys    [name_the_product, plain_words]
}
