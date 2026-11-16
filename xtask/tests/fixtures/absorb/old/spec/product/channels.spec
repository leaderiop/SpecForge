// Channels: two active ones.

channel cli "Command Line" {
  description       "The store's command-line tool."
  interaction_model request_response
  status            active
  tags              ["primary"]
}

channel web "Web Page" {
  description "A page that lists the entries."
}
