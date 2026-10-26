// What the lamp does for the people who buy it.

feature all_day_light "Light that follows the day" {
  status     done
  priority   high
  effort     m
  problem    "Desk light that is right at 9am is too cold and bright by 6pm."
  solution   "The lamp warms its colour slowly through the afternoon, with no setup."
  acceptance [
    "Colour shifts from cool to warm between 2pm and 8pm by default",
    "A reviewer can describe the effect without reading the manual",
  ]
  owner      "product"
}

feature one_dial "One dial, no app" {
  status     done
  priority   high
  effort     s
  problem    "Smart lamps hide basic controls behind an app and an account."
  solution   "A single dial sets brightness; pressing it switches off the daylight shift."
  acceptance ["Every control works with the dial alone"]
  owner      "product"
}

feature shelf_ready_box "Box that sells on the shelf" {
  status     in_progress
  priority   medium
  effort     s
  problem    "In a store, nobody is there to explain what the lamp does."
  solution   "The box shows the morning-to-evening colour change in one picture."
  acceptance [
    "A shopper can say what the lamp does after five seconds with the box",
    "The box fits the partners' standard shelf depth",
  ]
  depends_on [all_day_light]
  owner      "design"
}

feature preorder "Pre-order with a ship date" {
  status     done
  priority   critical
  effort     m
  problem    "Early fans want to commit before stock lands, but only with a clear date."
  solution   "The online store takes pre-orders and shows the confirmed ship week."
  acceptance ["The ship week on the page matches the logistics plan"]
  owner      "growth"
}
