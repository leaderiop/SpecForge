// A type body in the software extension's own syntax. The type keyword has
// a body parser, so the core grammar's E001s inside it are meant to be
// suppressed (plan 01, D10).
type Money "Money" {
  amount   Decimal @min(0)
  currency "USD" | "EUR"
  meta     { key: string }
}
