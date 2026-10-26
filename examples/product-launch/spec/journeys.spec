// How each persona moves from hearing about the lamp to having it.

use "audience"
use "features"

journey discover_and_preorder "Discover and pre-order" {
  persona  remote_worker
  channels [newsletter, web_store]
  features [all_day_light, preorder]
  priority critical
  flow     [
    "Reads the newsletter story about afternoon eye strain",
    "Watches the 20-second day-to-evening clip on the store page",
    "Pre-orders and sees the confirmed ship week",
    "Gets a shipping email the week it leaves the warehouse",
  ]
}

journey stock_the_shelf "Stock the shelf" {
  persona  retail_buyer
  channels [retail_partners]
  features [shelf_ready_box, one_dial]
  priority high
  flow     [
    "Receives a sample and the sell-in deck",
    "Tries the dial in a store mock-up",
    "Places a first order for 40 stores",
  ]
}
