// A seed file: its invariant joins whole and draws its referrers.

invariant w_isolated "Extensions Are Isolated" {
  guarantee "No extension reads another's memory."
  risk      high
}
