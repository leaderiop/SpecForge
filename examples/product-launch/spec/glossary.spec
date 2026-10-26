use "plan"

// The team's shared words, so marketing, sales and logistics mean the same thing.

term sell_through "sell-through" {
  definition "The share of stock a retailer received that customers actually bought, over a period."
  context    "Retail partners judge a new product by its sell-through after eight weeks."
  see_also   [sell_in]
  module     retail
}

term sell_in "sell-in" {
  definition "Getting a retailer to place an order and put the product on its shelves."
  module     retail
}

term ship_week "ship week" {
  definition "The calendar week in which a pre-ordered lamp leaves the warehouse."
  aliases    ["ship date"]
  module     marketing
}
