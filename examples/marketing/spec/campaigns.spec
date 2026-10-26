// The launch, as three campaigns: who each targets, what it says, and when.

use "audiences"
use "messages"

campaign launch_social "Launch on social" {
  goal      "2,000 trial sign-ups from freelancers in the launch month."
  channel   social
  starts    2027-03-02
  ends      2027-03-31
  budget    15000
  audiences [freelance_designers]
  messages  [hero]
}

campaign studio_outreach "Studio outreach" {
  goal      "150 studios on a team plan by the end of April."
  channel   email
  starts    2027-03-09
  ends      2027-04-30
  budget    6000
  audiences [small_studios]
  messages  [team_seats, hero]
}

campaign customer_upgrade "Upgrade current customers" {
  goal      "5% of free users move to Lumen Pro."
  channel   email
  starts    2027-03-02
  ends      2027-03-16
  budget    2000
  audiences [current_customers]
  messages  [upgrade_offer]
}
