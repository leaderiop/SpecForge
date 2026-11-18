// Journeys: two, each listed by a deliverable, one by both.

use "product/channels"
use "product/features"
use "product/personas"

journey record_an_entry "Record an Entry" {
  persona  writer
  channels [cli]
  features [store_entries]
  priority critical
  tags     ["daily"]
  flow     ["Writer opens the store", "Writer saves an entry", "Writer sees the entry listed"]
}

journey browse_entries "Browse Entries" {
  persona  writer
  channels [cli, web]
  flow     ["Writer lists the entries", "Writer opens one"]
}
