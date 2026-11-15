// Outside the sample: a behavior, an invariant, and a feature naming a through behavior (no referrer kind).

behavior other_run "Run Something Else" {
  category command
  contract "The host MUST run something else."
}

invariant other_whole "Other Is Whole" {
  guarantee "Something else is whole."
  risk      low
}
