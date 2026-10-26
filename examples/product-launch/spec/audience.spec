// Who the lamp is for, and where they meet it.

use "features"

persona remote_worker "Remote worker" {
  description     "Works from a home desk eight hours a day and notices eye strain by late afternoon."
  technical_level non_technical
  status          active
  goals           ["Comfortable light all day", "A desk that looks tidy on video calls"]
  pain_points     ["Overhead light is harsh", "Smart lamps need an app and an account"]
  key_features    [all_day_light, one_dial]
}

persona retail_buyer "Retail buyer" {
  description     "Chooses which products a homeware chain stocks for the season."
  technical_level non_technical
  status          active
  goals           ["Products that sell through in eight weeks", "Packaging that explains itself on a shelf"]
  pain_points     ["Late samples", "Unclear margins"]
  key_features    [shelf_ready_box]
}

channel web_store "Online store" {
  description       "The brand's own shop, where pre-orders open first."
  interaction_model self_service
  url               "https://example.com/halo"
  status            active
}

channel retail_partners "Retail partners" {
  description       "Homeware chains that stock the lamp in stores."
  interaction_model in_person
  status            active
}

channel newsletter "Newsletter" {
  description       "The monthly email to existing customers."
  interaction_model broadcast
  status            active
}
