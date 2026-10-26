// Requirements, written as behaviors: each states its contract and the
// verify obligations a test must discharge.

use "invariants/links"
use "product/phases"

behavior shorten_url "Shorten a URL" {
  category   command
  features   [phase_shorten]
  invariants [codes_are_unique]
  contract   """
    Given a URL, the service MUST refuse it with a reason when it is not
    an absolute http or https URL. Otherwise it MUST store the URL under a
    new 7-character code and return the code.
  """
  verify unit "a relative URL is refused"
  verify unit "a valid URL gets a 7-character code"
}

behavior resolve_code "Resolve a short code" {
  category   query
  features   [phase_redirect]
  invariants [redirect_is_faithful]
  contract   """
    Given a code, the service MUST redirect to the URL stored under it.
    An unknown code MUST return not found and MUST NOT redirect.
  """
  verify unit "an unknown code returns not found"
  verify integration "shorten then resolve redirects to the original URL"
}

behavior count_click "Count a click" {
  category   command
  features   [phase_click_counts]
  invariants [redirect_is_faithful]
  contract   """
    Each successful redirect MUST add one to the click count of its code.
    Counting MUST NOT delay or change the redirect.
  """
  verify unit "one redirect adds exactly one click"
}
