// spec-driven-method — a spec-driven method (in the style of GSD or SpecKit)
// expressed with the builtin extensions, applied to a small link shortener.
//
// The method's layers map onto kinds:
//   roadmap milestone  -> milestone      (product/roadmap.spec)
//   phase              -> feature        (product/phases.spec)
//   shipped artifact   -> deliverable    (product/roadmap.spec)
//   requirement        -> behavior       (behaviors/links.spec)
//   must-hold truth    -> invariant      (invariants/links.spec)
//   verification step  -> verify obligation on a behavior or an invariant

spec "spec-driven-method" {
  version "0.1.0"
}
