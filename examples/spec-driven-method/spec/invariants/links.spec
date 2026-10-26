// Must-hold truths: what stays true whatever phase is being built.

invariant codes_are_unique "Short codes are unique" {
  guarantee "No two stored links share a short code."
  risk      high
  verify property "shortening many URLs never produces the same code twice"
}

invariant redirect_is_faithful "A redirect goes where it was told" {
  guarantee "A code always redirects to exactly the URL stored for it, byte for byte."
  risk      critical
  verify property "for any stored URL, resolving its code returns that URL unchanged"
}
